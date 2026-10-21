//! Bedrock provider wiring tests (task 3.1).
//!
//! Tests Bedrock config parsing and provider construction through build_provider.
//! No live AWS calls.

use std::fs;

use opi_coding_agent::config::{OpiConfig, load_config_file};

fn write_temp_config(dir: &std::path::Path, contents: &str) -> std::path::PathBuf {
    let path = dir.join("config.toml");
    fs::write(&path, contents).unwrap();
    path
}

// ---------------------------------------------------------------------------
// Config parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_bedrock_config_with_region() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.bedrock]
region = "eu-west-1"
"#,
    );
    let config = load_config_file(&path).unwrap();
    assert_eq!(
        config.providers.bedrock.region.as_deref(),
        Some("eu-west-1")
    );
}

#[test]
fn parse_bedrock_config_with_access_key() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.bedrock]
access_key_id = "AKIAEXAMPLE"
secret_access_key_env = "MY_SECRET_KEY"
region = "ap-southeast-1"
"#,
    );
    let config = load_config_file(&path).unwrap();
    assert_eq!(
        config
            .providers
            .bedrock
            .access_key_id
            .as_ref()
            .map(secrecy::ExposeSecret::expose_secret),
        Some("AKIAEXAMPLE")
    );
    assert_eq!(
        config.providers.bedrock.secret_access_key_env.as_deref(),
        Some("MY_SECRET_KEY")
    );
    assert_eq!(
        config.providers.bedrock.region.as_deref(),
        Some("ap-southeast-1")
    );
}

#[test]
fn parse_bedrock_config_with_session_token() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.bedrock]
region = "us-east-1"
session_token_env = "AWS_SESSION_TOKEN"
"#,
    );
    let config = load_config_file(&path).unwrap();
    assert_eq!(
        config.providers.bedrock.session_token_env.as_deref(),
        Some("AWS_SESSION_TOKEN")
    );
}

#[test]
fn parse_bedrock_config_with_profile() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.bedrock]
profile = "production"
region = "us-west-2"
"#,
    );
    let config = load_config_file(&path).unwrap();
    assert_eq!(
        config.providers.bedrock.profile.as_deref(),
        Some("production")
    );
}

#[test]
fn parse_bedrock_config_with_base_url() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.bedrock]
region = "us-east-1"
base_url = "https://custom-bedrock-endpoint.example.com"
"#,
    );
    let config = load_config_file(&path).unwrap();
    assert_eq!(
        config.providers.bedrock.base_url.as_deref(),
        Some("https://custom-bedrock-endpoint.example.com")
    );
}

#[test]
fn default_bedrock_config_has_no_explicit_credentials() {
    let config = OpiConfig::default();
    assert!(config.providers.bedrock.access_key_id.is_none());
    assert!(config.providers.bedrock.secret_access_key_env.is_none());
    assert!(config.providers.bedrock.session_token_env.is_none());
    assert!(config.providers.bedrock.profile.is_none());
    assert!(config.providers.bedrock.base_url.is_none());
}

#[test]
fn parse_bedrock_config_with_proxy() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.bedrock]
region = "us-east-1"

[providers.bedrock.proxy]
url = "http://proxy.example.com:8080"
"#,
    );
    let config = load_config_file(&path).unwrap();
    let proxy = config
        .providers
        .bedrock
        .proxy
        .as_ref()
        .expect("proxy should be set");
    assert_eq!(proxy.url, "http://proxy.example.com:8080");
}

// ---------------------------------------------------------------------------
// Factory route -> migrated local AWS chain -> real adapter (task 20.19.1)
// ---------------------------------------------------------------------------

