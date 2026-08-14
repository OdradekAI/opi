//! Behavioral tests for the top-level `opi doctor` command (Phase 7 task 7.4).
//!
//! Two layers:
//! - **Library API** tests exercise the pure `doctor` module (`DoctorScope`,
//!   `run_doctor`, `DoctorReport`, formatters) directly. These pin scope
//!   parsing, per-scope diagnostics, the exit-code policy, the NDJSON shape,
//!   and the credential-value non-leak guarantee without spawning anything.
//! - A single **binary parse smoke** proves unknown doctor scopes return before
//!   credential-aware orchestration. Behavioral output contracts use the
//!   injected production command core in the binary's unit tests.
//!
//! No test makes a network call or requires real credentials. Provider scope
//! checks credential *presence* only; the credential *value* is never emitted.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use opi_agent::{Diagnostic, Severity};
use opi_coding_agent::config::{ConfigError, OpiConfig};
use opi_coding_agent::diagnostic_bridge::{diagnostic_from_config, diagnostic_from_package};
use opi_coding_agent::doctor::{
    DoctorContext, DoctorEntry, DoctorReport, DoctorScope, format_json, format_text, run_doctor,
    run_doctor_command, run_doctor_with_store,
};
use opi_coding_agent::package_resolver::{
    InstalledPackageScope, PackageDiagnostic, PackageDiagnosticSeverity,
};

const ANTHROPIC_ENV: &str = "ANTHROPIC_API_KEY";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_config(model: &str) -> OpiConfig {
    let mut config = OpiConfig::default();
    config.defaults.model = model.to_string();
    config
}

#[allow(clippy::type_complexity)]
fn ctx<'a>(
    config: &'a OpiConfig,
    sessions_dir: &'a Path,
    env_var: &'a dyn Fn(&str) -> Option<String>,
) -> DoctorContext<'a> {
    DoctorContext {
        config,
        config_error: None,
        workspace_root: Path::new("."),
        user_config_dir: Path::new("."),
        sessions_dir,
        term: None,
        term_program: None,
        term_features: None,
        no_color: false,
        colorterm: None,
        env_var,
        store_probe: &EMPTY_STORE_PROBE,
    }
}

fn no_env(_: &str) -> Option<String> {
    None
}

/// Shared empty probe map for tests that do not exercise StoreCredential.
static EMPTY_STORE_PROBE: std::sync::LazyLock<
    std::collections::HashMap<String, opi_ai::CredentialSource>,
> = std::sync::LazyLock::new(std::collections::HashMap::new);

struct DoctorPresenceBackend {
    secret_get_calls: Arc<AtomicUsize>,
    presence_calls: Arc<AtomicUsize>,
}

struct DoctorOperationalProbeBackend;

impl opi_coding_agent::credential_store::KeyringBackend for DoctorOperationalProbeBackend {
    fn get(
        &self,
        service: &str,
        _provider_id: &str,
    ) -> Result<Option<String>, opi_coding_agent::credential_store::BackendError> {
        assert_eq!(service, "opi.presence");
        Err(opi_coding_agent::credential_store::BackendError::Other(
            "credential service access denied".to_owned(),
        ))
    }

    fn set(
        &self,
        _service: &str,
        _provider_id: &str,
        _value: &str,
    ) -> Result<(), opi_coding_agent::credential_store::BackendError> {
        Err(opi_coding_agent::credential_store::BackendError::Other(
            "unused set".to_owned(),
        ))
    }

    fn delete(
        &self,
        _service: &str,
        _provider_id: &str,
    ) -> Result<(), opi_coding_agent::credential_store::BackendError> {
        Err(opi_coding_agent::credential_store::BackendError::Other(
            "unused delete".to_owned(),
        ))
    }
}

impl opi_coding_agent::credential_store::KeyringBackend for DoctorPresenceBackend {
    fn get(
        &self,
        service: &str,
        provider_id: &str,
    ) -> Result<Option<String>, opi_coding_agent::credential_store::BackendError> {
        if service == "opi.presence" {
            self.presence_calls.fetch_add(1, Ordering::SeqCst);
            Ok((provider_id == "anthropic").then(|| "api_key".to_owned()))
        } else {
            self.secret_get_calls.fetch_add(1, Ordering::SeqCst);
            Err(opi_coding_agent::credential_store::BackendError::Other(
                "secret get forbidden".to_owned(),
            ))
        }
    }

    fn set(
        &self,
        _service: &str,
        _provider_id: &str,
        _value: &str,
    ) -> Result<(), opi_coding_agent::credential_store::BackendError> {
        Err(opi_coding_agent::credential_store::BackendError::Other(
            "unused set".to_owned(),
        ))
    }

    fn delete(
        &self,
        _service: &str,
        _provider_id: &str,
    ) -> Result<(), opi_coding_agent::credential_store::BackendError> {
        Err(opi_coding_agent::credential_store::BackendError::Other(
            "unused delete".to_owned(),
        ))
    }
}

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

/// Collect the distinct scope strings present in a report's NDJSON output.
fn scope_strings(report: &DoctorReport) -> Vec<String> {
    let mut scopes: Vec<String> = report
        .entries
        .iter()
        .map(|e| DoctorScope::as_str(&e.scope).to_string())
        .collect();
    scopes.sort();
    scopes.dedup();
    scopes
}

// ---------------------------------------------------------------------------
// Scope parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_list_empty_is_ok_empty() {
    // Empty/blank input means "all scopes" at the call site (caller treats
    // empty as ALL), so parsing itself succeeds with an empty selection.
    assert!(DoctorScope::parse_list("").unwrap().is_empty());
    assert!(DoctorScope::parse_list("   ").unwrap().is_empty());
}

#[test]
fn parse_list_subset() {
    let scopes = DoctorScope::parse_list("config,tui").unwrap();
    assert_eq!(scopes, vec![DoctorScope::Config, DoctorScope::Tui]);
}

#[test]
fn parse_list_trims_whitespace() {
    let scopes = DoctorScope::parse_list(" config , rpc ,tui").unwrap();
    assert_eq!(
        scopes,
        vec![DoctorScope::Config, DoctorScope::Rpc, DoctorScope::Tui]
    );
}

#[test]
fn parse_list_unknown_token_errors() {
    assert!(DoctorScope::parse_list("bogus").is_err());
    assert!(DoctorScope::parse_list("config,notascope").is_err());
}

#[test]
fn all_six_scopes_listed() {
    // The doctor surface must cover exactly the six design scopes.
    assert_eq!(
        DoctorScope::ALL.len(),
        6,
        "expected exactly six doctor scopes"
    );
}

// ---------------------------------------------------------------------------
// Config scope
// ---------------------------------------------------------------------------

#[test]
fn config_scope_reports_resolved_model() {
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let report = run_doctor(&[DoctorScope::Config], &ctx(&config, dir.path(), &no_env));
    assert!(
        !report.entries.is_empty(),
        "config scope must emit >=1 entry"
    );
    let has_model = format_text(&report).contains("claude-test-model");
    assert!(
        has_model,
        "config scope should mention the resolved model, got: {}",
        format_text(&report)
    );
    assert_eq!(report.entries[0].diagnostic.source, "config");
}

#[test]
fn config_scope_reports_proxy_for_custom_provider() {
    use opi_ai::AuthScheme;
    use opi_coding_agent::config::{CustomProviderConfig, ProviderProxyConfig};

    let dir = tempfile::tempdir().unwrap();
    let mut config = test_config("acme:foo");
    config.providers.custom.insert(
        "acme".into(),
        CustomProviderConfig {
            id: "acme".into(),
            name: "Acme".into(),
            base_url: Some("https://api.acme.example".into()),
            api_key_env: "ACME_API_KEY".into(),
            auth_scheme: AuthScheme::Bearer,
            proxy: Some(ProviderProxyConfig {
                url: "http://proxy.internal:3128".into(),
                no_proxy: None,
            }),
            headers: Vec::new(),
            models: Vec::new(),
        },
    );

    let report = run_doctor(&[DoctorScope::Config], &ctx(&config, dir.path(), &no_env));
    let proxy = report
        .entries
        .iter()
        .find(|entry| entry.diagnostic.code == "doctor_config_proxy")
        .unwrap_or_else(|| panic!("no doctor_config_proxy diagnostic: {:?}", report.entries));
    assert_eq!(proxy.diagnostic.severity, Severity::Info);
    assert!(
        format_text(&report).contains("proxy configured for selected provider \"acme\""),
        "expected custom-provider proxy message, got: {}",
        format_text(&report)
    );
    assert!(
        !format_text(&report)
            .contains("no explicit proxy configured for selected provider \"acme\""),
        "custom provider proxy should not be reported as absent, got: {}",
        format_text(&report)
    );
}

