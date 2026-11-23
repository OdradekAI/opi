//! Registration records and typed dispatch adapters for the [`Models`](crate::models::Models) facade.

use super::*;

use std::any::{Any, TypeId};
use std::marker::PhantomData;
use std::sync::Arc;

use futures_util::StreamExt;
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
/// [`Models`](crate::Models) with a compatible registration (same exact [`ApiId`], same
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
/// [`Models`](crate::Models) with a compatible registration (same exact [`ApiId`], same
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

impl Models {
    /// Register a lookup-only provider with its redacted auth descriptor and
    /// compatibility metadata.
    ///
    /// Replaces any existing entry with the same provider id and removes its
    /// dispatch resolver. Use [`register_route`](Self::register_route) for a
    /// route that may start model attempts.
    ///
    /// # Errors
    ///
    /// Propagates [`RegistrationError::EmptyProviderId`] from the registry.
    pub fn register(
        &mut self,
        provider: Box<dyn Provider>,
        auth: AuthDescriptor,
        compat: CompatMetadata,
    ) -> Result<(), RegistrationError> {
        let id = provider.id().to_owned();
        let resource_owner = provider
            .session_resources()
            .map(|owner| owner.new_namespace());
        self.registry.registry.register_provider(provider)?;
        self.clear_catalog_entry(&id);
        self.replace_resource_owner(&id, resource_owner);
        self.auth_owner.builtin_default_auth.remove(&id);
        self.auth_owner.resolvers.remove(&id);
        self.auth_owner.oauth_routes.remove(&id);
        self.auth_owner.sources.remove(&id);
        self.auth_owner.probed.remove(&id);
        self.auth_owner.auth.insert(id.clone(), auth);
        self.compat.insert(id, compat);
        Ok(())
    }

    /// Register a dispatchable route: a concrete provider plus its per-request
    /// [`AuthResolver`] and non-secret source classification.
    ///
    /// The provider is registered in the underlying registry; the resolver and
    /// source classification are retained so
    /// [`prepare_call`](Self::prepare_call) can resolve and freeze authentication
    /// for the route. Replaces any existing route with the same provider id.
    ///
    /// # Errors
    ///
    /// Propagates [`RegistrationError::EmptyProviderId`] from the registry.
    pub fn register_route(
        &mut self,
        provider: Box<dyn Provider>,
        resolver: Arc<dyn AuthResolver>,
        source: AuthProvenanceSource,
        compat: CompatMetadata,
    ) -> Result<(), RegistrationError> {
        let id = provider.id().to_owned();
        let resource_owner = provider
            .session_resources()
            .map(|owner| owner.new_namespace());
        self.registry.registry.register_provider(provider)?;
        self.clear_catalog_entry(&id);
        self.replace_resource_owner(&id, resource_owner);
        self.auth_owner.builtin_default_auth.remove(&id);
        self.auth_owner.oauth_routes.remove(&id);
        self.auth_owner.auth.remove(&id);
        self.auth_owner.probed.remove(&id);
        self.auth_owner.resolvers.insert(id.clone(), resolver);
        self.auth_owner.sources.insert(id.clone(), source);
        self.compat.insert(id, compat);
        Ok(())
    }

