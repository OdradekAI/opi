//! Lower-level operation contracts shared by provider dispatch and API adapters.
//!
//! These values and backend traits sit below the protocol adapter namespace so
//! the provider contract never needs to import `api::contract`.

use std::marker::PhantomData;
use std::time::Duration;

use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

use crate::auth::ResolvedAuth;
use crate::callbacks::RequestCallbacks;
pub use crate::errors::{ProviderError, ProviderErrorSummary};
pub use crate::model_info::ModelInfo;
pub use crate::provider::{EventStream, Request, validate_request_for_model};
use crate::telemetry::TelemetryContext;
use crate::types::ApiId;
use crate::types::deferred::{DeferredError, DeferredHandle};

/// Neutral options accepted by [`Models::stream_simple`](crate::Models::stream_simple) /
/// [`Models::complete_simple`](crate::Models::complete_simple).
///
/// `Some` fields override the corresponding request fields before the shared
/// compiler runs; `None` leaves the request untouched.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SimpleOptions {
    /// Explicit tool selection on an adapter that admits this neutral option.
    pub tool_choice: Option<SimpleToolChoice>,
    /// Sampling temperature override.
    pub temperature: Option<f64>,
    /// Output-limit override.
    pub max_tokens: Option<u64>,
    /// Per-call timeout override.
    pub timeout: Option<Duration>,
    /// Stop-sequence override (applied when non-empty).
    pub stop_sequences: Vec<String>,
}

/// Neutral tool selection. Adapter-specific forced-tool forms remain native.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimpleToolChoice {
    Auto,
    None,
}

impl SimpleOptions {
    pub(crate) fn validate_default(&self) -> Result<(), ProviderError> {
        if self.tool_choice.is_some() {
            return Err(ProviderError::UnsupportedCapability(
                ProviderErrorSummary::attested_static(
                    "this adapter does not support the neutral tool selection option",
                ),
            ));
        }
        Ok(())
    }
    pub(crate) fn apply(&self, request: &mut Request) {
        if self.temperature.is_some() {
            request.temperature = self.temperature;
        }
        if self.max_tokens.is_some() {
            request.max_tokens = self.max_tokens;
        }
        if self.timeout.is_some() {
            request.timeout = self.timeout;
        }
        if !self.stop_sequences.is_empty() {
            request.stop_sequences = self.stop_sequences.clone();
        }
    }
}

// Deferred operation contract
// ---------------------------------------------------------------------------

impl DeferredError {
    /// Map a local handle validation failure to the fetch/cancel cause.
    pub(crate) fn into_provider_error(self, api: &str) -> ProviderError {
        let diagnostic = match self {
            Self::Unsupported { provider } => {
                format!(
                    "no deferred backend is registered for provider '{provider}' and API '{api}'"
                )
            }
            error => error.to_string(),
        };
        ProviderError::Config(crate::provider::ProviderErrorSummary::sanitized(diagnostic))
    }
}

/// Per-call state for one deferred fetch (pi `DeferredFetchOptions`).
///
/// `wait_ms` follows pi semantics: zero (the default) performs exactly one
/// status check; larger values are a provider long-poll budget the backend
/// interprets. The SDK never polls implicitly regardless of this value.
#[derive(Debug, Clone, Default)]
pub struct DeferredFetchCall {
    /// Maximum provider long-poll budget in milliseconds; `0` checks once.
    pub wait_ms: u64,
    /// Caller-supplied time in milliseconds since the Unix epoch, on the
    /// same time base as [`DeferredHandle::expires_at_ms`]. `None` leaves
    /// expiry enforcement to the backend.
    pub now_ms: Option<u64>,
    /// Per-attempt body-inclusive timeout. `None` applies no timeout.
    pub timeout: Option<Duration>,
    /// Cancellation token for the local fetch attempt. Cancelling aborts
    /// the local fetch only; it never cancels the remote deferred job.
    pub cancel: CancellationToken,
    /// Extra HTTP headers appended to the outbound request.
    pub extra_headers: Vec<(String, String)>,
    /// Functional callbacks (payload/response observers).
    pub callbacks: RequestCallbacks,
    /// Explicit frozen telemetry carrier (AIR-054). Passes through to the
    /// backend's request path; never auto-captures.
    pub telemetry_context: Option<TelemetryContext>,
}

