//! Azure OpenAI Provider profile.
//!
//! Endpoint/resource/deployment resolution and catalog construction live here.
//! The wire implementation is owned by the canonical Azure API adapter.

use std::fmt;
use std::sync::Arc;

use crate::http::HttpClient;
use crate::model_info::WireApi;
use crate::provider::{ModelInfo, ProviderError, ProviderErrorSummary};
use crate::providers::openai::{CompatConfig, OpenAiChatProvider};
use crate::registry::ModelCapabilities;

/// Default Azure OpenAI API version (source: pi 1.3.0
/// `azure-openai-config.ts` `DEFAULT_AZURE_API_VERSION`).
const DEFAULT_API_VERSION: &str = "v1";

/// Azure hosts whose configured base URL is normalized to the resource root
/// (opi's builder appends the `/openai/...` path segments itself).
const AZURE_HOST_SUFFIXES: [&str; 3] = [
    ".openai.azure.com",
    ".cognitiveservices.azure.com",
    ".ai.azure.com",
];

/// Base-path shapes treated as "the resource root" on an Azure host.
const AZURE_ROOT_PATHS: [&str; 5] = ["", "/", "/openai", "/openai/v1", "/openai/v1/responses"];

/// Resolve the Azure base URL (task 20.38, pinned source precedence):
/// explicit base URL, then resource name, then the model's base URL; a
/// typed failure when none is configured. Azure hosts carrying one of the
/// known `/openai[/v1][/responses]` path shapes normalize to the resource
/// root; non-Azure hosts keep their configured path.
pub fn resolve_azure_base_url(
    explicit_base_url: Option<&str>,
    resource_name: Option<&str>,
    model_base_url: Option<&str>,
) -> Result<String, ProviderError> {
    let explicit = explicit_base_url
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let resolved = explicit
        .map(str::to_owned)
        .or_else(|| {
            resource_name
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|resource| format!("https://{resource}.openai.azure.com"))
        })
        .or_else(|| {
            model_base_url
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
        })
        .ok_or_else(|| {
            azure_config_error(
                "Azure OpenAI base URL is required. Set it via config [providers.azure] endpoint or AZURE_OPENAI_ENDPOINT env var.",
            )
        })?;

    normalize_azure_base_url(&resolved)
}

/// Normalize a resolved base URL: strip the trailing slash and, on Azure
/// hosts, collapse the known `/openai[/v1][/responses]` path shapes to the
/// resource root. Invalid URLs fail typed.
fn normalize_azure_base_url(base_url: &str) -> Result<String, ProviderError> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(azure_config_error(
            "Azure OpenAI base URL must not be empty.",
        ));
    }
    let url = url::Url::parse(trimmed).map_err(|_| {
        ProviderError::Config(ProviderErrorSummary::sanitized(format!(
            "Invalid Azure OpenAI base URL: {trimmed}"
        )))
    })?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(azure_config_error(
            "Azure OpenAI requires an absolute HTTP or HTTPS API base URL.",
        ));
    }
    let host = url.host_str().unwrap_or_default();
    let is_azure_host = AZURE_HOST_SUFFIXES
        .iter()
        .any(|suffix| host.ends_with(suffix));
    if !is_azure_host {
        return Ok(trimmed.to_owned());
    }
    let path = url.path().trim_end_matches('/');
    if AZURE_ROOT_PATHS.contains(&path) {
        return Ok(format!("{}://{host}", url.scheme()));
    }
    Ok(trimmed.to_owned())
}

pub(crate) fn responses_url(endpoint: &str, api_version: &str) -> String {
    let mut url = url::Url::parse(endpoint).expect("Azure endpoint was validated at resolution");
    let path = url.path().trim_end_matches('/');
    let standard_resource = url.host_str().is_some_and(|host| {
        AZURE_HOST_SUFFIXES
            .iter()
            .any(|suffix| host.ends_with(suffix))
    }) && AZURE_ROOT_PATHS.contains(&path);
    let path = if standard_resource {
        "/openai/v1/responses".to_owned()
    } else if path.ends_with("/responses") {
        path.to_owned()
    } else {
        format!("{path}/responses")
    };
    url.set_path(&path);
    url.query_pairs_mut()
        .append_pair("api-version", api_version);
    url.into()
}

/// Resolve the request api-version: explicit value wins, else the pinned
/// source default `v1`.
pub fn resolve_azure_api_version(explicit: Option<&str>) -> String {
    explicit
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_API_VERSION)
        .to_owned()
}

/// Parse a `modelId=deployment,modelId2=deployment2` map (source:
/// `AZURE_OPENAI_DEPLOYMENT_NAME_MAP`).
pub(crate) fn parse_deployment_map(
    value: Option<&str>,
) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let Some(value) = value else { return map };
    for entry in value.split(',') {
        let Some((model_id, deployment)) = entry.trim().split_once('=') else {
            continue;
        };
        let (model_id, deployment) = (model_id.trim(), deployment.trim());
        if model_id.is_empty() || deployment.is_empty() {
            continue;
        }
        map.insert(model_id.to_owned(), deployment.to_owned());
    }
    map
}

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

fn azure_config_error(message: &'static str) -> ProviderError {
    ProviderError::Config(ProviderErrorSummary::attested_static(message))
}

/// Azure OpenAI service profile.
///
/// The profile composes the shared OpenAI Chat adapter and stores immutable
/// endpoint, deployment and route metadata. Wire transport and
/// request-scoped options are implemented by the canonical Azure API adapter.
#[derive(Clone)]
pub struct AzureOpenAIProvider {
    pub(crate) endpoint: String,
    pub(crate) api_version: String,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) inner: OpenAiChatProvider,
    pub(crate) client: Arc<HttpClient>,
    /// Optional `modelId=deployment` map (task 20.38): explicit per-call
    /// deployment options win, then this map, then the model id.
    pub(crate) deployment_map: Option<String>,
}

impl fmt::Debug for AzureOpenAIProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AzureOpenAIProvider")
            .field("endpoint", &self.endpoint)
            .field("api_version", &self.api_version)
            .field("models", &self.models.len())
            .finish()
    }
}

impl AzureOpenAIProvider {
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
        let inner = OpenAiChatProvider::new_for_profile(
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
        let inner = OpenAiChatProvider::new_for_profile(
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
        let inner = OpenAiChatProvider::new_for_profile(
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
        self.inner = OpenAiChatProvider::new_for_profile(
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
