//! Phase 14.1 credential store integration tests.
//!
//! Covers the versioned envelope codec (round-trip + malformed/unknown
//! distinct errors), the cross-process mutation lock (serialization + bounded
//! timeout under contention), the credential resolver (keychain-first with
//! headless env fallback), and the redaction invariant (only the secret-free
//! lock may exist on disk; secrets never reach error output). All tests use
//! the injected [`FakeKeyringBackend`] and a temp user-config root; none touch
//! the OS keychain.

use std::sync::Arc;
use std::time::Duration;

use opi_ai::credential::{Credential, CredentialSource, CredentialStore, CredentialStoreError};
use opi_coding_agent::credential_store::{
    ApiKeySource, CredentialResolver, EnvLookup, FakeKeyringBackend, KEYCHAIN_SERVICE,
    KeychainCredentialStore,
};
use secrecy::{ExposeSecret, SecretString};
use tempfile::TempDir;

const API_KEY: &str = "sk-test-api-key-DO-NOT-LEAK";
const ACCESS: &str = "atk-test-access-DO-NOT-LEAK";
const REFRESH: &str = "rtk-test-refresh-DO-NOT-LEAK";
const COPILOT_BASE_URL: &str = "https://copilot.enterprise.example/api";

fn secret(value: &str) -> SecretString {
    SecretString::new(value.to_owned().into_boxed_str())
}

/// A store over a fresh temp user-config root + the given fake backend, with a
/// short lock timeout suitable for contention tests.
fn store_with(backend: FakeKeyringBackend) -> (TempDir, KeychainCredentialStore) {
    let dir = TempDir::new().expect("temp dir");
    let store = KeychainCredentialStore::with_lock_timeout(
        Box::new(backend),
        dir.path().to_path_buf(),
        Duration::from_millis(80),
    );
    (dir, store)
}

fn api_key_credential() -> Credential {
    Credential::ApiKey(secret(API_KEY))
}

fn oauth_credential() -> Credential {
    Credential::OAuthToken {
        access: secret(ACCESS),
        refresh: secret(REFRESH),
        expires_at: None,
        base_url: Some(COPILOT_BASE_URL.to_owned()),
    }
}

#[tokio::test]
async fn api_key_envelope_round_trips_and_probes_present() {
    let (_dir, store) = store_with(FakeKeyringBackend::new());

    assert_eq!(store.probe("anthropic").await, CredentialSource::Absent);
    assert!(store.read("anthropic").await.unwrap().is_none());

    store
        .write("anthropic", &api_key_credential())
        .await
        .unwrap();

    let probed = store.probe("anthropic").await;
    assert!(
        matches!(probed, CredentialSource::Present { .. }),
        "expected Present, got {probed:?}"
    );
    // Probe label is non-secret.
    assert!(!format!("{probed:?}").contains(API_KEY));

    let read_back = store
        .read("anthropic")
        .await
        .unwrap()
        .expect("entry present after write");
    match read_back {
        Credential::ApiKey(key) => {
            assert_eq!(key.expose_secret(), API_KEY);
        }
        other => panic!("expected ApiKey, got {other:?}"),
    }
}

#[tokio::test]
async fn oauth_envelope_round_trips_and_preserves_base_url() {
    let (_dir, store) = store_with(FakeKeyringBackend::new());

    store.write("copilot", &oauth_credential()).await.unwrap();

    let read_back = store
        .read("copilot")
        .await
        .unwrap()
        .expect("oauth entry present");
    match read_back {
        Credential::OAuthToken {
            access,
            refresh,
            base_url,
            expires_at,
        } => {
            assert_eq!(access.expose_secret(), ACCESS);
            assert_eq!(refresh.expose_secret(), REFRESH);
            assert_eq!(base_url.as_deref(), Some(COPILOT_BASE_URL));
            assert!(expires_at.is_none());
        }
        other => panic!("expected OAuthToken, got {other:?}"),
    }
}

#[tokio::test]
async fn delete_removes_entry() {
    let (_dir, store) = store_with(FakeKeyringBackend::new());
    store.write("openai", &api_key_credential()).await.unwrap();
    assert!(matches!(
        store.probe("openai").await,
        CredentialSource::Present { .. }
    ));
    store.delete("openai").await.unwrap();
    assert_eq!(store.probe("openai").await, CredentialSource::Absent);
    assert!(store.read("openai").await.unwrap().is_none());
}

