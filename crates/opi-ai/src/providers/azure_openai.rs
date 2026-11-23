//! Azure OpenAI Provider profile.
//!
//! Endpoint/resource/deployment resolution and catalog construction live here.
//! The wire implementation is owned by the canonical Azure API adapter.

use std::fmt;
use std::sync::{Arc, OnceLock};

use crate::auth::{AuthContext, AuthScheme, ResolvedAuth};
use crate::catalog::{CatalogModel, CatalogOperation};
use crate::http::HttpClient;
use crate::model_info::OpenAiCompletionsCompat as CompatConfig;
use crate::model_info::WireApi;
use crate::provider::operation::SimpleOptions;
use crate::provider::routes::{AzureRouteContext, OpenAiConfiguredRoute};
use crate::provider::{
    EventStream, ModelInfo, Provider, ProviderError, ProviderErrorSummary, Request,
};
use crate::registry::ModelCapabilities;

const AZURE_API_VERSION: &str = "v1";
const AZURE_HOST_SUFFIXES: [&str; 3] = [
    ".openai.azure.com",
    ".cognitiveservices.azure.com",
    ".ai.azure.com",
];
const AZURE_ROOT_PATHS: [&str; 5] = ["", "/", "/openai", "/openai/v1", "/openai/v1/responses"];

fn azure_config_error(message: &'static str) -> ProviderError {
    ProviderError::Config(ProviderErrorSummary::attested_static(message))
}

/// Resolve and normalize an Azure route base URL.
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
                "Azure OpenAI base URL is required. Configure an Azure endpoint or set AZURE_OPENAI_BASE_URL or AZURE_OPENAI_RESOURCE_NAME.",
            )
        })?;
    normalize_azure_base_url(&resolved)
}

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
    if !AZURE_HOST_SUFFIXES
        .iter()
        .any(|suffix| host.ends_with(suffix))
    {
        return Ok(trimmed.to_owned());
    }
    let path = url.path().trim_end_matches('/');
    if AZURE_ROOT_PATHS.contains(&path) {
        return Ok(format!("{}://{host}", url.scheme()));
    }
    Ok(trimmed.to_owned())
}

/// Resolve the Azure API version used by a provider factory.
pub fn resolve_azure_api_version(explicit: Option<&str>) -> String {
    explicit
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(AZURE_API_VERSION)
        .to_owned()
}

