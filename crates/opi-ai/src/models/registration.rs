//! Registration records and typed dispatch adapters for the [`Models`](crate::models::Models) facade.

use std::any::{Any, TypeId};
use std::marker::PhantomData;
use std::sync::Arc;

use futures_util::stream;

use crate::api::contract::{
    ChatApi, DeferredApi, DeferredCancelCall, DeferredFetchCall, SimpleOptions,
};
use crate::auth::{AuthResolver, ResolvedAuth};
use crate::provider::{
    EventStream, ModelInfo, Provider, ProviderError, ProviderErrorSummary, Request,
};
use crate::types::{ApiId, deferred::DeferredHandle};

/// Handle to one registered [`crate::api::ImageApi`]; the only way to create
/// its typed [`ImageNativeOptions`].
///
/// The handle carries no instance capability: its options stay valid for any
/// [`Models`] with a compatible registration (same exact [`ApiId`], same
/// options type), and a compatible registration upsert never invalidates
/// them.
pub struct ImageApiHandle<O: Send + 'static> {
    pub(super) api_id: ApiId,
    pub(super) _options: PhantomData<fn() -> O>,
}

impl<O: Send + 'static> ImageApiHandle<O> {
    /// The exact API identity of the registration that produced this handle.
    pub fn api_id(&self) -> &ApiId {
        &self.api_id
    }

    /// Wrap one options value, tagging it with the registration's API/type
    /// identity.
    pub fn options(&self, value: O) -> ImageNativeOptions<O> {
        ImageNativeOptions {
            api_id: self.api_id.clone(),
            type_id: TypeId::of::<O>(),
            value,
        }
    }
}

/// Typed native image options created by an [`ImageApiHandle`].
pub struct ImageNativeOptions<O: Send + 'static> {
    pub(super) api_id: ApiId,
    pub(super) type_id: TypeId,
    pub(super) value: O,
}

impl<O: Send + 'static + Clone> Clone for ImageNativeOptions<O> {
    fn clone(&self) -> Self {
        Self {
            api_id: self.api_id.clone(),
            type_id: self.type_id,
            value: self.value.clone(),
        }
    }
}

impl<O: Send + 'static> std::fmt::Debug for ImageNativeOptions<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageNativeOptions")
            .field("api_id", &self.api_id)
            .finish()
    }
}

/// Type-erased storage for one registered image adapter.
#[derive(Clone)]
pub(crate) struct ImageApiRegistration {
    pub(super) builtin_auth: bool,
    pub(super) provider_id: String,
    pub(super) options_type: TypeId,
    pub(super) dispatch: Arc<dyn ImageApiDispatch>,
    pub(super) resolver: Arc<dyn AuthResolver>,
    pub(super) records: Vec<crate::images::ImageModelRecord>,
}

/// Object-safe dispatch over one fixed image options type.
pub(super) trait ImageApiDispatch: Send + Sync {
    fn default_options_erased(&self) -> Box<dyn std::any::Any + Send>;
    fn validate_erased(
        &self,
        record: &crate::images::ImageModelRecord,
        context: &crate::images::ImagesContext,
        options: &(dyn std::any::Any + Send),
        call: &crate::images::ImagesCall,
    ) -> Result<(), ProviderError>;
    fn generate_erased<'a>(
        &'a self,
        record: crate::images::ImageModelRecord,
        context: crate::images::ImagesContext,
        options: Box<dyn std::any::Any + Send>,
        auth: crate::auth::ResolvedAuth,
        call: crate::images::ImagesCall,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::images::ImagesResult> + Send + 'a>>;
}

/// Typed wrapper storing one fixed-`O` image adapter.
pub(super) struct TypedImageApi<O: Send + 'static> {
    pub(super) api: Arc<dyn crate::api::ImageApi<O>>,
}

impl<O: Send + 'static> ImageApiDispatch for TypedImageApi<O> {
    fn default_options_erased(&self) -> Box<dyn std::any::Any + Send> {
        Box::new(self.api.default_options())
    }

    fn validate_erased(
        &self,
        record: &crate::images::ImageModelRecord,
        context: &crate::images::ImagesContext,
        options: &(dyn std::any::Any + Send),
        call: &crate::images::ImagesCall,
    ) -> Result<(), ProviderError> {
        let options = options
            .downcast_ref::<O>()
            .expect("registration owns its options type");
        self.api.validate_request(record, context, options, call)
    }

    fn generate_erased<'a>(
        &'a self,
        record: crate::images::ImageModelRecord,
        context: crate::images::ImagesContext,
        options: Box<dyn std::any::Any + Send>,
        auth: crate::auth::ResolvedAuth,
        call: crate::images::ImagesCall,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::images::ImagesResult> + Send + 'a>>
    {
        let Ok(options) = options.downcast::<O>() else {
            unreachable!("dispatch only receives its registration's options type");
        };
        Box::pin(async move {
            self.api
                .generate_images_prepared(record, context, *options, auth, call)
                .await
        })
    }
}

