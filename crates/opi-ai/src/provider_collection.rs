//! The builtin provider collection (task 20.34).
//!
//! Turns the pinned [`crate::providers`] profile table plus the bundled
//! [`Catalog`] into registered providers on an existing [`Models`]
//! collection, reusing the completed wire adapters — one
//! [`crate::api_mapped::ApiMappedProvider`] per profile
//! over per-wire routes built from the existing constructors. Canonical ids
//! replace the legacy opi ids (`gemini`, `vertex`, `bedrock`,
//! `openai-responses`): nothing here registers a permanent alias. Source-backed
//! profiles remain registered while credentials and dynamic endpoint inputs
//! resolve lazily for each call.
//! Classifier and image builtins register through the typed classifier/image
//! seams with catalog records. Construction is data-only: no environment,
//! store, or network access happens here; env keys resolve at dispatch time
//! through the injected [`AuthContext`].

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::auth::{AuthContext, AuthProvenanceSource, AuthResolver, AuthScheme, ResolvedAuth};
use crate::catalog::{Catalog, CatalogModel, CatalogOperation};
use crate::classifier::ClassifierModelRecord;
use crate::http::HttpClient;
use crate::images::{ImageCostRates, ImageModelRecord, ImageOutputModality};
use crate::models::{AuthDescriptor, CompatMetadata, Models};
use crate::registry::RegistrationError;

use crate::model_info::{ModelInfo, ModelPricing, PricingTier, WireApi};
use crate::provider::{
    EventStream, ModelInfo as ProviderModelInfo, Provider, ProviderError, Request,
};
use crate::providers::{BuiltinEndpoint, BuiltinProviderProfile, builtin_provider_profiles};
use crate::stream::Pricing;

/// Per-construction configuration the builtin table cannot supply itself.
#[derive(Debug, Clone, Default)]
pub struct BuiltinOverrides {
    /// Azure API base; omitted values resolve from the injected source context.
    pub azure_endpoint: Option<String>,
    /// Azure API version (e.g. `2024-10-21`).
    pub azure_api_version: Option<String>,
    /// Vertex project fallback for `vertex_auth`.
    pub vertex_project: Option<String>,
    /// Vertex location (e.g. `us-central1`).
    pub vertex_location: Option<String>,
    /// Vertex source owner and endpoint overrides; fields here take precedence.
    pub vertex_auth: crate::auth::builtin_vertex::BuiltinVertexConfig,
    /// Bedrock source-chain inputs and isolated source transports.
    pub aws_auth: crate::auth::builtin_aws::BuiltinAwsConfig,
    /// Per-provider default base URL overrides (tests point profiles at
    /// local mock servers).
    pub base_urls: BTreeMap<String, String>,
}

/// What `register_builtin` registered and what it skipped, with reasons.
#[derive(Debug, Default, Clone)]
pub struct BuiltinReport {
    /// Canonical provider ids registered for chat dispatch/lookup.
    pub registered: Vec<String>,
    /// `(provider, reason)` pairs for profiles not registered.
    pub skipped: Vec<(String, String)>,
    /// Registered classifier API ids.
    pub classifier_apis: Vec<String>,
    /// Registered image API ids.
    pub image_apis: Vec<String>,
}

/// Typed failure of builtin construction.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BuiltinError {
    /// The bundled catalog is unavailable or invalid.
    #[error("builtin catalog unavailable: {0}")]
    Catalog(String),
    /// One catalog entry could not be converted.
    #[error("provider {provider} model {model}: {reason}")]
    InvalidModel {
        /// Owning provider id.
        provider: String,
        /// Model id.
        model: String,
        /// Why the entry is invalid.
        reason: String,
    },
    /// Registration into the collection failed.
    #[error("registration failed for {provider}: {source}")]
    Registration {
        /// Provider id.
        provider: String,
        /// Underlying registration error.
        #[source]
        source: RegistrationError,
    },
    /// A provider could not be constructed from its profile.
    #[error("provider {provider} construction failed: {message}")]
    Construction {
        /// Provider id.
        provider: String,
        /// Construction failure message.
        message: String,
    },
}

/// Env-key auth resolver for one builtin profile.
///
/// Reads through the injected [`AuthContext`] (never the process
/// environment directly) and resolves the profile's scheme. An unset or
/// empty variable is the typed missing-credential signal, so availability
/// checks report the provider as unconfigured instead of failing the call.
#[derive(Clone)]
pub struct BuiltinEnvAuthResolver {
    context: Arc<dyn AuthContext>,
    env_var: String,
    scheme: AuthScheme,
}

impl BuiltinEnvAuthResolver {
    /// Build a resolver reading `env_var` through `context` as `scheme`.
    pub fn new(
        context: Arc<dyn AuthContext>,
        env_var: impl Into<String>,
        scheme: AuthScheme,
    ) -> Self {
        Self {
            context,
            env_var: env_var.into(),
            scheme,
        }
    }
}

impl AuthResolver for BuiltinEnvAuthResolver {
    fn resolve<'a>(
        &'a self,
    ) -> crate::credential::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        let scheme = self.scheme.clone();
        let env_var = self.env_var.clone();
        let value = self.context.env(&env_var);
        Box::pin(async move {
            match value {
                Ok(Some(secret)) => Ok(ResolvedAuth {
                    env: Default::default(),
                    scheme,
                    secret: secrecy::SecretString::from(secret),
                    base_url: None,
                    account_id: None,
                    provenance: crate::auth::AuthProvenance {
                        source: AuthProvenanceSource::Environment {
                            name: env_var.clone(),
                        },
                        fallback: crate::auth::AuthFallback::NotAttempted,
                    },
                }),
                Ok(None) => Err(ProviderError::CredentialNeeded {
                    provider_id: env_var.clone(),
                }),
                Err(error) => Err(ProviderError::Config(
                    crate::provider::ProviderErrorSummary::sanitized(format!(
                        "auth context read failed for {env_var}: {error}"
                    )),
                )),
            }
        })
    }
}

