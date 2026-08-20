//! Interactive TUI mode using opi-tui for terminal rendering.
//!
//! The agent prompt runs in a spawned tokio task while the TUI render loop
//! continues to poll crossterm events and redraw at ~20 fps. Agent callbacks
//! update shared `TuiState`, which the render loop reads each frame.

use std::collections::VecDeque;
use std::future::Future;
use std::io;
use std::path::Path;
use std::pin::Pin;
use std::sync::OnceLock;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::TestBackend;
use ratatui::prelude::*;

use opi_agent::diagnostic::{DiagnosticPayload, RedactionMode};
use opi_agent::event::AgentEvent;
use opi_agent::loop_types::AgentError;
use opi_agent::message::AgentMessage;
use opi_ai::message::{AssistantContent, Message};
use opi_ai::stream::AssistantStreamEvent;
use opi_tui::terminal_image::{
    CapabilitySource, TerminalGraphicsProtocol, detect_graphics_protocol,
};
use opi_tui::{
    AppState, AppStatus, Key, KeyCombo, Keybindings, Message as TuiMessage, Role as TuiRole,
    SelectListState, Shell, StatusBar, Theme, ToolCallStatus, resolve_theme,
};
use opi_tui::{AwaitingTrustState, TrustChoice, TrustPrompt};
use opi_tui::{ImageData, ImagePayload, MediaType as TuiMediaType};
use opi_tui::{PermissionChoice, PermissionPrompt, PermissionSummary};
use tokio::sync::{mpsc, oneshot};

use crate::execution::InteractivePermissionBroker;
use crate::harness::{CodingHarness, ProviderAuthFailure, SessionMetadata, provider_auth_failure};
use crate::interactive_auth::auth_command_requires_presenter;
use crate::interactive_auth::{
    AuthCommandOutcome, AuthCommandServices, LoginTerminalControl, dispatch_auth_command,
    is_auth_command, is_terminal_restore_failure,
};
use crate::oauth::{OAuthEndpointConfig, TuiLoginPresenter};
use crate::project_trust::{
    ProjectStartupPlan, ProjectTrustStore, TrustDecision, TrustError, apply_ui_choice,
};

// ---------------------------------------------------------------------------
// Interactive project-trust prompt and decision resolution
// ---------------------------------------------------------------------------

/// Source of an interactive project-trust prompt.
///
/// The production implementation drives the real TUI (see [`run_trust_prompt`]);
/// tests inject a canned/recording implementation. [`Self::ask`] renders the
/// five-way [`TrustChoice`] prompt and resolves to the user's choice, or `None`
/// when the prompt is cancelled/closed (which resolves to
/// [`TrustDecision::Untrusted`] without loading project resources).
pub trait InteractiveTrustPrompt: Send {
    /// Render the prompt for `project_path` and await exactly one choice.
    fn ask(
        &mut self,
        project_path: &Path,
    ) -> Pin<Box<dyn Future<Output = Option<TrustChoice>> + Send + '_>>;
}

/// Resolve the interactive trust decision from a startup plan.
///
/// A pre-decided plan (CLI override, embedder resolver vote, stored entry, or
/// global default) **bypasses** the prompt entirely. An undecided plan with
/// trust-requiring resources renders the prompt via `prompt`; the choice is
/// translated + persisted by [`apply_ui_choice`]. A cancelled/closed prompt
/// (`None`) resolves to [`TrustDecision::Untrusted`] so no project resources
/// load. Persistence and store reload failures are returned so startup cannot
/// silently treat a durable choice as session-only. The returned decision feeds
/// the two-stage config/resource gate and
/// `CodingHarnessBuilder::trust_decision`, and provably precedes provider
/// construction, installed-package/adapter startup, and harness build.
pub async fn resolve_interactive_trust_decision(
    plan: &ProjectStartupPlan,
    user_config_dir: &Path,
    project_root: &Path,
    prompt: &mut dyn InteractiveTrustPrompt,
) -> Result<TrustDecision, TrustError> {
    match plan.decision {
        // Bypass: a decisive earlier layer (CLI/resolver/store/default) wins.
        TrustDecision::Trusted | TrustDecision::Untrusted => Ok(plan.decision),
        TrustDecision::Undecided => match prompt.ask(project_root).await {
            Some(choice) => {
                // The store loaded during preflight, but reload here because
                // another process may have changed it while the prompt waited.
                let store = ProjectTrustStore::load(user_config_dir)?;
                apply_ui_choice(choice, &store, project_root)
            }
            // Cancelled/closed prompt -> safe deny; no project resources load.
            None => Ok(TrustDecision::Untrusted),
        },
    }
}

/// Render the five-way trust prompt on the real terminal and await one choice.
/// Returns `None` if the user cancels (Esc) or the
/// terminal closes before a choice. `Err` only on terminal I/O.
///
/// The blocking crossterm loop runs in `spawn_blocking` and sends the choice
/// through a oneshot; awaiting the receiver yields exactly one response, and a
/// dropped sender (Esc/cancel) yields `None`.
pub async fn run_trust_prompt(project_path: &Path) -> io::Result<Option<TrustChoice>> {
    let (response_tx, response_rx) = tokio::sync::oneshot::channel::<TrustChoice>();
    let path = project_path.to_path_buf();
    let join = tokio::task::spawn_blocking(move || run_trust_prompt_terminal(&path, response_tx));
    match join.await {
        Ok(Ok(())) => Ok(response_rx.await.ok()),
        Ok(Err(e)) => Err(e),
        Err(panic) => Err(io::Error::other(format!("trust prompt panicked: {panic}"))),
    }
}

/// Blocking terminal prompt loop: drives [`AppState::AwaitingTrust`] (exercising
/// the production variant + its [`AppStatus`] projection in the status bar) and
/// the [`TrustPrompt`] widget, sending the chosen [`TrustChoice`] through
/// `response_tx`. On Esc the sender is dropped (the receiver yields `None`).
fn run_trust_prompt_terminal(
    project_path: &Path,
    response_tx: tokio::sync::oneshot::Sender<TrustChoice>,
) -> io::Result<()> {
    let mut ops = CrosstermTrustPromptTerminalOps::default();
    run_trust_prompt_terminal_with_ops(project_path, response_tx, &mut ops)
}

struct TrustPromptRender<'a> {
    project_display: &'a str,
    status: opi_tui::AppStatus,
    prompt: &'a TrustPrompt,
}

trait TrustPromptTerminalOps {
    fn enable_raw_mode(&mut self) -> io::Result<()>;
    fn enter_alternate_screen(&mut self) -> io::Result<()>;
    fn construct_terminal(&mut self) -> io::Result<()>;
    fn draw(&mut self, render: &TrustPromptRender<'_>) -> io::Result<()>;
    fn poll(&mut self, timeout: std::time::Duration) -> io::Result<bool>;
    fn read(&mut self) -> io::Result<Event>;
    fn leave_alternate_screen(&mut self) -> io::Result<()>;
    fn disable_raw_mode(&mut self) -> io::Result<()>;
}

#[derive(Default)]
struct CrosstermTrustPromptTerminalOps {
    terminal: Option<Terminal<CrosstermBackend<io::Stdout>>>,
}

impl TrustPromptTerminalOps for CrosstermTrustPromptTerminalOps {
    fn enable_raw_mode(&mut self) -> io::Result<()> {
        terminal::enable_raw_mode()
    }

    fn enter_alternate_screen(&mut self) -> io::Result<()> {
        crossterm::execute!(io::stdout(), EnterAlternateScreen)
    }

    fn construct_terminal(&mut self) -> io::Result<()> {
        let backend = CrosstermBackend::new(io::stdout());
        self.terminal = Some(Terminal::new(backend)?);
        Ok(())
    }

