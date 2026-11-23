//! Public Models construction, request admission, execution and cleanup.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use opi_ai::api::contract::{SimpleOptions, SimpleToolChoice};
use opi_ai::auth::AuthScheme;
use opi_ai::callbacks::RequestCallbacks;
use opi_ai::http::HttpClient;
use opi_ai::message::{
    AssistantContent, InputContent, Message, OutputContent, ToolDef, ToolResultMessage, UserMessage,
};
use opi_ai::models::{ChatModelRecord, Models};
use opi_ai::test_support::{MockProvider, ScriptedDeferredBackend, base_assistant, text_response};
use opi_ai::transcript::{self, Context as ShorthandContext, TranscriptContext, normalize_context};
use opi_ai::{AuthProvenanceSource, WireApi};
use secrecy::SecretString;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::fixtures::{
    OwnedRecordingContext, anthropic_text_sse, base_request, message_text, openai_completions_sse,
    sse_template, user_request,
};

fn completions_info(model: &str) -> opi_ai::model_info::ModelInfo {
    opi_ai::model_info::ModelInfo::new(
        model,
        "Matrix Model",
        WireApi::OpenAiCompletions,
        opi_ai::registry::ModelCapabilities::new(100_000, 4_096),
    )
}

fn anthropic_tool_call_sse() -> String {
    concat!(
        "event: message_start\n",
        r#"data: {"type":"message_start","message":{"id":"msg_tool","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":20,"output_tokens":0}}}"#, "\n\n",
        "event: content_block_start\n",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"get_weather","input":{}}}"#, "\n\n",
        "event: content_block_delta\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"city\":\"Paris\"}"}}"#, "\n\n",
        "event: content_block_stop\n",
        r#"data: {"type":"content_block_stop","index":0}"#, "\n\n",
        "event: message_delta\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"tool_use","stop_sequence":null},"usage":{"output_tokens":9}}"#, "\n\n",
        "event: message_stop\n",
        r#"data: {"type":"message_stop"}"#, "\n\n",
    )
    .into()
}

#[tokio::test]
async fn simple_options_port_across_wires_and_mismatched_native_fails_typed_preflight() {
    let peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(body_partial_json(
            serde_json::json!({"tool_choice": "none"}),
        ))
        .respond_with(sse_template(openai_completions_sse()))
        .expect(1)
        .mount(&peer)
        .await;

    let info = completions_info("port-model");
    let mut models = Models::new();
    models
        .register_chat_api(
            Arc::new(
                opi_ai::api::openai_completions::OpenAiCompletionsChatApi::new(
                    Arc::new(opi_ai::providers::openai::OpenAiChatProvider::for_route(
                        Some(peer.uri()),
                        "openai-chat-port".into(),
                        Default::default(),
                        vec![info.clone()],
                        Arc::new(HttpClient::new()),
                    )),
                    info.clone(),
                )
                .expect("completions api"),
            ),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("sk-port"),
            )),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // A foreign-wire native registration supplies its typed options.
    let pi_info = opi_ai::model_info::ModelInfo::new(
        "pi-port",
        "Pi Port",
        WireApi::PiMessages,
        opi_ai::registry::ModelCapabilities::new(100_000, 8_192),
    );
    let pi_handle = models
        .register_chat_api(
            Arc::new(
                opi_ai::api::pi_messages::PiMessagesChatApi::new(
                    opi_ai::providers::pi_messages::PiMessagesProvider::new(Some(
                        "http://127.0.0.1:1".into(),
                    )),
                    pi_info.clone(),
                )
                .expect("pi api"),
            ),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("sk-pi"),
            )),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    let record = ChatModelRecord::new("openai-chat-port", info.clone());
    let options = SimpleOptions {
        tool_choice: Some(SimpleToolChoice::None),
        temperature: None,
        max_tokens: None,
        timeout: None,
        stop_sequences: vec![],
    };
    let message = models
        .complete_simple(record.clone(), user_request("port-model", "Hello"), options)
        .await
        .expect("neutral tool_choice:none crosses wires");
    assert_eq!(message_text(&message), "Hello world");

    // Native options typed for the pi wire can never bind to the registered
    // openai-completions record: typed preflight failure before auth/network.
    let wrong = pi_handle.options(opi_ai::api::pi_messages::PiMessagesOptions::native(
        None, None, false,
    ));
    let result = models
        .complete(record, user_request("port-model", "Hello"), wrong)
        .await;
    assert!(result.is_err(), "mismatched native options must fail typed");
    peer.verify().await;
}

