//! Request projection, dispatch ownership, and the timing of observable failures.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use opi_ai::api::contract::{ChatApi, ChatApiHandle};
use opi_ai::auth::{AuthScheme, ResolvedAuth, StaticAuthResolver};
use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
use opi_ai::message::{
    AssistantContent, AssistantMessage, InputContent, Message, ToolDef, UserMessage,
};
use opi_ai::model_info::{AnthropicMessagesCompat, WireCompat};
use opi_ai::provider::{EventStream, Provider, ProviderError, Request};
use opi_ai::stream::{ChatStream, StopReason, Usage};
use opi_ai::transcript::{Context, SystemMessage, normalize_context};
use opi_ai::types::ApiId;
use opi_ai::{
    AuthProvenanceSource, ChatModelRecord, ModelCapabilities, ModelInfo, Models, WireApi,
};
use secrecy::SecretString;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::fixtures::{CHAT_OK, bodies, request};

struct ForwardingAnthropic(opi_ai::providers::anthropic::AnthropicProvider);

impl Provider for ForwardingAnthropic {
    fn id(&self) -> &str {
        self.0.id()
    }
    fn models(&self) -> &[ModelInfo] {
        self.0.models()
    }
    fn stream_prepared(&self, request: Request, auth: ResolvedAuth) -> EventStream {
        self.0.stream_prepared(request, auth)
    }
}

#[tokio::test]
async fn forwarding_adapter_preserves_one_advanced_strict_and_retry_budget_owner() {
    let server = MockServer::start().await;
    for response in [
        ResponseTemplate::new(429).insert_header("retry-after", "0"),
        ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": {"type": "invalid_request_error", "message": "Too many strict tools (65)."}
        })),
        ResponseTemplate::new(429).insert_header("retry-after", "0"),
    ] {
        Mock::given(method("POST"))
            .respond_with(response)
            .up_to_n_times(1)
            .mount(&server)
            .await;
    }
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string("event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"model\":\"strict\",\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\nevent: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":1}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n"))
        .mount(&server).await;
    let model = ModelInfo::new(
        "strict",
        "Strict",
        WireApi::AnthropicMessages,
        ModelCapabilities::new(100_000, 1_000),
    )
    .with_compat(WireCompat::AnthropicMessages(AnthropicMessagesCompat {
        supports_strict_tools: true,
        supports_eager_tool_input_streaming: false,
        force_adaptive_thinking: false,
        supports_temperature: true,
        supports_mid_convo_system_messages: false,
        supports_mid_convo_tool_changes: false,
        ..Default::default()
    }))
    .unwrap();
    let provider = opi_ai::providers::anthropic::AnthropicProvider::for_route(
        "test".into(),
        vec![model],
        Some(server.uri()),
        Default::default(),
        Arc::new(opi_ai::http::HttpClient::new()),
        false,
    );
    let mut models = Models::new();
    models
        .register_route(
            Box::new(ForwardingAnthropic(provider)),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("fake-key"),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let mut input = request("test:strict");
    input.max_tokens = Some(64);
    input.context = normalize_context(Context {
        system_prompt: None, messages: vec![],
        tools: vec![ToolDef {
            name: "tool".into(), description: "d".into(),
            input_schema: serde_json::json!({"type":"object","properties":{"x":{"type":"string"}},"required":["x"]}),
            constrained_sampling: Some(ConstrainedSampling::JsonSchema { strict: StrictPreference::Prefer }),
        }],
    }).unwrap();
    let callbacks = Arc::new(AtomicUsize::new(0));
    let count = callbacks.clone();
    input.callbacks =
        opi_ai::callbacks::RequestCallbacks::new().with_on_payload(Arc::new(move |_| {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(None)
        }));
    let call = models
        .prepare_call_with_policy(
            "test:strict",
            input,
            opi_ai::retry::AcquisitionPolicy::with_max_retries(1),
        )
        .await
        .unwrap();
    let first = ChatStream::new(call.start_attempt().unwrap())
        .result()
        .await
        .unwrap_err();
    assert!(matches!(first.cause(), ProviderError::RateLimited { .. }));
    assert_eq!(
        first.message().projection.as_ref(),
        Some(&call.route().projection)
    );
    let admission = call.retry_admission().unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(admission.delay_ms)).await;
    let strict = ChatStream::new(call.start_attempt().unwrap())
        .result()
        .await
        .unwrap_err();
    assert!(matches!(
        strict.cause(),
        ProviderError::StrictToolLimits { .. }
    ));
    assert!(call.strict_variant_admission().is_some());
    assert_eq!(
        strict.message().projection.as_ref(),
        Some(&call.route().projection)
    );
    assert_eq!(
        bodies(&server).await.len(),
        2,
        "an inner driver cannot consume the strict branch"
    );
    let third = ChatStream::new(call.start_attempt().unwrap())
        .result()
        .await
        .unwrap_err();
    assert!(matches!(third.cause(), ProviderError::RateLimited { .. }));
    assert_eq!(
        third.message().projection.as_ref(),
        Some(&call.route().projection)
    );
    let admission = call.retry_admission().unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(admission.delay_ms)).await;
    ChatStream::new(call.start_attempt().unwrap())
        .result()
        .await
        .unwrap();
    let sent = bodies(&server).await;
    assert_eq!(sent.len(), 4);
    assert_eq!(sent[0]["tools"][0]["strict"], true);
    assert_eq!(sent[1]["tools"][0]["strict"], true);
    assert!(sent[2]["tools"][0].get("strict").is_none());
    assert!(sent[3]["tools"][0].get("strict").is_none());
    assert_eq!(callbacks.load(Ordering::SeqCst), 2);
}