    fn draw(&mut self, render: &TrustPromptRender<'_>) -> io::Result<()> {
        let terminal = self
            .terminal
            .as_mut()
            .ok_or_else(|| io::Error::other("trust-prompt terminal is not constructed"))?;
        terminal.draw(|f| {
            let chunks =
                Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(f.area());
            f.render_widget(
                StatusBar::new(render.project_display.to_owned(), render.status, None),
                chunks[0],
            );
            f.render_widget(render.prompt.clone(), chunks[1]);
        })?;
        Ok(())
    }

    fn poll(&mut self, timeout: std::time::Duration) -> io::Result<bool> {
        event::poll(timeout)
    }

    fn read(&mut self) -> io::Result<Event> {
        event::read()
    }

    fn leave_alternate_screen(&mut self) -> io::Result<()> {
        crossterm::execute!(io::stdout(), LeaveAlternateScreen)
    }

    fn disable_raw_mode(&mut self) -> io::Result<()> {
        terminal::disable_raw_mode()
    }
}

struct TrustPromptTerminalGuard<'a, O: TrustPromptTerminalOps> {
    ops: &'a mut O,
    raw_mode_enabled: bool,
    alternate_screen_entered: bool,
}

impl<'a, O: TrustPromptTerminalOps> TrustPromptTerminalGuard<'a, O> {
    fn enter(ops: &'a mut O) -> io::Result<Self> {
        let mut guard = Self {
            ops,
            raw_mode_enabled: false,
            alternate_screen_entered: false,
        };
        guard.ops.enable_raw_mode()?;
        guard.raw_mode_enabled = true;
        guard.ops.enter_alternate_screen()?;
        guard.alternate_screen_entered = true;
        guard.ops.construct_terminal()?;
        Ok(guard)
    }
}

impl<O: TrustPromptTerminalOps> Drop for TrustPromptTerminalGuard<'_, O> {
    fn drop(&mut self) {
        if self.alternate_screen_entered {
            let _ = self.ops.leave_alternate_screen();
        }
        if self.raw_mode_enabled {
            let _ = self.ops.disable_raw_mode();
        }
    }
}

fn run_trust_prompt_terminal_with_ops<O: TrustPromptTerminalOps>(
    project_path: &Path,
    response_tx: tokio::sync::oneshot::Sender<TrustChoice>,
    ops: &mut O,
) -> io::Result<()> {
    // Construct the AwaitingTrust payload (the oneshot is the transport, not
    // cosmetic state) and project its status for the status bar.
    let app_state = AppState::AwaitingTrust(AwaitingTrustState {
        project_path: project_path.to_path_buf(),
        response_tx,
    });
    let status = app_state.status();
    let AppState::AwaitingTrust(awaiting) = app_state else {
        return Ok(());
    };
    let mut response_tx = Some(awaiting.response_tx);
    let project_display = project_path.to_string_lossy().to_string();
    let has_parent = std::fs::canonicalize(project_path)
        .ok()
        .is_some_and(|canonical| canonical.parent().is_some());
    let mut prompt = if has_parent {
        TrustPrompt::new()
    } else {
        TrustPrompt::without_parent()
    };
    let terminal = TrustPromptTerminalGuard::enter(ops)?;
    loop {
        terminal.ops.draw(&TrustPromptRender {
            project_display: &project_display,
            status,
            prompt: &prompt,
        })?;
        if !terminal.ops.poll(std::time::Duration::from_millis(50))? {
            continue;
        }
        if let Event::Key(key) = terminal.ops.read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Down | KeyCode::Char('j') => prompt.move_next(),
                KeyCode::Up | KeyCode::Char('k') => prompt.move_prev(),
                KeyCode::Enter => {
                    if let Some(tx) = response_tx.take() {
                        let _ = tx.send(prompt.selected());
                    }
                    break Ok(());
                }
                KeyCode::Esc => break Ok(()), // drop sender -> receiver yields None (cancel)
                KeyCode::Char(c @ '1'..='5') => {
                    if let Some(tc) = prompt.choice_at((c as u8 - b'1') as usize) {
                        if let Some(tx) = response_tx.take() {
                            let _ = tx.send(tc);
                        }
                        break Ok(());
                    }
                }
                _ => {}
            }
        }
    }
}

/// Production [`InteractiveTrustPrompt`] backed by [`run_trust_prompt`].
pub struct TuiTrustPrompt;

impl InteractiveTrustPrompt for TuiTrustPrompt {
    fn ask(
        &mut self,
        project_path: &Path,
    ) -> Pin<Box<dyn Future<Output = Option<TrustChoice>> + Send + '_>> {
        let path = project_path.to_path_buf();
        Box::pin(async move { run_trust_prompt(&path).await.ok().flatten() })
    }
}

// ---------------------------------------------------------------------------
// Interactive capability-permission broker and prompt link
// ---------------------------------------------------------------------------

/// One prompt request flowing from the routed bash backend (agent task) to the
/// interactive event loop: the redaction-safe identity summary and the oneshot
/// responder that receives the user's [`PermissionChoice`].
pub struct PermissionPromptRequest {
    pub summary: PermissionSummary,
    pub responder: oneshot::Sender<PermissionChoice>,
}

/// A pending permission prompt held in [`TuiState`]: the selection widget (with
/// its cursor) and the oneshot responder that carries the user's choice back to
/// the broker (and thus to the waiting tool call).
struct PendingPermission {
    prompt: PermissionPrompt,
    responder: oneshot::Sender<PermissionChoice>,
}

/// Production [`InteractivePermissionBroker`] backed by the interactive TUI.
///
/// The broker does NOT hold `TuiState` directly (it is constructed before the
/// state exists). It talks to the event loop over an mpsc channel: each
/// `resolve_ask` sends a [`PermissionPromptRequest`] (carrying its own oneshot
/// responder) and awaits the choice. The event loop receives the request, sets
/// `TuiState::pending_permission`, renders the [`PermissionPrompt`] widget, and
/// relays the user's choice through the responder.
///
/// Cancellation/drop safety: a closed
/// loop (terminal close) drops the receiver so `send` fails →
/// [`PermissionChoice::Deny`]; a dropped responder likewise resolves the await
/// to `Deny` via `unwrap_or`. The tool call therefore always surfaces a stable
/// `permission_denied` (or proceeds on allow); it never panics or hangs on a
/// closed/cancelled prompt. A run cancellation drops the in-flight tool future
/// (standard task drop), so abort does not strand the broker either.
pub struct TuiPermissionBroker {
    tx: mpsc::Sender<PermissionPromptRequest>,
}

impl TuiPermissionBroker {
    pub fn new(tx: mpsc::Sender<PermissionPromptRequest>) -> Self {
        Self { tx }
    }
}

impl InteractivePermissionBroker for TuiPermissionBroker {
    fn resolve_ask(
        &self,
        summary: PermissionSummary,
    ) -> Pin<Box<dyn Future<Output = PermissionChoice> + Send + '_>> {
        let tx = self.tx.clone();
        Box::pin(async move {
            let (responder, response) = oneshot::channel();
            let request = PermissionPromptRequest { summary, responder };
            // A closed loop (terminal close / shutdown) drops the receiver; treat
            // that as Deny so the tool call fails closed with permission_denied.
            if tx.send(request).await.is_err() {
                return PermissionChoice::Deny;
            }
            // A dropped responder (loop accepted the request then closed) -> Deny.
            response.await.unwrap_or(PermissionChoice::Deny)
        })
    }
}

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
    /// A pending mid-execution capability-permission prompt. When
    /// `Some`, the event loop renders the [`PermissionPrompt`] overlay and
    /// captures the user's choice; `None` otherwise.
    pending_permission: Option<PendingPermission>,
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

/// Capture produced by the hidden, headless interactive entry-point driver.
///
/// The driver is an integration-test seam: it cannot be selected by terminal
/// input, configuration, or environment variables.
#[doc(hidden)]
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct InteractiveTuiTestCapture {
    pub user_messages: usize,
    pub provider_calls: usize,
    pub retries: usize,
    pub presenter_constructions: usize,
    pub system_messages: Vec<String>,
    pub terminal_transitions: Vec<String>,
}

