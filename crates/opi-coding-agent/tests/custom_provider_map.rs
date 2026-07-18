use opi_ai::model_info::{ThinkingLevel, WireApi, WireCompat};
use opi_coding_agent::config::{ConfigError, ConfigSource, load_config_file, resolve_config};

static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
    use opi_coding_agent::provider_factory::build_provider;
    use tokio_util::sync::CancellationToken;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let _env_guard = ENV_MUTEX.lock().expect("env lock");
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
    let original = std::env::var_os(env_name);
    // SAFETY: this test uses a task-unique environment variable and restores it.
    unsafe { std::env::remove_var(env_name) };
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
    let provider = build_provider(&config).expect("custom provider constructs before auth exists");
    assert_eq!(provider.id(), "acme");
    assert_eq!(provider.models().len(), 3);

    for (index, model) in ["claude", "chat", "responses"].into_iter().enumerate() {
        // SAFETY: this test uses a task-unique environment variable and restores it.
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
        let mut stream = provider.stream(request);
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

    match original {
        Some(value) => {
            // SAFETY: restoring the task-unique environment variable.
            unsafe { std::env::set_var(env_name, value) };
        }
        None => {
            // SAFETY: restoring the task-unique environment variable.
            unsafe { std::env::remove_var(env_name) };
        }
    }
}
