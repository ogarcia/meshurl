use base64::Engine;
use meshurl::encoder::{encode_url, modem_preset_from_str, region_code_from_str};
use meshurl::models::{
    ChannelInfo, ChannelRole, DEFAULT_PSK, LoRaInfo, MeshtasticDisplay, POSITION_OPTIONS, PskType,
    generate_random_psk, get_preset_params, hash_phrase_to_psk,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Padding, Paragraph},
};
use ratatui_textarea::{CursorMove, TextArea};
use std::io::Write;
use std::process::Command;

use crate::tui::app::{ActivePanel, EncodeDrawState, EncodeState};
use crate::tui::widgets::{
    centered_popup, channel_list_item, channel_scroll_indicator, channel_total_lines,
    lora_info_lines, lora_scroll_info, truncate_to_columns,
};

/// Preferred width of the channel editing popup.
const CHANNEL_POPUP_WIDTH: u16 = 35;
/// Preferred width of the LoRa editing popup.
const LORA_POPUP_WIDTH: u16 = 40;
/// Preferred size of the single-line input overlays.
const INPUT_OVERLAY_WIDTH: u16 = 40;
const INPUT_OVERLAY_HEIGHT: u16 = 3;
/// Columns a popup field value may occupy before it is shortened.
const POPUP_VALUE_COLUMNS: usize = 22;

/// PSK mode as offered by the channel popup.
///
/// The library's [`PskMode`] carries the key material inside the variant, which
/// suits the CLI parser (`psk_mode=base64:...`) but leaves the popup with two
/// places holding the same value. They drifted apart: editing wrote to
/// `psk_value` while the channel was built from the enum payload, so every PSK
/// typed into the TUI was silently dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PskModeKind {
    Default,
    None,
    Random,
    Base64,
    Passphrase,
}

impl PskModeKind {
    /// Modes in the order the popup cycles through them.
    const ALL: &'static [PskModeKind] = &[
        PskModeKind::Default,
        PskModeKind::None,
        PskModeKind::Random,
        PskModeKind::Base64,
        PskModeKind::Passphrase,
    ];

    /// Whether the mode needs the user to supply a value.
    fn needs_value(self) -> bool {
        matches!(self, PskModeKind::Base64 | PskModeKind::Passphrase)
    }

    /// Returns the next or previous mode, wrapping around.
    fn cycle(self, forward: bool) -> Self {
        let modes = Self::ALL;
        let current = modes.iter().position(|m| *m == self).unwrap_or(0);
        let len = modes.len();
        let next = if forward {
            (current + 1) % len
        } else {
            (current + len - 1) % len
        };
        modes[next]
    }
}

impl std::fmt::Display for PskModeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            PskModeKind::Default => "Default",
            PskModeKind::None => "None",
            PskModeKind::Random => "Random",
            PskModeKind::Base64 => "Base64",
            PskModeKind::Passphrase => "Passphrase",
        };
        write!(f, "{}", name)
    }
}

pub struct ChannelPopupState {
    pub channel_index: Option<usize>,
    pub name: String,
    pub psk_mode: PskModeKind,
    pub psk_value: String,
    pub uplink_enabled: bool,
    pub downlink_enabled: bool,
    pub position_index: usize,
    pub muted: bool,
    pub selected_field: usize,
    pub editing_name: bool,
    pub name_textarea: TextArea<'static>,
    pub editing_psk: bool,
    pub psk_textarea: TextArea<'static>,
}

pub struct LoRaPopupState {
    pub region: String,
    pub modem_preset: String,
    pub tx_power: i32,
    pub hop_limit: u32,
    pub channel_num: u32,
    pub tx_enabled: bool,
    pub use_preset: bool,
    pub override_frequency: f32,
    pub frequency_offset: f32,
    pub sx126x_rx_boosted_gain: bool,
    pub override_duty_cycle: bool,
    pub pa_fan_disabled: bool,
    pub ignore_mqtt: bool,
    pub ok_mqtt: bool,
    pub selected_field: usize,
}

impl LoRaPopupState {
    pub fn new() -> Self {
        Self {
            region: "EU868".to_string(),
            modem_preset: "LongFast".to_string(),
            tx_power: 0,
            hop_limit: 3,
            channel_num: 0,
            tx_enabled: true,
            use_preset: true,
            override_frequency: 0.0,
            frequency_offset: 0.0,
            sx126x_rx_boosted_gain: false,
            override_duty_cycle: false,
            pa_fan_disabled: false,
            ignore_mqtt: true,
            ok_mqtt: false,
            selected_field: 0,
        }
    }

    pub fn from_lora(lora: &LoRaInfo) -> Self {
        let region = lora.region.to_mesh_string().to_string();
        let modem_preset = lora.modem_preset.to_mesh_string().to_string();

        Self {
            region,
            modem_preset,
            tx_power: lora.tx_power,
            hop_limit: lora.hop_limit,
            channel_num: lora.channel_num,
            tx_enabled: lora.tx_enabled,
            use_preset: lora.use_preset,
            override_frequency: lora.override_frequency,
            frequency_offset: lora.frequency_offset,
            sx126x_rx_boosted_gain: lora.sx126x_rx_boosted_gain,
            override_duty_cycle: lora.override_duty_cycle,
            pa_fan_disabled: lora.pa_fan_disabled,
            ignore_mqtt: lora.ignore_mqtt,
            ok_mqtt: lora.config_ok_to_mqtt,
            selected_field: 0,
        }
    }

    pub fn to_lora_info(&self) -> LoRaInfo {
        let region = region_code_from_str(&self.region);
        let modem_preset = modem_preset_from_str(&self.modem_preset);

        let (bandwidth, spread_factor, coding_rate) = get_preset_params(modem_preset);

        LoRaInfo {
            region,
            modem_preset,
            use_preset: self.use_preset,
            tx_enabled: self.tx_enabled,
            tx_power: self.tx_power,
            bandwidth,
            spread_factor,
            coding_rate,
            hop_limit: self.hop_limit,
            channel_num: self.channel_num,
            override_duty_cycle: self.override_duty_cycle,
            sx126x_rx_boosted_gain: self.sx126x_rx_boosted_gain,
            override_frequency: self.override_frequency,
            frequency_offset: self.frequency_offset,
            pa_fan_disabled: self.pa_fan_disabled,
            ignore_mqtt: self.ignore_mqtt,
            config_ok_to_mqtt: self.ok_mqtt,
            ignore_incoming: vec![],
        }
    }
}

const LORA_REGIONS: &[&str] = &[
    "US", "EU433", "EU868", "CN", "JP", "ANZ", "KR", "TW", "RU", "IN", "NZ865", "TH", "Lora24",
    "UA433", "UA868",
];

const LORA_MODEM_PRESETS: &[&str] = &[
    "LongFast",
    "LongSlow",
    "VeryLongSlow",
    "MediumSlow",
    "MediumFast",
    "ShortSlow",
    "ShortFast",
    "LongModerate",
    "ShortTurbo",
];

