//! TOML config loading (S9.1/S9.1.1).
//!
//! Loads and resolves opi configuration with precedence:
//! CLI > env > project config > user config > built-in defaults.
//!
//! Phase 1 fields: model, max_iterations, tool_timeout_ms, theme,
//! thinking, providers.anthropic.api_key_env.
//!
//! Phase 2 fields: providers.{openai,openrouter,mistral,openai_responses,gemini}
//! config with api_key_env, base_url, and OpenRouter-specific referer.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Resolved config (public API — all fields present)
// ---------------------------------------------------------------------------

/// Top-level opi configuration (fully resolved).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OpiConfig {
    pub defaults: DefaultsConfig,
    pub thinking: ThinkingConfig,
    pub providers: ProvidersConfig,
    pub keybindings: KeybindingsConfig,
    pub retry: opi_ai::retry::RetryConfig,
    pub compaction: CompactionConfigSection,
    pub extensions: ExtensionsConfig,
    pub packages: PackagesConfig,
}

/// `[defaults]` section.
#[derive(Debug, Clone, PartialEq)]
pub struct DefaultsConfig {
    pub model: String,
    pub max_iterations: u32,
    pub tool_timeout_ms: u64,
    pub max_image_bytes: u64,
    pub theme: String,
    pub allow_mutating_tools: bool,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            model: "anthropic:claude-sonnet-4".into(),
            max_iterations: 50,
            tool_timeout_ms: 30_000,
            max_image_bytes: crate::image::DEFAULT_MAX_IMAGE_BYTES,
            theme: "default".into(),
            allow_mutating_tools: false,
        }
    }
}

/// `[thinking]` section.
#[derive(Debug, Clone, PartialEq)]
pub struct ThinkingConfig {
    pub enabled: bool,
    pub budget_tokens: u32,
}

impl Default for ThinkingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            budget_tokens: 10_000,
        }
    }
}

/// `[providers]` section.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProvidersConfig {
    pub anthropic: AnthropicProviderConfig,
    pub openai: GenericProviderConfig,
    pub openrouter: OpenRouterProviderConfig,
    pub mistral: GenericProviderConfig,
    pub openai_responses: GenericProviderConfig,
    pub gemini: GenericProviderConfig,
    pub bedrock: BedrockProviderConfig,
    pub azure: AzureProviderConfig,
    pub vertex: VertexProviderConfig,
    pub openai_compatible: BTreeMap<String, OpenAiCompatibleProviderConfig>,
}

/// `[providers.anthropic]` section.
#[derive(Debug, Clone, PartialEq)]
pub struct AnthropicProviderConfig {
    pub api_key_env: String,
    pub base_url: Option<String>,
    pub proxy: Option<ProviderProxyConfig>,
}

impl Default for AnthropicProviderConfig {
    fn default() -> Self {
        Self {
            api_key_env: "ANTHROPIC_API_KEY".into(),
            base_url: None,
            proxy: None,
        }
    }
}

/// Generic provider config (api_key_env + optional base_url + optional proxy).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GenericProviderConfig {
    pub api_key_env: String,
    pub base_url: Option<String>,
    pub proxy: Option<ProviderProxyConfig>,
}

/// OpenRouter-specific provider config.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OpenRouterProviderConfig {
    pub api_key_env: String,
    pub base_url: Option<String>,
    pub referer: Option<String>,
    pub proxy: Option<ProviderProxyConfig>,
}

/// `[providers.bedrock]` section.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BedrockProviderConfig {
    /// Explicit access key ID (overrides env var).
    pub access_key_id: Option<String>,
    /// Env var name for secret access key (default: AWS_SECRET_ACCESS_KEY).
    pub secret_access_key_env: Option<String>,
    /// Env var name for session token (default: AWS_SESSION_TOKEN).
    pub session_token_env: Option<String>,
    /// AWS region (default: us-east-1).
    pub region: Option<String>,
    /// AWS config profile name for credential file lookup.
    pub profile: Option<String>,
    /// Override base URL for Bedrock runtime API.
    pub base_url: Option<String>,
    /// Proxy configuration.
    pub proxy: Option<ProviderProxyConfig>,
}

/// `[providers.azure]` section.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AzureProviderConfig {
    /// Env var name for the Azure OpenAI API key (default: AZURE_OPENAI_API_KEY).
    pub api_key_env: String,
    /// Azure OpenAI endpoint (e.g. `https://myresource.openai.azure.com`).
    pub endpoint: Option<String>,
    /// Azure API version (default: 2024-06-01).
    pub api_version: Option<String>,
    /// Deployment names to advertise in --list-models.
    pub deployments: Vec<String>,
    /// Proxy configuration.
    pub proxy: Option<ProviderProxyConfig>,
}

