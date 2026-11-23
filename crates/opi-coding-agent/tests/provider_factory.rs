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
        projection: Default::default(),
        transport: Default::default(),
        model: model.into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

// ---------------------------------------------------------------------------
// Provider construction: correct id() per provider
// ---------------------------------------------------------------------------

#[test]
fn anthropic_provider_construction() {
    let provider = opi_ai::providers::anthropic::AnthropicProvider::new(None);
    assert_eq!(provider.id(), "anthropic");
}

#[test]
fn openai_provider_construction() {
    let provider = opi_ai::providers::openai::OpenAiChatProvider::new(None);
    assert_eq!(provider.id(), "openai");
}

#[test]
fn openrouter_provider_construction() {
    let provider = opi_ai::providers::openrouter::openrouter_provider(None);
    assert_eq!(provider.id(), "openrouter");
}

#[test]
fn mistral_provider_construction() {
    let provider = opi_ai::providers::mistral::mistral_provider(None);
    assert_eq!(provider.id(), "mistral");
}

#[test]
fn openai_responses_provider_construction() {
    let provider = opi_ai::providers::openai_responses::OpenAiResponsesProvider::new(None);
    assert_eq!(provider.id(), "openai-responses");
}

#[test]
fn gemini_provider_construction() {
    let provider = opi_ai::providers::gemini::GeminiProvider::new(None);
    // The adapter keeps its historical id; host routes republish it as
    // `google` through the canonical wrapper.
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
    let provider = opi_ai::providers::anthropic::AnthropicProvider::new(None);
    assert_same_models(
        provider.models(),
        &opi_ai::providers::anthropic::model_catalog(),
    );
}

#[test]
fn openai_chat_model_catalog_matches_constructor() {
    let provider = opi_ai::providers::openai::OpenAiChatProvider::new(None);
    assert_same_models(
        provider.models(),
        &opi_ai::providers::openai::model_catalog(),
    );
}

#[test]
fn openai_responses_model_catalog_matches_constructor() {
    let provider = opi_ai::providers::openai_responses::OpenAiResponsesProvider::new(None);
    assert_same_models(
        provider.models(),
        &opi_ai::providers::openai_responses::model_catalog(),
    );
}

#[tokio::test]
async fn openai_builtin_catalog_dispatches_released_models_and_tool_capabilities() {
    use opi_ai::auth::{AuthContext, AuthContextError};
    use opi_ai::catalog::{Catalog, CatalogOperation};
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    use opi_ai::credential::{Credential, CredentialStore};
    use opi_ai::message::{InputContent, Message, ToolDef, UserMessage};
    use opi_ai::providers::collection::{BuiltinOverrides, register_builtin};
    use opi_ai::transcript::{Context, SystemMessage, normalize_context};
    use opi_ai::{Models, ModelsError, RegistryError};
    use opi_coding_agent::credential_store::FakeKeyringBackend;
    use opi_coding_agent::provider_factory::{build_harness_collection, build_provider_bundle};
    use serde_json::{Value, json};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    struct FixtureAuth;
    impl AuthContext for FixtureAuth {
        fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
            Ok((name == "OPENAI_API_KEY").then(|| "sk-sdk-fixture".into()))
        }

        fn file_exists(&self, _: &str) -> Result<bool, AuthContextError> {
            Ok(false)
        }
    }

    let server = MockServer::start().await;
    let api_base = format!("{}/v1", server.uri());
    let mut sdk = Models::new();
    let mut overrides = BuiltinOverrides::default();
    overrides
        .base_urls
        .insert("openai".into(), api_base.clone());
    register_builtin(
        &mut sdk,
        Arc::new(FixtureAuth),
        Arc::new(opi_ai::http::HttpClient::new()),
        &overrides,
    )
    .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let mut config = OpiConfig::default();
    config.defaults.model = "openai:chat-latest".into();
    config.providers.openai.base_url = Some(server.uri());
    config.providers.openai_responses.base_url = Some(format!("{api_base}/"));
    let bundle = build_provider_bundle(
        &config,
        dir.path().to_path_buf(),
        Box::new(|| Box::new(FakeKeyringBackend::new())),
    )
    .await
    .unwrap();
    for provider in ["openai", "openai-responses"] {
        bundle
            .store
            .write(provider, &Credential::ApiKey("sk-cli-fixture".into()))
            .await
            .unwrap();
    }
    let mut routes = vec![(bundle.provider, bundle.auth_resolver)];
    routes.extend(bundle.extra_routes);
    let runtime =
        build_harness_collection(routes, Vec::new(), Vec::new(), "openai", Some(bundle.store));

    let catalog = Catalog::builtin().unwrap();
    assert_eq!(
        catalog
            .models("openai", CatalogOperation::Classifier)
            .unwrap()
            .count(),
        1
    );
    sdk.classifier_model("openai", "gpt-6-luna").unwrap();
    let published: Vec<_> = catalog
        .models("openai", CatalogOperation::Chat)
        .unwrap()
        .map(|record| {
            record
                .to_chat_model_info()
                .unwrap()
                .with_base_url(api_base.clone())
        })
        .collect();
    assert_eq!(published.len(), 44);
    let routes = [
        (&sdk, "openai", "Bearer sk-sdk-fixture"),
        (runtime.as_ref(), "openai", "Bearer sk-cli-fixture"),
        (
            runtime.as_ref(),
            "openai-responses",
            "Bearer sk-cli-fixture",
        ),
    ];
    for (models, provider, _) in routes {
        assert_eq!(models.get_models(Some(provider)), published);
        for retired in [
            "gpt-5.2-chat-latest",
            "gpt-5.3-chat-latest",
            "gpt-realtime-2.1",
        ] {
            let spec = format!("{provider}:{retired}");
            assert!(models.get_model(provider, retired).is_none());
            assert!(matches!(
                models.prepare_call(&spec, minimal_request(&spec)).await,
                Err(ModelsError::Registry(RegistryError::UnknownModel { provider: actual, model }))
                    if actual == provider && model == retired
            ));
        }
    }
    assert!(server.received_requests().await.unwrap().is_empty());

    let cases = [
        ("chat-latest", false),
        ("gpt-5.6-cyber", false),
        ("gpt-6.1-terra", true),
        ("gpt-5.4", true),
        ("gpt-5.4-mini", true),
        ("gpt-5.4-pro", true),
        ("gpt-5.5", true),
    ];
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"fixture\",\"status\":\"completed\",\"output\":[]}}\n\n",
            "text/event-stream",
        ))
        .expect((routes.len() * cases.len()) as u64)
        .mount(&server)
        .await;

    let user = |text: &str| {
        Message::User(UserMessage {
            content: vec![InputContent::Text { text: text.into() }],
            timestamp_ms: 0,
        })
    };
    let strict = ToolDef {
        name: "lookup".into(),
        description: "Look up a path".into(),
        input_schema: json!({"type":"object","properties":{"path":{"type":"string"}}}),
        constrained_sampling: Some(ConstrainedSampling::JsonSchema {
            strict: StrictPreference::Require,
        }),
    };
    let grammar = ToolDef {
        name: "parse".into(),
        description: "Parse an expression".into(),
        input_schema: json!({"type":"object","properties":{"input":{"type":"string"}},"required":["input"]}),
        constrained_sampling: Some(ConstrainedSampling::Grammar {
            openai_lark: Some("start: \"hello\"".into()),
            openai_regex: None,
        }),
    };
    let tool_context = normalize_context(Context {
        system_prompt: Some("Initial instructions".into()),
        tools: vec![strict],
        messages: vec![
            user("before"),
            Message::System(SystemMessage {
                content: "Later instruction".into(),
                tools_added: vec![grammar],
                ..Default::default()
            }),
            user("after"),
        ],
    })
    .unwrap();
    let text_context = normalize_context(Context {
        messages: vec![user("hello")],
        ..Default::default()
    })
    .unwrap();

    for (models, provider, _) in routes {
        for (id, with_tools) in cases {
            let spec = format!("{provider}:{id}");
            let mut request = minimal_request(&spec);
            request.timeout = Some(std::time::Duration::from_secs(5));
            request.context = if with_tools {
                tool_context.clone()
            } else {
                text_context.clone()
            };
            let result = models
                .prepare_call(&spec, request)
                .await
                .unwrap()
                .start_attempt_with_telemetry()
                .unwrap()
                .result()
                .await
                .unwrap();
            assert_eq!(
                result.stop_reason,
                opi_ai::stream::StopReason::Stop,
                "{spec}"
            );
        }
    }

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), routes.len() * cases.len());
    for ((_, _, bearer), group) in routes.into_iter().zip(sent.chunks_exact(cases.len())) {
        for ((id, with_tools), request) in cases.into_iter().zip(group) {
            assert_eq!(request.url.path(), "/v1/responses");
            assert_eq!(request.headers["authorization"], bearer);
            let body: Value = serde_json::from_slice(&request.body).unwrap();
            assert_eq!(body["model"], id);
            assert_eq!(body["stream"], true);
            if !with_tools {
                continue;
            }
            let input = body["input"].as_array().unwrap();
            let tools = body["tools"].as_array().unwrap();
            assert!(
                input
                    .iter()
                    .any(|item| item["role"] == "developer"
                        && item["content"] == "Later instruction"),
                "{id}: {input:?}"
            );
            assert!(
                !input.iter().any(|item| matches!(
                    item["type"].as_str(),
                    Some("tool_search_call" | "tool_search_output")
                )),
                "{id}"
            );
            let inline = input.iter().find(|item| item["type"] == "additional_tools");
            let grammar = if id == "gpt-6.1-terra" {
                assert_eq!(tools.len(), 1);
                &inline.unwrap()["tools"][0]
            } else {
                assert_eq!(tools.len(), 2, "{id}");
                assert!(inline.is_none(), "{id}");
                &tools[1]
            };
            assert_eq!(tools[0]["name"], "lookup");
            assert_eq!(tools[0]["strict"], true);
            assert_eq!(grammar["name"], "parse");
            assert_eq!(grammar["type"], "custom");
            assert_eq!(grammar["format"]["syntax"], "lark");
        }
    }
}

