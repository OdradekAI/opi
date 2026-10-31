//! OpenRouter image-generation direct adapter (task 20.45).
//!
//! Reproduces the pinned pi `openrouter-images` module: image generation
//! over OpenRouter's nonstream Chat Completions endpoint. One POST of
//! `{model, messages, stream:false, modalities}` to `<base>/chat/completions`
//! (trailing slashes stripped from the record's base URL), Bearer auth, a
//! 600000 ms body-inclusive per-attempt timeout (the source OpenAI SDK
//! default), zero extra retries by default with the source retry policy
//! (`x-should-retry` override, 408/409/429/5xx/no-status retryable,
//! `retry-after-ms`/`retry-after` delays capped at 60 s unless configured),
//! and first-choice ordered parsing: the choice's text content becomes the
//! leading text output, then each `message.images` entry whose URL is a
//! `data:<mime>;base64,<data>` URL becomes an image output with its encoded
//! original preserved; non-data URLs, malformed data URLs and later choices
//! are skipped without failing the call. Usage keeps the provider's cache
//! split and pricing uses the record's base rates only (the image path's
//! base-cost exception; tiers never apply). No URL fetch, file write, pixel
//! decode or streaming variant exists here.

use std::time::Duration;

use secrecy::ExposeSecret;

use crate::api::ImageApi;
use crate::auth::{AuthScheme, ResolvedAuth};
use crate::http::HttpClient;
use crate::images::{
    ImageCostRates, ImageModelRecord, ImageOutputModality, ImagesCall, ImagesContext, ImagesCost,
    ImagesOutput, ImagesResult, ImagesStopReason,
};
use crate::models::ApiId;
use crate::provider::{ProviderError, ProviderErrorSummary};
use crate::stream::Usage;

/// The exact pi image API identity this adapter serves.
pub const OPENROUTER_IMAGES_API_ID: &str = "openrouter-images";

/// Source default endpoint (pi `OPENROUTER_BASE_URL`).
pub const OPENROUTER_IMAGES_BASE_URL: &str = "https://openrouter.ai/api/v1";

/// Source default per-attempt timeout: OpenAI SDK default 600000 ms covering
/// the whole nonstream response body read and parse, not only the headers.
pub const OPENROUTER_IMAGES_TIMEOUT: Duration = Duration::from_millis(600_000);

/// Source default server-delay cap for retries (pi
/// `DEFAULT_MAX_RETRY_DELAY_MS`); `0` disables the cap.
pub const OPENROUTER_IMAGES_MAX_RETRY_DELAY_MS: u64 = 60_000;

/// Typed native options of the OpenRouter image adapter.
///
/// The source `ImagesOptions` carries no protocol-native fields for this
/// adapter: the wire body is fully determined by the record and context.
/// Only the retry policy is adapter-configurable; the default performs zero
/// extra retries, matching the pinned source.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OpenRouterImagesOptions {
    /// Extra retry attempts after the first failure. Defaults to `0`.
    pub max_retries: u32,
    /// Cap for server-requested retry delays in milliseconds; `None` uses
    /// the 60 s source default, `Some(0)` disables the cap.
    pub max_retry_delay_ms: Option<u64>,
}

/// The OpenRouter image adapter: fixed `O = OpenRouterImagesOptions`.
pub struct OpenRouterImagesApi {
    http: HttpClient,
    record: ImageModelRecord,
    api_id: ApiId,
}

impl OpenRouterImagesApi {
    /// Build the adapter over a shared [`HttpClient`] with its complete
    /// record.
    pub fn new(http: HttpClient, record: ImageModelRecord) -> Self {
        let api_id = record.api.clone();
        Self {
            http,
            record,
            api_id,
        }
    }
}

impl ImageApi<OpenRouterImagesOptions> for OpenRouterImagesApi {
    fn api_id(&self) -> &ApiId {
        &self.api_id
    }

    fn provider_id(&self) -> &str {
        &self.record.provider_id
    }

    fn complete_record(&self) -> &ImageModelRecord {
        &self.record
    }

    fn default_options(&self) -> OpenRouterImagesOptions {
        OpenRouterImagesOptions::default()
    }

