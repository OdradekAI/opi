//! TypeSafe System One requests, replies and retry behavior.

use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opi_ai::AuthProvenanceSource;
use opi_ai::api::ClassifierApi;
use opi_ai::api::system_one::{
    TYPESAFE_SYSTEM_ONE_API_ID, TypesafeSystemOneApi, TypesafeSystemOneOptions,
    default_typesafe_classifier_record,
};
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::classifier::{
    ClassifierAnswer, ClassifierCall, ClassifierContext, ClassifierImage, ClassifierModelRecord,
    ClassifierQuestion, ClassifierResult, ClassifierStopReason,
};
use opi_ai::http::HttpClient;
use opi_ai::model_info::{ModelPricing, PricingTier};
use opi_ai::models::Models;
use opi_ai::provider::ProviderError;
use opi_ai::stream::Pricing;
use opi_ai::types::ApiId;
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::failure_fixture::{assert_syntax_failure, classify};
use super::fixtures::{assert_failure_diagnostics_private, classify_context};

pub(super) fn classifier_pricing() -> ModelPricing {
    // Base input 3.0/M; above 100 prompt tokens the tier raises input to 5.0/M.
    ModelPricing::try_new(
        Pricing {
            input_cost_per_mtok: 3.0,
            output_cost_per_mtok: 15.0,
            cache_read_cost_per_mtok: 0.3,
            cache_write_cost_per_mtok: 3.75,
        },
        vec![PricingTier {
            input_tokens_above: 100.into(),
            pricing: Pricing {
                input_cost_per_mtok: 5.0,
                output_cost_per_mtok: 15.0,
                cache_read_cost_per_mtok: 0.3,
                cache_write_cost_per_mtok: 3.75,
            },
        }],
    )
    .expect("valid pricing")
}

pub(super) fn classifier_record(base_url: String) -> ClassifierModelRecord {
    ClassifierModelRecord {
        headers: Vec::new(),
        input_limits: None,
        compat: None,
        provider_id: "typesafe".into(),
        id: "jev-latest".into(),
        name: "Jev".into(),
        api: ApiId::new(TYPESAFE_SYSTEM_ONE_API_ID),
        base_url,
        input: vec![opi_ai::classifier::ClassifierInputModality::Text],
        context_window: 64000,
        cost: Some(classifier_pricing()),
    }
}

/// A resolver that fails with `CredentialNeeded` to pin the missing-key
/// pre-dispatch rejection through the public seam.
pub(super) struct MissingKeyResolver;

impl AuthResolver for MissingKeyResolver {
    fn resolve<'a>(
        &'a self,
    ) -> opi_ai::credential::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        Box::pin(async move {
            Err(ProviderError::CredentialNeeded {
                provider_id: "typesafe".into(),
            })
        })
    }
}

pub(super) fn classifier_models(
    record: &ClassifierModelRecord,
) -> (
    Models,
    opi_ai::models::ClassifierApiHandle<TypesafeSystemOneOptions>,
) {
    let mut models = Models::new();
    let handle = models
        .register_classifier_api(
            Arc::new(TypesafeSystemOneApi::new(HttpClient::new(), record.clone())),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register classifier api");
    (models, handle)
}

fn wire_answers() -> serde_json::Value {
    serde_json::json!({
        "category": {
            "type": "choice",
            "choice": "success",
            "probabilities": { "success": 0.9, "failure": 0.1 },
            "confidence": 0.8
        },
        "satisfaction": { "type": "score", "score": 2, "confidence": 0.7 },
        "approved": { "type": "noul", "noul": 0.95 }
    })
}

pub(super) fn ok_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({ "answers": wire_answers() }))
}

