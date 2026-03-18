use base64::Engine;
use meshurl::encoder::{encode_url, modem_preset_from_str, region_code_from_str};
use meshurl::models::{
    get_preset_params, ChannelInfo, ChannelRole, LoRaInfo, MeshtasticConfig, PskMode, PskType,
    POSITION_OPTIONS,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Padding, Paragraph},
    Frame,
};
use ratatui_textarea::{CursorMove, TextArea};
use std::io::Write;

use crate::tui::app::ActivePanel;
use crate::tui::widgets::{
    channel_list_item, channel_scroll_indicator, channel_total_lines, lora_info_lines,
    lora_scroll_info,
};

pub struct ChannelPopupState {
    pub channel_index: Option<usize>,
    pub name: String,
    pub psk_mode: PskMode,
    pub uplink_enabled: bool,
    pub downlink_enabled: bool,
    pub position_index: usize,
    pub muted: bool,
    pub selected_field: usize,
    pub editing_name: bool,
    pub name_textarea: TextArea<'static>,
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
        let region = format!("{:?}", lora.region);
        let modem_preset = format!("{:?}", lora.modem_preset);

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
        Self {
            channel_index: None,
            name: String::new(),
            psk_mode: PskMode::Default,
            uplink_enabled: false,
            downlink_enabled: false,
            position_index: 0,
            muted: false,
            selected_field: 0,
            editing_name: false,
            name_textarea,
        }
    }

    pub fn from_channel(index: usize, channel: &ChannelInfo) -> Self {
        let psk_mode = if channel.psk.is_empty() {
            PskMode::None
        } else if channel.psk == "AQ==" {
            PskMode::Default
        } else {
            PskMode::Random
        };

        let name_textarea = TextArea::default();
        let position_precision = channel.position_precision.unwrap_or(0);
        let position_index = POSITION_OPTIONS
            .iter()
            .position(|(_, v)| *v == position_precision)
            .unwrap_or(0);
        Self {
            channel_index: Some(index),
            name: channel.name.clone(),
            psk_mode,
            uplink_enabled: channel.uplink_enabled,
            downlink_enabled: channel.downlink_enabled,
            position_index,
            muted: channel.is_client_muted,
            selected_field: 0,
            editing_name: false,
            name_textarea,
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

    pub fn to_channel_info(&self, default_index: usize) -> (usize, ChannelInfo) {
        use base64::engine::general_purpose::STANDARD;
        use std::time::{SystemTime, UNIX_EPOCH};

        let index = self.channel_index.unwrap_or(default_index);

        let (psk, psk_type) = match self.psk_mode {
            PskMode::Default => ("AQ==".to_string(), PskType::Default),
            PskMode::None => (String::new(), PskType::None),
            PskMode::Random => {
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64;
                let mut bytes = vec![0u8; 32];
                let mut rng = seed;
                for byte in bytes.iter_mut() {
                    rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                    *byte = (rng >> 16) as u8;
                }
                let psk = STANDARD.encode(&bytes);
                (psk, PskType::Aes256)
            }
        };

        (
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
                position_precision: Some(POSITION_OPTIONS[self.position_index].1),
                is_client_muted: self.muted,
            },
        )
    }
}

const POPUP_FIELDS: &[&str] = &[
    "Name", "PSK Mode", "Uplink", "Downlink", "Position", "Muted", "Save", "Cancel",
];