#[test]
fn openrouter_model_catalog_matches_constructor() {
    let provider = opi_ai::providers::openrouter::openrouter_provider(None);
    assert_same_models(
        provider.models(),
        &opi_ai::providers::openrouter::model_catalog(),
    );
}

#[test]
fn mistral_model_catalog_matches_constructor() {
    let provider = opi_ai::providers::mistral::mistral_provider(None);
    assert_same_models(
        provider.models(),
        &opi_ai::providers::mistral::model_catalog(),
    );
}

#[test]
fn gemini_model_catalog_matches_constructor() {
    let provider = opi_ai::providers::gemini::GeminiProvider::new(None);
    assert_same_models(
        provider.models(),
        &opi_ai::providers::gemini::model_catalog(),
    );
}

#[test]
fn google_catalog_is_shared_by_sdk_standalone_and_cli_factory() {
    use opi_ai::catalog::{Catalog, CatalogOperation};
    use opi_ai::providers::collection::{BuiltinOverrides, register_builtin};
    use opi_ai::providers::gemini::{GeminiProvider, model_catalog};

    let standalone = GeminiProvider::new(None);
    assert_eq!(standalone.models().len(), 23);
    let expected: Vec<_> = Catalog::builtin()
        .unwrap()
        .models("google", CatalogOperation::Chat)
        .unwrap()
        .map(|row| row.to_chat_model_info().unwrap())
        .collect();
    assert_eq!(expected.len(), 23);
    assert_eq!(standalone.models(), expected);
    assert_eq!(model_catalog(), expected);

    let mut sdk = opi_ai::Models::new();
    register_builtin(
        &mut sdk,
        Arc::new(opi_ai::auth::AmbientAuthContext),
        Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    assert_eq!(sdk.get_models(Some("google")), expected);

    let mut config = OpiConfig::default();
    config.defaults.model = "google:gemini-pro-latest".into();
    let cli = opi_coding_agent::provider_factory::build_provider(&config).unwrap();
    assert_eq!(cli.id(), "google");
    assert_eq!(cli.models(), expected);
    for id in ["gemini-pro-latest", "gemini-robotics-er-2-preview"] {
        assert!(sdk.get_model("google", id).is_some(), "{id}");
        assert!(cli.models().iter().any(|model| model.id == id), "{id}");
    }
}

#[tokio::test]
async fn google_published_models_dispatch_through_sdk_and_cli() {
    use futures_util::StreamExt;
    use opi_ai::auth::{AuthContext, AuthContextError};
    use opi_ai::credential::{Credential, CredentialStore};
    use opi_ai::providers::collection::{BuiltinOverrides, register_builtin};
    use opi_ai::{ChatModelRecord, Models, ModelsError, RegistryError, ThinkingLevel};
    use opi_coding_agent::credential_store::FakeKeyringBackend;
    use opi_coding_agent::provider_factory::{
        build_harness_collection, build_provider, build_provider_bundle,
    };
    use serde_json::{Value, json};
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    struct GoogleFixtureAuth;
    impl AuthContext for GoogleFixtureAuth {
        fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
            Ok((name == "GEMINI_API_KEY").then(|| "sdk-gemini-key".into()))
        }

        fn file_exists(&self, _: &str) -> Result<bool, AuthContextError> {
            Ok(false)
        }
    }

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(concat!(
                    "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"ok\"}]},",
                    "\"finishReason\":\"STOP\"}]}\n\n"
                )),
        )
        .mount(&server)
        .await;
    let mut sdk = Models::new();
    let mut overrides = BuiltinOverrides::default();
    overrides
        .base_urls
        .insert("google".into(), format!("{}/sdk/v1beta", server.uri()));
    register_builtin(
        &mut sdk,
        Arc::new(GoogleFixtureAuth),
        Arc::new(opi_ai::http::HttpClient::new()),
        &overrides,
    )
    .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let mut config = OpiConfig::default();
    config.defaults.model = "google:gemini-pro-latest".into();
    config.providers.google.base_url = Some(format!("{}/cli-proxy/", server.uri()));
    config.providers.google.api_key_env = "OPI_TEST_GOOGLE_WIRE_API_KEY".into();
    let bundle = build_provider_bundle(
        &config,
        dir.path().to_path_buf(),
        Box::new(|| Box::new(FakeKeyringBackend::new())),
    )
    .await
    .unwrap();
    bundle
        .store
        .write("google", &Credential::ApiKey("cli-gemini-key".into()))
        .await
        .unwrap();
    let mut routes = vec![(bundle.provider, bundle.auth_resolver)];
    routes.extend(bundle.extra_routes);
    let runtime =
        build_harness_collection(routes, Vec::new(), Vec::new(), "google", Some(bundle.store));
    let direct = build_provider(&config).unwrap();
    let entries = [
        (&sdk, "/sdk", "sdk-gemini-key"),
        (runtime.as_ref(), "/cli-proxy", "cli-gemini-key"),
    ];

    let cases = [
        ("gemini-pro-latest", ThinkingLevel::Low),
        ("gemini-robotics-er-2-preview", ThinkingLevel::Minimal),
    ];
    let levels = [
        ThinkingLevel::None,
        ThinkingLevel::High,
        ThinkingLevel::Minimal,
        ThinkingLevel::Max,
    ];
    let mut expected_requests = Vec::new();
    for (models, prefix, key) in entries {
        for (id, minimum) in cases {
            let model = models.get_model("google", id).unwrap();
            let levels_map = &model.thinking_level_map;
            assert_eq!(
                levels_map.nearest_supported(ThinkingLevel::Minimal),
                Some(minimum)
            );
            assert_eq!(
                levels_map.nearest_supported(ThinkingLevel::Max),
                Some(ThinkingLevel::High)
            );
            let record = ChatModelRecord::new("google", model);
            let spec = format!("google:{id}");
            for level in levels {
                let mut request = minimal_request(&spec);
                request.timeout = Some(std::time::Duration::from_secs(3));
                if level != ThinkingLevel::None {
                    request.thinking = ThinkingConfig {
                        enabled: true,
                        level,
                        budget_tokens: None,
                    };
                }
                let result = models
                    .complete_simple(record.clone(), request, Default::default())
                    .await
                    .unwrap_or_else(|error| panic!("{prefix} {spec} {level:?}: {error:?}"));
                assert_eq!(result.provider, "google", "{prefix}");
                assert_eq!(result.model, id);
                expected_requests.push((prefix, id, key, level));
            }
        }
    }
    let captured = server.received_requests().await.unwrap();
    assert_eq!(captured.len(), 16);
    for (request, (prefix, id, key, level)) in captured.iter().zip(expected_requests) {
        assert_eq!(
            request.url.path(),
            format!("{prefix}/v1beta/models/{id}:streamGenerateContent")
        );
        assert_eq!(request.url.query(), Some("alt=sse"));
        assert_eq!(request.headers["x-goog-api-key"], key);
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        let thinking = if level == ThinkingLevel::None {
            json!({"thinkingBudget": 0})
        } else {
            json!({"includeThoughts": true, "thinkingBudget": -1})
        };
        assert_eq!(
            body["generationConfig"]["thinkingConfig"], thinking,
            "{id} {level:?}"
        );
    }

    for retired in [
        "gemini-2.0-flash",
        "gemini-3.1-flash-lite-image",
        "gemini-3.1-flash-live-preview",
    ] {
        let spec = format!("google:{retired}");
        for (models, _, _) in entries {
            assert!(models.get_model("google", retired).is_none());
            assert!(matches!(
                models.prepare_call(&spec, minimal_request(&spec)).await,
                Err(ModelsError::Registry(RegistryError::UnknownModel { provider, model }))
                    if provider == "google" && model == retired
            ));
        }
        let error = direct
            .stream_prepared(
                minimal_request(&spec),
                opi_ai::test_support::resolved_auth(),
            )
            .next()
            .await
            .expect("a typed error for a retired Google model")
            .unwrap_err();
        assert!(matches!(
            error,
            opi_ai::provider::ProviderError::UnknownModel { provider_id, model_id }
                if provider_id == "google" && model_id == retired
        ));
    }
    let vertex_spec = "google-vertex:gemini-2.5-flash";
    let result = direct
        .stream_prepared(
            minimal_request(vertex_spec),
            opi_ai::test_support::resolved_auth(),
        )
        .next()
        .await;
    assert!(matches!(
        result,
        Some(Err(opi_ai::provider::ProviderError::UnknownModel { provider_id, model_id }))
            if provider_id == "google" && model_id == vertex_spec
    ));
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        captured.len()
    );
}

