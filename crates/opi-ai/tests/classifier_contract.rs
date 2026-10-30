//! Tasks 20.46-20.49: typed classifier public path + the typesafe-system-one,
//! cloudflare-workers-ai-system-one, llama-cpp-classify and openai-decisions
//! adapters.
//!
//! Pins the pinned pi `typesafe-system-one` contract through the public
//! `Models::classify` seam: System One wire shape with the `noul` mapping,
//! per-question typed answer parsing (source-required validations only),
//! billed-usage retention on malformed answers, tiered catalog pricing, the
//! source retry policy (default two extra retries, fresh body-inclusive
//! timeout per attempt, no default timeout), cancellation and pre-dispatch
//! rejections.

use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opi_ai::AuthProvenanceSource;
use opi_ai::api::ClassifierApi;
use opi_ai::api::cloudflare::{
    CLOUDFLARE_WORKERS_AI_SYSTEM_ONE_API_ID, CloudflareSystemOneOptions,
    CloudflareWorkersAiSystemOneApi, default_cloudflare_classifier_records,
};
use opi_ai::api::decisions::{OPENAI_DECISIONS_API_ID, OpenAiDecisionsApi, OpenAiDecisionsOptions};
use opi_ai::api::llama::{
    LLAMA_CPP_CLASSIFY_API_ID, LlamaCppClassifyApi, LlamaCppClassifyOptions, label_probabilities,
    peak_confidence, render_question,
};
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
use opi_ai::models::{ApiId, Models, ModelsError};
use opi_ai::provider::ProviderError;
use opi_ai::stream::Pricing;
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------

