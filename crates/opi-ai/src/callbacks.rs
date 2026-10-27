//! Functional request callbacks (task 20.51, AIR-059/V-51 local slice).
//!
//! Four callback families ride an ordinary [`crate::provider::Request`] and
//! fire at source-defined timing inside the real direct adapters — never by
//! re-deriving wire facts from a normalized [`crate::provider::EventStream`]:
//!
//! - `transform_headers` — applied exactly once per logical call to the merged
//!   per-call header set (configured profile, route-managed, per-request).
//!   Protected identity/auth/signing entries may be retained unchanged when
//!   present in the input. The output may not add, change or remove them,
//!   duplicate a name, or carry an invalid name/value. Removing or rewriting a
//!   non-protected entry is the supported omission/set/removal behavior.
//! - `on_payload` — invoked after sampling/compat/wire projection with the
//!   final request body. `Ok(None)` keeps it; `Ok(Some(v))` replaces it after
//!   validation (JSON object, unchanged model identity, output budget not
//!   raised). The callback runs once per payload variant; an ordinary
//!   acquisition retry re-enters the adapter with the same variant and reuses
//!   the frozen body byte-for-byte.
//! - `on_response` — invoked with status and headers at the adapter's
//!   source-defined response observation point, before body processing.
//!   Codex SSE also observes non-success statuses before classification;
//!   successful response handoff seals ordinary acquisition retry. Failures
//!   without a response (connect errors, timeouts) are not notified.
//! - `on_raw` — invoked with each parsed provider wire event, in wire order,
//!   strictly before its normalized emission. Read-only; a failure terminates
//!   the current call with a typed error while preserving the already
//!   delivered partial.
//!
//! Google wires (Gemini/Vertex) support the payload and header-transform
//! families and parsed raw observation but reject response observers with a typed
//! [`crate::provider::ProviderError::UnsupportedCapability`] before dispatch.
//!
//! Callback failures are functional failures: they surface as
//! [`crate::provider::ProviderError::CallbackPhase`] (never retryable, never
//! swallowed by telemetry, never triggering a new call).

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use crate::provider::{ProviderError, ProviderErrorSummary};

/// The payload-variant label of the ordinary call path. Strict variants
/// (task 20.11) add their own labels; an ordinary acquisition retry reuses
/// this variant's frozen payload.
pub const ORDINARY_PAYLOAD_VARIANT: &str = "ordinary";

/// The payload-variant label of the single rebuilt Anthropic strict
/// downgrade (task 20.11, V-34/V-36): its own freeze slot, so the payload
/// callback re-runs once on the rebuilt body and ordinary retries of the
/// downgrade variant reuse it.
pub const STRICT_DOWNGRADE_PAYLOAD_VARIANT: &str = "anthropic_strict_downgrade";

/// A callback's typed functional failure. The detail is redacted before it
/// reaches any public error surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallbackError {
    detail: String,
}

impl CallbackError {
    /// Capture a failure detail (already free of secrets on the caller side;
    /// the public surface redacts it again defensively).
    pub fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }

    fn into_provider_error(self, phase: &str) -> ProviderError {
        ProviderError::CallbackPhase(ProviderErrorSummary::sanitized(format!(
            "{phase} callback failed: {}",
            self.detail
        )))
    }
}

impl std::fmt::Display for CallbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for CallbackError {}

/// Header-transform callback: receives the merged per-call header set and
/// returns the replacement set. Protected values may be retained unchanged;
/// adding, changing or removing a protected identity is rejected.
pub type HeaderTransformFn =
    Arc<dyn Fn(&[(String, String)]) -> Result<Vec<(String, String)>, CallbackError> + Send + Sync>;

/// Payload callback: receives the post-projection wire body. `Ok(None)` keeps
/// it; `Ok(Some(v))` replaces it after validation.
pub type PayloadFn = Arc<
    dyn Fn(&serde_json::Value) -> Result<Option<serde_json::Value>, CallbackError> + Send + Sync,
