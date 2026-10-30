//! Shared test utilities for mock-provider testing.
//!
//! Provides `MockProvider` for deterministic, fixture-based provider simulation
//! across all workspace crates. No live API calls.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use crate::auth::{
    AuthProvenance, AuthProvenanceSource, AuthResolver, AuthScheme, ResolvedAuth,
    StaticAuthResolver,
};
use crate::credential::BoxAuthFuture;
use crate::deferred::{DeferredApi, DeferredCancelCall, DeferredFetchCall, DeferredHandle};
use crate::message::AssistantMessage;
use crate::model_info::WireApi;
use crate::models::ApiId;
use crate::provider::{EventStream, ModelInfo, Provider, ProviderError, Request};
use crate::registry::ModelCapabilities;
use crate::stream::{AssistantStreamEvent, StopReason, Usage};
use crate::{CompatMetadata, Models};
use secrecy::SecretString;

/// Minimal native-helper environment rooted in an owned fixture directory.
/// No credential or proxy environment is inherited. Windows reads only its
/// system directory for PowerShell and native runtime initialization.
#[doc(hidden)]
pub fn portable_auth_process_snapshot(cwd: &std::path::Path) -> crate::auth::AuthProcessSnapshot {
    let mut env = std::collections::BTreeMap::from([
        ("HOME".to_owned(), cwd.to_string_lossy().into_owned()),
        ("TEMP".to_owned(), cwd.to_string_lossy().into_owned()),
        ("TMP".to_owned(), cwd.to_string_lossy().into_owned()),
    ]);
    #[cfg(windows)]
    {
        let system = std::env::var("SystemRoot").expect("Windows fixture system directory");
        env.insert("SystemRoot".into(), system.clone());
        env.insert("WINDIR".into(), system.clone());
        env.insert("USERPROFILE".into(), cwd.to_string_lossy().into_owned());
        env.insert(
            "PATH".into(),
            format!("{system}\\System32;{system}\\System32\\WindowsPowerShell\\v1.0"),
        );
    }
    #[cfg(unix)]
    env.insert("PATH".into(), "/usr/bin:/bin".into());
    crate::auth::AuthProcessSnapshot {
        env,
        cwd: cwd.to_owned(),
    }
}

/// An `AuthResolver` that increments a shared counter on every `resolve` and
/// returns a fixed non-secret credential. Used to prove collection-owned auth
/// is resolved once per logical call (not per retry attempt).
#[doc(hidden)]
pub struct CountingAuthResolver {
    count: Arc<AtomicU32>,
}

#[doc(hidden)]
impl CountingAuthResolver {
    /// Create a resolver that increments `count` on each resolution.
    pub fn new(count: Arc<AtomicU32>) -> Self {
        Self { count }
    }
}

impl AuthResolver for CountingAuthResolver {
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        let count = self.count.clone();
        Box::pin(async move {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(resolved_auth())
        })
    }
}

/// Build a non-secret [`ResolvedAuth`] for tests that drive `stream_prepared`
/// directly. The secret is a fixed placeholder and the provenance defaults;
/// tests that need a specific secret or base URL construct their own
/// `ResolvedAuth`.
#[doc(hidden)]
pub fn resolved_auth() -> ResolvedAuth {
    ResolvedAuth {
        env: Default::default(),
        scheme: AuthScheme::ApiKey,
        secret: SecretString::from("test-key"),
        base_url: None,
        account_id: None,
        provenance: AuthProvenance::default(),
    }
}

/// Bearer-scheme sibling of [`resolved_auth`] for wires that attach the secret
/// as an `Authorization: Bearer` header (openai_chat, openai_responses,
/// openai_codex_responses, vertex, and api-mapped routes onto those wires).
/// Those adapters reject a prepared ApiKey scheme before the wire.
#[doc(hidden)]
pub fn resolved_bearer_auth() -> ResolvedAuth {
    ResolvedAuth {
        env: Default::default(),
        scheme: AuthScheme::Bearer,
        secret: SecretString::from("test-key"),
        base_url: None,
        account_id: None,
        provenance: AuthProvenance::default(),
    }
}

