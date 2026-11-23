//! Collection auth descriptors, credential probes and redaction.

use super::*;

static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------
// SecretKey redaction
// ---------------------------------------------------------------------------

#[test]
fn secret_key_redacts_in_debug_and_display() {
    let key = SecretKey::new(SECRET_VALUE);
    let debug = format!("{key:?}");
    let display = format!("{key}");
    assert_eq!(debug, "<redacted>");
    assert_eq!(display, "<redacted>");
    assert!(!debug.contains(SECRET_VALUE));
    assert!(!display.contains(SECRET_VALUE));
    // The value is still accessible programmatically by callers that need it.
    assert_eq!(key.as_str(), SECRET_VALUE);
    assert!(key.is_present());
}

#[test]
fn secret_key_empty_is_not_present() {
    let key = SecretKey::new("");
    assert!(!key.is_present());
}

// ---------------------------------------------------------------------------
// AuthDescriptor on-demand configuration checks (20.18: checks moved off the
// IO-free descriptor onto Models::check_auth; the descriptor itself stays
// secret-free data)
// ---------------------------------------------------------------------------

fn check_collection(descriptor: AuthDescriptor) -> Models {
    let mut collection = Models::new();
    collection
        .register(
            text_mock("checkprov", "ok"),
            descriptor,
            CompatMetadata::default(),
        )
        .unwrap();
    collection
}

