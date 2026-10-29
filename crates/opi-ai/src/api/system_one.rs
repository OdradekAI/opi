//! TypeSafe System One direct adapter (task 20.46).
//!
//! Reproduces the pinned pi `typesafe-system-one` module: classification
//! over TypeSafe's native System One protocol (`POST <base>/systemone` with
//! `{model, state, questions}`). Public `bool` questions and answers map to
//! the wire-level `noul` type on both sides; choice and score questions keep
//! their type and criteria shape. Bearer auth, no default timeout (a fresh
//! body-inclusive per-attempt timeout only when the caller sets one), the
//! source retry policy with **two extra retries by default**
//! (`x-should-retry` override, 408/409/429/5xx/no-status retryable,
//! `retry-after-ms`/`retry-after` delays capped at 60 s unless configured),
//! and per-question typed answer parsing that validates only what the source
//! requires: record shape, wire type agreement, string choices and finite
//! numbers. A request with malformed answers was still billed, so its
//! reported usage survives the parsing failure. Usage prices chat-style
//! through the record's catalog tiers. Image input is rejected before any
//! request, and `temperature` is a typed option System One cannot apply: it
//! never reaches the wire.
//!
//! The module also owns the shared System One transport core (pi
//! `SystemOneTransport`): every service that serves System One models —
//! the direct TypeSafe endpoint and the Cloudflare Workers AI REST
//! endpoint — differs only in URL, request envelope and response unwrap,
//! and drives the same attempt loop, guard set and output parser.

use std::time::Duration;

use secrecy::ExposeSecret;

use crate::api::ClassifierApi;
use crate::auth::{AuthScheme, ResolvedAuth};
use crate::classifier::{
    ClassifierAnswer, ClassifierCall, ClassifierContext, ClassifierModelRecord, ClassifierResult,
    ClassifierStopReason,
};
use crate::http::HttpClient;
use crate::models::ApiId;
use crate::provider::{ProviderError, ProviderErrorSummary};

/// The exact pi classifier API identity this adapter serves.
pub const TYPESAFE_SYSTEM_ONE_API_ID: &str = "typesafe-system-one";

/// Source default endpoint (the pi test fixture base URL; pi's generated
/// catalog data `data/typesafe.json` is absent from the pinned source).
pub const TYPESAFE_SYSTEM_ONE_BASE_URL: &str = "https://api.typesafe.ai/v1";

/// Source label used in error messages.
const LABEL: &str = "System One API";

/// Source default server-delay cap for retries (pi
/// `DEFAULT_MAX_RETRY_DELAY_MS`); `0` disables the cap.
const MAX_RETRY_DELAY_MS: u64 = 60_000;

/// Typed native options of the TypeSafe System One adapter.
///
/// Mirrors the source `ClassifierOptions`: `temperature` divides answer
/// logits before normalization where an API can apply it — System One has no
/// temperature field, so the value stays typed but never reaches the wire —
/// and the retry policy defaults to two extra retries (the pi
/// `postClassifierRequest` default), unlike the image adapter's zero.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TypesafeSystemOneOptions {
    /// Logit divisor the adapter cannot apply (kept for seam parity; System
    /// One has no temperature field). Never serialized.
    pub temperature: Option<f64>,
    /// Extra retry attempts after the first failure. Defaults to `2`.
    pub max_retries: u32,
    /// Cap for server-requested retry delays in milliseconds; `None` uses
    /// the 60 s source default, `Some(0)` disables the cap.
    pub max_retry_delay_ms: Option<u64>,
}

/// The TypeSafe System One adapter: fixed `O = TypesafeSystemOneOptions`.
pub struct TypesafeSystemOneApi {
    http: HttpClient,
    record: ClassifierModelRecord,
    api_id: ApiId,
}

impl TypesafeSystemOneApi {
    /// Build the adapter over a shared [`HttpClient`] with its complete
    /// record.
    pub fn new(http: HttpClient, record: ClassifierModelRecord) -> Self {
        let api_id = record.api.clone();
        Self {
            http,
            record,
            api_id,
        }
    }
}

