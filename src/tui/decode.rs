use meshurl::decoder::decode_url;
use meshurl::models::MeshtasticConfig;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph},
    Frame,
};
use ratatui_textarea::TextArea;

use crate::tui::app::{ActivePanel, DecodeState};
use crate::tui::widgets::{
    channel_list_item, channel_scroll_indicator, channel_total_lines, lora_info_lines,
    lora_scroll_info,
};

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
                .map(|(i, ch)| channel_list_item(i, ch))
                .collect();

            let total_lines = channel_total_lines(&config.channels);
            let block_height = chunks[2].height;
            let selected_idx = channels_list_state.selected().unwrap_or(0);

            let scroll_indicator = channel_scroll_indicator(
                total_lines,
                block_height,
                selected_idx,
                true, // has scroll state in decode
                channels_scroll,
            );

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
                let scroll_info = lora_scroll_info(lora, chunks[3].height, lora_scroll);
                *lora_max_scroll = scroll_info.max_scroll;

                let all_lines = lora_info_lines(lora);
                let start_idx = scroll_info.clamped_scroll as usize;
                let end_idx = (start_idx + scroll_info.visible_lines).min(all_lines.len());
                let visible_lines: Vec<Line> =
                    all_lines[start_idx..end_idx].iter().cloned().collect();

                let lora_border_color = if active_panel == ActivePanel::Lora {
                    Color::Yellow
                } else {
                    Color::DarkGray
                };

                let lora_block = Block::default()
                    .title(lora_title)
                    .title_bottom(Line::from(scroll_info.indicator).right_aligned())
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

    let has_valid_config = matches!(config_result, Some(Ok(_)));

    let footer_text = match active_panel {
        ActivePanel::Url => {
            if editing_url {
                "[1] Decode  [2] Encode  [Enter] Decode  [Esc] Exit edit"
            } else if has_valid_config {
                "[1] Decode  [2] Encode  [M] Modify  [Enter] Edit  [Del] Clear  [Esc] Quit"
            } else {
                "[1] Decode  [2] Encode  [Enter] Edit  [Del] Clear  [Esc] Quit"
            }
        }
        ActivePanel::Channels => {
            if has_valid_config {
                "[1] Decode  [2] Encode  [M] Modify  [Tab/Shift+Tab] Switch  [↑↓] Scroll  [Del] Clear  [Esc] Quit"
            } else {
                "[1] Decode  [2] Encode  [Tab/Shift+Tab] Switch  [↑↓] Scroll  [Del] Clear  [Esc] Quit"
            }
        }
        ActivePanel::Lora => {
            if has_valid_config {
                "[1] Decode  [2] Encode  [M] Modify  [Tab/Shift+Tab] Switch  [↑↓] Scroll  [Del] Clear  [Esc] Quit"
            } else {
                "[1] Decode  [2] Encode  [Tab/Shift+Tab] Switch  [↑↓] Scroll  [Del] Clear  [Esc] Quit"
            }
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
    state: &mut DecodeState,
) -> bool {
    use ratatui::crossterm::event::KeyCode;

    match key.code {
        KeyCode::Delete => {
            *state.textarea = TextArea::default();
            *state.config_result = None;
            *state.channels_scroll = 0;
            *state.lora_scroll = 0;
            *state.lora_max_scroll = 0;
            state.channels_list_state.select(None);
            true
        }
        KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Left | KeyCode::Right => {
            if *state.editing_url {
                state.textarea.input(key);
            }
            false
        }
        KeyCode::Up => {
            if *state.active_panel == ActivePanel::Channels {
                if state.channels_list_state.selected().unwrap_or(0) > 0 {
                    state
                        .channels_list_state
                        .select(Some(state.channels_list_state.selected().unwrap() - 1));
                } else {
                    state.channels_list_state.select(Some(0));
                }
                *state.channels_scroll = state.channels_list_state.selected().unwrap_or(0);
            } else if *state.active_panel == ActivePanel::Lora {
                *state.lora_scroll = state
                    .lora_scroll
                    .saturating_sub(1)
                    .min(*state.lora_max_scroll);
            }
            false
        }
        KeyCode::Down => {
            if *state.active_panel == ActivePanel::Channels {
                if let Some(config) = state.config_result {
                    if let Ok(cfg) = config {
                        let max = cfg.channels.len().saturating_sub(1);
                        let current = state.channels_list_state.selected().unwrap_or(0);
                        if current < max {
                            state.channels_list_state.select(Some(current + 1));
                        }
                    }
                }
                *state.channels_scroll = state.channels_list_state.selected().unwrap_or(0);
            } else if *state.active_panel == ActivePanel::Lora {
                *state.lora_scroll = (*state.lora_scroll + 1).min(*state.lora_max_scroll);
            }
            false
        }
        KeyCode::Enter => {
            if *state.editing_url {
                let text = state.textarea.lines().first().map_or("", |l| l.as_str());
                if !text.is_empty() {
                    let result = decode_url(text).map_err(|e| e.to_string());
                    *state.config_result = Some(result.clone());
                    if result.is_ok() {
                        state.channels_list_state.select(Some(0));
                        *state.lora_scroll = 0;
                        *state.editing_url = false;
                    }
                }
            } else {
                *state.editing_url = true;
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
