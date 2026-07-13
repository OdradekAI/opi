//! Centralized provider/model/auth construction (Workstream 10.1, task 10.2).
//!
//! This module is the single place in `opi-coding-agent` that turns CLI config,
//! env vars, and package/extension provider inputs into [`opi_ai::Provider`]
//! values, [`opi_ai::ProviderRegistry`] / [`opi_ai::ProviderCollection`]
//! lookups, and redacted auth descriptors. Every run-mode startup path
//! (`--list-models`, non-interactive, JSON, RPC, interactive) and the
//! [`crate::harness::CodingHarness`] model registry are built here.
//!
//! # Routing through the provider collection/auth seam
//!
//! The factory produces [`opi_ai::ProviderCollection`] (the Workstream 10.1
//! seam) so provider+model lookup, OpenAI-compatible compatibility metadata,
//! and the auth contract live on one type:
//!
//! - [`build_collection_for_listing`] registers each config-sourced provider
//!   via [`ProviderCollection::register`] with a derived [`AuthDescriptor`] and
//!   [`CompatMetadata`], exercising the auth seam. Listing never dispatches, so
//!   attaching descriptors cannot gate or alter output.
//! - [`assemble_harness_collection`] wraps an already-built active provider
//!   (plus extension providers/model overrides) via
//!   [`ProviderCollection::from_registry`]. Those entries are not config-sourced
//!   and the active provider's credentials are validated at build time, so no
//!   descriptor is attached and dispatch behavior is unchanged.
//!
//! # Centralization contract
//!
//! `tests/provider_factory.rs::provider_policy_is_centralized` asserts that
//! construction-policy symbols (`ProviderRegistry::new`, `parse_model_spec`,
//! the per-provider builders, credential helpers, ...) appear only in this
//! file across `crates/opi-coding-agent/src/`.
//!
//! # Unstable
//!
//! Part of the unstable 0.x extension substrate; breaking changes may occur
//! between minor versions.

use std::path::PathBuf;
use std::sync::Arc;

use opi_agent::diagnostic::Diagnostic;
use opi_agent::extension::ExtensionRegistry;
use opi_ai::provider::{EventStream, ModelInfo, Provider, ProviderError, Request};
use opi_ai::{AuthDescriptor, CompatMetadata, ProviderCollection, ProviderRegistry};
use secrecy::ExposeSecret;

use crate::config::{OpenAiCompatibleProviderConfig, OpiConfig, build_http_client};
use crate::credential_store::{AuthSource, CredentialResolver};
use crate::diagnostic_bridge::diagnostic_for_model_registry_error;
use crate::oauth::OAuthProviderRegistry;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Error from runtime provider construction (the active provider for a run).
#[derive(Debug, thiserror::Error)]
pub enum ProviderBuildError {
    #[error("{0}")]
    Auth(String),
    #[error("{0}")]
    Config(String),
    #[error(transparent)]
    Provider(#[from] ProviderError),
}

/// Error from lightweight provider builders used by `--list-models`.
///
/// `MissingCredentials` — the provider has no API key / credentials configured;
/// skip silently and try the next provider.
///
/// `Config` — the config file contains a broken setting (e.g. invalid proxy
/// URL); report the error and exit.
#[derive(Debug, thiserror::Error)]
pub enum ListModelsError {
    #[error("missing credentials")]
    MissingCredentials,
    #[error("{0}")]
    Config(String),
}

/// The provider, credential store, and OAuth registry produced at production
/// startup. Callers that need the store and registry for `/login`, `/logout`,
/// or `CredentialNeeded` same-turn retry (i.e. the interactive TUI) take the
/// full bundle; non-interactive and RPC callers extract only `provider`.
pub struct ProviderBundle {
    /// The active runtime provider.
    pub provider: Box<dyn Provider>,
    /// The OS-keychain-backed credential store (for `/login`/`/logout`).
    pub store: Arc<crate::credential_store::KeychainCredentialStore>,
    /// The credential resolver built over the same store.
    pub resolver: crate::credential_store::CredentialResolver,
    /// The built-in OAuth provider registry.
    pub registry: OAuthProviderRegistry,
}

// ---------------------------------------------------------------------------
// HTTP client + credential helpers
// ---------------------------------------------------------------------------

/// Build an HTTP client, adapting proxy/config errors into [`ProviderBuildError`].
fn build_proxied_client(
    proxy_config: Option<&crate::config::ProviderProxyConfig>,
) -> Result<Arc<opi_ai::http::HttpClient>, ProviderBuildError> {
    build_http_client(proxy_config).map_err(|e| {
        ProviderBuildError::Config(format!(
            "failed to build HTTP client with proxy config: {e}"
        ))
    })
}

fn resolve_env_name(configured: &str, default: &str) -> String {
    if configured.is_empty() {
        default.into()
    } else {
        configured.into()
    }
}

fn require_api_key(env_name: &str) -> Result<String, ProviderBuildError> {
    let key = std::env::var(env_name).map_err(|_| {
        ProviderBuildError::Auth(format!(
            "missing API key: set {env_name} environment variable"
        ))
    })?;
    if key.trim().is_empty() {
        return Err(ProviderBuildError::Auth(format!(
            "empty API key: {env_name} is set but empty"
        )));
    }
    Ok(key)
}

fn non_empty_env_var(env_name: &str) -> Option<String> {
    std::env::var(env_name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn require_list_models_api_key(env_name: &str) -> Result<String, ListModelsError> {
    non_empty_env_var(env_name).ok_or(ListModelsError::MissingCredentials)
}

/// Read AWS credentials from environment variables.
fn resolve_bedrock_env_credentials() -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let akid = non_empty_env_var("AWS_ACCESS_KEY_ID");
    let sak = non_empty_env_var("AWS_SECRET_ACCESS_KEY");
    let token = non_empty_env_var("AWS_SESSION_TOKEN");
    let region =
        non_empty_env_var("AWS_REGION").or_else(|| non_empty_env_var("AWS_DEFAULT_REGION"));
    (akid, sak, token, region)
}

/// AWS shared credentials file path.
fn aws_credentials_path() -> Option<PathBuf> {
    std::env::var("AWS_SHARED_CREDENTIALS_FILE")
        .ok()
        .map(PathBuf::from)
        .or_else(|| aws_home_dir().map(|h| h.join(".aws").join("credentials")))
}

/// AWS shared config file path.
fn aws_config_path() -> Option<PathBuf> {
    std::env::var("AWS_CONFIG_FILE")
        .ok()
        .map(PathBuf::from)
        .or_else(|| aws_home_dir().map(|h| h.join(".aws").join("config")))
}

/// Home directory for AWS shared-credential path resolution.
fn aws_home_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
}

fn profile_api_key_env_default(provider_id: &str) -> String {
    format!(
        "{}_API_KEY",
        provider_id.replace('-', "_").to_ascii_uppercase()
    )
}

// ---------------------------------------------------------------------------
// Model-spec resolution
// ---------------------------------------------------------------------------

/// Parse a `provider:model` spec into its `(provider, model)` halves.
///
/// This is the canonical spec resolver for the crate; both the run-mode
/// startup paths and the harness use it.
pub fn parse_model_spec(spec: &str) -> Result<(&str, &str), String> {
    let Some((provider, model)) = spec.split_once(':') else {
        return Err("invalid model spec: expected provider:model".into());
    };
    if provider.is_empty() || model.is_empty() {
        return Err("invalid model spec: expected provider:model".into());
    }
    Ok((provider, model))
}

// ---------------------------------------------------------------------------
// MetadataProvider — registers the active provider's id/models into a registry
// ---------------------------------------------------------------------------

/// Wrapper that contributes a provider's `id()`/`models()` metadata to a
/// [`ProviderRegistry`] without being dispatchable. Used by
/// [`assemble_harness_collection`] so the active provider's models appear in
/// model listing / picker / resolution alongside extension providers.
struct MetadataProvider {
    id: String,
    models: Vec<ModelInfo>,
}

impl MetadataProvider {
    fn from_provider(provider: &dyn Provider) -> Self {
        Self {
            id: provider.id().to_owned(),
            models: provider.models().to_vec(),
        }
    }
}

impl Provider for MetadataProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream(&self, _request: Request) -> EventStream {
        let id = self.id.clone();
        Box::pin(futures_util::stream::once(async move {
            Err(ProviderError::StreamError(format!(
                "metadata-only provider '{id}' in the harness model registry cannot dispatch"
            )))
        }))
    }
}

