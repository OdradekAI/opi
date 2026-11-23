//! OpenAI Decisions requests, refusal handling and usage accounting.

use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex};

use opi_ai::AuthProvenanceSource;
use opi_ai::api::ClassifierApi;
use opi_ai::api::openai_decisions::{
    OPENAI_DECISIONS_API_ID, OpenAiDecisionsApi, OpenAiDecisionsOptions,
};
use opi_ai::auth::{AuthScheme, ResolvedAuth};
use opi_ai::classifier::{
    ClassifierAnswer, ClassifierCall, ClassifierContext, ClassifierImage, ClassifierModelRecord,
    ClassifierQuestion, ClassifierStopReason,
};
use opi_ai::http::HttpClient;
use opi_ai::model_info::{ModelPricing, PricingTier};
use opi_ai::models::Models;
use opi_ai::provider::ProviderError;
use opi_ai::stream::Pricing;
use opi_ai::types::ApiId;
use secrecy::SecretString;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::failure_fixture::assert_syntax_failure;
use super::fixtures::bool_context;

/// gpt-6-luna catalog pricing from the pinned test: $0.1/M input with a
/// long-context tier at $0.2/M above 272K tokens.
fn decisions_pricing() -> ModelPricing {
    ModelPricing::try_new(
        Pricing {
            input_cost_per_mtok: 0.1,
            ..Pricing::default()
        },
        vec![PricingTier {
            input_tokens_above: 272_000.into(),
            pricing: Pricing {
                input_cost_per_mtok: 0.2,
                ..Pricing::default()
            },
        }],
    )
    .expect("valid decisions pricing")
}

/// A gpt-6-luna record rooted at the mock server through the OpenAI `/v1`
/// base URL.
pub(super) fn decisions_record(server_uri: &str) -> ClassifierModelRecord {
    ClassifierModelRecord {
        headers: Vec::new(),
        input_limits: None,
        compat: None,
        provider_id: "openai".into(),
        id: "gpt-6-luna".into(),
        name: "GPT-6 Luna".into(),
        api: ApiId::new(OPENAI_DECISIONS_API_ID),
        base_url: format!("{server_uri}/v1"),
        input: vec![
            opi_ai::classifier::ClassifierInputModality::Text,
            opi_ai::classifier::ClassifierInputModality::Image,
        ],
        context_window: 922_000,
        cost: Some(decisions_pricing()),
    }
}

pub(super) fn decisions_models(
    record: &ClassifierModelRecord,
    secret: &str,
) -> (
    Models,
    opi_ai::models::ClassifierApiHandle<OpenAiDecisionsOptions>,
) {
    let mut models = Models::new();
    let handle = models
        .register_classifier_api(
            Arc::new(OpenAiDecisionsApi::new(HttpClient::new(), record.clone())),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from(secret),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register decisions classifier api");
    (models, handle)
}

/// The pinned three-question context with an empty `failure` description:
/// empty descriptions stay off the wire.
fn decisions_context() -> ClassifierContext {
    ClassifierContext {
        state: serde_json::json!({ "text": "The deployment succeeded, thank you." }),
        questions: [
            (
                "category".to_owned(),
                ClassifierQuestion::Choice {
                    instructions: "Classify the message".into(),
                    criteria: [
                        ("success".to_owned(), "Successful".to_owned()),
                        ("failure".to_owned(), String::new()),
                    ]
                    .into_iter()
                    .collect(),
                },
            ),
            (
                "satisfaction".to_owned(),
                ClassifierQuestion::Score {
                    instructions: "Score satisfaction".into(),
                    levels: vec!["low".into(), "neutral".into(), "high".into()],
                },
            ),
            (
                "approved".to_owned(),
                ClassifierQuestion::Bool {
                    instructions: "Does the user approve?".into(),
                    true_criteria: "Approval".into(),
                    false_criteria: "No approval".into(),
                },
            ),
        ]
        .into_iter()
        .collect(),
        images: None,
    }
}

/// Wire answers from the API reference and live gpt-6-luna requests.
fn decisions_wire_answers() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "type": "choice", "name": "category", "choice": "success",
            "probabilities": [
                { "value": "success", "probability": 0.9 },
                { "value": "failure", "probability": 0.1 },
            ],
            "confidence": 0.8,
        }),
        serde_json::json!({
            "type": "score", "name": "satisfaction", "score": 1.8,
            "probabilities": [
                { "value": 0, "label": "low", "probability": 0.05 },
                { "value": 1, "label": "neutral", "probability": 0.1 },
                { "value": 2, "label": "high", "probability": 0.85 },
            ],
            "confidence": 0.7,
        }),
        serde_json::json!({ "type": "predicate", "name": "approved", "probability": 0.95 }),
    ]
}

