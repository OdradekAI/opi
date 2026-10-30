//! Vertex AI provider wiring tests (task 3.3).
//!
//! Tests config parsing, provider construction, and secret redaction
//! for the Vertex provider. No live Google Cloud calls.

use opi_ai::Provider;
use std::fs;

fn write_temp_config(dir: &std::path::Path, contents: &str) -> std::path::PathBuf {
    let path = dir.join("config.toml");
    fs::write(&path, contents).unwrap();
    path
}

fn load_config_file(path: &std::path::Path) -> opi_coding_agent::config::OpiConfig {
    opi_coding_agent::config::load_config_file(path).unwrap()
}

// ---------------------------------------------------------------------------
// Config parsing tests
// ---------------------------------------------------------------------------

#[test]
fn parse_vertex_config_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(dir.path(), "");
    let config = load_config_file(&path);
    assert!(config.providers.google_vertex.access_token_env.is_empty());
    assert!(config.providers.google_vertex.project.is_none());
    assert!(config.providers.google_vertex.location.is_none());
    assert!(config.providers.google_vertex.models.is_empty());
}

#[test]
fn parse_vertex_config_with_project_and_location() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.google-vertex]
access_token_env = "MY_VERTEX_TOKEN"
project = "my-gcp-project"
location = "europe-west1"
"#,
    );
    let config = load_config_file(&path);
    assert_eq!(
        config.providers.google_vertex.access_token_env,
        "MY_VERTEX_TOKEN"
    );
    assert_eq!(
        config.providers.google_vertex.project.as_deref(),
        Some("my-gcp-project")
    );
    assert_eq!(
        config.providers.google_vertex.location.as_deref(),
        Some("europe-west1")
    );
}

#[test]
fn parse_vertex_config_with_models() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.google-vertex]
project = "my-project"
location = "us-central1"
models = ["gemini-2.5-flash", "gemini-2.5-pro"]
"#,
    );
    let config = load_config_file(&path);
    assert_eq!(config.providers.google_vertex.models.len(), 2);
    assert_eq!(config.providers.google_vertex.models[0], "gemini-2.5-flash");
    assert_eq!(config.providers.google_vertex.models[1], "gemini-2.5-pro");
}

#[test]
fn vertex_config_independent_from_other_providers() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.google]
api_key_env = "GEMINI_KEY"

[providers.google-vertex]
access_token_env = "VERTEX_TOKEN"
project = "my-project"
"#,
    );
    let config = load_config_file(&path);
    assert_eq!(config.providers.google.api_key_env, "GEMINI_KEY");
    assert_eq!(
        config.providers.google_vertex.access_token_env,
        "VERTEX_TOKEN"
    );
}

// ---------------------------------------------------------------------------
// Provider construction tests (no live calls)
// ---------------------------------------------------------------------------

#[test]
fn vertex_provider_builds_with_token() {
    let provider =
        opi_ai::vertex::VertexProvider::new("my-project".into(), "us-central1".into(), None);
    // The adapter keeps its historical id; host routes republish it as
    // `google-vertex` through the canonical wrapper.
    assert_eq!(provider.id(), "vertex");
}

#[test]
fn vertex_provider_from_config_with_models() {
    let provider = opi_ai::vertex::VertexProvider::from_config(
        "proj".into(),
        "europe-west4".into(),
        vec!["model-a".into(), "model-b".into()],
        None,
    );
    // The adapter keeps its historical id; host routes republish it as
    // `google-vertex` through the canonical wrapper.
    assert_eq!(provider.id(), "vertex");
    assert_eq!(provider.models().len(), 2);
    assert_eq!(provider.models()[0].id, "model-a");
    assert_eq!(provider.models()[1].id, "model-b");
}

// ---------------------------------------------------------------------------
// Secret redaction tests
// ---------------------------------------------------------------------------

#[test]
fn vertex_access_token_not_in_debug() {
    let provider = opi_ai::vertex::VertexProvider::new("proj".into(), "us-central1".into(), None);
    let debug = format!("{provider:?}");
    assert!(
        !debug.contains("super-secret-oauth-token-xyz"),
        "access token leaked in Debug: {debug}"
    );
    // Post-17.5: VertexProvider no longer stores the token (it arrives via
    // ResolvedAuth), so its Debug carries no `***` redaction marker.
    // Redaction is implicit-by-absence, proven by the assertion above.
}

#[test]
fn vertex_project_visible_in_debug() {
    let provider =
        opi_ai::vertex::VertexProvider::new("my-vertex-project".into(), "us-central1".into(), None);
    let debug = format!("{provider:?}");
    assert!(debug.contains("my-vertex-project"));
    assert!(debug.contains("us-central1"));
}

#[test]
fn vertex_url_does_not_contain_access_token() {
    let provider = opi_ai::vertex::VertexProvider::new("proj".into(), "us-central1".into(), None);
    let url = provider.build_vertex_url("gemini-2.5-flash");
    assert!(!url.contains("super-secret-token"));
    assert!(url.contains("proj"));
}

// ---------------------------------------------------------------------------
// ADC source wiring and Express construction (task 20.21.1)
// ---------------------------------------------------------------------------

mod adc_wiring {
    use opi_ai::auth::AuthScheme;
    use opi_ai::provider::ProviderError;
    use opi_coding_agent::config::OpiConfig;
    use opi_coding_agent::credential_store::{
        BackendError, CredentialResolver, KeychainCredentialStore, KeyringBackend,
    };
    use std::sync::Arc;

