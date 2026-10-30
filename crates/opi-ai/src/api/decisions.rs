//! OpenAI's Decisions API (task 20.49).
//!
//! One `POST <base>/decisions` with `{model, input, questions}` per
//! classification (pi `openai-decisions`). The state is sent as JSON text;
//! with images the input becomes one user message with the state as
//! `input_text` followed by `input_image` data URLs, at most 128 of them.
//! Questions map to Decisions types: `choice` keeps its options (empty
//! descriptions stay off the wire), `score` keeps its levels and `bool`
//! becomes a `predicate` whose criteria meanings join the instructions —
//! predicates have no criteria field, and a System One `noul` and a
//! Decisions `predicate` are not interchangeable. Answers come back as an
//! array matched by name; refusals, missing answers, mistyped shapes and
//! non-finite required numbers are typed errors that keep the billed usage
//! (source-required validations only). Usage prices chat-style through the
//! record's catalog tiers.
//!
//! Cloudflare in front of `api.openai.com` answers 504 with an HTML page
//! when a request runs longer than about five seconds; retrying the same
//! input runs into it again, so 504 never retries and the failure explains
//! the gateway limit instead of surfacing the page. Everything else rides
//! the shared classifier retry policy: two extra retries by default,
//! Retry-After-aware capped delays and fresh per-attempt body-inclusive
//! timeouts. Only OpenAI API keys work on this route: an empty key fails
//! typed before any request. `temperature` exists on the pi
//! `ClassifierOptions` type but the Decisions request never reads it, so
//! the typed options here carry the retry knobs only.

use std::collections::BTreeMap;
use std::time::Duration;

use crate::api::system_one::{
    AttemptFailure, cap_ms, format_http_error, map_status_error, retry_delay_ms,
    server_retry_delay_ms,
};
use crate::api::{ApiId, ClassifierApi, ClassifierFuture};
use crate::auth::{AuthScheme, ResolvedAuth};
use crate::classifier::{
    ClassifierAnswer, ClassifierCall, ClassifierContext, ClassifierImage, ClassifierModelRecord,
    ClassifierQuestion, ClassifierResult, ClassifierStopReason,
};
use crate::http::HttpClient;
use crate::provider::{ProviderError, ProviderErrorSummary};
use secrecy::ExposeSecret;

/// Exact classifier API identity: pi `openai-decisions`.
pub const OPENAI_DECISIONS_API_ID: &str = "openai-decisions";

/// Source default endpoint (pi model catalog); decisions post to
/// `decisions` under this root. pi's generated catalog data is absent from
/// the pinned source; the curated record documents the tested model.
pub const OPENAI_DECISIONS_BASE_URL: &str = "https://api.openai.com/v1";

/// Source label used in error messages.
const LABEL: &str = "OpenAI Decisions";

/// The endpoint accepts at most this many image parts per request.
const MAX_IMAGES: usize = 128;

/// HTTP statuses that fail at once although the policy would retry them:
/// a gateway 504 re-runs the same oversized input and times out again.
const NO_RETRY_STATUSES: [u16; 1] = [504];

/// The 504 replacement message: the source explains the gateway limit
/// instead of surfacing Cloudflare's HTML page.
const GATEWAY_TIMEOUT_MESSAGE: &str = concat!(
    "OpenAI Decisions error (504): the request timed out at the gateway. ",
    "Very large inputs (above roughly 600K tokens) currently exceed its time limit."
);

/// Typed native options of the OpenAI Decisions adapter.
///
/// Mirrors the source `ClassifierOptions` shape: `temperature` stays on the
/// pi type but the Decisions request never reads it, and the retry policy
/// defaults to two extra retries.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenAiDecisionsOptions {
    /// Extra retry attempts after the first failure. Defaults to `2`.
    pub max_retries: u32,
    /// Cap for server-requested retry delays in milliseconds; `None` uses
    /// the 60 s source default, `Some(0)` disables the cap.
    pub max_retry_delay_ms: Option<u64>,
}

impl Default for OpenAiDecisionsOptions {
    fn default() -> Self {
        Self {
            max_retries: 2,
            max_retry_delay_ms: None,
        }
    }
}

