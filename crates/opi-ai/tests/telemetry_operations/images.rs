//! Frozen telemetry through OpenRouter image calls and their HTTP callbacks.

use opi_ai::api::openrouter_images::OpenRouterImagesApi;
use opi_ai::auth::{AuthProvenanceSource, AuthScheme, StaticAuthResolver};
use opi_ai::callbacks::RequestCallbacks;
use opi_ai::http::HttpClient;
use opi_ai::images::{
    ImageModelRecord, ImageOutputModality, ImagesCall, ImagesContext, ImagesInput, ImagesOutput,
    ImagesStopReason,
};
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

fn image_record(base_url: String) -> ImageModelRecord {
    ImageModelRecord {
        cost_tiers: Vec::new(),
        headers: Vec::new(),
        input_limits: None,
        input: vec![
            opi_ai::catalog::CatalogInputModality::Text,
            opi_ai::catalog::CatalogInputModality::Image,
        ],
        provider_id: "openrouter".into(),
        id: "google/gemini-2.5-flash-image".into(),
        name: "Gemini 2.5 Flash Image (via OpenRouter)".into(),
        api: ApiId::new("openrouter-images"),
        base_url,
        output: vec![ImageOutputModality::Image],
        cost: None,
    }
}

fn image_models(
    record: &ImageModelRecord,
) -> (
    Models,
    opi_ai::models::ImageApiHandle<opi_ai::api::openrouter_images::OpenRouterImagesOptions>,
) {
    let mut models = Models::new();
    let handle = models
        .register_image_api(
            std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
            std::sync::Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                secrecy::SecretString::from(SECRET_CANARY),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register image api");
    (models, handle)
}

fn image_context() -> ImagesContext {
    ImagesContext {
        input: vec![ImagesInput::Text {
            text: PROMPT_CANARY.into(),
        }],
    }
}

fn image_ok_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "id": "gen-123",
        "choices": [
            {
                "message": {
                    "role": "assistant",
                    "content": "Done",
                    "images": [
                        { "image_url": "data:image/png;base64,QUJD" }
                    ]
                },
                "finish_reason": "stop"
            }
        ]
    }))
}

#[tokio::test]
async fn image_generation_freezes_context_off_wire_and_keeps_business_identical() {
    let (a_recording, a_dispatch) = recorder();
    let (b_recording, b_dispatch) = recorder();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(image_ok_response())
        .mount(&server)
        .await;
    let record = image_record(server.uri());
    let (models, handle) = image_models(&record);

    let parent =
        dispatcher::with_default(&a_dispatch, || tracing::span!(Level::INFO, "caller_parent"));
    let parent_id = parent.id().unwrap().into_u64();
    let context = dispatcher::with_default(&b_dispatch, || TelemetryContext::from_span(parent));

    let markers = RequestCallbacks::new()
        .with_on_payload(std::sync::Arc::new(|_body| {
            tracing::span!(Level::INFO, "image_payload_marker").in_scope(|| ());
            Ok(None)
        }))
        .with_on_response(std::sync::Arc::new(|_status, _headers| {
            tracing::span!(Level::INFO, "image_response_marker").in_scope(|| ());
            Ok(())
        }));
    let call = ImagesCall {
        telemetry_context: Some(context.clone()),
        callbacks: markers,
        ..Default::default()
    };
    let result = await_under_ambient(
        &b_dispatch,
        models.generate_images(record.clone(), image_context(), call),
    )
    .await;
    assert_eq!(result.stop_reason, ImagesStopReason::Stop);
    assert_eq!(
        result.output,
        vec![
            ImagesOutput::Text {
                text: "Done".into()
            },
            ImagesOutput::Image {
                mime_type: "image/png".into(),
                data: "QUJD".into()
            }
        ]
    );

    // The native entry freezes the same carrier through its typed options.
    let call = ImagesCall {
        telemetry_context: Some(context),
        callbacks: RequestCallbacks::new()
            .with_on_payload(std::sync::Arc::new(|_body| {
                tracing::span!(Level::INFO, "image_payload_marker").in_scope(|| ());
                Ok(None)
            }))
            .with_on_response(std::sync::Arc::new(|_status, _headers| {
                tracing::span!(Level::INFO, "image_response_marker").in_scope(|| ());
                Ok(())
            })),
        ..Default::default()
    };
    let native = models
        .generate_images_native(
            record.clone(),
            image_context(),
            call,
            handle.options(Default::default()),
        )
        .await;
    assert_eq!(native.stop_reason, ImagesStopReason::Stop);
    assert_eq!(native.output, result.output);

    let spans = probe_spans(&a_recording);
    assert!(
        spans
            .iter()
            .any(|(name, _)| *name == "image_payload_marker")
            && spans
                .iter()
                .any(|(name, _)| *name == "image_response_marker")
    );
    assert_all_parented(&spans, parent_id);
    assert!(b_recording.lock().unwrap().spans.is_empty());

    // Invariance twin without any context: same output, no new observation.
    let spans_before_twin = probe_spans(&a_recording).len();
    let twin = models
        .generate_images(record, image_context(), ImagesCall::default())
        .await;
    assert_eq!(twin.stop_reason, ImagesStopReason::Stop);
    assert_eq!(twin.output, result.output);
    assert_eq!(probe_spans(&a_recording).len(), spans_before_twin);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 3);
    for sent in &requests {
        let body: serde_json::Value = serde_json::from_slice(&sent.body).expect("json body");
        assert_no_telemetry_keys(&body);
        assert!(
            !sent
                .headers
                .keys()
                .any(|name| name.as_str().to_ascii_lowercase().contains("telemetry")),
            "no telemetry header may cross the image wire"
        );
    }
    assert_no_canaries(&a_recording);
}
