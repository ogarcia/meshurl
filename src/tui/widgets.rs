use meshurl::models::{
    ChannelInfo, ChannelRole, LoRaInfo, MeshtasticDisplay, NodeInfo, POSITION_OPTIONS, PskType,
};
use std::time::{Duration, Instant};

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, ListItem, Padding, Paragraph},
};

/// How long a toast stays on screen.
pub const TOAST_DURATION: Duration = Duration::from_secs(2);

#[derive(Clone)]
pub struct ToastMessage {
    pub text: String,
    pub is_success: bool,
    pub is_uncertain: bool,
    /// When the toast stops being shown.
    ///
    /// Counting down draw calls instead, as this used to, tied the lifetime of
    /// the message to the redraw rate: the same toast lasted two seconds at 60
    /// frames per second and minutes at one frame per keystroke.
    pub expires_at: Instant,
}

impl ToastMessage {
    pub fn new(text: impl Into<String>, is_success: bool, is_uncertain: bool) -> Self {
        Self {
            text: text.into(),
            is_success,
            is_uncertain,
            expires_at: Instant::now() + TOAST_DURATION,
        }
    }

    pub fn has_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

/// Horizontal space the toast borders and padding take from its text.
const TOAST_CHROME_WIDTH: u16 = 4;
/// Columns left between the toast and the right edge of the screen.
const TOAST_RIGHT_MARGIN: u16 = 1;
/// Text plus a top and bottom border.
const TOAST_HEIGHT: u16 = 3;

pub fn render_toast(f: &mut Frame, toast: &ToastMessage) {
    let color = if toast.is_uncertain {
        Color::Yellow
    } else if toast.is_success {
        Color::Green
    } else {
        Color::Red
    };
    let area = f.area();
    let toast_area = match toast_area(area, &toast.text) {
        Some(rect) => rect,
        None => return,
    };
    f.render_widget(Clear, toast_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::new(1, 1, 0, 0))
        .border_style(Style::default().fg(color));
    let paragraph = Paragraph::new(toast.text.clone())
        .style(Style::default().fg(color))
        .block(block);
    f.render_widget(paragraph, toast_area);
}

/// Places the toast in the top-right corner, clamped to the screen.
///
/// Returns `None` when the screen is too small to hold it at all. The width is
/// measured in terminal columns rather than bytes, so a message carrying
/// multi-byte characters does not overflow the box it is drawn in.
fn toast_area(area: Rect, text: &str) -> Option<Rect> {
    if area.width <= TOAST_RIGHT_MARGIN || area.height == 0 {
        return None;
    }

    let text_width = Line::from(text).width() as u16;
    let available = area.width - TOAST_RIGHT_MARGIN;
    let width = text_width
        .saturating_add(TOAST_CHROME_WIDTH)
        .min(available)
        .max(1);
    let height = TOAST_HEIGHT.min(area.height);

    let x = area.x + available - width;
    // Sit one row below the top edge when there is room for it.
    let y = area.y + u16::from(area.height > height);

    Some(Rect::new(x, y, width, height))
}

/// Margin kept around a centered popup when the screen is big enough.
const POPUP_MARGIN: u16 = 4;

/// Centers a popup of at most `width` x `height` inside `area`.
///
/// The requested size is clamped to what the screen can actually hold, so a
/// small terminal shrinks the popup instead of overflowing the buffer.
/// Returns `None` when there is nothing to draw into.
pub fn centered_popup(area: Rect, width: u16, height: u16) -> Option<Rect> {
    if area.width == 0 || area.height == 0 {
        return None;
    }

    let width = width.min(area.width.saturating_sub(POPUP_MARGIN).max(1));
    let height = height.min(area.height.saturating_sub(POPUP_MARGIN).max(1));

    let x = area.x + (area.width - width) / 2;
    let y = area.y + (area.height - height) / 2;

    Some(Rect::new(x, y, width, height))
}

/// Width of `text` in terminal columns.
pub fn text_width(text: &str) -> usize {
    Line::from(text).width()
}

/// Lines the key hints are packed into, longest a footer may grow to.
///
/// Below about fifty columns even three lines are not enough, and a footer
/// taller than this would take the screen from the panels it describes.
pub const FOOTER_MAX_LINES: usize = 3;

/// Packs the key hints into lines no wider than `width`.
///
/// A single line is used whenever the hints fit on one, so a wide terminal
/// keeps the row the extra lines would have cost. Entries are never split, and
/// anything past [`FOOTER_MAX_LINES`] is dropped: it could not be read anyway,
/// so the hints are listed most useful first.
pub fn footer_lines(keys: &[&str], width: u16) -> Vec<String> {
    /// Columns between two hints.
    const GAP: usize = 2;

    let width = width as usize;
    let mut lines: Vec<String> = Vec::new();
    let mut used = 0;

    for key in keys {
        let key_width = text_width(key);

        match lines.last_mut() {
            // Keep it on this line while it fits. An entry wider than the
            // terminal goes on a line of its own and is clipped there.
            Some(line) if used + GAP + key_width <= width => {
                line.push_str("  ");
                line.push_str(key);
                used += GAP + key_width;
            }
            _ => {
                if lines.len() == FOOTER_MAX_LINES {
                    break;
                }
                lines.push((*key).to_string());
                used = key_width;
            }
        }
    }

    lines
}

/// Shortens `text` to `max_columns` terminal columns, marking the cut with an
/// ellipsis.
///
/// Slicing by byte offset panics whenever the cut lands inside a multi-byte
/// character, which channel names routinely contain.
pub fn truncate_to_columns(text: &str, max_columns: usize) -> String {
    if text_width(text) <= max_columns {
        return text.to_string();
    }

    // Reserve one column for the ellipsis itself.
    let budget = max_columns.saturating_sub(1);
    let mut truncated = String::new();
    let mut used = 0;

    for character in text.chars() {
        let width = text_width(character.encode_utf8(&mut [0; 4]));
        if used + width > budget {
            break;
        }
        truncated.push(character);
        used += width;
    }

    truncated.push('…');
    truncated
}

pub fn yes_no(value: bool) -> &'static str {
    if value { "Yes" } else { "No" }
}