const LORA_FIELDS: &[&str] = &[
    "Region",
    "Modem Preset",
    "TX Power",
    "Hop Limit",
    "Channel",
    "TX Enabled",
    "Use Preset",
    "Override Freq",
    "Freq Offset",
    "SX126x RX",
    "Duty Cycle",
    "PA Fan Disabled",
    "Ignore MQTT",
    "OK to MQTT",
    "Save",
    "Cancel",
];

impl ChannelPopupState {
    pub fn new() -> Self {
        let name_textarea = TextArea::default();
        let psk_textarea = TextArea::default();
        Self {
            channel_index: None,
            name: String::new(),
            psk_mode: PskModeKind::Default,
            psk_value: String::new(),
            uplink_enabled: false,
            downlink_enabled: false,
            position_index: 0,
            muted: false,
            selected_field: 0,
            editing_name: false,
            name_textarea,
            editing_psk: false,
            psk_textarea,
        }
    }

    pub fn from_channel(index: usize, channel: &ChannelInfo) -> Self {
        let (psk_mode, psk_value) = if channel.psk.is_empty() {
            (PskModeKind::None, String::new())
        } else if channel.psk == DEFAULT_PSK {
            (PskModeKind::Default, String::new())
        } else {
            (PskModeKind::Base64, channel.psk.clone())
        };

        let name_textarea = TextArea::default();
        let psk_textarea = TextArea::default();
        let position_precision = channel.position_precision.unwrap_or(0);
        let position_index = POSITION_OPTIONS
            .iter()
            .position(|(_, v)| *v == position_precision)
            .unwrap_or(0);
        Self {
            channel_index: Some(index),
            name: channel.name.clone(),
            psk_mode,
            psk_value,
            uplink_enabled: channel.uplink_enabled,
            downlink_enabled: channel.downlink_enabled,
            position_index,
            muted: channel.is_client_muted,
            selected_field: 0,
            editing_name: false,
            name_textarea,
            editing_psk: false,
            psk_textarea,
        }
    }

    pub fn start_editing_name(&mut self) {
        let current_name = self.name.clone();
        self.editing_name = true;
        self.name_textarea = TextArea::new(vec![current_name]);
        self.name_textarea.move_cursor(CursorMove::End);
    }

    pub fn finish_editing_name(&mut self) {
        self.name = self
            .name_textarea
            .lines()
            .first()
            .map_or(String::new(), |l| l.to_string());
        self.editing_name = false;
    }

    pub fn cancel_editing_name(&mut self) {
        self.name_textarea = TextArea::default();
        self.editing_name = false;
    }

    pub fn start_editing_psk(&mut self) {
        let current_psk = self.psk_value.clone();
        self.editing_psk = true;
        self.psk_textarea = TextArea::new(vec![current_psk]);
        self.psk_textarea.move_cursor(CursorMove::End);
    }

    pub fn cancel_editing_psk(&mut self) {
        self.psk_textarea = TextArea::default();
        self.editing_psk = false;
    }

    /// Resolves the selected mode and entered value into a channel PSK.
    ///
    /// Invalid or missing key material is reported instead of being replaced by
    /// a fallback: silently falling back to the default PSK would hand the user
    /// the well-known public key while the popup claimed their own was in use.
    fn resolve_psk(&self) -> Result<(String, PskType), String> {
        match self.psk_mode {
            PskModeKind::Default => Ok((DEFAULT_PSK.to_string(), PskType::Default)),
            PskModeKind::None => Ok((String::new(), PskType::None)),
            PskModeKind::Random => Ok((generate_random_psk()?, PskType::Aes256)),
            PskModeKind::Base64 => {
                let psk = self.psk_value.trim();
                if psk.is_empty() {
                    return Err("Enter a base64 PSK first".to_string());
                }

                let bytes = decode_base64_psk(psk)?;
                Ok((psk.to_string(), PskType::from_bytes(&bytes)))
            }
            PskModeKind::Passphrase => {
                if self.psk_value.is_empty() {
                    return Err("Enter a passphrase first".to_string());
                }
                Ok((hash_phrase_to_psk(&self.psk_value), PskType::Aes256))
            }
        }
    }

    pub fn to_channel_info(&self, default_index: usize) -> Result<(usize, ChannelInfo), String> {
        let index = self.channel_index.unwrap_or(default_index);
        let (psk, psk_type) = self.resolve_psk()?;

        Ok((
            index,
            ChannelInfo {
                index,
                role: if index == 0 {
                    ChannelRole::Primary
                } else {
                    ChannelRole::Secondary
                },
                name: self.name.clone(),
                psk,
                psk_type,
                uplink_enabled: self.uplink_enabled,
                downlink_enabled: self.downlink_enabled,
                // "Disabled" is the absence of the setting, not a precision of 0.
                position_precision: match POSITION_OPTIONS[self.position_index].1 {
                    0 => None,
                    precision => Some(precision),
                },
                is_client_muted: self.muted,
            },
        ))
    }
}

/// Frames a toast is shown for.
const TOAST_FRAMES: u8 = 120;

/// Raises a toast, replacing whatever was on screen.
fn show_toast(
    toast: &mut Option<crate::tui::app::ToastMessage>,
    toast_timer: &mut u8,
    text: &str,
    is_success: bool,
) {
    *toast = Some(crate::tui::app::ToastMessage {
        text: text.to_string(),
        is_success,
        is_uncertain: false,
    });
    *toast_timer = TOAST_FRAMES;
}

/// Decodes a base64 PSK, checking it is a usable AES key length.
fn decode_base64_psk(psk: &str) -> Result<Vec<u8>, String> {
    use base64::engine::general_purpose::STANDARD;

    let bytes = STANDARD
        .decode(psk)
        .map_err(|_| "Invalid base64 PSK".to_string())?;

    match bytes.len() {
        16 | 32 => Ok(bytes),
        _ => Err("PSK must be 16 or 32 bytes".to_string()),
    }
}

/// Renumbers channels after they have been reordered or removed.
///
/// Both the index and the role derive from the position in the list, so they
/// have to be rewritten together every time that position changes.
fn reindex_channels(channels: &mut [ChannelInfo]) {
    for (i, channel) in channels.iter_mut().enumerate() {
        channel.index = i;
        channel.role = if i == 0 {
            ChannelRole::Primary
        } else {
            ChannelRole::Secondary
        };
    }
}

/// Moves the channel selection by `delta`, keeping it inside the list.
///
/// An unbounded selection is not just a cosmetic problem: the reorder and edit
/// keys index the channel list with it.
fn move_channel_selection(list_state: &mut ListState, len: usize, delta: isize) {
    if len == 0 {
        list_state.select(None);
        return;
    }

    let last = len - 1;
    let next = match list_state.selected() {
        Some(current) => current.min(last).saturating_add_signed(delta).min(last),
        None => 0,
    };
    list_state.select(Some(next));
}

fn get_popup_fields(psk_mode: PskModeKind) -> Vec<&'static str> {
    let mut fields = vec!["Name", "PSK Mode"];

    if psk_mode.needs_value() {
        fields.push("PSK");
    }

    fields.extend_from_slice(&["Uplink", "Downlink", "Position", "Muted", "Save", "Cancel"]);
    fields
}