pub fn draw_encode_mode(
    f: &mut Frame,
    encode_config: &MeshtasticConfig,
    encoded_url: &Option<String>,
    active_panel: ActivePanel,
    encode_channels_state: &mut ListState,
    lora_popup: &Option<LoRaPopupState>,
    lora_scroll: u16,
    lora_max_scroll: &mut u16,
) {
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
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(title, chunks[0]);

    let channels_title = format!(" 📋 Channels ({} found) ", encode_config.channels.len());

    let total_lines = channel_total_lines(&encode_config.channels);
    let block_height = chunks[1].height;
    let selected_idx = encode_channels_state.selected().unwrap_or(0);

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
        .border_style(if active_panel == ActivePanel::Channels {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    if encode_config.channels.is_empty() {
        let help = Paragraph::new("No channels. Press [A] to add a channel")
            .style(Style::default().fg(Color::DarkGray))
            .block(channels_block);
        f.render_widget(help, chunks[1]);
    } else {
        let items: Vec<ListItem> = encode_config
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
        f.render_stateful_widget(list, chunks[1], encode_channels_state);
    }

    let lora_title = " 📻 LoRa Config ";

    if let Some(lora) = &encode_config.lora {
        let scroll_info = lora_scroll_info(lora, chunks[2].height, lora_scroll);
        *lora_max_scroll = scroll_info.max_scroll;

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
            .border_style(if active_panel == ActivePanel::Lora {
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
            .border_style(if active_panel == ActivePanel::Lora {
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
    let url_text = encoded_url.as_deref().unwrap_or("(Press G to generate)");
    let url_style = if encoded_url.is_some() {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let url_para = Paragraph::new(url_text).style(url_style).block(
        Block::default()
            .title(url_title)
            .borders(Borders::ALL)
            .padding(Padding::new(1, 1, 0, 0))
            .border_style(if active_panel == ActivePanel::UrlEncode {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            }),
    );
    f.render_widget(url_para, chunks[3]);

    let copy_hint = if encoded_url.is_some() {
        "  [C] Copy"
    } else {
        ""
    };
    let footer_text = match active_panel {
        ActivePanel::Channels => format!("[1] Decode  [2] Encode  [Tab/Shift+Tab] Switch  [A] Add  [D] Delete  [E] LoRa  [G] Generate{}  [Del] Clear", copy_hint),
        ActivePanel::Lora => format!("[1] Decode  [2] Encode  [Tab/Shift+Tab] Switch  [A] Add  [E] LoRa  [G] Generate{}  [Del] Clear", copy_hint),
        ActivePanel::UrlEncode => format!("[1] Decode  [2] Encode  [Tab/Shift+Tab] Switch  [A] Add  [E] LoRa  [G] Generate{}  [Del] Clear", copy_hint),
        ActivePanel::Url => format!("[1] Decode  [2] Encode  [Tab/Shift+Tab] Switch  [A] Add  [E] LoRa  [G] Generate{}  [Del] Clear", copy_hint),
    };
    let footer = Paragraph::new(footer_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[4]);

    if let Some(lora_state) = lora_popup {
        draw_lora_popup(f, lora_state, f.area());
    }
}

pub fn handle_encode_keys(
    key: ratatui::crossterm::event::KeyEvent,
    encode_config: &mut MeshtasticConfig,
    encoded_url: &mut Option<String>,
    active_panel: &mut ActivePanel,
    encode_channels_state: &mut ListState,
    channel_popup: &mut Option<ChannelPopupState>,
    lora_popup: &mut Option<LoRaPopupState>,
    lora_scroll: &mut u16,
    lora_max_scroll: u16,
) -> bool {
    use ratatui::crossterm::event::KeyCode;

    if lora_popup.is_some() {
        let popup = lora_popup.as_mut().unwrap();

        let result = handle_lora_popup_keys(key, popup);

        match result {
            Some(lora_info) => {
                encode_config.lora = Some(lora_info);
                *lora_popup = None;
            }
            None => {
                if key.code == KeyCode::Esc {
                    *lora_popup = None;
                } else if key.code == KeyCode::Enter {
                    let field = LORA_FIELDS[popup.selected_field];
                    if field == "Cancel" {
                        *lora_popup = None;
                    }
                }
            }
        }
        return true;
    }

    if channel_popup.is_some() {
        let popup = channel_popup.as_mut().unwrap();

        let result = handle_popup_keys(key, popup);

        match result {
            Some((idx, mut channel)) => {
                if idx < encode_config.channels.len() && popup.channel_index.is_some() {
                    channel.index = idx;
                    channel.role = if idx == 0 {
                        ChannelRole::Primary
                    } else {
                        ChannelRole::Secondary
                    };
                    encode_config.channels[idx] = channel;
                    encode_channels_state.select(Some(idx));
                } else if encode_config.channels.len() < 8 {
                    channel.index = encode_config.channels.len();
                    channel.role = if channel.index == 0 {
                        ChannelRole::Primary
                    } else {
                        ChannelRole::Secondary
                    };
                    encode_config.channels.push(channel);
                    encode_channels_state.select(Some(encode_config.channels.len() - 1));
                }
                *channel_popup = None;
            }
            None => {
                if key.code == KeyCode::Esc {
                    *channel_popup = None;
                } else if key.code == KeyCode::Enter {
                    let field = POPUP_FIELDS[popup.selected_field];
                    if field == "Cancel" {
                        *channel_popup = None;
                    }
                }
            }
        }
        return true;
    }

    match key.code {
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if let Some(url) = encoded_url {
                let encoded = base64::engine::general_purpose::STANDARD.encode(url.as_bytes());
                let osc52 = format!("\x1b]52;c;{}\x07", encoded);
                std::io::stdout().write_all(osc52.as_bytes()).ok();
                std::io::stdout().flush().ok();
            }
            true
        }
        KeyCode::Delete => {
            encode_config.channels.clear();
            encode_config.lora = None;
            *encoded_url = None;
            encode_channels_state.select(None);
            *lora_scroll = 0;
            *lora_popup = None;
            *channel_popup = None;
            true
        }
        KeyCode::Char('a') | KeyCode::Char('A') => {
            if encode_config.channels.len() < 8 {
                *channel_popup = Some(ChannelPopupState::new());
            }
            true
        }
        KeyCode::Enter => {
            if let Some(selected) = encode_channels_state.selected() {
                if selected < encode_config.channels.len() {
                    let channel = &encode_config.channels[selected];
                    *channel_popup = Some(ChannelPopupState::from_channel(selected, channel));
                }
            }
            true
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if let Some(selected) = encode_channels_state.selected() {
                if selected < encode_config.channels.len() {
                    encode_config.channels.remove(selected);
                    for (i, channel) in encode_config.channels.iter_mut().enumerate() {
                        channel.index = i;
                        channel.role = if i == 0 {
                            ChannelRole::Primary
                        } else {
                            ChannelRole::Secondary
                        };
                    }
                    if encode_config.channels.is_empty() {
                        encode_channels_state.select(None);
                    } else if selected >= encode_config.channels.len() {
                        encode_channels_state.select(Some(encode_config.channels.len() - 1));
                    }
                }
            }
            false
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            *lora_popup = Some(match &encode_config.lora {
                Some(lora) => LoRaPopupState::from_lora(lora),
                None => LoRaPopupState::new(),
            });
            true
        }
        KeyCode::Char('g') | KeyCode::Char('G') => {
            if !encode_config.channels.is_empty() {
                match encode_url(encode_config) {
                    Ok(url) => *encoded_url = Some(url),
                    Err(e) => *encoded_url = Some(format!("Error: {}", e)),
                }
            }
            true
        }
        KeyCode::Up => {
            if *active_panel == ActivePanel::Channels {
                if let Some(selected) = encode_channels_state.selected() {
                    if selected > 0 {
                        encode_channels_state.select(Some(selected - 1));
                    }
                } else {
                    encode_channels_state.select(Some(0));
                }
            } else if *active_panel == ActivePanel::Lora {
                *lora_scroll = lora_scroll.saturating_sub(1);
            }
            false
        }
        KeyCode::Down => {
            if *active_panel == ActivePanel::Channels {
                if let Some(selected) = encode_channels_state.selected() {
                    encode_channels_state.select(Some(selected + 1));
                } else {
                    encode_channels_state.select(Some(0));
                }
            } else if *active_panel == ActivePanel::Lora {
                *lora_scroll = (*lora_scroll + 1).min(lora_max_scroll);
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
    let width = 35.min(area.width - 4);
    let height = (POPUP_FIELDS.len() as u16 + 2).min(area.height - 4);
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;

    let popup_rect = ratatui::layout::Rect::new(x, y, width, height);

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

    for (i, field) in POPUP_FIELDS.iter().enumerate() {
        let row_y = inner_rect.y + i as u16;
        if row_y >= inner_rect.y + inner_rect.height {
            break;
        }

        let is_selected = i == state.selected_field;

        let value = match *field {
            "Name" => {
                if state.name.is_empty() {
                    String::new()
                } else {
                    let max_len = 22;
                    if state.name.len() > max_len {
                        format!("{}…", &state.name[..max_len])
                    } else {
                        state.name.clone()
                    }
                }
            }
            "PSK Mode" => match state.psk_mode {
                PskMode::Default => "Default".to_string(),
                PskMode::None => "None".to_string(),
                PskMode::Random => "Random".to_string(),
            },
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

    if state.editing_name {
        let overlay_width = 40.min(area.width - 4);
        let overlay_height = 3;
        let overlay_x = (area.width - overlay_width) / 2;
        let overlay_y = (area.height - overlay_height) / 2;

        let overlay_rect =
            ratatui::layout::Rect::new(overlay_x, overlay_y, overlay_width, overlay_height);

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
}

pub fn draw_lora_popup(f: &mut Frame, state: &LoRaPopupState, area: ratatui::layout::Rect) {
    let width = 40.min(area.width - 4);
    let height = (LORA_FIELDS.len() + 2) as u16;
    let height = height.min(area.height - 4);
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;

    let popup_rect = ratatui::layout::Rect::new(x, y, width, height);

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
                    let idx = LORA_REGIONS
                        .iter()
                        .position(|r| r == &state.region)
                        .unwrap_or(0);
                    let len = LORA_REGIONS.len();
                    let new_idx = ((idx as isize) + dir + len as isize) as usize % len;
                    state.region = LORA_REGIONS[new_idx].to_string();
                    None
                }
                "Modem Preset" => {
                    let idx = LORA_MODEM_PRESETS
                        .iter()
                        .position(|p| p == &state.modem_preset)
                        .unwrap_or(0);
                    let len = LORA_MODEM_PRESETS.len();
                    let new_idx = ((idx as isize) + dir + len as isize) as usize % len;
                    state.modem_preset = LORA_MODEM_PRESETS[new_idx].to_string();
                    None
                }
                "TX Power" => {
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
                    None
                }
                "Hop Limit" => {
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
                    None
                }
                "Channel" => {
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
) -> Option<(usize, ChannelInfo)> {
    use ratatui::crossterm::event::KeyCode;

    if state.editing_name {
        if matches!(key.code, KeyCode::Enter) {
            state.finish_editing_name();
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

    match key.code {
        KeyCode::Up => {
            if state.selected_field > 0 {
                state.selected_field -= 1;
            } else {
                state.selected_field = POPUP_FIELDS.len() - 1;
            }
            None
        }
        KeyCode::Down => {
            if state.selected_field < POPUP_FIELDS.len() - 1 {
                state.selected_field += 1;
            } else {
                state.selected_field = 0;
            }
            None
        }
        _ => {
            let field = POPUP_FIELDS[state.selected_field];
            match field {
                "Save" => {
                    if is_enter {
                        return Some(state.to_channel_info(0));
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
                "PSK Mode" => {
                    if cycle_forward || cycle_backward {
                        state.psk_mode = match state.psk_mode {
                            PskMode::Default => PskMode::None,
                            PskMode::None => PskMode::Random,
                            PskMode::Random => PskMode::Default,
                        };
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