#[test]
fn config_scope_surfaces_config_error_as_error_severity() {
    // A config read failure must surface as an Error-severity shared diagnostic
    // (exit code 2), not an internal doctor failure (exit code 1).
    let config = test_config("anthropic:claude-test-model");
    let err = ConfigError::Read {
        path: std::path::PathBuf::from("/nonexistent/config.toml"),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "config file not found"),
    };
    let expected = diagnostic_from_config(&err);
    assert_eq!(expected.severity, Severity::Error);

    let dir = tempfile::tempdir().unwrap();
    let ctx = DoctorContext {
        config_error: Some(&err),
        ..ctx(&config, dir.path(), &no_env)
    };
    let report = run_doctor(&[DoctorScope::Config], &ctx);
    assert!(
        report.has_errors(),
        "config error should produce an error-severity diagnostic"
    );
    assert_eq!(report.exit_code(), 2);
    assert!(
        report
            .entries
            .iter()
            .any(|e| e.diagnostic.severity == Severity::Error)
    );
}

#[test]
fn doctor_json_redacts_absolute_path_in_config_details() {
    // A config error carries the config file path in `details`; the public
    // --json boundary must redact it (Phase 7 design: details are redacted
    // structured metadata, absolute paths are not emitted by default).
    let config = test_config("anthropic:claude-test-model");
    let leak_path = "/tmp/opi-secret-leak/config.toml";
    let err = ConfigError::Read {
        path: std::path::PathBuf::from(leak_path),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "config file not found"),
    };
    let dir = tempfile::tempdir().unwrap();
    let ctx = DoctorContext {
        config_error: Some(&err),
        ..ctx(&config, dir.path(), &no_env)
    };
    let report = run_doctor(&[DoctorScope::Config], &ctx);
    let json = format_json(&report);
    assert!(
        !json.contains(leak_path),
        "absolute config path leaked into doctor --json details: {json}",
    );
    assert!(
        json.contains("[REDACTED]"),
        "expected a redaction marker in details, got: {json}",
    );
}

#[test]
fn doctor_outputs_redact_diagnostic_message_and_action() {
    let secret = "sk-proj-1234567890abcdefghijklmnopqrstuv";
    let report = DoctorReport {
        entries: vec![DoctorEntry {
            scope: DoctorScope::Package,
            diagnostic: Diagnostic::new(
                Severity::Warning,
                "package_diagnostic",
                "package",
                format!("read C:\\Users\\alice\\.config\\opi\\packages\\p\\package.toml: {secret}"),
            )
            .action("open C:\\Users\\alice\\.config\\opi\\config.toml"),
        }],
    };

    let json = format_json(&report);
    let text = format_text(&report);

    for output in [&json, &text] {
        assert!(!output.contains("alice"), "OS username leaked: {output}");
        assert!(!output.contains(secret), "provider key leaked: {output}");
        assert!(
            output.contains("[REDACTED]"),
            "expected redaction marker in output: {output}"
        );
    }
}

// ---------------------------------------------------------------------------
// Provider scope (network-free; credential presence only)
// ---------------------------------------------------------------------------

#[test]
fn provider_scope_credential_present_is_info() {
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let map: HashMap<&str, String> = [(ANTHROPIC_ENV, "sk-present".into())].into_iter().collect();
    let env = |n: &str| map.get(n).cloned();
    let report = run_doctor(&[DoctorScope::Provider], &ctx(&config, dir.path(), &env));
    assert!(
        report
            .entries
            .iter()
            .any(|e| e.diagnostic.severity == Severity::Info),
        "present credentials should be Info, got: {:?}",
        report.entries
    );
    assert!(!report.has_errors());
}

#[test]
fn provider_scope_credential_absent_is_warning() {
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let report = run_doctor(&[DoctorScope::Provider], &ctx(&config, dir.path(), &no_env));
    assert!(
        report
            .entries
            .iter()
            .any(|e| e.diagnostic.severity == Severity::Warning),
        "absent credentials should be Warning, got: {:?}",
        report.entries
    );
    // Missing credentials is a warning, not an error -> still exit 0.
    assert!(!report.has_errors());
    assert_eq!(report.exit_code(), 0);
}

#[test]
fn provider_scope_uses_live_oauth_subscription_and_custom_availability() {
    use opi_ai::{AuthScheme, CredentialSource};
    use opi_coding_agent::config::CustomProviderConfig;

    let dir = tempfile::tempdir().unwrap();
    let mut custom_config = test_config("acme:model");
    custom_config.providers.custom.insert(
        "acme".into(),
        CustomProviderConfig {
            id: "acme".into(),
            name: "Acme".into(),
            base_url: Some("https://api.acme.example".into()),
            api_key_env: "ACME_API_KEY".into(),
            auth_scheme: AuthScheme::Bearer,
            proxy: None,
            headers: Vec::new(),
            models: Vec::new(),
        },
    );

    let cases = [
        (
            test_config("anthropic:claude-test-model"),
            HashMap::from([(
                "ANTHROPIC_OAUTH_TOKEN",
                "oauth-env-canary-DO-NOT-LEAK".to_owned(),
            )]),
            HashMap::new(),
            "env ANTHROPIC_OAUTH_TOKEN",
        ),
        (
            test_config("github-copilot:gpt-4.1"),
            HashMap::new(),
            HashMap::new(),
            "keychain opi:github-copilot",
        ),
        (
            test_config("openai-codex:gpt-5.6-sol"),
            HashMap::new(),
            HashMap::new(),
            "keychain opi:openai-codex",
        ),
        (
            custom_config.clone(),
            HashMap::from([("ACME_API_KEY", "custom-env-canary-DO-NOT-LEAK".to_owned())]),
            HashMap::new(),
            "env ACME_API_KEY",
        ),
        (
            custom_config,
            HashMap::new(),
            HashMap::from([(
                "acme".to_owned(),
                CredentialSource::Present {
                    label: "fake custom store".into(),
                },
            )]),
            "keychain opi:acme",
        ),
    ];

    for (config, env_values, store_probe, expected_probe) in cases {
        let env = |name: &str| env_values.get(name).cloned();
        let report = run_doctor(
            &[DoctorScope::Provider],
            &DoctorContext {
                store_probe: &store_probe,
                ..ctx(&config, dir.path(), &env)
            },
        );
        assert!(
            !report
                .entries
                .iter()
                .any(|entry| entry.diagnostic.code == "doctor_provider_unknown"),
            "{} was classified as unknown: {:?}",
            config.defaults.model,
            report.entries
        );
        let credential = report
            .entries
            .iter()
            .find(|entry| {
                matches!(
                    entry.diagnostic.code,
                    "doctor_provider_credentials" | "doctor_provider_credential_backend"
                )
            })
            .unwrap_or_else(|| panic!("{} has no credential diagnostic", config.defaults.model));
        assert_eq!(
            credential
                .diagnostic
                .details
                .as_ref()
                .and_then(|details| details["credential_probe"].as_str()),
            Some(expected_probe),
            "{}",
            config.defaults.model
        );
        let rendered = format!("{}{}", format_text(&report), format_json(&report));
        for secret in [
            "oauth-env-canary-DO-NOT-LEAK",
            "custom-env-canary-DO-NOT-LEAK",
        ] {
            assert!(!rendered.contains(secret), "secret leaked: {rendered}");
        }
    }
}

