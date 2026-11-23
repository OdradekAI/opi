//! OpenRouter Images wire and projection assertions.

use super::*;

#[tokio::test]
async fn images_first_choice_orders_text_then_data_url_images() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "gen-123",
            "choices": [
                {
                    "message": {
                        "role": "assistant",
                        "content": "Done",
                        "images": [
                            { "image_url": "data:image/png;base64,QUJD" },
                            { "image_url": "https://example.com/not-a-data-url.png" },
                            { "image_url": { "url": "data:image/jpeg;base64,R0lG" } },
                            { "image_url": "data:broken-no-base64-marker" },
                            { "image_url": { } }
                        ]
                    },
                    "finish_reason": "stop"
                },
                {
                    "message": {
                        "content": "second choice",
                        "images": [{ "image_url": "data:image/png;base64,SRVDT05E" }]
                    }
                }
            ]
        })))
        .mount(&server)
        .await;

    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let models = image_models(&record);
    let context = ImagesContext {
        input: vec![ImagesInput::Text {
            text: "draw a cat".into(),
        }],
    };
    let result = models.generate_images(record, context, images_call()).await;

    assert_eq!(result.stop_reason, ImagesStopReason::Stop);
    assert_eq!(result.response_id.as_deref(), Some("gen-123"));
    assert_eq!(
        result.output,
        vec![
            ImagesOutput::Text {
                text: "Done".into()
            },
            ImagesOutput::Image {
                mime_type: "image/png".into(),
                data: "QUJD".into()
            },
            ImagesOutput::Image {
                mime_type: "image/jpeg".into(),
                data: "R0lG".into()
            },
        ]
    );
    assert!(result.error.is_none());
    assert!(result.error_message.is_none());

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 1);
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).expect("json request body");
    assert_eq!(body["model"], "google/gemini-2.5-flash-image");
    assert_eq!(body["stream"], false);
    assert_eq!(body["modalities"], serde_json::json!(["image"]));
    assert_eq!(
        body["messages"],
        serde_json::json!([
            { "role": "user", "content": [text_part_json("draw a cat")] }
        ])
    );
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
async fn images_modalities_empty_metadata_still_requests_image() {
    assert_images_modalities(vec![], &["image"]).await;
}

#[tokio::test]
async fn images_modalities_text_only_metadata_requests_image_then_text() {
    assert_images_modalities(vec![ImageOutputModality::Text], &["image", "text"]).await;
}

#[tokio::test]
async fn images_modalities_image_only_metadata_requests_image() {
    assert_images_modalities(vec![ImageOutputModality::Image], &["image"]).await;
}

#[tokio::test]
async fn images_modalities_reorder_text_before_image_metadata() {
    assert_images_modalities(
        vec![ImageOutputModality::Text, ImageOutputModality::Image],
        &["image", "text"],
    )
    .await;
}

#[tokio::test]
async fn images_modalities_deduplicate_output_metadata() {
    assert_images_modalities(
        vec![
            ImageOutputModality::Image,
            ImageOutputModality::Text,
            ImageOutputModality::Image,
            ImageOutputModality::Text,
        ],
        &["image", "text"],
    )
    .await;
}

async fn assert_images_modalities(output: Vec<ImageOutputModality>, expected: &[&str]) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "gen-modalities",
            "choices": [{ "message": { "content": "text only", "images": [] } }]
        })))
        .expect(1)
        .mount(&server)
        .await;
    let record = image_record(server.uri(), output);
    let models = image_models(&record);
    let result = models
        .generate_images(record, ImagesContext::default(), images_call())
        .await;

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(result.stop_reason, ImagesStopReason::Stop);
    assert_eq!(result.response_id.as_deref(), Some("gen-modalities"));
    assert_eq!(
        result.output,
        vec![ImagesOutput::Text {
            text: "text only".into()
        }]
    );
    assert!(result.error.is_none());
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
    assert_eq!(body["modalities"], serde_json::json!(expected));
}

