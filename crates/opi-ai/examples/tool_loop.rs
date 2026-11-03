//! Agent-owned tool loop: declare a tool through the canonical transcript,
//! execute each admitted tool call, close it with a `ToolResultMessage`, and
//! continue until the assistant produces final text.
//!
//! Compile: `cargo check -p opi-ai --examples`.
//! Run: `ANTHROPIC_API_KEY=... cargo run -p opi-ai --example tool_loop`.

use std::sync::Arc;

use opi_ai::anthropic::AnthropicProvider;
use opi_ai::message::{
    AssistantContent, InputContent, Message, OutputContent, ToolCall, ToolDef,
    ToolNotExecutedReason, ToolResultMessage, UserMessage,
};
use opi_ai::provider::{CacheRetention, Request, TransportPreference};
use opi_ai::transcript::{self, TranscriptContext};
use opi_ai::{
    AuthProvenanceSource, AuthScheme, ChatModelRecord, Models, SimpleOptions, StaticAuthResolver,
};
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;

const MODEL_SPEC: &str = "anthropic:claude-sonnet-4-5-20250514";

struct ToolOutcome {
    text: String,
    is_error: bool,
    not_executed: Option<ToolNotExecutedReason>,
}

/// The caller is the trusted execution boundary (AIR-048): a malformed final
/// argument text is recorded as not executed, never guessed into a call.
fn execute_weather_tool(call: &ToolCall) -> ToolOutcome {
    if call.arguments_error.is_some() {
        return ToolOutcome {
            text: "tool arguments were not a complete JSON object".into(),
            is_error: true,
            not_executed: Some(ToolNotExecutedReason::NonObjectArguments),
        };
    }
    let arguments: serde_json::Value =
        serde_json::from_str(&call.arguments).unwrap_or_else(|_| serde_json::json!({}));
    let city = arguments.get("city").and_then(|value| value.as_str());
    let report = match city {
        Some("Tokyo") => "22 C, clear".to_string(),
        Some(other) => format!("no data for {other}"),
        None => "missing required field: city".to_string(),
    };
    ToolOutcome {
        text: report,
        is_error: city.is_none(),
        not_executed: None,
    }
}

fn request_for(context: TranscriptContext) -> Request {
    Request {
        projection: Default::default(),
        transport: TransportPreference::default(),
        model: MODEL_SPEC.into(),
        context,
        max_tokens: Some(1024),
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        callbacks: Default::default(),
        telemetry_context: None,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut models = Models::new();
    models.register_route(
        Box::new(AnthropicProvider::new(None)),
        Arc::new(StaticAuthResolver::new(
            AuthScheme::ApiKey,
            SecretString::from(std::env::var("ANTHROPIC_API_KEY")?),
        )),
        AuthProvenanceSource::Environment {
            name: "ANTHROPIC_API_KEY".into(),
        },
        Default::default(),
    )?;

    let info = models
        .get_model("anthropic", "claude-sonnet-4-5-20250514")
        .ok_or("claude-sonnet-4-5-20250514 is not in the builtin anthropic catalog")?;
    let record = ChatModelRecord::new("anthropic", info);

    // Tools are declared on the transcript context; normalization folds them
    // into the leading system message (AIR-020).
    let tools = vec![ToolDef {
        name: "get_weather".into(),
        description: "Current weather for one city.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": { "city": { "type": "string" } },
            "required": ["city"],
        }),
        constrained_sampling: None,
    }];

    let mut history: Vec<Message> = vec![Message::User(UserMessage {
        content: vec![InputContent::Text {
            text: "What is the weather in Tokyo? Use the tool.".into(),
        }],
        timestamp_ms: 0,
    })];

    // Continuation: every round replays the full canonical chronology
    // (assistant tool-call turns and their tool results included).
    for _round in 0..8 {
        let context = transcript::normalize_context(transcript::Context {
            system_prompt: Some("You are a concise assistant.".into()),
            messages: history.clone(),
            tools: tools.clone(),
        })?;
        let assistant = models
            .complete_simple(
                record.clone(),
                request_for(context),
                SimpleOptions::default(),
            )
            .await
            .map_err(|failure| format!("call failed: {:?}", failure.cause()))?;

        let tool_calls: Vec<ToolCall> = assistant
            .content
            .iter()
            .filter_map(|content| match content {
                AssistantContent::ToolCall { tool_call } => Some(tool_call.clone()),
                _ => None,
            })
            .collect();

        if tool_calls.is_empty() {
            for content in &assistant.content {
                if let AssistantContent::Text { text, .. } = content {
                    println!("{text}");
                }
            }
            return Ok(());
        }

        history.push(Message::Assistant(assistant));
        for tool_call in tool_calls {
            let outcome = execute_weather_tool(&tool_call);
            history.push(Message::ToolResult(ToolResultMessage {
                tool_call_id: tool_call.id,
                tool_name: tool_call.name,
                content: vec![OutputContent::Text { text: outcome.text }],
                details: None,
                is_error: outcome.is_error,
                not_executed: outcome.not_executed,
                truncated: false,
                timestamp_ms: 0,
            }));
        }
    }

    Err("tool loop did not converge".into())
}
