//! Actual Anthropic -> Agent terminal accounting -> session disk recovery.
use opi_agent::authority::ToolRegistry;
use opi_agent::event::AgentEvent;
use opi_agent::hooks::AgentHooks;
use opi_agent::loop_types::{
    AgentError, AgentLoopConfig, AgentLoopContext, InferenceConfig, ModelSelection, NextTurnState,
};
use opi_agent::message::AgentMessage;
use opi_agent::session::SessionReader;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::model_info::{ModelInfo, ModelPricing, WireApi};
use opi_ai::models::CompatMetadata;
use opi_ai::providers::anthropic::AnthropicProvider;
use opi_ai::registry::ModelCapabilities;
use opi_ai::stream::{Pricing, Usage};
use opi_ai::{AuthProvenanceSource, AuthScheme, Models, StaticAuthResolver};
use opi_coding_agent::session_coordinator::SessionCoordinator;
use secrecy::SecretString;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

struct Hooks;
impl AgentHooks for Hooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        Ok(messages
            .iter()
            .filter_map(|m| match m {
                AgentMessage::Llm(m) => Some(m.clone()),
                _ => None,
            })
            .collect())
    }
}
fn usage(messages: &[AgentMessage]) -> Usage {
    messages
        .iter()
        .rev()
        .find_map(|m| match m {
            AgentMessage::Llm(Message::Assistant(a)) => Some(a.usage.clone()),
            _ => None,
        })
        .unwrap()
}
async fn check(failure: bool) {
    let server = MockServer::start().await;
    let frames = [
        json!({"type":"message_start","message":{"id":"usage-response","model":"usage-model","usage":{"input_tokens":11,"output_tokens":7,"cache_read_input_tokens":5,"cache_creation_input_tokens":3,"cache_creation":{"ephemeral_1h_input_tokens":2},"output_tokens_details":{"thinking_tokens":4}}}}),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"answer"}}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"input_tokens":0,"output_tokens":2,"cache_read_input_tokens":0,"cache_creation_input_tokens":0,"cache_creation":{"ephemeral_1h_input_tokens":0},"output_tokens_details":{"thinking_tokens":1}}}),
        if failure {
            json!({"type":"error","error":{"type":"overloaded_error","message":"fixture"}})
        } else {
            json!({"type":"message_stop"})
        },
    ];
    let sse = frames
        .iter()
        .map(|f| format!("event: {}\ndata: {f}\n\n", f["type"].as_str().unwrap()))
        .collect::<String>();
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse),
        )
        .expect(1)
        .mount(&server)
        .await;
    let info = ModelInfo::new(
        "usage-model",
        "Fixture",
        WireApi::AnthropicMessages,
        ModelCapabilities::new(100_000, 4096).with_streaming(true),
    )
    .with_pricing(
        ModelPricing::try_new(
            Pricing {
                input_cost_per_mtok: 1.0,
                output_cost_per_mtok: 2.0,
                cache_read_cost_per_mtok: 3.0,
                cache_write_cost_per_mtok: 4.0,
            },
            vec![],
        )
        .unwrap(),
    )
    .unwrap();
    let provider = AnthropicProvider::for_route(
        "anthropic".into(),
        vec![info],
        Some(server.uri()),
        Default::default(),
        Arc::new(opi_ai::http::HttpClient::new()),
        false,
    );
    let mut models = Models::new();
    models
        .register_route(
            Box::new(provider),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("fixture-key"),
            )),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();
    let context = AgentLoopContext {
        collection: Arc::new(models),
        registry: Arc::new(ToolRegistry::from_tools(vec![]).unwrap()),
        authorizer: None,
        evidence_health: opi_agent::evidence::EvidenceHealth::healthy(),
        evidence_sink: None,
        state: NextTurnState::new(
            vec![AgentMessage::Llm(Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "fixture".into(),
                }],
                timestamp_ms: 1,
            }))],
            ModelSelection::new("anthropic", "usage-model"),
            InferenceConfig::default(),
        ),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: None,
        session_id: None,
    };
    let events = Arc::new(Mutex::new(vec![]));
    let received = events.clone();
    let outcome = opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &Hooks,
        Box::new(move |e| {
            if let AgentEvent::MessageEnd { message, .. } = e {
                received.lock().unwrap().push(message);
            }
        }),
        CancellationToken::new(),
    )
    .await;
    let messages = outcome.state().context.clone();
    assert_eq!(outcome.into_execution_result().is_err(), failure);
    let expected = Usage::reported(0, 2, 0, 0, Some(0), Some(1));
    for actual in [usage(&messages), usage(&events.lock().unwrap())] {
        assert_eq!(
            (
                actual.input_tokens,
                actual.output_tokens,
                actual.cache_read_tokens,
                actual.cache_write_tokens,
                actual.cache_write_1h_tokens,
                actual.reasoning_tokens
            ),
            (0, 2, 0, 0, Some(0), Some(1))
        );
        assert!((actual.cost.total_cost() - 0.000004).abs() < 1e-12);
    }
    if !failure {
        let dir = tempfile::tempdir().unwrap();
        let config = opi_agent::compaction::CompactionConfig {
            enabled: false,
            ..Default::default()
        };
        let mut coordinator = SessionCoordinator::new_for_test(
            dir.path(),
            dir.path().to_str().unwrap(),
            config.clone(),
            "anthropic:usage-model",
        )
        .unwrap();
        coordinator.on_turn_end(&messages, &expected, 0).unwrap();
        let path = coordinator.session_path().to_path_buf();
        let id = coordinator.session_id().to_owned();
        drop(coordinator);
        let before = std::fs::read(&path).unwrap();
        let (_, entries, recovery) = SessionReader::read_with_recovery(&path).unwrap();
        assert!(recovery.is_clean());
        let restored = opi_agent::session_context::reconstruct_context(&entries, &recovery);
        assert_eq!(usage(&restored.messages), usage(&messages));
        let coordinator = SessionCoordinator::open_existing(
            path.clone(),
            id,
            &entries,
            restored.messages.len(),
            config,
            "anthropic:usage-model",
        )
        .unwrap();
        assert_eq!(coordinator.usage().total_input_tokens(), 0);
        assert_eq!(coordinator.usage().total_output_tokens(), 2);
        assert_eq!(coordinator.usage().total_cache_write_1h_tokens(), Some(0));
        assert_eq!(coordinator.usage().total_reasoning_tokens(), Some(1));
        assert_eq!(std::fs::read(path).unwrap(), before);
        let persisted: Vec<Value> = String::from_utf8(before)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        println!("{}", json!({"agent_usage_recovery":persisted}));
    }
}
#[tokio::test]
async fn corrected_usage_reaches_agent_and_disk_recovery() {
    check(false).await;
}
#[tokio::test]
async fn corrected_failure_partial_reaches_agent_message_end() {
    check(true).await;
}