fn parse_deployment_map(value: Option<&str>) -> std::collections::HashMap<String, String> {
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

/// Chat metadata from the reviewed Azure catalog.
pub fn model_catalog() -> Vec<ModelInfo> {
    static MODELS: OnceLock<Vec<ModelInfo>> = OnceLock::new();
    MODELS
        .get_or_init(|| {
            let entries: Vec<serde_json::Value> = serde_json::from_slice(include_bytes!(
                "../../resources/catalog/providers/azure.json"
            ))
            .expect("validated Azure OpenAI provider catalog");
            entries
                .iter()
                .map(|entry| CatalogModel::parse("azure", entry).expect("validated Azure record"))
                .filter(|entry| entry.operation == CatalogOperation::Chat)
                .map(|entry| {
                    entry
                        .to_chat_model_info()
                        .expect("validated Azure chat record")
                })
                .collect()
        })
        .clone()
}

/// Resolve the wire model name from explicit deployment, then the configured
/// deployment map, then the catalog ID. Lookup and results use the catalog ID.
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

/// Azure's catalog-backed Chat and Responses route.
pub struct AzureCatalogProvider {
    models: Vec<ModelInfo>,
    context: Arc<dyn AuthContext>,
    client: Arc<HttpClient>,
    api_version: Option<String>,
    endpoint: Option<String>,
}

impl AzureCatalogProvider {
    /// Build the reviewed Azure route. An explicit endpoint overrides per-call
    /// endpoint discovery. Model metadata is available before resolving auth.
    pub fn new(
        endpoint: Option<String>,
        api_version: Option<String>,
        context: Arc<dyn AuthContext>,
        client: Arc<HttpClient>,
    ) -> Result<Self, ProviderError> {
        let endpoint = endpoint
            .as_deref()
            .map(|base| resolve_azure_base_url(Some(base), None, None))
            .transpose()?;
        Ok(Self {
            endpoint,
            ..Self::from_models(model_catalog(), context, client, api_version)
        })
    }

    pub(crate) fn from_models(
        models: Vec<ModelInfo>,
        context: Arc<dyn AuthContext>,
        client: Arc<HttpClient>,
        api_version: Option<String>,
    ) -> Self {
        Self {
            models,
            context,
            client,
            api_version,
            endpoint: None,
        }
    }
}

impl Provider for AzureCatalogProvider {
    fn id(&self) -> &str {
        "azure"
    }
    fn models(&self) -> &[ModelInfo] {
        &self.models
    }
    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        _: &SimpleOptions,
    ) -> Result<(), ProviderError> {
        if let Some(base) = model.base_url.as_deref().filter(|base| !base.is_empty()) {
            resolve_azure_base_url(Some(base), None, None)?;
        }
        if !matches!(
            model.wire_api,
            WireApi::OpenAiCompletions | WireApi::AzureOpenAiResponses
        ) {
            return Err(ProviderError::Config(
                ProviderErrorSummary::attested_static(
                    "Azure builtin models require Chat or Azure Responses",
                ),
            ));
        }
        Ok(())
    }
    fn stream_prepared(&self, request: Request, auth: ResolvedAuth) -> EventStream {
        let model_id = request
            .model
            .strip_prefix("azure:")
            .unwrap_or(&request.model);
        let Some(model) = self
            .models
            .iter()
            .find(|model| model.id == model_id)
            .cloned()
        else {
            let error = ProviderError::UnknownModel {
                provider_id: "azure".into(),
                model_id: model_id.into(),
            };
            return Box::pin(futures_util::stream::once(async move { Err(error) }));
        };
        self.stream_model_resource_attempt(
            model,
            Default::default(),
            request,
            auth,
            crate::provider::AttemptContext {
                invocation_id: uuid::Uuid::new_v4().to_string(),
                attempt: 1,
                max_attempts: 1,
                payload_variant: crate::provider::PayloadVariant::Ordinary,
            },
            None,
        )
    }
    fn prepare_model_call(
        &self,
        model: &mut ModelInfo,
        auth: &mut ResolvedAuth,
    ) -> Result<Option<Arc<dyn Provider>>, ProviderError> {
        let env = |name: &str| {
            if let Some(value) = auth.env.get(name) {
                return Ok(Some(secrecy::ExposeSecret::expose_secret(value).to_owned()));
            }
            self.context.env(name).map_err(|error| {
                ProviderError::Config(ProviderErrorSummary::from_untrusted(error.to_string()))
            })
        };
        let base = match &self.endpoint {
            Some(endpoint) => Some(endpoint.clone()),
            None => env("AZURE_OPENAI_BASE_URL")?,
        };
        let resource = if base.as_deref().is_none_or(|base| base.trim().is_empty()) {
            env("AZURE_OPENAI_RESOURCE_NAME")?
        } else {
            None
        };
        let endpoint = resolve_azure_base_url(
            base.as_deref(),
            resource.as_deref(),
            model.base_url.as_deref(),
        )?;
        let version = match &self.api_version {
            Some(version) => Some(version.clone()),
            None => env("AZURE_OPENAI_API_VERSION")?,
        };
        let deployment_map = env("AZURE_OPENAI_DEPLOYMENT_NAME_MAP")?;
        model.base_url = Some(endpoint.clone());
        if model.wire_api == WireApi::AzureOpenAiResponses {
            let provider =
                AzureOpenAIProvider::for_records(Some(endpoint), vec![model.clone()], version)?
                    .with_client(self.client.clone())
                    .with_deployment_map(deployment_map);
            return Ok(Some(Arc::new(provider)));
        }
        // Resolve one source route before the acquisition owner starts. Every
        // attempt reuses this adapter, endpoint and deployment snapshot.
        let mut url = url::Url::parse(&endpoint).expect("validated Azure endpoint");
        if url.host_str().is_some_and(|host| {
            [
                ".openai.azure.com",
                ".cognitiveservices.azure.com",
                ".ai.azure.com",
            ]
            .iter()
            .any(|suffix| host.ends_with(suffix))
        }) && url.path().trim_matches('/').is_empty()
        {
            url.set_path("/openai/v1");
        }
        model.base_url = Some(url.to_string().trim_end_matches('/').into());
        let deployment = resolve_deployment_name(None, deployment_map.as_deref(), &model.id);
        auth.scheme = AuthScheme::Bearer;
        let provider = OpenAiConfiguredRoute::for_route(
            model.base_url.clone(),
            "azure".into(),
            Default::default(),
            vec![model.clone()],
            self.client.clone(),
        )
        .with_wire_model(deployment);
        Ok(Some(Arc::new(provider)))
    }
    fn stream_model_resource_attempt(
        &self,
        mut model: ModelInfo,
        options: SimpleOptions,
        request: Request,
        mut auth: ResolvedAuth,
        attempt: crate::provider::AttemptContext,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        // Direct prepared dispatch has no collection preparation owner.
        match self.prepare_model_call(&mut model, &mut auth) {
            Ok(Some(provider)) => provider
                .stream_model_resource_attempt(model, options, request, auth, attempt, resources),
            Ok(None) => unreachable!("Azure always freezes a concrete wire adapter"),
            Err(error) => Box::pin(futures_util::stream::once(async move { Err(error) })),
        }
    }
    fn replace_model_catalog(&mut self, models: Vec<ModelInfo>) -> Result<(), ProviderError> {
        self.models = models;
        Ok(())
    }
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
        let inner = OpenAiConfiguredRoute::new_for_profile(
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
        let inner = OpenAiConfiguredRoute::new_for_profile(
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
        let inner = OpenAiConfiguredRoute::new_for_profile(
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
        self.inner = OpenAiConfiguredRoute::new_for_profile(
            self.endpoint.clone(),
            "azure".into(),
            compat,
            vec![],
            vec![],
        );
        self
    }

    /// Resolve a per-call endpoint using the provider-owned Azure rules.
    pub(crate) fn resolve_endpoint(
        &self,
        explicit_base_url: Option<&str>,
        resource_name: Option<&str>,
        model_base_url: Option<&str>,
    ) -> Result<String, ProviderError> {
        resolve_azure_base_url(
            explicit_base_url,
            resource_name,
            model_base_url.or(Some(self.endpoint.as_str())),
        )
    }

    /// Resolve the deployment used on the wire for one catalog model.
    pub(crate) fn deployment_for(&self, explicit: Option<&str>, model_id: &str) -> String {
        if let Some(explicit) = explicit.map(str::trim).filter(|value| !value.is_empty()) {
            return explicit.to_owned();
        }
        parse_deployment_map(self.deployment_map.as_deref())
            .get(model_id)
            .cloned()
            .unwrap_or_else(|| model_id.to_owned())
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