#[tokio::test]
async fn request_callbacks_observe_once_and_protected_headers_fail_typed() {
    let peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(wiremock::matchers::header("x-assembled", "matrix"))
        .respond_with(sse_template(openai_completions_sse()))
        .expect(1)
        .mount(&peer)
        .await;

    let info = completions_info("cb-model");
    let mut models = Models::new();
    let register = |models: &mut Models, base: String, info: opi_ai::model_info::ModelInfo| {
        models
            .register_chat_api(
                Arc::new(
                    opi_ai::api::openai_completions::OpenAiCompletionsChatApi::new(
                        Arc::new(opi_ai::providers::openai::OpenAiChatProvider::for_route(
                            Some(base),
                            "cb-provider".into(),
                            Default::default(),
                            vec![info.clone()],
                            Arc::new(HttpClient::new()),
                        )),
                        info,
                    )
                    .expect("cb api"),
                ),
                Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    AuthScheme::Bearer,
                    SecretString::from("sk-cb"),
                )),
                AuthProvenanceSource::Static,
            )
            .unwrap();
    };
    register(&mut models, peer.uri(), info.clone());

    let headers = Arc::new(AtomicUsize::new(0));
    let payloads = Arc::new(AtomicUsize::new(0));
    let responses = Arc::new(AtomicUsize::new(0));
    let raws = Arc::new(AtomicUsize::new(0));
    let callbacks = {
        let (h, p, r, w) = (
            headers.clone(),
            payloads.clone(),
            responses.clone(),
            raws.clone(),
        );
        RequestCallbacks::new()
            .with_header_transform(Arc::new(move |values: &[(String, String)]| {
                h.fetch_add(1, Ordering::SeqCst);
                let mut merged = values.to_vec();
                merged.push(("x-assembled".into(), "matrix".into()));
                Ok(merged)
            }))
            .with_on_payload(Arc::new(move |_body| {
                p.fetch_add(1, Ordering::SeqCst);
                Ok(None)
            }))
            .with_on_response(Arc::new(move |_status, _headers| {
                r.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }))
            .with_on_raw(Arc::new(move |_event| {
                w.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }))
    };
    let mut request = user_request("cb-model", "Hello");
    request.callbacks = callbacks;
    let message = models
        .complete_simple(
            ChatModelRecord::new("cb-provider", info.clone()),
            request,
            SimpleOptions::default(),
        )
        .await
        .expect("callback-observed call completes");
    assert_eq!(message_text(&message), "Hello world");
    assert_eq!(headers.load(Ordering::SeqCst), 1, "header transform once");
    assert_eq!(payloads.load(Ordering::SeqCst), 1, "payload observed once");
    assert_eq!(
        responses.load(Ordering::SeqCst),
        1,
        "response observed once"
    );
    assert!(
        raws.load(Ordering::SeqCst) >= 1,
        "raw provider events observed"
    );
    peer.verify().await;

    // A protected identity header change fails typed before the wire.
    let guard_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(sse_template(openai_completions_sse()))
        .mount(&guard_peer)
        .await;
    let mut guard_models = Models::new();
    register(&mut guard_models, guard_peer.uri(), info);
    let mut request = user_request("cb-model", "Hello");
    request.callbacks =
        RequestCallbacks::new().with_header_transform(Arc::new(|values: &[(String, String)]| {
            let mut merged = values.to_vec();
            merged.push(("authorization".into(), "Bearer override".into()));
            Ok(merged)
        }));
    let failure = guard_models
        .complete_simple(
            ChatModelRecord::new("cb-provider", completions_info("cb-model")),
            request,
            SimpleOptions::default(),
        )
        .await;
    assert!(
        failure.is_err(),
        "protected header mutation is typed failure"
    );
    assert!(
        guard_peer.received_requests().await.unwrap().is_empty(),
        "typed failure must precede the wire"
    );
}

