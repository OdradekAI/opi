//! The builtin provider collection (task 20.34).
//!
//! Turns the pinned [`crate::providers`] profile table plus the bundled
//! [`Catalog`] into registered providers on an existing [`Models`]
//! collection, reusing the completed wire adapters — one
//! [`crate::api_mapped::ApiMappedProvider`] per profile
//! over per-wire routes built from the existing constructors. Canonical ids
//! replace the legacy opi ids (`gemini`, `vertex`, `bedrock`,
//! `openai-responses`): nothing here registers a permanent alias. Azure and
//! google-vertex need configuration (endpoint / project+location) and stay
//! lookup-skipped with a typed reason until supplied; bedrock registers with
//! a credential-chain label and needs an explicit route before dispatch.
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
use std::str::FromStr;

use crate::model_info::{ModelInfo, ModelPricing, PricingTier, WireApi};
use crate::provider::{
    EventStream, ModelInfo as ProviderModelInfo, Provider, ProviderError, Request,
};
use crate::providers::{BuiltinEndpoint, BuiltinProviderProfile, builtin_provider_profiles};
use crate::stream::Pricing;

/// Per-construction configuration the builtin table cannot supply itself.
#[derive(Debug, Clone, Default)]
pub struct BuiltinOverrides {
    /// Azure endpoint; without it the azure profile stays skipped.
    pub azure_endpoint: Option<String>,
    /// Azure API version (e.g. `2024-10-21`).
    pub azure_api_version: Option<String>,
    /// Vertex project; with `vertex_location` it enables google-vertex.
    pub vertex_project: Option<String>,
    /// Vertex location (e.g. `us-central1`).
    pub vertex_location: Option<String>,
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
/// `bedrock`) stay reusable as-is; this wrapper only renames the public
/// identity and catalog. Every dispatch, resource and refresh call
/// delegates to the inner provider unchanged.
struct CanonicalRoute {
    id: String,
    models: Vec<ModelInfo>,
    inner: Box<dyn Provider>,
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
    fn rewrite_model_spec(&self, mut request: Request) -> Request {
        let canonical_prefix = format!("{}:", self.id);
        if let Some(rest) = request.model.strip_prefix(&canonical_prefix) {
            request.model = format!("{}:{rest}", self.inner.id());
        }
        request
    }
}

impl Provider for CanonicalRoute {
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
        let request = self.rewrite_model_spec(request);
        self.inner.stream_prepared(request, auth)
    }
    fn stream_attempt(
        &self,
        request: Request,
        auth: ResolvedAuth,
        attempt: crate::provider::AttemptContext,
    ) -> EventStream {
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
        self.inner.replace_model_catalog(models.clone())?;
        self.models = models;
        Ok(())
    }
    fn refresh_models(
        &self,
    ) -> crate::credential::BoxAuthFuture<'_, Result<Option<Vec<ModelInfo>>, ProviderError>> {
        self.inner.refresh_models()
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
            inner,
        })
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
                .and_then(|value| value.as_u64())
                .ok_or_else(|| invalid("tier threshold is not a non-negative integer"))?;
            let pricing = read_rates(tier)
                .ok_or_else(|| invalid("tier cost rates are not finite numbers"))?;
            tiers.push(PricingTier {
                input_tokens_above,
                pricing,
            });
        }
    }
    ModelPricing::try_new(base, tiers).map_err(|error| invalid(&error.to_string()))
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
    let wire_api = WireApi::from_str(&entry.api).map_err(|error| invalid(error.to_string()))?;
    let context_window = entry
        .context_window
        .ok_or_else(|| invalid("chat entry has no context window".into()))?;
    let max_tokens = entry.max_tokens.unwrap_or_else(|| context_window.min(4096));
    let mut capabilities = crate::ModelCapabilities::new(context_window, max_tokens);
    if entry
        .input
        .iter()
        .any(|modality| matches!(modality, crate::catalog::CatalogInputModality::Image))
    {
        capabilities = capabilities.with_images(true);
    }
    if entry.reasoning {
        capabilities = capabilities.with_thinking(true);
    }
    let mut model = ModelInfo::new(&entry.id, &entry.name, wire_api, capabilities);
    // An explicit provider override (tests point profiles at local mocks)
    // wins over per-entry catalog base URLs.
    if forced_base.is_none() && !entry.base_url.is_empty() {
        model = model.with_base_url(entry.base_url.clone());
    }
    model
        .with_pricing(catalog_pricing(entry)?)
        .map_err(|error| invalid(error.to_string()))
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
        provider_id: entry.provider.clone(),
        id: entry.id.clone(),
        name: entry.name.clone(),
        api: crate::models::ApiId::new(&entry.api),
        base_url: entry.base_url.clone(),
        input,
        context_window: entry.context_window.unwrap_or(0),
        cost: catalog_pricing(entry)?,
    })
}