struct ProjectionAuth {
    calls: AtomicUsize,
    fail: bool,
}
impl opi_ai::auth::AuthResolver for ProjectionAuth {
    fn resolve(
        &self,
    ) -> opi_ai::credential::BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            if self.fail {
                Err(ProviderError::AuthFailed(
                    opi_ai::provider::ProviderErrorSummary::sanitized("synthetic auth refusal"),
                ))
            } else {
                Ok(opi_ai::test_support::resolved_bearer_auth())
            }
        })
    }
}

fn projection_request(changed: bool) -> Request {
    let mut input = request("projection:model");
    let mut messages = vec![
        Message::System(SystemMessage {
            content: "initial policy".into(),
            timestamp_ms: 0,
            ..Default::default()
        }),
        Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "before".into(),
            }],
            timestamp_ms: 1,
        }),
    ];
    if changed {
        messages.push(Message::System(SystemMessage {
            content: "later policy".into(),
            sections: vec![opi_ai::transcript::SystemSection {
                name: "scope".into(),
                value: Some("current scope".into()),
            }],
            timestamp_ms: 2,
            ..Default::default()
        }));
        messages.push(Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "after".into(),
            }],
            timestamp_ms: 3,
        }));
    }
    input.context = normalize_context(Context {
        messages,
        ..Default::default()
    })
    .unwrap();
    input
}

