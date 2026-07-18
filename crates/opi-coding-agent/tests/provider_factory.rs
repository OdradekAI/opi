//! Tests for provider factory construction across all 6 providers.
//!
//! Each test constructs a provider with a dummy API key and verifies the
//! provider reports the correct ID. Config integration tests verify that
//! TOML-deserialized provider configs resolve to the right env var names.

use std::sync::{Arc, Mutex};

use opi_ai::provider::{CacheRetention, Provider, Request, ThinkingConfig};
use opi_ai::test_support::MockProvider;
use opi_coding_agent::config::{
    GenericProviderConfig, OpenRouterProviderConfig, OpiConfig, load_config_file,
};
use tokio_util::sync::CancellationToken;

static ENV_MUTEX: Mutex<()> = Mutex::new(());
struct OrderingKeyringBackend {
    inner: opi_coding_agent::credential_store::FakeKeyringBackend,
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl OrderingKeyringBackend {
    fn inner(&self) -> &opi_coding_agent::credential_store::FakeKeyringBackend {
        &self.inner
    }

    fn record_entry_creation(&self) {
        self.events
            .lock()
            .expect("ordering events")
            .push("entry_creation");
    }
}

impl opi_coding_agent::credential_store::KeyringBackend for OrderingKeyringBackend {
    fn get(
        &self,
        service: &str,
        provider_id: &str,
    ) -> Result<Option<String>, opi_coding_agent::credential_store::BackendError> {
        self.record_entry_creation();
        opi_coding_agent::credential_store::KeyringBackend::get(self.inner(), service, provider_id)
    }

    fn set(
        &self,
        service: &str,
        provider_id: &str,
        value: &str,
    ) -> Result<(), opi_coding_agent::credential_store::BackendError> {
        self.record_entry_creation();
        opi_coding_agent::credential_store::KeyringBackend::set(
            self.inner(),
            service,
            provider_id,
            value,
        )
    }

    fn delete(
        &self,
        service: &str,
        provider_id: &str,
    ) -> Result<(), opi_coding_agent::credential_store::BackendError> {
        self.record_entry_creation();
        opi_coding_agent::credential_store::KeyringBackend::delete(
            self.inner(),
            service,
            provider_id,
        )
    }
}

impl Drop for OrderingKeyringBackend {
    fn drop(&mut self) {
        self.events
            .lock()
            .expect("ordering events")
            .push("guard_drop");
    }
}

fn assert_native_entry_drop_order(events: &Arc<Mutex<Vec<&'static str>>>) {
    let events = events.lock().expect("ordering events");
    assert_eq!(events.first(), Some(&"native_install"), "{events:?}");
    let first_entry = events
        .iter()
        .position(|event| *event == "entry_creation")
        .expect("at least one keyring entry creation");
    let guard_drop = events
        .iter()
        .position(|event| *event == "guard_drop")
        .expect("native guard drop event");
    assert!(0 < first_entry && first_entry < guard_drop, "{events:?}");
    assert_eq!(events.last(), Some(&"guard_drop"), "{events:?}");
}

struct EnvVarGuard {
    key: String,
    original: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    fn set(key: &str, value: &str) -> Self {
        let guard = Self {
            key: key.to_owned(),
            original: std::env::var_os(key),
        };
        unsafe { std::env::set_var(key, value) };
        guard
    }

    /// Remove an env var for the test duration, restoring the original on drop.
    /// Used to assert missing-credential diagnostics without depending on host
    /// env state.
    fn remove(key: &str) -> Self {
        let guard = Self {
            key: key.to_owned(),
            original: std::env::var_os(key),
        };
        unsafe { std::env::remove_var(key) };
        guard
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => unsafe { std::env::set_var(&self.key, value) },
            None => unsafe { std::env::remove_var(&self.key) },
        }
    }
}

fn with_env_vars<F, R>(vars: &[(&str, &str)], f: F) -> R
where
    F: FnOnce() -> R,
{
    let _lock = ENV_MUTEX.lock().unwrap();
    let _guards: Vec<_> = vars
        .iter()
        .map(|(key, value)| EnvVarGuard::set(key, value))
        .collect();
    f()
}

fn with_env_var<F, R>(key: &str, value: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    with_env_vars(&[(key, value)], f)
}

/// Remove and set env vars atomically under a single mutex lock (composing
/// `with_env_absent` + `with_env_vars` would re-lock and deadlock). Used for
/// tests that must both clear host credentials and set test values, e.g.
/// bedrock missing-credential isolation with redirected AWS file paths.
fn with_env_managed<F, R>(remove: &[&str], set: &[(&str, &str)], f: F) -> R
where
    F: FnOnce() -> R,
{
    let _lock = ENV_MUTEX.lock().unwrap();
    let _removed: Vec<_> = remove.iter().map(|k| EnvVarGuard::remove(k)).collect();
    let _set: Vec<_> = set
        .iter()
        .map(|(key, value)| EnvVarGuard::set(key, value))
        .collect();
    f()
}

fn minimal_request(model: &str) -> Request {
    Request {
        model: model.into(),
        system: None,
        messages: vec![],
        tools: vec![],
        max_tokens: None,
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    }
}

// ---------------------------------------------------------------------------
// Provider construction: correct id() per provider
// ---------------------------------------------------------------------------

#[test]
fn anthropic_provider_construction() {
    let provider = opi_ai::anthropic::AnthropicProvider::new("test-key".into(), None);
    assert_eq!(provider.id(), "anthropic");
}

#[test]
fn openai_provider_construction() {
    let provider = opi_ai::openai_chat::OpenAiChatProvider::new("test-key".into(), None);
    assert_eq!(provider.id(), "openai");
}

#[test]
fn openrouter_provider_construction() {
    let provider = opi_ai::openrouter::openrouter_provider("test-key".into(), None);
    assert_eq!(provider.id(), "openrouter");
}

#[test]
fn mistral_provider_construction() {
    let provider = opi_ai::mistral::mistral_provider("test-key".into(), None);
    assert_eq!(provider.id(), "mistral");
}

#[test]
fn openai_responses_provider_construction() {
    let provider = opi_ai::openai_responses::OpenAiResponsesProvider::new("test-key".into(), None);
    assert_eq!(provider.id(), "openai-responses");
}

#[test]
fn gemini_provider_construction() {
    let provider = opi_ai::gemini::GeminiProvider::new("test-key".into(), None);
    assert_eq!(provider.id(), "gemini");
}

fn assert_same_models(
    actual: &[opi_ai::provider::ModelInfo],
    expected: &[opi_ai::provider::ModelInfo],
) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(actual.id, expected.id);
        assert_eq!(actual.display_name, expected.display_name);
        assert_eq!(
            format!("{:?}", actual.capabilities),
            format!("{:?}", expected.capabilities)
        );
    }
}

#[test]
fn anthropic_model_catalog_matches_constructor() {
    let provider = opi_ai::anthropic::AnthropicProvider::new("test-key".into(), None);
    assert_same_models(provider.models(), &opi_ai::anthropic::model_catalog());
}

#[test]
fn openai_chat_model_catalog_matches_constructor() {
    let provider = opi_ai::openai_chat::OpenAiChatProvider::new("test-key".into(), None);
    assert_same_models(provider.models(), &opi_ai::openai_chat::model_catalog());
}

#[test]
fn openai_responses_model_catalog_matches_constructor() {
    let provider = opi_ai::openai_responses::OpenAiResponsesProvider::new("test-key".into(), None);
    assert_same_models(
        provider.models(),
        &opi_ai::openai_responses::model_catalog(),
    );
}

#[test]
fn openrouter_model_catalog_matches_constructor() {
    let provider = opi_ai::openrouter::openrouter_provider("test-key".into(), None);
    assert_same_models(provider.models(), &opi_ai::openrouter::model_catalog());
}

#[test]
fn mistral_model_catalog_matches_constructor() {
    let provider = opi_ai::mistral::mistral_provider("test-key".into(), None);
    assert_same_models(provider.models(), &opi_ai::mistral::model_catalog());
}

#[test]
fn gemini_model_catalog_matches_constructor() {
    let provider = opi_ai::gemini::GeminiProvider::new("test-key".into(), None);
    assert_same_models(provider.models(), &opi_ai::gemini::model_catalog());
}