#[tokio::test]
async fn classify_http_failure_diagnostics_do_not_echo_provider_body() {
    let canary = "synthetic-classifier-body-private-canary";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": { "message": canary }
        })))
        .expect(1)
        .mount(&server)
        .await;
    let record = classifier_record(server.uri());
    let (models, _) = classifier_models(&record);
    let result = models
        .classify(
            record.clone(),
            classify_context(),
            ClassifierCall::default(),
        )
        .await;

    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    assert_failure_diagnostics_private(&result, &record, canary);
    assert!(matches!(
        result.error,
        Some(ProviderError::ServiceFault { status: 400, .. })
    ));
    assert_eq!(
        result.error_message.as_deref(),
        Some("System One API HTTP 400: [REDACTED]")
    );
    assert!(result.usage.is_none());
}

#[tokio::test]
async fn classify_failure_diagnostics_do_not_echo_probability_keys() {
    let canary = "synthetic-probability-key-private-canary";
    let mut answers = wire_answers();
    answers["category"]["probabilities"][canary] = serde_json::json!("not a number");
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "answers": answers,
            "usage": { "input_tokens": 10, "output_tokens": 2 }
        })))
        .expect(1)
        .mount(&server)
        .await;
    let record = classifier_record(server.uri());
    let (models, _) = classifier_models(&record);
    let result = models
        .classify(
            record.clone(),
            classify_context(),
            ClassifierCall::default(),
        )
        .await;

    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    assert_failure_diagnostics_private(&result, &record, canary);
    assert!(matches!(result.error, Some(ProviderError::StreamError(_))));
    assert!(result.answers.is_empty());
    let usage = result
        .usage
        .expect("billed usage survives malformed probabilities");
    assert_eq!((usage.input_tokens, usage.output_tokens), (10, 2));
}

/// A TCP server that accepts one request and then never responds: proves
/// timeout and cancellation behavior without request accounting.
async fn stalled_server() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stalled server");
    let addr = listener.local_addr().expect("stalled server addr");
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept stalled stream");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read request");
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        // Headers complete: hold the connection open without ever responding.
        std::future::pending::<()>().await;
    });
    format!("http://{addr}")
}

// ---------------------------------------------------------------------
// Wire shape and answer parsing
// ---------------------------------------------------------------------

#[tokio::test]
async fn classify_maps_bool_to_noul_and_parses_all_answer_types() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ok_response())
        .expect(1)
        .mount(&server)
        .await;

    let record = classifier_record(server.uri());
    let (models, handle) = classifier_models(&record);
    // Temperature is a typed option System One cannot apply: it must stay
    // off the wire (the source module has no temperature field).
    let options = handle.options(TypesafeSystemOneOptions {
        temperature: Some(1.5),
        ..Default::default()
    });
    let result = models
        .classify_native(
            record,
            classify_context(),
            ClassifierCall::default(),
            options,
        )
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    assert!(result.error.is_none());
    assert_eq!(
        result.answers.get("category"),
        Some(&ClassifierAnswer::Choice {
            choice: "success".into(),
            probabilities: [("success".to_owned(), 0.9), ("failure".to_owned(), 0.1),]
                .into_iter()
                .collect(),
            confidence: 0.8,
        })
    );
    assert_eq!(
        result.answers.get("satisfaction"),
        Some(&ClassifierAnswer::Score {
            score: 2.0,
            confidence: 0.7,
        })
    );
    assert_eq!(
        result.answers.get("approved"),
        Some(&ClassifierAnswer::Bool { probability: 0.95 })
    );
    assert!(result.usage.is_none());
    assert!(result.cost.is_none());

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 1);
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).expect("json request body");
    assert_eq!(body["model"], "jev-latest");
    assert_eq!(
        body["state"],
        serde_json::json!({ "text": "The deployment succeeded, thank you." })
    );
    // bool questions ride the wire as `noul`; choice/score keep their type
    // and criteria shape.
    assert_eq!(
        body["questions"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["category", "satisfaction", "approved"]
    );
    assert_eq!(
        body["questions"]["category"]["criteria"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["success", "failure"]
    );
    assert_eq!(body["questions"]["category"]["type"], "choice");
    assert_eq!(
        body["questions"]["category"]["criteria"],
        serde_json::json!({ "success": "Successful", "failure": "Failed" })
    );
    assert_eq!(body["questions"]["satisfaction"]["type"], "score");
    assert_eq!(
        body["questions"]["satisfaction"]["levels"],
        serde_json::json!(["low", "neutral", "high"])
    );
    assert_eq!(body["questions"]["approved"]["type"], "noul");
    assert_eq!(
        body["questions"]["approved"]["criteria"],
        serde_json::json!({ "true": "Approval", "false": "No approval" })
    );
    // System One has no temperature field.
    assert!(body.get("temperature").is_none());
    assert!(
        sent[0]
            .headers
            .get("authorization")
            .expect("bearer header")
            .to_str()
            .unwrap()
            .starts_with("Bearer ")
    );
}

#[tokio::test]
async fn classify_preserves_odd_question_ids() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "answers": { "__proto__": { "type": "noul", "noul": 0.75 } }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let record = classifier_record(server.uri());
    let (models, _handle) = classifier_models(&record);
    let context = ClassifierContext {
        state: serde_json::json!({}),
        questions: [(
            "__proto__".to_owned(),
            ClassifierQuestion::Bool {
                instructions: "Is this true?".into(),
                true_criteria: "Yes".into(),
                false_criteria: "No".into(),
            },
        )]
        .into_iter()
        .collect(),
        images: None,
    };
    let result = models
        .classify(record, context, ClassifierCall::default())
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    assert_eq!(
        result.answers.get("__proto__"),
        Some(&ClassifierAnswer::Bool { probability: 0.75 })
    );
}

