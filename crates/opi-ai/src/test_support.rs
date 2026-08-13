//! Shared test utilities for mock-provider testing (task 1.17).
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
use crate::message::AssistantMessage;
use crate::model_info::WireApi;
use crate::provider::{EventStream, ModelInfo, Provider, ProviderError, Request};
use crate::registry::ModelCapabilities;
use crate::stream::{AssistantStreamEvent, StopReason, Usage};
use crate::{CompatMetadata, ProviderCollection};
use secrecy::SecretString;

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
/// directly (Phase 17.5: `Provider::stream` is gone; every dispatch goes through
/// `stream_prepared(request, auth)`). The secret is a fixed placeholder and the
/// provenance defaults; tests that need a specific secret or base_url construct
/// their own `ResolvedAuth`.
#[doc(hidden)]
pub fn resolved_auth() -> ResolvedAuth {
    ResolvedAuth {
        scheme: AuthScheme::ApiKey,
        secret: SecretString::from("test-key"),
        base_url: None,
        account_id: None,
        provenance: AuthProvenance::default(),
    }
}

/// Build a [`ProviderCollection`] with one dispatchable route for `provider`,
/// using a dummy static resolver. Mock providers ignore the resolved auth, so
/// this lets tests dispatch through the Phase 17.2 `prepare_call` path without
/// supplying real credentials.
#[doc(hidden)]
pub fn single_route_collection(provider: Box<dyn Provider>) -> ProviderCollection {
    let mut collection = ProviderCollection::new();
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

/// A response that a mock provider can return per `stream()` call.
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
}

/// A mock provider that returns pre-programmed response sequences.
///
/// Each call to `stream()` pops the next response from the queue.
/// Tracks call history for assertions.
#[doc(hidden)]
pub struct MockProvider {
    id: String,
    models: Vec<ModelInfo>,
    responses: Arc<Mutex<Vec<MockResponse>>>,
    call_log: Arc<Mutex<Vec<Request>>>,
}

impl MockProvider {
    /// Create a new mock provider with the given response sequences.
    ///
    /// Each element of `responses` is a complete batch of stream events
    /// returned by one `stream()` call. Batches are consumed in order.
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
        }
    }

    /// Number of times `stream()` has been called.
    pub fn stream_call_count(&self) -> usize {
        self.call_log.lock().unwrap().len()
    }

    /// Snapshot the `messages` field of every `Request` passed to `stream()`
    /// so far. Useful for asserting which messages the provider observed
    /// during a test run.
    pub fn recorded_messages(&self) -> Vec<Vec<crate::message::Message>> {
        self.call_log
            .lock()
            .unwrap()
            .iter()
            .map(|r| r.messages.clone())
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
        response_id: None,
        usage: Usage::default(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp_ms: crate::time::now_ms(),
    }
}

/// Helper: build a text-only response (Start ->TextDelta ->Done).
pub fn text_response(text: &str) -> Vec<AssistantStreamEvent> {
    let mut partial = base_assistant();
    partial
        .content
        .push(crate::message::AssistantContent::Text { text: text.into() });
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

/// Helper: build an error response (Start ->Error).
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

    /// Prepared dispatch path (Phase 17.2): ignore the supplied resolved auth —
    /// the mock does not authenticate. This lets the Agent's
    /// `ProviderCollection::prepare_call` path drive the mock in tests;
    /// `stream_call_count` then records one entry per attempt, so a counting
    /// resolver can prove auth is resolved once across retries.
    fn stream_prepared(&self, request: Request, _auth: crate::auth::ResolvedAuth) -> EventStream {
        self.call_log.lock().unwrap().push(request);
        let mut responses = self.responses.lock().unwrap();
        assert!(
            !responses.is_empty(),
            "MockProvider: stream() called more times than responses were configured"
        );
        let response = responses.remove(0);
        match response {
            MockResponse::Events(events) => {
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
        }
    }

    fn replace_model_catalog(&mut self, models: Vec<ModelInfo>) -> Result<(), ProviderError> {
        self.models = models;
        Ok(())
    }
}