pub fn channel_total_lines(channels: &[ChannelInfo]) -> usize {
    channels
        .iter()
        .map(|ch| {
            let base_lines = 4;
            let has_position = ch.position_precision.is_some_and(|p| p > 0);
            base_lines + if has_position { 1 } else { 0 }
        })
        .sum()
}

pub fn channel_scroll_indicator(
    total_lines: usize,
    block_height: u16,
    selected_idx: usize,
    has_scroll_state: bool,
    scroll_offset: usize,
) -> &'static str {
    let visible_lines = block_height.saturating_sub(4) as usize;

    if total_lines <= visible_lines {
        return "";
    }

    if has_scroll_state {
        if scroll_offset == 0 {
            " [ ↓ more ] "
        } else if scroll_offset >= total_lines.saturating_sub(visible_lines) {
            " [ ↑ more ] "
        } else {
            " [ ↕ more ] "
        }
    } else {
        if selected_idx == 0 {
            " [ ↓ more ] "
        } else {
            " [ ↑ more ] "
        }
    }
}

pub struct ScrollInfo {
    pub visible_lines: usize,
    pub max_scroll: u16,
    pub clamped_scroll: u16,
    pub indicator: &'static str,
}

pub fn lora_scroll_info(lora: &LoRaInfo, block_height: u16, scroll_offset: u16) -> ScrollInfo {
    let all_lines = lora_info_lines(lora);
    let total_lora_lines = all_lines.len();
    let visible_lora_lines = (block_height.saturating_sub(4)) as usize;
    let max_scroll = total_lora_lines.saturating_sub(visible_lora_lines) as u16;
    let clamped_scroll = scroll_offset.min(max_scroll);

    let indicator = if total_lora_lines > visible_lora_lines {
        if clamped_scroll == 0 {
            " [ ↓ more ] "
        } else if clamped_scroll >= max_scroll {
            " [ ↑ more ] "
        } else {
            " [ ↕ more ] "
        }
    } else {
        ""
    };

    ScrollInfo {
        visible_lines: visible_lora_lines,
        max_scroll,
        clamped_scroll,
        indicator,
    }
}

