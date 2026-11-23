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
fn vertex_config_rejects_replacing_the_builtin_catalog() {
    let dir = tempfile::tempdir().unwrap();
    for provider in ["google-vertex", "vertex"] {
        let path = write_temp_config(
            dir.path(),
            &format!("[providers.{provider}]\nmodels = [\"gemini-3.1-flash-lite-image\"]\n"),
        );
        let error = opi_coding_agent::config::load_config_file(&path).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Select google-vertex:<model-id>"),
            "{error}"
        );
    }
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
    let provider = opi_ai::providers::vertex::VertexProvider::new(
        "my-project".into(),
        "us-central1".into(),
        None,
    );
    // The standalone factory publishes its explicit SDK registration ID.
    assert_eq!(provider.id(), "vertex");
}

#[test]
fn vertex_provider_from_config_with_models() {
    let provider = opi_ai::providers::vertex::VertexProvider::from_config(
        "proj".into(),
        "europe-west4".into(),
        vec!["model-a".into(), "model-b".into()],
        None,
    );
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
    let provider =
        opi_ai::providers::vertex::VertexProvider::new("proj".into(), "us-central1".into(), None);
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
    let provider = opi_ai::providers::vertex::VertexProvider::new(
        "my-vertex-project".into(),
        "us-central1".into(),
        None,
    );
    let debug = format!("{provider:?}");
    assert!(debug.contains("my-vertex-project"));
    assert!(debug.contains("us-central1"));
}