    fn validate_request(
        &self,
        record: &ImageModelRecord,
        _context: &ImagesContext,
        options: &OpenRouterImagesOptions,
        call: &ImagesCall,
    ) -> Result<(), ProviderError> {
        self.validate_options(options)?;
        validate_image_call(record, call)
    }

    fn generate_images_prepared<'a>(
        &'a self,
        record: ImageModelRecord,
        context: ImagesContext,
        options: OpenRouterImagesOptions,
        auth: ResolvedAuth,
        call: ImagesCall,
    ) -> crate::api::ImageFuture<'a> {
        Box::pin(async move {
            generate_openrouter_images(
                self.http.client().clone(),
                record,
                context,
                options,
                auth,
                call,
            )
            .await
        })
    }
}

/// The curated default record for the OpenRouter image adapter.
///
/// pi's generated catalog data (`data/openrouter.json`) is absent from the
/// pinned source, so opi ships one documented baseline entry; a registrar
/// replaces it wholesale through [`ImageApi::complete_record`] — the record
/// never enters chat catalog reads. Unavailable catalog pricing stays unknown.
pub fn default_openrouter_image_record() -> ImageModelRecord {
    ImageModelRecord {
        cost_tiers: Vec::new(),
        headers: Vec::new(),
        input_limits: None,
        input: vec![
            crate::catalog::CatalogInputModality::Text,
            crate::catalog::CatalogInputModality::Image,
        ],
        provider_id: "openrouter".into(),
        id: "google/gemini-2.5-flash-image".into(),
        name: "Gemini 2.5 Flash Image (via OpenRouter)".into(),
        api: ApiId::new(OPENROUTER_IMAGES_API_ID),
        base_url: OPENROUTER_IMAGES_BASE_URL.into(),
        output: vec![ImageOutputModality::Image],
        cost: None,
    }
}

/// Build the nonstream Chat-wire request body from the record and context.
fn build_request_body(record: &ImageModelRecord, context: &ImagesContext) -> serde_json::Value {
    let content: Vec<serde_json::Value> = context
        .input
        .iter()
        .map(|item| match item {
            crate::images::ImagesInput::Text { text } => {
                serde_json::json!({ "type": "text", "text": text })
            }
            crate::images::ImagesInput::Image { mime_type, data } => serde_json::json!({
                "type": "image_url",
                "image_url": { "url": format!("data:{mime_type};base64,{data}") }
            }),
        })
        .collect();
    let modalities: Vec<&str> = record
        .output
        .iter()
        .map(|modality| match modality {
            ImageOutputModality::Text => "text",
            ImageOutputModality::Image => "image",
        })
        .collect();
    serde_json::json!({
        "model": record.id,
        "messages": [ { "role": "user", "content": content } ],
        "stream": false,
        "modalities": modalities,
    })
}

/// One terminal failure of the attempt loop with its retry classification.
struct AttemptFailure {
    status: Option<u16>,
    x_should_retry: Option<bool>,
    server_retry_delay_ms: Option<f64>,
    error: ProviderError,
    message: String,
}

fn validate_image_call(record: &ImageModelRecord, call: &ImagesCall) -> Result<(), ProviderError> {
    crate::images::validate_image_request(record, call)?;
    if call.callbacks.has_on_raw() || call.callbacks.has_header_transform() {
        return Err(ProviderError::UnsupportedCapability(
            ProviderErrorSummary::attested_static(
                "openrouter-images supports payload and response observers only",
            ),
        ));
    }
    if call.timeout.is_some_and(|timeout| timeout.is_zero()) {
        return Err(ProviderError::Config(
            ProviderErrorSummary::attested_static("image timeout must be a positive duration"),
        ));
    }
    Ok(())
}

