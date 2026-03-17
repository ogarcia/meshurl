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

use crate::tui::app::ActivePanel;
use crate::tui::widgets::{channel_list_item, lora_info_lines};

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
                let all_lines = lora_info_lines(lora);

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
