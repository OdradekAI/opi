//! Azure OpenAI provider protocol fixture target index.
//!
//! Shared Chat and native Responses coverage are grouped by wire, lifecycle,
//! callbacks, options and reusable fixtures.

use futures_util::StreamExt;
use opi_ai::api::azure_openai::AzureOpenAIWireAdapter;
use opi_ai::provider::{CacheRetention, Provider, ProviderError};
use opi_ai::providers::azure_openai::AzureOpenAIProvider;
use opi_ai::stream::AssistantStreamEvent;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "azure_openai_fixtures/chat_callbacks.rs"]
mod chat_callbacks;
#[path = "azure_openai_fixtures/chat_lifecycle.rs"]
mod chat_lifecycle;
#[path = "azure_openai_fixtures/chat_wire.rs"]
mod chat_wire;
#[path = "azure_openai_fixtures/fixtures.rs"]
mod fixtures;
#[path = "azure_openai_fixtures/responses.rs"]
mod responses;

pub(crate) use fixtures::{
    AzureStallPoint, assert_direct_projection_prefix, collect_events, collect_items,
    direct_error_cause, error_sse_fixture, make_provider, make_provider_with_deployments,
    spawn_stalled_azure_error_body_server, spawn_stalled_azure_server, text_request,
    text_sse_fixture, tool_call_sse_fixture, tool_request, wire_adapter,
};
