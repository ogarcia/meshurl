use meshurl::decoder::DecodeResult;
use meshurl::models::MeshtasticConfig;
use ratatui::crossterm::cursor::Show;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Frame, Terminal, backend::CrosstermBackend, widgets::ListState};
use ratatui_textarea::TextArea;
use std::io;
use std::ops::ControlFlow;
use std::time::{Duration, Instant};

/// How long to wait for an event when nothing is pending.
///
/// Only affects how promptly the loop notices an external change; user input
/// wakes the poll immediately.
const IDLE_POLL: Duration = Duration::from_secs(1);

pub use crate::tui::widgets::{ToastMessage, render_toast};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Decode,
    Encode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActivePanel {
    Url,
    Channels,
    Lora,
    UrlEncode,
}

pub struct AppState {
    pub app_mode: AppMode,
    pub textarea: TextArea<'static>,
    pub config_result: Option<Result<DecodeResult, String>>,
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
    /// What the last destructive key took away, so it can be put back.
    pub undo: Option<crate::tui::encode::EncodeUndo>,
}

pub struct DecodeState<'a> {
    pub active_panel: &'a mut ActivePanel,
    pub textarea: &'a mut TextArea<'static>,
    pub config_result: &'a mut Option<Result<DecodeResult, String>>,
    pub editing_url: &'a mut bool,
    pub channels_scroll: &'a mut usize,
    pub lora_scroll: &'a mut u16,
    pub lora_max_scroll: &'a mut u16,
    pub channels_list_state: &'a mut ListState,
}

pub struct EncodeState<'a> {
    pub encode_config: &'a mut MeshtasticConfig,
    pub encoded_url: &'a mut Option<String>,
    pub active_panel: &'a mut ActivePanel,
    pub encode_channels_state: &'a mut ListState,
    pub channel_popup: &'a mut Option<crate::tui::encode::ChannelPopupState>,
    pub lora_popup: &'a mut Option<crate::tui::encode::LoRaPopupState>,
    pub lora_scroll: &'a mut u16,
    pub lora_max_scroll: &'a mut u16,
    pub toast: &'a mut Option<ToastMessage>,
    pub undo: &'a mut Option<crate::tui::encode::EncodeUndo>,
}

pub struct DecodeDrawState<'a> {
    pub textarea: &'a TextArea<'static>,
    pub config_result: &'a Option<Result<DecodeResult, String>>,
    pub active_panel: ActivePanel,
    pub editing_url: bool,
    pub channels_scroll: usize,
    pub channels_list_state: &'a mut ListState,
    pub lora_scroll: u16,
    pub lora_max_scroll: &'a mut u16,
}

pub struct EncodeDrawState<'a> {
    pub encode_config: &'a MeshtasticConfig,
    pub encoded_url: &'a Option<String>,
    pub active_panel: ActivePanel,
    pub encode_channels_state: &'a mut ListState,
    pub lora_popup: &'a Option<crate::tui::encode::LoRaPopupState>,
    pub lora_scroll: u16,
    pub lora_max_scroll: &'a mut u16,
    pub can_undo: bool,
}

impl AppState {
    /// Whether a popup is currently claiming the keyboard.
    fn has_popup(&self) -> bool {
        self.app_mode == AppMode::Encode
            && (self.channel_popup.is_some() || self.lora_popup.is_some())
    }
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
            undo: None,
        }
    }
}

/// Keys that work everywhere, including while a popup is open.
///
/// Esc is handled separately: a popup takes it to close itself.
fn is_global_key(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
        && key.modifiers.contains(KeyModifiers::CONTROL)
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    install_panic_hook();

    enable_raw_mode()?;

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    execute!(terminal.backend_mut(), EnterAlternateScreen)?;

    let mut state = AppState::default();

    let result = run_inner(&mut terminal, &mut state);

    restore_terminal();

    result
}

/// Restores the terminal to its original state.
///
/// Errors are deliberately ignored: this runs both on the normal exit path and
/// from the panic hook, where there is nothing useful left to do with them. Every
/// step is attempted even if an earlier one fails, so a single failure cannot
/// leave the terminal in raw mode or on the alternate screen.
fn restore_terminal() {
    let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
    let _ = disable_raw_mode();
}

