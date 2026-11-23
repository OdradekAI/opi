//! OpenAI Responses Models, callbacks, native options and source-compatibility assertions.

use super::fixtures::*;
use super::*;

#[tokio::test]
async fn responses_payload_callback_sees_projected_input_body() {
    let sse = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_1\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hi\"}]}],\"usage\":{\"input_tokens\":5,\"output_tokens\":2}}}\n\n",
    );
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let seen: std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorder = seen.clone();
    let provider = OpenAiResponsesProvider::new(Some(server.uri()));
    let mut request = responses_text_request();
    request.callbacks = RequestCallbacks::new().with_on_payload(std::sync::Arc::new(move |body| {
        recorder.lock().unwrap().push(body.clone());
        Ok(None)
    }));

    collect_stream(provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()))
        .await;

    let bodies = seen.lock().unwrap().clone();
    assert_eq!(bodies.len(), 1);
    assert!(
        bodies[0]["input"].is_array(),
        "payload callback sees the projected `input` field, not raw messages"
    );
    let sent = server.received_requests().await.unwrap()[0].clone();
    let wire_body: serde_json::Value =
        serde_json::from_slice(&sent.body).expect("captured body is JSON");
    assert_eq!(bodies[0], wire_body);
}

#[tokio::test]
async fn responses_async_response_and_raw_callbacks_follow_source_order() {
    let order: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let raw_recorder = order.clone();
    let response_recorder = order.clone();
    let sse = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_1\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hi\"}]}],\"usage\":{\"input_tokens\":5,\"output_tokens\":2}}}\n\n",
    );
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = OpenAiResponsesProvider::new(Some(server.uri()));
    let mut request = responses_text_request();
    request.callbacks = RequestCallbacks::new()
        .with_async_header_transform(|headers| async move {
            tokio::task::yield_now().await;
            Ok(headers)
        })
        .with_async_on_payload(|_| async {
            tokio::task::yield_now().await;
            Ok(None)
        })
        .with_async_on_response(move |status, _headers| {
            let response_recorder = response_recorder.clone();
            async move {
                tokio::task::yield_now().await;
                response_recorder
                    .lock()
                    .unwrap()
                    .push(format!("response:{status}"));
                Ok(())
            }
        })
        .with_async_on_raw(move |event| {
            let raw_recorder = raw_recorder.clone();
            async move {
                tokio::task::yield_now().await;
                raw_recorder
                    .lock()
                    .unwrap()
                    .push(format!("raw:{}", event["type"].as_str().unwrap_or("?")));
                Ok(())
            }
        });

    collect_stream(provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()))
        .await;

    let order = order.lock().unwrap();
    assert_eq!(
        order.iter().map(String::as_str).collect::<Vec<_>>(),
        vec![
            "response:200",
            "raw:response.created",
            "raw:response.completed"
        ],
        "response first, then parsed wire events in stream order"
    );
}

#[tokio::test]
async fn responses_request_body_store_false_by_default() {
    let provider = responses_provider();
    let body = provider
        .build_request_body(&responses_tool_request())
        .unwrap();
    assert_eq!(
        body["store"], false,
        "source semantics: store:false is sent unconditionally by default"
    );
}

#[test]
fn responses_off_thinking_uses_the_catalog_mapping() {
    let provider = responses_provider();
    for (model, reasoning) in [
        ("gpt-5.4", Some(serde_json::json!({"effort": "none"}))),
        ("o3", None),
    ] {
        let mut request = responses_tool_request();
        request.model = format!("openai-responses:{model}");
        for enabled in [false, true] {
            request.thinking = ThinkingConfig {
                enabled,
                budget_tokens: None,
                level: ThinkingLevel::None,
            };
            let body = provider.build_request_body(&request).unwrap();
            assert_eq!(body.get("reasoning"), reasoning.as_ref(), "{model}");
        }
    }
}

#[test]
fn responses_copilot_route_skips_off_effort() {
    use std::sync::Arc as StdArc;

    use opi_ai::http::HttpClient;

    let mut provider = OpenAiResponsesProvider::with_auth_extra(
        None,
        ResponsesConfig::default(),
        "github-copilot".into(),
        Vec::new(),
        StdArc::new(HttpClient::new()),
    );
    provider
        .replace_model_catalog(vec![o3_responses_model()])
        .expect("replace catalog");
    let mut request = responses_tool_request();
    request.model = "github-copilot:o3".into();
    assert!(
        provider
            .build_request_body(&request)
            .unwrap()
            .get("reasoning")
            .is_none(),
        "Sign-in-with-ChatGPT/copilot routes skip the off effort (source)"
    );
}

#[test]
fn responses_xai_route_always_includes_encrypted_content() {
    use std::sync::Arc as StdArc;

    use opi_ai::http::HttpClient;

    let mut provider = OpenAiResponsesProvider::with_auth_extra(
        None,
        ResponsesConfig::default(),
        "xai".into(),
        Vec::new(),
        StdArc::new(HttpClient::new()),
    );
    provider
        .replace_model_catalog(vec![o3_responses_model()])
        .expect("replace catalog");
    let mut request = responses_tool_request();
    request.model = "xai:o3".into();
    let body = provider.build_request_body(&request).unwrap();
    assert_eq!(
        body["include"],
        serde_json::json!(["reasoning.encrypted_content"]),
        "xai reasoning models always request encrypted reasoning"
    );
}

#[test]
fn responses_off_effort_omits_summary_and_include() {
    let provider = responses_provider();
    let mut request = responses_tool_request();
    request.model = "openai-responses:o3".into();
    let body = provider.build_request_body(&request).unwrap();
    assert!(
        body["reasoning"].get("summary").is_none(),
        "the off branch sends effort only (source)"
    );
    assert!(
        body.get("include").is_none(),
        "encrypted reasoning is requested only when an effort runs"
    );
}

#[tokio::test]
async fn models_native_options_reach_the_responses_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(responses_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let (models, handle) = registered_responses_models(server.uri(), o3_responses_model()).await;
    let mut value = OpenAiResponsesOptions::default();
    value.tool_choice = Some(ResponsesToolChoice::Named("list_dir".into()));
    value.reasoning_effort = Some(ThinkingLevel::High);
    value.reasoning_summary = Some(ResponsesReasoningSummary::Detailed);
    value.service_tier = Some(ResponsesServiceTier::Flex);
    let options = handle.options(value);

    let request = responses_public_request("o3", Some(ThinkingLevel::Minimal));
    let message = models
        .complete(responses_record(), request, options)
        .await
        .expect("native responses call completes");
    assert_eq!(message.stop_reason, opi_ai::stream::StopReason::Stop);

    let sent = &server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
    assert_eq!(
        body["tool_choice"],
        serde_json::json!({"type": "function", "name": "list_dir"})
    );
    assert_eq!(
        body["reasoning"],
        serde_json::json!({"effort": "high", "summary": "detailed"})
    );
    assert_eq!(
        body["include"],
        serde_json::json!(["reasoning.encrypted_content"])
    );
    assert_eq!(body["service_tier"], serde_json::json!("flex"));
    assert_eq!(body["store"], serde_json::json!(false));
    server.verify().await;
}