fn classifier_pricing() -> ModelPricing {
    // Base input 3.0/M; above 100 prompt tokens the tier raises input to 5.0/M.
    ModelPricing::try_new(
        Pricing {
            input_cost_per_mtok: 3.0,
            output_cost_per_mtok: 15.0,
            cache_read_cost_per_mtok: 0.3,
            cache_write_cost_per_mtok: 3.75,
        },
        vec![PricingTier {
            input_tokens_above: 100,
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

fn classifier_record(base_url: String) -> ClassifierModelRecord {
    ClassifierModelRecord {
        provider_id: "typesafe".into(),
        id: "jev-latest".into(),
        name: "Jev".into(),
        api: ApiId::new(TYPESAFE_SYSTEM_ONE_API_ID),
        base_url,
        input: vec![opi_ai::classifier::ClassifierInputModality::Text],
        context_window: 64000,
        cost: classifier_pricing(),
    }
}

/// A resolver that fails with `CredentialNeeded` to pin the missing-key
/// pre-dispatch rejection through the public seam.
struct MissingKeyResolver;

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

fn classifier_models(
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

fn classify_context() -> ClassifierContext {
    ClassifierContext {
        state: serde_json::json!({ "text": "The deployment succeeded, thank you." }),
        questions: [
            (
                "category".to_owned(),
                ClassifierQuestion::Choice {
                    instructions: "Classify the message".into(),
                    criteria: [
                        ("success".to_owned(), "Successful".to_owned()),
                        ("failure".to_owned(), "Failed".to_owned()),
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

fn ok_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({ "answers": wire_answers() }))
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
    assert_eq!(usage.total_tokens, Some(3));

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
    // pi formatProviderError with the label prefix.
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .starts_with("System One API error (429)"),
        "message was {:?}",
        result.error_message
    );
}

#[tokio::test]
async fn classify_http_failures_format_body_and_map_status() {
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
        Some("System One API error (500): internal glitch")
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

// ---------------------------------------------------------------------
// Selection, registration lifecycle, callbacks, telemetry
// ---------------------------------------------------------------------

#[tokio::test]
async fn classifier_selection_rejects_unknown_before_auth_and_resolves_registered() {
    let server = MockServer::start().await;
    let record = classifier_record(server.uri());
    let counter = Arc::new(AtomicU32::new(0));
    let mut models = Models::new();
    models
        .register_classifier_api(
            Arc::new(TypesafeSystemOneApi::new(HttpClient::new(), record.clone())),
            Arc::new(opi_ai::test_support::CountingAuthResolver::new(
                counter.clone(),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register classifier api");

    // Unknown id: typed rejection.
    match models.classifier_model("typesafe", "unknown-model") {
        Err(ModelsError::NotAClassifierModel { provider, id }) => {
            assert_eq!(
                (provider.as_str(), id.as_str()),
                ("typesafe", "unknown-model")
            );
        }
        other => panic!("expected NotAClassifierModel, got {other:?}"),
    }
    // A chat-model id never matches classifier selection either.
    assert!(matches!(
        models.classifier_model("typesafe", "jev-chat-model"),
        Err(ModelsError::NotAClassifierModel { .. })
    ));
    // The registered record resolves, and selection never resolves auth.
    let resolved = models
        .classifier_model("typesafe", "jev-latest")
        .expect("registered record resolves");
    assert_eq!(resolved.id, "jev-latest");
    assert_eq!(counter.load(AtomicOrdering::SeqCst), 0);
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn classify_without_registration_fails_without_dispatch() {
    let models = Models::new();
    let record = ClassifierModelRecord {
        provider_id: "typesafe".into(),
        id: "jev-latest".into(),
        name: "Jev".into(),
        api: ApiId::new("typesafe-system-one"),
        base_url: "http://127.0.0.1:9".into(),
        input: vec![opi_ai::classifier::ClassifierInputModality::Text],
        context_window: 64000,
        cost: classifier_pricing(),
    };
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("no classifier implementation for api"),
        "message was {:?}",
        result.error_message
    );
}

#[tokio::test]
async fn classifier_handle_options_survive_re_registration_and_reject_mismatch() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ok_response())
        .expect(1)
        .mount(&server)
        .await;

    let record = classifier_record(server.uri());
    let (mut models, handle) = classifier_models(&record);
    let options = handle.options(TypesafeSystemOneOptions::default());

    // A compatible re-registration keeps handle-created options valid.
    models
        .register_classifier_api(
            Arc::new(TypesafeSystemOneApi::new(HttpClient::new(), record.clone())),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("re-register");
    let first = models
        .classify_native(
            record.clone(),
            classify_context(),
            ClassifierCall::default(),
            options,
        )
        .await;
    assert_eq!(first.stop_reason, ClassifierStopReason::Stop);

    // Options tagged for another api id are a typed pre-dispatch failure.
    let mut other_record = classifier_record("http://127.0.0.1:9".into());
    other_record.api = ApiId::new("llama-cpp-classify");
    let other_handle = models
        .register_classifier_api(
            Arc::new(TypesafeSystemOneApi::new(HttpClient::new(), other_record)),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register other api");
    let mismatched = other_handle.options(TypesafeSystemOneOptions::default());
    let second = models
        .classify_native(
            record,
            classify_context(),
            ClassifierCall::default(),
            mismatched,
        )
        .await;
    assert_eq!(second.stop_reason, ClassifierStopReason::Error);
    assert!(
        second
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("do not match the registration"),
        "message was {:?}",
        second.error_message
    );
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
async fn classify_passes_frozen_telemetry_context() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ok_response())
        .expect(1)
        .mount(&server)
        .await;

    let record = classifier_record(server.uri());
    let (models, _handle) = classifier_models(&record);
    let call = ClassifierCall {
        telemetry_context: Some(opi_ai::telemetry::TelemetryContext::noop()),
        ..Default::default()
    };
    let result = models.classify(record, classify_context(), call).await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
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
        vec![opi_ai::classifier::ClassifierInputModality::Text]
    );
    assert_eq!(
        record.cost.base,
        Pricing {
            input_cost_per_mtok: 0.0,
            output_cost_per_mtok: 0.0,
            cache_read_cost_per_mtok: 0.0,
            cache_write_cost_per_mtok: 0.0,
        }
    );
    assert!(record.cost.tiers.is_empty());
}

// =====================================================================
// Task 20.47: cloudflare-workers-ai-system-one adapter
// =====================================================================

/// Resolver carrying the Cloudflare account identity next to the Bearer
/// secret (pi resolves `CLOUDFLARE_ACCOUNT_ID` alongside the API key).
struct CloudflareAccountResolver {
    account_id: Option<String>,
}

impl AuthResolver for CloudflareAccountResolver {
    fn resolve<'a>(
        &'a self,
    ) -> opi_ai::credential::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        let account_id = self.account_id.clone();
        Box::pin(async move {
            Ok(ResolvedAuth {
                scheme: AuthScheme::Bearer,
                secret: SecretString::from("cf-key"),
                base_url: None,
                account_id,
                provenance: Default::default(),
            })
        })
    }
}

fn cloudflare_models(
    record: &ClassifierModelRecord,
    account_id: Option<String>,
) -> (
    Models,
    opi_ai::models::ClassifierApiHandle<CloudflareSystemOneOptions>,
) {
    let mut models = Models::new();
    let handle = models
        .register_classifier_api(
            Arc::new(CloudflareWorkersAiSystemOneApi::new(
                HttpClient::new(),
                record.clone(),
            )),
            Arc::new(CloudflareAccountResolver { account_id }),
            AuthProvenanceSource::Static,
        )
        .expect("register cloudflare classifier api");
    (models, handle)
}

/// `{CLOUDFLARE_ACCOUNT_ID}` placeholder URL rooted at the mock server.
fn cloudflare_record(server_uri: &str, id: &str, cost: ModelPricing) -> ClassifierModelRecord {
    ClassifierModelRecord {
        provider_id: "cloudflare-workers-ai".into(),
        id: id.into(),
        name: id.into(),
        api: ApiId::new(CLOUDFLARE_WORKERS_AI_SYSTEM_ONE_API_ID),
        base_url: format!("{server_uri}/accounts/{{CLOUDFLARE_ACCOUNT_ID}}/ai"),
        input: vec![opi_ai::classifier::ClassifierInputModality::Text],
        context_window: 64000,
        cost,
    }
}

fn cloudflare_jev(server_uri: &str) -> ClassifierModelRecord {
    cloudflare_record(server_uri, "typesafe/jev", zero_pricing())
}

fn zero_pricing() -> ModelPricing {
    ModelPricing::try_new(Pricing::default(), Vec::new()).expect("zero pricing is valid")
}

// Jev's run-record payload from the pinned test
// (https://developers.cloudflare.com/ai/models/typesafe/jev/).
fn jev_run_answers() -> serde_json::Value {
    serde_json::json!({
        "model": "jev-1.13.0",
        "answers": {
            "category": {
                "type": "choice",
                "choice": "failure",
                "confidence": 0.8,
                "probabilities": { "success": 0.13, "failure": 0.87 }
            },
            "satisfaction": { "type": "score", "score": 1, "confidence": 0.6 },
            "approved": { "type": "noul", "noul": 0.95 }
        },
        "usage": { "input_tokens": 426, "output_tokens": 73 }
    })
}

fn run_record_response(state: &str, result: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "result": { "state": state, "result": result, "gatewayMetadata": { "keySource": "Unified" } },
        "success": true,
        "errors": [],
        "messages": []
    }))
}

#[tokio::test]
async fn cloudflare_posts_run_envelope_with_account_url_and_noul() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(run_record_response("Completed", jev_run_answers()))
        .expect(1)
        .mount(&server)
        .await;

    let record = cloudflare_jev(&server.uri());
    let (models, _handle) = cloudflare_models(&record, Some("account-id".into()));
    // The ordinary classify driver (no native options): the simple request
    // shape is the adapter's default options.
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    let usage = result.usage.expect("usage reported");
    assert_eq!(usage.input_tokens, 426);
    assert_eq!(usage.output_tokens, 73);
    assert_eq!(usage.total_tokens, Some(499));
    assert_eq!(
        result.answers.get("approved"),
        Some(&ClassifierAnswer::Bool { probability: 0.95 })
    );

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 1);
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).expect("json body");
    // The Cloudflare envelope wraps the System One request in `input` and
    // carries the full model id.
    assert_eq!(body["model"], "typesafe/jev");
    assert_eq!(
        body["input"]["state"],
        serde_json::json!({ "text": "The deployment succeeded, thank you." })
    );
    assert_eq!(body["input"]["questions"]["category"]["type"], "choice");
    assert_eq!(body["input"]["questions"]["satisfaction"]["type"], "score");
    assert_eq!(body["input"]["questions"]["approved"]["type"], "noul");
    assert!(body.get("state").is_none());
    assert!(body.get("questions").is_none());
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
async fn cloudflare_parses_direct_envelope_and_ignores_temperature() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "model": "clef",
                "answers": {
                    "category": {
                        "type": "choice",
                        "choice": "success",
                        "probabilities": { "success": 0.8368, "failure": 0.1632 },
                        "confidence": 0.4538
                    },
                    "satisfaction": { "type": "score", "score": 2, "confidence": 0.5 },
                    "approved": { "type": "noul", "noul": 0.9912 }
                },
                "usage": { "input_tokens": 222, "output_tokens": 0 }
            },
            "success": true,
            "errors": [],
            "messages": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    let record = cloudflare_jev(&server.uri());
    let (models, handle) = cloudflare_models(&record, Some("account-id".into()));
    // Native options captured on the wire: temperature stays off it.
    let options = handle.options(CloudflareSystemOneOptions {
        temperature: Some(0.7),
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
    let usage = result.usage.expect("usage reported");
    assert_eq!(usage.input_tokens, 222);
    assert_eq!(usage.output_tokens, 0);
    assert_eq!(usage.total_tokens, Some(222));
    assert_eq!(
        result.answers.get("category"),
        Some(&ClassifierAnswer::Choice {
            choice: "success".into(),
            probabilities: [
                ("success".to_owned(), 0.8368),
                ("failure".to_owned(), 0.1632)
            ]
            .into_iter()
            .collect(),
            confidence: 0.4538,
        })
    );
    assert_eq!(
        result.answers.get("approved"),
        Some(&ClassifierAnswer::Bool {
            probability: 0.9912
        })
    );

    let sent = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).expect("json body");
    assert!(body.get("temperature").is_none());
    assert!(body["input"].get("temperature").is_none());
}

#[tokio::test]
async fn cloudflare_reports_incomplete_runs_and_envelope_errors() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(run_record_response("Queued", serde_json::Value::Null))
        .expect(1)
        .mount(&server)
        .await;

    let record = cloudflare_jev(&server.uri());
    let (models, _handle) = cloudflare_models(&record, Some("account-id".into()));
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("run did not complete (state: Queued)"),
        "message was {:?}",
        result.error_message
    );

    // success: false aggregates the envelope's error messages.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": null,
            "success": false,
            "errors": [ { "code": 5007, "message": "No such model" } ],
            "messages": []
        })))
        .expect(1)
        .mount(&server)
        .await;
    let record = cloudflare_jev(&server.uri());
    let (models, _handle) = cloudflare_models(&record, Some("account-id".into()));
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("Cloudflare Workers AI error: No such model"),
        "message was {:?}",
        result.error_message
    );

    // No usable message strings falls back to the source's generic text.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": null,
            "success": false,
            "errors": [ { "code": 7003 }, "not-a-record" ],
            "messages": []
        })))
        .expect(1)
        .mount(&server)
        .await;
    let record = cloudflare_jev(&server.uri());
    let (models, _handle) = cloudflare_models(&record, Some("account-id".into()));
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("Cloudflare Workers AI request failed"),
        "message was {:?}",
        result.error_message
    );
}

