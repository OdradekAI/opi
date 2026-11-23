//! Payload admission, response observers, and retry reuse of the frozen request.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use opi_ai::callbacks::RequestCallbacks;
use opi_ai::http::HttpClient;
use opi_ai::provider::{ModelInfo, Provider, ProviderError, Request};
use opi_ai::registry::ModelCapabilities;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::fixtures::{make_openai_chat_request, request_body_json};

async fn payload_wire_fixture(
    responses: bool,
) -> (MockServer, opi_ai::Models, opi_ai::ChatModelRecord, Request) {
    let server = MockServer::start().await;
    let sse = if responses {
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"r1\",\"status\":\"completed\",\"output\":[]}}\n\n"
    } else {
        "data: {\"id\":\"c1\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n"
    };
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sse))
        .mount(&server)
        .await;
    let provider: Box<dyn Provider> = if responses {
        Box::new(
            opi_ai::providers::openai_responses::OpenAiResponsesProvider::new(Some(server.uri())),
        )
    } else {
        Box::new(opi_ai::providers::openai::OpenAiChatProvider::new(Some(
            server.uri(),
        )))
    };
    let record = opi_ai::ChatModelRecord::new(provider.id(), provider.models()[0].clone());
    let mut models = opi_ai::Models::new();
    models
        .register_route(
            provider,
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::Bearer,
                secrecy::SecretString::from("offline-payload-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let request = make_openai_chat_request(CancellationToken::new(), None);
    (server, models, record, request)
}

async fn assert_invalid_payload_wire_shapes(
    responses: bool,
    cases: Vec<(&str, serde_json::Value)>,
) {
    let (server, models, record, request) = payload_wire_fixture(responses).await;
    let mut unexpected = Vec::new();
    for (label, patch) in cases {
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = calls.clone();
        let mut request = request.clone();
        request.callbacks = RequestCallbacks::new().with_on_payload(Arc::new(move |body| {
            counted.fetch_add(1, Ordering::SeqCst);
            let mut replacement = body.clone();
            for (key, value) in patch.as_object().unwrap() {
                replacement[key] = value.clone();
            }
            Ok(Some(replacement))
        }));
        let sent_before = server.received_requests().await.unwrap().len();
        let result = models
            .complete_simple(record.clone(), request, Default::default())
            .await;
        let sent = server.received_requests().await.unwrap().len() - sent_before;
        let typed_failure = result
            .as_ref()
            .err()
            .is_some_and(|failure| matches!(failure.cause(), ProviderError::CallbackPhase(_)));
        if !typed_failure || sent != 0 || calls.load(Ordering::SeqCst) != 1 {
            unexpected.push(format!(
                "{label}: typed={typed_failure}, sends={sent}, result={result:?}"
            ));
        }
    }
    assert!(unexpected.is_empty(), "{}", unexpected.join("\n"));
}

#[test]
fn request_callbacks_default_empty_and_has_no_wire_observers() {
    let callbacks = RequestCallbacks::new();
    assert!(callbacks.is_empty());
    assert!(!callbacks.has_wire_observers());
}

#[test]
fn payload_replacement_retargeting_model_identity_fails_typed() {
    let callbacks = RequestCallbacks::new().with_on_payload(Arc::new(|body| {
        let mut replacement = body.clone();
        replacement["model"] = serde_json::json!("claude-evil-rewrite");
        Ok(Some(replacement))
    }));
    let outcome = callbacks.apply_payload(
        "ordinary",
        serde_json::json!({"model": "claude-sonnet", "max_tokens": 16}),
    );
    match outcome {
        Err(ProviderError::CallbackPhase(_)) => {}
        other => panic!("model retarget must fail typed, got {other:?}"),
    }
}

#[test]
fn payload_replacement_raising_output_budget_fails_typed() {
    let callbacks = RequestCallbacks::new().with_on_payload(Arc::new(|body| {
        let mut replacement = body.clone();
        replacement["max_tokens"] = serde_json::json!(999_999);
        Ok(Some(replacement))
    }));
    let outcome = callbacks.apply_payload(
        "ordinary",
        serde_json::json!({"model": "claude-sonnet", "max_tokens": 16}),
    );
    assert!(
        matches!(outcome, Err(ProviderError::CallbackPhase(_))),
        "raising the resolved output budget must fail typed"
    );
}

#[test]
fn payload_validation_covers_wire_budget_paths_and_required_model() {
    for pointer in [
        "/max_tokens",
        "/max_completion_tokens",
        "/max_output_tokens",
        "/generationConfig/maxOutputTokens",
        "/inferenceConfig/maxTokens",
    ] {
        for changed in [
            serde_json::json!(65),
            serde_json::json!(-1),
            serde_json::json!("64"),
            serde_json::Value::Null,
        ] {
            let mut original =
                serde_json::json!({"model":"frozen", "generationConfig":{}, "inferenceConfig":{}});
            let (parent, key) = pointer.rsplit_once('/').unwrap();
            original
                .pointer_mut(parent)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert(key.into(), serde_json::json!(64));
            let mut replacement = original.clone();
            *replacement.pointer_mut(pointer).unwrap() = changed;
            let callbacks = RequestCallbacks::new()
                .with_on_payload(Arc::new(move |_| Ok(Some(replacement.clone()))));
            assert!(
                matches!(
                    callbacks.apply_payload("ordinary", original),
                    Err(ProviderError::CallbackPhase(_))
                ),
                "wire budget {pointer} must be checked"
            );
        }
    }
    for replacement in [
        serde_json::json!({}),
        serde_json::json!({"model":null}),
        serde_json::json!({"model":false}),
    ] {
        let callbacks = RequestCallbacks::new()
            .with_on_payload(Arc::new(move |_| Ok(Some(replacement.clone()))));
        assert!(matches!(
            callbacks.apply_payload("ordinary", serde_json::json!({"model":"frozen"})),
            Err(ProviderError::CallbackPhase(_))
        ));
    }
}

#[tokio::test]
async fn async_callbacks_yield_freeze_once_and_preserve_observer_order() {
    let order = Arc::new(std::sync::Mutex::new(Vec::new()));
    let headers_order = order.clone();
    let payload_order = order.clone();
    let response_order = order.clone();
    let raw_order = order.clone();
    let callbacks = RequestCallbacks::new()
        .with_async_header_transform(move |headers| {
            let order = headers_order.clone();
            async move {
                tokio::task::yield_now().await;
                order.lock().unwrap().push("headers");
                Ok(headers)
            }
        })
        .with_async_on_payload(move |body| {
            let order = payload_order.clone();
            async move {
                tokio::task::yield_now().await;
                order.lock().unwrap().push("payload");
                Ok(Some(body))
            }
        })
        .with_async_on_response(move |status, _headers| {
            let order = response_order.clone();
            async move {
                tokio::task::yield_now().await;
                assert_eq!(status, 200);
                order.lock().unwrap().push("response");
                Ok(())
            }
        })
        .with_async_on_raw(move |raw| {
            let order = raw_order.clone();
            async move {
                tokio::task::yield_now().await;
                assert_eq!(raw["wire"], true);
                order.lock().unwrap().push("raw");
                Ok(())
            }
        });
    let headers = vec![("x-custom".into(), "first".into())];
    assert!(matches!(
        callbacks.apply_header_transform(&headers),
        Err(ProviderError::CallbackPhase(_))
    ));
    assert!(matches!(
        callbacks.apply_payload("ordinary", serde_json::json!({})),
        Err(ProviderError::CallbackPhase(_))
    ));
    assert!(matches!(
        callbacks.notify_response(200, &[]),
        Err(ProviderError::CallbackPhase(_))
    ));
    assert!(matches!(
        callbacks.notify_raw(&serde_json::json!({})),
        Err(ProviderError::CallbackPhase(_))
    ));
    assert_eq!(
        callbacks
            .apply_header_transform_async(&headers)
            .await
            .unwrap(),
        headers
    );
    assert_eq!(
        callbacks.apply_header_transform_async(&[]).await.unwrap(),
        headers
    );
    let body = serde_json::json!({"model":"frozen", "max_tokens":64});
    let (first, second) = tokio::join!(
        callbacks.apply_payload_async("ordinary", body.clone()),
        callbacks.apply_payload_async("ordinary", body.clone())
    );
    assert_eq!(first.unwrap(), body);
    assert_eq!(second.unwrap(), body);
    callbacks.notify_response_async(200, &[]).await.unwrap();
    callbacks
        .notify_raw_async(&serde_json::json!({"wire":true}))
        .await
        .unwrap();
    assert_eq!(
        *order.lock().unwrap(),
        ["headers", "payload", "response", "raw"]
    );
}

#[tokio::test]
async fn payload_callback_chat_wire_shapes_fail_before_dispatch() {
    assert_invalid_payload_wire_shapes(
        false,
        vec![
            ("messages scalar", serde_json::json!({"messages": 123})),
            ("message scalar", serde_json::json!({"messages": [123]})),
            (
                "message content scalar",
                serde_json::json!({"messages": [{"role":"user","content":123}]}),
            ),
            (
                "tool call scalar",
                serde_json::json!({"messages": [{"role":"assistant","tool_calls":[123]}]}),
            ),
            ("tools scalar", serde_json::json!({"tools": 123})),
            ("tool scalar", serde_json::json!({"tools": [123]})),
            (
                "function wrapper scalar",
                serde_json::json!({"tools": [{"type":"function","function":123}]}),
            ),
            (
                "tool choice scalar",
                serde_json::json!({"tool_choice": 123}),
            ),
            (
                "response format scalar",
                serde_json::json!({"response_format": 123}),
            ),
            ("non-streaming", serde_json::json!({"stream": false})),
        ],
    )
    .await;
}

#[tokio::test]
async fn payload_callback_responses_wire_shapes_fail_before_dispatch() {
    assert_invalid_payload_wire_shapes(
        true,
        vec![
            ("input scalar", serde_json::json!({"input": 123})),
            ("input item scalar", serde_json::json!({"input": [123]})),
            ("tools scalar", serde_json::json!({"tools": 123})),
            ("tool scalar", serde_json::json!({"tools": [123]})),
            (
                "parameters scalar",
                serde_json::json!({"tools": [{"type":"function","name":"probe","parameters":123}]}),
            ),
            (
                "custom format scalar",
                serde_json::json!({"tools": [{"type":"custom","name":"probe","format":123}]}),
            ),
            (
                "tool choice scalar",
                serde_json::json!({"tool_choice": 123}),
            ),
            ("reasoning scalar", serde_json::json!({"reasoning": 123})),
            ("text wrapper scalar", serde_json::json!({"text": 123})),
            ("non-streaming", serde_json::json!({"stream": false})),
        ],
    )
    .await;
}

#[tokio::test]
async fn payload_callback_keeps_legal_chat_and_responses_wire_changes() {
    for responses in [false, true] {
        let (server, models, record, mut request) = payload_wire_fixture(responses).await;
        request.callbacks = RequestCallbacks::new().with_on_payload(Arc::new(move |body| {
            let mut replacement = body.clone();
            replacement["vendor_extension"] = serde_json::json!({"arbitrary":[1,true]});
            if responses {
                replacement["input"] = serde_json::json!("replacement input");
                replacement["tools"] = serde_json::json!([
                    {"type":"function","name":"probe","parameters":{"type":"object"}},
                    {"type":"custom","name":"custom_probe","format":{"type":"text"}},
                    {"type":"future_vendor_tool","future_field":123}
                ]);
                replacement["text"] = serde_json::json!({"format":{"type":"text"}});
            } else {
                replacement["messages"] = serde_json::json!([
                    {"role":"user","content":[{"type":"text","text":"replacement input"}]}
                ]);
                replacement["tools"] = serde_json::json!([
                    {"type":"function","function":{"name":"probe","parameters":{"type":"object"}}},
                    {"type":"custom","custom":{"name":"custom_probe","format":{"type":"text"}}}
                ]);
                replacement["response_format"] = serde_json::json!({"type":"json_object"});
            }
            Ok(Some(replacement))
        }));
        models
            .complete_simple(record, request, Default::default())
            .await
            .expect("shape-preserving callback change remains supported");
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 1);
        let body = request_body_json(&sent[0]);
        assert_eq!(
            body["vendor_extension"]["arbitrary"],
            serde_json::json!([1, true])
        );
        assert_eq!(
            body["tools"].as_array().unwrap().len(),
            if responses { 3 } else { 2 }
        );
        assert_eq!(
            body[if responses { "input" } else { "messages" }],
            if responses {
                serde_json::json!("replacement input")
            } else {
                serde_json::json!([{"role":"user","content":[{"type":"text","text":"replacement input"}]}])
            }
        );
    }
}