/// A route wrapper republishing one concrete provider under its canonical
/// profile id with the profile's catalog subset.
///
/// The wrapped adapters with fixed internal ids (`gemini`, `vertex`,
/// `bedrock`) stay reusable as-is. Profile authentication and session headers
/// are applied before dispatch; a prepared replacement retains those policies
/// and the selected model snapshot. Dispatch, resource and refresh ownership
/// remains with the inner provider.
struct CanonicalRoute {
    id: String,
    models: Vec<ModelInfo>,
    inner: Arc<dyn Provider>,
    builtin_auth: bool,
}

impl std::fmt::Debug for CanonicalRoute {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CanonicalRoute")
            .field("id", &self.id)
            .field("models", &self.models.len())
            .finish_non_exhaustive()
    }
}

impl CanonicalRoute {
    /// Republish the model spec under the inner adapter's historical id:
    /// prefix-sensitive adapters (Bedrock model families) parse their own
    /// `id:` prefix, so the canonical spec must not leak through.
    fn wire_auth(&self, wire: Option<WireApi>, mut auth: ResolvedAuth) -> ResolvedAuth {
        if self.builtin_auth
            && !matches!(auth.scheme, AuthScheme::AnthropicFederation)
            && matches!(
                auth.provenance.source,
                AuthProvenanceSource::CredentialStore { .. }
                    | AuthProvenanceSource::Environment { .. }
            )
        {
            let header_owned = matches!(&auth.provenance.source, AuthProvenanceSource::Environment { name } if name == "ANTHROPIC_AUTH_TOKEN");
            if !header_owned
                && (wire == Some(WireApi::GoogleGenerativeAi)
                    || (wire == Some(WireApi::AnthropicMessages)
                        && self.id != "github-copilot"
                        && self.id != "cloudflare-ai-gateway"))
            {
                auth.scheme = AuthScheme::ApiKey;
            }
        }
        auth
    }
    fn request_wire(&self, request: &Request) -> Option<WireApi> {
        let id = request
            .model
            .strip_prefix(&format!("{}:", self.id))
            .unwrap_or(&request.model);
        self.models
            .iter()
            .find(|model| model.id == id)
            .map(|model| model.wire_api)
    }
    fn rewrite_model_spec(&self, mut request: Request) -> Request {
        if self.builtin_auth
            && matches!(self.id.as_str(), "opencode" | "opencode-go")
            && !request
                .extra_headers
                .iter()
                .any(|(name, _)| name.eq_ignore_ascii_case("x-opencode-session"))
            && let Some(session) = request
                .session_id
                .as_ref()
                .filter(|session| !session.is_empty())
        {
            request
                .extra_headers
                .push(("x-opencode-session".into(), session.clone()));
        }
        let canonical_prefix = format!("{}:", self.id);
        if let Some(rest) = request.model.strip_prefix(&canonical_prefix) {
            request.model = format!("{}:{rest}", self.inner.id());
        }
        request
    }
}

impl Provider for CanonicalRoute {
    fn prepare_model_call(
        &self,
        model: &mut ModelInfo,
        auth: &mut ResolvedAuth,
    ) -> Result<Option<Arc<dyn Provider>>, ProviderError> {
        *auth = self.wire_auth(Some(model.wire_api), auth.clone());
        Ok(self.inner.prepare_model_call(model, auth)?.map(|inner| {
            Arc::new(Self {
                id: self.id.clone(),
                models: vec![model.clone()],
                inner,
                builtin_auth: self.builtin_auth,
            }) as Arc<dyn Provider>
        }))
    }

    fn validate_simple_options(
        &self,
        model: &ProviderModelInfo,
        options: &crate::models::SimpleOptions,
    ) -> Result<(), ProviderError> {
        self.inner.validate_simple_options(model, options)
    }
    fn id(&self) -> &str {
        &self.id
    }
    fn models(&self) -> &[ProviderModelInfo] {
        &self.models
    }
    fn stream_prepared(&self, request: Request, auth: ResolvedAuth) -> EventStream {
        let auth = self.wire_auth(self.request_wire(&request), auth);
        let request = self.rewrite_model_spec(request);
        self.inner.stream_prepared(request, auth)
    }
    fn stream_attempt(
        &self,
        request: Request,
        auth: ResolvedAuth,
        attempt: crate::provider::AttemptContext,
    ) -> EventStream {
        let auth = self.wire_auth(self.request_wire(&request), auth);
        let request = self.rewrite_model_spec(request);
        self.inner.stream_attempt(request, auth, attempt)
    }
    fn stream_resource_attempt(
        &self,
        request: Request,
        auth: ResolvedAuth,
        attempt: crate::provider::AttemptContext,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        let auth = self.wire_auth(self.request_wire(&request), auth);
        let request = self.rewrite_model_spec(request);
        self.inner
            .stream_resource_attempt(request, auth, attempt, resources)
    }
    fn stream_model_resource_attempt(
        &self,
        model: ProviderModelInfo,
        options: crate::models::SimpleOptions,
        request: Request,
        auth: ResolvedAuth,
        attempt: crate::provider::AttemptContext,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        let auth = self.wire_auth(Some(model.wire_api), auth);
        let request = self.rewrite_model_spec(request);
        self.inner
            .stream_model_resource_attempt(model, options, request, auth, attempt, resources)
    }
    fn acquisition_policy_default(&self) -> crate::retry::AcquisitionPolicy {
        self.inner.acquisition_policy_default()
    }
    fn session_resources(&self) -> Option<&dyn crate::codex_resources::SessionResourceOwner> {
        self.inner.session_resources()
    }
    fn replace_model_catalog(&mut self, models: Vec<ModelInfo>) -> Result<(), ProviderError> {
        let inner = Arc::get_mut(&mut self.inner).ok_or_else(|| {
            ProviderError::Config(crate::provider::ProviderErrorSummary::attested_static(
                "cannot replace a shared canonical route catalog",
            ))
        })?;
        inner.replace_model_catalog(models.clone())?;
        self.models = models;
        Ok(())
    }
    fn refresh_models(
        &self,
        context: crate::catalog::CatalogRefreshContext,
    ) -> crate::credential::BoxAuthFuture<'_, Result<Option<Vec<ModelInfo>>, ProviderError>> {
        self.inner.refresh_models(context)
    }
}

