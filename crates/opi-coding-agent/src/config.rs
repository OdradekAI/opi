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

use crate::project_trust::ProjectTrustDefault;

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
    pub execution: ExecutionConfig,
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
    /// Phase 14 opt-in: when `Some(Keychain)`, API-key built-in providers are
    /// described as [`opi_ai::AuthDescriptor::StoreCredential`] and doctor /
    /// `--list-models` probe the OS keychain (keychain-first, env fallback).
    /// Defaults to `None` (env), preserving pre-Phase-14 behavior.
    pub credential_backend: Option<CredentialBackendSource>,
    /// Phase 15.8.1 `[defaults] default_project_trust` policy (`ask` default).
    /// Read from the **pre-trust** (global) config so a project cannot
    /// self-authorize via its own `[defaults]` block.
    pub default_project_trust: ProjectTrustDefault,
}

/// Where an API-key built-in provider sources its credential.
///
/// `Env` (default): the configured environment variable. `Keychain`: the OS
/// keychain via the credential store, with env fallback on a headless host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CredentialBackendSource {
    /// Environment variable (pre-Phase-14 default).
    #[default]
    Env,
    /// OS keychain via the credential store.
    Keychain,
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
            credential_backend: None,
            default_project_trust: ProjectTrustDefault::Ask,
        }
    }
}

/// Run mode matched by deterministic execution rules (Phase 16). This is a
/// distinct three-variant enum from `policy::RunMode` (which has no `Rpc`
/// variant): the router and rule set reason about all three invocation kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionRunMode {
    Interactive,
    NonInteractive,
    Rpc,
}

impl std::fmt::Display for ExecutionRunMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Interactive => f.write_str("interactive"),
            Self::NonInteractive => f.write_str("non-interactive"),
            Self::Rpc => f.write_str("rpc"),
        }
    }
}

/// `command.execute` routing strategy (Phase 16). Default [`ExecutionStrategy::Fixed`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStrategy {
    #[default]
    Fixed,
    Rules,
    Model,
}

impl std::fmt::Display for ExecutionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fixed => f.write_str("fixed"),
            Self::Rules => f.write_str("rules"),
            Self::Model => f.write_str("model"),
        }
    }
}

/// One capability-permission decision for an adapter id (Phase 16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionDecision {
    /// Ineligible and model-invisible.
    Deny,
    /// Selectable but requires an interactive grant.
    Ask,
    /// Persistent user authorization.
    Allow,
}

/// One `[[execution.rules]]` entry. `modes: None` is the catch-all rule (matches
/// every mode); `modes: Some([])` is rejected at config time as malformed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct ExecutionRule {
    #[serde(default)]
    pub modes: Option<Vec<ExecutionRunMode>>,
    pub backend: String,
}

/// `[execution]` section (Phase 16). Layered TOML resolves `strategy`, `backend`,
/// and `rules` in precedence order; `permissions` is resolved from user-authorized
/// layers only (project `[execution.permissions]` is rejected even when trusted).
/// `rules` and `permissions` use REPLACE-if-present overlay semantics across
/// layers (intentionally NOT accumulating like `extensions.paths`).
/// `permissions` replaces the whole map when present; it does not merge per adapter id.
///
/// ```
/// use opi_coding_agent::config::{
///     ConfigSource, PermissionDecision, resolve_config,
/// };
///
/// let root = tempfile::tempdir().unwrap();
/// let user = root.path().join("user.toml");
/// let explicit = root.path().join("explicit.toml");
/// std::fs::write(&user, "[execution.permissions]\nlocal = \"deny\"\n").unwrap();
/// std::fs::write(
///     &explicit,
///     "[execution.permissions]\n\"opi-sandbox\" = \"allow\"\n",
/// )
/// .unwrap();
///
/// let config = resolve_config(ConfigSource {
///     cli_model: None,
///     config_path: Some(explicit),
///     env_model: None,
///     project_dir: None,
///     user_config_path: Some(user),
/// })
/// .unwrap();
///
/// assert_eq!(
///     config.execution.permissions.get("opi-sandbox"),
///     Some(&PermissionDecision::Allow),
/// );
/// assert!(!config.execution.permissions.contains_key("local"));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionConfig {
    pub strategy: ExecutionStrategy,
    pub backend: String,
    pub rules: Vec<ExecutionRule>,
    pub permissions: BTreeMap<String, PermissionDecision>,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        // The spec default is `[execution] strategy = "fixed", backend = "local"`.
        Self {
            strategy: ExecutionStrategy::Fixed,
            backend: "local".to_string(),
            rules: Vec::new(),
            permissions: BTreeMap::new(),
        }
    }
}