/// Build a [`Models`] with one dispatchable route for `provider`,
/// using a dummy static resolver. Mock providers ignore the resolved auth, so
/// this lets tests dispatch through `prepare_call` without supplying real
/// credentials.
#[doc(hidden)]
pub fn single_route_collection(provider: Box<dyn Provider>) -> Models {
    let mut collection = Models::new();
    collection
        .register_route(
            provider,
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("test-key"),
            )) as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .expect("test collection: registering one route must succeed");
    collection
}

/// A response that a mock provider can return for one prepared attempt.
#[doc(hidden)]
pub enum MockResponse {
    /// Successful stream of assistant events.
    Events(Vec<AssistantStreamEvent>),
    /// Provider error (e.g. rate-limited, timeout).
    Error(ProviderError),
    /// Successful stream events followed by a mid-stream error.
    ///
    /// Models a provider that begins streaming content (Start/deltas) and then
    /// fails partway through, used to assert retry-after-partial-output policy.
    ///
    /// Do NOT include a terminal `Done` event in `events`: the agent loop exits
    /// the stream on `Done` (before reading further), so the trailing error
    /// would never be observed. Build the partial prefix only (e.g. `Start` +
    /// `TextDelta`).
    EventsThenError(Vec<AssistantStreamEvent>, ProviderError),
    /// Successful stream events withheld for a number of prepared attempts
    /// (task 20.53.8, pinned faux `pendingFetches`).
    ///
    /// The first `pending_attempts` prepared attempts each fail with
    /// [`ProviderError::RateLimited`] carrying `retry_after_ms` (the
    /// chat-level analog of the pinned `pollAfterMs` hint, reusing the
    /// existing retry mechanism instead of timers); the next attempt yields
    /// `events`. `pending_attempts: 0` behaves exactly like
    /// [`MockResponse::Events`]. One variant value spans those attempts,
    /// mirroring the pinned per-entry countdown.
    DelayedAfter {
        pending_attempts: usize,
        retry_after_ms: Option<u64>,
        events: Vec<AssistantStreamEvent>,
    },
    /// No events until the request's cancellation token fires, then a
    /// terminal [`ProviderError::Cancelled`] (task 20.53.8).
    ///
    /// Models a hanging attempt for deterministic cancellation tests: the
    /// attempt-boundary form of the pinned signal-aborted-to-error behavior.
    /// The stream never completes on its own; a pre-cancelled request fails
    /// immediately.
    HangingUntilCancelled,
}

/// A mock provider that returns pre-programmed response sequences.
///
/// Each `stream_prepared` attempt pops the next response from the queue.
/// Tracks call history for assertions.
#[doc(hidden)]
pub struct MockProvider {
    id: String,
    models: Vec<ModelInfo>,
    responses: Arc<Mutex<Vec<MockResponse>>>,
    call_log: Arc<Mutex<Vec<Request>>>,
    deferred_scripting: Option<DeferredScripting>,
}

/// Deferred-submission scripting attached to a [`MockProvider`] (task
/// 20.50): every prepared attempt registers its scripted final events with
/// the shared [`ScriptedDeferredBackend`] and returns a pending
/// `StopReason::Deferred` terminal carrying the new handle (pi faux
/// `options.deferred`).
#[doc(hidden)]
pub struct DeferredScripting {
    backend: Arc<ScriptedDeferredBackend>,
    pending_fetches: u64,
    poll_after_ms: Option<u64>,
}

impl MockProvider {
    /// Create a new mock provider with the given response sequences.
    ///
    /// Each element of `responses` is a complete batch of stream events
    /// returned by one prepared attempt. Batches are consumed in order.
    pub fn new(id: &str, responses: Vec<Vec<AssistantStreamEvent>>) -> Self {
        Self::new_with_errors(
            id,
            responses.into_iter().map(MockResponse::Events).collect(),
        )
    }

