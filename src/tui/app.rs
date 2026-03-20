use meshurl::models::MeshtasticConfig;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    style::{Color, Style},
    widgets::{Block, Borders, ListState, Padding, Paragraph},
    Frame, Terminal,
};
use ratatui_textarea::TextArea;
use std::io;

#[derive(Clone, Copy, PartialEq)]
pub enum AppMode {
    Decode,
    Encode,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ActivePanel {
    Url,
    Channels,
    Lora,
    UrlEncode,
}

#[derive(Clone)]
pub struct ToastMessage {
    pub text: String,
    pub is_success: bool,
    pub is_uncertain: bool,
}

pub struct AppState {
    pub app_mode: AppMode,
    pub textarea: TextArea<'static>,
    pub config_result: Option<Result<MeshtasticConfig, String>>,
    pub encode_config: MeshtasticConfig,
    pub encoded_url: Option<String>,
    pub active_panel: ActivePanel,
    pub editing_url: bool,
    pub channels_scroll: usize,
    pub lora_scroll: u16,
    pub lora_max_scroll: u16,
    pub channels_list_state: ListState,
    pub encode_channels_state: ListState,
    pub channel_popup: Option<crate::tui::encode::ChannelPopupState>,
    pub lora_popup: Option<crate::tui::encode::LoRaPopupState>,
    pub toast: Option<ToastMessage>,
    pub toast_timer: u8,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            app_mode: AppMode::Decode,
            textarea: TextArea::default(),
            config_result: None,
            encode_config: MeshtasticConfig::new(),
            encoded_url: None,
            active_panel: ActivePanel::Url,
            editing_url: false,
            channels_scroll: 0,
            lora_scroll: 0,
            lora_max_scroll: 0,
            channels_list_state: ListState::default(),
            encode_channels_state: ListState::default(),
            channel_popup: None,
            lora_popup: None,
            toast: None,
            toast_timer: 0,
        }
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    execute!(terminal.backend_mut(), EnterAlternateScreen)?;

    let mut state = AppState::default();

    let result = run_inner(&mut terminal, &mut state);

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    terminal.show_cursor()?;

    result
}