#[tokio::test]
async fn duration_is_measured_once_drained_and_survives_serialization() {
    let peer = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(sse_template(openai_completions_sse()))
        .expect(1)
        .mount(&peer)
        .await;
    let info = completions_info("dur-model");
    let mut models = Models::new();
    models
        .register_chat_api(
            Arc::new(
                opi_ai::api::openai_completions::OpenAiCompletionsChatApi::new(
                    Arc::new(opi_ai::providers::openai::OpenAiChatProvider::for_route(
                        Some(peer.uri()),
                        "dur-provider".into(),
                        Default::default(),
                        vec![info.clone()],
                        Arc::new(HttpClient::new()),
                    )),
                    info,
                )
                .expect("dur api"),
            ),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("sk-dur"),
            )),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let message = models
        .complete_simple(
            ChatModelRecord::new("dur-provider", completions_info("dur-model")),
            user_request("dur-model", "Hello"),
            SimpleOptions::default(),
        )
        .await
        .expect("timed call");
    let measured = message
        .duration_ms
        .expect("adapter activation-to-terminal duration is observed");

    // Serialization retains the known optional duration; forwarding never
    // recomputes (a second identical roundtrip keeps the exact value).
    let encoded = serde_json::to_vec(&message).unwrap();
    let decoded: opi_ai::message::AssistantMessage = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded.duration_ms, Some(measured));
    let reencoded = serde_json::to_vec(&decoded).unwrap();
    let redecoded: opi_ai::message::AssistantMessage = serde_json::from_slice(&reencoded).unwrap();
    assert_eq!(redecoded.duration_ms, Some(measured));

    peer.verify().await;
}

#[tokio::test]
async fn manual_tool_loop_continues_after_serde_roundtrip() {
    let peer = MockServer::start().await;
    // Anthropic replays the unchanged tool set as by-value tool_addition on
    // every continuation round, so rounds are distinguished by body content,
    // not by tool presence.
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(|request: &wiremock::Request| {
            let body = String::from_utf8_lossy(&request.body);
            let sse = if body.contains("tool_result") {
                anthropic_text_sse()
            } else {
                anthropic_tool_call_sse()
            };
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse)
        })
        .expect(2)
        .mount(&peer)
        .await;

    let info = opi_ai::model_info::ModelInfo::new(
        "claude-sonnet-4-5-20250514",
        "Loop Model",
        WireApi::AnthropicMessages,
        opi_ai::registry::ModelCapabilities::new(200_000, 8_192),
    );
    let mut models = Models::new();
    models
        .register_route(
            Box::new(opi_ai::providers::anthropic::AnthropicProvider::for_route(
                "anthropic-loop".into(),
                vec![info.clone()],
                Some(peer.uri()),
                Default::default(),
                Arc::new(HttpClient::new()),
                false,
            )),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("sk-loop"),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();

    let weather = ToolDef {
        name: "get_weather".into(),
        description: "Look up the weather".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {"city": {"type": "string"}},
            "required": ["city"]
        }),
        constrained_sampling: None,
    };
    let shorthand = ShorthandContext {
        system_prompt: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "Weather in Paris?".into(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![weather],
    };
    let normalized = normalize_context(shorthand).unwrap();
    let normalized_for_loop = normalized.clone();

    let mut request = base_request("claude-sonnet-4-5-20250514");
    request.context = normalized;
    let first = models
        .complete_simple(
            ChatModelRecord::new("anthropic-loop", info.clone()),
            request,
            SimpleOptions::default(),
        )
        .await
        .expect("tool round");
    let tool_call = first
        .content
        .iter()
        .find_map(|block| match block {
            AssistantContent::ToolCall { tool_call } => Some(tool_call.clone()),
            _ => None,
        })
        .expect("assistant proposes the declared tool");
    assert_eq!(tool_call.name, "get_weather");

    // The caller is the trusted execution boundary: append the assistant
    // tool-call turn and an executed result, then serialize the whole
    // normalized transcript and rebuild it before continuing — the
    // model-visible chronology is serde-portable without losing tool
    // association.
    let mut messages = transcript::TranscriptContext::into_messages(normalized_for_loop);
    messages.push(Message::Assistant(first));
    messages.push(Message::ToolResult(ToolResultMessage {
        tool_call_id: tool_call.id.clone(),
        tool_name: tool_call.name.clone(),
        content: vec![OutputContent::Text { text: "18C".into() }],
        details: None,
        is_error: false,
        not_executed: None,
        truncated: false,
        timestamp_ms: 0,
    }));
    let rebuilt_shorthand = ShorthandContext {
        system_prompt: None,
        messages,
        tools: vec![],
    };
    let rebuilt = normalize_context(rebuilt_shorthand).unwrap();
    let encoded = serde_json::to_vec(&rebuilt).unwrap();
    let decoded: TranscriptContext = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(
        decoded.messages().len(),
        4,
        "system + user + assistant tool call + tool result survive serde"
    );

    let mut request = base_request("claude-sonnet-4-5-20250514");
    request.context = decoded;
    let second = models
        .complete_simple(
            ChatModelRecord::new("anthropic-loop", info),
            request,
            SimpleOptions::default(),
        )
        .await
        .expect("continuation round");
    assert_eq!(message_text(&second), "Hello world");
    peer.verify().await;
}