// ---------------------------------------------------------------------
// Usage, tiers, and billed-request retention
// ---------------------------------------------------------------------

#[tokio::test]
async fn classify_prices_reported_usage_with_catalog_tiers() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "answers": wire_answers(),
            "usage": { "input_tokens": 308, "output_tokens": 23 }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let record = classifier_record(server.uri());
    let (models, _handle) = classifier_models(&record);
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    let usage = result.usage.expect("usage reported");
    assert_eq!(usage.input_tokens, 308);
    assert_eq!(usage.output_tokens, 23);
    assert_eq!(usage.cache_read_tokens, 0);
    assert_eq!(usage.cache_write_tokens, 0);
    assert_eq!(usage.total_tokens, Some(331));
    // 331 prompt tokens exceed the 100-token threshold: the tier's 5.0/M
    // input rate prices the whole request (pi chat-style tiered cost).
    let cost = result.cost.expect("cost reported");
    let expected_input = 5.0 / 1_000_000.0 * 308.0;
    let expected_output = 15.0 / 1_000_000.0 * 23.0;
    assert!((cost.input_cost - expected_input).abs() < 1e-12);
    assert!((cost.output_cost - expected_output).abs() < 1e-12);
    assert!(cost.cache_read_cost.abs() < 1e-12);
    assert!(cost.cache_write_cost.abs() < 1e-12);
    assert!((cost.total_cost() - (expected_input + expected_output)).abs() < 1e-12);
}

#[tokio::test]
async fn classify_tolerates_malformed_and_absent_usage() {
    let malformed = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "answers": wire_answers(),
            "usage": { "input_tokens": "many", "output_tokens": 3 }
        })))
        .expect(1)
        .mount(&malformed)
        .await;
    let record = classifier_record(malformed.uri());
    let (models, _handle) = classifier_models(&record);
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    let usage = result.usage.expect("usage present");
    assert_eq!(usage.input_tokens, 0);
    assert_eq!(usage.output_tokens, 3);
    assert!(!usage.reported, "malformed input is not a reported zero");
    assert_eq!(usage.total_tokens, None);
    assert!(result.cost.is_none());

    let absent = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "answers": wire_answers(),
            "usage": { "cost": 0.1 }
        })))
        .expect(1)
        .mount(&absent)
        .await;
    let record = classifier_record(absent.uri());
    let (models, _handle) = classifier_models(&record);
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    assert!(result.usage.is_none());
    assert!(result.cost.is_none());
}