pub fn draw_encode_mode(f: &mut Frame, state: &mut EncodeDrawState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Min(6),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(f.area());

    let title = Paragraph::new(" 📡 MeshURL - Encode ")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(title, chunks[0]);

    let channels_title = format!(
        " 📋 Channels ({} found) ",
        state.encode_config.channels.len()
    );

    let total_lines = channel_total_lines(&state.encode_config.channels);
    let block_height = chunks[1].height;
    let selected_idx = state.encode_channels_state.selected().unwrap_or(0);

    let channels_scroll_indicator = channel_scroll_indicator(
        total_lines,
        block_height,
        selected_idx,
        false, // no scroll state in encode, use selected_idx
        0,
    );

    let channels_block = Block::default()
        .title(channels_title)
        .title_bottom(Line::from(channels_scroll_indicator).right_aligned())
        .borders(Borders::ALL)
        .padding(Padding::new(1, 1, 1, 1))
        .border_style(if state.active_panel == ActivePanel::Channels {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    if state.encode_config.channels.is_empty() {
        let help = Paragraph::new("No channels. Press [A] to add a channel")
            .style(Style::default().fg(Color::DarkGray))
            .block(channels_block);
        f.render_widget(help, chunks[1]);
    } else {
        let items: Vec<ListItem> = state
            .encode_config
            .channels
            .iter()
            .enumerate()
            .map(|(i, ch)| channel_list_item(i, ch))
            .collect();

        let list = List::new(items).block(channels_block).highlight_style(
            Style::default()
                .bg(Color::Rgb(0x1a, 0x1a, 0x1a))
                .add_modifier(ratatui::style::Modifier::BOLD),
        );
        f.render_stateful_widget(list, chunks[1], state.encode_channels_state);
    }

    let lora_title = " 📻 LoRa Config ";

    if let Some(lora) = &state.encode_config.lora {
        let scroll_info = lora_scroll_info(lora, chunks[2].height, state.lora_scroll);
        *state.lora_max_scroll = scroll_info.max_scroll;

        let all_lines = lora_info_lines(lora);

        let lines: Vec<Line> = if scroll_info.max_scroll > 0 {
            all_lines
                .into_iter()
                .skip(scroll_info.clamped_scroll as usize)
                .take(scroll_info.visible_lines)
                .collect()
        } else {
            all_lines
        };

        let lora_block = Block::default()
            .title(lora_title)
            .title_bottom(Line::from(scroll_info.indicator).right_aligned())
            .borders(Borders::ALL)
            .padding(Padding::new(1, 1, 1, 1))
            .border_style(if state.active_panel == ActivePanel::Lora {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            });

        let lora_para = Paragraph::new(lines).block(lora_block);
        f.render_widget(lora_para, chunks[2]);
    } else {
        let lora_block = Block::default()
            .title(lora_title)
            .borders(Borders::ALL)
            .padding(Padding::new(1, 1, 1, 1))
            .border_style(if state.active_panel == ActivePanel::Lora {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            });
        let help = Paragraph::new("No LoRa config. Optional - press [E] to edit")
            .style(Style::default().fg(Color::DarkGray))
            .block(lora_block);
        f.render_widget(help, chunks[2]);
    }

    let url_title = " 🔗 Generated URL ";
    let url_text = state
        .encoded_url
        .as_deref()
        .unwrap_or("(Press G to generate)");
    let url_style = if state.encoded_url.is_some() {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let url_para = Paragraph::new(url_text).style(url_style).block(
        Block::default()
            .title(url_title)
            .borders(Borders::ALL)
            .padding(Padding::new(1, 1, 0, 0))
            .border_style(if state.active_panel == ActivePanel::UrlEncode {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            }),
    );
    f.render_widget(url_para, chunks[3]);

    let copy_hint = if state.encoded_url.is_some() {
        "  [C] Copy"
    } else {
        ""
    };

    let can_reorder = state.encode_config.channels.len() >= 2;
    let reorder_hint = if can_reorder { " [+]/[-] Move" } else { "" };
    let footer_text = match state.active_panel {
        ActivePanel::Channels => format!(
            "[1] Decode  [2] Encode  [Tab/Shift+Tab] Switch  [A] Add  [D] Delete{}  [E] LoRa  [G] Generate{}  [Del] Clear",
            reorder_hint, copy_hint
        ),
        ActivePanel::Lora => format!(
            "[1] Decode  [2] Encode  [Tab/Shift+Tab] Switch  [A] Add  [E] LoRa  [G] Generate{}  [Del] Clear",
            copy_hint
        ),
        ActivePanel::UrlEncode => format!(
            "[1] Decode  [2] Encode  [Tab/Shift+Tab] Switch  [A] Add  [E] LoRa  [G] Generate{}  [Del] Clear",
            copy_hint
        ),
        ActivePanel::Url => format!(
            "[1] Decode  [2] Encode  [Tab/Shift+Tab] Switch  [A] Add  [E] LoRa  [G] Generate{}  [Del] Clear",
            copy_hint
        ),
    };
    let footer = Paragraph::new(footer_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[4]);

    if let Some(lora_state) = state.lora_popup {
        draw_lora_popup(f, lora_state, f.area());
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum CopyMethod {
    Tool,
    Osc52,
}

pub fn copy_to_clipboard(text: &str) -> Result<CopyMethod, String> {
    if Command::new("wl-copy")
        .arg(text)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Ok(CopyMethod::Tool);
    }

    if Command::new("xclip")
        .args(["-selection", "clipboard"])
        .arg(text)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Ok(CopyMethod::Tool);
    }

    if Command::new("xsel")
        .args(["--clipboard", "--input"])
        .arg(text)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Ok(CopyMethod::Tool);
    }

    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
    let osc52 = format!("\x1b]52;c;{}\x07", encoded);

    let mut stdout = std::io::stdout();
    if stdout.write_all(osc52.as_bytes()).is_ok() && stdout.flush().is_ok() {
        return Ok(CopyMethod::Osc52);
    }

    Err("Failed to copy. Install wl-clipboard or xclip".to_string())
}

pub fn handle_encode_keys(
    key: ratatui::crossterm::event::KeyEvent,
    state: &mut EncodeState,
) -> bool {
    use ratatui::crossterm::event::KeyCode;

    if state.lora_popup.is_some() {
        let popup = state.lora_popup.as_mut().unwrap();

        let result = handle_lora_popup_keys(key, popup);

        match result {
            Some(lora_info) => {
                state.encode_config.lora = Some(lora_info);
                *state.lora_popup = None;
            }
            None => {
                if key.code == KeyCode::Esc {
                    *state.lora_popup = None;
                } else if key.code == KeyCode::Enter {
                    let field = LORA_FIELDS[popup.selected_field];
                    if field == "Cancel" {
                        *state.lora_popup = None;
                    }
                }
            }
        }
        return true;
    }

    if state.channel_popup.is_some() {
        let popup = state.channel_popup.as_mut().unwrap();

        if popup.editing_psk && matches!(key.code, KeyCode::Esc) {
            popup.cancel_editing_psk();
            return true;
        }

        let result = handle_popup_keys(key, popup, state.toast, state.toast_timer);

        match result {
            Some((idx, mut channel)) => {
                if idx < state.encode_config.channels.len() && popup.channel_index.is_some() {
                    channel.index = idx;
                    channel.role = if idx == 0 {
                        ChannelRole::Primary
                    } else {
                        ChannelRole::Secondary
                    };
                    state.encode_config.channels[idx] = channel;
                    state.encode_channels_state.select(Some(idx));
                } else if state.encode_config.channels.len() < 8 {
                    channel.index = state.encode_config.channels.len();
                    channel.role = if channel.index == 0 {
                        ChannelRole::Primary
                    } else {
                        ChannelRole::Secondary
                    };
                    state.encode_config.channels.push(channel);
                    state
                        .encode_channels_state
                        .select(Some(state.encode_config.channels.len() - 1));
                }
                *state.channel_popup = None;
            }
            None => {
                if key.code == KeyCode::Esc {
                    *state.channel_popup = None;
                } else if key.code == KeyCode::Enter {
                    let popup_fields = get_popup_fields(popup.psk_mode);
                    let field = popup_fields[popup.selected_field];
                    if field == "Cancel" {
                        *state.channel_popup = None;
                    }
                }
            }
        }
        return true;
    }

    match key.code {
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if let Some(url) = state.encoded_url.clone() {
                let result = copy_to_clipboard(&url);
                let is_ok = result.is_ok();
                let is_uncertain = matches!(result, Ok(CopyMethod::Osc52));
                *state.toast = Some(crate::tui::app::ToastMessage {
                    text: match result {
                        Ok(CopyMethod::Tool) => "Copied to clipboard!".to_string(),
                        Ok(CopyMethod::Osc52) => {
                            "Seems copied (if not work install wl-clipboard or xclip)".to_string()
                        }
                        Err(e) => e,
                    },
                    is_success: is_ok,
                    is_uncertain,
                });
                *state.toast_timer = TOAST_FRAMES;
            }
            true
        }
        KeyCode::Delete => {
            state.encode_config.channels.clear();
            state.encode_config.lora = None;
            *state.encoded_url = None;
            state.encode_channels_state.select(None);
            *state.lora_scroll = 0;
            *state.lora_popup = None;
            *state.channel_popup = None;
            true
        }
        KeyCode::Char('a') | KeyCode::Char('A') => {
            if state.encode_config.channels.len() < 8 {
                *state.channel_popup = Some(ChannelPopupState::new());
            }
            true
        }
        KeyCode::Enter => {
            if let Some(selected) = state
                .encode_channels_state
                .selected()
                .and_then(|s| (s < state.encode_config.channels.len()).then_some(s))
            {
                let channel = &state.encode_config.channels[selected];
                *state.channel_popup = Some(ChannelPopupState::from_channel(selected, channel));
            }
            true
        }
        KeyCode::Char('+') => {
            // `s + 1 < len` rather than `s < len - 1`: the latter underflows on
            // an empty list, which Tab can select into.
            if let Some(idx) = state
                .encode_channels_state
                .selected()
                .and_then(|s| (s + 1 < state.encode_config.channels.len()).then_some(s))
            {
                state.encode_config.channels.swap(idx, idx + 1);
                reindex_channels(&mut state.encode_config.channels);
                state.encode_channels_state.select(Some(idx + 1));
            }
            false
        }
        KeyCode::Char('-') => {
            if let Some(idx) = state
                .encode_channels_state
                .selected()
                .and_then(|s| (s > 0 && s < state.encode_config.channels.len()).then_some(s))
            {
                state.encode_config.channels.swap(idx, idx - 1);
                reindex_channels(&mut state.encode_config.channels);
                state.encode_channels_state.select(Some(idx - 1));
            }
            false
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if let Some(selected) = state
                .encode_channels_state
                .selected()
                .and_then(|s| (s < state.encode_config.channels.len()).then_some(s))
            {
                state.encode_config.channels.remove(selected);
                reindex_channels(&mut state.encode_config.channels);
                if state.encode_config.channels.is_empty() {
                    state.encode_channels_state.select(None);
                } else if selected >= state.encode_config.channels.len() {
                    state
                        .encode_channels_state
                        .select(Some(state.encode_config.channels.len() - 1));
                }
            }
            false
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            *state.lora_popup = Some(match &state.encode_config.lora {
                Some(lora) => LoRaPopupState::from_lora(lora),
                None => LoRaPopupState::new(),
            });
            true
        }
        KeyCode::Char('g') | KeyCode::Char('G') => {
            if !state.encode_config.channels.is_empty() {
                match encode_url(state.encode_config) {
                    Ok(url) => *state.encoded_url = Some(url),
                    Err(e) => *state.encoded_url = Some(format!("Error: {}", e)),
                }
            }
            true
        }
        KeyCode::Up => {
            if *state.active_panel == ActivePanel::Channels {
                move_channel_selection(
                    state.encode_channels_state,
                    state.encode_config.channels.len(),
                    -1,
                );
            } else if *state.active_panel == ActivePanel::Lora {
                *state.lora_scroll = state.lora_scroll.saturating_sub(1);
            }
            false
        }
        KeyCode::Down => {
            if *state.active_panel == ActivePanel::Channels {
                move_channel_selection(
                    state.encode_channels_state,
                    state.encode_config.channels.len(),
                    1,
                );
            } else if *state.active_panel == ActivePanel::Lora {
                *state.lora_scroll = (*state.lora_scroll + 1).min(*state.lora_max_scroll);
            }
            false
        }
        _ => false,
    }
}

pub fn handle_encode_tab(
    key: ratatui::crossterm::event::KeyEvent,
    active_panel: &mut ActivePanel,
    channels_list_state: &mut ListState,
) {
    use ratatui::crossterm::event::KeyCode;

    let new_panel = if key
        .modifiers
        .contains(ratatui::crossterm::event::KeyModifiers::SHIFT)
        || matches!(key.code, KeyCode::BackTab)
    {
        match *active_panel {
            ActivePanel::UrlEncode => ActivePanel::Lora,
            ActivePanel::Channels => ActivePanel::UrlEncode,
            ActivePanel::Lora => ActivePanel::Channels,
            ActivePanel::Url => ActivePanel::Channels,
        }
    } else {
        match *active_panel {
            ActivePanel::UrlEncode => ActivePanel::Channels,
            ActivePanel::Channels => ActivePanel::Lora,
            ActivePanel::Lora => ActivePanel::UrlEncode,
            ActivePanel::Url => ActivePanel::Channels,
        }
    };

    if new_panel == ActivePanel::Channels && channels_list_state.selected().is_none() {
        channels_list_state.select(Some(0));
    }
    *active_panel = new_panel;
}

pub fn draw_channel_popup(f: &mut Frame, state: &ChannelPopupState) {
    let area = f.area();
    let popup_fields = get_popup_fields(state.psk_mode);
    let height = popup_fields.len() as u16 + 2;
    let popup_rect = match centered_popup(area, CHANNEL_POPUP_WIDTH, height) {
        Some(rect) => rect,
        None => return,
    };

    f.render_widget(Clear, popup_rect);

    let title_block = Block::default()
        .title(" Edit Channel ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD),
        );
    f.render_widget(title_block, popup_rect);

    let inner_rect = popup_rect.inner(ratatui::layout::Margin::new(1, 1));

    let popup_fields = get_popup_fields(state.psk_mode);

    for (i, field) in popup_fields.iter().enumerate() {
        let row_y = inner_rect.y + i as u16;
        if row_y >= inner_rect.y + inner_rect.height {
            break;
        }

        let is_selected = i == state.selected_field;

        let value = match *field {
            "Name" => truncate_to_columns(&state.name, POPUP_VALUE_COLUMNS),
            "PSK Mode" => state.psk_mode.to_string(),
            "PSK" => truncate_to_columns(&state.psk_value, POPUP_VALUE_COLUMNS),
            "Uplink" => if state.uplink_enabled { "✓" } else { "✗" }.to_string(),
            "Downlink" => if state.downlink_enabled { "✓" } else { "✗" }.to_string(),
            "Position" => POSITION_OPTIONS[state.position_index].0.to_string(),
            "Muted" => if state.muted { "✓" } else { "✗" }.to_string(),
            _ => "".to_string(),
        };

        let prefix = if is_selected { "► " } else { "  " };
        let line = if value.is_empty() {
            format!("{}{}", prefix, field)
        } else {
            format!("{}{}: {}", prefix, field, value)
        };

        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let p = Paragraph::new(line).style(style);
        f.render_widget(
            p,
            ratatui::layout::Rect::new(inner_rect.x, row_y, inner_rect.width, 1),
        );
    }

    if state.editing_name
        && let Some(overlay_rect) = centered_popup(area, INPUT_OVERLAY_WIDTH, INPUT_OVERLAY_HEIGHT)
    {
        f.render_widget(Clear, overlay_rect);

        let bg_block = Block::default()
            .title(" Name ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green))
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            );
        f.render_widget(bg_block, overlay_rect);

        let input_rect = overlay_rect.inner(ratatui::layout::Margin::new(1, 1));
        let mut textarea = state.name_textarea.clone();
        textarea.set_cursor_line_style(Style::default());
        textarea.set_block(Block::default().borders(Borders::NONE));
        f.render_widget(&textarea, input_rect);
    }

    if state.editing_psk
        && let Some(overlay_rect) = centered_popup(area, INPUT_OVERLAY_WIDTH, INPUT_OVERLAY_HEIGHT)
    {
        f.render_widget(Clear, overlay_rect);

        let psk_title = match state.psk_mode {
            PskModeKind::Base64 => " PSK (base64) ",
            PskModeKind::Passphrase => " PSK (passphrase) ",
            _ => " PSK ",
        };

        let bg_block = Block::default()
            .title(psk_title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green))
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            );
        f.render_widget(bg_block, overlay_rect);

        let input_rect = overlay_rect.inner(ratatui::layout::Margin::new(1, 1));
        let mut textarea = state.psk_textarea.clone();
        textarea.set_cursor_line_style(Style::default());
        textarea.set_block(Block::default().borders(Borders::NONE));
        f.render_widget(&textarea, input_rect);
    }
}

