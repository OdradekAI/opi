//! Operation registration ownership for the [`Models`](crate::models::Models) facade.

use std::collections::BTreeMap;

use super::registration::{
    ChatApiRegistration, ClassifierApiRegistration, DeferredApiRegistration, ImageApiRegistration,
    no_classifier_api_result, no_image_api_result,
};
use super::*;
use crate::types::ApiId;

/// Operation registration owner behind the [`Models`] facade.
///
/// Chat, image, classifier and deferred adapters share exact API identity
/// registration and capture rules, so their maps move together behind one
/// private owner.
pub(crate) struct OperationOwner {
    pub(crate) chat_apis: BTreeMap<ApiId, ChatApiRegistration>,
    pub(crate) image_apis: BTreeMap<(String, ApiId), ImageApiRegistration>,
    pub(crate) classifier_apis: BTreeMap<(String, ApiId), ClassifierApiRegistration>,
    pub(crate) deferred_apis: BTreeMap<ApiId, DeferredApiRegistration>,
}

impl OperationOwner {
    pub(crate) fn new() -> Self {
        Self {
            chat_apis: BTreeMap::new(),
            image_apis: BTreeMap::new(),
            classifier_apis: BTreeMap::new(),
            deferred_apis: BTreeMap::new(),
        }
    }
}

impl Models {
    /// Operation-qualified classifier selection: resolve the registered
    /// classifier record for (`provider`, bare `id`).
    ///
    /// Chat models and unknown ids fail typed here — before any auth
    /// resolution — so a classification can never be routed through a
    /// chat/image model.
    pub fn classifier_model(
        &self,
        provider: &str,
        id: &str,
    ) -> Result<crate::classifier::ClassifierModelRecord, ModelsError> {
        if let Some(entry) = self.catalog_owner.dynamic_catalogs.get(provider) {
            return entry
                .models
                .iter()
                .find_map(|model| match model {
                    crate::catalog::CatalogRecord::Classifier(record)
                        if record.id == id
                            && self
                                .operation_owner
                                .classifier_apis
                                .contains_key(&(provider.to_owned(), record.api.clone())) =>
                    {
                        Some(record.clone())
                    }
                    _ => None,
                })
                .ok_or_else(|| ModelsError::NotAClassifierModel {
                    provider: provider.into(),
                    id: id.into(),
                });
        }
        for registration in self.operation_owner.classifier_apis.values() {
            for record in &registration.records {
                if record.provider_id == provider && record.id == id {
                    return Ok(record.clone());
                }
            }
        }
        Err(ModelsError::NotAClassifierModel {
            provider: provider.to_owned(),
            id: id.to_owned(),
        })
    }