#[tokio::test]
async fn payload_callback_codex_corrupt_input_fails_before_transport() {
    struct CodexAuth;
    impl opi_ai::auth::AuthResolver for CodexAuth {
        fn resolve<'a>(
            &'a self,
        ) -> opi_ai::BoxAuthFuture<'a, Result<opi_ai::auth::ResolvedAuth, ProviderError>> {
            Box::pin(async {
                let mut auth = opi_ai::test_support::resolved_bearer_auth();
                auth.account_id = Some("offline-payload-account".into());
                Ok(auth)
            })
        }
    }
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"r1\",\"status\":\"completed\",\"output\":[]}}\n\n",
        ))
        .mount(&server)
        .await;
    let model = ModelInfo::new(
        "gpt-5.4",
        "GPT-5.4",
        opi_ai::WireApi::OpenAiCodexResponses,
        ModelCapabilities::new(100_000, 10_000).with_streaming(true),
    );
    let provider = opi_ai::providers::openai_codex::OpenAiCodexResponsesProvider::new(
        Some(server.uri()),
        vec![model.clone()],
        Arc::new(HttpClient::new()),
    );
    let record = opi_ai::ChatModelRecord::new(provider.id(), model);
    let mut models = opi_ai::Models::new();
    models
        .register_route(
            Box::new(provider),
            Arc::new(CodexAuth),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let mut unexpected = Vec::new();
    for transport in [
        opi_ai::provider::TransportPreference::Sse,
        opi_ai::provider::TransportPreference::Websocket,
        opi_ai::provider::TransportPreference::Auto,
    ] {
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = calls.clone();
        let mut request = make_openai_chat_request(CancellationToken::new(), None);
        request.transport = transport;
        request.callbacks = RequestCallbacks::new().with_on_payload(Arc::new(move |body| {
            counted.fetch_add(1, Ordering::SeqCst);
            let mut replacement = body.clone();
            replacement["input"] = serde_json::json!(123);
            Ok(Some(replacement))
        }));
        let sent_before = server.received_requests().await.unwrap().len();
        let result = models
            .complete_simple(record.clone(), request, Default::default())
            .await;
        let sent = server.received_requests().await.unwrap();
        let posts = sent[sent_before..]
            .iter()
            .filter(|request| request.method == "POST")
            .count();
        let upgrades = sent[sent_before..]
            .iter()
            .filter(|request| request.method == "GET")
            .count();
        let typed_failure = result
            .as_ref()
            .err()
            .is_some_and(|failure| matches!(failure.cause(), ProviderError::CallbackPhase(_)));
        if !typed_failure || posts != 0 || upgrades != 0 || calls.load(Ordering::SeqCst) != 1 {
            unexpected.push(format!(
                "{transport:?}: typed={typed_failure}, posts={posts}, upgrades={upgrades}, result={result:?}"
            ));
        }
    }
    assert!(unexpected.is_empty(), "{}", unexpected.join("\n"));
}