#[derive(Default)]
struct InteractiveTuiTestObserver {
    user_messages: AtomicU64,
    provider_calls: AtomicU64,
    retries: AtomicU64,
    presenter_constructions: AtomicU64,
}

/// Hidden login dependencies for a scripted outer-TUI test.
#[doc(hidden)]
#[derive(Clone)]
pub struct InteractiveTuiTestAuthServices {
    presenter: Arc<dyn opi_ai::auth::LoginPresenter>,
    client: reqwest::Client,
    endpoint_base_url: String,
    login_timeout: std::time::Duration,
    codex_device_timeout: std::time::Duration,
    terminal_failure: InteractiveTuiTestTerminalFailure,
}

impl InteractiveTuiTestAuthServices {
    #[doc(hidden)]
    pub fn new(
        presenter: Arc<dyn opi_ai::auth::LoginPresenter>,
        client: reqwest::Client,
        endpoint_base_url: String,
        login_timeout: std::time::Duration,
        codex_device_timeout: std::time::Duration,
    ) -> Self {
        Self {
            presenter,
            client,
            endpoint_base_url,
            login_timeout,
            codex_device_timeout,
            terminal_failure: InteractiveTuiTestTerminalFailure::None,
        }
    }

    #[doc(hidden)]
    pub fn with_terminal_failure(mut self, failure: InteractiveTuiTestTerminalFailure) -> Self {
        self.terminal_failure = failure;
        self
    }
}

/// Terminal failure injected into the hidden scripted outer-TUI adapter.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InteractiveTuiTestTerminalFailure {
    #[default]
    None,
    Suspend,
    Resume,
}

#[derive(Clone)]
struct InteractiveTuiTestDriver {
    id: u64,
    inputs: Vec<String>,
    capture: Arc<Mutex<InteractiveTuiTestCapture>>,
    auth: Option<InteractiveTuiTestAuthServices>,
    abort_readiness: Option<Arc<tokio::sync::Semaphore>>,
}

static INTERACTIVE_TUI_TEST_DRIVER: OnceLock<Mutex<Option<InteractiveTuiTestDriver>>> =
    OnceLock::new();
static NEXT_INTERACTIVE_TUI_TEST_DRIVER_ID: AtomicU64 = AtomicU64::new(1);
static TUI_LOGIN_PRESENTER_CONSTRUCTIONS: AtomicU64 = AtomicU64::new(0);

fn interactive_tui_test_driver_slot() -> &'static Mutex<Option<InteractiveTuiTestDriver>> {
    INTERACTIVE_TUI_TEST_DRIVER.get_or_init(|| Mutex::new(None))
}

/// RAII guard for the hidden headless TUI driver.
#[doc(hidden)]
pub struct InteractiveTuiTestDriverGuard {
    id: u64,
    capture: Arc<Mutex<InteractiveTuiTestCapture>>,
}

impl InteractiveTuiTestDriverGuard {
    #[doc(hidden)]
    pub fn capture(&self) -> InteractiveTuiTestCapture {
        self.capture
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

impl Drop for InteractiveTuiTestDriverGuard {
    fn drop(&mut self) {
        let mut slot = interactive_tui_test_driver_slot()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if slot.as_ref().is_some_and(|driver| driver.id == self.id) {
            *slot = None;
        }
    }
}

/// Install one RAII-scoped, race-safe headless script for `run_interactive_tui`.
/// The reserved input `"<escape>"` emits an Escape key without a trailing
/// Enter, allowing permission-denial and modal-cancellation paths to be driven.
/// `"<abort>"` emits the configured default Escape abort after the next real
/// provider turn starts, including while ordinary prompt input is disabled.
#[doc(hidden)]
pub fn install_interactive_tui_test_driver<I, S>(
    inputs: I,
) -> io::Result<InteractiveTuiTestDriverGuard>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    install_interactive_tui_test_driver_inner(inputs, None, None)
}

/// Install a scripted outer-TUI driver whose `"<abort>"` inputs wait for
/// fixture-owned provider readiness instead of the earlier `TurnStart` event.
#[doc(hidden)]
pub fn install_interactive_tui_test_driver_with_abort_readiness<I, S>(
    inputs: I,
    abort_readiness: Arc<tokio::sync::Semaphore>,
) -> io::Result<InteractiveTuiTestDriverGuard>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    install_interactive_tui_test_driver_inner(inputs, None, Some(abort_readiness))
}

/// Install a scripted outer-TUI driver with injected login dependencies.
#[doc(hidden)]
pub fn install_interactive_tui_test_driver_with_auth<I, S>(
    inputs: I,
    auth: InteractiveTuiTestAuthServices,
) -> io::Result<InteractiveTuiTestDriverGuard>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    install_interactive_tui_test_driver_inner(inputs, Some(auth), None)
}

fn install_interactive_tui_test_driver_inner<I, S>(
    inputs: I,
    auth: Option<InteractiveTuiTestAuthServices>,
    abort_readiness: Option<Arc<tokio::sync::Semaphore>>,
) -> io::Result<InteractiveTuiTestDriverGuard>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let id = NEXT_INTERACTIVE_TUI_TEST_DRIVER_ID.fetch_add(1, Ordering::Relaxed);
    let capture = Arc::new(Mutex::new(InteractiveTuiTestCapture::default()));
    let driver = InteractiveTuiTestDriver {
        id,
        inputs: inputs.into_iter().map(Into::into).collect(),
        capture: capture.clone(),
        auth,
        abort_readiness,
    };
    let mut slot = interactive_tui_test_driver_slot()
        .lock()
        .map_err(|_| io::Error::other("headless TUI driver lock poisoned"))?;
    if slot.is_some() {
        return Err(io::Error::other(
            "a headless TUI driver is already installed",
        ));
    }
    *slot = Some(driver);
    Ok(InteractiveTuiTestDriverGuard { id, capture })
}