impl ClassifierApi<TypesafeSystemOneOptions> for TypesafeSystemOneApi {
    fn api_id(&self) -> &ApiId {
        &self.api_id
    }

    fn provider_id(&self) -> &str {
        "typesafe"
    }

    fn complete_record(&self) -> &ClassifierModelRecord {
        &self.record
    }

    fn default_options(&self) -> TypesafeSystemOneOptions {
        TypesafeSystemOneOptions {
            temperature: None,
            max_retries: 2,
            max_retry_delay_ms: None,
        }
    }

    fn classify_prepared<'a>(
        &'a self,
        record: ClassifierModelRecord,
        context: ClassifierContext,
        options: TypesafeSystemOneOptions,
        auth: ResolvedAuth,
        call: ClassifierCall,
    ) -> crate::api::ClassifierFuture<'a> {
        Box::pin(async move {
            let transport = SystemOneTransport {
                api_id: TYPESAFE_SYSTEM_ONE_API_ID,
                label: LABEL,
                url: format!("{}/systemone", record.base_url.trim_end_matches('/')),
                payload: |body| body,
                output: typesafe_output,
            };
            classify_over_system_one(
                self.http.client().clone(),
                record,
                context,
                SystemOneCoreOptions {
                    max_retries: options.max_retries,
                    max_retry_delay_ms: options.max_retry_delay_ms,
                },
                auth,
                call,
                transport,
            )
            .await
        })
    }
}

/// The curated default record for the TypeSafe classifier adapter.
///
/// pi's generated catalog data (`data/typesafe.json`) is absent from the
/// pinned source, so opi ships one documented baseline entry (the pinned
/// test fixture identity: `typesafe/jev-latest`, 64000-token context, text
/// input); a registrar replaces it wholesale through
/// [`ClassifierApi::complete_record`] — the record never enters chat catalog
/// reads. Unknown pricing stays at the source generator's `0` fallback.
pub fn default_typesafe_classifier_record() -> ClassifierModelRecord {
    ClassifierModelRecord {
        provider_id: "typesafe".into(),
        id: "jev-latest".into(),
        name: "Jev".into(),
        api: ApiId::new(TYPESAFE_SYSTEM_ONE_API_ID),
        base_url: TYPESAFE_SYSTEM_ONE_BASE_URL.into(),
        input: vec![crate::classifier::ClassifierInputModality::Text],
        context_window: 64000,
        cost: crate::model_info::ModelPricing::try_new(
            crate::stream::Pricing::default(),
            Vec::new(),
        )
        .expect("zero pricing is valid"),
    }
}

/// Build the System One wire request body: public `bool` questions ride as
/// `noul`, choice and score keep their shape.
fn build_request_body(
    record: &ClassifierModelRecord,
    context: &ClassifierContext,
) -> serde_json::Value {
    let questions: serde_json::Map<String, serde_json::Value> = context
        .questions
        .iter()
        .map(|(id, question)| {
            let value = match question {
                crate::classifier::ClassifierQuestion::Choice {
                    instructions,
                    criteria,
                } => serde_json::json!({
                    "type": "choice",
                    "instructions": instructions,
                    "criteria": criteria,
                }),
                crate::classifier::ClassifierQuestion::Score {
                    instructions,
                    levels,
                } => serde_json::json!({
                    "type": "score",
                    "instructions": instructions,
                    "levels": levels,
                }),
                crate::classifier::ClassifierQuestion::Bool {
                    instructions,
                    true_criteria,
                    false_criteria,
                } => serde_json::json!({
                    "type": "noul",
                    "instructions": instructions,
                    "criteria": { "true": true_criteria, "false": false_criteria },
                }),
            };
            (id.clone(), value)
        })
        .collect();
    serde_json::json!({
        "model": record.id,
        "state": context.state,
        "questions": questions,
    })
}

/// One terminal failure of the attempt loop with its retry classification.
pub(super) struct AttemptFailure {
    pub status: Option<u16>,
    pub x_should_retry: Option<bool>,
    pub server_retry_delay_ms: Option<f64>,
    pub error: ProviderError,
    pub message: String,
}