/// Renders the contents of a node info URL.
pub fn node_info_lines(node: &NodeInfo) -> Vec<Line<'_>> {
    let label = Style::default().fg(Color::DarkGray);
    let value = Style::default().fg(Color::White);

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Node Num: ", label),
            Span::styled(node.num.to_string(), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Name: ", label),
            Span::styled(node.long_name.as_str(), value),
        ]),
        Line::from(vec![
            Span::styled("Short: ", label),
            Span::styled(node.short_name.as_str(), value),
        ]),
        Line::from(vec![
            Span::styled("Model: ", label),
            Span::styled(node.hw_model.as_str(), Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("Role: ", label),
            Span::styled(node.role.to_string(), Style::default().fg(Color::Green)),
        ]),
    ];

    if let Some(public_key) = &node.public_key {
        lines.push(Line::from(vec![
            Span::styled("Public Key: ", label),
            Span::styled(public_key.as_str(), Style::default().fg(Color::Magenta)),
        ]));
    }

    if node.is_unmessagable {
        lines.push(Line::from(vec![
            Span::styled("Unmessagable: ", label),
            Span::styled("Yes", Style::default().fg(Color::Red)),
        ]));
    }

    if node.manually_verified {
        lines.push(Line::from(vec![
            Span::styled("Key Verified: ", label),
            Span::styled("Yes", Style::default().fg(Color::Green)),
        ]));
    }

    if node.should_ignore {
        lines.push(Line::from(vec![
            Span::styled("Shared to Ignore: ", label),
            Span::styled("Yes", Style::default().fg(Color::Red)),
        ]));
    }

    lines
}