fn active_interactive_tui_test_driver() -> Option<InteractiveTuiTestDriver> {
    interactive_tui_test_driver_slot()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

/// Reset the hidden production-presenter construction probe.
#[doc(hidden)]
pub fn reset_interactive_tui_presenter_construction_count() {
    TUI_LOGIN_PRESENTER_CONSTRUCTIONS.store(0, Ordering::SeqCst);
}

/// Read the hidden production-presenter construction probe.
#[doc(hidden)]
pub fn interactive_tui_presenter_construction_count() -> usize {
    TUI_LOGIN_PRESENTER_CONSTRUCTIONS.load(Ordering::SeqCst) as usize
}

struct PendingPrompt {
    handle: tokio::task::JoinHandle<Result<Vec<AgentMessage>, AgentError>>,
    may_arm_retry: bool,
}

struct PromptAuthStateMachine {
    pending: Option<PendingPrompt>,
    pending_auth_provider: Option<String>,
    output_began: Arc<AtomicBool>,
    test_observer: Option<Arc<InteractiveTuiTestObserver>>,
}

enum PromptCompletion {
    Success,
    Cancelled,
    CredentialNeeded { provider_id: String },
    Error(String),
}

impl PromptAuthStateMachine {
    fn new(output_began: Arc<AtomicBool>) -> Self {
        Self {
            pending: None,
            pending_auth_provider: None,
            output_began,
            test_observer: None,
        }
    }

    fn with_test_observer(mut self, observer: Arc<InteractiveTuiTestObserver>) -> Self {
        self.test_observer = Some(observer);
        self
    }

    fn is_running(&self) -> bool {
        self.pending.is_some()
    }

    fn submit_prompt(&mut self, input: String, harness: Arc<tokio::sync::Mutex<CodingHarness>>) {
        self.pending_auth_provider = None;
        self.output_began.store(false, Ordering::SeqCst);
        if let Some(observer) = &self.test_observer {
            observer.user_messages.fetch_add(1, Ordering::SeqCst);
        }
        self.pending = Some(PendingPrompt {
            may_arm_retry: true,
            handle: tokio::spawn(async move {
                let mut harness = harness.lock().await;
                let pending = harness.take_pending_images();
                if pending.is_empty() {
                    harness.prompt(&input).await
                } else {
                    let mut content = vec![opi_ai::message::InputContent::Text { text: input }];
                    content.extend(pending);
                    harness.prompt_with_content(content).await
                }
            }),
        });
    }

    fn route_auth_outcome(
        &mut self,
        outcome: AuthCommandOutcome,
        harness: Arc<tokio::sync::Mutex<CodingHarness>>,
    ) -> io::Result<Option<String>> {
        let retry_provider = match &outcome {
            AuthCommandOutcome::LoggedIn { provider_id }
                if self.pending_auth_provider.as_deref() == Some(provider_id.as_str()) =>
            {
                Some(provider_id.clone())
            }
            AuthCommandOutcome::LoggedIn { .. } | AuthCommandOutcome::Failed { .. } => {
                self.pending_auth_provider = None;
                None
            }
            _ => None,
        };
        let message = route_auth_outcome(outcome)?;
        if retry_provider.is_some() {
            self.pending_auth_provider = None;
            self.output_began.store(false, Ordering::SeqCst);
            if let Some(observer) = &self.test_observer {
                observer.retries.fetch_add(1, Ordering::SeqCst);
            }
            self.pending = Some(PendingPrompt {
                may_arm_retry: false,
                handle: tokio::spawn(async move {
                    let mut harness = harness.lock().await;
                    harness.retry_last_prompt().await
                }),
            });
        }
        Ok(message)
    }

    async fn poll_completion(&mut self) -> Option<PromptCompletion> {
        if !self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.handle.is_finished())
        {
            return None;
        }
        self.finish_pending().await
    }

    async fn await_completion(&mut self) -> Option<PromptCompletion> {
        self.finish_pending().await
    }

    async fn finish_pending(&mut self) -> Option<PromptCompletion> {
        let pending = self.pending.take()?;
        let result = pending.handle.await;
        let output_began = self.output_began.load(Ordering::SeqCst);
        let completion = match result {
            Ok(Ok(_)) => PromptCompletion::Success,
            Ok(Err(AgentError::Cancelled)) => PromptCompletion::Cancelled,
            Ok(Err(error)) => match provider_auth_failure(&error) {
                Some(
                    ProviderAuthFailure::CredentialNeeded(provider_id)
                    | ProviderAuthFailure::AccountIdMissing(provider_id),
                ) if pending.may_arm_retry && !output_began => {
                    self.pending_auth_provider = Some(provider_id.to_owned());
                    PromptCompletion::CredentialNeeded {
                        provider_id: provider_id.to_owned(),
                    }
                }
                Some(_) | None => {
                    self.pending_auth_provider = None;
                    PromptCompletion::Error(error.to_string())
                }
            },
            Err(error) => {
                self.pending_auth_provider = None;
                PromptCompletion::Error(error.to_string())
            }
        };
        Some(completion)
    }
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

enum TuiEventPoll {
    Event(Event),
    Pending,
    Exhausted,
}

trait TuiEventSource {
    fn poll_event(
        &mut self,
        timeout: std::time::Duration,
        input_enabled: bool,
    ) -> io::Result<TuiEventPoll>;
}

struct CrosstermEventSource;

impl TuiEventSource for CrosstermEventSource {
    fn poll_event(
        &mut self,
        timeout: std::time::Duration,
        _input_enabled: bool,
    ) -> io::Result<TuiEventPoll> {
        if event::poll(timeout)? {
            Ok(TuiEventPoll::Event(event::read()?))
        } else {
            Ok(TuiEventPoll::Pending)
        }
    }
}

trait TuiTerminal: LoginTerminalControl {
    fn draw_state(&mut self, state: &TuiState) -> io::Result<()>;
}

impl TuiTerminal for Terminal<CrosstermBackend<io::Stdout>> {
    fn draw_state(&mut self, state: &TuiState) -> io::Result<()> {
        self.draw(|frame| render_tui_state(frame, state))?;
        Ok(())
    }
}

fn render_tui_state(frame: &mut Frame<'_>, state: &TuiState) {
    frame.render_widget(build_shell(state), frame.area());
    // Render the modal permission prompt as a centered overlay on
    // top of the shell when a prompt is pending.
    if let Some(pending) = &state.pending_permission {
        let area = centered_rect(70, 50, frame.area());
        frame.render_widget(ratatui::widgets::Clear, area);
        frame.render_widget(pending.prompt.clone(), area);
    }
}

fn format_startup_diagnostic(payload: &DiagnosticPayload) -> String {
    let granular = payload
        .details
        .as_ref()
        .and_then(|details| details.get("code"))
        .and_then(serde_json::Value::as_str)
        .filter(|code| !code.is_empty());
    let action = payload
        .action
        .as_deref()
        .map(|action| format!(" (action: {action})"))
        .unwrap_or_default();
    match granular {
        Some(code) => format!(
            "[{}] {}::{code}: {}{action}",
            payload.severity, payload.source, payload.message
        ),
        None => payload.to_string(),
    }
}

fn startup_diagnostic_messages(harness: &CodingHarness) -> Vec<TuiMessage> {
    harness
        .resource_metadata()
        .diagnostic_payloads(RedactionMode::Summary)
        .iter()
        .map(|payload| TuiMessage::new(TuiRole::System, format_startup_diagnostic(payload)))
        .collect()
}

/// Run the outer interactive TUI and its prompt/auth state machine.
///
/// A pre-output `CredentialNeeded` retains one pending turn. A successful
/// explicit login for that same provider retries the turn exactly once without
/// appending a duplicate user message. Different-provider login and every
/// cancellation, presenter, OAuth, store, terminal, or mid-stream revocation
/// failure perform no retry.
pub async fn run_interactive_tui(
    harness: CodingHarness,
    model: String,
    theme_name: &str,
    keybindings: Keybindings,
) -> Result<(), Box<dyn std::error::Error>> {
    let theme = resolve_interactive_theme(&harness, theme_name);
    // Startup diagnostics are projected once into the initial state before the
    // first frame. Later redraws reuse that state instead of re-reading metadata,
    // preserving order without duplicating refusals.
    let initial_messages = startup_diagnostic_messages(&harness);
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
        messages: initial_messages,
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
        pending_permission: None,
    }));

    // Wire agent events into shared state before wrapping harness.
    let output_began = Arc::new(AtomicBool::new(false));
    let callback_output_began = output_began.clone();
    let test_driver = active_interactive_tui_test_driver();
    let test_observer = test_driver
        .as_ref()
        .map(|_| Arc::new(InteractiveTuiTestObserver::default()));
    let callback_test_observer = test_observer.clone();
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
                callback_output_began.store(true, Ordering::SeqCst);
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
                diagnostics,
                ..
            } => {
                let formatted_diagnostics: Vec<String> = diagnostics
                    .iter()
                    .map(crate::diagnostic_bridge::format_tool_diagnostic)
                    .collect();
                for diagnostic in &formatted_diagnostics {
                    s.messages
                        .push(TuiMessage::new(TuiRole::System, diagnostic.clone()));
                }
                if tool_name == "bash"
                    && let Some(details) = details
                    && let Some(contract) = crate::tool::format_effective_contract(details)
                {
                    s.messages.push(TuiMessage::new(TuiRole::System, contract));
                }
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
                        ToolCallStatus::Error(
                            formatted_diagnostics
                                .first()
                                .cloned()
                                .unwrap_or_else(|| "tool execution failed".to_string()),
                        )
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
                if let Some(observer) = &callback_test_observer {
                    observer.provider_calls.fetch_add(1, Ordering::SeqCst);
                }
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

    // Capture the authentication services before the harness moves into
    // Arc<Mutex>; the dispatcher constructs the concrete registry per command.
    let credential_store = harness
        .credential_store
        .take()
        .ok_or_else(|| io::Error::other("interactive credential store is not configured"))?;
    let oauth = OAuthDispatchRuntime {
        endpoints: harness.oauth_endpoints.clone(),
        client: harness.oauth_http_client.clone(),
    };

    // Take the permission-prompt channel receiver out of the
    // harness before it is shared, so the event loop can drain it each frame.
    let mut permission_prompt_rx = harness.permission_prompt_rx.take();

    let harness = Arc::new(tokio::sync::Mutex::new(harness));

    if let Some(driver) = test_driver {
        return run_headless_interactive_tui_driver(
            driver,
            &harness,
            &state,
            credential_store,
            oauth,
            output_began,
            test_observer.expect("headless TUI driver has an observer"),
            &mut permission_prompt_rx,
        )
        .await;
    }

    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut events = CrosstermEventSource;

    // Main TUI loop
    let result = tui_event_loop(
        &mut terminal,
        &mut events,
        &harness,
        &state,
        &mut permission_prompt_rx,
        TuiLoopRuntime {
            credential_store,
            oauth,
            output_began,
            test_auth: None,
            test_observer: None,
        },
    )
    .await;

    // Restore terminal
    terminal::disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

struct TuiLoopRuntime<'a> {
    credential_store: Arc<crate::credential_store::KeychainCredentialStore>,
    oauth: OAuthDispatchRuntime,
    output_began: Arc<AtomicBool>,
    test_auth: Option<&'a InteractiveTuiTestAuthServices>,
    test_observer: Option<Arc<InteractiveTuiTestObserver>>,
}