    /// Create a mock provider that can return errors between successful responses.
    pub fn new_with_errors(id: &str, responses: Vec<MockResponse>) -> Self {
        Self {
            id: id.to_owned(),
            models: vec![ModelInfo::new(
                "mock-model",
                "Mock Model",
                WireApi::OpenAiCompletions,
                ModelCapabilities::new(100_000, 4_096)
                    .with_images(true)
                    .with_streaming(true),
            )],
            responses: Arc::new(Mutex::new(responses)),
            call_log: Arc::new(Mutex::new(Vec::new())),
            deferred_scripting: None,
        }
    }

    /// Create a mock provider with custom models.
    pub fn new_with_models(
        id: &str,
        models: Vec<ModelInfo>,
        responses: Vec<Vec<AssistantStreamEvent>>,
    ) -> Self {
        Self {
            id: id.to_owned(),
            models,
            responses: Arc::new(Mutex::new(
                responses.into_iter().map(MockResponse::Events).collect(),
            )),
            call_log: Arc::new(Mutex::new(Vec::new())),
            deferred_scripting: None,
        }
    }

    /// Create a mock provider with custom models and scripted responses that
    /// can interleave errors between successful event batches.
    pub fn new_with_models_and_errors(
        id: &str,
        models: Vec<ModelInfo>,
        responses: Vec<MockResponse>,
    ) -> Self {
        Self {
            id: id.to_owned(),
            models,
            responses: Arc::new(Mutex::new(responses)),
            call_log: Arc::new(Mutex::new(Vec::new())),
            deferred_scripting: None,
        }
    }

    /// Attach deferred-submission scripting (task 20.50): every prepared
    /// attempt registers its scripted final events with `backend` as a
    /// deferred response and returns a pending terminal with the new
    /// handle. `pending_fetches` fetches stay pending before the final
    /// events become ready; `poll_after_ms` stamps the handle's poll
    /// advice (caller-owned schedule, never an SDK polling default).
    pub fn with_deferred(
        mut self,
        backend: Arc<ScriptedDeferredBackend>,
        pending_fetches: u64,
        poll_after_ms: Option<u64>,
    ) -> Self {
        self.deferred_scripting = Some(DeferredScripting {
            backend,
            pending_fetches,
            poll_after_ms,
        });
        self
    }

    /// Number of prepared attempts made through `stream_prepared`.
    pub fn stream_call_count(&self) -> usize {
        self.call_log.lock().unwrap().len()
    }

    /// Snapshot the `messages` field of every `Request` passed to
    /// `stream_prepared` so far. Useful for asserting which messages the
    /// provider observed during a test run.
    pub fn recorded_messages(&self) -> Vec<Vec<crate::message::Message>> {
        self.call_log
            .lock()
            .unwrap()
            .iter()
            .map(|r| r.context.messages().to_vec())
            .collect()
    }

    /// Clone the shared call-log handle. Lets a test hold a reference to the
    /// recorded requests even after the provider is moved into a `Box<dyn
    /// Provider>`.
    pub fn call_log_handle(&self) -> Arc<Mutex<Vec<Request>>> {
        Arc::clone(&self.call_log)
    }
}

