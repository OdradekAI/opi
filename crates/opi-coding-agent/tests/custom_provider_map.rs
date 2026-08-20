use opi_ai::model_info::{ThinkingLevel, WireApi, WireCompat};
use opi_coding_agent::config::{ConfigError, ConfigSource, load_config_file, resolve_config};

static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_MUTEX
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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
        // SAFETY: callers hold ENV_MUTEX for this guard's entire lifetime.
        unsafe { std::env::set_var(key, value) };
        guard
    }

    fn remove(key: &str) -> Self {
        let guard = Self {
            key: key.to_owned(),
            original: std::env::var_os(key),
        };
        // SAFETY: callers hold ENV_MUTEX for this guard's entire lifetime.
        unsafe { std::env::remove_var(key) };
        guard
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.original {
            // SAFETY: the guard is dropped before its caller releases ENV_MUTEX.
            Some(value) => unsafe { std::env::set_var(&self.key, value) },
            // SAFETY: the guard is dropped before its caller releases ENV_MUTEX.
            None => unsafe { std::env::remove_var(&self.key) },
        }
    }
}

fn write_config(root: &std::path::Path, relative: &str, contents: &str) -> std::path::PathBuf {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, contents).unwrap();
    path
}

#[test]
fn custom_provider_api_and_base_url_precedence() {
    let root = tempfile::tempdir().unwrap();
    let path = write_config(
        root.path(),
        "config.toml",
        r#"
[providers.custom.acme]
name = "Acme"
base_url = "https://provider.example"
api_key_env = "ACME_API_KEY"
auth_scheme = "bearer"
api = "openai-completions"

[[providers.custom.acme.models]]
id = "inherited"
display_name = "Inherited"
context_window = 128000
max_output_tokens = 4096

[[providers.custom.acme.models]]
id = "anthropic"
display_name = "Anthropic"
api = "anthropic-messages"
base_url = "https://model.example"
context_window = 200000
max_output_tokens = 8192
"#,
    );

    let config = load_config_file(&path).unwrap();
    let provider = &config.providers.custom["acme"];
    assert_eq!(provider.id, "acme");
    assert_eq!(provider.name, "Acme");
    assert_eq!(provider.models[0].wire_api, WireApi::OpenAiCompletions);
    assert_eq!(provider.models[0].base_url, None);
    assert_eq!(provider.models[1].wire_api, WireApi::AnthropicMessages);
    assert_eq!(
        provider.models[1].base_url.as_deref(),
        Some("https://model.example")
    );
    assert_eq!(
        provider.base_url.as_deref(),
        Some("https://provider.example")
    );
}

