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
use crate::provider::ProviderError;
use crate::telemetry::TelemetryContext;

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

pub(crate) fn validate_image_request(
    record: &ImageModelRecord,
    call: &ImagesCall,
) -> Result<(), ProviderError> {
    crate::api::shared::headers::ProviderHeaders::default()
        .merge_model_request(&record.headers, &[], &call.extra_headers)
        .map(|_| ())
}

pub use crate::types::images::*;