#[tokio::test]
async fn native_sampling_uses_native_effort_summary_default_and_request_overrides() {
    for (effort, summary, request_temperature, expected, wire_effort) in [
        (Some(ThinkingLevel::High), None, None, 0.9, "high"),
        (
            None,
            Some(ResponsesReasoningSummary::Detailed),
            None,
            0.6,
            "medium",
        ),
        (None, None, None, 0.2, "none"),
        (Some(ThinkingLevel::High), None, Some(0.4), 0.4, "high"),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/responses"))
            .respond_with(ResponseTemplate::new(200).set_body_string(responses_sse_ok()))
            .expect(1)
            .mount(&server)
            .await;
        let model = o3_responses_model()
            .with_sampling_params(
                [("temperature".into(), serde_json::json!(0.1))]
                    .into_iter()
                    .collect(),
            )
            .with_sampling_level_params(
                ThinkingLevel::None,
                [("temperature".into(), serde_json::json!(0.2))]
                    .into_iter()
                    .collect(),
            )
            .with_sampling_level_params(
                ThinkingLevel::Medium,
                [("temperature".into(), serde_json::json!(0.6))]
                    .into_iter()
                    .collect(),
            )
            .with_sampling_level_params(
                ThinkingLevel::High,
                [("temperature".into(), serde_json::json!(0.9))]
                    .into_iter()
                    .collect(),
            );
        let (models, handle) = registered_responses_models(server.uri(), model.clone()).await;
        let mut request = responses_public_request("o3", Some(ThinkingLevel::Low));
        if let Some(value) = request_temperature {
            request
                .sampling_params
                .get_or_insert_default()
                .insert("temperature".into(), serde_json::json!(value));
        }
        let mut options = OpenAiResponsesOptions::default();
        options.reasoning_effort = effort;
        options.reasoning_summary = summary;
        models
            .complete(
                ChatModelRecord::new("openai-responses", model),
                request,
                handle.options(options),
            )
            .await
            .expect("native call completes");
        let sent = server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
        assert_eq!(body["reasoning"]["effort"], wire_effort);
        assert_eq!(body["temperature"], serde_json::json!(expected));
        server.verify().await;
    }
}

#[tokio::test]
async fn models_native_unsupported_effort_fails_typed_before_auth() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(responses_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(0)
        .mount(&server)
        .await;

    let (models, handle) = registered_responses_models(server.uri(), o3_responses_model()).await;
    let mut value = OpenAiResponsesOptions::default();
    value.reasoning_effort = Some(ThinkingLevel::XHigh);
    let options = handle.options(value);
    let failure = models
        .complete(
            responses_record(),
            responses_public_request("o3", None),
            options,
        )
        .await
        .expect_err("unsupported native effort fails typed preflight");
    assert!(
        matches!(failure.cause(), opi_ai::provider::ProviderError::Config(_)),
        "typed preflight failure, got {:?}",
        failure.cause()
    );
    server.verify().await;
}

#[tokio::test]
async fn models_simple_neutral_effort_reaches_responses_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(responses_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let mut models = Models::new();
    let provider = std::sync::Arc::new(OpenAiResponsesProvider::new(Some(server.uri())));
    let api = OpenAiResponsesChatApi::new(provider, o3_responses_model()).expect("record");
    models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::Bearer,
                secrecy::SecretString::from("offline-responses-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register");

    let request = responses_public_request("o3", Some(ThinkingLevel::XHigh));
    let message = models
        .complete_simple(responses_record(), request, Default::default())
        .await
        .expect("simple responses call completes");
    assert_eq!(message.stop_reason, opi_ai::stream::StopReason::Stop);

    let sent = &server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
    assert_eq!(
        body["reasoning"],
        serde_json::json!({"effort": "high", "summary": "auto"}),
        "neutral entry clamps to the nearest supported level and maps summary default auto"
    );
    assert_eq!(
        body["include"],
        serde_json::json!(["reasoning.encrypted_content"])
    );
    server.verify().await;
}

#[tokio::test]
async fn models_direct_adapter_uses_injected_auth_and_returns_result() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(header("authorization", "Bearer injected-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(responses_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let (models, handle) = registered_responses_models(server.uri(), o3_responses_model()).await;
    let options = handle.options(OpenAiResponsesOptions::default());
    let message = models
        .complete_direct(
            responses_record(),
            responses_public_request("o3", None),
            options,
            opi_ai::auth::ResolvedAuth {
                env: Default::default(),
                scheme: opi_ai::auth::AuthScheme::Bearer,
                secret: secrecy::SecretString::from("injected-key"),
                base_url: None,
                account_id: None,
                provenance: Default::default(),
            },
        )
        .await
        .expect("direct responses call completes");
    assert_eq!(message.stop_reason, opi_ai::stream::StopReason::Stop);
    server.verify().await;
}

#[tokio::test]
async fn models_direct_adapter_rejects_wrong_wire_records() {
    let chat_model = ModelInfo::new(
        "o3",
        "o3",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(200_000, 100_000).with_streaming(true),
    );
    let provider = std::sync::Arc::new(OpenAiResponsesProvider::new(None));
    let failure = OpenAiResponsesChatApi::new(provider, chat_model)
        .expect_err("wrong-wire records fail before authentication");
    assert!(
        matches!(failure, opi_ai::provider::ProviderError::Config(_)),
        "typed rejection, got {failure:?}"
    );
}

#[tokio::test]
async fn responses_long_cache_retention_sends_prompt_cache_retention() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(responses_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let (models, handle) = registered_responses_models(server.uri(), o3_responses_model()).await;
    let mut request = responses_public_request("o3", None);
    request.cache_retention = CacheRetention::Long;
    request.session_id = Some("sess-123".into());
    let options = handle.options(OpenAiResponsesOptions::default());
    models
        .complete(responses_record(), request, options)
        .await
        .expect("long retention call completes");

    let sent = &server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
    assert_eq!(body["prompt_cache_key"], serde_json::json!("sess-123"));
    assert_eq!(
        body["prompt_cache_retention"],
        serde_json::json!("24h"),
        "long retention maps to the source 24h retention window"
    );
    server.verify().await;
}

#[tokio::test]
async fn responses_reasoning_summary_deltas_stream_thinking_blocks() {
    let provider = responses_provider();
    let events: Vec<AssistantStreamEvent> = provider
        .stream_from_sse(&reasoning_sse(), CancellationToken::new())
        .filter_map(|event| async move { event.ok() })
        .collect()
        .await;
    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::ThinkingStart { .. }))
    );
    assert!(events
        .iter()
        .any(|event| matches!(event, AssistantStreamEvent::ThinkingDelta { delta, .. } if delta == "Think")));
    let done = events
        .iter()
        .find_map(|event| match event {
            AssistantStreamEvent::Done { message, .. } => Some(message.clone()),
            _ => None,
        })
        .expect("stream completes");
    let thinking = message_thinking(&done).expect("thinking block persisted");
    assert_eq!(thinking, "Thinking");
}