type ProjectionApi = ChatApiHandle<opi_ai::api::openai_completions::OpenAiCompletionsOptions>;
async fn projection_wire(
    native: bool,
    fail_auth: bool,
) -> (
    MockServer,
    Models,
    ProjectionApi,
    ChatModelRecord,
    Arc<ProjectionAuth>,
    Arc<opi_ai::providers::openai::OpenAiChatProvider>,
) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(CHAT_OK),
        )
        .mount(&server)
        .await;
    let model = ModelInfo::new(
        "model",
        "Projection",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(20_000, 1000).with_streaming(true),
    )
    .with_compat(WireCompat::OpenAiCompletions(
        opi_ai::model_info::OpenAiCompletionsCompat {
            supports_mid_convo_system_messages: Some(native),
            supports_mid_convo_tool_additions: Some(native),
            ..Default::default()
        },
    ))
    .unwrap();
    let provider = Arc::new(opi_ai::providers::openai::OpenAiChatProvider::for_route(
        Some(server.uri()),
        "projection".into(),
        Default::default(),
        vec![model.clone()],
        Arc::new(opi_ai::http::HttpClient::new()),
    ));
    let auth = Arc::new(ProjectionAuth {
        calls: AtomicUsize::new(0),
        fail: fail_auth,
    });
    let mut models = Models::new();
    let api = models
        .register_chat_api(
            Arc::new(
                opi_ai::api::openai_completions::OpenAiCompletionsChatApi::new(
                    provider.clone(),
                    model.clone(),
                )
                .unwrap(),
            ),
            auth.clone(),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    models
        .register_route(
            Box::new((*provider).clone()),
            auth.clone(),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    (
        server,
        models,
        api,
        ChatModelRecord::new("projection", model),
        auth,
        provider,
    )
}

fn assert_projection_facts(
    input: &Request,
    facts: &opi_ai::transcript::ProjectionFacts,
    mode: opi_ai::transcript::ProjectionMode,
) {
    use sha2::Digest as _;
    assert_eq!(
        facts.source_prefix.message_count,
        input.context.messages().len()
    );
    assert_eq!(
        facts.source_prefix.sha256,
        format!(
            "{:x}",
            sha2::Sha256::digest(serde_json::to_vec(input.context.messages()).unwrap())
        )
    );
    assert_eq!(facts.temporal.mode, mode);
    assert_eq!(
        facts.temporal.affected_ranges,
        if mode == opi_ai::transcript::ProjectionMode::Collapse {
            vec![opi_ai::transcript::MessageRange { start: 0, end: 3 }]
        } else {
            vec![]
        }
    );
    let encoded = serde_json::to_string(facts).unwrap();
    for secret in [
        "initial policy",
        "later policy",
        "current scope",
        "fake-key",
    ] {
        assert!(!encoded.contains(secret));
    }
}

async fn collect_projection(
    mut stream: impl futures_core::Stream<
        Item = Result<opi_ai::stream::AssistantStreamEvent, opi_ai::stream::CallFailure>,
    > + Unpin,
    input: &Request,
    mode: opi_ai::transcript::ProjectionMode,
) -> AssistantMessage {
    use futures_util::StreamExt as _;
    let mut terminal = None;
    let mut events = 0;
    while let Some(event) = stream.next().await {
        let event = event.unwrap();
        events += 1;
        let encoded = serde_json::to_value(&event).unwrap();
        let message = encoded
            .get("partial")
            .or_else(|| encoded.get("message"))
            .unwrap();
        let facts: opi_ai::transcript::ProjectionFacts =
            serde_json::from_value(message["projection"].clone()).unwrap();
        assert_projection_facts(input, &facts, mode);
        if let opi_ai::stream::AssistantStreamEvent::Done { message, .. } = event {
            terminal = Some(message);
        }
    }
    assert!(events >= 3, "Start, partial, terminal are all observed");
    let message = terminal.unwrap();
    assert_eq!(
        serde_json::from_slice::<AssistantMessage>(&serde_json::to_vec(&message).unwrap()).unwrap(),
        message
    );
    message
}

#[tokio::test]
async fn projection_policy_and_facts_follow_actual_native_simple_direct_and_prepared_calls() {
    use opi_ai::transcript::ProjectionMode;
    for (native, changed, mode) in [
        (false, false, ProjectionMode::Lossless),
        (false, true, ProjectionMode::Collapse),
        (true, true, ProjectionMode::Native),
    ] {
        let (server, models, api, record, auth, provider) = projection_wire(native, false).await;
        let mut input = projection_request(changed);
        if native {
            input.projection.temporal = opi_ai::transcript::TranscriptProjection::Strict;
        }
        let canonical = serde_json::to_vec(&input.context).unwrap();
        collect_projection(
            models.stream(
                record.clone(),
                input.clone(),
                api.options(Default::default()),
            ),
            &input,
            mode,
        )
        .await;
        collect_projection(
            models.stream_simple(record.clone(), input.clone(), Default::default()),
            &input,
            mode,
        )
        .await;
        collect_projection(
            models.stream_direct(
                record.clone(),
                input.clone(),
                api.options(Default::default()),
                opi_ai::test_support::resolved_bearer_auth(),
            ),
            &input,
            mode,
        )
        .await;
        collect_projection(
            ChatStream::new(
                provider
                    .stream_prepared(input.clone(), opi_ai::test_support::resolved_bearer_auth()),
            ),
            &input,
            mode,
        )
        .await;
        let prepared = models
            .prepare_call("projection:model", input.clone())
            .await
            .unwrap();
        assert_projection_facts(&input, &prepared.route().projection, mode);
        let message = collect_projection(
            ChatStream::new(prepared.start_attempt().unwrap()),
            &input,
            mode,
        )
        .await;
        assert_eq!(
            message.projection.as_ref(),
            Some(&prepared.route().projection)
        );
        assert_eq!(serde_json::to_vec(&input.context).unwrap(), canonical);
        assert_eq!(auth.calls.load(Ordering::SeqCst), 3);
        let sent = bodies(&server).await;
        assert_eq!(sent.len(), 5);
        if changed && native {
            assert_eq!(sent[0]["messages"].as_array().unwrap().len(), 4);
            assert_eq!(sent[0]["messages"][2]["role"], "system");
        } else if changed {
            assert_eq!(sent[0]["messages"].as_array().unwrap().len(), 3);
        }
    }
}

#[tokio::test]
async fn strict_projection_rejects_before_auth_callbacks_and_network() {
    let (server, models, api, record, auth, provider) = projection_wire(false, false).await;
    let mut input = projection_request(true);
    input.projection.temporal = opi_ai::transcript::TranscriptProjection::Strict;
    assert!(matches!(
        opi_ai::transcript::resolve_transcript(&input.context, false, input.projection.temporal),
        Err(
            opi_ai::transcript::TranscriptError::UnsupportedTemporalFidelity {
                mid_convo_system_messages: 1
            }
        )
    ));
    let callbacks = Arc::new(AtomicUsize::new(0));
    let count = callbacks.clone();
    input.callbacks =
        opi_ai::callbacks::RequestCallbacks::new().with_on_payload(Arc::new(move |_| {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(None)
        }));
    for failure in [
        models
            .complete(
                record.clone(),
                input.clone(),
                api.options(Default::default()),
            )
            .await
            .unwrap_err(),
        models
            .complete_simple(record.clone(), input.clone(), Default::default())
            .await
            .unwrap_err(),
        models
            .complete_direct(
                record,
                input.clone(),
                api.options(Default::default()),
                opi_ai::test_support::resolved_bearer_auth(),
            )
            .await
            .unwrap_err(),
        ChatStream::new(
            provider.stream_prepared(input.clone(), opi_ai::test_support::resolved_bearer_auth()),
        )
        .result()
        .await
        .unwrap_err(),
    ] {
        assert!(matches!(failure.cause(), ProviderError::Config(_)));
        assert!(
            failure.message().projection.is_none(),
            "rejected projection was never accepted"
        );
    }
    assert!(matches!(
        models.prepare_call("projection:model", input).await,
        Err(opi_ai::models::ModelsError::Provider(
            ProviderError::Config(_)
        ))
    ));
    assert_eq!(auth.calls.load(Ordering::SeqCst), 0);
    assert_eq!(callbacks.load(Ordering::SeqCst), 0);
    assert!(bodies(&server).await.is_empty());
}

#[tokio::test]
async fn accepted_projection_survives_auth_callback_partial_eof_and_cancel_failures() {
    use opi_ai::transcript::ProjectionMode;
    let (server, models, api, record, auth, _) = projection_wire(false, true).await;
    let input = projection_request(true);
    let failure = models
        .complete(record, input.clone(), api.options(Default::default()))
        .await
        .unwrap_err();
    assert!(matches!(failure.cause(), ProviderError::AuthFailed(_)));
    assert_projection_facts(
        &input,
        failure.message().projection.as_ref().unwrap(),
        ProjectionMode::Collapse,
    );
    assert_eq!(auth.calls.load(Ordering::SeqCst), 1);
    assert!(bodies(&server).await.is_empty());
    for phase in ["payload", "response", "partial", "eof", "cancel"] {
        let (server, _, _, _, _, provider) = projection_wire(false, false).await;
        server.reset().await;
        let body = if phase == "partial" {
            "data: {\"id\":\"partial\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"partial text\"},\"finish_reason\":null}]}\n\ndata: {invalid}\n\n"
        } else if phase == "eof" {
            ""
        } else {
            CHAT_OK
        };
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(body),
            )
            .mount(&server)
            .await;
        let mut input = projection_request(true);
        if phase == "payload" {
            input.callbacks =
                opi_ai::callbacks::RequestCallbacks::new().with_on_payload(Arc::new(|_| {
                    Err(opi_ai::callbacks::CallbackError::new("synthetic"))
                }));
        }
        if phase == "response" {
            input.callbacks =
                opi_ai::callbacks::RequestCallbacks::new().with_on_response(Arc::new(|_, _| {
                    Err(opi_ai::callbacks::CallbackError::new("synthetic"))
                }));
        }
        if phase == "cancel" {
            let cancel = input.cancel.clone();
            input.callbacks =
                opi_ai::callbacks::RequestCallbacks::new().with_on_payload(Arc::new(move |_| {
                    cancel.cancel();
                    Ok(None)
                }));
        }
        let failure = ChatStream::new(
            provider.stream_prepared(input.clone(), opi_ai::test_support::resolved_bearer_auth()),
        )
        .result()
        .await
        .unwrap_err();
        assert_projection_facts(
            &input,
            failure.message().projection.as_ref().unwrap(),
            ProjectionMode::Collapse,
        );
        if phase == "partial" {
            assert!(failure.message().content.iter().any(|block| matches!(block, AssistantContent::Text { text, .. } if text == "partial text")));
        }
        if phase == "cancel" {
            assert!(matches!(failure.cause(), ProviderError::Cancelled));
        }
        assert_eq!(
            bodies(&server).await.len(),
            usize::from(!matches!(phase, "payload" | "cancel")),
            "{phase}"
        );
    }
}