#[tokio::test]
async fn malformed_envelope_surfaces_distinct_error() {
    let backend = FakeKeyringBackend::new();
    backend.seed_raw(KEYCHAIN_SERVICE, "anthropic", "{ this is not json");
    let (_dir, store) = store_with(backend);

    match store.read("anthropic").await {
        Err(CredentialStoreError::MalformedEnvelope { provider, .. }) => {
            assert_eq!(provider, "anthropic");
        }
        other => panic!("expected MalformedEnvelope, got {other:?}"),
    }
}

#[tokio::test]
async fn unknown_envelope_version_surfaces_distinct_error() {
    // version 2 envelope: valid JSON + valid kind, but unknown version.
    let payload = r#"{"version":2,"kind":"api_key","api_key":"x"}"#;
    let backend = FakeKeyringBackend::new();
    backend.seed_raw(KEYCHAIN_SERVICE, "anthropic", payload);
    let (_dir, store) = store_with(backend);

    match store.read("anthropic").await {
        Err(CredentialStoreError::UnknownEnvelope {
            version: Some(2), ..
        }) => {}
        other => panic!("expected UnknownEnvelope version=2, got {other:?}"),
    }
}

#[tokio::test]
async fn unknown_envelope_kind_surfaces_distinct_error() {
    // version 1 but unknown kind.
    let payload = r#"{"version":1,"kind":"bogus","api_key":"x"}"#;
    let backend = FakeKeyringBackend::new();
    backend.seed_raw(KEYCHAIN_SERVICE, "anthropic", payload);
    let (_dir, store) = store_with(backend);

    match store.read("anthropic").await {
        Err(CredentialStoreError::UnknownEnvelope {
            kind: Some(ref k), ..
        }) if k == "bogus" => {}
        other => panic!("expected UnknownEnvelope kind=bogus, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mutation_lock_serializes_concurrent_writers() {
    // Two stores over CLONED (Arc-shared) backends + the same lock file: both
    // writes target genuinely shared state. The proof the lock serializes them
    // is that the two `set` critical-section windows do NOT overlap (without
    // the lock, two 120ms sets launched 20ms apart would overlap heavily).
    let dir = TempDir::new().unwrap();
    let backend = FakeKeyringBackend::new().with_set_delay(Duration::from_millis(120));
    let store_a = KeychainCredentialStore::with_lock_timeout(
        Box::new(backend.clone()),
        dir.path().to_path_buf(),
        Duration::from_secs(2),
    );
    let store_b = KeychainCredentialStore::with_lock_timeout(
        Box::new(backend.clone()),
        dir.path().to_path_buf(),
        Duration::from_secs(2),
    );

    let a = tokio::spawn(async move { store_a.write("anthropic", &api_key_credential()).await });
    // Give writer A a head start so it holds the lock first.
    tokio::time::sleep(Duration::from_millis(20)).await;
    let b = tokio::spawn(async move {
        store_b
            .write("anthropic", &Credential::ApiKey(secret("sk-other")))
            .await
    });
    a.await.unwrap().unwrap();
    b.await.unwrap().unwrap();

    // (1) Serialization proof: the two set windows do not overlap. If the lock
    // were removed from KeychainCredentialStore::write, both 120ms sets would
    // run concurrently and these windows would overlap.
    let mut windows = backend.set_windows();
    windows.sort_by_key(|(start, _)| *start);
    assert_eq!(
        windows.len(),
        2,
        "expected exactly two recorded set windows, got {windows:?}"
    );
    let (a_start, a_end) = windows[0];
    let (b_start, b_end) = windows[1];
    assert!(
        b_start >= a_end,
        "writers overlapped (not serialized): A={a_start:?}..{a_end:?}, B={b_start:?}..{b_end:?}"
    );

    // (2) No corruption: the persisted value is exactly one of the two written
    // secrets (the locked overwrite is atomic, never torn).
    let persisted = backend
        .raw_entry(KEYCHAIN_SERVICE, "anthropic")
        .expect("entry present after both writes");
    let valid = persisted.contains(API_KEY) ^ persisted.contains("sk-other");
    assert!(
        valid,
        "persisted envelope should contain exactly one written secret: {persisted:?}"
    );
    // The OTHER writer's secret must not be in the final envelope.
    if persisted.contains(API_KEY) {
        assert!(!persisted.contains("sk-other"));
    } else {
        assert!(!persisted.contains(API_KEY));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mutation_lock_times_out_under_contention() {
    // Writer A holds the lock for ~150ms (slow backend). Writer B has an 80ms
    // lock timeout and must time out with a redacted Backend error.
    let dir = TempDir::new().unwrap();
    let store_a = KeychainCredentialStore::with_lock_timeout(
        Box::new(FakeKeyringBackend::new().with_set_delay(Duration::from_millis(150))),
        dir.path().to_path_buf(),
        Duration::from_secs(2),
    );
    let store_b = KeychainCredentialStore::with_lock_timeout(
        Box::new(FakeKeyringBackend::new()),
        dir.path().to_path_buf(),
        Duration::from_millis(80),
    );

    let a = tokio::spawn(async move { store_a.write("anthropic", &api_key_credential()).await });
    tokio::time::sleep(Duration::from_millis(20)).await;
    let b_err = store_b
        .write("anthropic", &api_key_credential())
        .await
        .unwrap_err();
    // Let A finish so the temp dir cleanup is clean.
    a.await.unwrap().unwrap();

    match b_err {
        CredentialStoreError::Backend { reason, .. } => {
            assert!(
                reason.contains("timeout"),
                "expected timeout reason, got {reason:?}"
            );
            // Reason is coordination-only: never a secret.
            assert!(!reason.contains(API_KEY));
        }
        other => panic!("expected Backend(timeout), got {other:?}"),
    }
}

#[tokio::test]
async fn resolver_reads_api_key_from_store_when_present() {
    let (_dir, store) = store_with(FakeKeyringBackend::new());
    store
        .write("anthropic", &api_key_credential())
        .await
        .unwrap();
    let resolver = CredentialResolver::new(
        Arc::new(store),
        Arc::new(|_name: &str| -> Option<String> { None }),
    );

    let resolved = resolver
        .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
        .await
        .expect("resolved");
    assert_eq!(resolved.value.expose_secret(), API_KEY);
    assert!(matches!(resolved.source, ApiKeySource::Store));
}

#[tokio::test]
async fn resolver_falls_back_to_env_when_store_absent() {
    let (_dir, store) = store_with(FakeKeyringBackend::new());
    // Store is empty; env has the key.
    let env_lookup: EnvLookup = {
        let env_value = API_KEY.to_owned();
        Arc::new(move |_name: &str| Some(env_value.clone()))
    };
    let resolver = CredentialResolver::new(Arc::new(store), env_lookup);

    let resolved = resolver
        .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
        .await
        .expect("resolved from env");
    assert_eq!(resolved.value.expose_secret(), API_KEY);
    match resolved.source {
        ApiKeySource::Env {
            env_var,
            backend_unavailable,
        } => {
            assert_eq!(env_var, "ANTHROPIC_API_KEY");
            assert!(
                !backend_unavailable,
                "absent store must not set backend_unavailable"
            );
        }
        other => panic!("expected Env source, got {other:?}"),
    }
}

#[tokio::test]
async fn headless_api_key_env_fallback() {
    // Acceptance scenario: keychain backend unavailable -> resolver resolves
    // the API key from the configured env source, records the
    // backend-unavailable fallback flag, and exposes no plaintext artifact.
    let (_dir, store) = store_with(FakeKeyringBackend::new().with_unavailable());
    // Probe of an unavailable backend surfaces BackendUnavailable.
    assert!(matches!(
        store.probe("anthropic").await,
        CredentialSource::BackendUnavailable { .. }
    ));

    let env_lookup: EnvLookup = {
        let env_value = API_KEY.to_owned();
        Arc::new(move |_name: &str| Some(env_value.clone()))
    };
    let resolver = CredentialResolver::new(Arc::new(store), env_lookup);

    let resolved = resolver
        .resolve_api_key("anthropic", "ANTHROPIC_API_KEY")
        .await
        .expect("resolved from env on headless host");
    assert_eq!(resolved.value.expose_secret(), API_KEY);
    match resolved.source {
        ApiKeySource::Env {
            ref env_var,
            backend_unavailable,
        } => {
            assert_eq!(env_var, "ANTHROPIC_API_KEY");
            assert!(
                backend_unavailable,
                "headless fallback must report backend_unavailable"
            );
        }
        other => panic!("expected Env source, got {other:?}"),
    }

    // No plaintext artifact: the resolved key never appears in the source
    // debug, and the resolver holds the value only behind SecretString.
    let source_debug = format!("{:?}", resolved.source);
    assert!(
        !source_debug.contains(API_KEY),
        "source leaked key: {source_debug}"
    );
}

#[tokio::test]
async fn redaction_only_secret_free_lock_exists_outside_fake_keyring() {
    // After persisting a credential, the temp user-config root must contain
    // only the secret-free credential.lock — never a plaintext credential
    // artifact. The FakeKeyringBackend is in-memory, so no envelope is written
    // to disk at all. The scan also covers the read-back Credential Debug and
    // the formatted MalformedEnvelope error channel (spec lines 263-267).
    let backend = FakeKeyringBackend::new();
    // Seed a malformed payload that embeds the API-key secret, so the
    // formatted-error redaction check below is non-vacuous (a regression that
    // echoed the raw payload into the error would leak it).
    let malformed_with_secret = format!(r#"{{ "version": 1, "api_key": "{API_KEY}" "#);
    backend.seed_raw(
        KEYCHAIN_SERVICE,
        "malformed-provider",
        &malformed_with_secret,
    );
    let (dir, store) = store_with(backend);
    store
        .write("anthropic", &api_key_credential())
        .await
        .unwrap();
    store.write("copilot", &oauth_credential()).await.unwrap();

    let mut entries: Vec<std::ffi::OsString> = std::fs::read_dir(dir.path())
        .expect("read temp dir")
        .filter_map(Result::ok)
        .map(|e| e.file_name())
        .collect();
    entries.sort();
    assert_eq!(
        entries,
        vec![std::ffi::OsString::from("credential.lock")],
        "only the secret-free lock may exist outside the fake keyring"
    );

    // The lock file itself holds no secret.
    let lock_contents =
        std::fs::read_to_string(dir.path().join("credential.lock")).unwrap_or_default();
    for secret in [API_KEY, ACCESS, REFRESH] {
        assert!(
            !lock_contents.contains(secret),
            "lock file leaked secret {secret:?}"
        );
    }

    // Read-back credential Debug never leaks access/refresh (the serialized
    // envelope leaves the secret only behind SecretString's redacting Debug).
    let read_back = store
        .read("copilot")
        .await
        .unwrap()
        .expect("oauth entry present");
    let cred_debug = format!("{read_back:?}");
    for secret in [API_KEY, ACCESS, REFRESH] {
        assert!(
            !cred_debug.contains(secret),
            "Credential Debug leaked secret {secret:?}: {cred_debug}"
        );
    }

    // Formatted-error channel: a malformed envelope error never echoes the
    // secret-bearing payload (seeded above with an embedded API-key secret).
    let err = store.read("malformed-provider").await.unwrap_err();
    let err_display = format!("{err}");
    let err_debug = format!("{err:?}");
    for text in [&err_display, &err_debug] {
        assert!(
            !text.contains(API_KEY),
            "malformed-envelope error leaked the payload secret: {text}"
        );
        assert!(
            !text.contains(ACCESS) && !text.contains(REFRESH),
            "error leaked access/refresh: {text}"
        );
    }
}

#[tokio::test]
async fn keychain_store_reaches_production_construction() {
    // Acceptance scenario `phase14-keychain-backend-production-construction`:
    // instantiate KeychainCredentialStore over an injected fake keyring,
    // round-trip the envelope, compose the store with CredentialResolver, and
    // reach provider construction without touching the user keychain or writing
    // plaintext credential material.
    use opi_coding_agent::credential_store::{CredentialResolver, KeychainCredentialStore};

    let dir = TempDir::new().unwrap();
    let store: Arc<KeychainCredentialStore> = Arc::new(KeychainCredentialStore::new(
        Box::new(FakeKeyringBackend::new()),
        dir.path().to_path_buf(),
    ));

    // Round-trip a stored API-key envelope (the resolver's keychain source).
    store
        .write("anthropic", &api_key_credential())
        .await
        .unwrap();
    assert!(matches!(
        store.probe("anthropic").await,
        CredentialSource::Present { .. }
    ));

    // Compose with a resolver whose env lookup is empty, so the key must come
    // from the store.
    let resolver = CredentialResolver::new(Arc::clone(&store), {
        let lookup: EnvLookup = Arc::new(|_name: &str| -> Option<String> { None });
        lookup
    });

    let mut config = opi_coding_agent::config::OpiConfig::default();
    config.defaults.model = "anthropic:claude-store-construction".to_owned();
    let provider =
        opi_coding_agent::provider_factory::build_provider_with_resolver(&config, &resolver)
            .await
            .expect("provider constructs from the resolved key");
    assert_eq!(provider.id(), "anthropic");

    // No plaintext credential artifact on disk: only the secret-free lock.
    let mut entries: Vec<std::ffi::OsString> = std::fs::read_dir(dir.path())
        .expect("read temp dir")
        .filter_map(Result::ok)
        .map(|e| e.file_name())
        .collect();
    entries.sort();
    assert_eq!(
        entries,
        vec![std::ffi::OsString::from("credential.lock")],
        "only the secret-free lock may exist outside the fake keyring"
    );
    let lock_contents =
        std::fs::read_to_string(dir.path().join("credential.lock")).unwrap_or_default();
    assert!(!lock_contents.contains(API_KEY));
}
