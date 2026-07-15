//! Interactive TUI mode using opi-tui for terminal rendering.
//!
//! The agent prompt runs in a spawned tokio task while the TUI render loop
//! continues to poll crossterm events and redraw at ~20 fps. Agent callbacks
//! update shared `TuiState`, which the render loop reads each frame.

use std::io;
use std::sync::{Arc, Mutex};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use opi_agent::event::AgentEvent;
use opi_agent::loop_types::AgentError;
use opi_agent::message::AgentMessage;
use opi_ai::message::{AssistantContent, Message};
use opi_ai::stream::AssistantStreamEvent;
use opi_tui::terminal_image::{
    CapabilitySource, TerminalGraphicsProtocol, detect_graphics_protocol,
};
use opi_tui::{
    AppState, Key, KeyCombo, Keybindings, Message as TuiMessage, Role as TuiRole, SelectListState,
    Shell, Theme, ToolCallStatus, resolve_theme,
};
use opi_tui::{ImageData, ImagePayload, MediaType as TuiMediaType};

use crate::harness::{CodingHarness, SessionMetadata};
use crate::interactive_auth::{
    AuthCommandOutcome, AuthCommandServices, dispatch_auth_command, is_terminal_restore_failure,
};
use crate::oauth::{OAuthProviderRegistry, TuiLoginPresenter};

/// Shared state mutated by the agent callback and read by the TUI render loop.
struct TuiState {
    messages: Vec<TuiMessage>,
    input_text: String,
    app_state: AppState,
    model: String,
    active_tool: Option<(String, String, ToolCallStatus)>,
    /// True when a TextDelta has been received for the current streaming cycle.
    /// Prevents MessageEnd from pushing a duplicate text message.
    streaming_started: bool,
    theme: Theme,
    keybindings: Keybindings,
    total_tokens: u64,
    cost_usd: Option<f64>,
    graphics_protocol: TerminalGraphicsProtocol,
    picker: Option<PickerOverlay>,
}

