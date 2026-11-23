//! Bedrock provider fixture tests (task 3.1).
//!
//! Tests cover: text streaming, tool calls, usage, provider errors, error mapping,
//! model-family routing, credential redaction, and no live AWS calls.

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_core::Stream;
use futures_util::{StreamExt, pin_mut};
use opi_ai::api::bedrock::event_stream;
use opi_ai::api::bedrock::map_bedrock_status;
use opi_ai::auth::aws::sigv4::AwsCredentials;
use opi_ai::credential::BoxAuthFuture;
use opi_ai::http::HttpClient;
use opi_ai::message::{
    AssistantContent, AssistantMessage, ImageSource, InputContent, MediaType, Message,
    OutputContent, ToolCall, ToolDef, ToolResultMessage, UserMessage,
};
use opi_ai::provider::{CacheRetention, Provider, ProviderError, ProviderErrorCategory, Request};
use opi_ai::providers::bedrock::BedrockProvider;
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use opi_ai::{
    AuthFallback, AuthProvenance, AuthProvenanceSource, AuthResolver, AuthScheme,
    AwsCredentialSource, CompatMetadata, Models, ModelsError, ResolvedAuth,
};
use secrecy::SecretString;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[allow(dead_code)]
#[path = "support/tls_fixtures.rs"]
mod builtin_tls_fixtures;

#[path = "bedrock_fixtures/aws_source_transport.rs"]
mod aws_source_transport;

#[path = "bedrock_fixtures/builtin_routes.rs"]
mod builtin_routes;
#[path = "bedrock_fixtures/constrained_guard.rs"]
mod constrained_guard;
#[path = "bedrock_fixtures/converse_adapter.rs"]
mod converse_adapter;

// ---------------------------------------------------------------------------
// Topic modules and shared fixture owner
// ---------------------------------------------------------------------------

#[path = "bedrock_fixtures/fixtures.rs"]
mod fixtures;
#[path = "bedrock_fixtures/lifecycle.rs"]
mod lifecycle;
#[path = "bedrock_fixtures/wire.rs"]
mod wire;

use fixtures::*;