// ---------------------------------------------------------------------------
// Built-in provider ids
// ---------------------------------------------------------------------------

/// The fixed set of built-in provider ids, in registration order.
pub(crate) const BUILT_IN_PROVIDER_IDS: &[&str] = &[
    "anthropic",
    "openai",
    "openrouter",
    "mistral",
    "openai-responses",
    "gemini",
    "bedrock",
    "azure",
    "vertex",
];

/// Public accessor for the built-in provider id list (the binary target is a
/// separate crate and cannot see the `pub(crate)` const directly).
pub fn built_in_provider_ids() -> &'static [&'static str] {
    BUILT_IN_PROVIDER_IDS
}

// ---------------------------------------------------------------------------
// Lightweight per-provider builders for --list-models
// (silent skip on missing credentials)
// ---------------------------------------------------------------------------

fn build_anthropic(
    config: &OpiConfig,
) -> Result<opi_ai::anthropic::AnthropicProvider, ListModelsError> {
    let api_key = require_list_models_api_key(&config.providers.anthropic.api_key_env)?;
    let client = build_proxied_client_for_listing(config.providers.anthropic.proxy.as_ref())?;
    Ok(opi_ai::anthropic::AnthropicProvider::with_client(
        api_key,
        config.providers.anthropic.base_url.clone(),
        client,
    ))
}

fn build_openai(
    config: &OpiConfig,
) -> Result<opi_ai::openai_chat::OpenAiChatProvider, ListModelsError> {
    let env_name = resolve_env_name(&config.providers.openai.api_key_env, "OPENAI_API_KEY");
    let api_key = require_list_models_api_key(&env_name)?;
    let client = build_proxied_client_for_listing(config.providers.openai.proxy.as_ref())?;
    Ok(opi_ai::openai_chat::OpenAiChatProvider::with_client(
        api_key,
        config.providers.openai.base_url.clone(),
        "openai".into(),
        vec![],
        client,
    ))
}

fn build_openrouter(
    config: &OpiConfig,
) -> Result<opi_ai::openai_chat::OpenAiChatProvider, ListModelsError> {
    let env_name = resolve_env_name(
        &config.providers.openrouter.api_key_env,
        "OPENROUTER_API_KEY",
    );
    let api_key = require_list_models_api_key(&env_name)?;
    let client = build_proxied_client_for_listing(config.providers.openrouter.proxy.as_ref())?;
    if let Some(ref referer) = config.providers.openrouter.referer {
        let base_url = config
            .providers
            .openrouter
            .base_url
            .clone()
            .unwrap_or_else(|| "https://openrouter.ai/api".into());
        let compat = opi_ai::openai_chat::CompatConfig::default();
        let extra_headers = vec![
            ("HTTP-Referer".into(), referer.clone()),
            ("X-Title".into(), "opi".into()),
        ];
        let temp = opi_ai::openrouter::openrouter_provider(
            String::new(),
            config.providers.openrouter.base_url.clone(),
        );
        let models = temp.models().to_vec();
        Ok(opi_ai::openai_chat::OpenAiChatProvider::new_for_profile(
            api_key,
            base_url,
            "openrouter".into(),
            compat,
            extra_headers,
            models,
        )
        .with_shared_client(client))
    } else {
        Ok(opi_ai::openrouter::openrouter_provider(
            api_key,
            config.providers.openrouter.base_url.clone(),
        )
        .with_shared_client(client))
    }
}

fn build_mistral(
    config: &OpiConfig,
) -> Result<opi_ai::openai_chat::OpenAiChatProvider, ListModelsError> {
    let env_name = resolve_env_name(&config.providers.mistral.api_key_env, "MISTRAL_API_KEY");
    let api_key = require_list_models_api_key(&env_name)?;
    let client = build_proxied_client_for_listing(config.providers.mistral.proxy.as_ref())?;
    Ok(
        opi_ai::mistral::mistral_provider(api_key, config.providers.mistral.base_url.clone())
            .with_shared_client(client),
    )
}

fn build_openai_responses(
    config: &OpiConfig,
) -> Result<opi_ai::openai_responses::OpenAiResponsesProvider, ListModelsError> {
    let env_name = resolve_env_name(
        &config.providers.openai_responses.api_key_env,
        "OPENAI_API_KEY",
    );
    let api_key = require_list_models_api_key(&env_name)?;
    let client =
        build_proxied_client_for_listing(config.providers.openai_responses.proxy.as_ref())?;
    Ok(
        opi_ai::openai_responses::OpenAiResponsesProvider::with_client(
            api_key,
            config.providers.openai_responses.base_url.clone(),
            client,
        ),
    )
}

fn build_gemini(config: &OpiConfig) -> Result<opi_ai::gemini::GeminiProvider, ListModelsError> {
    let env_name = resolve_env_name(&config.providers.gemini.api_key_env, "GEMINI_API_KEY");
    let api_key = require_list_models_api_key(&env_name)?;
    let client = build_proxied_client_for_listing(config.providers.gemini.proxy.as_ref())?;
    Ok(opi_ai::gemini::GeminiProvider::with_client(
        api_key,
        config.providers.gemini.base_url.clone(),
        client,
    ))
}