fn projection_tool(name: &str) -> ToolDef {
    ToolDef {
        name: name.into(),
        description: name.into(),
        input_schema: serde_json::json!({"type":"object"}),
        constrained_sampling: None,
    }
}

#[tokio::test]
async fn effective_tool_noops_do_not_disable_native_additions_or_change_the_canonical_prefix() {
    use opi_ai::transcript::{ProjectionMode, ToolReference};
    let (server, models, api, record, auth, _) = projection_wire(true, false).await;
    for add in [false, true] {
        let mut input = projection_request(false);
        let mut messages = input.context.into_messages();
        let Message::System(head) = &mut messages[0] else {
            unreachable!()
        };
        head.tools_added = vec![projection_tool("initial")];
        input.context = normalize_context(Context {
            messages,
            ..Default::default()
        })
        .unwrap();
        if add {
            input
                .context
                .append_system_change(SystemMessage {
                    tools_added: vec![projection_tool("later")],
                    timestamp_ms: 2,
                    ..Default::default()
                })
                .unwrap();
        }
        input
            .context
            .append_system_change(SystemMessage {
                tools_added: vec![projection_tool(if add { "later" } else { "initial" })],
                tools_removed: vec![ToolReference {
                    name: "unknown".into(),
                }],
                timestamp_ms: 3,
                ..Default::default()
            })
            .unwrap();
        assert!(
            opi_ai::transcript::has_non_additive_tool_changes(input.context.messages()),
            "the raw-history helper retains its existing contract"
        );
        input.projection.temporal = opi_ai::transcript::TranscriptProjection::Strict;
        let canonical = serde_json::to_vec(&input.context).unwrap();
        let message = models
            .complete(
                record.clone(),
                input.clone(),
                api.options(Default::default()),
            )
            .await
            .unwrap();
        assert_projection_facts(
            &input,
            message.projection.as_ref().unwrap(),
            if add {
                ProjectionMode::Native
            } else {
                ProjectionMode::Lossless
            },
        );
        assert_eq!(serde_json::to_vec(&input.context).unwrap(), canonical);
        let body = bodies(&server).await.pop().unwrap();
        assert_eq!(body["tools"].as_array().unwrap().len(), 1);
        let additions: Vec<_> = body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|message| message.get("tools").is_some())
            .collect();
        assert_eq!(additions.len(), usize::from(add));
        if add {
            assert_eq!(additions[0]["tools"][0]["function"]["name"], "later");
        }
        // A genuine removal remains unsupported by the same AddOnly route.
        input
            .context
            .append_system_change(SystemMessage {
                tools_removed: vec![ToolReference {
                    name: "initial".into(),
                }],
                timestamp_ms: 4,
                ..Default::default()
            })
            .unwrap();
        let count = auth.calls.load(Ordering::SeqCst);
        let failure = models
            .complete(record.clone(), input, api.options(Default::default()))
            .await
            .unwrap_err();
        assert!(matches!(failure.cause(), ProviderError::Config(_)));
        assert!(failure.message().projection.is_none());
        assert_eq!(auth.calls.load(Ordering::SeqCst), count);
    }
    assert_eq!(auth.calls.load(Ordering::SeqCst), 2);
    assert_eq!(bodies(&server).await.len(), 2);
}

