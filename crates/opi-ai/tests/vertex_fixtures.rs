//! Google Vertex AI fixture target index.
//!
//! Vertex protocol assertions are grouped by wire, lifecycle, auth source,
//! callback and adapter boundaries. Shared builders live in `fixtures.rs`.

use futures_util::StreamExt;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{
    CacheRetention, EventStream, Provider, ProviderError, Request, ThinkingConfig,
};
use opi_ai::providers::vertex::VertexProvider;
use opi_ai::registry::ProviderRegistry;
use opi_ai::stream::AssistantStreamEvent;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "vertex_fixtures/adc_external.rs"]
mod adc_external;
#[path = "vertex_fixtures/adc_impersonation.rs"]
mod adc_impersonation;
#[path = "vertex_fixtures/adc_vertex.rs"]
mod adc_vertex;
#[path = "vertex_fixtures/builtin_routes.rs"]
mod builtin_routes;
#[path = "vertex_fixtures/callbacks.rs"]
mod callbacks;
#[path = "vertex_fixtures/constrained_guard.rs"]
mod constrained_guard;
#[path = "vertex_fixtures/fixtures.rs"]
mod fixtures;
#[path = "vertex_fixtures/google_41.rs"]
mod google_41;
#[path = "vertex_fixtures/lifecycle.rs"]
mod lifecycle;
#[path = "vertex_fixtures/wire.rs"]
mod wire;

pub(crate) use fixtures::{lifecycle_text_request, text_sse_fixture};
