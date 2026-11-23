//! Native Mistral Conversations adapter topic index.

use super::*;
use opi_ai::WireApi;
use opi_ai::api::mistral::MistralConversationsWireAdapter;
use opi_ai::api::mistral::{
    MistralConversationsChatApi, MistralOptions, MistralPromptMode, MistralReasoningEffort,
    MistralToolChoice,
};
use opi_ai::message::{AssistantContent, MediaType, OutputContent, ToolCall};
use opi_ai::model_info::{ThinkingLevel, ThinkingLevelMap, ThinkingLevelMapping};
use opi_ai::providers::mistral::MistralConversationsProvider;
use opi_ai::registry::ModelCapabilities;
use opi_ai::stream::StopReason;
use secrecy::SecretString;
use std::time::{Duration, Instant};

#[path = "native_conversations/callbacks.rs"]
mod callbacks;
#[path = "native_conversations/fixtures.rs"]
mod fixtures;
#[path = "native_conversations/lifecycle.rs"]
mod lifecycle;
#[path = "native_conversations/replay.rs"]
mod replay;
#[path = "native_conversations/wire.rs"]
mod wire;

pub(crate) use fixtures::{
    SMALL_NATIVE, base_req_native, body_native, done_message_native, effort_model_native,
    first_error_native, map_native, model_native, mount_native, provider_native, req_native,
    run_native, sse_ok_native,
};

pub(crate) use replay::mistral_complete;