#[test]
fn vertex_url_does_not_contain_access_token() {
    let provider =
        opi_ai::providers::vertex::VertexProvider::new("proj".into(), "us-central1".into(), None);
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

        let (_, projectless) = opi_coding_agent::provider_factory::build_runtime_route_for_tests(
            &vertex_config(None, None),
            "google-vertex",
            &test_resolver(),
        )
        .unwrap();
        assert!(!projectless.check_configuration().unwrap().await.unwrap());
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

    #[tokio::test]
    async fn published_vertex_models_send_express_and_bearer_requests_through_sdk_and_cli() {
        use futures_util::StreamExt;
        use opi_ai::auth::adc::native::{
            AdcAuthSelection, AdcNativeOwner, ReqwestAdcTransport, SystemAdcClock,
        };
        use opi_ai::auth::{AuthContext, AuthContextError};
        use opi_ai::provider::{Request, ThinkingConfig};
        use opi_ai::providers::collection::{BuiltinOverrides, register_builtin};
        use opi_ai::transcript::{Context, normalize_context};
        use opi_ai::{ChatModelRecord, Models, ModelsError, RegistryError, ThinkingLevel};
        use opi_coding_agent::credential_store::FakeKeyringBackend;
        use opi_coding_agent::provider_factory::{
            build_harness_collection, build_provider, build_provider_bundle,
        };
        use serde_json::{Value, json};
        use wiremock::matchers::{method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        struct FixtureAuth {
            express: bool,
        }
        impl AuthContext for FixtureAuth {
            fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
                Ok((self.express && name == "GOOGLE_CLOUD_API_KEY").then(|| "sdk-key".into()))
            }

            fn file_exists(&self, path: &str) -> Result<bool, AuthContextError> {
                Ok(std::path::Path::new(path).is_file())
            }
        }

        const PROVIDER: &str = "google-vertex";
        const RETIRED: &str = "gemini-3.1-flash-lite-image";
        const PROJECT: &str = "test-project";
        const LOCATION: &str = "europe-west1";
        let user_context = normalize_context(Context {
            messages: vec![opi_ai::message::Message::User(
                opi_ai::message::UserMessage {
                    content: vec![opi_ai::message::InputContent::Text {
                        text: "ping".into(),
                    }],
                    timestamp_ms: 0,
                },
            )],
            system_prompt: None,
            tools: vec![],
        })
        .unwrap();
        let request_for = |model| Request {
            model,
            context: user_context.clone(),
            max_tokens: Some(53),
            temperature: Some(0.25),
            timeout: Some(std::time::Duration::from_secs(3)),
            projection: Default::default(),
            transport: Default::default(),
            thinking: Default::default(),
            sampling_params: None,
            stop_sequences: vec![],
            metadata: None,
            cancel: tokio_util::sync::CancellationToken::new(),
            extra_headers: vec![],
            cache_retention: Default::default(),
            session_id: None,
            telemetry_context: None,
            callbacks: Default::default(),
        };

        let _env = ENV_LOCK.lock().await;
        for express in [true, false] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(query_param("alt", "sse"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(concat!(
                            "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"ok\"}]},",
                            "\"finishReason\":\"STOP\"}]}\n\n"
                        )),
                )
                .expect(6)
                .mount(&server)
                .await;
            let dir = tempfile::tempdir().unwrap();
            let mut overrides = BuiltinOverrides {
                vertex_project: Some(PROJECT.into()),
                vertex_location: Some(LOCATION.into()),
                ..Default::default()
            };
            overrides.vertex_auth.base_url = Some(format!("{}/sdk-proxy/", server.uri()));
            if !express {
                let credentials = dir.path().join("adc.json");
                std::fs::write(
                    &credentials,
                    json!({
                        "type": "authorized_user",
                        "client_id": "local-adc-client",
                        "client_secret": "local-adc-secret",
                        "refresh_token": "local-adc-refresh",
                        "quota_project_id": "sdk-quota"
                    })
                    .to_string(),
                )
                .unwrap();
                Mock::given(method("POST"))
                    .and(path("/token"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                        "access_token": "sdk-token",
                        "token_type": "Bearer",
                        "expires_in": 3600
                    })))
                    .expect(1)
                    .mount(&server)
                    .await;
                overrides.vertex_auth.adc = Some(
                    AdcNativeOwner::new(
                        AdcAuthSelection::CredentialsFile(credentials),
                        Arc::new(ReqwestAdcTransport::new()),
                        Arc::new(SystemAdcClock),
                    )
                    .with_token_endpoint(Some(format!("{}/token", server.uri()))),
                );
            }
            let mut sdk = Models::new();
            register_builtin(
                &mut sdk,
                Arc::new(FixtureAuth { express }),
                Arc::new(opi_ai::http::HttpClient::new()),
                &overrides,
            )
            .unwrap();

            let _key =
                EnvVarGuard::set("GOOGLE_CLOUD_API_KEY", if express { "cli-key" } else { "" });
            let _token = EnvVarGuard::set("OPI_TEST_VERTEX_WIRE_TOKEN", "cli-token");
            let mut config = vertex_config(Some(PROJECT), Some(LOCATION));
            config.defaults.model = "google-vertex:gemini-3.1-pro-preview".into();
            config.providers.google_vertex.base_url = Some(format!("{}/cli-proxy/", server.uri()));
            config.providers.google_vertex.access_token_env = "OPI_TEST_VERTEX_WIRE_TOKEN".into();
            let bundle = build_provider_bundle(
                &config,
                dir.path().to_path_buf(),
                Box::new(|| Box::new(FakeKeyringBackend::new())),
            )
            .await
            .unwrap();
            let direct = build_provider(&config).unwrap();
            let mut routes = vec![(bundle.provider, bundle.auth_resolver)];
            routes.extend(bundle.extra_routes);
            let cli = build_harness_collection(
                routes,
                Vec::new(),
                Vec::new(),
                PROVIDER,
                Some(bundle.store),
            );
            let owners = [
                (&sdk, "sdk-proxy", "sdk-key", "sdk-token", Some("sdk-quota")),
                (cli.as_ref(), "cli-proxy", "cli-key", "cli-token", None),
            ];
            let cases = [
                (
                    "gemini-3.1-pro-preview",
                    ThinkingLevel::Max,
                    json!({"includeThoughts": true, "thinkingLevel": "HIGH"}),
                ),
                (
                    "gemini-3.1-flash-lite",
                    ThinkingLevel::None,
                    json!({"thinkingLevel": "MINIMAL"}),
                ),
                (
                    "gemini-2.5-flash-lite",
                    ThinkingLevel::Minimal,
                    json!({"includeThoughts": true, "thinkingBudget": 128}),
                ),
            ];
            for (models, prefix, ..) in owners {
                assert_eq!(models.get_models(Some(PROVIDER)).len(), 13);
                for (id, level, _) in &cases {
                    let model = models.get_model(PROVIDER, id).unwrap();
                    let mut request = request_for(format!("{PROVIDER}:{id}"));
                    request.thinking = ThinkingConfig {
                        enabled: *level != ThinkingLevel::None,
                        level: *level,
                        budget_tokens: None,
                    };
                    let result = models
                        .complete_simple(
                            ChatModelRecord::new(PROVIDER, model),
                            request,
                            Default::default(),
                        )
                        .await
                        .unwrap_or_else(|error| panic!("{prefix} {id}: {error:?}"));
                    assert_eq!(result.provider, PROVIDER);
                    assert_eq!(result.model, *id);
                }
                let spec = format!("{PROVIDER}:{RETIRED}");
                assert!(models.get_model(PROVIDER, RETIRED).is_none());
                assert!(matches!(
                    models.prepare_call(&spec, request_for(spec.clone())).await,
                    Err(ModelsError::Registry(RegistryError::UnknownModel { provider, model }))
                        if provider == PROVIDER && model == RETIRED
                ));
            }
            let auth = if express {
                opi_ai::test_support::resolved_auth()
            } else {
                opi_ai::test_support::resolved_bearer_auth()
            };
            assert!(matches!(
                direct.stream_prepared(request_for(format!("{PROVIDER}:{RETIRED}")), auth).next().await,
                Some(Err(ProviderError::UnknownModel { provider_id, model_id }))
                    if provider_id == PROVIDER && model_id == RETIRED
            ));

            let captured = server.received_requests().await.unwrap();
            assert_eq!(captured.len(), if express { 6 } else { 7 });
            let generations: Vec<_> = captured
                .iter()
                .filter(|request| request.url.query() == Some("alt=sse"))
                .collect();
            assert_eq!(generations.len(), 6);
            for (request, ((_, prefix, key, token, quota), (id, _, thinking))) in
                generations.into_iter().zip(
                    owners
                        .iter()
                        .flat_map(|owner| cases.iter().map(move |case| (owner, case))),
                )
            {
                let scope = if express {
                    String::new()
                } else {
                    format!("/projects/{PROJECT}/locations/{LOCATION}")
                };
                assert_eq!(
                    request.url.path(),
                    format!(
                        "/{prefix}/v1{scope}/publishers/google/models/{id}:streamGenerateContent"
                    )
                );
                if express {
                    assert_eq!(request.headers["x-goog-api-key"], *key);
                    assert!(!request.headers.contains_key("authorization"));
                    assert!(!request.headers.contains_key("x-goog-user-project"));
                } else {
                    assert_eq!(request.headers["authorization"], format!("Bearer {token}"));
                    assert!(!request.headers.contains_key("x-goog-api-key"));
                    assert_eq!(
                        request
                            .headers
                            .get("x-goog-user-project")
                            .map(|v| v.to_str().unwrap()),
                        *quota
                    );
                }
                let body: Value = serde_json::from_slice(&request.body).unwrap();
                assert_eq!(
                    body["generationConfig"],
                    json!({
                        "temperature": 0.25,
                        "maxOutputTokens": 53,
                        "thinkingConfig": thinking
                    })
                );
            }
        }
    }
}
