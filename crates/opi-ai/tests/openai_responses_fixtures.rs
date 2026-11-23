//! OpenAI Responses API fixture target index.
//!
//! Protocol assertions live in topic modules. Shared request and response
//! builders live in `fixtures.rs` so their owner is explicit.

use futures_util::StreamExt;
use opi_ai::api::openai_responses::{
    OpenAiResponsesChatApi, OpenAiResponsesOptions, ResponsesReasoningSummary,
    ResponsesServiceTier, ResponsesToolChoice,
};
use opi_ai::callbacks::RequestCallbacks;
use opi_ai::message::{InputContent, Message, ToolDef, UserMessage};
use opi_ai::models::{ChatModelRecord, Models};
use opi_ai::provider::{
    CacheRetention, EventStream, Provider, ProviderError, Request, ThinkingConfig,
};
use opi_ai::providers::openai_responses::{OpenAiResponsesProvider, ResponsesConfig};
use opi_ai::registry::ProviderRegistry;
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::{ModelCapabilities, ModelInfo, ThinkingLevel, WireApi};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "openai_responses_fixtures/constrained_guard.rs"]
mod constrained_guard;
#[path = "openai_responses_fixtures/fixtures.rs"]
mod fixtures;
#[path = "openai_responses_fixtures/lifecycle.rs"]
mod lifecycle;
#[path = "openai_responses_fixtures/wire.rs"]
mod wire;
