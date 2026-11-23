//! OpenRouter protocol fixture target index.
//!
//! Shared Chat compatibility and OpenRouter Images protocol coverage are
//! grouped by wire, lifecycle, callbacks and reusable fixtures.

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

#[path = "openrouter_fixtures/chat_lifecycle.rs"]
mod chat_lifecycle;
#[path = "openrouter_fixtures/chat_wire.rs"]
mod chat_wire;
#[path = "openrouter_fixtures/fixtures.rs"]
mod fixtures;
#[path = "openrouter_fixtures/images.rs"]
mod images;
#[path = "support/pending_auth.rs"]
mod pending_auth;

pub(crate) use fixtures::{collect_stream, openrouter_provider, spawn_stalled_openrouter_server};