pub fn draw_lora_popup(f: &mut Frame, state: &LoRaPopupState, area: ratatui::layout::Rect) {
    let height = (LORA_FIELDS.len() + 2) as u16;
    let popup_rect = match centered_popup(area, LORA_POPUP_WIDTH, height) {
        Some(rect) => rect,
        None => return,
    };

    f.render_widget(Clear, popup_rect);

    let title_block = Block::default()
        .title(" Edit LoRa ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD),
        );
    f.render_widget(title_block, popup_rect);

    let inner = popup_rect.inner(ratatui::layout::Margin::new(1, 1));

    let items: Vec<Line> = LORA_FIELDS
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let is_selected = i == state.selected_field;
            let prefix = if is_selected { "► " } else { "  " };

            let value = match *field {
                "Region" => state.region.clone(),
                "Modem Preset" => state.modem_preset.clone(),
                "TX Power" => {
                    if state.tx_power == 0 {
                        "0 (default)".to_string()
                    } else {
                        format!("{} dBm", state.tx_power)
                    }
                }
                "Hop Limit" => format!("{}", state.hop_limit),
                "Channel" => format!("{}", state.channel_num),
                "TX Enabled" => if state.tx_enabled { "✓" } else { "✗" }.to_string(),
                "Use Preset" => if state.use_preset { "✓" } else { "✗" }.to_string(),
                "Override Freq" => format!("{} MHz", state.override_frequency),
                "Freq Offset" => format!("{} kHz", state.frequency_offset),
                "SX126x RX" => if state.sx126x_rx_boosted_gain {
                    "✓"
                } else {
                    "✗"
                }
                .to_string(),
                "Duty Cycle" => if state.override_duty_cycle {
                    "✓"
                } else {
                    "✗"
                }
                .to_string(),
                "PA Fan Disabled" => if state.pa_fan_disabled { "✓" } else { "✗" }.to_string(),
                "Ignore MQTT" => if state.ignore_mqtt { "✓" } else { "✗" }.to_string(),
                "OK to MQTT" => if state.ok_mqtt { "✓" } else { "✗" }.to_string(),
                _ => "".to_string(),
            };

            let has_value = !value.is_empty();
            let line = if has_value {
                format!("{}{}: {}", prefix, field, value)
            } else {
                format!("{}{}", prefix, field)
            };
            if is_selected {
                if has_value {
                    Line::from(vec![
                        Span::raw(format!("{}{}: ", prefix, field)),
                        Span::styled(
                            value,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(ratatui::style::Modifier::BOLD),
                        ),
                    ])
                } else {
                    Line::from(vec![Span::styled(
                        format!("{}{}", prefix, field),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    )])
                }
            } else {
                Line::from(line)
            }
        })
        .collect();

    let list = Paragraph::new(items)
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().fg(Color::White));
    f.render_widget(list, inner);
}