/// Build a Bedrock provider for `--list-models`. The credential-resolution
/// shape here (env + config + profile/files, fed to `resolve_credentials`)
/// is mirrored by the Bedrock arm of `build_runtime_provider`; extracting a
/// shared resolver is intentionally deferred (Phase 10 audit, opus-F-33) to
/// keep the factory extraction behavior-neutral.
fn build_bedrock(config: &OpiConfig) -> Result<opi_ai::bedrock::BedrockProvider, ListModelsError> {
    let bedrock_config = &config.providers.bedrock;
    let (akid, sak, token, env_region) = resolve_bedrock_env_credentials();
    let env_profile = std::env::var("AWS_PROFILE").ok();
    let profile_name = bedrock_config.profile.as_deref().or(env_profile.as_deref());
    let credentials_file = aws_credentials_path();
    let config_file = aws_config_path();
    let secret_key = bedrock_config
        .secret_access_key_env
        .as_deref()
        .and_then(non_empty_env_var);
    let session_token = bedrock_config
        .session_token_env
        .as_deref()
        .and_then(non_empty_env_var);
    let input = opi_ai::bedrock::credentials::CredentialResolutionInput {
        config_access_key_id: bedrock_config.access_key_id.as_deref(),
        config_secret_access_key: secret_key.as_deref(),
        config_session_token: session_token.as_deref(),
        config_region: bedrock_config.region.as_deref(),
        env_access_key_id: akid.as_deref(),
        env_secret_access_key: sak.as_deref(),
        env_session_token: token.as_deref(),
        env_region: env_region.as_deref(),
        profile_name,
        credentials_file_path: credentials_file.as_deref(),
        config_file_path: config_file.as_deref(),
    };
    let resolved = opi_ai::bedrock::credentials::resolve_credentials(&input);
    let (bedrock_creds, _) = resolved.ok_or(ListModelsError::MissingCredentials)?;
    let client = build_proxied_client_for_listing(bedrock_config.proxy.as_ref())?;
    Ok(opi_ai::bedrock::BedrockProvider::from_credentials(
        bedrock_creds,
        bedrock_config.base_url.clone(),
        client,
    ))
}

fn build_azure(
    config: &OpiConfig,
) -> Result<opi_ai::azure_openai::AzureOpenAIProvider, ListModelsError> {
    let azure_config = &config.providers.azure;
    let env_name = resolve_env_name(&azure_config.api_key_env, "AZURE_OPENAI_API_KEY");
    let api_key = require_list_models_api_key(&env_name)?;
    if azure_config.deployments.is_empty() {
        return Err(ListModelsError::Config(
            "azure provider has no deployments configured".into(),
        ));
    }
    let provider = opi_ai::azure_openai::AzureOpenAIProvider::from_config(
        api_key,
        azure_config.endpoint.clone(),
        azure_config.deployments.clone(),
        azure_config.api_version.clone(),
    )
    .map_err(|e| ListModelsError::Config(e.to_string()))?;
    Ok(provider.with_client(build_proxied_client_for_listing(
        azure_config.proxy.as_ref(),
    )?))
}

fn build_vertex(config: &OpiConfig) -> Result<opi_ai::vertex::VertexProvider, ListModelsError> {
    let vertex_config = &config.providers.vertex;
    let env_name = resolve_env_name(&vertex_config.access_token_env, "VERTEX_ACCESS_TOKEN");
    let access_token = require_list_models_api_key(&env_name)?;
    let project = vertex_config
        .project
        .as_deref()
        .ok_or_else(|| ListModelsError::Config("vertex provider requires project".into()))?;
    let location = vertex_config
        .location
        .as_deref()
        .ok_or_else(|| ListModelsError::Config("vertex provider requires location".into()))?;
    let provider = if vertex_config.models.is_empty() {
        opi_ai::vertex::VertexProvider::new(
            access_token,
            project.into(),
            location.into(),
            vertex_config.base_url.clone(),
        )
    } else {
        opi_ai::vertex::VertexProvider::from_config(
            access_token,
            project.into(),
            location.into(),
            vertex_config.models.clone(),
            vertex_config.base_url.clone(),
        )
    };
    Ok(provider.with_client(build_proxied_client_for_listing(
        vertex_config.proxy.as_ref(),
    )?))
}

fn build_list_models_provider(
    config: &OpiConfig,
    provider_id: &str,
) -> Result<Box<dyn Provider>, ListModelsError> {
    match provider_id {
        "anthropic" => Ok(Box::new(build_anthropic(config)?) as Box<dyn Provider>),
        "openai" => Ok(Box::new(build_openai(config)?) as Box<dyn Provider>),
        "openrouter" => Ok(Box::new(build_openrouter(config)?) as Box<dyn Provider>),
        "mistral" => Ok(Box::new(build_mistral(config)?) as Box<dyn Provider>),
        "openai-responses" => Ok(Box::new(build_openai_responses(config)?) as Box<dyn Provider>),
        "gemini" => Ok(Box::new(build_gemini(config)?) as Box<dyn Provider>),
        "bedrock" => Ok(Box::new(build_bedrock(config)?) as Box<dyn Provider>),
        "azure" => Ok(Box::new(build_azure(config)?) as Box<dyn Provider>),
        "vertex" => Ok(Box::new(build_vertex(config)?) as Box<dyn Provider>),
        other => Err(ListModelsError::Config(format!(
            "unknown provider in built-in list: {other}"
        ))),
    }
}

/// `build_proxied_client` adapted for the list-models error type.
fn build_proxied_client_for_listing(
    proxy_config: Option<&crate::config::ProviderProxyConfig>,
) -> Result<Arc<opi_ai::http::HttpClient>, ListModelsError> {
    build_http_client(proxy_config).map_err(|e| {
        ListModelsError::Config(format!(
            "failed to build HTTP client with proxy config: {e}"
        ))
    })
}

// ---------------------------------------------------------------------------
// openai_compatible profile builders
// ---------------------------------------------------------------------------

fn build_runtime_openai_compatible_profile(
    profile: &OpenAiCompatibleProviderConfig,
) -> Result<opi_ai::openai_chat::OpenAiChatProvider, ProviderBuildError> {
    let default_env = profile_api_key_env_default(&profile.id);
    let env_name = resolve_env_name(&profile.api_key_env, &default_env);
    let api_key = require_api_key(&env_name)?;
    let client = build_proxied_client(profile.proxy.as_ref())?;
    build_openai_compatible_profile(profile, api_key, client).map_err(ProviderBuildError::Config)
}

fn build_list_models_openai_compatible_profile(
    profile: &OpenAiCompatibleProviderConfig,
) -> Result<opi_ai::openai_chat::OpenAiChatProvider, ListModelsError> {
    let default_env = profile_api_key_env_default(&profile.id);
    let env_name = resolve_env_name(&profile.api_key_env, &default_env);
    let api_key = require_list_models_api_key(&env_name)?;
    let client = build_proxied_client_for_listing(profile.proxy.as_ref())?;
    build_openai_compatible_profile(profile, api_key, client).map_err(ListModelsError::Config)
}

