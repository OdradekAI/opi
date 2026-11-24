//! Classifier contracts: public context, result and model-record types.
//!
//! The classifier operation is a non-streaming sibling of the chat seam:
//! callers pass a [`ClassifierContext`] (a JSON state object, typed questions
//! and optional images), a complete [`ClassifierModelRecord`] that names the
//! owning classifier API, and receive a [`ClassifierResult`] that never
//! rejects for provider failures — failures ride `stop_reason =
//! Error`/`Aborted` with a typed [`ProviderError`] and a display message,
//! exactly like pi's `ClassifierResult`.
//!
//! Validation stays source-required only: answer parsing checks record
//! shape, wire type agreement, string choices and finite numbers. No
//! universal probability-range, sum, argmax or label-membership rule exists
//! here; per-protocol adapters own their extra rules.

use tokio_util::sync::CancellationToken;

use crate::callbacks::RequestCallbacks;
use crate::provider::ProviderError;
#[cfg(any(feature = "builtins", feature = "provider-protocol-common"))]
use crate::stream::Usage;
use crate::telemetry::TelemetryContext;

/// Per-call classifier values: timeout, cancellation, headers, callbacks and
/// telemetry.
///
/// Mirrors the per-request fields of the chat seam's `Request` that a
/// classifier call consumes. `timeout` covers the whole response body read
/// and parse and is **fresh for every retry attempt**; there is no default
/// timeout (the pinned source leaves `timeoutMs` undefined by default).
#[derive(Clone, Default)]
pub struct ClassifierCall {
    /// Per-attempt body-inclusive timeout. `None` applies no timeout.
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

impl std::fmt::Debug for ClassifierCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClassifierCall")
            .field("timeout", &self.timeout)
            .field("extra_headers", &self.extra_headers.len())
            .field("callbacks", &self.callbacks)
            .finish_non_exhaustive()
    }
}

/// Validate the operation's frozen record and caller-managed headers before
/// authentication. Protocol adapters add their own narrower input rules.
pub(crate) fn validate_classifier_request(
    record: &ClassifierModelRecord,
    context: &ClassifierContext,
    call: &ClassifierCall,
) -> Result<(), ProviderError> {
    if context
        .images
        .as_ref()
        .is_some_and(|images| !images.is_empty())
        && !record.input.contains(&ClassifierInputModality::Image)
    {
        return Err(ProviderError::UnsupportedCapability(
            crate::provider::ProviderErrorSummary::attested_static(
                "classifier model does not support image input",
            ),
        ));
    }
    crate::api::shared::headers::ProviderHeaders::default()
        .merge_model_request(&record.headers, &[], &call.extra_headers)
        .map(|_| ())
}

/// Classifier protocols report input/output parent counts without cache
/// buckets. Preserve usable counts while marking a partial report unknown;
/// a malformed or absent bucket must not become a reliable reported zero.
#[cfg(any(feature = "builtins", feature = "provider-protocol-common"))]
pub(crate) fn parse_classifier_usage(value: Option<&serde_json::Value>) -> Option<Usage> {
    let value = value?.as_object()?;
    let count = |name: &str| {
        value
            .get(name)
            .and_then(serde_json::Value::as_u64)
            .and_then(|count| u32::try_from(count).ok())
    };
    let input = count("input_tokens");
    let output = count("output_tokens");
    if input.is_none() && output.is_none() {
        return None;
    }
    let reported = input.is_some() && output.is_some();
    Some(Usage {
        input_tokens: input.unwrap_or_default(),
        output_tokens: output.unwrap_or_default(),
        total_tokens: input
            .zip(output)
            .map(|(input, output)| u64::from(input) + u64::from(output)),
        reported,
        ..Usage::unknown()
    })
}

#[cfg(any(feature = "builtins", feature = "provider-protocol-common"))]
pub(crate) use crate::types::classifier::cost_for_usage;
pub use crate::types::classifier::*;