    /// Register one typed custom chat adapter (AIR-045).
    ///
    /// The adapter is keyed by its exact [`ApiId`] with its fixed options type
    /// `O`; empty and whitespace identities remain distinct valid keys.
    /// Registration installs a **catalog-invisible** provider route (the
    /// adapter's complete record never appears in `get_model`/`all_models`)
    /// plus the route's [`AuthResolver`], so every entry — native, simple,
    /// direct — reaches the same frozen binding and collector. Registering
    /// again with the same ApiId replaces the entry (compatible upsert);
    /// options created before the upsert keep working when the options type is
    /// unchanged and fail a typed pre-auth check when it is not.
    ///
    /// # Errors
    ///
    /// - [`RegistrationError::EmptyProviderId`] if the provider id is empty.
    pub fn register_chat_api<O: Send + 'static>(
        &mut self,
        api: Arc<dyn ChatApi<O>>,
        resolver: Arc<dyn AuthResolver>,
        source: AuthProvenanceSource,
    ) -> Result<ChatApiHandle<O>, RegistrationError> {
        let api_id = api.api_id().clone();
        let provider_id = api.provider_id().to_owned();
        if provider_id.trim().is_empty() {
            return Err(RegistrationError::EmptyProviderId);
        }
        let resource_owner = api.session_resources().map(|owner| owner.new_namespace());
        let dispatch: Arc<dyn ChatApiDispatch> = Arc::new(TypedChatApi { api });
        // Catalog-invisible route: a Provider with no models keeps the adapter
        // dispatchable through provider-route entries without listing it.
        self.registry
            .registry
            .register_provider(Box::new(ChatApiRouteProvider {
                provider_id: provider_id.clone(),
                dispatch: Arc::clone(&dispatch),
            }))?;
        self.clear_catalog_entry(&provider_id);
        self.replace_resource_owner(&provider_id, resource_owner.clone());
        self.auth_owner.builtin_default_auth.remove(&provider_id);
        self.auth_owner.auth.remove(&provider_id);
        self.auth_owner.oauth_routes.remove(&provider_id);
        self.auth_owner.probed.remove(&provider_id);
        self.auth_owner
            .resolvers
            .insert(provider_id.clone(), resolver);
        self.auth_owner.sources.insert(provider_id.clone(), source);
        self.compat.remove(&provider_id);
        self.operation_owner.chat_apis.insert(
            api_id.clone(),
            ChatApiRegistration {
                provider_id,
                options_type: TypeId::of::<O>(),
                dispatch,
                resource_owner,
            },
        );
        Ok(ChatApiHandle::new(api_id))
    }

    /// Register one typed image adapter (task 20.45): the fixed-`O`
    /// [`crate::api::ImageApi`] implementation, its provider auth resolver and provenance
    /// source. The registration owns the adapter's complete
    /// [`crate::images::ImageModelRecord`] for operation-qualified selection; image records
    /// never enter the chat catalog. An upsert for the same provider and
    /// [`ApiId`] replaces that registration wholesale.
    pub fn register_image_api<O: Send + 'static>(
        &mut self,
        api: std::sync::Arc<dyn crate::api::ImageApi<O>>,
        resolver: std::sync::Arc<dyn AuthResolver>,
        source: AuthProvenanceSource,
    ) -> Result<ImageApiHandle<O>, RegistrationError> {
        let api_id = api.api_id().clone();
        let provider_id = api.provider_id().to_owned();
        if provider_id.trim().is_empty() {
            return Err(RegistrationError::EmptyProviderId);
        }
        let record = api.complete_record().clone();
        self.auth_owner.sources.insert(provider_id.clone(), source);
        self.catalog_owner.publication_generation = Arc::new(());
        self.operation_owner.image_apis.insert(
            (provider_id.clone(), api_id.clone()),
            ImageApiRegistration {
                builtin_auth: false,
                provider_id,
                options_type: std::any::TypeId::of::<O>(),
                dispatch: Arc::new(TypedImageApi { api }),
                resolver,
                records: vec![record],
            },
        );
        Ok(ImageApiHandle {
            api_id,
            _options: PhantomData,
        })
    }

    /// Append or replace an image model within its exact provider/API registration.
    pub fn register_image_model_record(
        &mut self,
        record: crate::images::ImageModelRecord,
    ) -> Result<(), ModelsError> {
        let Some(registration) = self
            .operation_owner
            .image_apis
            .get_mut(&(record.provider_id.clone(), record.api.clone()))
        else {
            return Err(ModelsError::ImageApiNotRegistered {
                api: record.api.as_str().to_owned(),
            });
        };
        match registration
            .records
            .iter()
            .position(|existing| existing.id == record.id)
        {
            Some(index) => registration.records[index] = record,
            None => registration.records.push(record),
        }
        Ok(())
    }

    /// Register one typed classifier adapter (task 20.46): the fixed-`O`
    /// [`crate::api::ClassifierApi`] implementation, its provider auth
    /// resolver and provenance source. The registration owns the adapter's
    /// complete [`crate::classifier::ClassifierModelRecord`] for
    /// operation-qualified selection; classifier records never enter the
    /// chat catalog, and an upsert with the same provider and exact [`ApiId`]
    /// replaces that registration wholesale. One shared conformance spans
    /// every real classifier protocol through this seam.
    pub fn register_classifier_api<O: Send + 'static>(
        &mut self,
        api: std::sync::Arc<dyn crate::api::ClassifierApi<O>>,
        resolver: std::sync::Arc<dyn AuthResolver>,
        source: AuthProvenanceSource,
    ) -> Result<ClassifierApiHandle<O>, RegistrationError> {
        let api_id = api.api_id().clone();
        let provider_id = api.provider_id().to_owned();
        if provider_id.trim().is_empty() {
            return Err(RegistrationError::EmptyProviderId);
        }
        let record = api.complete_record().clone();
        self.auth_owner.sources.insert(provider_id.clone(), source);
        self.catalog_owner.publication_generation = Arc::new(());
        self.operation_owner.classifier_apis.insert(
            (provider_id.clone(), api_id.clone()),
            ClassifierApiRegistration {
                builtin_auth: false,
                provider_id,
                options_type: std::any::TypeId::of::<O>(),
                dispatch: Arc::new(TypedClassifierApi { api }),
                resolver,
                records: vec![record],
            },
        );
        Ok(ClassifierApiHandle {
            api_id,
            _options: PhantomData,
        })
    }

    /// Append (or replace, keyed by provider/model id) one additional
    /// classifier record to the registration matching its provider and `record.api`.
    ///
    /// One provider serves many classifier models under one API (pi's
    /// Cloudflare Workers AI family); registration replaces the model set
    /// wholesale, and a record whose API has no registration is a typed
    /// [`ModelsError::ClassifierApiNotRegistered`] rejection.
    pub fn register_classifier_model_record(
        &mut self,
        record: crate::classifier::ClassifierModelRecord,
    ) -> Result<(), ModelsError> {
        let Some(registration) = self
            .operation_owner
            .classifier_apis
            .get_mut(&(record.provider_id.clone(), record.api.clone()))
        else {
            return Err(ModelsError::ClassifierApiNotRegistered {
                api: record.api.as_str().to_owned(),
            });
        };
        let key = (record.provider_id.clone(), record.id.clone());
        match registration
            .records
            .iter()
            .position(|existing| (existing.provider_id.clone(), existing.id.clone()) == key)
        {
            Some(index) => registration.records[index] = record,
            None => registration.records.push(record),
        }
        Ok(())
    }

    /// Register one typed deferred-response backend (task 20.50): the
    /// fixed-`O` [`DeferredApi`] implementation, its provider auth resolver
    /// and provenance source. The registration owns the backend's route
    /// identity (`ApiId`, provider, model); an upsert with the same exact
    /// [`ApiId`] replaces the previous registration wholesale.
    ///
    /// Deferred submission stays a normal generation through the chat
    /// surface; only the explicit fetch/cancel operations resolve here,
    /// after validating the handle's route identity against this
    /// registration.
    pub fn register_deferred_api<O: Send + 'static>(
        &mut self,
        api: std::sync::Arc<dyn DeferredApi<O>>,
        resolver: std::sync::Arc<dyn AuthResolver>,
        source: AuthProvenanceSource,
    ) -> Result<DeferredApiHandle<O>, RegistrationError> {
        let api_id = api.api_id().clone();
        let provider_id = api.provider_id().to_owned();
        if provider_id.trim().is_empty() {
            return Err(RegistrationError::EmptyProviderId);
        }
        let model_id = api.model_id().to_owned();
        self.auth_owner.sources.insert(provider_id.clone(), source);
        self.catalog_owner.publication_generation = Arc::new(());
        self.operation_owner.deferred_apis.insert(
            api_id.clone(),
            DeferredApiRegistration {
                api_id: api_id.clone(),
                provider_id,
                model_id,
                dispatch: Arc::new(TypedDeferredApi { api }),
                resolver,
            },
        );
        Ok(DeferredApiHandle::new(api_id))
    }

    /// Submit one explicit asynchronous generation through a registered
    /// deferred backend. The request is validated against the backend's fixed
    /// provider/model identity before authentication; no implicit polling or
    /// ordinary-route fallback occurs.
    pub fn stream_deferred_submit(
        &self,
        api: &ApiId,
        request: Request,
    ) -> crate::stream::ChatStream {
        let Some(registration) = self.operation_owner.deferred_apis.get(api) else {
            let api = api.as_str().to_owned();
            return crate::stream::ChatStream::new(Box::pin(stream::once(async move {
                Err::<AssistantStreamEvent, ProviderError>(ProviderError::Config(
                    ProviderErrorSummary::sanitized(format!(
                        "no deferred backend is registered for API '{}'",
                        api
                    )),
                ))
            })));
        };
        let model_id = request
            .model
            .split_once(':')
            .map(|(_, model)| model)
            .unwrap_or(&request.model);
        if request
            .model
            .split_once(':')
            .is_some_and(|(provider, _)| provider != registration.provider_id)
            || model_id != registration.model_id
        {
            let expected = format!("{}:{}", registration.provider_id, registration.model_id);
            return crate::stream::ChatStream::new(Box::pin(stream::once(async move {
                Err::<AssistantStreamEvent, ProviderError>(ProviderError::Config(
                    ProviderErrorSummary::sanitized(format!(
                        "deferred submit request model does not match registered route '{expected}'"
                    )),
                ))
            })));
        }
        let options = registration.dispatch.default_options_erased();
        let resolver = Arc::clone(&registration.resolver);
        let dispatch = Arc::clone(&registration.dispatch);
        let telemetry_context = request.telemetry_context.clone();
        let stream = stream::once(async move {
            match resolver.resolve().await {
                Ok(auth) => {
                    let mut events = dispatch.submit_erased(request, options.as_ref(), auth);
                    let mut collected = Vec::new();
                    while let Some(event) = events.next().await {
                        collected.push(event);
                    }
                    collected
                }
                Err(error) => vec![Err(error)],
            }
        })
        .flat_map(stream::iter);
        let stream = Box::pin(stream) as EventStream;
        let attributes = crate::telemetry::AiRequestAttributes::new(
            registration.provider_id.clone(),
            registration.model_id.clone(),
            registration.api_id.as_str().to_owned(),
        );
        crate::stream::ChatStream::new_with_telemetry(
            stream,
            telemetry_context.as_ref(),
            attributes,
        )
        .with_requested_route(
            registration.provider_id.clone(),
            registration.model_id.clone(),
        )
    }

    /// Submit one explicit asynchronous generation and await its first
    /// normalized outcome. A successful result has `StopReason::Deferred` and
    /// a serializable handle; a provider may also complete immediately.
    pub async fn submit_deferred(
        &self,
        api: &ApiId,
        request: Request,
    ) -> Result<crate::message::AssistantMessage, CallFailure> {
        self.stream_deferred_submit(api, request).result().await
    }

    /// Validate the handle against its exact API and provider registration,
    /// then its model and optional expiry, before authentication (AIR-043).
    /// Returns the typed [`DeferredError`] without resolving credentials
    /// or dispatching a backend operation.
    pub fn validate_deferred_handle(
        &self,
        handle: &DeferredHandle,
        now_ms: Option<u64>,
    ) -> Result<(), DeferredError> {
        let registration = self.deferred_registration(handle)?;
        handle.validate_identity(
            &registration.provider_id,
            &registration.model_id,
            registration.api_id.as_str(),
        )?;
        if let Some(now_ms) = now_ms {
            handle.validate_expiry(now_ms)?;
        }
        Ok(())
    }

    /// Fetch one deferred response as a [`crate::ChatStream`]: validate the handle
    /// (identity, then expiry), resolve authentication through the
    /// backend's registered resolver, and dispatch exactly one fetch.
    ///
    /// Pre-dispatch failures (missing provider/API registration, route mismatch,
    /// expiry, missing credential) surface as one typed `Err` stream item,
    /// which [`crate::ChatStream`] converts into the canonical `CallFailure`.
    /// A missing registration diagnostic identifies the requested provider and API.
    /// When it synthesizes a failure message without a partial, it uses the
    /// handle's provider and model IDs.
    /// `wait_ms = 0` (pi wait0) performs a single status check; the SDK
    /// never polls implicitly. When the call carries a telemetry context,
    /// the SDK installs the Pi-shaped `pi.ai.poll` observer at this boundary
    /// and completes it with the same result that [`crate::ChatStream::result`]
    /// returns.
    ///
    /// Direct iteration delivers the terminal as soon as it is cached.
    /// The next read drives the retained source to EOF, releases it and
    /// settles the poll observer before returning public EOF. Cancelling a
    /// pending tail releases the source and preserves the delivered outcome.
    /// Dropping the direct collector after a successful `Done` releases the
    /// source immediately and settles its poll observer with the cached result.
    ///
    /// [`crate::ChatStream::start`] schedules the fetch independently of the
    /// consumer and forwards a successful `Done` before draining the source.
    /// Public EOF and the result await source release and poll settlement.
    /// The task continues driving the source after consumer Drop.
    pub fn stream_deferred(
        &self,
        handle: &DeferredHandle,
        call: DeferredFetchCall,
    ) -> crate::stream::ChatStream {
        let telemetry_context = call.telemetry_context.clone();
        let attributes = crate::telemetry::AiRequestAttributes::new(
            handle.provider.clone(),
            handle.model_id.clone(),
            handle.api.clone(),
        )
        .as_poll(true);
        crate::stream::ChatStream::new_with_telemetry(
            self.stream_deferred_source(handle, call),
            telemetry_context.as_ref(),
            attributes,
        )
        .with_requested_route(handle.provider.clone(), handle.model_id.clone())
        .with_tail_drain_on_completion()
    }

    fn stream_deferred_source(
        &self,
        handle: &DeferredHandle,
        call: DeferredFetchCall,
    ) -> EventStream {
        let validated = self
            .deferred_registration(handle)
            .and_then(|registration| {
                handle
                    .validate_identity(
                        &registration.provider_id,
                        &registration.model_id,
                        registration.api_id.as_str(),
                    )
                    .map(|()| registration)
            })
            .and_then(|registration| match call.now_ms {
                Some(now_ms) => handle.validate_expiry(now_ms).map(|()| registration),
                None => Ok(registration),
            });
        let (registration, options) = match validated {
            Ok(registration) => {
                let options = registration.dispatch.default_options_erased();
                (registration, options)
            }
            Err(error) => {
                let provider_error = error.into_provider_error(&handle.api);
                return Box::pin(stream::once(async move { Err(provider_error) }));
            }
        };
        let resolver = Arc::clone(&registration.resolver);
        let dispatch = Arc::clone(&registration.dispatch);
        let handle = handle.clone();
        let cancel = call.cancel.clone();
        let setup: BoxFuture<'static, EventStream> = Box::pin(async move {
            match resolver.resolve().await {
                Ok(auth) => dispatch.fetch_erased(&handle, options.as_ref(), auth, call),
                Err(error) => Box::pin(stream::once(async move {
                    Err::<AssistantStreamEvent, ProviderError>(error)
                })) as EventStream,
            }
        });
        let fetched = futures_util::FutureExt::flatten_stream(setup);
        // Local abort (AIR-043): cancelling the fetch token ends the local
        // attempt with a typed Cancelled failure and never reaches the
        // remote deferred job.
        if cancel.is_cancelled() {
            return Box::pin(stream::once(async move {
                Err::<AssistantStreamEvent, ProviderError>(ProviderError::Cancelled)
            }));
        }
        Box::pin(stream::unfold(
            (fetched, cancel, false),
            |(mut inner, cancel, done)| async move {
                if done {
                    return None;
                }
                // None (outer) = cancelled; Some(None) = stream end.
                let outcome = {
                    let mut cancelled = std::pin::pin!(cancel.cancelled());
                    let selected = futures_util::future::select(inner.next(), &mut cancelled).await;
                    match selected {
                        futures_util::future::Either::Left((item, _)) => Some(item),
                        futures_util::future::Either::Right(((), _)) => None,
                    }
                };
                match outcome {
                    Some(Some(event)) => Some((event, (inner, cancel, false))),
                    Some(None) => None,
                    None => Some((
                        Err::<AssistantStreamEvent, ProviderError>(ProviderError::Cancelled),
                        (inner, cancel, true),
                    )),
                }
            },
        ))
    }

    /// Fetch one deferred response to the single final outcome: pending
    /// results are `Ok` with [`crate::stream::StopReason::Deferred`] and the
    /// handle on the returned message; ready results deliver the final
    /// assistant message; every failure is one `Err(CallFailure)`.
    /// The returned future drives source release and poll settlement. Use
    /// [`Self::fetch_deferred_ready`] to receive the outcome as soon as the
    /// first terminal is normalized.
    pub async fn fetch_deferred(
        &self,
        handle: &DeferredHandle,
        call: DeferredFetchCall,
    ) -> Result<crate::message::AssistantMessage, CallFailure> {
        self.stream_deferred(handle, call).result().await
    }

    /// Fetch once and return the first normalized outcome while the execution
    /// task drives source release and poll settlement independently.
    ///
    /// Routing, authentication and provider wait use [`Self::stream_deferred`].
    /// A still-pending response returns `Ok` with
    /// [`crate::stream::StopReason::Deferred`] and its handle. A caller
    /// schedules any subsequent fetch. Failures return a typed [`CallFailure`].
    ///
    /// The first poll starts the task. Dropping the waiter after that leaves
    /// the task running until it finishes or its runtime shuts down. The
    /// request's token can stop an in-flight fetch or its tail locally. After the
    /// outcome is cached, local cancellation of the tail preserves that
    /// outcome. Remote cancellation uses [`Self::cancel_deferred`]. Use
    /// [`Self::fetch_deferred`] when the caller needs to await source release
    /// and poll settlement or observe a panic during the drain.
    ///
    /// # Panics
    ///
    /// Panics if polled outside a Tokio runtime. A source panic before the
    /// terminal is resumed in this future. Once the terminal is cached, a
    /// tail panic reaches Tokio's panic hook.
    pub async fn fetch_deferred_ready(
        &self,
        handle: &DeferredHandle,
        call: DeferredFetchCall,
    ) -> Result<crate::message::AssistantMessage, CallFailure> {
        self.stream_deferred(handle, call).first_outcome().await
    }

    /// Cancel one deferred response remotely, best effort (AIR-043/AIR-044).
    ///
    /// The SDK validates the handle's route as it does for fetch. A missing
    /// registration reports the requested provider and API. Backend failures
    /// are returned to the caller. Local fetch cancellation never reaches
    /// this operation.
    pub async fn cancel_deferred(
        &self,
        handle: &DeferredHandle,
        call: DeferredCancelCall,
    ) -> Result<(), ProviderError> {
        let registration = self
            .deferred_registration(handle)
            .map_err(|error| error.into_provider_error(&handle.api))?;
        handle
            .validate_identity(
                &registration.provider_id,
                &registration.model_id,
                registration.api_id.as_str(),
            )
            .map_err(|error| error.into_provider_error(&handle.api))?;
        if call.cancel.is_cancelled() {
            return Err(ProviderError::Cancelled);
        }
        let options = registration.dispatch.default_options_erased();
        let auth = tokio::select! {
            biased;
            _ = call.cancel.cancelled() => return Err(ProviderError::Cancelled),
            auth = registration.resolver.resolve() => auth?,
        };
        if call.cancel.is_cancelled() {
            return Err(ProviderError::Cancelled);
        }
        registration
            .dispatch
            .cancel_erased(handle, options.as_ref(), auth, call)
            .await
    }

    fn deferred_registration(
        &self,
        handle: &DeferredHandle,
    ) -> Result<&DeferredApiRegistration, DeferredError> {
        self.operation_owner
            .deferred_apis
            .get(&ApiId::new(handle.api.clone()))
            .filter(|registration| registration.provider_id == handle.provider)
            .ok_or_else(|| DeferredError::Unsupported {
                provider: handle.provider.clone(),
            })
    }
}