#[tokio::test]
async fn classify_missing_answer_is_error_but_keeps_billed_usage() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "answers": { "approved": { "type": "noul", "noul": 0.95 } },
            "usage": { "input_tokens": 10, "output_tokens": 2 }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let record = classifier_record(server.uri());
    let (models, _handle) = classifier_models(&record);
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(result.answers.is_empty());
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("did not return an answer for category"),
        "message was {:?}",
        result.error_message
    );
    // The request was billed, so its usage survives the parsing failure.
    let usage = result.usage.expect("usage retained");
    assert_eq!(usage.input_tokens, 10);
    assert_eq!(usage.output_tokens, 2);
    assert!(result.error.is_some());
}

#[tokio::test]
async fn classify_rejects_nonfinite_and_mistyped_answers() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "answers": {
                "category": { "type": "choice", "choice": "success",
                              "probabilities": { "success": 0.5, "failure": "high" },
                              "confidence": 0.8 },
                "satisfaction": { "type": "score", "score": 2, "confidence": 0.7 },
                "approved": { "type": "noul", "noul": 0.95 }
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let record = classifier_record(server.uri());
    let (models, _handle) = classifier_models(&record);
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(result.answers.is_empty());
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("probability for category.failure"),
        "message was {:?}",
        result.error_message
    );
}

// ---------------------------------------------------------------------
// Pre-dispatch rejections
// ---------------------------------------------------------------------