/// Helper: build a base `AssistantMessage` for fixture construction.
pub fn base_assistant() -> AssistantMessage {
    AssistantMessage {
        content: vec![],
        api: crate::ApiKind::Anthropic,
        provider: "mock".into(),
        model: "mock-model".into(),
        response_model: None,
        // Mock histories carry no wire provenance: replay treats them as
        // legacy-unknown (fail-closed) exactly like hand-built records.
        wire: None,
        response_id: None,
        usage: Usage::default(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp_ms: crate::time::now_ms(),
        diagnostics: Vec::new(),
        duration_ms: None,
        provider_thinking_level: None,
        deferred: None,
    }
}

/// Helper: build a text-only response (Start ->TextDelta ->Done).
pub fn text_response(text: &str) -> Vec<AssistantStreamEvent> {
    let mut partial = base_assistant();
    partial
        .content
        .push(crate::message::AssistantContent::Text {
            text: text.into(),
            signature: None,
        });
    vec![
        AssistantStreamEvent::Start {
            partial: base_assistant(),
        },
        AssistantStreamEvent::TextDelta {
            content_index: 0,
            delta: text.into(),
            partial: partial.clone(),
        },
        AssistantStreamEvent::Done {
            reason: StopReason::Stop,
            message: partial,
        },
    ]
}

/// Helper: build a tool-call response (Start ->ToolCallEnd ->Done).
pub fn tool_call_response(
    tool_call_id: &str,
    tool_name: &str,
    arguments: &str,
) -> Vec<AssistantStreamEvent> {
    let tool_call = crate::message::ToolCall {
        id: tool_call_id.into(),
        name: tool_name.into(),
        arguments: arguments.into(),
        arguments_error: None,
        signature: None,
    };
    let mut partial = base_assistant();
    partial
        .content
        .push(crate::message::AssistantContent::ToolCall {
            tool_call: tool_call.clone(),
        });
    vec![
        AssistantStreamEvent::Start {
            partial: base_assistant(),
        },
        AssistantStreamEvent::ToolCallEnd {
            content_index: 0,
            tool_call,
            partial: partial.clone(),
        },
        AssistantStreamEvent::Done {
            reason: StopReason::ToolUse,
            message: partial,
        },
    ]
}

/// Helper: build an ordered multi-block response (Start -> per-block events
/// -> Done) (task 20.53.8).
///
/// Emits one event per block in order - `TextDelta` for text blocks and
/// `ToolCallEnd` for tool-call blocks, with `content_index` matching the
/// block position and each partial containing only the emitted prefix -
/// and finishes with `Done`, using `StopReason::ToolUse`
/// when any tool call is present, else `StopReason::Stop`. This is the
/// ordered multi-block/tool fixture the single-purpose `text_response` and
/// `tool_call_response` helpers cannot express.
///
/// # Panics
///
/// Panics for `Thinking`/`RedactedThinking` blocks: this fixture scripts
/// only the text/tool ordering contract.
pub fn blocks_response(blocks: Vec<crate::message::AssistantContent>) -> Vec<AssistantStreamEvent> {
    let uses_tools = blocks
        .iter()
        .any(|block| matches!(block, crate::message::AssistantContent::ToolCall { .. }));
    let mut partial = base_assistant();
    let mut events = vec![AssistantStreamEvent::Start {
        partial: base_assistant(),
    }];
    for (content_index, block) in blocks.iter().enumerate() {
        partial.content.push(block.clone());
        match block {
            crate::message::AssistantContent::Text { text, .. } => {
                events.push(AssistantStreamEvent::TextDelta {
                    content_index,
                    delta: text.clone(),
                    partial: partial.clone(),
                });
            }
            crate::message::AssistantContent::ToolCall { tool_call } => {
                events.push(AssistantStreamEvent::ToolCallEnd {
                    content_index,
                    tool_call: tool_call.clone(),
                    partial: partial.clone(),
                });
            }
            other => panic!("blocks_response scripts only Text and ToolCall blocks, got {other:?}"),
        }
    }
    events.push(AssistantStreamEvent::Done {
        reason: if uses_tools {
            StopReason::ToolUse
        } else {
            StopReason::Stop
        },
        message: partial,
    });
    events
}

/// Helper: build an error response (Start ->Error).
///
/// Emits the legacy in-band `Error` terminal on purpose: this fixture models
/// historical/mock producers so downstream suites exercise the `ChatStream`
/// conversion to `Err(CallFailure)` that real providers no longer produce.
pub fn error_response(error_message: &str) -> Vec<AssistantStreamEvent> {
    let mut partial = base_assistant();
    partial.error_message = Some(error_message.into());
    vec![
        AssistantStreamEvent::Start {
            partial: base_assistant(),
        },
        AssistantStreamEvent::Error {
            reason: StopReason::Error,
            message: partial,
        },
    ]
}

impl Provider for MockProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    /// Ignore the supplied resolved auth because the mock does not authenticate.
    /// This lets the Agent's
    /// `Models::prepare_call` path drive the mock in tests;
    /// `stream_call_count` then records one entry per attempt, so a counting
    /// resolver can prove auth is resolved once across retries.
    fn stream_prepared(&self, request: Request, _auth: crate::auth::ResolvedAuth) -> EventStream {
        let cancel = request.cancel.clone();
        self.call_log.lock().unwrap().push(request);
        let mut responses = self.responses.lock().unwrap();
        assert!(
            !responses.is_empty(),
            "MockProvider: stream_prepared() called more times than responses were configured"
        );
        // A `DelayedAfter` entry stays at the head while its countdown lasts;
        // everything else is consumed in order.
        let response = match responses.first_mut() {
            Some(MockResponse::DelayedAfter {
                pending_attempts,
                retry_after_ms,
                ..
            }) if *pending_attempts > 0 => {
                *pending_attempts -= 1;
                MockResponse::Error(ProviderError::RateLimited {
                    retry_after_ms: *retry_after_ms,
                })
            }
            _ => responses.remove(0),
        };
        match response {
            MockResponse::Events(events) => {
                if let Some(scripting) = &self.deferred_scripting {
                    // Deferred submission (task 20.50): register the
                    // scripted final events and deliver a pending terminal.
                    let handle = scripting.backend.submit(
                        events,
                        scripting.pending_fetches,
                        scripting.poll_after_ms,
                    );
                    let mut pending = base_assistant();
                    pending.provider = self.id.clone();
                    pending.model = self.models[0].id.clone();
                    pending.stop_reason = StopReason::Deferred;
                    pending.deferred = Some(Box::new(handle));
                    let stream = futures_util::stream::iter(vec![
                        Ok::<_, ProviderError>(AssistantStreamEvent::Start {
                            partial: pending.clone(),
                        }),
                        Ok(AssistantStreamEvent::Done {
                            reason: StopReason::Deferred,
                            message: pending,
                        }),
                    ]);
                    return Box::pin(stream);
                }
                let stream =
                    futures_util::stream::iter(events.into_iter().map(Ok::<_, ProviderError>));
                Box::pin(stream)
            }
            MockResponse::Error(e) => {
                let stream = futures_util::stream::iter(vec![Err(e)]);
                Box::pin(stream)
            }
            MockResponse::EventsThenError(events, e) => {
                let ok_events = events.into_iter().map(Ok::<_, ProviderError>);
                let err = std::iter::once(Err(e));
                let stream = futures_util::stream::iter(ok_events.chain(err));
                Box::pin(stream)
            }
            MockResponse::DelayedAfter { events, .. } => {
                let stream =
                    futures_util::stream::iter(events.into_iter().map(Ok::<_, ProviderError>));
                Box::pin(stream)
            }
            MockResponse::HangingUntilCancelled => {
                let stream = futures_util::stream::once(async move {
                    cancel.cancelled().await;
                    Err(ProviderError::Cancelled)
                });
                Box::pin(stream)
            }
        }
    }

    fn replace_model_catalog(&mut self, models: Vec<ModelInfo>) -> Result<(), ProviderError> {
        self.models = models;
        Ok(())
    }
}