#[derive(Clone)]
struct PickerOverlay {
    kind: PickerKind,
    title: String,
    state: SelectListState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PickerKind {
    Model,
    Session,
    Branch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BranchPickerCommand {
    Branch,
    Tree,
}

impl BranchPickerCommand {
    fn title(self) -> &'static str {
        match self {
            Self::Branch => "Select branch",
            Self::Tree => "Session tree",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SessionForkCommand {
    Fork,
    Clone,
}

/// One pre-output turn that may be retried after an explicit successful login.
#[doc(hidden)]
#[derive(Default)]
pub struct PendingAuthTurn {
    provider_id: Option<String>,
}

impl PendingAuthTurn {
    /// Retain only typed pre-output credential failures.
    #[doc(hidden)]
    pub fn capture_error(&mut self, error: &AgentError) {
        if let AgentError::CredentialNeeded { provider_id } = error {
            self.provider_id = Some(provider_id.clone());
        }
    }

    fn clear(&mut self) {
        self.provider_id = None;
    }

    fn consume_login(&mut self, outcome: &AuthCommandOutcome) -> bool {
        let AuthCommandOutcome::LoggedIn { provider_id } = outcome else {
            return false;
        };
        if self.provider_id.as_deref() != Some(provider_id.as_str()) {
            return false;
        }
        self.provider_id = None;
        true
    }
}

/// Spawn the no-duplicate-message retry used by the TUI after `/login`.
#[doc(hidden)]
pub fn spawn_pending_auth_retry(
    pending_turn: &mut PendingAuthTurn,
    outcome: &AuthCommandOutcome,
    harness: Arc<tokio::sync::Mutex<CodingHarness>>,
) -> Option<tokio::task::JoinHandle<Result<Vec<AgentMessage>, AgentError>>> {
    pending_turn.consume_login(outcome).then(|| {
        tokio::spawn(async move {
            let mut harness = harness.lock().await;
            harness.retry_last_prompt().await
        })
    })
}

impl SessionForkCommand {
    fn verb(self) -> &'static str {
        match self {
            Self::Fork => "forked",
            Self::Clone => "cloned",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum PickerAction {
    SelectModel(String),
    SelectSession(String),
    SelectBranch(String),
    Cancel,
}

pub async fn run_interactive_tui(
    harness: CodingHarness,
    model: String,
    theme_name: &str,
    keybindings: Keybindings,
) -> Result<(), Box<dyn std::error::Error>> {
    let theme = resolve_interactive_theme(&harness, theme_name);
    if theme.name != theme_name {
        eprintln!("opi: warning: unknown theme {theme_name:?}, using default");
    }
    let graphics_protocol = detect_graphics_protocol(
        std::env::var("TERM").ok().as_deref(),
        std::env::var("TERM_PROGRAM").ok().as_deref(),
        std::env::var("TERM_FEATURES").ok().as_deref(),
        &CapabilitySource::EnvVars,
    );
    let state = Arc::new(Mutex::new(TuiState {
        messages: Vec::new(),
        input_text: String::new(),
        app_state: AppState::Idle,
        model: model.clone(),
        active_tool: None,
        streaming_started: false,
        theme,
        keybindings,
        total_tokens: 0,
        cost_usd: None,
        graphics_protocol,
        picker: None,
    }));

    // Wire agent events into shared state before wrapping harness
    let state_clone = state.clone();
    let mut harness = harness;
    harness.subscribe(Box::new(move |event| {
        let mut s = state_clone.lock().unwrap();
        match event {
            AgentEvent::MessageStart { .. } => {
                s.app_state = AppState::Streaming;
                s.streaming_started = false;
            }
            AgentEvent::MessageUpdate {
                assistant_event, ..
            } => {
                if let AssistantStreamEvent::TextDelta { delta, .. } = assistant_event.as_ref() {
                    if !s.streaming_started {
                        s.messages
                            .push(TuiMessage::new(TuiRole::Assistant, delta.clone()));
                        s.streaming_started = true;
                    } else if let Some(msg) = s.messages.last_mut() {
                        msg.content.push_str(delta);
                    }
                }
            }
            AgentEvent::MessageEnd {
                message: AgentMessage::Llm(Message::Assistant(a)),
            } => {
                s.total_tokens += a.usage.total_tokens();
                for content in &a.content {
                    match content {
                        AssistantContent::Text { text } if !s.streaming_started => {
                            s.messages
                                .push(TuiMessage::new(TuiRole::Assistant, text.clone()));
                        }
                        AssistantContent::ToolCall { tool_call } => {
                            s.active_tool = Some((
                                tool_call.name.clone(),
                                tool_call.arguments.clone(),
                                ToolCallStatus::Running,
                            ));
                        }
                        _ => {}
                    }
                }
                s.streaming_started = false;
            }
            AgentEvent::ToolExecutionStart {
                tool_name, args, ..
            } => {
                s.app_state = AppState::ToolExecuting;
                s.active_tool = Some((
                    tool_name.clone(),
                    format!("{args}"),
                    ToolCallStatus::Running,
                ));
            }
            AgentEvent::ToolExecutionEnd {
                tool_name,
                is_error,
                details,
                result,
                ..
            } => {
                // Render diff for edit tool results that have before/after details.
                if !is_error
                    && tool_name == "edit"
                    && let Some(d) = details
                    && let (Some(path), Some(before), Some(after)) =
                        (d.get("path"), d.get("before"), d.get("after"))
                {
                    let path_str = path.as_str().unwrap_or("unknown");
                    let before_str = before.as_str().unwrap_or("");
                    let after_str = after.as_str().unwrap_or("");
                    s.messages
                        .push(TuiMessage::diff(path_str, before_str, after_str));
                }
                // Extract image content from tool result.
                let protocol = s.graphics_protocol;
                if let Some(content_arr) = result.as_array() {
                    for item in content_arr {
                        if item.get("type").and_then(|v| v.as_str()) == Some("image")
                            && let Some(source) = item.get("source")
                        {
                            let bytes = if source.get("type").and_then(|v| v.as_str())
                                == Some("bytes")
                            {
                                source
                                    .get("data")
                                    .and_then(|v| v.as_array())
                                    .map(|arr| {
                                        arr.iter()
                                            .filter_map(|v| v.as_u64().map(|n| n as u8))
                                            .collect::<Vec<u8>>()
                                    })
                                    .unwrap_or_default()
                            } else if source.get("type").and_then(|v| v.as_str()) == Some("base64")
                            {
                                use base64::Engine;
                                source
                                    .get("data")
                                    .and_then(|v| v.as_str())
                                    .and_then(|d| {
                                        base64::engine::general_purpose::STANDARD.decode(d).ok()
                                    })
                                    .unwrap_or_default()
                            } else {
                                vec![]
                            };
                            if !bytes.is_empty() {
                                let media_type = item.get("media_type").and_then(|v| v.as_str());
                                let tui_media = match media_type {
                                    Some("image/jpeg") => TuiMediaType::Jpeg,
                                    Some("image/gif") => TuiMediaType::Gif,
                                    Some("image/webp") => TuiMediaType::WebP,
                                    _ => TuiMediaType::Png,
                                };
                                let image_data = ImageData {
                                    bytes,
                                    media_type: tui_media,
                                    width: None,
                                    height: None,
                                };
                                s.messages.push(TuiMessage::image(
                                    TuiRole::Tool,
                                    ImagePayload {
                                        data: image_data,
                                        protocol,
                                    },
                                ));
                            }
                        }
                    }
                }
                if let Some((name, args, _)) = &s.active_tool
                    && name == tool_name
                {
                    let status = if *is_error {
                        ToolCallStatus::Error("failed".into())
                    } else {
                        ToolCallStatus::Success
                    };
                    s.active_tool = Some((name.clone(), args.clone(), status));
                }
                s.app_state = AppState::Streaming;
            }
            AgentEvent::AgentEnd { .. } => {
                s.app_state = AppState::Idle;
                s.active_tool = None;
            }
            AgentEvent::TurnStart => {
                s.app_state = AppState::Thinking;
            }
            AgentEvent::CompactionStart { reason } => {
                s.messages.push(TuiMessage::new(
                    TuiRole::System,
                    format!("[compaction started: {reason:?}]"),
                ));
            }
            AgentEvent::CompactionEnd {
                reason,
                result,
                aborted,
                error_message,
            } => {
                let summary = if *aborted {
                    format!(
                        "[compaction aborted ({reason:?}): {}]",
                        error_message.clone().unwrap_or_default()
                    )
                } else if let Some(r) = result {
                    format!(
                        "[compaction done ({reason:?}): {} -> {} tokens]",
                        r.tokens_before, r.tokens_after
                    )
                } else {
                    format!("[compaction done ({reason:?})]")
                };
                s.messages.push(TuiMessage::new(TuiRole::System, summary));
            }
            AgentEvent::SessionPersistError { message } => {
                s.messages.push(TuiMessage::new(
                    TuiRole::System,
                    format!("[session persist error: {message}]"),
                ));
            }
            _ => {}
        }
    }));

    // Capture the store & registry before the harness moves into Arc<Mutex>;
    // the authentication dispatcher owns their interactive command access.
    let credential_store = harness
        .credential_store
        .take()
        .ok_or_else(|| io::Error::other("interactive credential store is not configured"))?;
    let oauth_registry = harness
        .oauth_registry
        .take()
        .ok_or_else(|| io::Error::other("interactive OAuth registry is not configured"))?;

    let harness = Arc::new(tokio::sync::Mutex::new(harness));

    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main TUI loop
    let result = tui_event_loop(
        &mut terminal,
        &harness,
        &state,
        credential_store,
        oauth_registry,
    )
    .await;

    // Restore terminal
    terminal::disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn tui_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    harness: &Arc<tokio::sync::Mutex<CodingHarness>>,
    state: &Arc<Mutex<TuiState>>,
    credential_store: Arc<crate::credential_store::KeychainCredentialStore>,
    oauth_registry: OAuthProviderRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut pending: Option<tokio::task::JoinHandle<Result<Vec<AgentMessage>, AgentError>>> = None;
    let mut pending_auth_turn = PendingAuthTurn::default();
    let mut cancel_token = harness.lock().await.cancel_token();

    loop {
        // Render current state
        {
            let s = state.lock().unwrap();
            let shell = build_shell(&s);
            terminal.draw(|frame| frame.render_widget(shell, frame.area()))?;
        }

        // Check if pending prompt finished (non-blocking)
        if let Some(handle) = &mut pending
            && handle.is_finished()
        {
            match handle.await {
                Ok(Ok(_messages)) => {
                    let mut s = state.lock().unwrap();
                    s.app_state = AppState::Idle;
                }
                Ok(Err(AgentError::Cancelled)) => {
                    let mut s = state.lock().unwrap();
                    s.app_state = AppState::Idle;
                }
                Ok(Err(
                    ref error @ AgentError::CredentialNeeded {
                        ref provider_id, ..
                    },
                )) => {
                    pending_auth_turn.capture_error(error);
                    let mut s = state
                        .lock()
                        .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
                    s.messages.push(TuiMessage::new(
                        TuiRole::System,
                        format!(
                            "[credential needed for '{provider_id}': run /login {provider_id}]"
                        ),
                    ));
                    s.app_state = AppState::Idle;
                }
                Ok(Err(e)) => {
                    let mut s = state.lock().unwrap();
                    s.messages
                        .push(TuiMessage::new(TuiRole::System, format!("error: {e}")));
                    s.app_state = AppState::Idle;
                }
                Err(e) => {
                    let mut s = state.lock().unwrap();
                    s.messages
                        .push(TuiMessage::new(TuiRole::System, format!("error: {e}")));
                    s.app_state = AppState::Idle;
                }
            }

            // Refresh cost from the harness session (pricing lookup may yield
            // a number; if the model isn't in the table we leave it as-is).
            {
                let h = harness.lock().await;
                if let Some(session) = h.session()
                    && let Some(cost) = session.cost_summary()
                {
                    state.lock().unwrap().cost_usd = Some(cost.total_cost());
                }
            }

            // Refresh cancel token — Agent::maybe_reset_cancel() creates a new one
            // after cancellation, so the old token would be stale.
            cancel_token = harness.lock().await.cancel_token();
            pending = None;
        }

        // Poll for terminal events (non-blocking with timeout)
        if event::poll(std::time::Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            let kb = state.lock().unwrap().keybindings.clone();
            if let Some(action) = {
                let mut s = state.lock().unwrap();
                handle_picker_key(&mut s, key.code)
            } {
                match action {
                    PickerAction::SelectModel(model) => {
                        let result = {
                            let mut h = harness.lock().await;
                            h.set_model_validated(model).map(str::to_owned)
                        };
                        let mut s = state.lock().unwrap();
                        match result {
                            Ok(actual_model) => {
                                s.model = actual_model.clone();
                                s.messages.push(TuiMessage::new(
                                    TuiRole::System,
                                    format!("[model switched: {actual_model}]"),
                                ));
                            }
                            Err(e) => {
                                s.messages.push(TuiMessage::new(
                                    TuiRole::System,
                                    format!("[model switch failed: {e}]"),
                                ));
                            }
                        }
                    }
                    PickerAction::SelectSession(session_id) => {
                        let result = {
                            let mut h = harness.lock().await;
                            h.resume_session_id(&session_id)
                        };
                        let mut s = state.lock().unwrap();
                        match result {
                            Ok(count) => s.messages.push(TuiMessage::new(
                                TuiRole::System,
                                format!("[session resumed: {session_id}, {count} messages]"),
                            )),
                            Err(e) => s.messages.push(TuiMessage::new(
                                TuiRole::System,
                                format!("[session resume failed: {e}]"),
                            )),
                        }
                    }
                    PickerAction::SelectBranch(tip_id) => {
                        let result = {
                            let mut h = harness.lock().await;
                            h.resume_session_branch_tip(&tip_id)
                        };
                        let mut s = state.lock().unwrap();
                        match result {
                            Ok(count) => s.messages.push(TuiMessage::new(
                                TuiRole::System,
                                format!("[branch selected: {tip_id}, {count} messages]"),
                            )),
                            Err(e) => s.messages.push(TuiMessage::new(
                                TuiRole::System,
                                format!("[branch select failed: {e}]"),
                            )),
                        }
                    }
                    PickerAction::Cancel => {}
                }
                continue;
            }

            if matches_key_combo(key.code, key.modifiers, &kb.submit) {
                // Ignore submit while agent is running
                if pending.is_some() {
                    continue;
                }

                let input = {
                    let mut s = state.lock().unwrap();
                    let text = s.input_text.trim().to_string();
                    s.input_text.clear();
                    text
                };

                if input == "exit" || input == "quit" {
                    // Cancel any pending task on exit
                    if let Some(handle) = pending.take() {
                        cancel_token.cancel();
                        let _ = handle.await;
                    }
                    return Ok(());
                }
                if input.is_empty() {
                    continue;
                }

                if input == "/model" {
                    let items = {
                        let h = harness.lock().await;
                        h.model_picker_items()
                    };
                    let mut s = state.lock().unwrap();
                    if items.is_empty() {
                        s.messages.push(TuiMessage::new(
                            TuiRole::System,
                            "[model picker: no models available]",
                        ));
                    } else {
                        s.picker = Some(PickerOverlay {
                            kind: PickerKind::Model,
                            title: "Select model".into(),
                            state: SelectListState::new(items),
                        });
                    }
                    continue;
                }

                // Phase 13.4: /name, /label, /unlabel, /session info dispatch
                // through the typed session-metadata path. The parser returns
                // None for bare `/session` so the resume-picker block below
                // continues to handle it (non-regression).
                if let Some(command) = parse_session_metadata_command(&input) {
                    let message = {
                        let mut h = harness.lock().await;
                        apply_session_metadata_command(&mut h, command)
                    };
                    state
                        .lock()
                        .unwrap()
                        .messages
                        .push(TuiMessage::new(TuiRole::System, message));
                    continue;
                }

                if input == "/session" {
                    let dir = crate::session_cli::session_dir();
                    let items = crate::picker::session_picker_items(&dir).unwrap_or_default();
                    let mut s = state.lock().unwrap();
                    if items.is_empty() {
                        s.messages.push(TuiMessage::new(
                            TuiRole::System,
                            "[session picker: no sessions available]",
                        ));
                    } else {
                        s.picker = Some(PickerOverlay {
                            kind: PickerKind::Session,
                            title: "Resume session".into(),
                            state: SelectListState::new(items),
                        });
                    }
                    continue;
                }

                if let Some(command) = branch_picker_command(&input) {
                    let items_result = {
                        let h = harness.lock().await;
                        h.branch_picker_items()
                    };
                    let mut s = state.lock().unwrap();
                    match items_result {
                        Ok(items) if items.is_empty() => {
                            s.messages.push(TuiMessage::new(
                                TuiRole::System,
                                "[branch picker: no branches available]",
                            ));
                        }
                        Ok(items) => {
                            s.picker = Some(PickerOverlay {
                                kind: PickerKind::Branch,
                                title: command.title().into(),
                                state: SelectListState::new(items),
                            });
                        }
                        Err(e) => {
                            s.messages.push(TuiMessage::new(
                                TuiRole::System,
                                format!("[branch picker failed: {e}]"),
                            ));
                        }
                    }
                    continue;
                }

                if let Some(command) = session_fork_command(&input) {
                    let result = {
                        let mut h = harness.lock().await;
                        h.fork_current_session()
                    };
                    let mut s = state.lock().unwrap();
                    match result {
                        Ok((session_id, count)) => s.messages.push(TuiMessage::new(
                            TuiRole::System,
                            format!(
                                "[session {}: {session_id}, {count} messages]",
                                command.verb()
                            ),
                        )),
                        Err(e) => s.messages.push(TuiMessage::new(
                            TuiRole::System,
                            format!("[session {} failed: {e}]", command.verb()),
                        )),
                    }
                    continue;
                }

                if let Some(rest) = input.strip_prefix("/image ") {
                    let path = rest.trim();
                    if path.is_empty() {
                        let mut s = state.lock().unwrap();
                        s.messages.push(TuiMessage::new(
                            TuiRole::System,
                            "[/image: usage: /image <path>]".to_string(),
                        ));
                    } else {
                        let image_path = std::path::PathBuf::from(path);
                        let max_bytes = {
                            let h = harness.lock().await;
                            h.config().defaults.max_image_bytes
                        };
                        match crate::image::load_image_with_limit(&image_path, max_bytes) {
                            Ok(img) => {
                                harness.lock().await.queue_images(vec![img]);
                                let mut s = state.lock().unwrap();
                                s.messages.push(TuiMessage::new(
                                    TuiRole::System,
                                    format!("[image queued: {}]", image_path.display()),
                                ));
                            }
                            Err(e) => {
                                let mut s = state.lock().unwrap();
                                s.messages.push(TuiMessage::new(
                                    TuiRole::System,
                                    format!("[/image error: {e}]"),
                                ));
                            }
                        }
                    }
                    continue;
                }

                let presenter = TuiLoginPresenter::new();
                let auth_outcome = dispatch_auth_command(
                    &input,
                    terminal,
                    AuthCommandServices {
                        store: credential_store.as_ref(),
                        registry: &oauth_registry,
                        presenter: &presenter,
                    },
                )
                .await;
                let retry = spawn_pending_auth_retry(
                    &mut pending_auth_turn,
                    &auth_outcome,
                    harness.clone(),
                );
                if let Some(message) = route_auth_outcome(auth_outcome)? {
                    let mut s = state
                        .lock()
                        .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
                    s.messages.push(TuiMessage::new(TuiRole::System, message));
                    if retry.is_some() {
                        s.app_state = AppState::Thinking;
                    }
                    drop(s);
                    if let Some(handle) = retry {
                        pending = Some(handle);
                    }
                    continue;
                }

                // Add user message to display
                pending_auth_turn.clear();
                {
                    let mut s = state.lock().unwrap();
                    s.messages
                        .push(TuiMessage::new(TuiRole::User, input.clone()));
                    s.app_state = AppState::Thinking;
                }

                // Spawn agent prompt in background task
                let h = harness.clone();
                let handle = tokio::spawn(async move {
                    let mut h = h.lock().await;
                    let pending = h.take_pending_images();
                    if pending.is_empty() {
                        h.prompt(&input).await
                    } else {
                        let mut content = vec![opi_ai::message::InputContent::Text { text: input }];
                        content.extend(pending);
                        h.prompt_with_content(content).await
                    }
                });
                pending = Some(handle);
            } else if matches_key_combo(key.code, key.modifiers, &kb.abort) {
                if pending.is_some() {
                    cancel_token.cancel();
                } else {
                    return Ok(());
                }
            } else if matches_key_combo(key.code, key.modifiers, &kb.new_line) {
                if pending.is_none() {
                    state.lock().unwrap().input_text.push('\n');
                }
            } else {
                match key.code {
                    KeyCode::Char(c) if pending.is_none() => {
                        state.lock().unwrap().input_text.push(c);
                    }
                    KeyCode::Backspace if pending.is_none() => {
                        state.lock().unwrap().input_text.pop();
                    }
                    _ => {}
                }
            }
        }
    }
}

fn resolve_interactive_theme(harness: &CodingHarness, theme_name: &str) -> Theme {
    harness
        .resolve_theme(theme_name)
        .unwrap_or_else(|_| resolve_theme(theme_name))
}

fn route_auth_outcome(outcome: AuthCommandOutcome) -> io::Result<Option<String>> {
    if is_terminal_restore_failure(&outcome) {
        return Err(io::Error::other("terminal restore failed"));
    }
    Ok(match outcome {
        AuthCommandOutcome::NotHandled => None,
        AuthCommandOutcome::Usage(usage) => Some(format!("[{usage}]")),
        AuthCommandOutcome::LoggedIn { provider_id } => {
            Some(format!("[/login: {provider_id} succeeded]"))
        }
        AuthCommandOutcome::LoggedOut { provider_id } => {
            Some(format!("[/logout: {provider_id} deleted]"))
        }
        AuthCommandOutcome::Failed { message } => {
            Some(format!("[authentication command failed: {message}]"))
        }
    })
}

fn build_shell(s: &TuiState) -> Shell {
    let mut shell = Shell::new(s.model.clone())
        .input_text(s.input_text.clone())
        .state(s.app_state)
        .theme(s.theme.clone());

    if s.total_tokens > 0 {
        shell = shell.token_count(s.total_tokens);
    }

    if let Some(cost) = s.cost_usd {
        shell = shell.cost_usd(cost);
    }

    if !s.messages.is_empty() {
        shell = shell.messages(s.messages.clone());
    }

    if let Some((name, args, status)) = &s.active_tool {
        shell = shell.active_tool(name.clone(), args.clone(), status.clone());
    }

    if let Some(picker) = &s.picker {
        shell = shell.picker(picker.title.clone(), picker.state.clone());
    }

    shell
}

fn branch_picker_command(input: &str) -> Option<BranchPickerCommand> {
    match input {
        "/branch" => Some(BranchPickerCommand::Branch),
        "/tree" => Some(BranchPickerCommand::Tree),
        _ => None,
    }
}

/// A parsed Phase 13.4 session-metadata slash command (Phase 13.4). Returned
/// by [`parse_session_metadata_command`]. Bare `/session` (resume picker) and
/// every other input return `None` so existing dispatch paths are unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionMetadataCommand {
    Name(String),
    Label(String),
    Unlabel(String),
    Info,
    Usage(String),
}

/// Parse a Phase 13.4 session-metadata slash command from raw TUI input.
///
/// Returns `Some` only for:
/// - `/name <name>` (non-empty) or a local usage hint for empty args
/// - `/label <label>` (non-empty) or a local usage hint for empty args
/// - `/unlabel <label>` (non-empty) or a local usage hint for empty args
/// - the exact form `/session info`
///
/// Returns `None` for bare `/session` so the existing resume-picker path
/// continues to handle it, and for every other input (including `/model`,
/// `/branch`, `/tree`, `/fork`, `/clone`, `/image`, and free-form prompts).
pub fn parse_session_metadata_command(input: &str) -> Option<SessionMetadataCommand> {
    let trimmed = input.trim();
    if trimmed == "/name" {
        return Some(SessionMetadataCommand::Usage("usage: /name <name>".into()));
    }
    if let Some(rest) = trimmed.strip_prefix("/name ") {
        let name = rest.trim();
        if !name.is_empty() {
            return Some(SessionMetadataCommand::Name(name.to_owned()));
        }
        return Some(SessionMetadataCommand::Usage("usage: /name <name>".into()));
    }
    if trimmed == "/label" {
        return Some(SessionMetadataCommand::Usage(
            "usage: /label <label>".into(),
        ));
    }
    if let Some(rest) = trimmed.strip_prefix("/label ") {
        let label = rest.trim();
        if !label.is_empty() {
            return Some(SessionMetadataCommand::Label(label.to_owned()));
        }
        return Some(SessionMetadataCommand::Usage(
            "usage: /label <label>".into(),
        ));
    }
    if trimmed == "/unlabel" {
        return Some(SessionMetadataCommand::Usage(
            "usage: /unlabel <label>".into(),
        ));
    }
    if let Some(rest) = trimmed.strip_prefix("/unlabel ") {
        let label = rest.trim();
        if !label.is_empty() {
            return Some(SessionMetadataCommand::Unlabel(label.to_owned()));
        }
        return Some(SessionMetadataCommand::Usage(
            "usage: /unlabel <label>".into(),
        ));
    }
    if trimmed == "/session info" {
        return Some(SessionMetadataCommand::Info);
    }
    None
}

/// Apply a parsed session-metadata command against the harness (Phase 13.4)
/// and return the human-readable status line the TUI should display. This is
/// the shared dispatch path for `/name`, `/label`, `/unlabel`, and
/// `/session info`, extracted so tests can exercise command → harness →
/// coordinator → JSONL without driving the terminal event loop. Command
/// failures are reported in the returned string before any success is claimed.
pub fn apply_session_metadata_command(
    harness: &mut CodingHarness,
    command: SessionMetadataCommand,
) -> String {
    match command {
        SessionMetadataCommand::Name(name) => match harness.set_session_name(name.clone()) {
            Ok(()) => format!("[session name set: {name}]"),
            Err(e) => format!("[/name failed: {e}]"),
        },
        SessionMetadataCommand::Label(label) => match harness.add_label(label.clone()) {
            Ok(()) => format!("[label added: {label}]"),
            Err(e) => format!("[/label failed: {e}]"),
        },
        SessionMetadataCommand::Unlabel(label) => match harness.remove_label(label.clone()) {
            Ok(()) => format!("[label removed: {label}]"),
            Err(e) => format!("[/unlabel failed: {e}]"),
        },
        SessionMetadataCommand::Info => match harness.session_metadata() {
            Some(meta) => format_session_metadata(&meta),
            None => "[session info: no active session]".to_string(),
        },
        SessionMetadataCommand::Usage(usage) => format!("[{usage}]"),
    }
}

/// Render the live session metadata as a single-line TUI status message.
fn format_session_metadata(meta: &SessionMetadata) -> String {
    let name = meta.name.clone().unwrap_or_else(|| "(unset)".to_string());
    let labels = if meta.labels.is_empty() {
        "(none)".to_string()
    } else {
        meta.labels.join(", ")
    };
    let branch = meta
        .active_branch
        .clone()
        .unwrap_or_else(|| "(none)".to_string());
    let thinking = if meta.thinking.enabled { "on" } else { "off" };
    format!(
        "[session name: {name} | labels: {labels} | branch: {branch} | model: {} | thinking: {thinking}]",
        meta.model
    )
}

fn session_fork_command(input: &str) -> Option<SessionForkCommand> {
    match input {
        "/fork" => Some(SessionForkCommand::Fork),
        "/clone" => Some(SessionForkCommand::Clone),
        _ => None,
    }
}

fn handle_picker_key(s: &mut TuiState, code: KeyCode) -> Option<PickerAction> {
    let picker = s.picker.as_mut()?;
    match code {
        KeyCode::Esc => {
            s.picker = None;
            Some(PickerAction::Cancel)
        }
        KeyCode::Enter => {
            let item = picker.state.confirm().cloned();
            let kind = picker.kind;
            s.picker = None;
            match (kind, item) {
                (PickerKind::Model, Some(item)) => Some(PickerAction::SelectModel(item.id)),
                (PickerKind::Session, Some(item)) => Some(PickerAction::SelectSession(item.id)),
                (PickerKind::Branch, Some(item)) => Some(PickerAction::SelectBranch(item.id)),
                (_, None) => Some(PickerAction::Cancel),
            }
        }
        KeyCode::Down => {
            picker.state.move_down();
            None
        }
        KeyCode::Up => {
            picker.state.move_up();
            None
        }
        KeyCode::PageDown => {
            picker.state.page_down(10);
            None
        }
        KeyCode::PageUp => {
            picker.state.page_up(10);
            None
        }
        KeyCode::Backspace => {
            let mut filter = picker.state.filter().to_string();
            filter.pop();
            picker.state.set_filter(filter);
            None
        }
        KeyCode::Char(c) => {
            let mut filter = picker.state.filter().to_string();
            filter.push(c);
            picker.state.set_filter(filter);
            None
        }
        _ => None,
    }
}

fn matches_key_combo(code: KeyCode, modifiers: KeyModifiers, combo: &KeyCombo) -> bool {
    let key_matches = match (code, &combo.key) {
        (KeyCode::Enter, Key::Enter) => true,
        (KeyCode::Esc, Key::Escape) => true,
        (KeyCode::Tab, Key::Tab) => true,
        (KeyCode::Backspace, Key::Backspace) => true,
        (KeyCode::Char(c), Key::Char(expected)) => c == *expected,
        _ => false,
    };
    if !key_matches {
        return false;
    }
    combo.modifiers.alt == modifiers.contains(KeyModifiers::ALT)
        && combo.modifiers.ctrl == modifiers.contains(KeyModifiers::CONTROL)
        && combo.modifiers.shift == modifiers.contains(KeyModifiers::SHIFT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OpiConfig;
    use crate::resource::{DiscoveryLayer, ResourceDiscoveryLayers};
    use opi_ai::test_support::MockProvider;
    use opi_tui::SelectItem;

    #[test]
    fn auth_outcome_routing_makes_only_terminal_restore_failure_fatal() {
        let ordinary = route_auth_outcome(AuthCommandOutcome::Failed {
            message: "authentication failed".to_owned(),
        })
        .expect("ordinary auth failure continues the TUI");
        assert_eq!(
            ordinary,
            Some("[authentication command failed: authentication failed]".to_owned())
        );

        let fatal = route_auth_outcome(AuthCommandOutcome::Failed {
            message: "terminal restore failed".to_owned(),
        })
        .expect_err("terminal restoration failure must leave the event loop");
        assert_eq!(fatal.kind(), io::ErrorKind::Other);
        assert_eq!(fatal.to_string(), "terminal restore failed");
    }

    fn state_with_picker(kind: PickerKind) -> TuiState {
        TuiState {
            messages: Vec::new(),
            input_text: String::new(),
            app_state: AppState::Idle,
            model: "mock:old".into(),
            active_tool: None,
            streaming_started: false,
            theme: Theme::default(),
            keybindings: Keybindings::default(),
            total_tokens: 0,
            cost_usd: None,
            graphics_protocol: TerminalGraphicsProtocol::Fallback,
            picker: Some(PickerOverlay {
                kind,
                title: "Pick".into(),
                state: SelectListState::new(vec![SelectItem {
                    id: "mock:new".into(),
                    display: "New".into(),
                    metadata: "mock".into(),
                }]),
            }),
        }
    }

    #[test]
    fn model_picker_enter_returns_selected_model() {
        let mut state = state_with_picker(PickerKind::Model);
        let action = handle_picker_key(&mut state, KeyCode::Enter);
        assert_eq!(action, Some(PickerAction::SelectModel("mock:new".into())));
        assert!(state.picker.is_none());
    }

    #[test]
    fn session_picker_enter_returns_selected_session() {
        let mut state = state_with_picker(PickerKind::Session);
        let action = handle_picker_key(&mut state, KeyCode::Enter);
        assert_eq!(action, Some(PickerAction::SelectSession("mock:new".into())));
        assert!(state.picker.is_none());
    }

    #[test]
    fn branch_picker_enter_returns_selected_tip() {
        let mut state = state_with_picker(PickerKind::Branch);
        let action = handle_picker_key(&mut state, KeyCode::Enter);
        assert_eq!(action, Some(PickerAction::SelectBranch("mock:new".into())));
        assert!(state.picker.is_none());
    }

    #[test]
    fn branch_picker_command_parses_branch_and_tree() {
        assert_eq!(
            branch_picker_command("/branch"),
            Some(BranchPickerCommand::Branch)
        );
        assert_eq!(
            branch_picker_command("/tree"),
            Some(BranchPickerCommand::Tree)
        );
        assert_eq!(branch_picker_command("/session"), None);
        assert_eq!(BranchPickerCommand::Branch.title(), "Select branch");
        assert_eq!(BranchPickerCommand::Tree.title(), "Session tree");
    }

    #[test]
    fn session_fork_command_parses_fork_and_clone() {
        assert_eq!(
            session_fork_command("/fork"),
            Some(SessionForkCommand::Fork)
        );
        assert_eq!(
            session_fork_command("/clone"),
            Some(SessionForkCommand::Clone)
        );
        assert_eq!(session_fork_command("/branch"), None);
        assert_eq!(SessionForkCommand::Fork.verb(), "forked");
        assert_eq!(SessionForkCommand::Clone.verb(), "cloned");
    }

    #[test]
    fn interactive_theme_resolver_uses_harness_discovered_themes() {
        let tmp = tempfile::tempdir().unwrap();
        let theme_dir = tmp.path().join("operator-theme");
        std::fs::create_dir_all(&theme_dir).unwrap();
        std::fs::write(
            theme_dir.join("theme.toml"),
            r##"
name = "operator-theme"
description = "Operator theme"

[colors]
role_user = "Red"
status_bg = "#1a1a2e"
"##,
        )
        .unwrap();

        let provider = MockProvider::new("mock", vec![]);
        let harness = CodingHarness::builder(
            Box::new(provider),
            "mock:mock-model".into(),
            OpiConfig::default(),
            tmp.path().to_path_buf(),
        )
        .resource_layers(ResourceDiscoveryLayers {
            themes: vec![DiscoveryLayer {
                root: theme_dir,
                subdirectory: None,
                precedence: 2,
            }],
            ..Default::default()
        })
        .build();

        let theme = resolve_interactive_theme(&harness, "operator-theme");

        assert_eq!(theme.name, "operator-theme");
    }
}
