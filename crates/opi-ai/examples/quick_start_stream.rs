//! Streaming quick start: iterate the neutral entry's provider-neutral
//! events as they arrive, then read the terminal event's final message.
//!
//! Compile: `cargo check -p opi-ai --examples`.
//! Run: `ANTHROPIC_API_KEY=... cargo run -p opi-ai --example quick_start_stream`.

use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::anthropic::AnthropicProvider;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Request, TransportPreference};
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::transcript;
use opi_ai::{
    AuthProvenanceSource, AuthScheme, ChatModelRecord, Models, SimpleOptions, StaticAuthResolver,
};
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;

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

    let context = transcript::normalize_context(transcript::Context {
        system_prompt: Some("You are concise.".into()),
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "Count from one to five.".into(),
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

    // `stream_simple` returns a lazy `ChatCall`: it is a
    // `Stream<Item = Result<AssistantStreamEvent, CallFailure>>` whose
    // terminal absorbs into permanent EOF. `Done` carries the final
    // `AssistantMessage`; one `Err` item is the single final failure.
    let mut call = models.stream_simple(record, request, SimpleOptions::default());
    while let Some(event) = call.next().await {
        match event.map_err(|failure| format!("call failed: {:?}", failure.cause()))? {
            AssistantStreamEvent::TextDelta { delta, .. } => print!("{delta}"),
            AssistantStreamEvent::Done { message, .. } => {
                println!();
                eprintln!("-- stop reason: {:?}", message.stop_reason);
            }
            _ => {}
        }
    }
    Ok(())
}