/// The OpenAI Decisions adapter: fixed `O = OpenAiDecisionsOptions`.
pub struct OpenAiDecisionsApi {
    http: HttpClient,
    record: ClassifierModelRecord,
    api_id: ApiId,
}

impl OpenAiDecisionsApi {
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

impl ClassifierApi<OpenAiDecisionsOptions> for OpenAiDecisionsApi {
    fn api_id(&self) -> &ApiId {
        &self.api_id
    }

    fn provider_id(&self) -> &str {
        &self.record.provider_id
    }

    fn complete_record(&self) -> &ClassifierModelRecord {
        &self.record
    }

    fn default_options(&self) -> OpenAiDecisionsOptions {
        OpenAiDecisionsOptions::default()
    }

    fn validate_request(
        &self,
        record: &ClassifierModelRecord,
        context: &ClassifierContext,
        options: &OpenAiDecisionsOptions,
        call: &ClassifierCall,
    ) -> Result<(), ProviderError> {
        self.validate_options(options)?;
        crate::classifier::validate_classifier_request(record, context, call)?;
        if context
            .images
            .as_ref()
            .is_some_and(|images| images.len() > MAX_IMAGES)
        {
            return Err(ProviderError::UnsupportedCapability(
                ProviderErrorSummary::sanitized(format!(
                    "{LABEL} accepts at most {MAX_IMAGES} images, got {}",
                    context.images.as_ref().map_or(0, Vec::len)
                )),
            ));
        }
        Ok(())
    }

