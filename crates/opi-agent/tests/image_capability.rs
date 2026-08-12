//! Agent-side image capability gating.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use opi_agent::hooks::AgentHooks;
use opi_agent::loop_types::{
    AgentError, AgentLoopConfig, AgentLoopContext, InferenceConfig, ModelSelection, NextTurnState,
};
use opi_agent::message::AgentMessage;
use opi_ai::WireApi;
use opi_ai::message::{ImageSource, InputContent, MediaType, Message, UserMessage};
use opi_ai::provider::{EventStream, ModelInfo, Provider, Request};
use opi_ai::registry::ModelCapabilities;
use opi_ai::test_support::single_route_collection;
use tokio_util::sync::CancellationToken;

struct TextOnlyProvider {
    calls: Arc<AtomicUsize>,
    models: Vec<ModelInfo>,
}

impl TextOnlyProvider {
    fn new(calls: Arc<AtomicUsize>) -> Self {
        Self {
            calls,
            models: vec![ModelInfo::new(
                "text-only",
                "Text Only",
                WireApi::OpenAiCompletions,
                ModelCapabilities::new(8192, 1024).with_streaming(true),
            )],
        }
    }
}

impl Provider for TextOnlyProvider {
    fn id(&self) -> &str {
        "mock"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream(&self, _request: Request) -> EventStream {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(futures_util::stream::empty())
    }
}

struct TestHooks;

impl AgentHooks for TestHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        Ok(messages
            .iter()
            .filter_map(|m| match m {
                AgentMessage::Llm(message) => Some(message.clone()),
                _ => None,
            })
            .collect())
    }
}

#[tokio::test]
async fn image_input_to_text_only_model_fails_before_provider_call() {
    let calls = Arc::new(AtomicUsize::new(0));
    let context = AgentLoopContext {
        collection: Arc::new(single_route_collection(Box::new(TextOnlyProvider::new(
            calls.clone(),
        )))),
        tools: vec![],
        state: NextTurnState::new(
            vec![AgentMessage::Llm(Message::User(UserMessage {
                content: vec![
                    InputContent::Text {
                        text: "describe".into(),
                    },
                    InputContent::Image {
                        source: ImageSource::Bytes {
                            data: vec![0x89, 0x50, 0x4e, 0x47],
                        },
                        media_type: MediaType::Png,
                    },
                ],
                timestamp_ms: 0,
            }))],
            ModelSelection::parse_spec("mock:text-only").unwrap(),
            InferenceConfig::default(),
        ),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: None,
        trace: None,
        session_id: None,
    };

    let err = opi_agent::agent_loop(
        context,
        AgentLoopConfig::default(),
        &TestHooks,
        Box::new(|_| {}),
        CancellationToken::new(),
    )
    .await
    .unwrap_err();

    assert!(
        matches!(err, AgentError::Provider(ref message) if message.contains("does not support image input")),
        "unexpected error: {err}"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