#[tokio::test]
async fn cloudflare_reports_unexpected_envelope_shapes() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([1, 2, 3])))
        .mount(&server)
        .await;

    let record = cloudflare_jev(&server.uri());
    let (models, _handle) = cloudflare_models(&record, Some("account-id".into()));
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("Cloudflare Workers AI returned an unexpected response"),
        "message was {:?}",
        result.error_message
    );

    // A completed run whose inner result is not a record is also unexpected.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(run_record_response("Completed", serde_json::json!("boom")))
        .expect(1)
        .mount(&server)
        .await;
    let record = cloudflare_jev(&server.uri());
    let (models, _handle) = cloudflare_models(&record, Some("account-id".into()));
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("Cloudflare Workers AI returned an unexpected response"),
        "message was {:?}",
        result.error_message
    );

    // A failed answer parse inside a valid envelope still keeps the billed
    // usage (shared system-one rule, pinned here through the run envelope).
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(run_record_response(
            "Completed",
            serde_json::json!({
                "answers": {},
                "usage": { "input_tokens": 100, "output_tokens": 5 }
            }),
        ))
        .expect(1)
        .mount(&server)
        .await;
    let record = cloudflare_jev(&server.uri());
    let (models, _handle) = cloudflare_models(&record, Some("account-id".into()));
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("did not return an answer for approved"),
        "message was {:?}",
        result.error_message
    );
    let usage = result.usage.expect("billed usage retained");
    assert_eq!(usage.input_tokens, 100);
    assert_eq!(usage.output_tokens, 5);
}

#[tokio::test]
async fn cloudflare_prices_clef_through_catalog_rates() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "model": "clef",
                "answers": jev_run_answers()["answers"],
                "usage": { "input_tokens": 222, "output_tokens": 0 }
            },
            "success": true
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Curated clef pricing from the pinned test: $0.24 per million input
    // tokens. Register the curated records repointed at the mock server and
    // classify through the registered clef record.
    let mut records: Vec<ClassifierModelRecord> = default_cloudflare_classifier_records();
    for record in &mut records {
        record.base_url = format!("{}/accounts/{{CLOUDFLARE_ACCOUNT_ID}}/ai", server.uri());
    }
    let jev = records.remove(0);
    let (mut models, _handle) = cloudflare_models(&jev, Some("account-id".into()));
    for record in records {
        models
            .register_classifier_model_record(record)
            .expect("register curated record");
    }
    let clef = models
        .classifier_model("cloudflare-workers-ai", "@cf/cloudflare/clef")
        .expect("curated clef resolvable");
    assert_eq!(clef.id, "@cf/cloudflare/clef");

    let result = models
        .classify(clef, classify_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    let cost = result.cost.expect("cost reported");
    let expected = 222.0 * 0.24 / 1_000_000.0;
    assert!(
        (cost.input_cost - expected).abs() < 1e-12,
        "input cost {} vs {expected}",
        cost.input_cost
    );
    assert!(cost.output_cost.abs() < 1e-12);
}

#[tokio::test]
async fn cloudflare_multi_model_registration_resolves_all_curated_records() {
    let mut models = Models::new();
    let mut records = default_cloudflare_classifier_records();
    let first = records.remove(0);
    let adapter = CloudflareWorkersAiSystemOneApi::new(HttpClient::new(), first.clone());
    models
        .register_classifier_api(
            Arc::new(adapter),
            Arc::new(CloudflareAccountResolver { account_id: None }),
            AuthProvenanceSource::Static,
        )
        .expect("register");
    for record in records {
        models
            .register_classifier_model_record(record)
            .expect("append records");
    }
    for id in [
        "typesafe/jev",
        "@cf/cloudflare/clef",
        "@cf/cloudflare/clef-flash",
    ] {
        let record = models
            .classifier_model("cloudflare-workers-ai", id)
            .unwrap_or_else(|error| panic!("resolve {id}: {error}"));
        assert_eq!(record.id, id);
    }
    // Chat models and unknown ids stay typed rejections.
    assert!(matches!(
        models.classifier_model("cloudflare-workers-ai", "unknown"),
        Err(ModelsError::NotAClassifierModel { .. })
    ));
    // Appending a record whose api has no registration is a typed error.
    let orphan = ClassifierModelRecord {
        api: ApiId::new("no-such-classifier-api"),
        ..cloudflare_jev("http://127.0.0.1:9")
    };
    assert!(
        matches!(
            models.register_classifier_model_record(orphan),
            Err(ModelsError::ClassifierApiNotRegistered { .. })
        ),
        "expected ClassifierApiNotRegistered"
    );
}