/// Scoped process-env guard for the AWS variables the factory's Bedrock
/// resolver reads. This is the only env-touching test in this binary.
struct ScopedAwsEnv(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl ScopedAwsEnv {
    fn set(values: &[(&'static str, &str)]) -> Self {
        const NAMES: &[&str] = &[
            "AWS_ACCESS_KEY_ID",
            "AWS_SECRET_ACCESS_KEY",
            "AWS_SESSION_TOKEN",
            "AWS_REGION",
            "AWS_DEFAULT_REGION",
            "AWS_PROFILE",
            "AWS_SHARED_CREDENTIALS_FILE",
            "AWS_CONFIG_FILE",
            "AWS_BEARER_TOKEN_BEDROCK",
        ];
        let original = NAMES
            .iter()
            .map(|name| (*name, std::env::var_os(name)))
            .collect();
        for name in NAMES {
            // SAFETY: this test owns the process env for its duration and
            // restores every variable below; no other test in this binary
            // reads these variables.
            unsafe { std::env::remove_var(name) };
        }
        for (name, value) in values {
            // SAFETY: see above.
            unsafe { std::env::set_var(name, value) };
        }
        Self(original)
    }
}

impl Drop for ScopedAwsEnv {
    fn drop(&mut self) {
        for (name, value) in self.0.drain(..) {
            match value {
                // SAFETY: restoring the value captured at construction.
                Some(value) => unsafe { std::env::set_var(name, value) },
                // SAFETY: see above.
                None => unsafe { std::env::remove_var(name) },
            }
        }
    }
}

#[tokio::test]
async fn factory_bedrock_route_resolves_local_credentials_and_streams() {
    use futures_util::StreamExt;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // Isolated shared-config files so the chain cannot read a real ~/.aws.
    let files = tempfile::tempdir().unwrap();
    let credentials_file = files.path().join("credentials");
    let config_file = files.path().join("config");
    std::fs::write(&credentials_file, "").unwrap();
    std::fs::write(&config_file, "").unwrap();
    let _env = ScopedAwsEnv::set(&[
        ("AWS_ACCESS_KEY_ID", "AKIAFACTORYWIRING"),
        ("AWS_SECRET_ACCESS_KEY", "factory-wiring-secret"),
        (
            "AWS_SHARED_CREDENTIALS_FILE",
            credentials_file.to_str().unwrap(),
        ),
        ("AWS_CONFIG_FILE", config_file.to_str().unwrap()),
    ]);

    let mut body = Vec::new();
    for (event_type, payload) in [
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"text":"Hello!"},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        ("messageStop", r#"{"stopReason":"end_turn"}"#),
        (
            "metadata",
            r#"{"usage":{"inputTokens":3,"outputTokens":2}}"#,
        ),
    ] {
        body.extend_from_slice(&opi_ai::bedrock::event_stream::build_test_frame(
            event_type,
            "application/json",
            payload.as_bytes(),
        ));
    }
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(body, "application/vnd.amazon.eventstream"),
        )
        .mount(&server)
        .await;

    let mut config = OpiConfig::default();
    config.defaults.model = "bedrock:anthropic.claude-sonnet-4-20250514-v2:0".to_owned();
    config.providers.bedrock.base_url = Some(server.uri());
    let user_dir = tempfile::tempdir().unwrap();
    let bundle = opi_coding_agent::provider_factory::build_provider_bundle(
        &config,
        user_dir.path().to_path_buf(),
        Box::new(|| Box::new(opi_coding_agent::credential_store::FakeKeyringBackend::new())),
    )
    .await
    .expect("factory bedrock route builds lazily");

    let mut collection = opi_ai::Models::new();
    collection
        .register_route(
            bundle.provider,
            bundle.auth_resolver,
            opi_ai::AuthProvenanceSource::Static,
            opi_ai::CompatMetadata::default(),
        )
        .expect("register factory bedrock route");

    // The selected local source is observable through the management seam.
    let resolved = collection
        .get_auth("bedrock", Default::default())
        .await
        .expect("factory route resolves")
        .expect("environment pair selected");
    assert!(matches!(
        resolved.provenance.source,
        opi_ai::AuthProvenanceSource::AwsSigV4 {
            source: opi_ai::AwsCredentialSource::Environment
        }
    ));

    let mut request = opi_ai::provider::Request {
        model: "bedrock:anthropic.claude-sonnet-4-20250514-v2:0".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![opi_ai::message::Message::User(
                opi_ai::message::UserMessage {
                    content: vec![opi_ai::message::InputContent::Text {
                        text: "Hello".into(),
                    }],
                    timestamp_ms: 0,
                },
            )],
            tools: vec![],
        })
        .expect("valid context"),
        max_tokens: Some(64),
        temperature: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: tokio_util::sync::CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: opi_ai::provider::CacheRetention::None,
        session_id: None,
    };
    let _ = &mut request;
    let prepared = collection
        .prepare_call("bedrock:anthropic.claude-sonnet-4-20250514-v2:0", request)
        .await
        .expect("prepare factory bedrock call");
    let mut attempt = prepared.start_attempt().expect("start factory attempt");
    let mut saw_done = false;
    while let Some(item) = attempt.next().await {
        if matches!(item, Ok(opi_ai::stream::AssistantStreamEvent::Done { .. })) {
            saw_done = true;
        }
    }
    drop(attempt);
    assert!(saw_done, "factory route must stream the lifecycle");

    let received = server.received_requests().await.expect("recorded requests");
    let authorization = received[0]
        .headers
        .get("authorization")
        .expect("factory route signs the request")
        .to_str()
        .expect("authorization is ASCII");
    assert!(authorization.contains("AKIAFACTORYWIRING"));
}
