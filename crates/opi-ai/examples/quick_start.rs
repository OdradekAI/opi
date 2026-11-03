//! Complete ordinary quick start: register the Anthropic route with injected
//! auth, resolve the caller-owned model record, and drive one chat call to
//! its single final message through the neutral entry.
//!
//! Compile: `cargo check -p opi-ai --examples`.
//! Run: `ANTHROPIC_API_KEY=... cargo run -p opi-ai --example quick_start`.

use std::sync::Arc;

use opi_ai::anthropic::AnthropicProvider;
use opi_ai::message::{AssistantContent, InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Request, TransportPreference};
use opi_ai::transcript;
use opi_ai::{
    AuthProvenanceSource, AuthScheme, ChatModelRecord, Models, SimpleOptions, StaticAuthResolver,
};
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // SDK construction never reads product configuration: auth is injected
    // per route (AIR-021).
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

    // Caller-owned record: provider id, bare model id, and the exact wire
    // from the registry's model metadata - valid without catalog membership.
    let info = models
        .get_model("anthropic", "claude-sonnet-4-5-20250514")
        .ok_or("claude-sonnet-4-5-20250514 is not in the builtin anthropic catalog")?;
    let record = ChatModelRecord::new("anthropic", info);

    // Canonical transcript input (AIR-020): the system prompt and tool
    // declarations fold into one leading system message at normalization.
    let context = transcript::normalize_context(transcript::Context {
        system_prompt: Some("You are concise.".into()),
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "In one short sentence, say hello.".into(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![],
    })?;

    let request = Request {
        projection: Default::default(),
        transport: TransportPreference::default(),
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
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
    };

    // Neutral entry: construction freezes the owned call, authentication
    // starts on the first poll, and `complete_simple` drains to the single
    // final outcome (`Ok(AssistantMessage)` or one `Err(CallFailure)`).
    let message = models
        .complete_simple(record, request, SimpleOptions::default())
        .await
        .map_err(|failure| format!("call failed: {:?}", failure.cause()))?;

    for content in &message.content {
        if let AssistantContent::Text { text, .. } = content {
            println!("{text}");
        }
    }
    Ok(())
}