/// `[providers.vertex]` section.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VertexProviderConfig {
    /// Env var name for the OAuth2 access token (default: VERTEX_ACCESS_TOKEN).
    pub access_token_env: String,
    /// GCP project ID.
    pub project: Option<String>,
    /// GCP location/region (e.g. `us-central1`).
    pub location: Option<String>,
    /// Model names to advertise in --list-models.
    pub models: Vec<String>,
    /// Override base URL for Vertex AI API.
    pub base_url: Option<String>,
    /// Proxy configuration.
    pub proxy: Option<ProviderProxyConfig>,
}

/// Configured OpenAI-compatible provider profile.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OpenAiCompatibleProviderConfig {
    /// Provider id used in `provider:model` specs.
    pub id: String,
    /// Env var name for the API key.
    pub api_key_env: String,
    /// Base URL without the `/v1/chat/completions` suffix.
    pub base_url: String,
    /// Models advertised by this profile.
    pub models: Vec<ConfiguredModelConfig>,
    /// Optional role override for system messages.
    pub system_role_override: Option<String>,
    /// Optional request field name for max token limits.
    pub max_tokens_field: Option<String>,
    /// Whether tool result messages should include a `name` field.
    pub tool_result_name_field: bool,
    /// Whether usage can appear throughout the stream.
    pub usage_in_stream: bool,
    /// Emit `"strict": true` on function tool definitions (Phase 12.3).
    pub strict_tool_schema: bool,
    /// Emit a top-level `reasoning_effort` for reasoning models (Phase 12.3).
    pub reasoning_effort: Option<String>,
    /// Emit `prompt_cache_key` for OpenAI prompt-cache affinity (Phase 12.3).
    pub cache_key: Option<String>,
    /// Extra request headers (session-affinity) applied to every request.
    pub extra_headers: Vec<(String, String)>,
    /// Compatibility-metadata flag for legacy assistant-after-tool-result wires.
    pub require_assistant_after_tool_result: bool,
    /// Proxy configuration.
    pub proxy: Option<ProviderProxyConfig>,
}

/// Model metadata from provider profile configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConfiguredModelConfig {
    pub id: String,
    pub display_name: String,
    pub context_window: u64,
    pub max_output_tokens: u64,
    pub supports_images: bool,
    pub supports_streaming: bool,
    pub supports_thinking: bool,
    /// Model-level override of the profile `system_role_override` (wins when
    /// present). Phase 12.3 provider/model override precedence.
    pub system_role_override: Option<String>,
    /// Model-level override of the profile `max_tokens_field` (wins when
    /// present). Phase 12.3 provider/model override precedence.
    pub max_tokens_field: Option<String>,
}

/// Per-provider proxy configuration from `[providers.*.proxy]`.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderProxyConfig {
    pub url: String,
    pub no_proxy: Option<String>,
}

/// `[keybindings]` section.
#[derive(Debug, Clone, PartialEq)]
pub struct KeybindingsConfig {
    pub submit: String,
    pub abort: String,
    pub new_line: String,
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            submit: "enter".into(),
            abort: "escape".into(),
            new_line: "alt+enter".into(),
        }
    }
}

/// `[compaction]` section.
#[derive(Debug, Clone, PartialEq)]
pub struct CompactionConfigSection {
    pub enabled: bool,
    pub threshold_tokens: u64,
}

impl Default for CompactionConfigSection {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold_tokens: 100_000,
        }
    }
}

/// `[extensions]` section.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExtensionsConfig {
    pub paths: Vec<PathBuf>,
}

/// `[packages]` section.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PackagesConfig {
    pub paths: Vec<PathBuf>,
}

