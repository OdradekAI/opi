//! Behavioral tests for the provider collection/auth seam (task 10.1).
//!
//! DoD: opi-ai exposes a provider collection/auth contract that owns provider
//! and model lookup, static API-key and env-auth descriptors, optional refresh
//! capability, OpenAI-compatible compatibility metadata, stream dispatch, an
//! explicit complete-dispatch decision compatible with the current streaming
//! Provider trait, redacted missing/invalid auth diagnostics, and a registry
//! regression asserting all built-in providers still resolve.

use opi_ai::message::{AssistantContent, AssistantMessage};
use opi_ai::provider::{CacheRetention, Provider, Request, ThinkingConfig};
use opi_ai::provider_collection::{
    AuthDescriptor, AuthStatus, CollectionError, CompatMetadata, CompletedRequest,
    ProviderCollection, SecretKey,
};
use opi_ai::registry::ProviderRegistry;
use opi_ai::test_support::{MockProvider, text_response};
use opi_ai::{ModelCapabilities, ModelInfo, WireApi};
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Request/message helpers
// ---------------------------------------------------------------------------

fn minimal_request(model: &str) -> Request {
    Request {
        model: model.into(),
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
    }
}

/// Concatenate all text content carried by an assistant message.
fn assistant_text(message: &AssistantMessage) -> String {
    message
        .content
        .iter()
        .filter_map(|content| match content {
            AssistantContent::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// Build a mock provider that streams a single text response.
fn text_mock(id: &str, text: &str) -> Box<dyn Provider> {
    Box::new(MockProvider::new(id, vec![text_response(text)]))
}

/// Build a mock provider with `count` identical text response batches, for
/// tests that dispatch more than once.
fn text_mock_repeated(id: &str, text: &str, count: usize) -> Box<dyn Provider> {
    let responses = (0..count).map(|_| text_response(text)).collect();
    Box::new(MockProvider::new(id, responses))
}

const SECRET_VALUE: &str = "sk-super-secret-value-DO-NOT-LEAK";

struct StreamProvider {
    id: &'static str,
    events:
        Mutex<Option<Vec<Result<opi_ai::AssistantStreamEvent, opi_ai::provider::ProviderError>>>>,
}

impl StreamProvider {
    fn new(
        id: &'static str,
        events: Vec<Result<opi_ai::AssistantStreamEvent, opi_ai::provider::ProviderError>>,
    ) -> Self {
        Self {
            id,
            events: Mutex::new(Some(events)),
        }
    }
}

impl Provider for StreamProvider {
    fn id(&self) -> &str {
        self.id
    }

    fn models(&self) -> &[opi_ai::provider::ModelInfo] {
        static MODELS: std::sync::OnceLock<Vec<opi_ai::provider::ModelInfo>> =
            std::sync::OnceLock::new();
        MODELS.get_or_init(|| {
            vec![opi_ai::provider::ModelInfo::new(
                "mock-model",
                "Mock Model",
                WireApi::OpenAiCompletions,
                ModelCapabilities::new(128_000, 4096).with_streaming(true),
            )]
        })
    }

    fn stream(&self, _request: Request) -> opi_ai::provider::EventStream {
        let events = self.events.lock().unwrap().take().unwrap_or_default();
        Box::pin(futures_util::stream::iter(events))
    }
}

static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

fn with_env_var<R>(name: &str, value: Option<&str>, f: impl FnOnce() -> R) -> R {
    let _guard = ENV_TEST_LOCK.lock().unwrap();
    let original = std::env::var_os(name);
    match value {
        Some(value) => unsafe { std::env::set_var(name, value) },
        None => unsafe { std::env::remove_var(name) },
    }

    let result = f();

    match original {
        Some(value) => unsafe { std::env::set_var(name, value) },
        None => unsafe { std::env::remove_var(name) },
    }
    result
}

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
// AuthDescriptor resolution (redacted, no provider needed)
// ---------------------------------------------------------------------------

#[test]
fn auth_descriptor_static_key_resolves_configured_and_missing() {
    let configured = AuthDescriptor::StaticApiKey {
        value: SecretKey::new(SECRET_VALUE),
    };
    assert_eq!(configured.resolve(), AuthStatus::Configured);

    let missing = AuthDescriptor::StaticApiKey {
        value: SecretKey::new(""),
    };
    match missing.resolve() {
        AuthStatus::Missing { source } => {
            // Source names the reason but never leaks a value.
            assert!(!source.contains(SECRET_VALUE));
        }
        other => panic!("expected Missing, got {other:?}"),
    }
}

#[test]
fn auth_descriptor_treats_whitespace_as_missing() {
    let missing = AuthDescriptor::StaticApiKey {
        value: SecretKey::new("   "),
    };
    assert!(matches!(missing.resolve(), AuthStatus::Missing { .. }));
}

#[test]
fn auth_descriptor_env_key_missing_when_var_unset() {
    // Read-only: relies on the var being unset. Unique name avoids collisions.
    let descriptor = AuthDescriptor::EnvApiKey {
        env_var: "OPI_TEST_PROV_COLL_DEFINITELY_UNSET_9F2A7C".into(),
    };
    match descriptor.resolve() {
        AuthStatus::Missing { source } => {
            assert!(source.contains("OPI_TEST_PROV_COLL_DEFINITELY_UNSET_9F2A7C"));
            assert!(!source.contains(SECRET_VALUE));
        }
        AuthStatus::Configured => panic!("expected Missing for unset env var"),
    }
}

#[test]
fn auth_descriptor_env_key_treats_whitespace_as_missing() {
    let env_var = "OPI_TEST_PROV_COLL_WHITESPACE_ONLY_0E7B3D";
    let status = with_env_var(env_var, Some("   "), || {
        AuthDescriptor::EnvApiKey {
            env_var: env_var.into(),
        }
        .resolve()
    });
    match status {
        AuthStatus::Missing { source } => {
            assert!(source.contains(env_var));
            assert!(source.contains("set but empty"));
            assert!(!source.contains(SECRET_VALUE));
        }
        AuthStatus::Configured => panic!("expected Missing for whitespace-only env var"),
    }
}

#[test]
fn auth_descriptor_resolved_is_configured_without_secret_value() {
    let descriptor = AuthDescriptor::Resolved {
        source: "env OPI_TEST_RESOLVED_SOURCE".into(),
    };
    assert_eq!(descriptor.resolve(), AuthStatus::Configured);
    let rendered = format!("{descriptor:?}");
    assert!(rendered.contains("OPI_TEST_RESOLVED_SOURCE"));
    assert!(!rendered.contains(SECRET_VALUE));
}

#[test]
fn auth_descriptor_resolved_treats_empty_source_as_missing() {
    let descriptor = AuthDescriptor::Resolved {
        source: "   ".into(),
    };
    assert!(matches!(descriptor.resolve(), AuthStatus::Missing { .. }));
}

// ---------------------------------------------------------------------------
// Acceptance scenario: provider_collection_dispatches_with_redacted_auth
// ---------------------------------------------------------------------------

#[tokio::test]
async fn provider_collection_dispatches_with_redacted_auth() {
    let mut collection = ProviderCollection::new();
    collection
        .register(
            text_mock_repeated("mock", "hello from mock", 2),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new(SECRET_VALUE),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    // Auth status is Configured and the descriptor never leaks the secret.
    assert_eq!(collection.auth_status("mock"), Some(AuthStatus::Configured));
    let descriptor_debug = format!("{:?}", collection.auth_descriptor("mock").unwrap());
    assert!(!descriptor_debug.contains(SECRET_VALUE));

    // Stream dispatch flows through the collection and reaches Done.
    let stream = collection
        .dispatch_stream("mock:mock-model", minimal_request("mock:mock-model"))
        .unwrap();
    use futures_util::StreamExt;
    let events: Vec<_> = stream.collect::<Vec<_>>().await;
    let done_message = events
        .into_iter()
        .filter_map(|event| match event.unwrap() {
            opi_ai::AssistantStreamEvent::Done { message, .. } => Some(message),
            _ => None,
        })
        .next()
        .expect("stream produced a Done event");
    assert_eq!(assistant_text(&done_message), "hello from mock");

    // Complete-dispatch decision: drain the streaming trait to a terminal.
    let completed = collection
        .dispatch_complete("mock:mock-model", minimal_request("mock:mock-model"))
        .await
        .unwrap();
    match completed {
        CompletedRequest::Done { message, .. } => {
            assert_eq!(assistant_text(&message), "hello from mock");
        }
        other => panic!("expected CompletedRequest::Done, got {other:?}"),
    }
}

#[tokio::test]
async fn provider_collection_dispatch_rejects_missing_auth_with_redacted_diagnostic() {
    let mut collection = ProviderCollection::new();
    collection
        .register(
            text_mock("noauth", "should not stream"),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new(""),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    assert!(matches!(
        collection.auth_status("noauth"),
        Some(AuthStatus::Missing { .. })
    ));

    let err = match collection
        .dispatch_stream("noauth:mock-model", minimal_request("noauth:mock-model"))
    {
        Err(error) => error,
        Ok(_) => panic!("expected AuthNotConfigured error, got a stream"),
    };
    match err {
        CollectionError::AuthNotConfigured {
            ref provider,
            ref detail,
        } => {
            assert_eq!(provider.as_str(), "noauth");
            // Diagnostic is redacted: it never carries the secret value.
            assert!(!detail.contains(SECRET_VALUE));
            assert!(!format!("{err}").contains(SECRET_VALUE));
        }
        other => panic!("expected AuthNotConfigured, got {other:?}"),
    }

    // Complete-dispatch also rejects before touching the provider.
    let complete_err = collection
        .dispatch_complete("noauth:mock-model", minimal_request("noauth:mock-model"))
        .await
        .unwrap_err();
    assert!(matches!(
        complete_err,
        CollectionError::AuthNotConfigured { .. }
    ));
}

#[tokio::test]
async fn dispatch_complete_returns_terminal_error_event() {
    let message = AssistantMessage {
        content: vec![AssistantContent::Text {
            text: "terminal failure".into(),
        }],
        api: opi_ai::ApiKind::OpenAi,
        provider: "mock".into(),
        model: "mock-model".into(),
        response_model: None,
        response_id: None,
        usage: opi_ai::stream::Usage::default(),
        stop_reason: opi_ai::stream::StopReason::Error,
        error_message: Some("terminal failure".into()),
        timestamp_ms: 0,
    };
    let mut collection = ProviderCollection::new();
    collection
        .register(
            Box::new(StreamProvider::new(
                "mock",
                vec![Ok(opi_ai::AssistantStreamEvent::Error {
                    reason: opi_ai::stream::StopReason::Error,
                    message,
                })],
            )),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("configured"),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    let completed = collection
        .dispatch_complete("mock:mock-model", minimal_request("mock:mock-model"))
        .await
        .unwrap();
    assert!(matches!(completed, CompletedRequest::Error { .. }));
}

#[tokio::test]
async fn dispatch_complete_propagates_mid_stream_provider_error() {
    let mut collection = ProviderCollection::new();
    collection
        .register(
            Box::new(StreamProvider::new(
                "mock",
                vec![Err(opi_ai::provider::ProviderError::StreamError(
                    "mid-stream failure".into(),
                ))],
            )),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("configured"),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    let err = collection
        .dispatch_complete("mock:mock-model", minimal_request("mock:mock-model"))
        .await
        .unwrap_err();
    assert!(matches!(err, CollectionError::Provider(_)));
    assert!(err.to_string().contains("mid-stream failure"));
}

#[tokio::test]
async fn dispatch_complete_rejects_empty_stream() {
    let mut collection = ProviderCollection::new();
    collection
        .register(
            Box::new(StreamProvider::new("mock", Vec::new())),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("configured"),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    let err = collection
        .dispatch_complete("mock:mock-model", minimal_request("mock:mock-model"))
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("stream ended without a terminal event")
    );
}

// ---------------------------------------------------------------------------
// Acceptance scenario: collection_supports_provider_correctness_fixtures
// ---------------------------------------------------------------------------

#[tokio::test]
async fn collection_supports_provider_correctness_fixtures() {
    use opi_ai::provider::ModelInfo;

    // An OpenAI-compatible profile provider, as Phase 12 fixtures will exercise.
    let profile_model = ModelInfo::new(
        "profile-model",
        "Profile Model",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(128_000, 4_096)
            .with_images(true)
            .with_streaming(true),
    );
    let profile_provider = Box::new(MockProvider::new_with_models(
        "openrouter-profile",
        vec![profile_model],
        vec![text_response("profile response")],
    ));

    let mut collection = ProviderCollection::new();
    collection
        .register(
            profile_provider,
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new(SECRET_VALUE),
            },
            CompatMetadata {
                openai_compatible: true,
                profile: Some("openrouter".into()),
            },
        )
        .unwrap();

    // Model lookup through the collection (no CLI harness constructed).
    let (resolved, model) = collection
        .resolve("openrouter-profile:profile-model")
        .unwrap();
    assert_eq!(resolved.id(), "openrouter-profile");
    assert_eq!(model.id, "profile-model");

    let caps = collection
        .capabilities("openrouter-profile:profile-model")
        .unwrap();
    assert_eq!(caps.context_window, 128_000);
    assert!(caps.supports_images);

    // Compatibility metadata has a home on the collection.
    let compat = collection.compat("openrouter-profile").unwrap();
    assert!(compat.openai_compatible);
    assert_eq!(compat.profile.as_deref(), Some("openrouter"));

    // Auth diagnostics resolve through the collection.
    let status = collection.auth_status("openrouter-profile").unwrap();
    assert_eq!(status, AuthStatus::Configured);
    let descriptor_debug = format!(
        "{:?}",
        collection.auth_descriptor("openrouter-profile").unwrap()
    );
    assert!(!descriptor_debug.contains(SECRET_VALUE));

    // Stream dispatch works without the CLI product harness.
    let completed = collection
        .dispatch_complete(
            "openrouter-profile:profile-model",
            minimal_request("openrouter-profile:profile-model"),
        )
        .await
        .unwrap();
    match completed {
        CompletedRequest::Done { message, .. } => {
            assert_eq!(assistant_text(&message), "profile response");
        }
        other => panic!("expected CompletedRequest::Done, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Optional refresh extension point (documented no-op in Phase 10)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn collection_refresh_is_a_documented_noop_extension_point() {
    let mut collection = ProviderCollection::new();
    let result = collection.refresh().await;
    assert!(result.is_ok());
}

// ---------------------------------------------------------------------------
// from_registry wrapping (for the coding-agent provider factory in 10.2)
// ---------------------------------------------------------------------------

#[test]
fn collection_wraps_existing_registry_via_from_registry() {
    let mut registry = ProviderRegistry::new();
    registry.register(text_mock("wrapped", "wrapped response"));

    let collection = ProviderCollection::from_registry(registry);
    // Underlying registry is accessible for list-models / overrides.
    assert_eq!(collection.registry().provider_ids(), vec!["wrapped"]);
    // Model lookup flows through the wrapped registry.
    let (provider, _) = collection.resolve("wrapped:mock-model").unwrap();
    assert_eq!(provider.id(), "wrapped");
    // Auth descriptor defaults to absent for pre-registered providers.
    assert!(collection.auth_descriptor("wrapped").is_none());
    assert!(collection.auth_status("wrapped").is_none());
}

// ---------------------------------------------------------------------------
// Phase 14.1: credential store substrate + StoreCredential dispatch gate
//
// These cover the opi-ai substrate only: the Credential/CredentialSource
// types, the object-safe CredentialStore trait, the AuthDescriptor::
// StoreCredential variant, and the precomputed-probe dispatch gate. Concrete
// keychain/env/resolver/lock behavior lives in opi-coding-agent tests.
// ---------------------------------------------------------------------------

use opi_ai::credential::{
    BoxAuthFuture, Credential, CredentialSource, CredentialStore, CredentialStoreError,
};
use std::collections::HashMap;
// `Mutex` is already imported at the top of this file; only `Arc` is new here.
use std::sync::Arc;

fn secret(value: &str) -> secrecy::SecretString {
    // secrecy 0.10 SecretString::new takes `Box<str>`.
    secrecy::SecretString::new(value.into())
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
    let cred = Credential::ApiKey(secret(STORE_API_KEY));
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

#[test]
fn auth_descriptor_store_credential_resolves_configured_not_authoritative() {
    // resolve() is intentionally not authoritative for StoreCredential: the
    // descriptor is secret-free and cannot perform IO, so it returns
    // Configured (never blocks). The real gate is the injected probe
    // consulted in dispatch_stream.
    let descriptor = store_descriptor("anthropic");
    assert_eq!(descriptor.resolve(), AuthStatus::Configured);
    // The descriptor itself carries no secret material.
    let debug = format!("{descriptor:?}");
    assert!(!debug.contains(STORE_API_KEY));
    assert!(debug.contains("anthropic"));
}

#[tokio::test]
async fn store_credential_dispatch_proceeds_when_probe_present() {
    let mut collection = ProviderCollection::new();
    collection
        .register(
            text_mock("storeprov", "ok"),
            store_descriptor("storeprov"),
            CompatMetadata::default(),
        )
        .unwrap();
    collection.set_probe(
        "storeprov",
        CredentialSource::Present {
            label: "keychain opi:storeprov".to_owned(),
        },
    );
    let stream = collection.dispatch_stream(
        "storeprov:mock-model",
        minimal_request("storeprov:mock-model"),
    );
    assert!(
        stream.is_ok(),
        "Present probe should dispatch: {:?}",
        stream.err()
    );
}

#[tokio::test]
async fn store_credential_dispatch_rejects_when_probe_absent_with_redacted_detail() {
    let mut collection = ProviderCollection::new();
    collection
        .register(
            text_mock("absentprov", "should not stream"),
            store_descriptor("absentprov"),
            CompatMetadata::default(),
        )
        .unwrap();
    collection.set_probe("absentprov", CredentialSource::Absent);

    // Match instead of unwrap_err: the stream Ok type is not Debug.
    let err = match collection.dispatch_stream(
        "absentprov:mock-model",
        minimal_request("absentprov:mock-model"),
    ) {
        Err(error) => error,
        Ok(_) => panic!("expected AuthNotConfigured error, got a stream"),
    };
    match err {
        CollectionError::AuthNotConfigured { provider, detail } => {
            assert_eq!(provider, "absentprov");
            // detail is the redacted display_source, never a secret.
            assert_eq!(detail, "keychain opi:absentprov");
            assert!(!detail.contains(STORE_API_KEY));
        }
        other => panic!("expected AuthNotConfigured, got {other:?}"),
    }
}

#[tokio::test]
async fn store_credential_dispatch_proceeds_when_backend_unavailable() {
    // BackendUnavailable proceeds intentionally: keychain-required enforcement
    // is the live per-stream resolver's job (T2), not this non-live status gate.
    let mut collection = ProviderCollection::new();
    collection
        .register(
            text_mock("unavailprov", "ok"),
            store_descriptor("unavailprov"),
            CompatMetadata::default(),
        )
        .unwrap();
    collection.set_probe(
        "unavailprov",
        CredentialSource::BackendUnavailable {
            reason: "no daemon".to_owned(),
        },
    );
    let stream = collection.dispatch_stream(
        "unavailprov:mock-model",
        minimal_request("unavailprov:mock-model"),
    );
    assert!(
        stream.is_ok(),
        "BackendUnavailable should dispatch: {:?}",
        stream.err()
    );
}

#[tokio::test]
async fn store_credential_dispatch_proceeds_when_no_probe_injected() {
    // No probe injected -> non-gated, matching from_registry's "no descriptor"
    // semantics. The factory always injects a probe for StoreCredential
    // providers before listing; this pins the fallback contract.
    let mut collection = ProviderCollection::new();
    collection
        .register(
            text_mock("noprobe", "ok"),
            store_descriptor("noprobe"),
            CompatMetadata::default(),
        )
        .unwrap();
    assert_eq!(collection.probe("noprobe"), None);
    let stream =
        collection.dispatch_stream("noprobe:mock-model", minimal_request("noprobe:mock-model"));
    assert!(
        stream.is_ok(),
        "no-probe should be non-gated: {:?}",
        stream.err()
    );
}

/// In-memory fake credential store proving the trait is object-safe behind
/// `Arc<dyn CredentialStore>` and exercising probe/read/write/delete without
/// any keychain.
struct FakeStore {
    entries: Mutex<HashMap<String, Credential>>,
}

impl CredentialStore for FakeStore {
    fn read<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, Result<Option<Credential>, CredentialStoreError>> {
        Box::pin(async move { Ok(self.entries.lock().unwrap().get(provider_id).cloned()) })
    }
    fn write<'a>(
        &'a self,
        provider_id: &'a str,
        cred: &'a Credential,
    ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
        Box::pin(async move {
            self.entries
                .lock()
                .unwrap()
                .insert(provider_id.to_owned(), cred.clone());
            Ok(())
        })
    }
    fn delete<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
        Box::pin(async move {
            self.entries.lock().unwrap().remove(provider_id);
            Ok(())
        })
    }
    fn probe<'a>(&'a self, provider_id: &'a str) -> BoxAuthFuture<'a, CredentialSource> {
        Box::pin(async move {
            if self.entries.lock().unwrap().contains_key(provider_id) {
                CredentialSource::Present {
                    label: format!("keychain opi:{provider_id}"),
                }
            } else {
                CredentialSource::Absent
            }
        })
    }
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

    let api_key = Credential::ApiKey(secret(STORE_API_KEY));
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

// ---------------------------------------------------------------------------
// Task 14.6: Dynamic provider model refresh (substrate-only)
// ---------------------------------------------------------------------------

/// A mock provider that implements `refresh_models` to return a dynamic catalog.
struct RefreshProvider {
    id: &'static str,
    builtin_models: Vec<ModelInfo>,
    refreshed_models: std::sync::Mutex<Option<Vec<ModelInfo>>>,
    events: std::sync::Mutex<
        Option<Vec<Result<opi_ai::AssistantStreamEvent, opi_ai::provider::ProviderError>>>,
    >,
}

impl RefreshProvider {
    fn new(id: &'static str, builtin_models: Vec<ModelInfo>) -> Self {
        Self {
            id,
            builtin_models,
            refreshed_models: std::sync::Mutex::new(None),
            events: std::sync::Mutex::new(Some(Vec::new())),
        }
    }

    fn with_refresh(mut self, models: Vec<ModelInfo>) -> Self {
        self.refreshed_models = std::sync::Mutex::new(Some(models));
        self
    }
}

impl Provider for RefreshProvider {
    fn id(&self) -> &str {
        self.id
    }

    fn models(&self) -> &[ModelInfo] {
        // Leak a static to satisfy the &[ModelInfo] return type.
        // Only used in tests; the leak is intentional and bounded.
        Box::leak(self.builtin_models.clone().into_boxed_slice())
    }

    fn stream(&self, _request: Request) -> opi_ai::provider::EventStream {
        let events = self.events.lock().unwrap().take().unwrap_or_default();
        Box::pin(futures_util::stream::iter(events))
    }

    fn refresh_models(
        &self,
    ) -> opi_ai::credential::BoxAuthFuture<
        '_,
        Result<Option<Vec<ModelInfo>>, opi_ai::provider::ProviderError>,
    > {
        let result = self.refreshed_models.lock().unwrap().clone();
        Box::pin(async move { Ok(result) })
    }
}

fn model_info(id: &str, display: &str) -> ModelInfo {
    ModelInfo::new(
        id,
        display,
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(128_000, 4096),
    )
}

/// A mock provider whose `refresh_models` returns an error.
struct ErrorRefreshProvider {
    id: &'static str,
    error_msg: &'static str,
}

impl Provider for ErrorRefreshProvider {
    fn id(&self) -> &str {
        self.id
    }

    fn models(&self) -> &[ModelInfo] {
        &[]
    }

    fn stream(&self, _request: Request) -> opi_ai::provider::EventStream {
        Box::pin(futures_util::stream::empty())
    }

    fn refresh_models(
        &self,
    ) -> opi_ai::credential::BoxAuthFuture<
        '_,
        Result<Option<Vec<ModelInfo>>, opi_ai::provider::ProviderError>,
    > {
        let msg = self.error_msg.to_owned();
        Box::pin(async move { Err(opi_ai::provider::ProviderError::ProviderSide(msg)) })
    }
}

/// Shared mutable refresh catalog behind an `Arc<Mutex<>>` so tests can
/// change the result between refresh calls.
struct MutableRefreshProvider {
    id: &'static str,
    builtin_models: Vec<ModelInfo>,
    refresh_result: Arc<std::sync::Mutex<Option<Vec<ModelInfo>>>>,
}

impl MutableRefreshProvider {
    fn new(
        id: &'static str,
        builtin_models: Vec<ModelInfo>,
        initial_refresh: Vec<ModelInfo>,
    ) -> Self {
        Self {
            id,
            builtin_models,
            refresh_result: Arc::new(std::sync::Mutex::new(Some(initial_refresh))),
        }
    }
}

impl Provider for MutableRefreshProvider {
    fn id(&self) -> &str {
        self.id
    }

    fn models(&self) -> &[ModelInfo] {
        // Leak is intentional and bounded — test-only.
        Box::leak(self.builtin_models.clone().into_boxed_slice())
    }

    fn stream(&self, _request: Request) -> opi_ai::provider::EventStream {
        Box::pin(futures_util::stream::empty())
    }

    fn refresh_models(
        &self,
    ) -> opi_ai::credential::BoxAuthFuture<
        '_,
        Result<Option<Vec<ModelInfo>>, opi_ai::provider::ProviderError>,
    > {
        let result = self.refresh_result.lock().unwrap().clone();
        Box::pin(async move { Ok(result) })
    }
}

#[tokio::test]
async fn refresh_models_is_atomic_substrate() {
    // Mixed static + dynamic providers.
    let mut collection = ProviderCollection::new();

    // Static provider — default refresh_models returns Ok(None).
    let static_prov = text_mock("staticprov", "static");
    collection
        .register(
            static_prov,
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("key"),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    // Dynamic provider A — refresh returns a new catalog.
    let dyn_a =
        RefreshProvider::new("dyna", vec![model_info("old-a", "Old A")]).with_refresh(vec![
            model_info("fresh-a1", "Fresh A1"),
            model_info("fresh-a2", "Fresh A2"),
        ]);
    collection
        .register(
            Box::new(dyn_a),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("key"),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    // Dynamic provider B — refresh returns a different catalog.
    let dyn_b = RefreshProvider::new("dynb", vec![model_info("old-b", "Old B")])
        .with_refresh(vec![model_info("fresh-b", "Fresh B")]);
    collection
        .register(
            Box::new(dyn_b),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("key"),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    // Before refresh: built-in models are visible.
    assert!(collection.resolve("dyna:old-a").is_ok());
    assert!(collection.resolve("dynb:old-b").is_ok());
    // Fresh models not yet visible.
    assert!(collection.resolve("dyna:fresh-a1").is_err());

    // Refresh succeeds.
    collection.refresh().await.unwrap();

    // After refresh: dynamic catalogs replace built-in models.
    // Old models gone, fresh models visible.
    assert!(
        collection.resolve("dyna:old-a").is_err(),
        "old built-in model should be replaced by dynamic catalog"
    );
    let (prov_a1, model_a1) = collection.resolve("dyna:fresh-a1").unwrap();
    assert_eq!(prov_a1.id(), "dyna");
    assert_eq!(model_a1.id, "fresh-a1");
    assert!(collection.resolve("dyna:fresh-a2").is_ok());

    let (prov_b, model_b) = collection.resolve("dynb:fresh-b").unwrap();
    assert_eq!(prov_b.id(), "dynb");
    assert_eq!(model_b.id, "fresh-b");

    // Static provider is unaffected.
    assert!(collection.resolve("staticprov:mock-model").is_ok());

    // all_models includes dynamic catalogs.
    let all: Vec<(&str, &str)> = collection
        .registry()
        .all_models()
        .into_iter()
        .map(|(pid, m)| (pid, m.id.as_str()))
        .collect();
    // Dynamic provider A: old-a gone, fresh-a1 + fresh-a2 present.
    assert!(!all.contains(&("dyna", "old-a")));
    assert!(all.contains(&("dyna", "fresh-a1")));
    assert!(all.contains(&("dyna", "fresh-a2")));
    // Dynamic provider B: old-b gone, fresh-b present.
    assert!(!all.contains(&("dynb", "old-b")));
    assert!(all.contains(&("dynb", "fresh-b")));
}

#[tokio::test]
async fn refresh_models_atomic_rollback_on_error() {
    let mut collection = ProviderCollection::new();

    // Dynamic provider that succeeds on refresh.
    let good =
        RefreshProvider::new("good", vec![]).with_refresh(vec![model_info("good-model", "Good")]);
    collection
        .register(
            Box::new(good),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("key"),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    // Dynamic provider that fails on refresh.
    collection
        .register(
            Box::new(ErrorRefreshProvider {
                id: "bad",
                error_msg: "failed to refresh models",
            }),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("key"),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    // Verify initial state: no models yet (empty built-in).
    assert!(collection.resolve("good:good-model").is_err());

    // Refresh fails because "bad" returns an error.
    let err = collection.refresh().await.unwrap_err();
    assert!(
        err.to_string().contains("failed to refresh models"),
        "expected error message, got: {err}"
    );

    // Atomic rollback: good's catalog must NOT be installed.
    assert!(
        collection.resolve("good:good-model").is_err(),
        "good's catalog should NOT be visible after rollback"
    );
}

#[tokio::test]
async fn refresh_models_deterministic_ordering() {
    // Refresh evaluates providers in deterministic (sorted) id order.
    // Test: the result is the same regardless of registration order.
    let mut collection = ProviderCollection::new();

    // Register in non-alphabetical order.
    for id in ["zulu", "alpha", "mike"] {
        let prov = RefreshProvider::new(id, vec![])
            .with_refresh(vec![model_info(&format!("{id}-refreshed"), "refreshed")]);
        collection
            .register(
                Box::new(prov),
                AuthDescriptor::StaticApiKey {
                    value: SecretKey::new("key"),
                },
                CompatMetadata::default(),
            )
            .unwrap();
    }

    collection.refresh().await.unwrap();

    // All refreshed models are present.
    assert!(collection.resolve("alpha:alpha-refreshed").is_ok());
    assert!(collection.resolve("mike:mike-refreshed").is_ok());
    assert!(collection.resolve("zulu:zulu-refreshed").is_ok());
}

#[tokio::test]
async fn refresh_models_repeated_refresh_replaces() {
    let mut collection = ProviderCollection::new();

    let prov = MutableRefreshProvider::new("dyn", vec![], vec![model_info("v1", "Version 1")]);
    let refresh_result = Arc::clone(&prov.refresh_result);
    collection
        .register(
            Box::new(prov),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("key"),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    // First refresh.
    collection.refresh().await.unwrap();
    assert!(collection.resolve("dyn:v1").is_ok());
    assert!(collection.resolve("dyn:v2").is_err());

    // Change the refresh result for the next call.
    *refresh_result.lock().unwrap() = Some(vec![
        model_info("v2", "Version 2"),
        model_info("v3", "Version 3"),
    ]);

    // Second refresh — replaces v1 with v2+v3.
    collection.refresh().await.unwrap();
    assert!(
        collection.resolve("dyn:v1").is_err(),
        "v1 should be replaced"
    );
    assert!(collection.resolve("dyn:v2").is_ok());
    assert!(collection.resolve("dyn:v3").is_ok());
}

#[tokio::test]
async fn refresh_models_empty_catalog_is_valid() {
    let mut collection = ProviderCollection::new();

    let dyn_prov = RefreshProvider::new("dyn", vec![model_info("old", "Old")]).with_refresh(vec![]); // Empty catalog.
    collection
        .register(
            Box::new(dyn_prov),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("key"),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    assert!(collection.resolve("dyn:old").is_ok());

    collection.refresh().await.unwrap();

    // Empty dynamic catalog: old model gone, no new models.
    assert!(collection.resolve("dyn:old").is_err());
    let dyn_models: Vec<_> = collection
        .registry()
        .all_models()
        .into_iter()
        .filter(|(pid, _)| *pid == "dyn")
        .collect();
    assert!(
        dyn_models.is_empty(),
        "empty dynamic catalog should yield no models: got {dyn_models:?}"
    );
}