#[tokio::test]
async fn unknown_direct_models_validate_strict_fidelity_on_the_actual_wire_before_callbacks() {
    let server = MockServer::start().await;
    let providers: Vec<Box<dyn Provider>> = vec![
        Box::new(opi_ai::providers::openai::OpenAiChatProvider::new(Some(
            server.uri(),
        ))),
        Box::new(
            opi_ai::providers::openai_responses::OpenAiResponsesProvider::new(Some(server.uri())),
        ),
        Box::new(opi_ai::providers::gemini::GeminiProvider::new(Some(
            server.uri(),
        ))),
    ];
    let callbacks = Arc::new(AtomicUsize::new(0));
    for provider in providers {
        let mut input = projection_request(true);
        input.model = format!("{}:unlisted-deployment", provider.id());
        input.projection.temporal = opi_ai::transcript::TranscriptProjection::Strict;
        let count = callbacks.clone();
        input.callbacks =
            opi_ai::callbacks::RequestCallbacks::new().with_on_payload(Arc::new(move |_| {
                count.fetch_add(1, Ordering::SeqCst);
                Ok(None)
            }));
        let failure = ChatStream::new(
            provider.stream_prepared(input, opi_ai::test_support::resolved_bearer_auth()),
        )
        .result()
        .await
        .unwrap_err();
        assert!(matches!(failure.cause(), ProviderError::Config(_)));
        assert!(failure.message().projection.is_none());
    }
    assert_eq!(callbacks.load(Ordering::SeqCst), 0);
    assert!(bodies(&server).await.is_empty());
}

#[tokio::test]
async fn collapsed_projection_ranges_use_original_coordinates_after_an_earlier_noop() {
    use opi_ai::transcript::{ProjectionMode, ToolReference};
    use sha2::Digest as _;
    let (server, models, api, record, auth, _) = projection_wire(false, false).await;
    let mut input = projection_request(false);
    input
        .context
        .append_system_change(SystemMessage {
            tools_removed: vec![ToolReference {
                name: "unknown".into(),
            }],
            timestamp_ms: 2,
            ..Default::default()
        })
        .unwrap();
    input
        .context
        .append_system_change(SystemMessage {
            content: "real later change".into(),
            timestamp_ms: 3,
            ..Default::default()
        })
        .unwrap();
    let canonical = serde_json::to_vec(input.context.messages()).unwrap();
    let message = models
        .complete(
            record.clone(),
            input.clone(),
            api.options(Default::default()),
        )
        .await
        .unwrap();
    let facts = message.projection.unwrap();
    assert_eq!(facts.source_prefix.message_count, 4);
    assert_eq!(
        facts.source_prefix.sha256,
        format!("{:x}", sha2::Sha256::digest(&canonical))
    );
    assert_eq!(facts.temporal.mode, ProjectionMode::Collapse);
    assert_eq!(
        facts.temporal.affected_ranges,
        vec![opi_ai::transcript::MessageRange { start: 0, end: 4 }]
    );
    assert_eq!(
        serde_json::to_vec(input.context.messages()).unwrap(),
        canonical
    );
    assert_eq!(bodies(&server).await.len(), 1);
    input.projection.temporal = opi_ai::transcript::TranscriptProjection::Strict;
    let failure = models
        .complete(record, input, api.options(Default::default()))
        .await
        .unwrap_err();
    assert!(matches!(failure.cause(), ProviderError::Config(_)));
    assert_eq!(auth.calls.load(Ordering::SeqCst), 1);
    assert_eq!(bodies(&server).await.len(), 1);
}

