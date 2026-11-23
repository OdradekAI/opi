//! Google Gemini provider fixture target index.
//!
//! Gemini protocol assertions are grouped by wire, lifecycle, callbacks,
//! constrained sampling and Google 4.0 adapter boundaries. Shared builders
//! and local transport helpers live in `fixtures.rs`.

use futures_util::StreamExt;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{
    CacheRetention, EventStream, Provider, ProviderError, Request, ThinkingConfig,
};
use opi_ai::providers::gemini::GeminiProvider;
use opi_ai::registry::ProviderRegistry;
use opi_ai::stream::AssistantStreamEvent;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "gemini_fixtures/callbacks.rs"]
mod callbacks;
#[path = "gemini_fixtures/constrained_guard.rs"]
mod constrained_guard;
#[path = "gemini_fixtures/fixtures.rs"]
mod fixtures;
#[path = "gemini_fixtures/google_40.rs"]
mod google_40;
#[path = "gemini_fixtures/lifecycle.rs"]
mod lifecycle;
#[path = "gemini_fixtures/wire.rs"]
mod wire;

pub(crate) use fixtures::{
    GeminiStallPoint, assert_gemini_cancelled, collect_stream, direct_cause, gemini_http_request,
    gemini_provider, spawn_stalled_gemini_error_body_server,
};
