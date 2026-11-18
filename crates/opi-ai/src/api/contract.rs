//! Canonical operation contracts used by Models and protocol adapters.
//!
//! Provider profiles and wire implementations are separate owners: this module
//! defines the typed operation seam and neutral chat options only.

use std::any::{Any, TypeId};
use std::marker::PhantomData;
use std::time::Duration;

use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

use crate::auth::ResolvedAuth;
use crate::callbacks::RequestCallbacks;
use crate::provider::{EventStream, ModelInfo, ProviderError, ProviderErrorSummary, Request};
use crate::telemetry::TelemetryContext;
use crate::types::ApiId;
use crate::types::deferred::{DeferredError, DeferredHandle};

// Typed custom chat API registration (AIR-045)
// ---------------------------------------------------------------------------

/// Neutral options accepted by [`Models::stream_simple`] /
/// [`Models::complete_simple`].
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

/// One custom chat adapter with its fixed, typed native options type `O`.
///
/// Object-safe: registrations hold `Arc<dyn ChatApi<O>>`. The adapter owns
/// its exact [`ApiId`], its provider route identity, its complete record, and
/// the neutral-to-native mapping used by the simple entry.
pub trait ChatApi<O: Send + 'static>: Send + Sync {
    /// Validate explicit neutral options before auth. Adapters opt in only to
    /// options their simple mapping consumes; defaults reject new wire inputs.
    fn validate_simple_options(
        &self,
        _model: &ModelInfo,
        options: &SimpleOptions,
    ) -> Result<(), ProviderError> {
        options.validate_default()
    }
    /// Validate the fields consumed by this API's native entry before auth.
    /// Native adapters that replace neutral fields validate their own options
    /// and retain the common structural/model checks.
    fn validate_native_request(
        &self,
        model: &ModelInfo,
        request: &Request,
        _options: &O,
    ) -> Result<(), ProviderError> {
        crate::provider::validate_request_for_model(self.provider_id(), Some(model), request)
    }

    /// Dispatch with the caller's frozen complete record. The default keeps
    /// existing custom adapters source-compatible; metadata-aware adapters
    /// consume this record rather than their construction-time exemplar.
    fn stream_model_resource_prepared(
        &self,
        _model: ModelInfo,
        request: Request,
        options: O,
        auth: ResolvedAuth,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        self.stream_resource_prepared(request, options, auth, resources)
    }

    /// Map the neutral options consumed by this registered API.
    fn simple_options_for(&self, _options: &SimpleOptions) -> O {
        self.simple_options()
    }

    /// Instance-owned connection resources, when this API has a transport pool.
    fn session_resources(&self) -> Option<&dyn crate::codex_resources::SessionResourceOwner> {
        None
    }

    /// Consume a generation registered before authentication. Resource-owning
    /// APIs forward it to their real adapter; other APIs use the ordinary seam.
    fn stream_resource_prepared(
        &self,
        request: Request,
        options: O,
        auth: ResolvedAuth,
        _resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        self.stream_prepared(request, options, auth)
    }
    /// The exact API identity this adapter serves.
    fn api_id(&self) -> &ApiId;

    /// The provider route this adapter owns.
    fn provider_id(&self) -> &str;

    /// The adapter's complete record. Serves documentation and callers that
    /// need a starting record; it never enters catalog reads.
    fn complete_record(&self) -> ModelInfo;

    /// The adapter's neutral options mapping used by [`Models::stream_simple`].
    fn simple_options(&self) -> O;

    /// Dispatch one prepared request with resolved authentication. This is
    /// the same prepared-auth seam [`Provider::stream_prepared`] uses; the
    /// adapter consumes the supplied [`ResolvedAuth`] directly and never
    /// consults a resolver.
    fn stream_prepared(&self, request: Request, options: O, auth: ResolvedAuth) -> EventStream;
}

/// Handle to one registered [`ChatApi`]; the only way to create its typed
/// [`NativeOptions`].
///
/// The handle carries no instance capability: its options stay valid for any
/// [`Models`] with a compatible registration (same exact [`ApiId`], same
/// options type), and a compatible registration upsert never invalidates
/// them.
pub struct ChatApiHandle<O: Send + 'static> {
    api_id: ApiId,
    _options: PhantomData<fn() -> O>,
}

impl<O: Send + 'static> ChatApiHandle<O> {
    pub(crate) fn new(api_id: ApiId) -> Self {
        Self {
            api_id,
            _options: PhantomData,
        }
    }

    /// The exact API identity of the registration that produced this handle.
    pub fn api_id(&self) -> &ApiId {
        &self.api_id
    }

    /// Wrap one options value, tagging it with the registration's API/type
    /// identity so portability and mismatch checks stay truthful.
    pub fn options(&self, value: O) -> NativeOptions<O> {
        NativeOptions {
            api_id: self.api_id.clone(),
            type_id: TypeId::of::<O>(),
            value,
        }
    }
}

/// Typed native options created by a [`ChatApiHandle`].
///
/// Carries the API/type tag of its creating registration, not an instance
/// capability: the same value may cross [`Models`] instances whose
/// registrations share the exact [`ApiId`] and options type.
pub struct NativeOptions<O: Send + 'static> {
    api_id: ApiId,
    type_id: TypeId,
    value: O,
}

impl<O: Send + 'static> NativeOptions<O> {
    /// The exact API identity tagged at creation.
    pub fn api_id(&self) -> &ApiId {
        &self.api_id
    }

    pub(crate) fn into_call_options(self) -> CallOptions {
        CallOptions::Native {
            type_id: self.type_id,
            value: Box::new(self.value),
        }
    }
}

impl<O: Send + 'static + Clone> Clone for NativeOptions<O> {
    fn clone(&self) -> Self {
        Self {
            api_id: self.api_id.clone(),
            type_id: self.type_id,
            value: self.value.clone(),
        }
    }
}

impl<O: Send + 'static> std::fmt::Debug for NativeOptions<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeOptions")
            .field("api_id", &self.api_id)
            .finish()
    }
}

pub(crate) enum CallOptions {
    Simple(SimpleOptions),
    Native {
        type_id: TypeId,
        value: Box<dyn Any + Send>,
    },
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
                crate::provider::ProviderErrorSummary::attested_static(
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
