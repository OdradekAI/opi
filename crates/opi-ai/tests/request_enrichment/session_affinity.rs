//! Session affinity headers, cache keys, and retention-aware wire policy.

use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::http::HttpClient;
use opi_ai::provider::{CacheRetention, ModelInfo, Provider, Request};
use opi_ai::registry::ModelCapabilities;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::fixtures::{
    drain, header_value, make_anthropic_request, make_openai_chat_request, request_body_json,
};

fn make_openai_responses_request(model: &str, session_id: &str) -> Request {
    let mut request = make_openai_chat_request(CancellationToken::new(), Some(session_id.into()));
    request.model = model.into();
    request
}

#[tokio::test]
async fn openai_chat_session_id_becomes_prompt_cache_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"delta\":{\"content\":\"hi\"},\"index\":0,\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = opi_ai::providers::openai::OpenAiChatProvider::new(Some(server.uri()));
    let request = make_openai_chat_request(cancel, Some("sess-abc123".into()));

    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    while let Some(item) = stream.next().await {
        match item {
            Ok(_) => {}
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
    let request = server.received_requests().await.unwrap().remove(0);
    assert_eq!(
        request_body_json(&request)["prompt_cache_key"],
        "sess-abc123"
    );
    assert_eq!(header_value(&request, "prompt-cache-key"), None);
}

#[tokio::test]
async fn openai_chat_session_id_clamps_to_64_chars() {
    let session_id = "a".repeat(100);
    let clamped = session_id[..64].to_owned();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"delta\":{\"content\":\"hi\"},\"index\":0,\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = opi_ai::providers::openai::OpenAiChatProvider::new(Some(server.uri()));
    let request = make_openai_chat_request(cancel, Some(session_id));

    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    while let Some(item) = stream.next().await {
        match item {
            Ok(_) => {}
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
    let request = server.received_requests().await.unwrap().remove(0);
    assert_eq!(request_body_json(&request)["prompt_cache_key"], clamped);
}

#[tokio::test]
async fn openai_chat_no_session_id_no_cache_key_header() {
    let server = MockServer::start().await;
    // The mock accepts any POST without requiring a prompt-cache-key header.
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"delta\":{\"content\":\"hi\"},\"index\":0,\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = opi_ai::providers::openai::OpenAiChatProvider::new(Some(server.uri()));
    let request = make_openai_chat_request(cancel, None);

    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    while let Some(item) = stream.next().await {
        match item {
            Ok(_) => {}
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
    let request = server.received_requests().await.unwrap().remove(0);
    assert!(
        request_body_json(&request)
            .get("prompt_cache_key")
            .is_none()
    );
    assert_eq!(header_value(&request, "prompt-cache-key"), None);
}

#[tokio::test]
async fn session_affinity_wire_mappings() {
    let chat_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string("data: [DONE]\n\n"))
        .expect(4)
        .mount(&chat_server)
        .await;
    let models = vec![ModelInfo::new(
        "model",
        "Model",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(8_192, 1_024).with_streaming(true),
    )];
    let direct = opi_ai::providers::openai::OpenAiChatProvider::new(Some(chat_server.uri()));
    drain(direct.stream_prepared(
        make_openai_chat_request(CancellationToken::new(), Some("session-direct".into())),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;
    let compatible = opi_ai::providers::openai::OpenAiChatProvider::new_for_profile(
        chat_server.uri(),
        "compatible".into(),
        opi_ai::providers::openai::CompatConfig {
            send_session_affinity_headers: Some(true),
            ..Default::default()
        },
        vec![],
        models.clone(),
    );
    drain(compatible.stream_prepared(
        make_openai_chat_request(CancellationToken::new(), Some("session-compatible".into())),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;
    let compatible_default = opi_ai::providers::openai::OpenAiChatProvider::new_for_profile(
        chat_server.uri(),
        "compatible-default".into(),
        Default::default(),
        vec![],
        models,
    );
    drain(compatible_default.stream_prepared(
        make_openai_chat_request(CancellationToken::new(), Some("session-default".into())),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;
    let mut disabled =
        make_openai_chat_request(CancellationToken::new(), Some("session-disabled".into()));
    disabled.cache_retention = CacheRetention::Disabled;
    drain(compatible.stream_prepared(disabled, opi_ai::test_support::resolved_bearer_auth())).await;

    let chat_requests = chat_server.received_requests().await.unwrap();
    assert_eq!(chat_requests.len(), 4);
    assert_eq!(
        request_body_json(&chat_requests[0])["prompt_cache_key"],
        "session-direct"
    );
    for name in ["session_id", "x-client-request-id", "x-session-affinity"] {
        assert_eq!(header_value(&chat_requests[0], name), None);
        assert_eq!(
            header_value(&chat_requests[1], name),
            Some("session-compatible")
        );
        assert_eq!(header_value(&chat_requests[2], name), None);
        assert_eq!(
            header_value(&chat_requests[3], name),
            Some("session-disabled"),
            "explicit Chat affinity is independent of prompt-cache retention"
        );
    }
    for request in &chat_requests[1..] {
        assert!(request_body_json(request).get("prompt_cache_key").is_none());
    }

    let responses_server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string("data: [DONE]\n\n"))
        .expect(5)
        .mount(&responses_server)
        .await;
    let standard = opi_ai::providers::openai_responses::OpenAiResponsesProvider::new(Some(
        responses_server.uri(),
    ));
    drain(standard.stream_prepared(
        make_openai_responses_request("openai-responses:model", "session-standard"),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;
    let direct_with_session_header =
        opi_ai::providers::openai_responses::OpenAiResponsesProvider::new_with_config(
            Some(responses_server.uri()),
            opi_ai::providers::openai_responses::ResponsesConfig {
                send_session_id_header: false,
                ..Default::default()
            },
        );
    drain(direct_with_session_header.stream_prepared(
        make_openai_responses_request("openai-responses:model", "session-direct-header"),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;
    let mut disabled = make_openai_responses_request("openai-responses:model", "session-disabled");
    disabled.cache_retention = CacheRetention::Disabled;
    drain(standard.stream_prepared(disabled, opi_ai::test_support::resolved_bearer_auth())).await;
    let custom_model = ModelInfo::new(
        "model",
        "Model",
        opi_ai::WireApi::OpenAiResponses,
        ModelCapabilities::new(128_000, 16_384).with_streaming(true),
    );
    let custom_default = opi_ai::providers::openai_responses::OpenAiResponsesProvider::for_route(
        Some(responses_server.uri()),
        "custom-responses".into(),
        opi_ai::api::shared::headers::ProviderHeaders::default(),
        vec![custom_model.clone()],
        Arc::new(HttpClient::new()),
    );
    drain(custom_default.stream_prepared(
        make_openai_responses_request("custom-responses:model", "session-custom-default"),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;
    let custom_opt_in_model = custom_model
        .with_compat(opi_ai::WireCompat::OpenAiResponses(
            opi_ai::model_info::OpenAiResponsesCompat {
                send_session_id_header: true,
                ..Default::default()
            },
        ))
        .unwrap();
    let custom_opt_in = opi_ai::providers::openai_responses::OpenAiResponsesProvider::for_route(
        Some(responses_server.uri()),
        "custom-responses".into(),
        opi_ai::api::shared::headers::ProviderHeaders::default(),
        vec![custom_opt_in_model],
        Arc::new(HttpClient::new()),
    );
    drain(custom_opt_in.stream_prepared(
        make_openai_responses_request("custom-responses:model", "session-custom-opt-in"),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;

    let response_requests = responses_server.received_requests().await.unwrap();
    assert_eq!(response_requests.len(), 5);
    assert_eq!(
        request_body_json(&response_requests[0])["prompt_cache_key"],
        "session-standard"
    );
    assert_eq!(
        header_value(&response_requests[0], "x-client-request-id"),
        Some("session-standard")
    );
    assert_eq!(
        header_value(&response_requests[0], "session_id"),
        Some("session-standard")
    );

    assert_eq!(
        request_body_json(&response_requests[1])["prompt_cache_key"],
        "session-direct-header"
    );
    assert_eq!(
        header_value(&response_requests[1], "x-client-request-id"),
        Some("session-direct-header")
    );
    assert_eq!(header_value(&response_requests[1], "session_id"), None);

    for request in &response_requests[2..4] {
        assert!(request_body_json(request).get("prompt_cache_key").is_none());
        for name in ["session_id", "session-id", "x-client-request-id"] {
            assert_eq!(header_value(request, name), None);
        }
    }
    assert_eq!(
        request_body_json(&response_requests[4])["prompt_cache_key"],
        "session-custom-opt-in"
    );
    assert_eq!(
        header_value(&response_requests[4], "session_id"),
        Some("session-custom-opt-in")
    );
    assert_eq!(
        header_value(&response_requests[4], "x-client-request-id"),
        Some("session-custom-opt-in")
    );

    let anthropic_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .expect(1)
        .mount(&anthropic_server)
        .await;
    let anthropic =
        opi_ai::providers::anthropic::AnthropicProvider::new(Some(anthropic_server.uri()));
    let mut request = make_anthropic_request(CancellationToken::new(), None);
    request.session_id = Some("session-anthropic".into());
    drain(anthropic.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth())).await;
    let request = anthropic_server
        .received_requests()
        .await
        .unwrap()
        .remove(0);
    for name in [
        "prompt-cache-key",
        "session-id",
        "session_id",
        "x-client-request-id",
        "x-session-affinity",
    ] {
        assert_eq!(header_value(&request, name), None);
    }
}

#[cfg(test)]
mod responses_generated_affinity {
    use super::*;
    use opi_ai::model_info::{OpenAiResponsesSourceCompat, SessionAffinityFormat};
    use opi_ai::providers::openai_responses::{
        OpenAiResponsesProvider, ResponsesAffinityPolicy, ResponsesConfig,
    };

    const RESPONSE: &str = concat!(
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_probe\",\"status\":\"in_progress\",\"model\":\"model\",\"output\":[]}}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_probe\",\"status\":\"completed\",\"model\":\"model\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"ok\"}]}],\"usage\":{\"input_tokens\":3,\"output_tokens\":5}}}\n\n",
    );

    async fn capture(
        policy: ResponsesAffinityPolicy,
        flag: bool,
        format: Option<SessionAffinityFormat>,
        provider_id: &str,
        retention: CacheRetention,
        session: Option<&str>,
        extra: Vec<(String, String)>,
    ) -> wiremock::Request {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(RESPONSE),
            )
            .expect(1)
            .mount(&server)
            .await;
        let provider = OpenAiResponsesProvider::with_auth_extra(
            Some(server.uri()),
            ResponsesConfig {
                send_session_id_header: flag,
                source: OpenAiResponsesSourceCompat {
                    session_affinity_format: format,
                    ..Default::default()
                },
                ..Default::default()
            },
            provider_id.into(),
            vec![],
            Arc::new(HttpClient::new()),
        )
        .with_affinity_policy(policy);
        let mut request = make_openai_responses_request(&format!("{provider_id}:model"), "");
        request.session_id = session.map(str::to_owned);
        request.cache_retention = retention;
        request.extra_headers = extra;
        let result = opi_ai::ChatStream::new(
            provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()),
        )
        .result()
        .await
        .expect("real Responses terminal");
        assert_eq!(result.stop_reason, opi_ai::stream::StopReason::Stop);
        assert_eq!(result.usage.input_tokens, 3);
        assert_eq!(result.usage.output_tokens, 5);
        let mut sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 1);
        server.verify().await;
        sent.remove(0)
    }

    fn assert_affinity(
        request: &wiremock::Request,
        key: Option<&str>,
        session: Option<&str>,
        client: Option<&str>,
        router: Option<&str>,
    ) {
        assert_eq!(
            request_body_json(request)
                .get("prompt_cache_key")
                .and_then(serde_json::Value::as_str),
            key
        );
        assert_eq!(
            header_value(request, "session_id"),
            session,
            "literal session_id"
        );
        assert_eq!(
            header_value(request, "x-client-request-id"),
            client,
            "source client request id"
        );
        assert_eq!(
            header_value(request, "x-session-id"),
            router,
            "router session id"
        );
        assert_eq!(
            header_value(request, "x-session-affinity"),
            None,
            "Responses does not inherit Chat's third header"
        );
    }

    #[tokio::test]
    async fn direct_long_session_keeps_raw_header_ids_and_clamps_only_cache_key() {
        let session = "source-session-".repeat(8);
        let sent = capture(
            ResponsesAffinityPolicy::Direct,
            true,
            None,
            "openai-responses",
            CacheRetention::Short,
            Some(&session),
            vec![],
        )
        .await;
        assert_affinity(
            &sent,
            Some(&session[..64]),
            Some(&session),
            Some(&session),
            None,
        );
    }

    #[tokio::test]
    async fn direct_false_omits_only_session_id_even_with_explicit_openai_format() {
        let sent = capture(
            ResponsesAffinityPolicy::Direct,
            false,
            Some(SessionAffinityFormat::OpenAi),
            "openai-responses",
            CacheRetention::Short,
            Some("source-session"),
            vec![],
        )
        .await;
        assert_affinity(
            &sent,
            Some("source-session"),
            None,
            Some("source-session"),
            None,
        );
    }

    async fn disabled(format: SessionAffinityFormat, policy: ResponsesAffinityPolicy) {
        let sent = capture(
            policy,
            true,
            Some(format),
            "openai-responses",
            CacheRetention::Disabled,
            Some("source-session"),
            vec![],
        )
        .await;
        assert_affinity(&sent, None, None, None, None);
    }

    #[tokio::test]
    async fn disabled_retention_omits_generated_openai_affinity() {
        disabled(
            SessionAffinityFormat::OpenAi,
            ResponsesAffinityPolicy::Direct,
        )
        .await;
    }

    #[tokio::test]
    async fn disabled_retention_omits_generated_openai_nosession_affinity() {
        disabled(
            SessionAffinityFormat::OpenAiNoSession,
            ResponsesAffinityPolicy::Direct,
        )
        .await;
    }

    #[tokio::test]
    async fn disabled_retention_omits_generated_openrouter_affinity() {
        disabled(
            SessionAffinityFormat::OpenRouter,
            ResponsesAffinityPolicy::Direct,
        )
        .await;
    }

    #[tokio::test]
    async fn custom_optin_enabled_keeps_full_source_mapping() {
        let sent = capture(
            ResponsesAffinityPolicy::CustomOptIn,
            true,
            Some(SessionAffinityFormat::OpenAi),
            "custom",
            CacheRetention::Long,
            Some("source-session"),
            vec![],
        )
        .await;
        assert_affinity(
            &sent,
            Some("source-session"),
            Some("source-session"),
            Some("source-session"),
            None,
        );
    }

    #[tokio::test]
    async fn custom_disabled_and_false_optin_keep_zero_generated_affinity() {
        for (policy, flag) in [
            (ResponsesAffinityPolicy::CustomDisabled, true),
            (ResponsesAffinityPolicy::CustomOptIn, false),
        ] {
            let sent = capture(
                policy,
                flag,
                Some(SessionAffinityFormat::OpenAi),
                "custom",
                CacheRetention::Short,
                Some("source-session"),
                vec![],
            )
            .await;
            assert_affinity(&sent, None, None, None, None);
        }
    }

    #[tokio::test]
    async fn custom_optin_does_not_bypass_disabled_retention() {
        disabled(
            SessionAffinityFormat::OpenRouter,
            ResponsesAffinityPolicy::CustomOptIn,
        )
        .await;
    }

    #[tokio::test]
    async fn explicit_format_wins_over_openrouter_provider_inference() {
        for format in [
            SessionAffinityFormat::OpenAi,
            SessionAffinityFormat::OpenAiNoSession,
        ] {
            let sent = capture(
                ResponsesAffinityPolicy::Direct,
                true,
                Some(format),
                "openrouter",
                CacheRetention::Short,
                Some("source-session"),
                vec![],
            )
            .await;
            assert_affinity(
                &sent,
                Some("source-session"),
                (format == SessionAffinityFormat::OpenAi).then_some("source-session"),
                Some("source-session"),
                None,
            );
        }
    }

    #[tokio::test]
    async fn explicit_caller_affinity_header_overrides_default_and_survives_disabled_cache() {
        for retention in [CacheRetention::Short, CacheRetention::Disabled] {
            let sent = capture(
                ResponsesAffinityPolicy::Direct,
                true,
                Some(SessionAffinityFormat::OpenRouter),
                "openai-responses",
                retention,
                Some("source-session"),
                vec![("x-session-id".into(), "caller-value".into())],
            )
            .await;
            assert_affinity(
                &sent,
                (retention != CacheRetention::Disabled).then_some("source-session"),
                None,
                None,
                Some("caller-value"),
            );
        }
    }

    #[tokio::test]
    async fn direct_false_does_not_disable_openrouter_header_family() {
        let sent = capture(
            ResponsesAffinityPolicy::Direct,
            false,
            Some(SessionAffinityFormat::OpenRouter),
            "openai-responses",
            CacheRetention::Short,
            Some("source-session"),
            vec![],
        )
        .await;
        assert_affinity(
            &sent,
            Some("source-session"),
            None,
            None,
            Some("source-session"),
        );
    }

    #[tokio::test]
    async fn absent_and_empty_session_emit_no_generated_affinity() {
        for session in [None, Some("")] {
            let sent = capture(
                ResponsesAffinityPolicy::Direct,
                true,
                None,
                "openai-responses",
                CacheRetention::Short,
                session,
                vec![],
            )
            .await;
            assert_affinity(&sent, None, None, None, None);
        }
    }

    #[tokio::test]
    async fn chat_disabled_cache_preserves_explicitly_enabled_affinity() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).insert_header("content-type", "text/event-stream").set_body_string("data: {\"id\":\"chat_probe\",\"object\":\"chat.completion.chunk\",\"model\":\"model\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n"))
            .expect(1).mount(&server).await;
        let provider = opi_ai::providers::openai::OpenAiChatProvider::new_for_profile(
            server.uri(),
            "compatible".into(),
            opi_ai::providers::openai::CompatConfig {
                send_session_affinity_headers: Some(true),
                ..Default::default()
            },
            vec![],
            vec![],
        );
        let mut request =
            make_openai_chat_request(CancellationToken::new(), Some("source-session".into()));
        request.cache_retention = CacheRetention::Disabled;
        let result = opi_ai::ChatStream::new(
            provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()),
        )
        .result()
        .await
        .expect("real Chat terminal");
        assert_eq!(result.stop_reason, opi_ai::stream::StopReason::Stop);
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 1);
        for name in ["session_id", "x-client-request-id", "x-session-affinity"] {
            assert_eq!(header_value(&sent[0], name), Some("source-session"));
        }
        assert!(
            request_body_json(&sent[0])
                .get("prompt_cache_key")
                .is_none()
        );
        server.verify().await;
    }
}
