//! Provider-neutral route contexts consumed by protocol adapters.
//!
//! A route context carries identity, catalog, endpoint and injected transport
//! state. Concrete provider profiles may expose aliases and constructors, but
//! API adapters depend on these lower contract values instead of profile types.

#[cfg(feature = "provider-gemini")]
use std::sync::Arc;

#[cfg(feature = "provider-gemini")]
use crate::http::HttpClient;
#[cfg(feature = "provider-gemini")]
use crate::model_info::WireApi;
#[cfg(feature = "provider-gemini")]
use crate::provider::ModelInfo;

/// Route state shared by Google Gemini-compatible wire adapters.
#[cfg(feature = "provider-gemini")]
#[derive(Clone)]
pub struct GeminiRouteContext {
    pub(crate) provider_identity: String,
    pub(crate) base_url: String,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) client: Arc<HttpClient>,
    pub(crate) wire: WireApi,
}

/// Route state owned by Google Vertex AI and consumed by its wire adapter.
#[cfg(feature = "provider-vertex")]
#[derive(Clone)]
pub struct VertexRouteContext {
    pub(crate) provider_identity: String,
    pub(crate) project: String,
    pub(crate) location: String,
    pub(crate) base_url: String,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) inner: GeminiRouteContext,
    pub(crate) client: Arc<HttpClient>,
    pub(crate) express: bool,
}