pub fn lora_info_lines(lora: &LoRaInfo) -> Vec<Line<'_>> {
    let region_color = Color::Cyan;
    let preset_color = Color::Yellow;
    let value_color = Color::White;
    let enabled_color = Color::Green;
    let disabled_color = Color::Red;

    let (bandwidth, spread_factor, coding_rate) = lora.modem_parameters();

    let all_lines = vec![
        Line::from(vec![
            Span::styled("Region: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                lora.region.to_mesh_string(),
                Style::default().fg(region_color),
            ),
        ]),
        Line::from(vec![
            Span::styled("Modem Preset: ", Style::default().fg(Color::DarkGray)),
            // Says "Custom" for manual parameters, where there is no preset to
            // name and the protobuf default would read as LongFast.
            Span::styled(lora.modem.name(), Style::default().fg(preset_color)),
        ]),
        Line::from(vec![
            Span::styled("Use Preset: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                yes_no(lora.modem.uses_preset()),
                Style::default().fg(if lora.modem.uses_preset() {
                    enabled_color
                } else {
                    disabled_color
                }),
            ),
        ]),
        if lora.tx_enabled {
            let tx_power_str = if lora.tx_power == 0 {
                "0 (maximum safe power)".to_string()
            } else {
                format!("{} dBm", lora.tx_power)
            };
            Line::from(vec![
                Span::styled("TX Power: ", Style::default().fg(Color::DarkGray)),
                Span::styled(tx_power_str, Style::default().fg(value_color)),
            ])
        } else {
            Line::from(vec![
                Span::styled("TX Enabled: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    yes_no(lora.tx_enabled),
                    Style::default().fg(if lora.tx_enabled {
                        enabled_color
                    } else {
                        disabled_color
                    }),
                ),
            ])
        },
        Line::from(vec![
            Span::styled("Bandwidth: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} kHz", bandwidth),
                Style::default().fg(value_color),
            ),
        ]),
        Line::from(vec![
            Span::styled("Spread Factor: ", Style::default().fg(Color::DarkGray)),
            Span::styled(spread_factor.to_string(), Style::default().fg(value_color)),
        ]),
        Line::from(vec![
            Span::styled("Coding Rate: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("4/{}", coding_rate),
                Style::default().fg(value_color),
            ),
        ]),
        Line::from(vec![
            Span::styled("Hop Limit: ", Style::default().fg(Color::DarkGray)),
            Span::styled(lora.hop_limit.to_string(), Style::default().fg(value_color)),
        ]),
        Line::from(vec![
            Span::styled("Channel Num: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                lora.channel_num.to_string(),
                Style::default().fg(value_color),
            ),
        ]),
        Line::from(vec![
            Span::styled("Duty Cycle: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                yes_no(lora.override_duty_cycle),
                Style::default().fg(if lora.override_duty_cycle {
                    enabled_color
                } else {
                    disabled_color
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled("SX126x RX: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                yes_no(lora.sx126x_rx_boosted_gain),
                Style::default().fg(if lora.sx126x_rx_boosted_gain {
                    enabled_color
                } else {
                    disabled_color
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled("Frequency: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} MHz", lora.override_frequency),
                Style::default().fg(value_color),
            ),
        ]),
        Line::from(vec![
            Span::styled("Offset: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} kHz", lora.frequency_offset),
                Style::default().fg(value_color),
            ),
        ]),
        Line::from(vec![
            Span::styled("PA Fan Disabled: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                yes_no(lora.pa_fan_disabled),
                Style::default().fg(if lora.pa_fan_disabled {
                    enabled_color
                } else {
                    disabled_color
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled("Ignore MQTT: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                yes_no(lora.ignore_mqtt),
                Style::default().fg(if lora.ignore_mqtt {
                    enabled_color
                } else {
                    disabled_color
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled("OK to MQTT: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                yes_no(lora.config_ok_to_mqtt),
                Style::default().fg(if lora.config_ok_to_mqtt {
                    enabled_color
                } else {
                    disabled_color
                }),
            ),
        ]),
    ];

    all_lines
}

pub fn channel_list_item(index: usize, channel: &ChannelInfo) -> ListItem<'_> {
    let role_color = if channel.role == ChannelRole::Primary {
        Color::Green
    } else {
        Color::Blue
    };

    let name_val = if channel.name.is_empty() {
        if index == 0 { "(primary channel)" } else { "" }
    } else {
        &channel.name
    };

    let psk_display = if channel.psk.is_empty() {
        "(none)".to_string()
    } else {
        channel.psk.clone()
    };

    let psk_type_str = match channel.psk_type {
        PskType::None => "None",
        PskType::Default => "Default",
        PskType::Aes128 => "AES128",
        PskType::Aes256 => "AES256",
        _ => "Unknown",
    };

    let psk_type_color = match channel.psk_type {
        PskType::None => Color::DarkGray,
        PskType::Default => Color::Yellow,
        PskType::Aes128 => Color::Cyan,
        PskType::Aes256 => Color::Magenta,
        _ => Color::White,
    };

    let lock_icon = match channel.psk_type {
        PskType::None | PskType::Default => "🔓",
        _ => "🔒",
    };

    let uplink_color = if channel.uplink_enabled {
        Color::Green
    } else {
        Color::Red
    };
    let downlink_color = if channel.downlink_enabled {
        Color::Green
    } else {
        Color::Red
    };
    let uplink_text = if channel.uplink_enabled { "✓" } else { "✗" };
    let downlink_text = if channel.downlink_enabled {
        "✓"
    } else {
        "✗"
    };

    let role_str = if channel.role == ChannelRole::Primary {
        "PRIMARY"
    } else {
        "SECONDARY"
    };

    let muted_indicator = if channel.is_client_muted {
        Span::styled(" 🔇", Style::default().fg(Color::Yellow))
    } else {
        Span::raw("")
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                format!("Channel {} ", index),
                Style::default()
                    .fg(role_color)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled(format!("({})", role_str), Style::default().fg(role_color)),
            muted_indicator,
        ]),
        Line::from(format!("  Name:     {}", name_val)),
        Line::from(vec![
            Span::raw("  PSK:      "),
            Span::styled(psk_display, Style::default().fg(psk_type_color)),
            Span::raw(" ("),
            Span::styled(psk_type_str, Style::default().fg(psk_type_color)),
            Span::raw(") "),
            Span::raw(lock_icon),
        ]),
        Line::from(vec![
            Span::raw("  Uplink:   "),
            Span::styled(uplink_text, Style::default().fg(uplink_color)),
            Span::raw("    "),
            Span::raw("Downlink: "),
            Span::styled(downlink_text, Style::default().fg(downlink_color)),
        ]),
    ];

    if let Some(precision) = channel.position_precision.filter(|&p| p > 0) {
        let precision_text = POSITION_OPTIONS
            .iter()
            .find(|(_, v)| *v == precision)
            .map(|(name, _)| *name)
            .unwrap_or("Unknown");
        let extra = format!("Position: {}", precision_text);
        lines.push(Line::from(format!("  {}", extra)));
    }

    ListItem::new(lines).style(Style::default().fg(Color::White))
}

#[cfg(test)]
mod tests {
    use super::*;
    use meshurl::models::UserRole;
    use ratatui::{Terminal, backend::TestBackend};

    /// The hints of a busy encode screen, in the order the footer lists them.
    const FOOTER_KEYS: &[&str] = &[
        "[1] Decode",
        "[2] Encode",
        "[Tab/Shift+Tab] Switch",
        "[A] Add",
        "[Enter] Edit",
        "[D/Del] Delete",
        "[U] Undo",
        "[Q] Quit",
    ];

    /// A contact as a `/v/` URL hands it over, with the flags off.
    fn a_contact() -> NodeInfo {
        NodeInfo {
            num: 1,
            long_name: "Dom 6734".to_string(),
            short_name: "6734".to_string(),
            hw_model: "HELTEC_V3".to_string(),
            role: UserRole::Client,
            public_key: None,
            is_unmessagable: false,
            should_ignore: false,
            manually_verified: false,
        }
    }

    /// What the panel reads as, lines and all.
    fn panel_text(lines: &[Line<'_>]) -> String {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect()
            })
            .collect::<Vec<String>>()
            .join("\n")
    }

    #[test]
    fn the_node_panel_reports_the_contact_flags() {
        let mut node = a_contact();

        let plain = panel_text(&node_info_lines(&node));
        assert!(!plain.contains("Key Verified"), "{}", plain);
        assert!(!plain.contains("Ignore"), "{}", plain);

        node.manually_verified = true;
        node.should_ignore = true;

        let flagged = panel_text(&node_info_lines(&node));
        assert!(flagged.contains("Key Verified: Yes"), "{}", flagged);
        assert!(flagged.contains("Shared to Ignore: Yes"), "{}", flagged);
    }

    #[test]
    fn a_wide_terminal_keeps_the_footer_on_one_line() {
        let lines = footer_lines(FOOTER_KEYS, 200);

        assert_eq!(lines.len(), 1, "no row is spent on a second line");
        assert_eq!(lines[0], FOOTER_KEYS.join("  "));
    }

    #[test]
    fn a_narrow_terminal_gets_more_lines() {
        let lines = footer_lines(FOOTER_KEYS, 60);

        assert!(lines.len() > 1, "the hints did not fit on one line");
        for line in &lines {
            assert!(
                text_width(line) <= 60,
                "{:?} is wider than the terminal",
                line
            );
        }
    }

    #[test]
    fn no_hint_is_split_across_lines() {
        // Wrapping on whitespace would break "[Enter] Edit" in half.
        for width in 20..=200 {
            let lines = footer_lines(FOOTER_KEYS, width);
            let packed: Vec<&str> = lines
                .iter()
                .flat_map(|line| line.split("  "))
                .filter(|entry| !entry.is_empty())
                .collect();

            for entry in &packed {
                assert!(
                    FOOTER_KEYS.contains(entry),
                    "{:?} is not a whole hint, at width {}",
                    entry,
                    width
                );
            }
        }
    }

    #[test]
    fn every_hint_survives_a_terminal_wide_enough_for_them() {
        for width in 60..=200 {
            let listed = footer_lines(FOOTER_KEYS, width).join("  ");
            for key in FOOTER_KEYS {
                assert!(
                    listed.contains(key),
                    "{} went missing at width {}",
                    key,
                    width
                );
            }
        }
    }

    #[test]
    fn the_footer_stops_growing() {
        // Past this the hints cannot be read anyway, and a taller footer would
        // take the screen from the panels it describes.
        let lines = footer_lines(FOOTER_KEYS, 12);

        assert_eq!(lines.len(), FOOTER_MAX_LINES);
    }

    #[test]
    fn a_hint_wider_than_the_terminal_gets_its_own_line() {
        let lines = footer_lines(&["[Tab/Shift+Tab] Switch", "[Q] Quit"], 10);

        assert_eq!(lines, ["[Tab/Shift+Tab] Switch", "[Q] Quit"]);
    }

    #[test]
    fn the_hints_are_measured_in_columns_not_bytes() {
        // The arrows in decode mode are three bytes each and one column wide.
        let lines = footer_lines(&["[\u{2191}\u{2193}] Scroll", "[Q] Quit"], 22);

        assert_eq!(lines.len(), 1, "both fit in 22 columns");
    }

    fn toast(text: &str) -> ToastMessage {
        ToastMessage::new(text, true, false)
    }

    fn draw_toast(width: u16, height: u16, text: &str) {
        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).expect("test backend starts");
        terminal
            .draw(|f| render_toast(f, &toast(text)))
            .expect("toast renders");
    }

    /// The longest message the TUI can raise, on a screen too small for it.
    const LONG_TOAST: &str = "Seems copied (if not work install wl-clipboard or xclip)";

    #[test]
    fn a_toast_expires_after_its_duration() {
        let toast = toast("Copied to clipboard!");

        assert!(!toast.has_expired());
        assert!(toast.expires_at > Instant::now());
        assert!(toast.expires_at <= Instant::now() + TOAST_DURATION);
    }

    #[test]
    fn an_elapsed_toast_reports_itself_expired() {
        let mut toast = toast("Copied to clipboard!");
        // A lifetime measured in time, not in draw calls.
        toast.expires_at = Instant::now() - Duration::from_millis(1);

        assert!(toast.has_expired());
    }

    #[test]
    fn renders_on_a_narrow_screen() {
        draw_toast(20, 10, LONG_TOAST);
    }

    #[test]
    fn renders_on_a_tiny_screen() {
        draw_toast(1, 1, LONG_TOAST);
        draw_toast(2, 2, "ok");
    }

    #[test]
    fn renders_with_multi_byte_text() {
        // Bytes and columns disagree here: naive byte length overflows the box.
        draw_toast(40, 10, "Copiado 🐙🐙🐙");
    }

    #[test]
    fn area_stays_inside_the_screen() {
        let screen = Rect::new(0, 0, 20, 10);
        let area = toast_area(screen, LONG_TOAST).expect("fits after clamping");

        assert!(area.right() <= screen.right());
        assert!(area.bottom() <= screen.bottom());
    }

    #[test]
    fn area_hugs_the_right_edge() {
        let screen = Rect::new(0, 0, 80, 24);
        let area = toast_area(screen, "Copied to clipboard!").expect("fits");

        assert_eq!(area.right(), screen.right() - TOAST_RIGHT_MARGIN);
        assert_eq!(area.width, 20 + TOAST_CHROME_WIDTH);
        assert_eq!(area.y, 1);
    }

    #[test]
    fn area_is_none_when_there_is_no_room() {
        assert!(toast_area(Rect::new(0, 0, 1, 10), "x").is_none());
        assert!(toast_area(Rect::new(0, 0, 20, 0), "x").is_none());
    }

    #[test]
    fn popup_is_centered_and_leaves_a_margin() {
        let screen = Rect::new(0, 0, 80, 24);
        let popup = centered_popup(screen, 40, 10).expect("fits");

        assert_eq!(popup.width, 40);
        assert_eq!(popup.height, 10);
        assert_eq!(popup.x, 20);
        assert_eq!(popup.y, 7);
    }

    #[test]
    fn popup_shrinks_to_fit_a_small_screen() {
        let screen = Rect::new(0, 0, 10, 6);
        let popup = centered_popup(screen, 40, 18).expect("fits after clamping");

        assert!(popup.right() <= screen.right());
        assert!(popup.bottom() <= screen.bottom());
    }

    #[test]
    fn popup_survives_a_screen_smaller_than_the_margin() {
        let screen = Rect::new(0, 0, 2, 2);
        let popup = centered_popup(screen, 40, 18).expect("still yields a rect");

        assert!(popup.right() <= screen.right());
        assert!(popup.bottom() <= screen.bottom());
    }

    #[test]
    fn popup_is_none_on_an_empty_screen() {
        assert!(centered_popup(Rect::new(0, 0, 0, 10), 10, 3).is_none());
        assert!(centered_popup(Rect::new(0, 0, 10, 0), 10, 3).is_none());
    }

    #[test]
    fn truncation_leaves_short_text_alone() {
        assert_eq!(truncate_to_columns("short", 22), "short");
        assert_eq!(truncate_to_columns("", 22), "");
    }

    #[test]
    fn truncation_marks_the_cut() {
        let truncated = truncate_to_columns("0123456789", 5);

        assert_eq!(truncated, "0123\u{2026}");
        assert_eq!(text_width(&truncated), 5);
    }

    #[test]
    fn truncation_does_not_split_multi_byte_characters() {
        // Byte slicing at column 22 lands inside the last octopus and panics.
        let name = "Channel for tests \u{1f419}\u{1f419}\u{1f419}";
        let truncated = truncate_to_columns(name, 22);

        assert!(text_width(&truncated) <= 22);
        assert!(truncated.ends_with('\u{2026}'));
    }

    #[test]
    fn area_is_measured_in_columns_not_bytes() {
        let screen = Rect::new(0, 0, 80, 24);
        // Four bytes, two columns.
        let area = toast_area(screen, "🐙").expect("fits");

        assert_eq!(area.width, 2 + TOAST_CHROME_WIDTH);
    }
}
