//! Mistral provider fixture target index.
//!
//! Shared Chat compatibility coverage and the native Mistral Conversations adapter
//! are grouped by wire, lifecycle, callbacks and reusable fixtures.

use futures_util::StreamExt;
use opi_ai::message::{InputContent, Message, ToolDef, UserMessage};
use opi_ai::provider::{
    CacheRetention, EventStream, Provider, ProviderError, Request, ThinkingConfig,
};
use opi_ai::providers::openai::{CompatConfig, OpenAiChatProvider};
use opi_ai::registry::ProviderRegistry;
use opi_ai::stream::AssistantStreamEvent;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "mistral_fixtures/chat_callbacks.rs"]
mod chat_callbacks;
#[path = "mistral_fixtures/chat_lifecycle.rs"]
mod chat_lifecycle;
#[path = "mistral_fixtures/chat_wire.rs"]
mod chat_wire;
#[path = "mistral_fixtures/fixtures.rs"]
mod fixtures;
#[path = "mistral_fixtures/native_conversations.rs"]
mod native_conversations;

pub(crate) use fixtures::{
    collect_stream, mistral_provider, spawn_stalled_mistral_server, write_chunk,
};