>;

/// Response callback: receives the HTTP status and response headers.
pub type ResponseFn =
    Arc<dyn Fn(u16, &[(String, String)]) -> Result<(), CallbackError> + Send + Sync>;

/// Raw-event callback: receives each parsed provider wire event before
/// normalization.
pub type RawFn = Arc<dyn Fn(&serde_json::Value) -> Result<(), CallbackError> + Send + Sync>;

type HeaderList = Vec<(String, String)>;
type CallbackFuture<T> = Pin<Box<dyn Future<Output = Result<T, CallbackError>> + Send>>;
type AsyncHeaderTransformFn = Arc<dyn Fn(HeaderList) -> CallbackFuture<HeaderList> + Send + Sync>;
type AsyncPayloadFn =
    Arc<dyn Fn(serde_json::Value) -> CallbackFuture<Option<serde_json::Value>> + Send + Sync>;
type AsyncResponseFn = Arc<dyn Fn(u16, HeaderList) -> CallbackFuture<()> + Send + Sync>;
type AsyncRawFn = Arc<dyn Fn(serde_json::Value) -> CallbackFuture<()> + Send + Sync>;

#[derive(Clone)]
enum HeaderTransform {
    Sync(HeaderTransformFn),
    Async(AsyncHeaderTransformFn),
}
#[derive(Clone)]
enum Payload {
    Sync(PayloadFn),
    Async(AsyncPayloadFn),
}
#[derive(Clone)]
enum Response {
    Sync(ResponseFn),
    Async(AsyncResponseFn),
}
#[derive(Clone)]
enum Raw {
    Sync(RawFn),
    Async(AsyncRawFn),
}

#[derive(Default)]
struct RequestCallbacksInner {
    header_transform: Option<HeaderTransform>,
    on_payload: Option<Payload>,
    on_response: Option<Response>,
    on_raw: Option<Raw>,
}

#[derive(Default)]
struct CallState {
    payloads: Mutex<HashMap<String, Result<serde_json::Value, ProviderError>>>,
    headers: Mutex<Option<Result<HeaderList, ProviderError>>>,
    // Only call-local callback execution is serialized. Cache locks are never
    // retained while awaiting a functional callback.
    execution: tokio::sync::Mutex<()>,
}

/// Functional callback definitions carried by [`crate::provider::Request`].
///
/// Cloning shares definitions. SDK call boundaries create a fresh private
/// execution scope; retries of that logical call share its frozen headers and
/// per-variant payloads. Reusing a definition set never reuses another call's
/// wire input. Synchronous builders remain available; async builders receive
/// owned input and may yield without blocking the runtime.
#[derive(Clone, Default)]
pub struct RequestCallbacks {
    inner: Arc<RequestCallbacksInner>,
    state: Arc<CallState>,
    scoped: bool,
    // A forwarding adapter must retain the existing attempt's variant instead
    // of creating a nested acquisition owner. This is SDK-owned call state,
    // never a caller-supplied callback or wire field.
    attempt: Option<crate::provider::AttemptContext>,
}

impl std::fmt::Debug for RequestCallbacks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestCallbacks")
            .field("header_transform", &self.inner.header_transform.is_some())
            .field("on_payload", &self.inner.on_payload.is_some())
            .field("on_response", &self.inner.on_response.is_some())
            .field("on_raw", &self.inner.on_raw.is_some())
            .finish_non_exhaustive()
    }
}

impl RequestCallbacks {
    pub(crate) fn with_attempt(&self, attempt: crate::provider::AttemptContext) -> Self {
        let mut callbacks = self.clone();
        callbacks.attempt = Some(attempt);
        callbacks
    }