#[tokio::test]
async fn images_text_only_and_empty_results_stay_success() {
    let text_server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "gen-text-only",
            "choices": [
                { "message": { "role": "assistant", "content": "text only", "images": [] } }
            ]
        })))
        .expect(1)
        .mount(&text_server)
        .await;
    let text_record = image_record(text_server.uri(), vec![ImageOutputModality::Image]);
    let text_models = image_models(&text_record);
    let text_only = text_models
        .generate_images(
            text_record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            images_call(),
        )
        .await;
    assert_eq!(text_only.stop_reason, ImagesStopReason::Stop);
    assert_eq!(
        text_only.output,
        vec![ImagesOutput::Text {
            text: "text only".into()
        }]
    );

    let empty_server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "gen-empty",
            "choices": [
                { "message": { "role": "assistant", "content": "", "images": [] } }
            ]
        })))
        .expect(1)
        .mount(&empty_server)
        .await;
    let empty_record = image_record(empty_server.uri(), vec![ImageOutputModality::Image]);
    let empty_models = image_models(&empty_record);
    let empty = empty_models
        .generate_images(
            empty_record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            images_call(),
        )
        .await;
    assert_eq!(empty.stop_reason, ImagesStopReason::Stop);
    assert!(empty.output.is_empty());
}

#[tokio::test]
async fn images_zero_choices_is_success() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "gen-none",
            "choices": []
        })))
        .mount(&server)
        .await;

    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let models = image_models(&record);
    let result = models
        .generate_images(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            images_call(),
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Stop);
    assert!(result.output.is_empty());
    assert_eq!(result.response_id.as_deref(), Some("gen-none"));
}

#[tokio::test]
async fn images_usage_splits_cache_buckets_and_costs_use_base_rates() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "gen-usage",
            "choices": [
                { "message": { "role": "assistant", "content": "ok" } }
            ],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 40,
                "prompt_tokens_details": {
                    "cached_tokens": 30,
                    "cache_write_tokens": 10
                }
            }
        })))
        .mount(&server)
        .await;

    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let models = image_models(&record);
    let result = models
        .generate_images(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            images_call(),
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Stop);
    let usage = result.usage.expect("usage reported");
    assert!(usage.reported);
    // cacheRead = reportedCached - cacheWrite when cacheWrite > 0;
    // input excludes both cache buckets (pi parseUsage).
    assert_eq!(usage.input_tokens, 70);
    assert_eq!(usage.cache_read_tokens, 20);
    assert_eq!(usage.cache_write_tokens, 10);
    assert_eq!(usage.output_tokens, 40);
    assert_eq!(usage.total_tokens, Some(140));

    // Base-rate cost exception: the adapter prices with the record's base
    // per-million rates only (never tiers).
    let cost = result.cost.expect("cost computed");
    let eps = 1e-12;
    assert!((cost.input - 70.0 * 3.0 / 1_000_000.0).abs() < eps);
    assert!((cost.output - 40.0 * 15.0 / 1_000_000.0).abs() < eps);
    assert!((cost.cache_read - 20.0 * 0.3 / 1_000_000.0).abs() < eps);
    assert!((cost.cache_write - 10.0 * 3.75 / 1_000_000.0).abs() < eps);
    assert!(
        (cost.total - (cost.input + cost.output + cost.cache_read + cost.cache_write)).abs() < eps
    );
}