fn decisions_usage() -> serde_json::Value {
    serde_json::json!({
        "input_tokens": 164,
        "input_tokens_details": { "cached_tokens": 0, "cache_write_tokens": 0 },
        "output_tokens": 0,
        "output_tokens_details": { "reasoning_tokens": 0 },
        "total_tokens": 164,
    })
}

fn decisions_ok(answers: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "model": "gpt-6-luna",
        "answers": answers,
        "usage": decisions_usage(),
    }))
}

#[tokio::test]
async fn decisions_maps_questions_and_answers_back_by_name() {
    let server = MockServer::start().await;
    // Answers arrive out of question order: they are matched by name.
    let mut reversed = decisions_wire_answers();
    reversed.reverse();
    Mock::given(method("POST"))
        .and(path("/v1/decisions"))
        .respond_with(decisions_ok(serde_json::Value::Array(reversed)))
        .expect(1)
        .mount(&server)
        .await;

    let record = decisions_record(&server.uri());
    let (models, _handle) = decisions_models(&record, "secret");
    let result = models
        .classify(record, decisions_context(), ClassifierCall::default())
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    assert_eq!(
        result.answers.get("category"),
        Some(&ClassifierAnswer::Choice {
            choice: "success".into(),
            probabilities: [("failure".to_owned(), 0.1), ("success".to_owned(), 0.9)]
                .into_iter()
                .collect(),
            confidence: 0.8,
        })
    );
    assert_eq!(
        result.answers.get("satisfaction"),
        Some(&ClassifierAnswer::Score {
            score: 1.8,
            confidence: 0.7
        })
    );
    assert_eq!(
        result.answers.get("approved"),
        Some(&ClassifierAnswer::Bool { probability: 0.95 })
    );
    let usage = result.usage.expect("usage reported");
    assert_eq!(usage.input_tokens, 164);
    assert_eq!(usage.output_tokens, 0);
    assert_eq!(usage.total_tokens, Some(164));
    assert_eq!(usage.cache_read_tokens, 0);
    // 164 tokens at the base $0.1/M rate.
    let cost = result.cost.expect("cost reported");
    assert!((cost.total_cost() - 0.0000164).abs() < 1e-12);

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].url.path(), "/v1/decisions");
    assert_eq!(
        sent[0].headers.get("authorization").unwrap(),
        "Bearer secret"
    );
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).expect("json body");
    assert_eq!(body["model"], "gpt-6-luna");
    // Without images the input is the state serialized to a JSON *string*.
    assert_eq!(
        body["input"],
        serde_json::json!(r#"{"text":"The deployment succeeded, thank you."}"#)
    );
    assert!(body.get("temperature").is_none());
    // Questions and choices retain the caller's declaration order.
    let questions = body["questions"].as_array().expect("questions array");
    assert_eq!(questions.len(), 3);
    assert_eq!(questions[2]["type"], "predicate");
    assert_eq!(questions[2]["name"], "approved");
    assert_eq!(
        questions[2]["instructions"],
        "Does the user approve?\n\nTrue means: Approval\nFalse means: No approval"
    );
    assert!(questions[2].get("choices").is_none());
    assert!(questions[2].get("levels").is_none());
    assert_eq!(questions[0]["type"], "choice");
    assert_eq!(questions[0]["name"], "category");
    assert_eq!(
        questions[0]["choices"],
        serde_json::json!([
            { "value": "success", "description": "Successful" },
            { "value": "failure" },
        ])
    );
    assert_eq!(questions[1]["type"], "score");
    assert_eq!(questions[1]["name"], "satisfaction");
    assert_eq!(
        questions[1]["levels"],
        serde_json::json!([
            { "label": "low" },
            { "label": "neutral" },
            { "label": "high" },
        ])
    );
}

#[tokio::test]
async fn decisions_prices_long_context_at_tier_rate() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/decisions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "answers": decisions_wire_answers(),
            "usage": { "input_tokens": 300_000, "output_tokens": 0 },
        })))
        .expect(1)
        .mount(&server)
        .await;

    let record = decisions_record(&server.uri());
    let (models, _handle) = decisions_models(&record, "secret");
    let result = models
        .classify(record, decisions_context(), ClassifierCall::default())
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    // 300K tokens exceeds the 272K threshold: the whole request prices at
    // the $0.2/M long-context rate.
    let cost = result.cost.expect("cost reported");
    assert!((cost.total_cost() - 0.06).abs() < 1e-9);
}

