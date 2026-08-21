//! Startup diagnostics for eagerly-built extra dispatch routes.
//!
//! A route whose construction fails for a non-secret CONFIG reason (bad proxy
//! URL, malformed profile) is dropped from the dispatch collection. Without a
//! diagnostic, the failure only surfaces later as an "unknown model" error
//! when the user switches to that provider, with nothing pointing back at the
//! broken setting. These tests pin the redacted skip diagnostic and the
//! fail-open startup posture.

use opi_coding_agent::config::{OpiConfig, ProviderProxyConfig, build_http_client};
use opi_coding_agent::credential_store::{
    FakeKeyringBackend, KEYCHAIN_PRESENCE_SERVICE, KEYCHAIN_SERVICE, KeyringBackendFactory,
};
use opi_coding_agent::provider_factory::build_provider_bundle;

/// A proxy URL the HTTP client rejects deterministically (unclosed IPv6 host
/// bracket), standing in for any invalid non-secret config value.
const BROKEN_PROXY_URL: &str = "http://[::1";

fn seeded_anthropic_backend() -> KeyringBackendFactory {
    Box::new(|| {
        let backend = FakeKeyringBackend::new();
        backend.seed_raw(
            KEYCHAIN_SERVICE,
            "anthropic",
            r#"{"version":1,"kind":"api_key","api_key":"test-route-diagnostics"}"#,
        );
        backend.seed_raw(KEYCHAIN_PRESENCE_SERVICE, "anthropic", "api_key");
        Box::new(backend)
    })
}

#[tokio::test]
async fn broken_extra_route_config_emits_redacted_skip_diagnostic() {
    // Precondition: the malformed proxy URL really is rejected by the shared
    // HTTP client builder, i.e. this is a Config-class construction failure.
    assert!(
        build_http_client(Some(&ProviderProxyConfig {
            url: BROKEN_PROXY_URL.to_owned(),
            no_proxy: None,
        }))
        .is_err(),
        "the malformed proxy URL must be rejected at client construction"
    );

    let dir = tempfile::tempdir().expect("temp dir");
    let mut config = OpiConfig::default();
    config.defaults.model = "anthropic:claude-sonnet-4-5-20250514".into();
    config.providers.openai.proxy = Some(ProviderProxyConfig {
        url: BROKEN_PROXY_URL.to_owned(),
        no_proxy: None,
    });

    // Startup stays fail-open: the broken non-active route is dropped, not
    // fatal.
    let bundle = build_provider_bundle(
        &config,
        dir.path().to_path_buf(),
        seeded_anthropic_backend(),
    )
    .await
    .expect("provider bundle builds despite the broken extra route");
    assert_eq!(bundle.provider.id(), "anthropic");

    let extra_ids: Vec<&str> = bundle.extra_routes.iter().map(|(p, _)| p.id()).collect();
    assert!(
        !extra_ids.contains(&"openai"),
        "the broken openai route must be dropped, got {extra_ids:?}"
    );
    assert!(
        extra_ids.contains(&"gemini"),
        "a sibling route without config problems stays registered, got {extra_ids:?}"
    );

    // Exactly one skip diagnostic names the dropped provider; it is redacted:
    // the config-derived URL text never crosses into it.
    use opi_agent::diagnostic::{RedactionMode, SOURCE_PROVIDER, Severity};
    let skips: Vec<_> = bundle
        .diagnostics
        .iter()
        .map(|d| d.redacted_payload(RedactionMode::Summary))
        .filter(|payload| payload.code == "provider_route_skipped")
        .collect();
    assert_eq!(skips.len(), 1, "diagnostics: {skips:?}");
    assert_eq!(skips[0].severity, Severity::Warning);
    assert_eq!(skips[0].source, SOURCE_PROVIDER);
    let encoded = serde_json::to_string(&skips[0]).expect("diagnostic serializes");
    assert!(encoded.contains("openai"), "{encoded}");
    assert!(
        !encoded.contains(BROKEN_PROXY_URL),
        "the raw proxy URL must not appear in the diagnostic: {encoded}"
    );
}

#[tokio::test]
async fn healthy_extra_routes_add_no_skip_diagnostics() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut config = OpiConfig::default();
    config.defaults.model = "anthropic:claude-sonnet-4-5-20250514".into();

    let bundle = build_provider_bundle(
        &config,
        dir.path().to_path_buf(),
        seeded_anthropic_backend(),
    )
    .await
    .expect("provider bundle builds");
    assert_eq!(bundle.provider.id(), "anthropic");
    assert!(
        bundle.diagnostics.iter().all(|d| {
            d.redacted_payload(opi_agent::diagnostic::RedactionMode::Summary)
                .code
                != "provider_route_skipped"
        }),
        "a default config with no broken extra routes emits no skip diagnostics: {:?}",
        bundle.diagnostics
    );
}
