//! Deferred-response contracts (task 20.50, AIR-043/AIR-044).
//!
//! Deferred submission is a normal generation: a capable backend returns an
//! assistant message with [`crate::stream::StopReason::Deferred`] plus a
//! [`DeferredHandle`] instead of final content. Fetch and cancellation are
//! explicit operations on that handle — the SDK performs exactly one status
//! check per call at wait0 and never polls implicitly. No real remote
//! deferred protocol is claimed here; the published seam serves scripted
//! and custom backends.

use std::marker::PhantomData;
use std::time::Duration;

use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

use crate::auth::ResolvedAuth;
use crate::callbacks::RequestCallbacks;
use crate::models::ApiId;
use crate::provider::{EventStream, ProviderError};
use crate::telemetry::TelemetryContext;
pub use crate::types::deferred::{DeferredError, DeferredHandle, RouteMismatchDetail};

impl DeferredError {
    /// Map this typed pre-dispatch failure onto the provider error cause
    /// classification used by the stream/fetch/cancel surfaces.
    pub(crate) fn into_provider_error(self) -> ProviderError {
        ProviderError::Config(crate::provider::ProviderErrorSummary::sanitized(
            self.to_string(),
        ))
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
    /// Caller-owned monotonic clock reading used for SDK expiry
    /// validation; `None` skips the SDK check and leaves expiry to the
    /// backend.
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
/// the ordinary `fetch_deferred`/`cancel_deferred` entries dispatch with
/// the registration's default options.
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