/// Per-call state for one best-effort deferred cancellation (pi
/// `DeferredCancelOptions`).
#[derive(Debug, Clone, Default)]
pub struct DeferredCancelCall {
    /// Per-attempt body-inclusive timeout. `None` applies no timeout.
    pub timeout: Option<Duration>,
    /// Cancellation token for the local cancel attempt, independent of any
    /// fetch token (AIR-044 independent cancellation budget).
    pub cancel: CancellationToken,
    /// Extra HTTP headers appended to the outbound request.
    pub extra_headers: Vec<(String, String)>,
    /// Functional callbacks (payload/response observers).
    pub callbacks: RequestCallbacks,
    /// Explicit frozen telemetry carrier (AIR-054). Passes through to the
    /// backend's request path; never auto-captures.
    pub telemetry_context: Option<TelemetryContext>,
}

/// A registered deferred backend serving one provider route (pi
/// `fetchDeferred`/`cancelDeferred` on capable API modules).
///
/// The adapter owns its exact [`ApiId`], its provider route identity, and
/// the model it serves; the SDK validates handle identity against that
/// triple before resolving authentication.
pub trait DeferredApi<O: Send + 'static>: Send + Sync {
    /// Exact API identity of this backend.
    fn api_id(&self) -> &ApiId;
    /// Owning provider id.
    fn provider_id(&self) -> &str;
    /// The single model this backend serves deferred responses for.
    fn model_id(&self) -> &str;
    /// Default options the ordinary deferred entries dispatch with.
    fn default_options(&self) -> O;
    /// Submit one asynchronous generation. The request carries the complete
    /// normal chat input, cancellation and callback state; the backend maps
    /// a successful submission to a terminal `StopReason::Deferred` event.
    ///
    /// This is an explicit caller-owned entry. Ordinary provider generation
    /// remains synchronous unless the caller selects this operation.
    fn submit_prepared(
        &self,
        _request: crate::provider::Request,
        _options: &O,
        _auth: ResolvedAuth,
    ) -> EventStream {
        Box::pin(futures_util::stream::once(async {
            Err(ProviderError::Config(
                ProviderErrorSummary::attested_static(
                    "this deferred backend does not implement submission",
                ),
            ))
        }))
    }
    /// Fetch the deferred response once. Still-pending outcomes are a
    /// delivered terminal with [`crate::stream::StopReason::Deferred`];
    /// ready outcomes deliver the final assistant message.
    fn fetch_prepared(
        &self,
        handle: &DeferredHandle,
        options: &O,
        auth: ResolvedAuth,
        call: DeferredFetchCall,
    ) -> EventStream;
    /// Best-effort remote cancellation. Failure is reported, never
    /// guaranteed.
    fn cancel_prepared<'a>(
        &'a self,
        handle: &'a DeferredHandle,
        options: &'a O,
        auth: ResolvedAuth,
        call: DeferredCancelCall,
    ) -> BoxFuture<'a, Result<(), ProviderError>>;
}

/// Typed registration receipt for one deferred backend.
///
/// The options type parameter records the backend's native options type;
/// ordinary `fetch_deferred`/`cancel_deferred` entries dispatch with the
/// registration's default options.
pub struct DeferredApiHandle<O: Send + 'static> {
    /// Exact API identity the registration owns.
    pub api_id: ApiId,
    _options: PhantomData<O>,
}

impl<O: Send + 'static> DeferredApiHandle<O> {
    pub(crate) fn new(api_id: ApiId) -> Self {
        Self {
            api_id,
            _options: PhantomData,
        }
    }
}

impl<O: Send + 'static> Clone for DeferredApiHandle<O> {
    fn clone(&self) -> Self {
        Self {
            api_id: self.api_id.clone(),
            _options: PhantomData,
        }
    }
}

impl<O: Send + 'static> std::fmt::Debug for DeferredApiHandle<O> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DeferredApiHandle")
            .field("api_id", &self.api_id)
            .finish()
    }
}
