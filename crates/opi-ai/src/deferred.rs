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

/// A deferred-response handle returned by a capable backend (pi
/// `DeferredHandle`).
///
/// Identity is source-owned (`provider`, `model_id`, `api`, `id`); optional
/// expiry and poll hints are provider advice, never an SDK polling
/// schedule. `data` carries provider conversion data needed to reconstruct
/// the final assistant message. The handle never carries a credential.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeferredHandle {
    /// Provider id that produced the deferred response.
    pub provider: String,
    /// Bare model id the deferred generation ran on.
    pub model_id: String,
    /// Wire/api identity the deferred route serves.
    pub api: String,
    /// Provider token, such as a response id or batch id plus row id.
    pub id: String,
    /// Optional absolute expiry in epoch milliseconds (pi field name
    /// `expiresAt`, without a unit suffix).
    #[serde(rename = "expiresAt", default, skip_serializing_if = "Option::is_none")]
    pub expires_at_ms: Option<u64>,
    /// Optional provider poll advice in milliseconds; caller-owned only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poll_after_ms: Option<u64>,
    /// Provider conversion data for the final message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl DeferredHandle {
    /// Validate this handle's route identity against the resolved
    /// (`provider`, `model_id`, `api`) route before any authentication.
    pub fn validate_identity(
        &self,
        provider: &str,
        model_id: &str,
        api: &str,
    ) -> Result<(), DeferredError> {
        if self.provider != provider || self.model_id != model_id || self.api != api {
            return Err(DeferredError::RouteMismatch(Box::new(
                RouteMismatchDetail {
                    handle_provider: self.provider.clone(),
                    handle_model_id: self.model_id.clone(),
                    handle_api: self.api.clone(),
                    route_provider: provider.to_owned(),
                    route_model_id: model_id.to_owned(),
                    route_api: api.to_owned(),
                },
            )));
        }
        Ok(())
    }

    /// Validate expiry against a caller-injected monotonic reading.
    ///
    /// The SDK never reads a clock itself: the caller (agent loop, consumer
    /// fixture) owns time so tests stay deterministic and the SDK gains no
    /// hidden polling or timing behavior.
    pub fn validate_expiry(&self, now_ms: u64) -> Result<(), DeferredError> {
        if let Some(expires_at_ms) = self.expires_at_ms
            && now_ms >= expires_at_ms
        {
            return Err(DeferredError::Expired {
                handle_id: self.id.clone(),
                expires_at_ms,
                now_ms,
            });
        }
        Ok(())
    }
}

/// Identity triple of a handle and the route it failed to match (boxed in
/// [`DeferredError::RouteMismatch`] to keep the error type small).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error(
    "handle {handle_provider}/{handle_model_id}/{handle_api} does not match route {route_provider}/{route_model_id}/{route_api}"
)]
pub struct RouteMismatchDetail {
    /// Handle's provider id.
    pub handle_provider: String,
    /// Handle's bare model id.
    pub handle_model_id: String,
    /// Handle's wire/api identity.
    pub handle_api: String,
    /// Resolved route's provider id.
    pub route_provider: String,
    /// Resolved route's bare model id.
    pub route_model_id: String,
    /// Resolved route's wire/api identity.
    pub route_api: String,
}

/// Typed deferred-operation failures surfaced by the SDK surface.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum DeferredError {
    /// The resolved route does not match the handle's identity.
    #[error("deferred handle route mismatch: {0}")]
    RouteMismatch(#[from] Box<RouteMismatchDetail>),
    /// The handle expired according to the caller-injected clock reading.
    #[error("deferred handle '{handle_id}' expired at {expires_at_ms} (now {now_ms})")]
    Expired {
        handle_id: String,
        expires_at_ms: u64,
        now_ms: u64,
    },
    /// No deferred backend serves the handle's provider (pi: "does not
    /// support deferred responses").
    #[error("provider '{provider}' does not support deferred responses")]
    Unsupported {
        /// Provider id the fetch/cancel targeted.
        provider: String,
    },
}

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