#[tokio::test]
async fn decisions_sends_images_after_state_in_one_user_message() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/decisions"))
        .respond_with(decisions_ok(serde_json::Value::Array(
            decisions_wire_answers(),
        )))
        .expect(1)
        .mount(&server)
        .await;

    let record = decisions_record(&server.uri());
    let (models, _handle) = decisions_models(&record, "secret");
    let mut context = decisions_context();
    context.images = Some(vec![
        ClassifierImage {
            data: "aW1hZ2U=".into(),
            mime_type: "image/png".into(),
        },
        ClassifierImage {
            data: "aW1hZ2U=".into(),
            mime_type: "image/jpeg".into(),
        },
    ]);
    let result = models
        .classify(record, context, ClassifierCall::default())
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    let sent = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).expect("json body");
    assert_eq!(
        body["input"],
        serde_json::json!([{
            "role": "user",
            "content": [
                { "type": "input_text", "text": r#"{"text":"The deployment succeeded, thank you."}"# },
                { "type": "input_image", "image_url": "data:image/png;base64,aW1hZ2U=" },
                { "type": "input_image", "image_url": "data:image/jpeg;base64,aW1hZ2U=" },
            ],
        }])
    );
}

#[tokio::test]
async fn decisions_rejects_more_than_128_images_before_sending() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/decisions"))
        .respond_with(decisions_ok(serde_json::Value::Array(
            decisions_wire_answers(),
        )))
        .expect(0)
        .mount(&server)
        .await;

    let record = decisions_record(&server.uri());
    let (models, _handle) = decisions_models(&record, "secret");
    let mut context = decisions_context();
    context.images = Some(
        std::iter::repeat_n(
            ClassifierImage {
                data: "aW1hZ2U=".into(),
                mime_type: "image/png".into(),
            },
            129,
        )
        .collect(),
    );
    let result = models
        .classify(record, context, ClassifierCall::default())
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("OpenAI Decisions accepts at most 128 images, got 129"),
        "message was {:?}",
        result.error_message
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn decisions_refusal_fails_result_but_keeps_billed_usage() {
    let server = MockServer::start().await;
    let answers = decisions_wire_answers();
    Mock::given(method("POST"))
        .and(path("/v1/decisions"))
        .respond_with(decisions_ok(serde_json::json!([
            answers[0], answers[1],
            { "type": "refusal", "name": "approved" },
        ])))
        .expect(1)
        .mount(&server)
        .await;

    let record = decisions_record(&server.uri());
    let (models, _handle) = decisions_models(&record, "secret");
    let result = models
        .classify(record, decisions_context(), ClassifierCall::default())
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(result.answers.is_empty());
    assert_eq!(
        result.error_message.as_deref(),
        Some("OpenAI Decisions refused to answer approved")
    );
    // The request was still billed: the reported usage survives.
    assert_eq!(result.usage.expect("usage retained").input_tokens, 164);
}

#[tokio::test]
async fn decisions_missing_mistyped_and_nonfinite_answers_are_typed_errors() {
    // Missing answer for one question.
    let missing = MockServer::start().await;
    let answers = decisions_wire_answers();
    Mock::given(method("POST"))
        .and(path("/v1/decisions"))
        .respond_with(decisions_ok(serde_json::json!([answers[0], answers[1]])))
        .expect(1)
        .mount(&missing)
        .await;
    let record = decisions_record(&missing.uri());
    let (models, _handle) = decisions_models(&record, "secret");
    let result = models
        .classify(record, decisions_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("OpenAI Decisions did not return an answer for approved"),
        "message was {:?}",
        result.error_message
    );

    // A score answer where the question is a predicate.
    let mistyped = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/decisions"))
        .respond_with(decisions_ok(serde_json::json!([
            answers[0], answers[1],
            { "type": "score", "name": "approved", "score": 1, "confidence": 0.5 },
        ])))
        .expect(1)
        .mount(&mistyped)
        .await;
    let record = decisions_record(&mistyped.uri());
    let (models, _handle) = decisions_models(&record, "secret");
    let result = models
        .classify(record, decisions_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("OpenAI Decisions did not return a predicate answer for approved"),
        "message was {:?}",
        result.error_message
    );

    // A non-number confidence fails the required-number check.
    let nonfinite = MockServer::start().await;
    let mut string_confidence = answers[0].clone();
    string_confidence["confidence"] = serde_json::json!("0.8");
    Mock::given(method("POST"))
        .and(path("/v1/decisions"))
        .respond_with(decisions_ok(serde_json::json!([
            string_confidence,
            answers[1],
            answers[2],
        ])))
        .expect(1)
        .mount(&nonfinite)
        .await;
    let record = decisions_record(&nonfinite.uri());
    let (models, _handle) = decisions_models(&record, "secret");
    let result = models
        .classify(record, decisions_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("OpenAI Decisions returned an invalid confidence for category"),
        "message was {:?}",
        result.error_message
    );
}

