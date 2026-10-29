//! Fixed-`O` operation APIs beyond chat.
//!
//! [`ImageApi`] and [`ClassifierApi`] mirror [`crate::models::ChatApi`]:
//! object-safe adapter traits with one typed native-options type `O`,
//! registered through [`crate::models::Models::register_image_api`] /
//! [`crate::models::Models::register_classifier_api`] and dispatched with
//! the caller's complete [`crate::images::ImageModelRecord`] /
//! [`crate::classifier::ClassifierModelRecord`]. The module family keeps
//! operation-specific adapters (images, classifiers) beside the chat seam
//! without folding them into the chat wire types: one shared conformance
//! spans all real classifier protocols through the same seam.

use std::future::Future;
use std::pin::Pin;

use crate::auth::ResolvedAuth;
use crate::classifier::{
    ClassifierCall, ClassifierContext, ClassifierModelRecord, ClassifierResult,
};
use crate::images::{ImageModelRecord, ImagesCall, ImagesContext, ImagesResult};
use crate::models::ApiId;

/// A boxed image-generation future, the object-safe return of
/// [`ImageApi::generate_images_prepared`].
pub type ImageFuture<'a> = Pin<Box<dyn Future<Output = ImagesResult> + Send + 'a>>;

/// One image-generation adapter with its fixed, typed native options type
/// `O`.
///
/// Object-safe: registrations hold `Arc<dyn ImageApi<O>>`. The adapter owns
/// its exact [`ApiId`] (the pi image API string, e.g. `openrouter-images`),
/// its provider identity, its complete record and the generation dispatch.
/// Implementations never resolve auth themselves: the resolved auth arrives
/// prepared, exactly like the chat seam's `stream_prepared`.
pub trait ImageApi<O: Send + 'static>: Send + Sync {
    /// The exact image API identity this adapter serves.
    fn api_id(&self) -> &ApiId;

    /// The provider route this adapter owns.
    fn provider_id(&self) -> &str;

    /// The adapter's complete record. Serves documentation and callers that
    /// need a starting record; it never enters chat catalog reads.
    fn complete_record(&self) -> &ImageModelRecord;

    /// The adapter's typed options used by the ordinary entry.
    fn default_options(&self) -> O;

    /// Validate the native options before auth. Default accepts everything;
    /// adapters with invariants override this.
    fn validate_options(&self, _options: &O) -> Result<(), crate::provider::ProviderError> {
        Ok(())
    }

    /// Dispatch one generation with resolved authentication. Provider
    /// failures stay inside the returned [`ImagesResult`]; this method never
    /// panics and never consults a resolver.
    fn generate_images_prepared<'a>(
        &'a self,
        record: ImageModelRecord,
        context: ImagesContext,
        options: O,
        auth: ResolvedAuth,
        call: ImagesCall,
    ) -> ImageFuture<'a>;
}

/// A boxed classify future, the object-safe return of
/// [`ClassifierApi::classify_prepared`].
pub type ClassifierFuture<'a> = Pin<Box<dyn Future<Output = ClassifierResult> + Send + 'a>>;

/// One classifier adapter with its fixed, typed native options type `O`.
///
/// Object-safe: registrations hold `Arc<dyn ClassifierApi<O>>`. The adapter
/// owns its exact [`ApiId`] (the pi classifier API string, e.g.
/// `typesafe-system-one`), its provider identity, its complete record and
/// the classify dispatch. Implementations never resolve auth themselves:
/// the resolved auth arrives prepared, exactly like the chat seam's
/// `stream_prepared`. One shared conformance spans every real classifier
/// protocol through this trait.
pub trait ClassifierApi<O: Send + 'static>: Send + Sync {
    /// The exact classifier API identity this adapter serves.
    fn api_id(&self) -> &ApiId;

    /// The provider route this adapter owns.
    fn provider_id(&self) -> &str;

    /// The adapter's complete record. Serves documentation and callers that
    /// need a starting record; it never enters chat catalog reads.
    fn complete_record(&self) -> &ClassifierModelRecord;

    /// The adapter's typed options used by the ordinary entry.
    fn default_options(&self) -> O;

    /// Validate the native options before auth. Default accepts everything;
    /// adapters with invariants override this.
    fn validate_options(&self, _options: &O) -> Result<(), crate::provider::ProviderError> {
        Ok(())
    }

    /// Dispatch one classification with resolved authentication. Provider
    /// failures stay inside the returned [`ClassifierResult`]; this method
    /// never panics and never consults a resolver.
    fn classify_prepared<'a>(
        &'a self,
        record: ClassifierModelRecord,
        context: ClassifierContext,
        options: O,
        auth: ResolvedAuth,
        call: ClassifierCall,
    ) -> ClassifierFuture<'a>;
}

pub mod cloudflare;
pub mod openrouter_images;
pub mod system_one;
