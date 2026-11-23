//! Policy contracts carried by protocol route contexts.
//!
//! These values are deliberately separate from the route state declaration.
//! Provider factories choose them, while protocol adapters consume them.  The
//! contract module contains only typed policy data and no wire or provider
//! implementation.

#[cfg(feature = "provider-openai-chat")]
use std::collections::HashMap;

#[cfg(any(
    feature = "provider-openai-chat",
    feature = "provider-openai-responses"
))]
use crate::auth::AuthInvalidPolicy;
#[cfg(feature = "provider-openai-responses")]
use crate::model_info::OpenAiResponsesSourceCompat;
#[cfg(feature = "provider-openai-chat")]
use crate::model_info::{
    ChatInstructionRole, ChatStoreField, ChatThinkingFormat, OpenAiCompletionsCompat,
};

/// Per-model OpenAI-compatible compatibility override.
#[cfg(feature = "provider-openai-chat")]
#[derive(Debug, Clone, Default)]
pub struct OpenAiModelCompatOverride {
    pub instruction_role: Option<ChatInstructionRole>,
    pub store: Option<ChatStoreField>,
    pub thinking_format: Option<ChatThinkingFormat>,
    pub max_tokens_field: Option<String>,
}

/// Provider-selected policy for the OpenAI Chat Completions adapter.
#[cfg(feature = "provider-openai-chat")]
#[derive(Debug, Clone)]
pub struct OpenAiRoutePolicy {
    pub(crate) compat: OpenAiCompletionsCompat,
    pub(crate) model_overrides: HashMap<String, OpenAiModelCompatOverride>,
    pub(crate) catalog_compat: bool,
    pub(crate) auth_invalid_policy: AuthInvalidPolicy,
    pub(crate) direct_prompt_cache_key: bool,
    pub(crate) copilot_initiator: bool,
}

/// Native OpenAI Responses request configuration.
#[cfg(feature = "provider-openai-responses")]
#[derive(Debug, Clone)]
pub struct ResponsesConfig {
    pub source: OpenAiResponsesSourceCompat,
    pub store: Option<bool>,
    pub reasoning_effort: Option<String>,
    pub send_session_id_header: bool,
}

#[cfg(feature = "provider-openai-responses")]
impl Default for ResponsesConfig {
    fn default() -> Self {
        Self {
            source: crate::model_info::OpenAiResponsesCompat::default().source,
            store: None,
            reasoning_effort: None,
            send_session_id_header: true,
        }
    }
}

/// Construction-time affinity policy for the reusable Responses route.
#[cfg(feature = "provider-openai-responses")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponsesAffinityPolicy {
    Direct,
    CustomDisabled,
    CustomOptIn,
}

/// Provider-selected policy for the OpenAI Responses adapter.
#[cfg(feature = "provider-openai-responses")]
#[derive(Debug, Clone)]
pub struct OpenAiResponsesRoutePolicy {
    pub(crate) catalog_compat: bool,
    pub(crate) auth_invalid_policy: AuthInvalidPolicy,
    pub(crate) affinity_policy: ResponsesAffinityPolicy,
    pub(crate) copilot_headers: bool,
    pub(crate) chatgpt_signin: Option<bool>,
}
