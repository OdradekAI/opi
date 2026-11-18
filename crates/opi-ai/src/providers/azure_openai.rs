//! Azure OpenAI Provider profile.
//!
//! Endpoint/resource/deployment resolution and catalog construction live here.
//! The wire implementation is owned by the canonical Azure API adapter.

use std::fmt;
use std::sync::Arc;

use crate::http::HttpClient;
use crate::model_info::OpenAiCompletionsCompat as CompatConfig;
use crate::model_info::WireApi;
use crate::provider::routes::{
    AzureRouteContext, OpenAiRouteContext, parse_deployment_map, responses_url,
};
use crate::provider::{ModelInfo, ProviderError, ProviderErrorSummary};
use crate::registry::ModelCapabilities;

pub use crate::provider::routes::{resolve_azure_api_version, resolve_azure_base_url};

/// Resolve the deployment name for one model (pinned source precedence):
/// explicit deployment, then the configured deployment map, then the model
/// id. The deployment rides the request body's `model` field; Chat also
/// includes it in the URL path. The catalog id never changes.
pub fn resolve_deployment_name(
    explicit: Option<&str>,
    deployment_map: Option<&str>,
    model_id: &str,
) -> String {
    if let Some(explicit) = explicit.map(str::trim).filter(|value| !value.is_empty()) {
        return explicit.to_owned();
    }
    parse_deployment_map(deployment_map)
        .get(model_id)
        .cloned()
        .unwrap_or_else(|| model_id.to_owned())
}

/// Canonical provider-facing name for the neutral Azure route context.
pub type AzureOpenAIProvider = AzureRouteContext;

impl fmt::Debug for AzureRouteContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AzureOpenAIProvider")
            .field("endpoint", &self.endpoint)
            .field("api_version", &self.api_version)
            .field("models", &self.models.len())
            .finish()
    }
}

impl AzureRouteContext {
    /// Create a new Azure OpenAI provider.
    ///
    /// `deployment` is a default deployment name (used for model list display).
    /// The actual deployment is resolved from the model spec `azure:<deployment>`.
    pub fn new(
        endpoint: Option<String>,
        deployment: String,
        api_version: Option<String>,
    ) -> Result<Self, ProviderError> {
        let endpoint = resolve_azure_base_url(endpoint.as_deref(), None, None)?;
        let api_version = resolve_azure_api_version(api_version.as_deref());
        if deployment.trim().is_empty() {
            return Err(ProviderError::Config(
                ProviderErrorSummary::attested_static("Azure OpenAI deployment must not be empty"),
            ));
        }
        let models = vec![deployment_model(&deployment)];
        let inner = OpenAiRouteContext::new_for_profile(
            endpoint.clone(),
            "azure".into(),
            CompatConfig::default(),
            vec![],
            vec![],
        );
        Ok(Self {
            endpoint,
            api_version,
            models,
            inner,
            client: Arc::new(HttpClient::new()),
            deployment_map: None,
        })
    }

    /// Create from config with explicit deployment names for the model list.
    pub fn from_config(
        endpoint: Option<String>,
        deployments: Vec<String>,
        api_version: Option<String>,
    ) -> Result<Self, ProviderError> {
        let endpoint = resolve_azure_base_url(endpoint.as_deref(), None, None)?;
        let api_version = resolve_azure_api_version(api_version.as_deref());
        if deployments.is_empty()
            || deployments
                .iter()
                .any(|deployment| deployment.trim().is_empty())
        {
            return Err(ProviderError::Config(
                ProviderErrorSummary::attested_static(
                    "Azure OpenAI deployments must contain only non-empty deployment names",
                ),
            ));
        }
        let models = deployments
            .iter()
            .map(|deployment| deployment_model(deployment))
            .collect();
        let inner = OpenAiRouteContext::new_for_profile(
            endpoint.clone(),
            "azure".into(),
            CompatConfig::default(),
            vec![],
            vec![],
        );
        Ok(Self {
            endpoint,
            api_version,
            models,
            inner,
            client: Arc::new(HttpClient::new()),
            deployment_map: None,
        })
    }

    /// Create from explicit model records (task 20.38): each record's own
    /// wire selects the transport (chat or responses). The deployment for
    /// one dispatch resolves from the per-call options, the deployment
    /// map, then the record id.
    pub fn for_records(
        endpoint: Option<String>,
        models: Vec<ModelInfo>,
        api_version: Option<String>,
    ) -> Result<Self, ProviderError> {
        let endpoint = resolve_azure_base_url(endpoint.as_deref(), None, None)?;
        let api_version = resolve_azure_api_version(api_version.as_deref());
        if models.is_empty()
            || models
                .iter()
                .any(|model| model.id.trim().is_empty() || model.validate().is_err())
        {
            return Err(ProviderError::Config(
                ProviderErrorSummary::attested_static(
                    "Azure OpenAI model records must be complete and non-empty",
                ),
            ));
        }
        let inner = OpenAiRouteContext::new_for_profile(
            endpoint.clone(),
            "azure".into(),
            CompatConfig::default(),
            vec![],
            vec![],
        );
        Ok(Self {
            endpoint,
            api_version,
            models,
            inner,
            client: Arc::new(HttpClient::new()),
            deployment_map: None,
        })
    }

    /// Replace the HTTP client (for shared connection pooling).
    pub fn with_client(self, client: Arc<HttpClient>) -> Self {
        Self { client, ..self }
    }

    /// Apply an OpenAI-compatible profile compat config to the inner shared
    /// adapter. Azure is a first-class provider that
    /// routes request-body serialization through the shared OpenAI Chat
    /// adapter, so it honors the same compat flags (developer role, strict
    /// tool schema, max-tokens field, tool-result name) as config-driven
    /// profiles.
    pub fn with_compat(mut self, compat: CompatConfig) -> Self {
        self.inner = OpenAiRouteContext::new_for_profile(
            self.endpoint.clone(),
            "azure".into(),
            compat,
            vec![],
            vec![],
        );
        self
    }

    /// Build the Azure deployment URL for a given deployment name.
    pub fn build_azure_url(&self, deployment: &str) -> String {
        format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.endpoint, deployment, self.api_version
        )
    }

    /// Build the Azure Responses URL. Deployment belongs in `body.model`.
    pub fn build_azure_responses_url(&self, _deployment: &str) -> String {
        responses_url(&self.endpoint, &self.api_version)
    }

    /// Set the optional `modelId=deployment` map used when a dispatch has
    /// no explicit deployment option (task 20.38).
    pub fn with_deployment_map(mut self, map: Option<String>) -> Self {
        self.deployment_map = map;
        self
    }
}

fn deployment_model(deployment: &str) -> ModelInfo {
    ModelInfo::new(
        deployment,
        deployment,
        WireApi::AzureOpenAiCompletions,
        ModelCapabilities::new(128000, 16384)
            .with_images(true)
            .with_streaming(true),
    )
}
