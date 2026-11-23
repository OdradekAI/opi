//! Native and simple token limits on actual OpenAI Chat and Responses requests.

use std::sync::Arc;

use opi_ai::api::contract::{ChatApi, ChatApiHandle};
use opi_ai::auth::{AuthScheme, ResolvedAuth, StaticAuthResolver};
use opi_ai::message::{AssistantContent, AssistantMessage, Message};
use opi_ai::provider::{EventStream, Provider, Request};
use opi_ai::stream::{StopReason, Usage};
use opi_ai::transcript::{Context, SystemMessage, normalize_context};
use opi_ai::types::ApiId;
use opi_ai::{
    AuthProvenanceSource, ChatModelRecord, ModelCapabilities, ModelInfo, Models, WireApi,
};
use secrecy::SecretString;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::fixtures::{CHAT_OK, bodies, request};

const RESPONSES_OK: &str = "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"response-test\",\"model\":\"gpt-4o\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n";

struct RealWireApi {
    id: ApiId,
    provider: Arc<dyn Provider>,
    record: ModelInfo,
}

impl ChatApi<()> for RealWireApi {
    fn api_id(&self) -> &ApiId {
        &self.id
    }
    fn provider_id(&self) -> &str {
        self.provider.id()
    }
    fn complete_record(&self) -> ModelInfo {
        self.record.clone()
    }
    fn simple_options(&self) {}
    fn stream_prepared(&self, request: Request, (): (), auth: ResolvedAuth) -> EventStream {
        self.provider.stream_prepared(request, auth)
    }
}

async fn real_wire(
    wire: WireApi,
    context_window: u64,
    max_output: u64,
) -> (MockServer, Models, ChatApiHandle<()>, ChatModelRecord) {
    let server = MockServer::start().await;
    let (provider, sse): (Arc<dyn Provider>, _) = match wire {
        WireApi::OpenAiCompletions => (
            Arc::new(opi_ai::providers::openai::OpenAiChatProvider::new(Some(
                server.uri(),
            ))),
            CHAT_OK,
        ),
        WireApi::OpenAiResponses => (
            Arc::new(
                opi_ai::providers::openai_responses::OpenAiResponsesProvider::new(Some(
                    server.uri(),
                )),
            ),
            RESPONSES_OK,
        ),
        _ => unreachable!("the fixture only exposes the two migrated OpenAI wires"),
    };
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse),
        )
        .mount(&server)
        .await;
    let provider_id = provider.id().to_owned();
    let record = ModelInfo::new(
        "gpt-4o",
        "Test model",
        wire,
        ModelCapabilities::new(context_window, max_output).with_streaming(true),
    );
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            Arc::new(RealWireApi {
                id: ApiId::new(format!("test/{}", wire.as_str())),
                provider,
                record: record.clone(),
            }),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("fake-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    (
        server,
        models,
        handle,
        ChatModelRecord::new(provider_id, record),
    )
}

#[tokio::test]
async fn native_chat_omits_absent_max_while_simple_resolves_the_default() {
    let (server, models, api, record) = real_wire(WireApi::OpenAiCompletions, 10_000, 1_000).await;
    models
        .complete(record.clone(), request("openai:gpt-4o"), api.options(()))
        .await
        .unwrap();
    models
        .complete_simple(record, request("openai:gpt-4o"), Default::default())
        .await
        .unwrap();
    let sent = bodies(&server).await;
    assert!(sent[0].get("max_tokens").is_none());
    assert!(sent[0].get("max_completion_tokens").is_none());
    assert_eq!(sent[1]["max_completion_tokens"], 1_000);
}

#[tokio::test]
async fn native_chat_explicit_limit_is_not_context_clamped() {
    let (server, models, api, record) = real_wire(WireApi::OpenAiCompletions, 10_000, 1_000).await;
    let mut input = request("openai:gpt-4o");
    input.max_tokens = Some(20_000);
    models
        .complete(record, input, api.options(()))
        .await
        .unwrap();
    assert_eq!(bodies(&server).await[0]["max_completion_tokens"], 20_000);
}

#[tokio::test]
async fn native_responses_zero_omits_max_output_tokens() {
    let (server, models, api, record) = real_wire(WireApi::OpenAiResponses, 10_000, 1_000).await;
    let mut input = request("openai-responses:gpt-4o");
    input.max_tokens = Some(0);
    models
        .complete(record, input, api.options(()))
        .await
        .unwrap();
    assert!(bodies(&server).await[0].get("max_output_tokens").is_none());
}

#[tokio::test]
async fn native_responses_truthy_small_limit_uses_the_wire_floor() {
    let (server, models, api, record) = real_wire(WireApi::OpenAiResponses, 10_000, 1_000).await;
    for limit in [1, 15, 16, 17] {
        let mut input = request("openai-responses:gpt-4o");
        input.max_tokens = Some(limit);
        models
            .complete(record.clone(), input, api.options(()))
            .await
            .unwrap();
    }
    let sent = bodies(&server).await;
    assert_eq!(
        sent.iter()
            .map(|body| body["max_output_tokens"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![16, 16, 16, 17]
    );
}

#[tokio::test]
async fn simple_preparation_counts_canonical_system_changes_after_the_usage_anchor() {
    let (server, models, _api, record) = real_wire(WireApi::OpenAiCompletions, 10_000, 9_000).await;
    let mut input = request("openai:gpt-4o");
    input.max_tokens = Some(9_000);
    input.context = normalize_context(Context {
        system_prompt: None,
        messages: vec![
            Message::Assistant(AssistantMessage {
                projection: None,
                content: vec![AssistantContent::Text {
                    text: "done".into(),
                    signature: None,
                }],
                api: opi_ai::ApiKind::OpenAi,
                deferred: None,
                provider: "openai".into(),
                model: "gpt-4o".into(),
                response_model: None,
                wire: Some(WireApi::OpenAiCompletions),
                response_id: None,
                usage: Usage::reported(500, 20, 100, 50, None, None),
                stop_reason: StopReason::Stop,
                error_message: None,
                timestamp_ms: 2,
                diagnostics: vec![],
                duration_ms: None,
                raw_stop_reason: None,
                provider_thinking_level: None,
            }),
            Message::System(SystemMessage {
                content: "s".repeat(3_500),
                timestamp_ms: 3,
                ..Default::default()
            }),
        ],
        tools: vec![],
    })
    .unwrap();
    models
        .complete_simple(record, input, Default::default())
        .await
        .unwrap();
    // Positive usage 670 plus the later 3500 UTF-16 units / 3.5 = 1000.
    assert_eq!(bodies(&server).await[0]["max_completion_tokens"], 4_234);
}
