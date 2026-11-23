//! Frozen telemetry through Typesafe System One classification and callbacks.

use opi_ai::api::system_one::{TYPESAFE_SYSTEM_ONE_API_ID, TypesafeSystemOneApi};
use opi_ai::auth::{AuthProvenanceSource, AuthScheme, StaticAuthResolver};
use opi_ai::callbacks::RequestCallbacks;
use opi_ai::classifier::{
    ClassifierAnswer, ClassifierCall, ClassifierContext, ClassifierModelRecord, ClassifierQuestion,
    ClassifierStopReason,
};
use opi_ai::http::HttpClient;
use opi_ai::models::Models;
use opi_ai::telemetry::TelemetryContext;
use opi_ai::types::ApiId;
use tracing::Level;
use tracing_core::dispatcher;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::fixtures::{
    PROMPT_CANARY, SECRET_CANARY, assert_all_parented, assert_no_canaries,
    assert_no_telemetry_keys, await_under_ambient, probe_spans, recorder,
};

fn classifier_record(base_url: String) -> ClassifierModelRecord {
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
        cost: None,
    }
}

fn classifier_models(
    record: &ClassifierModelRecord,
) -> (
    Models,
    opi_ai::models::ClassifierApiHandle<opi_ai::api::system_one::TypesafeSystemOneOptions>,
) {
    let mut models = Models::new();
    let handle = models
        .register_classifier_api(
            std::sync::Arc::new(TypesafeSystemOneApi::new(HttpClient::new(), record.clone())),
            std::sync::Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                secrecy::SecretString::from(SECRET_CANARY),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register classifier api");
    (models, handle)
}

fn classify_state() -> ClassifierContext {
    ClassifierContext {
        state: serde_json::json!({ "text": PROMPT_CANARY }),
        questions: [(
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
        )]
        .into_iter()
        .collect(),
        images: None,
    }
}

fn classifier_ok_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "answers": {
            "category": {
                "type": "choice",
                "choice": "success",
                "probabilities": { "success": 0.9, "failure": 0.1 },
                "confidence": 0.8
            }
        }
    }))
}

fn marker_callbacks() -> RequestCallbacks {
    RequestCallbacks::new()
        .with_on_payload(std::sync::Arc::new(|_body| {
            tracing::span!(Level::INFO, "classifier_payload_marker").in_scope(|| ());
            Ok(None)
        }))
        .with_on_response(std::sync::Arc::new(|_status, _headers| {
            tracing::span!(Level::INFO, "classifier_response_marker").in_scope(|| ());
            Ok(())
        }))
}

#[tokio::test]
async fn classifier_calls_freeze_context_off_wire_and_keep_business_identical() {
    let (a_recording, a_dispatch) = recorder();
    let (b_recording, b_dispatch) = recorder();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(classifier_ok_response())
        .mount(&server)
        .await;
    let record = classifier_record(server.uri());
    let (models, handle) = classifier_models(&record);

    let parent =
        dispatcher::with_default(&a_dispatch, || tracing::span!(Level::INFO, "caller_parent"));
    let parent_id = parent.id().unwrap().into_u64();
    let context = dispatcher::with_default(&b_dispatch, || TelemetryContext::from_span(parent));

    let call = ClassifierCall {
        telemetry_context: Some(context.clone()),
        callbacks: marker_callbacks(),
        ..Default::default()
    };
    let result = await_under_ambient(
        &b_dispatch,
        models.classify(record.clone(), classify_state(), call),
    )
    .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    assert!(result.error.is_none());
    assert_eq!(
        result.answers.get("category"),
        Some(&ClassifierAnswer::Choice {
            choice: "success".into(),
            probabilities: [("success".to_owned(), 0.9), ("failure".to_owned(), 0.1)]
                .into_iter()
                .collect(),
            confidence: 0.8,
        })
    );

    // Native entry with typed options freezes the same carrier.
    let call = ClassifierCall {
        telemetry_context: Some(context),
        callbacks: marker_callbacks(),
        ..Default::default()
    };
    let native = models
        .classify_native(
            record.clone(),
            classify_state(),
            call,
            handle.options(Default::default()),
        )
        .await;
    assert_eq!(native.stop_reason, ClassifierStopReason::Stop);
    assert_eq!(native.answers, result.answers);

    let spans = probe_spans(&a_recording);
    assert_all_parented(&spans, parent_id);
    assert!(b_recording.lock().unwrap().spans.is_empty());

    // Invariance twin without any context: same answers, no new observation.
    let spans_before_twin = probe_spans(&a_recording).len();
    let twin = models
        .classify(record, classify_state(), ClassifierCall::default())
        .await;
    assert_eq!(twin.stop_reason, ClassifierStopReason::Stop);
    assert_eq!(twin.answers, result.answers);
    assert_eq!(probe_spans(&a_recording).len(), spans_before_twin);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 3);
    for sent in &requests {
        let body: serde_json::Value = serde_json::from_slice(&sent.body).expect("json body");
        assert_no_telemetry_keys(&body);
    }
    assert_no_canaries(&a_recording);
}