fn build_openai_compatible_profile(
    profile: &OpenAiCompatibleProviderConfig,
    api_key: String,
    client: Arc<opi_ai::http::HttpClient>,
) -> Result<opi_ai::openai_chat::OpenAiChatProvider, String> {
    if profile.id.trim().is_empty() {
        return Err("openai-compatible profile id cannot be empty".into());
    }
    if profile.base_url.trim().is_empty() {
        return Err(format!(
            "openai-compatible profile '{}' requires base_url",
            profile.id
        ));
    }
    if profile.models.is_empty() {
        return Err(format!(
            "openai-compatible profile '{}' requires at least one model",
            profile.id
        ));
    }

    let mut models = Vec::with_capacity(profile.models.len());
    for model in &profile.models {
        if model.id.trim().is_empty() {
            return Err(format!(
                "openai-compatible profile '{}' has a model with an empty id",
                profile.id
            ));
        }
        models.push(ModelInfo {
            id: model.id.clone(),
            display_name: if model.display_name.is_empty() {
                model.id.clone()
            } else {
                model.display_name.clone()
            },
            context_window: model.context_window,
            max_output_tokens: model.max_output_tokens,
            supports_images: model.supports_images,
            supports_streaming: model.supports_streaming,
            supports_thinking: model.supports_thinking,
        });
    }

    let compat = opi_ai::openai_chat::CompatConfig {
        system_role_override: profile.system_role_override.clone(),
        max_tokens_field: profile
            .max_tokens_field
            .clone()
            .unwrap_or_else(|| "max_tokens".into()),
        tool_result_name_field: profile.tool_result_name_field,
        usage_in_stream: profile.usage_in_stream,
        strict_tool_schema: profile.strict_tool_schema,
        reasoning_effort: profile.reasoning_effort.clone(),
        cache_key: profile.cache_key.clone(),
        require_assistant_after_tool_result: profile.require_assistant_after_tool_result,
        chat_completions_path: profile
            .chat_completions_path
            .clone()
            .unwrap_or_else(|| "/v1/chat/completions".into()),
    };

    // Session-affinity headers from config (Phase 12.3).
    let extra_headers: Vec<(String, String)> = profile.extra_headers.clone();

    // Model-level compat overrides win over the provider-level default for the
    // models that declare them (Phase 12.3 provider/model precedence).
    let mut model_overrides: std::collections::HashMap<
        String,
        opi_ai::openai_chat::ModelCompatOverride,
    > = std::collections::HashMap::new();
    for model in &profile.models {
        if model.system_role_override.is_some() || model.max_tokens_field.is_some() {
            model_overrides.insert(
                model.id.clone(),
                opi_ai::openai_chat::ModelCompatOverride {
                    system_role_override: model.system_role_override.clone(),
                    max_tokens_field: model.max_tokens_field.clone(),
                },
            );
        }
    }

    Ok(opi_ai::openai_chat::OpenAiChatProvider::new_for_profile(
        api_key,
        profile.base_url.clone(),
        profile.id.clone(),
        compat,
        extra_headers,
        models,
    )
    .with_model_overrides(model_overrides)
    .with_shared_client(client))
}

// ---------------------------------------------------------------------------
// Runtime provider construction (the active provider for a run)
// ---------------------------------------------------------------------------

/// Build the active runtime provider for the model spec in `config.defaults.model`.
pub fn build_provider(config: &OpiConfig) -> Result<Box<dyn Provider>, ProviderBuildError> {
    let spec = &config.defaults.model;
    let (provider_id, _) = parse_model_spec(spec).map_err(|_| {
        ProviderBuildError::Config(format!(
            "invalid model spec: {spec:?} (expected provider:model)"
        ))
    })?;

    build_runtime_provider(config, provider_id, None)
}

/// Build the active provider, resolving its API key via `resolver`
/// (keychain-first with env fallback). This is the Phase 14 production path;
/// it composes [`crate::credential_store::CredentialResolver`] with provider
/// construction. Bedrock and other non-API-key providers ignore the resolver
/// and use their existing credential chain.
pub async fn build_provider_with_resolver(
    config: &OpiConfig,
    resolver: &crate::credential_store::CredentialResolver,
) -> Result<Box<dyn Provider>, ProviderBuildError> {
    let spec = &config.defaults.model;
    let (provider_id, _) = parse_model_spec(spec).map_err(|_| {
        ProviderBuildError::Config(format!(
            "invalid model spec: {spec:?} (expected provider:model)"
        ))
    })?;
    // Resolve via the configured env var name (keychain -> env fallback).
    let pre_resolved = if let Some(env_name) = api_key_env_name(config, provider_id) {
        resolver
            .resolve_api_key(provider_id, &env_name)
            .await
            .map(|resolved| {
                // `source` is diagnostic-only; the secret is exposed only at
                // this narrow construction boundary.
                resolved.value.expose_secret().to_owned()
            })
    } else {
        // Non-API-key providers (bedrock) have no env_name; fall through to
        // their own credential chain below.
        None
    };
    // Surface the headless backend-unavailable fallback as a warning so the
    // diagnostic is visible without blocking construction (env fallback keeps
    // the provider usable).
    build_runtime_provider(config, provider_id, pre_resolved)
}

// ---------------------------------------------------------------------------
// Phase 14.2 OAuth provider construction
// ---------------------------------------------------------------------------

/// Default Copilot API base URL. A stored OAuth credential may carry an
/// enterprise host (its `base_url`), which takes precedence at construction.
const COPILOT_DEFAULT_BASE_URL: &str = "https://api.individual.githubcopilot.com";

/// Default Codex API base URL.
const CODEX_DEFAULT_BASE_URL: &str = "https://api.openai.com";

/// The static required Copilot Chat headers (the "captured required headers"
/// minus `X-Initiator`, which is per-request and deferred — see RESIDUAL).
fn copilot_extra_headers() -> Vec<(String, String)> {
    vec![
        ("Editor-Version".into(), "opi/0.7.0".into()),
        ("Editor-Plugin-Version".into(), "opi/0.7.0".into()),
        ("Copilot-Integration-Id".into(), "vscode-chat".into()),
        ("Openai-Intent".into(), "conversation-edits".into()),
    ]
}

/// The static required Codex Responses headers.
fn codex_extra_headers() -> Vec<(String, String)> {
    vec![
        ("OpenAI-Beta".into(), "responses=experimental".into()),
        ("originator".into(), "opi".into()),
        ("accept".into(), "text/event-stream".into()),
    ]
}