/// One bundled image entry into its complete record.
fn image_record(entry: &CatalogModel) -> Result<ImageModelRecord, BuiltinError> {
    Ok(ImageModelRecord {
        provider_id: entry.provider.clone(),
        id: entry.id.clone(),
        name: entry.name.clone(),
        api: crate::models::ApiId::new(&entry.api),
        base_url: entry.base_url.clone(),
        output: vec![ImageOutputModality::Image],
        cost: ImageCostRates {
            input: entry.cost.input,
            output: entry.cost.output,
            cache_read: entry.cost.cache_read,
            cache_write: entry.cost.cache_write,
        },
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
    }
    register_classifiers(models, catalog, &context, &mut report)?;
    register_images(models, catalog, &context, &mut report)?;
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
            let Some(endpoint) = overrides.azure_endpoint.clone() else {
                report
                    .skipped
                    .push((id.into(), "azure requires a configured endpoint".into()));
                return Ok(());
            };
            let deployments: Vec<String> = catalog
                .models(id, CatalogOperation::Chat)
                .map(|entries| entries.map(|entry| entry.id.clone()).collect::<Vec<_>>())
                .unwrap_or_default();
            let provider = crate::azure_openai::AzureOpenAIProvider::from_config(
                Some(endpoint),
                deployments,
                overrides.azure_api_version.clone(),
            )
            .map_err(|error| BuiltinError::Construction {
                provider: id.into(),
                message: error.to_string(),
            })?;
            register_env_route(
                models,
                profile,
                Box::new(provider),
                context,
                CompatMetadata {
                    openai_compatible: true,
                    profile: Some(profile.name.to_owned()),
                },
            )?;
            report.registered.push(id.into());
            return Ok(());
        }
        BuiltinEndpoint::VertexLocation => {
            match (&overrides.vertex_project, &overrides.vertex_location) {
                (Some(project), Some(location)) => {
                    let forced = overrides.base_urls.get(profile.id).map(String::as_str);
                    let infos = chat_infos(catalog, profile, forced)?;
                    let base_url = profile_base_url(profile, overrides);
                    let provider = crate::vertex::VertexProvider::new(
                        project.clone(),
                        location.clone(),
                        base_url,
                    )
                    .with_models(infos);
                    let provider =
                        canonical_route(id, provider_models(&provider), Box::new(provider));
                    register_env_route(
                        models,
                        profile,
                        provider,
                        context,
                        CompatMetadata::default(),
                    )?;
                    report.registered.push(id.into());
                }
                _ => report.skipped.push((
                    id.into(),
                    "google-vertex requires project and location configuration".into(),
                )),
            }
            return Ok(());
        }
        BuiltinEndpoint::BedrockRegion => {
            let forced = overrides.base_urls.get(profile.id).map(String::as_str);
            let infos = chat_infos(catalog, profile, forced)?;
            let provider = crate::bedrock::BedrockProvider::new(
                profile_base_url(profile, overrides),
                Arc::clone(client),
            )
            .with_models(infos);
            let provider = canonical_route(id, provider_models(&provider), Box::new(provider));
            // Lookup-only until an explicit AWS route is supplied.
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
                false,
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
                    .with_models(subset.clone()),
            ),
            WireApi::PiMessages => Box::new(
                crate::pi_messages::PiMessagesProvider::new(base_url.clone())
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
                    .with_models(subset.clone()),
            ),
            other => {
                return Err(BuiltinError::Construction {
                    provider: id.into(),
                    message: format!("no builtin route for wire {other}"),
                });
            }
        };
        routes.insert(wire, canonical_route(id, subset, route));
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
    report: &mut BuiltinReport,
) -> Result<(), BuiltinError> {
    // (provider, api id, env key) triples sourced from the profile table.
    let plans: &[(&str, &str)] = &[
        ("typesafe", "typesafe-system-one"),
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
        let record = classifier_record(first)?;
        let http = HttpClient::new();
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
                .register_classifier_model_record(classifier_record(extra)?)
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
    models
        .register_classifier_api(api, resolver, source)
        .map(|_| ())
}

fn register_images(
    models: &mut Models,
    catalog: &Catalog,
    context: &Arc<dyn AuthContext>,
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
    let record = image_record(first)?;
    let api = Arc::new(crate::api::openrouter_images::OpenRouterImagesApi::new(
        HttpClient::new(),
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
    report.image_apis.push("openrouter-images".into());
    Ok(())
}
