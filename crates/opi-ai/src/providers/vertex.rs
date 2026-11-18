//! Google Vertex AI provider profile.
//!
//! Endpoint, route identity, project/location configuration, model catalog and
//! shared HTTP construction live here. Vertex request framing and transport
//! live in [`crate::api::vertex`].

use std::fmt;
use std::sync::Arc;

use crate::api::vertex::GoogleVertexOptions;
use crate::http::HttpClient;
use crate::model_info::WireApi;
use crate::provider::{ModelInfo, Provider};
use crate::providers::gemini::GeminiProvider;
use crate::registry::ModelCapabilities;

/// Google Vertex AI provider profile.
///
/// The profile owns route identity and construction state; the canonical API
/// adapter owns request projection, authentication headers and streaming.
#[derive(Clone)]
pub struct VertexProvider {
    pub(crate) provider_identity: String,
    pub(crate) project: String,
    pub(crate) location: String,
    pub(crate) base_url: String,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) inner: GeminiProvider,
    pub(crate) client: Arc<HttpClient>,
    /// Temporary per-dispatch native options. This state moves to the Vertex
    /// wire adapter in the dedicated Vertex profile slice.
    pub(crate) native_options: Option<GoogleVertexOptions>,
    /// API-key Express mode: project/location-less wire routed to the global
    /// `aiplatform.googleapis.com` collection endpoint.
    pub(crate) express: bool,
}

impl fmt::Debug for VertexProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VertexProvider")
            .field("project", &self.project)
            .field("location", &self.location)
            .field("models", &self.models.len())
            .finish()
    }
}

impl VertexProvider {
    /// Create a new Vertex AI provider (ADC/Bearer mode with project and
    /// location).
    pub fn new(project: String, location: String, base_url: Option<String>) -> Self {
        let base_url = base_url.unwrap_or_else(|| default_vertex_base_url(&location));
        let inner = GeminiProvider::new(None).with_wire(WireApi::GoogleVertex);
        let models = model_catalog();
        Self {
            provider_identity: "vertex".into(),
            project,
            location,
            base_url,
            models,
            inner,
            client: Arc::new(HttpClient::new()),
            native_options: None,
            express: false,
        }
    }

    /// Create the API-key Express provider (no project/location): requests
    /// go to `{base}/v1/publishers/google/models/{model}:streamGenerateContent`
    /// with the `x-goog-api-key` header.
    pub fn express(base_url: Option<String>) -> Self {
        let base_url = base_url.unwrap_or_else(|| "https://aiplatform.googleapis.com".to_owned());
        let inner = GeminiProvider::new(None).with_wire(WireApi::GoogleVertex);
        let models = model_catalog();
        Self {
            provider_identity: "vertex".into(),
            project: String::new(),
            location: String::new(),
            base_url,
            models,
            inner,
            client: Arc::new(HttpClient::new()),
            native_options: None,
            express: true,
        }
    }

    /// Whether this provider routes Express (API-key) requests.
    pub fn is_express(&self) -> bool {
        self.express
    }

    /// Create from config with explicit model list.
    pub fn from_config(
        project: String,
        location: String,
        models: Vec<String>,
        base_url: Option<String>,
    ) -> Self {
        let base_url = base_url.unwrap_or_else(|| default_vertex_base_url(&location));
        let inner = GeminiProvider::new(None).with_wire(WireApi::GoogleVertex);
        let model_list = models
            .iter()
            .map(|id| {
                ModelInfo::new(
                    id,
                    id,
                    WireApi::GoogleVertex,
                    ModelCapabilities::new(1_000_000, 65536)
                        .with_images(true)
                        .with_streaming(true),
                )
            })
            .collect();
        Self {
            provider_identity: "vertex".into(),
            project,
            location,
            base_url,
            models: model_list,
            inner,
            client: Arc::new(HttpClient::new()),
            native_options: None,
            express: false,
        }
    }

    /// Replace the HTTP client (for shared connection pooling).
    pub fn with_client(self, client: Arc<HttpClient>) -> Self {
        Self { client, ..self }
    }

    /// Bind capture and replay to the registered canonical provider identity.
    #[cfg(feature = "builtins")]
    pub(crate) fn with_provider_identity(mut self, provider: &str) -> Self {
        self.provider_identity = provider.to_owned();
        self.inner = self.inner.with_provider_identity(provider);
        self
    }

    /// Override the model records (task 20.41): direct adapters and tests
    /// bind complete custom records; the builtin catalog stays untouched.
    pub fn with_models(mut self, models: Vec<ModelInfo>) -> Self {
        self.inner = self.inner.with_models(models);
        self.models = self.inner.models().to_vec();
        self
    }

    /// Bind typed native options for one dispatch (task 20.41): the tool and
    /// thinking intents project onto the shared Google body engine and an
    /// explicit `project`/`location` re-routes the ADC URL (inert under the
    /// Express API-key route, which carries neither).
    pub fn with_native_options(mut self, options: GoogleVertexOptions) -> Self {
        if let Some(project) = options.project.as_ref() {
            self.project = project.clone();
        }
        if let Some(location) = options.location.as_ref() {
            self.location = location.clone();
        }
        self.native_options = Some(options);
        self
    }
}

/// Default Vertex base URL for a location: `global` pins the shared domain;
/// every other location gets its regional subdomain.
fn default_vertex_base_url(location: &str) -> String {
    if location == "global" {
        "https://aiplatform.googleapis.com".to_owned()
    } else {
        format!("https://{location}-aiplatform.googleapis.com")
    }
}

/// Built-in Vertex model metadata without credentials or HTTP construction.
pub fn model_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo::new(
            "gemini-2.5-flash",
            "Gemini 2.5 Flash (Vertex)",
            WireApi::GoogleVertex,
            ModelCapabilities::new(1_000_000, 65536)
                .with_images(true)
                .with_streaming(true),
        ),
        ModelInfo::new(
            "gemini-2.5-pro",
            "Gemini 2.5 Pro (Vertex)",
            WireApi::GoogleVertex,
            ModelCapabilities::new(1_000_000, 65536)
                .with_images(true)
                .with_streaming(true),
        ),
    ]
}
