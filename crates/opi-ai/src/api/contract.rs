//! Canonical operation contracts used by Models and protocol adapters.
//!
//! Provider profiles and wire implementations are separate owners: this module
//! defines the typed operation seam and neutral chat options only.

use std::any::{Any, TypeId};
use std::marker::PhantomData;

use crate::auth::ResolvedAuth;
use crate::provider::{EventStream, ModelInfo, ProviderError, Request};
use crate::types::ApiId;

pub use crate::provider::operation::{
    DeferredApi, DeferredApiHandle, DeferredCancelCall, DeferredFetchCall, SimpleOptions,
    SimpleToolChoice,
};

// Typed custom chat API registration (AIR-045)
// ---------------------------------------------------------------------------

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

    /// Normalize a neutral simple request before shared preparation.
    ///
    /// Adapter routes that preserve the requested thinking level override this
    /// default, while ordinary routes clamp to their catalog's nearest level.
    fn prepare_simple_request(&self, model: &ModelInfo, request: &mut Request) {
        if request.thinking.enabled
            && let Some(level) = model
                .thinking_level_map
                .nearest_supported(request.thinking.level)
        {
            request.thinking.level = level;
        }
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