impl OpiConfig {
    /// Apply `--execution-backend` / `--execution-strategy` CLI overrides on top
    /// of the layered TOML configuration.
    ///
    /// `--execution-backend <id>` selects the `fixed` strategy with that backend
    /// (per the design: it "selects the fixed strategy"). `--execution-strategy`
    /// sets the strategy. **Both touch only `strategy`/`backend` — they never
    /// grant trust or permission** (the `permissions` map and trust records are
    /// byte-identical before and after). If both are supplied, `--execution-backend`
    /// sets the backend and `fixed`, then `--execution-strategy` may override the
    /// strategy while keeping the backend. This is the deterministic resolution
    /// hook the runtime (16.7) calls after `resolve_config`; it is direct-testable.
    pub fn apply_execution_overrides(
        &mut self,
        backend: Option<&str>,
        strategy: Option<ExecutionStrategy>,
    ) {
        if let Some(backend) = backend {
            self.execution.backend = backend.to_string();
            self.execution.strategy = ExecutionStrategy::Fixed;
        }
        if let Some(strategy) = strategy {
            self.execution.strategy = strategy;
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
    pub custom: BTreeMap<String, CustomProviderConfig>,
}

/// Fully validated `[providers.custom.<id>]` mapped provider.
///
/// One provider shares one credential environment source and auth scheme
/// across all routes. Provider API/base URL values are defaults; model values
/// win. Config loading accepts only Anthropic Messages, OpenAI Completions,
/// and OpenAI Responses custom wires, validates wire-tagged compatibility,
/// thinking maps, pricing tiers, and rejects provider-managed auth headers.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomProviderConfig {
    pub id: String,
    pub name: String,
    pub base_url: Option<String>,
    pub api_key_env: String,
    pub auth_scheme: opi_ai::AuthScheme,
    pub proxy: Option<ProviderProxyConfig>,
    pub headers: Vec<(String, String)>,
    pub models: Vec<opi_ai::ModelInfo>,
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
    /// Legacy compatibility metadata for reasoning profiles. Request thinking
    /// and the selected model's thinking map are authoritative for wire output.
    pub reasoning_effort: Option<String>,
    /// Emit `prompt_cache_key` for OpenAI prompt-cache affinity (Phase 12.3).
    pub cache_key: Option<String>,
    /// Extra request headers (session-affinity) applied to every request.
    pub extra_headers: Vec<(String, String)>,
    /// Compatibility-metadata flag for legacy assistant-after-tool-result wires.
    pub require_assistant_after_tool_result: bool,
    /// Optional chat completions endpoint path relative to base_url.
    pub chat_completions_path: Option<String>,
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
    sandbox: Option<TomlSandboxPresence>,
    execution: TomlExecution,
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
    credential_backend: Option<CredentialBackendSource>,
    default_project_trust: Option<ProjectTrustDefault>,
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
    custom: BTreeMap<String, TomlCustomProvider>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlCustomProvider {
    name: Option<String>,
    base_url: Option<String>,
    api_key_env: Option<String>,
    auth_scheme: Option<String>,
    api: Option<String>,
    headers: Option<BTreeMap<String, String>>,
    models: Option<Vec<TomlCustomModel>>,
    proxy: Option<TomlProxy>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlCustomModel {
    id: Option<String>,
    display_name: Option<String>,
    api: Option<String>,
    base_url: Option<String>,
    context_window: Option<u64>,
    max_output_tokens: Option<u64>,
    supports_images: Option<bool>,
    supports_streaming: Option<bool>,
    supports_thinking: Option<bool>,
    supports_cache_control: Option<bool>,
    supports_long_cache_retention: Option<bool>,
    thinking_level_map: Option<BTreeMap<String, toml::Value>>,
    compat: Option<TomlCustomCompat>,
    pricing: Option<TomlPricing>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlCustomCompat {
    api: Option<String>,
    supports_eager_tool_input_streaming: Option<bool>,
    force_adaptive_thinking: Option<bool>,
    supports_temperature: Option<bool>,
    system_role_override: Option<String>,
    max_tokens_field: Option<String>,
    tool_result_name_field: Option<bool>,
    usage_in_stream: Option<bool>,
    strict_tool_schema: Option<bool>,
    reasoning_effort: Option<String>,
    cache_key: Option<String>,
    send_session_affinity_headers: Option<bool>,
    require_assistant_after_tool_result: Option<bool>,
    chat_completions_path: Option<String>,
    supports_store: Option<bool>,
    supports_developer_role: Option<bool>,
    supports_reasoning_effort: Option<bool>,
    store: Option<bool>,
    strict_tools: Option<bool>,
    responses_path: Option<String>,
    send_session_id_header: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlPricing {
    input: Option<f64>,
    output: Option<f64>,
    cache_read: Option<f64>,
    cache_write: Option<f64>,
    tiers: Option<Vec<TomlPricingTier>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlPricingTier {
    input_tokens_above: Option<u64>,
    input: Option<f64>,
    output: Option<f64>,
    cache_read: Option<f64>,
    cache_write: Option<f64>,
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
    chat_completions_path: Option<String>,
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

/// Presence marker for the REMOVED `[sandbox]` table. Its field-local
/// deserializer checks the closed legacy shape without leaking that shape's
/// parse errors: every present table is classified by
/// `legacy_sandbox_rejection` as the same removed surface. This keeps unrelated
/// TOML deserialization errors on their ordinary paths and avoids parsing the
/// whole document twice.
#[derive(Debug, Clone)]
struct TomlSandboxPresence;

impl<'de> Deserialize<'de> for TomlSandboxPresence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = toml::Value::deserialize(deserializer)?;
        if !value.is_table() {
            return TomlSandbox::deserialize(value)
                .map(|_| Self)
                .map_err(serde::de::Error::custom);
        }
        // The shape stays deliberately closed, but both known and unknown
        // legacy table forms are one removed public surface with one remediation.
        // Naming the result makes that intentional collapse explicit: validation
        // still runs, and only its outward error classification is unified.
        let _closed_legacy_shape_result = TomlSandbox::deserialize(value);
        Ok(Self)
    }
}

/// Closed legacy shape used only by [`TomlSandboxPresence`].
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct TomlSandbox {
    mode: Option<String>,
    require: Option<bool>,
    fs: Option<bool>,
    network: Option<bool>,
    syscalls: Option<bool>,
}

/// Reject a layer that still carries the removed `[sandbox]` section with a
/// stable, actionable remediation pointing at the execution-backend surface
/// and the package workflow. Called at every layer-load site because
/// `merge_into` is layer-blind and cannot enforce the rejection itself.
fn legacy_sandbox_rejection(raw: &TomlConfig) -> Result<(), ConfigError> {
    if raw.sandbox.is_some() {
        return Err(ConfigError::LegacySandboxSection);
    }
    Ok(())
}

/// Shadow TOML for `[execution]`. `permissions` is present-marker for the
/// project-layer rejection check; it is only ever merged from user/explicit
/// layers (a project `[execution.permissions]` is rejected upstream).
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TomlExecution {
    strategy: Option<ExecutionStrategy>,
    backend: Option<String>,
    rules: Option<Vec<ExecutionRule>>,
    permissions: Option<BTreeMap<String, PermissionDecision>>,
}

impl TomlConfig {
    fn merge_into(
        mut self,
        config: &mut OpiConfig,
        custom: &mut BTreeMap<String, TomlCustomProvider>,
    ) {
        let custom_layer = std::mem::take(&mut self.providers.custom);
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
        if let Some(v) = self.defaults.credential_backend {
            config.defaults.credential_backend = Some(v);
        }
        if let Some(v) = self.defaults.default_project_trust {
            config.defaults.default_project_trust = v;
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
            if let Some(v) = profile.chat_completions_path {
                target.chat_completions_path = Some(v);
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
        merge_custom_layer(custom, custom_layer);
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
        // `[execution]`: strategy/backend/rules use REPLACE-if-present overlay
        // (NOT accumulating like extensions.paths/packages.paths). `permissions`
        // is replace-the-whole-map too; it must arrive only from user/explicit
        // layers — project `[execution.permissions]` is rejected upstream by
        // `reject_project_execution_permissions` before any merge_into call.
        if let Some(v) = self.execution.strategy {
            config.execution.strategy = v;
        }
        if let Some(v) = self.execution.backend {
            config.execution.backend = v;
        }
        if let Some(v) = self.execution.rules {
            config.execution.rules = v;
        }
        if let Some(v) = self.execution.permissions {
            config.execution.permissions = v;
        }
    }
}

fn merge_custom_layer(
    target: &mut BTreeMap<String, TomlCustomProvider>,
    layer: BTreeMap<String, TomlCustomProvider>,
) {
    for (id, profile) in layer {
        let current = target.entry(id).or_default();
        if profile.name.is_some() {
            current.name = profile.name;
        }
        if profile.base_url.is_some() {
            current.base_url = profile.base_url;
        }
        if profile.api_key_env.is_some() {
            current.api_key_env = profile.api_key_env;
        }
        if profile.auth_scheme.is_some() {
            current.auth_scheme = profile.auth_scheme;
        }
        if profile.api.is_some() {
            current.api = profile.api;
        }
        if profile.headers.is_some() {
            current.headers = profile.headers;
        }
        if profile.models.is_some() {
            current.models = profile.models;
        }
        if profile.proxy.is_some() {
            current.proxy = profile.proxy;
        }
    }
}

fn validate_custom_providers(
    raw: BTreeMap<String, TomlCustomProvider>,
) -> Result<BTreeMap<String, CustomProviderConfig>, ConfigError> {
    raw.into_iter()
        .map(|(id, provider)| {
            let validated = validate_custom_provider(&id, provider)?;
            Ok((id, validated))
        })
        .collect()
}

/// Validate the final merged provider-id namespace used by both listing and
/// runtime model-spec dispatch.
pub fn validate_provider_namespace(config: &OpiConfig) -> Result<(), ConfigError> {
    const DEPRECATED_PROVIDER_IDS: &[&str] = &["copilot", "codex"];

    let mut configured = BTreeMap::<String, &'static str>::new();
    let validate = |id: &str,
                    source: &'static str,
                    configured: &mut BTreeMap<String, &'static str>|
     -> Result<(), ConfigError> {
        if id.trim().is_empty() {
            return Err(ConfigError::InvalidProviderNamespace {
                provider: id.to_owned(),
                message: "provider id must not be empty".into(),
            });
        }
        if id.contains(':') {
            return Err(ConfigError::InvalidProviderNamespace {
                provider: id.to_owned(),
                message: "provider id must not contain ':'".into(),
            });
        }
        if crate::provider_factory::built_in_provider_ids().contains(&id)
            || DEPRECATED_PROVIDER_IDS.contains(&id)
        {
            return Err(ConfigError::InvalidProviderNamespace {
                provider: id.to_owned(),
                message: "provider id is reserved".into(),
            });
        }
        if let Some(previous) = configured.insert(id.to_owned(), source) {
            return Err(ConfigError::InvalidProviderNamespace {
                provider: id.to_owned(),
                message: format!("provider id is configured in both {previous} and {source}"),
            });
        }
        Ok(())
    };

    for id in config.providers.openai_compatible.keys() {
        validate(id, "providers.openai_compatible", &mut configured)?;
    }
    for id in config.providers.custom.keys() {
        validate(id, "providers.custom", &mut configured)?;
    }
    Ok(())
}

/// Validate the `[execution]` section structure. Rule-mode charset (only
/// `interactive`/`non-interactive`/`rpc`) is enforced by the `ExecutionRunMode`
/// serde enum; this checks catch-all presence/position and the strategy=rules
/// requirement. Called at every site that produces a final `OpiConfig`.
pub fn validate_execution_config(config: &OpiConfig) -> Result<(), ConfigError> {
    validate_rules(&config.execution)
}

fn validate_rules(exec: &ExecutionConfig) -> Result<(), ConfigError> {
    let mut catch_all_count = 0usize;
    let mut seen_catch_all = false;
    for rule in &exec.rules {
        match &rule.modes {
            // `modes = []` (explicit empty) is malformed: a catch-all must OMIT
            // the key, not pass an empty array.
            Some(modes) if modes.is_empty() => {
                return Err(invalid_exec(
                    "rules.modes",
                    "empty modes array; omit `modes` for a catch-all rule",
                ));
            }
            Some(_) => {
                if seen_catch_all {
                    return Err(invalid_exec(
                        "rules",
                        "a catch-all rule (no `modes`) must be last",
                    ));
                }
            }
            None => {
                seen_catch_all = true;
                catch_all_count += 1;
                if catch_all_count > 1 {
                    return Err(invalid_exec(
                        "rules",
                        "at most one catch-all rule (no `modes`) is allowed",
                    ));
                }
            }
        }
    }
    if exec.strategy == ExecutionStrategy::Rules {
        if exec.rules.is_empty() {
            return Err(invalid_exec(
                "strategy",
                "rules strategy requires at least one rule with a final catch-all",
            ));
        }
        if catch_all_count != 1 {
            return Err(invalid_exec(
                "rules",
                "rules strategy requires exactly one final catch-all rule (no `modes`)",
            ));
        }
    }
    Ok(())
}

/// Reject a project layer that sets `[execution.permissions]`. Persistent
/// capability permission is user-owned: a project may request a strategy or
/// backend (after the existing project-trust gate) but cannot create persistent
/// `allow`. Called at BOTH project-merge sites (`finalize_with_project` and
/// `merge_project_config`) BEFORE `merge_into`, because `merge_into` is
/// layer-blind and cannot enforce this itself.
fn reject_project_execution_permissions(project_raw: &TomlConfig) -> Result<(), ConfigError> {
    if project_raw.execution.permissions.is_some() {
        return Err(invalid_exec(
            "permissions",
            "project layer may not set [execution.permissions]; persistent \
             permission is user-owned",
        ));
    }
    Ok(())
}

fn invalid_exec(field: &'static str, message: &str) -> ConfigError {
    ConfigError::InvalidExecutionConfig {
        field: field.to_string(),
        message: message.to_string(),
    }
}

fn validate_custom_provider(
    id: &str,
    raw: TomlCustomProvider,
) -> Result<CustomProviderConfig, ConfigError> {
    if id.trim().is_empty() {
        return Err(invalid_custom(id, None, "id", "must not be empty"));
    }
    let name = raw.name.unwrap_or_else(|| id.to_owned());
    if name.trim().is_empty() {
        return Err(invalid_custom(id, None, "name", "must not be empty"));
    }
    let api_key_env = raw
        .api_key_env
        .unwrap_or_else(|| format!("{}_API_KEY", id.replace('-', "_").to_ascii_uppercase()));
    if api_key_env.trim().is_empty() {
        return Err(invalid_custom(id, None, "api_key_env", "must not be empty"));
    }
    let auth_scheme = match raw.auth_scheme.as_deref().unwrap_or("bearer") {
        "bearer" => opi_ai::AuthScheme::Bearer,
        "api-key" | "api_key" => opi_ai::AuthScheme::ApiKey,
        _ => {
            return Err(invalid_custom(
                id,
                None,
                "auth_scheme",
                "must be 'bearer' or 'api-key'",
            ));
        }
    };
    let models = raw
        .models
        .ok_or_else(|| invalid_custom(id, None, "models", "at least one model is required"))?;
    if models.is_empty() {
        return Err(invalid_custom(
            id,
            None,
            "models",
            "at least one model is required",
        ));
    }

    let mut seen = std::collections::BTreeSet::new();
    let mut validated_models = Vec::with_capacity(models.len());
    for raw_model in models {
        let model_id = raw_model.id.clone().unwrap_or_default();
        if model_id.trim().is_empty() {
            return Err(invalid_custom(id, None, "models.id", "must not be empty"));
        }
        if !seen.insert(model_id.clone()) {
            return Err(invalid_custom(
                id,
                Some(&model_id),
                "id",
                "duplicate model id",
            ));
        }
        let wire = custom_wire(
            id,
            &model_id,
            raw_model.api.as_deref().or(raw.api.as_deref()),
        )?;
        let context_window = raw_model.context_window.unwrap_or_default();
        let max_output_tokens = raw_model.max_output_tokens.unwrap_or_default();
        if context_window == 0 {
            return Err(invalid_custom(
                id,
                Some(&model_id),
                "context_window",
                "must be greater than zero",
            ));
        }
        if max_output_tokens == 0 {
            return Err(invalid_custom(
                id,
                Some(&model_id),
                "max_output_tokens",
                "must be greater than zero",
            ));
        }
        let supports_thinking = raw_model.supports_thinking.unwrap_or(false);
        let capabilities = opi_ai::ModelCapabilities::new(context_window, max_output_tokens)
            .with_images(raw_model.supports_images.unwrap_or(false))
            .with_streaming(raw_model.supports_streaming.unwrap_or(true))
            .with_thinking(supports_thinking)
            .with_cache_control(raw_model.supports_cache_control.unwrap_or(false))
            .with_long_cache_retention(raw_model.supports_long_cache_retention.unwrap_or(false));
        let mut model = opi_ai::ModelInfo::new(
            &model_id,
            raw_model
                .display_name
                .clone()
                .unwrap_or_else(|| model_id.clone()),
            wire,
            capabilities,
        );
        model.base_url = raw_model.base_url;
        model.thinking_level_map = custom_thinking_map(
            id,
            &model_id,
            supports_thinking,
            raw_model.thinking_level_map,
        )?;
        model.compat = custom_compat(id, &model_id, wire, raw_model.compat)?;
        if let Some(pricing) = raw_model.pricing {
            model.pricing = Some(custom_pricing(id, &model_id, pricing)?);
        }
        model
            .validate()
            .map_err(|error| invalid_custom(id, Some(&model_id), "model", &error.to_string()))?;
        validated_models.push(model);
    }

    if auth_scheme == opi_ai::AuthScheme::ApiKey
        && validated_models
            .iter()
            .any(|model| model.wire_api != opi_ai::WireApi::AnthropicMessages)
    {
        return Err(invalid_custom(
            id,
            None,
            "auth_scheme",
            "api-key auth is only valid for an all-Anthropic route set",
        ));
    }
    if raw.base_url.as_deref().is_none_or(str::is_empty)
        && validated_models
            .iter()
            .any(|model| model.base_url.as_deref().is_none_or(str::is_empty))
    {
        return Err(invalid_custom(
            id,
            None,
            "base_url",
            "a provider default or model override is required for every route",
        ));
    }

    let headers: Vec<(String, String)> = raw.headers.unwrap_or_default().into_iter().collect();
    opi_ai::ProviderHeaders::try_new(headers.clone())
        .map_err(|error| invalid_custom(id, None, "headers", &error.to_string()))?;
    let proxy = match raw.proxy {
        Some(proxy) => {
            let url = proxy
                .url
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| invalid_custom(id, None, "proxy.url", "must not be empty"))?;
            Some(ProviderProxyConfig {
                url,
                no_proxy: proxy.no_proxy,
            })
        }
        None => None,
    };

    Ok(CustomProviderConfig {
        id: id.into(),
        name,
        base_url: raw.base_url,
        api_key_env,
        auth_scheme,
        proxy,
        headers,
        models: validated_models,
    })
}

fn custom_wire(
    provider_id: &str,
    model_id: &str,
    value: Option<&str>,
) -> Result<opi_ai::WireApi, ConfigError> {
    let value = value.ok_or_else(|| {
        invalid_custom(
            provider_id,
            Some(model_id),
            "api",
            "provider or model API is required",
        )
    })?;
    match value {
        "anthropic-messages" => Ok(opi_ai::WireApi::AnthropicMessages),
        "openai-completions" => Ok(opi_ai::WireApi::OpenAiCompletions),
        "openai-responses" => Ok(opi_ai::WireApi::OpenAiResponses),
        "openai-codex-responses" => Err(invalid_custom(
            provider_id,
            Some(model_id),
            "api",
            "openai-codex-responses is built-in-only",
        )),
        _ => Err(invalid_custom(
            provider_id,
            Some(model_id),
            "api",
            "unsupported custom provider wire",
        )),
    }
}

fn custom_thinking_map(
    provider_id: &str,
    model_id: &str,
    supports_thinking: bool,
    raw: Option<BTreeMap<String, toml::Value>>,
) -> Result<opi_ai::ThinkingLevelMap, ConfigError> {
    let mut map = if supports_thinking {
        opi_ai::ThinkingLevelMap::reasoning_default()
    } else {
        opi_ai::ThinkingLevelMap::disabled()
    };
    for (level, value) in raw.unwrap_or_default() {
        let level = level.parse::<opi_ai::ThinkingLevel>().map_err(|_| {
            invalid_custom(
                provider_id,
                Some(model_id),
                "thinking_level_map",
                "contains an unknown thinking level",
            )
        })?;
        let mapping = match value {
            toml::Value::Boolean(true) => opi_ai::ThinkingLevelMapping::Identity,
            toml::Value::Boolean(false) => opi_ai::ThinkingLevelMapping::Unsupported,
            toml::Value::String(value) if !value.trim().is_empty() => {
                opi_ai::ThinkingLevelMapping::Mapped(value)
            }
            _ => {
                return Err(invalid_custom(
                    provider_id,
                    Some(model_id),
                    "thinking_level_map",
                    "values must be true, false, or non-empty strings",
                ));
            }
        };
        map = map.with_mapping(level, mapping);
    }
    Ok(map)
}

fn custom_compat(
    provider_id: &str,
    model_id: &str,
    wire: opi_ai::WireApi,
    raw: Option<TomlCustomCompat>,
) -> Result<opi_ai::WireCompat, ConfigError> {
    let raw = raw.unwrap_or_default();
    if let Some(api) = raw.api.as_deref()
        && custom_wire(provider_id, model_id, Some(api))? != wire
    {
        return Err(invalid_custom(
            provider_id,
            Some(model_id),
            "compat.api",
            "must match the model API",
        ));
    }
    reject_wrong_compat_fields(provider_id, model_id, wire, &raw)?;
    Ok(match wire {
        opi_ai::WireApi::AnthropicMessages => {
            opi_ai::WireCompat::AnthropicMessages(opi_ai::model_info::AnthropicMessagesCompat {
                supports_eager_tool_input_streaming: raw
                    .supports_eager_tool_input_streaming
                    .unwrap_or_default(),
                force_adaptive_thinking: raw.force_adaptive_thinking.unwrap_or_default(),
                supports_temperature: raw.supports_temperature.unwrap_or_default(),
            })
        }
        opi_ai::WireApi::OpenAiCompletions => {
            let mut compat = opi_ai::model_info::OpenAiCompletionsCompat {
                system_role_override: raw.system_role_override,
                tool_result_name_field: raw.tool_result_name_field.unwrap_or_default(),
                usage_in_stream: raw.usage_in_stream.unwrap_or_default(),
                strict_tool_schema: raw.strict_tool_schema.unwrap_or_default(),
                reasoning_effort: raw.reasoning_effort,
                cache_key: raw.cache_key,
                send_session_affinity_headers: raw
                    .send_session_affinity_headers
                    .unwrap_or_default(),
                require_assistant_after_tool_result: raw
                    .require_assistant_after_tool_result
                    .unwrap_or_default(),
                supports_store: raw.supports_store.unwrap_or_default(),
                supports_developer_role: raw.supports_developer_role.unwrap_or_default(),
                supports_reasoning_effort: raw.supports_reasoning_effort.unwrap_or_default(),
                ..Default::default()
            };
            if let Some(value) = raw.max_tokens_field {
                compat.max_tokens_field = value;
            }
            if let Some(value) = raw.chat_completions_path {
                compat.chat_completions_path = value;
            }
            opi_ai::WireCompat::OpenAiCompletions(compat)
        }
        opi_ai::WireApi::OpenAiResponses => {
            let mut compat = opi_ai::model_info::OpenAiResponsesCompat {
                store: raw.store,
                reasoning_effort: raw.reasoning_effort,
                strict_tools: raw.strict_tools.unwrap_or_default(),
                send_session_id_header: raw.send_session_id_header.unwrap_or(false),
                ..Default::default()
            };
            if let Some(value) = raw.responses_path {
                compat.responses_path = value;
            }
            opi_ai::WireCompat::OpenAiResponses(compat)
        }
        _ => unreachable!("custom_wire restricts the wire set"),
    })
}

fn reject_wrong_compat_fields(
    provider_id: &str,
    model_id: &str,
    wire: opi_ai::WireApi,
    raw: &TomlCustomCompat,
) -> Result<(), ConfigError> {
    macro_rules! reject_fields {
        ($($field:ident),+ $(,)?) => {{
            $(
                if raw.$field.is_some() {
                    return Err(invalid_custom(
                        provider_id,
                        Some(model_id),
                        concat!("compat.", stringify!($field)),
                        "is not valid for the model API",
                    ));
                }
            )+
        }};
    }

    match wire {
        opi_ai::WireApi::AnthropicMessages => reject_fields!(
            system_role_override,
            max_tokens_field,
            tool_result_name_field,
            usage_in_stream,
            strict_tool_schema,
            reasoning_effort,
            cache_key,
            send_session_affinity_headers,
            require_assistant_after_tool_result,
            chat_completions_path,
            supports_store,
            supports_developer_role,
            supports_reasoning_effort,
            store,
            strict_tools,
            responses_path,
            send_session_id_header,
        ),
        opi_ai::WireApi::OpenAiCompletions => reject_fields!(
            supports_eager_tool_input_streaming,
            force_adaptive_thinking,
            supports_temperature,
            store,
            strict_tools,
            responses_path,
            send_session_id_header,
        ),
        opi_ai::WireApi::OpenAiResponses => reject_fields!(
            supports_eager_tool_input_streaming,
            force_adaptive_thinking,
            supports_temperature,
            system_role_override,
            max_tokens_field,
            tool_result_name_field,
            usage_in_stream,
            strict_tool_schema,
            cache_key,
            send_session_affinity_headers,
            require_assistant_after_tool_result,
            chat_completions_path,
            supports_store,
            supports_developer_role,
            supports_reasoning_effort,
        ),
        _ => unreachable!("custom_wire restricts the wire set"),
    }
    Ok(())
}

fn custom_pricing(
    provider_id: &str,
    model_id: &str,
    raw: TomlPricing,
) -> Result<opi_ai::ModelPricing, ConfigError> {
    fn pricing(
        input: Option<f64>,
        output: Option<f64>,
        cache_read: Option<f64>,
        cache_write: Option<f64>,
    ) -> opi_ai::stream::Pricing {
        opi_ai::stream::Pricing {
            input_cost_per_mtok: input.unwrap_or_default(),
            output_cost_per_mtok: output.unwrap_or_default(),
            cache_read_cost_per_mtok: cache_read.unwrap_or_default(),
            cache_write_cost_per_mtok: cache_write.unwrap_or_default(),
        }
    }
    let base = pricing(raw.input, raw.output, raw.cache_read, raw.cache_write);
    let tiers = raw
        .tiers
        .unwrap_or_default()
        .into_iter()
        .map(|tier| opi_ai::PricingTier {
            input_tokens_above: tier.input_tokens_above.unwrap_or_default(),
            pricing: pricing(tier.input, tier.output, tier.cache_read, tier.cache_write),
        })
        .collect();
    opi_ai::ModelPricing::try_new(base, tiers)
        .map_err(|error| invalid_custom(provider_id, Some(model_id), "pricing", &error.to_string()))
}

fn invalid_custom(
    provider: &str,
    model: Option<&str>,
    field: &'static str,
    message: &str,
) -> ConfigError {
    ConfigError::InvalidCustomProvider {
        provider: provider.into(),
        model: model.map(str::to_owned),
        field,
        message: message.into(),
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
    #[error(
        "invalid custom provider '{provider}'{model_suffix} field '{field}': {message}",
        model_suffix = model.as_ref().map(|model| format!(" model '{model}'")).unwrap_or_default()
    )]
    InvalidCustomProvider {
        provider: String,
        model: Option<String>,
        field: &'static str,
        message: String,
    },
    #[error("invalid provider id '{provider}': {message}")]
    InvalidProviderNamespace { provider: String, message: String },
    #[error("invalid execution config field '{field}': {message}")]
    InvalidExecutionConfig { field: String, message: String },
    #[error("{remediation}", remediation = crate::diagnostics::LEGACY_SANDBOX_REMEDIATION)]
    LegacySandboxSection,
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
    legacy_sandbox_rejection(&raw)?;
    let mut config = OpiConfig::default();
    let mut custom = BTreeMap::new();
    raw.merge_into(&mut config, &mut custom);
    config.providers.custom = validate_custom_providers(custom)?;
    validate_provider_namespace(&config)?;
    validate_execution_config(&config)?;
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

/// Raw global and explicit configuration layers staged before project trust is
/// resolved.
///
/// Staging parses only user-authorized inputs. The project layer is not read
/// until [`Self::finalize_with_project`] is called with `true`, and all raw
/// layers are then merged and validated exactly once in normal precedence
/// order.
#[derive(Debug, Clone)]
pub struct StagedConfig {
    user: TomlConfig,
    explicit: Option<TomlConfig>,
    cli_model: Option<String>,
    env_model: Option<String>,
    project_dir: Option<PathBuf>,
}

impl StagedConfig {
    /// Return the project-trust default from global/user-authorized layers
    /// only. A project cannot influence this value through `.opi/config.toml`.
    pub fn global_default_project_trust(&self) -> ProjectTrustDefault {
        self.explicit
            .as_ref()
            .and_then(|config| config.defaults.default_project_trust)
            .or(self.user.defaults.default_project_trust)
            .unwrap_or(ProjectTrustDefault::Ask)
    }

    /// Finalize the staged layers, optionally reading and merging the project
    /// layer between user config and the explicit `--config` layer.
    pub fn finalize_with_project(self, include_project: bool) -> Result<OpiConfig, ConfigError> {
        let mut config = OpiConfig::default();
        let mut custom = BTreeMap::new();
        legacy_sandbox_rejection(&self.user)?;
        self.user.merge_into(&mut config, &mut custom);

        if include_project && let Some(project_dir) = &self.project_dir {
            let project_config_path = project_dir.join(".opi").join("config.toml");
            let project_raw = load_raw_config(&project_config_path)?;
            // Persistent capability permission is user-owned: reject a project
            // `[execution.permissions]` BEFORE merging (merge_into is layer-blind).
            reject_project_execution_permissions(&project_raw)?;
            legacy_sandbox_rejection(&project_raw)?;
            project_raw.merge_into(&mut config, &mut custom);
        }

        let has_explicit = self.explicit.is_some();
        if let Some(explicit) = self.explicit {
            legacy_sandbox_rejection(&explicit)?;
            explicit.merge_into(&mut config, &mut custom);
        }

        // Env model only applies when --config was NOT explicitly provided,
        // so that an explicit config file's model takes precedence over env.
        if !has_explicit && let Some(env_model) = self.env_model {
            config.defaults.model = env_model;
        }
        if let Some(cli_model) = self.cli_model {
            config.defaults.model = cli_model;
        }

        config.providers.custom = validate_custom_providers(custom)?;
        validate_provider_namespace(&config)?;
        validate_execution_config(&config)?;
        Ok(config)
    }
}

/// Parse the global and explicit config layers without reading project config
/// or validating the combined provider namespace.
pub fn stage_config(source: ConfigSource) -> Result<StagedConfig, ConfigError> {
    let user_path = source.user_config_path.unwrap_or_else(user_config_path);
    let user = load_raw_config(&user_path)?;
    let explicit = match source.config_path {
        Some(config_path) => {
            if !config_path.exists() {
                return Err(ConfigError::Read {
                    path: config_path,
                    source: std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "config file not found",
                    ),
                });
            }
            Some(load_raw_config(&config_path)?)
        }
        None => None,
    };
    Ok(StagedConfig {
        user,
        explicit,
        cli_model: source.cli_model,
        env_model: source.env_model,
        project_dir: source.project_dir,
    })
}

/// Resolve configuration from all sources with correct precedence:
/// CLI > env > project config > user config > built-in defaults.
pub fn resolve_config(source: ConfigSource) -> Result<OpiConfig, ConfigError> {
    stage_config(source)?.finalize_with_project(true)
}

/// Stage 1 of two-stage trust-gated resolution (task 15.7): resolve every layer
/// except the project `.opi/config.toml`. This is the config used to obtain
/// trust inputs and the config applied when the project is untrusted.
pub fn resolve_pre_trust_config(source: ConfigSource) -> Result<OpiConfig, ConfigError> {
    stage_config(source)?.finalize_with_project(false)
}

/// Stage 2: merge the project `.opi/config.toml` layer onto a pre-trust config
/// for a trusted project. Regular fields, `[providers.openai_compatible]`, and
/// `[providers.custom]` entries are merged and the combined provider namespace
/// is re-validated. Equivalent in result to a full `resolve_config` that
/// includes the project layer, but expressed as an incremental stage so the
/// trust boundary is explicit at the call site.
pub fn merge_project_config(
    mut config: OpiConfig,
    project_dir: &Path,
) -> Result<OpiConfig, ConfigError> {
    let project_config_path = project_dir.join(".opi").join("config.toml");
    let project_raw = load_raw_config(&project_config_path)?;
    // Persistent capability permission is user-owned: reject a project
    // `[execution.permissions]` BEFORE merging (merge_into is layer-blind).
    reject_project_execution_permissions(&project_raw)?;
    legacy_sandbox_rejection(&project_raw)?;
    let mut project_custom = BTreeMap::new();
    project_raw.merge_into(&mut config, &mut project_custom);
    for (id, provider) in validate_custom_providers(project_custom)? {
        config.providers.custom.insert(id, provider);
    }
    validate_provider_namespace(&config)?;
    validate_execution_config(&config)?;
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