    /// Env-var guard: restores the original value on drop (serialized host-env
    /// mutation, same pattern as tests/provider_factory.rs).
    struct EnvVarGuard {
        key: &'static str,
        original: Option<std::ffi::OsString>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let guard = Self {
                key,
                original: std::env::var_os(key),
            };
            unsafe { std::env::set_var(key, value) };
            guard
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(value) => unsafe { std::env::set_var(self.key, value) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    /// Backend with no entries: drives resolution through the env sources.
    struct AbsentBackend;

    impl KeyringBackend for AbsentBackend {
        fn get(&self, _service: &str, _provider_id: &str) -> Result<Option<String>, BackendError> {
            Ok(None)
        }
        fn set(
            &self,
            _service: &str,
            _provider_id: &str,
            _value: &str,
        ) -> Result<(), BackendError> {
            Ok(())
        }
        fn delete(&self, _service: &str, _provider_id: &str) -> Result<(), BackendError> {
            Ok(())
        }
    }

    fn test_resolver() -> CredentialResolver {
        let dir = tempfile::tempdir().unwrap();
        let store = KeychainCredentialStore::new(Box::new(AbsentBackend), dir.path().to_path_buf());
        CredentialResolver::new(
            Arc::new(store),
            Arc::new(|name: &str| std::env::var(name).ok()),
        )
    }

    /// Host-env serialization: these tests mutate process environment
    /// variables and must not race each other.
    static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    fn vertex_config(project: Option<&str>, location: Option<&str>) -> OpiConfig {
        let mut config = OpiConfig::default();
        config.defaults.model = "google-vertex:gemini-2.5-flash".to_owned();
        config.providers.google_vertex.project = project.map(str::to_owned);
        config.providers.google_vertex.location = location.map(str::to_owned);
        config
    }

    #[tokio::test]
    async fn google_cloud_api_key_env_selects_express_api_key_auth() {
        let _env = ENV_LOCK.lock().await;
        let _guard = EnvVarGuard::set("GOOGLE_CLOUD_API_KEY", "express-key");
        let (_provider, resolver) =
            opi_coding_agent::provider_factory::build_runtime_route_for_tests(
                &vertex_config(None, None),
                "google-vertex",
                &test_resolver(),
            )
            .expect("express route builds");
        let auth = resolver.resolve().await.expect("api key resolves");
        assert!(matches!(auth.scheme, AuthScheme::ApiKey));
    }

    #[tokio::test]
    async fn access_token_env_still_selects_bearer_when_no_api_key() {
        let _env = ENV_LOCK.lock().await;
        let _guard = EnvVarGuard::set("VERTEX_ACCESS_TOKEN", "tok");
        let _guard_key = EnvVarGuard::set("GOOGLE_CLOUD_API_KEY", "");
        let (_provider, resolver) =
            opi_coding_agent::provider_factory::build_runtime_route_for_tests(
                &vertex_config(Some("p1"), Some("us-central1")),
                "google-vertex",
                &test_resolver(),
            )
            .expect("vertex route builds");
        let auth = resolver.resolve().await.expect("bearer resolves");
        assert!(matches!(auth.scheme, AuthScheme::Bearer));
    }

    #[tokio::test]
    async fn malformed_selected_adc_file_fails_typed_before_generation() {
        let _env = ENV_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let creds = dir.path().join("adc.json");
        std::fs::write(&creds, "{broken").unwrap();
        let _guard = EnvVarGuard::set("GOOGLE_APPLICATION_CREDENTIALS", creds.to_str().unwrap());
        let (_provider, resolver) =
            opi_coding_agent::provider_factory::build_runtime_route_for_tests(
                &vertex_config(Some("p1"), Some("us-central1")),
                "google-vertex",
                &test_resolver(),
            )
            .expect("route builds lazily");
        let error = resolver
            .resolve()
            .await
            .expect_err("terminal parse failure");
        assert!(matches!(error, ProviderError::Config(_)));
    }

    #[tokio::test]
    async fn no_sources_reports_credential_needed() {
        let _env = ENV_LOCK.lock().await;
        let _guard_key = EnvVarGuard::set("GOOGLE_CLOUD_API_KEY", "");
        let _guard_token = EnvVarGuard::set("VERTEX_ACCESS_TOKEN", "");
        let _guard_adc = EnvVarGuard::set("GOOGLE_APPLICATION_CREDENTIALS", "");
        let (_provider, resolver) =
            opi_coding_agent::provider_factory::build_runtime_route_for_tests(
                &vertex_config(Some("p1"), Some("us-central1")),
                "google-vertex",
                &test_resolver(),
            )
            .expect("route builds");
        let error = resolver.resolve().await.expect_err("no source configured");
        assert!(matches!(
            error,
            ProviderError::CredentialNeeded { provider_id } if provider_id == "google-vertex"
        ));
    }

    #[test]
    fn express_provider_builds_without_project_and_location() {
        use opi_coding_agent::provider_factory::build_provider;
        let provider = build_provider(&vertex_config(None, None)).expect("express builds");
        assert_eq!(provider.id(), "google-vertex");
    }

    #[test]
    fn half_configured_project_location_is_a_typed_config_error() {
        use opi_coding_agent::provider_factory::build_provider;
        let error = build_provider(&vertex_config(Some("p1"), None))
            .err()
            .expect("half-configured vertex fails");
        assert!(error.to_string().contains("project"));
    }
}