#[tokio::test]
async fn classify_rejects_images_before_any_request() {
    let server = MockServer::start().await;
    let record = classifier_record(server.uri());
    let (models, _handle) = classifier_models(&record);
    let mut context = classify_context();
    context.images = Some(vec![ClassifierImage {
        mime_type: "image/png".into(),
        data: "aW1hZ2U=".into(),
    }]);
    let result = models
        .classify(record, context, ClassifierCall::default())
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert_eq!(
        result.error_message.as_deref(),
        Some("System One API does not support image input")
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn classify_missing_key_fails_typed_before_dispatch() {
    let server = MockServer::start().await;
    let record = classifier_record(server.uri());
    let mut models = Models::new();
    models
        .register_classifier_api(
            Arc::new(TypesafeSystemOneApi::new(HttpClient::new(), record.clone())),
            Arc::new(MissingKeyResolver),
            AuthProvenanceSource::Static,
        )
        .expect("register classifier api");
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert_eq!(
        result.error_message.as_deref(),
        Some("No API key for provider: typesafe")
    );
    assert!(matches!(
        result.error,
        Some(ProviderError::CredentialNeeded { .. })
    ));
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn classify_rejects_unsupported_callback_families() {
    let server = MockServer::start().await;
    let record = classifier_record(server.uri());
    let (models, _handle) = classifier_models(&record);
    let call = ClassifierCall {
        callbacks: opi_ai::callbacks::RequestCallbacks::new()
            .with_on_raw(Arc::new(|_event| Ok(()))),
        ..Default::default()
    };
    let result = models.classify(record, classify_context(), call).await;

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
    assert!(matches!(
        result.error,
        Some(ProviderError::UnsupportedCapability(_))
    ));
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

// ---------------------------------------------------------------------
// Retry policy, timeout, cancellation
// ---------------------------------------------------------------------

#[tokio::test]
async fn classify_defaults_to_two_extra_retries() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after-ms", "0"))
        .up_to_n_times(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ok_response())
        .mount(&server)
        .await;

    let record = classifier_record(server.uri());
    let (models, _handle) = classifier_models(&record);
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    // 429 x2 + success: three hits prove the default of two extra retries.
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}

#[tokio::test]
async fn classify_zero_retries_fail_immediately_with_typed_rate_limit() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after-ms", "0")
                .set_body_string("too busy"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let record = classifier_record(server.uri());
    let (models, handle) = classifier_models(&record);
    let options = handle.options(TypesafeSystemOneOptions {
        max_retries: 0,
        ..Default::default()
    });
    let result = models
        .classify_native(
            record,
            classify_context(),
            ClassifierCall::default(),
            options,
        )
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    assert!(matches!(
        result.error,
        Some(ProviderError::RateLimited { .. })
    ));
    // The public diagnostic retains the service and status without its body.
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .starts_with("System One API HTTP 429: [REDACTED]"),
        "message was {:?}",
        result.error_message
    );
}

#[tokio::test]
async fn classify_http_failures_redact_body_and_map_status() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal glitch"))
        .expect(1)
        .mount(&server)
        .await;

    let record = classifier_record(server.uri());
    let (models, handle) = classifier_models(&record);
    let options = handle.options(TypesafeSystemOneOptions {
        max_retries: 0,
        ..Default::default()
    });
    let result = models
        .classify_native(
            record,
            classify_context(),
            ClassifierCall::default(),
            options,
        )
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert_eq!(
        result.error_message.as_deref(),
        Some("System One API HTTP 500: [REDACTED]")
    );
}

#[tokio::test]
async fn classify_timeout_is_per_attempt_and_body_inclusive() {
    let server = MockServer::start().await;
    // Hit 1: fast 500 with retry-after-ms 0 -> retried immediately.
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(
            ResponseTemplate::new(500)
                .insert_header("retry-after-ms", "0")
                .set_body_string("glitch"),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    // Hit 2: the 200 response body stalls past the 200 ms per-attempt
    // timeout: the attempt fails as a retryable no-status timeout...
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ok_response().set_delay(Duration::from_millis(600)))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    // Hit 3: immediate success. A single global deadline would have expired
    // during hit 2's stall and failed the call.
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ok_response())
        .mount(&server)
        .await;

    let record = classifier_record(server.uri());
    let (models, handle) = classifier_models(&record);
    let options = handle.options(TypesafeSystemOneOptions {
        max_retries: 2,
        ..Default::default()
    });
    let call = ClassifierCall {
        timeout: Some(Duration::from_millis(200)),
        ..Default::default()
    };
    let result = models
        .classify_native(record, classify_context(), call, options)
        .await;

    assert_eq!(
        result.stop_reason,
        ClassifierStopReason::Stop,
        "message was {:?}",
        result.error_message
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}

#[tokio::test]
async fn classify_timeout_failure_uses_source_message() {
    let uri = stalled_server().await;
    let record = classifier_record(uri);
    let (models, handle) = classifier_models(&record);
    let options = handle.options(TypesafeSystemOneOptions {
        max_retries: 0,
        ..Default::default()
    });
    let call = ClassifierCall {
        timeout: Some(Duration::from_millis(50)),
        ..Default::default()
    };
    let result = models
        .classify_native(record, classify_context(), call, options)
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert_eq!(
        result.error_message.as_deref(),
        Some("Request timed out after 50ms")
    );
    assert!(matches!(result.error, Some(ProviderError::Timeout)));
}

#[tokio::test]
async fn classify_has_no_default_timeout_and_cancel_maps_to_aborted() {
    let uri = stalled_server().await;
    let record = classifier_record(uri);
    let (models, _handle) = classifier_models(&record);
    let cancel = CancellationToken::new();
    let call = ClassifierCall {
        cancel: cancel.clone(),
        ..Default::default()
    };
    let mut task =
        tokio::spawn(async move { models.classify(record, classify_context(), call).await });
    // The stalled server holds the request: without a caller timeout the
    // call stays pending (the source has no default timeout)...
    let pending = tokio::time::timeout(Duration::from_millis(300), &mut task).await;
    assert!(pending.is_err(), "classify must not finish on its own");
    // ...until the caller cancels.
    cancel.cancel();
    let result: ClassifierResult = tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .expect("task finishes after cancel")
        .expect("task joins");
    assert_eq!(result.stop_reason, ClassifierStopReason::Aborted);
    assert!(matches!(result.error, Some(ProviderError::Cancelled)));
}

#[tokio::test]
async fn classify_callbacks_observe_payload_then_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ok_response())
        .expect(1)
        .mount(&server)
        .await;

    let record = classifier_record(server.uri());
    let (models, _handle) = classifier_models(&record);
    let events: Arc<Mutex<Vec<&'static str>>> = Arc::new(Mutex::new(Vec::new()));

    let payload_events = events.clone();
    let response_events = events.clone();
    let observed_payload: Arc<Mutex<Option<serde_json::Value>>> = Arc::new(Mutex::new(None));
    let payload_sink = observed_payload.clone();
    let call = ClassifierCall {
        callbacks: opi_ai::callbacks::RequestCallbacks::new()
            .with_on_payload(Arc::new(move |payload| {
                payload_sink.lock().unwrap().replace(payload.clone());
                payload_events.lock().unwrap().push("payload");
                Ok(None)
            }))
            .with_on_response(Arc::new(move |status, _headers| {
                assert_eq!(status, 200);
                response_events.lock().unwrap().push("response");
                Ok(())
            })),
        ..Default::default()
    };
    let result = models.classify(record, classify_context(), call).await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    assert_eq!(*events.lock().unwrap(), vec!["payload", "response"]);
    let payload = observed_payload.lock().unwrap().clone().expect("payload");
    assert_eq!(payload["model"], "jev-latest");
    assert_eq!(payload["questions"]["approved"]["type"], "noul");
}