#[tokio::test]
async fn provider_scope_uses_stored_kind_for_precedence_and_wrong_kind_rejection() {
    use opi_ai::credential::{Credential, CredentialStore};
    use opi_ai::{AuthScheme, ModelCapabilities, ModelInfo, WireApi};
    use opi_coding_agent::config::CustomProviderConfig;
    use opi_coding_agent::credential_store::{FakeKeyringBackend, KeychainCredentialStore};
    use secrecy::SecretString;

    fn secret(value: &str) -> SecretString {
        SecretString::new(value.to_owned().into_boxed_str())
    }

    let mut custom_config = test_config("acme:model");
    custom_config.providers.custom.insert(
        "acme".into(),
        CustomProviderConfig {
            id: "acme".into(),
            name: "Acme".into(),
            base_url: Some("https://api.acme.example".into()),
            api_key_env: "ACME_API_KEY".into(),
            auth_scheme: AuthScheme::Bearer,
            proxy: None,
            headers: Vec::new(),
            models: vec![ModelInfo::new(
                "model",
                "Model",
                WireApi::OpenAiCompletions,
                ModelCapabilities::new(8_192, 1_024),
            )],
        },
    );

    let dir = tempfile::tempdir().unwrap();
    let store = KeychainCredentialStore::new(
        Box::new(FakeKeyringBackend::new()),
        dir.path().to_path_buf(),
    );
    store
        .write("anthropic", &Credential::ApiKey(secret("stored-api-key")))
        .await
        .unwrap();
    let anthropic_env = |name: &str| {
        (name == "ANTHROPIC_OAUTH_TOKEN").then(|| "oauth-env-canary-DO-NOT-LEAK".to_owned())
    };
    let anthropic_report = run_doctor_with_store(
        &[DoctorScope::Provider],
        &ctx(
            &test_config("anthropic:claude-test-model"),
            dir.path(),
            &anthropic_env,
        ),
        &store,
    )
    .await;
    let anthropic_details = anthropic_report
        .entries
        .iter()
        .find_map(|entry| entry.diagnostic.details.as_ref())
        .expect("Anthropic credential details");
    assert_eq!(
        anthropic_details["credential_probe"], "env ANTHROPIC_OAUTH_TOKEN",
        "stored API key must not mask the higher-precedence OAuth environment source"
    );

    store
        .write(
            "acme",
            &Credential::OAuthToken {
                access: secret("custom-oauth-access"),
                refresh: secret("custom-oauth-refresh"),
                expires_at: None,
                base_url: None,
                account_id: None,
            },
        )
        .await
        .unwrap();
    let custom_env =
        |name: &str| (name == "ACME_API_KEY").then(|| "custom-api-canary-DO-NOT-LEAK".to_owned());
    let custom_report = run_doctor_with_store(
        &[DoctorScope::Provider],
        &ctx(&custom_config, dir.path(), &custom_env),
        &store,
    )
    .await;
    let custom_details = custom_report
        .entries
        .iter()
        .find_map(|entry| entry.diagnostic.details.as_ref())
        .expect("custom credential details");
    assert_eq!(custom_details["credentials_present"], false);
    assert_eq!(
        custom_details["credential_probe"],
        "keychain opi:acme contains oauth_token; expected api_key"
    );

    let rendered = format!(
        "{}{}{}{}",
        format_text(&anthropic_report),
        format_json(&anthropic_report),
        format_text(&custom_report),
        format_json(&custom_report)
    );
    for canary in [
        "oauth-env-canary-DO-NOT-LEAK",
        "custom-api-canary-DO-NOT-LEAK",
        "custom-oauth-access",
        "custom-oauth-refresh",
    ] {
        assert!(!rendered.contains(canary), "secret leaked: {rendered}");
    }
}

#[test]
fn provider_scope_never_emits_credential_value() {
    // The credential *value* must never appear in any diagnostic field, even
    // though doctor inspects credential presence for the selected provider.
    let sentinel = "sk-test-DO-NOT-LEAK-1234567890";
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let map: HashMap<&str, String> = [(ANTHROPIC_ENV, sentinel.into())].into_iter().collect();
    let env = |n: &str| map.get(n).cloned();
    let report = run_doctor(&[DoctorScope::Provider], &ctx(&config, dir.path(), &env));
    let json = format_json(&report);
    let text = format_text(&report);
    assert!(
        !json.contains(sentinel),
        "credential value leaked into JSON output: {json}"
    );
    assert!(
        !text.contains(sentinel),
        "credential value leaked into text output: {text}"
    );
}

#[test]
fn provider_scope_bedrock_requires_access_key_and_secret() {
    let config = test_config("bedrock:anthropic.claude-test");
    let dir = tempfile::tempdir().unwrap();

    let only_access_key: HashMap<&str, String> =
        [("AWS_ACCESS_KEY_ID", "akid".into())].into_iter().collect();
    let only_access_key_env = |n: &str| only_access_key.get(n).cloned();
    let missing_secret = run_doctor(
        &[DoctorScope::Provider],
        &ctx(&config, dir.path(), &only_access_key_env),
    );
    assert!(
        missing_secret
            .entries
            .iter()
            .any(|e| e.diagnostic.severity == Severity::Warning
                && e.diagnostic
                    .details
                    .as_ref()
                    .is_some_and(|details| details["credentials_present"] == false)),
        "bedrock should warn when only AWS_ACCESS_KEY_ID is present: {:?}",
        missing_secret.entries
    );

    let complete: HashMap<&str, String> = [
        ("AWS_ACCESS_KEY_ID", "akid".into()),
        ("AWS_SECRET_ACCESS_KEY", "secret".into()),
    ]
    .into_iter()
    .collect();
    let complete_env = |n: &str| complete.get(n).cloned();
    let present = run_doctor(
        &[DoctorScope::Provider],
        &ctx(&config, dir.path(), &complete_env),
    );
    assert!(
        present
            .entries
            .iter()
            .any(|e| e.diagnostic.severity == Severity::Info
                && e.diagnostic
                    .details
                    .as_ref()
                    .is_some_and(|details| details["credentials_present"] == true)),
        "bedrock should report credentials present when access key and secret are present: {:?}",
        present.entries
    );
}

#[test]
fn provider_scope_bedrock_accepts_config_access_key_with_custom_secret_env() {
    let mut config = test_config("bedrock:anthropic.claude-test");
    config.providers.bedrock.access_key_id = Some("CONFIG_AKID".into());
    config.providers.bedrock.secret_access_key_env = Some("BEDROCK_SECRET".into());
    let dir = tempfile::tempdir().unwrap();
    let env_values: HashMap<&str, String> =
        [("BEDROCK_SECRET", "secret".into())].into_iter().collect();
    let env = |n: &str| env_values.get(n).cloned();

    let report = run_doctor(&[DoctorScope::Provider], &ctx(&config, dir.path(), &env));

    assert!(
        report.entries.iter().any(|e| {
            e.diagnostic.severity == Severity::Info
                && e.diagnostic.details.as_ref().is_some_and(|details| {
                    details["credentials_present"] == true
                        && details["credential_probe"]
                            == "config access_key_id + env BEDROCK_SECRET"
                })
        }),
        "bedrock should report config access_key_id plus custom secret env as present: {:?}",
        report.entries
    );
}

#[test]
fn provider_scope_bedrock_accepts_configured_profile() {
    let mut config = test_config("bedrock:anthropic.claude-test");
    config.providers.bedrock.profile = Some("dev".into());
    let dir = tempfile::tempdir().unwrap();

    let report = run_doctor(&[DoctorScope::Provider], &ctx(&config, dir.path(), &no_env));

    assert!(
        report.entries.iter().any(|e| {
            e.diagnostic.severity == Severity::Info
                && e.diagnostic.details.as_ref().is_some_and(|details| {
                    details["credentials_present"] == true
                        && details["credential_probe"] == "profile dev"
                })
        }),
        "bedrock should report configured AWS profile as present: {:?}",
        report.entries
    );
}

#[test]
fn provider_scope_bedrock_accepts_aws_profile_env() {
    let config = test_config("bedrock:anthropic.claude-test");
    let dir = tempfile::tempdir().unwrap();
    let env_values: HashMap<&str, String> = [("AWS_PROFILE", "dev".into())].into_iter().collect();
    let env = |n: &str| env_values.get(n).cloned();

    let report = run_doctor(&[DoctorScope::Provider], &ctx(&config, dir.path(), &env));

    assert!(
        report.entries.iter().any(|e| {
            e.diagnostic.severity == Severity::Info
                && e.diagnostic.details.as_ref().is_some_and(|details| {
                    details["credentials_present"] == true
                        && details["credential_probe"] == "env AWS_PROFILE dev"
                })
        }),
        "bedrock should report AWS_PROFILE as present: {:?}",
        report.entries
    );
}