#[test]
fn bedrock_model_catalog_matches_constructor() {
    let provider = opi_ai::bedrock::BedrockProvider::new(
        opi_ai::bedrock::sigv4::AwsCredentials {
            access_key_id: "test".into(),
            secret_access_key: "test".into(),
            session_token: None,
            region: "us-east-1".into(),
        },
        None,
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
    );
    assert_same_models(provider.models(), &opi_ai::bedrock::model_catalog());
}

#[test]
fn vertex_model_catalog_matches_constructor() {
    let provider = opi_ai::vertex::VertexProvider::new(
        "test-token".into(),
        "test-project".into(),
        "us-central1".into(),
        None,
    );
    assert_same_models(provider.models(), &opi_ai::vertex::model_catalog());
}

#[tokio::test]
async fn native_keyring_precedes_live_provider_construction() {
    use opi_coding_agent::credential_store::{
        FakeKeyringBackend, KEYCHAIN_PRESENCE_SERVICE, KEYCHAIN_SERVICE, KeyringBackendFactory,
    };
    use opi_coding_agent::provider_factory::build_provider_bundle;

    let events = Arc::new(Mutex::new(Vec::new()));
    let factory_events = Arc::clone(&events);
    let backend_factory: KeyringBackendFactory = Box::new(move || {
        let backend = FakeKeyringBackend::new();
        factory_events
            .lock()
            .expect("ordering events")
            .push("native_install");
        backend.seed_raw(
            KEYCHAIN_SERVICE,
            "anthropic",
            r#"{"version":1,"kind":"api_key","api_key":"test-provider-ordering"}"#,
        );
        backend.seed_raw(KEYCHAIN_PRESENCE_SERVICE, "anthropic", "api_key");
        Box::new(OrderingKeyringBackend {
            inner: backend,
            events: Arc::clone(&factory_events),
        })
    });

    let dir = tempfile::tempdir().expect("temp dir");
    let mut config = OpiConfig::default();
    config.defaults.model = "anthropic:claude-sonnet-4-5-20250514".into();
    assert!(
        events.lock().expect("ordering events").is_empty(),
        "backend construction must remain lazy until provider startup"
    );
    let bundle = build_provider_bundle(&config, dir.path().to_path_buf(), backend_factory)
        .await
        .expect("provider bundle builds after native installation");
    assert_eq!(bundle.provider.id(), "anthropic");
    drop(bundle);
    assert_native_entry_drop_order(&events);
}

#[test]
fn provider_bundle_retains_redacted_backend_fallback_diagnostic() {
    use opi_agent::diagnostic::{RedactionMode, SOURCE_PROVIDER, Severity};
    use opi_coding_agent::credential_store::FakeKeyringBackend;
    use opi_coding_agent::provider_factory::build_provider_bundle;

    const ENV_VAR: &str = "OPI_TEST_BACKEND_FALLBACK_KEY_D1D4";
    const SECRET_CANARY: &str = "d1d4-secret-canary-must-not-appear";

    let dir = tempfile::tempdir().expect("temp dir");
    let mut config = OpiConfig::default();
    config.defaults.model = "anthropic:claude-sonnet-4-5-20250514".into();
    config.providers.anthropic.api_key_env = ENV_VAR.into();

    with_env_managed(
        &["ANTHROPIC_OAUTH_TOKEN"],
        &[(ENV_VAR, SECRET_CANARY)],
        || {
            let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
            let bundle = runtime
                .block_on(build_provider_bundle(
                    &config,
                    dir.path().to_path_buf(),
                    Box::new(|| Box::new(FakeKeyringBackend::new().with_unavailable())),
                ))
                .expect("environment fallback keeps provider construction usable");

            assert_eq!(bundle.provider.id(), "anthropic");
            assert_eq!(
                bundle.diagnostics.len(),
                1,
                "factory must retain exactly one backend fallback diagnostic"
            );
            let payload = bundle.diagnostics[0].redacted_payload(RedactionMode::Summary);
            assert_eq!(payload.severity, Severity::Warning);
            assert_eq!(payload.source, SOURCE_PROVIDER);
            assert_eq!(payload.code, "provider_credential_backend_unavailable");

            let encoded = serde_json::to_string(&payload).expect("diagnostic serializes");
            assert!(encoded.contains("anthropic"), "{encoded}");
            assert!(encoded.contains(ENV_VAR), "{encoded}");
            assert!(encoded.contains("environment_fallback"), "{encoded}");
            assert!(!encoded.contains(SECRET_CANARY), "{encoded}");
        },
    );
}

#[test]
fn build_provider_production_returns_store_owning_bundle() {
    use std::future::Future;

    use opi_coding_agent::provider_factory::{
        ProviderBuildError, ProviderBundle, build_provider_production,
    };

    fn assert_bundle_output<F>(_: F)
    where
        F: Future<Output = Result<ProviderBundle, ProviderBuildError>>,
    {
    }

    let config = OpiConfig::default();
    assert_bundle_output(build_provider_production(
        &config,
        std::path::PathBuf::from("unused-unpolled-keyring-path"),
    ));
}

#[tokio::test]
async fn provider_bundle_retains_native_store_while_provider_is_callable() {
    use futures_util::StreamExt;
    use opi_coding_agent::credential_store::{
        CredentialResolver, FakeKeyringBackend, KeychainCredentialStore,
    };
    use opi_coding_agent::oauth::OAuthProviderRegistry;
    use opi_coding_agent::provider_factory::ProviderBundle;

    let events = Arc::new(Mutex::new(vec!["native_install"]));
    let dir = tempfile::tempdir().expect("temp dir");
    let store = Arc::new(KeychainCredentialStore::new(
        Box::new(OrderingKeyringBackend {
            inner: FakeKeyringBackend::new(),
            events: Arc::clone(&events),
        }),
        dir.path().to_path_buf(),
    ));
    let resolver = CredentialResolver::production(store.clone());
    let registry = OAuthProviderRegistry::registry_with_builtins();
    let bundle = ProviderBundle {
        provider: Box::new(MockProvider::new(
            "owned-provider",
            vec![opi_ai::test_support::text_response("delegated")],
        )),
        store,
        resolver,
        registry,
        diagnostics: Vec::new(),
    };

    assert!(!events.lock().unwrap().contains(&"guard_drop"));
    assert_eq!(bundle.provider.id(), "owned-provider");
    assert_eq!(bundle.provider.models()[0].id, "mock-model");
    let mut stream = bundle.provider.stream(minimal_request("mock-model"));
    assert!(
        stream.next().await.is_some(),
        "stream delegates to inner provider"
    );
    assert!(!events.lock().unwrap().contains(&"guard_drop"));

    drop(stream);
    drop(bundle);
    assert_eq!(events.lock().unwrap().last(), Some(&"guard_drop"));
}

#[tokio::test]
async fn anthropic_stored_api_key_routes_to_api_key_wire_auth() {
    use futures_util::StreamExt;
    use opi_ai::credential::{Credential, CredentialStore};
    use opi_coding_agent::credential_store::{
        CredentialResolver, FakeKeyringBackend, KeychainCredentialStore,
    };
    use opi_coding_agent::oauth::OAuthProviderRegistry;
    use secrecy::SecretString;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(KeychainCredentialStore::new(
        Box::new(FakeKeyringBackend::new()),
        dir.path().to_path_buf(),
    ));
    let api_key = "sk-anthropic-stored-api";
    store
        .write(
            "anthropic",
            &Credential::ApiKey(SecretString::new(api_key.to_owned().into_boxed_str())),
        )
        .await
        .unwrap();
    let resolver =
        CredentialResolver::new(store, Arc::new(|_name: &str| -> Option<String> { None }));
    let registry = OAuthProviderRegistry::registry_with_builtins();
    let mut config = OpiConfig::default();
    config.defaults.model = "anthropic:claude-sonnet-4-5-20250514".into();
    config.providers.anthropic.base_url = Some(server.uri());

    let provider = opi_coding_agent::provider_factory::build_provider_with_oauth(
        &config, &resolver, &registry,
    )
    .await
    .expect("Anthropic API-key provider builds");
    let mut stream = provider.stream(minimal_request("anthropic:claude-sonnet-4-5-20250514"));
    while let Some(event) = stream.next().await {
        if event.as_ref().is_ok_and(|event| event.is_terminal()) || event.is_err() {
            break;
        }
    }

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "exactly one Anthropic request");
    assert_eq!(
        requests[0]
            .headers
            .get("x-api-key")
            .map(|value| value.to_str().unwrap()),
        Some(api_key)
    );
    assert!(requests[0].headers.get("authorization").is_none());
}