/// Scriptable fake for the AWS auth-source HTTP transport.
///
/// Records every send (method/url/headers/body/timeout) and replays scripted
/// responses in order; unscripted sends replay `default`. Header and body
/// values stay out of `Debug` output — requests carry credential material.
#[derive(Default)]
pub struct FakeSourceTransport {
    sends: std::sync::Mutex<Vec<crate::auth::aws::source_http::SourceHttpRequest>>,
    responses: std::sync::Mutex<
        std::collections::VecDeque<
            Result<
                crate::auth::aws::source_http::SourceHttpResponse,
                crate::auth::aws::source_http::SourceHttpError,
            >,
        >,
    >,
    default: std::sync::Mutex<
        Option<
            Result<
                crate::auth::aws::source_http::SourceHttpResponse,
                crate::auth::aws::source_http::SourceHttpError,
            >,
        >,
    >,
}

impl FakeSourceTransport {
    /// Script the responses replayed in send order.
    pub fn new(
        responses: Vec<
            Result<
                crate::auth::aws::source_http::SourceHttpResponse,
                crate::auth::aws::source_http::SourceHttpError,
            >,
        >,
    ) -> Self {
        Self {
            responses: std::sync::Mutex::new(responses.into_iter().collect()),
            ..Self::default()
        }
    }