#[tokio::test]
async fn two_instances_share_a_store_but_keep_routes_isolated_and_clean_up() {
    let left_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(sse_template(openai_completions_sse()))
        .mount(&left_peer)
        .await;

    let store = Arc::new(opi_ai::mutation::InMemoryCredentialStore::new());

    let mut left = Models::new();
    left.set_credential_store(store.clone());
    left.register_route(
        Box::new(opi_ai::providers::openai::OpenAiChatProvider::for_route(
            Some(left_peer.uri()),
            "openai-chat".into(),
            Default::default(),
            vec![completions_info("iso-model")],
            Arc::new(HttpClient::new()),
        )),
        Arc::new(opi_ai::auth::StaticAuthResolver::new(
            AuthScheme::Bearer,
            SecretString::from("sk-left"),
        )),
        AuthProvenanceSource::Static,
        Default::default(),
    )
    .unwrap();

    let mut right = Models::new();
    right.set_credential_store(store.clone());
    right
        .register_route(
            Box::new(MockProvider::new_with_models(
                "iso-mock",
                vec![completions_info("mock-model")],
                vec![text_response("shared store twin")],
            )),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("sk-right"),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();

    let left_message = left
        .complete_simple(
            ChatModelRecord::new("openai-chat", completions_info("iso-model")),
            user_request("iso-model", "Hello"),
            SimpleOptions::default(),
        )
        .await
        .expect("left instance route");
    assert_eq!(message_text(&left_message), "Hello world");

    // Route isolation: registrations never leak across instances sharing a
    // credential store.
    assert!(right.get_model("openai-chat", "iso-model").is_none());
    assert!(left.get_model("iso-mock", "mock-model").is_none());

    // Availability is instance-scoped (AIR-046 management reads).
    let left_ids: Vec<String> = left
        .get_available(None)
        .await
        .unwrap()
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    let right_ids: Vec<String> = right
        .get_available(None)
        .await
        .unwrap()
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    assert!(left_ids.contains(&"openai-chat".to_string()));
    assert!(!left_ids.contains(&"iso-mock".to_string()));
    assert!(right_ids.contains(&"iso-mock".to_string()));
    assert!(!right_ids.contains(&"openai-chat".to_string()));

    left.cleanup_session_resources(None)
        .await
        .expect("left cleanup");
    right
        .cleanup_session_resources(None)
        .await
        .expect("right cleanup");
}

#[test]
fn constructors_own_default_memory_without_a_runtime() {
    assert!(tokio::runtime::Handle::try_current().is_err());
    for models in [
        Models::new(),
        Models::from_registry(opi_ai::ProviderRegistry::new()),
    ] {
        assert!(models.provider_ids().is_empty());
        let barrier = models.mutation_barrier().expect("constructor owns a store");
        assert_eq!(barrier.cutoff(), 0);
        let report = barrier.report();
        assert!(report.is_complete());
        assert!(report.captured.is_empty());
        assert!(report.late.is_empty());
    }
    assert!(tokio::runtime::Handle::try_current().is_err());
}

#[tokio::test]
async fn empty_models_registers_no_provider_and_static_reads_do_no_io() {
    let context = OwnedRecordingContext::new(&[]);
    let mut models = Models::new();
    models.set_auth_context(context.clone());

    // An empty Models is not advertised as populated.
    assert!(
        models
            .get_model("anthropic", "claude-sonnet-4-5-20250514")
            .is_none()
    );
    assert!(models.get_available(None).await.unwrap().is_empty());

    // Registration with an injected route and static queries perform zero
    // env/store/network I/O: the recording context sees no reads and the
    // provider call log stays empty.
    let mut models = Models::new();
    models.set_auth_context(context.clone());
    let provider = MockProvider::new("never", vec![text_response("unused")]);
    let calls = provider.call_log_handle();
    models
        .register_route(
            Box::new(provider),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("sk-empty"),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    assert_eq!(context.read_count(), 0);
    assert!(
        models.get_model("never", "mock-model").is_some(),
        "registration itself advertises the route's models"
    );
    let available: Vec<String> = models
        .get_available(None)
        .await
        .unwrap()
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    assert!(available.contains(&"never".to_string()));
    assert_eq!(
        context.read_count(),
        0,
        "static queries performed zero env I/O"
    );
    assert!(calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn lawful_collection_over_sixty_four_messages_is_accepted() {
    let peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(sse_template(openai_completions_sse()))
        .expect(1)
        .mount(&peer)
        .await;
    let info = opi_ai::model_info::ModelInfo::new(
        "bulk-model",
        "Bulk Model",
        WireApi::OpenAiCompletions,
        opi_ai::registry::ModelCapabilities::new(1_000_000, 8_192),
    );
    let mut models = Models::new();
    models
        .register_chat_api(
            Arc::new(
                opi_ai::api::openai_completions::OpenAiCompletionsChatApi::new(
                    Arc::new(opi_ai::providers::openai::OpenAiChatProvider::for_route(
                        Some(peer.uri()),
                        "bulk-provider".into(),
                        Default::default(),
                        vec![info.clone()],
                        Arc::new(HttpClient::new()),
                    )),
                    info,
                )
                .expect("bulk api"),
            ),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("sk-bulk"),
            )),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    let mut messages = Vec::new();
    for index in 0..70u32 {
        messages.push(Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: format!("turn {index}"),
            }],
            timestamp_ms: 0,
        }));
        messages.push(Message::Assistant(base_assistant()));
    }
    let context = normalize_context(ShorthandContext {
        system_prompt: Some("bulk chronology".into()),
        messages,
        tools: vec![],
    })
    .expect("70 turns normalize");
    let mut request = base_request("bulk-model");
    request.context = context;
    let message = models
        .complete_simple(
            ChatModelRecord::new("bulk-provider", completions_info("bulk-model")),
            request,
            SimpleOptions::default(),
        )
        .await
        .expect("large lawful acceptance");
    assert_eq!(message_text(&message), "Hello world");
    let sent = peer.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).expect("wire body decodes");
    let wire_messages = body["messages"].as_array().expect("messages array");
    assert_eq!(
        wire_messages.len(),
        71,
        "system + 70 merged user/assistant turns reach the wire"
    );
    assert!(
        serde_json::to_string(&body).unwrap().contains("turn 69"),
        "the latest lawful turn is not truncated"
    );
}