#[test]
fn custom_provider_decodes_three_wires_thinking_compat_and_pricing() {
    let root = tempfile::tempdir().unwrap();
    let path = write_config(
        root.path(),
        "config.toml",
        r#"
[providers.custom.acme]
name = "Acme"
base_url = "https://api.acme.example"
api_key_env = "ACME_API_KEY"
auth_scheme = "bearer"
headers = { "X-Acme" = "opi" }

[[providers.custom.acme.models]]
id = "claude-model"
display_name = "Claude Model"
api = "anthropic-messages"
context_window = 200000
max_output_tokens = 32000
supports_images = true
supports_streaming = true
supports_thinking = true
thinking_level_map = { off = true, minimal = "low", xhigh = false, max = false }

[providers.custom.acme.models.compat]
api = "anthropic-messages"
supports_eager_tool_input_streaming = false

[providers.custom.acme.models.pricing]
input = 3.0
output = 15.0
cache_read = 0.3
cache_write = 3.75

[[providers.custom.acme.models.pricing.tiers]]
input_tokens_above = 272000
input = 6.0
output = 22.5
cache_read = 0.6
cache_write = 7.5

[[providers.custom.acme.models]]
id = "chat-model"
api = "openai-completions"
context_window = 128000
max_output_tokens = 8192

[providers.custom.acme.models.compat]
api = "openai-completions"
max_tokens_field = "max_completion_tokens"
strict_tool_schema = true
reasoning_effort = "low"
supports_reasoning_effort = true

[[providers.custom.acme.models]]
id = "responses-model"
api = "openai-responses"
context_window = 128000
max_output_tokens = 8192

[providers.custom.acme.models.compat]
api = "openai-responses"
store = false
reasoning_effort = "high"
strict_tools = true
send_session_id_header = false
"#,
    );

    let config = load_config_file(&path).unwrap();
    let provider = &config.providers.custom["acme"];
    assert_eq!(provider.api_key_env, "ACME_API_KEY");
    assert_eq!(provider.headers, vec![("X-Acme".into(), "opi".into())]);
    assert_eq!(provider.models.len(), 3);

    let claude = &provider.models[0];
    assert_eq!(
        claude
            .thinking_level_map
            .resolve(ThinkingLevel::None)
            .unwrap(),
        None
    );
    assert_eq!(
        claude
            .thinking_level_map
            .resolve(ThinkingLevel::Minimal)
            .unwrap(),
        Some("low".into())
    );
    assert!(
        claude
            .thinking_level_map
            .resolve(ThinkingLevel::XHigh)
            .is_err()
    );
    let pricing = claude.pricing.as_ref().unwrap();
    assert_eq!(pricing.base.input_cost_per_mtok, 3.0);
    assert_eq!(pricing.effective(272000).input_cost_per_mtok, 3.0);
    assert_eq!(pricing.effective(272001).input_cost_per_mtok, 6.0);
    assert!(matches!(
        claude.compat,
        WireCompat::AnthropicMessages(ref compat)
            if !compat.supports_eager_tool_input_streaming
    ));
    assert!(matches!(
        provider.models[1].compat,
        WireCompat::OpenAiCompletions(ref compat)
            if compat.max_tokens_field == "max_completion_tokens"
                && compat.strict_tool_schema
                && compat.reasoning_effort.as_deref() == Some("low")
    ));
    assert!(matches!(
        provider.models[2].compat,
        WireCompat::OpenAiResponses(ref compat)
            if compat.store == Some(false)
                && compat.reasoning_effort.as_deref() == Some("high")
                && compat.strict_tools
                && !compat.send_session_id_header
    ));
}

#[test]
fn invalid_custom_provider_contracts_fail_at_load() {
    let cases = [
        (
            "unknown wire",
            r#"[providers.custom.bad]
api_key_env = "KEY"
auth_scheme = "bearer"
api = "not-a-wire"
[[providers.custom.bad.models]]
id = "m"
context_window = 1
max_output_tokens = 1
"#,
        ),
        (
            "disabled wire",
            r#"[providers.custom.bad]
api_key_env = "KEY"
auth_scheme = "bearer"
api = "openai-codex-responses"
[[providers.custom.bad.models]]
id = "m"
context_window = 1
max_output_tokens = 1
"#,
        ),
        (
            "missing api",
            r#"[providers.custom.bad]
api_key_env = "KEY"
auth_scheme = "bearer"
[[providers.custom.bad.models]]
id = "m"
context_window = 1
max_output_tokens = 1
"#,
        ),
        (
            "duplicate model",
            r#"[providers.custom.bad]
api_key_env = "KEY"
auth_scheme = "bearer"
api = "openai-completions"
[[providers.custom.bad.models]]
id = "m"
context_window = 1
max_output_tokens = 1
[[providers.custom.bad.models]]
id = "m"
context_window = 1
max_output_tokens = 1
"#,
        ),
        (
            "compat mismatch",
            r#"[providers.custom.bad]
api_key_env = "KEY"
auth_scheme = "bearer"
api = "openai-completions"
[[providers.custom.bad.models]]
id = "m"
context_window = 1
max_output_tokens = 1
[providers.custom.bad.models.compat]
api = "anthropic-messages"
"#,
        ),
        (
            "invalid limits",
            r#"[providers.custom.bad]
api_key_env = "KEY"
auth_scheme = "bearer"
api = "openai-completions"
[[providers.custom.bad.models]]
id = "m"
context_window = 0
max_output_tokens = 0
"#,
        ),
        (
            "invalid tier",
            r#"[providers.custom.bad]
api_key_env = "KEY"
auth_scheme = "bearer"
api = "openai-completions"
[[providers.custom.bad.models]]
id = "m"
context_window = 1
max_output_tokens = 1
[providers.custom.bad.models.pricing]
input = 1
output = 1
cache_read = 0
cache_write = 0
[[providers.custom.bad.models.pricing.tiers]]
input_tokens_above = 0
input = 1
output = 1
cache_read = 0
cache_write = 0
"#,
        ),
        (
            "reserved header",
            r#"[providers.custom.bad]
api_key_env = "KEY"
auth_scheme = "bearer"
api = "openai-completions"
headers = { authorization = "secret" }
[[providers.custom.bad.models]]
id = "m"
context_window = 1
max_output_tokens = 1
"#,
        ),
        (
            "invalid header name",
            r#"[providers.custom.bad]
api_key_env = "KEY"
auth_scheme = "bearer"
api = "openai-completions"
headers = { "bad:name" = "value" }
[[providers.custom.bad.models]]
id = "m"
context_window = 1
max_output_tokens = 1
"#,
        ),
        (
            "invalid auth combination",
            r#"[providers.custom.bad]
api_key_env = "KEY"
auth_scheme = "api-key"
api = "openai-responses"
[[providers.custom.bad.models]]
id = "m"
context_window = 1
max_output_tokens = 1
"#,
        ),
    ];

    for (name, contents) in cases {
        let root = tempfile::tempdir().unwrap();
        let path = write_config(root.path(), "config.toml", contents);
        let error = load_config_file(&path).unwrap_err().to_string();
        assert!(
            error.contains("bad"),
            "{name}: error must carry provider id: {error}"
        );
        assert!(!error.contains("secret"), "{name}: secret leaked: {error}");
    }
}