#[test]
fn provider_scope_bedrock_custom_secret_env_does_not_replace_default_env_pair() {
    let mut config = test_config("bedrock:anthropic.claude-test");
    config.providers.bedrock.secret_access_key_env = Some("BEDROCK_SECRET".into());
    let dir = tempfile::tempdir().unwrap();
    let env_values: HashMap<&str, String> = [
        ("AWS_ACCESS_KEY_ID", "akid".into()),
        ("AWS_SECRET_ACCESS_KEY", "secret".into()),
    ]
    .into_iter()
    .collect();
    let env = |n: &str| env_values.get(n).cloned();

    let report = run_doctor(&[DoctorScope::Provider], &ctx(&config, dir.path(), &env));

    assert!(
        report.entries.iter().any(|e| {
            e.diagnostic.severity == Severity::Info
                && e.diagnostic.details.as_ref().is_some_and(|details| {
                    details["credentials_present"] == true
                        && details["credential_probe"]
                            == "env AWS_ACCESS_KEY_ID + env AWS_SECRET_ACCESS_KEY"
                })
        }),
        "bedrock should preserve the fixed default env pair when a custom secret env is configured: {:?}",
        report.entries
    );
}

#[test]
fn provider_scope_bedrock_config_access_requires_configured_secret_env() {
    let mut config = test_config("bedrock:anthropic.claude-test");
    config.providers.bedrock.access_key_id = Some("CONFIG_AKID".into());
    let dir = tempfile::tempdir().unwrap();
    let env_values: HashMap<&str, String> = [("AWS_SECRET_ACCESS_KEY", "secret".into())]
        .into_iter()
        .collect();
    let env = |n: &str| env_values.get(n).cloned();

    let report = run_doctor(&[DoctorScope::Provider], &ctx(&config, dir.path(), &env));

    assert!(
        report.entries.iter().any(|e| {
            e.diagnostic.severity == Severity::Warning
                && e.diagnostic.details.as_ref().is_some_and(|details| {
                    details["credentials_present"] == false
                        && details["credential_probe"]
                            == "config access_key_id + configured secret env"
                })
        }),
        "bedrock config access_key_id must not pair with the fixed default secret env unless it is configured: {:?}",
        report.entries
    );
}

// ---------------------------------------------------------------------------
// Session scope
// ---------------------------------------------------------------------------

#[test]
fn session_scope_reports_session_count() {
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    // Drop two fake session JSONL files into the sessions dir.
    std::fs::write(dir.path().join("aaa.jsonl"), "{}\n").unwrap();
    std::fs::write(dir.path().join("bbb.jsonl"), "{}\n").unwrap();
    let report = run_doctor(&[DoctorScope::Session], &ctx(&config, dir.path(), &no_env));
    let text = format_text(&report);
    assert!(
        text.contains('2') || text.contains("two"),
        "session scope should report the session count, got: {text}"
    );
    assert_eq!(report.entries[0].diagnostic.source, "session");
    assert!(!report.has_errors());
}

#[test]
fn session_scope_missing_createable_dir_is_info() {
    // A not-yet-created sessions dir under an existing parent is normal on a
    // fresh install and must not be an error.
    let config = test_config("anthropic:claude-test-model");
    let parent = tempfile::tempdir().unwrap();
    let missing = parent.path().join("sessions");
    let report = run_doctor(&[DoctorScope::Session], &ctx(&config, &missing, &no_env));
    assert!(
        !report.has_errors(),
        "missing-but-createable sessions dir should not error, got: {:?}",
        report.entries
    );
}

// ---------------------------------------------------------------------------
// TUI scope
// ---------------------------------------------------------------------------

#[test]
fn tui_scope_detects_iterm_protocol_and_no_color() {
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let ctx = DoctorContext {
        term: Some("xterm-256color"),
        term_program: Some("iTerm.app"),
        term_features: None,
        no_color: true,
        colorterm: None,
        ..ctx(&config, dir.path(), &no_env)
    };
    let report = run_doctor(&[DoctorScope::Tui], &ctx);
    let text = format_text(&report).to_lowercase();
    assert!(
        text.contains("iterm"),
        "tui scope should report the iTerm2 protocol, got: {text}"
    );
    assert!(
        text.contains("no color") || text.contains("no_color"),
        "tui scope should report no-color state, got: {text}"
    );
    assert_eq!(report.entries[0].diagnostic.source, "tui");
}

#[test]
fn tui_scope_fallback_when_no_graphics_protocol() {
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let report = run_doctor(&[DoctorScope::Tui], &ctx(&config, dir.path(), &no_env));
    assert!(!report.has_errors());
    assert_eq!(report.entries[0].diagnostic.source, "tui");
}

// ---------------------------------------------------------------------------
// RPC scope
// ---------------------------------------------------------------------------

#[test]
fn rpc_scope_reports_schema_version() {
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let report = run_doctor(&[DoctorScope::Rpc], &ctx(&config, dir.path(), &no_env));
    let text = format_text(&report);
    let version = opi_coding_agent::rpc::RPC_SCHEMA_VERSION;
    assert!(
        text.contains(&version.to_string()),
        "rpc scope should report the schema version {version}, got: {text}"
    );
    assert_eq!(report.entries[0].diagnostic.source, "rpc");
    assert!(!report.has_errors());
}

// ---------------------------------------------------------------------------
// Package scope (delegates to the installed-package resolver)
// ---------------------------------------------------------------------------

#[test]
fn package_scope_empty_workspace_is_info_no_errors() {
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let report = run_doctor(&[DoctorScope::Package], &ctx(&config, dir.path(), &no_env));
    assert!(
        !report.entries.is_empty(),
        "package scope must emit >=1 entry even with no packages"
    );
    assert!(
        !report.has_errors(),
        "empty workspace should not produce package errors, got: {:?}",
        report.entries
    );
    assert!(
        report
            .entries
            .iter()
            .all(|e| e.diagnostic.source == "package")
    );
}

// ---------------------------------------------------------------------------
// Whole-report behavior
// ---------------------------------------------------------------------------

#[test]
fn run_doctor_all_scopes_covers_every_scope() {
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let report = run_doctor(DoctorScope::ALL, &ctx(&config, dir.path(), &no_env));
    let scopes = scope_strings(&report);
    assert_eq!(
        scopes.len(),
        6,
        "default doctor run must cover all six scopes, got: {scopes:?}"
    );
}

#[test]
fn exit_code_no_errors_is_zero() {
    let report = DoctorReport::default();
    assert_eq!(report.exit_code(), 0);
}

#[test]
fn exit_code_with_error_is_two() {
    let config = test_config("anthropic:claude-test-model");
    let err = ConfigError::Read {
        path: std::path::PathBuf::from("/nonexistent/config.toml"),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
    };
    let dir = tempfile::tempdir().unwrap();
    let ctx = DoctorContext {
        config_error: Some(&err),
        ..ctx(&config, dir.path(), &no_env)
    };
    let report = run_doctor(DoctorScope::ALL, &ctx);
    assert_eq!(report.exit_code(), 2);
}

#[test]
fn format_json_is_ndjson_with_required_fields() {
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let report = run_doctor(DoctorScope::ALL, &ctx(&config, dir.path(), &no_env));
    let json = format_json(&report);
    assert!(!json.trim().is_empty(), "json output must not be empty");
    for line in json.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("line is not valid JSON: {line}\nerror: {e}"));
        assert!(value.get("scope").is_some(), "missing scope: {line}");
        assert!(value.get("severity").is_some(), "missing severity: {line}");
        assert!(value.get("code").is_some(), "missing code: {line}");
        assert!(value.get("source").is_some(), "missing source: {line}");
        assert!(value.get("message").is_some(), "missing message: {line}");
    }
}

// ===========================================================================
// Phase 7 task 7.6 — redaction + shared-diagnostic-shape guards
//
// `phase7_doctor_redacts_sensitive_values` closes the credentialed-URL leak
// path the 7.4 evaluator deferred (a package source URL carrying a GitHub PAT
// or user:password userinfo must not survive the `--json` boundary). It drives
// the real production bridge (`diagnostic_from_package`) so the assertion
// covers the path that produces `details.package_source`, not a synthetic
// shortcut. `phase7_shared_diagnostics_used_by_doctor` proves every doctor
// entry is the shared `opi_agent::Diagnostic` at the public boundary (SC 1).
// ===========================================================================

