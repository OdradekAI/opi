//! Bedrock constrained sampling guard tests.

use super::*;
use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
use opi_ai::message::ToolDef;
use opi_ai::provider::{ProviderError, ThinkingConfig};

fn constrained_tool(strict: StrictPreference) -> ToolDef {
    ToolDef {
        name: "structured".into(),
        description: "d".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {"path": {"type": "string"}},
            "required": ["path"]
        }),
        constrained_sampling: Some(ConstrainedSampling::JsonSchema { strict }),
    }
}

fn guarded_request(tools: Vec<ToolDef>) -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "bedrock:anthropic.claude-3-5-sonnet-20240620-v1:0".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text { text: "go".into() }],
                timestamp_ms: 0,
            })],
            tools,
        })
        .expect("valid context declarations"),
        max_tokens: Some(64),
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

#[test]
fn require_constrained_sampling_fails_typed_before_dispatch() {
    let provider = BedrockProvider::new(None, Arc::new(opi_ai::http::HttpClient::new()));
    let error = provider
        .build_converse_body(&guarded_request(vec![constrained_tool(
            StrictPreference::Require,
        )]))
        .expect_err("require never silently drops on this wire");
    assert!(
        matches!(error, ProviderError::Config(_)),
        "typed pre-dispatch failure, got {error:?}"
    );
}

#[test]
fn prefer_constrained_sampling_projects_non_strict() {
    let provider = BedrockProvider::new(None, Arc::new(opi_ai::http::HttpClient::new()));
    let body = provider
        .build_converse_body(&guarded_request(vec![constrained_tool(
            StrictPreference::Prefer,
        )]))
        .expect("prefer projects non-strict");
    let serialized = body.to_string();
    assert!(
        !serialized.contains("\"strict\""),
        "no strict flag is invented on this wire: {serialized}"
    );
}