#[tokio::test]
async fn cloudflare_rejects_image_input_before_any_request() {
    let server = MockServer::start().await;
    let record = cloudflare_jev(&server.uri());
    let (models, _handle) = cloudflare_models(&record, Some("account-id".into()));
    let context = ClassifierContext {
        images: Some(vec![ClassifierImage {
            mime_type: "image/png".into(),
            data: "aGk=".into(),
        }]),
        ..classify_context()
    };
    let result = models
        .classify(record, context, ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("Cloudflare Workers AI does not support image input"),
        "message was {:?}",
        result.error_message
    );
    assert!(matches!(
        result.error,
        Some(ProviderError::UnsupportedCapability { .. })
    ));
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn cloudflare_missing_key_is_typed_pre_dispatch_failure() {
    let server = MockServer::start().await;
    let record = cloudflare_jev(&server.uri());
    let mut models = Models::new();
    models
        .register_classifier_api(
            Arc::new(CloudflareWorkersAiSystemOneApi::new(
                HttpClient::new(),
                record.clone(),
            )),
            Arc::new(MissingCloudflareKeyResolver),
            AuthProvenanceSource::Static,
        )
        .expect("register");
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert_eq!(
        result.error_message.as_deref(),
        Some("No API key for provider: cloudflare-workers-ai")
    );
    assert!(matches!(
        result.error,
        Some(ProviderError::CredentialNeeded { .. })
    ));
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

struct MissingCloudflareKeyResolver;

impl AuthResolver for MissingCloudflareKeyResolver {
    fn resolve<'a>(
        &'a self,
    ) -> opi_ai::credential::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        Box::pin(async move {
            Err(ProviderError::CredentialNeeded {
                provider_id: "cloudflare-workers-ai".into(),
            })
        })
    }
}

#[tokio::test]
async fn cloudflare_missing_account_keeps_placeholder_and_fails_typed() {
    // The placeholder stays in the host when no account identity resolved,
    // so the literal URL is unsendable (pi keeps the placeholder verbatim
    // in `resolveCloudflareModel`'s fallback).
    let mut record = cloudflare_jev("http://{CLOUDFLARE_ACCOUNT_ID}");
    record.base_url = "http://{CLOUDFLARE_ACCOUNT_ID}/accounts/ai".into();
    let (models, _handle) = cloudflare_models(&record, None);
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(result.error.is_some());
    assert!(result.answers.is_empty());
}

#[tokio::test]
async fn cloudflare_adapter_guards_api_and_auth_scheme() {
    // A typesafe-system-one record on the cloudflare adapter is a typed
    // pre-dispatch rejection.
    let mut record = cloudflare_jev("http://127.0.0.1:9");
    record.api = ApiId::new(TYPESAFE_SYSTEM_ONE_API_ID);
    let adapter = CloudflareWorkersAiSystemOneApi::new(HttpClient::new(), record.clone());
    let result = adapter
        .classify_prepared(
            record,
            classify_context(),
            CloudflareSystemOneOptions::default(),
            opi_ai::test_support::resolved_bearer_auth(),
            ClassifierCall::default(),
        )
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

    // Non-Bearer auth is a typed config rejection before any request.
    let record = cloudflare_jev("http://127.0.0.1:9");
    let adapter = CloudflareWorkersAiSystemOneApi::new(HttpClient::new(), record.clone());
    let result = adapter
        .classify_prepared(
            record,
            classify_context(),
            CloudflareSystemOneOptions::default(),
            opi_ai::test_support::resolved_auth(),
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

// =====================================================================
// Task 20.48: llama-cpp-classify adapter
// =====================================================================

/// One token per character: the token id is the character code.
fn llama_char_tokens(content: &str) -> Vec<u64> {
    content.chars().map(|c| c as u64).collect()
}

/// Maps multi-character labels ("Yes"/"No") to single tokens, as a real
/// vocabulary would.
fn llama_word_tokens(content: &str) -> Vec<u64> {
    let words: std::collections::HashMap<&str, u64> =
        [("Yes", 89u64), ("No", 78u64)].into_iter().collect();
    let mut tokens = Vec::new();
    for part in content.split('\n') {
        // Reinsert the newline as its own token, then the text between.
        if !tokens.is_empty() {
            tokens.push(b'\n' as u64);
        }
        if let Some(id) = words.get(part) {
            tokens.push(*id);
        } else {
            tokens.extend(llama_char_tokens(part));
        }
    }
    tokens
}

/// Completion log-probabilities by prompt: bool (Yes/No), score digits or
/// the three choice letters.
fn llama_answer_by_prompt(prompt: &str) -> Vec<(u64, f64)> {
    if prompt.contains("Answer Yes or No.") {
        vec![(b'Y' as u64, -0.05), (b'N' as u64, -3.0)]
    } else if prompt.contains("Answer with one level number.") {
        vec![
            (b'2' as u64, -0.2),
            (b'1' as u64, -1.8),
            (b'0' as u64, -4.0),
        ]
    } else {
        vec![
            (b'B' as u64, -0.3),
            (b'A' as u64, -1.5),
            (b'C' as u64, -3.0),
        ]
    }
}

type LlamaTokenize = Box<dyn Fn(&str) -> Vec<u64> + Send + Sync>;
type LlamaNext = Box<dyn Fn(&str, u64) -> Vec<(u64, f64)> + Send + Sync>;

/// A fake llama-server answering the three classify endpoints. Dynamic by
/// request body, unlike the fixed system-one wiremock templates.
struct LlamaServer {
    tokenize: LlamaTokenize,
    next: LlamaNext,
    /// Fixed prompt overriding the default template rendering.
    template_prompt: Option<String>,
    /// When set, `/completion` answers with this HTTP status instead.
    fail_completion: Option<u16>,
}

impl LlamaServer {
    fn new() -> Self {
        Self {
            tokenize: Box::new(llama_word_tokens),
            next: Box::new(|prompt, _depth| llama_answer_by_prompt(prompt)),
            template_prompt: None,
            fail_completion: None,
        }
    }
}

impl wiremock::Respond for LlamaServer {
    fn respond(&self, request: &wiremock::Request) -> ResponseTemplate {
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap_or_default();
        match request.url.path() {
            "/tokenize" => {
                let content = body["content"].as_str().unwrap_or_default();
                let tokens: Vec<u64> = (self.tokenize)(content);
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "tokens": tokens }))
            }
            "/apply-template" => {
                let prompt = match &self.template_prompt {
                    Some(fixed) => fixed.clone(),
                    None => {
                        let messages = body["messages"].as_array().cloned().unwrap_or_default();
                        let mut prompt = String::new();
                        for message in &messages {
                            prompt.push_str(&format!(
                                "<|{}|>\n{}\n",
                                message["role"].as_str().unwrap_or_default(),
                                message["content"].as_str().unwrap_or_default()
                            ));
                        }
                        prompt.push_str("<|assistant|>\n");
                        prompt
                    }
                };
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "prompt": prompt }))
            }
            "/completion" => {
                if let Some(status) = self.fail_completion {
                    return ResponseTemplate::new(status)
                        .set_body_string(r#"{"error":{"message":"context overflow"}}"#);
                }
                let prompt = body["prompt"].as_str().unwrap_or_default();
                let depth = body["n_probs"].as_u64().unwrap_or(0);
                let top_logprobs: Vec<serde_json::Value> = (self.next)(prompt, depth)
                    .into_iter()
                    .map(|(id, logprob)| {
                        serde_json::json!({
                            "id": id,
                            "token": char::from_u32(id as u32).unwrap_or_default(),
                            "bytes": [],
                            "logprob": logprob,
                        })
                    })
                    .collect();
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "content": "A",
                    "completion_probabilities": [
                        { "id": 65, "token": "A", "top_logprobs": top_logprobs }
                    ]
                }))
            }
            _ => ResponseTemplate::new(404).set_body_string("not found"),
        }
    }
}