/// The no-registration failure result, matching pi's dispatch message.
pub(super) fn no_image_api_result(
    record: &crate::images::ImageModelRecord,
) -> crate::images::ImagesResult {
    crate::images::ImagesResult::failure(
        record,
        crate::images::ImagesStopReason::Error,
        crate::provider::ProviderError::Config(crate::provider::ProviderErrorSummary::sanitized(
            format!(
                "No API provider registered for api: {}",
                record.api.as_str()
            ),
        )),
        format!(
            "No API provider registered for api: {}",
            record.api.as_str()
        ),
    )
}

/// Handle to one registered [`crate::api::ClassifierApi`]; the only way to
/// create its typed [`ClassifierNativeOptions`].
///
/// The handle carries no instance capability: its options stay valid for any
/// [`Models`] with a compatible registration (same exact [`ApiId`], same
/// options type), and a compatible registration upsert never invalidates
/// them.
pub struct ClassifierApiHandle<O: Send + 'static> {
    pub(super) api_id: ApiId,
    pub(super) _options: PhantomData<fn() -> O>,
}

impl<O: Send + 'static> ClassifierApiHandle<O> {
    /// The exact API identity of the registration that produced this handle.
    pub fn api_id(&self) -> &ApiId {
        &self.api_id
    }

    /// Wrap one options value, tagging it with the registration's API/type
    /// identity.
    pub fn options(&self, value: O) -> ClassifierNativeOptions<O> {
        ClassifierNativeOptions {
            api_id: self.api_id.clone(),
            type_id: TypeId::of::<O>(),
            value,
        }
    }
}

/// Typed native classifier options created by a [`ClassifierApiHandle`].
pub struct ClassifierNativeOptions<O: Send + 'static> {
    pub(super) api_id: ApiId,
    pub(super) type_id: TypeId,
    pub(super) value: O,
}

impl<O: Send + 'static + Clone> Clone for ClassifierNativeOptions<O> {
    fn clone(&self) -> Self {
        Self {
            api_id: self.api_id.clone(),
            type_id: self.type_id,
            value: self.value.clone(),
        }
    }
}

impl<O: Send + 'static> std::fmt::Debug for ClassifierNativeOptions<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClassifierNativeOptions")
            .field("api_id", &self.api_id)
            .finish()
    }
}

/// Type-erased storage for one registered classifier adapter and its model
/// records (the adapter's complete record seeds the set; providers serving
/// multiple models under one API append the rest).
#[derive(Clone)]
pub(crate) struct ClassifierApiRegistration {
    pub(super) builtin_auth: bool,
    pub(super) provider_id: String,
    pub(super) options_type: TypeId,
    pub(super) dispatch: Arc<dyn ClassifierApiDispatch>,
    pub(super) resolver: Arc<dyn AuthResolver>,
    pub(super) records: Vec<crate::classifier::ClassifierModelRecord>,
}

pub(crate) struct DeferredApiRegistration {
    pub(super) api_id: ApiId,
    pub(super) provider_id: String,
    pub(super) model_id: String,
    pub(super) dispatch: Arc<dyn DeferredApiDispatch>,
    pub(super) resolver: Arc<dyn AuthResolver>,
}

/// Object-safe dispatch over one fixed deferred options type.
pub(super) trait DeferredApiDispatch: Send + Sync {
    fn default_options_erased(&self) -> Box<dyn std::any::Any + Send>;
    fn submit_erased(
        &self,
        request: Request,
        options: &(dyn std::any::Any + Send),
        auth: crate::auth::ResolvedAuth,
    ) -> crate::provider::EventStream;
    fn fetch_erased(
        &self,
        handle: &DeferredHandle,
        options: &(dyn std::any::Any + Send),
        auth: crate::auth::ResolvedAuth,
        call: DeferredFetchCall,
    ) -> crate::provider::EventStream;
    fn cancel_erased<'a>(
        &'a self,
        handle: &'a DeferredHandle,
        options: &'a (dyn std::any::Any + Send),
        auth: crate::auth::ResolvedAuth,
        call: DeferredCancelCall,
    ) -> futures_util::future::BoxFuture<'a, Result<(), ProviderError>>;
}

pub(super) struct TypedDeferredApi<O: Send + 'static> {
    pub(super) api: std::sync::Arc<dyn DeferredApi<O>>,
}