    pub(crate) fn inherited_attempt(&self) -> Option<crate::provider::AttemptContext> {
        self.attempt.clone()
    }
    /// An empty callback definition set.
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn for_call(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            state: Arc::default(),
            scoped: true,
            attempt: None,
        }
    }

    pub(crate) fn ensure_call_scope(&self) -> Self {
        if self.scoped {
            self.clone()
        } else {
            self.for_call()
        }
    }

    fn definitions_mut(&mut self) -> &mut RequestCallbacksInner {
        Arc::get_mut(&mut self.inner)
            .expect("RequestCallbacks builders run before the set is shared")
    }

    /// Attach a synchronous assembled-header transform.
    pub fn with_header_transform(mut self, transform: HeaderTransformFn) -> Self {
        self.definitions_mut().header_transform = Some(HeaderTransform::Sync(transform));
        self
    }
    /// Attach a synchronous post-projection payload callback.
    pub fn with_on_payload(mut self, callback: PayloadFn) -> Self {
        self.definitions_mut().on_payload = Some(Payload::Sync(callback));
        self
    }
    /// Attach a synchronous response observer.
    pub fn with_on_response(mut self, callback: ResponseFn) -> Self {
        self.definitions_mut().on_response = Some(Response::Sync(callback));
        self
    }
    /// Attach a synchronous parsed raw-event observer.
    pub fn with_on_raw(mut self, callback: RawFn) -> Self {
        self.definitions_mut().on_raw = Some(Raw::Sync(callback));
        self
    }

    /// Attach an async-capable transform receiving an owned assembled-header set.
    pub fn with_async_header_transform<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(Vec<(String, String)>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Vec<(String, String)>, CallbackError>> + Send + 'static,
    {
        self.definitions_mut().header_transform =
            Some(HeaderTransform::Async(Arc::new(move |headers| {
                Box::pin(callback(headers))
            })));
        self
    }
    /// Attach an async-capable payload callback receiving the owned wire body.
    pub fn with_async_on_payload<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Option<serde_json::Value>, CallbackError>> + Send + 'static,
    {
        self.definitions_mut().on_payload = Some(Payload::Async(Arc::new(move |body| {
            Box::pin(callback(body))
        })));
        self
    }
    /// Attach an async-capable observer receiving owned status/header facts.
    pub fn with_async_on_response<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(u16, Vec<(String, String)>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), CallbackError>> + Send + 'static,
    {
        self.definitions_mut().on_response =
            Some(Response::Async(Arc::new(move |status, headers| {
                Box::pin(callback(status, headers))
            })));
        self
    }
    /// Attach an ordered async-capable observer receiving a parsed raw wire value.
    pub fn with_async_on_raw<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), CallbackError>> + Send + 'static,
    {
        self.definitions_mut().on_raw =
            Some(Raw::Async(Arc::new(move |event| Box::pin(callback(event)))));
        self
    }

    /// Whether no callback family is attached.
    pub fn is_empty(&self) -> bool {
        self.inner.header_transform.is_none()
            && self.inner.on_payload.is_none()
            && self.inner.on_response.is_none()
            && self.inner.on_raw.is_none()
    }
    /// Whether a response or raw observer is attached.
    pub fn has_wire_observers(&self) -> bool {
        self.has_response_observer() || self.inner.on_raw.is_some()
    }
    pub(crate) fn has_response_observer(&self) -> bool {
        self.inner.on_response.is_some()
    }

    /// Apply a synchronous header transform, freezing its result for this call.
    /// An async-only transform requires [`Self::apply_header_transform_async`].
    pub fn apply_header_transform(
        &self,
        merged: &[(String, String)],
    ) -> Result<HeaderList, ProviderError> {
        let transform = match self.inner.header_transform.as_ref() {
            None => return Ok(merged.to_vec()),
            Some(HeaderTransform::Sync(callback)) => callback,
            Some(HeaderTransform::Async(_)) => return Err(async_required("header transform")),
        };
        let _execution = self
            .state
            .execution
            .try_lock()
            .map_err(|_| execution_in_progress())?;
        if let Some(frozen) = self
            .state
            .headers
            .lock()
            .expect("frozen headers poisoned")
            .as_ref()
        {
            return frozen.clone();
        }
        let result = transform(merged)
            .map_err(|error| error.into_provider_error("header transform"))
            .and_then(|output| validate_headers(merged, output));
        *self.state.headers.lock().expect("frozen headers poisoned") = Some(result.clone());
        result
    }

    /// Apply and await the assembled-header transform once for this logical call.
    pub async fn apply_header_transform_async(
        &self,
        merged: &[(String, String)],
    ) -> Result<HeaderList, ProviderError> {
        let Some(transform) = self.inner.header_transform.as_ref() else {
            return Ok(merged.to_vec());
        };
        let _execution = self.state.execution.lock().await;
        if let Some(frozen) = self
            .state
            .headers
            .lock()
            .expect("frozen headers poisoned")
            .as_ref()
        {
            return frozen.clone();
        }
        let output = match transform {
            HeaderTransform::Sync(callback) => callback(merged),
            HeaderTransform::Async(callback) => callback(merged.to_vec()).await,
        };
        let result = output
            .map_err(|error| error.into_provider_error("header transform"))
            .and_then(|output| validate_headers(merged, output));
        *self.state.headers.lock().expect("frozen headers poisoned") = Some(result.clone());
        result
    }

    /// Apply a synchronous payload callback and validate/freeze its variant.
    /// An async-only callback requires [`Self::apply_payload_async`].
    pub fn apply_payload(
        &self,
        variant: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, ProviderError> {
        let callback = match self.inner.on_payload.as_ref() {
            None => return Ok(body),
            Some(Payload::Sync(callback)) => callback,
            Some(Payload::Async(_)) => return Err(async_required("payload")),
        };
        let _execution = self
            .state
            .execution
            .try_lock()
            .map_err(|_| execution_in_progress())?;
        if let Some(frozen) = self
            .state
            .payloads
            .lock()
            .expect("frozen payloads poisoned")
            .get(variant)
        {
            return frozen.clone();
        }
        let result = callback(&body)
            .map_err(|error| error.into_provider_error("payload"))
            .and_then(|replacement| validate_and_replace(body, replacement));
        self.state
            .payloads
            .lock()
            .expect("frozen payloads poisoned")
            .insert(variant.to_owned(), result.clone());
        result
    }

    /// Await, validate and freeze the payload callback once per admitted variant.
    pub async fn apply_payload_async(
        &self,
        variant: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, ProviderError> {
        let Some(callback) = self.inner.on_payload.as_ref() else {
            return Ok(body);
        };
        let _execution = self.state.execution.lock().await;
        if let Some(frozen) = self
            .state
            .payloads
            .lock()
            .expect("frozen payloads poisoned")
            .get(variant)
        {
            return frozen.clone();
        }
        let replacement = match callback {
            Payload::Sync(callback) => callback(&body),
            Payload::Async(callback) => callback(body.clone()).await,
        };
        let result = replacement
            .map_err(|error| error.into_provider_error("payload"))
            .and_then(|replacement| validate_and_replace(body, replacement));
        self.state
            .payloads
            .lock()
            .expect("frozen payloads poisoned")
            .insert(variant.to_owned(), result.clone());
        result
    }

    /// Notify a synchronous response observer; async-only observers fail typed.
    pub fn notify_response(
        &self,
        status: u16,
        headers: &[(String, String)],
    ) -> Result<(), ProviderError> {
        match self.inner.on_response.as_ref() {
            None => Ok(()),
            Some(Response::Sync(callback)) => {
                callback(status, headers).map_err(|error| error.into_provider_error("response"))
            }
            Some(Response::Async(_)) => Err(async_required("response")),
        }
    }
    /// Await the functional response observer at the protocol's handoff point.
    pub async fn notify_response_async(
        &self,
        status: u16,
        headers: &[(String, String)],
    ) -> Result<(), ProviderError> {
        let result = match self.inner.on_response.as_ref() {
            None => return Ok(()),
            Some(Response::Sync(callback)) => callback(status, headers),
            Some(Response::Async(callback)) => callback(status, headers.to_vec()).await,
        };
        result.map_err(|error| error.into_provider_error("response"))
    }
    /// Notify a synchronous raw observer; async-only observers fail typed.
    pub fn notify_raw(&self, event: &serde_json::Value) -> Result<(), ProviderError> {
        match self.inner.on_raw.as_ref() {
            None => Ok(()),
            Some(Raw::Sync(callback)) => {
                callback(event).map_err(|error| error.into_provider_error("raw"))
            }
            Some(Raw::Async(_)) => Err(async_required("raw")),
        }
    }
    /// Await each raw observation before its normalized event is produced.
    pub async fn notify_raw_async(&self, event: &serde_json::Value) -> Result<(), ProviderError> {
        let result = match self.inner.on_raw.as_ref() {
            None => return Ok(()),
            Some(Raw::Sync(callback)) => callback(event),
            Some(Raw::Async(callback)) => callback(event.clone()).await,
        };
        result.map_err(|error| error.into_provider_error("raw"))
    }
}

fn async_required(phase: &str) -> ProviderError {
    ProviderError::CallbackPhase(ProviderErrorSummary::sanitized(format!(
        "{phase} callback requires asynchronous invocation"
    )))
}
fn execution_in_progress() -> ProviderError {
    ProviderError::CallbackPhase(ProviderErrorSummary::attested_static(
        "a callback is already executing for this logical call",
    ))
}
fn validate_headers(
    merged: &[(String, String)],
    output: HeaderList,
) -> Result<HeaderList, ProviderError> {
    crate::provider_headers::validate_transform_output(&output, merged).map_err(|error| {
        ProviderError::CallbackPhase(ProviderErrorSummary::sanitized(format!(
            "header transform output is invalid: {error}"
        )))
    })?;
    Ok(output)
}
fn validate_and_replace(
    body: serde_json::Value,
    replacement: Option<serde_json::Value>,
) -> Result<serde_json::Value, ProviderError> {
    match replacement {
        Some(replacement) => {
            validate_payload_replacement(&body, &replacement)?;
            Ok(replacement)
        }
        None => Ok(body),
    }
}

fn validate_payload_replacement(
    original: &serde_json::Value,
    replacement: &serde_json::Value,
) -> Result<(), ProviderError> {
    let fail = |rule: &str| {
        ProviderError::CallbackPhase(ProviderErrorSummary::sanitized(format!(
            "payload callback replacement violates the {rule}"
        )))
    };
    if !replacement.is_object() {
        return Err(fail("JSON object wire shape"));
    }
    if original.get("model") != replacement.get("model") {
        return Err(fail("frozen model identity"));
    }
    const BUDGET_PATHS: [&str; 5] = [
        "/max_tokens",
        "/max_completion_tokens",
        "/max_output_tokens",
        "/generationConfig/maxOutputTokens",
        "/inferenceConfig/maxTokens",
    ];
    let ceiling = BUDGET_PATHS
        .iter()
        .filter_map(|path| original.pointer(path).and_then(serde_json::Value::as_u64))
        .min();
    for path in BUDGET_PATHS {
        match replacement.pointer(path) {
            Some(value) => {
                let value = value
                    .as_u64()
                    .ok_or_else(|| fail("integer output budget wire shape"))?;
                if ceiling.is_some_and(|ceiling| value > ceiling) {
                    return Err(fail("resolved output budget ceiling"));
                }
            }
            None if original.pointer(path).is_some() => {
                return Err(fail("resolved output budget ceiling"));
            }
            None => {}
        }
    }
    Ok(())
}