#[test]
fn phase7_doctor_redacts_sensitive_values() {
    let credentialed =
        "https://ghp_01234567890123456789012345678901234567@github.com/owner/repo.git";
    let pd = PackageDiagnostic {
        scope: InstalledPackageScope::Project,
        source: credentialed.to_string(),
        severity: PackageDiagnosticSeverity::Warning,
        code: "source_unavailable".to_string(),
        message: "package source unreachable".to_string(),
    };
    let diagnostic = diagnostic_from_package(&pd);
    // The shared bridge places the raw source into details.package_source.
    assert_eq!(
        diagnostic.details.as_ref().unwrap()["package_source"],
        credentialed,
        "precondition: the raw credentialed URL must reach details before redaction"
    );

    let report = DoctorReport {
        entries: vec![DoctorEntry {
            scope: DoctorScope::Package,
            diagnostic,
        }],
    };

    let json = format_json(&report);
    assert!(
        !json.contains("ghp_01234567890123456789012345678901234567"),
        "GitHub PAT leaked through doctor --json details.package_source: {json}",
    );
    assert!(
        !json.contains(":s3cr3t@") && !json.contains("ghp_"),
        "credentialed-URL credential leaked through doctor --json: {json}",
    );
    assert!(
        json.contains("[REDACTED]"),
        "expected a redaction marker in doctor --json, got: {json}",
    );

    // A user:password userinfo URL must also be scrubbed at the boundary.
    let pd2 = PackageDiagnostic {
        scope: InstalledPackageScope::Project,
        source: "https://alice:s3cr3t@gitlab.example.com/o/r.git".to_string(),
        ..pd
    };
    let report2 = DoctorReport {
        entries: vec![DoctorEntry {
            scope: DoctorScope::Package,
            diagnostic: diagnostic_from_package(&pd2),
        }],
    };
    let json2 = format_json(&report2);
    assert!(
        !json2.contains("s3cr3t") && !json2.contains("alice:s3cr3t@"),
        "userinfo credentials leaked through doctor --json: {json2}",
    );
    assert!(
        json2.contains("[REDACTED]"),
        "expected redaction, got: {json2}"
    );
}

#[test]
fn phase7_shared_diagnostics_used_by_doctor() {
    // SC 1: doctor emits the shared `opi_agent::Diagnostic` shape (stable
    // severity/code/source/message) at its public boundary, not ad-hoc strings.
    let config = test_config("anthropic:claude-test-model");
    let dir = tempfile::tempdir().unwrap();
    let report = run_doctor(DoctorScope::ALL, &ctx(&config, dir.path(), &no_env));

    // Every entry's diagnostic IS the shared Diagnostic (source is a stable
    // shared SOURCE_* vocabulary token; code is stable snake_case).
    let valid_sources = ["config", "provider", "package", "session", "tui", "rpc"];
    assert!(!report.entries.is_empty());
    for entry in &report.entries {
        assert!(
            valid_sources.contains(&entry.diagnostic.source),
            "doctor diagnostic source {:?} is not in the shared SOURCE_* vocabulary",
            entry.diagnostic.source,
        );
        assert!(
            !entry.diagnostic.code.is_empty()
                && entry
                    .diagnostic
                    .code
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
            "doctor diagnostic code {:?} is not stable snake_case",
            entry.diagnostic.code,
        );
    }

    // The NDJSON output flattens the shared Diagnostic fields (not a custom
    // shape): every line carries severity/code/source/message.
    let json = format_json(&report);
    for line in json.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("doctor --json line is not valid JSON: {line}\nerror: {e}"));
        for field in ["severity", "code", "source", "message"] {
            assert!(
                value.get(field).is_some(),
                "doctor --json line does not carry shared Diagnostic field {field}: {line}"
            );
        }
    }

    // A package diagnostic built through the shared bridge carries the stable
    // shared code `package_diagnostic` (CODE_PACKAGE_DIAGNOSTIC) and source
    // `package`, proving the shared shape crosses the doctor boundary.
    let pd = PackageDiagnostic {
        scope: InstalledPackageScope::Project,
        source: "https://example.com/o/r.git".to_string(),
        severity: PackageDiagnosticSeverity::Warning,
        code: "source_unavailable".to_string(),
        message: "unreachable".to_string(),
    };
    let bridged = diagnostic_from_package(&pd);
    assert_eq!(bridged.code, "package_diagnostic");
    assert_eq!(bridged.source, "package");
}

// ===========================================================================
// Phase 8 task 8.6 — public diagnostic message/action redaction contract.
//
// Pins that the public doctor formatters scrub real-format Anthropic keys,
// credentialed-URL userinfo, and host absolute paths from BOTH `message` and
// `action`, regardless of the JSON or text rendering. Both fields are routed
// through the shared `redacted_payload(Summary)` boundary, so the same
// redaction contract must hold for either formatter.
// ===========================================================================

#[test]
fn phase8_public_diagnostic_message_redaction() {
    let key = "sk-ant-api03-1234567890abcdefghijklmnopqrstuv";
    let credentialed = "https://alice:s3cr3t@host/repo.git";
    let posix_path = "/Users/alice/.config/opi/config.toml";
    let win_path = "C:\\Users\\alice\\.config\\opi\\packages\\p\\package.toml";
    let report = DoctorReport {
        entries: vec![DoctorEntry {
            scope: DoctorScope::Package,
            diagnostic: Diagnostic::new(
                Severity::Warning,
                "package_diagnostic",
                "package",
                format!("read {posix_path} then {win_path}: key {key} src {credentialed}"),
            )
            .action(format!("open {win_path} and check {key} at {credentialed}")),
        }],
    };

    let json = format_json(&report);
    let text = format_text(&report);

    for output in [&json, &text] {
        assert!(!output.contains(key), "Anthropic key leaked: {output}");
        assert!(
            !output.contains("s3cr3t"),
            "userinfo password leaked: {output}"
        );
        assert!(!output.contains("alice"), "OS username leaked: {output}");
        assert!(
            output.contains("[REDACTED]"),
            "expected a redaction marker in output: {output}"
        );
    }
}

// ===========================================================================
// Binary (integration) tests — spawn the real `opi` binary.
// ===========================================================================

fn opi_bin() -> String {
    env!("CARGO_BIN_EXE_opi").to_owned()
}

fn run_unknown_scope_smoke() -> std::process::Output {
    let bin = opi_bin();
    let tmp = tempfile::tempdir().unwrap();
    std::process::Command::new(&bin)
        .args(["doctor", "--scope", "bogus"])
        .current_dir(tmp.path())
        .env_clear()
        .output()
        .unwrap_or_else(|e| panic!("failed to run {bin}: {e}"))
}

#[test]
fn doctor_unknown_scope_exits_one() {
    // An unknown scope token is an internal doctor command failure -> exit 1.
    let output = run_unknown_scope_smoke();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 for unknown scope\nstderr: {stderr}",
    );
}

#[test]
fn package_doctor_remains_a_distinct_parseable_subcommand() {
    use clap::Parser;
    use opi_coding_agent::cli::{Cli, Command, PackageCommand};

    let cli = Cli::try_parse_from(["opi", "package", "doctor"])
        .expect("package doctor remains parseable");
    assert!(matches!(
        cli.command,
        Some(Command::Package {
            command: PackageCommand::Doctor { json: false }
        })
    ));
}

// ---------------------------------------------------------------------------
// Phase 14.1: stored-credential probe surfaces (acceptance scenario)
// ---------------------------------------------------------------------------