#[test]
fn bedrock_model_catalog_matches_constructor() {
    let provider = opi_ai::providers::bedrock::BedrockProvider::new(
        None,
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
    );
    assert_same_models(
        provider.models(),
        &opi_ai::providers::bedrock::model_catalog(),
    );
}

#[test]
fn vertex_catalog_is_shared_by_sdk_standalone_and_cli_factory() {
    use opi_ai::catalog::{Catalog, CatalogOperation};
    use opi_ai::providers::collection::{BuiltinOverrides, register_builtin};
    use opi_ai::providers::vertex::{VertexProvider, model_catalog};
    use opi_coding_agent::provider_factory::build_provider;

    let express = VertexProvider::express(None);
    let regional = VertexProvider::new("test-project".into(), "us-central1".into(), None);
    assert_eq!(express.models().len(), 13);
    let expected: Vec<_> = Catalog::builtin()
        .unwrap()
        .models("google-vertex", CatalogOperation::Chat)
        .unwrap()
        .map(|row| row.to_chat_model_info().unwrap())
        .collect();
    assert_eq!(expected.len(), 13);
    assert_eq!(express.models(), expected);
    assert_eq!(regional.models(), expected);
    assert_eq!(model_catalog(), expected);

    let mut sdk = opi_ai::Models::new();
    register_builtin(
        &mut sdk,
        Arc::new(opi_ai::auth::AmbientAuthContext),
        Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    assert_eq!(sdk.get_models(Some("google-vertex")), expected);

    let mut config = OpiConfig::default();
    config.defaults.model = "google-vertex:gemini-3.1-pro-preview".into();
    for project_location in [None, Some(("test-project", "us-central1"))] {
        config.providers.google_vertex.project =
            project_location.map(|(project, _)| project.into());
        config.providers.google_vertex.location =
            project_location.map(|(_, location)| location.into());
        let cli = build_provider(&config).unwrap();
        assert_eq!(cli.id(), "google-vertex");
        assert_eq!(cli.models(), expected);
    }
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

#[tokio::test]
async fn build_provider_bundle_populates_eager_extra_dispatch_routes() {
    use opi_coding_agent::credential_store::{
        FakeKeyringBackend, KEYCHAIN_PRESENCE_SERVICE, KEYCHAIN_SERVICE, KeyringBackendFactory,
    };
    use opi_coding_agent::provider_factory::build_provider_bundle;

    let backend_factory: KeyringBackendFactory = Box::new(|| {
        let backend = FakeKeyringBackend::new();
        backend.seed_raw(
            KEYCHAIN_SERVICE,
            "anthropic",
            r#"{"version":1,"kind":"api_key","api_key":"test-eager-routes"}"#,
        );
        backend.seed_raw(KEYCHAIN_PRESENCE_SERVICE, "anthropic", "api_key");
        Box::new(backend)
    });

    let dir = tempfile::tempdir().expect("temp dir");
    let mut config = OpiConfig::default();
    config.defaults.model = "anthropic:claude-sonnet-4-5-20250514".into();
    let bundle = build_provider_bundle(&config, dir.path().to_path_buf(), backend_factory)
        .await
        .expect("provider bundle builds");

    // Phase 17.5: eager multi-route construction. Every built-in provider that
    // can construct a wire adapter WITHOUT credentials (lazy credential IO) is
    // registered in `extra_routes`, so a cross-provider switch resolves through
    // one collection without reconstructing the Agent.
    let extra_ids: Vec<&str> = bundle.extra_routes.iter().map(|(p, _)| p.id()).collect();
    for expected in [
        "openai",
        "openrouter",
        "mistral",
        "openai-responses",
        "google",
    ] {
        assert!(
            extra_ids.contains(&expected),
            "extra_routes must contain {expected}, got {extra_ids:?}"
        );
    }
    assert!(
        !extra_ids.contains(&"anthropic"),
        "active provider must not be in extra_routes, got {extra_ids:?}"
    );
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
fn build_provider_production_future_has_provider_bundle_output_type() {
    use std::future::Future;

    use opi_coding_agent::provider_factory::{
        ProviderBuildError, ProviderBundle, build_provider_production,
    };

    fn assert_provider_bundle_output<F>(_: F)
    where
        F: Future<Output = Result<ProviderBundle, ProviderBuildError>>,
    {
    }

    let config = OpiConfig::default();
    // Compile-time return-type coverage only: this future is intentionally
    // unpolled. The separate async lifetime test below owns runtime behavior.
    assert_provider_bundle_output(build_provider_production(
        &config,
        std::path::PathBuf::from("compile-only-unpolled-keyring-path"),
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
        auth_resolver: Arc::new(opi_ai::StaticAuthResolver::new(
            opi_ai::AuthScheme::ApiKey,
            secrecy::SecretString::from("test"),
        )),
        extra_routes: Vec::new(),
        store,
        resolver,
        registry,
        diagnostics: Vec::new(),
    };

    assert!(!events.lock().unwrap().contains(&"guard_drop"));
    assert_eq!(bundle.provider.id(), "owned-provider");
    assert_eq!(bundle.provider.models()[0].id, "mock-model");
    let mut stream = bundle.provider.stream_prepared(
        minimal_request("mock-model"),
        opi_ai::test_support::resolved_auth(),
    );
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
async fn github_copilot_factory_builds_one_three_wire_provider() {
    use std::collections::BTreeSet;

    use opi_ai::WireApi;
    use opi_coding_agent::credential_store::{
        CredentialResolver, FakeKeyringBackend, KeychainCredentialStore,
    };
    use opi_coding_agent::oauth::OAuthProviderRegistry;
    use opi_coding_agent::provider_factory::build_provider_with_oauth;

    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(KeychainCredentialStore::new(
        Box::new(FakeKeyringBackend::new()),
        dir.path().to_path_buf(),
    ));
    let resolver = CredentialResolver::new(store, Arc::new(|_: &str| None));
    let mut config = OpiConfig::default();
    config.defaults.model = "github-copilot:gemini-4-argon".into();

    let provider = build_provider_with_oauth(
        &config,
        &resolver,
        &OAuthProviderRegistry::registry_with_builtins(),
    )
    .await
    .expect("static GitHub Copilot factory");

    assert_eq!(provider.id(), "github-copilot");
    assert_eq!(provider.models().len(), 27);
    assert_eq!(
        provider
            .models()
            .iter()
            .map(|model| model.wire_api)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            WireApi::AnthropicMessages,
            WireApi::OpenAiCompletions,
            WireApi::OpenAiResponses,
        ])
    );
}

#[tokio::test]
async fn openai_codex_factory_uses_dedicated_provider() {
    use opi_ai::WireApi;
    use opi_coding_agent::credential_store::{
        CredentialResolver, FakeKeyringBackend, KeychainCredentialStore,
    };
    use opi_coding_agent::oauth::OAuthProviderRegistry;
    use opi_coding_agent::provider_factory::build_provider_with_oauth;

    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(KeychainCredentialStore::new(
        Box::new(FakeKeyringBackend::new()),
        dir.path().to_path_buf(),
    ));
    let resolver = CredentialResolver::new(store, Arc::new(|_: &str| None));
    let mut config = OpiConfig::default();
    config.defaults.model = "openai-codex:gpt-5.6-sol".into();

    let provider = build_provider_with_oauth(
        &config,
        &resolver,
        &OAuthProviderRegistry::registry_with_builtins(),
    )
    .await
    .expect("dedicated OpenAI Codex factory");

    assert_eq!(provider.id(), "openai-codex");
    assert_eq!(provider.models().len(), 8);
    assert!(
        provider
            .models()
            .iter()
            .all(|model| model.wire_api == WireApi::OpenAiCodexResponses)
    );

    let factory_source = include_str!("../src/provider_factory.rs");
    let build_codex = factory_source
        .split("async fn build_codex_oauth")
        .nth(1)
        .and_then(|tail| tail.split("pub async fn build_provider_with_oauth").next())
        .expect("Codex factory block");
    assert!(
        !build_codex.contains("OpenAiResponsesProvider"),
        "Codex factory must not use the standard Responses provider"
    );

    let standard_source = include_str!("../../opi-ai/src/api/openai_responses.rs");
    for forbidden in [
        "derive_codex_account_id",
        "codex/responses",
        "chatgpt-account-id",
    ] {
        assert!(
            !standard_source.contains(forbidden),
            "standard Responses module still exposes Codex toggle `{forbidden}`"
        );
    }
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
            &Credential::ApiKey(SecretString::new(api_key.to_owned().into_boxed_str()).into()),
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
    let mut stream = provider.stream_prepared(
        minimal_request("anthropic:claude-sonnet-4-5-20250514"),
        opi_ai::auth::ResolvedAuth {
            env: Default::default(),
            scheme: opi_ai::AuthScheme::ApiKey,
            secret: secrecy::SecretString::from("sk-anthropic-stored-api"),
            base_url: None,
            account_id: None,
            provenance: Default::default(),
        },
    );
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
    let compat = opi_ai::providers::openai::CompatConfig::default();
    // Get the default model list from the convenience function.
    let temp = opi_ai::providers::openrouter::openrouter_provider(None);
    let models = temp.models().to_vec();
    let provider = opi_ai::providers::openai::OpenAiChatProvider::new_for_profile(
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
instruction_role = "developer"
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
    assert_eq!(
        profile.instruction_role,
        Some(opi_ai::model_info::ChatInstructionRole::Developer)
    );
    assert_eq!(
        profile.max_tokens_field.as_deref(),
        Some("max_completion_tokens")
    );
    assert!(profile.tool_result_name_field);
    assert_eq!(profile.usage_in_stream, Some(true));
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
[providers.google]
api_key_env = "MY_GEMINI_KEY"
base_url = "https://custom-gemini.example.com"
"#,
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();
    assert_eq!(config.providers.google.api_key_env, "MY_GEMINI_KEY");
    assert_eq!(
        config.providers.google.base_url.as_deref(),
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

[providers.google]
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
    assert_eq!(config.providers.google.api_key_env, "KEY_G");
    assert_eq!(config.providers.mistral.api_key_env, "KEY_M");
    assert_eq!(config.providers.openrouter.api_key_env, "KEY_OR");
    assert_eq!(config.providers.openai_responses.api_key_env, "KEY_OAR");
}

// ---------------------------------------------------------------------------
// Phase 10.2: provider construction routes through the collection/auth seam
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serializes process-env mutation; the awaited checks never re-acquire this lock.
async fn provider_factory_routes_through_collection() {
    use opi_ai::{AuthCheckKind, AuthDescriptor};
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

    // Serialize ambient-env mutation for the whole (now async) body.
    let _env_lock = ENV_MUTEX.lock().unwrap();
    let _env = EnvVarGuard::set(env_var, "test-key");
    let collection = build_collection_for_listing(&config, &std::collections::HashMap::new())
        .expect("listing collection builds through the factory");
    {
        // Auth + compat metadata for the config-sourced profile live on the
        // collection; the on-demand check reads the env source and the
        // descriptor records the variable name.
        let check = collection
            .check_auth("testprof")
            .await
            .expect("check")
            .expect("configured through the env key");
        assert_eq!(check.kind, AuthCheckKind::ApiKey);
        assert_eq!(check.source.as_deref(), Some(env_var));
        match collection.auth_descriptor("testprof") {
            Some(AuthDescriptor::EnvApiKey { env_var: name }) => assert_eq!(name, env_var),
            other => panic!("expected profile EnvApiKey auth, got {other:?}"),
        }
        let compat = collection
            .compat("testprof")
            .expect("profile compat metadata attached");
        assert!(compat.openai_compatible);
        assert_eq!(compat.profile.as_deref(), Some("testprof"));
    }
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
instruction_role = "system"
max_tokens_field = "max_tokens"
strict_tool_schema = true
cache_key = "profile-cache"
require_assistant_after_tool_result = true

[providers.openai_compatible.myprof.extra_headers]
X-Session-Id = "sess-1"

[[providers.openai_compatible.myprof.models]]
id = "reasoning-model"
display_name = "Reasoning"
context_window = 128000
max_output_tokens = 4096
instruction_role = "developer"
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
    assert_eq!(
        profile.instruction_role,
        Some(opi_ai::model_info::ChatInstructionRole::System)
    );
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
    assert_eq!(
        reasoning.instruction_role,
        Some(opi_ai::model_info::ChatInstructionRole::Developer)
    );
    assert_eq!(
        reasoning.max_tokens_field.as_deref(),
        Some("max_completion_tokens")
    );
    let plain = profile
        .models
        .iter()
        .find(|m| m.id == "plain-model")
        .expect("plain model");
    assert!(plain.instruction_role.is_none());
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

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serializes process-env mutation; the awaited checks never re-acquire this lock.
async fn listing_collection_skips_whitespace_only_credentials() {
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

    // Whitespace-only credentials now stay unlisted through the on-demand
    // configuration check: config-valid profiles register, but the check
    // filters them out of availability (20.18).
    let _env_lock = ENV_MUTEX.lock().unwrap();
    let _openai = EnvVarGuard::set(openai_env, "   ");
    let _profile = EnvVarGuard::set(profile_env, "\t ");
    let collection = build_collection_for_listing(&config, &std::collections::HashMap::new())
        .expect("listing collection builds");
    assert!(
        collection
            .check_auth("testprof")
            .await
            .expect("check")
            .is_none(),
        "whitespace-only env value must not be available"
    );
    assert!(
        collection.check_auth("openai").await.unwrap().is_none(),
        "whitespace-only builtin env value must not be available"
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serializes process-env mutation; the awaited checks never re-acquire this lock.
async fn listing_collection_uses_resolved_auth_for_constructed_builtin() {
    use opi_ai::{AuthCheckKind, AuthDescriptor};
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

    let _env_lock = ENV_MUTEX.lock().unwrap();
    let _env = EnvVarGuard::set(env_var, "test-key");
    let collection = build_collection_for_listing(&config, &std::collections::HashMap::new())
        .expect("listing collection builds");
    let check = collection
        .check_auth("openai")
        .await
        .expect("check")
        .expect("configured");
    assert_eq!(check.kind, AuthCheckKind::ApiKey);
    assert!(check.source.is_some());
    match collection.auth_descriptor("openai") {
        Some(AuthDescriptor::Resolved { source }) => {
            assert_eq!(source, &format!("env {env_var}"))
        }
        other => panic!("expected builtin Resolved auth, got {other:?}"),
    }
}

#[tokio::test]
async fn metadata_only_provider_dispatch_returns_explicit_error() {
    use opi_coding_agent::provider_factory::build_harness_collection;

    // A lookup-only extension provider (registered without a route resolver) is
    // not dispatchable. 17.5: the metadata-only rejection surfaces at
    // prepare_call as ModelsError::RouteNotDispatchable.
    let collection = build_harness_collection(
        Vec::new(),
        vec![Box::new(MockProvider::new("metadata-provider", vec![]))],
        Vec::new(),
        "metadata-provider",
        None,
    );

    let error = collection
        .prepare_call(
            "metadata-provider:mock-model",
            minimal_request("metadata-provider:mock-model"),
        )
        .await
        .expect_err("metadata provider should not dispatch");
    let message = error.to_string();
    assert!(
        message.contains("no dispatchable route"),
        "expected no-dispatchable-route error, got {message:?}"
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
        c.providers.google_vertex.project = Some("test-project".into());
        c.providers.google_vertex.location = Some("us-central1".into());
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
        ("google", &[("GEMINI_API_KEY", "test-key")], noop),
        (
            "amazon-bedrock",
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
            "google-vertex",
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
            expected_wire: WireApi::OpenAiResponses,
            route: Route::OpenAiResponses,
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
            provider_id: "google",
            expected_wire: WireApi::GoogleGenerativeAi,
            route: Route::Gemini,
        },
        RouteCase {
            provider_id: "google-vertex",
            expected_wire: WireApi::GoogleVertex,
            route: Route::Vertex,
        },
        RouteCase {
            provider_id: "amazon-bedrock",
            expected_wire: WireApi::BedrockConverseStream,
            route: Route::Bedrock,
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
        let selected_model = if case.provider_id == "amazon-bedrock" {
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
            "google" => {
                config.providers.google.api_key_env = ROUTE_KEY_ENV.into();
                config.providers.google.base_url = Some(server.uri());
            }
            "google-vertex" => {
                config.providers.google_vertex.access_token_env = ROUTE_KEY_ENV.into();
                config.providers.google_vertex.project = Some("test-project".into());
                config.providers.google_vertex.location = Some("us-central1".into());
                config.providers.google_vertex.base_url = Some(server.uri());
            }
            "amazon-bedrock" => {
                config.providers.amazon_bedrock.access_key_id = Some("test-akid".into());
                config.providers.amazon_bedrock.secret_access_key_env = Some(ROUTE_KEY_ENV.into());
                config.providers.amazon_bedrock.region = Some("us-east-1".into());
                config.providers.amazon_bedrock.base_url = Some(server.uri());
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

        let auth = if case.provider_id == "amazon-bedrock" {
            opi_ai::ResolvedAuth::aws_sigv4(
                opi_ai::AwsSigV4Credentials {
                    access_key_id: "test-access-key".into(),
                    secret_access_key: "test-secret".into(),
                    session_token: None,
                    region: "us-east-1".into(),
                    expires_at: None,
                },
                opi_ai::AuthProvenance {
                    source: opi_ai::AuthProvenanceSource::AwsSigV4 {
                        source: opi_ai::AwsCredentialSource::Environment,
                    },
                    fallback: opi_ai::AuthFallback::NotAttempted,
                },
            )
        } else {
            // Each wire attaches only its accepted scheme: anthropic accepts
            // ApiKey/Bearer, Gemini attaches ApiKey, and every other
            // built-in single-wire provider attaches Bearer.
            let scheme = match case.provider_id {
                "anthropic" | "google" => opi_ai::AuthScheme::ApiKey,
                _ => opi_ai::AuthScheme::Bearer,
            };
            opi_ai::auth::ResolvedAuth {
                env: Default::default(),
                scheme,
                secret: secrecy::SecretString::from("test-secret"),
                base_url: None,
                account_id: None,
                provenance: Default::default(),
            }
        };
        let mut stream = provider.stream_prepared(
            minimal_request(&format!("{}:{model_id}", case.provider_id)),
            auth,
        );
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
fn azure_default_factory_uses_the_catalog_even_for_an_unknown_default() {
    use opi_ai::catalog::{Catalog, CatalogOperation};
    use opi_coding_agent::provider_factory::build_provider;

    let mut config = OpiConfig::default();
    config.defaults.model = "azure:gpt-5.3-chat-latest".into();
    config.providers.azure.endpoint = Some("http://127.0.0.1:9".into());

    let provider = build_provider(&config).unwrap();
    let published: Vec<_> = Catalog::builtin()
        .unwrap()
        .models("azure", CatalogOperation::Chat)
        .unwrap()
        .map(|record| record.to_chat_model_info().unwrap())
        .collect();
    assert_eq!(provider.models().len(), 45);
    assert_eq!(provider.models(), published);
    assert!(
        !provider
            .models()
            .iter()
            .any(|model| model.id == "gpt-5.3-chat-latest")
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serialize the test's Azure environment across requests.
async fn azure_published_models_dispatch_through_sdk_and_cli() {
    use futures_util::StreamExt;
    use opi_ai::auth::{AuthContext, AuthContextError};
    use opi_ai::constrained_sampling::ConstrainedSampling;
    use opi_ai::credential::{Credential, CredentialStore};
    use opi_ai::message::{InputContent, Message, ToolDef, UserMessage};
    use opi_ai::providers::collection::{BuiltinOverrides, register_builtin};
    use opi_ai::transcript::{Context, SystemMessage, normalize_context};
    use opi_ai::{ChatModelRecord, Models, ModelsError, RegistryError, ThinkingLevel};
    use opi_coding_agent::credential_store::FakeKeyringBackend;
    use opi_coding_agent::provider_factory::{
        build_harness_collection, build_provider, build_provider_bundle,
    };
    use serde_json::{Value, json};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const MAP: &str = "gpt-5.6-cyber=cyber-prod, gpt-6.1-terra=terra-prod, deepseek-v4-pro=deepseek-prod, \
        gpt-5.2-chat-latest=retired, gpt-5.3-chat-latest=retired, gpt-realtime-2.1=retired";
    struct FixtureAuth(String);
    impl AuthContext for FixtureAuth {
        fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
            Ok(match name {
                "AZURE_OPENAI_API_KEY" => Some("sdk-azure-key".into()),
                "AZURE_OPENAI_BASE_URL" => Some(self.0.clone()),
                "AZURE_OPENAI_DEPLOYMENT_NAME_MAP" => Some(MAP.into()),
                _ => None,
            })
        }
        fn file_exists(&self, _: &str) -> Result<bool, AuthContextError> {
            Ok(false)
        }
    }

    let server = MockServer::start().await;
    let unused = MockServer::start().await;
    let base = format!("{}/openai/v1", server.uri());
    let mut sdk = Models::new();
    register_builtin(
        &mut sdk,
        Arc::new(FixtureAuth(base.clone())),
        Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();

    let _lock = ENV_MUTEX.lock().unwrap();
    let _deployment = EnvVarGuard::set("AZURE_OPENAI_DEPLOYMENT_NAME_MAP", MAP);
    let _endpoint = EnvVarGuard::set("AZURE_OPENAI_BASE_URL", &unused.uri());
    let _version = EnvVarGuard::set("AZURE_OPENAI_API_VERSION", "unused-version");
    let dir = tempfile::tempdir().unwrap();
    let mut config = OpiConfig::default();
    config.defaults.model = "azure:chat-latest".into();
    config.providers.azure.endpoint = Some(base);
    config.providers.azure.api_version = Some("fixture-version".into());
    config.providers.azure.api_key_env = "OPI_TEST_AZURE_WIRE_API_KEY".into();
    let bundle = build_provider_bundle(
        &config,
        dir.path().to_path_buf(),
        Box::new(|| Box::new(FakeKeyringBackend::new())),
    )
    .await
    .unwrap();
    bundle
        .store
        .write("azure", &Credential::ApiKey("cli-azure-key".into()))
        .await
        .unwrap();
    let mut routes = vec![(bundle.provider, bundle.auth_resolver)];
    routes.extend(bundle.extra_routes);
    let runtime =
        build_harness_collection(routes, Vec::new(), Vec::new(), "azure", Some(bundle.store));
    let direct = build_provider(&config).unwrap();

    let published = sdk.get_models(Some("azure"));
    assert_eq!(published.len(), 45);
    assert_eq!(runtime.get_models(Some("azure")), published);
    assert_eq!(direct.models(), published);
    let entries = [
        (&sdk, "v1", "sdk-azure-key"),
        (runtime.as_ref(), "fixture-version", "cli-azure-key"),
    ];
    for id in [
        "gpt-5.2-chat-latest",
        "gpt-5.3-chat-latest",
        "gpt-realtime-2.1",
        "terra-prod",
    ] {
        let spec = format!("azure:{id}");
        for (models, _, _) in entries {
            assert!(models.get_model("azure", id).is_none());
            assert!(
                matches!(
                    models.prepare_call(&spec, minimal_request(&spec)).await,
                    Err(ModelsError::Registry(RegistryError::UnknownModel { provider, model }))
                        if provider == "azure" && model == id
                ),
                "{spec}"
            );
        }
        assert!(matches!(
            direct.stream_prepared(minimal_request(&spec), opi_ai::test_support::resolved_auth()).next().await,
            Some(Err(opi_ai::provider::ProviderError::UnknownModel { provider_id, model_id }))
                if provider_id == "azure" && model_id == id
        ));
    }
    assert!(server.received_requests().await.unwrap().is_empty());

    let cases = [
        (
            "chat-latest",
            "chat-latest",
            ThinkingLevel::None,
            Some("none"),
        ),
        ("gpt-5.6-cyber", "cyber-prod", ThinkingLevel::None, None),
        (
            "gpt-5.6-cyber",
            "cyber-prod",
            ThinkingLevel::Max,
            Some("max"),
        ),
        (
            "gpt-6.1-terra",
            "terra-prod",
            ThinkingLevel::None,
            Some("none"),
        ),
        (
            "gpt-6.1-terra",
            "terra-prod",
            ThinkingLevel::Minimal,
            Some("low"),
        ),
        (
            "gpt-6.1-terra",
            "terra-prod",
            ThinkingLevel::Max,
            Some("max"),
        ),
        (
            "deepseek-v4-pro",
            "deepseek-prod",
            ThinkingLevel::Max,
            Some("high"),
        ),
    ];
    Mock::given(method("POST"))
        .and(path("/openai/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            concat!("data: ", r#"{"type":"response.completed","response":{"id":"fixture","status":"completed","output":[]}}"#, "\n\n"),
            "text/event-stream",
        ))
        .expect(12)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/openai/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            concat!("data: ", r#"{"id":"chatcmpl-fixture","model":"deepseek-prod","choices":[{"index":0,"delta":{"content":"ok"},"finish_reason":"stop"}]}"#, "\n\ndata: [DONE]\n\n"),
            "text/event-stream",
        ))
        .expect(2)
        .mount(&server)
        .await;

    let user_message = || {
        Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "hello".into(),
            }],
            timestamp_ms: 0,
        })
    };
    let grammar = ToolDef {
        name: "parse".into(),
        description: "Parse an expression".into(),
        input_schema: json!({"type":"object","properties":{"input":{"type":"string"}},"required":["input"]}),
        constrained_sampling: Some(ConstrainedSampling::Grammar {
            openai_lark: Some("start: \"hello\"".into()),
            openai_regex: None,
        }),
    };
    for (models, _, _) in entries {
        for (id, _, level, _) in cases {
            let model = models.get_model("azure", id).unwrap();
            let mut request = minimal_request(&format!("azure:{id}"));
            request.thinking.enabled = level != ThinkingLevel::None;
            request.thinking.level = level;
            request.max_tokens = Some(8);
            request.temperature = Some(0.125);
            request.timeout = Some(std::time::Duration::from_secs(5));
            request.cache_retention = CacheRetention::Long;
            request.session_id = Some("azure-fixture-session".into());
            let mut context = Context {
                messages: vec![user_message()],
                ..Default::default()
            };
            match id {
                "deepseek-v4-pro" => context.messages.extend([
                    Message::System(SystemMessage {
                        content: "Later system instruction".into(),
                        ..Default::default()
                    }),
                    user_message(),
                ]),
                "gpt-5.6-cyber" | "gpt-6.1-terra" => context.tools.push(grammar.clone()),
                _ => {}
            }
            request.context = normalize_context(context).unwrap();
            let result = if matches!(
                (id, level),
                ("gpt-6.1-terra", ThinkingLevel::Minimal) | ("deepseek-v4-pro", ThinkingLevel::Max)
            ) {
                models
                    .complete_simple(
                        ChatModelRecord::new("azure", model),
                        request,
                        Default::default(),
                    )
                    .await
            } else {
                let spec = request.model.clone();
                models
                    .prepare_call(&spec, request)
                    .await
                    .unwrap()
                    .start_attempt_with_telemetry()
                    .unwrap()
                    .result()
                    .await
            }
            .unwrap_or_else(|error| panic!("{id}/{level:?}: {error:?}"));
            assert_eq!(result.model, id);
            assert_eq!(result.stop_reason, opi_ai::stream::StopReason::Stop);
        }
    }

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), entries.len() * cases.len());
    assert!(unused.received_requests().await.unwrap().is_empty());
    for ((_, version, key), sent) in entries.into_iter().zip(sent.chunks_exact(cases.len())) {
        for ((id, deployment, level, effort), request) in cases.into_iter().zip(sent) {
            let body: Value = serde_json::from_slice(&request.body).unwrap();
            assert_eq!(body["model"], deployment);
            assert_eq!(body["stream"], true);
            if id == "deepseek-v4-pro" {
                assert_eq!(request.url.path(), "/openai/v1/chat/completions");
                assert!(request.url.query().is_none());
                assert_eq!(request.headers["authorization"], format!("Bearer {key}"));
                assert!(request.headers.get("api-key").is_none());
                assert_eq!(body["reasoning_effort"], effort.unwrap());
                assert!(body.get("thinking").is_none());
                assert!(
                    body.as_object()
                        .unwrap()
                        .keys()
                        .all(|name| !name.starts_with("prompt_cache"))
                );
                assert!(body["messages"].as_array().unwrap().iter().any(|item| {
                    item["role"] == "system" && item["content"] == "Later system instruction"
                }));
            } else {
                assert_eq!(request.url.path(), "/openai/v1/responses");
                assert_eq!(
                    request.url.query(),
                    Some(format!("api-version={version}").as_str())
                );
                assert_eq!(request.headers["api-key"], key);
                assert!(request.headers.get("authorization").is_none());
                assert_eq!(body["store"], false);
                assert_eq!(body["max_output_tokens"], 16);
                assert_eq!(body["temperature"], 0.125);
                let reasoning = effort.map(|effort| {
                    if level == ThinkingLevel::None {
                        json!({"effort": effort})
                    } else {
                        json!({"effort": effort, "summary": "auto"})
                    }
                });
                assert_eq!(body.get("reasoning"), reasoning.as_ref(), "{id}/{level:?}");
                let include =
                    (level != ThinkingLevel::None).then(|| json!(["reasoning.encrypted_content"]));
                assert_eq!(body.get("include"), include.as_ref(), "{id}/{level:?}");
                if id != "chat-latest" {
                    assert_eq!(body["tools"][0]["format"]["syntax"], "lark");
                }
            }
        }
    }
}

#[test]
fn azure_constructor_rejects_empty_selected_deployment() {
    assert!(matches!(
        opi_ai::providers::azure_openai::AzureOpenAIProvider::new(
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
        opi_ai::providers::azure_openai::AzureOpenAIProvider::from_config(
            Some("https://test.openai.azure.com".into()),
            vec![],
            None,
        ),
        Err(opi_ai::provider::ProviderError::Config(message))
            if message.contains("deployment")
    ));
}

/// Credentials are resolved by each route's per-call auth resolver at
/// `prepare_call`, so missing credentials never prevent adapter construction,
/// including Bedrock's AWS credential chain.
#[test]
fn build_provider_builds_all_credentials_lazily_including_bedrock() {
    use opi_coding_agent::config::OpiConfig;
    use opi_coding_agent::provider_factory::build_provider;

    // Lazy construction: removing the single env var must NOT fail provider
    // construction for single-credential providers (the credential is resolved
    // later at dispatch).
    let lazy_cases: &[(&str, &[&str])] = &[
        ("anthropic", &["ANTHROPIC_API_KEY"]),
        ("azure", &["AZURE_OPENAI_API_KEY"]),
        ("google-vertex", &["VERTEX_ACCESS_TOKEN"]),
    ];
    for (id, remove) in lazy_cases.iter().copied() {
        with_env_managed(remove, &[], || {
            let mut config = OpiConfig::default();
            config.defaults.model = format!("{id}:test-model");
            // Azure/vertex still need non-secret endpoint/project+location config
            // to construct their adapter; the credential is the only lazy half.
            if id == "azure" {
                config.providers.azure.endpoint = Some("https://test.openai.azure.com".into());
            } else if id == "google-vertex" {
                config.providers.google_vertex.project = Some("test-project".into());
                config.providers.google_vertex.location = Some("us-central1".into());
            }
            let provider = build_provider(&config).unwrap_or_else(|e| {
                panic!("{id} must build lazily without a credential, got {e:?}")
            });
            assert_eq!(provider.id(), id, "{id} lazy construction");
        });
    }

    // Bedrock credential-chain exhaustion is lazy too. Redirect the shared
    // files so no real ~/.aws is read; the credential-free adapter still
    // constructs and the route resolver reports absence during preparation.
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
            config.defaults.model = "amazon-bedrock:test-model".into();
            let provider = build_provider(&config)
                .unwrap_or_else(|error| panic!("Bedrock must build lazily, got {error:?}"));
            assert_eq!(provider.id(), "amazon-bedrock");
        },
    );
}

/// Invalid endpoint/config surfaces a Config-class diagnostic at build time
/// with a remediation message. DoD: "invalid endpoints emit safe diagnostics
/// with provider/error class and remediation". Covers the openai_compatible
/// profile base_url requirement (runtime + listing arms), vertex missing
/// project/location, and an invalid explicit Azure endpoint.
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
        missing_project.defaults.model = "google-vertex:test-model".into();
        missing_project.providers.google_vertex.location = Some("us-central1".into());
        match build_provider(&missing_project) {
            Err(ProviderBuildError::Config(msg)) => assert!(
                msg.contains("project"),
                "vertex missing-project Config should mention project, got: {msg:?}"
            ),
            Err(e) => panic!("vertex missing-project should be Config, got {e:?}"),
            Ok(_) => panic!("vertex missing-project should be Config, got Ok provider"),
        }

        let mut missing_location = opi_coding_agent::config::OpiConfig::default();
        missing_location.defaults.model = "google-vertex:test-model".into();
        missing_location.providers.google_vertex.project = Some("test-project".into());
        match build_provider(&missing_location) {
            Err(ProviderBuildError::Config(msg)) => assert!(
                msg.contains("location"),
                "vertex missing-location Config should mention location, got: {msg:?}"
            ),
            Err(e) => panic!("vertex missing-location should be Config, got {e:?}"),
            Ok(_) => panic!("vertex missing-location should be Config, got Ok provider"),
        }
    });

    let mut azure = OpiConfig::default();
    azure.defaults.model = "azure:chat-latest".into();
    azure.providers.azure.endpoint = Some("invalid-url".into());
    let Err(error) = build_provider(&azure) else {
        panic!("invalid Azure endpoint was accepted");
    };
    assert!(error.to_string().contains("Invalid Azure OpenAI base URL"));
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
instruction_role = "system"
max_tokens_field = "max_tokens"

[[providers.openai_compatible.prof.models]]
id = "m1"
display_name = "M1"
context_window = 128000
max_output_tokens = 4096
instruction_role = "developer"
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
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: Some("sys".into()),
        ..Default::default()
    })
    .expect("valid context declarations");
    request.max_tokens = Some(1024);
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
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
        .stream_prepared(
            minimal_request("prof:unknown"),
            opi_ai::test_support::resolved_auth(),
        )
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
/// `Models::from_registry` with NO auth descriptors, distinct from
/// `build_collection_for_listing`'s register-with-AuthDescriptor listing path.
/// DoD: "assembles harness model lookup through from_registry without claiming
/// auth-gated runtime dispatch". The existing
/// `metadata_only_provider_dispatch_returns_explicit_error` asserts dispatch
/// behavior; this pins the from_registry seam by asserting NO auth descriptor
/// is attached to a lookup-only extension provider whose models still resolve.
#[tokio::test]
async fn build_harness_collection_uses_from_registry_seam() {
    use opi_coding_agent::provider_factory::build_harness_collection;

    let collection = build_harness_collection(
        Vec::new(),
        vec![Box::new(MockProvider::new("mock-active", vec![]))],
        Vec::new(),
        "mock-active",
        None,
    );

    // from_registry path: the lookup-only provider contributes metadata but NO
    // auth descriptor (the listing path attaches Resolved descriptors).
    assert!(
        collection.auth_descriptor("mock-active").is_none(),
        "from_registry collection must not attach an auth descriptor for the active provider"
    );
    assert!(
        collection
            .check_auth("mock-active")
            .await
            .expect("check")
            .is_none(),
        "from_registry collection must leave the active provider unchecked"
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

#[test]
fn build_harness_collection_registers_provider_owned_deferred_route() {
    use opi_ai::auth::{AuthScheme, StaticAuthResolver};
    use opi_ai::providers::openai_responses::OpenAiResponsesProvider;
    use opi_ai::types::deferred::DeferredHandle;
    use opi_coding_agent::provider_factory::build_harness_collection;

    let collection = build_harness_collection(
        vec![(
            Box::new(OpenAiResponsesProvider::new(Some(
                "http://127.0.0.1:1/v1".into(),
            ))) as Box<dyn Provider>,
            Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                secrecy::SecretString::from("test-key"),
            )),
        )],
        Vec::new(),
        Vec::new(),
        "openai-responses",
        None,
    );

    let handle = DeferredHandle {
        provider: "openai-responses".into(),
        model_id: "o3".into(),
        api: "openai-responses".into(),
        id: "resp_test".into(),
        expires_at_ms: None,
        poll_after_ms: None,
        data: None,
    };
    collection
        .validate_deferred_handle(&handle, None)
        .expect("provider factory registers the OpenAI deferred route");
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
        "Models::from_registry",
        "Models::new",
        "fn parse_model_spec",
        "fn build_provider",
        "fn build_runtime_route",
        "fn build_list_models_metadata",
        "fn openai_compatible_model_catalog",
        "fn build_openai_compatible_profile",
        "fn build_collection_for_listing",
        "fn available_model_entries",
        "fn logout_stored_credential",
        "fn build_collection_for_listing_with_store",
        "fn build_harness_collection",
        "fn materialize_active_overrides",
        "fn resolve_env_name",
        "fn aws_credentials_path",
        "fn aws_config_path",
        "fn aws_home_dir",
        "fn profile_api_key_env_default",
        "fn auth_descriptor_for",
        "fn auth_descriptor_for_profile",
        "fn compat_metadata_for",
        "struct ListingMetadataProvider",
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

// ---------------------------------------------------------------------------
// Task 20.36 (AIR-022 / V-11): retired Chat compat keys error with migration
// pointers at every product config position, before provider construction.
// ---------------------------------------------------------------------------

#[test]
fn openai_compatible_profile_retired_keys_fail_with_migration_errors() {
    use opi_coding_agent::config::load_config_file;

    for (key, value, pointer) in [
        ("system_role_override", "\"developer\"", "instruction_role"),
        ("reasoning_effort", "\"medium\"", "per-call thinking intent"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("retired.toml");
        std::fs::write(
            &path,
            format!(
                r#"
[providers.openai_compatible.legacy]
api_key_env = "LEGACY_KEY"
base_url = "https://legacy.example.com"
{key} = {value}

[[providers.openai_compatible.legacy.models]]
id = "m"
context_window = 8000
max_output_tokens = 1024
"#
            ),
        )
        .unwrap();
        let error = load_config_file(&path)
            .expect_err("retired profile keys fail at load with a migration pointer");
        let message = error.to_string();
        assert!(
            message.contains(key) && message.contains(pointer),
            "migration error must name the retired key and its replacement: {message}"
        );
    }
}

#[test]
fn openai_compatible_model_retired_role_key_fails_with_migration_error() {
    use opi_coding_agent::config::load_config_file;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("retired-model.toml");
    std::fs::write(
        &path,
        r#"
[providers.openai_compatible.legacy]
api_key_env = "LEGACY_KEY"
base_url = "https://legacy.example.com"

[[providers.openai_compatible.legacy.models]]
id = "m"
context_window = 8000
max_output_tokens = 1024
system_role_override = "developer"
"#,
    )
    .unwrap();
    let error = load_config_file(&path)
        .expect_err("model-level retired keys fail with a migration pointer");
    let message = error.to_string();
    assert!(
        message.contains("system_role_override") && message.contains("instruction_role"),
        "migration error names the replacement: {message}"
    );
}

#[test]
fn custom_chat_model_compat_retired_keys_fail_typed() {
    use opi_coding_agent::config::load_config_file;

    for (key, value) in [
        ("system_role_override", "\"developer\""),
        ("supports_store", "true"),
        ("supports_developer_role", "true"),
        ("reasoning_effort", "\"low\""),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("retired-compat.toml");
        std::fs::write(
            &path,
            format!(
                r#"
[providers.custom.legacy]
api_key_env = "LEGACY_KEY"

[[providers.custom.legacy.models]]
id = "m"
api = "openai-completions"
context_window = 8000
max_output_tokens = 1024

[providers.custom.legacy.models.compat]
api = "openai-completions"
{key} = {value}
"#
            ),
        )
        .unwrap();
        let error = load_config_file(&path)
            .expect_err("chat-wire compat retired keys fail at load, not serde-ignore");
        let message = error.to_string();
        assert!(
            message.contains("retired key"),
            "each retired compat key errors with a migration pointer: {message}"
        );
    }
}

#[test]
fn custom_chat_model_compat_normalized_keys_parse() {
    use opi_ai::model_info::{ChatInstructionRole, ChatStoreField, ChatThinkingFormat};
    use opi_coding_agent::config::load_config_file;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("normalized-compat.toml");
    std::fs::write(
        &path,
        r#"
[providers.custom.norm]
api_key_env = "NORM_KEY"
base_url = "https://norm.example.com"

[[providers.custom.norm.models]]
id = "m"
api = "openai-completions"
context_window = 8000
max_output_tokens = 1024

[providers.custom.norm.models.compat]
api = "openai-completions"
store_field = "send_false"
instruction_role = "developer_for_reasoning"
thinking_format = "zai"
supports_reasoning_effort = false
"#,
    )
    .unwrap();
    let config = load_config_file(&path).expect("normalized keys parse");
    let model = &config.providers.custom["norm"].models[0];
    assert!(matches!(
        &model.compat,
        opi_ai::WireCompat::OpenAiCompletions(compat)
            if compat.store == Some(ChatStoreField::SendFalse)
                && compat.instruction_role == Some(ChatInstructionRole::DeveloperForReasoning)
                && compat.thinking_format == Some(ChatThinkingFormat::Zai)
                && compat.supports_reasoning_effort == Some(false)
    ));
}
