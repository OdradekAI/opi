//! Behavioral tests for task 2.1: OpenAI-compatible chat provider.
//!
//! DoD: "fixtures cover text, tool call, usage, error; implements Provider trait
//! with SSE streaming; exposes compat config points for role mapping
//! (developer/system), usage-in-stream, max_tokens field naming, and tool_result
//! name field so downstream profiles (OpenRouter, Mistral) can override behavior"
//!
//! All tests use fixture strings — no live provider calls (red flag #10).

use futures_util::StreamExt;
use opi_ai::api::openai_completions::{
    OpenAiChatEvent, OpenAiChatMapper, ParsedEvent, parse_sse_events,
};
use opi_ai::message::{AssistantContent, AssistantMessage, ToolCall};
use opi_ai::provider::{CacheRetention, EventStream, Provider, validate_request_capabilities};
use opi_ai::providers::openai::{CompatConfig, OpenAiChatProvider};
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::Request;
use opi_ai::provider::ThinkingConfig;
use tokio_util::sync::CancellationToken;

use opi_ai::model_info::{ChatInstructionRole, ChatStoreField};

use opi_ai::message::{ImageSource, MediaType, OutputContent, ToolDef, ToolResultMessage};
use opi_ai::provider::{ModelInfo, ProviderError, ProviderErrorCategory};
use opi_ai::registry::ModelCapabilities;

use opi_ai::callbacks::RequestCallbacks;

use opi_ai::api::contract::{SimpleOptions, SimpleToolChoice};
use opi_ai::api::openai_completions::{
    OpenAiCompletionsChatApi, OpenAiCompletionsOptions, OpenAiToolChoice,
};
use opi_ai::models::{ChatModelRecord, Models};
use std::sync::Arc as StdArc;

#[path = "openai_chat_fixtures/constrained_guard.rs"]
mod constrained_guard;
#[path = "openai_chat_fixtures/fixtures.rs"]
mod fixtures;
#[path = "openai_chat_fixtures/lifecycle.rs"]
mod lifecycle;
#[path = "openai_chat_fixtures/wire.rs"]
mod wire;