    /// A transport that fails every send with a transport error.
    pub fn default_never() -> Self {
        Self::default()
    }

    /// Number of sends observed so far.
    pub fn send_count(&self) -> usize {
        self.sends.lock().expect("send log").len()
    }

    /// One recorded send (panic-free assertions: index must exist).
    pub fn send_at(&self, index: usize) -> crate::auth::aws::source_http::SourceHttpRequest {
        self.sends.lock().expect("send log")[index].clone()
    }

    /// A snapshot of all recorded sends.
    pub fn sends_snapshot(&self) -> Vec<crate::auth::aws::source_http::SourceHttpRequest> {
        self.sends.lock().expect("send log").clone()
    }
}

impl crate::auth::aws::source_http::AwsSourceTransport for FakeSourceTransport {
    fn send(
        &self,
        request: crate::auth::aws::source_http::SourceHttpRequest,
    ) -> futures_util::future::BoxFuture<
        '_,
        Result<
            crate::auth::aws::source_http::SourceHttpResponse,
            crate::auth::aws::source_http::SourceHttpError,
        >,
    > {
        use futures_util::FutureExt as _;
        self.sends.lock().expect("send log").push(request);
        let response = self
            .responses
            .lock()
            .expect("response queue")
            .pop_front()
            .or_else(|| self.default.lock().expect("default").clone())
            .unwrap_or(Err(
                crate::auth::aws::source_http::SourceHttpError::Transport,
            ));
        async move { response }.boxed()
    }
}

impl std::fmt::Debug for FakeSourceTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FakeSourceTransport")
            .field("sends", &self.send_count())
            .finish()
    }
}

/// A source transport that always fails (for fixtures that must never
/// perform a real exchange).
pub struct NeverTransport;

impl crate::auth::aws::source_http::AwsSourceTransport for NeverTransport {
    fn send(
        &self,
        _request: crate::auth::aws::source_http::SourceHttpRequest,
    ) -> futures_util::future::BoxFuture<
        '_,
        Result<
            crate::auth::aws::source_http::SourceHttpResponse,
            crate::auth::aws::source_http::SourceHttpError,
        >,
    > {
        use futures_util::FutureExt as _;
        async { Err(crate::auth::aws::source_http::SourceHttpError::Transport) }.boxed()
    }
}

/// Write a one-shot executable helper script for the current platform and
/// return the ADC-executable command string (a quote-aware direct argv; the
/// caller never parses it through a shell). Acceptance test sources stay
/// platform-neutral by delegating the platform split to this helper.
pub fn portable_executable_script(
    dir: &std::path::Path,
    name: &str,
    #[allow(unused_variables)] unix_body: &str,
    #[allow(unused_variables)] windows_body: &str,
) -> String {
    #[cfg(unix)]
    {
        let path = dir.join(format!("{name}.sh"));
        std::fs::write(&path, format!("#!/bin/sh\n{unix_body}\n")).expect("write helper script");
        format!("sh \"{}\"", path.display())
    }
    #[cfg(windows)]
    {
        let path = dir.join(format!("{name}.ps1"));
        std::fs::write(&path, format!("{windows_body}\n")).expect("write helper script");
        format!("powershell -NoProfile -File \"{}\"", path.display())
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (dir, name, unix_body, windows_body);
        unimplemented!("portable executable helpers support unix and windows");
    }
}

