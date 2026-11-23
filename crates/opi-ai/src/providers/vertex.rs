//! Google Vertex AI provider profile.
//!
//! Endpoint, route identity, project/location configuration, model catalog and
//! shared HTTP construction live here. Vertex request framing and transport
//! live in [`crate::api::vertex`].

use std::fmt;
use std::sync::{Arc, OnceLock};

use crate::catalog::CatalogModel;
use crate::http::HttpClient;
use crate::model_info::WireApi;
use crate::provider::ModelInfo;
use crate::provider::routes::{GeminiRouteContext, VertexRouteContext};
use crate::registry::ModelCapabilities;

/// Canonical provider-facing name for the neutral Vertex route context.
pub type VertexProvider = VertexRouteContext;

/// Build the Vertex publisher-model collection for the selected auth scope.
///
/// Endpoint selection is provider configuration. Wire adapters consume the
/// resolved route through `VertexRouteContext` and do not select hosts.
pub(crate) fn vertex_collection_url(
    base_url: Option<&str>,
    project_location: Option<(&str, &str)>,
) -> String {
    let base = base_url.map(str::to_owned).unwrap_or_else(|| {
        match project_location.map(|(_, location)| location) {
            None | Some("global") => "https://aiplatform.googleapis.com".into(),
            Some(location @ ("us" | "eu")) => {
                format!("https://aiplatform.{location}.rep.googleapis.com")
            }
            Some(location) => format!("https://{location}-aiplatform.googleapis.com"),
        }
    });
    let base = base.trim_end_matches('/');
    match project_location {
        Some((project, location)) => {
            format!("{base}/v1/projects/{project}/locations/{location}/publishers/google/models")
        }
        None => format!("{base}/v1/publishers/google/models"),
    }
}

impl fmt::Debug for VertexRouteContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VertexProvider")
            .field("project", &self.project)
            .field("location", &self.location)
            .field("models", &self.inner.models.len())
            .finish()
    }
}

impl VertexRouteContext {
    /// Create a new Vertex AI provider (ADC/Bearer mode with project and
    /// location).
    pub fn new(project: String, location: String, base_url: Option<String>) -> Self {
        Self::from_models(
            "vertex",
            Some((project, location)),
            base_url,
            model_catalog(),
            Arc::new(HttpClient::new()),
        )
    }

    /// Create the API-key Express provider (no project/location): requests
    /// go to `{base}/v1/publishers/google/models/{model}:streamGenerateContent`
    /// with the `x-goog-api-key` header.
    pub fn express(base_url: Option<String>) -> Self {
        Self::from_models(
            "vertex",
            None,
            base_url,
            model_catalog(),
            Arc::new(HttpClient::new()),
        )
    }

    /// Whether the factory configured a projectless route.
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
        let models = models
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
        Self::from_models(
            "vertex",
            Some((project, location)),
            base_url,
            models,
            Arc::new(HttpClient::new()),
        )
    }

    pub(crate) fn from_models(
        provider_identity: &str,
        project_location: Option<(String, String)>,
        base_url: Option<String>,
        models: Vec<ModelInfo>,
        client: Arc<HttpClient>,
    ) -> Self {
        let express = project_location.is_none();
        let (project, location) = project_location.unwrap_or_default();
        Self {
            project,
            location,
            base_url,
            inner: GeminiRouteContext::from_models(
                provider_identity,
                WireApi::GoogleVertex,
                None,
                models,
                client,
            ),
            express,
        }
    }

    /// Resolve the collection endpoint for the selected Express or ADC scope.
    pub(crate) fn collection_url(&self, project_location: Option<(&str, &str)>) -> String {
        vertex_collection_url(self.base_url.as_deref(), project_location)
    }

    /// Replace the HTTP client (for shared connection pooling).
    pub fn with_client(mut self, client: Arc<HttpClient>) -> Self {
        self.inner.client = client;
        self
    }

    /// Override the model records (task 20.41): direct adapters and tests
    /// bind complete custom records; the builtin catalog stays untouched.
    pub fn with_models(mut self, models: Vec<ModelInfo>) -> Self {
        self.inner = self.inner.with_models(models);
        self
    }
}

/// Chat metadata from the reviewed Vertex catalog.
pub fn model_catalog() -> Vec<ModelInfo> {
    static MODELS: OnceLock<Vec<ModelInfo>> = OnceLock::new();
    MODELS
        .get_or_init(|| {
            let entries: Vec<serde_json::Value> = serde_json::from_slice(include_bytes!(
                "../../resources/catalog/providers/google-vertex.json"
            ))
            .expect("validated Google Vertex AI provider catalog");
            entries
                .iter()
                .map(|entry| {
                    CatalogModel::parse("google-vertex", entry)
                        .and_then(|row| row.to_chat_model_info())
                        .expect("validated Vertex chat record")
                })
                .collect()
        })
        .clone()
}

/// Build the catalog-backed route registered as `google-vertex`.
///
/// A Bearer token requires a project and location. An API key uses Express.
pub fn google_vertex_provider(
    project_location: Option<(String, String)>,
    base_url: Option<String>,
    client: Arc<HttpClient>,
) -> VertexProvider {
    VertexProvider::from_models(
        "google-vertex",
        project_location,
        base_url,
        model_catalog(),
        client,
    )
}