#[tokio::test]
async fn replay_strict_is_independent_and_rejects_before_auth_without_copying_thinking_into_facts()
{
    use opi_ai::replay::{ReplayError, ReplayLoss, ReplayProjection, ReplayRoute};
    let (server, models, api, record, auth, provider) = projection_wire(false, false).await;
    let mut input = projection_request(false);
    let mut messages = input.context.into_messages();
    messages.push(Message::Assistant(AssistantMessage {
        projection: None,
        content: vec![AssistantContent::Thinking {
            thinking: "private thinking bytes".into(),
            signature: Some("opaque private signature".into()),
        }],
        api: opi_ai::ApiKind::OpenAi,
        provider: "different".into(),
        model: "different".into(),
        response_model: None,
        wire: Some(WireApi::OpenAiCompletions),
        response_id: None,
        usage: Usage::unknown(),
        stop_reason: StopReason::Stop,
        raw_stop_reason: None,
        error_message: None,
        timestamp_ms: 2,
        diagnostics: vec![],
        duration_ms: None,
        provider_thinking_level: None,
        deferred: None,
    }));
    input.context = normalize_context(Context {
        messages,
        ..Default::default()
    })
    .unwrap();
    let canonical = serde_json::to_vec(&input.context).unwrap();
    let message = models
        .complete(
            record.clone(),
            input.clone(),
            api.options(Default::default()),
        )
        .await
        .unwrap();
    let facts = message.projection.unwrap();
    assert_projection_facts(&input, &facts, opi_ai::transcript::ProjectionMode::Lossless);
    assert_eq!(
        facts.replay.losses,
        vec![ReplayLoss::DroppedThinkingBlocks { count: 1 }]
    );
    let encoded = serde_json::to_string(&facts).unwrap();
    assert!(!encoded.contains("private thinking bytes"));
    assert!(!encoded.contains("opaque private signature"));
    input.projection.replay = ReplayProjection::Strict;
    assert!(matches!(
        opi_ai::replay::project_replay(
            input.context.messages(),
            &ReplayRoute {
                wire: WireApi::OpenAiCompletions,
                provider: "projection".into(),
                model: "model".into(),
                supports_images: false
            },
            ReplayProjection::Strict
        ),
        Err(ReplayError::StrictLoss(_))
    ));
    for failure in [
        models
            .complete(
                record.clone(),
                input.clone(),
                api.options(Default::default()),
            )
            .await
            .unwrap_err(),
        models
            .complete_simple(record.clone(), input.clone(), Default::default())
            .await
            .unwrap_err(),
        models
            .complete_direct(
                record,
                input.clone(),
                api.options(Default::default()),
                opi_ai::test_support::resolved_bearer_auth(),
            )
            .await
            .unwrap_err(),
        ChatStream::new(
            provider.stream_prepared(input.clone(), opi_ai::test_support::resolved_bearer_auth()),
        )
        .result()
        .await
        .unwrap_err(),
    ] {
        assert!(matches!(failure.cause(), ProviderError::Config(_)));
        assert!(failure.message().projection.is_none());
    }
    assert!(matches!(
        models.prepare_call("projection:model", input.clone()).await,
        Err(opi_ai::models::ModelsError::Provider(
            ProviderError::Config(_)
        ))
    ));
    assert_eq!(auth.calls.load(Ordering::SeqCst), 1);
    assert_eq!(bodies(&server).await.len(), 1);
    assert_eq!(serde_json::to_vec(&input.context).unwrap(), canonical);
}