#[tokio::test]
async fn decisions_does_not_retry_gateway_timeouts_and_explains() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/decisions"))
        .respond_with(
            ResponseTemplate::new(504)
                .insert_header("retry-after-ms", "0")
                .set_body_string("<!DOCTYPE html><html>Gateway time-out</html>"),
        )
        // The same input would time out again: 504 never retries.
        .expect(1)
        .mount(&server)
        .await;

    let record = decisions_record(&server.uri());
    let (models, _handle) = decisions_models(&record, "secret");
    let result = models
        .classify(record, decisions_context(), ClassifierCall::default())
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    let message = result.error_message.as_deref().unwrap_or_default();
    assert!(
        message.starts_with("OpenAI Decisions error (504): the request timed out at the gateway"),
        "message was {message:?}"
    );
    assert!(
        message.contains(
            "Very large inputs (above roughly 600K tokens) currently exceed its time limit."
        ),
        "message was {message:?}"
    );
    assert!(!message.contains("<html>"));
}

#[tokio::test]
async fn decisions_still_retries_other_server_errors_with_frozen_payload() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/decisions"))
        .respond_with(
            ResponseTemplate::new(503)
                .insert_header("retry-after-ms", "0")
                .set_body_string("busy"),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/decisions"))
        .respond_with(decisions_ok(serde_json::Value::Array(
            decisions_wire_answers(),
        )))
        .mount(&server)
        .await;

    let record = decisions_record(&server.uri());
    let (models, _handle) = decisions_models(&record, "secret");
    let result = models
        .classify(record, decisions_context(), ClassifierCall::default())
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 2);
    // The payload is frozen before the retry loop: both attempts are
    // byte-for-byte identical.
    assert_eq!(sent[0].body, sent[1].body);
}

#[tokio::test]
async fn decisions_reports_redacted_api_error_status() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/decisions"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": { "message": "Decision input exceeds the token limit.", "type": "invalid_request_error" },
        })))
        .expect(1)
        .mount(&server)
        .await;

    let record = decisions_record(&server.uri());
    let (models, handle) = decisions_models(&record, "secret");
    let options = handle.options(OpenAiDecisionsOptions {
        max_retries: 0,
        ..Default::default()
    });
    let result = models
        .classify_native(
            record,
            decisions_context(),
            ClassifierCall::default(),
            options,
        )
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    let message = result.error_message.as_deref().unwrap_or_default();
    assert!(
        message.contains("OpenAI Decisions HTTP 400: [REDACTED]"),
        "message was {message:?}"
    );
    assert!(
        !message.contains("Decision input exceeds the token limit."),
        "message was {message:?}"
    );
}