impl AttemptFailure {
    fn is_retryable(&self) -> bool {
        if let Some(should_retry) = self.x_should_retry {
            return should_retry;
        }
        match self.status {
            None => true,
            Some(status) => status == 408 || status == 409 || status == 429 || status >= 500,
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn generate_openrouter_images(
    client: reqwest::Client,
    record: ImageModelRecord,
    context: ImagesContext,
    options: OpenRouterImagesOptions,
    auth: ResolvedAuth,
    mut call: ImagesCall,
) -> ImagesResult {
    call.callbacks = call.callbacks.for_call();
    if let Err(error) = validate_image_call(&record, &call) {
        return failure_from_error(&record, &call, error);
    }
    // Pre-dispatch rejections: Bearer-only wire, unsupported callback
    // families, and a positive timeout (the source SDK requires a positive
    // integer).
    if auth.scheme != AuthScheme::Bearer {
        return ImagesResult::failure(
            &record,
            ImagesStopReason::Error,
            ProviderError::Config(ProviderErrorSummary::sanitized(format!(
                "openrouter-images requires Bearer auth scheme, got {:?}",
                auth.scheme
            ))),
            "openrouter-images requires Bearer auth scheme".to_owned(),
        );
    }
    let timeout = call.timeout.unwrap_or(OPENROUTER_IMAGES_TIMEOUT);
    let headers = match crate::ProviderHeaders::default().merge_model_request(
        &record.headers,
        &[],
        &call.extra_headers,
    ) {
        Ok(headers) => headers,
        Err(error) => return failure_from_error(&record, &call, error),
    };
    let url = format!("{}/chat/completions", record.base_url.trim_end_matches('/'));
    let telemetry = call.telemetry_context.clone();

    // Payload freeze and the wire attempt both observe the caller's frozen
    // telemetry section (AIR-054): the payload observer routes its
    // observation to the frozen parent/Dispatch exactly like the attempt.
    let attempt = async {
        let mut body = build_request_body(&record, &context);
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

        let mut retries_remaining = options.max_retries;
        let mut attempt_index: u32 = 0;
        loop {
            let deadline = tokio::time::Instant::now() + timeout;
            let request = client
                .post(&url)
                .bearer_auth(auth.secret.expose_secret())
                .json(&body);
            let request = headers.iter().fold(request, |builder, (name, value)| {
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
                    let body_bytes =
                        crate::http::read_bounded_error_body(response, &call.cancel, Some(timeout))
                            .await
                            .unwrap_or_default();
                    let body_text = body_bytes
                        .as_deref()
                        .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
                        .unwrap_or_default();
                    let message = format_http_error(status, &body_text);
                    return Err(AttemptFailure {
                        status: Some(status.as_u16()),
                        x_should_retry,
                        server_retry_delay_ms: server_retry_delay_ms(&headers),
                        error: map_status_error(status, &headers, &body_text),
                        message,
                    });
                }
                let observed_headers: Vec<(String, String)> = headers
                    .iter()
                    .map(|(name, value)| {
                        (
                            name.as_str().to_ascii_lowercase(),
                            value.to_str().unwrap_or_default().to_string(),
                        )
                    })
                    .collect();
                let body_text = response.text().await.map_err(|error| AttemptFailure {
                    status: None,
                    x_should_retry: None,
                    server_retry_delay_ms: None,
                    error: ProviderError::Network(ProviderErrorSummary::sanitized(
                        error.to_string(),
                    )),
                    message: error.to_string(),
                })?;
                let parsed: serde_json::Value =
                    serde_json::from_str(&body_text).map_err(|error| {
                        let message = format!("invalid openrouter-images response body: {error}");
                        AttemptFailure {
                            status: None,
                            x_should_retry: None,
                            server_retry_delay_ms: None,
                            error: ProviderError::StreamError(ProviderErrorSummary::sanitized(
                                message.clone(),
                            )),
                            message,
                        }
                    })?;
                Ok((parsed, status.as_u16(), observed_headers))
            };
            let attempt_result = tokio::select! {
                biased;
                _ = call.cancel.cancelled() => {
                    return ImagesResult::failure(
                        &record,
                        ImagesStopReason::Aborted,
                        ProviderError::Cancelled,
                        "Request aborted".to_owned(),
                    );
                }
                result = tokio::time::timeout_at(deadline, send) => result,
            };
            match attempt_result {
                Ok(Ok((parsed, status, headers))) => {
                    // JSON acquisition has handed off successfully. Callback
                    // time and failures cannot restart a generation attempt.
                    let observed = tokio::select! {
                        biased;
                        _ = call.cancel.cancelled() => Err(ProviderError::Cancelled),
                        result = call.callbacks.notify_response_async(status, &headers) => result,
                    };
                    if let Err(error) = observed {
                        return failure_from_error(&record, &call, error);
                    }
                    return parse_openrouter_images_response(&record, &parsed);
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
                                return ImagesResult::failure(
                                    &record,
                                    ImagesStopReason::Error,
                                    failure.error,
                                    message,
                                );
                            }
                        };
                    let sleep = tokio::time::sleep(Duration::from_millis(delay_ms));
                    tokio::select! {
                        biased;
                        _ = call.cancel.cancelled() => {
                            return ImagesResult::failure(
                                &record,
                                ImagesStopReason::Aborted,
                                ProviderError::Cancelled,
                                "Request aborted".to_owned(),
                            );
                        }
                        _ = sleep => {}
                    }
                    attempt_index += 1;
                }
                Err(_elapsed) => {
                    let failure = AttemptFailure {
                        status: None,
                        x_should_retry: None,
                        server_retry_delay_ms: None,
                        error: ProviderError::Timeout,
                        message: "request timed out".to_owned(),
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

    match telemetry {
        Some(context) => context.instrument(attempt).await,
        None => attempt.await,
    }
}

fn cap_ms(max_retry_delay_ms: Option<u64>) -> u64 {
    max_retry_delay_ms.unwrap_or(OPENROUTER_IMAGES_MAX_RETRY_DELAY_MS)
}

fn failure_result(
    record: &ImageModelRecord,
    call: &ImagesCall,
    failure: AttemptFailure,
) -> ImagesResult {
    if call.cancel.is_cancelled() {
        return ImagesResult::failure(
            record,
            ImagesStopReason::Aborted,
            ProviderError::Cancelled,
            "Request aborted".to_owned(),
        );
    }
    ImagesResult::failure(
        record,
        ImagesStopReason::Error,
        failure.error,
        failure.message,
    )
}

fn failure_from_error(
    record: &ImageModelRecord,
    call: &ImagesCall,
    error: ProviderError,
) -> ImagesResult {
    if call.cancel.is_cancelled() {
        return ImagesResult::failure(
            record,
            ImagesStopReason::Aborted,
            ProviderError::Cancelled,
            "Request aborted".to_owned(),
        );
    }
    let message = error.to_string();
    ImagesResult::failure(record, ImagesStopReason::Error, error, message)
}

/// pi `formatProviderError` without prefix for the OpenAI SDK shape: the JSON
/// `error.message` when present, else the bounded raw body.
fn format_http_error(status: reqwest::StatusCode, body: &str) -> String {
    let suffix = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|parsed| {
            parsed
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(|message| message.as_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| crate::http::safe_excerpt(body));
    format!("{}: {suffix}", status.as_u16())
}

fn map_status_error(
    status: reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
    body: &str,
) -> ProviderError {
    let code = status.as_u16();
    let summary = ProviderErrorSummary::sanitized(format_http_error(status, body));
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
fn server_retry_delay_ms(headers: &reqwest::header::HeaderMap) -> Option<f64> {
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

fn retry_delay_ms(
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

/// Parse the nonstream response: first choice only, ordered text-then-images,
/// response id, usage split and base-rate cost.
fn parse_openrouter_images_response(
    record: &ImageModelRecord,
    parsed: &serde_json::Value,
) -> ImagesResult {
    let mut result = ImagesResult {
        api: record.api.as_str().to_owned(),
        provider: record.provider_id.clone(),
        model: record.id.clone(),
        output: Vec::new(),
        response_id: parsed
            .get("id")
            .and_then(|id| id.as_str())
            .map(str::to_owned),
        usage: None,
        cost: None,
        stop_reason: ImagesStopReason::Stop,
        error_message: None,
        error: None,
    };

    if let Some(usage) = parsed.get("usage").and_then(parse_image_usage) {
        if usage.reported {
            result.cost = record.cost.as_ref().and_then(|rates| {
                base_rate_cost(
                    rates,
                    u64::from(usage.input_tokens),
                    u64::from(usage.output_tokens),
                    u64::from(usage.cache_read_tokens),
                    u64::from(usage.cache_write_tokens),
                )
            });
        }
        result.usage = Some(usage);
    }

    let choice = parsed
        .get("choices")
        .and_then(|choices| choices.as_array())
        .and_then(|choices| choices.first());
    if let Some(message) = choice
        .and_then(|choice| choice.get("message"))
        .filter(|message| message.is_object())
    {
        if let Some(content) = message.get("content").and_then(|content| content.as_str())
            && !content.is_empty()
        {
            result.output.push(ImagesOutput::Text {
                text: content.to_owned(),
            });
        }
        if let Some(images) = message.get("images").and_then(|images| images.as_array()) {
            for image in images {
                let image_url = match image.get("image_url") {
                    Some(serde_json::Value::String(url)) => Some(url.as_str()),
                    Some(serde_json::Value::Object(object)) => {
                        object.get("url").and_then(|url| url.as_str())
                    }
                    _ => None,
                };
                let Some(url) = image_url else { continue };
                let Some(decoded) = parse_data_url(url) else {
                    continue;
                };
                result.output.push(ImagesOutput::Image {
                    mime_type: decoded.0,
                    data: decoded.1,
                });
            }
        }
    }

    result
}

/// `data:<mime>;base64,<payload>` parser: the pinned source regex, returning
/// `(mime, payload)` or `None` for non-data URLs and malformed entries.
fn parse_data_url(url: &str) -> Option<(String, String)> {
    let rest = url.strip_prefix("data:")?;
    let separator = rest.find(";base64,")?;
    let mime = &rest[..separator];
    let payload = &rest[separator + ";base64,".len()..];
    if mime.is_empty() || payload.is_empty() {
        return None;
    }
    Some((mime.to_owned(), payload.to_owned()))
}

fn parse_image_usage(raw: &serde_json::Value) -> Option<Usage> {
    let raw = raw.as_object()?;
    let prompt = raw.get("prompt_tokens").and_then(serde_json::Value::as_u64);
    let output = raw
        .get("completion_tokens")
        .and_then(serde_json::Value::as_u64);
    let details = raw.get("prompt_tokens_details");
    let cached = details.and_then(|value| value.get("cached_tokens"));
    let write = details.and_then(|value| value.get("cache_write_tokens"));
    let cached_count = cached.and_then(serde_json::Value::as_u64);
    let write_count = write.and_then(serde_json::Value::as_u64);
    if prompt.is_none() && output.is_none() && cached_count.is_none() && write_count.is_none() {
        return None;
    }
    let cache_write = write_count.unwrap_or_default();
    let cache_read = cached_count.unwrap_or_default().saturating_sub(cache_write);
    let input = prompt
        .unwrap_or_default()
        .saturating_sub(cache_read)
        .saturating_sub(cache_write);
    let output_count = output.unwrap_or_default();
    let reported = prompt.is_some()
        && output.is_some()
        && details.is_none_or(serde_json::Value::is_object)
        && cached.is_none_or(|value| value.as_u64().is_some())
        && write.is_none_or(|value| value.as_u64().is_some())
        && [input, output_count, cache_read, cache_write]
            .iter()
            .all(|count| u32::try_from(*count).is_ok());
    Some(Usage {
        input_tokens: u32::try_from(input).unwrap_or(u32::MAX),
        output_tokens: u32::try_from(output_count).unwrap_or(u32::MAX),
        cache_read_tokens: u32::try_from(cache_read).unwrap_or(u32::MAX),
        cache_write_tokens: u32::try_from(cache_write).unwrap_or(u32::MAX),
        total_tokens: reported.then(|| input + output_count + cache_read + cache_write),
        reported,
        ..Usage::unknown()
    })
}

/// pi `parseUsage` cost: base per-million rates only (the image path's
/// base-cost exception; tiers never apply).
fn base_rate_cost(
    rates: &ImageCostRates,
    input: u64,
    output: u64,
    cache_read: u64,
    cache_write: u64,
) -> Option<ImagesCost> {
    if [
        rates.input,
        rates.output,
        rates.cache_read,
        rates.cache_write,
    ]
    .iter()
    .any(|rate| !rate.is_finite() || *rate < 0.0)
    {
        return None;
    }
    let input_cost = (rates.input / 1_000_000.0) * input as f64;
    let output_cost = (rates.output / 1_000_000.0) * output as f64;
    let cache_read_cost = (rates.cache_read / 1_000_000.0) * cache_read as f64;
    let cache_write_cost = (rates.cache_write / 1_000_000.0) * cache_write as f64;
    let total = input_cost + output_cost + cache_read_cost + cache_write_cost;
    total.is_finite().then_some(ImagesCost {
        input: input_cost,
        output: output_cost,
        cache_read: cache_read_cost,
        cache_write: cache_write_cost,
        total,
    })
}