/// Build the Anthropic OAuth provider (Bearer auth with the beta header,
/// applied inside the provider's stream). Auth is resolved per-request via
/// [`AuthSource::Store`], refreshing near expiry through the resolver. `base_url`
/// follows the config default (Anthropic PKCE derives no per-credential base URL).
async fn build_anthropic_oauth(
    config: &OpiConfig,
    resolver: &CredentialResolver,
    registry: &OAuthProviderRegistry,
) -> Result<Box<dyn Provider>, ProviderBuildError> {
    let oauth = registry.lookup("anthropic").ok_or_else(|| {
        ProviderBuildError::Config("no anthropic OAuth provider registered".into())
    })?;
    let client = build_proxied_client(config.providers.anthropic.proxy.as_ref())?;
    let provider = opi_ai::anthropic::AnthropicProvider::with_auth(
        Arc::new(AuthSource::Store {
            resolver: Arc::new(resolver.clone()),
            provider_id: "anthropic".into(),
            oauth,
        }),
        config.providers.anthropic.base_url.clone(),
        client,
    );
    Ok(Box::new(provider))
}

/// Build the Anthropic OAuth provider from a non-refreshable
/// `ANTHROPIC_OAUTH_TOKEN` environment variable. Precedence: a stored OAuth
/// credential wins (-> [`build_anthropic_oauth`]); this path is the fallback
/// when `ANTHROPIC_OAUTH_TOKEN` is present but no credential is stored. The
/// token is used until 401, then the turn stops (CredentialRevoked) and
/// explicit re-login is required — no auto-refresh.
async fn build_anthropic_env_oauth(
    config: &OpiConfig,
    resolver: &CredentialResolver,
) -> Result<Box<dyn Provider>, ProviderBuildError> {
    let client = build_proxied_client(config.providers.anthropic.proxy.as_ref())?;
    let provider = opi_ai::anthropic::AnthropicProvider::with_auth(
        Arc::new(AuthSource::EnvOAuthToken {
            env_var: "ANTHROPIC_OAUTH_TOKEN".into(),
            env_lookup: resolver.env_lookup(),
        }),
        config.providers.anthropic.base_url.clone(),
        client,
    );
    Ok(Box::new(provider))
}

/// Build the Copilot Chat provider (OpenAI Chat compat profile) over a stored
/// OAuth credential. The per-credential `base_url` (enterprise host) is read
/// from the store at construction; absent that, the default Copilot host.
async fn build_copilot_oauth(
    resolver: &CredentialResolver,
    registry: &OAuthProviderRegistry,
) -> Result<Box<dyn Provider>, ProviderBuildError> {
    let oauth = registry
        .lookup("copilot")
        .ok_or_else(|| ProviderBuildError::Config("no copilot OAuth provider registered".into()))?;
    let base_url = resolver
        .read_oauth_base_url("copilot")
        .await
        .map_err(ProviderBuildError::Provider)?
        .unwrap_or_else(|| COPILOT_DEFAULT_BASE_URL.to_owned());
    let client = build_proxied_client(None)?;
    // Copilot Chat uses `/chat/completions` (no `/v1`); the Copilot compat
    // profile overrides the path the default OpenAI config would use.
    let compat = opi_ai::openai_chat::CompatConfig {
        chat_completions_path: "/chat/completions".into(),
        ..Default::default()
    };
    let provider = opi_ai::openai_chat::OpenAiChatProvider::with_auth(
        Arc::new(AuthSource::Store {
            resolver: Arc::new(resolver.clone()),
            provider_id: "copilot".into(),
            oauth,
        }),
        Some(base_url),
        compat,
        "copilot".into(),
        copilot_extra_headers(),
        client,
    );
    Ok(Box::new(provider))
}

/// Build the Codex Responses provider (Codex compat profile) over a stored
/// OAuth credential, targeting `/codex/responses` with the account-id
/// derivation and the required Codex headers. The base URL is the stored
/// credential's `base_url` when present, otherwise [`CODEX_DEFAULT_BASE_URL`]
/// (production Codex PKCE carries no base URL, so the default always wins).
async fn build_codex_oauth(
    resolver: &CredentialResolver,
    registry: &OAuthProviderRegistry,
) -> Result<Box<dyn Provider>, ProviderBuildError> {
    let oauth = registry
        .lookup("codex")
        .ok_or_else(|| ProviderBuildError::Config("no codex OAuth provider registered".into()))?;
    let base_url = resolver
        .read_oauth_base_url("codex")
        .await
        .map_err(ProviderBuildError::Provider)?
        .unwrap_or_else(|| CODEX_DEFAULT_BASE_URL.to_owned());
    let client = build_proxied_client(None)?;
    let config = opi_ai::openai_responses::ResponsesConfig {
        responses_path: "/codex/responses".into(),
        derive_codex_account_id: true,
        ..Default::default()
    };
    let provider = opi_ai::openai_responses::OpenAiResponsesProvider::with_auth_extra(
        Arc::new(AuthSource::Store {
            resolver: Arc::new(resolver.clone()),
            provider_id: "codex".into(),
            oauth,
        }),
        Some(base_url),
        config,
        "codex".into(),
        codex_extra_headers(),
        client,
    );
    Ok(Box::new(provider))
}

/// Build the active provider, routing Anthropic/Copilot/Codex to their OAuth
/// builders when an OAuth credential is in play, and falling through to the
/// API-key resolver path for everything else. This is the Phase 14.2 production
/// routing entry point; [`build_provider_production`] constructs the resolver +
/// registry and delegates here.
///
/// Routing:
/// - `copilot:` / `codex:` model specs are OAuth-only -> their OAuth builder.
/// - `anthropic:` -> precedence: stored OAuth credential >
///   `ANTHROPIC_OAUTH_TOKEN` env > API-key env/fallback.
/// - everything else -> the API-key path.
pub async fn build_provider_with_oauth(
    config: &OpiConfig,
    resolver: &CredentialResolver,
    registry: &OAuthProviderRegistry,
) -> Result<Box<dyn Provider>, ProviderBuildError> {
    let spec = &config.defaults.model;
    let (provider_id, _) = parse_model_spec(spec).map_err(|_| {
        ProviderBuildError::Config(format!(
            "invalid model spec: {spec:?} (expected provider:model)"
        ))
    })?;
    match provider_id {
        "copilot" => build_copilot_oauth(resolver, registry).await,
        "codex" => build_codex_oauth(resolver, registry).await,
        "anthropic" => {
            let has_store = resolver
                .has_oauth_credential("anthropic")
                .await
                .map_err(ProviderBuildError::Provider)?;
            if has_store {
                build_anthropic_oauth(config, resolver, registry).await
            } else if resolver.env_value("ANTHROPIC_OAUTH_TOKEN").is_some() {
                build_anthropic_env_oauth(config, resolver).await
            } else {
                build_provider_with_resolver(config, resolver).await
            }
        }
        _ => build_provider_with_resolver(config, resolver).await,
    }
}