// ---------------------------------------------------------------------------
// TOML deserialization structs (Option fields detect presence)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlConfig {
    defaults: TomlDefaults,
    thinking: TomlThinking,
    providers: TomlProviders,
    keybindings: TomlKeybindings,
    retry: TomlRetry,
    compaction: TomlCompaction,
    extensions: TomlResourcePaths,
    packages: TomlResourcePaths,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlDefaults {
    model: Option<String>,
    max_iterations: Option<u32>,
    tool_timeout_ms: Option<u64>,
    max_image_bytes: Option<u64>,
    theme: Option<String>,
    allow_mutating_tools: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlThinking {
    enabled: Option<bool>,
    budget_tokens: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlProviders {
    anthropic: TomlAnthropic,
    bedrock: TomlBedrockProvider,
    openai: TomlGenericProvider,
    openrouter: TomlOpenRouterProvider,
    mistral: TomlGenericProvider,
    openai_responses: TomlGenericProvider,
    gemini: TomlGenericProvider,
    azure: TomlAzureProvider,
    vertex: TomlVertexProvider,
    openai_compatible: BTreeMap<String, TomlOpenAiCompatibleProvider>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlAnthropic {
    api_key_env: Option<String>,
    base_url: Option<String>,
    proxy: Option<TomlProxy>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlBedrockProvider {
    access_key_id: Option<String>,
    secret_access_key_env: Option<String>,
    session_token_env: Option<String>,
    region: Option<String>,
    profile: Option<String>,
    base_url: Option<String>,
    proxy: Option<TomlProxy>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlAzureProvider {
    api_key_env: Option<String>,
    endpoint: Option<String>,
    api_version: Option<String>,
    deployments: Option<Vec<String>>,
    proxy: Option<TomlProxy>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlVertexProvider {
    access_token_env: Option<String>,
    project: Option<String>,
    location: Option<String>,
    models: Option<Vec<String>>,
    base_url: Option<String>,
    proxy: Option<TomlProxy>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlGenericProvider {
    api_key_env: Option<String>,
    base_url: Option<String>,
    proxy: Option<TomlProxy>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlOpenRouterProvider {
    api_key_env: Option<String>,
    base_url: Option<String>,
    referer: Option<String>,
    proxy: Option<TomlProxy>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlOpenAiCompatibleProvider {
    api_key_env: Option<String>,
    base_url: Option<String>,
    models: Option<Vec<TomlConfiguredModel>>,
    system_role_override: Option<String>,
    max_tokens_field: Option<String>,
    tool_result_name_field: Option<bool>,
    usage_in_stream: Option<bool>,
    strict_tool_schema: Option<bool>,
    reasoning_effort: Option<String>,
    cache_key: Option<String>,
    extra_headers: Option<BTreeMap<String, String>>,
    require_assistant_after_tool_result: Option<bool>,
    proxy: Option<TomlProxy>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlConfiguredModel {
    id: Option<String>,
    display_name: Option<String>,
    context_window: Option<u64>,
    max_output_tokens: Option<u64>,
    supports_images: Option<bool>,
    supports_streaming: Option<bool>,
    supports_thinking: Option<bool>,
    system_role_override: Option<String>,
    max_tokens_field: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlProxy {
    url: Option<String>,
    no_proxy: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlKeybindings {
    submit: Option<String>,
    abort: Option<String>,
    new_line: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlRetry {
    max_attempts: Option<u32>,
    initial_delay_ms: Option<u64>,
    max_delay_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlCompaction {
    enabled: Option<bool>,
    threshold_tokens: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlResourcePaths {
    paths: Option<Vec<PathBuf>>,
}

impl TomlConfig {
    fn merge_into(self, config: &mut OpiConfig) {
        if let Some(v) = self.defaults.model {
            config.defaults.model = v;
        }
        if let Some(v) = self.defaults.max_iterations {
            config.defaults.max_iterations = v;
        }
        if let Some(v) = self.defaults.tool_timeout_ms {
            config.defaults.tool_timeout_ms = v;
        }
        if let Some(v) = self.defaults.max_image_bytes {
            config.defaults.max_image_bytes = v;
        }
        if let Some(v) = self.defaults.theme {
            config.defaults.theme = v;
        }
        if let Some(v) = self.defaults.allow_mutating_tools {
            config.defaults.allow_mutating_tools = v;
        }
        if let Some(v) = self.thinking.enabled {
            config.thinking.enabled = v;
        }
        if let Some(v) = self.thinking.budget_tokens {
            config.thinking.budget_tokens = v;
        }
        if let Some(v) = self.providers.anthropic.api_key_env {
            config.providers.anthropic.api_key_env = v;
        }
        if let Some(v) = self.providers.anthropic.base_url {
            config.providers.anthropic.base_url = Some(v);
        }
        if let Some(p) = self.providers.anthropic.proxy
            && let Some(url) = p.url.filter(|s| !s.trim().is_empty())
        {
            config.providers.anthropic.proxy = Some(ProviderProxyConfig {
                url,
                no_proxy: p.no_proxy,
            });
        }
        if let Some(v) = self.providers.bedrock.access_key_id {
            config.providers.bedrock.access_key_id = Some(v);
        }
        if let Some(v) = self.providers.bedrock.secret_access_key_env {
            config.providers.bedrock.secret_access_key_env = Some(v);
        }
        if let Some(v) = self.providers.bedrock.session_token_env {
            config.providers.bedrock.session_token_env = Some(v);
        }
        if let Some(v) = self.providers.bedrock.region {
            config.providers.bedrock.region = Some(v);
        }
        if let Some(v) = self.providers.bedrock.profile {
            config.providers.bedrock.profile = Some(v);
        }
        if let Some(v) = self.providers.bedrock.base_url {
            config.providers.bedrock.base_url = Some(v);
        }
        if let Some(p) = self.providers.bedrock.proxy
            && let Some(url) = p.url.filter(|s| !s.trim().is_empty())
        {
            config.providers.bedrock.proxy = Some(ProviderProxyConfig {
                url,
                no_proxy: p.no_proxy,
            });
        }
        if let Some(v) = self.providers.azure.api_key_env {
            config.providers.azure.api_key_env = v;
        }
        if let Some(v) = self.providers.azure.endpoint {
            config.providers.azure.endpoint = Some(v);
        }
        if let Some(v) = self.providers.azure.api_version {
            config.providers.azure.api_version = Some(v);
        }
        if let Some(v) = self.providers.azure.deployments {
            config.providers.azure.deployments = v;
        }
        if let Some(p) = self.providers.azure.proxy
            && let Some(url) = p.url.filter(|s| !s.trim().is_empty())
        {
            config.providers.azure.proxy = Some(ProviderProxyConfig {
                url,
                no_proxy: p.no_proxy,
            });
        }
        if let Some(v) = self.providers.vertex.access_token_env {
            config.providers.vertex.access_token_env = v;
        }
        if let Some(v) = self.providers.vertex.project {
            config.providers.vertex.project = Some(v);
        }
        if let Some(v) = self.providers.vertex.location {
            config.providers.vertex.location = Some(v);
        }
        if let Some(v) = self.providers.vertex.models {
            config.providers.vertex.models = v;
        }
        if let Some(v) = self.providers.vertex.base_url {
            config.providers.vertex.base_url = Some(v);
        }
        if let Some(p) = self.providers.vertex.proxy
            && let Some(url) = p.url.filter(|s| !s.trim().is_empty())
        {
            config.providers.vertex.proxy = Some(ProviderProxyConfig {
                url,
                no_proxy: p.no_proxy,
            });
        }
        if let Some(v) = self.providers.openai.api_key_env {
            config.providers.openai.api_key_env = v;
        }
        if let Some(v) = self.providers.openai.base_url {
            config.providers.openai.base_url = Some(v);
        }
        if let Some(p) = self.providers.openai.proxy
            && let Some(url) = p.url.filter(|s| !s.trim().is_empty())
        {
            config.providers.openai.proxy = Some(ProviderProxyConfig {
                url,
                no_proxy: p.no_proxy,
            });
        }
        if let Some(v) = self.providers.openrouter.api_key_env {
            config.providers.openrouter.api_key_env = v;
        }
        if let Some(v) = self.providers.openrouter.base_url {
            config.providers.openrouter.base_url = Some(v);
        }
        if let Some(v) = self.providers.openrouter.referer {
            config.providers.openrouter.referer = Some(v);
        }
        if let Some(p) = self.providers.openrouter.proxy
            && let Some(url) = p.url.filter(|s| !s.trim().is_empty())
        {
            config.providers.openrouter.proxy = Some(ProviderProxyConfig {
                url,
                no_proxy: p.no_proxy,
            });
        }
        if let Some(v) = self.providers.mistral.api_key_env {
            config.providers.mistral.api_key_env = v;
        }
        if let Some(v) = self.providers.mistral.base_url {
            config.providers.mistral.base_url = Some(v);
        }
        if let Some(p) = self.providers.mistral.proxy
            && let Some(url) = p.url.filter(|s| !s.trim().is_empty())
        {
            config.providers.mistral.proxy = Some(ProviderProxyConfig {
                url,
                no_proxy: p.no_proxy,
            });
        }
        if let Some(v) = self.providers.openai_responses.api_key_env {
            config.providers.openai_responses.api_key_env = v;
        }
        if let Some(v) = self.providers.openai_responses.base_url {
            config.providers.openai_responses.base_url = Some(v);
        }
        if let Some(p) = self.providers.openai_responses.proxy
            && let Some(url) = p.url.filter(|s| !s.trim().is_empty())
        {
            config.providers.openai_responses.proxy = Some(ProviderProxyConfig {
                url,
                no_proxy: p.no_proxy,
            });
        }
        if let Some(v) = self.providers.gemini.api_key_env {
            config.providers.gemini.api_key_env = v;
        }
        if let Some(v) = self.providers.gemini.base_url {
            config.providers.gemini.base_url = Some(v);
        }
        if let Some(p) = self.providers.gemini.proxy
            && let Some(url) = p.url.filter(|s| !s.trim().is_empty())
        {
            config.providers.gemini.proxy = Some(ProviderProxyConfig {
                url,
                no_proxy: p.no_proxy,
            });
        }
        for (id, profile) in self.providers.openai_compatible {
            let target = config
                .providers
                .openai_compatible
                .entry(id.clone())
                .or_insert_with(|| OpenAiCompatibleProviderConfig {
                    id: id.clone(),
                    ..Default::default()
                });
            target.id = id;
            if let Some(v) = profile.api_key_env {
                target.api_key_env = v;
            }
            if let Some(v) = profile.base_url {
                target.base_url = v;
            }
            if let Some(v) = profile.system_role_override {
                target.system_role_override = Some(v);
            }
            if let Some(v) = profile.max_tokens_field {
                target.max_tokens_field = Some(v);
            }
            if let Some(v) = profile.tool_result_name_field {
                target.tool_result_name_field = v;
            }
            if let Some(v) = profile.usage_in_stream {
                target.usage_in_stream = v;
            }
            if let Some(v) = profile.strict_tool_schema {
                target.strict_tool_schema = v;
            }
            if let Some(v) = profile.reasoning_effort {
                target.reasoning_effort = Some(v);
            }
            if let Some(v) = profile.cache_key {
                target.cache_key = Some(v);
            }
            if let Some(map) = profile.extra_headers {
                target.extra_headers = map.into_iter().collect();
            }
            if let Some(v) = profile.require_assistant_after_tool_result {
                target.require_assistant_after_tool_result = v;
            }
            if let Some(models) = profile.models {
                target.models = models
                    .into_iter()
                    .map(|model| {
                        let id = model.id.unwrap_or_default();
                        ConfiguredModelConfig {
                            display_name: model.display_name.unwrap_or_else(|| id.clone()),
                            id,
                            context_window: model.context_window.unwrap_or_default(),
                            max_output_tokens: model.max_output_tokens.unwrap_or_default(),
                            supports_images: model.supports_images.unwrap_or(false),
                            supports_streaming: model.supports_streaming.unwrap_or(true),
                            supports_thinking: model.supports_thinking.unwrap_or(false),
                            system_role_override: model.system_role_override,
                            max_tokens_field: model.max_tokens_field,
                        }
                    })
                    .collect();
            }
            if let Some(p) = profile.proxy
                && let Some(url) = p.url.filter(|s| !s.trim().is_empty())
            {
                target.proxy = Some(ProviderProxyConfig {
                    url,
                    no_proxy: p.no_proxy,
                });
            }
        }
        if let Some(v) = self.keybindings.submit {
            config.keybindings.submit = v;
        }
        if let Some(v) = self.keybindings.abort {
            config.keybindings.abort = v;
        }
        if let Some(v) = self.keybindings.new_line {
            config.keybindings.new_line = v;
        }
        if let Some(v) = self.retry.max_attempts {
            config.retry.max_attempts = v;
        }
        if let Some(v) = self.retry.initial_delay_ms {
            config.retry.initial_delay_ms = v;
        }
        if let Some(v) = self.retry.max_delay_ms {
            config.retry.max_delay_ms = v;
        }
        if let Some(v) = self.compaction.enabled {
            config.compaction.enabled = v;
        }
        if let Some(v) = self.compaction.threshold_tokens {
            config.compaction.threshold_tokens = v;
        }
        if let Some(paths) = self.extensions.paths {
            config.extensions.paths.extend(paths);
        }
        if let Some(paths) = self.packages.paths {
            config.packages.paths.extend(paths);
        }
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors from config loading and parsing.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to parse config file {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: Box<toml::de::Error>,
    },
    #[error("failed to read config file {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

/// Load and parse a TOML config file. Returns defaults if the file doesn't
/// exist. Returns a clear error for malformed TOML.
pub fn load_config_file(path: &Path) -> Result<OpiConfig, ConfigError> {
    if !path.exists() {
        return Ok(OpiConfig::default());
    }
    let contents = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    parse_toml(&contents, path)
}

fn parse_toml(contents: &str, path: &Path) -> Result<OpiConfig, ConfigError> {
    let raw: TomlConfig = toml::from_str(contents).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source: Box::new(source),
    })?;
    let mut config = OpiConfig::default();
    raw.merge_into(&mut config);
    Ok(config)
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

/// External configuration sources for precedence resolution.
pub struct ConfigSource {
    /// Model from CLI `--model` flag.
    pub cli_model: Option<String>,
    /// Explicit config path from CLI `--config` flag.
    pub config_path: Option<PathBuf>,
    /// Model from env var `OPI_MODEL`.
    pub env_model: Option<String>,
    /// Project root directory (for `.opi/config.toml`).
    pub project_dir: Option<PathBuf>,
    /// User config file path override (for testing). When `None`, uses
    /// the platform-default path from `user_config_path()`.
    pub user_config_path: Option<PathBuf>,
}

/// Resolve configuration from all sources with correct precedence:
/// CLI > env > project config > user config > built-in defaults.
pub fn resolve_config(source: ConfigSource) -> Result<OpiConfig, ConfigError> {
    let user_path = source.user_config_path.unwrap_or_else(user_config_path);
    let mut config = load_config_file(&user_path)?;

    if let Some(project_dir) = &source.project_dir {
        let project_config_path = project_dir.join(".opi").join("config.toml");
        let project_raw = load_raw_config(&project_config_path)?;
        project_raw.merge_into(&mut config);
    }

    // --config file overrides project and user config
    if let Some(config_path) = &source.config_path {
        if !config_path.exists() {
            return Err(ConfigError::Read {
                path: config_path.clone(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "config file not found"),
            });
        }
        let cli_raw = load_raw_config(config_path)?;
        cli_raw.merge_into(&mut config);
    }

    // Env model only applies when --config was NOT explicitly provided,
    // so that an explicit config file's model takes precedence over env.
    if source.config_path.is_none()
        && let Some(env_model) = &source.env_model
    {
        config.defaults.model = env_model.clone();
    }

    if let Some(cli_model) = &source.cli_model {
        config.defaults.model = cli_model.clone();
    }

    Ok(config)
}

fn load_raw_config(path: &Path) -> Result<TomlConfig, ConfigError> {
    if !path.exists() {
        return Ok(TomlConfig::default());
    }
    let contents = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str(&contents).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source: Box::new(source),
    })
}

/// Return the platform-specific user config file path.
pub fn user_config_path() -> PathBuf {
    user_config_dir().join("config.toml")
}

/// Return the platform-specific user config directory.
///
/// This is the directory where `config.toml` and global context files
/// (`AGENTS.md`, `CLAUDE.md`) live.
///
/// - Windows: `%APPDATA%\opi\`
/// - Unix: `~/.config/opi/`
pub fn user_config_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var("APPDATA")
            .map(|p| PathBuf::from(p).join("opi"))
            .unwrap_or_else(|_| PathBuf::from(".opi"))
    } else {
        dirs_home()
            .map(|h| h.join(".config").join("opi"))
            .unwrap_or_else(|| PathBuf::from(".opi"))
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

// ---------------------------------------------------------------------------
// HTTP client construction from proxy config
// ---------------------------------------------------------------------------

/// Build an HTTP client with optional proxy configuration.
///
/// When an explicit proxy config is provided, it is used directly.
/// Otherwise, falls back to environment variable detection
/// (`HTTPS_PROXY` / `HTTP_PROXY` / `NO_PROXY`).
pub fn build_http_client(
    proxy_config: Option<&ProviderProxyConfig>,
) -> Result<std::sync::Arc<opi_ai::http::HttpClient>, reqwest::Error> {
    let mut builder = opi_ai::http::HttpClientBuilder::new();
    if let Some(proxy) = proxy_config {
        builder = builder.proxy(opi_ai::http::ProxyConfig {
            url: Some(proxy.url.clone()),
            no_proxy: proxy.no_proxy.clone(),
        });
    } else {
        let env_proxy = opi_ai::http::proxy_from_env();
        if env_proxy.url.is_some() {
            builder = builder.proxy(env_proxy);
        }
    }
    builder.build().map(std::sync::Arc::new)
}