impl AttemptFailure {
    pub fn is_retryable(&self) -> bool {
        if let Some(should_retry) = self.x_should_retry {
            return should_retry;
        }
        match self.status {
            None => true,
            Some(status) => status == 408 || status == 409 || status == 429 || status >= 500,
        }
    }
}

/// Differences between services that serve System One models (the pi
/// `SystemOneTransport` shape): one shared conformance drives every System
/// One endpoint family.
pub(crate) struct SystemOneTransport {
    /// Exact classifier API identity this transport serves (the guard).
    pub api_id: &'static str,
    /// Service label used in error messages.
    pub label: &'static str,
    /// Absolute request URL.
    pub url: String,
    /// Wraps the System One wire request `{model, state, questions}` in the
    /// service's request envelope.
    pub payload: fn(serde_json::Value) -> serde_json::Value,
    /// Extracts the System One output `{answers, usage}` record from the
    /// service's response envelope; `Err` is the display message.
    pub output: fn(&serde_json::Value, &str) -> Result<serde_json::Value, String>,
}

/// Retry knobs every System One options struct contributes to the core.
pub(crate) struct SystemOneCoreOptions {
    /// Extra retry attempts after the first failure.
    pub max_retries: u32,
    /// Cap for server-requested retry delays; `None` uses the 60 s source
    /// default, `Some(0)` disables the cap.
    pub max_retry_delay_ms: Option<u64>,
}

