//! Functional request callbacks (task 20.51, AIR-059/V-51 local slice).
//!
//! Four callback families ride an ordinary [`crate::provider::Request`] and
//! fire at source-defined timing inside the real direct adapters — never by
//! re-deriving wire facts from a normalized [`crate::provider::EventStream`]:
//!
//! - `transform_headers` — applied exactly once per dispatch to the merged
//!   per-call header set (configured profile, route-managed, per-request).
//!   Protected identity/auth/signing names are never part of that set; the
//!   transform output is revalidated and may not introduce a protected name,
//!   duplicate a name, or carry an invalid name/value. Removing or rewriting a
//!   non-protected entry is the supported omission/set/removal behavior.
//! - `on_payload` — invoked after sampling/compat/wire projection with the
//!   final request body. `Ok(None)` keeps it; `Ok(Some(v))` replaces it after
//!   validation (JSON object, unchanged model identity, output budget not
//!   raised). The callback runs once per payload variant; an ordinary
//!   acquisition retry re-enters the adapter with the same variant and reuses
//!   the frozen body byte-for-byte.
//! - `on_response` — invoked with the HTTP status and response headers once a
//!   response has been received, before any stream event is normalized
//!   downstream. Failures that never produce a response (connect errors,
//!   timeouts) are not notified.
//! - `on_raw` — invoked with each parsed provider wire event, in wire order,
//!   strictly before its normalized emission. Read-only; a failure terminates
//!   the current call with a typed error while preserving the already
//!   delivered partial.
//!
//! Google wires (Gemini/Vertex) support the payload and header-transform
//! families but reject response/raw observers with a typed
//! [`crate::provider::ProviderError::UnsupportedCapability`] before dispatch.
//!
//! Callback failures are functional failures: they surface as
//! [`crate::provider::ProviderError::CallbackPhase`] (never retryable, never
//! swallowed by telemetry, never triggering a new call).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::provider::{ProviderError, ProviderErrorSummary};

/// The payload-variant label of the ordinary call path. Strict variants
/// (task 20.11) add their own labels; an ordinary acquisition retry reuses
/// this variant's frozen payload.
pub const ORDINARY_PAYLOAD_VARIANT: &str = "ordinary";

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
/// returns the replacement set. Protected names cannot appear in the output.
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

#[derive(Default)]
struct RequestCallbacksInner {
    header_transform: Option<HeaderTransformFn>,
    on_payload: Option<PayloadFn>,
    on_response: Option<ResponseFn>,
    on_raw: Option<RawFn>,
    /// Frozen per-variant bodies produced by the payload callback. Shared
    /// through `Request` clones, so an ordinary acquisition retry (which
    /// re-clones the frozen request) reuses the frozen body without
    /// re-invoking the callback.
    frozen_payloads: Mutex<HashMap<String, serde_json::Value>>,
}

/// The functional-callback set carried by [`crate::provider::Request`].
///
/// Cloning shares the callback set and the frozen payload map (the value is an
/// `Arc`), which is exactly the once-per-variant freezing contract across
/// ordinary acquisition retries of one logical call. Constructing a second
/// `RequestCallbacks` for a new logical call starts a fresh, empty set.
#[derive(Clone, Default)]
pub struct RequestCallbacks {
    inner: Arc<RequestCallbacksInner>,
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
    /// An empty callback set (the default).
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach a header-transform callback (consuming builder).
    pub fn with_header_transform(mut self, transform: HeaderTransformFn) -> Self {
        Arc::get_mut(&mut self.inner)
            .expect("RequestCallbacks builders run before the set is shared")
            .header_transform = Some(transform);
        self
    }

    /// Attach a payload callback (consuming builder).
    pub fn with_on_payload(mut self, on_payload: PayloadFn) -> Self {
        Arc::get_mut(&mut self.inner)
            .expect("RequestCallbacks builders run before the set is shared")
            .on_payload = Some(on_payload);
        self
    }

    /// Attach a response callback (consuming builder).
    pub fn with_on_response(mut self, on_response: ResponseFn) -> Self {
        Arc::get_mut(&mut self.inner)
            .expect("RequestCallbacks builders run before the set is shared")
            .on_response = Some(on_response);
        self
    }

    /// Attach a raw-event callback (consuming builder).
    pub fn with_on_raw(mut self, on_raw: RawFn) -> Self {
        Arc::get_mut(&mut self.inner)
            .expect("RequestCallbacks builders run before the set is shared")
            .on_raw = Some(on_raw);
        self
    }