/// Installs a panic hook that restores the terminal before reporting the panic.
///
/// Without it a panic inside the TUI leaves the user on the alternate screen with
/// raw mode still enabled, hiding both the panic message and their own shell.
fn install_panic_hook() {
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        previous_hook(info);
    }));
}

fn run_inner(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut needs_redraw = true;

    loop {
        if needs_redraw {
            terminal.draw(|f| {
                draw(f, state);
            })?;
            needs_redraw = false;
        }

        // Wake up in time to clear the toast, and otherwise only when something
        // happens. Redrawing on a fixed 60 Hz tick, as this used to, kept the
        // process busy for a screen that changes only on a keystroke.
        let timeout = state
            .toast
            .as_ref()
            .map(|toast| toast.expires_at.saturating_duration_since(Instant::now()))
            .unwrap_or(IDLE_POLL);

        if event::poll(timeout)? {
            let event = event::read()?;
            // Anything that reaches us changes the screen, a resize included.
            needs_redraw = true;

            if let Event::Key(key) = event
                && key.kind == KeyEventKind::Press
                && handle_key(state, key).is_break()
            {
                return Ok(());
            }
        }

        if state.toast.as_ref().is_some_and(ToastMessage::has_expired) {
            state.toast = None;
            needs_redraw = true;
        }
    }
}

/// Applies one key press to the application state.
///
/// Returns `ControlFlow::Break` when the key asks to quit. Kept out of the
/// event loop so the whole keyboard behaviour can be exercised without a
/// terminal.
fn handle_key(state: &mut AppState, key: KeyEvent) -> ControlFlow<()> {
    let is_editing_in_url = state.active_panel == ActivePanel::Url && state.editing_url;
    let is_decode_mode = state.app_mode == AppMode::Decode;

    if is_decode_mode && is_editing_in_url && !matches!(key.code, KeyCode::Esc | KeyCode::Enter) {
        state.textarea.input(key);
    } else {
        let is_editing_channel_name = state.app_mode == AppMode::Encode
            && state.channel_popup.as_ref().is_some_and(|p| p.editing_name);

        let is_editing_channel_psk = state.app_mode == AppMode::Encode
            && state.channel_popup.as_ref().is_some_and(|p| p.editing_psk);

        if is_editing_channel_name && !matches!(key.code, KeyCode::Esc | KeyCode::Enter) {
            if let Some(popup) = state.channel_popup.as_mut() {
                popup.name_textarea.input(key);
            }
        } else if is_editing_channel_psk && !matches!(key.code, KeyCode::Esc | KeyCode::Enter) {
            if let Some(popup) = state.channel_popup.as_mut() {
                popup.psk_textarea.input(key);
            }
        } else if state.has_popup() && !is_global_key(key) {
            // A popup owns the keyboard: switching mode or
            // quitting from under it used to leave the popup
            // open in the state, waiting on the other screen.
            let mut encode_state = EncodeState {
                encode_config: &mut state.encode_config,
                encoded_url: &mut state.encoded_url,
                active_panel: &mut state.active_panel,
                encode_channels_state: &mut state.encode_channels_state,
                channel_popup: &mut state.channel_popup,
                lora_popup: &mut state.lora_popup,
                lora_scroll: &mut state.lora_scroll,
                lora_max_scroll: &mut state.lora_max_scroll,
                toast: &mut state.toast,
                undo: &mut state.undo,
            };
            crate::tui::encode::handle_encode_keys(key, &mut encode_state);
        } else {
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => return ControlFlow::Break(()),
                KeyCode::Char('c') | KeyCode::Char('C')
                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    return ControlFlow::Break(());
                }
                KeyCode::Char('1') => {
                    state.app_mode = AppMode::Decode;
                    state.active_panel = ActivePanel::Url;
                }
                KeyCode::Char('2') => {
                    state.app_mode = AppMode::Encode;
                    state.active_panel = ActivePanel::Channels;
                }
                // Only a channel configuration can be carried into encode mode;
                // node URLs describe a device, not channels.
                KeyCode::Char('m') | KeyCode::Char('M') => match state.config_result.as_ref() {
                    Some(Ok(DecodeResult::Channel(config)))
                        if state.app_mode == AppMode::Decode =>
                    {
                        state.encode_config = config.clone();
                        state.app_mode = AppMode::Encode;
                        state.active_panel = ActivePanel::Channels;
                        state.encode_channels_state.select(Some(0));
                    }
                    _ => {}
                },
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
                // An open popup never reaches here: `has_popup` sends Esc to
                // the encode handler, which closes one layer at a time. Two
                // places acting on the same key is what let Esc in the channel
                // name box close the whole popup.
                KeyCode::Esc => {
                    if state.active_panel == ActivePanel::Url && state.editing_url {
                        state.editing_url = false;
                    } else {
                        return ControlFlow::Break(());
                    }
                }
                _ => {
                    if state.app_mode == AppMode::Encode {
                        let mut encode_state = EncodeState {
                            encode_config: &mut state.encode_config,
                            encoded_url: &mut state.encoded_url,
                            active_panel: &mut state.active_panel,
                            encode_channels_state: &mut state.encode_channels_state,
                            channel_popup: &mut state.channel_popup,
                            lora_popup: &mut state.lora_popup,
                            lora_scroll: &mut state.lora_scroll,
                            lora_max_scroll: &mut state.lora_max_scroll,
                            toast: &mut state.toast,
                            undo: &mut state.undo,
                        };
                        crate::tui::encode::handle_encode_keys(key, &mut encode_state);
                    } else {
                        let mut decode_state = DecodeState {
                            active_panel: &mut state.active_panel,
                            textarea: &mut state.textarea,
                            config_result: &mut state.config_result,
                            editing_url: &mut state.editing_url,
                            channels_scroll: &mut state.channels_scroll,
                            lora_scroll: &mut state.lora_scroll,
                            lora_max_scroll: &mut state.lora_max_scroll,
                            channels_list_state: &mut state.channels_list_state,
                        };
                        crate::tui::decode::handle_decode_keys(key, &mut decode_state);
                    }
                }
            }
        }
    }

    ControlFlow::Continue(())
}