struct RawProjectionApi {
    id: ApiId,
    eof: bool,
}
impl ChatApi<()> for RawProjectionApi {
    fn api_id(&self) -> &ApiId {
        &self.id
    }
    fn provider_id(&self) -> &str {
        "raw-projection"
    }
    fn complete_record(&self) -> ModelInfo {
        ModelInfo::new(
            "model",
            "Raw",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(4096, 1024),
        )
    }
    fn simple_options(&self) {}
    fn stream_prepared(&self, _: Request, _: (), _: ResolvedAuth) -> EventStream {
        let mut start = opi_ai::test_support::base_assistant();
        start.provider = "raw-projection".into();
        start.model = "model".into();
        start.wire = Some(WireApi::OpenAiCompletions);
        let mut latest = start.clone();
        latest.content = vec![AssistantContent::Text {
            text: "latest delivered".into(),
            signature: None,
        }];
        latest.usage.input_tokens = 7;
        latest.usage.output_tokens = 3;
        latest.usage.reported = true;
        latest.duration_ms = Some(17);
        let mut items = vec![
            Ok(opi_ai::stream::AssistantStreamEvent::Start { partial: start }),
            Ok(opi_ai::stream::AssistantStreamEvent::TextDelta {
                content_index: 0,
                delta: "latest delivered".into(),
                partial: latest,
            }),
        ];
        if !self.eof {
            items.push(Err(ProviderError::StreamError(
                opi_ai::provider::ProviderErrorSummary::sanitized("synthetic raw failure"),
            )));
        }
        Box::pin(futures_util::stream::iter(items))
    }
}
async fn check_custom_projection_failure(eof: bool) {
    let mut models = Models::new();
    let api = RawProjectionApi {
        id: ApiId::new("raw-projection"),
        eof,
    };
    let record = ChatModelRecord::new("raw-projection", api.complete_record());
    let handle = models
        .register_chat_api(
            Arc::new(api),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("synthetic"),
            )),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let input = projection_request(true);
    let failure = models
        .complete(record, input.clone(), handle.options(()))
        .await
        .unwrap_err();
    assert!(matches!(failure.cause(), ProviderError::StreamError(_)));
    assert!(
        matches!(&failure.message().content[..], [AssistantContent::Text { text, .. }] if text=="latest delivered")
    );
    assert_eq!(failure.message().usage.input_tokens, 7);
    assert_eq!(failure.message().usage.output_tokens, 3);
    assert_eq!(failure.message().duration_ms, Some(17));
    assert_projection_facts(
        &input,
        failure.message().projection.as_ref().unwrap(),
        opi_ai::transcript::ProjectionMode::Collapse,
    );
}
#[tokio::test]
async fn custom_projection_raw_failure_preserves_the_latest_delivered_partial() {
    check_custom_projection_failure(false).await
}
#[tokio::test]
async fn custom_projection_eof_preserves_the_latest_delivered_partial() {
    check_custom_projection_failure(true).await
}

