use meshurl::models::{ChannelInfo, ChannelRole, LoRaInfo, PskType, POSITION_OPTIONS};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::ListItem,
};

pub fn yes_no(value: bool) -> &'static str {
    if value {
        "Yes"
    } else {
        "No"
    }
}

pub fn channel_total_lines(channels: &[ChannelInfo]) -> usize {
    channels
        .iter()
        .map(|ch| {
            let base_lines = 4;
            let has_position =
                ch.position_precision.is_some() && ch.position_precision.unwrap() > 0;
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

pub fn lora_info_lines(lora: &LoRaInfo) -> Vec<Line<'_>> {
    let region_color = Color::Cyan;
    let preset_color = Color::Yellow;
    let value_color = Color::White;
    let enabled_color = Color::Green;
    let disabled_color = Color::Red;

    let all_lines = vec![
        Line::from(vec![
            Span::styled("Region: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:?}", lora.region),
                Style::default().fg(region_color),
            ),
        ]),
        Line::from(vec![
            Span::styled("Modem Preset: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:?}", lora.modem_preset),
                Style::default().fg(preset_color),
            ),
        ]),
        Line::from(vec![
            Span::styled("Use Preset: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                yes_no(lora.use_preset),
                Style::default().fg(if lora.use_preset {
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
                format!("{} kHz", lora.bandwidth),
                Style::default().fg(value_color),
            ),
        ]),
        Line::from(vec![
            Span::styled("Spread Factor: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                lora.spread_factor.to_string(),
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
        if index == 0 {
            "(primary channel)"
        } else {
            ""
        }
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

    if channel.position_precision.is_some() && channel.position_precision.unwrap() > 0 {
        let precision = channel.position_precision.unwrap();
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
