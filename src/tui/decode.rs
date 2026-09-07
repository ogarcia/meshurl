use meshurl::decoder::{DecodeResult, decode_url};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Padding, Paragraph},
};
use ratatui_textarea::TextArea;

use crate::tui::app::{ActivePanel, DecodeDrawState, DecodeState};
use crate::tui::widgets::{
    channel_list_item, channel_scroll_indicator, channel_total_lines, footer_lines,
    lora_info_lines, lora_scroll_info, node_info_lines,
};

pub fn draw_decode_mode(f: &mut Frame, state: &mut DecodeDrawState) {
    // Packed before the layout: the lines they need decide the footer height,
    // so a wide terminal keeps the row a taller footer would have cost it.
    let footer = footer_lines(&decode_keys(state), f.area().width);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(12),
            Constraint::Min(6),
            Constraint::Length(footer.len() as u16),
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
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(title, chunks[0]);

    let border_color = if state.active_panel == ActivePanel::Url {
        if state.editing_url {
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

    if state.active_panel == ActivePanel::Url && state.editing_url {
        let mut text_area_edit = state.textarea.clone();
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
        let text = state.textarea.lines().first().map_or("", |l| l.as_str());
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

    // A node URL carries a device description, not channels, so the middle
    // panel changes what it is showing rather than refusing the URL.
    let decoded_node = match state.config_result {
        Some(Ok(DecodeResult::Node(node))) => Some(node),
        _ => None,
    };
    let decoded_config = match state.config_result {
        Some(Ok(DecodeResult::Channel(config))) => Some(config),
        _ => None,
    };

    let main_title = match state.config_result {
        Some(Ok(DecodeResult::Channel(config))) => {
            format!(" 📋 Channels ({} found) ", config.channels.len())
        }
        Some(Ok(DecodeResult::Node(_))) => " 📇 Node ".to_string(),
        Some(Err(_)) => " 📋 Channels (error) ".to_string(),
        None => " 📋 Channels ".to_string(),
    };
    let main_border_color = if state.active_panel == ActivePanel::Channels {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let main_block = || {
        Block::default()
            .title(main_title.clone())
            .borders(Borders::ALL)
            .padding(Padding::new(1, 1, 1, 1))
            .border_style(Style::default().fg(main_border_color))
    };

    if let Some(config) = decoded_config {
        let items: Vec<ListItem> = config
            .channels
            .iter()
            .enumerate()
            .map(|(i, ch)| channel_list_item(i, ch))
            .collect();

        let total_lines = channel_total_lines(&config.channels);
        let block_height = chunks[2].height;
        let selected_idx = state.channels_list_state.selected().unwrap_or(0);

        let scroll_indicator = channel_scroll_indicator(
            total_lines,
            block_height,
            selected_idx,
            true, // has scroll state in decode
            state.channels_scroll,
        );

        let channels_block =
            main_block().title_bottom(Line::from(scroll_indicator).right_aligned());

        let mut list = List::new(items).block(channels_block);
        if state.active_panel == ActivePanel::Channels {
            list = list.highlight_style(
                Style::default()
                    .bg(Color::Rgb(0x1a, 0x1a, 0x1a))
                    .add_modifier(ratatui::style::Modifier::BOLD),
            );
        }
        f.render_stateful_widget(list, chunks[2], state.channels_list_state);
    } else if let Some(node) = decoded_node {
        let node_para = Paragraph::new(node_info_lines(node)).block(main_block());
        f.render_widget(node_para, chunks[2]);
    } else {
        let (text, style) = match state.config_result {
            Some(Err(e)) => (format!("Error: {}", e), Style::default().fg(Color::Red)),
            _ => (
                "Enter a URL above and press Decode".to_string(),
                Style::default().fg(Color::DarkGray),
            ),
        };
        let para = Paragraph::new(text).style(style).block(main_block());
        f.render_widget(para, chunks[2]);
    }

    let lora_title = " 📻 LoRa Config ";
    let lora_border_color = if state.active_panel == ActivePanel::Lora {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let lora_block = || {
        Block::default()
            .title(lora_title)
            .borders(Borders::ALL)
            .padding(Padding::new(1, 1, 1, 1))
            .border_style(Style::default().fg(lora_border_color))
    };

    match decoded_config.and_then(|config| config.lora.as_ref()) {
        Some(lora) => {
            let scroll_info = lora_scroll_info(lora, chunks[3].height, state.lora_scroll);
            *state.lora_max_scroll = scroll_info.max_scroll;

            let all_lines = lora_info_lines(lora);
            let start_idx = scroll_info.clamped_scroll as usize;
            let end_idx = (start_idx + scroll_info.visible_lines).min(all_lines.len());
            let visible_lines: Vec<Line> = all_lines[start_idx..end_idx].to_vec();

            let block =
                lora_block().title_bottom(Line::from(scroll_info.indicator).right_aligned());
            f.render_widget(Paragraph::new(visible_lines).block(block), chunks[3]);
        }
        None => {
            let help = match state.config_result {
                Some(Ok(DecodeResult::Node(_))) => "Node URLs carry no LoRa config",
                Some(Ok(DecodeResult::Channel(_))) => "No LoRa config in URL",
                _ => "Decode a URL to see LoRa config",
            };
            let para = Paragraph::new(help)
                .style(Style::default().fg(Color::DarkGray))
                .block(lora_block());
            f.render_widget(para, chunks[3]);
        }
    }

    let footer = Paragraph::new(footer.join("\n")).style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[4]);
}

/// The key hints for decode mode, most useful first.
///
/// Only lists keys that do something on the focused panel: the footer used to
/// advertise [M] with nothing decoded, and cover a panel this mode never
/// focuses. The order matters, as a narrow terminal drops the tail.
fn decode_keys(state: &DecodeDrawState) -> Vec<&'static str> {
    let has_valid_config = matches!(state.config_result, Some(Ok(_)));

    let mut keys: Vec<&str> = vec!["[1] Decode", "[2] Encode"];

    match state.active_panel {
        ActivePanel::Url if state.editing_url => {
            keys.push("[Enter] Decode");
            keys.push("[Esc] Exit edit");
        }
        ActivePanel::Url => {
            keys.push("[Enter] Edit");
            if has_valid_config {
                keys.push("[M] Modify");
            }
            keys.push("[Shift+Del] Clear all");
            keys.push("[Q] Quit");
        }
        _ => {
            keys.push("[Tab/Shift+Tab] Switch");
            keys.push("[↑↓] Scroll");
            if has_valid_config {
                keys.push("[M] Modify");
            }
            keys.push("[Shift+Del] Clear all");
            keys.push("[Q] Quit");
        }
    }

    keys
}

pub fn handle_decode_keys(key: ratatui::crossterm::event::KeyEvent, state: &mut DecodeState) {
    use ratatui::crossterm::event::{KeyCode, KeyModifiers};

    match key.code {
        // Shift+Delete clears the screen, the same key encode mode uses for
        // it. Delete on its own used to do this, and it was read before the
        // URL box got a look at the key, so pressing it while typing a URL
        // threw the URL away instead of deleting a character.
        KeyCode::Delete if key.modifiers.contains(KeyModifiers::SHIFT) => {
            *state.textarea = TextArea::default();
            *state.config_result = None;
            *state.channels_scroll = 0;
            *state.lora_scroll = 0;
            *state.lora_max_scroll = 0;
            state.channels_list_state.select(None);
        }
        KeyCode::Char(_)
        | KeyCode::Backspace
        | KeyCode::Delete
        | KeyCode::Left
        | KeyCode::Right => {
            if *state.editing_url {
                state.textarea.input(key);
            }
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
        }
        KeyCode::Down => {
            if *state.active_panel == ActivePanel::Channels {
                if let Some(Ok(DecodeResult::Channel(cfg))) = state.config_result {
                    let max = cfg.channels.len().saturating_sub(1);
                    let current = state.channels_list_state.selected().unwrap_or(0);
                    if current < max {
                        state.channels_list_state.select(Some(current + 1));
                    }
                }
                *state.channels_scroll = state.channels_list_state.selected().unwrap_or(0);
            } else if *state.active_panel == ActivePanel::Lora {
                *state.lora_scroll = (*state.lora_scroll + 1).min(*state.lora_max_scroll);
            }
        }
        KeyCode::Enter => {
            if *state.editing_url {
                let text = state.textarea.lines().first().map_or("", |l| l.as_str());
                if !text.is_empty() {
                    match decode_url(text) {
                        Ok(decoded) => {
                            state.channels_list_state.select(Some(0));
                            *state.lora_scroll = 0;
                            *state.config_result = Some(Ok(decoded));
                        }
                        Err(e) => *state.config_result = Some(Err(e.to_string())),
                    }
                    *state.editing_url = false;
                }
            } else {
                *state.editing_url = true;
            }
        }
        _ => {}
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::DecodeState;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::widgets::ListState;
    use ratatui::{Terminal, backend::TestBackend};
    use ratatui_textarea::CursorMove;

    /// A real node URL, as shared by the Meshtastic app.
    const NODE_URL: &str =
        "https://meshtastic.org/v/#CAESJQoLIXRlc3QwMDAwMDESEEdhbGljaWEgQ2FsaWRhZGUaBPCfkJk";
    const CHANNEL_URL: &str = "https://meshtastic.org/e/#CgsSAQEoATABOgIIDQ";

    /// Types `url` into the URL field and decodes it, as a user would.
    fn decode(url: &str) -> Option<Result<DecodeResult, String>> {
        let mut active_panel = ActivePanel::Url;
        let mut textarea = TextArea::new(vec![url.to_string()]);
        let mut config_result = None;
        let mut editing_url = true;
        let mut channels_scroll = 0;
        let mut lora_scroll = 0;
        let mut lora_max_scroll = 0;
        let mut channels_list_state = ListState::default();

        let mut state = DecodeState {
            active_panel: &mut active_panel,
            textarea: &mut textarea,
            config_result: &mut config_result,
            editing_url: &mut editing_url,
            channels_scroll: &mut channels_scroll,
            lora_scroll: &mut lora_scroll,
            lora_max_scroll: &mut lora_max_scroll,
            channels_list_state: &mut channels_list_state,
        };
        handle_decode_keys(KeyEvent::from(KeyCode::Enter), &mut state);

        config_result
    }

    /// A decode-mode state that keeps what the keys leave behind.
    struct Decoder {
        textarea: TextArea<'static>,
        config_result: Option<Result<DecodeResult, String>>,
        editing_url: bool,
        channels_list_state: ListState,
    }

    impl Decoder {
        /// A decoded URL, sitting on the URL panel with editing finished.
        fn with(url: &str) -> Self {
            let mut decoder = Self {
                textarea: TextArea::new(vec![url.to_string()]),
                config_result: None,
                editing_url: true,
                channels_list_state: ListState::default(),
            };
            decoder.press(KeyCode::Enter);
            decoder
        }

        fn press(&mut self, code: KeyCode) {
            self.press_key(KeyEvent::from(code));
        }

        fn press_shift(&mut self, code: KeyCode) {
            self.press_key(KeyEvent::new(code, KeyModifiers::SHIFT));
        }

        fn press_key(&mut self, key: KeyEvent) {
            let mut active_panel = ActivePanel::Url;
            let mut channels_scroll = 0;
            let mut lora_scroll = 0;
            let mut lora_max_scroll = 0;

            let mut state = DecodeState {
                active_panel: &mut active_panel,
                textarea: &mut self.textarea,
                config_result: &mut self.config_result,
                editing_url: &mut self.editing_url,
                channels_scroll: &mut channels_scroll,
                lora_scroll: &mut lora_scroll,
                lora_max_scroll: &mut lora_max_scroll,
                channels_list_state: &mut self.channels_list_state,
            };
            handle_decode_keys(key, &mut state);
        }

        fn url(&self) -> String {
            self.textarea
                .lines()
                .first()
                .map_or(String::new(), |line| line.to_string())
        }
    }

    #[test]
    fn shift_delete_clears_the_screen() {
        let mut decoder = Decoder::with(CHANNEL_URL);
        assert!(decoder.config_result.is_some(), "it decoded first");

        decoder.press_shift(KeyCode::Delete);

        assert_eq!(decoder.url(), "");
        assert!(decoder.config_result.is_none());
    }

    #[test]
    fn delete_no_longer_throws_the_url_away() {
        // Delete was read before the URL box, so it cleared the screen instead
        // of deleting a character. Encode mode had the same trap.
        let mut decoder = Decoder::with(CHANNEL_URL);
        decoder.press(KeyCode::Enter);
        assert!(decoder.editing_url, "back in the URL box");

        decoder.press(KeyCode::Delete);

        assert!(!decoder.url().is_empty(), "the URL was not thrown away");
        assert!(decoder.config_result.is_some(), "nor the decoded result");
    }

    #[test]
    fn delete_edits_the_url_while_it_is_being_typed() {
        let mut decoder = Decoder::with(CHANNEL_URL);
        decoder.press(KeyCode::Enter);
        decoder.textarea.move_cursor(CursorMove::Head);

        decoder.press(KeyCode::Delete);

        assert_eq!(
            decoder.url(),
            &CHANNEL_URL[1..],
            "it took the character under the cursor"
        );
    }

    #[test]
    fn delete_does_nothing_outside_the_url_box() {
        let mut decoder = Decoder::with(CHANNEL_URL);

        decoder.press(KeyCode::Delete);

        assert_eq!(decoder.url(), CHANNEL_URL);
        assert!(decoder.config_result.is_some());
    }

    fn render(result: Option<Result<DecodeResult, String>>) -> String {
        render_at(result, 100, 40)
    }

    fn render_at(result: Option<Result<DecodeResult, String>>, width: u16, height: u16) -> String {
        let mut channels_list_state = ListState::default();
        let mut lora_max_scroll = 0;
        let textarea = TextArea::default();
        let mut draw_state = DecodeDrawState {
            textarea: &textarea,
            config_result: &result,
            active_panel: ActivePanel::Channels,
            editing_url: false,
            channels_scroll: 0,
            channels_list_state: &mut channels_list_state,
            lora_scroll: 0,
            lora_max_scroll: &mut lora_max_scroll,
        };

        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).expect("test backend starts");
        terminal
            .draw(|f| draw_decode_mode(f, &mut draw_state))
            .expect("decode mode renders");

        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn the_footer_takes_the_lines_its_hints_need() {
        // One line on a wide screen, so the row is not spent for nothing, and
        // as many as it takes on a narrow one rather than cutting hints off.
        for width in [120, 80, 60] {
            let screen = render_at(None, width, 40);
            for key in ["[Shift+Del] Clear all", "[Q] Quit"] {
                assert!(screen.contains(key), "{} is listed at width {}", key, width);
            }
        }
    }

    #[test]
    fn decode_mode_renders_on_a_tiny_screen() {
        for (width, height) in [(1, 1), (3, 3), (20, 6), (40, 12)] {
            render_at(None, width, height);
        }
    }

    #[test]
    fn a_node_url_decodes_instead_of_being_refused() {
        // This used to report "Node URLs are not supported in TUI decode mode".
        let result = decode(NODE_URL);

        match result {
            Some(Ok(DecodeResult::Node(node))) => {
                assert_eq!(node.long_name, "Galicia Calidade");
            }
            other => panic!("expected a node, got {:?}", other.map(|r| r.is_ok())),
        }
    }

    #[test]
    fn a_node_url_renders_its_details() {
        let rendered = render(decode(NODE_URL));

        assert!(rendered.contains("Node"), "the panel is titled Node");
        assert!(rendered.contains("Galicia Calidade"), "the name is shown");
        assert!(
            rendered.contains("Node URLs carry no LoRa config"),
            "the LoRa panel explains itself"
        );
    }

    #[test]
    fn a_channel_url_still_renders_its_channels() {
        let rendered = render(decode(CHANNEL_URL));

        assert!(rendered.contains("Channels (1 found)"));
    }

    #[test]
    fn an_invalid_url_still_reports_the_error() {
        let rendered = render(decode("https://meshtastic.org/e/#"));

        assert!(rendered.contains("Error:"));
    }
}