/// The shared System One attempt loop: guards, payload freeze, retries,
/// per-attempt timeouts and the shared output parser.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn classify_over_system_one(
    client: reqwest::Client,
    record: ClassifierModelRecord,
    context: ClassifierContext,
    options: SystemOneCoreOptions,
    auth: ResolvedAuth,
    call: ClassifierCall,
    transport: SystemOneTransport,
) -> ClassifierResult {
    let label = transport.label;
    // Pre-dispatch rejections: the transport guard, Bearer-only wire,
    // unsupported callback families, and image input.
    if record.api.as_str() != transport.api_id {
        let message = format!("Unsupported classifier API: {}", record.api.as_str());
        return ClassifierResult::failure(
            &record,
            ClassifierStopReason::Error,
            ProviderError::Config(ProviderErrorSummary::sanitized(message.clone())),
            message,
        );
    }
    if auth.scheme != AuthScheme::Bearer {
        let message = format!("{label} requires Bearer auth scheme");
        return ClassifierResult::failure(
            &record,
            ClassifierStopReason::Error,
            ProviderError::Config(ProviderErrorSummary::sanitized(format!(
                "{label} requires Bearer auth scheme, got {:?}",
                auth.scheme
            ))),
            message,
        );
    }
    if call.callbacks.has_on_raw() || call.callbacks.has_header_transform() {
        let message = format!("{label} supports payload and response observers only");
        return ClassifierResult::failure(
            &record,
            ClassifierStopReason::Error,
            ProviderError::UnsupportedCapability(ProviderErrorSummary::sanitized(message.clone())),
            message,
        );
    }
    if let Some(images) = context.images.as_ref()
        && !images.is_empty()
    {
        let message = format!("{label} does not support image input");
        return ClassifierResult::failure(
            &record,
            ClassifierStopReason::Error,
            ProviderError::UnsupportedCapability(ProviderErrorSummary::sanitized(message.clone())),
            message,
        );
    }

    let mut body = (transport.payload)(build_request_body(&record, &context));
    // The payload observer runs once per logical call; an ordinary retry
    // reuses the frozen body byte-for-byte (the shared callback seam).
    match call
        .callbacks
        .apply_payload_async(crate::callbacks::ORDINARY_PAYLOAD_VARIANT, body.clone())
        .await
    {
        Ok(replaced) => body = replaced,
        Err(error) => return failure_from_error(&record, &call, error),
    }

    let url = transport.url;

    let attempt = async {
        let mut retries_remaining = options.max_retries;
        let mut attempt_index: u32 = 0;
        loop {
            // A fresh per-attempt timeout when the caller set one; no
            // default (the source leaves `timeoutMs` undefined).
            let deadline = call
                .timeout
                .map(|timeout| tokio::time::Instant::now() + timeout);
            let request = client
                .post(&url)
                .bearer_auth(auth.secret.expose_secret())
                .json(&body);
            let request = call
                .extra_headers
                .iter()
                .fold(request, |builder, (name, value)| {
                    builder.header(name, value)
                });
            let send = async {
                let response = request.send().await.map_err(|error| {
                    if error.is_timeout() {
                        AttemptFailure {
                            status: None,
                            x_should_retry: None,
                            server_retry_delay_ms: None,
                            error: ProviderError::Timeout,
                            message: "request timed out".to_owned(),
                        }
                    } else {
                        AttemptFailure {
                            status: None,
                            x_should_retry: None,
                            server_retry_delay_ms: None,
                            error: ProviderError::Network(ProviderErrorSummary::sanitized(
                                error.to_string(),
                            )),
                            message: error.to_string(),
                        }
                    }
                })?;
                let status = response.status();
                let headers = response.headers().clone();
                let x_should_retry = headers
                    .get("x-should-retry")
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| match value.trim() {
                        "true" => Some(true),
                        "false" => Some(false),
                        _ => None,
                    });
                if !status.is_success() {
                    let read_timeout = call.timeout.unwrap_or(Duration::from_secs(30));
                    let body_bytes = crate::http::read_bounded_error_body(
                        response,
                        &call.cancel,
                        Some(read_timeout),
                    )
                    .await
                    .unwrap_or_default();
                    let body_text = body_bytes
                        .as_deref()
                        .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
                        .unwrap_or_default();
                    let message = format_http_error(label, status, &body_text);
                    return Err(AttemptFailure {
                        status: Some(status.as_u16()),
                        x_should_retry,
                        server_retry_delay_ms: server_retry_delay_ms(&headers),
                        error: map_status_error(label, status, &headers, &body_text),
                        message,
                    });
                }
                // The response observer fires at the source observation
                // point: after the status is known, before body processing.
                let observed_headers: Vec<(String, String)> = headers
                    .iter()
                    .map(|(name, value)| {
                        (
                            name.as_str().to_ascii_lowercase(),
                            value.to_str().unwrap_or_default().to_string(),
                        )
                    })
                    .collect();
                if let Err(error) = call
                    .callbacks
                    .notify_response_async(status.as_u16(), &observed_headers)
                    .await
                {
                    return Err(AttemptFailure {
                        status: Some(status.as_u16()),
                        x_should_retry: None,
                        server_retry_delay_ms: None,
                        error: error.clone(),
                        message: error.to_string(),
                    });
                }
                let body_text = response.text().await.map_err(|error| AttemptFailure {
                    status: None,
                    x_should_retry: None,
                    server_retry_delay_ms: None,
                    error: ProviderError::Network(ProviderErrorSummary::sanitized(
                        error.to_string(),
                    )),
                    message: error.to_string(),
                })?;
                Ok(body_text)
            };
            let attempt_result = match deadline {
                Some(deadline) => {
                    tokio::select! {
                        biased;
                        _ = call.cancel.cancelled() => {
                            return ClassifierResult::failure(
                                &record,
                                ClassifierStopReason::Aborted,
                                ProviderError::Cancelled,
                                "Request aborted".to_owned(),
                            );
                        }
                        result = tokio::time::timeout_at(deadline, send) => result,
                    }
                }
                None => {
                    tokio::select! {
                        biased;
                        _ = call.cancel.cancelled() => {
                            return ClassifierResult::failure(
                                &record,
                                ClassifierStopReason::Aborted,
                                ProviderError::Cancelled,
                                "Request aborted".to_owned(),
                            );
                        }
                        result = send => Ok(result),
                    }
                }
            };
            match attempt_result {
                Ok(Ok(body_text)) => {
                    return parse_system_one_response(
                        label,
                        transport.output,
                        &record,
                        &context,
                        &body_text,
                    );
                }
                Ok(Err(failure)) => {
                    if retries_remaining == 0 || !failure.is_retryable() {
                        return failure_result(&record, &call, failure);
                    }
                    retries_remaining -= 1;
                    let delay_ms =
                        match retry_delay_ms(&failure, attempt_index, options.max_retry_delay_ms) {
                            Ok(delay) => delay,
                            Err(capped) => {
                                let message = format!(
                                    "Server requested {}s retry delay (max: {}s). {}",
                                    capped.div_ceil(1000),
                                    cap_ms(options.max_retry_delay_ms).div_ceil(1000),
                                    failure.message
                                );
                                return ClassifierResult::failure(
                                    &record,
                                    ClassifierStopReason::Error,
                                    failure.error,
                                    message,
                                );
                            }
                        };
                    let sleep = tokio::time::sleep(Duration::from_millis(delay_ms));
                    tokio::select! {
                        biased;
                        _ = call.cancel.cancelled() => {
                            return ClassifierResult::failure(
                                &record,
                                ClassifierStopReason::Aborted,
                                ProviderError::Cancelled,
                                "Request aborted".to_owned(),
                            );
                        }
                        _ = sleep => {}
                    }
                    attempt_index += 1;
                }
                Err(_elapsed) => {
                    // A caller timeout is retryable (the source timeout
                    // error carries no status) and reports the source
                    // message with the configured milliseconds.
                    let failure = AttemptFailure {
                        status: None,
                        x_should_retry: None,
                        server_retry_delay_ms: None,
                        error: ProviderError::Timeout,
                        message: format!(
                            "Request timed out after {}ms",
                            call.timeout
                                .map(|timeout| timeout.as_millis())
                                .unwrap_or_default()
                        ),
                    };
                    if retries_remaining == 0 {
                        return failure_result(&record, &call, failure);
                    }
                    retries_remaining -= 1;
                    attempt_index += 1;
                }
            }
        }
    };

    match call.telemetry_context.clone() {
        Some(context) => context.instrument(attempt).await,
        None => attempt.await,
    }
}