async fn mount_llama(server: &MockServer, llama: LlamaServer) {
    Mock::given(method("POST"))
        .respond_with(llama)
        .mount(server)
        .await;
}

struct LlamaAuthResolver;

impl AuthResolver for LlamaAuthResolver {
    fn resolve<'a>(
        &'a self,
    ) -> opi_ai::credential::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        Box::pin(async move {
            Ok(ResolvedAuth {
                scheme: AuthScheme::Bearer,
                secret: SecretString::from("local"),
                base_url: None,
                account_id: None,
                provenance: Default::default(),
            })
        })
    }
}

/// A `qwen` classifier record rooted at the mock server through the
/// OpenAI-compatible `/v1` base URL (pi's llama.cpp models carry it).
fn llama_record(server_uri: &str) -> ClassifierModelRecord {
    ClassifierModelRecord {
        provider_id: "llama.cpp".into(),
        id: "qwen".into(),
        name: "qwen".into(),
        api: ApiId::new(LLAMA_CPP_CLASSIFY_API_ID),
        base_url: format!("{server_uri}/v1"),
        input: vec![opi_ai::classifier::ClassifierInputModality::Text],
        context_window: 32768,
        cost: zero_pricing(),
    }
}

fn llama_models(
    record: &ClassifierModelRecord,
) -> (
    Models,
    opi_ai::models::ClassifierApiHandle<LlamaCppClassifyOptions>,
) {
    let mut models = Models::new();
    let handle = models
        .register_classifier_api(
            Arc::new(LlamaCppClassifyApi::new(HttpClient::new(), record.clone())),
            Arc::new(LlamaAuthResolver),
            AuthProvenanceSource::Static,
        )
        .expect("register llama classifier api");
    (models, handle)
}

fn llama_context() -> ClassifierContext {
    let mut questions = std::collections::BTreeMap::new();
    questions.insert(
        "team".to_owned(),
        ClassifierQuestion::Choice {
            instructions: "Which team should handle this?".into(),
            criteria: [
                ("billing", "Payments and refunds"),
                ("technical", "Bugs and outages"),
                ("sales", ""),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect(),
        },
    );
    questions.insert(
        "urgent".to_owned(),
        ClassifierQuestion::Bool {
            instructions: "Does this convey urgency?".into(),
            true_criteria: "The user needs help soon".into(),
            false_criteria: "No time pressure".into(),
        },
    );
    questions.insert(
        "severity".to_owned(),
        ClassifierQuestion::Score {
            instructions: "How severe is this?".into(),
            levels: vec!["low".into(), "medium".into(), "high".into()],
        },
    );
    ClassifierContext {
        state: serde_json::json!({ "message": "Help! My payouts have been failing for 3 days." }),
        questions,
        images: None,
    }
}

fn llama_pick_context() -> ClassifierContext {
    let mut questions = std::collections::BTreeMap::new();
    questions.insert(
        "pick".to_owned(),
        ClassifierQuestion::Choice {
            instructions: "Pick one".into(),
            criteria: [("a", ""), ("b", "")]
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect(),
        },
    );
    ClassifierContext {
        state: serde_json::json!({}),
        questions,
        images: None,
    }
}

async fn llama_requests(server: &MockServer) -> Vec<(String, serde_json::Value)> {
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|request| {
            let path = request.url.path().to_owned();
            let body = serde_json::from_slice(&request.body).unwrap_or_default();
            (path, body)
        })
        .collect()
}