fn temporal_cause_count(error: &(dyn std::error::Error + 'static)) -> Option<usize> {
    let mut current = Some(error);
    while let Some(error) = current {
        if let Some(opi_ai::transcript::TranscriptError::UnsupportedTemporalFidelity {
            mid_convo_system_messages,
        }) = error.downcast_ref::<opi_ai::transcript::TranscriptError>()
        {
            return Some(*mid_convo_system_messages);
        }
        current = error.source();
    }
    None
}

#[tokio::test]
async fn typed_temporal_cause_is_recoverable_from_every_real_entry() {
    let (server, models, api, record, auth, provider) = projection_wire(false, false).await;
    let mut input = projection_request(true);
    input.projection.temporal = opi_ai::transcript::TranscriptProjection::Strict;
    for failure in [
        models
            .complete(
                record.clone(),
                input.clone(),
                api.options(Default::default()),
            )
            .await
            .unwrap_err(),
        models
            .complete_simple(record.clone(), input.clone(), Default::default())
            .await
            .unwrap_err(),
        models
            .complete_direct(
                record,
                input.clone(),
                api.options(Default::default()),
                opi_ai::test_support::resolved_bearer_auth(),
            )
            .await
            .unwrap_err(),
        ChatStream::new(
            provider.stream_prepared(input.clone(), opi_ai::test_support::resolved_bearer_auth()),
        )
        .result()
        .await
        .unwrap_err(),
    ] {
        assert!(matches!(failure.cause(), ProviderError::Config(_)));
        assert_eq!(temporal_cause_count(failure.cause()), Some(1));
        assert!(failure.message().projection.is_none());
    }
    let error = models
        .prepare_call("projection:model", input)
        .await
        .unwrap_err();
    assert_eq!(temporal_cause_count(&error), Some(1));
    assert_eq!(auth.calls.load(Ordering::SeqCst), 0);
    assert!(bodies(&server).await.is_empty());
}

fn advanced_projection_carrier(
    error: opi_ai::ModelsError,
) -> Box<opi_ai::provider::ProviderStreamFailure> {
    match error {
        opi_ai::ModelsError::Provider(ProviderError::StreamFailure(failure)) => failure,
        other => panic!("accepted projection lost its recoverable carrier: {other:?}"),
    }
}

#[tokio::test]
async fn advanced_projection_auth_refusal_retains_formed_facts() {
    let (server, models, _, _, auth, _) = projection_wire(false, true).await;
    let input = projection_request(true);
    let failure = advanced_projection_carrier(
        models
            .prepare_call("projection:model", input.clone())
            .await
            .unwrap_err(),
    );
    assert!(matches!(failure.cause(), ProviderError::AuthFailed(_)));
    assert_projection_facts(
        &input,
        failure.message().projection.as_ref().unwrap(),
        opi_ai::transcript::ProjectionMode::Collapse,
    );
    assert_eq!(auth.calls.load(Ordering::SeqCst), 1);
    assert!(bodies(&server).await.is_empty());
}

struct FinalizationRefusal {
    provider: Arc<opi_ai::providers::openai::OpenAiChatProvider>,
    calls: Arc<AtomicUsize>,
}
impl Provider for FinalizationRefusal {
    fn id(&self) -> &str {
        self.provider.id()
    }
    fn models(&self) -> &[ModelInfo] {
        self.provider.models()
    }
    fn stream_prepared(&self, request: Request, auth: ResolvedAuth) -> EventStream {
        self.provider.stream_prepared(request, auth)
    }
    fn prepare_model_call(
        &self,
        _: &mut ModelInfo,
        _: &mut ResolvedAuth,
    ) -> Result<Option<Arc<dyn Provider>>, ProviderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(ProviderError::Config(
            opi_ai::provider::ProviderErrorSummary::sanitized("synthetic finalization refusal"),
        ))
    }
}
#[tokio::test]
async fn advanced_projection_finalization_refusal_retains_formed_facts() {
    let (server, mut models, _, _, auth, provider) = projection_wire(false, false).await;
    let calls = Arc::new(AtomicUsize::new(0));
    models
        .register_route(
            Box::new(FinalizationRefusal {
                provider,
                calls: calls.clone(),
            }),
            auth.clone(),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let input = projection_request(true);
    let failure = advanced_projection_carrier(
        models
            .prepare_call("projection:model", input.clone())
            .await
            .unwrap_err(),
    );
    assert!(matches!(failure.cause(), ProviderError::Config(_)));
    assert_projection_facts(
        &input,
        failure.message().projection.as_ref().unwrap(),
        opi_ai::transcript::ProjectionMode::Collapse,
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(auth.calls.load(Ordering::SeqCst), 1);
    assert!(bodies(&server).await.is_empty());
}

struct PendingProjectionAuth {
    started: Arc<tokio::sync::Notify>,
    calls: AtomicUsize,
}
impl opi_ai::auth::AuthResolver for PendingProjectionAuth {
    fn resolve(
        &self,
    ) -> opi_ai::credential::BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            self.started.notify_one();
            std::future::pending().await
        })
    }
}
#[tokio::test]
async fn advanced_projection_auth_cancellation_retains_formed_facts_but_pre_cancel_remains_unformed()
 {
    let (server, mut models, _, _, _, provider) = projection_wire(false, false).await;
    let started = Arc::new(tokio::sync::Notify::new());
    let auth = Arc::new(PendingProjectionAuth {
        started: started.clone(),
        calls: AtomicUsize::new(0),
    });
    models
        .register_route(
            Box::new((*provider).clone()),
            auth.clone(),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let input = projection_request(true);
    let cancel = input.cancel.clone();
    let future = models.prepare_call("projection:model", input.clone());
    tokio::pin!(future);
    tokio::select! { _=started.notified()=>{}, result=&mut future=>panic!("auth should be pending: {result:?}") }
    cancel.cancel();
    let error = tokio::time::timeout(std::time::Duration::from_secs(1), future)
        .await
        .unwrap()
        .unwrap_err();
    assert!(error.is_call_cancelled());
    let failure = advanced_projection_carrier(error);
    assert!(matches!(failure.cause(), ProviderError::Cancelled));
    assert_projection_facts(
        &input,
        failure.message().projection.as_ref().unwrap(),
        opi_ai::transcript::ProjectionMode::Collapse,
    );
    let pre = projection_request(true);
    pre.cancel.cancel();
    assert!(matches!(
        models.prepare_call("projection:model", pre).await,
        Err(opi_ai::ModelsError::CallCancelled)
    ));
    assert_eq!(auth.calls.load(Ordering::SeqCst), 1);
    assert!(bodies(&server).await.is_empty());
}

struct RefusingProjectionAuth(ProviderError);
impl opi_ai::auth::AuthResolver for RefusingProjectionAuth {
    fn resolve(
        &self,
    ) -> opi_ai::credential::BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
        Box::pin(async { Err(self.0.clone()) })
    }
}
#[tokio::test]
async fn resolver_cancellation_and_login_refusal_do_not_fabricate_logical_call_cancellation() {
    let (server, mut models, _, _, _, provider) = projection_wire(false, false).await;
    for cause in [
        ProviderError::Cancelled,
        ProviderError::LoginCancelled {
            provider_id: "projection".into(),
        },
        ProviderError::AuthFailed(
            opi_ai::provider::ProviderErrorSummary::authentication_rejected(),
        ),
    ] {
        models
            .register_route(
                Box::new((*provider).clone()),
                Arc::new(RefusingProjectionAuth(cause.clone())),
                AuthProvenanceSource::Static,
                Default::default(),
            )
            .unwrap();
        let input = projection_request(true);
        let error = models
            .prepare_call("projection:model", input.clone())
            .await
            .unwrap_err();
        assert!(!error.is_call_cancelled());
        input.cancel.cancel();
        assert!(
            !error.is_call_cancelled(),
            "origin does not depend on a subsequently cancelled caller token"
        );
        let failure = advanced_projection_carrier(error);
        assert_eq!(failure.cause(), &cause);
        assert_projection_facts(
            &input,
            failure.message().projection.as_ref().unwrap(),
            opi_ai::transcript::ProjectionMode::Collapse,
        );
    }
    assert!(bodies(&server).await.is_empty());
}