// ---------------------------------------------------------------------------
// OpenRouter with custom referer header
// ---------------------------------------------------------------------------

#[test]
fn openrouter_with_custom_referer() {
    let compat = opi_ai::openai_chat::CompatConfig::default();
    // Get the default model list from the convenience function.
    let temp = opi_ai::openrouter::openrouter_provider(String::new(), None);
    let models = temp.models().to_vec();
    let provider = opi_ai::openai_chat::OpenAiChatProvider::new_for_profile(
        "test-key".into(),
        "https://openrouter.ai/api".into(),
        "openrouter".into(),
        compat,
        vec![
            ("HTTP-Referer".into(), "https://custom.example.com".into()),
            ("X-Title".into(), "opi".into()),
        ],
        models,
    );
    assert_eq!(provider.id(), "openrouter");
}

// ---------------------------------------------------------------------------
// Defaults config: provider structs
// ---------------------------------------------------------------------------

#[test]
fn generic_provider_default_has_empty_env() {
    let cfg = GenericProviderConfig::default();
    assert!(cfg.api_key_env.is_empty());
    assert!(cfg.base_url.is_none());
}

#[test]
fn openrouter_provider_default_has_empty_env() {
    let cfg = OpenRouterProviderConfig::default();
    assert!(cfg.api_key_env.is_empty());
    assert!(cfg.base_url.is_none());
    assert!(cfg.referer.is_none());
}

#[test]
fn opi_config_default_anthropic_env() {
    let config = OpiConfig::default();
    assert_eq!(config.providers.anthropic.api_key_env, "ANTHROPIC_API_KEY");
}

// ---------------------------------------------------------------------------
// TOML deserialization: all provider sections
// ---------------------------------------------------------------------------

#[test]
fn toml_parses_openai_provider() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[providers.openai]
api_key_env = "MY_OPENAI_KEY"
base_url = "https://custom.openai.example.com"
"#,
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();
    assert_eq!(config.providers.openai.api_key_env, "MY_OPENAI_KEY");
    assert_eq!(
        config.providers.openai.base_url.as_deref(),
        Some("https://custom.openai.example.com")
    );
}

#[test]
fn toml_parses_openai_compatible_profile_with_models_and_flags() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[providers.openai_compatible.localai]
api_key_env = "LOCALAI_API_KEY"
base_url = "https://localai.example.com"
system_role_override = "developer"
max_tokens_field = "max_completion_tokens"
tool_result_name_field = true
usage_in_stream = true

[providers.openai_compatible.localai.proxy]
url = "http://proxy.example.com:8080"

[[providers.openai_compatible.localai.models]]
id = "local-model"
display_name = "Local Model"
context_window = 128000
max_output_tokens = 4096
supports_images = true
supports_streaming = true
supports_thinking = true
"#,
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();

    let profile = config
        .providers
        .openai_compatible
        .get("localai")
        .expect("profile should be parsed");
    assert_eq!(profile.id, "localai");
    assert_eq!(profile.api_key_env, "LOCALAI_API_KEY");
    assert_eq!(profile.base_url, "https://localai.example.com");
    assert_eq!(profile.system_role_override.as_deref(), Some("developer"));
    assert_eq!(
        profile.max_tokens_field.as_deref(),
        Some("max_completion_tokens")
    );
    assert!(profile.tool_result_name_field);
    assert!(profile.usage_in_stream);
    assert_eq!(
        profile.proxy.as_ref().map(|proxy| proxy.url.as_str()),
        Some("http://proxy.example.com:8080")
    );

    let model = profile.models.first().expect("model should be parsed");
    assert_eq!(model.id, "local-model");
    assert_eq!(model.display_name, "Local Model");
    assert_eq!(model.context_window, 128000);
    assert_eq!(model.max_output_tokens, 4096);
    assert!(model.supports_images);
    assert!(model.supports_streaming);
    assert!(model.supports_thinking);
}

#[test]
fn toml_parses_openai_compatible_profile_custom_chat_completions_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[providers.openai_compatible.zai]
api_key_env = "ZAI_API_KEY"
base_url = "https://open.bigmodel.cn"
chat_completions_path = "/api/paas/v4/chat/completions"

[[providers.openai_compatible.zai.models]]
id = "glm-4.5-flash"
display_name = "GLM 4.5 Flash"
context_window = 128000
max_output_tokens = 8192
"#,
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();
    let profile = config
        .providers
        .openai_compatible
        .get("zai")
        .expect("zai profile should be parsed");
    assert_eq!(
        profile.chat_completions_path.as_deref(),
        Some("/api/paas/v4/chat/completions")
    );

    // Profiles without the field default to None; the factory then resolves
    // the path to "/v1/chat/completions".
    let dir2 = tempfile::tempdir().unwrap();
    let path2 = dir2.path().join("config2.toml");
    std::fs::write(
        &path2,
        r#"
[providers.openai_compatible.plain]
api_key_env = "PLAIN_API_KEY"
base_url = "https://plain.example.com"

[[providers.openai_compatible.plain.models]]
id = "m"
display_name = "m"
context_window = 8000
max_output_tokens = 1024
"#,
    )
    .unwrap();
    let config2 = load_config_file(&path2).unwrap();
    let plain = config2
        .providers
        .openai_compatible
        .get("plain")
        .expect("plain profile should be parsed");
    assert!(
        plain.chat_completions_path.is_none(),
        "absent chat_completions_path must default to None"
    );
}

#[test]
fn toml_parses_openrouter_provider() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[providers.openrouter]
api_key_env = "MY_OPENROUTER_KEY"
referer = "https://myapp.example.com"
"#,
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();
    assert_eq!(config.providers.openrouter.api_key_env, "MY_OPENROUTER_KEY");
    assert_eq!(
        config.providers.openrouter.referer.as_deref(),
        Some("https://myapp.example.com")
    );
}

#[test]
fn toml_parses_mistral_provider() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[providers.mistral]
api_key_env = "MY_MISTRAL_KEY"
"#,
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();
    assert_eq!(config.providers.mistral.api_key_env, "MY_MISTRAL_KEY");
}

#[test]
fn toml_parses_openai_responses_provider() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[providers.openai_responses]
api_key_env = "MY_OPENAI_KEY"
"#,
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();
    assert_eq!(
        config.providers.openai_responses.api_key_env,
        "MY_OPENAI_KEY"
    );
}

#[test]
fn toml_parses_gemini_provider() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[providers.gemini]
api_key_env = "MY_GEMINI_KEY"
base_url = "https://custom-gemini.example.com"
"#,
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();
    assert_eq!(config.providers.gemini.api_key_env, "MY_GEMINI_KEY");
    assert_eq!(
        config.providers.gemini.base_url.as_deref(),
        Some("https://custom-gemini.example.com")
    );
}

#[test]
fn toml_multiple_providers_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[providers.anthropic]
api_key_env = "KEY_A"

[providers.openai]
api_key_env = "KEY_O"

[providers.gemini]
api_key_env = "KEY_G"

[providers.mistral]
api_key_env = "KEY_M"

[providers.openrouter]
api_key_env = "KEY_OR"

[providers.openai_responses]
api_key_env = "KEY_OAR"
"#,
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();
    assert_eq!(config.providers.anthropic.api_key_env, "KEY_A");
    assert_eq!(config.providers.openai.api_key_env, "KEY_O");
    assert_eq!(config.providers.gemini.api_key_env, "KEY_G");
    assert_eq!(config.providers.mistral.api_key_env, "KEY_M");
    assert_eq!(config.providers.openrouter.api_key_env, "KEY_OR");
    assert_eq!(config.providers.openai_responses.api_key_env, "KEY_OAR");
}

// ---------------------------------------------------------------------------
// Phase 10.2: provider construction routes through the collection/auth seam
// ---------------------------------------------------------------------------

