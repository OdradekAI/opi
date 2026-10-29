//! Image-generation contracts: public context, result and model-record types.
//!
//! The image operation is a non-streaming sibling of the chat seam: callers
//! pass an [`ImagesContext`] (ordered text and data-URL image inputs), a
//! complete [`ImageModelRecord`] that names the owning image API, and receive
//! an [`ImagesResult`] that never rejects for provider failures — failures ride
//! `stop_reason = Error`/`Aborted` with a typed [`ProviderError`] and a
//! display message, exactly like pi's `AssistantImages`.
//!
//! No URL fetch, file write, pixel decode or streaming variant exists here:
//! image inputs are base64 data with their MIME type, image outputs preserve
//! the provider's encoded original.

use tokio_util::sync::CancellationToken;

use crate::callbacks::RequestCallbacks;
use crate::models::ApiId;
use crate::provider::ProviderError;
use crate::stream::Usage;
use crate::telemetry::TelemetryContext;

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
/// enters the image path. Unknown catalog pricing stays `0.0`, matching the
/// source generator's fallback, and remains distinct from absent usage.
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
    /// Output modalities. Order is preserved on the wire.
    pub output: Vec<ImageOutputModality>,
    /// Base per-million pricing rates.
    pub cost: ImageCostRates,
}

/// Per-call image-generation values: timeout, cancellation, headers,
/// callbacks and telemetry.
///
/// Mirrors the per-request fields of the chat seam's `Request` that an image
/// call consumes. `timeout` must be positive when present; adapters apply
/// their own source default (the OpenRouter image adapter uses 600000 ms,
/// body-inclusive) when it is `None`.
#[derive(Clone, Default)]
pub struct ImagesCall {
    /// Per-attempt HTTP timeout covering the whole response body read and
    /// parse. `Some(zero)` is a typed pre-dispatch rejection.
    pub timeout: Option<std::time::Duration>,
    /// Cancellation token; cancelling maps the result to `Aborted`.
    pub cancel: CancellationToken,
    /// Extra HTTP headers appended to the outbound request.
    pub extra_headers: Vec<(String, String)>,
    /// Functional callbacks (payload/response observers). Adapter-unsupported
    /// families are rejected typed before dispatch.
    pub callbacks: RequestCallbacks,
    /// Explicit frozen telemetry carrier (task 20.54). Passes through to the
    /// adapter's request path; never auto-captures.
    pub telemetry_context: Option<TelemetryContext>,
}

impl std::fmt::Debug for ImagesCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImagesCall")
            .field("timeout", &self.timeout)
            .field("extra_headers", &self.extra_headers.len())
            .field("callbacks", &self.callbacks)
            .finish_non_exhaustive()
    }
}

/// The image-generation result: pi `AssistantImages` semantics.
///
/// Provider failures never surface as `Err`: they keep the record identity,
/// an empty output, `stop_reason = Error`/`Aborted`, a sanitized display
/// message and the typed [`ProviderError`] cause.
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
    /// Base-rate cost computed from the record and usage, when usage exists.
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