pub(super) fn cap_ms(max_retry_delay_ms: Option<u64>) -> u64 {
    max_retry_delay_ms.unwrap_or(MAX_RETRY_DELAY_MS)
}

fn failure_result(
    record: &ClassifierModelRecord,
    call: &ClassifierCall,
    failure: AttemptFailure,
) -> ClassifierResult {
    if call.cancel.is_cancelled() {
        return ClassifierResult::failure(
            record,
            ClassifierStopReason::Aborted,
            ProviderError::Cancelled,
            "Request aborted".to_owned(),
        );
    }
    ClassifierResult::failure(
        record,
        ClassifierStopReason::Error,
        failure.error,
        failure.message,
    )
}

fn failure_from_error(
    record: &ClassifierModelRecord,
    call: &ClassifierCall,
    error: ProviderError,
) -> ClassifierResult {
    if call.cancel.is_cancelled() {
        return ClassifierResult::failure(
            record,
            ClassifierStopReason::Aborted,
            ProviderError::Cancelled,
            "Request aborted".to_owned(),
        );
    }
    let message = error.to_string();
    ClassifierResult::failure(record, ClassifierStopReason::Error, error, message)
}

/// pi `formatProviderError` with the label prefix: `"<label> error
/// (<status>): <body|message>"`, the body truncated to the source cap.
pub(super) fn format_http_error(label: &str, status: reqwest::StatusCode, body: &str) -> String {
    let suffix = crate::http::safe_excerpt(body);
    format!("{label} error ({}): {suffix}", status.as_u16())
}

pub(super) fn map_status_error(
    label: &str,
    status: reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
    body: &str,
) -> ProviderError {
    let code = status.as_u16();
    let summary = ProviderErrorSummary::sanitized(format_http_error(label, status, body));
    match code {
        429 => ProviderError::RateLimited {
            retry_after_ms: crate::retry::parse_retry_after(headers),
        },
        408 | 504 => ProviderError::Timeout,
        401 | 403 => ProviderError::AuthFailed(summary),
        _ => ProviderError::ServiceFault {
            status: code,
            error_name: None,
            retry_after_ms: None,
            detail: summary,
        },
    }
}