/// Force a [`crate::identity::UuidV7Generator`] sequence to its exhaustion
/// boundary (test hook; production exhaustion needs `2^41` calls).
///
/// Task 20.53.4 / P20-S53-4: keeps
/// [`crate::identity::UuidV7Error::SequenceExhausted`] observable in
/// deterministic tests.
pub fn force_sequence_exhausted(generator: &mut crate::identity::UuidV7Generator) {
    generator.set_sequence_for_tests((1u64 << 41) - 1);
}

// ---------------------------------------------------------------------------
// Scripted deferred backend (task 20.50)
// ---------------------------------------------------------------------------

#[doc(hidden)]
struct ScriptedDeferredEntry {
    handle: DeferredHandle,
    final_events: Vec<AssistantStreamEvent>,
    pending_fetches: u64,
    cancelled: bool,
}

#[doc(hidden)]
struct ScriptedDeferredState {
    entries: std::collections::BTreeMap<String, ScriptedDeferredEntry>,
    next_id: u64,
    fetch_count: usize,
    cancelled_handles: Vec<DeferredHandle>,
    fail_cancels: bool,
    /// Whether each fetch carried an explicit telemetry carrier (AIR-054
    /// participation evidence for the deferred surface).
    fetch_telemetry_present: Vec<bool>,
    cancel_telemetry_present: Vec<bool>,
}

/// Deterministic scripted deferred backend (pi faux `fetchDeferred`/
/// `cancelDeferred`): pending fetches return the original handle, the
/// countdown then delivers the registered final events, unknown or
/// cancelled handles fail typed, and cancel records every attempt while
/// optionally reporting best-effort failure.
///
/// Shares entries with a [`MockProvider`] built through
/// [`MockProvider::with_deferred`]; the same backend survives collection
/// rebuilds so a reopened consumer fetches without resubmitting.
pub struct ScriptedDeferredBackend {
    api_id: ApiId,
    provider_id: String,
    model_id: String,
    state: Mutex<ScriptedDeferredState>,
}

impl ScriptedDeferredBackend {
    /// Create a backend serving `provider_id`/`model_id` under the fixed
    /// `scripted-deferred` API identity.
    pub fn new(provider_id: &str, model_id: &str) -> Self {
        Self {
            api_id: ApiId::new("scripted-deferred"),
            provider_id: provider_id.to_owned(),
            model_id: model_id.to_owned(),
            state: Mutex::new(ScriptedDeferredState {
                entries: std::collections::BTreeMap::new(),
                next_id: 1,
                fetch_count: 0,
                cancelled_handles: Vec::new(),
                fail_cancels: false,
                fetch_telemetry_present: Vec::new(),
                cancel_telemetry_present: Vec::new(),
            }),
        }
    }

    /// Register one deferred response: the handle's eventual final events,
    /// how many fetches stay pending first, and optional poll/expiry advice.
    pub fn submit(
        &self,
        final_events: Vec<AssistantStreamEvent>,
        pending_fetches: u64,
        poll_after_ms: Option<u64>,
    ) -> DeferredHandle {
        self.submit_with_expiry(final_events, pending_fetches, poll_after_ms, None)
    }

    /// Register one deferred response with an absolute expiry (epoch
    /// milliseconds); the SDK validates expiry against the caller-injected
    /// clock reading.
    pub fn submit_with_expiry(
        &self,
        final_events: Vec<AssistantStreamEvent>,
        pending_fetches: u64,
        poll_after_ms: Option<u64>,
        expires_at_ms: Option<u64>,
    ) -> DeferredHandle {
        let mut state = self.state.lock().unwrap();
        let id = format!("deferred-{}", state.next_id);
        state.next_id += 1;
        let handle = DeferredHandle {
            provider: self.provider_id.clone(),
            model_id: self.model_id.clone(),
            api: self.api_id.as_str().to_owned(),
            id: id.clone(),
            expires_at_ms,
            poll_after_ms,
            data: None,
        };
        state.entries.insert(
            id,
            ScriptedDeferredEntry {
                handle: handle.clone(),
                final_events,
                pending_fetches,
                cancelled: false,
            },
        );
        handle
    }