    /// Classify through the registered adapter for the record's exact API
    /// identity using the adapter's default options (the ordinary classify
    /// driver).
    ///
    /// Never rejects: selection, auth and provider failures stay inside the
    /// returned [`crate::classifier::ClassifierResult`] with `stop_reason =
    /// Error`/`Aborted` and the typed cause.
    /// Captures owned inputs and the current registration at creation; auth
    /// starts only when polled. Later registration changes do not retarget it.
    pub fn classify(
        &self,
        record: crate::classifier::ClassifierModelRecord,
        context: crate::classifier::ClassifierContext,
        call: crate::classifier::ClassifierCall,
    ) -> crate::api::ClassifierFuture<'static> {
        let registration = self
            .operation_owner
            .classifier_apis
            .get(&(record.provider_id.clone(), record.api.clone()))
            .cloned();
        Box::pin(async move {
            let Some(registration) = registration else {
                return no_classifier_api_result(&record);
            };
            let options = registration.dispatch.default_options_erased();
            Self::dispatch_classifier(&registration, record, context, options, call).await
        })
    }

    /// Classify with typed native options created by a
    /// [`ClassifierApiHandle`]. The options must carry the same exact
    /// [`ApiId`] and options type as the registration; a mismatch is a typed
    /// pre-dispatch failure result.
    pub fn classify_native<O: Send + 'static>(
        &self,
        record: crate::classifier::ClassifierModelRecord,
        context: crate::classifier::ClassifierContext,
        call: crate::classifier::ClassifierCall,
        options: ClassifierNativeOptions<O>,
    ) -> crate::api::ClassifierFuture<'static> {
        let registration = self
            .operation_owner
            .classifier_apis
            .get(&(record.provider_id.clone(), record.api.clone()))
            .cloned();
        Box::pin(async move {
            let Some(registration) = registration else {
                return no_classifier_api_result(&record);
            };
            if options.type_id != std::any::TypeId::of::<O>()
                || options.api_id != record.api
                || options.type_id != registration.options_type
            {
                let message = format!(
                    "classifier options tagged for API '{}' do not match the registration for '{}'",
                    options.api_id.as_str(),
                    record.api.as_str()
                );
                return crate::classifier::ClassifierResult::failure(
                    &record,
                    crate::classifier::ClassifierStopReason::Error,
                    crate::provider::ProviderError::Config(
                        crate::provider::ProviderErrorSummary::sanitized(message.clone()),
                    ),
                    message,
                );
            }
            Self::dispatch_classifier(
                &registration,
                record,
                context,
                Box::new(options.value),
                call,
            )
            .await
        })
    }

    async fn dispatch_classifier(
        registration: &ClassifierApiRegistration,
        record: crate::classifier::ClassifierModelRecord,
        context: crate::classifier::ClassifierContext,
        options: Box<dyn std::any::Any + Send>,
        call: crate::classifier::ClassifierCall,
    ) -> crate::classifier::ClassifierResult {
        if let Err(error) =
            registration
                .dispatch
                .validate_erased(&record, &context, options.as_ref(), &call)
        {
            let message = match &error {
                ProviderError::UnsupportedCapability(summary) => summary.as_str().to_owned(),
                _ => error.to_string(),
            };
            return crate::classifier::ClassifierResult::failure(
                &record,
                crate::classifier::ClassifierStopReason::Error,
                error,
                message,
            );
        }
        let resolution = tokio::select! {
            biased;
            _ = call.cancel.cancelled() => Err(ProviderError::Cancelled),
            auth = async { registration.resolver.resolve().await } => auth,
        };
        let auth = match resolution {
            Ok(auth) => auth,
            Err(error) => {
                let message = if matches!(
                    error,
                    crate::provider::ProviderError::CredentialNeeded { .. }
                ) {
                    format!("No API key for provider: {}", registration.provider_id)
                } else {
                    error.to_string()
                };
                return crate::classifier::ClassifierResult::failure(
                    &record,
                    if matches!(error, ProviderError::Cancelled) {
                        crate::classifier::ClassifierStopReason::Aborted
                    } else {
                        crate::classifier::ClassifierStopReason::Error
                    },
                    error,
                    message,
                );
            }
        };
        registration
            .dispatch
            .classify_erased(record, context, options, auth, call)
            .await
    }

    /// List classifier records whose selected source is configured, without
    /// refreshing credentials or performing network work.
    pub async fn get_available_classifiers(
        &self,
        provider: Option<&str>,
    ) -> Result<Vec<crate::classifier::ClassifierModelRecord>, ModelsError> {
        let mut records = Vec::new();
        let mut allowed = HashMap::new();
        for ((_, api_id), registration) in &self.operation_owner.classifier_apis {
            let id = &registration.provider_id;
            if provider.is_some_and(|wanted| wanted != id) {
                continue;
            }
            let builtin_check = registration
                .builtin_auth
                .then(|| self.auth_owner.builtin_auth_checks.get(id))
                .flatten();
            let available = if let Some(check) = builtin_check {
                if let Some(value) = allowed.get(id) {
                    *value
                } else {
                    let value = check()
                        .await
                        .map_err(|error| ModelsError::AuthResolution {
                            provider: id.clone(),
                            error: Box::new(error),
                        })?
                        .is_some_and(|status| status.classifiers_allowed);
                    allowed.insert(id.clone(), value);
                    value
                }
            } else {
                match registration.resolver.check_configuration() {
                    Some(check) => check.await.map_err(|error| ModelsError::AuthResolution {
                        provider: id.clone(),
                        error: Box::new(error),
                    })?,
                    None => true,
                }
            };
            if available {
                if let Some(entry) = self.catalog_owner.dynamic_catalogs.get(id) {
                    records.extend(entry.models.iter().filter_map(|model| match model {
                        crate::catalog::CatalogRecord::Classifier(record)
                            if &record.api == api_id =>
                        {
                            Some(record.clone())
                        }
                        _ => None,
                    }));
                } else {
                    records.extend(registration.records.iter().cloned());
                }
            }
        }
        Ok(records)
    }

    /// Operation-qualified image selection: resolve the registered image
    /// record for (`provider`, bare `id`).
    ///
    /// Chat models and unknown ids fail typed here — before any auth
    /// resolution — so an image generation can never be routed through a
    /// chat/classifier model.
    pub fn image_model(
        &self,
        provider: &str,
        id: &str,
    ) -> Result<crate::images::ImageModelRecord, ModelsError> {
        if let Some(entry) = self.catalog_owner.dynamic_catalogs.get(provider) {
            return entry
                .models
                .iter()
                .find_map(|model| match model {
                    crate::catalog::CatalogRecord::Image(record)
                        if record.id == id
                            && self
                                .operation_owner
                                .image_apis
                                .contains_key(&(provider.to_owned(), record.api.clone())) =>
                    {
                        Some(record.clone())
                    }
                    _ => None,
                })
                .ok_or_else(|| ModelsError::NotAnImageModel {
                    provider: provider.into(),
                    id: id.into(),
                });
        }
        for registration in self.operation_owner.image_apis.values() {
            for record in &registration.records {
                if record.provider_id == provider && record.id == id {
                    return Ok(record.clone());
                }
            }
        }
        Err(ModelsError::NotAnImageModel {
            provider: provider.to_owned(),
            id: id.to_owned(),
        })
    }

    /// Generate images through the registered adapter for the record's exact
    /// API identity using the adapter's default options (the ordinary image
    /// driver).
    ///
    /// Never rejects: selection, auth and provider failures stay inside the
    /// returned [`crate::images::ImagesResult`] with `stop_reason = Error`/`Aborted` and the
    /// typed cause.
    /// Captures owned inputs and the current registration at creation; auth
    /// starts only when polled. Later registration changes do not retarget it.
    pub fn generate_images(
        &self,
        record: crate::images::ImageModelRecord,
        context: crate::images::ImagesContext,
        call: crate::images::ImagesCall,
    ) -> crate::api::ImageFuture<'static> {
        let registration = self
            .operation_owner
            .image_apis
            .get(&(record.provider_id.clone(), record.api.clone()))
            .cloned();
        Box::pin(async move {
            let Some(registration) = registration else {
                return no_image_api_result(&record);
            };
            let options = registration.dispatch.default_options_erased();
            Self::dispatch_image(&registration, record, context, options, call).await
        })
    }

    /// Generate images with typed native options created by an
    /// [`ImageApiHandle`]. The options must carry the same exact [`ApiId`]
    /// and options type as the registration; a mismatch is a typed
    /// pre-dispatch failure result.
    pub fn generate_images_native<O: Send + 'static>(
        &self,
        record: crate::images::ImageModelRecord,
        context: crate::images::ImagesContext,
        call: crate::images::ImagesCall,
        options: ImageNativeOptions<O>,
    ) -> crate::api::ImageFuture<'static> {
        let registration = self
            .operation_owner
            .image_apis
            .get(&(record.provider_id.clone(), record.api.clone()))
            .cloned();
        Box::pin(async move {
            let Some(registration) = registration else {
                return no_image_api_result(&record);
            };
            if options.type_id != std::any::TypeId::of::<O>()
                || options.api_id != record.api
                || options.type_id != registration.options_type
            {
                return crate::images::ImagesResult::failure(
                    &record,
                    crate::images::ImagesStopReason::Error,
                    crate::provider::ProviderError::Config(
                        crate::provider::ProviderErrorSummary::sanitized(format!(
                            "image options tagged for API '{}' do not match the registration for '{}'",
                            options.api_id.as_str(),
                            record.api.as_str()
                        )),
                    ),
                    format!(
                        "image options tagged for API '{}' do not match the registration for '{}'",
                        options.api_id.as_str(),
                        record.api.as_str()
                    ),
                );
            }
            Self::dispatch_image(
                &registration,
                record,
                context,
                Box::new(options.value),
                call,
            )
            .await
        })
    }

    async fn dispatch_image(
        registration: &ImageApiRegistration,
        record: crate::images::ImageModelRecord,
        context: crate::images::ImagesContext,
        options: Box<dyn std::any::Any + Send>,
        call: crate::images::ImagesCall,
    ) -> crate::images::ImagesResult {
        if let Err(error) =
            registration
                .dispatch
                .validate_erased(&record, &context, options.as_ref(), &call)
        {
            let message = match &error {
                ProviderError::UnsupportedCapability(summary) => summary.as_str().to_owned(),
                _ => error.to_string(),
            };
            return crate::images::ImagesResult::failure(
                &record,
                crate::images::ImagesStopReason::Error,
                error,
                message,
            );
        }
        let resolution = tokio::select! {
            biased;
            _ = call.cancel.cancelled() => Err(ProviderError::Cancelled),
            auth = async { registration.resolver.resolve().await } => auth,
        };
        let auth = match resolution {
            Ok(auth) => auth,
            Err(error) => {
                let message = if matches!(
                    error,
                    crate::provider::ProviderError::CredentialNeeded { .. }
                ) {
                    format!("No API key for provider: {}", registration.provider_id)
                } else {
                    error.to_string()
                };
                return crate::images::ImagesResult::failure(
                    &record,
                    if matches!(error, ProviderError::Cancelled) {
                        crate::images::ImagesStopReason::Aborted
                    } else {
                        crate::images::ImagesStopReason::Error
                    },
                    error,
                    message,
                );
            }
        };
        registration
            .dispatch
            .generate_erased(record, context, options, auth, call)
            .await
    }
}
