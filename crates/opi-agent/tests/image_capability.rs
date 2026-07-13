//! Agent-side image capability gating.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use opi_agent::hooks::AgentHooks;
use opi_agent::loop_types::{AgentError, AgentLoopConfig, AgentLoopContext};
use opi_agent::message::AgentMessage;
use opi_ai::message::{ImageSource, InputContent, MediaType, Message, UserMessage};
use opi_ai::provider::{EventStream, ModelInfo, Provider, Request};
use tokio_util::sync::CancellationToken;

struct TextOnlyProvider {
    calls: Arc<AtomicUsize>,
    models: Vec<ModelInfo>,
}

impl TextOnlyProvider {
    fn new(calls: Arc<AtomicUsize>) -> Self {
        Self {
            calls,
            models: vec![ModelInfo {
                id: "text-only".into(),
                display_name: "Text Only".into(),
                context_window: 8192,
                max_output_tokens: 1024,
                supports_images: false,
                supports_streaming: true,
                supports_thinking: false,
            }],
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
        provider: Box::new(TextOnlyProvider::new(calls.clone())),
        tools: vec![],
        messages: vec![AgentMessage::Llm(Message::User(UserMessage {
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
        model: "mock:text-only".into(),
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