#[tokio::test]
async fn llama_answers_choice_bool_and_score_from_label_logprobs() {
    let server = MockServer::start().await;
    mount_llama(&server, LlamaServer::new()).await;

    let record = llama_record(&server.uri());
    let (models, _handle) = llama_models(&record);
    let result = models
        .classify(record, llama_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    assert!(result.error_message.is_none());

    // Choice: labels A/B/C over the alphabetical keys billing/sales/technical;
    // next-token logprobs A=-1.5, B=-0.3, C=-3.
    let expected = label_probabilities(&[-1.5, -0.3, -3.0], 1.0);
    let team = result.answers.get("team").expect("team answered");
    match team {
        ClassifierAnswer::Choice {
            choice: chosen,
            probabilities,
            confidence,
        } => {
            assert_eq!(chosen, "sales");
            assert!((probabilities["billing"] - expected[0]).abs() < 1e-9);
            assert!((probabilities["sales"] - expected[1]).abs() < 1e-9);
            assert!((probabilities["technical"] - expected[2]).abs() < 1e-9);
            assert!((confidence - peak_confidence(&expected)).abs() < 1e-9);
            // Hand-derived softmax, not the helper's own arithmetic.
            assert!((probabilities["sales"] - 0.73075).abs() < 1e-4);
            assert!((confidence - 0.59613).abs() < 1e-4);
        }
        other => panic!("team is not a choice answer: {other:?}"),
    }

    // Bool: probability of the `true` key from Yes/No logprobs.
    let urgent = result.answers.get("urgent").expect("urgent answered");
    match urgent {
        ClassifierAnswer::Bool { probability } => {
            assert!((probability - 0.95026).abs() < 1e-4);
        }
        other => panic!("urgent is not a bool answer: {other:?}"),
    }

    // Score: expected level from digit logprobs, TypeSafe confidence.
    let severity = result.answers.get("severity").expect("severity answered");
    match severity {
        ClassifierAnswer::Score { score, confidence } => {
            let levels = label_probabilities(&[-4.0, -1.8, -0.2], 1.0);
            assert!((score - (levels[1] + 2.0 * levels[2])).abs() < 1e-9);
            assert!((score - 1.79858).abs() < 1e-4);
            assert!((confidence - peak_confidence(&levels)).abs() < 1e-9);
        }
        other => panic!("severity is not a score answer: {other:?}"),
    }

    // Wire: the `/v1` base URL is stripped, every body carries the model id
    // and the Bearer key, and the completion readout carries the source shape.
    let requests = llama_requests(&server).await;
    assert!(!requests.is_empty());
    for (path, body) in &requests {
        assert!(
            ["tokenize", "apply-template", "completion"]
                .iter()
                .any(|end| path.ends_with(end)),
            "unexpected endpoint: {path}"
        );
        assert_eq!(body["model"], "qwen");
    }
    let completion = requests
        .iter()
        .find(|(path, _)| path.ends_with("/completion"))
        .expect("completion request");
    assert_eq!(completion.1["n_predict"], 1);
    assert_eq!(completion.1["n_probs"], 256);
    assert_eq!(completion.1["post_sampling_probs"], false);
    assert_eq!(completion.1["cache_prompt"], true);
    assert_eq!(completion.1["temperature"].as_f64(), Some(0.0));
    let template = requests
        .iter()
        .find(|(path, _)| path.ends_with("/apply-template"))
        .expect("template request");
    assert_eq!(
        template.1["chat_template_kwargs"],
        serde_json::json!({ "enable_thinking": false })
    );
    assert_eq!(template.1["messages"][0]["role"], "system");
    assert!(
        template.1["messages"][0]["content"]
            .as_str()
            .unwrap_or_default()
            .starts_with("You answer one question")
    );
    assert_eq!(template.1["messages"][1]["role"], "user");

    // wiremock's recorded URL carries the path only; the endpoints sit
    // directly at the server root, which proves the `/v1` base is stripped.
    let raw = server.received_requests().await.unwrap_or_default();
    for request in &raw {
        assert!(
            ["/tokenize", "/apply-template", "/completion"].contains(&request.url.path()),
            "unexpected path: {}",
            request.url.path()
        );
        assert_eq!(
            request.headers.get("authorization").unwrap(),
            "Bearer local"
        );
    }
}

#[test]
fn llama_renders_questions_with_state_repetition_and_labels() {
    let context = llama_context();
    let rendered = render_question(&context, "team").expect("team renders");
    let state = "State:\n{\n \"message\": \"Help! My payouts have been failing for 3 days.\"\n}";
    assert_eq!(rendered.labels, ["A", "B", "C"]);
    assert_eq!(rendered.keys, ["billing", "sales", "technical"]);
    assert_eq!(
        rendered.content,
        [
            state,
            "",
            "Task: answer each of the following questions about the state.",
            "",
            "Question: How severe is this?",
            "",
            "Levels:",
            "0. low",
            "1. medium",
            "2. high",
            "",
            "Question: Which team should handle this?",
            "",
            "Options:",
            "- billing: Payments and refunds",
            "- sales",
            "- technical: Bugs and outages",
            "",
            "Question: Does this convey urgency?",
            "",
            "Yes means: The user needs help soon",
            "No means: No time pressure",
            "",
            state,
            "",
            "Question: Which team should handle this?",
            "",
            "Options:",
            "A. billing: Payments and refunds",
            "B. sales",
            "C. technical: Bugs and outages",
            "",
            "Answer with one letter.",
        ]
        .join("\n")
    );

    // Everything before the final question is shared across the questions.
    let prefix = |id: &str| {
        let content = render_question(&context, id).unwrap().content;
        content[..content.rfind("Question:").unwrap()].to_owned()
    };
    assert_eq!(prefix("urgent"), prefix("team"));
    assert_eq!(prefix("severity"), prefix("team"));
    assert!(
        render_question(&context, "urgent")
            .unwrap()
            .content
            .ends_with("No means: No time pressure\n\nAnswer Yes or No.")
    );
    assert!(
        render_question(&context, "severity")
            .unwrap()
            .content
            .ends_with("2. high\n\nAnswer with one level number.")
    );
}

#[tokio::test]
async fn llama_divides_logprobs_by_temperature() {
    let server = MockServer::start().await;
    let mut llama = LlamaServer::new();
    llama.tokenize = Box::new(llama_char_tokens);
    llama.next = Box::new(|_prompt, _depth| vec![(b'A' as u64, -0.1), (b'B' as u64, -2.5)]);
    mount_llama(&server, llama).await;

    let record = llama_record(&server.uri());
    let (models, handle) = llama_models(&record);
    let result = models
        .classify_native(
            record,
            llama_pick_context(),
            ClassifierCall::default(),
            handle.options(LlamaCppClassifyOptions {
                temperature: Some(2.0),
                max_retries: 2,
                max_retry_delay_ms: None,
            }),
        )
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    // exp(-0.05) / (exp(-0.05) + exp(-1.25)) and its complement.
    match result.answers.get("pick").expect("pick answered") {
        ClassifierAnswer::Choice { probabilities, .. } => {
            assert!((probabilities["a"] - 0.76852).abs() < 1e-4);
            assert!((probabilities["b"] - 0.23148).abs() < 1e-4);
        }
        other => panic!("pick is not a choice answer: {other:?}"),
    }
}

#[tokio::test]
async fn llama_rejects_non_positive_temperature_before_requests() {
    let server = MockServer::start().await;
    mount_llama(&server, LlamaServer::new()).await;

    let record = llama_record(&server.uri());
    let (models, handle) = llama_models(&record);
    let result = models
        .classify_native(
            record,
            llama_context(),
            ClassifierCall::default(),
            handle.options(LlamaCppClassifyOptions {
                temperature: Some(0.0),
                max_retries: 2,
                max_retry_delay_ms: None,
            }),
        )
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("Temperature must be a positive number, got 0"),
        "message was {:?}",
        result.error_message
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn llama_escalates_readout_depth_and_fails_without_invented_zeros() {
    // A label outside the shallow top list re-reads deeper and recovers.
    let server = MockServer::start().await;
    let mut deep = LlamaServer::new();
    deep.tokenize = Box::new(llama_char_tokens);
    deep.next = Box::new(|_prompt, depth| {
        if depth < 4096 {
            vec![(b'A' as u64, -0.1)]
        } else {
            vec![(b'A' as u64, -0.1), (b'B' as u64, -9.0)]
        }
    });
    mount_llama(&server, deep).await;
    let record = llama_record(&server.uri());
    let (models, _handle) = llama_models(&record);
    let result = models
        .classify(record, llama_pick_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    let depths: Vec<u64> = llama_requests(&server)
        .await
        .into_iter()
        .filter(|(path, _)| path.ends_with("/completion"))
        .map(|(_, body)| body["n_probs"].as_u64().unwrap_or_default())
        .collect();
    assert_eq!(depths, [256, 4096]);

    // A label never ranked within the deepest readout is an error, not a
    // zero-probability answer.
    let server = MockServer::start().await;
    let mut never = LlamaServer::new();
    never.tokenize = Box::new(llama_char_tokens);
    never.next = Box::new(|_prompt, _depth| vec![(b'A' as u64, -0.1)]);
    mount_llama(&server, never).await;
    let record = llama_record(&server.uri());
    let (models, _handle) = llama_models(&record);
    let result = models
        .classify(record, llama_pick_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(result.answers.is_empty());
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("did not rank labels B for pick within the top 32768 tokens"),
        "message was {:?}",
        result.error_message
    );
    let depths: Vec<u64> = llama_requests(&server)
        .await
        .into_iter()
        .filter(|(path, _)| path.ends_with("/completion"))
        .map(|(_, body)| body["n_probs"].as_u64().unwrap_or_default())
        .collect();
    assert_eq!(depths, [256, 4096, 32768]);
}

#[tokio::test]
async fn llama_closes_reasoning_blocks_and_reads_labels_in_reply_position() {
    // A template that leaves a reasoning block open gets it closed.
    let server = MockServer::start().await;
    let mut closed = LlamaServer::new();
    closed.tokenize = Box::new(llama_char_tokens);
    closed.template_prompt = Some("<|assistant|>\n<think>".into());
    mount_llama(&server, closed).await;
    let record = llama_record(&server.uri());
    let (models, _handle) = llama_models(&record);
    let result = models
        .classify(record, llama_pick_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    let completion = llama_requests(&server)
        .await
        .into_iter()
        .find(|(path, _)| path.ends_with("/completion"))
        .expect("completion request");
    assert_eq!(completion.1["prompt"], "<|assistant|>\n<think></think>");

    // A tokenizer that merges a newline with the label falls back to the
    // label alone.
    let server = MockServer::start().await;
    let mut merging = LlamaServer::new();
    merging.tokenize = Box::new(|content| {
        if content.starts_with('\n') && content.len() > 1 {
            vec![1000]
        } else {
            llama_char_tokens(content)
        }
    });
    mount_llama(&server, merging).await;
    let record = llama_record(&server.uri());
    let (models, _handle) = llama_models(&record);
    let result = models
        .classify(record, llama_pick_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);

    // A label the vocabulary splits into several tokens is a typed error.
    let server = MockServer::start().await;
    let mut split = LlamaServer::new();
    split.tokenize = Box::new(llama_char_tokens);
    mount_llama(&server, split).await;
    let record = llama_record(&server.uri());
    let (models, _handle) = llama_models(&record);
    let mut context = llama_pick_context();
    context.questions.insert(
        "ok".to_owned(),
        ClassifierQuestion::Bool {
            instructions: "OK?".into(),
            true_criteria: String::new(),
            false_criteria: String::new(),
        },
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
            .contains(r#"Label "Yes" is not a single token for qwen"#),
        "message was {:?}",
        result.error_message
    );
}

#[tokio::test]
async fn llama_caches_label_tokens_per_server_and_model() {
    let server = MockServer::start().await;
    mount_llama(&server, LlamaServer::new()).await;

    let record = llama_record(&server.uri());
    let (models, _handle) = llama_models(&record);
    let first = models
        .classify(record.clone(), llama_context(), ClassifierCall::default())
        .await;
    assert_eq!(first.stop_reason, ClassifierStopReason::Stop);
    let first_tokenizations = llama_requests(&server)
        .await
        .into_iter()
        .filter(|(path, _)| path.ends_with("/tokenize"))
        .count();
    assert!(first_tokenizations > 0);

    let second = models
        .classify(record, llama_context(), ClassifierCall::default())
        .await;
    assert_eq!(second.stop_reason, ClassifierStopReason::Stop);
    let total_tokenizations = llama_requests(&server)
        .await
        .into_iter()
        .filter(|(path, _)| path.ends_with("/tokenize"))
        .count();
    assert_eq!(total_tokenizations, first_tokenizations);
}

#[tokio::test]
async fn llama_validates_option_counts_before_requests() {
    let server = MockServer::start().await;
    mount_llama(&server, LlamaServer::new()).await;

    let record = llama_record(&server.uri());
    let (models, _handle) = llama_models(&record);

    let mut too_many = llama_pick_context();
    too_many.questions.insert(
        "pick".to_owned(),
        ClassifierQuestion::Choice {
            instructions: "Pick".into(),
            criteria: (0..63)
                .map(|index| (format!("option{index}"), String::new()))
                .collect(),
        },
    );
    let result = models
        .classify(record.clone(), too_many, ClassifierCall::default())
        .await;
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("A choice question needs 2 to 62 options, got 63"),
        "message was {:?}",
        result.error_message
    );

    let mut too_few = llama_pick_context();
    too_few.questions.insert(
        "rate".to_owned(),
        ClassifierQuestion::Score {
            instructions: "Rate".into(),
            levels: vec!["only".into()],
        },
    );
    too_few.questions.remove("pick");
    let result = models
        .classify(record, too_few, ClassifierCall::default())
        .await;
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("A score question needs 2 to 10 levels, got 1"),
        "message was {:?}",
        result.error_message
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn llama_observers_see_only_completion_payloads_and_responses() {
    let server = MockServer::start().await;
    mount_llama(&server, LlamaServer::new()).await;

    let record = llama_record(&server.uri());
    let (models, _handle) = llama_models(&record);
    let payloads = Arc::new(AtomicU32::new(0));
    let statuses = Arc::new(Mutex::new(Vec::<u16>::new()));
    let mut call = ClassifierCall::default();
    let payload_count = Arc::clone(&payloads);
    let status_log = Arc::clone(&statuses);
    call.callbacks = opi_ai::callbacks::RequestCallbacks::new()
        .with_on_payload(Arc::new(move |body: &serde_json::Value| {
            payload_count.fetch_add(1, AtomicOrdering::SeqCst);
            let mut replaced = body.clone();
            replaced["id_slot"] = serde_json::json!(1);
            Ok(Some(replaced))
        }))
        .with_on_response(Arc::new(
            move |status: u16, _headers: &[(String, String)]| {
                status_log.lock().unwrap().push(status);
                Ok(())
            },
        ));

    let result = models.classify(record, llama_pick_context(), call).await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    // Only the completion is observed: one payload across the whole classify.
    assert_eq!(payloads.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(*statuses.lock().unwrap(), vec![200u16]);
    let completion = llama_requests(&server)
        .await
        .into_iter()
        .find(|(path, _)| path.ends_with("/completion"))
        .expect("completion request");
    assert_eq!(completion.1["id_slot"], 1);
}

#[tokio::test]
async fn llama_reports_http_errors_api_mismatch_and_images() {
    // A server error surfaces with the status and body, no retry.
    let server = MockServer::start().await;
    let mut failing = LlamaServer::new();
    failing.fail_completion = Some(400);
    mount_llama(&server, failing).await;
    let record = llama_record(&server.uri());
    let (models, handle) = llama_models(&record);
    let result = models
        .classify_native(
            record,
            llama_context(),
            ClassifierCall::default(),
            handle.options(LlamaCppClassifyOptions {
                temperature: None,
                max_retries: 0,
                max_retry_delay_ms: None,
            }),
        )
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    let message = result.error_message.as_deref().unwrap_or_default();
    assert!(
        message.contains("llama.cpp error (400)"),
        "message was {message}"
    );
    assert!(
        message.contains("context overflow"),
        "message was {message}"
    );

    // Adapter-level api mismatch rejects before any request.
    let server = MockServer::start().await;
    mount_llama(&server, LlamaServer::new()).await;
    let record = llama_record(&server.uri());
    let adapter = LlamaCppClassifyApi::new(HttpClient::new(), record.clone());
    let mut mismatched = record.clone();
    mismatched.api = ApiId::new(TYPESAFE_SYSTEM_ONE_API_ID);
    let result = adapter
        .classify_prepared(
            mismatched,
            llama_context(),
            adapter.default_options(),
            opi_ai::test_support::resolved_bearer_auth(),
            ClassifierCall::default(),
        )
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    let message = result.error_message.as_deref().unwrap_or_default();
    assert!(
        message.contains("Unsupported classifier API: typesafe-system-one"),
        "message was {message}"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 0);

    // Image input rejects before any request.
    let record = llama_record(&server.uri());
    let (models, _handle) = llama_models(&record);
    let mut imaged = llama_context();
    imaged.images = Some(vec![ClassifierImage {
        mime_type: "image/png".into(),
        data: "aGk=".into(),
    }]);
    let result = models
        .classify(record, imaged, ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    let message = result.error_message.as_deref().unwrap_or_default();
    assert!(
        message.contains("llama.cpp classification does not support image input"),
        "message was {message}"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

// ---------------------------------------------------------------------
// OpenAI Decisions (task 20.49)
// ---------------------------------------------------------------------

/// gpt-6-luna catalog pricing from the pinned test: $0.1/M input with a
/// long-context tier at $0.2/M above 272K tokens.
fn decisions_pricing() -> ModelPricing {
    ModelPricing::try_new(
        Pricing {
            input_cost_per_mtok: 0.1,
            ..Pricing::default()
        },
        vec![PricingTier {
            input_tokens_above: 272_000,
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
fn decisions_record(server_uri: &str) -> ClassifierModelRecord {
    ClassifierModelRecord {
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
        cost: decisions_pricing(),
    }
}

fn decisions_models(
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
    // Questions ride as an array in the map's alphabetical order; the pi
    // source order is insertion-based and opi's questions are a BTreeMap.
    let questions = body["questions"].as_array().expect("questions array");
    assert_eq!(questions.len(), 3);
    assert_eq!(questions[0]["type"], "predicate");
    assert_eq!(questions[0]["name"], "approved");
    assert_eq!(
        questions[0]["instructions"],
        "Does the user approve?\n\nTrue means: Approval\nFalse means: No approval"
    );
    assert!(questions[0].get("choices").is_none());
    assert!(questions[0].get("levels").is_none());
    assert_eq!(questions[1]["type"], "choice");
    assert_eq!(questions[1]["name"], "category");
    // Choice options ride in the criteria map's alphabetical order (pi
    // preserves insertion order; opi's criteria are a BTreeMap).
    assert_eq!(
        questions[1]["choices"],
        serde_json::json!([
            { "value": "failure" },
            { "value": "success", "description": "Successful" },
        ])
    );
    assert_eq!(questions[2]["type"], "score");
    assert_eq!(questions[2]["name"], "satisfaction");
    assert_eq!(
        questions[2]["levels"],
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
async fn decisions_reports_api_error_body() {
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
        message.contains("OpenAI Decisions error (400)"),
        "message was {message:?}"
    );
    assert!(
        message.contains("Decision input exceeds the token limit."),
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
async fn four_classifier_protocols_share_one_models_dispatch() {
    // One Models serves all four real classifier protocols; each record
    // routes through the same public classify dispatch while keeping its
    // own wire shape (System One `noul` vs Decisions `predicate` bool are
    // not interchangeable).
    let typesafe_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "answers": { "q": { "type": "noul", "noul": 0.95 } },
        })))
        .expect(1)
        .mount(&typesafe_server)
        .await;

    let cloudflare_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(run_record_response(
            "Completed",
            serde_json::json!({ "answers": { "q": { "type": "noul", "noul": 0.9 } } }),
        ))
        .expect(1)
        .mount(&cloudflare_server)
        .await;

    let llama_server = MockServer::start().await;
    mount_llama(&llama_server, LlamaServer::new()).await;

    let decisions_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/decisions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "answers": [ { "type": "predicate", "name": "q", "probability": 0.7 } ],
        })))
        .expect(1)
        .mount(&decisions_server)
        .await;

    let mut models = Models::new();
    let typesafe_record = classifier_record(typesafe_server.uri());
    models
        .register_classifier_api(
            Arc::new(TypesafeSystemOneApi::new(
                HttpClient::new(),
                typesafe_record.clone(),
            )),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register typesafe");
    let cloudflare_record = cloudflare_jev(&cloudflare_server.uri());
    models
        .register_classifier_api(
            Arc::new(CloudflareWorkersAiSystemOneApi::new(
                HttpClient::new(),
                cloudflare_record.clone(),
            )),
            Arc::new(CloudflareAccountResolver {
                account_id: Some("account-id".into()),
            }),
            AuthProvenanceSource::Static,
        )
        .expect("register cloudflare");
    let llama_record = llama_record(&llama_server.uri());
    models
        .register_classifier_api(
            Arc::new(LlamaCppClassifyApi::new(
                HttpClient::new(),
                llama_record.clone(),
            )),
            Arc::new(LlamaAuthResolver),
            AuthProvenanceSource::Static,
        )
        .expect("register llama");
    let decisions_record = decisions_record(&decisions_server.uri());
    models
        .register_classifier_api(
            Arc::new(OpenAiDecisionsApi::new(
                HttpClient::new(),
                decisions_record.clone(),
            )),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("secret"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register decisions");

    let mut question = std::collections::BTreeMap::new();
    question.insert(
        "q".to_owned(),
        ClassifierQuestion::Bool {
            instructions: "Is this true?".into(),
            true_criteria: "Yes".into(),
            false_criteria: "No".into(),
        },
    );
    let bool_context = ClassifierContext {
        state: serde_json::json!({ "text": "ok" }),
        questions: question,
        images: None,
    };

    // System One bool rides as `noul`.
    let result = models
        .classify(
            typesafe_record,
            bool_context.clone(),
            ClassifierCall::default(),
        )
        .await;
    assert_eq!(
        result.stop_reason,
        ClassifierStopReason::Stop,
        "message was {:?}",
        result.error_message
    );
    assert_eq!(
        result.answers.get("q"),
        Some(&ClassifierAnswer::Bool { probability: 0.95 })
    );

    // Cloudflare wraps the same System One shape in its run envelope.
    let result = models
        .classify(
            cloudflare_record,
            bool_context.clone(),
            ClassifierCall::default(),
        )
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    assert_eq!(
        result.answers.get("q"),
        Some(&ClassifierAnswer::Bool { probability: 0.9 })
    );

    // llama.cpp reads the bool from Yes/No label log-probabilities.
    let result = models
        .classify(llama_record, llama_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    assert!(result.answers.contains_key("urgent"));

    // Decisions maps the same public bool to a `predicate` question.
    let result = models
        .classify(decisions_record, bool_context, ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    assert_eq!(
        result.answers.get("q"),
        Some(&ClassifierAnswer::Bool { probability: 0.7 })
    );

    // The wires differ: `noul` for System One, `predicate` for Decisions.
    let typesafe_sent = typesafe_server.received_requests().await.unwrap();
    let body: serde_json::Value =
        serde_json::from_slice(&typesafe_sent[0].body).expect("json body");
    assert_eq!(body["questions"]["q"]["type"], "noul");
    let decisions_sent = decisions_server.received_requests().await.unwrap();
    let body: serde_json::Value =
        serde_json::from_slice(&decisions_sent[0].body).expect("json body");
    assert_eq!(body["questions"][0]["type"], "predicate");
}