/// Acceptance scenario `phase14-store-probe-surfaces` (doctor half): with a
/// keychain-backend config, doctor consults the injected redacted probe and
/// distinguishes Present / Absent / BackendUnavailable with distinct
/// severities+messages, and never emits the credential value. Exercises the
/// production `run_doctor` path with a hand-built store_probe map (the async
/// outer orchestration in `run_doctor_cli` produces exactly this map).
/// Acceptance scenario `phase14-store-probe-surfaces` (doctor half): with a
/// keychain-backend config, a REAL secret-bearing credential is seeded in the
/// store; doctor probes it (Present, label-only — never reading the secret) and
/// emits only the redacted label. The seeded access/refresh secrets must NOT
/// appear in either output mode. This is non-vacuous: the secrets ARE in the
/// store, so a regression that read+emitted the secret would fail this test.
/// (The Absent-vs-BackendUnavailable distinction is covered by
/// `stored_credential_backend_unavailable_is_distinct_from_absent`.)
#[tokio::test]
async fn stored_credential_probe_is_redacted() {
    use opi_ai::credential::CredentialStore;
    use opi_ai::{AuthDescriptor, Credential, CredentialSource};
    use opi_coding_agent::config::CredentialBackendSource;
    use opi_coding_agent::credential_store::{FakeKeyringBackend, KeychainCredentialStore};
    use secrecy::SecretString;

    let mut config = OpiConfig::default();
    config.defaults.model = "anthropic:claude-store-probe".into();
    config.defaults.credential_backend = Some(CredentialBackendSource::Keychain);

    // Keychain backend -> StoreCredential descriptor for the API-key provider.
    let descriptor =
        opi_coding_agent::provider_factory::auth_descriptor_for(&config, "anthropic").unwrap();
    assert!(
        matches!(descriptor, AuthDescriptor::StoreCredential { .. }),
        "expected StoreCredential descriptor, got {descriptor:?}"
    );

    let secret_access = "atk-doctor-probe-DO-NOT-LEAK";
    let secret_refresh = "rtk-doctor-probe-DO-NOT-LEAK";

    let dir = tempfile::tempdir().unwrap();
    let store = KeychainCredentialStore::new(
        Box::new(FakeKeyringBackend::new()),
        dir.path().to_path_buf(),
    );
    // Seed a real OAuth credential carrying access + refresh secrets.
    store
        .write(
            "anthropic",
            &Credential::OAuthToken {
                access: SecretString::new(secret_access.to_owned().into_boxed_str()),
                refresh: SecretString::new(secret_refresh.to_owned().into_boxed_str()),
                expires_at: None,
                base_url: Some("https://copilot.example/api".to_owned()),
                account_id: None,
            },
        )
        .await
        .unwrap();

    // Probe -> Present{label}, carrying NO secret. The async outer command path
    // (run_doctor_cli) builds exactly this map from store.probe.
    let probed = store.probe("anthropic").await;
    assert!(
        matches!(probed, CredentialSource::Present { .. }),
        "expected Present probe, got {probed:?}"
    );
    let mut store_probe = HashMap::new();
    store_probe.insert("anthropic".to_string(), probed);

    let env_probe = |_: &str| None;
    let report = run_doctor(
        &[DoctorScope::Provider],
        &DoctorContext {
            config: &config,
            config_error: None,
            workspace_root: Path::new("."),
            user_config_dir: dir.path(),
            sessions_dir: dir.path(),
            term: None,
            term_program: None,
            term_features: None,
            no_color: false,
            colorterm: None,
            env_var: &env_probe,
            store_probe: &store_probe,
        },
    );

    let entry = report
        .entries
        .iter()
        .find(|e| e.diagnostic.source == opi_agent::diagnostic::SOURCE_PROVIDER)
        .expect("provider diagnostic present");
    assert_eq!(entry.diagnostic.severity, Severity::Info);

    // NON-VACUOUS redaction: access + refresh ARE in the store; doctor only
    // probed, so neither may appear in text or JSON output.
    let text = format_text(&report);
    let json = format_json(&report);
    assert!(!text.contains(secret_access), "text leaked access: {text}");
    assert!(
        !text.contains(secret_refresh),
        "text leaked refresh: {text}"
    );
    assert!(!json.contains(secret_access), "json leaked access: {json}");
    assert!(
        !json.contains(secret_refresh),
        "json leaked refresh: {json}"
    );
}

#[tokio::test]
async fn doctor_store_probe_uses_async_command_orchestration() {
    use opi_ai::credential::{Credential, CredentialStore};
    use opi_coding_agent::config::CredentialBackendSource;
    use opi_coding_agent::credential_store::{FakeKeyringBackend, KeychainCredentialStore};
    use secrecy::SecretString;

    let canary = "sk-doctor-orchestration-DO-NOT-LEAK";
    let env_probe = |_: &str| None;

    for (model, backend, credential) in [
        (
            "anthropic:present",
            FakeKeyringBackend::new(),
            Some(Credential::ApiKey(SecretString::new(
                canary.to_owned().into_boxed_str(),
            ))),
        ),
        ("openai:absent", FakeKeyringBackend::new(), None),
        (
            "gemini:unavailable",
            FakeKeyringBackend::new().with_unavailable(),
            None,
        ),
    ] {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = KeychainCredentialStore::new(Box::new(backend), dir.path().to_path_buf());
        if let Some(credential) = credential {
            let provider = model.split_once(':').expect("model spec").0;
            store
                .write(provider, &credential)
                .await
                .expect("seed fake store");
        }
        let mut config = OpiConfig::default();
        config.defaults.model = model.into();
        config.defaults.credential_backend = Some(CredentialBackendSource::Keychain);
        let report = run_doctor_with_store(
            &[DoctorScope::Provider],
            &ctx(&config, dir.path(), &env_probe),
            &store,
        )
        .await;
        let rendered = format!("{}{}", format_text(&report), format_json(&report));
        assert!(
            !rendered.contains(canary),
            "doctor leaked secret: {rendered}"
        );
        if model.contains("present") {
            assert!(rendered.contains("credentials present"), "{rendered}");
        } else if model.contains("absent") {
            assert!(rendered.contains("credentials not set"), "{rendered}");
        } else {
            assert!(rendered.contains("backend unavailable"), "{rendered}");
        }
    }
}

#[tokio::test]
async fn native_keyring_precedes_doctor_orchestration() {
    use opi_coding_agent::config::CredentialBackendSource;
    use opi_coding_agent::credential_store::{FakeKeyringBackend, KeyringBackendFactory};

    let events = Arc::new(Mutex::new(Vec::new()));
    let factory_events = Arc::clone(&events);
    let backend_factory: KeyringBackendFactory = Box::new(move || {
        let backend = FakeKeyringBackend::new();
        factory_events
            .lock()
            .expect("ordering events")
            .push("native_install");
        Box::new(OrderingKeyringBackend {
            inner: backend,
            events: Arc::clone(&factory_events),
        })
    });
    let dir = tempfile::tempdir().expect("temp dir");
    let mut config = OpiConfig::default();
    config.defaults.model = "anthropic:doctor-ordering".into();
    config.defaults.credential_backend = Some(CredentialBackendSource::Keychain);
    assert!(
        events.lock().expect("ordering events").is_empty(),
        "backend construction must remain lazy until the command core"
    );
    let report = run_doctor_command(
        &[DoctorScope::Provider],
        &ctx(&config, dir.path(), &no_env),
        dir.path().to_path_buf(),
        backend_factory,
    )
    .await;
    assert!(
        format_text(&report).contains("credentials not set"),
        "mock entry should be created only after native installation"
    );
    assert_native_entry_drop_order(&events);
}

#[tokio::test]
async fn doctor_presence_probe_never_reads_secret() {
    use opi_coding_agent::config::CredentialBackendSource;
    use opi_coding_agent::credential_store::KeychainCredentialStore;

    let secret_get_calls = Arc::new(AtomicUsize::new(0));
    let presence_calls = Arc::new(AtomicUsize::new(0));
    let dir = tempfile::tempdir().expect("temp dir");
    let store = KeychainCredentialStore::new(
        Box::new(DoctorPresenceBackend {
            secret_get_calls: Arc::clone(&secret_get_calls),
            presence_calls: Arc::clone(&presence_calls),
        }),
        dir.path().to_path_buf(),
    );
    let mut config = OpiConfig::default();
    config.defaults.model = "anthropic:doctor-presence".into();
    config.defaults.credential_backend = Some(CredentialBackendSource::Keychain);

    let report = run_doctor_with_store(
        &[DoctorScope::Provider],
        &ctx(&config, dir.path(), &no_env),
        &store,
    )
    .await;
    assert!(format_text(&report).contains("credentials present"));
    assert_eq!(secret_get_calls.load(Ordering::SeqCst), 0);
    assert!(presence_calls.load(Ordering::SeqCst) > 0);
}