impl<O: Send + 'static> DeferredApiDispatch for TypedDeferredApi<O> {
    fn default_options_erased(&self) -> Box<dyn std::any::Any + Send> {
        Box::new(self.api.default_options())
    }

    fn submit_erased(
        &self,
        request: Request,
        options: &(dyn std::any::Any + Send),
        auth: crate::auth::ResolvedAuth,
    ) -> crate::provider::EventStream {
        let Some(options) = options.downcast_ref::<O>() else {
            unreachable!("dispatch only receives its registration's options type")
        };
        self.api.submit_prepared(request, options, auth)
    }

    fn fetch_erased(
        &self,
        handle: &DeferredHandle,
        options: &(dyn std::any::Any + Send),
        auth: crate::auth::ResolvedAuth,
        call: DeferredFetchCall,
    ) -> crate::provider::EventStream {
        let Some(options) = options.downcast_ref::<O>() else {
            unreachable!("dispatch only receives its registration's options type")
        };
        self.api.fetch_prepared(handle, options, auth, call)
    }

    fn cancel_erased<'a>(
        &'a self,
        handle: &'a DeferredHandle,
        options: &'a (dyn std::any::Any + Send),
        auth: crate::auth::ResolvedAuth,
        call: DeferredCancelCall,
    ) -> futures_util::future::BoxFuture<'a, Result<(), ProviderError>> {
        let Some(options) = options.downcast_ref::<O>() else {
            unreachable!("dispatch only receives its registration's options type")
        };
        self.api.cancel_prepared(handle, options, auth, call)
    }
}

/// Object-safe dispatch over one fixed classifier options type.
pub(super) trait ClassifierApiDispatch: Send + Sync {
    fn default_options_erased(&self) -> Box<dyn std::any::Any + Send>;
    fn validate_erased(
        &self,
        record: &crate::classifier::ClassifierModelRecord,
        context: &crate::classifier::ClassifierContext,
        options: &(dyn std::any::Any + Send),
        call: &crate::classifier::ClassifierCall,
    ) -> Result<(), ProviderError>;
    fn classify_erased<'a>(
        &'a self,
        record: crate::classifier::ClassifierModelRecord,
        context: crate::classifier::ClassifierContext,
        options: Box<dyn std::any::Any + Send>,
        auth: crate::auth::ResolvedAuth,
        call: crate::classifier::ClassifierCall,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::classifier::ClassifierResult> + Send + 'a>,
    >;
}

/// Typed wrapper storing one fixed-`O` classifier adapter.
pub(super) struct TypedClassifierApi<O: Send + 'static> {
    pub(super) api: Arc<dyn crate::api::ClassifierApi<O>>,
}

impl<O: Send + 'static> ClassifierApiDispatch for TypedClassifierApi<O> {
    fn default_options_erased(&self) -> Box<dyn std::any::Any + Send> {
        Box::new(self.api.default_options())
    }

    fn validate_erased(
        &self,
        record: &crate::classifier::ClassifierModelRecord,
        context: &crate::classifier::ClassifierContext,
        options: &(dyn std::any::Any + Send),
        call: &crate::classifier::ClassifierCall,
    ) -> Result<(), ProviderError> {
        let Some(options) = options.downcast_ref::<O>() else {
            unreachable!("dispatch validates the registered options type");
        };
        self.api.validate_request(record, context, options, call)
    }

    fn classify_erased<'a>(
        &'a self,
        record: crate::classifier::ClassifierModelRecord,
        context: crate::classifier::ClassifierContext,
        options: Box<dyn std::any::Any + Send>,
        auth: crate::auth::ResolvedAuth,
        call: crate::classifier::ClassifierCall,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::classifier::ClassifierResult> + Send + 'a>,
    > {
        let Ok(options) = options.downcast::<O>() else {
            unreachable!("dispatch only receives its registration's options type");
        };
        Box::pin(async move {
            self.api
                .classify_prepared(record, context, *options, auth, call)
                .await
        })
    }
}

/// The no-registration failure result.
pub(super) fn no_classifier_api_result(
    record: &crate::classifier::ClassifierModelRecord,
) -> crate::classifier::ClassifierResult {
    crate::classifier::ClassifierResult::failure(
        record,
        crate::classifier::ClassifierStopReason::Error,
        crate::provider::ProviderError::Config(crate::provider::ProviderErrorSummary::sanitized(
            format!(
                "no classifier implementation for api: {}",
                record.api.as_str()
            ),
        )),
        format!(
            "no classifier implementation for api: {}",
            record.api.as_str()
        ),
    )
}

/// Type-erased storage for one registered adapter.
pub(crate) struct ChatApiRegistration {
    pub(super) provider_id: String,
    pub(super) options_type: TypeId,
    pub(super) dispatch: Arc<dyn ChatApiDispatch>,
    // The namespace follows this exact adapter revision, independently of a
    // later provider-route replacement under the same provider id.
    pub(super) resource_owner: Option<Arc<dyn crate::codex_resources::SessionResourceOwner>>,
}