#[tokio::test]
async fn classify_extra_headers_and_trailing_slash_base() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .and(wiremock::matchers::header("x-source", "test"))
        .respond_with(ok_response())
        .expect(1)
        .mount(&server)
        .await;

    // Trailing slashes on the base URL are stripped when joining.
    let record = classifier_record(format!("{}/", server.uri()));
    let (models, _handle) = classifier_models(&record);
    let call = ClassifierCall {
        extra_headers: vec![("x-source".into(), "test".into())],
        ..Default::default()
    };
    let result = models.classify(record, classify_context(), call).await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
}

// ---------------------------------------------------------------------
// Adapter-level contract (prepared dispatch)
// ---------------------------------------------------------------------

#[tokio::test]
async fn system_one_adapter_rejects_mismatched_api_and_non_bearer_auth() {
    let mut record = classifier_record("http://127.0.0.1:9".into());
    record.api = ApiId::new("cloudflare-workers-ai-system-one");
    let adapter = TypesafeSystemOneApi::new(HttpClient::new(), record.clone());
    let auth = opi_ai::test_support::resolved_bearer_auth();

    let result = adapter
        .classify_prepared(
            record,
            classify_context(),
            TypesafeSystemOneOptions::default(),
            auth,
            ClassifierCall::default(),
        )
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("Unsupported classifier API: cloudflare-workers-ai-system-one"),
        "message was {:?}",
        result.error_message
    );

    // Non-Bearer auth is a typed config rejection before any request.
    let record = classifier_record("http://127.0.0.1:9".into());
    let adapter = TypesafeSystemOneApi::new(HttpClient::new(), record.clone());
    let api_key_auth = opi_ai::test_support::resolved_auth();
    let result = adapter
        .classify_prepared(
            record,
            classify_context(),
            TypesafeSystemOneOptions::default(),
            api_key_auth,
            ClassifierCall::default(),
        )
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("requires Bearer auth scheme"),
        "message was {:?}",
        result.error_message
    );
}

