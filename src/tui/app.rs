use meshurl::models::MeshtasticConfig;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, widgets::ListState, Frame, Terminal};
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

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    execute!(terminal.backend_mut(), EnterAlternateScreen)?;

    let mut app_mode = AppMode::Decode;
    let mut config_result: Option<Result<MeshtasticConfig, String>> = None;
    let mut encode_config = MeshtasticConfig::new();
    let mut encoded_url: Option<String> = None;
    let mut active_panel = ActivePanel::Url;
    let mut editing_url = false;
    let mut channels_scroll: usize = 0;
    let mut lora_scroll: u16 = 0;
    let mut lora_max_scroll: u16 = 0;
    let mut channels_list_state = ListState::default();
    let mut encode_channels_state = ListState::default();
    let mut textarea = TextArea::default();
    let mut channel_popup: Option<crate::tui::encode::ChannelPopupState> = None;
    let mut lora_popup: Option<crate::tui::encode::LoRaPopupState> = None;

    let result = run_inner(
        &mut terminal,
        &mut app_mode,
        &mut textarea,
        &mut config_result,
        &mut encode_config,
        &mut encoded_url,
        &mut active_panel,
        &mut editing_url,
        &mut channels_scroll,
        &mut lora_scroll,
        &mut lora_max_scroll,
        &mut channels_list_state,
        &mut encode_channels_state,
        &mut channel_popup,
        &mut lora_popup,
    );

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    terminal.show_cursor()?;

    result
}

fn run_inner(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app_mode: &mut AppMode,
    textarea: &mut TextArea,
    config_result: &mut Option<Result<MeshtasticConfig, String>>,
    encode_config: &mut MeshtasticConfig,
    encoded_url: &mut Option<String>,
    active_panel: &mut ActivePanel,
    editing_url: &mut bool,
    channels_scroll: &mut usize,
    lora_scroll: &mut u16,
    lora_max_scroll: &mut u16,
    mut channels_list_state: &mut ListState,
    mut encode_channels_state: &mut ListState,
    channel_popup: &mut Option<crate::tui::encode::ChannelPopupState>,
    lora_popup: &mut Option<crate::tui::encode::LoRaPopupState>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| {
            draw(
                f,
                *app_mode,
                textarea,
                config_result,
                encode_config,
                &encoded_url,
                *active_panel,
                *editing_url,
                *channels_scroll,
                *lora_scroll,
                lora_max_scroll,
                &mut channels_list_state,
                &mut encode_channels_state,
                &channel_popup,
                &lora_popup,
            );
        })?;

        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let is_editing_in_url = *active_panel == ActivePanel::Url && *editing_url;
                    let is_decode_mode = *app_mode == AppMode::Decode;

                    if is_decode_mode
                        && is_editing_in_url
                        && !matches!(key.code, KeyCode::Esc | KeyCode::Enter)
                    {
                        textarea.input(key);
                    } else {
                        let is_editing_channel_name = *app_mode == AppMode::Encode
                            && channel_popup.as_ref().map_or(false, |p| p.editing_name);

                        if is_editing_channel_name
                            && !matches!(key.code, KeyCode::Esc | KeyCode::Enter)
                        {
                            if let Some(popup) = channel_popup.as_mut() {
                                popup.name_textarea.input(key);
                            }
                        } else {
                            match key.code {
                                KeyCode::Char('1') => {
                                    *app_mode = AppMode::Decode;
                                    *active_panel = ActivePanel::Url;
                                }
                                KeyCode::Char('2') => {
                                    *app_mode = AppMode::Encode;
                                    *active_panel = ActivePanel::Channels;
                                }
                                KeyCode::Tab | KeyCode::BackTab => {
                                    if *app_mode == AppMode::Encode {
                                        crate::tui::encode::handle_encode_tab(
                                            key,
                                            active_panel,
                                            channels_list_state,
                                        );
                                    } else {
                                        crate::tui::decode::handle_decode_tab(
                                            key,
                                            active_panel,
                                            channels_list_state,
                                        );
                                    }
                                    *editing_url = false;
                                }
                                KeyCode::Esc => {
                                    if *active_panel == ActivePanel::Url && *editing_url {
                                        *editing_url = false;
                                    } else if *app_mode == AppMode::Encode
                                        && channel_popup.as_ref().map_or(false, |p| p.editing_name)
                                    {
                                        if let Some(popup) = channel_popup.as_mut() {
                                            popup.cancel_editing_name();
                                        }
                                    } else if *app_mode == AppMode::Encode
                                        && channel_popup.is_some()
                                    {
                                        *channel_popup = None;
                                    } else if *app_mode == AppMode::Encode && lora_popup.is_some() {
                                        *lora_popup = None;
                                    } else {
                                        return Ok(());
                                    }
                                }
                                _ => {
                                    if *app_mode == AppMode::Encode {
                                        crate::tui::encode::handle_encode_keys(
                                            key,
                                            encode_config,
                                            encoded_url,
                                            active_panel,
                                            encode_channels_state,
                                            channel_popup,
                                            lora_popup,
                                            lora_scroll,
                                            *lora_max_scroll,
                                        );
                                    } else {
                                        crate::tui::decode::handle_decode_keys(
                                            key,
                                            active_panel,
                                            textarea,
                                            config_result,
                                            editing_url,
                                            channels_scroll,
                                            lora_scroll,
                                            lora_max_scroll,
                                            channels_list_state,
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

fn draw(
    f: &mut Frame,
    app_mode: AppMode,
    textarea: &TextArea,
    config_result: &Option<Result<MeshtasticConfig, String>>,
    encode_config: &MeshtasticConfig,
    encoded_url: &Option<String>,
    active_panel: ActivePanel,
    editing_url: bool,
    channels_scroll: usize,
    lora_scroll: u16,
    lora_max_scroll: &mut u16,
    channels_list_state: &mut ListState,
    encode_channels_state: &mut ListState,
    channel_popup: &Option<crate::tui::encode::ChannelPopupState>,
    lora_popup: &Option<crate::tui::encode::LoRaPopupState>,
) {
    if app_mode == AppMode::Encode {
        crate::tui::encode::draw_encode_mode(
            f,
            encode_config,
            encoded_url,
            active_panel,
            encode_channels_state,
            lora_popup,
            lora_scroll,
            lora_max_scroll,
        );

        if let Some(popup_state) = channel_popup {
            crate::tui::encode::draw_channel_popup(f, popup_state);
        }
        return;
    }

    crate::tui::decode::draw_decode_mode(
        f,
        textarea,
        config_result,
        active_panel,
        editing_url,
        channels_scroll,
        channels_list_state,
        lora_scroll,
        lora_max_scroll,
    );
}