#[test]
fn provider_factory_routes_through_collection() {
    use opi_ai::{AuthDescriptor, AuthStatus};
    use opi_coding_agent::config::load_config_file;
    use opi_coding_agent::provider_factory::{auth_descriptor_for, build_collection_for_listing};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let env_var = "OPI_TEST_FACTORY_ROUTE_9F2A7C11";
    std::fs::write(
        &path,
        format!(
            r#"
[providers.openai_compatible.testprof]
api_key_env = "{env_var}"
base_url = "https://testprof.example.com"

[[providers.openai_compatible.testprof.models]]
id = "test-model"
display_name = "Test Model"
context_window = 128000
max_output_tokens = 4096
supports_images = false
supports_streaming = true
supports_thinking = false
"#
        ),
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();

    // Auth-policy mapping is centralized in the factory and deterministic
    // (no environment variable needs to be set).
    match auth_descriptor_for(&config, "openai") {
        Some(AuthDescriptor::EnvApiKey { env_var }) => assert_eq!(env_var, "OPENAI_API_KEY"),
        other => panic!("expected openai EnvApiKey, got {other:?}"),
    }
    assert!(auth_descriptor_for(&config, "not-a-real-provider").is_none());

    with_env_var(env_var, "test-key", || {
        let collection = build_collection_for_listing(&config, &std::collections::HashMap::new())
            .expect("listing collection builds through the factory");

        // The factory returns the ProviderCollection/auth-seam type and the profile
        // model is resolvable through it.
        let (_provider, model) = collection
            .resolve("testprof:test-model")
            .expect("profile model resolves through the collection");
        assert_eq!(model.id, "test-model");

        // Auth + compat metadata for the config-sourced profile live on the collection.
        assert_eq!(
            collection.auth_status("testprof"),
            Some(AuthStatus::Configured)
        );
        match collection.auth_descriptor("testprof") {
            Some(AuthDescriptor::Resolved { source }) => {
                assert_eq!(source, &format!("env {env_var}"))
            }
            other => panic!("expected profile Resolved auth, got {other:?}"),
        }
        let compat = collection
            .compat("testprof")
            .expect("profile compat metadata attached");
        assert!(compat.openai_compatible);
        assert_eq!(compat.profile.as_deref(), Some("testprof"));
    });
}

// ---------------------------------------------------------------------------
// Phase 12 task 12.3 — OpenAI-compatible profile flags + override precedence
// ---------------------------------------------------------------------------

#[test]
fn openai_compatible_profile_overrides() {
    use opi_coding_agent::provider_factory::build_provider;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let env_var = "OPI_TEST_COMPAT_OVERRIDES_4F2A91";
    std::fs::write(
        &path,
        format!(
            r#"
[defaults]
model = "myprof:reasoning-model"

[providers.openai_compatible.myprof]
api_key_env = "{env_var}"
base_url = "https://myprof.example.com"
system_role_override = "system"
max_tokens_field = "max_tokens"
strict_tool_schema = true
reasoning_effort = "medium"
cache_key = "profile-cache"
require_assistant_after_tool_result = true

[providers.openai_compatible.myprof.extra_headers]
X-Session-Id = "sess-1"

[[providers.openai_compatible.myprof.models]]
id = "reasoning-model"
display_name = "Reasoning"
context_window = 128000
max_output_tokens = 4096
system_role_override = "developer"
max_tokens_field = "max_completion_tokens"

[[providers.openai_compatible.myprof.models]]
id = "plain-model"
display_name = "Plain"
context_window = 128000
max_output_tokens = 4096
"#
        ),
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();

    let profile = config
        .providers
        .openai_compatible
        .get("myprof")
        .expect("profile should be parsed");
    assert!(profile.strict_tool_schema);
    assert_eq!(profile.reasoning_effort.as_deref(), Some("medium"));
    assert_eq!(profile.cache_key.as_deref(), Some("profile-cache"));
    assert!(profile.require_assistant_after_tool_result);
    assert_eq!(
        profile.extra_headers,
        vec![("X-Session-Id".to_string(), "sess-1".to_string())]
    );
    let reasoning = profile
        .models
        .iter()
        .find(|m| m.id == "reasoning-model")
        .expect("reasoning model");
    assert_eq!(reasoning.system_role_override.as_deref(), Some("developer"));
    assert_eq!(
        reasoning.max_tokens_field.as_deref(),
        Some("max_completion_tokens")
    );
    let plain = profile
        .models
        .iter()
        .find(|m| m.id == "plain-model")
        .expect("plain model");
    assert!(plain.system_role_override.is_none());
    assert!(plain.max_tokens_field.is_none());

    // The config-driven profile routes through the shared factory path.
    with_env_var(env_var, "test-key", || {
        build_provider(&config).expect("config-driven profile builds through the factory");
    });
}

#[test]
fn config_driven_compatible_profiles_are_preferred() {
    use opi_coding_agent::provider_factory::{ProviderBuildError, build_provider};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let env_var = "OPI_TEST_COMPAT_PREFERRED_7B1C04";
    std::fs::write(
        &path,
        format!(
            r#"
[defaults]
model = "acmeprof:acme-model"

[providers.openai_compatible.acmeprof]
api_key_env = "{env_var}"
base_url = "https://acme.example.com"

[[providers.openai_compatible.acmeprof.models]]
id = "acme-model"
display_name = "Acme"
context_window = 128000
max_output_tokens = 4096
"#
        ),
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();

    // A compatible provider is representable through profile metadata + config
    // overrides and routes through build_provider — the config-driven path is
    // preferred for OpenAI-compatible breadth rather than a new first-class
    // adapter (Phase 12 non-goal guard).
    with_env_var(env_var, "test-key", || {
        let provider = build_provider(&config).expect("config-driven profile is preferred");
        assert_eq!(provider.id(), "acmeprof");
    });

    // A provider id that is neither built-in nor a declared profile does NOT
    // silently become a first-class adapter — it fails explicitly. This guards
    // against broad first-class provider expansion as the default path.
    let mut cfg2 = config.clone();
    cfg2.defaults.model = "brand-new-vendor:some-model".into();
    match build_provider(&cfg2) {
        Ok(_) => panic!("unknown provider must not become first-class"),
        Err(ProviderBuildError::Config(_)) => {}
        Err(other) => panic!("expected Config build error, got {other:?}"),
    }
}

#[test]
fn listing_collection_skips_whitespace_only_credentials() {
    use opi_coding_agent::config::load_config_file;
    use opi_coding_agent::provider_factory::build_collection_for_listing;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let openai_env = "OPI_TEST_FACTORY_OPENAI_WS_ONLY_3E7B91";
    let profile_env = "OPI_TEST_FACTORY_PROFILE_WS_ONLY_3E7B91";
    std::fs::write(
        &path,
        format!(
            r#"
[providers.openai]
api_key_env = "{openai_env}"

[providers.openai_compatible.testprof]
api_key_env = "{profile_env}"
base_url = "https://testprof.example.com"

[[providers.openai_compatible.testprof.models]]
id = "test-model"
display_name = "Test Model"
context_window = 128000
max_output_tokens = 4096
supports_images = false
supports_streaming = true
supports_thinking = false
"#
        ),
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();

    with_env_vars(&[(openai_env, "   "), (profile_env, "\t ")], || {
        let collection = build_collection_for_listing(&config, &std::collections::HashMap::new())
            .expect("whitespace credentials are skipped");
        let provider_ids = collection.provider_ids();
        assert!(!provider_ids.contains(&"openai"));
        assert!(!provider_ids.contains(&"testprof"));
    });
}

#[test]
fn listing_collection_uses_resolved_auth_for_constructed_builtin() {
    use opi_ai::{AuthDescriptor, AuthStatus};
    use opi_coding_agent::config::load_config_file;
    use opi_coding_agent::provider_factory::build_collection_for_listing;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let env_var = "OPI_TEST_FACTORY_OPENAI_RESOLVED_25E1AC";
    std::fs::write(
        &path,
        format!(
            r#"
[providers.openai]
api_key_env = "{env_var}"
"#
        ),
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();

    with_env_var(env_var, "test-key", || {
        let collection = build_collection_for_listing(&config, &std::collections::HashMap::new())
            .expect("listing collection builds");
        assert_eq!(
            collection.auth_status("openai"),
            Some(AuthStatus::Configured)
        );
        match collection.auth_descriptor("openai") {
            Some(AuthDescriptor::Resolved { source }) => {
                assert_eq!(source, &format!("env {env_var}"))
            }
            other => panic!("expected builtin Resolved auth, got {other:?}"),
        }
    });
}

#[tokio::test]
async fn metadata_only_provider_dispatch_returns_explicit_error() {
    use opi_coding_agent::provider_factory::assemble_harness_collection;

    let provider = MockProvider::new("metadata-provider", vec![]);
    let (collection, diagnostics) = assemble_harness_collection(&provider, None);
    assert!(diagnostics.is_empty());

    let error = collection
        .dispatch_complete(
            "metadata-provider:mock-model",
            minimal_request("metadata-provider:mock-model"),
        )
        .await
        .expect_err("metadata provider should not dispatch");
    let message = error.to_string();
    assert!(
        message.contains("metadata-only provider"),
        "expected metadata-only dispatch error, got {message:?}"
    );
    assert!(
        message.contains("'metadata-provider'"),
        "expected active provider id in dispatch error, got {message:?}"
    );
}

#[test]
fn built_in_openai_compatible_metadata_is_set() {
    use opi_coding_agent::provider_factory::compat_metadata_for;

    for provider in ["openai", "openrouter", "mistral"] {
        assert!(
            compat_metadata_for(provider).openai_compatible,
            "{provider} should advertise OpenAI-compatible chat metadata"
        );
    }
    assert!(!compat_metadata_for("anthropic").openai_compatible);
}

// ---------------------------------------------------------------------------
// Phase 12 task 12.8 — provider-family wiring + auth/endpoint diagnostics +
// profile precedence through the production factory call site
// ---------------------------------------------------------------------------

/// Every built-in provider family must route through the centralized
/// `build_provider` factory (the Phase 10 collection seam), not be constructed
/// by `opi_ai` constructors that bypass it. DoD: "runtime startup builds the
/// active provider through build_provider" + "provider-family wiring through
/// production call sites". The existing `*_provider_construction` tests above
/// call the `opi_ai` constructors directly; this test is the factory-routing
/// proof for all nine families.
#[test]
fn build_provider_wires_each_builtin_provider_family() {
    use opi_coding_agent::config::OpiConfig;
    use opi_coding_agent::provider_factory::build_provider;

    fn configure_azure(c: &mut OpiConfig) {
        c.providers.azure.endpoint = Some("https://test.openai.azure.com".into());
    }
    fn configure_vertex(c: &mut OpiConfig) {
        c.providers.vertex.project = Some("test-project".into());
        c.providers.vertex.location = Some("us-central1".into());
    }
    fn noop(_: &mut OpiConfig) {}

    // Factored into a type alias so the cases binding does not trip
    // clippy::type_complexity; the loop body is identical for every family.
    type FamilyCase = (
        &'static str,
        &'static [(&'static str, &'static str)],
        fn(&mut OpiConfig),
    );
    let cases: &[FamilyCase] = &[
        ("anthropic", &[("ANTHROPIC_API_KEY", "test-key")], noop),
        ("openai", &[("OPENAI_API_KEY", "test-key")], noop),
        ("openrouter", &[("OPENROUTER_API_KEY", "test-key")], noop),
        ("mistral", &[("MISTRAL_API_KEY", "test-key")], noop),
        ("openai-responses", &[("OPENAI_API_KEY", "test-key")], noop),
        ("gemini", &[("GEMINI_API_KEY", "test-key")], noop),
        (
            "bedrock",
            &[
                ("AWS_ACCESS_KEY_ID", "test-akid"),
                ("AWS_SECRET_ACCESS_KEY", "test-sak"),
            ],
            noop,
        ),
        (
            "azure",
            &[("AZURE_OPENAI_API_KEY", "test-key")],
            configure_azure,
        ),
        (
            "vertex",
            &[("VERTEX_ACCESS_TOKEN", "test-token")],
            configure_vertex,
        ),
    ];

    for (id, env, configure) in cases.iter().copied() {
        with_env_vars(env, || {
            let mut config = OpiConfig::default();
            config.defaults.model = format!("{id}:test-model");
            configure(&mut config);
            let provider = build_provider(&config)
                .unwrap_or_else(|e| panic!("build_provider({id}) should succeed, got: {e:?}"));
            assert_eq!(
                provider.id(),
                id,
                "build_provider must route each built-in family through the factory"
            );
        });
    }
}

#[tokio::test]
async fn builtin_single_wire_models_route_by_declared_wire() {
    use futures_util::StreamExt;
    use opi_ai::model_info::WireApi;
    use opi_coding_agent::provider_factory::build_provider;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[derive(Clone, Copy)]
    enum Route {
        Anthropic,
        OpenAiChat,
        OpenAiResponses,
        Gemini,
        Vertex,
        Bedrock,
        Azure,
    }

    struct RouteCase {
        provider_id: &'static str,
        expected_wire: WireApi,
        route: Route,
    }

    let cases = [
        RouteCase {
            provider_id: "anthropic",
            expected_wire: WireApi::AnthropicMessages,
            route: Route::Anthropic,
        },
        RouteCase {
            provider_id: "openai",
            expected_wire: WireApi::OpenAiCompletions,
            route: Route::OpenAiChat,
        },
        RouteCase {
            provider_id: "openai-responses",
            expected_wire: WireApi::OpenAiResponses,
            route: Route::OpenAiResponses,
        },
        RouteCase {
            provider_id: "openrouter",
            expected_wire: WireApi::OpenAiCompletions,
            route: Route::OpenAiChat,
        },
        RouteCase {
            provider_id: "mistral",
            expected_wire: WireApi::OpenAiCompletions,
            route: Route::OpenAiChat,
        },
        RouteCase {
            provider_id: "gemini",
            expected_wire: WireApi::GoogleGenerativeAi,
            route: Route::Gemini,
        },
        RouteCase {
            provider_id: "vertex",
            expected_wire: WireApi::GoogleVertex,
            route: Route::Vertex,
        },
        RouteCase {
            provider_id: "bedrock",
            expected_wire: WireApi::BedrockConverseStream,
            route: Route::Bedrock,
        },
        RouteCase {
            provider_id: "azure",
            expected_wire: WireApi::AzureOpenAiCompletions,
            route: Route::Azure,
        },
    ];

    const ROUTE_KEY_ENV: &str = "OPI_TEST_FACTORY_ROUTE_KEY_14_15";

    for case in cases {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(404).set_body_string("offline route capture"))
            .mount(&server)
            .await;

        let mut config = OpiConfig::default();
        let selected_model = if case.provider_id == "bedrock" {
            "anthropic.route-model"
        } else {
            "route-model"
        };
        config.defaults.model = format!("{}:{selected_model}", case.provider_id);
        match case.provider_id {
            "anthropic" => {
                config.providers.anthropic.api_key_env = ROUTE_KEY_ENV.into();
                config.providers.anthropic.base_url = Some(server.uri());
            }
            "openai" => {
                config.providers.openai.api_key_env = ROUTE_KEY_ENV.into();
                config.providers.openai.base_url = Some(server.uri());
            }
            "openai-responses" => {
                config.providers.openai_responses.api_key_env = ROUTE_KEY_ENV.into();
                config.providers.openai_responses.base_url = Some(server.uri());
            }
            "openrouter" => {
                config.providers.openrouter.api_key_env = ROUTE_KEY_ENV.into();
                config.providers.openrouter.base_url = Some(server.uri());
            }
            "mistral" => {
                config.providers.mistral.api_key_env = ROUTE_KEY_ENV.into();
                config.providers.mistral.base_url = Some(server.uri());
            }
            "gemini" => {
                config.providers.gemini.api_key_env = ROUTE_KEY_ENV.into();
                config.providers.gemini.base_url = Some(server.uri());
            }
            "vertex" => {
                config.providers.vertex.access_token_env = ROUTE_KEY_ENV.into();
                config.providers.vertex.project = Some("test-project".into());
                config.providers.vertex.location = Some("us-central1".into());
                config.providers.vertex.base_url = Some(server.uri());
            }
            "bedrock" => {
                config.providers.bedrock.access_key_id = Some("test-akid".into());
                config.providers.bedrock.secret_access_key_env = Some(ROUTE_KEY_ENV.into());
                config.providers.bedrock.region = Some("us-east-1".into());
                config.providers.bedrock.base_url = Some(server.uri());
            }
            "azure" => {
                config.providers.azure.api_key_env = ROUTE_KEY_ENV.into();
                config.providers.azure.endpoint = Some(server.uri());
                assert!(
                    config.providers.azure.deployments.is_empty(),
                    "exercise the selected-model fallback branch"
                );
            }
            other => unreachable!("unhandled route case {other}"),
        }

        let provider = with_env_var(ROUTE_KEY_ENV, "test-secret", || {
            build_provider(&config).unwrap_or_else(|error| {
                panic!("build_provider({}) failed: {error:?}", case.provider_id)
            })
        });
        let model = provider
            .models()
            .first()
            .unwrap_or_else(|| panic!("{} factory returned an empty catalog", case.provider_id));
        assert_eq!(
            model.wire_api, case.expected_wire,
            "{}:{} wire metadata",
            case.provider_id, model.id
        );
        model.validate().unwrap();
        let model_id = model.id.clone();

        let mut stream =
            provider.stream(minimal_request(&format!("{}:{model_id}", case.provider_id)));
        tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
            .await
            .unwrap_or_else(|_| panic!("{} stream did not issue a request", case.provider_id));
        drop(stream);

        let requests = server.received_requests().await.unwrap();
        assert_eq!(
            requests.len(),
            1,
            "{} must emit exactly one request",
            case.provider_id
        );
        let request = &requests[0];
        assert_eq!(request.method.as_str(), "POST", "{}", case.provider_id);
        let expected_path = match case.route {
            Route::Anthropic => "/v1/messages".to_owned(),
            Route::OpenAiChat => "/v1/chat/completions".to_owned(),
            Route::OpenAiResponses => "/v1/responses".to_owned(),
            Route::Gemini => {
                format!("/v1beta/models/{model_id}:streamGenerateContent")
            }
            Route::Vertex => format!(
                "/v1/projects/test-project/locations/us-central1/publishers/google/models/{model_id}:streamGenerateContent"
            ),
            Route::Bedrock => format!("/model/{model_id}/converse-stream"),
            Route::Azure => {
                format!("/openai/deployments/{model_id}/chat/completions")
            }
        };
        assert_eq!(
            request.url.path(),
            expected_path,
            "{} concrete route",
            case.provider_id
        );
        match case.route {
            Route::Gemini | Route::Vertex => {
                assert_eq!(request.url.query(), Some("alt=sse"), "{}", case.provider_id);
            }
            Route::Azure => {
                assert_eq!(
                    request.url.query(),
                    Some("api-version=2024-06-01"),
                    "azure API version route"
                );
            }
            _ => assert!(request.url.query().is_none(), "{}", case.provider_id),
        }
        match case.route {
            Route::Anthropic => {
                assert_eq!(
                    request
                        .headers
                        .get("x-api-key")
                        .and_then(|value| value.to_str().ok()),
                    Some("test-secret")
                );
            }
            Route::OpenAiChat | Route::OpenAiResponses | Route::Vertex => {
                assert_eq!(
                    request
                        .headers
                        .get("authorization")
                        .and_then(|value| value.to_str().ok()),
                    Some("Bearer test-secret"),
                    "{} auth transport",
                    case.provider_id
                );
            }
            Route::Gemini => {
                assert_eq!(
                    request
                        .headers
                        .get("x-goog-api-key")
                        .and_then(|value| value.to_str().ok()),
                    Some("test-secret")
                );
            }
            Route::Bedrock => {
                assert!(
                    request
                        .headers
                        .get("authorization")
                        .and_then(|value| value.to_str().ok())
                        .is_some_and(|value| value.starts_with("AWS4-HMAC-SHA256 ")),
                    "Bedrock must use SigV4 ConverseStream transport"
                );
            }
            Route::Azure => {
                assert_eq!(
                    request
                        .headers
                        .get("api-key")
                        .and_then(|value| value.to_str().ok()),
                    Some("test-secret")
                );
            }
        }
    }
}

#[test]
fn single_wire_factory_guard_rejects_mismatch_before_stream() {
    use opi_ai::model_info::WireApi;
    use opi_coding_agent::provider_factory::validate_single_wire_provider;

    let mut mismatched = opi_ai::ModelInfo::new(
        "mismatch",
        "Mismatch",
        WireApi::OpenAiCompletions,
        opi_ai::ModelCapabilities::new(8_192, 1_024),
    );
    mismatched.compat = opi_ai::WireCompat::AnthropicMessages(Default::default());
    let provider = MockProvider::new_with_models("factory-test", vec![mismatched], vec![]);
    let call_log = provider.call_log_handle();

    assert!(matches!(
        validate_single_wire_provider(Box::new(provider), WireApi::OpenAiCompletions),
        Err(
            opi_coding_agent::provider_factory::ProviderBuildError::Provider(
                opi_ai::provider::ProviderError::WireCompatMismatch { .. }
            )
        )
    ));
    assert!(
        call_log.lock().unwrap().is_empty(),
        "factory validation must fail before Provider::stream can emit a request"
    );
}

#[test]
fn azure_default_factory_catalog_uses_selected_deployment_wire() {
    use opi_ai::model_info::WireApi;
    use opi_coding_agent::provider_factory::build_provider;

    const KEY_ENV: &str = "OPI_TEST_AZURE_SELECTED_DEPLOYMENT_14_15";
    let mut config = OpiConfig::default();
    config.defaults.model = "azure:selected-deployment".into();
    config.providers.azure.api_key_env = KEY_ENV.into();
    config.providers.azure.endpoint = Some("http://127.0.0.1:9".into());
    assert!(config.providers.azure.deployments.is_empty());

    let provider = with_env_var(KEY_ENV, "test-key", || build_provider(&config).unwrap());
    assert_eq!(provider.models().len(), 1);
    assert_eq!(provider.models()[0].id, "selected-deployment");
    assert_eq!(
        provider.models()[0].wire_api,
        WireApi::AzureOpenAiCompletions
    );
}

#[test]
fn azure_constructor_rejects_empty_selected_deployment() {
    assert!(matches!(
        opi_ai::azure_openai::AzureOpenAIProvider::new(
            "test-key".into(),
            Some("https://test.openai.azure.com".into()),
            " \t".into(),
            None,
        ),
        Err(opi_ai::provider::ProviderError::Config(message))
            if message.contains("deployment")
    ));
}

#[test]
fn azure_config_constructor_rejects_empty_deployment_catalog() {
    assert!(matches!(
        opi_ai::azure_openai::AzureOpenAIProvider::from_config(
            "test-key".into(),
            Some("https://test.openai.azure.com".into()),
            vec![],
            None,
        ),
        Err(opi_ai::provider::ProviderError::Config(message))
            if message.contains("deployment")
    ));
}

/// Credentials are validated at build time before any provider construction
/// or dispatch. DoD: "validates credentials at build time" + "missing or
/// invalid credentials emit safe diagnostics with provider/error class and
/// remediation". Covers both cred-validation shapes: `require_api_key`
/// (anthropic/azure/vertex -> "missing API key: set {env}") and bedrock
/// credential-chain exhaustion ("no AWS credentials found: ...") through the
/// production `build_provider` boundary.
#[test]
fn build_provider_returns_auth_error_for_missing_credentials() {
    use opi_coding_agent::config::OpiConfig;
    use opi_coding_agent::provider_factory::{ProviderBuildError, build_provider};

    // require_api_key path: removing the single env var yields an Auth
    // diagnostic naming the env var + remediation.
    let require_key_cases: &[(&str, &[&str], /* expected env name */ &str)] = &[
        ("anthropic", &["ANTHROPIC_API_KEY"], "ANTHROPIC_API_KEY"),
        ("azure", &["AZURE_OPENAI_API_KEY"], "AZURE_OPENAI_API_KEY"),
        ("vertex", &["VERTEX_ACCESS_TOKEN"], "VERTEX_ACCESS_TOKEN"),
    ];
    for (id, remove, env_name) in require_key_cases.iter().copied() {
        with_env_managed(remove, &[], || {
            let mut config = OpiConfig::default();
            config.defaults.model = format!("{id}:test-model");
            // Azure/vertex also need endpoint/project+location to reach their
            // cred check, but require_api_key runs first so these are only
            // here to keep the failure strictly about credentials.
            if id == "azure" {
                config.providers.azure.endpoint = Some("https://test.openai.azure.com".into());
            } else if id == "vertex" {
                config.providers.vertex.project = Some("test-project".into());
                config.providers.vertex.location = Some("us-central1".into());
            }
            match build_provider(&config) {
                Err(ProviderBuildError::Auth(msg)) => {
                    assert!(
                        msg.contains(env_name),
                        "{id} Auth diagnostic should name {env_name}, got: {msg:?}"
                    );
                    assert!(
                        msg.contains("API key"),
                        "{id} Auth diagnostic should describe the missing key, got: {msg:?}"
                    );
                }
                Err(e) => panic!("{id} missing-cred should be Auth, got {e:?}"),
                Ok(p) => panic!(
                    "{id} missing-cred should be Auth, got Ok provider {}",
                    p.id()
                ),
            }
        });
    }

    // Bedrock credential-chain exhaustion: clear all AWS env sources and
    // redirect the shared credential/config file paths to nonexistent temp
    // paths so NO real ~/.aws is read. resolve_credentials returns None and
    // the factory surfaces the Auth remediation.
    let dir = tempfile::tempdir().unwrap();
    let ghost_credentials = dir.path().join("no-credentials-file");
    let ghost_config = dir.path().join("no-config-file");
    let ghost_credentials_str = ghost_credentials.to_string_lossy().into_owned();
    let ghost_config_str = ghost_config.to_string_lossy().into_owned();
    with_env_managed(
        &[
            "AWS_ACCESS_KEY_ID",
            "AWS_SECRET_ACCESS_KEY",
            "AWS_SESSION_TOKEN",
            "AWS_REGION",
            "AWS_DEFAULT_REGION",
            "AWS_PROFILE",
        ],
        &[
            ("AWS_SHARED_CREDENTIALS_FILE", &ghost_credentials_str),
            ("AWS_CONFIG_FILE", &ghost_config_str),
        ],
        || {
            let mut config = OpiConfig::default();
            config.defaults.model = "bedrock:test-model".into();
            match build_provider(&config) {
                Err(ProviderBuildError::Auth(msg)) => {
                    assert!(
                        msg.contains("no AWS credentials found"),
                        "bedrock Auth diagnostic should describe exhaustion, got: {msg:?}"
                    );
                    assert!(
                        msg.contains("AWS_ACCESS_KEY_ID"),
                        "bedrock Auth diagnostic should remediate with the env var, got: {msg:?}"
                    );
                }
                Err(e) => panic!("bedrock missing-cred should be Auth, got {e:?}"),
                Ok(_) => panic!("bedrock missing-cred should be Auth, got Ok provider"),
            }
        },
    );
}

/// Invalid endpoint/config surfaces a Config-class diagnostic at build time
/// with a remediation message. DoD: "invalid endpoints emit safe diagnostics
/// with provider/error class and remediation". Covers the openai_compatible
/// profile base_url requirement (runtime + listing arms), vertex missing
/// project/location, and azure no-deployments on the listing path.
#[test]
fn build_provider_returns_config_error_for_invalid_endpoint_config() {
    use opi_coding_agent::config::load_config_file;
    use opi_coding_agent::provider_factory::{
        ListModelsError, ProviderBuildError, build_collection_for_listing, build_provider,
    };

    // openai_compatible profile with an empty base_url is the only
    // config-loaded invalid-endpoint diagnostic. Both the runtime path
    // (build_provider) and the listing path (build_collection_for_listing)
    // must surface Config.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profile-empty-base-url.toml");
    let env_var = "OPI_TEST_PROFILE_EMPTY_BASE_URL_2E8C41";
    std::fs::write(
        &path,
        format!(
            r#"
[providers.openai_compatible.nobaseurl]
api_key_env = "{env_var}"
base_url = ""

[[providers.openai_compatible.nobaseurl.models]]
id = "m1"
display_name = "M1"
context_window = 128000
max_output_tokens = 4096
"#
        ),
    )
    .unwrap();
    with_env_var(env_var, "test-key", || {
        let config = load_config_file(&path).unwrap();

        let mut runtime_cfg = config.clone();
        runtime_cfg.defaults.model = "nobaseurl:m1".into();
        match build_provider(&runtime_cfg) {
            Err(ProviderBuildError::Config(msg)) => assert!(
                msg.contains("base_url"),
                "runtime empty-base_url Config should mention base_url, got: {msg:?}"
            ),
            Err(e) => panic!("runtime empty-base_url should be Config, got {e:?}"),
            Ok(_) => panic!("runtime empty-base_url should be Config, got Ok provider"),
        }

        match build_collection_for_listing(&config, &std::collections::HashMap::new()) {
            Err(ListModelsError::Config(msg)) => assert!(
                msg.contains("base_url"),
                "listing empty-base_url Config should mention base_url, got: {msg:?}"
            ),
            Err(e) => panic!("listing empty-base_url should be Config, got {e:?}"),
            Ok(_) => panic!("listing empty-base_url should be Config, got Ok collection"),
        }
    });

    // Vertex missing project / location on the runtime path -> Config.
    with_env_var("VERTEX_ACCESS_TOKEN", "test-token", || {
        let mut missing_project = opi_coding_agent::config::OpiConfig::default();
        missing_project.defaults.model = "vertex:test-model".into();
        missing_project.providers.vertex.location = Some("us-central1".into());
        match build_provider(&missing_project) {
            Err(ProviderBuildError::Config(msg)) => assert!(
                msg.contains("project"),
                "vertex missing-project Config should mention project, got: {msg:?}"
            ),
            Err(e) => panic!("vertex missing-project should be Config, got {e:?}"),
            Ok(_) => panic!("vertex missing-project should be Config, got Ok provider"),
        }

        let mut missing_location = opi_coding_agent::config::OpiConfig::default();
        missing_location.defaults.model = "vertex:test-model".into();
        missing_location.providers.vertex.project = Some("test-project".into());
        match build_provider(&missing_location) {
            Err(ProviderBuildError::Config(msg)) => assert!(
                msg.contains("location"),
                "vertex missing-location Config should mention location, got: {msg:?}"
            ),
            Err(e) => panic!("vertex missing-location should be Config, got {e:?}"),
            Ok(_) => panic!("vertex missing-location should be Config, got Ok provider"),
        }
    });

    // Azure deployments-empty on the listing path -> Config. Other families
    // are isolated (no creds) so azure is the only provider that does not
    // skip, deterministically reaching the deployments check.
    let dir2 = tempfile::tempdir().unwrap();
    let ghost_credentials = dir2.path().join("no-aws-credentials");
    let ghost_config = dir2.path().join("no-aws-config");
    let gc = ghost_credentials.to_string_lossy().into_owned();
    let gf = ghost_config.to_string_lossy().into_owned();
    with_env_managed(
        &[
            "ANTHROPIC_API_KEY",
            "OPENAI_API_KEY",
            "OPENROUTER_API_KEY",
            "MISTRAL_API_KEY",
            "GEMINI_API_KEY",
            "VERTEX_ACCESS_TOKEN",
            "AWS_ACCESS_KEY_ID",
            "AWS_SECRET_ACCESS_KEY",
            "AWS_SESSION_TOKEN",
            "AWS_REGION",
            "AWS_DEFAULT_REGION",
            "AWS_PROFILE",
        ],
        &[
            ("AZURE_OPENAI_API_KEY", "test-key"),
            ("AWS_SHARED_CREDENTIALS_FILE", &gc),
            ("AWS_CONFIG_FILE", &gf),
        ],
        || {
            let mut cfg = opi_coding_agent::config::OpiConfig::default();
            cfg.providers.azure.endpoint = Some("https://test.openai.azure.com".into());
            match build_collection_for_listing(&cfg, &std::collections::HashMap::new()) {
                Err(ListModelsError::Config(msg)) => assert!(
                    msg.contains("deployments"),
                    "azure no-deployments Config should mention deployments, got: {msg:?}"
                ),
                Err(e) => panic!("azure no-deployments listing should be Config, got {e:?}"),
                Ok(_) => panic!("azure no-deployments listing should be Config, got Ok collection"),
            }
        },
    );
}

/// Model-level profile overrides must win over the provider-level default
/// through the full config -> factory -> provider production path, observable
/// in the dispatched request body. DoD: "provider-level and model-level
/// profile overrides obey documented precedence". The existing
/// `model_level_override_takes_precedence_over_provider_profile` (opi-ai)
/// proves precedence at the helper-built provider; this proves
/// `build_provider` threads the per-model `ModelCompatOverride` from a parsed
/// TOML config all the way to the wire.
#[tokio::test]
async fn openai_compatible_profile_model_override_wins_through_config_path() {
    use futures_util::StreamExt;
    use opi_coding_agent::config::load_config_file;
    use opi_coding_agent::provider_factory::build_provider;
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    let env_var = "OPI_TEST_PROFILE_OVERRIDE_WINS_8D1F30";
    let _guard = EnvVarGuard::set(env_var, "test-key");

    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("profile-overrides.toml");
    // Profile default: system role "system", max-tokens field "max_tokens".
    // Model m1 overrides: system role "developer", max-tokens field
    // "max_completion_tokens" — these must win at the request body.
    std::fs::write(
        &config_path,
        format!(
            r#"
[defaults]
model = "prof:m1"

[providers.openai_compatible.prof]
api_key_env = "{env_var}"
base_url = "{base}"
system_role_override = "system"
max_tokens_field = "max_tokens"

[[providers.openai_compatible.prof.models]]
id = "m1"
display_name = "M1"
context_window = 128000
max_output_tokens = 4096
system_role_override = "developer"
max_tokens_field = "max_completion_tokens"
"#,
            base = server.uri()
        ),
    )
    .unwrap();

    // The dispatched body must carry the MODEL-level override winning over the
    // profile default: developer role (not system) + max_completion_tokens
    // (not max_tokens). If the factory failed to thread model_overrides, this
    // matcher would not match and server.verify() would fail.
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(body_partial_json(serde_json::json!({
            "messages": [{"role": "developer", "content": "sys"}],
            "max_completion_tokens": 1024,
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(
                    "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"},\"finish_reason\":null}]}\n\
                     data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\
                     data: [DONE]\n\n",
                )
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let config = load_config_file(&config_path).unwrap();
    let provider = build_provider(&config).expect("profile builds through the factory");

    let mut request = minimal_request("prof:m1");
    request.system = Some("sys".into());
    request.max_tokens = Some(1024);
    let mut stream = provider.stream(request);
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) if event.is_terminal() => break,
            Err(_) => break,
            _ => {}
        }
    }

    server.verify().await;
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serializes process-env mutation; awaited mapped streaming never re-acquires this lock.
async fn openai_compatible_profiles_lower_through_mapped_provider() {
    use futures_util::StreamExt;
    use opi_coding_agent::config::load_config_file;
    use opi_coding_agent::provider_factory::build_provider;

    let _env_guard = ENV_MUTEX.lock().expect("env lock");
    let env_var = "OPI_TEST_OPENAI_COMPAT_MAPPED_1416";
    let _guard = EnvVarGuard::set(env_var, "test-key");
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("mapped-profile.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
[defaults]
model = "prof:m1"

[providers.openai_compatible.prof]
api_key_env = "{env_var}"
base_url = "http://127.0.0.1:9"

[[providers.openai_compatible.prof.models]]
id = "m1"
display_name = "M1"
context_window = 128000
max_output_tokens = 4096
"#
        ),
    )
    .unwrap();
    let config = load_config_file(&config_path).unwrap();
    let provider = build_provider(&config).unwrap();
    let model_count_before_unknown_request = provider.models().len();
    let error = provider
        .stream(minimal_request("prof:unknown"))
        .next()
        .await
        .unwrap()
        .unwrap_err();
    assert!(matches!(
        error,
        opi_ai::provider::ProviderError::UnknownModel {
            ref provider_id,
            ref model_id
        } if provider_id == "prof" && model_id == "unknown"
    ));
    assert_eq!(provider.id(), "prof");
    assert_eq!(model_count_before_unknown_request, 1);
}

/// The harness model-lookup collection is built via
/// `ProviderCollection::from_registry` with NO auth descriptors, distinct from
/// `build_collection_for_listing`'s register-with-AuthDescriptor listing path.
/// DoD: "assembles harness model lookup through from_registry without claiming
/// auth-gated runtime dispatch". The existing
/// `metadata_only_provider_dispatch_returns_explicit_error` asserts dispatch
/// behavior (which would be unchanged by a `new`+`register` rewrite); this
/// pins the from_registry seam by asserting NO auth descriptor is attached
/// while the active provider's models still resolve.
#[test]
fn assemble_harness_collection_uses_from_registry_seam() {
    use opi_coding_agent::provider_factory::assemble_harness_collection;

    let provider = MockProvider::new("mock-active", vec![]);
    let (collection, diagnostics) = assemble_harness_collection(&provider, None);
    assert!(diagnostics.is_empty());

    // from_registry path: the active provider contributes metadata but NO
    // auth descriptor (the listing path attaches Resolved descriptors).
    assert!(
        collection.auth_descriptor("mock-active").is_none(),
        "from_registry collection must not attach an auth descriptor for the active provider"
    );
    assert!(
        collection.auth_status("mock-active").is_none(),
        "from_registry collection must not attach auth status for the active provider"
    );

    // The active provider's metadata IS resolvable, proving the harness model
    // lookup reaches the registry that from_registry wraps.
    assert!(
        collection.provider_ids().contains(&"mock-active"),
        "active provider metadata should be registered in the harness collection"
    );
    let (_resolved_provider, model) = collection
        .resolve("mock-active:mock-model")
        .expect("active provider model resolves through the from_registry collection");
    assert_eq!(model.id, "mock-model");
}

/// Strip Rust line and nested block comments while preserving string/char
/// literal contents, so documentation comments do not trip source scans.
fn strip_rust_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'/' && i + 1 < bytes.len() {
            if bytes[i + 1] == b'/' {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            } else if bytes[i + 1] == b'*' {
                let mut depth = 1;
                i += 2;
                while i < bytes.len() && depth > 0 {
                    if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                        depth += 1;
                        i += 2;
                    } else if bytes[i] == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                        depth -= 1;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                continue;
            }
        }
        if c == b'"' || c == b'\'' {
            let quote = c;
            out.push(c as char);
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    out.push(bytes[i] as char);
                    out.push(bytes[i + 1] as char);
                    i += 2;
                    continue;
                }
                out.push(bytes[i] as char);
                if bytes[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        out.push(c as char);
        i += 1;
    }
    out
}

/// Phase 10.2 centralization contract: every provider/model/auth
/// construction-policy symbol in `src/` lives only in `provider_factory.rs`.
/// Includes a vacuous-allowlist guard so the test cannot pass with an empty or
/// stale token set.
#[test]
fn provider_policy_is_centralized() {
    use std::fs;
    use std::path::{Path, PathBuf};

    fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).expect("read src dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                collect_rs(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }

    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let allow = "provider_factory.rs";

    let tokens = [
        "ProviderRegistry::new",
        "ProviderCollection::from_registry",
        "ProviderCollection::new",
        "fn parse_model_spec",
        "fn build_provider",
        "fn build_runtime_provider",
        "fn build_list_models_metadata",
        "fn listing_auth_available",
        "fn openai_compatible_model_catalog",
        "fn build_runtime_openai_compatible_profile",
        "fn build_openai_compatible_profile",
        "fn build_collection_for_listing",
        "fn build_collection_for_listing_with_store",
        "fn assemble_harness_collection",
        "fn require_api_key",
        "fn non_empty_env_var",
        "fn resolve_env_name",
        "fn resolve_bedrock_env_credentials",
        "fn aws_credentials_path",
        "fn aws_config_path",
        "fn aws_home_dir",
        "fn profile_api_key_env_default",
        "fn auth_descriptor_for",
        "fn resolved_auth_descriptor_for",
        "fn resolved_auth_descriptor_for_profile",
        "fn auth_descriptor_for_profile",
        "fn compat_metadata_for",
        "struct MetadataProvider",
        "BUILT_IN_PROVIDER_IDS",
    ];

    let mut files = Vec::new();
    collect_rs(&src_dir, &mut files);

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        let rel = file
            .strip_prefix(&src_dir)
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");
        let text = strip_rust_comments(&fs::read_to_string(file).unwrap_or_default());
        for token in tokens {
            if text.contains(token) && rel != allow {
                violations.push(format!("token `{token}` appears in `{rel}`"));
            }
        }
    }

    // Vacuous-allowlist guard: provider_factory.rs must contain every token,
    // otherwise the centralization test would pass trivially.
    let factory_text = strip_rust_comments(
        &fs::read_to_string(src_dir.join(allow)).expect("provider_factory.rs exists"),
    );
    let missing: Vec<&str> = tokens
        .iter()
        .filter(|t| !factory_text.contains(*t))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "provider_factory.rs is missing centralized tokens {missing:?} (allowlist is vacuous)"
    );

    assert!(
        violations.is_empty(),
        "provider construction policy is not centralized:\n{}",
        violations.join("\n")
    );
}
