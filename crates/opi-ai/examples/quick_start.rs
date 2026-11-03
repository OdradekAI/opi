//! Complete ordinary quick start: register lazy builtins, select a model,
//! and drive one chat call to its single final message through the neutral entry.
//!
//! Compile: `cargo check -p opi-ai --examples`.
//! Run: `ANTHROPIC_API_KEY=... cargo run -p opi-ai --example quick_start`.

use std::future::Future;
use std::sync::Arc;

use opi_ai::CallFailure;
use opi_ai::http::HttpClient;
use opi_ai::message::{AssistantContent, AssistantMessage, InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Request, TransportPreference};
use opi_ai::provider_collection::{BuiltinOverrides, register_builtin};
use opi_ai::transcript;
use opi_ai::{AmbientAuthContext, AuthContext, ChatModelRecord, Models, SimpleOptions};
use tokio_util::sync::CancellationToken;

/// Construct and statically select without reading authentication sources.
pub(crate) fn setup(
    context: Arc<dyn AuthContext>,
    client: Arc<HttpClient>,
    overrides: &BuiltinOverrides,
) -> Result<(Models, ChatModelRecord), Box<dyn std::error::Error>> {
    let mut models = Models::new();
    models.set_auth_context(context.clone());
    register_builtin(&mut models, context, client, overrides)?;
    let info = models
        .get_model("anthropic", "claude-sonnet-4-5")
        .ok_or("claude-sonnet-4-5 is not in the builtin anthropic catalog")?;
    Ok((models, ChatModelRecord::new("anthropic", info)))
}

/// Build the actual owned call; authentication starts only on first poll.
pub(crate) fn start(
    models: &Models,
    record: ChatModelRecord,
) -> Result<
    impl Future<Output = Result<AssistantMessage, CallFailure>> + Send + 'static + use<>,
    Box<dyn std::error::Error>,
> {
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
        model: "anthropic:claude-sonnet-4-5".into(),
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

    Ok(models.complete_simple(record, request, SimpleOptions::default()))
}

pub(crate) async fn finish(
    call: impl Future<Output = Result<AssistantMessage, CallFailure>>,
) -> Result<AssistantMessage, Box<dyn std::error::Error>> {
    let message = call
        .await
        .map_err(|failure| format!("call failed: {:?}", failure.cause()))?;
    for content in &message.content {
        if let AssistantContent::Text { text, .. } = content {
            println!("{text}");
        }
    }
    Ok(message)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (models, record) = setup(
        Arc::new(AmbientAuthContext),
        Arc::new(HttpClient::new()),
        &BuiltinOverrides::default(),
    )?;
    let call = start(&models, record)?;
    finish(call).await?;
    Ok(())
}