/// Build the [`ProviderBundle`] (provider + store + resolver + OAuth registry)
/// for production startup. Callers that need the store and registry for
/// `/login`, `/logout`, or `CredentialNeeded` retry take the full bundle; others
/// extract only the provider.
pub async fn build_provider_bundle(
    config: &OpiConfig,
    user_config_dir: std::path::PathBuf,
) -> Result<ProviderBundle, ProviderBuildError> {
    let store = Arc::new(crate::credential_store::KeychainCredentialStore::new(
        Box::new(crate::credential_store::KeyringCoreBackend::new()),
        user_config_dir,
    ));
    let resolver = crate::credential_store::CredentialResolver::production(store.clone());
    let registry = crate::oauth::OAuthProviderRegistry::registry_with_builtins();
    let provider = build_provider_with_oauth(config, &resolver, &registry).await?;
    Ok(ProviderBundle {
        provider,
        store,
        resolver,
        registry,
    })
}

/// Build the active provider through the production credential resolver: a
/// [`crate::credential_store::KeychainCredentialStore`] over the platform
/// keychain with env fallback, plus the built-in OAuth provider registry. Routes
/// Anthropic/Copilot/Codex to their OAuth builders when an OAuth credential is
/// stored, and falls through to the API-key path otherwise. This is a
/// convenience wrapper around [`build_provider_bundle`]; callers that need the
/// store + registry for `/login`/`/logout` should call [`build_provider_bundle`]
/// directly.
pub async fn build_provider_production(
    config: &OpiConfig,
    user_config_dir: std::path::PathBuf,
) -> Result<Box<dyn Provider>, ProviderBuildError> {
    build_provider_bundle(config, user_config_dir)
        .await
        .map(|bundle| bundle.provider)
}

/// Resolve the API-key env var name for a built-in provider, or `None` for
/// providers that do not source a single API key (bedrock).
fn api_key_env_name(config: &OpiConfig, provider_id: &str) -> Option<String> {
    match provider_id {
        "anthropic" => Some(config.providers.anthropic.api_key_env.clone()),
        "openai" => Some(resolve_env_name(
            &config.providers.openai.api_key_env,
            "OPENAI_API_KEY",
        )),
        "openrouter" => Some(resolve_env_name(
            &config.providers.openrouter.api_key_env,
            "OPENROUTER_API_KEY",
        )),
        "mistral" => Some(resolve_env_name(
            &config.providers.mistral.api_key_env,
            "MISTRAL_API_KEY",
        )),
        "openai-responses" => Some(resolve_env_name(
            &config.providers.openai_responses.api_key_env,
            "OPENAI_API_KEY",
        )),
        "gemini" => Some(resolve_env_name(
            &config.providers.gemini.api_key_env,
            "GEMINI_API_KEY",
        )),
        "azure" => Some(resolve_env_name(
            &config.providers.azure.api_key_env,
            "AZURE_OPENAI_API_KEY",
        )),
        "vertex" => Some(resolve_env_name(
            &config.providers.vertex.access_token_env,
            "VERTEX_ACCESS_TOKEN",
        )),
        _ => None,
    }
}

/// Return a pre-resolved key if present, otherwise read it from `env_name`.
fn resolved_or_env(
    pre_resolved: Option<String>,
    env_name: &str,
) -> Result<String, ProviderBuildError> {
    match pre_resolved {
        Some(key) => Ok(key),
        None => require_api_key(env_name),
    }
}