/// Server-requested retry delay in milliseconds, or `Err(delay)` when the
/// request exceeds the configured cap (pi `validateServerRetryDelayMs`).
pub(super) fn server_retry_delay_ms(headers: &reqwest::header::HeaderMap) -> Option<f64> {
    if let Some(value) = headers.get("retry-after-ms").and_then(|v| v.to_str().ok())
        && let Ok(parsed) = value.trim().parse::<f64>()
        && parsed.is_finite()
    {
        return Some(parsed);
    }
    if let Some(value) = headers.get("retry-after").and_then(|v| v.to_str().ok()) {
        let trimmed = value.trim();
        if let Ok(seconds) = trimmed.parse::<f64>()
            && seconds.is_finite()
        {
            return Some(seconds * 1000.0);
        }
        if let Some(delay) = crate::retry::parse_http_date_delay(trimmed) {
            return Some(delay as f64);
        }
    }
    None
}

pub(super) fn retry_delay_ms(
    failure: &AttemptFailure,
    attempt_index: u32,
    max_retry_delay_ms: Option<u64>,
) -> Result<u64, u64> {
    if let Some(delay) = failure.server_retry_delay_ms {
        let cap = cap_ms(max_retry_delay_ms);
        if cap > 0 && delay > cap as f64 {
            return Err(delay as u64);
        }
        return Ok(delay.max(0.0) as u64);
    }
    // Exponential backoff with jitter in [0.75, 1.0): the source policy.
    let base = (0.5_f64 * 2_f64.powi(attempt_index as i32)).min(8.0) * 1000.0;
    let jitter = 0.75 + (jitter_nanos() % 25) as f64 / 100.0;
    Ok((base * jitter) as u64)
}

fn jitter_nanos() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos() as u64)
        .unwrap_or(0)
}

/// pi `tokenCount`: a finite positive number counts, anything else is zero.
fn token_count(value: Option<&serde_json::Value>) -> u32 {
    value
        .and_then(serde_json::Value::as_f64)
        .filter(|tokens| tokens.is_finite() && *tokens > 0.0)
        .map(|tokens| tokens as u32)
        .unwrap_or(0)
}

/// The direct TypeSafe endpoint returns the System One output unwrapped:
/// any JSON object is the output record itself.
fn typesafe_output(body: &serde_json::Value, label: &str) -> Result<serde_json::Value, String> {
    if body.is_object() {
        Ok(body.clone())
    } else {
        Err(format!("{label} returned an unexpected response"))
    }
}

/// Parse the System One response: the transport unwraps its envelope,
/// usage is set before answers (a request with malformed answers was still
/// billed), and every known question must have a type-matching answer.
fn parse_system_one_response(
    label: &str,
    output: fn(&serde_json::Value, &str) -> Result<serde_json::Value, String>,
    record: &ClassifierModelRecord,
    context: &ClassifierContext,
    body_text: &str,
) -> ClassifierResult {
    let parsed: serde_json::Value = match serde_json::from_str(body_text) {
        Ok(value) => value,
        Err(error) => {
            let message = format!("{label} returned an unexpected response: {error}");
            return ClassifierResult::failure(
                record,
                ClassifierStopReason::Error,
                ProviderError::StreamError(ProviderErrorSummary::sanitized(message.clone())),
                message,
            );
        }
    };
    let parsed = match output(&parsed, label) {
        Ok(output) => output,
        Err(message) => {
            return ClassifierResult::failure(
                record,
                ClassifierStopReason::Error,
                ProviderError::StreamError(ProviderErrorSummary::sanitized(message.clone())),
                message,
            );
        }
    };

    let mut result = ClassifierResult::in_progress(record);

    // Usage first: a malformed-answers request keeps its billed usage.
    let usage_raw = parsed.get("usage").filter(|usage| usage.is_object());
    let input = usage_raw
        .map(|usage| token_count(usage.get("input_tokens")))
        .unwrap_or(0);
    let output = usage_raw
        .map(|usage| token_count(usage.get("output_tokens")))
        .unwrap_or(0);
    let has_tokens = usage_raw.is_some_and(|usage| {
        usage.get("input_tokens").is_some() || usage.get("output_tokens").is_some()
    });
    if has_tokens {
        let usage = crate::stream::Usage {
            input_tokens: input,
            output_tokens: output,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cache_write_1h_tokens: None,
            reasoning_tokens: None,
            total_tokens: Some(u64::from(input) + u64::from(output)),
            service_tier: None,
            reported: true,
        };
        // Chat-style tiered cost from the record's catalog pricing.
        let rates = record.cost.effective_for_usage(&usage);
        result.cost = Some(crate::stream::calculate_cost(&usage, &rates));
        result.usage = Some(usage);
    }

    let answers = match parsed.get("answers").filter(|answers| answers.is_object()) {
        Some(answers) => answers,
        None => {
            result.stop_reason = ClassifierStopReason::Error;
            result.error_message = Some(format!("{label} returned an unexpected response"));
            result.error = Some(ProviderError::StreamError(ProviderErrorSummary::sanitized(
                format!("{label} returned an unexpected response"),
            )));
            return result;
        }
    };

    for (id, question) in &context.questions {
        let parse = match answers.get(id) {
            Some(answer) => parse_answer(label, answer, question, id),
            None => Err(format!("{label} did not return an answer for {id}")),
        };
        match parse {
            Ok(answer) => {
                result.answers.insert(id.clone(), answer);
            }
            Err(message) => {
                // Source catch semantics: answers clear, billed usage stays.
                result.answers.clear();
                result.stop_reason = ClassifierStopReason::Error;
                result.error_message = Some(message.clone());
                result.error = Some(ProviderError::StreamError(ProviderErrorSummary::sanitized(
                    message,
                )));
                return result;
            }
        }
    }

    result
}