#[tokio::test]
async fn payload_callback_freezes_once_across_ordinary_acquisition_retry() {
    use opi_ai::auth::{AuthProvenanceSource, StaticAuthResolver};
    use opi_ai::http::HttpClientBuilder;
    use opi_ai::models::Models;
    use opi_ai::providers::openai::OpenAiChatProvider;
    use opi_ai::retry::AcquisitionPolicy;
    use opi_ai::stream::ChatStream;
    use secrecy::SecretString;

    let server = MockServer::start().await;
    // First send: retryable 429 with an immediate advisory hint.
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after-ms", "1"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    // Second send: a terminal streaming completion.
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"delta\":{\"content\":\"hi\"},\"index\":0,\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let client = Arc::new(HttpClientBuilder::new().build().expect("client build"));
    let provider =
        OpenAiChatProvider::with_client(Some(server.uri()), "openai".to_owned(), vec![], client);
    let mut models = Models::new();
    models
        .register_route(
            Box::new(provider),
            Arc::new(StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::Bearer,
                SecretString::from("freeze-key"),
            )),
            AuthProvenanceSource::Static,
            opi_ai::models::CompatMetadata {
                openai_compatible: true,
                profile: None,
            },
        )
        .expect("register route");

    let payload_calls = Arc::new(AtomicUsize::new(0));
    let calls = Arc::clone(&payload_calls);
    let mut request = make_openai_chat_request(CancellationToken::new(), None);
    request.callbacks = RequestCallbacks::new().with_on_payload(Arc::new(move |body| {
        calls.fetch_add(1, Ordering::SeqCst);
        let mut replacement = body.clone();
        replacement["frozen_marker"] = serde_json::json!("replaced-once");
        Ok(Some(replacement))
    }));

    let mut policy = AcquisitionPolicy::with_max_retries(1);
    policy.jitter_seed = Some(0);
    let call = models
        .prepare_call_with_policy("openai:gpt-4o", request, policy)
        .await
        .expect("prepare call");

    let first = ChatStream::new(call.start_attempt().expect("first attempt"))
        .result()
        .await
        .expect_err("first attempt is a retryable 429");
    assert!(matches!(first.cause(), ProviderError::RateLimited { .. }));
    let admission = call.retry_admission().expect("retry admitted");
    assert_eq!(admission.retry_index, 1);
    tokio::time::sleep(std::time::Duration::from_millis(admission.delay_ms.max(1))).await;

    ChatStream::new(call.start_attempt().expect("second attempt"))
        .result()
        .await
        .expect("second attempt completes");

    assert_eq!(
        payload_calls.load(Ordering::SeqCst),
        1,
        "payload callback runs exactly once per variant across ordinary retry"
    );
    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[1].body, sent[0].body, "frozen request bytes");
    let first_body = request_body_json(&sent[0]);
    let second_body = request_body_json(&sent[1]);
    assert_eq!(
        first_body["frozen_marker"],
        serde_json::json!("replaced-once")
    );
    assert_eq!(
        second_body, first_body,
        "the ordinary retry reuses the frozen payload byte-for-byte"
    );
}