#[tokio::test]
async fn doctor_fails_closed_on_operational_and_corrupt_store_probes_with_env_present() {
    use opi_coding_agent::credential_store::{
        FakeKeyringBackend, KEYCHAIN_PRESENCE_SERVICE, KeychainCredentialStore, KeyringBackend,
    };

    let env_canary = "doctor-fail-closed-env-canary-DO-NOT-LEAK";
    let env_probe = |name: &str| (name == ANTHROPIC_ENV).then(|| env_canary.to_owned());
    let config = test_config("anthropic:claude-test-model");

    let corrupt = FakeKeyringBackend::new();
    corrupt.seed_raw(KEYCHAIN_PRESENCE_SERVICE, "anthropic", "corrupt-marker");
    let cases: [(&str, Box<dyn KeyringBackend>); 2] = [
        ("operational", Box::new(DoctorOperationalProbeBackend)),
        ("corrupt", Box::new(corrupt)),
    ];

    for (name, backend) in cases {
        let dir = tempfile::tempdir().unwrap();
        let store = KeychainCredentialStore::new(backend, dir.path().to_path_buf());
        let report = run_doctor_with_store(
            &[DoctorScope::Provider],
            &ctx(&config, dir.path(), &env_probe),
            &store,
        )
        .await;
        let credential = report
            .entries
            .iter()
            .find(|entry| {
                matches!(
                    entry.diagnostic.code,
                    "doctor_provider_credentials" | "doctor_provider_credential_backend"
                )
            })
            .unwrap_or_else(|| panic!("{name}: missing credential diagnostic"));
        assert_eq!(
            credential
                .diagnostic
                .details
                .as_ref()
                .expect("credential details")["credentials_present"],
            false,
            "{name}: fail-closed store probe must not use API-key env fallback"
        );
        let rendered = format!("{}{}", format_text(&report), format_json(&report));
        assert!(!rendered.contains(env_canary), "{name}: {rendered}");
    }
}

/// BackendUnavailable must be a *distinct* diagnostic from Absent (spec SC1:
/// doctor distinguishes "missing entry" from "no keychain daemon").
#[test]
fn stored_credential_backend_unavailable_is_distinct_from_absent() {
    use opi_ai::CredentialSource;
    use opi_coding_agent::config::CredentialBackendSource;

    let mut config = OpiConfig::default();
    config.defaults.model = "anthropic:claude-distinct".into();
    config.defaults.credential_backend = Some(CredentialBackendSource::Keychain);

    let dir = tempfile::tempdir().unwrap();
    let env_probe = |_: &str| None;

    let mut absent = HashMap::new();
    absent.insert("anthropic".to_string(), CredentialSource::Absent);
    let absent_report = run_doctor(
        &[DoctorScope::Provider],
        &DoctorContext {
            config: &config,
            config_error: None,
            workspace_root: Path::new("."),
            user_config_dir: dir.path(),
            sessions_dir: dir.path(),
            term: None,
            term_program: None,
            term_features: None,
            no_color: false,
            colorterm: None,
            env_var: &env_probe,
            store_probe: &absent,
        },
    );

    let mut unavail = HashMap::new();
    unavail.insert(
        "anthropic".to_string(),
        CredentialSource::BackendUnavailable {
            reason: "no keychain daemon".to_owned(),
        },
    );
    let unavail_report = run_doctor(
        &[DoctorScope::Provider],
        &DoctorContext {
            config: &config,
            config_error: None,
            workspace_root: Path::new("."),
            user_config_dir: dir.path(),
            sessions_dir: dir.path(),
            term: None,
            term_program: None,
            term_features: None,
            no_color: false,
            colorterm: None,
            env_var: &env_probe,
            store_probe: &unavail,
        },
    );

    // Different diagnostic codes: Absent -> doctor_provider_credentials,
    // BackendUnavailable -> doctor_provider_credential_backend.
    let absent_code = absent_report
        .entries
        .iter()
        .find(|e| e.diagnostic.source == opi_agent::diagnostic::SOURCE_PROVIDER)
        .map(|e| e.diagnostic.code)
        .expect("absent provider diagnostic");
    let unavail_code = unavail_report
        .entries
        .iter()
        .find(|e| e.diagnostic.source == opi_agent::diagnostic::SOURCE_PROVIDER)
        .map(|e| e.diagnostic.code)
        .expect("unavailable provider diagnostic");
    assert_ne!(absent_code, unavail_code);
    assert_eq!(unavail_code, "doctor_provider_credential_backend");
}

// ---------------------------------------------------------------------------
// Package scope — Phase 16.5 execution-package lifecycle + drift
// ---------------------------------------------------------------------------

