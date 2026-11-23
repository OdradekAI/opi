//! The producer result observed at the terminal of a single public Chat call.

use opi_ai::WireApi;
use opi_ai::message::AssistantContent;

use super::fixtures::{BUILTIN_CASES, BuiltinCapture, capture_builtin};

#[tokio::test]
async fn builtin_producer_output_captures_identity_reasoning_and_terminal_facts() {
    for case @ (provider, wire, details) in BUILTIN_CASES {
        let BuiltinCapture {
            peer,
            message: captured,
            ..
        } = capture_builtin(case, 1).await;
        let responses = wire != WireApi::OpenAiCompletions;
        assert_eq!(captured.provider, provider);
        assert_eq!(captured.model, "requested-alias");
        assert_eq!(captured.wire, Some(wire));
        let raw_stop = if responses { "completed" } else { "stop" };
        assert_eq!(captured.raw_stop_reason.as_deref(), Some(raw_stop));
        assert_eq!(captured.response_model.as_deref(), Some("resolved-model"));
        assert_eq!(captured.usage.input_tokens, 3);
        let (thinking, signature) = captured
            .content
            .iter()
            .find_map(|block| match block {
                AssistantContent::Thinking {
                    thinking,
                    signature,
                } => Some((thinking, signature)),
                _ => None,
            })
            .expect("actual adapter must capture its thinking output");
        assert_eq!(thinking, "Plan");
        if !responses && !details {
            assert_eq!(signature.as_deref(), Some("reasoning_content"));
        }
        if responses {
            let expected = (wire != WireApi::AzureOpenAiResponses).then_some("high");
            assert_eq!(captured.provider_thinking_level.as_deref(), expected);
        }
        peer.verify().await;
    }
}