    /// Whether no callback is attached. Adapters use this as the zero-cost
    /// fast path.
    pub fn is_empty(&self) -> bool {
        self.inner.header_transform.is_none()
            && self.inner.on_payload.is_none()
            && self.inner.on_response.is_none()
            && self.inner.on_raw.is_none()
    }

    /// Whether a response or raw observer is attached. Google wires reject
    /// these observers with a typed unsupported-capability failure before
    /// dispatch.
    pub fn has_wire_observers(&self) -> bool {
        self.inner.on_response.is_some() || self.inner.on_raw.is_some()
    }

    /// Apply the header transform to the merged per-call header set.
    ///
    /// The output replaces the merged set after revalidation: names must be
    /// valid, unique, and non-protected (`RESERVED_PROVIDER_HEADERS`, the
    /// `amz-sdk-*` signing family, case-insensitive). Validation failure is a
    /// typed callback-phase failure; without a transform the merged set is
    /// returned unchanged.
    pub fn apply_header_transform(
        &self,
        merged: &[(String, String)],
    ) -> Result<Vec<(String, String)>, ProviderError> {
        let Some(transform) = self.inner.header_transform.as_ref() else {
            return Ok(merged.to_vec());
        };
        let output =
            transform(merged).map_err(|error| error.into_provider_error("header transform"))?;
        crate::provider_headers::validate_transform_output(&output).map_err(|error| {
            ProviderError::CallbackPhase(ProviderErrorSummary::sanitized(format!(
                "header transform output is invalid: {error}"
            )))
        })?;
        Ok(output)
    }

    /// Apply the payload callback for `variant` to the post-projection
    /// `body`.
    ///
    /// Runs the callback at most once per variant; later same-variant calls
    /// (ordinary acquisition retries through the shared `Request`) return the
    /// frozen body without re-invoking the callback. A replacement must be a
    /// JSON object, must not retarget the model identity, and must not raise
    /// the resolved output budget; violations are typed callback failures.
    pub fn apply_payload(
        &self,
        variant: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, ProviderError> {
        let Some(on_payload) = self.inner.on_payload.as_ref() else {
            return Ok(body);
        };
        if let Some(frozen) = self
            .inner
            .frozen_payloads
            .lock()
            .expect("frozen payload map poisoned")
            .get(variant)
        {
            return Ok(frozen.clone());
        }
        let replacement =
            match on_payload(&body).map_err(|error| error.into_provider_error("payload"))? {
                Some(replacement) => {
                    validate_payload_replacement(&body, &replacement)?;
                    replacement
                }
                // `Ok(None)` keeps the projected body; freeze it so a same-variant
                // retry does not re-invoke the callback.
                None => body,
            };
        self.inner
            .frozen_payloads
            .lock()
            .expect("frozen payload map poisoned")
            .insert(variant.to_string(), replacement.clone());
        Ok(replacement)
    }

    /// Notify the response observer with the received status and headers.
    pub fn notify_response(
        &self,
        status: u16,
        headers: &[(String, String)],
    ) -> Result<(), ProviderError> {
        let Some(on_response) = self.inner.on_response.as_ref() else {
            return Ok(());
        };
        on_response(status, headers).map_err(|error| error.into_provider_error("response"))
    }

    /// Notify the raw observer with one parsed provider wire event.
    pub fn notify_raw(&self, event: &serde_json::Value) -> Result<(), ProviderError> {
        let Some(on_raw) = self.inner.on_raw.as_ref() else {
            return Ok(());
        };
        on_raw(event).map_err(|error| error.into_provider_error("raw"))
    }
}

/// Post-transform wire validation: the replacement must stay a JSON object,
/// keep the model identity, and not raise the resolved output budget.
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
    if let (Some(original_model), Some(replacement_model)) = (
        original.get("model").and_then(|value| value.as_str()),
        replacement.get("model").and_then(|value| value.as_str()),
    ) && original_model != replacement_model
    {
        return Err(fail("frozen model identity"));
    }
    if let (Some(original_limit), Some(replacement_limit)) = (
        original.get("max_tokens").and_then(|value| value.as_u64()),
        replacement
            .get("max_tokens")
            .and_then(|value| value.as_u64()),
    ) && replacement_limit > original_limit
    {
        return Err(fail("resolved output budget ceiling"));
    }
    if let (Some(original_limit), Some(replacement_limit)) = (
        original
            .get("max_output_tokens")
            .and_then(|value| value.as_u64()),
        replacement
            .get("max_output_tokens")
            .and_then(|value| value.as_u64()),
    ) && replacement_limit > original_limit
    {
        return Err(fail("resolved output budget ceiling"));
    }
    Ok(())
}