// ---------------------------------------------------------------------
// Curated baseline record
// ---------------------------------------------------------------------

#[test]
fn default_record_is_curated_typesafe_jev() {
    let record = default_typesafe_classifier_record();
    assert_eq!(record.provider_id, "typesafe");
    assert_eq!(record.id, "jev-latest");
    assert_eq!(record.name, "Jev");
    assert_eq!(record.api.as_str(), TYPESAFE_SYSTEM_ONE_API_ID);
    assert_eq!(record.base_url, "https://api.typesafe.ai/v1");
    assert_eq!(record.context_window, 64000);
    assert_eq!(
        record.input,
        vec![
            opi_ai::classifier::ClassifierInputModality::Text,
            opi_ai::classifier::ClassifierInputModality::Image
        ]
    );
    assert_eq!(
        record.compat.as_ref().unwrap().image_input,
        Some(opi_ai::classifier::ClassifierImageInput::Assets)
    );
    assert_eq!(
        record.compat.as_ref().unwrap().send_image_opt_in_header,
        Some(true)
    );
    assert_eq!(
        record.cost.as_ref().unwrap().base,
        Pricing {
            input_cost_per_mtok: 0.0,
            output_cost_per_mtok: 0.0,
            cache_read_cost_per_mtok: 0.0,
            cache_write_cost_per_mtok: 0.0,
        }
    );
    assert!(record.cost.as_ref().unwrap().tiers.is_empty());
}

#[tokio::test]
async fn typesafe_json_syntax_failure_does_not_retry() {
    assert_syntax_failure("typesafe").await;
}

#[tokio::test]
async fn network_failure_and_body_timeout_keep_acquisition_retry() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    for drop_before_headers in [false, true] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let headers_sent = Arc::new(AtomicU32::new(0));
        let header_counter = headers_sent.clone();
        let server = tokio::spawn(async move {
            let mut workers = Vec::new();
            for attempt in 0..2 {
                let (mut socket, _) = listener.accept().await.unwrap();
                let counter = header_counter.clone();
                workers.push(tokio::spawn(async move {
                let mut received = Vec::new();
                let mut bytes = [0; 4096];
                loop {
                    let count = socket.read(&mut bytes).await.unwrap();
                    assert_ne!(count, 0, "request must complete before the response");
                    received.extend_from_slice(&bytes[..count]);
                    if let Some(end) = received.windows(4).position(|v| v == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&received[..end]);
                        let length = headers.lines().find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().unwrap())
                        }).unwrap();
                        if received.len() >= end + 4 + length { break; }
                    }
                }
                let body = serde_json::json!({"answers": {"q": {"type": "noul", "noul": 0.7}}}).to_string();
                if attempt == 0 && drop_before_headers {
                    return;
                }
                let headers = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                socket.write_all(headers.as_bytes()).await.unwrap();
                socket.flush().await.unwrap();
                counter.fetch_add(1, AtomicOrdering::SeqCst);
                if attempt == 0 {
                    socket.write_all(&body.as_bytes()[..1]).await.unwrap();
                    socket.flush().await.unwrap();
                    tokio::time::sleep(Duration::from_millis(600)).await;
                    let _ = socket.write_all(&body.as_bytes()[1..]).await;
                } else {
                    socket.write_all(body.as_bytes()).await.unwrap();
                }
            }));
            }
            for worker in workers {
                worker.await.unwrap();
            }
        });
        let result = classify(
            base_url,
            "typesafe",
            Some(1),
            ClassifierCall {
                timeout: Some(Duration::from_millis(200)),
                ..Default::default()
            },
        )
        .await;
        tokio::time::timeout(Duration::from_secs(3), server)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            headers_sent.load(AtomicOrdering::SeqCst),
            2 - u32::from(drop_before_headers)
        );
        assert_eq!(
            result.stop_reason,
            ClassifierStopReason::Stop,
            "{:?}",
            result.error
        );
    }
}