fn run_inner(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| {
            draw(f, state);
        })?;

        if state.toast_timer > 0 {
            state.toast_timer -= 1;
        }
        if state.toast_timer == 0 {
            state.toast = None;
        }

        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let is_editing_in_url =
                        state.active_panel == ActivePanel::Url && state.editing_url;
                    let is_decode_mode = state.app_mode == AppMode::Decode;

                    if is_decode_mode
                        && is_editing_in_url
                        && !matches!(key.code, KeyCode::Esc | KeyCode::Enter)
                    {
                        state.textarea.input(key);
                    } else {
                        let is_editing_channel_name = state.app_mode == AppMode::Encode
                            && state
                                .channel_popup
                                .as_ref()
                                .map_or(false, |p| p.editing_name);

                        let is_editing_channel_psk = state.app_mode == AppMode::Encode
                            && state
                                .channel_popup
                                .as_ref()
                                .map_or(false, |p| p.editing_psk);

                        if is_editing_channel_name
                            && !matches!(key.code, KeyCode::Esc | KeyCode::Enter)
                        {
                            if let Some(popup) = state.channel_popup.as_mut() {
                                popup.name_textarea.input(key);
                            }
                        } else if is_editing_channel_psk
                            && !matches!(key.code, KeyCode::Esc | KeyCode::Enter)
                        {
                            if let Some(popup) = state.channel_popup.as_mut() {
                                popup.psk_textarea.input(key);
                            }
                        } else {
                            match key.code {
                                KeyCode::Char('1') => {
                                    state.app_mode = AppMode::Decode;
                                    state.active_panel = ActivePanel::Url;
                                }
                                KeyCode::Char('2') => {
                                    state.app_mode = AppMode::Encode;
                                    state.active_panel = ActivePanel::Channels;
                                }
                                KeyCode::Char('m') | KeyCode::Char('M') => {
                                    if state.app_mode == AppMode::Decode {
                                        if let Some(Ok(config)) = &state.config_result.clone() {
                                            state.encode_config = config.clone();
                                            state.app_mode = AppMode::Encode;
                                            state.active_panel = ActivePanel::Channels;
                                            state.encode_channels_state.select(Some(0));
                                        }
                                    }
                                }
                                KeyCode::Tab | KeyCode::BackTab => {
                                    if state.app_mode == AppMode::Encode {
                                        crate::tui::encode::handle_encode_tab(
                                            key,
                                            &mut state.active_panel,
                                            &mut state.encode_channels_state,
                                        );
                                    } else {
                                        crate::tui::decode::handle_decode_tab(
                                            key,
                                            &mut state.active_panel,
                                            &mut state.channels_list_state,
                                        );
                                    }
                                    state.editing_url = false;
                                }
                                KeyCode::Esc => {
                                    if state.active_panel == ActivePanel::Url && state.editing_url {
                                        state.editing_url = false;
                                    } else if state.app_mode == AppMode::Encode
                                        && state
                                            .channel_popup
                                            .as_ref()
                                            .map_or(false, |p| p.editing_name)
                                    {
                                        if let Some(popup) = state.channel_popup.as_mut() {
                                            popup.cancel_editing_name();
                                        }
                                    } else if state.app_mode == AppMode::Encode
                                        && state
                                            .channel_popup
                                            .as_ref()
                                            .map_or(false, |p| p.editing_psk)
                                    {
                                        if let Some(popup) = state.channel_popup.as_mut() {
                                            popup.cancel_editing_psk();
                                        }
                                    } else if state.app_mode == AppMode::Encode
                                        && state.channel_popup.is_some()
                                    {
                                        state.channel_popup = None;
                                    } else if state.app_mode == AppMode::Encode
                                        && state.lora_popup.is_some()
                                    {
                                        state.lora_popup = None;
                                    } else {
                                        return Ok(());
                                    }
                                }
                                _ => {
                                    if state.app_mode == AppMode::Encode {
                                        crate::tui::encode::handle_encode_keys(
                                            key,
                                            &mut state.encode_config,
                                            &mut state.encoded_url,
                                            &mut state.active_panel,
                                            &mut state.encode_channels_state,
                                            &mut state.channel_popup,
                                            &mut state.lora_popup,
                                            &mut state.lora_scroll,
                                            state.lora_max_scroll,
                                            &mut state.toast,
                                            &mut state.toast_timer,
                                        );
                                    } else {
                                        crate::tui::decode::handle_decode_keys(
                                            key,
                                            &mut state.active_panel,
                                            &mut state.textarea,
                                            &mut state.config_result,
                                            &mut state.editing_url,
                                            &mut state.channels_scroll,
                                            &mut state.lora_scroll,
                                            &mut state.lora_max_scroll,
                                            &mut state.channels_list_state,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn draw(f: &mut Frame, state: &mut AppState) {
    if state.app_mode == AppMode::Encode {
        crate::tui::encode::draw_encode_mode(
            f,
            &state.encode_config,
            &state.encoded_url,
            state.active_panel,
            &mut state.encode_channels_state,
            &state.lora_popup,
            state.lora_scroll,
            &mut state.lora_max_scroll,
        );

        if let Some(popup_state) = &state.channel_popup {
            crate::tui::encode::draw_channel_popup(f, popup_state);
        }

        if let Some(toast) = &state.toast {
            let color = if toast.is_uncertain {
                Color::Yellow
            } else if toast.is_success {
                Color::Green
            } else {
                Color::Red
            };
            let area = f.area();
            let width = toast.text.len() as u16 + 4;
            let toast_area =
                ratatui::layout::Rect::new(area.width.saturating_sub(width) - 1, 1, width, 3);
            f.render_widget(ratatui::widgets::Clear, toast_area);
            let block = Block::default()
                .borders(Borders::ALL)
                .padding(Padding::new(1, 1, 0, 0))
                .border_style(Style::default().fg(color));
            let paragraph = Paragraph::new(toast.text.clone())
                .style(Style::default().fg(color))
                .block(block);
            f.render_widget(paragraph, toast_area);
        }
        return;
    }

    crate::tui::decode::draw_decode_mode(
        f,
        &state.textarea,
        &state.config_result,
        state.active_panel,
        state.editing_url,
        state.channels_scroll,
        &mut state.channels_list_state,
        state.lora_scroll,
        &mut state.lora_max_scroll,
    );

    if let Some(toast) = &state.toast {
        let color = if toast.is_uncertain {
            Color::Yellow
        } else if toast.is_success {
            Color::Green
        } else {
            Color::Red
        };
        let area = f.area();
        let width = toast.text.len() as u16 + 4;
        let toast_area =
            ratatui::layout::Rect::new(area.width.saturating_sub(width) - 1, 1, width, 3);
        f.render_widget(ratatui::widgets::Clear, toast_area);
        let block = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::new(1, 1, 0, 0))
            .border_style(Style::default().fg(color));
        let paragraph = Paragraph::new(toast.text.clone())
            .style(Style::default().fg(color))
            .block(block);
        f.render_widget(paragraph, toast_area);
    }
}
