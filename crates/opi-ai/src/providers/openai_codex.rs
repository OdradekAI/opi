//! OpenAI Codex provider profile.
//!
//! This module owns Codex identity, endpoint and construction state. The
//! subscription Responses wire and native options live in the API adapter;
//! pooled WebSocket resources live in the transport layer.

use std::sync::Arc;

use crate::http::HttpClient;
use crate::model_info::ModelInfo;
use crate::provider::routes::{OPENAI_CODEX_PROVIDER_ID, OpenAiCodexRouteContext};

pub(crate) const PROVIDER_ID: &str = OPENAI_CODEX_PROVIDER_ID;
const DEFAULT_BASE_URL: &str = "https://chatgpt.com/backend-api";

/// Canonical provider-facing name for the neutral Codex route context.
pub type OpenAiCodexResponsesProvider = OpenAiCodexRouteContext;

impl OpenAiCodexRouteContext {
    pub fn new(base_url: Option<String>, models: Vec<ModelInfo>, client: Arc<HttpClient>) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.into()),
            models,
            client,
            ws_state: Arc::default(),
        }
    }
}