fn draw(f: &mut Frame, state: &mut AppState) {
    if state.app_mode == AppMode::Encode {
        let mut encode_draw_state = EncodeDrawState {
            encode_config: &state.encode_config,
            encoded_url: &state.encoded_url,
            active_panel: state.active_panel,
            encode_channels_state: &mut state.encode_channels_state,
            lora_popup: &state.lora_popup,
            lora_scroll: state.lora_scroll,
            lora_max_scroll: &mut state.lora_max_scroll,
            can_undo: state.undo.is_some(),
        };
        crate::tui::encode::draw_encode_mode(f, &mut encode_draw_state);

        if let Some(popup_state) = &state.channel_popup {
            crate::tui::encode::draw_channel_popup(f, popup_state);
        }

        if let Some(toast) = &state.toast {
            render_toast(f, toast);
        }
        return;
    }

    let mut decode_draw_state = DecodeDrawState {
        textarea: &state.textarea,
        config_result: &state.config_result,
        active_panel: state.active_panel,
        editing_url: state.editing_url,
        channels_scroll: state.channels_scroll,
        channels_list_state: &mut state.channels_list_state,
        lora_scroll: state.lora_scroll,
        lora_max_scroll: &mut state.lora_max_scroll,
    };
    crate::tui::decode::draw_decode_mode(f, &mut decode_draw_state);

    if let Some(toast) = &state.toast {
        render_toast(f, toast);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::encode::{ChannelPopupState, LoRaPopupState};

    fn press(state: &mut AppState, code: KeyCode) -> ControlFlow<()> {
        handle_key(state, KeyEvent::from(code))
    }

    fn press_with(state: &mut AppState, code: KeyCode, modifiers: KeyModifiers) -> ControlFlow<()> {
        handle_key(state, KeyEvent::new(code, modifiers))
    }

    /// Encode mode with the channel editing popup open.
    fn with_channel_popup() -> AppState {
        AppState {
            app_mode: AppMode::Encode,
            channel_popup: Some(ChannelPopupState::new()),
            ..Default::default()
        }
    }

    /// Encode mode with the LoRa popup open.
    fn with_lora_popup() -> AppState {
        AppState {
            app_mode: AppMode::Encode,
            lora_popup: Some(LoRaPopupState::new()),
            ..Default::default()
        }
    }

    #[test]
    fn q_quits() {
        let mut state = AppState::default();

        assert!(press(&mut state, KeyCode::Char('q')).is_break());
        assert!(press(&mut state, KeyCode::Char('Q')).is_break());
    }

    #[test]
    fn ctrl_c_quits() {
        let mut state = AppState::default();

        let flow = press_with(&mut state, KeyCode::Char('c'), KeyModifiers::CONTROL);

        assert!(flow.is_break());
    }

    #[test]
    fn esc_still_quits_with_nothing_open() {
        let mut state = AppState::default();

        assert!(press(&mut state, KeyCode::Esc).is_break());
    }

    #[test]
    fn q_does_not_quit_while_typing_a_url() {
        let mut state = AppState {
            editing_url: true,
            ..Default::default()
        };

        let flow = press(&mut state, KeyCode::Char('q'));

        assert!(flow.is_continue());
        assert_eq!(state.textarea.lines()[0], "q");
    }

    #[test]
    fn mode_keys_do_not_reach_through_the_channel_popup() {
        let mut state = with_channel_popup();

        // '1' used to switch to decode mode and strand the popup in the state.
        assert!(press(&mut state, KeyCode::Char('1')).is_continue());

        assert_eq!(state.app_mode, AppMode::Encode);
        assert!(state.channel_popup.is_some());
    }

    #[test]
    fn mode_keys_do_not_reach_through_the_lora_popup() {
        let mut state = with_lora_popup();

        assert!(press(&mut state, KeyCode::Char('2')).is_continue());
        assert!(press(&mut state, KeyCode::Char('m')).is_continue());

        assert_eq!(state.app_mode, AppMode::Encode);
        assert!(state.lora_popup.is_some());
    }

    #[test]
    fn q_does_not_quit_from_inside_a_popup() {
        let mut state = with_lora_popup();

        let flow = press(&mut state, KeyCode::Char('q'));

        assert!(flow.is_continue());
        assert!(state.lora_popup.is_some());
    }

    #[test]
    fn ctrl_c_quits_even_from_inside_a_popup() {
        let mut state = with_lora_popup();

        let flow = press_with(&mut state, KeyCode::Char('c'), KeyModifiers::CONTROL);

        assert!(flow.is_break());
    }

    #[test]
    fn esc_closes_a_popup_before_quitting() {
        let mut state = with_lora_popup();

        assert!(press(&mut state, KeyCode::Esc).is_continue());
        assert!(state.lora_popup.is_none());

        // A second Esc, with nothing left open, quits.
        assert!(press(&mut state, KeyCode::Esc).is_break());
    }

    #[test]
    fn esc_closes_the_name_box_before_the_channel_popup() {
        // The name box used to take the whole popup down with it, because both
        // this handler and the encode one acted on Esc.
        let mut state = with_channel_popup();
        state
            .channel_popup
            .as_mut()
            .expect("the popup is open")
            .start_editing_name();

        assert!(press(&mut state, KeyCode::Esc).is_continue());
        let popup = state.channel_popup.as_ref().expect("the popup stays open");
        assert!(!popup.editing_name, "the name box closed");

        // Only now, with nothing on top of it, does the popup close.
        assert!(press(&mut state, KeyCode::Esc).is_continue());
        assert!(state.channel_popup.is_none());
    }

    #[test]
    fn mode_keys_work_with_no_popup_open() {
        let mut state = AppState::default();

        assert!(press(&mut state, KeyCode::Char('2')).is_continue());
        assert_eq!(state.app_mode, AppMode::Encode);

        assert!(press(&mut state, KeyCode::Char('1')).is_continue());
        assert_eq!(state.app_mode, AppMode::Decode);
    }
}