async fn tui_event_loop<T: TuiTerminal, E: TuiEventSource>(
    terminal: &mut T,
    events: &mut E,
    harness: &Arc<tokio::sync::Mutex<CodingHarness>>,
    state: &Arc<Mutex<TuiState>>,
    permission_prompt_rx: &mut Option<mpsc::Receiver<PermissionPromptRequest>>,
    runtime: TuiLoopRuntime<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let TuiLoopRuntime {
        credential_store,
        oauth,
        output_began,
        test_auth,
        test_observer,
    } = runtime;
    let mut interaction = PromptAuthStateMachine::new(output_began);
    let test_presenter_observer = test_observer.clone();
    if let Some(observer) = test_observer {
        interaction = interaction.with_test_observer(observer);
    }
    let mut cancel_token = harness.lock().await.cancel_token();

    loop {
        // Drain any pending capability-permission prompt from the
        // routed bash backend (agent task) into the TUI state. At most one is
        // pending at a time (BashTool is Sequential), so the last request wins.
        if let Some(rx) = permission_prompt_rx.as_mut() {
            while let Ok(req) = rx.try_recv() {
                let mut s = state.lock().unwrap();
                s.pending_permission = Some(PendingPermission {
                    prompt: PermissionPrompt::new(req.summary),
                    responder: req.responder,
                });
            }
        }

        // Render current state
        {
            let s = state.lock().unwrap();
            terminal.draw_state(&s)?;
        }

        // Check if the shared prompt/auth state machine finished a turn.
        if let Some(completion) = interaction.poll_completion().await {
            apply_prompt_completion_to_tui(completion, state)?;

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

            // Arm the next run's cancellation generation after completion; the
            // next prompt consumes this exact token before awaited preflight.
            cancel_token = harness.lock().await.cancel_token();
        }

        // Poll for terminal events (non-blocking with timeout). A pending
        // permission prompt is modal and must receive keys even while a turn is
        // running, so deliver events when the agent is idle OR a prompt awaits.
        let permission_pending = state.lock().unwrap().pending_permission.is_some();
        let polled = events.poll_event(
            std::time::Duration::from_millis(50),
            !interaction.is_running() || permission_pending,
        )?;
        if matches!(polled, TuiEventPoll::Exhausted) {
            return Err(io::Error::other("scripted TUI input ended without exit").into());
        }
        if matches!(polled, TuiEventPoll::Pending) {
            tokio::task::yield_now().await;
        }
        if let TuiEventPoll::Event(Event::Key(key)) = polled {
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
                            h.set_model_validated(model)
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

            // A pending capability-permission prompt is modal — it
            // captures all keys (arrows/enter/esc resolve it; others are
            // ignored) and never falls through to submit/abort/text input. Esc
            // resolves to Deny (NOT the run-abort path).
            let permission_consumed = {
                let mut s = state.lock().unwrap();
                if s.pending_permission.is_some() {
                    handle_permission_key(&mut s, key.code);
                    true
                } else {
                    false
                }
            };
            if permission_consumed {
                continue;
            }

            if matches_key_combo(key.code, key.modifiers, &kb.submit) {
                // Ignore submit while agent is running
                if interaction.is_running() {
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
                    if interaction.is_running() {
                        cancel_token.cancel();
                        let _ = interaction.await_completion().await;
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

                // Dispatch /name, /label, /unlabel, and /session info
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

                if is_auth_command(&input) {
                    let outcome = if let Some(auth) = test_auth {
                        if auth_command_requires_presenter(&input) {
                            record_tui_login_presenter_construction();
                            if let Some(observer) = &test_presenter_observer {
                                observer
                                    .presenter_constructions
                                    .fetch_add(1, Ordering::SeqCst);
                            }
                        }
                        dispatch_auth_command(
                            &input,
                            terminal,
                            AuthCommandServices::with_test_services(
                                credential_store.as_ref(),
                                auth.presenter.as_ref(),
                                auth.client.clone(),
                                auth.endpoint_base_url.clone(),
                                auth.login_timeout,
                                auth.codex_device_timeout,
                            ),
                        )
                        .await
                    } else {
                        let presenter = new_tui_login_presenter();
                        dispatch_interactive_auth_input(
                            &input,
                            terminal,
                            credential_store.as_ref(),
                            &oauth,
                            &presenter,
                        )
                        .await
                    };
                    let routed = interaction
                        .route_auth_outcome(outcome, harness.clone())?
                        .expect("classified auth command must be routed");
                    let mut s = state
                        .lock()
                        .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
                    s.messages.push(TuiMessage::new(TuiRole::System, routed));
                    if interaction.is_running() {
                        s.app_state = AppState::Thinking;
                    }
                    continue;
                }

                // Add user message to display
                {
                    let mut s = state.lock().unwrap();
                    s.messages
                        .push(TuiMessage::new(TuiRole::User, input.clone()));
                    s.app_state = AppState::Thinking;
                }

                interaction.submit_prompt(input, harness.clone());
            } else if matches_key_combo(key.code, key.modifiers, &kb.abort) {
                if interaction.is_running() {
                    cancel_token.cancel();
                } else {
                    return Ok(());
                }
            } else if matches_key_combo(key.code, key.modifiers, &kb.new_line) {
                if !interaction.is_running() {
                    state.lock().unwrap().input_text.push('\n');
                }
            } else {
                match key.code {
                    KeyCode::Char(c) if !interaction.is_running() => {
                        state.lock().unwrap().input_text.push(c);
                    }
                    KeyCode::Backspace if !interaction.is_running() => {
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

fn apply_prompt_completion_to_tui(
    completion: PromptCompletion,
    state: &Arc<Mutex<TuiState>>,
) -> io::Result<()> {
    let mut state = state
        .lock()
        .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
    match completion {
        PromptCompletion::Success | PromptCompletion::Cancelled => {}
        PromptCompletion::CredentialNeeded { provider_id } => {
            state.messages.push(TuiMessage::new(
                TuiRole::System,
                format!("[credential needed for '{provider_id}': run /login {provider_id}]"),
            ));
        }
        PromptCompletion::Error(error) => state
            .messages
            .push(TuiMessage::new(TuiRole::System, format!("error: {error}"))),
    }
    state.app_state = AppState::Idle;
    Ok(())
}

fn new_tui_login_presenter() -> TuiLoginPresenter {
    record_tui_login_presenter_construction();
    TuiLoginPresenter::new()
}

fn record_tui_login_presenter_construction() {
    TUI_LOGIN_PRESENTER_CONSTRUCTIONS.fetch_add(1, Ordering::SeqCst);
}

#[derive(Clone)]
struct OAuthDispatchRuntime {
    endpoints: OAuthEndpointConfig,
    client: reqwest::Client,
}

async fn dispatch_interactive_auth_input<T: LoginTerminalControl>(
    input: &str,
    terminal: &mut T,
    credential_store: &crate::credential_store::KeychainCredentialStore,
    oauth: &OAuthDispatchRuntime,
    presenter: &dyn opi_ai::auth::LoginPresenter,
) -> AuthCommandOutcome {
    dispatch_auth_command(
        input,
        terminal,
        AuthCommandServices::new(
            credential_store,
            presenter,
            oauth.endpoints.clone(),
            oauth.client.clone(),
        ),
    )
    .await
}

struct HeadlessTuiTerminal {
    terminal: Terminal<TestBackend>,
    capture: Arc<Mutex<InteractiveTuiTestCapture>>,
    failure: InteractiveTuiTestTerminalFailure,
}

impl TuiTerminal for HeadlessTuiTerminal {
    fn draw_state(&mut self, state: &TuiState) -> io::Result<()> {
        match self.terminal.draw(|frame| render_tui_state(frame, state)) {
            Ok(_) => Ok(()),
            Err(error) => match error {},
        }
    }
}

impl LoginTerminalControl for HeadlessTuiTerminal {
    fn suspend_for_login(&mut self) -> io::Result<()> {
        self.capture
            .lock()
            .map_err(|_| io::Error::other("headless TUI capture lock poisoned"))?
            .terminal_transitions
            .push("suspend".into());
        match self.failure {
            InteractiveTuiTestTerminalFailure::Suspend => {
                Err(io::Error::other("injected terminal suspension failure"))
            }
            _ => Ok(()),
        }
    }

    fn resume_after_login(&mut self) -> io::Result<()> {
        self.capture
            .lock()
            .map_err(|_| io::Error::other("headless TUI capture lock poisoned"))?
            .terminal_transitions
            .push("resume".into());
        match self.failure {
            InteractiveTuiTestTerminalFailure::Resume => {
                Err(io::Error::other("injected terminal restoration failure"))
            }
            _ => Ok(()),
        }
    }
}

struct ScriptedTuiEvent {
    event: Event,
    required_provider_calls: Option<usize>,
}

struct ScriptedTuiEventSource {
    events: VecDeque<ScriptedTuiEvent>,
    observer: Arc<InteractiveTuiTestObserver>,
    abort_readiness: Option<Arc<tokio::sync::Semaphore>>,
}

impl ScriptedTuiEventSource {
    fn new(
        inputs: Vec<String>,
        observer: Arc<InteractiveTuiTestObserver>,
        abort_readiness: Option<Arc<tokio::sync::Semaphore>>,
    ) -> Self {
        let mut events = VecDeque::new();
        let mut abort_count = 0;
        for input in inputs {
            if input == "<escape>" {
                events.push_back(ScriptedTuiEvent {
                    event: Event::Key(crossterm::event::KeyEvent::new(
                        KeyCode::Esc,
                        KeyModifiers::NONE,
                    )),
                    required_provider_calls: None,
                });
                continue;
            }
            if input == "<abort>" {
                abort_count += 1;
                events.push_back(ScriptedTuiEvent {
                    event: Event::Key(crossterm::event::KeyEvent::new(
                        KeyCode::Esc,
                        KeyModifiers::NONE,
                    )),
                    required_provider_calls: Some(abort_count),
                });
                continue;
            }
            events.extend(input.chars().map(|character| ScriptedTuiEvent {
                event: Event::Key(crossterm::event::KeyEvent::new(
                    KeyCode::Char(character),
                    KeyModifiers::NONE,
                )),
                required_provider_calls: None,
            }));
            events.push_back(ScriptedTuiEvent {
                event: Event::Key(crossterm::event::KeyEvent::new(
                    KeyCode::Enter,
                    KeyModifiers::NONE,
                )),
                required_provider_calls: None,
            });
        }
        Self {
            events,
            observer,
            abort_readiness,
        }
    }
}

impl TuiEventSource for ScriptedTuiEventSource {
    fn poll_event(
        &mut self,
        _timeout: std::time::Duration,
        input_enabled: bool,
    ) -> io::Result<TuiEventPoll> {
        let Some(next) = self.events.front() else {
            return Ok(TuiEventPoll::Exhausted);
        };
        let provider_gate_open = next.required_provider_calls.is_some_and(|required| {
            self.abort_readiness.as_ref().map_or_else(
                || self.observer.provider_calls.load(Ordering::SeqCst) >= required as u64,
                |readiness| readiness.available_permits() >= required,
            )
        });
        if !input_enabled && !provider_gate_open {
            return Ok(TuiEventPoll::Pending);
        }
        Ok(self
            .events
            .pop_front()
            .map_or(TuiEventPoll::Exhausted, |scripted| {
                TuiEventPoll::Event(scripted.event)
            }))
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_headless_interactive_tui_driver(
    driver: InteractiveTuiTestDriver,
    harness: &Arc<tokio::sync::Mutex<CodingHarness>>,
    state: &Arc<Mutex<TuiState>>,
    credential_store: Arc<crate::credential_store::KeychainCredentialStore>,
    oauth: OAuthDispatchRuntime,
    output_began: Arc<AtomicBool>,
    test_observer: Arc<InteractiveTuiTestObserver>,
    permission_prompt_rx: &mut Option<mpsc::Receiver<PermissionPromptRequest>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let InteractiveTuiTestDriver {
        inputs,
        capture,
        auth,
        abort_readiness,
        ..
    } = driver;
    let mut terminal = HeadlessTuiTerminal {
        terminal: Terminal::new(TestBackend::new(100, 30))?,
        capture: capture.clone(),
        failure: auth
            .as_ref()
            .map_or(InteractiveTuiTestTerminalFailure::None, |auth| {
                auth.terminal_failure
            }),
    };
    let mut events = ScriptedTuiEventSource::new(inputs, test_observer.clone(), abort_readiness);
    let result = tui_event_loop(
        &mut terminal,
        &mut events,
        harness,
        state,
        permission_prompt_rx,
        TuiLoopRuntime {
            credential_store,
            oauth,
            output_began,
            test_auth: auth.as_ref(),
            test_observer: Some(test_observer.clone()),
        },
    )
    .await;

    let system_messages = state
        .lock()
        .map_err(|_| io::Error::other("TUI state lock poisoned"))?
        .messages
        .iter()
        .filter(|message| message.role == TuiRole::System)
        .map(|message| message.content.clone())
        .collect();
    let mut capture = capture
        .lock()
        .map_err(|_| io::Error::other("headless TUI capture lock poisoned"))?;
    capture.user_messages = test_observer.user_messages.load(Ordering::SeqCst) as usize;
    capture.provider_calls = test_observer.provider_calls.load(Ordering::SeqCst) as usize;
    capture.retries = test_observer.retries.load(Ordering::SeqCst) as usize;
    capture.presenter_constructions =
        test_observer.presenter_constructions.load(Ordering::SeqCst) as usize;
    capture.system_messages = system_messages;
    drop(capture);
    result
}

fn build_shell(s: &TuiState) -> Shell {
    let mut shell = Shell::new(s.model.clone())
        .input_text(s.input_text.clone())
        .state(if s.pending_permission.is_some() {
            AppStatus::AwaitingPermission
        } else {
            s.app_state.status()
        })
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

/// A parsed session-metadata slash command. Returned
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

/// Parse a session-metadata slash command from raw TUI input.
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

/// Apply a parsed session-metadata command against the harness
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

/// Resolve a pending capability-permission prompt from a key press. The prompt
/// is modal: `Esc` denies, `Enter` confirms the highlighted
/// choice, `Up`/`Down` move the cursor; any other key is ignored. On `Esc`/
/// `Enter` the user's [`PermissionChoice`] is sent through the responder and
/// the prompt is cleared; the broker then resumes the waiting tool call (allow
/// dispatches, deny surfaces `permission_denied`). A dropped responder (broker
/// gone) is silently ignored — the broker already resolved to `Deny`.
fn handle_permission_key(s: &mut TuiState, code: KeyCode) {
    let choice = match s.pending_permission.as_mut() {
        Some(pending) => match code {
            KeyCode::Esc => Some(PermissionChoice::Deny),
            KeyCode::Enter => Some(pending.prompt.selected()),
            KeyCode::Down => {
                pending.prompt.move_next();
                None
            }
            KeyCode::Up => {
                pending.prompt.move_prev();
                None
            }
            _ => None,
        },
        None => None,
    };
    if let Some(choice) = choice
        && let Some(pending) = s.pending_permission.take()
    {
        let _ = pending.responder.send(choice);
    }
}

/// Centered overlay rect (standard ratatui helper) for modal widgets.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(percent_y),
            Constraint::Percentage(100 - percent_y),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(percent_x),
            Constraint::Percentage(100 - percent_x),
        ])
        .split(popup[0])[0]
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

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum TrustPromptFailure {
        EnterAlternateScreen,
        ConstructTerminal,
        Draw,
        Poll,
        Read,
    }

    struct RecordingTrustPromptTerminalOps {
        failure: TrustPromptFailure,
        cleanup: Vec<&'static str>,
    }

    struct SuccessfulTrustPromptTerminalOps {
        key: KeyCode,
    }

    impl TrustPromptTerminalOps for SuccessfulTrustPromptTerminalOps {
        fn enable_raw_mode(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn enter_alternate_screen(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn construct_terminal(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn draw(&mut self, _render: &TrustPromptRender<'_>) -> io::Result<()> {
            Ok(())
        }

        fn poll(&mut self, _timeout: std::time::Duration) -> io::Result<bool> {
            Ok(true)
        }

        fn read(&mut self) -> io::Result<Event> {
            Ok(Event::Key(crossterm::event::KeyEvent::new(
                self.key,
                KeyModifiers::NONE,
            )))
        }

        fn leave_alternate_screen(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn disable_raw_mode(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl TrustPromptTerminalOps for RecordingTrustPromptTerminalOps {
        fn enable_raw_mode(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn enter_alternate_screen(&mut self) -> io::Result<()> {
            if self.failure == TrustPromptFailure::EnterAlternateScreen {
                Err(io::Error::other("injected alternate-screen failure"))
            } else {
                Ok(())
            }
        }

        fn construct_terminal(&mut self) -> io::Result<()> {
            if self.failure == TrustPromptFailure::ConstructTerminal {
                Err(io::Error::other("injected terminal construction failure"))
            } else {
                Ok(())
            }
        }

        fn draw(&mut self, _render: &TrustPromptRender<'_>) -> io::Result<()> {
            if self.failure == TrustPromptFailure::Draw {
                Err(io::Error::other("injected draw failure"))
            } else {
                Ok(())
            }
        }

        fn poll(&mut self, _timeout: std::time::Duration) -> io::Result<bool> {
            if self.failure == TrustPromptFailure::Poll {
                Err(io::Error::other("injected poll failure"))
            } else {
                Ok(true)
            }
        }

        fn read(&mut self) -> io::Result<Event> {
            if self.failure == TrustPromptFailure::Read {
                Err(io::Error::other("injected read failure"))
            } else {
                Ok(Event::Key(crossterm::event::KeyEvent::new(
                    KeyCode::Esc,
                    KeyModifiers::NONE,
                )))
            }
        }

        fn leave_alternate_screen(&mut self) -> io::Result<()> {
            self.cleanup.push("leave_alternate_screen");
            Ok(())
        }

        fn disable_raw_mode(&mut self) -> io::Result<()> {
            self.cleanup.push("disable_raw_mode");
            Ok(())
        }
    }

    #[test]
    fn trust_prompt_cleanup_matches_successfully_entered_terminal_states() {
        let cases = [
            (
                TrustPromptFailure::EnterAlternateScreen,
                vec!["disable_raw_mode"],
            ),
            (
                TrustPromptFailure::ConstructTerminal,
                vec!["leave_alternate_screen", "disable_raw_mode"],
            ),
            (
                TrustPromptFailure::Draw,
                vec!["leave_alternate_screen", "disable_raw_mode"],
            ),
            (
                TrustPromptFailure::Poll,
                vec!["leave_alternate_screen", "disable_raw_mode"],
            ),
            (
                TrustPromptFailure::Read,
                vec!["leave_alternate_screen", "disable_raw_mode"],
            ),
        ];

        for (failure, expected_cleanup) in cases {
            let (tx, _rx) = tokio::sync::oneshot::channel();
            let mut ops = RecordingTrustPromptTerminalOps {
                failure,
                cleanup: Vec::new(),
            };

            run_trust_prompt_terminal_with_ops(Path::new("project"), tx, &mut ops)
                .expect_err("each case injects one terminal failure");

            assert_eq!(ops.cleanup, expected_cleanup, "failure: {failure:?}");
        }
    }

    #[test]
    fn trust_prompt_at_filesystem_root_omits_parent_choice() {
        let canonical = std::fs::canonicalize(".").unwrap();
        let filesystem_root = canonical.ancestors().last().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut ops = SuccessfulTrustPromptTerminalOps {
            key: KeyCode::Char('2'),
        };

        run_trust_prompt_terminal_with_ops(filesystem_root, tx, &mut ops).unwrap();

        assert_eq!(rx.blocking_recv().unwrap(), TrustChoice::TrustSession);
    }

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
            pending_permission: None,
        }
    }

    /// A `TuiState` with a pending capability-permission prompt whose responder
    /// is wired to `rx` (Phase 16.10). Returns the state and the receiver.
    fn state_with_pending_permission(
        choice_seed: PermissionChoice,
    ) -> (TuiState, oneshot::Receiver<PermissionChoice>) {
        let (tx, rx) = oneshot::channel();
        let mut state = state_with_picker(PickerKind::Model);
        state.picker = None;
        let mut prompt = PermissionPrompt::new(PermissionSummary {
            adapter_id: "opi-sandbox".to_string(),
            package_name: "mock-pkg".to_string(),
            run_mode_label: "interactive".to_string(),
        });
        // Start the cursor on the seeded choice so Enter confirms it.
        while prompt.selected() != choice_seed {
            prompt.move_next();
        }
        state.pending_permission = Some(PendingPermission {
            prompt,
            responder: tx,
        });
        (state, rx)
    }

    #[test]
    fn permission_prompt_esc_resolves_to_deny_and_clears() {
        let (mut state, rx) = state_with_pending_permission(PermissionChoice::AllowOnce);
        handle_permission_key(&mut state, KeyCode::Esc);
        assert!(state.pending_permission.is_none(), "Esc clears the prompt");
        assert_eq!(rx.blocking_recv(), Ok(PermissionChoice::Deny));
    }

    #[test]
    fn permission_prompt_enter_confirms_highlighted_choice() {
        let (mut state, rx) = state_with_pending_permission(PermissionChoice::AllowSession);
        handle_permission_key(&mut state, KeyCode::Enter);
        assert!(state.pending_permission.is_none());
        assert_eq!(rx.blocking_recv(), Ok(PermissionChoice::AllowSession));
    }

    #[test]
    fn permission_prompt_arrows_move_without_resolving() {
        let (mut state, mut rx) = state_with_pending_permission(PermissionChoice::AllowOnce);
        handle_permission_key(&mut state, KeyCode::Down);
        handle_permission_key(&mut state, KeyCode::Down);
        assert!(
            state.pending_permission.is_some(),
            "arrows move the cursor without resolving"
        );
        // Cursor advanced AllowOnce -> AllowSession -> Deny (clamped).
        let selected = state
            .pending_permission
            .as_ref()
            .expect("prompt still pending")
            .prompt
            .selected();
        assert_eq!(selected, PermissionChoice::Deny);
        // Nothing sent yet.
        assert!(rx.try_recv().is_err(), "no choice sent on arrow keys");
    }

    #[test]
    fn permission_prompt_ignores_unrelated_keys() {
        let (mut state, mut rx) = state_with_pending_permission(PermissionChoice::AllowOnce);
        handle_permission_key(&mut state, KeyCode::Char('x'));
        assert!(
            state.pending_permission.is_some(),
            "unrelated key is ignored"
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn permission_prompt_is_absent_does_nothing() {
        // No pending prompt: handle_permission_key is a no-op (never panics).
        let (mut state, _rx) = state_with_pending_permission(PermissionChoice::AllowOnce);
        state.pending_permission = None;
        handle_permission_key(&mut state, KeyCode::Enter);
        assert!(state.pending_permission.is_none());
    }

    fn render_permission_prompt_whole_frame(width: u16, height: u16) -> String {
        let (state, _response) = state_with_pending_permission(PermissionChoice::AllowOnce);
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render_tui_state(frame, &state))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let mut rendered = String::new();
        for y in 0..buffer.area.height {
            let mut line = String::new();
            for x in 0..buffer.area.width {
                line.push_str(buffer.cell((x, y)).unwrap().symbol());
            }
            rendered.push_str(line.trim_end());
            rendered.push('\n');
        }
        rendered
    }

    #[test]
    fn permission_prompt_production_geometry_whole_frame_80x24() {
        insta::assert_snapshot!(
            "permission_prompt_production_geometry_whole_frame_80x24",
            render_permission_prompt_whole_frame(80, 24)
        );
    }

    #[test]
    fn permission_prompt_production_geometry_whole_frame_120x40() {
        insta::assert_snapshot!(
            "permission_prompt_production_geometry_whole_frame_120x40",
            render_permission_prompt_whole_frame(120, 40)
        );
    }

    /// A no-op keyring backend so tests can construct a `KeychainCredentialStore`
    /// (required by `TuiLoopRuntime`) without a real OS keyring. The credential
    /// store is only used for `/login`; the permission path never touches it.
    struct NoopKeyringBackend;
    impl crate::credential_store::KeyringBackend for NoopKeyringBackend {
        fn get(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Option<String>, crate::credential_store::BackendError> {
            Ok(None)
        }
        fn set(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<(), crate::credential_store::BackendError> {
            Ok(())
        }
        fn delete(&self, _: &str, _: &str) -> Result<(), crate::credential_store::BackendError> {
            Ok(())
        }
    }

    /// Phase 16.10 D.2 iter3 must-fix: drive the PRODUCTION `tui_event_loop` with
    /// a pending capability-permission prompt and scripted keys, proving the
    /// loop-internal production blocks: (1) the broker-channel drain populates
    /// `pending_permission`; (2) the modal key guard consumes the Enter and
    /// relays the highlighted choice through the responder (instead of submitting
    /// an empty prompt); (3) the frame redraws while the prompt is pending. The
    /// components (`handle_permission_key`, `TuiPermissionBroker`) are tested in
    /// isolation elsewhere; this proves their loop wiring.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn production_loop_drains_permission_prompt_and_relays_choice() {
        let workspace = tempfile::tempdir().unwrap();
        let provider = opi_ai::test_support::MockProvider::new(
            "mock",
            vec![opi_ai::test_support::text_response("ok")],
        );
        let harness = CodingHarness::builder(
            Box::new(provider),
            "mock:mock-model".to_string(),
            OpiConfig::default(),
            workspace.path().to_path_buf(),
            crate::project_trust::TrustDecision::Trusted,
        )
        .build();
        let harness = Arc::new(tokio::sync::Mutex::new(harness));

        // Pre-load a permission-prompt request on a channel (the broker sends one
        // when the bash tool hits an `ask` during a run). This receiver is the
        // loop's `permission_prompt_rx`.
        let (tx, rx) = mpsc::channel::<PermissionPromptRequest>(8);
        let (resp_tx, resp_rx) = oneshot::channel::<PermissionChoice>();
        tx.send(PermissionPromptRequest {
            summary: PermissionSummary {
                adapter_id: "opi-sandbox".to_string(),
                package_name: "mock-pkg".to_string(),
                run_mode_label: "interactive".to_string(),
            },
            responder: resp_tx,
        })
        .await
        .unwrap();
        let mut permission_prompt_rx = Some(rx);

        let capture = Arc::new(Mutex::new(InteractiveTuiTestCapture::default()));
        let mut terminal = HeadlessTuiTerminal {
            terminal: Terminal::new(TestBackend::new(80, 24)).unwrap(),
            capture: capture.clone(),
            failure: InteractiveTuiTestTerminalFailure::None,
        };
        // Scripted keys: "" -> a single Enter event. While the prompt is pending
        // the modal guard captures it (confirming AllowOnce) rather than letting
        // it submit an empty prompt; then "exit" terminates the loop.
        let mut events = ScriptedTuiEventSource::new(
            vec![String::new(), "exit".to_string()],
            Arc::new(InteractiveTuiTestObserver::default()),
            None,
        );
        let state = Arc::new(Mutex::new(TuiState {
            messages: Vec::new(),
            input_text: String::new(),
            app_state: AppState::Idle,
            model: "mock:mock-model".to_string(),
            active_tool: None,
            streaming_started: false,
            theme: Theme::default(),
            keybindings: Keybindings::default(),
            total_tokens: 0,
            cost_usd: None,
            graphics_protocol: TerminalGraphicsProtocol::Fallback,
            picker: None,
            pending_permission: None,
        }));

        let credential_store = Arc::new(crate::credential_store::KeychainCredentialStore::new(
            Box::new(NoopKeyringBackend),
            workspace.path().to_path_buf(),
        ));
        let runtime = TuiLoopRuntime {
            credential_store,
            oauth: OAuthDispatchRuntime {
                endpoints: OAuthEndpointConfig::production(),
                client: reqwest::Client::new(),
            },
            output_began: Arc::new(AtomicBool::new(false)),
            test_auth: None,
            test_observer: None,
        };

        // The loop runs until "exit" (Ok) or scripted-input exhaustion (Err);
        // either way the prompt must have been drained + resolved first.
        let _ = tui_event_loop(
            &mut terminal,
            &mut events,
            &harness,
            &state,
            &mut permission_prompt_rx,
            runtime,
        )
        .await;

        // The drain populated pending_permission, the modal guard captured the
        // Enter and confirmed the highlighted AllowOnce, and the choice was
        // relayed through the responder (the broker's waiting tool call resumes).
        assert_eq!(
            resp_rx.await,
            Ok(PermissionChoice::AllowOnce),
            "the production loop must drain the prompt and relay the choice"
        );
        assert!(
            state.lock().unwrap().pending_permission.is_none(),
            "pending permission cleared after the choice"
        );
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
            crate::project_trust::TrustDecision::Trusted,
        )
        .resource_layers(ResourceDiscoveryLayers {
            themes: vec![DiscoveryLayer {
                kind: crate::resource::DiscoveryLayerKind::Explicit,
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