    /// Number of fetch operations dispatched through this backend.
    pub fn fetch_count(&self) -> usize {
        self.state.lock().unwrap().fetch_count
    }

    /// Every handle passed to `cancel_deferred`, in order.
    pub fn cancelled_handles(&self) -> Vec<DeferredHandle> {
        self.state.lock().unwrap().cancelled_handles.clone()
    }

    /// Make subsequent cancellations report best-effort failure.
    pub fn fail_cancels(&self) {
        self.state.lock().unwrap().fail_cancels = true;
    }

    /// Whether each dispatched fetch/cancel carried an explicit telemetry
    /// carrier (AIR-054): `true` when the call supplied one.
    pub fn fetch_telemetry_present(&self) -> Vec<bool> {
        self.state.lock().unwrap().fetch_telemetry_present.clone()
    }

    #[doc(hidden)]
    pub fn cancel_telemetry_present(&self) -> Vec<bool> {
        self.state.lock().unwrap().cancel_telemetry_present.clone()
    }

    fn scripted_error(message: String) -> ProviderError {
        ProviderError::Config(crate::provider::ProviderErrorSummary::sanitized(message))
    }
}

impl DeferredApi<()> for ScriptedDeferredBackend {
    fn api_id(&self) -> &ApiId {
        &self.api_id
    }

    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn default_options(&self) {}

    fn fetch_prepared(
        &self,
        handle: &DeferredHandle,
        _options: &(),
        _auth: ResolvedAuth,
        call: DeferredFetchCall,
    ) -> EventStream {
        let mut state = self.state.lock().unwrap();
        state.fetch_count += 1;
        state
            .fetch_telemetry_present
            .push(call.telemetry_context.is_some());
        let Some(entry) = state.entries.get_mut(&handle.id) else {
            let error =
                Self::scripted_error(format!("Unknown scripted deferred response: {}", handle.id));
            return Box::pin(futures_util::stream::once(async move {
                Err::<AssistantStreamEvent, ProviderError>(error)
            }));
        };
        if entry.cancelled {
            let error = Self::scripted_error(format!(
                "Scripted deferred response was cancelled: {}",
                handle.id
            ));
            return Box::pin(futures_util::stream::once(async move {
                Err::<AssistantStreamEvent, ProviderError>(error)
            }));
        }
        if entry.pending_fetches > 0 {
            entry.pending_fetches -= 1;
            let mut pending = base_assistant();
            pending.provider = self.provider_id.clone();
            pending.model = self.model_id.clone();
            pending.stop_reason = StopReason::Deferred;
            pending.deferred = Some(Box::new(entry.handle.clone()));
            let events = vec![
                Ok::<_, ProviderError>(AssistantStreamEvent::Start {
                    partial: pending.clone(),
                }),
                Ok(AssistantStreamEvent::Done {
                    reason: StopReason::Deferred,
                    message: pending,
                }),
            ];
            return Box::pin(futures_util::stream::iter(events));
        }
        let events = entry
            .final_events
            .iter()
            .cloned()
            .map(Ok::<_, ProviderError>)
            .collect::<Vec<_>>();
        Box::pin(futures_util::stream::iter(events))
    }

    fn cancel_prepared<'a>(
        &'a self,
        handle: &'a DeferredHandle,
        _options: &'a (),
        _auth: ResolvedAuth,
        call: DeferredCancelCall,
    ) -> crate::credential::BoxAuthFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            let mut state = self.state.lock().unwrap();
            state.cancelled_handles.push(handle.clone());
            state
                .cancel_telemetry_present
                .push(call.telemetry_context.is_some());
            if let Some(entry) = state.entries.get_mut(&handle.id) {
                entry.cancelled = true;
            }
            if state.fail_cancels {
                return Err(Self::scripted_error(
                    "scripted deferred cancel failed (best-effort reporting)".to_owned(),
                ));
            }
            Ok(())
        })
    }
}
