//! OpenAI Codex provider profile.
//!
//! This module owns Codex identity, endpoint and construction state. The
//! subscription Responses wire and native options live in the API adapter;
//! pooled WebSocket resources live in the transport layer.

use std::sync::Arc;

use crate::http::HttpClient;
use crate::model_info::ModelInfo;
use crate::transport::codex::WsTransportState;

pub(crate) const PROVIDER_ID: &str = "openai-codex";
const DEFAULT_BASE_URL: &str = "https://chatgpt.com/backend-api";

/// OpenAI Codex provider using the subscription-specific Responses route.
///
/// Authentication is supplied per-call by the provider collection via
/// [`crate::auth::ResolvedAuth`] on `Provider::stream_prepared`. The wire
/// adapter owns request serialization, transport and stream behavior.
#[derive(Clone)]
pub struct OpenAiCodexResponsesProvider {
    pub(crate) base_url: String,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) client: Arc<HttpClient>,
    pub(crate) ws_state: Arc<WsTransportState>,
}

impl OpenAiCodexResponsesProvider {
    pub fn new(base_url: Option<String>, models: Vec<ModelInfo>, client: Arc<HttpClient>) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.into()),
            models,
            client,
            ws_state: Arc::default(),
        }
    }
}
