//! Integration test for thinking config passthrough.

use std::sync::{Arc, Mutex};

use opi_agent::hooks::AgentHooks;
use opi_agent::loop_types::{
    AgentLoopConfig, AgentLoopContext, InferenceConfig, ModelSelection, NextTurnState,
};
use opi_agent::message::AgentMessage;
use opi_agent::{agent_loop, event::AgentEventSink};
use opi_ai::WireApi;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{EventStream, ModelInfo, Provider, Request, ThinkingConfig};
use opi_ai::registry::ModelCapabilities;
use opi_ai::stream::{AssistantStreamEvent, StopReason, Usage};
use opi_ai::test_support::single_route_collection;

struct CaptureProvider {
    requests: Arc<Mutex<Vec<Request>>>,
    models: Vec<ModelInfo>,
}

impl Provider for CaptureProvider {
    fn stream(&self, request: Request) -> EventStream {
        self.requests.lock().unwrap().push(request);
        let msg = opi_ai::message::AssistantMessage {
            content: vec![opi_ai::message::AssistantContent::Text { text: "ok".into() }],
            usage: Usage::default(),
            stop_reason: StopReason::Stop,
            ..opi_ai::test_support::base_assistant()
        };
        Box::pin(futures_util::stream::iter(vec![Ok(
            AssistantStreamEvent::Done {
                reason: StopReason::Stop,
                message: msg,
            },
        )]))
    }

    fn id(&self) -> &str {
        "test"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }
}

struct NoopHooks;
impl AgentHooks for NoopHooks {
    fn convert_to_llm(
        &self,
        messages: &[AgentMessage],
    ) -> Result<Vec<Message>, opi_agent::loop_types::AgentError> {
        let mut result = Vec::new();
        for msg in messages {
            if let AgentMessage::Llm(m) = msg {
                result.push(m.clone());
            }
        }
        Ok(result)
    }
}

fn user_msg(text: &str) -> AgentMessage {
    AgentMessage::Llm(Message::User(UserMessage {
        content: vec![InputContent::Text { text: text.into() }],
        timestamp_ms: 0,
    }))
}

fn noop_sink() -> AgentEventSink {
    Box::new(|_| {})
}

#[tokio::test]
async fn thinking_config_passed_to_provider() {
    let captured = Arc::new(Mutex::new(Vec::new()));
    let provider = CaptureProvider {
        requests: captured.clone(),
        models: vec![ModelInfo::new(
            "test-model",
            "Test Model",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(8192, 1024).with_thinking(true),
        )],
    };
    let context = AgentLoopContext {
        collection: Arc::new(single_route_collection(Box::new(provider))),
        tools: vec![],
        state: NextTurnState::new(
            vec![user_msg("hi")],
            ModelSelection::parse_spec("test:test-model").unwrap(),
            InferenceConfig {
                thinking: ThinkingConfig {
                    enabled: true,
                    budget_tokens: Some(10_000),
                    level: opi_ai::ThinkingLevel::Medium,
                },
                ..Default::default()
            },
        ),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: None,
        trace: None,
        session_id: None,
    };
    let config = AgentLoopConfig {
        max_turns: 1,
        ..Default::default()
    };
    let cancel = tokio_util::sync::CancellationToken::new();
    let _ = agent_loop(context, config, &NoopHooks, noop_sink(), cancel).await;
    let reqs = captured.lock().unwrap();
    assert_eq!(reqs.len(), 1);
    assert!(reqs[0].thinking.enabled);
    assert_eq!(reqs[0].thinking.budget_tokens, Some(10_000));
}

#[tokio::test]
async fn thinking_disabled_by_default() {
    let captured = Arc::new(Mutex::new(Vec::new()));
    let provider = CaptureProvider {
        requests: captured.clone(),
        models: vec![ModelInfo::new(
            "test-model",
            "Test Model",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(8192, 1024).with_thinking(true),
        )],
    };
    let context = AgentLoopContext {
        collection: Arc::new(single_route_collection(Box::new(provider))),
        tools: vec![],
        state: NextTurnState::new(
            vec![user_msg("hi")],
            ModelSelection::parse_spec("test:test-model").unwrap(),
            InferenceConfig::default(),
        ),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: None,
        trace: None,
        session_id: None,
    };
    let config = AgentLoopConfig {
        max_turns: 1,
        ..Default::default()
    };
    let cancel = tokio_util::sync::CancellationToken::new();
    let _ = agent_loop(context, config, &NoopHooks, noop_sink(), cancel).await;
    let reqs = captured.lock().unwrap();
    assert_eq!(reqs.len(), 1);
    assert!(!reqs[0].thinking.enabled);
}