pub fn handle_lora_popup_keys(
    key: ratatui::crossterm::event::KeyEvent,
    state: &mut LoRaPopupState,
) -> Option<LoRaInfo> {
    use ratatui::crossterm::event::KeyCode;

    let is_enter = matches!(key.code, KeyCode::Enter);
    let cycle_forward = matches!(key.code, KeyCode::Right | KeyCode::Char(' '));
    let cycle_backward = matches!(key.code, KeyCode::Left);

    if !is_enter
        && !cycle_forward
        && !cycle_backward
        && !matches!(key.code, KeyCode::Up | KeyCode::Down)
    {
        return None;
    }

    match key.code {
        KeyCode::Up => {
            if state.selected_field > 0 {
                state.selected_field -= 1;
            } else {
                state.selected_field = LORA_FIELDS.len() - 1;
            }
            None
        }
        KeyCode::Down => {
            if state.selected_field < LORA_FIELDS.len() - 1 {
                state.selected_field += 1;
            } else {
                state.selected_field = 0;
            }
            None
        }
        _ => {
            let field = LORA_FIELDS[state.selected_field];
            let dir: isize = if cycle_backward { -1 } else { 1 };
            match field {
                "Save" => {
                    if is_enter {
                        return Some(state.to_lora_info());
                    }
                    None
                }
                "Cancel" => None,
                "Region" => {
                    if cycle_forward || cycle_backward {
                        let idx = LORA_REGIONS
                            .iter()
                            .position(|r| r.eq_ignore_ascii_case(&state.region))
                            .unwrap_or(0);
                        let len = LORA_REGIONS.len();
                        let new_idx = ((idx as isize) + dir + len as isize) as usize % len;
                        state.region = LORA_REGIONS[new_idx].to_string();
                    }
                    None
                }
                "Modem Preset" => {
                    if cycle_forward || cycle_backward {
                        let idx = LORA_MODEM_PRESETS
                            .iter()
                            .position(|p| p.eq_ignore_ascii_case(&state.modem_preset))
                            .unwrap_or(0);
                        let len = LORA_MODEM_PRESETS.len();
                        let new_idx = ((idx as isize) + dir + len as isize) as usize % len;
                        state.modem_preset = LORA_MODEM_PRESETS[new_idx].to_string();
                    }
                    None
                }
                "TX Power" => {
                    if cycle_forward || cycle_backward {
                        state.tx_power = if dir > 0 {
                            if state.tx_power < 30 {
                                state.tx_power + 1
                            } else {
                                0
                            }
                        } else {
                            if state.tx_power > 0 {
                                state.tx_power - 1
                            } else {
                                30
                            }
                        };
                    }
                    None
                }
                "Hop Limit" => {
                    if cycle_forward || cycle_backward {
                        state.hop_limit = if dir > 0 {
                            if state.hop_limit < 7 {
                                state.hop_limit + 1
                            } else {
                                1
                            }
                        } else {
                            if state.hop_limit > 1 {
                                state.hop_limit - 1
                            } else {
                                7
                            }
                        };
                    }
                    None
                }
                "Channel" => {
                    if cycle_forward || cycle_backward {
                        state.channel_num = if dir > 0 {
                            if state.channel_num < 255 {
                                state.channel_num + 1
                            } else {
                                0
                            }
                        } else {
                            if state.channel_num > 0 {
                                state.channel_num - 1
                            } else {
                                255
                            }
                        };
                    }
                    None
                }
                "TX Enabled" => {
                    if cycle_forward || cycle_backward {
                        state.tx_enabled = !state.tx_enabled;
                    }
                    None
                }
                "Use Preset" => {
                    if cycle_forward || cycle_backward {
                        state.use_preset = !state.use_preset;
                    }
                    None
                }
                "Override Freq" => {
                    if cycle_forward {
                        state.override_frequency = if state.override_frequency < 1000.0 {
                            state.override_frequency + 1.0
                        } else {
                            0.0
                        };
                    } else if cycle_backward {
                        state.override_frequency = if state.override_frequency > 0.0 {
                            state.override_frequency - 1.0
                        } else {
                            1000.0
                        };
                    }
                    None
                }
                "Freq Offset" => {
                    if cycle_forward {
                        state.frequency_offset = if state.frequency_offset < 100.0 {
                            state.frequency_offset + 1.0
                        } else {
                            -100.0
                        };
                    } else if cycle_backward {
                        state.frequency_offset = if state.frequency_offset > -100.0 {
                            state.frequency_offset - 1.0
                        } else {
                            100.0
                        };
                    }
                    None
                }
                "SX126x RX" => {
                    if cycle_forward || cycle_backward {
                        state.sx126x_rx_boosted_gain = !state.sx126x_rx_boosted_gain;
                    }
                    None
                }
                "Duty Cycle" => {
                    if cycle_forward || cycle_backward {
                        state.override_duty_cycle = !state.override_duty_cycle;
                    }
                    None
                }
                "PA Fan Disabled" => {
                    if cycle_forward || cycle_backward {
                        state.pa_fan_disabled = !state.pa_fan_disabled;
                    }
                    None
                }
                "Ignore MQTT" => {
                    if cycle_forward || cycle_backward {
                        state.ignore_mqtt = !state.ignore_mqtt;
                    }
                    None
                }
                "OK to MQTT" => {
                    if cycle_forward || cycle_backward {
                        state.ok_mqtt = !state.ok_mqtt;
                    }
                    None
                }
                _ => None,
            }
        }
    }
}