#[tokio::test]
async fn responses_reasoning_signature_persists_encrypted_content() {
    let sse = concat!(
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_e\",\"status\":\"in_progress\",\"model\":\"o3\",\"output\":[]}}\n\n",
        "data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"reasoning\",\"id\":\"rs_9\"}}\n\n",
        "data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"reasoning\",\"id\":\"rs_9\",\"summary\":[{\"type\":\"summary_text\",\"text\":\"Plan\"}],\"encrypted_content\":\"ENC\"}}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_e\",\"status\":\"completed\",\"model\":\"o3\",\"output\":[],\"usage\":{\"input_tokens\":5,\"output_tokens\":2}}}\n\n",
    );
    let provider = responses_provider();
    let events: Vec<AssistantStreamEvent> = provider
        .stream_from_sse(sse, CancellationToken::new())
        .filter_map(|event| async move { event.ok() })
        .collect()
        .await;
    let done = events
        .iter()
        .find_map(|event| match event {
            AssistantStreamEvent::Done { message, .. } => Some(message.clone()),
            _ => None,
        })
        .expect("stream completes");
    let signature = done
        .content
        .iter()
        .find_map(|content| match content {
            opi_ai::message::AssistantContent::Thinking { signature, .. } => signature.clone(),
            _ => None,
        })
        .expect("reasoning item persisted a signature");
    let item: serde_json::Value = serde_json::from_str(&signature).expect("signature is JSON");
    assert_eq!(item["id"], serde_json::json!("rs_9"));
    assert_eq!(item["encrypted_content"], serde_json::json!("ENC"));
}

#[tokio::test]
async fn responses_completed_backfills_encrypted_signature_from_terminal_output() {
    let sse = concat!(
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_b\",\"status\":\"in_progress\",\"model\":\"o3\",\"output\":[]}}\n\n",
        "data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"reasoning\",\"id\":\"rs_b\"}}\n\n",
        "data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"reasoning\",\"id\":\"rs_b\",\"summary\":[{\"type\":\"summary_text\",\"text\":\"Ponder\"}]}}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_b\",\"status\":\"completed\",\"model\":\"o3\",\"output\":[{\"type\":\"reasoning\",\"id\":\"rs_b\",\"summary\":[{\"type\":\"summary_text\",\"text\":\"Ponder\"}],\"encrypted_content\":\"BACKFILL\"}],\"usage\":{\"input_tokens\":5,\"output_tokens\":2}}}\n\n",
    );
    let provider = responses_provider();
    let events: Vec<AssistantStreamEvent> = provider
        .stream_from_sse(sse, CancellationToken::new())
        .filter_map(|event| async move { event.ok() })
        .collect()
        .await;
    let done = events
        .iter()
        .find_map(|event| match event {
            AssistantStreamEvent::Done { message, .. } => Some(message.clone()),
            _ => None,
        })
        .expect("stream completes");
    let signature = done
        .content
        .iter()
        .find_map(|content| match content {
            opi_ai::message::AssistantContent::Thinking { signature, .. } => signature.clone(),
            _ => None,
        })
        .expect("reasoning signature persisted");
    let item: serde_json::Value = serde_json::from_str(&signature).expect("signature is JSON");
    assert_eq!(
        item["encrypted_content"],
        serde_json::json!("BACKFILL"),
        "terminal response output backfills the omitted encrypted content"
    );
}

#[tokio::test]
async fn responses_mid_convo_effort_stamps_configuration_updates() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(responses_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    use opi_ai::http::HttpClient;
    use opi_ai::model_info::{OpenAiResponsesCompat, WireCompat};
    let model = o3_responses_model()
        .with_compat(WireCompat::OpenAiResponses(OpenAiResponsesCompat {
            source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                supports_mid_convo_effort: Some(true),
                ..Default::default()
            },
            ..Default::default()
        }))
        .expect("compat");
    let provider = std::sync::Arc::new(OpenAiResponsesProvider::for_route(
        Some(server.uri()),
        "openai-responses".into(),
        Default::default(),
        vec![model.clone()],
        std::sync::Arc::new(HttpClient::new()),
    ));
    let api = OpenAiResponsesChatApi::new(provider, model.clone()).expect("record");
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::Bearer,
                secrecy::SecretString::from("offline-responses-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register");

    let mut request = responses_public_request("o3", None);
    let mut context = responses_tool_context();
    context.messages.insert(
        0,
        Message::Assistant(history_assistant_message(
            vec![opi_ai::message::AssistantContent::Text {
                text: "Earlier".into(),
                signature: None,
            }],
            Some("low".into()),
        )),
    );
    request.context = opi_ai::transcript::normalize_context(context).expect("valid context");
    let mid_convo_record = ChatModelRecord::new("openai-responses", model.clone());
    let mut value = OpenAiResponsesOptions::default();
    value.reasoning_effort = Some(ThinkingLevel::High);
    let options = handle.options(value);
    let message = models
        .complete(mid_convo_record, request, options)
        .await
        .expect("mid-convo effort call completes");

    let sent = &server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
    let input = body["input"].as_array().unwrap();
    let updates: Vec<&serde_json::Value> = input
        .iter()
        .filter(|item| item.get("type").and_then(|t| t.as_str()) == Some("configuration_update"))
        .collect();
    assert_eq!(
        updates.len(),
        2,
        "one stamp before the earlier same-model reply, one at the end: {input:?}"
    );
    assert_eq!(updates[0]["reasoning"]["effort"], serde_json::json!("low"));
    assert_eq!(updates[1]["reasoning"]["effort"], serde_json::json!("high"));
    assert_eq!(
        body["reasoning"]["effort"],
        serde_json::json!("medium"),
        "the request effort stays fixed while configuration items carry the level"
    );
    assert_eq!(
        message.provider_thinking_level.as_deref(),
        Some("high"),
        "the reply records its provider thinking level for the next stamp"
    );
    server.verify().await;
}

#[tokio::test]
async fn responses_done_message_carries_measured_response_duration() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(responses_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let (models, handle) = registered_responses_models(server.uri(), o3_responses_model()).await;
    let options = handle.options(OpenAiResponsesOptions::default());
    let message = models
        .complete(
            responses_record(),
            responses_public_request("o3", None),
            options,
        )
        .await
        .expect("duration call completes");
    assert!(
        message.duration_ms.is_some(),
        "terminal messages carry the measured response duration"
    );
}