#[tokio::test]
async fn decisions_rejects_other_apis_missing_keys_and_raw_observers() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/decisions"))
        .respond_with(decisions_ok(serde_json::Value::Array(
            decisions_wire_answers(),
        )))
        .expect(0)
        .mount(&server)
        .await;

    // A record for another classifier API.
    let mut other = decisions_record(&server.uri());
    other.api = ApiId::new("typesafe-system-one");
    let (models, _handle) = decisions_models(&other, "secret");
    let result = models
        .classify(other, decisions_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("Unsupported classifier API: typesafe-system-one"),
        "message was {:?}",
        result.error_message
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 0);

    // An empty API key: Decisions is API-key-only and fails typed.
    let record = decisions_record(&server.uri());
    let (models, _handle) = decisions_models(&record, "");
    let result = models
        .classify(record, decisions_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("No API key for provider: openai"),
        "message was {:?}",
        result.error_message
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 0);

    // Raw observers are not part of the classifier callback surface.
    let record = decisions_record(&server.uri());
    let (models, _handle) = decisions_models(&record, "secret");
    let call = ClassifierCall {
        callbacks: opi_ai::callbacks::RequestCallbacks::new()
            .with_on_raw(Arc::new(|_event| Ok(()))),
        ..Default::default()
    };
    let result = models.classify(record, decisions_context(), call).await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("payload and response observers only"),
        "message was {:?}",
        result.error_message
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn decisions_observers_transform_payload_and_see_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/decisions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "answers": decisions_wire_answers(),
            "usage": decisions_usage(),
        })))
        .expect(1)
        .mount(&server)
        .await;

    let record = decisions_record(&server.uri());
    let (models, _handle) = decisions_models(&record, "secret");
    let payloads = Arc::new(AtomicU32::new(0));
    let statuses = Arc::new(Mutex::new(Vec::new()));
    let call = ClassifierCall {
        callbacks: opi_ai::callbacks::RequestCallbacks::new()
            .with_on_payload({
                let payloads = payloads.clone();
                Arc::new(move |body: &serde_json::Value| {
                    payloads.fetch_add(1, AtomicOrdering::SeqCst);
                    let mut replaced = body.clone();
                    replaced["metadata_probe"] = serde_json::json!("probe");
                    Ok(Some(replaced))
                })
            })
            .with_on_response({
                let statuses = statuses.clone();
                Arc::new(move |status: u16, _headers: &[(String, String)]| {
                    statuses.lock().unwrap().push(status);
                    Ok(())
                })
            }),
        ..Default::default()
    };
    let result = models.classify(record, decisions_context(), call).await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    assert_eq!(payloads.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(*statuses.lock().unwrap(), vec![200]);
    let sent = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).expect("json body");
    // The transformed payload is what reached the wire.
    assert_eq!(body["metadata_probe"], "probe");
}

#[tokio::test]
async fn decisions_text_only_record_rejects_images_on_direct_path() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(decisions_ok(serde_json::json!(decisions_wire_answers())))
        .mount(&server)
        .await;
    let mut record = decisions_record(&server.uri());
    record.input = vec![opi_ai::classifier::ClassifierInputModality::Text];
    let adapter = OpenAiDecisionsApi::new(HttpClient::new(), record.clone());
    let mut context = decisions_context();
    context.images = Some(vec![ClassifierImage {
        mime_type: "image/png".into(),
        data: "QUJD".into(),
    }]);
    let result = adapter
        .classify_prepared(
            record,
            context,
            adapter.default_options(),
            ResolvedAuth {
                env: Default::default(),
                scheme: AuthScheme::Bearer,
                secret: SecretString::from("fixture-key"),
                base_url: None,
                account_id: None,
                provenance: Default::default(),
            },
            ClassifierCall::default(),
        )
        .await;
    assert!(matches!(
        result.error,
        Some(ProviderError::UnsupportedCapability(_))
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn decisions_json_syntax_failure_does_not_retry() {
    assert_syntax_failure("decisions").await;
}

#[tokio::test]
async fn decisions_failed_question_clears_answers_and_keeps_accounting() {
    for invalid_first in [false, true] {
        for kind in ["refusal", "missing", "wrong_type", "invalid_number"] {
            let (valid, invalid) = if invalid_first {
                ("b", "a")
            } else {
                ("a", "b")
            };
            let valid = serde_json::json!({"name": valid, "type": "predicate", "probability": 0.7});
            let invalid = match kind {
                "refusal" => serde_json::json!({"name": invalid, "type": "refusal"}),
                "wrong_type" => {
                    serde_json::json!({"name": invalid, "type": "score", "score": 1})
                }
                _ => {
                    serde_json::json!({"name": invalid, "type": "predicate", "probability": "bad"})
                }
            };
            let mut answers = vec![valid];
            if kind != "missing" {
                answers.push(invalid);
            }
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(decisions_ok(serde_json::json!(answers)))
                .expect(1)
                .mount(&server)
                .await;
            let record = decisions_record(&server.uri());
            let (models, _) = decisions_models(&record, "fixture-key");
            let result = models
                .classify(record, bool_context(&["a", "b"]), Default::default())
                .await;
            assert_eq!(result.stop_reason, ClassifierStopReason::Error);
            assert!(
                result.answers.is_empty(),
                "{kind}, invalid_first={invalid_first}: {:?}",
                result.answers
            );
            assert!(matches!(result.error, Some(ProviderError::StreamError(_))));
            let usage = result.usage.unwrap();
            assert!(usage.reported);
            assert_eq!(usage.input_tokens, 164);
            let cost = result.cost.unwrap().total_cost();
            assert!((cost - 164.0 * 0.1 / 1_000_000.0).abs() < 1e-12);
        }
    }
}