/// pi `requiredNumber`: a finite number or an error naming the field.
fn required_number(
    label: &str,
    value: Option<&serde_json::Value>,
    field: &str,
) -> Result<f64, String> {
    value
        .and_then(serde_json::Value::as_f64)
        .filter(|number| number.is_finite())
        .ok_or_else(|| format!("{label} returned an invalid {field}"))
}

/// Parse one typed answer against its question, validating only what the
/// source requires (shape, wire type, string choice, finite numbers).
fn parse_answer(
    label: &str,
    answer: &serde_json::Value,
    question: &crate::classifier::ClassifierQuestion,
    id: &str,
) -> Result<ClassifierAnswer, String> {
    if !answer.is_object() {
        return Err(format!("{label} did not return an answer for {id}"));
    }
    let wire_type = answer.get("type").and_then(serde_json::Value::as_str);
    match question {
        crate::classifier::ClassifierQuestion::Choice { .. } => {
            if wire_type != Some("choice") {
                return Err(format!("{label} did not return a choice answer for {id}"));
            }
            let choice = answer
                .get("choice")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| format!("{label} did not return a choice answer for {id}"))?
                .to_owned();
            let probabilities_raw = answer.get("probabilities");
            if !probabilities_raw.is_some_and(|probabilities| probabilities.is_object()) {
                return Err(format!("{label} returned invalid probabilities for {id}"));
            }
            let mut probabilities = std::collections::BTreeMap::new();
            for (key, probability) in probabilities_raw
                .expect("checked object")
                .as_object()
                .expect("checked object")
            {
                let value = required_number(
                    label,
                    Some(probability),
                    &format!("probability for {id}.{key}"),
                )?;
                probabilities.insert(key.clone(), value);
            }
            let confidence = required_number(
                label,
                answer.get("confidence"),
                &format!("confidence for {id}"),
            )?;
            Ok(ClassifierAnswer::Choice {
                choice,
                probabilities,
                confidence,
            })
        }
        crate::classifier::ClassifierQuestion::Score { .. } => {
            if wire_type != Some("score") {
                return Err(format!("{label} did not return a score answer for {id}"));
            }
            let score = required_number(label, answer.get("score"), &format!("score for {id}"))?;
            let confidence = required_number(
                label,
                answer.get("confidence"),
                &format!("confidence for {id}"),
            )?;
            Ok(ClassifierAnswer::Score { score, confidence })
        }
        crate::classifier::ClassifierQuestion::Bool { .. } => {
            if wire_type != Some("noul") {
                return Err(format!("{label} did not return a bool answer for {id}"));
            }
            let probability =
                required_number(label, answer.get("noul"), &format!("probability for {id}"))?;
            Ok(ClassifierAnswer::Bool { probability })
        }
    }
}
