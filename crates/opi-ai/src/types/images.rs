//! Image input, output, model metadata and result values.

use super::ApiId;
use super::usage::Usage;
use crate::errors::ProviderError;

/// One ordered image-generation input: text or a base64-encoded image.
#[derive(Debug, Clone, PartialEq)]
pub enum ImagesInput {
    /// Plain text prompt segment.
    Text { text: String },
    /// Base64-encoded image bytes with their MIME type (for example
    /// `image/png`). The data stays encoded exactly as supplied.
    Image { mime_type: String, data: String },
}

/// The complete image-generation input.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImagesContext {
    /// Ordered prompt segments.
    pub input: Vec<ImagesInput>,
}

/// One ordered image-generation output: text or a data-URL-decoded image.
#[derive(Debug, Clone, PartialEq)]
pub enum ImagesOutput {
    /// Text the model produced.
    Text { text: String },
    /// Image the model produced: the provider's base64 payload stays encoded
    /// exactly as returned, together with its MIME type.
    Image { mime_type: String, data: String },
}

/// Terminal state of one image-generation call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImagesStopReason {
    /// The call completed (including text-only, empty-image and zero-choice
    /// responses, which stay successes per the pinned adapter source).
    Stop,
    /// The provider call failed; `error`/`error_message` carry the cause.
    Error,
    /// The caller cancelled the call.
    Aborted,
}

/// Per-million base rates used for result pricing.
///
/// Only the base rates apply: the pinned OpenRouter image adapter prices with
/// the record's flat rates (its "base-cost exception"); tiered pricing never
/// enters the image path. A model with unknown rates has `cost: None`;
/// explicit zero rates describe known free usage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageCostRates {
    /// USD per million input tokens.
    pub input: f64,
    /// USD per million output tokens.
    pub output: f64,
    /// USD per million cache-read tokens.
    pub cache_read: f64,
    /// USD per million cache-write tokens.
    pub cache_write: f64,
}

impl Default for ImageCostRates {
    fn default() -> Self {
        Self {
            input: 0.0,
            output: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
        }
    }
}

/// USD cost computed from the record's base rates and the reported usage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImagesCost {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    /// Sum of the four buckets.
    pub total: f64,
}

/// One output modality an image model supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageOutputModality {
    /// The model can return text blocks alongside images.
    Text,
    /// The model returns images (always supported).
    Image,
}

/// The complete image-model record one generation call binds to.
///
/// The record carries the exact image API identity (pi string, e.g.
/// `openrouter-images`), the owning provider, the endpoint base URL, the
/// output modalities and the base pricing rates. It is the operation-
/// qualified counterpart of the chat seam's complete record: chat models
/// never satisfy image selection, and image records never enter the chat
/// catalog.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageModelRecord {
    /// Owning provider id (for example `openrouter`).
    pub provider_id: String,
    /// Bare model id.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Exact image API identity (pi string).
    pub api: ApiId,
    /// Endpoint base URL (trailing slashes are stripped when joining).
    pub base_url: String,
    /// Declared input modalities, retained as source metadata.
    pub input: Vec<crate::types::catalog::CatalogInputModality>,
    /// Ordered ordinary header overrides; `None` removes an earlier value.
    pub headers: Vec<(String, Option<String>)>,
    /// Source input-limit metadata; it does not impose SDK resize policy.
    pub input_limits: Option<serde_json::Value>,
    /// Supported output modalities, retained as metadata. OpenRouter requests
    /// image then text when text is present, otherwise image only, regardless
    /// of metadata order or duplicates.
    pub output: Vec<ImageOutputModality>,
    /// Base per-million pricing rates, or `None` when catalog pricing is unknown.
    pub cost: Option<ImageCostRates>,
    /// Catalog tiers retained as metadata. Image generation uses only `cost`
    /// base rates, even when the request exceeds one of these thresholds.
    pub cost_tiers: Vec<crate::types::model::PricingTier>,
}

/// The image-generation result: pi `AssistantImages` semantics.
///
/// Provider failures never surface as `Err`: they keep the record identity,
/// `stop_reason = Error`/`Aborted`, a sanitized display message and the typed
/// [`ProviderError`] cause. Failures before successful JSON acquisition have
/// empty output. A response-observer failure or cancellation after acquisition
/// retains received output, response id and known usage/cost; unknown facts
/// remain unknown.
#[derive(Debug, Clone)]
pub struct ImagesResult {
    /// Image API identity the record named.
    pub api: String,
    /// Owning provider id.
    pub provider: String,
    /// Model id.
    pub model: String,
    /// Ordered outputs (text first when the provider sends content, then
    /// images, first choice only).
    pub output: Vec<ImagesOutput>,
    /// Provider response id, when reported.
    pub response_id: Option<String>,
    /// Reported usage with the provider's cache-bucket split, when reported.
    pub usage: Option<Usage>,
    /// Base-rate cost when rates and the reported usage are both known.
    pub cost: Option<ImagesCost>,
    /// Terminal state.
    pub stop_reason: ImagesStopReason,
    /// Sanitized display message for `Error`/`Aborted`.
    pub error_message: Option<String>,
    /// Typed cause for `Error`/`Aborted`.
    pub error: Option<ProviderError>,
}

impl ImagesResult {
    /// A pre-dispatch or provider failure result for the given record
    /// identity: empty output, typed cause and display message.
    pub fn failure(
        record: &ImageModelRecord,
        stop_reason: ImagesStopReason,
        error: ProviderError,
        message: String,
    ) -> Self {
        Self {
            api: record.api.as_str().to_owned(),
            provider: record.provider_id.clone(),
            model: record.id.clone(),
            output: Vec::new(),
            response_id: None,
            usage: None,
            cost: None,
            stop_reason,
            error_message: Some(message),
            error: Some(error),
        }
    }
}