#[tokio::test]
async fn images_http_failure_diagnostics_do_not_echo_provider_body() {
    let canary = "synthetic-image-private-canary";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": { "message": canary }
        })))
        .expect(1)
        .mount(&server)
        .await;
    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let models = image_models(&record);
    let result = models
        .generate_images(
            record.clone(),
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            images_call(),
        )
        .await;

    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    assert_eq!(result.stop_reason, ImagesStopReason::Error);
    assert_eq!(result.api, record.api.as_str());
    assert_eq!(result.provider, record.provider_id);
    assert_eq!(result.model, record.id);
    assert!(matches!(
        result.error,
        Some(ProviderError::ServiceFault { status: 400, .. })
    ));
    let error = result.error.as_ref().expect("typed HTTP failure");
    for diagnostic in [
        format!("{result:?}"),
        result.error_message.clone().expect("display diagnostic"),
        error.to_string(),
        format!("{error:?}"),
    ] {
        assert!(!diagnostic.contains(canary), "diagnostic: {diagnostic}");
    }
    assert_eq!(
        result.error_message.as_deref(),
        Some("HTTP 400: [REDACTED]")
    );
    assert!(result.usage.is_none());
}

#[tokio::test]
async fn images_base_url_trailing_slash_and_extra_headers() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("x-custom-image", "v1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "gen-hdr",
            "choices": []
        })))
        .mount(&server)
        .await;

    let record = image_record(
        format!("{}/", server.uri()),
        vec![ImageOutputModality::Image],
    );
    let models = image_models(&record);
    let result = models
        .generate_images(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text { text: "hi".into() }],
            },
            ImagesCall {
                extra_headers: vec![("x-custom-image".into(), "v1".into())],
                ..images_call()
            },
        )
        .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Stop);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn images_protected_headers_never_reach_direct_transport() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"choices": []})))
        .mount(&server)
        .await;
    let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
    let adapter = OpenRouterImagesApi::new(HttpClient::new(), record.clone());
    for header in ["Authorization", "aPi-KeY", "Host"] {
        let result = adapter
            .generate_images_prepared(
                record.clone(),
                ImagesContext::default(),
                adapter.default_options(),
                ResolvedAuth {
                    env: Default::default(),
                    scheme: AuthScheme::Bearer,
                    secret: secrecy::SecretString::from("fixture-key"),
                    base_url: None,
                    account_id: None,
                    provenance: Default::default(),
                },
                ImagesCall {
                    extra_headers: vec![(header.into(), "untrusted-override".into())],
                    ..images_call()
                },
            )
            .await;
        assert!(
            matches!(result.error, Some(ProviderError::RequestFailed(_))),
            "{:?}",
            result.error
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn images_empty_and_partial_usage_do_not_become_reliable_zero() {
    for raw in [
        serde_json::json!({}),
        serde_json::json!({"prompt_tokens": "many", "completion_tokens": 7}),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"choices": [], "usage": raw})),
            )
            .mount(&server)
            .await;
        let record = image_record(server.uri(), vec![ImageOutputModality::Image]);
        let models = image_models(&record);
        let result = models
            .generate_images(record, ImagesContext::default(), images_call())
            .await;
        assert_eq!(result.stop_reason, ImagesStopReason::Stop);
        if raw.get("completion_tokens").is_some() {
            let usage = result.usage.expect("valid output count survives");
            assert_eq!(usage.output_tokens, 7);
            assert!(!usage.reported);
            assert_eq!(usage.total_tokens, None);
        } else {
            assert!(result.usage.is_none());
        }
        assert!(result.cost.is_none());
    }
}

#[tokio::test]
async fn images_unknown_prices_stay_distinct_from_known_free_usage() {
    assert!(default_openrouter_image_record().cost.is_none());
    for cost in [None, Some(ImageCostRates::default())] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [], "usage": {"prompt_tokens": 12, "completion_tokens": 0}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let mut record = image_record(server.uri(), vec![ImageOutputModality::Image]);
        record.cost = cost;
        let models = image_models(&record);
        let result = models
            .generate_images(record, ImagesContext::default(), images_call())
            .await;
        let usage = result.usage.expect("valid usage survives unknown prices");
        assert!(usage.reported);
        assert_eq!(usage.input_tokens, 12);
        assert_eq!(result.cost.map(|cost| cost.total), cost.map(|_| 0.0));
    }
}