    fn classify_prepared<'a>(
        &'a self,
        record: ClassifierModelRecord,
        context: ClassifierContext,
        options: OpenAiDecisionsOptions,
        auth: ResolvedAuth,
        mut call: ClassifierCall,
    ) -> ClassifierFuture<'a> {
        Box::pin(async move {
            call.callbacks = call.callbacks.for_call();
            if let Err(error) = self.validate_request(&record, &context, &options, &call) {
                return failure_from_error(&record, &call, error);
            }
            let label = LABEL;
            // Pre-dispatch rejections: the transport guard, Bearer-only
            // wire, the API-key requirement, unsupported callback families.
            if record.api.as_str() != OPENAI_DECISIONS_API_ID {
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
            let secret = auth.secret.expose_secret().to_owned();
            if secret.is_empty() {
                // Only OpenAI API keys work on this route; the source fails
                // a missing key before any request.
                let message = format!("No API key for provider: {}", record.provider_id);
                return ClassifierResult::failure(
                    &record,
                    ClassifierStopReason::Error,
                    ProviderError::CredentialNeeded {
                        provider_id: record.provider_id.clone(),
                    },
                    message,
                );
            }
            if call.callbacks.has_on_raw() || call.callbacks.has_header_transform() {
                let message = format!("{label} supports payload and response observers only");
                return ClassifierResult::failure(
                    &record,
                    ClassifierStopReason::Error,
                    ProviderError::UnsupportedCapability(ProviderErrorSummary::sanitized(
                        message.clone(),
                    )),
                    message,
                );
            }
            // The image cap rejects before any request, like the source
            // `wireInput`.
            let input = match wire_input(&context) {
                Ok(input) => input,
                Err(message) => {
                    return ClassifierResult::failure(
                        &record,
                        ClassifierStopReason::Error,
                        ProviderError::UnsupportedCapability(ProviderErrorSummary::sanitized(
                            message.clone(),
                        )),
                        message,
                    );
                }
            };
            // Questions ride as an array in the map's alphabetical order;
            // the pi source order is insertion-based.
            let questions: Vec<serde_json::Value> = context
                .questions
                .iter()
                .map(|(name, question)| wire_question(name, question))
                .collect();
            let body = serde_json::json!({
                "model": record.id,
                "input": input,
                "questions": questions,
            });
            // The payload observer runs once per logical call; an ordinary
            // retry reuses the frozen body byte-for-byte.
            let body = match call
                .callbacks
                .apply_payload_async(crate::callbacks::ORDINARY_PAYLOAD_VARIANT, body)
                .await
            {
                Ok(replaced) => replaced,
                Err(error) => return failure_from_error(&record, &call, error),
            };

            let url = format!("{}/decisions", record.base_url.trim_end_matches('/'));
            let headers = match crate::ProviderHeaders::default().merge_model_request(
                &record.headers,
                &[],
                &call.extra_headers,
            ) {
                Ok(headers) => headers,
                Err(error) => return failure_from_error(&record, &call, error),
            };

            let attempt = async {
                let mut retries_remaining = options.max_retries;
                let mut attempt_index: u32 = 0;
                loop {
                    // A fresh per-attempt timeout when the caller set one;
                    // no default (the source leaves `timeoutMs` undefined).
                    let deadline = call
                        .timeout
                        .map(|timeout| tokio::time::Instant::now() + timeout);
                    let request = self
                        .http
                        .client()
                        .post(&url)
                        .bearer_auth(&secret)
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
                            let code = status.as_u16();
                            // The gateway 504 explains itself instead of
                            // surfacing the HTML page.
                            let message = if NO_RETRY_STATUSES.contains(&code) {
                                GATEWAY_TIMEOUT_MESSAGE.to_owned()
                            } else {
                                format_http_error(label, status, &body_text)
                            };
                            return Err(AttemptFailure {
                                status: Some(code),
                                x_should_retry,
                                server_retry_delay_ms: server_retry_delay_ms(&headers),
                                error: map_status_error(label, status, &headers, &body_text),
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
                                let message =
                                    format!("{label} returned an unexpected response: {error}");
                                AttemptFailure {
                                    status: None,
                                    x_should_retry: None,
                                    server_retry_delay_ms: None,
                                    error: ProviderError::StreamError(
                                        ProviderErrorSummary::sanitized(message.clone()),
                                    ),
                                    message,
                                }
                            })?;
                        Ok((parsed, status.as_u16(), observed_headers))
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
                        Ok(Ok((parsed, status, headers))) => {
                            // Response observers run after complete JSON acquisition,
                            // outside its retry and timeout ownership.
                            let observed = tokio::select! {
                                biased;
                                _ = call.cancel.cancelled() => Err(ProviderError::Cancelled),
                                result = call.callbacks.notify_response_async(status, &headers) => result,
                            };
                            if let Err(error) = observed {
                                return failure_from_error(&record, &call, error);
                            }
                            return parse_decisions_response(&record, &context, &parsed);
                        }
                        Ok(Err(failure)) => {
                            // A listed no-retry status fails at once even
                            // though the shared policy would retry it.
                            let no_retry = failure
                                .status
                                .is_some_and(|status| NO_RETRY_STATUSES.contains(&status));
                            if retries_remaining == 0 || no_retry || !failure.is_retryable() {
                                return failure_result(&record, &call, failure);
                            }
                            retries_remaining -= 1;
                            let delay_ms = match retry_delay_ms(
                                &failure,
                                attempt_index,
                                options.max_retry_delay_ms,
                            ) {
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
                            // A caller timeout is retryable (the source
                            // timeout error carries no status) and reports
                            // the source message with the configured
                            // milliseconds.
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
        })
    }
}

/// The curated default record: pi ships no generated catalog for the
/// Decisions API; the pinned test documents `gpt-6-luna` with its
/// long-context tier.
pub fn default_decisions_classifier_record() -> ClassifierModelRecord {
    ClassifierModelRecord {
        headers: Vec::new(),
        input_limits: None,
        provider_id: "openai".into(),
        id: "gpt-6-luna".into(),
        name: "GPT-6 Luna".into(),
        api: ApiId::new(OPENAI_DECISIONS_API_ID),
        base_url: OPENAI_DECISIONS_BASE_URL.into(),
        input: vec![
            crate::classifier::ClassifierInputModality::Text,
            crate::classifier::ClassifierInputModality::Image,
        ],
        context_window: 922_000,
        cost: Some(
            crate::model_info::ModelPricing::try_new(
                crate::stream::Pricing {
                    input_cost_per_mtok: 0.1,
                    ..crate::stream::Pricing::default()
                },
                vec![crate::model_info::PricingTier {
                    input_tokens_above: 272_000,
                    pricing: crate::stream::Pricing {
                        input_cost_per_mtok: 0.2,
                        ..crate::stream::Pricing::default()
                    },
                }],
            )
            .expect("luna pricing is valid"),
        ),
    }
}

/// The request `input`: the state serialized to a JSON string, or one user
/// message with the state as `input_text` followed by the images as data
/// URLs. More than [`MAX_IMAGES`] images is a typed pre-request error.
fn wire_input(context: &ClassifierContext) -> Result<serde_json::Value, String> {
    let state = serde_json::to_string(&context.state).expect("a JSON value always serializes");
    let images: &[ClassifierImage] = context.images.as_deref().unwrap_or(&[]);
    if images.is_empty() {
        return Ok(serde_json::Value::String(state));
    }
    if images.len() > MAX_IMAGES {
        return Err(format!(
            "{LABEL} accepts at most {MAX_IMAGES} images, got {}",
            images.len()
        ));
    }
    let mut content = vec![serde_json::json!({ "type": "input_text", "text": state })];
    for image in images {
        content.push(serde_json::json!({
            "type": "input_image",
            "image_url": format!("data:{};base64,{}", image.mime_type, image.data),
        }));
    }
    Ok(serde_json::json!([{ "role": "user", "content": content }]))
}

/// One question in Decisions wire shape: `choice` keeps its options (empty
/// descriptions omitted), `score` its levels and `bool` becomes a
/// `predicate` whose criteria meanings join the instructions.
fn wire_question(name: &str, question: &ClassifierQuestion) -> serde_json::Value {
    match question {
        ClassifierQuestion::Choice {
            instructions,
            criteria,
        } => serde_json::json!({
            "type": "choice",
            "name": name,
            "instructions": instructions,
            "choices": criteria
                .iter()
                .map(|(value, description)| {
                    if description.is_empty() {
                        serde_json::json!({ "value": value })
                    } else {
                        serde_json::json!({ "value": value, "description": description })
                    }
                })
                .collect::<Vec<_>>(),
        }),
        ClassifierQuestion::Score {
            instructions,
            levels,
        } => serde_json::json!({
            "type": "score",
            "name": name,
            "instructions": instructions,
            "levels": levels
                .iter()
                .map(|level| serde_json::json!({ "label": level }))
                .collect::<Vec<_>>(),
        }),
        ClassifierQuestion::Bool { .. } => serde_json::json!({
            "type": "predicate",
            "name": name,
            "instructions": predicate_instructions(question),
        }),
    }
}

/// Predicate instructions with the meanings of true and false appended;
/// predicates have no criteria field.
fn predicate_instructions(question: &ClassifierQuestion) -> String {
    let ClassifierQuestion::Bool {
        instructions,
        true_criteria,
        false_criteria,
    } = question
    else {
        return question.instructions().to_owned();
    };
    let mut meanings = Vec::new();
    if !true_criteria.is_empty() {
        meanings.push(format!("True means: {true_criteria}"));
    }
    if !false_criteria.is_empty() {
        meanings.push(format!("False means: {false_criteria}"));
    }
    if meanings.is_empty() {
        instructions.clone()
    } else {
        format!("{instructions}\n\n{}", meanings.join("\n"))
    }
}

/// A required finite number (pi `requiredNumber`).
fn required_number(
    label: &str,
    value: Option<&serde_json::Value>,
    field: &str,
) -> Result<f64, String> {
    value
        .and_then(|value| value.as_f64())
        .filter(|number| number.is_finite())
        .ok_or_else(|| format!("{label} returned an invalid {field}"))
}

/// A choice answer's probabilities: `[{value, probability}]` entries keyed
/// by option value.
fn choice_probabilities(
    value: Option<&serde_json::Value>,
    label: &str,
    id: &str,
) -> Result<BTreeMap<String, f64>, String> {
    let Some(entries) = value.and_then(|value| value.as_array()) else {
        return Err(format!("{label} returned invalid probabilities for {id}"));
    };
    let mut probabilities = BTreeMap::new();
    for entry in entries {
        let Some(option) = entry.get("value").and_then(|value| value.as_str()) else {
            return Err(format!("{label} returned invalid probabilities for {id}"));
        };
        let probability = required_number(
            label,
            entry.get("probability"),
            &format!("probability for {id}.{option}"),
        )?;
        probabilities.insert(option.to_owned(), probability);
    }
    Ok(probabilities)
}

/// One wire answer parsed against its question; `Err` is the display
/// message. Source-required validations only.
fn parse_answer(
    label: &str,
    answer: &serde_json::Value,
    question: &ClassifierQuestion,
    id: &str,
) -> Result<ClassifierAnswer, String> {
    if answer.get("type").and_then(|kind| kind.as_str()) == Some("refusal") {
        return Err(format!("{label} refused to answer {id}"));
    }
    match question {
        ClassifierQuestion::Choice { .. } => {
            if answer.get("type").and_then(|kind| kind.as_str()) != Some("choice")
                || answer
                    .get("choice")
                    .and_then(|choice| choice.as_str())
                    .is_none()
            {
                return Err(format!("{label} did not return a choice answer for {id}"));
            }
            let probabilities = choice_probabilities(answer.get("probabilities"), label, id)?;
            let confidence = required_number(
                label,
                answer.get("confidence"),
                &format!("confidence for {id}"),
            )?;
            Ok(ClassifierAnswer::Choice {
                choice: answer["choice"].as_str().expect("checked above").to_owned(),
                probabilities,
                confidence,
            })
        }
        ClassifierQuestion::Score { .. } => {
            if answer.get("type").and_then(|kind| kind.as_str()) != Some("score") {
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
        ClassifierQuestion::Bool { .. } => {
            if answer.get("type").and_then(|kind| kind.as_str()) != Some("predicate") {
                return Err(format!(
                    "{label} did not return a predicate answer for {id}"
                ));
            }
            let probability = required_number(
                label,
                answer.get("probability"),
                &format!("probability for {id}"),
            )?;
            Ok(ClassifierAnswer::Bool { probability })
        }
    }
}

/// Parse a Decisions response body. Usage first: a request with malformed
/// or refused answers was still billed, so the reported usage and cost
/// survive every parse failure.
fn parse_decisions_response(
    record: &ClassifierModelRecord,
    context: &ClassifierContext,
    parsed: &serde_json::Value,
) -> ClassifierResult {
    let label = LABEL;
    if !parsed.is_object() {
        return unexpected_response(record);
    }

    let mut result = ClassifierResult::in_progress(record);

    // Usage first: a malformed-answers request keeps its billed usage.
    if let Some(usage) = crate::classifier::parse_classifier_usage(parsed.get("usage")) {
        // Chat-style tiered cost from the record's catalog pricing.
        result.cost = crate::classifier::cost_for_usage(record, &usage);
        result.usage = Some(usage);
    }

    // Answers ride as an array; each is matched to its question by name.
    let Some(answers) = parsed.get("answers").filter(|answers| answers.is_array()) else {
        result.stop_reason = ClassifierStopReason::Error;
        result.error_message = Some(format!("{label} returned an unexpected response"));
        result.error = Some(ProviderError::StreamError(ProviderErrorSummary::sanitized(
            format!("{label} returned an unexpected response"),
        )));
        return result;
    };
    let mut by_name: BTreeMap<&str, &serde_json::Value> = BTreeMap::new();
    for answer in answers.as_array().expect("checked above") {
        if let Some(name) = answer.get("name").and_then(|name| name.as_str()) {
            by_name.insert(name, answer);
        }
    }
    for (id, question) in &context.questions {
        let parse = match by_name.get(id.as_str()) {
            Some(answer) => parse_answer(label, answer, question, id),
            None => Err(format!("{label} did not return an answer for {id}")),
        };
        match parse {
            Ok(answer) => {
                result.answers.insert(id.clone(), answer);
            }
            Err(message) => {
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

fn unexpected_response(record: &ClassifierModelRecord) -> ClassifierResult {
    let message = format!("{LABEL} returned an unexpected response");
    ClassifierResult::failure(
        record,
        ClassifierStopReason::Error,
        ProviderError::StreamError(ProviderErrorSummary::sanitized(message.clone())),
        message,
    )
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