/// The registration snapshot captured by a call at creation.
pub(super) struct ChatApiEntry {
    pub(super) provider_id: String,
    pub(super) options_type: TypeId,
    pub(super) dispatch: Arc<dyn ChatApiDispatch>,
}

/// Object-safe dispatch over one fixed options type.
pub(super) trait ChatApiDispatch: Send + Sync {
    fn validate_simple_erased(
        &self,
        model: &ModelInfo,
        options: &SimpleOptions,
    ) -> Result<(), ProviderError>;
    fn prepare_simple_erased(&self, model: &ModelInfo, request: &mut Request);
    fn skips_shared_request_preparation(&self, model: &ModelInfo) -> bool;
    fn simple_options_erased(&self, options: &SimpleOptions) -> Box<dyn Any + Send>;
    fn validate_native_erased(
        &self,
        model: &ModelInfo,
        request: &Request,
        options: &(dyn Any + Send),
    ) -> Result<(), ProviderError>;
    fn stream_prepared_erased(
        &self,
        model: ModelInfo,
        request: Request,
        options: Box<dyn Any + Send>,
        auth: ResolvedAuth,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream;
}

pub(super) struct TypedChatApi<O: Send + 'static> {
    pub(super) api: Arc<dyn ChatApi<O>>,
}

impl<O: Send + 'static> ChatApiDispatch for TypedChatApi<O> {
    fn validate_simple_erased(
        &self,
        model: &ModelInfo,
        options: &SimpleOptions,
    ) -> Result<(), ProviderError> {
        self.api.validate_simple_options(model, options)
    }
    fn prepare_simple_erased(&self, model: &ModelInfo, request: &mut Request) {
        self.api.prepare_simple_request(model, request)
    }
    fn skips_shared_request_preparation(&self, model: &ModelInfo) -> bool {
        self.api.skips_shared_request_preparation(model)
    }
    fn simple_options_erased(&self, options: &SimpleOptions) -> Box<dyn Any + Send> {
        Box::new(self.api.simple_options_for(options))
    }

    fn validate_native_erased(
        &self,
        model: &ModelInfo,
        request: &Request,
        options: &(dyn Any + Send),
    ) -> Result<(), ProviderError> {
        let options = options.downcast_ref::<O>().ok_or_else(|| {
            ProviderError::Config(ProviderErrorSummary::attested_static(
                "native options type does not match the registered chat api",
            ))
        })?;
        self.api.validate_native_request(model, request, options)
    }

    fn stream_prepared_erased(
        &self,
        model: ModelInfo,
        request: Request,
        options: Box<dyn Any + Send>,
        auth: ResolvedAuth,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        match options.downcast::<O>() {
            Ok(options) => self
                .api
                .stream_model_resource_prepared(model, request, *options, auth, resources),
            Err(_) => Box::pin(stream::once(async {
                Err(ProviderError::Config(
                    ProviderErrorSummary::attested_static(
                        "native options type does not match the registered chat api",
                    ),
                ))
            })),
        }
    }
}

/// Catalog-invisible provider route serving one registered adapter through
/// provider-route entries (the simple entry dispatches here and the adapter's
/// `simple_options` mapping supplies its typed options).
pub(super) struct ChatApiRouteProvider {
    pub(super) provider_id: String,
    pub(super) dispatch: Arc<dyn ChatApiDispatch>,
}

impl Provider for ChatApiRouteProvider {
    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        options: &SimpleOptions,
    ) -> Result<(), ProviderError> {
        self.dispatch.validate_simple_erased(model, options)
    }

    fn prepare_simple_request(&self, model: &ModelInfo, request: &mut Request) {
        self.dispatch.prepare_simple_erased(model, request)
    }

    fn skips_shared_request_preparation(&self, model: &ModelInfo) -> bool {
        self.dispatch.skips_shared_request_preparation(model)
    }
    fn id(&self) -> &str {
        &self.provider_id
    }

    fn models(&self) -> &[ModelInfo] {
        // Catalog-invisible by contract: no records are listed or resolvable.
        &[]
    }

    fn stream_prepared(&self, _request: Request, _auth: ResolvedAuth) -> EventStream {
        Box::pin(stream::once(async move {
            Err(ProviderError::Config(
                ProviderErrorSummary::attested_static(
                    "registered chat APIs require a complete model record through Models",
                ),
            ))
        }))
    }

    fn stream_model_resource_attempt(
        &self,
        model: ModelInfo,
        simple: SimpleOptions,
        request: Request,
        auth: ResolvedAuth,
        _attempt: crate::provider::AttemptContext,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        let options = self.dispatch.simple_options_erased(&simple);
        self.dispatch
            .stream_prepared_erased(model, request, options, auth, resources)
    }
}