#[test]
fn wrong_wire_custom_compat_fields_fail_at_load() {
    let cases = [
        (
            "anthropic rejects Responses field",
            "anthropic-messages",
            "strict_tools = true",
            "compat.strict_tools",
        ),
        (
            "Responses rejects Chat field",
            "openai-responses",
            "max_tokens_field = \"max_completion_tokens\"",
            "compat.max_tokens_field",
        ),
        (
            "Chat rejects Anthropic field",
            "openai-completions",
            "supports_temperature = false",
            "compat.supports_temperature",
        ),
    ];

    for (name, api, compat, expected_field) in cases {
        let root = tempfile::tempdir().unwrap();
        let path = write_config(
            root.path(),
            "config.toml",
            &format!(
                r#"
[providers.custom.bad]
base_url = "https://api.example"
api_key_env = "BAD_API_KEY"
auth_scheme = "bearer"

[[providers.custom.bad.models]]
id = "m"
api = "{api}"
context_window = 128000
max_output_tokens = 4096

[providers.custom.bad.models.compat]
{compat}
"#
            ),
        );
        let error = load_config_file(&path).expect_err(name);
        match &error {
            ConfigError::InvalidCustomProvider {
                provider,
                model,
                field,
                ..
            } => {
                assert_eq!(provider, "bad", "{name}");
                assert_eq!(model.as_deref(), Some("m"), "{name}");
                assert_eq!(*field, expected_field, "{name}");
            }
            _ => panic!("{name}: {error}"),
        }
    }
}