#[tokio::test]
async fn deferred_pending_then_explicit_fetch_serves_the_recorded_final() {
    // A scripted deferred producer returns a pending handle first; only the
    // caller's explicit fetch resumes it and serves the recorded final
    // exactly once — no automatic polling (P20-ACTOR-08).
    let backend = Arc::new(ScriptedDeferredBackend::new(
        "deferred-matrix",
        "mock-model",
    ));
    let mut models = Models::new();
    models
        .register_route(
            Box::new(
                MockProvider::new("deferred-matrix", vec![text_response("unused direct")])
                    .with_deferred(backend.clone(), 1, Some(60_000)),
            ),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("sk-defer"),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    models
        .register_deferred_api(
            backend.clone() as Arc<dyn opi_ai::api::contract::DeferredApi<()>>,
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("sk-defer"),
            )),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    let pending = models
        .complete_simple(
            ChatModelRecord::new("deferred-matrix", completions_info("defer-model")),
            user_request("mock-model", "Hello"),
            SimpleOptions::default(),
        )
        .await
        .expect("deferred round");
    assert_eq!(pending.stop_reason, opi_ai::stream::StopReason::Deferred);
    assert!(pending.deferred.is_some(), "pending handle returned");
    assert_eq!(backend.fetch_count(), 0, "no automatic polling");

    let handle = backend.submit(text_response("deferred done"), 0, None);
    let resumed = models
        .fetch_deferred(&handle, opi_ai::api::contract::DeferredFetchCall::default())
        .await
        .expect("explicit fetch serves the recorded final");
    assert_eq!(message_text(&resumed), "deferred done");
    assert_eq!(backend.fetch_count(), 1);
}