#[tokio::test]
async fn auth_descriptor_static_key_checks_configured_and_missing() {
    let configured = check_collection(AuthDescriptor::StaticApiKey {
        value: SecretKey::new(SECRET_VALUE),
    });
    let check = configured
        .check_auth("checkprov")
        .await
        .unwrap()
        .expect("non-empty static key is configured");
    assert_eq!(check.kind, AuthCheckKind::ApiKey);
    assert!(!format!("{check:?}").contains(SECRET_VALUE));

    let missing = check_collection(AuthDescriptor::StaticApiKey {
        value: SecretKey::new(""),
    });
    assert!(
        missing.check_auth("checkprov").await.unwrap().is_none(),
        "empty static key is unconfigured"
    );
    let whitespace = check_collection(AuthDescriptor::StaticApiKey {
        value: SecretKey::new("   "),
    });
    assert!(
        whitespace.check_auth("checkprov").await.unwrap().is_none(),
        "whitespace-only static key is unconfigured"
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serializes process-env mutation; the awaited check never re-acquires this lock.
async fn auth_descriptor_env_key_checks_through_the_context_on_demand() {
    // Unset in the ambient environment: unconfigured, not an error.
    let unset = check_collection(AuthDescriptor::EnvApiKey {
        env_var: "OPI_TEST_PROV_COLL_DEFINITELY_UNSET_9F2A7C".into(),
    });
    assert!(unset.check_auth("checkprov").await.unwrap().is_none());

    // Mutate the shared environment across awaits under the test lock.
    let env_var = "OPI_TEST_PROV_COLL_WHITESPACE_ONLY_0E7B3D";
    let _guard = ENV_TEST_LOCK.lock().unwrap();
    let original = std::env::var_os(env_var);
    unsafe { std::env::set_var(env_var, "   ") };

    let whitespace = check_collection(AuthDescriptor::EnvApiKey {
        env_var: env_var.into(),
    });
    let check = whitespace.check_auth("checkprov").await.unwrap();
    assert!(check.is_none(), "whitespace-only env value is unconfigured");

    unsafe { std::env::set_var(env_var, "value-set-for-check") };
    let present = check_collection(AuthDescriptor::EnvApiKey {
        env_var: env_var.into(),
    });
    let check = present
        .check_auth("checkprov")
        .await
        .unwrap()
        .expect("env-backed provider is configured");

    match original {
        Some(value) => unsafe { std::env::set_var(env_var, value) },
        None => unsafe { std::env::remove_var(env_var) },
    }
    drop(_guard);

    assert_eq!(check.source.as_deref(), Some(env_var));
    assert_eq!(check.kind, AuthCheckKind::ApiKey);
    assert!(!format!("{check:?}").contains("value-set-for-check"));
}

#[tokio::test]
async fn auth_descriptor_resolved_label_checks_configured() {
    let configured = check_collection(AuthDescriptor::Resolved {
        source: "env OPI_TEST_RESOLVED_SOURCE".into(),
    });
    let check = configured
        .check_auth("checkprov")
        .await
        .unwrap()
        .expect("labeled source is configured");
    assert_eq!(
        check.source.as_deref(),
        Some("env OPI_TEST_RESOLVED_SOURCE")
    );

    let empty = check_collection(AuthDescriptor::Resolved {
        source: "   ".into(),
    });
    assert!(empty.check_auth("checkprov").await.unwrap().is_none());
}

const STORE_API_KEY: &str = "sk-store-api-key-DO-NOT-LEAK";
const STORE_ACCESS: &str = "atk-store-access-DO-NOT-LEAK";
const STORE_REFRESH: &str = "rtk-store-refresh-DO-NOT-LEAK";

fn store_descriptor(provider: &str) -> AuthDescriptor {
    AuthDescriptor::StoreCredential {
        key: provider.to_owned(),
        display_source: format!("keychain opi:{provider}"),
    }
}

#[test]
fn credential_source_display_label_and_presence() {
    let present = CredentialSource::Present {
        label: "keychain opi:anthropic".to_owned(),
    };
    assert!(present.is_present());
    assert_eq!(present.display_source(), "keychain opi:anthropic");

    let absent = CredentialSource::Absent;
    assert!(!absent.is_present());
    assert_eq!(absent.display_source(), "absent");

    let unavail = CredentialSource::BackendUnavailable {
        reason: "no keychain daemon".to_owned(),
    };
    assert!(!unavail.is_present());
    assert!(unavail.display_source().contains("no keychain daemon"));
}

#[test]
fn credential_api_key_redacts_in_debug() {
    let cred = Credential::ApiKey(secret(STORE_API_KEY).into());
    let debug = format!("{cred:?}");
    assert!(
        !debug.contains(STORE_API_KEY),
        "ApiKey leaked in Debug: {debug}"
    );
    assert!(debug.contains("redacted"));
}

#[test]
fn credential_oauth_token_redacts_secrets_but_keeps_base_url() {
    let cred = Credential::OAuthToken {
        access: secret(STORE_ACCESS),
        refresh: secret(STORE_REFRESH),
        expires_at: None,
        base_url: Some("https://copilot.example/api".to_owned()),
        account_id: None,
        client_id: None,
        enterprise_domain: None,
        available_model_ids: None,
        scopes: None,
    };
    let debug = format!("{cred:?}");
    assert!(!debug.contains(STORE_ACCESS), "access leaked: {debug}");
    assert!(!debug.contains(STORE_REFRESH), "refresh leaked: {debug}");
    // base_url is non-secret and must survive redaction.
    assert!(
        debug.contains("https://copilot.example/api"),
        "base_url dropped: {debug}"
    );
}

#[tokio::test]
async fn auth_descriptor_store_credential_checks_via_probe_snapshot() {
    // Empty constructor-owned memory without a probe snapshot leaves the
    // store-backed provider unconfigured.
    let mut collection = Models::new();
    collection
        .register(
            text_mock("anthropic", "ok"),
            store_descriptor("anthropic"),
            CompatMetadata::default(),
        )
        .unwrap();
    assert!(collection.check_auth("anthropic").await.unwrap().is_none());

    // An injected Present probe snapshot checks configured, label-only.
    collection.set_probe(
        "anthropic",
        opi_ai::CredentialSource::Present {
            label: "keychain opi:anthropic".into(),
        },
    );
    let check = collection
        .check_auth("anthropic")
        .await
        .unwrap()
        .expect("present probe snapshot is configured");
    assert_eq!(check.source.as_deref(), Some("keychain opi:anthropic"));
    assert!(
        collection
            .get_auth("anthropic", Default::default())
            .await
            .unwrap()
            .is_none(),
        "a configured metadata probe never supplies authentication"
    );

    // The descriptor itself carries no secret material.
    let debug = format!("{:?}", store_descriptor("anthropic"));
    assert!(!debug.contains(STORE_API_KEY));
    assert!(debug.contains("anthropic"));
}

#[tokio::test]
async fn store_credential_dispatch_proceeds_when_probe_present() {
    // A present credential resolves and dispatches through prepare_call.
    let mut collection = Models::new();
    collection
        .register_route(
            text_mock("storeprov", "ok"),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                secret(STORE_API_KEY),
            )) as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();
    let prepared = collection
        .prepare_call(
            "storeprov:mock-model",
            minimal_request("storeprov:mock-model"),
        )
        .await;
    assert!(
        prepared.is_ok(),
        "present credential should dispatch: {:?}",
        prepared.err()
    );
}

#[tokio::test]
async fn store_credential_dispatch_rejects_when_probe_absent_with_redacted_detail() {
    // An absent credential terminates the call before the provider is touched.
    let mut collection = Models::new();
    collection
        .register_route(
            text_mock("absentprov", "should not stream"),
            Arc::new(MissingAuthResolver {
                provider_id: "absentprov".to_owned(),
            }) as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();

    let err = collection
        .prepare_call(
            "absentprov:mock-model",
            minimal_request("absentprov:mock-model"),
        )
        .await
        .expect_err("absent credential must reject dispatch");
    assert!(
        matches!(
            accepted_models_failure_cause(&err, &minimal_request("absentprov:mock-model")),
            ProviderError::CredentialNeeded { provider_id } if provider_id == "absentprov"
        ),
        "got {err:?}"
    );
    // Diagnostic never carries the secret value.
    assert!(!format!("{err}").contains(STORE_API_KEY));
}

#[tokio::test]
async fn store_credential_dispatch_proceeds_when_backend_unavailable() {
    // Dispatch proceeds when auth resolves; backend availability is the live
    // resolver's concern, not the non-live status gate.
    let mut collection = Models::new();
    collection
        .register_route(
            text_mock("unavailprov", "ok"),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                secret(STORE_API_KEY),
            )) as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();
    let prepared = collection
        .prepare_call(
            "unavailprov:mock-model",
            minimal_request("unavailprov:mock-model"),
        )
        .await;
    assert!(
        prepared.is_ok(),
        "resolving credential should dispatch: {:?}",
        prepared.err()
    );
}

#[tokio::test]
async fn store_credential_dispatch_proceeds_when_no_probe_injected() {
    // No probe is needed on the dispatch path: a route with a resolver
    // dispatches regardless of the (metadata-only) probe substrate.
    let mut collection = Models::new();
    collection
        .register_route(
            text_mock("noprobe", "ok"),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                secret(STORE_API_KEY),
            )) as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();
    assert_eq!(collection.probe("noprobe"), None);
    let prepared = collection
        .prepare_call("noprobe:mock-model", minimal_request("noprobe:mock-model"))
        .await;
    assert!(
        prepared.is_ok(),
        "route should dispatch without a probe: {:?}",
        prepared.err()
    );
}