#[test]
fn package_scope_reports_execution_lifecycle_and_drift_at_top_level() {
    // SC16-03: the TOP-LEVEL `opi doctor` (not just `opi package doctor`) must
    // report execution-package trusted/enabled state and lock/executable-hash
    // drift without starting package code.
    use opi_coding_agent::cli::PackageCommand;
    use opi_coding_agent::package_activation;
    use opi_coding_agent::package_cli;
    use sha2::{Digest, Sha256};

    let user = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();

    // Execution-package fixture targeting the running host.
    let pkg = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(pkg.path().join("bin")).unwrap();
    let exe_content: &[u8] = b"#!/bin/sh\necho hi\n";
    let exe = pkg.path().join("bin").join("opi-sandbox");
    std::fs::write(&exe, exe_content).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let sha = format!("{:x}", Sha256::digest(exe_content));
    let target = package_activation::host_target_triple();
    let opi_version = package_activation::host_opi_version();
    let toml = format!(
        "version = \"0.8.0\"\n\
         opi_version = \"={opi_version}\"\n\
         name = \"opi-sandbox\"\n\
         description = \"doctor fixture\"\n\
         \n\
         [[contributions.adapters]]\n\
         capability = \"command.execute\"\n\
         id = \"opi-sandbox\"\n\
         transport = \"process-jsonl\"\n\
         command = \"bin/opi-sandbox\"\n\
         args = [\"backend\", \"--stdio\"]\n\
         protocol = \"command-execution-jsonl-v1\"\n\
         target = \"{target}\"\n\
         sha256 = \"{sha}\"\n\
         handshake_timeout_ms = 5000\n\
         adapter_config = {{}}\n"
    );
    std::fs::write(pkg.path().join("package.toml"), toml).unwrap();

    // Install globally: writes lock.contributions + untrusted/disabled record.
    let exit = package_cli::handle_package_command(
        &PackageCommand::Add {
            source: pkg.path().to_str().unwrap().into(),
            local: false,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(exit, 0);

    let config = test_config("anthropic:claude-test-model");
    let context = DoctorContext {
        config: &config,
        config_error: None,
        workspace_root: workspace.path(),
        user_config_dir: user.path(),
        sessions_dir: sessions.path(),
        term: None,
        term_program: None,
        term_features: None,
        no_color: false,
        colorterm: None,
        env_var: &no_env,
        store_probe: &EMPTY_STORE_PROBE,
    };

    // A freshly installed execution package is actionable until Package Trust
    // is confirmed, and uses the same stable code as runtime activation.
    let report = run_doctor(&[DoctorScope::Package], &context);
    let text = format_text(&report);
    assert!(
        text.contains("execution package"),
        "lifecycle reported: {text}"
    );
    assert!(
        text.contains("trusted=false"),
        "untrusted state reported: {text}"
    );
    assert!(
        report
            .entries
            .iter()
            .any(|e| e.diagnostic.source == "package"),
        "package-scope entries present"
    );
    let untrusted = report
        .entries
        .iter()
        .find(|e| e.diagnostic.code == "package_untrusted")
        .expect("stable runtime code on top-level doctor");
    assert!(untrusted.diagnostic.action.is_some());
    assert_eq!(report.exit_code(), 2);

    // Tamper with the executable: drift must surface as an error at the
    // top-level doctor, and no adapter process is started.
    std::fs::write(&exe, b"#!/bin/sh\necho pwned\n").unwrap();
    let report2 = run_doctor(&[DoctorScope::Package], &context);
    let text2 = format_text(&report2);
    assert!(
        text2.contains("drift"),
        "drift reported at top level: {text2}"
    );
    assert_eq!(
        report2.exit_code(),
        2,
        "drifted execution package is an error"
    );
}

/// SC16-14 doctor surfaces: the top-level `opi doctor` package scope uses the
/// stable execution-failure codes for actionable lifecycle failures and keeps
/// the doctor-local lifecycle code for informational state. Redaction wiring is proven by
/// seeding abs-path + secret canaries into the lifecycle details and asserting
/// `format_json` strips them with a `[REDACTED]` marker (the real lifecycle
/// payload carries no secrets/commands/PIDs/abs-paths, so this proves the
/// formatter path, not leak-freedom of the payload). The distinct
/// `opi package doctor` surface reports the same lifecycle + drift resolution.
/// This complements the runtime stable-code coverage in `execution_product.rs`.
#[test]
fn doctor_surfaces_emit_stable_redacted_package_codes() {
    use opi_coding_agent::cli::PackageCommand;
    use opi_coding_agent::package_activation;
    use opi_coding_agent::package_cli;
    use sha2::{Digest, Sha256};

    let user = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();

    // Execution-package fixture targeting the running host.
    let pkg = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(pkg.path().join("bin")).unwrap();
    let exe_content: &[u8] = b"#!/bin/sh\necho hi\n";
    let exe = pkg.path().join("bin").join("opi-sandbox");
    std::fs::write(&exe, exe_content).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let sha = format!("{:x}", Sha256::digest(exe_content));
    let target = package_activation::host_target_triple();
    let opi_version = package_activation::host_opi_version();
    let toml = format!(
        "version = \"0.8.0\"\n\
         opi_version = \"={opi_version}\"\n\
         name = \"opi-sandbox\"\n\
         description = \"doctor fixture\"\n\
         \n\
         [[contributions.adapters]]\n\
         capability = \"command.execute\"\n\
         id = \"opi-sandbox\"\n\
         transport = \"process-jsonl\"\n\
         command = \"bin/opi-sandbox\"\n\
         args = [\"backend\", \"--stdio\"]\n\
         protocol = \"command-execution-jsonl-v1\"\n\
         target = \"{target}\"\n\
         sha256 = \"{sha}\"\n\
         handshake_timeout_ms = 5000\n\
         adapter_config = {{}}\n"
    );
    std::fs::write(pkg.path().join("package.toml"), toml).unwrap();

    let exit = package_cli::handle_package_command(
        &PackageCommand::Add {
            source: pkg.path().to_str().unwrap().into(),
            local: false,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(exit, 0);

    let config = test_config("anthropic:claude-test-model");
    let context = DoctorContext {
        config: &config,
        config_error: None,
        workspace_root: workspace.path(),
        user_config_dir: user.path(),
        sessions_dir: sessions.path(),
        term: None,
        term_program: None,
        term_features: None,
        no_color: false,
        colorterm: None,
        env_var: &no_env,
        store_probe: &EMPTY_STORE_PROBE,
    };

    // Fresh installs are untrusted: keep the informational lifecycle observation
    // and also emit the actionable stable runtime code plus remediation.
    let report = run_doctor(&[DoctorScope::Package], &context);
    let lifecycle = report
        .entries
        .iter()
        .find(|e| e.diagnostic.code == "doctor_package_exec_lifecycle")
        .expect("stable lifecycle code on top-level doctor");
    assert_eq!(lifecycle.diagnostic.source, "package");
    let untrusted = report
        .entries
        .iter()
        .find(|e| e.diagnostic.code == "package_untrusted")
        .expect("stable execution code on top-level doctor");
    assert!(
        untrusted
            .diagnostic
            .details
            .as_ref()
            .and_then(|details| details.get("remediation"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|remediation| !remediation.is_empty())
    );

    // Redaction on the RENDERED JSON surface: doctor redacts at render time
    // (`format_json` calls `redacted_payload(Summary)` on every entry). To make
    // the render-time redaction-wiring claim non-vacuous, seed an abs-path +
    // secret canary into the lifecycle details and assert the rendered JSON
    // strips both and carries a [REDACTED] marker. (The real lifecycle payload
    // carries no secrets/commands/PIDs/abs-paths; this proves the FORMATTER path
    // redacts, not that the payload is leak-free by construction.)
    let canary_path = user
        .path()
        .join("C:\\Leak\\Canary\\secret.config")
        .to_string_lossy()
        .to_string();
    let canary_secret = "sk-proj-DOCTOR-CANARY-SECRET-1234567890abcdef";
    let mut canary_diagnostic = lifecycle.diagnostic.clone();
    if let Some(obj) = canary_diagnostic
        .details
        .as_mut()
        .and_then(|details| details.as_object_mut())
    {
        obj.insert("canary_path".into(), serde_json::json!(canary_path));
        obj.insert("canary_secret".into(), serde_json::json!(canary_secret));
    }
    let canary_report = DoctorReport {
        entries: vec![DoctorEntry {
            scope: DoctorScope::Package,
            diagnostic: canary_diagnostic,
        }],
    };
    let rendered_json = format_json(&canary_report);
    assert!(
        !rendered_json.contains(canary_secret),
        "rendered doctor JSON must not leak a credential: {rendered_json}"
    );
    assert!(
        !rendered_json.contains("secret.config"),
        "rendered doctor JSON must not leak an unnecessary absolute path: {rendered_json}"
    );
    // JSON carries the redaction marker on the scrubbed detail values.
    assert!(
        rendered_json.contains("[REDACTED]"),
        "rendered doctor JSON must carry a redaction marker: {rendered_json}"
    );

    // The package-doctor surface agrees that untrusted is actionable.
    let exit_untrusted = package_cli::handle_package_command(
        &PackageCommand::Doctor { json: false },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(exit_untrusted, 2);

    // Trusted but disabled uses contribution_disabled on the top-level doctor.
    let activation = package_activation::PackageActivationStore::global(user.path().to_path_buf());
    let mut records = activation.read_records().unwrap();
    records[0].trusted = true;
    records[0].enabled = true;
    activation.write_records(&records).unwrap();
    assert_eq!(
        package_cli::handle_package_command(
            &PackageCommand::Disable {
                name: "opi-sandbox".into(),
            },
            workspace.path().to_path_buf(),
            user.path().to_path_buf(),
        ),
        0
    );
    let disabled_report = run_doctor(&[DoctorScope::Package], &context);
    let disabled = disabled_report
        .entries
        .iter()
        .find(|e| e.diagnostic.code == "contribution_disabled")
        .expect("disabled package uses stable runtime code");
    assert!(disabled.diagnostic.action.is_some());

    // Re-enable before testing drift so the hash mismatch itself determines
    // the stable failure code.
    assert_eq!(
        package_cli::handle_package_command(
            &PackageCommand::Enable {
                name: "opi-sandbox".into(),
            },
            workspace.path().to_path_buf(),
            user.path().to_path_buf(),
        ),
        0
    );
    assert_eq!(run_doctor(&[DoctorScope::Package], &context).exit_code(), 0);

    // Drift invalidates Package Trust, so it correlates with runtime's
    // package_untrusted code and remediation.
    std::fs::write(&exe, b"#!/bin/sh\necho pwned\n").unwrap();
    let report2 = run_doctor(&[DoctorScope::Package], &context);
    let drift = report2
        .entries
        .iter()
        .find(|e| e.diagnostic.code == "package_untrusted")
        .expect("stable execution code on top-level doctor");
    assert_eq!(
        drift.diagnostic.severity,
        opi_agent::diagnostic::Severity::Error
    );
    assert!(
        drift
            .diagnostic
            .details
            .as_ref()
            .and_then(|details| details.get("remediation"))
            .and_then(serde_json::Value::as_str)
            .is_some(),
        "drift diagnostic must carry remediation: {:?}",
        drift.diagnostic,
    );
    assert!(
        drift.diagnostic.message.contains("opi-sandbox")
            && !drift.diagnostic.message.contains("pwned")
            && !drift.diagnostic.message.contains("echo"),
        "drift message must name only safe identifiers: {:?}",
        drift.diagnostic.message
    );
    // And the RENDERED output never carries the tampered content.
    let rendered_drift = format_json(&report2);
    assert!(
        !rendered_drift.contains("pwned"),
        "rendered doctor output must not leak the tampered executable content: {rendered_drift}"
    );
    let exit_drifted = package_cli::handle_package_command(
        &PackageCommand::Doctor { json: false },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(
        exit_drifted, 2,
        "package doctor must exit 2 on drift (same detection as top-level)"
    );
}