#[test]
fn custom_provider_final_merge_validates_once() {
    let root = tempfile::tempdir().unwrap();
    let user = write_config(
        root.path(),
        "user/config.toml",
        r#"
[providers.custom.acme]
api_key_env = "ACME_API_KEY"
auth_scheme = "bearer"
[[providers.custom.acme.models]]
id = "m"
context_window = 128000
max_output_tokens = 4096
"#,
    );
    write_config(
        root.path(),
        "project/.opi/config.toml",
        r#"
[providers.custom.acme]
api = "openai-completions"
base_url = "https://api.acme.example"
"#,
    );

    let config = resolve_config(ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: None,
        project_dir: Some(root.path().join("project")),
        user_config_path: Some(user),
    })
    .unwrap();
    assert_eq!(
        config.providers.custom["acme"].models[0].wire_api,
        WireApi::OpenAiCompletions
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serializes process-env mutation; awaited local HTTP work never re-acquires this lock.
async fn custom_mapped_provider_routes_three_wires_with_lazy_shared_env_auth() {
    use futures_util::StreamExt;
    use opi_ai::AssistantStreamEvent;
    use opi_ai::provider::{CacheRetention, Request, ThinkingConfig};
    use opi_ai::{AuthProvenanceSource, CompatMetadata, ProviderCollection};
    use opi_coding_agent::credential_store::FakeKeyringBackend;
    use opi_coding_agent::provider_factory::build_provider_bundle;
    use tokio_util::sync::CancellationToken;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let _env_lock = lock_env();
    let server = MockServer::start().await;
    for (route, body) in [
        (
            "/anthropic/v1/messages",
            "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"a\",\"model\":\"claude\",\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\n\
             event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
        ),
        (
            "/v1/chat/completions",
            "data: {\"id\":\"c\",\"model\":\"chat\",\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\n\n\
             data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n\
             data: [DONE]\n\n",
        ),
        (
            "/v1/responses",
            "event: response.created\ndata: {\"type\":\"response.created\",\"response\":{\"id\":\"r\",\"model\":\"responses\"}}\n\n\
             event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"r\",\"model\":\"responses\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n",
        ),
    ] {
        Mock::given(method("POST"))
            .and(path(route))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(body),
            )
            .mount(&server)
            .await;
    }

    let env_name = "OPI_TEST_CUSTOM_MAPPED_SHARED_AUTH_1416";
    let _env = EnvVarGuard::remove(env_name);
    let root = tempfile::tempdir().unwrap();
    let path = write_config(
        root.path(),
        "config.toml",
        &format!(
            r#"
[defaults]
model = "acme:claude"

[providers.custom.acme]
base_url = "{base}"
api_key_env = "{env_name}"
auth_scheme = "bearer"
headers = {{ "X-Acme" = "opi" }}

[[providers.custom.acme.models]]
id = "claude"
api = "anthropic-messages"
base_url = "{base}/anthropic"
context_window = 200000
max_output_tokens = 8192

[[providers.custom.acme.models]]
id = "chat"
api = "openai-completions"
context_window = 128000
max_output_tokens = 8192

[[providers.custom.acme.models]]
id = "responses"
api = "openai-responses"
context_window = 128000
max_output_tokens = 8192
"#,
            base = server.uri()
        ),
    );
    let config = load_config_file(&path).unwrap();
    let bundle = build_provider_bundle(
        &config,
        root.path().to_path_buf(),
        Box::new(|| Box::new(FakeKeyringBackend::new())),
    )
    .await
    .expect("custom provider bundle constructs before auth exists");
    assert_eq!(bundle.provider.id(), "acme");
    assert_eq!(bundle.provider.models().len(), 3);
    // Phase 17.5: auth moved off the provider object onto the collection route.
    // Register the bundle's resolver so prepare_call resolves env auth per
    // logical call (one per model), observing the env value current at each call.
    let mut collection = ProviderCollection::new();
    collection
        .register_route(
            bundle.provider,
            bundle.auth_resolver,
            AuthProvenanceSource::Environment {
                name: env_name.into(),
            },
            CompatMetadata::default(),
        )
        .expect("register acme route");

    for (index, model) in ["claude", "chat", "responses"].into_iter().enumerate() {
        // SAFETY: serialized by ENV_MUTEX and restored by EnvVarGuard.
        unsafe { std::env::set_var(env_name, format!("token-{index}")) };
        let request = Request {
            model: format!("acme:{model}"),
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
        };
        let spec = request.model.clone();
        let prepared = collection
            .prepare_call(&spec, request)
            .await
            .expect("prepare_call resolves the acme route");
        let mut stream = prepared.start_attempt().expect("start_attempt");
        while let Some(event) = stream.next().await {
            match event {
                Ok(
                    AssistantStreamEvent::Done { ref message, .. }
                    | AssistantStreamEvent::Error { ref message, .. },
                ) => {
                    assert_eq!(message.provider, "acme");
                    break;
                }
                Err(error) => panic!("mapped provider stream failed: {error}"),
                _ => {}
            }
        }
    }

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 3);
    for (index, request) in requests.iter().enumerate() {
        let expected_auth = format!("Bearer token-{index}");
        assert_eq!(
            request
                .headers
                .get("authorization")
                .and_then(|value| value.to_str().ok()),
            Some(expected_auth.as_str())
        );
        assert_eq!(
            request
                .headers
                .get("x-acme")
                .and_then(|value| value.to_str().ok()),
            Some("opi")
        );
    }
    assert!(
        requests[0].headers.get("anthropic-beta").is_none(),
        "custom Anthropic Bearer must not inherit direct Anthropic OAuth headers"
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serializes process-env mutation; awaited local HTTP work never re-acquires this lock.
async fn production_custom_provider_reloads_store_then_env_auth_for_each_stream() {
    use futures_util::StreamExt;
    use opi_ai::credential::Credential;
    use opi_ai::provider::{CacheRetention, Request, ThinkingConfig};
    use opi_ai::{AuthProvenanceSource, CompatMetadata, ProviderCollection};
    use opi_coding_agent::credential_store::{
        FakeKeyringBackend, KEYCHAIN_PRESENCE_SERVICE, KEYCHAIN_SERVICE, KeyringBackend,
    };
    use opi_coding_agent::provider_factory::build_provider_bundle;
    use secrecy::SecretString;
    use tokio_util::sync::CancellationToken;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let _env_lock = lock_env();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(
                    "data: {\"id\":\"c\",\"model\":\"chat\",\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\n\n\
                     data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n\
                     data: [DONE]\n\n",
                ),
        )
        .expect(2)
        .mount(&server)
        .await;

    let env_name = "OPI_TEST_CUSTOM_ROTATING_SOURCE_1416";
    let _env = EnvVarGuard::set(env_name, "env-token-after-rotation");

    let backend = FakeKeyringBackend::new();
    backend.seed_raw(KEYCHAIN_PRESENCE_SERVICE, "acme", "api_key");
    backend.seed_credential(
        KEYCHAIN_SERVICE,
        "acme",
        &Credential::ApiKey(SecretString::new("stored-token-at-build".into())),
    );
    let injected_backend = backend.clone();
    let root = tempfile::tempdir().unwrap();
    let path = write_config(
        root.path(),
        "config.toml",
        &format!(
            r#"
[defaults]
model = "acme:chat"

[providers.custom.acme]
base_url = "{base}"
api_key_env = "{env_name}"
auth_scheme = "bearer"

[[providers.custom.acme.models]]
id = "chat"
api = "openai-completions"
context_window = 128000
max_output_tokens = 8192
"#,
            base = server.uri()
        ),
    );
    let config = load_config_file(&path).unwrap();
    let bundle = build_provider_bundle(
        &config,
        root.path().to_path_buf(),
        Box::new(move || Box::new(injected_backend)),
    )
    .await
    .expect("custom provider builds with the stored credential present");

    let request = || Request {
        model: "acme:chat".into(),
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
    };
    // Phase 17.5: auth moved off the provider object onto the collection route.
    // Each prepare_call resolves auth once: the first call reads the stored
    // credential; after the store entries are deleted, the second call falls
    // back to the env value (a new logical call, not a mid-retry re-resolution).
    let mut collection = ProviderCollection::new();
    collection
        .register_route(
            bundle.provider,
            bundle.auth_resolver,
            AuthProvenanceSource::CredentialStore {
                kind: "keychain".into(),
            },
            CompatMetadata::default(),
        )
        .expect("register acme route");
    let prepared = collection
        .prepare_call("acme:chat", request())
        .await
        .expect("first prepare_call");
    let mut first = prepared.start_attempt().expect("first start_attempt");
    while let Some(event) = first.next().await {
        event.unwrap();
    }

    KeyringBackend::delete(&backend, KEYCHAIN_SERVICE, "acme").unwrap();
    KeyringBackend::delete(&backend, KEYCHAIN_PRESENCE_SERVICE, "acme").unwrap();
    let prepared = collection
        .prepare_call("acme:chat", request())
        .await
        .expect("second prepare_call");
    let mut second = prepared.start_attempt().expect("second start_attempt");
    while let Some(event) = second.next().await {
        event.unwrap();
    }

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0]
            .headers
            .get("authorization")
            .and_then(|value| value.to_str().ok()),
        Some("Bearer stored-token-at-build")
    );
    assert_eq!(
        requests[1]
            .headers
            .get("authorization")
            .and_then(|value| value.to_str().ok()),
        Some("Bearer env-token-after-rotation")
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serializes process-env mutation; awaited local HTTP work never re-acquires this lock.
async fn production_custom_provider_redacts_401_and_403_for_all_three_wires() {
    use futures_util::StreamExt;
    use opi_ai::provider::{
        CacheRetention, ProviderError, ProviderErrorSummary, Request, ThinkingConfig,
    };
    use opi_ai::{AuthProvenanceSource, CompatMetadata, ProviderCollection};
    use opi_coding_agent::credential_store::FakeKeyringBackend;
    use opi_coding_agent::provider_factory::build_provider_bundle;
    use tokio_util::sync::CancellationToken;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let _env_lock = lock_env();
    let env_name = "OPI_TEST_CUSTOM_AUTH_FAILURE_1416";
    let _env = EnvVarGuard::set(env_name, "factory-auth-token-DO-NOT-LEAK");

    for status in [401, 403] {
        for (api, model, route) in [
            ("anthropic-messages", "claude", "/v1/messages"),
            ("openai-completions", "chat", "/v1/chat/completions"),
            ("openai-responses", "responses", "/v1/responses"),
        ] {
            let server = MockServer::start().await;
            let response_canary = format!("echo-canary-{status}-{model}-DO-NOT-LEAK");
            Mock::given(method("POST"))
                .and(path(route))
                .respond_with(
                    ResponseTemplate::new(status).set_body_string(response_canary.clone()),
                )
                .expect(1)
                .mount(&server)
                .await;

            let root = tempfile::tempdir().unwrap();
            let config_path = write_config(
                root.path(),
                "config.toml",
                &format!(
                    r#"
[defaults]
model = "acme:{model}"

[providers.custom.acme]
base_url = "{base}"
api_key_env = "{env_name}"
auth_scheme = "bearer"

[[providers.custom.acme.models]]
id = "{model}"
api = "{api}"
context_window = 128000
max_output_tokens = 8192
"#,
                    base = server.uri()
                ),
            );
            let config = load_config_file(&config_path).unwrap();
            let bundle = build_provider_bundle(
                &config,
                root.path().to_path_buf(),
                Box::new(|| Box::new(FakeKeyringBackend::new())),
            )
            .await
            .expect("production custom provider bundle");
            let request = Request {
                model: format!("acme:{model}"),
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
            };

            // Phase 17.5: route through the collection so prepare_call resolves
            // auth; the 401/403 redaction is preserved by the provider error path.
            let mut collection = ProviderCollection::new();
            collection
                .register_route(
                    bundle.provider,
                    bundle.auth_resolver,
                    AuthProvenanceSource::Environment {
                        name: env_name.into(),
                    },
                    CompatMetadata::default(),
                )
                .expect("register acme route");
            let spec = request.model.clone();
            let prepared = collection
                .prepare_call(&spec, request)
                .await
                .expect("prepare_call");
            let error = prepared
                .start_attempt()
                .expect("start_attempt")
                .next()
                .await
                .expect("auth failure stream item")
                .expect_err("401/403 must fail");
            assert!(
                matches!(
                    &error,
                    ProviderError::AuthFailed(reason)
                        if reason == &ProviderErrorSummary::authentication_rejected()
                ),
                "{api} {status} must be a bodyless static-auth failure: {error:?}"
            );
            assert!(
                !error.is_retryable(),
                "{api} {status} auth failure must not be retryable"
            );
            let rendered = format!("{error:?} {error}");
            assert!(
                !rendered.contains(&response_canary),
                "{api} {status} leaked echoed response body: {rendered}"
            );
            assert!(
                !rendered.contains("factory-auth-token-DO-NOT-LEAK"),
                "{api} {status} leaked credential: {rendered}"
            );
        }
    }
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serializes process-env mutation; awaited local HTTP work never re-acquires this lock.
async fn custom_responses_affinity_requires_explicit_true_compat() {
    use futures_util::StreamExt;
    use opi_ai::provider::{CacheRetention, Request, ThinkingConfig};
    use opi_ai::{AuthProvenanceSource, CompatMetadata, ProviderCollection};
    use opi_coding_agent::credential_store::FakeKeyringBackend;
    use opi_coding_agent::provider_factory::build_provider_bundle;
    use tokio_util::sync::CancellationToken;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let _env_lock = lock_env();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(
                    "event: response.created\ndata: {\"type\":\"response.created\",\"response\":{\"id\":\"r\",\"model\":\"responses\"}}\n\n\
                     event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"r\",\"model\":\"responses\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n",
                ),
        )
        .expect(2)
        .mount(&server)
        .await;

    let env_name = "OPI_TEST_CUSTOM_RESPONSES_AFFINITY_1416";
    let _env = EnvVarGuard::set(env_name, "custom-responses-token");
    let root = tempfile::tempdir().unwrap();
    let path = write_config(
        root.path(),
        "config.toml",
        &format!(
            r#"
[defaults]
model = "acme:omitted"

[providers.custom.acme]
base_url = "{base}"
api_key_env = "{env_name}"
auth_scheme = "bearer"

[[providers.custom.acme.models]]
id = "omitted"
api = "openai-responses"
context_window = 128000
max_output_tokens = 8192

[[providers.custom.acme.models]]
id = "enabled"
api = "openai-responses"
context_window = 128000
max_output_tokens = 8192

[providers.custom.acme.models.compat]
send_session_id_header = true
"#,
            base = server.uri()
        ),
    );
    let config = load_config_file(&path).unwrap();
    let bundle = build_provider_bundle(
        &config,
        root.path().to_path_buf(),
        Box::new(|| Box::new(FakeKeyringBackend::new())),
    )
    .await
    .expect("custom responses provider bundle");
    // Phase 17.5: route through the collection so prepare_call resolves env auth.
    let mut collection = ProviderCollection::new();
    collection
        .register_route(
            bundle.provider,
            bundle.auth_resolver,
            AuthProvenanceSource::Environment {
                name: env_name.into(),
            },
            CompatMetadata::default(),
        )
        .expect("register acme route");

    for (model, session_id) in [
        ("omitted", "session-custom-omitted"),
        ("enabled", "session-custom-enabled"),
    ] {
        let request = Request {
            model: format!("acme:{model}"),
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
            session_id: Some(session_id.into()),
        };
        let spec = request.model.clone();
        let prepared = collection
            .prepare_call(&spec, request)
            .await
            .expect("prepare_call");
        let mut stream = prepared.start_attempt().expect("start_attempt");
        while let Some(event) = stream.next().await {
            event.unwrap();
        }
    }

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    let omitted_body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert!(omitted_body.get("prompt_cache_key").is_none());
    for name in ["session_id", "x-client-request-id"] {
        assert!(
            requests[0].headers.get(name).is_none(),
            "omitted compat must not emit {name}"
        );
    }

    let enabled_body: serde_json::Value = serde_json::from_slice(&requests[1].body).unwrap();
    assert_eq!(
        enabled_body["prompt_cache_key"],
        serde_json::Value::String("session-custom-enabled".into())
    );
    assert_eq!(
        requests[1]
            .headers
            .get("session_id")
            .and_then(|value| value.to_str().ok()),
        Some("session-custom-enabled")
    );
    let request_id = requests[1]
        .headers
        .get("x-client-request-id")
        .and_then(|value| value.to_str().ok())
        .expect("explicit opt-in request id");
    assert_ne!(request_id, "session-custom-enabled");
}
