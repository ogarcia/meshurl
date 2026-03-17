use meshurl::decoder::decode_url;
use meshurl::models::{ChannelRole, MeshtasticConfig, PskType, POSITION_OPTIONS};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph},
    Frame,
};
use ratatui_textarea::TextArea;

use crate::tui::app::ActivePanel;

pub fn draw_decode_mode(
    f: &mut Frame,
    textarea: &TextArea,
    config_result: &Option<Result<MeshtasticConfig, String>>,
    active_panel: ActivePanel,
    editing_url: bool,
    channels_scroll: usize,
    channels_list_state: &mut ListState,
    lora_scroll: u16,
    lora_max_scroll: &mut u16,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(12),
            Constraint::Min(6),
            Constraint::Length(1),
        ])
        .split(f.area());

    let title = Paragraph::new(" 📡 MeshURL - Decode ")
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

    let border_color = if active_panel == ActivePanel::Url {
        if editing_url {
            Color::Green
        } else {
            Color::Yellow
        }
    } else {
        Color::DarkGray
    };

    let input_block = Block::default()
        .title(" 🔗 URL ")
        .borders(Borders::ALL)
        .padding(Padding::new(1, 1, 0, 0))
        .border_style(Style::default().fg(border_color));

    if active_panel == ActivePanel::Url && editing_url {
        let mut text_area_edit = textarea.clone();
        text_area_edit.set_cursor_line_style(Style::default());
        text_area_edit.set_block(
            Block::default()
                .title(" 🔗 URL [edit] ")
                .borders(Borders::ALL)
                .padding(Padding::new(1, 1, 0, 0))
                .border_style(Style::default().fg(Color::Green)),
        );
        f.render_widget(&text_area_edit, chunks[1]);
    } else {
        let text = textarea.lines().first().map_or("", |l| l.as_str());
        let placeholder = "Paste URL here...";
        let display_text = if text.is_empty() { placeholder } else { text };
        let text_style = if text.is_empty() {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };

        let url_para = Paragraph::new(display_text)
            .style(text_style)
            .block(input_block);
        f.render_widget(url_para, chunks[1]);
    }

    let channels_title = match config_result {
        Some(Ok(config)) => format!(" 📋 Channels ({} found) ", config.channels.len()),
        Some(Err(_)) => " 📋 Channels (error) ".to_string(),
        None => " 📋 Channels ".to_string(),
    };
    let channels_border_color = if active_panel == ActivePanel::Channels {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    match config_result {
        Some(Ok(config)) => {
            let items: Vec<ListItem> = config
                .channels
                .iter()
                .enumerate()
                .map(|(i, ch)| {
                    let role_color = if ch.role == ChannelRole::Primary {
                        Color::Green
                    } else {
                        Color::Blue
                    };

                    let name_val = if ch.name.is_empty() {
                        if i == 0 {
                            "(primary channel)"
                        } else {
                            ""
                        }
                    } else {
                        &ch.name
                    };
                    let psk_display = if ch.psk.is_empty() {
                        "(none)".to_string()
                    } else {
                        ch.psk.clone()
                    };
                    let psk_type_str = match ch.psk_type {
                        PskType::None => "None",
                        PskType::Default => "Default",
                        PskType::Aes128 => "AES128",
                        PskType::Aes256 => "AES256",
                        _ => "Unknown",
                    };

                    let psk_type_color = match ch.psk_type {
                        PskType::None => Color::DarkGray,
                        PskType::Default => Color::Yellow,
                        PskType::Aes128 => Color::Cyan,
                        PskType::Aes256 => Color::Magenta,
                        _ => Color::White,
                    };

                    let lock_icon = match ch.psk_type {
                        PskType::None | PskType::Default => "🔓",
                        _ => "🔒",
                    };

                    let uplink_color = if ch.uplink_enabled {
                        Color::Green
                    } else {
                        Color::Red
                    };
                    let downlink_color = if ch.downlink_enabled {
                        Color::Green
                    } else {
                        Color::Red
                    };
                    let uplink_text = if ch.uplink_enabled { "✓" } else { "✗" };
                    let downlink_text = if ch.downlink_enabled { "✓" } else { "✗" };

                    let role_str = if ch.role == ChannelRole::Primary {
                        "PRIMARY"
                    } else {
                        "SECONDARY"
                    };

                    let muted_indicator = if ch.is_client_muted {
                        Span::styled(" 🔇", Style::default().fg(Color::Yellow))
                    } else {
                        Span::raw("")
                    };

                    let mut lines = vec![
                        Line::from(vec![
                            Span::styled(
                                format!("Channel {} ", i),
                                Style::default()
                                    .fg(role_color)
                                    .add_modifier(ratatui::style::Modifier::BOLD),
                            ),
                            Span::styled(
                                format!("({})", role_str),
                                Style::default().fg(role_color),
                            ),
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

                    if ch.position_precision.is_some() && ch.position_precision.unwrap() > 0 {
                        let precision = ch.position_precision.unwrap();
                        let precision_text = POSITION_OPTIONS
                            .iter()
                            .find(|(_, v)| *v == precision)
                            .map(|(name, _)| *name)
                            .unwrap_or("Unknown");
                        let extra = format!("Position: {}", precision_text);
                        lines.push(Line::from(format!("  {}", extra)));
                    }

                    ListItem::new(lines).style(Style::default().fg(Color::White))
                })
                .collect();

            let visible_lines = (chunks[2].height.saturating_sub(4)) as usize;
            let total_items = config.channels.len();
            let total_lines = total_items * 4;

            let scroll_indicator = if total_lines > visible_lines {
                if channels_scroll == 0 {
                    " [ ↓ more ] "
                } else if channels_scroll >= total_items - 1 {
                    " [ ↑ more ] "
                } else {
                    " [ ↕ more ] "
                }
            } else {
                ""
            };

            let channels_block = Block::default()
                .title(channels_title)
                .title_bottom(Line::from(scroll_indicator).right_aligned())
                .borders(Borders::ALL)
                .padding(Padding::new(1, 1, 1, 1))
                .border_style(Style::default().fg(channels_border_color));

            let list = List::new(items).block(channels_block.clone());
            if active_panel == ActivePanel::Channels {
                let list = list.highlight_style(
                    Style::default()
                        .bg(Color::Rgb(0x1a, 0x1a, 0x1a))
                        .add_modifier(ratatui::style::Modifier::BOLD),
                );
                f.render_stateful_widget(list, chunks[2], channels_list_state);
            } else {
                f.render_stateful_widget(list, chunks[2], channels_list_state);
            }
        }

        Some(Err(e)) => {
            let channels_block = Block::default()
                .title(channels_title)
                .borders(Borders::ALL)
                .padding(Padding::new(1, 1, 1, 1))
                .border_style(Style::default().fg(channels_border_color));
            let error = Paragraph::new(format!("Error: {}", e))
                .style(Style::default().fg(Color::Red))
                .block(channels_block);
            f.render_widget(error, chunks[2]);
        }
        None => {
            let channels_block = Block::default()
                .title(channels_title)
                .borders(Borders::ALL)
                .padding(Padding::new(1, 1, 1, 1))
                .border_style(Style::default().fg(channels_border_color));
            let help = Paragraph::new("Enter a URL above and press Decode")
                .style(Style::default().fg(Color::DarkGray))
                .block(channels_block);
            f.render_widget(help, chunks[2]);
        }
    }

    let lora_title = " 📻 LoRa Config ";

    match config_result {
        Some(Ok(config)) => {
            if let Some(lora) = &config.lora {
                let region_color = Color::Cyan;
                let preset_color = Color::Yellow;
                let value_color = Color::White;

                let enabled_color = Color::Green;
                let disabled_color = Color::Red;

                fn yes_no(value: bool) -> &'static str {
                    if value {
                        "Yes"
                    } else {
                        "No"
                    }
                }

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

                let total_lora_lines = all_lines.len();
                let visible_lora_lines = (chunks[3].height.saturating_sub(4)) as usize;
                *lora_max_scroll = total_lora_lines.saturating_sub(visible_lora_lines) as u16;
                let clamped_scroll = (lora_scroll as usize).min(*lora_max_scroll as usize) as u16;
                let start_idx = clamped_scroll as usize;
                let end_idx = (start_idx + visible_lora_lines).min(total_lora_lines);
                let visible_lines: Vec<Line> =
                    all_lines[start_idx..end_idx].iter().cloned().collect();

                let lora_scroll_indicator = if total_lora_lines > visible_lora_lines {
                    if clamped_scroll == 0 {
                        " [ ↓ more ] "
                    } else if clamped_scroll >= *lora_max_scroll {
                        " [ ↑ more ] "
                    } else {
                        " [ ↕ more ] "
                    }
                } else {
                    ""
                };

                let lora_border_color = if active_panel == ActivePanel::Lora {
                    Color::Yellow
                } else {
                    Color::DarkGray
                };

                let lora_block = Block::default()
                    .title(lora_title)
                    .title_bottom(Line::from(lora_scroll_indicator).right_aligned())
                    .borders(Borders::ALL)
                    .padding(Padding::new(1, 1, 1, 1))
                    .border_style(Style::default().fg(lora_border_color));

                let lora_para = Paragraph::new(visible_lines).block(lora_block);
                f.render_widget(lora_para, chunks[3]);
            } else {
                let lora_border_color = if active_panel == ActivePanel::Lora {
                    Color::Yellow
                } else {
                    Color::DarkGray
                };
                let lora_block = Block::default()
                    .title(lora_title)
                    .borders(Borders::ALL)
                    .padding(Padding::new(1, 1, 1, 1))
                    .border_style(Style::default().fg(lora_border_color));
                let help = Paragraph::new("No LoRa config in URL")
                    .style(Style::default().fg(Color::DarkGray))
                    .block(lora_block);
                f.render_widget(help, chunks[3]);
            }
        }
        _ => {
            let lora_border_color = if active_panel == ActivePanel::Lora {
                Color::Yellow
            } else {
                Color::DarkGray
            };
            let lora_block = Block::default()
                .title(lora_title)
                .borders(Borders::ALL)
                .padding(Padding::new(1, 1, 1, 1))
                .border_style(Style::default().fg(lora_border_color));
            let help = Paragraph::new("Decode a URL to see LoRa config")
                .style(Style::default().fg(Color::DarkGray))
                .block(lora_block);
            f.render_widget(help, chunks[3]);
        }
    }

    let footer_text = match active_panel {
        ActivePanel::Url => {
            if editing_url {
                "[1] Decode  [2] Encode  [Enter] Decode  [Esc] Exit edit"
            } else {
                "[1] Decode  [2] Encode  [Enter] Edit  [Del] Clear  [Esc] Quit"
            }
        }
        ActivePanel::Channels => {
            "[1] Decode  [2] Encode  [Tab/Shift+Tab] Switch  [↑↓] Scroll  [Del] Clear  [Esc] Quit"
        }
        ActivePanel::Lora => {
            "[1] Decode  [2] Encode  [Tab/Shift+Tab] Switch  [↑↓] Scroll  [Del] Clear  [Esc] Quit"
        }
        ActivePanel::UrlEncode => {
            "[1] Decode  [2] Encode  [Tab/Shift+Tab] Switch  [G] Generate  [C] Copy  [Del] Clear"
        }
    };

    let footer = Paragraph::new(footer_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[4]);
}

pub fn handle_decode_keys(
    key: ratatui::crossterm::event::KeyEvent,
    active_panel: &ActivePanel,
    textarea: &mut TextArea,
    config_result: &mut Option<Result<MeshtasticConfig, String>>,
    editing_url: &mut bool,
    channels_scroll: &mut usize,
    lora_scroll: &mut u16,
    lora_max_scroll: &mut u16,
    channels_list_state: &mut ListState,
) -> bool {
    use ratatui::crossterm::event::KeyCode;

    match key.code {
        KeyCode::Delete => {
            *textarea = TextArea::default();
            *config_result = None;
            *channels_scroll = 0;
            *lora_scroll = 0;
            *lora_max_scroll = 0;
            channels_list_state.select(None);
            true
        }
        KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Left | KeyCode::Right => {
            if *editing_url {
                textarea.input(key);
            }
            false
        }
        KeyCode::Up => {
            if *active_panel == ActivePanel::Channels {
                if channels_list_state.selected().unwrap_or(0) > 0 {
                    channels_list_state.select(Some(channels_list_state.selected().unwrap() - 1));
                } else {
                    channels_list_state.select(Some(0));
                }
                *channels_scroll = channels_list_state.selected().unwrap_or(0);
            } else if *active_panel == ActivePanel::Lora {
                *lora_scroll = lora_scroll.saturating_sub(1).min(*lora_max_scroll);
            }
            false
        }
        KeyCode::Down => {
            if *active_panel == ActivePanel::Channels {
                if let Some(config) = config_result {
                    if let Ok(cfg) = config {
                        let max = cfg.channels.len().saturating_sub(1);
                        let current = channels_list_state.selected().unwrap_or(0);
                        if current < max {
                            channels_list_state.select(Some(current + 1));
                        }
                    }
                }
                *channels_scroll = channels_list_state.selected().unwrap_or(0);
            } else if *active_panel == ActivePanel::Lora {
                *lora_scroll = (*lora_scroll + 1).min(*lora_max_scroll);
            }
            false
        }
        KeyCode::Enter => {
            if *editing_url {
                let text = textarea.lines().first().map_or("", |l| l.as_str());
                if !text.is_empty() {
                    let result = decode_url(text).map_err(|e| e.to_string());
                    *config_result = Some(result.clone());
                    if result.is_ok() {
                        channels_list_state.select(Some(0));
                        *lora_scroll = 0;
                        *editing_url = false;
                    }
                }
            } else {
                *editing_url = true;
            }
            false
        }
        _ => false,
    }
}

pub fn handle_decode_tab(
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
            ActivePanel::Url => ActivePanel::Lora,
            ActivePanel::Channels => ActivePanel::Url,
            ActivePanel::Lora => ActivePanel::Channels,
            ActivePanel::UrlEncode => ActivePanel::Lora,
        }
    } else {
        match *active_panel {
            ActivePanel::Url => ActivePanel::Channels,
            ActivePanel::Channels => ActivePanel::Lora,
            ActivePanel::Lora => ActivePanel::Url,
            ActivePanel::UrlEncode => ActivePanel::Channels,
        }
    };

    if new_panel == ActivePanel::Channels && channels_list_state.selected().is_none() {
        channels_list_state.select(Some(0));
    }
    *active_panel = new_panel;
}