#[tokio::test]
async fn responses_setup_failure_before_activation_has_no_duration() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&server)
        .await;

    let provider = OpenAiResponsesProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        responses_public_request("o3", None),
        opi_ai::test_support::resolved_bearer_auth(),
    );
    let mut saw_done = false;
    while let Some(item) = stream.next().await {
        match item {
            Ok(AssistantStreamEvent::Done { .. }) => saw_done = true,
            Ok(_) => {}
            Err(cause) => {
                assert!(
                    !cause.is_retryable(),
                    "unauthorized is a terminal setup failure: {cause}"
                );
            }
        }
    }
    assert!(!saw_done, "setup failure never reaches a terminal Done");
    server.verify().await;
}

#[tokio::test]
async fn native_known_invalid_input_and_endpoint_fail_before_authentication() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    struct CountingAuth(Arc<AtomicUsize>);
    impl opi_ai::auth::AuthResolver for CountingAuth {
        fn resolve(
            &self,
        ) -> opi_ai::credential::BoxAuthFuture<'_, Result<opi_ai::auth::ResolvedAuth, ProviderError>>
        {
            Box::pin(async move {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(opi_ai::auth::ResolvedAuth {
                    env: Default::default(),
                    scheme: opi_ai::auth::AuthScheme::Bearer,
                    secret: "fixture-key".into(),
                    base_url: None,
                    account_id: None,
                    provenance: Default::default(),
                })
            })
        }
    }
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(responses_sse_ok()),
        )
        .expect(1)
        .mount(&server)
        .await;
    let model = ModelInfo::new(
        "probe",
        "Probe",
        WireApi::OpenAiResponses,
        ModelCapabilities::new(100000, 1000),
    )
    .with_base_url(server.uri());
    let api = OpenAiResponsesChatApi::new(
        Arc::new(OpenAiResponsesProvider::new(Some(server.uri()))),
        model.clone(),
    )
    .unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            Arc::new(api),
            Arc::new(CountingAuth(count.clone())),
            opi_ai::AuthProvenanceSource::Static,
        )
        .unwrap();
    let mut request = responses_public_request("probe", None);
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        tools: vec![],
        messages: vec![Message::User(opi_ai::message::UserMessage {
            content: vec![opi_ai::message::InputContent::Image {
                source: opi_ai::message::ImageSource::Base64 {
                    data: "AA==".into(),
                },
                media_type: opi_ai::message::MediaType::Png,
            }],
            timestamp_ms: 0,
        })],
    })
    .unwrap();
    assert!(
        models
            .complete(
                ChatModelRecord::new("openai-responses", model.clone()),
                request,
                handle.options(Default::default())
            )
            .await
            .is_err()
    );
    assert_eq!(count.load(Ordering::SeqCst), 0);
    for base in ["not-a-valid-url", "file:///tmp/not-an-http-endpoint"] {
        let mut invalid = model.clone();
        invalid.base_url = Some(base.into());
        let failure = models
            .complete(
                ChatModelRecord::new("openai-responses", invalid),
                responses_public_request("probe", None),
                handle.options(Default::default()),
            )
            .await
            .unwrap_err();
        assert!(matches!(failure.cause(), ProviderError::Config(_)));
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }
    assert!(server.received_requests().await.unwrap().is_empty());
    models
        .complete(
            ChatModelRecord::new("openai-responses", model),
            responses_public_request("probe", None),
            handle.options(Default::default()),
        )
        .await
        .unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn multiline_sse_line_endings_survive_http_chunk_splits() {
    async fn run(body: String, split: Option<usize>) -> Vec<AssistantStreamEvent> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let peer = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0; 65536];
            let received = socket.read(&mut request).await.unwrap();
            assert!(
                received > 0,
                "client sent request bytes before the response"
            );
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes()).await.unwrap();
            if let Some(split) = split {
                socket.write_all(&body.as_bytes()[..split]).await.unwrap();
                socket.flush().await.unwrap();
                tokio::time::sleep(std::time::Duration::from_millis(40)).await;
                socket.write_all(&body.as_bytes()[split..]).await.unwrap();
            } else {
                socket.write_all(body.as_bytes()).await.unwrap();
            }
            socket.shutdown().await.unwrap();
        });
        let provider = OpenAiResponsesProvider::new(Some(format!("http://{address}")));
        let results: Vec<_> = provider
            .stream_prepared(
                responses_text_request(),
                opi_ai::test_support::resolved_bearer_auth(),
            )
            .collect()
            .await;
        peer.await.unwrap();
        results
            .into_iter()
            .map(|event| event.expect("legal complete multiline event"))
            .collect()
    }

    let body = [
        serde_json::json!({"type":"response.created","response":{"id":"resp_1","status":"in_progress","model":"gpt-4o","output":[]}}),
        serde_json::json!({"type":"response.output_item.added","output_index":0,"item":{"type":"message","status":"in_progress","role":"assistant","content":[]}}),
        serde_json::json!({"type":"response.content_part.added","output_index":0,"content_index":0,"part":{"type":"output_text","text":""}}),
        serde_json::json!({"type":"response.output_text.delta","output_index":0,"content_index":0,"delta":"hello"}),
        serde_json::json!({"type":"response.output_text.done","output_index":0,"content_index":0,"text":"hello"}),
        serde_json::json!({"type":"response.output_item.done","output_index":0,"item":{"type":"message","status":"completed","role":"assistant","content":[{"type":"output_text","text":"hello"}]}}),
        serde_json::json!({"type":"response.completed","response":{"id":"resp_1","status":"completed","model":"gpt-4o","output":[{"type":"message","content":[{"type":"output_text","text":"hello"}]}],"usage":{"input_tokens":10,"output_tokens":5}}}),
    ].iter().map(|value| format!("data: {value}\n\n")).collect::<String>();
    for newline in ["\n", "\r\n", "\r"] {
        let multiline = body
            .lines()
            .map(|line| {
                if line.starts_with("data: {") {
                    line.replacen(',', &format!(",{newline}data: "), 1)
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join(newline)
            + newline;
        let split = multiline.find(&format!("{newline}data: ")).unwrap() + 1;
        let whole = run(multiline.clone(), None).await;
        let split = run(multiline, Some(split)).await;
        let kinds = |events: &[AssistantStreamEvent]| {
            events
                .iter()
                .map(|event| serde_json::to_value(event).unwrap()["type"].clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(kinds(&split), kinds(&whole), "newline={newline:?}");
        let final_message = |events: Vec<AssistantStreamEvent>| {
            events
                .into_iter()
                .find_map(|event| match event {
                    AssistantStreamEvent::Done { message, .. } => Some(message),
                    _ => None,
                })
                .expect("terminal outcome")
        };
        let whole = final_message(whole);
        let split = final_message(split);
        assert_eq!(split.content, whole.content);
        assert_eq!(split.stop_reason, whole.stop_reason);
        assert_eq!(split.usage, whole.usage);
        assert!(!split.content.is_empty());
    }
}

mod source_compat_controls {
    use super::*;
    use opi_ai::model_info::{
        OpenAiResponsesCompat, OpenAiResponsesSourceCompat, SessionAffinityFormat, WireCompat,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    fn source_model(base: Option<String>, source: OpenAiResponsesSourceCompat) -> ModelInfo {
        let mut model = o3_responses_model()
            .with_compat(WireCompat::OpenAiResponses(OpenAiResponsesCompat {
                source,
                ..Default::default()
            }))
            .unwrap();
        model.base_url = base;
        model
    }

    fn provider(base: String, provider_id: &str, model: ModelInfo) -> OpenAiResponsesProvider {
        OpenAiResponsesProvider::for_route(
            Some(base),
            provider_id.into(),
            Default::default(),
            vec![model],
            Arc::new(opi_ai::http::HttpClient::new()),
        )
        .with_affinity_policy(opi_ai::providers::openai_responses::ResponsesAffinityPolicy::Direct)
    }

    fn request(provider_id: &str, retention: CacheRetention) -> Request {
        let mut request = responses_public_request("o3", Some(ThinkingLevel::High));
        request.model = format!("{provider_id}:o3");
        let mut context = responses_tool_context();
        context
            .messages
            .push(Message::System(opi_ai::transcript::SystemMessage {
                content: "later instruction".into(),
                sections: vec![],
                tools_added: vec![],
                tools_removed: vec![],
                timestamp_ms: 1,
            }));
        request.context = opi_ai::transcript::normalize_context(context).unwrap();
        request.max_tokens = Some(7);
        request.temperature = Some(0.25);
        request.session_id = Some("raw-session-id".into());
        request.cache_retention = retention;
        request
    }

    #[test]
    fn responses_initial_instruction_uses_the_selected_input_role() {
        for flag in [None, Some(false), Some(true)] {
            for reasoning in [false, true] {
                let source = OpenAiResponsesSourceCompat {
                    supports_developer_role: flag,
                    supports_mid_convo_system_messages: Some(true),
                    ..Default::default()
                };
                let mut model = source_model(None, source);
                model.capabilities.supports_thinking = reasoning;
                let provider = provider("http://127.0.0.1:1".into(), "openai-responses", model);
                let request = request("openai-responses", CacheRetention::Short);
                let canonical = request.context.clone();
                let body = provider.build_request_body(&request).unwrap();
                assert_eq!(
                    body["input"][0]["role"],
                    if reasoning && flag != Some(false) {
                        "developer"
                    } else {
                        "system"
                    }
                );
                assert_eq!(body["input"][0]["content"], "You are helpful.");
                assert!(
                    body.get("instructions").is_none(),
                    "ordinary Responses carries the initial instruction once in input"
                );
                assert_eq!(request.context, canonical);
            }
        }
    }

    #[tokio::test]
    async fn responses_source_defaults_false_and_true_reach_actual_http() {
        for flag in [None, Some(false), Some(true)] {
            for retention in [
                CacheRetention::Disabled,
                CacheRetention::Short,
                CacheRetention::Long,
            ] {
                let server = MockServer::start().await;
                Mock::given(method("POST"))
                    .respond_with(
                        ResponseTemplate::new(200)
                            .insert_header("content-type", "text/event-stream")
                            .set_body_string(responses_sse_ok()),
                    )
                    .expect(1)
                    .mount(&server)
                    .await;
                let source = OpenAiResponsesSourceCompat {
                    supports_developer_role: flag,
                    supports_max_output_tokens: flag,
                    supports_long_cache_retention: flag,
                    supports_explicit_prompt_cache_mode: flag,
                    supports_strict_mode: flag,
                    supports_mid_convo_system_messages: Some(true),
                    ..Default::default()
                };
                let mut model = source_model(None, source);
                let WireCompat::OpenAiResponses(compat) = &mut model.compat else {
                    unreachable!()
                };
                compat.send_session_id_header = true;
                let provider = provider(server.uri(), "openai-responses", model);
                let request = request("openai-responses", retention);
                let canonical = request.context.clone();
                let message = opi_ai::stream::ChatStream::new(provider.stream_prepared(
                    request.clone(),
                    opi_ai::test_support::resolved_bearer_auth(),
                ))
                .result()
                .await
                .unwrap();
                assert!(message.projection.is_some());
                assert_eq!(request.context, canonical);
                let sent = server.received_requests().await.unwrap();
                assert_eq!(sent.len(), 1);
                let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
                let instruction = body["input"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|item| item["content"] == "later instruction")
                    .unwrap();
                assert_eq!(
                    instruction["role"],
                    if flag == Some(false) {
                        "system"
                    } else {
                        "developer"
                    }
                );
                assert_eq!(
                    body.get("max_output_tokens").cloned(),
                    (flag != Some(false)).then(|| serde_json::json!(16))
                );
                let strict = body["tools"][0].get("strict");
                assert_eq!(
                    strict,
                    (flag == Some(true)).then_some(&serde_json::Value::Bool(false))
                );
                let expected_options = if flag == Some(true) {
                    match retention {
                        CacheRetention::Disabled => Some(serde_json::json!({"mode":"explicit"})),
                        CacheRetention::Long => Some(serde_json::json!({"ttl":"30m"})),
                        _ => None,
                    }
                } else {
                    None
                };
                assert_eq!(body.get("prompt_cache_options"), expected_options.as_ref());
                let legacy = retention == CacheRetention::Long && flag.is_none();
                assert_eq!(
                    body.get("prompt_cache_retention")
                        .and_then(serde_json::Value::as_str),
                    legacy.then_some("24h")
                );
                assert_eq!(
                    body.get("prompt_cache_key").is_some(),
                    retention != CacheRetention::Disabled
                );
                for name in ["session_id", "x-client-request-id"] {
                    assert_eq!(
                        sent[0]
                            .headers
                            .get(name)
                            .map(|value| value.to_str().unwrap()),
                        (retention != CacheRetention::Disabled).then_some("raw-session-id")
                    );
                }
                server.verify().await;
            }
        }
    }

    #[tokio::test]
    async fn responses_strict_capability_preserves_unconstrained_initial_and_inline_schemas() {
        use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
        for preference in [
            None,
            Some(StrictPreference::Prefer),
            Some(StrictPreference::Require),
        ] {
            for native in [false, true] {
                let server = MockServer::start().await;
                Mock::given(method("POST"))
                    .respond_with(
                        ResponseTemplate::new(200)
                            .insert_header("content-type", "text/event-stream")
                            .set_body_string(responses_sse_ok()),
                    )
                    .expect(1)
                    .mount(&server)
                    .await;
                let schema =
                    serde_json::json!({"type":"object","properties":{"path":{"type":"string"}}});
                let tool = |name: &str| ToolDef {
                    name: name.into(),
                    description: name.into(),
                    input_schema: schema.clone(),
                    constrained_sampling: preference
                        .map(|strict| ConstrainedSampling::JsonSchema { strict }),
                };
                let source = OpenAiResponsesSourceCompat {
                    supports_strict_mode: Some(true),
                    supports_mid_convo_system_messages: Some(true),
                    supports_additional_tools: Some(true),
                    ..Default::default()
                };
                let model = source_model(None, source);
                let mut request = request("openai-responses", CacheRetention::Disabled);
                request.context =
                    opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                        system_prompt: Some("leading".into()),
                        tools: vec![tool("initial")],
                        messages: vec![
                            Message::User(UserMessage {
                                content: vec![InputContent::Text {
                                    text: "before".into(),
                                }],
                                timestamp_ms: 0,
                            }),
                            Message::System(opi_ai::transcript::SystemMessage {
                                tools_added: vec![tool("inline")],
                                ..Default::default()
                            }),
                            Message::User(UserMessage {
                                content: vec![InputContent::Text {
                                    text: "after".into(),
                                }],
                                timestamp_ms: 1,
                            }),
                        ],
                    })
                    .unwrap();
                let canonical = request.context.clone();
                let message = if native {
                    let (models, handle) =
                        registered_responses_models(server.uri(), model.clone()).await;
                    models
                        .complete(
                            ChatModelRecord::new("openai-responses", model),
                            request.clone(),
                            handle.options(Default::default()),
                        )
                        .await
                        .unwrap()
                } else {
                    let provider = provider(server.uri(), "openai-responses", model);
                    opi_ai::stream::ChatStream::new(provider.stream_prepared(
                        request.clone(),
                        opi_ai::test_support::resolved_bearer_auth(),
                    ))
                    .result()
                    .await
                    .unwrap()
                };
                assert!(message.projection.is_some());
                assert_eq!(request.context, canonical);
                let sent = server.received_requests().await.unwrap();
                assert_eq!(sent.len(), 1);
                let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
                let inline = body["input"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|item| item["type"] == "additional_tools")
                    .unwrap();
                for declaration in [&body["tools"][0], &inline["tools"][0]] {
                    assert_eq!(
                        declaration["strict"],
                        preference.is_some(),
                        "native={native}, preference={preference:?}"
                    );
                    if preference.is_none() {
                        assert_eq!(
                            declaration["parameters"], schema,
                            "capability alone never rewrites trusted optional properties"
                        );
                    } else {
                        assert_eq!(declaration["parameters"]["additionalProperties"], false);
                        assert_eq!(
                            declaration["parameters"]["required"],
                            serde_json::json!(["path"])
                        );
                    }
                }
                server.verify().await;
            }
        }
    }

    #[tokio::test]
    async fn responses_cache_controls_are_independent_of_max_role_and_each_other() {
        for (explicit, long, retention, expected_options, expected_legacy) in [
            (
                Some(true),
                Some(false),
                CacheRetention::Disabled,
                Some(serde_json::json!({"mode":"explicit"})),
                false,
            ),
            (Some(true), Some(false), CacheRetention::Long, None, false),
            (
                Some(true),
                None,
                CacheRetention::Long,
                Some(serde_json::json!({"ttl":"30m"})),
                false,
            ),
            (Some(true), Some(true), CacheRetention::Short, None, false),
            (Some(false), Some(true), CacheRetention::Long, None, true),
            (None, Some(false), CacheRetention::Long, None, false),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(responses_sse_ok()),
                )
                .expect(1)
                .mount(&server)
                .await;
            let source = OpenAiResponsesSourceCompat {
                supports_explicit_prompt_cache_mode: explicit,
                supports_long_cache_retention: long,
                supports_mid_convo_system_messages: Some(true),
                ..Default::default()
            };
            let provider = provider(server.uri(), "openai-responses", source_model(None, source));
            let mut request = request("openai-responses", retention);
            request.session_id = None;
            opi_ai::stream::ChatStream::new(
                provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()),
            )
            .result()
            .await
            .unwrap();
            let sent = server.received_requests().await.unwrap();
            assert_eq!(sent.len(), 1);
            let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
            assert_eq!(body.get("prompt_cache_options"), expected_options.as_ref());
            assert_eq!(
                body.get("prompt_cache_retention")
                    .and_then(serde_json::Value::as_str),
                expected_legacy.then_some("24h")
            );
            assert!(
                body.get("prompt_cache_key").is_none(),
                "cache mode and retention do not require a session/cache key"
            );
            assert_eq!(body["max_output_tokens"], 16);
            assert!(body["tools"][0].get("strict").is_none());
            assert!(
                body["input"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| item["role"] == "developer"
                        && item["content"] == "later instruction")
            );
            server.verify().await;
        }
    }

    #[tokio::test]
    async fn native_and_advanced_signin_use_prepared_auth_once_and_keep_known_failure_facts() {
        struct CountingAuth(Arc<AtomicUsize>);
        impl opi_ai::auth::AuthResolver for CountingAuth {
            fn resolve(
                &self,
            ) -> opi_ai::credential::BoxAuthFuture<
                '_,
                Result<opi_ai::auth::ResolvedAuth, ProviderError>,
            > {
                self.0.fetch_add(1, Ordering::SeqCst);
                Box::pin(async {
                    let mut auth = opi_ai::test_support::resolved_bearer_auth();
                    auth.secret = secrecy::SecretString::from("offline-signin-key");
                    Ok(auth)
                })
            }
        }
        for native in [false, true] {
            let fallback = MockServer::start().await;
            let source = OpenAiResponsesSourceCompat {
                supports_mid_convo_system_messages: Some(true),
                supports_mid_convo_effort: Some(true),
                supports_explicit_prompt_cache_mode: Some(true),
                ..Default::default()
            };
            let model = source_model(Some("https://api.openai.com/v1".into()), source);
            let provider = provider(fallback.uri(), "openai", model.clone());
            let auth_calls = Arc::new(AtomicUsize::new(0));
            let auth = Arc::new(CountingAuth(auth_calls.clone()));
            let bodies = Arc::new(Mutex::new(Vec::new()));
            let recorder = bodies.clone();
            let mut request = request("openai", CacheRetention::Long);
            request.callbacks =
                opi_ai::callbacks::RequestCallbacks::new().with_on_payload(Arc::new(move |body| {
                    recorder.lock().unwrap().push(body.clone());
                    Err(opi_ai::callbacks::CallbackError::new(
                        "intentional prepared capture sentinel",
                    ))
                }));
            let mut models = Models::new();
            let failure = if native {
                let api = OpenAiResponsesChatApi::new(Arc::new(provider), model.clone()).unwrap();
                let handle = models
                    .register_chat_api(Arc::new(api), auth, opi_ai::AuthProvenanceSource::Static)
                    .unwrap();
                let mut options = OpenAiResponsesOptions::default();
                options.reasoning_effort = Some(ThinkingLevel::High);
                models
                    .complete(
                        ChatModelRecord::new("openai", model),
                        request,
                        handle.options(options),
                    )
                    .await
                    .unwrap_err()
            } else {
                models
                    .register_route(
                        Box::new(provider),
                        auth,
                        opi_ai::AuthProvenanceSource::Static,
                        Default::default(),
                    )
                    .unwrap();
                let call = models
                    .prepare_call_with_policy(
                        "openai:o3",
                        request,
                        opi_ai::retry::AcquisitionPolicy::with_max_retries(1),
                    )
                    .await
                    .unwrap();
                let failure = opi_ai::stream::ChatStream::new(call.start_attempt().unwrap())
                    .result()
                    .await
                    .unwrap_err();
                assert!(
                    call.retry_admission().is_none(),
                    "callback refusal does not authorize transport retry"
                );
                failure
            };
            assert!(matches!(failure.cause(), ProviderError::CallbackPhase(_)));
            assert!(failure.message().projection.is_some());
            assert_eq!(auth_calls.load(Ordering::SeqCst), 1);
            let bodies = bodies.lock().unwrap().clone();
            assert_eq!(bodies.len(), 1);
            let body = &bodies[0];
            for field in [
                "max_output_tokens",
                "temperature",
                "prompt_cache_options",
                "prompt_cache_retention",
            ] {
                assert!(body.get(field).is_none(), "{field}, native={native}");
            }
            assert!(
                !body["input"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| item["type"] == "configuration_update")
            );
            assert!(fallback.received_requests().await.unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn responses_affinity_formats_follow_retention_and_preserve_full_ids() {
        for retention in [
            CacheRetention::Short,
            CacheRetention::Long,
            CacheRetention::Disabled,
        ] {
            for format in [
                SessionAffinityFormat::OpenAi,
                SessionAffinityFormat::OpenAiNoSession,
                SessionAffinityFormat::OpenRouter,
            ] {
                let server = MockServer::start().await;
                Mock::given(method("POST"))
                    .respond_with(
                        ResponseTemplate::new(200)
                            .insert_header("content-type", "text/event-stream")
                            .set_body_string(responses_sse_ok()),
                    )
                    .expect(1)
                    .mount(&server)
                    .await;
                let source = OpenAiResponsesSourceCompat {
                    session_affinity_format: Some(format),
                    supports_mid_convo_system_messages: Some(true),
                    ..Default::default()
                };
                let mut model = source_model(None, source);
                let WireCompat::OpenAiResponses(compat) = &mut model.compat else {
                    unreachable!()
                };
                compat.send_session_id_header = true;
                let provider = provider(server.uri(), "openai-responses", model);
                let mut request = request("openai-responses", retention);
                let full_id = "full-sdk-session-id-".repeat(8);
                request.session_id = Some(full_id.clone());
                opi_ai::stream::ChatStream::new(
                    provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()),
                )
                .result()
                .await
                .unwrap();
                let sent = server.received_requests().await.unwrap();
                assert_eq!(sent.len(), 1);
                let headers = &sent[0].headers;
                let enabled = retention != CacheRetention::Disabled;
                assert_eq!(
                    headers.contains_key("session_id"),
                    enabled && format == SessionAffinityFormat::OpenAi
                );
                assert_eq!(
                    headers.contains_key("x-client-request-id"),
                    enabled && format != SessionAffinityFormat::OpenRouter
                );
                assert_eq!(
                    headers.contains_key("x-session-id"),
                    enabled && format == SessionAffinityFormat::OpenRouter
                );
                if let Some(value) = headers.get(if format == SessionAffinityFormat::OpenRouter {
                    "x-session-id"
                } else {
                    "session_id"
                }) {
                    assert_eq!(value.to_str().unwrap(), full_id);
                }
                if let Some(value) = headers.get("x-client-request-id") {
                    assert_eq!(value.to_str().unwrap(), full_id);
                }
                let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
                assert_eq!(
                    body.get("prompt_cache_key")
                        .and_then(serde_json::Value::as_str),
                    enabled.then_some(&full_id[..64])
                );
                server.verify().await;
            }
        }
    }

    #[tokio::test]
    async fn responses_affinity_overrides_are_merged_before_transform_and_http() {
        for retention in [CacheRetention::Short, CacheRetention::Disabled] {
            for (mode, expected, before_transform) in [
                ("default", Some("raw-session-id"), Some("raw-session-id")),
                ("caller", Some("caller-value"), Some("caller-value")),
                ("model", Some("model-value"), Some("model-value")),
                ("remove", None, None),
                (
                    "transform",
                    Some("transformed-value"),
                    Some("raw-session-id"),
                ),
                ("route", Some("route-value"), Some("route-value")),
                (
                    "configured",
                    Some("configured-value"),
                    Some("configured-value"),
                ),
            ] {
                let server = MockServer::start().await;
                Mock::given(method("POST"))
                    .respond_with(
                        ResponseTemplate::new(200)
                            .insert_header("content-type", "text/event-stream")
                            .set_body_string(responses_sse_ok()),
                    )
                    .expect(1)
                    .mount(&server)
                    .await;
                let source = OpenAiResponsesSourceCompat {
                    session_affinity_format: Some(SessionAffinityFormat::OpenRouter),
                    supports_mid_convo_system_messages: Some(true),
                    ..Default::default()
                };
                let mut model = source_model(None, source.clone());
                if mode == "model" {
                    model
                        .headers
                        .push(("X-Session-Id".into(), Some("model-value".into())));
                }
                if mode == "remove" {
                    model.headers.push(("x-session-id".into(), None));
                }
                let provider = if mode == "route" {
                    OpenAiResponsesProvider::with_auth_extra(
                        Some(server.uri()),
                        ResponsesConfig {
                            source,
                            ..Default::default()
                        },
                        "openai-responses".into(),
                        vec![("X-Session-Id".into(), "route-value".into())],
                        Arc::new(opi_ai::http::HttpClient::new()),
                    )
                    .with_affinity_policy(
                        opi_ai::providers::openai_responses::ResponsesAffinityPolicy::Direct,
                    )
                } else {
                    let configured = if mode == "configured" {
                        vec![("X-Session-Id".into(), "configured-value".into())]
                    } else {
                        vec![]
                    };
                    OpenAiResponsesProvider::for_route(
                        Some(server.uri()),
                        "openai-responses".into(),
                        opi_ai::api::shared::headers::ProviderHeaders::try_new(configured).unwrap(),
                        vec![model],
                        Arc::new(opi_ai::http::HttpClient::new()),
                    )
                    .with_affinity_policy(
                        opi_ai::providers::openai_responses::ResponsesAffinityPolicy::Direct,
                    )
                };
                let seen = Arc::new(Mutex::new(Vec::new()));
                let recorder = seen.clone();
                let calls = Arc::new(AtomicUsize::new(0));
                let counter = calls.clone();
                let mut request = request("openai-responses", retention);
                let generated = retention != CacheRetention::Disabled;
                let expected = if mode == "default" && !generated {
                    None
                } else {
                    expected
                };
                let before_transform = if matches!(mode, "default" | "transform") && !generated {
                    None
                } else {
                    before_transform
                };
                if mode == "caller" {
                    request
                        .extra_headers
                        .push(("X-Session-Id".into(), "caller-value".into()));
                }
                request.callbacks = opi_ai::callbacks::RequestCallbacks::new()
                    .with_async_header_transform(move |mut headers| {
                        let recorder = recorder.clone();
                        let counter = counter.clone();
                        async move {
                            counter.fetch_add(1, Ordering::SeqCst);
                            *recorder.lock().unwrap() = headers.clone();
                            if mode == "transform" {
                                headers
                                    .retain(|(name, _)| !name.eq_ignore_ascii_case("x-session-id"));
                                headers.push(("x-session-id".into(), "transformed-value".into()));
                            }
                            Ok(headers)
                        }
                    });
                let message = opi_ai::stream::ChatStream::new(
                    provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()),
                )
                .result()
                .await
                .unwrap();
                assert!(message.projection.is_some());
                assert_eq!(calls.load(Ordering::SeqCst), 1);
                let seen = seen.lock().unwrap().clone();
                let observed: Vec<_> = seen
                    .iter()
                    .filter(|(name, _)| name.eq_ignore_ascii_case("x-session-id"))
                    .map(|(_, value)| value.as_str())
                    .collect();
                assert_eq!(
                    observed,
                    before_transform.into_iter().collect::<Vec<_>>(),
                    "mode={mode}: transform sees the merged generated default"
                );
                let sent = server.received_requests().await.unwrap();
                assert_eq!(sent.len(), 1);
                let values: Vec<_> = sent[0]
                    .headers
                    .get_all("x-session-id")
                    .iter()
                    .map(|value| value.to_str().unwrap())
                    .collect();
                assert_eq!(
                    values,
                    expected.into_iter().collect::<Vec<_>>(),
                    "mode={mode}: one effective ordinary header survives"
                );
                server.verify().await;
            }
        }
    }

    #[tokio::test]
    async fn responses_affinity_inference_uses_the_source_url_rule_and_explicit_override() {
        for (provider_id, path, format, router) in [
            ("proxy", "/openrouter.ai", None, true),
            (
                "proxy",
                "/openrouter.ai",
                Some(SessionAffinityFormat::OpenAiNoSession),
                false,
            ),
            ("openrouter", "/ordinary", None, true),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(responses_sse_ok()),
                )
                .expect(1)
                .mount(&server)
                .await;
            let source = OpenAiResponsesSourceCompat {
                session_affinity_format: format,
                supports_mid_convo_system_messages: Some(true),
                ..Default::default()
            };
            let provider = provider(
                server.uri(),
                provider_id,
                source_model(Some(format!("{}{path}", server.uri())), source),
            );
            let request = request(provider_id, CacheRetention::Short);
            opi_ai::stream::ChatStream::new(
                provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()),
            )
            .result()
            .await
            .unwrap();
            let sent = server.received_requests().await.unwrap();
            assert_eq!(sent.len(), 1);
            assert_eq!(sent[0].headers.contains_key("x-session-id"), router);
            assert_eq!(sent[0].headers.contains_key("x-client-request-id"), !router);
            assert!(!sent[0].headers.contains_key("session_id"));
            assert_eq!(
                sent[0].headers[if router {
                    "x-session-id"
                } else {
                    "x-client-request-id"
                }],
                "raw-session-id"
            );
            server.verify().await;
        }
    }

    #[tokio::test]
    async fn exact_prepared_signin_suppresses_only_source_qualified_branches_without_sends() {
        for (provider_id, base, key, signin) in [
            ("openai", "https://api.openai.com/v1", "oauth-access", true),
            ("openai", "https://api.openai.com/v1", "", true),
            (
                "openai",
                "https://api.openai.com/v1",
                "sk-project-key",
                false,
            ),
            (
                "openai-responses",
                "https://api.openai.com/v1",
                "oauth-access",
                false,
            ),
            ("openai", "https://api.openai.com", "oauth-access", false),
            (
                "openai",
                "https://api.openai.com/v1/",
                "oauth-access",
                false,
            ),
            (
                "openai",
                "https://api.openai.com.evil.invalid/v1",
                "oauth-access",
                false,
            ),
        ] {
            let fallback = MockServer::start().await;
            let source = OpenAiResponsesSourceCompat {
                supports_mid_convo_system_messages: Some(true),
                supports_mid_convo_effort: Some(true),
                supports_explicit_prompt_cache_mode: Some(true),
                ..Default::default()
            };
            let provider = provider(
                fallback.uri(),
                provider_id,
                source_model(Some(base.into()), source),
            );
            let mut request = request(provider_id, CacheRetention::Long);
            let absent_auth_body = provider.build_request_body(&request).unwrap();
            assert!(
                absent_auth_body.get("max_output_tokens").is_some(),
                "no prepared key means no signin classification"
            );
            let seen = Arc::new(Mutex::new(Vec::new()));
            let recorder = seen.clone();
            let response_calls = Arc::new(AtomicUsize::new(0));
            let response_counter = response_calls.clone();
            request.callbacks = opi_ai::callbacks::RequestCallbacks::new()
                .with_on_payload(Arc::new(move |body| {
                    recorder.lock().unwrap().push(body.clone());
                    Err(opi_ai::callbacks::CallbackError::new(
                        "intentional post-auth capture sentinel",
                    ))
                }))
                .with_on_response(Arc::new(move |_, _| {
                    response_counter.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }));
            let mut auth = opi_ai::test_support::resolved_bearer_auth();
            auth.secret = secrecy::SecretString::from(key);
            let failure = opi_ai::stream::ChatStream::new(provider.stream_prepared(request, auth))
                .result()
                .await
                .unwrap_err();
            assert!(matches!(failure.cause(), ProviderError::CallbackPhase(_)));
            assert!(failure.message().projection.is_some());
            let bodies = seen.lock().unwrap().clone();
            assert_eq!(bodies.len(), 1);
            let body = &bodies[0];
            assert_eq!(
                body.get("max_output_tokens").is_none(),
                signin,
                "{provider_id} {base}"
            );
            assert_eq!(body.get("temperature").is_none(), signin);
            assert_eq!(body.get("prompt_cache_options").is_none(), signin);
            assert!(body.get("prompt_cache_retention").is_none());
            let updates = body["input"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|item| item["type"] == "configuration_update")
                .count();
            assert_eq!(updates, usize::from(!signin));
            assert_eq!(response_calls.load(Ordering::SeqCst), 0);
            assert!(
                fallback.received_requests().await.unwrap().is_empty(),
                "capture sentinel precedes every transport send, including canonical external host"
            );
        }
    }
}