fn build_runtime_provider(
    config: &OpiConfig,
    provider_id: &str,
    pre_resolved: Option<String>,
) -> Result<Box<dyn Provider>, ProviderBuildError> {
    let spec = &config.defaults.model;
    match provider_id {
        "anthropic" => {
            let env_name = &config.providers.anthropic.api_key_env;
            let api_key = resolved_or_env(pre_resolved.clone(), env_name)?;
            let client = build_proxied_client(config.providers.anthropic.proxy.as_ref())?;
            let provider = opi_ai::anthropic::AnthropicProvider::with_client(
                api_key,
                config.providers.anthropic.base_url.clone(),
                client,
            );
            Ok(Box::new(provider) as Box<dyn Provider>)
        }
        "openai" => {
            let env_name = resolve_env_name(&config.providers.openai.api_key_env, "OPENAI_API_KEY");
            let api_key = resolved_or_env(pre_resolved.clone(), &env_name)?;
            let client = build_proxied_client(config.providers.openai.proxy.as_ref())?;
            let provider = opi_ai::openai_chat::OpenAiChatProvider::with_client(
                api_key,
                config.providers.openai.base_url.clone(),
                "openai".into(),
                vec![],
                client,
            );
            Ok(Box::new(provider) as Box<dyn Provider>)
        }
        "openrouter" => {
            let env_name = resolve_env_name(
                &config.providers.openrouter.api_key_env,
                "OPENROUTER_API_KEY",
            );
            let api_key = resolved_or_env(pre_resolved.clone(), &env_name)?;
            let client = build_proxied_client(config.providers.openrouter.proxy.as_ref())?;
            // If a custom referer is configured, build the provider directly with it.
            let provider = if let Some(ref referer) = config.providers.openrouter.referer {
                let base_url = config
                    .providers
                    .openrouter
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "https://openrouter.ai/api".into());
                let compat = opi_ai::openai_chat::CompatConfig::default();
                let extra_headers = vec![
                    ("HTTP-Referer".into(), referer.clone()),
                    ("X-Title".into(), "opi".into()),
                ];
                // Use the default model list from the openrouter module.
                let temp = opi_ai::openrouter::openrouter_provider(
                    String::new(),
                    config.providers.openrouter.base_url.clone(),
                );
                let models = temp.models().to_vec();
                opi_ai::openai_chat::OpenAiChatProvider::new_for_profile(
                    api_key,
                    base_url,
                    "openrouter".into(),
                    compat,
                    extra_headers,
                    models,
                )
                .with_shared_client(client)
            } else {
                opi_ai::openrouter::openrouter_provider(
                    api_key,
                    config.providers.openrouter.base_url.clone(),
                )
                .with_shared_client(client)
            };
            Ok(Box::new(provider) as Box<dyn Provider>)
        }
        "mistral" => {
            let env_name =
                resolve_env_name(&config.providers.mistral.api_key_env, "MISTRAL_API_KEY");
            let api_key = resolved_or_env(pre_resolved.clone(), &env_name)?;
            let client = build_proxied_client(config.providers.mistral.proxy.as_ref())?;
            let provider = opi_ai::mistral::mistral_provider(
                api_key,
                config.providers.mistral.base_url.clone(),
            )
            .with_shared_client(client);
            Ok(Box::new(provider) as Box<dyn Provider>)
        }
        "openai-responses" => {
            let env_name = resolve_env_name(
                &config.providers.openai_responses.api_key_env,
                "OPENAI_API_KEY",
            );
            let api_key = resolved_or_env(pre_resolved.clone(), &env_name)?;
            let client = build_proxied_client(config.providers.openai_responses.proxy.as_ref())?;
            let provider = opi_ai::openai_responses::OpenAiResponsesProvider::with_client(
                api_key,
                config.providers.openai_responses.base_url.clone(),
                client,
            );
            Ok(Box::new(provider) as Box<dyn Provider>)
        }
        "gemini" => {
            let env_name = resolve_env_name(&config.providers.gemini.api_key_env, "GEMINI_API_KEY");
            let api_key = resolved_or_env(pre_resolved.clone(), &env_name)?;
            let client = build_proxied_client(config.providers.gemini.proxy.as_ref())?;
            let provider = opi_ai::gemini::GeminiProvider::with_client(
                api_key,
                config.providers.gemini.base_url.clone(),
                client,
            );
            Ok(Box::new(provider) as Box<dyn Provider>)
        }
        "bedrock" => {
            let bedrock_config = &config.providers.bedrock;

            // Resolve credentials: config > env > profile
            let (akid, sak, token, env_region) = resolve_bedrock_env_credentials();
            let env_profile = std::env::var("AWS_PROFILE").ok();
            let profile_name = bedrock_config.profile.as_deref().or(env_profile.as_deref());
            let credentials_file = aws_credentials_path();
            let config_file = aws_config_path();

            // Read secret key from configured env var
            let secret_key = bedrock_config
                .secret_access_key_env
                .as_deref()
                .and_then(non_empty_env_var);

            // Read session token from configured env var
            let session_token = bedrock_config
                .session_token_env
                .as_deref()
                .and_then(non_empty_env_var);

            let input = opi_ai::bedrock::credentials::CredentialResolutionInput {
                config_access_key_id: bedrock_config.access_key_id.as_deref(),
                config_secret_access_key: secret_key.as_deref(),
                config_session_token: session_token.as_deref(),
                config_region: bedrock_config.region.as_deref(),
                env_access_key_id: akid.as_deref(),
                env_secret_access_key: sak.as_deref(),
                env_session_token: token.as_deref(),
                env_region: env_region.as_deref(),
                profile_name,
                credentials_file_path: credentials_file.as_deref(),
                config_file_path: config_file.as_deref(),
            };

            let resolved = opi_ai::bedrock::credentials::resolve_credentials(&input);

            let (bedrock_creds, _source) = resolved.ok_or_else(|| {
                ProviderBuildError::Auth(
                    "no AWS credentials found: set AWS_ACCESS_KEY_ID/AWS_SECRET_ACCESS_KEY env vars, configure [providers.bedrock], or set up AWS shared credentials/config profiles".into(),
                )
            })?;

            let client = build_proxied_client(bedrock_config.proxy.as_ref())?;
            let provider = opi_ai::bedrock::BedrockProvider::from_credentials(
                bedrock_creds,
                bedrock_config.base_url.clone(),
                client,
            );
            Ok(Box::new(provider) as Box<dyn Provider>)
        }
        "azure" => {
            let azure_config = &config.providers.azure;
            let env_name = resolve_env_name(&azure_config.api_key_env, "AZURE_OPENAI_API_KEY");
            let api_key = resolved_or_env(pre_resolved.clone(), &env_name)?;

            // Extract deployment name from model spec (azure:deployment-name)
            let deployment = spec.split_once(':').map(|(_, id)| id).unwrap_or("");

            let provider = if azure_config.deployments.is_empty() {
                opi_ai::azure_openai::AzureOpenAIProvider::new(
                    api_key,
                    azure_config.endpoint.clone(),
                    deployment.to_string(),
                    azure_config.api_version.clone(),
                )?
            } else {
                opi_ai::azure_openai::AzureOpenAIProvider::from_config(
                    api_key,
                    azure_config.endpoint.clone(),
                    azure_config.deployments.clone(),
                    azure_config.api_version.clone(),
                )?
            }
            .with_client(build_proxied_client(azure_config.proxy.as_ref())?);
            Ok(Box::new(provider) as Box<dyn Provider>)
        }
        "vertex" => {
            let vertex_config = &config.providers.vertex;
            let env_name = resolve_env_name(&vertex_config.access_token_env, "VERTEX_ACCESS_TOKEN");
            let access_token = resolved_or_env(pre_resolved.clone(), &env_name)?;

            let project = vertex_config.project.as_deref().ok_or_else(|| {
                ProviderBuildError::Config("vertex provider requires project".into())
            })?;
            let location = vertex_config.location.as_deref().ok_or_else(|| {
                ProviderBuildError::Config("vertex provider requires location".into())
            })?;

            let provider = if vertex_config.models.is_empty() {
                opi_ai::vertex::VertexProvider::new(
                    access_token,
                    project.into(),
                    location.into(),
                    vertex_config.base_url.clone(),
                )
            } else {
                opi_ai::vertex::VertexProvider::from_config(
                    access_token,
                    project.into(),
                    location.into(),
                    vertex_config.models.clone(),
                    vertex_config.base_url.clone(),
                )
            }
            .with_client(build_proxied_client(vertex_config.proxy.as_ref())?);
            Ok(Box::new(provider) as Box<dyn Provider>)
        }
        other => {
            if let Some(profile) = config.providers.openai_compatible.get(other) {
                let provider = build_runtime_openai_compatible_profile(profile)?;
                Ok(Box::new(provider) as Box<dyn Provider>)
            } else {
                Err(ProviderBuildError::Config(format!(
                    "unknown provider: {other}"
                )))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Auth + compat descriptor mapping (the auth-seam policy)
// ---------------------------------------------------------------------------

/// Derive the redacted auth descriptor a config-sourced built-in provider
/// reports through the collection. Returns `None` for ids that are not
/// config-sourced built-ins (e.g. extension-supplied provider ids).
///
/// Bedrock resolves credentials from several sources (env/profile/file); the
/// descriptor reflects its primary env var (`AWS_ACCESS_KEY_ID`) for redacted
/// status reporting and does not gate dispatch.
pub fn auth_descriptor_for(config: &OpiConfig, provider_id: &str) -> Option<AuthDescriptor> {
    let env_var = match provider_id {
        "anthropic" => config.providers.anthropic.api_key_env.clone(),
        "openai" => resolve_env_name(&config.providers.openai.api_key_env, "OPENAI_API_KEY"),
        "openrouter" => resolve_env_name(
            &config.providers.openrouter.api_key_env,
            "OPENROUTER_API_KEY",
        ),
        "mistral" => resolve_env_name(&config.providers.mistral.api_key_env, "MISTRAL_API_KEY"),
        "openai-responses" => resolve_env_name(
            &config.providers.openai_responses.api_key_env,
            "OPENAI_API_KEY",
        ),
        "gemini" => resolve_env_name(&config.providers.gemini.api_key_env, "GEMINI_API_KEY"),
        "azure" => resolve_env_name(&config.providers.azure.api_key_env, "AZURE_OPENAI_API_KEY"),
        "vertex" => resolve_env_name(
            &config.providers.vertex.access_token_env,
            "VERTEX_ACCESS_TOKEN",
        ),
        "bedrock" => "AWS_ACCESS_KEY_ID".to_string(),
        _ => return None,
    };
    // Phase 14 opt-in: API-key providers (everything except Bedrock's AWS
    // credential chain) describe their credential as keychain-sourced when the
    // user selects the keychain backend. The descriptor is secret-free; the
    // redacted probe state is injected separately by the caller.
    if config.defaults.credential_backend == Some(crate::config::CredentialBackendSource::Keychain)
        && provider_id != "bedrock"
    {
        Some(AuthDescriptor::StoreCredential {
            key: provider_id.to_owned(),
            display_source: format!("keychain opi:{provider_id}"),
        })
    } else {
        Some(AuthDescriptor::EnvApiKey { env_var })
    }
}

fn resolved_auth_descriptor_for(config: &OpiConfig, provider_id: &str) -> AuthDescriptor {
    let source = if provider_id == "bedrock" {
        "aws credential chain".to_string()
    } else if let Some(AuthDescriptor::EnvApiKey { env_var }) =
        auth_descriptor_for(config, provider_id)
    {
        format!("env {env_var}")
    } else {
        "configured".to_string()
    };
    AuthDescriptor::Resolved { source }
}

/// Derive the auth descriptor for a user-declared openai_compatible profile.
pub fn auth_descriptor_for_profile(profile: &OpenAiCompatibleProviderConfig) -> AuthDescriptor {
    let default = profile_api_key_env_default(&profile.id);
    let env_var = resolve_env_name(&profile.api_key_env, &default);
    AuthDescriptor::EnvApiKey { env_var }
}

fn resolved_auth_descriptor_for_profile(
    profile: &OpenAiCompatibleProviderConfig,
) -> AuthDescriptor {
    let default = profile_api_key_env_default(&profile.id);
    let env_var = resolve_env_name(&profile.api_key_env, &default);
    AuthDescriptor::Resolved {
        source: format!("env {env_var}"),
    }
}

/// Compat metadata for a built-in provider id. Built-ins do not carry
/// user-declared openai_compatible profile flags at the collection level.
pub fn compat_metadata_for(provider_id: &str) -> CompatMetadata {
    match provider_id {
        "openai" | "openrouter" | "mistral" => CompatMetadata {
            openai_compatible: true,
            profile: Some(provider_id.to_owned()),
        },
        _ => CompatMetadata::default(),
    }
}

/// Compat metadata for a user-declared openai_compatible profile.
pub fn compat_metadata_for_profile(profile: &OpenAiCompatibleProviderConfig) -> CompatMetadata {
    CompatMetadata {
        openai_compatible: true,
        profile: Some(profile.id.clone()),
    }
}

// ---------------------------------------------------------------------------
// Collection assembly
// ---------------------------------------------------------------------------

/// Build the provider collection for `--list-models` from CLI config + env.
///
/// Each config-sourced provider that successfully constructs is registered
/// through [`ProviderCollection::register`] with its derived auth descriptor
/// and compatibility metadata, so listing routes through the collection/auth
/// seam. Providers with missing credentials are skipped silently; broken
/// config (e.g. invalid proxy) is fatal.
pub fn build_collection_for_listing(
    config: &OpiConfig,
    store_probe: &std::collections::HashMap<String, opi_ai::CredentialSource>,
) -> Result<ProviderCollection, ListModelsError> {
    let mut collection = ProviderCollection::new();
    for provider_id in BUILT_IN_PROVIDER_IDS {
        match build_list_models_provider(config, provider_id) {
            Ok(provider) => {
                // Preserve the legacy Resolved descriptor for env-backed
                // providers; only keychain-backend (StoreCredential) providers
                // carry the secret-free store descriptor + injected probe.
                let auth = match auth_descriptor_for(config, provider_id) {
                    Some(store_auth @ AuthDescriptor::StoreCredential { .. }) => store_auth,
                    _ => resolved_auth_descriptor_for(config, provider_id),
                };
                let compat = compat_metadata_for(provider_id);
                if let Err(e) = collection.register(provider, auth.clone(), compat) {
                    return Err(ListModelsError::Config(format!(
                        "provider registration failed: {e}"
                    )));
                }
                // Inject the redacted, precomputed probe for StoreCredential
                // providers so the listing collection carries keychain state
                // without the secret.
                if matches!(auth, AuthDescriptor::StoreCredential { .. }) {
                    let source = store_probe
                        .get(*provider_id)
                        .cloned()
                        .unwrap_or(opi_ai::CredentialSource::Absent);
                    collection.set_probe(provider_id, source);
                }
            }
            Err(ListModelsError::MissingCredentials) => continue,
            Err(e @ ListModelsError::Config(_)) => return Err(e),
        }
    }
    for profile in config.providers.openai_compatible.values() {
        match build_list_models_openai_compatible_profile(profile) {
            Ok(provider) => {
                let auth = resolved_auth_descriptor_for_profile(profile);
                let compat = compat_metadata_for_profile(profile);
                if let Err(e) = collection.register(Box::new(provider), auth, compat) {
                    return Err(ListModelsError::Config(format!(
                        "profile registration failed: {e}"
                    )));
                }
            }
            Err(ListModelsError::MissingCredentials) => continue,
            Err(e @ ListModelsError::Config(_)) => return Err(e),
        }
    }
    Ok(collection)
}

/// Assemble the harness model-lookup collection from an already-built active
/// provider plus extension providers and model overrides.
///
/// The active provider is wrapped in `MetadataProvider` so its models appear
/// in listing/picker/resolution. Because the active provider and extension
/// providers are not config-sourced at this layer (the active provider's
/// credentials were validated at build time), the collection is built via
/// [`ProviderCollection::from_registry`] with no auth descriptors, preserving
/// the existing non-gated dispatch behavior.
pub fn assemble_harness_collection(
    provider: &dyn Provider,
    extension_registry: Option<&ExtensionRegistry>,
) -> (ProviderCollection, Vec<Diagnostic>) {
    let mut registry = ProviderRegistry::new();
    let mut diagnostics = Vec::new();

    if let Some(extension_registry) = extension_registry {
        for provider in extension_registry.collect_providers() {
            if let Err(e) = registry.register_provider(provider) {
                diagnostics.push(diagnostic_for_model_registry_error(format!(
                    "extension provider registration failed: {e}"
                )));
            }
        }
    }

    if let Err(e) = registry.register_provider(Box::new(MetadataProvider::from_provider(provider)))
    {
        diagnostics.push(diagnostic_for_model_registry_error(format!(
            "active provider metadata registration failed: {e}"
        )));
    }

    if let Some(extension_registry) = extension_registry {
        for (provider_id, model) in extension_registry.collect_model_overrides() {
            if let Err(e) = registry.register_model(&provider_id, model) {
                diagnostics.push(diagnostic_for_model_registry_error(format!(
                    "extension model override registration failed: {e}"
                )));
            }
        }
    }

    (ProviderCollection::from_registry(registry), diagnostics)
}
