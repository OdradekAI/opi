//! Pi Messages protocol fixture target index.
//!
//! Radius gateway/catalog behavior, endpoint and context projection, SSE
//! consumption, execution lifecycle, callbacks and replay regressions are
//! grouped by their owning contract with one integration executable.

use futures_util::StreamExt;
use opi_ai::WireApi;
use opi_ai::api::pi_messages::{
    PiMessagesChatApi, PiMessagesOptions, PiMessagesToolChoice, resolve_wire_cache_retention,
};
use opi_ai::callbacks::RequestCallbacks;
use opi_ai::message::{InputContent, Message, ToolDef, UserMessage};
use opi_ai::model_info::{ThinkingLevel, ThinkingLevelMap, ThinkingLevelMapping};
use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::providers::pi_messages::{BASE_URL, PiMessagesProvider};
use opi_ai::registry::ModelCapabilities;
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use secrecy::SecretString;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "chat_pi_messages/callbacks.rs"]
mod callbacks;
#[path = "chat_pi_messages/fixtures.rs"]
mod fixtures;
#[path = "chat_pi_messages/gateway.rs"]
mod gateway;
#[path = "chat_pi_messages/lifecycle.rs"]
mod lifecycle;
#[path = "chat_pi_messages/node5.rs"]
mod node5;
#[path = "chat_pi_messages/stream.rs"]
mod stream;
#[path = "chat_pi_messages/wire.rs"]
mod wire;

pub(crate) use fixtures::{
    MODEL_44, assert_direct_projection_prefix, base_req_44, body_44, clamped_model_44,
    done_message_44, env_lock_44, first_error_44, model_44, mount_44, req_44, run_44, sse_ok_44,
    thinking_model_44,
};