#[tokio::test]
async fn credential_store_is_object_safe_and_round_trips() {
    let store: Arc<dyn CredentialStore> = Arc::new(FakeStore {
        entries: Mutex::new(HashMap::new()),
    });
    // `Arc<dyn CredentialStore>` compiles => the trait is object-safe.

    // Missing entry probes Absent and reads None.
    assert_eq!(store.probe("anthropic").await, CredentialSource::Absent);
    assert!(store.read("anthropic").await.unwrap().is_none());

    let api_key = Credential::ApiKey(secret(STORE_API_KEY).into());
    store.write("anthropic", &api_key).await.unwrap();

    // Present probe carries only the non-secret label.
    let probed = store.probe("anthropic").await;
    assert_eq!(
        probed,
        CredentialSource::Present {
            label: "keychain opi:anthropic".to_owned()
        }
    );
    assert!(!format!("{probed:?}").contains(STORE_API_KEY));

    let read_back = store
        .read("anthropic")
        .await
        .unwrap()
        .expect("entry present after write");
    assert!(matches!(read_back, Credential::ApiKey(_)));
    // Debug of the read-back credential never leaks the secret.
    assert!(!format!("{read_back:?}").contains(STORE_API_KEY));

    store.delete("anthropic").await.unwrap();
    assert_eq!(store.probe("anthropic").await, CredentialSource::Absent);
    assert!(store.read("anthropic").await.unwrap().is_none());
}