pub fn handle_popup_keys(
    key: ratatui::crossterm::event::KeyEvent,
    state: &mut ChannelPopupState,
    toast: &mut Option<crate::tui::app::ToastMessage>,
    toast_timer: &mut u8,
) -> Option<(usize, ChannelInfo)> {
    use ratatui::crossterm::event::KeyCode;

    if state.editing_name {
        if matches!(key.code, KeyCode::Enter) {
            state.finish_editing_name();
        }
        return None;
    }

    if state.editing_psk {
        if matches!(key.code, KeyCode::Enter) {
            let entered = state
                .psk_textarea
                .lines()
                .first()
                .map_or(String::new(), |l| l.to_string());

            // Report a malformed key while the user still has the field open.
            // An empty field is allowed through so it can be cleared; saving
            // the channel is what refuses it.
            if state.psk_mode == PskModeKind::Base64
                && !entered.trim().is_empty()
                && let Err(message) = decode_base64_psk(entered.trim())
            {
                show_toast(toast, toast_timer, &message, false);
                return None;
            }

            state.psk_value = entered;
            state.editing_psk = false;
        }
        return None;
    }

    let is_enter = matches!(key.code, KeyCode::Enter);
    let cycle_forward = matches!(key.code, KeyCode::Right | KeyCode::Char(' '));
    let cycle_backward = matches!(key.code, KeyCode::Left);

    if !is_enter
        && !cycle_forward
        && !cycle_backward
        && !matches!(key.code, KeyCode::Up | KeyCode::Down)
    {
        return None;
    }

    let popup_fields = get_popup_fields(state.psk_mode);

    match key.code {
        KeyCode::Up => {
            if state.selected_field > 0 {
                state.selected_field -= 1;
            } else {
                state.selected_field = popup_fields.len() - 1;
            }
            None
        }
        KeyCode::Down => {
            if state.selected_field < popup_fields.len() - 1 {
                state.selected_field += 1;
            } else {
                state.selected_field = 0;
            }
            None
        }
        _ => {
            let field = popup_fields[state.selected_field];
            match field {
                "Save" => {
                    if is_enter {
                        match state.to_channel_info(0) {
                            Ok(channel) => return Some(channel),
                            // Keep the popup open so the key material is not lost.
                            Err(message) => show_toast(toast, toast_timer, &message, false),
                        }
                    }
                    None
                }
                "Cancel" => None,
                "Name" => {
                    if is_enter {
                        state.start_editing_name();
                    }
                    None
                }
                "PSK" => {
                    if is_enter {
                        state.start_editing_psk();
                    }
                    None
                }
                "PSK Mode" => {
                    if cycle_forward || cycle_backward {
                        state.psk_mode = state.psk_mode.cycle(cycle_forward);
                        // The value belongs to the mode that was just left.
                        state.psk_value.clear();
                    }
                    None
                }
                "Uplink" | "Downlink" | "Position" | "Muted" => {
                    if cycle_forward || cycle_backward {
                        if field == "Uplink" {
                            state.uplink_enabled = !state.uplink_enabled;
                        } else if field == "Downlink" {
                            state.downlink_enabled = !state.downlink_enabled;
                        } else if field == "Position" {
                            let len = POSITION_OPTIONS.len();
                            if cycle_forward {
                                state.position_index = (state.position_index + 1) % len;
                            } else {
                                state.position_index = (state.position_index + len - 1) % len;
                            }
                        } else if field == "Muted" {
                            state.muted = !state.muted;
                        }
                    }
                    None
                }
                _ => None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::ActivePanel;
    use meshurl::models::MeshtasticConfig;
    use ratatui::crossterm::event::{KeyCode, KeyEvent};
    use ratatui::{Terminal, backend::TestBackend};

    /// Feeds a key to the encode handler with the channel panel focused.
    fn press(config: &mut MeshtasticConfig, list_state: &mut ListState, code: KeyCode) {
        let mut active_panel = ActivePanel::Channels;
        let mut encoded_url = None;
        let mut channel_popup = None;
        let mut lora_popup = None;
        let mut lora_scroll = 0;
        let mut lora_max_scroll = 0;
        let mut toast = None;
        let mut toast_timer = 0;

        let mut state = EncodeState {
            encode_config: config,
            encoded_url: &mut encoded_url,
            active_panel: &mut active_panel,
            encode_channels_state: list_state,
            channel_popup: &mut channel_popup,
            lora_popup: &mut lora_popup,
            lora_scroll: &mut lora_scroll,
            lora_max_scroll: &mut lora_max_scroll,
            toast: &mut toast,
            toast_timer: &mut toast_timer,
        };

        handle_encode_keys(KeyEvent::from(code), &mut state);
    }

    fn config_with_channels(count: usize) -> MeshtasticConfig {
        let mut config = MeshtasticConfig::new();
        for i in 0..count {
            let mut channel: ChannelInfo = "default".parse().expect("valid channel spec");
            channel.name = format!("ch{}", i);
            config.channels.push(channel);
        }
        reindex_channels(&mut config.channels);
        config
    }

    /// Drives the popup the way a user does: cycle to `mode`, then type `value`.
    fn popup_with_psk(mode: PskModeKind, value: &str) -> ChannelPopupState {
        let mut popup = ChannelPopupState::new();
        let mut toast = None;
        let mut toast_timer = 0;

        // Move onto the "PSK Mode" field and cycle until the mode is selected.
        popup.selected_field = 1;
        while popup.psk_mode != mode {
            handle_popup_keys(
                KeyEvent::from(KeyCode::Right),
                &mut popup,
                &mut toast,
                &mut toast_timer,
            );
        }

        if mode.needs_value() {
            let fields = get_popup_fields(popup.psk_mode);
            popup.selected_field = fields
                .iter()
                .position(|field| *field == "PSK")
                .expect("modes needing a value expose a PSK field");

            // Enter opens the input overlay, the textarea takes the text, Enter commits.
            handle_popup_keys(
                KeyEvent::from(KeyCode::Enter),
                &mut popup,
                &mut toast,
                &mut toast_timer,
            );
            popup.psk_textarea = TextArea::new(vec![value.to_string()]);
            handle_popup_keys(
                KeyEvent::from(KeyCode::Enter),
                &mut popup,
                &mut toast,
                &mut toast_timer,
            );
        }

        popup
    }

    fn save(popup: &ChannelPopupState) -> Result<ChannelInfo, String> {
        popup.to_channel_info(0).map(|(_, channel)| channel)
    }

    const VALID_PSK: &str = "CcZBoFJbADPGEoSkkYPA3Ha23rr7WPcyUo1AjorGQIA=";

    #[test]
    fn base64_psk_survives_saving() {
        let popup = popup_with_psk(PskModeKind::Base64, VALID_PSK);

        let channel = save(&popup).expect("valid PSK saves");

        assert_eq!(channel.psk, VALID_PSK);
        assert_eq!(channel.psk_type, PskType::Aes256);
    }

    #[test]
    fn passphrase_hashes_the_entered_text() {
        let popup = popup_with_psk(PskModeKind::Passphrase, "my secret phrase");

        let channel = save(&popup).expect("valid passphrase saves");

        assert_eq!(channel.psk, hash_phrase_to_psk("my secret phrase"));
        assert_eq!(channel.psk_type, PskType::Aes256);
        // Regression: the empty hash is a well-known public constant.
        assert_ne!(channel.psk, hash_phrase_to_psk(""));
    }

    #[test]
    fn an_invalid_base64_psk_is_refused_not_downgraded() {
        let mut popup = popup_with_psk(PskModeKind::Base64, VALID_PSK);
        // Bypass field validation the way a stale value would.
        popup.psk_value = "MTIzNDU2".to_string();

        let error = save(&popup).expect_err("a short PSK is refused");

        assert_eq!(error, "PSK must be 16 or 32 bytes");
    }

    #[test]
    fn an_empty_base64_psk_is_refused() {
        let popup = popup_with_psk(PskModeKind::Base64, "");

        let error = save(&popup).expect_err("an empty PSK is refused");

        assert_eq!(error, "Enter a base64 PSK first");
    }

    #[test]
    fn an_empty_passphrase_is_refused() {
        let popup = popup_with_psk(PskModeKind::Passphrase, "");

        let error = save(&popup).expect_err("an empty passphrase is refused");

        assert_eq!(error, "Enter a passphrase first");
    }

    #[test]
    fn a_rejected_psk_raises_a_toast_and_keeps_the_popup_open() {
        let mut popup = popup_with_psk(PskModeKind::Base64, "");
        let mut toast = None;
        let mut toast_timer = 0;

        let fields = get_popup_fields(popup.psk_mode);
        popup.selected_field = fields
            .iter()
            .position(|field| *field == "Save")
            .expect("Save is always offered");
        let saved = handle_popup_keys(
            KeyEvent::from(KeyCode::Enter),
            &mut popup,
            &mut toast,
            &mut toast_timer,
        );

        assert!(saved.is_none());
        let toast = toast.expect("the failure is reported");
        assert_eq!(toast.text, "Enter a base64 PSK first");
        assert!(!toast.is_success);
    }

    #[test]
    fn the_simple_modes_still_save() {
        let default = save(&popup_with_psk(PskModeKind::Default, "")).expect("saves");
        assert_eq!(default.psk, DEFAULT_PSK);
        assert_eq!(default.psk_type, PskType::Default);

        let none = save(&popup_with_psk(PskModeKind::None, "")).expect("saves");
        assert!(none.psk.is_empty());
        assert_eq!(none.psk_type, PskType::None);

        let random = save(&popup_with_psk(PskModeKind::Random, "")).expect("saves");
        assert_eq!(random.psk_type, PskType::Aes256);
    }

    #[test]
    fn editing_a_channel_keeps_its_psk() {
        let mut channel: ChannelInfo = "default".parse().expect("valid channel spec");
        channel.psk = VALID_PSK.to_string();
        channel.psk_type = PskType::Aes256;

        let popup = ChannelPopupState::from_channel(0, &channel);
        let saved = save(&popup).expect("an unchanged channel saves");

        assert_eq!(popup.psk_mode, PskModeKind::Base64);
        assert_eq!(saved.psk, VALID_PSK);
    }

    #[test]
    fn cycling_the_mode_clears_the_stale_value() {
        let mut popup = popup_with_psk(PskModeKind::Base64, VALID_PSK);
        assert_eq!(popup.psk_value, VALID_PSK);

        let mut toast = None;
        let mut toast_timer = 0;
        popup.selected_field = 1;
        handle_popup_keys(
            KeyEvent::from(KeyCode::Right),
            &mut popup,
            &mut toast,
            &mut toast_timer,
        );

        assert_eq!(popup.psk_mode, PskModeKind::Passphrase);
        assert!(popup.psk_value.is_empty());
    }

    #[test]
    fn a_disabled_position_is_left_out() {
        let popup = ChannelPopupState::new();
        assert_eq!(POSITION_OPTIONS[popup.position_index].1, 0);

        let (_, channel) = popup.to_channel_info(0).expect("saves");

        assert_eq!(channel.position_precision, None);
    }

    #[test]
    fn a_selected_position_is_kept() {
        let mut popup = ChannelPopupState::new();
        popup.position_index = POSITION_OPTIONS
            .iter()
            .position(|(_, precision)| *precision == 14)
            .expect("14 is one of the offered precisions");

        let (_, channel) = popup.to_channel_info(0).expect("saves");

        assert_eq!(channel.position_precision, Some(14));
    }

    #[test]
    fn a_disabled_position_adds_no_module_settings() {
        use meshtastic_protobufs::meshtastic::ChannelSettings;

        let popup = ChannelPopupState::new();
        let (_, channel) = popup.to_channel_info(0).expect("saves");

        let settings = ChannelSettings::from(&channel);

        assert!(settings.module_settings.is_none());
    }

    #[test]
    fn a_psk_entered_in_the_popup_reaches_the_url() {
        use meshurl::decoder::{DecodeResult, decode_url};

        let popup = popup_with_psk(PskModeKind::Base64, VALID_PSK);
        let mut config = MeshtasticConfig::new();
        let (_, channel) = popup.to_channel_info(0).expect("valid PSK saves");
        config.channels.push(channel);

        let url = encode_url(&config).expect("config encodes");
        let decoded = decode_url(&url).expect("the URL decodes");

        match decoded {
            DecodeResult::Channel(config) => {
                assert_eq!(config.channels[0].psk, VALID_PSK);
                assert_eq!(config.channels[0].psk_type, PskType::Aes256);
            }
            DecodeResult::Node(_) => panic!("expected a channel URL"),
        }
    }

    #[test]
    fn mode_cycling_wraps_in_both_directions() {
        assert_eq!(PskModeKind::Passphrase.cycle(true), PskModeKind::Default);
        assert_eq!(PskModeKind::Default.cycle(false), PskModeKind::Passphrase);
        for mode in PskModeKind::ALL {
            assert_eq!(mode.cycle(true).cycle(false), *mode);
        }
    }

    fn draw_popup(width: u16, height: u16, popup: &ChannelPopupState) {
        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).expect("test backend starts");
        terminal
            .draw(|f| draw_channel_popup(f, popup))
            .expect("popup renders");
    }

    #[test]
    fn channel_popup_renders_a_multi_byte_name() {
        let mut popup = ChannelPopupState::new();
        // Byte slicing at column 22 lands inside the last octopus and panics.
        popup.name = "Channel for tests \u{1f419}\u{1f419}\u{1f419}".to_string();

        draw_popup(80, 30, &popup);
    }

    #[test]
    fn channel_popup_renders_on_a_tiny_screen() {
        let popup = ChannelPopupState::new();

        draw_popup(3, 3, &popup);
        draw_popup(1, 1, &popup);
    }

    #[test]
    fn lora_popup_renders_on_a_tiny_screen() {
        let popup = LoRaPopupState::new();
        let mut terminal = Terminal::new(TestBackend::new(3, 3)).expect("test backend starts");

        terminal
            .draw(|f| draw_lora_popup(f, &popup, f.area()))
            .expect("popup renders");
    }

    #[test]
    fn move_up_on_empty_list_does_not_panic() {
        let mut config = MeshtasticConfig::new();
        let mut list_state = ListState::default();
        // Tab into the channel panel selects index 0 even with no channels.
        list_state.select(Some(0));

        press(&mut config, &mut list_state, KeyCode::Char('+'));

        assert!(config.channels.is_empty());
    }

    #[test]
    fn move_down_on_empty_list_does_not_panic() {
        let mut config = MeshtasticConfig::new();
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        press(&mut config, &mut list_state, KeyCode::Char('-'));

        assert!(config.channels.is_empty());
    }

    #[test]
    fn selection_stops_at_the_last_channel() {
        let mut config = config_with_channels(2);
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        for _ in 0..5 {
            press(&mut config, &mut list_state, KeyCode::Down);
        }

        assert_eq!(list_state.selected(), Some(1));
    }

    #[test]
    fn selection_stops_at_the_first_channel() {
        let mut config = config_with_channels(2);
        let mut list_state = ListState::default();
        list_state.select(Some(1));

        for _ in 0..5 {
            press(&mut config, &mut list_state, KeyCode::Up);
        }

        assert_eq!(list_state.selected(), Some(0));
    }

    #[test]
    fn reordering_past_the_end_does_not_panic() {
        let mut config = config_with_channels(2);
        let mut list_state = ListState::default();
        // A stale selection left over from a longer list.
        list_state.select(Some(5));

        press(&mut config, &mut list_state, KeyCode::Char('-'));

        assert_eq!(config.channels.len(), 2);
    }

    #[test]
    fn reordering_renumbers_channels() {
        let mut config = config_with_channels(3);
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        press(&mut config, &mut list_state, KeyCode::Char('+'));

        assert_eq!(list_state.selected(), Some(1));
        assert_eq!(config.channels[0].name, "ch1");
        assert_eq!(config.channels[1].name, "ch0");
        // Index and role follow the position, not the channel.
        for (i, channel) in config.channels.iter().enumerate() {
            assert_eq!(channel.index, i);
            let expected_role = if i == 0 {
                ChannelRole::Primary
            } else {
                ChannelRole::Secondary
            };
            assert_eq!(channel.role, expected_role);
        }
    }

    #[test]
    fn deleting_renumbers_remaining_channels() {
        let mut config = config_with_channels(3);
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        press(&mut config, &mut list_state, KeyCode::Char('d'));

        assert_eq!(config.channels.len(), 2);
        assert_eq!(config.channels[0].name, "ch1");
        assert_eq!(config.channels[0].index, 0);
        assert_eq!(config.channels[0].role, ChannelRole::Primary);
    }

    #[test]
    fn deleting_the_last_channel_clears_the_selection() {
        let mut config = config_with_channels(1);
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        press(&mut config, &mut list_state, KeyCode::Char('d'));

        assert!(config.channels.is_empty());
        assert_eq!(list_state.selected(), None);
    }
}