/// Republish one concrete provider under its canonical profile id with the
/// profile's catalog subset (a no-op pass-through when the ids already
/// match). The coding agent uses this to keep its fixed-id adapters
/// (`gemini`/`vertex`/`bedrock`) dispatchable under canonical ids.
pub fn canonical_route(
    id: &str,
    models: Vec<ModelInfo>,
    inner: Box<dyn Provider>,
) -> Box<dyn Provider> {
    if inner.id() == id {
        inner
    } else {
        Box::new(CanonicalRoute {
            id: id.to_owned(),
            models,
            inner: Arc::from(inner),
            builtin_auth: false,
        })
    }
}

fn builtin_route(id: &str, models: Vec<ModelInfo>, inner: Box<dyn Provider>) -> Box<dyn Provider> {
    Box::new(CanonicalRoute {
        id: id.into(),
        models,
        inner: Arc::from(inner),
        builtin_auth: true,
    })
}

/// Azure's builtin profile uses its source Chat API as well as Responses.
/// The legacy direct Azure provider retains its separate deployment-URL API.
struct BuiltinAzureRoute {
    models: Vec<ModelInfo>,
    context: Arc<dyn AuthContext>,
    client: Arc<HttpClient>,
    api_version: Option<String>,
}

impl Provider for BuiltinAzureRoute {
    fn id(&self) -> &str {
        "azure"
    }
    fn models(&self) -> &[ModelInfo] {
        &self.models
    }
    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        _: &crate::models::SimpleOptions,
    ) -> Result<(), ProviderError> {
        if let Some(base) = model.base_url.as_deref().filter(|base| !base.is_empty()) {
            crate::azure_openai::resolve_azure_base_url(Some(base), None, None)?;
        }
        if !matches!(
            model.wire_api,
            WireApi::OpenAiCompletions | WireApi::AzureOpenAiResponses
        ) {
            return Err(ProviderError::Config(
                crate::provider::ProviderErrorSummary::attested_static(
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
                ProviderError::Config(crate::provider::ProviderErrorSummary::from_untrusted(
                    error.to_string(),
                ))
            })
        };
        let base = env("AZURE_OPENAI_BASE_URL")?;
        let resource = if base.as_deref().is_none_or(|base| base.trim().is_empty()) {
            env("AZURE_OPENAI_RESOURCE_NAME")?
        } else {
            None
        };
        let endpoint = crate::azure_openai::resolve_azure_base_url(
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
            let provider = crate::azure_openai::AzureOpenAIProvider::for_records(
                Some(endpoint),
                vec![model.clone()],
                version,
            )?
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
        let deployment = crate::azure_openai::resolve_deployment_name(
            None,
            deployment_map.as_deref(),
            &model.id,
        );
        auth.scheme = AuthScheme::Bearer;
        let provider = crate::openai_chat::OpenAiChatProvider::for_route(
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
        options: crate::models::SimpleOptions,
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

/// Catalog cost (base rates plus optional tiers) into typed pricing.
fn catalog_pricing(entry: &CatalogModel) -> Result<ModelPricing, BuiltinError> {
    let invalid = |reason: &str| BuiltinError::InvalidModel {
        provider: entry.provider.clone(),
        model: entry.id.clone(),
        reason: reason.to_owned(),
    };
    let read_rates = |value: &serde_json::Value| -> Option<Pricing> {
        Some(Pricing {
            input_cost_per_mtok: value.get("input")?.as_f64()?,
            output_cost_per_mtok: value.get("output")?.as_f64()?,
            cache_read_cost_per_mtok: value.get("cacheRead")?.as_f64()?,
            cache_write_cost_per_mtok: value.get("cacheWrite")?.as_f64()?,
        })
    };
    let raw_cost = entry
        .raw
        .get("cost")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let base = read_rates(&raw_cost).ok_or_else(|| invalid("cost rates are not finite numbers"))?;
    let mut tiers = Vec::new();
    if let Some(raw_tiers) = raw_cost.get("tiers").and_then(|tiers| tiers.as_array()) {
        for tier in raw_tiers {
            let input_tokens_above = tier
                .get("inputTokensAbove")
                .and_then(serde_json::Value::as_number)
                .filter(|threshold| threshold.as_f64().is_some_and(f64::is_finite))
                .cloned()
                .ok_or_else(|| invalid("tier threshold is not a finite number"))?;
            let pricing = read_rates(tier)
                .ok_or_else(|| invalid("tier cost rates are not finite numbers"))?;
            tiers.push(PricingTier {
                input_tokens_above,
                pricing,
            });
        }
    }
    let pricing = ModelPricing { base, tiers };
    pricing
        .validate_source_metadata()
        .map_err(|error| invalid(&error.to_string()))?;
    Ok(pricing)
}

/// One bundled chat entry into its complete registered [`ModelInfo`].
fn chat_model_info(
    entry: &CatalogModel,
    forced_base: Option<&str>,
) -> Result<ModelInfo, BuiltinError> {
    let invalid = |reason: String| BuiltinError::InvalidModel {
        provider: entry.provider.clone(),
        model: entry.id.clone(),
        reason,
    };
    let mut model = entry
        .to_chat_model_info()
        .map_err(|error| invalid(error.to_string()))?;
    model.capabilities.supports_streaming = true;
    if let Some(base) = forced_base {
        model.base_url = Some(base.to_owned());
    }
    Ok(model)
}

/// One bundled classifier entry into its complete record.
fn classifier_record(entry: &CatalogModel) -> Result<ClassifierModelRecord, BuiltinError> {
    let input = entry
        .input
        .iter()
        .map(|modality| match modality {
            crate::catalog::CatalogInputModality::Text => {
                crate::classifier::ClassifierInputModality::Text
            }
            crate::catalog::CatalogInputModality::Image => {
                crate::classifier::ClassifierInputModality::Image
            }
        })
        .collect();
    Ok(ClassifierModelRecord {
        headers: crate::catalog::wire::decode_record_headers(&entry.raw).map_err(|reason| {
            BuiltinError::InvalidModel {
                provider: entry.provider.clone(),
                model: entry.id.clone(),
                reason,
            }
        })?,
        input_limits: entry
            .raw
            .get("inputLimits")
            .filter(|value| !value.is_null())
            .cloned(),
        provider_id: entry.provider.clone(),
        id: entry.id.clone(),
        name: entry.name.clone(),
        api: crate::models::ApiId::new(&entry.api),
        base_url: entry.base_url.clone(),
        input,
        context_window: entry.context_window.unwrap_or(0),
        cost: entry
            .raw
            .get("cost")
            .filter(|cost| !cost.is_null())
            .map(|_| catalog_pricing(entry))
            .transpose()?,
    })
}

/// One bundled image entry into its complete record.
fn image_record(entry: &CatalogModel) -> Result<ImageModelRecord, BuiltinError> {
    Ok(ImageModelRecord {
        cost_tiers: crate::catalog::wire::decode_pricing_tiers(entry.raw.pointer("/cost/tiers"))
            .map_err(|reason| BuiltinError::InvalidModel {
                provider: entry.provider.clone(),
                model: entry.id.clone(),
                reason,
            })?,
        headers: crate::catalog::wire::decode_record_headers(&entry.raw).map_err(|reason| {
            BuiltinError::InvalidModel {
                provider: entry.provider.clone(),
                model: entry.id.clone(),
                reason,
            }
        })?,
        input_limits: entry
            .raw
            .get("inputLimits")
            .filter(|value| !value.is_null())
            .cloned(),
        input: entry.input.clone(),
        provider_id: entry.provider.clone(),
        id: entry.id.clone(),
        name: entry.name.clone(),
        api: crate::models::ApiId::new(&entry.api),
        base_url: entry.base_url.clone(),
        output: entry
            .raw
            .get("output")
            .and_then(serde_json::Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| match value.as_str() {
                        Some("image") => Some(ImageOutputModality::Image),
                        Some("text") => Some(ImageOutputModality::Text),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_else(|| vec![ImageOutputModality::Image]),
        cost: entry.cost.as_ref().map(|cost| ImageCostRates {
            input: cost.input,
            output: cost.output,
            cache_read: cost.cache_read,
            cache_write: cost.cache_write,
        }),
    })
}

fn openai_compatible(wires: &BTreeMap<WireApi, Vec<ModelInfo>>) -> bool {
    wires.keys().any(|wire| {
        matches!(
            wire,
            WireApi::OpenAiCompletions
                | WireApi::OpenAiResponses
                | WireApi::OpenAiCodexResponses
                | WireApi::MistralConversations
                | WireApi::PiMessages
        )
    })
}

/// Register every builtin profile from the bundled catalog onto `models`.
///
/// Chat profiles become [`crate::api_mapped::ApiMappedProvider`]
/// registrations over existing wire routes (or direct providers for the
/// single-wire adapters); classifier and image catalogs register through the
/// typed seams. Construction reads no environment, store, or network: env
/// keys resolve at dispatch through `context`.
pub fn register_builtin(
    models: &mut Models,
    context: Arc<dyn AuthContext>,
    client: Arc<HttpClient>,
    overrides: &BuiltinOverrides,
) -> Result<BuiltinReport, BuiltinError> {
    let catalog = Catalog::builtin().map_err(|error| BuiltinError::Catalog(error.to_string()))?;
    let mut report = BuiltinReport::default();

    for profile in builtin_provider_profiles() {
        register_chat_profile(
            models,
            catalog,
            profile,
            &context,
            &client,
            overrides,
            &mut report,
        )?;
        // A classifier-only profile may own the provider's default auth
        // management slot, but must not claim an existing custom chat route.
        models.mark_builtin_default_auth(profile.id, profile.classifier_only);
    }
    register_classifiers(models, catalog, &context, &client, overrides, &mut report)?;
    register_images(models, catalog, &context, &client, overrides, &mut report)?;
    for profile in builtin_provider_profiles() {
        let flow = models
            .oauth_flow_for_builtin(profile.id)
            .or_else(|| crate::auth::builtin::default_oauth_flow(profile.id));
        if let Some(flow) = &flow {
            models.register_oauth_flow(profile.id, flow.clone());
        }
        let factory = match profile.id {
            "google-vertex" => {
                let mut config = overrides.vertex_auth.clone();
                config.project = config.project.or_else(|| overrides.vertex_project.clone());
                config.location = config
                    .location
                    .or_else(|| overrides.vertex_location.clone());
                config.base_url = config
                    .base_url
                    .or_else(|| overrides.base_urls.get(profile.id).cloned());
                crate::auth::builtin_vertex::factory(context.clone(), config)
            }
            "amazon-bedrock" => {
                let mut config = overrides.aws_auth.clone();
                config.base_url = config
                    .base_url
                    .or_else(|| overrides.base_urls.get(profile.id).cloned());
                crate::auth::builtin_aws::factory(context.clone(), config)
            }
            _ => crate::auth::builtin::factory(profile, context.clone(), flow),
        };
        models.register_builtin_auth_factory(profile.id, factory);
    }
    Ok(report)
}

fn register_chat_profile(
    models: &mut Models,
    catalog: &Catalog,
    profile: &BuiltinProviderProfile,
    context: &Arc<dyn AuthContext>,
    client: &Arc<HttpClient>,
    overrides: &BuiltinOverrides,
    report: &mut BuiltinReport,
) -> Result<(), BuiltinError> {
    let id = profile.id;
    if profile.classifier_only {
        // typesafe ships only classifiers; the classifier seam registers it.
        report
            .skipped
            .push((id.into(), "classifier-only provider".into()));
        return Ok(());
    }
    match profile.endpoint {
        BuiltinEndpoint::AzureDeployment => {
            let records = chat_infos(
                catalog,
                profile,
                overrides
                    .azure_endpoint
                    .as_deref()
                    .or_else(|| overrides.base_urls.get(id).map(String::as_str)),
            )?;
            let provider = BuiltinAzureRoute {
                models: records,
                context: context.clone(),
                client: client.clone(),
                api_version: overrides.azure_api_version.clone(),
            };
            register_env_route(
                models,
                profile,
                Box::new(provider),
                context,
                CompatMetadata {
                    openai_compatible: true,
                    profile: Some(profile.name.into()),
                },
            )?;
            report.registered.push(id.into());
            return Ok(());
        }
        BuiltinEndpoint::VertexLocation => {
            let forced = overrides.base_urls.get(profile.id).map(String::as_str);
            let infos = chat_infos(catalog, profile, forced)?;
            let provider = crate::vertex::VertexProvider::new(
                overrides.vertex_project.clone().unwrap_or_default(),
                overrides.vertex_location.clone().unwrap_or_default(),
                profile_base_url(profile, overrides),
            )
            .with_models(infos)
            .with_provider_identity(id)
            .with_client(Arc::clone(client));
            let provider = builtin_route(id, provider_models(&provider), Box::new(provider));
            register_env_route(
                models,
                profile,
                provider,
                context,
                CompatMetadata::default(),
            )?;
            report.registered.push(id.into());
            return Ok(());
        }
        BuiltinEndpoint::BedrockRegion => {
            let forced = overrides.base_urls.get(profile.id).map(String::as_str);
            let infos = chat_infos(catalog, profile, forced)?;
            let provider = crate::bedrock::BedrockProvider::new(
                profile_base_url(profile, overrides),
                Arc::clone(client),
            )
            .with_models(infos)
            .with_provider_identity(id);
            let provider = builtin_route(id, provider_models(&provider), Box::new(provider));
            // The lazy source factory below binds the native chain after registration.
            models
                .register(
                    provider,
                    AuthDescriptor::Resolved {
                        source: profile
                            .credential_label
                            .unwrap_or("aws credential chain")
                            .to_owned(),
                    },
                    CompatMetadata::default(),
                )
                .map_err(|source| BuiltinError::Registration {
                    provider: id.into(),
                    source,
                })?;
            report.registered.push(id.into());
            return Ok(());
        }
        _ => {}
    }

    // Generic path: one mapped provider over per-wire routes.
    let forced = overrides.base_urls.get(profile.id).map(String::as_str);
    let infos = chat_infos(catalog, profile, forced)?;
    let mut routes: BTreeMap<WireApi, Box<dyn Provider>> = BTreeMap::new();
    let base_url = profile_base_url(profile, overrides);
    let headers = crate::ProviderHeaders::try_new(Vec::new()).map_err(|error| {
        BuiltinError::Construction {
            provider: id.into(),
            message: error.to_string(),
        }
    })?;
    for (wire, subset) in group_by_wire(infos)? {
        let route: Box<dyn Provider> = match wire {
            WireApi::OpenAiCompletions => {
                Box::new(crate::openai_chat::OpenAiChatProvider::for_route(
                    base_url.clone(),
                    id.to_owned(),
                    headers.clone(),
                    subset.clone(),
                    Arc::clone(client),
                ))
            }
            WireApi::AnthropicMessages => Box::new(crate::anthropic::AnthropicProvider::for_route(
                id.to_owned(),
                subset.clone(),
                base_url.clone(),
                headers.clone(),
                Arc::clone(client),
                id == "anthropic",
            )),
            WireApi::OpenAiResponses => {
                Box::new(crate::openai_responses::OpenAiResponsesProvider::for_route(
                    base_url.clone(),
                    id.to_owned(),
                    headers.clone(),
                    subset.clone(),
                    Arc::clone(client),
                ))
            }
            WireApi::MistralConversations => Box::new(
                crate::mistral::MistralConversationsProvider::new(base_url.clone())
                    .with_client(Arc::clone(client))
                    .with_models(subset.clone()),
            ),
            WireApi::PiMessages => Box::new(
                crate::pi_messages::PiMessagesProvider::new(base_url.clone())
                    .with_client(Arc::clone(client))
                    .with_models(subset.clone()),
            ),
            WireApi::OpenAiCodexResponses => Box::new(
                crate::openai_codex_responses::OpenAiCodexResponsesProvider::new(
                    base_url.clone(),
                    subset.clone(),
                    Arc::clone(client),
                ),
            ),
            WireApi::GoogleGenerativeAi => Box::new(
                crate::gemini::GeminiProvider::with_client(base_url.clone(), Arc::clone(client))
                    .with_models(subset.clone())
                    .with_provider_identity(id),
            ),
            other => {
                return Err(BuiltinError::Construction {
                    provider: id.into(),
                    message: format!("no builtin route for wire {other}"),
                });
            }
        };
        routes.insert(wire, builtin_route(id, subset, route));
    }
    let compat = CompatMetadata {
        openai_compatible: openai_compatible(&group_by_wire(chat_infos(
            catalog, profile, forced,
        )?)?),
        profile: Some(profile.name.to_owned()),
    };
    let mapped = crate::api_mapped::ApiMappedProvider::try_new(
        id.to_owned(),
        all_infos(catalog, profile, forced)?,
        routes,
    )
    .map_err(|error| BuiltinError::Construction {
        provider: id.into(),
        message: error.to_string(),
    })?;
    register_env_route(models, profile, Box::new(mapped), context, compat)?;
    report.registered.push(id.into());
    Ok(())
}

fn provider_models(provider: &dyn Provider) -> Vec<ModelInfo> {
    provider.models().to_vec()
}

fn chat_infos(
    catalog: &Catalog,
    profile: &BuiltinProviderProfile,
    forced_base: Option<&str>,
) -> Result<Vec<ModelInfo>, BuiltinError> {
    let entries: Vec<&CatalogModel> = catalog
        .models(profile.id, CatalogOperation::Chat)
        .map(|entries| entries.collect())
        .unwrap_or_default();
    if entries.is_empty() {
        return Err(BuiltinError::Construction {
            provider: profile.id.into(),
            message: "no chat entries in the bundled catalog".into(),
        });
    }
    entries
        .iter()
        .map(|entry| chat_model_info(entry, forced_base))
        .collect()
}

fn all_infos(
    catalog: &Catalog,
    profile: &BuiltinProviderProfile,
    forced_base: Option<&str>,
) -> Result<Vec<ModelInfo>, BuiltinError> {
    chat_infos(catalog, profile, forced_base)
}

fn group_by_wire(infos: Vec<ModelInfo>) -> Result<BTreeMap<WireApi, Vec<ModelInfo>>, BuiltinError> {
    let mut grouped: BTreeMap<WireApi, Vec<ModelInfo>> = BTreeMap::new();
    for model in infos {
        grouped.entry(model.wire_api).or_default().push(model);
    }
    Ok(grouped)
}

fn profile_base_url(
    profile: &BuiltinProviderProfile,
    overrides: &BuiltinOverrides,
) -> Option<String> {
    overrides
        .base_urls
        .get(profile.id)
        .cloned()
        .or_else(|| profile.base_url.map(str::to_owned))
}

fn register_env_route(
    models: &mut Models,
    profile: &BuiltinProviderProfile,
    provider: Box<dyn Provider>,
    context: &Arc<dyn AuthContext>,
    compat: CompatMetadata,
) -> Result<(), BuiltinError> {
    let source = match profile.api_key_env {
        Some(env_var) => {
            let resolver = BuiltinEnvAuthResolver::new(
                Arc::clone(context),
                env_var,
                profile.auth_scheme.clone(),
            );
            let source = AuthProvenanceSource::Environment {
                name: env_var.to_owned(),
            };
            return models
                .register_route(provider, Arc::new(resolver), source, compat)
                .map_err(|source| BuiltinError::Registration {
                    provider: profile.id.into(),
                    source,
                });
        }
        None => AuthProvenanceSource::Static,
    };
    // Env-less profiles (e.g. cloudflare-ai-gateway) register with a
    // label-only descriptor: dispatch needs an explicit route or credential.
    let descriptor = match profile.credential_label {
        Some(label) => AuthDescriptor::Resolved {
            source: label.to_owned(),
        },
        None => AuthDescriptor::Resolved {
            source: profile.name.to_owned(),
        },
    };
    let _ = source;
    models
        .register(provider, descriptor, compat)
        .map_err(|source| BuiltinError::Registration {
            provider: profile.id.into(),
            source,
        })
}

fn register_classifiers(
    models: &mut Models,
    catalog: &Catalog,
    context: &Arc<dyn AuthContext>,
    client: &Arc<HttpClient>,
    overrides: &BuiltinOverrides,
    report: &mut BuiltinReport,
) -> Result<(), BuiltinError> {
    // (provider, api id, env key) triples sourced from the profile table.
    let plans: &[(&str, &str)] = &[
        ("typesafe", "typesafe-system-one"),
        ("openrouter", "typesafe-system-one"),
        ("opencode", "typesafe-system-one"),
        ("vercel-ai-gateway", "typesafe-system-one"),
        ("openai", "openai-decisions"),
        ("cloudflare-workers-ai", "cloudflare-workers-ai-system-one"),
    ];
    for (provider_id, api_id) in plans {
        let profile = crate::providers::builtin_profile(provider_id)
            .expect("classifier builtin profile exists");
        let entries: Vec<&CatalogModel> = catalog
            .models(provider_id, CatalogOperation::Classifier)
            .map(|entries| entries.collect())
            .unwrap_or_default();
        let Some(first) = entries.first() else {
            continue;
        };
        let make_record = |entry: &CatalogModel| {
            let mut record = classifier_record(entry)?;
            if let Some(base) = overrides.base_urls.get(*provider_id) {
                record.base_url = base.clone();
            }
            Ok::<_, BuiltinError>(record)
        };
        let record = make_record(first)?;
        let http = client.as_ref().clone();
        let api_id_owned = (*api_id).to_owned();
        let result = match *api_id {
            "typesafe-system-one" => register_one_classifier(
                models,
                Box::new(crate::api::system_one::TypesafeSystemOneApi::new(
                    http,
                    record.clone(),
                )),
                profile,
                context,
                record,
            ),
            "openai-decisions" => register_one_classifier(
                models,
                Box::new(crate::api::decisions::OpenAiDecisionsApi::new(
                    http,
                    record.clone(),
                )),
                profile,
                context,
                record,
            ),
            _ => register_one_classifier(
                models,
                Box::new(
                    crate::api::cloudflare::CloudflareWorkersAiSystemOneApi::new(
                        http,
                        record.clone(),
                    ),
                ),
                profile,
                context,
                record,
            ),
        };
        result.map_err(|source| BuiltinError::Registration {
            provider: (*provider_id).to_owned(),
            source,
        })?;
        for extra in entries.iter().skip(1) {
            models
                .register_classifier_model_record(make_record(extra)?)
                .map_err(|error| BuiltinError::Construction {
                    provider: (*provider_id).to_owned(),
                    message: error.to_string(),
                })?;
        }
        report.classifier_apis.push(api_id_owned);
    }
    Ok(())
}

#[allow(clippy::type_complexity)]
fn register_one_classifier<O: Send + 'static>(
    models: &mut Models,
    api: Box<dyn crate::api::ClassifierApi<O>>,
    profile: &BuiltinProviderProfile,
    context: &Arc<dyn AuthContext>,
    _record: ClassifierModelRecord,
) -> Result<(), RegistrationError> {
    let provider_id = api.provider_id().to_owned();
    let api_id = api.api_id().clone();
    let api: Arc<dyn crate::api::ClassifierApi<O>> = Arc::from(api);
    let (resolver, source) = match profile.api_key_env {
        Some(env_var) => (
            Arc::new(BuiltinEnvAuthResolver::new(
                Arc::clone(context),
                env_var,
                profile.auth_scheme.clone(),
            )) as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Environment {
                name: env_var.to_owned(),
            },
        ),
        None => (
            Arc::new(crate::auth::StaticAuthResolver::new(
                profile.auth_scheme.clone(),
                secrecy::SecretString::from(String::new()),
            )) as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
        ),
    };
    models.register_classifier_api(api, resolver, source)?;
    models.mark_builtin_classifier_auth(&provider_id, &api_id);
    Ok(())
}

fn register_images(
    models: &mut Models,
    catalog: &Catalog,
    context: &Arc<dyn AuthContext>,
    client: &Arc<HttpClient>,
    overrides: &BuiltinOverrides,
    report: &mut BuiltinReport,
) -> Result<(), BuiltinError> {
    let Some(profile) = crate::providers::builtin_profile("openrouter") else {
        return Ok(());
    };
    let entries: Vec<&CatalogModel> = catalog
        .models("openrouter", CatalogOperation::Image)
        .map(|entries| entries.collect())
        .unwrap_or_default();
    let Some(first) = entries.first() else {
        return Ok(());
    };
    let make_record = |entry: &CatalogModel| {
        let mut record = image_record(entry)?;
        if let Some(base) = overrides.base_urls.get("openrouter") {
            record.base_url = base.clone();
        }
        Ok::<_, BuiltinError>(record)
    };
    let record = make_record(first)?;
    let api = Arc::new(crate::api::openrouter_images::OpenRouterImagesApi::new(
        client.as_ref().clone(),
        record,
    ));
    let resolver = Arc::new(BuiltinEnvAuthResolver::new(
        Arc::clone(context),
        profile.api_key_env.unwrap_or("OPENROUTER_API_KEY"),
        profile.auth_scheme.clone(),
    )) as Arc<dyn AuthResolver>;
    models
        .register_image_api(
            api,
            resolver,
            AuthProvenanceSource::Environment {
                name: "OPENROUTER_API_KEY".to_owned(),
            },
        )
        .map_err(|error| BuiltinError::Construction {
            provider: "openrouter".into(),
            message: error.to_string(),
        })?;
    models.mark_builtin_image_auth(
        "openrouter",
        &crate::models::ApiId::new("openrouter-images"),
    );
    for extra in entries.iter().skip(1) {
        models
            .register_image_model_record(make_record(extra)?)
            .map_err(|error| BuiltinError::Construction {
                provider: "openrouter".into(),
                message: error.to_string(),
            })?;
    }
    report.image_apis.push("openrouter-images".into());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_info::ModelCapabilities;

    #[test]
    fn canonical_route_replaces_uniquely_owned_inner_catalog() {
        let inner = crate::test_support::MockProvider::new("wire", vec![]);
        let mut route = CanonicalRoute {
            id: "canonical".into(),
            models: inner.models().to_vec(),
            inner: Arc::new(inner),
            builtin_auth: false,
        };
        let replacement = vec![ModelInfo::new(
            "replacement",
            "Replacement",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(4096, 1024),
        )];
        route.replace_model_catalog(replacement.clone()).unwrap();
        assert_eq!(route.models(), replacement);
        assert_eq!(route.inner.models(), replacement);
    }

    #[test]
    fn canonical_preparation_retains_selected_model_and_refuses_shared_catalog_mutation() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct PreparingRoute {
            models: Vec<ModelInfo>,
            prepared: Arc<dyn Provider>,
            preparations: Arc<AtomicUsize>,
        }
        impl Provider for PreparingRoute {
            fn id(&self) -> &str {
                "preparation-source"
            }
            fn models(&self) -> &[ModelInfo] {
                &self.models
            }
            fn prepare_model_call(
                &self,
                model: &mut ModelInfo,
                auth: &mut ResolvedAuth,
            ) -> Result<Option<Arc<dyn Provider>>, ProviderError> {
                self.preparations.fetch_add(1, Ordering::SeqCst);
                model.base_url = Some("https://prepared.invalid/frozen".into());
                auth.base_url = model.base_url.clone();
                Ok(Some(self.prepared.clone()))
            }
            fn stream_prepared(&self, _: Request, _: ResolvedAuth) -> EventStream {
                panic!("dispatch must use the selected prepared adapter")
            }
        }

        let prepared_inner: Arc<dyn Provider> =
            Arc::new(crate::test_support::MockProvider::new("wire", vec![]));
        let original_inner_models = prepared_inner.models().to_vec();
        let preparations = Arc::new(AtomicUsize::new(0));
        let route = builtin_route(
            "opencode",
            original_inner_models.clone(),
            Box::new(PreparingRoute {
                models: original_inner_models.clone(),
                prepared: prepared_inner.clone(),
                preparations: preparations.clone(),
            }),
        );
        let mut selected = original_inner_models[0].clone();
        let mut auth = crate::test_support::resolved_auth();
        let mut prepared = route
            .prepare_model_call(&mut selected, &mut auth)
            .unwrap()
            .unwrap();
        assert_eq!(preparations.load(Ordering::SeqCst), 1);
        assert_eq!(prepared.id(), "opencode");
        assert_eq!(prepared.models(), [selected.clone()]);
        assert_eq!(
            selected.base_url.as_deref(),
            Some("https://prepared.invalid/frozen")
        );
        assert_eq!(auth.base_url, selected.base_url);

        let error = Arc::get_mut(&mut prepared)
            .unwrap()
            .replace_model_catalog(vec![])
            .unwrap_err();
        assert!(matches!(error, ProviderError::Config(_)));
        assert_eq!(prepared.models(), [selected]);
        assert_eq!(prepared_inner.models(), original_inner_models);
        assert_eq!(preparations.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn builtin_pricing_ingestion_retains_finite_number_thresholds() {
        let mut entry = Catalog::builtin()
            .unwrap()
            .models("openai", CatalogOperation::Chat)
            .unwrap()
            .next()
            .unwrap()
            .clone();
        entry.raw["cost"]["tiers"] = serde_json::json!([
            {"inputTokensAbove": -2, "input": 99, "output": 0, "cacheRead": 0, "cacheWrite": 0},
            {"inputTokensAbove": -0.5, "input": 2, "output": 0, "cacheRead": 0, "cacheWrite": 0},
            {"inputTokensAbove": 0.5, "input": 3, "output": 0, "cacheRead": 0, "cacheWrite": 0},
            {"inputTokensAbove": 9_007_199_254_740_992_u64, "input": 4, "output": 0, "cacheRead": 0, "cacheWrite": 0},
            {"inputTokensAbove": 9_007_199_254_740_993_u64, "input": 5, "output": 0, "cacheRead": 0, "cacheWrite": 0}
        ]);
        let pricing = catalog_pricing(&entry).unwrap();
        assert_eq!(pricing.tiers.len(), 5);
        assert_eq!(pricing.effective(0).input_cost_per_mtok, 2.0);
        assert_eq!(pricing.effective(1).input_cost_per_mtok, 3.0);
        assert_eq!(
            pricing.tiers[4].input_tokens_above.as_u64(),
            Some(9_007_199_254_740_993)
        );
        assert_eq!(
            pricing.effective(9_007_199_254_740_994).input_cost_per_mtok,
            5.0
        );
    }
    struct NoAmbient;

    impl AuthContext for NoAmbient {
        fn env(&self, name: &str) -> Result<Option<String>, crate::auth::AuthContextError> {
            assert!(
                !matches!(name, "AZURE_OPENAI_BASE_URL" | "AZURE_OPENAI_RESOURCE_NAME"),
                "selected scoped fields must not read ambient values"
            );
            Ok(None)
        }

        fn file_exists(&self, _: &str) -> Result<bool, crate::auth::AuthContextError> {
            panic!("Azure route preparation must not discover credential files")
        }
    }

    #[test]
    fn azure_blank_scoped_base_allows_same_scope_resource_before_model_fallback() {
        for wire in [WireApi::OpenAiCompletions, WireApi::AzureOpenAiResponses] {
            for base in ["", "  \t "] {
                for model_base in [None, Some("https://wrong-model.example.com")] {
                    let route = BuiltinAzureRoute {
                        models: vec![],
                        context: Arc::new(NoAmbient),
                        client: Arc::new(HttpClient::new()),
                        api_version: None,
                    };
                    let mut model = ModelInfo::new(
                        "model",
                        "model",
                        wire,
                        ModelCapabilities::new(4096, 1024).with_streaming(true),
                    );
                    model.base_url = model_base.map(str::to_owned);
                    if let crate::WireCompat::AzureOpenAiResponses(compat) = &mut model.compat {
                        compat.source = crate::model_info::OpenAiResponsesSourceCompat {
                            supports_mid_convo_system_messages: Some(true),
                            supports_additional_tools: Some(true),
                            supports_tool_search: Some(false),
                            supports_strict_mode: Some(false),
                            supports_grammar_tools: Some(true),
                            supports_mid_convo_effort: Some(false),
                            ..Default::default()
                        };
                    }
                    let frozen_compat = model.compat.clone();
                    let mut auth = crate::test_support::resolved_auth();
                    auth.env = [
                        ("AZURE_OPENAI_BASE_URL".into(), base.into()),
                        (
                            "AZURE_OPENAI_RESOURCE_NAME".into(),
                            "scoped-resource".into(),
                        ),
                    ]
                    .into();
                    let prepared = route.prepare_model_call(&mut model, &mut auth);
                    assert_eq!(
                        model.compat, frozen_compat,
                        "auth endpoint finalization preserves frozen projection/source controls"
                    );
                    assert!(
                        prepared.is_ok(),
                        "wire={wire}, base={base:?}: {:?}",
                        prepared.err()
                    );
                    assert_eq!(
                        model.base_url.as_deref(),
                        Some(if wire == WireApi::OpenAiCompletions {
                            "https://scoped-resource.openai.azure.com/openai/v1"
                        } else {
                            "https://scoped-resource.openai.azure.com"
                        }),
                        "wire={wire}, base={base:?}, model_base={model_base:?}"
                    );
                }
            }
        }
    }
}
