//! Behavioral tests for the provider collection/auth seam (task 10.1).
//!
//! DoD: opi-ai exposes a provider collection/auth contract that owns provider
//! and model lookup, static API-key and env-auth descriptors, optional refresh
//! capability, OpenAI-compatible compatibility metadata, stream dispatch, an
//! explicit complete-dispatch decision compatible with the current streaming
//! Provider trait, redacted missing/invalid auth diagnostics, and a registry
//! regression asserting all built-in providers still resolve.

#[path = "support/catalog_store.rs"]
mod catalog_fixture;

use opi_ai::auth::{
    AuthProvenanceSource, AuthResolver, AuthScheme, ResolvedAuth, StaticAuthResolver,
};
use opi_ai::message::{AssistantContent, AssistantMessage};
use opi_ai::models::{
    AuthCheckKind, AuthDescriptor, CompatMetadata, Models, ModelsError, RefreshOutcome, SecretKey,
    drain_to_completion,
};
use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request, ThinkingConfig};
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

/// Concatenate all text content carried by an assistant message.
fn assistant_text(message: &AssistantMessage) -> String {
    message
        .content
        .iter()
        .filter_map(|content| match content {
            AssistantContent::Text { text, .. } => Some(text.as_str()),
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

fn collection_error_kind(error: &ModelsError) -> &'static str {
    match error {
        ModelsError::Registry(_) => "registry",
        ModelsError::RouteNotDispatchable { .. } => "route-not-dispatchable",
        ModelsError::RequestRouteMismatch { .. } => "request-route-mismatch",
        ModelsError::AttemptAlreadyActive => "attempt-already-active",
        ModelsError::CallCancelled => "call-cancelled",
        ModelsError::CredentialTerminated { .. } => "credential-terminated",
        ModelsError::RetryNotReady { .. } => "retry-not-ready",
        ModelsError::CallSealed { .. } => "call-sealed",
        ModelsError::Provider(_) => "provider",
        ModelsError::AuthContext(_) => "auth-context",
        ModelsError::CredentialStore(_) => "credential-store",
        ModelsError::AuthResolution { .. } => "auth-resolution",
        // The enum is non_exhaustive for downstream evolution; this local
        // exhaustive audit intentionally fails to compile on unseen variants.
        _unreachable_variant => unreachable!(),
    }
}

#[test]
fn collection_error_match_is_exhaustive_for_current_public_surface() {
    let error = ModelsError::RouteNotDispatchable {
        provider: "metadata-only".to_owned(),
    };
    assert_eq!(collection_error_kind(&error), "route-not-dispatchable");
}

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

    fn stream_prepared(
        &self,
        _request: Request,
        _auth: ResolvedAuth,
    ) -> opi_ai::provider::EventStream {
        let events = self.events.lock().unwrap().take().unwrap_or_default();
        Box::pin(futures_util::stream::iter(events))
    }
}

/// `AuthResolver` that always fails resolution with `CredentialNeeded`, proving
/// dispatch rejects before the provider is touched when auth is unavailable.
struct MissingAuthResolver {
    provider_id: String,
}

impl AuthResolver for MissingAuthResolver {
    fn resolve<'a>(
        &'a self,
    ) -> opi_ai::credential::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        let provider_id = self.provider_id.clone();
        Box::pin(async move { Err(ProviderError::CredentialNeeded { provider_id }) })
    }
}

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

// ---------------------------------------------------------------------------
// Task 20.2: owned producer release through the ChatStream seam
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dropping_chat_stream_without_draining_releases_the_attempt_slot() {
    // An undrained ChatStream must release the prepared call's single active
    // attempt slot (owned producer release): a sequential retry can start
    // immediately after the handle is dropped.
    let mut collection = Models::new();
    collection
        .register_route(
            text_mock_repeated("release", "ok", 2),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                secret("configured"),
            )) as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();

    let prepared = collection
        .prepare_call("release:mock-model", minimal_request("release:mock-model"))
        .await
        .unwrap();
    {
        let attempt = prepared.start_attempt().expect("first attempt starts");
        let _stream = opi_ai::stream::ChatStream::new(attempt);
        // Dropped without polling or draining.
    }
    let second = prepared.start_attempt();
    assert!(
        second.is_ok(),
        "dropping the ChatStream must release the attempt slot"
    );

    // Draining the second attempt through the same seam still completes.
    let attempt = second.unwrap();
    let message = drain_to_completion(attempt)
        .await
        .expect("second attempt completes");
    assert_eq!(assistant_text(&message), "ok");
}

// ---------------------------------------------------------------------------
// Acceptance scenario: provider_collection_dispatches_with_redacted_auth
// ---------------------------------------------------------------------------

#[tokio::test]
async fn provider_collection_dispatches_with_redacted_auth() {
    let mut collection = Models::new();
    collection
        .register_route(
            text_mock_repeated("mock", "hello from mock", 2),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                secret(SECRET_VALUE),
            )) as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();

    // Stream dispatch flows through prepare_call and reaches Done.
    let prepared = collection
        .prepare_call("mock:mock-model", minimal_request("mock:mock-model"))
        .await
        .expect("prepare_call resolves the route");
    // The prepared call's debug never leaks the resolved secret.
    assert!(
        !format!("{prepared:?}").contains(SECRET_VALUE),
        "secret leaked via prepared call debug"
    );
    let stream = prepared.start_attempt().expect("start_attempt");
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

    // Complete-dispatch decision: drain a fresh attempt to the final Result.
    let prepared = collection
        .prepare_call("mock:mock-model", minimal_request("mock:mock-model"))
        .await
        .unwrap();
    let attempt = prepared.start_attempt().unwrap();
    let message = drain_to_completion(attempt)
        .await
        .expect("Done terminal completes Ok");
    assert_eq!(assistant_text(&message), "hello from mock");
}

#[tokio::test]
async fn enabled_none_thinking_on_non_thinking_model_is_rejected_before_auth() {
    let mut collection = Models::new();
    collection
        .register_route(
            text_mock("no-thinking", "must not stream"),
            Arc::new(MissingAuthResolver {
                provider_id: "no-thinking".to_owned(),
            }) as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();
    let mut request = minimal_request("no-thinking:mock-model");
    request.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: Some(2048),
        level: opi_ai::ThinkingLevel::None,
    };

    let error = collection
        .prepare_call("no-thinking:mock-model", request)
        .await
        .expect_err("unsupported thinking must fail before auth resolution");

    assert!(matches!(
        error,
        ModelsError::Provider(ProviderError::UnsupportedCapability(_))
    ));
}

#[tokio::test]
async fn provider_collection_dispatch_rejects_missing_auth_with_redacted_diagnostic() {
    let mut collection = Models::new();
    collection
        .register_route(
            text_mock("noauth", "should not stream"),
            Arc::new(MissingAuthResolver {
                provider_id: "noauth".to_owned(),
            }) as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();

    // Missing auth terminates the call before the provider is touched.
    let err = match collection
        .prepare_call("noauth:mock-model", minimal_request("noauth:mock-model"))
        .await
    {
        Err(error) => error,
        Ok(_) => panic!("expected credential failure, got a prepared call"),
    };
    assert!(
        matches!(
            err,
            ModelsError::Provider(ProviderError::CredentialNeeded { .. })
        ),
        "got {err:?}"
    );
    // Diagnostic is redacted: it never carries the secret value.
    assert!(!format!("{err}").contains(SECRET_VALUE));
}

#[tokio::test]
async fn dispatch_complete_returns_terminal_error_event() {
    // A legacy in-band Error terminal (mock/historical producer shape) maps
    // to Err(CallFailure) with the delivered partial preserved.
    let message = AssistantMessage {
        content: vec![AssistantContent::Text {
            text: "terminal failure".into(),
            signature: None,
        }],
        api: opi_ai::ApiKind::OpenAi,
        provider: "mock".into(),
        model: "mock-model".into(),
        response_model: None,
        wire: None,
        response_id: None,
        usage: opi_ai::stream::Usage::default(),
        stop_reason: opi_ai::stream::StopReason::Error,
        error_message: Some("terminal failure".into()),
        timestamp_ms: 0,
        diagnostics: Vec::new(),
        duration_ms: None,
        raw_stop_reason: None,
        provider_thinking_level: None,

        deferred: None,
    };
    let mut collection = Models::new();
    collection
        .register_route(
            Box::new(StreamProvider::new(
                "mock",
                vec![Ok(opi_ai::AssistantStreamEvent::Error {
                    reason: opi_ai::stream::StopReason::Error,
                    message,
                })],
            )),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                secret("configured"),
            )) as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();

    let prepared = collection
        .prepare_call("mock:mock-model", minimal_request("mock:mock-model"))
        .await
        .unwrap();
    let stream = prepared.start_attempt().unwrap();
    let failure = drain_to_completion(stream)
        .await
        .expect_err("in-band error terminal drains to Err(CallFailure)");
    assert_eq!(assistant_text(failure.message()), "terminal failure");
    assert_eq!(
        failure.message().stop_reason,
        opi_ai::stream::StopReason::Error
    );
    assert!(matches!(
        failure.cause(),
        opi_ai::provider::ProviderError::StreamError(_)
    ));
}

#[tokio::test]
async fn dispatch_complete_propagates_mid_stream_provider_error() {
    let mut collection = Models::new();
    collection
        .register_route(
            Box::new(StreamProvider::new(
                "mock",
                vec![Err(opi_ai::provider::ProviderError::StreamError(
                    opi_ai::provider::ProviderErrorSummary::redacted(),
                ))],
            )),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                secret("configured"),
            )) as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();

    let prepared = collection
        .prepare_call("mock:mock-model", minimal_request("mock:mock-model"))
        .await
        .unwrap();
    let stream = prepared.start_attempt().unwrap();
    let failure = drain_to_completion(stream)
        .await
        .expect_err("mid-stream provider error drains to Err(CallFailure)");
    assert!(matches!(failure.cause(), ProviderError::StreamError(_)));
    assert_eq!(failure.cause().to_string(), "stream error: [REDACTED]");
}

#[tokio::test]
async fn dispatch_complete_rejects_empty_stream() {
    let mut collection = Models::new();
    collection
        .register_route(
            Box::new(StreamProvider::new("mock", Vec::new())),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                secret("configured"),
            )) as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();

    let prepared = collection
        .prepare_call("mock:mock-model", minimal_request("mock:mock-model"))
        .await
        .unwrap();
    let stream = prepared.start_attempt().unwrap();
    let failure = drain_to_completion(stream)
        .await
        .expect_err("empty stream must fail with the typed protocol failure");
    assert!(
        failure
            .cause()
            .to_string()
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

    let mut collection = Models::new();
    collection
        .register_route(
            profile_provider,
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                secret(SECRET_VALUE),
            )) as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
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

    // The prepared call's debug never leaks the resolved secret.
    let prepared = collection
        .prepare_call(
            "openrouter-profile:profile-model",
            minimal_request("openrouter-profile:profile-model"),
        )
        .await
        .unwrap();
    assert!(
        !format!("{prepared:?}").contains(SECRET_VALUE),
        "secret leaked via prepared call debug"
    );

    // Stream dispatch works without the CLI product harness.
    let stream = prepared.start_attempt().unwrap();
    let message = drain_to_completion(stream)
        .await
        .expect("Done terminal completes Ok");
    assert_eq!(assistant_text(&message), "profile response");
}

// ---------------------------------------------------------------------------
// Empty-collection refresh
// ---------------------------------------------------------------------------

#[tokio::test]
async fn collection_refresh_is_a_documented_noop_extension_point() {
    let mut collection = Models::new();
    // Offline refresh on a store-less collection performs no network work and
    // reports an empty outcome (no pi.dev/default startup policy exists).
    let report = collection.refresh(false).await;
    assert!(report.is_ok());
}

// ---------------------------------------------------------------------------
// from_registry wrapping
// ---------------------------------------------------------------------------

#[test]
fn collection_wraps_existing_registry_via_from_registry() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(text_mock("wrapped", "wrapped response"))
        .unwrap();

    let collection = Models::from_registry(registry);
    // Underlying registry is accessible for list-models / overrides.
    assert_eq!(collection.registry().provider_ids(), vec!["wrapped"]);
    // Model lookup flows through the wrapped registry.
    let (provider, _) = collection.resolve("wrapped:mock-model").unwrap();
    assert_eq!(provider.id(), "wrapped");
    // Auth descriptor defaults to absent for pre-registered providers, and the
    // on-demand check reports them unconfigured.
    assert!(collection.auth_descriptor("wrapped").is_none());
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
    // Without a live store or probe snapshot the store-backed provider is
    // simply unconfigured: the secret-free descriptor never fabricates a
    // Configured answer.
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
            err,
            ModelsError::Provider(ProviderError::CredentialNeeded { .. })
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

    fn stream_prepared(
        &self,
        _request: Request,
        _auth: ResolvedAuth,
    ) -> opi_ai::provider::EventStream {
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

    fn stream_prepared(
        &self,
        _request: Request,
        _auth: ResolvedAuth,
    ) -> opi_ai::provider::EventStream {
        Box::pin(futures_util::stream::empty())
    }

    fn refresh_models(
        &self,
    ) -> opi_ai::credential::BoxAuthFuture<
        '_,
        Result<Option<Vec<ModelInfo>>, opi_ai::provider::ProviderError>,
    > {
        let msg = self.error_msg;
        Box::pin(async move {
            Err(opi_ai::provider::ProviderError::ProviderSide(
                opi_ai::provider::ProviderErrorSummary::from_untrusted(msg),
            ))
        })
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

    fn stream_prepared(
        &self,
        _request: Request,
        _auth: ResolvedAuth,
    ) -> opi_ai::provider::EventStream {
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
    let mut collection = Models::new();

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
    collection.refresh(true).await.unwrap();

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
async fn refresh_models_reports_per_provider_and_publishes_successes() {
    const STATIC_PROVIDER_CANARY: &str = "static-refresh-provider-canary";
    let mut collection = Models::new();

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
                error_msg: STATIC_PROVIDER_CANARY,
            }),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("key"),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    // Verify initial state: no models yet (empty built-in).
    assert!(collection.resolve("good:good-model").is_err());

    // Refresh reports per-provider outcomes: a failing provider retains its
    // prior data and does not erase other providers' fresh catalogs.
    let report = collection.refresh(true).await.unwrap();
    let good_entry = report
        .find("good")
        .expect("good provider appears in the refresh report");
    assert_eq!(good_entry.outcome, RefreshOutcome::Published { models: 1 });
    let bad_entry = report
        .find("bad")
        .expect("bad provider appears in the refresh report");
    let RefreshOutcome::Failed(bad_error) = &bad_entry.outcome else {
        panic!(
            "bad provider must report its error, got {:?}",
            bad_entry.outcome
        )
    };
    assert_eq!(bad_error.to_string(), "provider error: [REDACTED]");
    assert!(!format!("{bad_error:?}").contains(STATIC_PROVIDER_CANARY));

    // Per-provider isolation: good's fresh catalog IS published despite bad.
    assert!(
        collection.resolve("good:good-model").is_ok(),
        "good's fresh catalog must publish even when bad fails"
    );
}

#[tokio::test]
async fn refresh_models_deterministic_ordering() {
    // Refresh evaluates providers in deterministic (sorted) id order.
    // Test: the result is the same regardless of registration order.
    let mut collection = Models::new();

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

    collection.refresh(true).await.unwrap();

    // Provider ids are exposed in deterministic sorted order regardless of
    // registration order, so a regression to insertion order would be caught.
    assert_eq!(
        collection.provider_ids(),
        vec!["alpha", "mike", "zulu"],
        "provider ids must be sorted regardless of registration order"
    );

    // All refreshed models are present.
    assert!(collection.resolve("alpha:alpha-refreshed").is_ok());
    assert!(collection.resolve("mike:mike-refreshed").is_ok());
    assert!(collection.resolve("zulu:zulu-refreshed").is_ok());
}

#[tokio::test]
async fn refresh_models_repeated_refresh_replaces() {
    let mut collection = Models::new();

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
    collection.refresh(true).await.unwrap();
    assert!(collection.resolve("dyn:v1").is_ok());
    assert!(collection.resolve("dyn:v2").is_err());

    // Change the refresh result for the next call.
    *refresh_result.lock().unwrap() = Some(vec![
        model_info("v2", "Version 2"),
        model_info("v3", "Version 3"),
    ]);

    // Second refresh — replaces v1 with v2+v3.
    collection.refresh(true).await.unwrap();
    assert!(
        collection.resolve("dyn:v1").is_err(),
        "v1 should be replaced"
    );
    assert!(collection.resolve("dyn:v2").is_ok());
    assert!(collection.resolve("dyn:v3").is_ok());
}

#[tokio::test]
async fn refresh_models_none_clears_prior_dynamic_snapshot() {
    let mut collection = Models::new();

    let provider = MutableRefreshProvider::new(
        "dyn",
        vec![model_info("builtin", "Built In")],
        vec![model_info("dynamic", "Dynamic")],
    );
    let refresh_result = Arc::clone(&provider.refresh_result);
    collection
        .register(
            Box::new(provider),
            AuthDescriptor::StaticApiKey {
                value: SecretKey::new("key"),
            },
            CompatMetadata::default(),
        )
        .unwrap();

    collection.refresh(true).await.unwrap();
    assert!(collection.resolve("dyn:dynamic").is_ok());
    assert!(collection.resolve("dyn:builtin").is_err());

    *refresh_result.lock().unwrap() = None;
    collection.refresh(true).await.unwrap();

    assert!(collection.resolve("dyn:dynamic").is_err());
    assert!(collection.resolve("dyn:builtin").is_ok());
}

#[tokio::test]
async fn refresh_models_empty_catalog_is_valid() {
    let mut collection = Models::new();

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

    collection.refresh(true).await.unwrap();

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

// ===========================================================================
// Phase 17 — collection-owned route + auth preparation (task 17.1 substrate).
// ===========================================================================

mod phase17 {
    use super::*;
    use futures_util::StreamExt;
    use opi_ai::auth::{
        AuthFallback, AuthProvenance, AuthProvenanceSource, AuthResolver, AuthScheme, ResolvedAuth,
    };
    use opi_ai::azure_openai::AzureOpenAIProvider;
    use opi_ai::credential::BoxAuthFuture;
    use opi_ai::models::CompatMetadata;
    use opi_ai::provider::{EventStream, Provider, ProviderError, Request};
    use opi_ai::test_support::{MockResponse, text_response};
    use secrecy::SecretString;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    /// A probe provider that records prepared-seam (`stream_prepared`) hits,
    /// emitting canned events regardless of the supplied auth (no HTTP). The
    /// The legacy direct stream entry is gone, so the `legacy` counter is vestigial:
    /// retained for call-site stability (marked `dead_code`) and never read.
    struct ProbeProvider {
        id: String,
        models: Vec<ModelInfo>,
        prepared: Arc<AtomicUsize>,
        #[allow(dead_code)]
        legacy: Arc<AtomicUsize>,
        /// When set, the first dispatch yields this retryable error instead
        /// of a successful response, so tests can drive a real
        /// fail-then-admitted-retry episode through the acquisition owner.
        first_error: Option<ProviderError>,
    }

    impl ProbeProvider {
        fn new(
            id: &str,
            model: &str,
            prepared: Arc<AtomicUsize>,
            legacy: Arc<AtomicUsize>,
        ) -> Self {
            Self {
                id: id.to_owned(),
                models: vec![ModelInfo::new(
                    model,
                    model,
                    WireApi::OpenAiCompletions,
                    ModelCapabilities::new(100_000, 4_096),
                )],
                prepared,
                legacy,
                first_error: None,
            }
        }

        fn with_first_error(mut self, error: ProviderError) -> Self {
            self.first_error = Some(error);
            self
        }
    }

    impl Provider for ProbeProvider {
        fn id(&self) -> &str {
            &self.id
        }
        fn models(&self) -> &[ModelInfo] {
            &self.models
        }
        fn stream_prepared(&self, _request: Request, _auth: ResolvedAuth) -> EventStream {
            let dispatch = self.prepared.fetch_add(1, Ordering::SeqCst);
            if dispatch == 0
                && let Some(error) = &self.first_error
            {
                return Box::pin(futures_util::stream::iter(vec![Err(error.clone())]));
            }
            let events = text_response("prepared");
            Box::pin(futures_util::stream::iter(
                events.into_iter().map(Ok::<_, ProviderError>),
            ))
        }
    }

    struct CancelAwareProvider {
        models: Vec<ModelInfo>,
        dispatches: Arc<AtomicUsize>,
        cancellations: Arc<AtomicUsize>,
    }

    struct CountingAzureProvider {
        inner: AzureOpenAIProvider,
        dispatches: Arc<AtomicUsize>,
    }

    impl Provider for CountingAzureProvider {
        fn id(&self) -> &str {
            self.inner.id()
        }

        fn models(&self) -> &[ModelInfo] {
            self.inner.models()
        }

        fn stream_prepared(&self, request: Request, auth: ResolvedAuth) -> EventStream {
            self.dispatches.fetch_add(1, Ordering::SeqCst);
            self.inner.stream_prepared(request, auth)
        }
    }

    impl Provider for CancelAwareProvider {
        fn id(&self) -> &str {
            "cancel-aware"
        }

        fn models(&self) -> &[ModelInfo] {
            &self.models
        }

        fn stream_prepared(&self, request: Request, _auth: ResolvedAuth) -> EventStream {
            self.dispatches.fetch_add(1, Ordering::SeqCst);
            let cancellations = self.cancellations.clone();
            Box::pin(futures_util::stream::once(async move {
                request.cancel.cancelled().await;
                cancellations.fetch_add(1, Ordering::SeqCst);
                Err(ProviderError::Cancelled)
            }))
        }
    }

    /// A resolver that counts how many times it is consulted.
    struct CountingResolver {
        count: Arc<AtomicUsize>,
        secret: SecretString,
    }

    impl AuthResolver for CountingResolver {
        fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
            let count = self.count.clone();
            let secret = self.secret.clone();
            Box::pin(async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok(ResolvedAuth {
                    env: Default::default(),
                    scheme: AuthScheme::ApiKey,
                    secret,
                    base_url: None,
                    account_id: None,
                    provenance: AuthProvenance::default(),
                })
            })
        }
    }

    struct PendingResolver {
        count: Arc<AtomicUsize>,
        entered: Arc<tokio::sync::Notify>,
    }

    impl AuthResolver for PendingResolver {
        fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
            let count = self.count.clone();
            let entered = self.entered.clone();
            Box::pin(async move {
                count.fetch_add(1, Ordering::SeqCst);
                entered.notify_one();
                std::future::pending().await
            })
        }
    }

    async fn wait_for_resolver_readiness(
        readiness: &tokio::sync::Notify,
        timeout: Duration,
    ) -> Result<(), String> {
        tokio::time::timeout(timeout, readiness.notified())
            .await
            .map_err(|_| "timed out waiting for auth resolver readiness".to_owned())
    }

    #[tokio::test]
    async fn bounded_resolver_readiness_guard_reports_timeout_instead_of_hanging() {
        let readiness = tokio::sync::Notify::new();
        let error = wait_for_resolver_readiness(&readiness, Duration::from_millis(10))
            .await
            .expect_err("an unentered resolver must time out");
        assert!(
            error.contains("auth resolver readiness"),
            "timeout identifies the stalled resolver boundary: {error}"
        );
    }

    struct ProvenanceResolver;

    impl AuthResolver for ProvenanceResolver {
        fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
            Box::pin(async move {
                let primary = AuthProvenanceSource::CredentialStore {
                    kind: "native-keyring".to_owned(),
                };
                let fallback = AuthProvenanceSource::Environment {
                    name: "ALPHA_API_KEY".to_owned(),
                };
                Ok(ResolvedAuth {
                    env: Default::default(),
                    scheme: AuthScheme::ApiKey,
                    secret: SecretString::from("secret"),
                    base_url: None,
                    account_id: None,
                    provenance: AuthProvenance {
                        source: fallback.clone(),
                        fallback: AuthFallback::Used {
                            from: primary,
                            to: fallback,
                            reason: "credential store unavailable".to_owned(),
                        },
                    },
                })
            })
        }
    }

    #[tokio::test]
    async fn prepare_call_resolves_output_limit_per_model_and_freezes_attempts() {
        let model = |id: &str, limit| {
            ModelInfo::new(
                id,
                id,
                WireApi::OpenAiCompletions,
                ModelCapabilities::new(200_000, limit),
            )
        };
        let provider = MockProvider::new_with_models_and_errors(
            "budget",
            vec![
                model("wide", 131_072),
                model("narrow", 8_192),
                model("overridden", 4_096),
            ],
            // Four fail-then-retry episodes: each spec's first send fails
            // retryably with a zero-delay hint and the admitted retry
            // succeeds.
            (0..4)
                .flat_map(|_| {
                    vec![
                        MockResponse::Error(ProviderError::RateLimited {
                            retry_after_ms: Some(0),
                        }),
                        MockResponse::Events(text_response("done")),
                    ]
                })
                .collect(),
        );
        let calls = provider.call_log_handle();
        let resolve_hits = Arc::new(AtomicUsize::new(0));
        let mut registry = ProviderRegistry::new();
        registry
            .register_model("budget", model("overridden", 65_536))
            .unwrap();
        let mut collection = Models::from_registry(registry);
        collection
            .register_route(
                Box::new(provider),
                Arc::new(CountingResolver {
                    count: resolve_hits.clone(),
                    secret: SecretString::from(SECRET_VALUE),
                }),
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .unwrap();

        for (id, explicit_limit, expected_limit) in [
            ("wide", None, 131_072),
            ("narrow", None, 8_192),
            ("overridden", None, 65_536),
            ("wide", Some(4_096), 4_096),
        ] {
            let spec = format!("budget:{id}");
            let mut request = minimal_request(&spec);
            request.max_tokens = explicit_limit;
            // Each episode is a real acquisition retry: the first send fails
            // retryably (zero-delay hint) and the admitted retry reuses the
            // same frozen compiled request, proving the frozen limit and
            // model across attempts.
            let prepared = collection
                .prepare_call_with_policy(
                    &spec,
                    request,
                    opi_ai::retry::AcquisitionPolicy::with_max_retries(1),
                )
                .await
                .unwrap();
            let mut first = prepared.start_attempt().unwrap();
            while let Some(item) = first.next().await {
                if let Err(error) = item {
                    assert!(
                        matches!(error, ProviderError::RateLimited { .. }),
                        "expected the scripted rate-limit failure, got {error:?}"
                    );
                    break;
                }
            }
            assert!(
                prepared.retry_admission().is_some(),
                "the scripted failure with budget must admit the retry"
            );
            let stream = prepared.start_attempt().unwrap();
            drain_to_completion(stream).await.unwrap();
            let calls = calls.lock().unwrap();
            let used = calls.len();
            for call in &calls[used - 2..] {
                assert_eq!(call.model, spec);
                assert_eq!(call.max_tokens, Some(expected_limit));
            }
        }
        assert_eq!(resolve_hits.load(Ordering::SeqCst), 4);
        assert_eq!(calls.lock().unwrap().len(), 8);
    }

    #[tokio::test]
    async fn prepare_call_clamps_response_to_current_context_and_fits_thinking() {
        use opi_ai::message::{InputContent, Message, UserMessage};
        let provider = MockProvider::new_with_models(
            "budget",
            vec![ModelInfo::new(
                "wide",
                "Wide",
                WireApi::OpenAiCompletions,
                ModelCapabilities::new(32_768, 32_768).with_thinking(true),
            )],
            (0..4).map(|_| text_response("done")).collect(),
        );
        let calls = provider.call_log_handle();
        let mut collection = Models::new();
        collection
            .register_route(
                Box::new(provider),
                Arc::new(CountingResolver {
                    count: Arc::new(AtomicUsize::new(0)),
                    secret: SecretString::from("test"),
                }),
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .unwrap();
        for explicit in [None, Some(30_000), Some(2_000)] {
            let mut request = minimal_request("budget:wide");
            request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                system_prompt: Some("s".repeat(4_000)),
                messages: vec![Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "u".repeat(80_000),
                    }],
                    timestamp_ms: 1,
                })],
                tools: vec![],
            })
            .expect("valid context declarations");
            request.max_tokens = explicit;
            request.thinking = ThinkingConfig {
                enabled: true,
                budget_tokens: Some(8_192),
                level: opi_ai::ThinkingLevel::Medium,
            };
            let prepared = collection
                .prepare_call("budget:wide", request)
                .await
                .unwrap();
            drain_to_completion(prepared.start_attempt().unwrap())
                .await
                .unwrap();
        }
        let mut exhausted = minimal_request("budget:wide");
        exhausted.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "u".repeat(140_000),
                }],
                timestamp_ms: 1,
            })],
            tools: vec![],
        })
        .expect("valid context declarations");
        let prepared = collection
            .prepare_call("budget:wide", exhausted)
            .await
            .unwrap();
        drain_to_completion(prepared.start_attempt().unwrap())
            .await
            .unwrap();
        let calls = calls.lock().unwrap();
        assert_eq!(
            calls.iter().map(|c| c.max_tokens).collect::<Vec<_>>(),
            vec![Some(4_671), Some(4_671), Some(2_000), Some(1)]
        );
        assert_eq!(calls[0].thinking.budget_tokens, Some(3_647));
        assert_eq!(calls[1].thinking.budget_tokens, Some(3_647));
        assert_eq!(calls[2].thinking.budget_tokens, Some(976));
    }

    #[tokio::test]
    async fn prepare_call_anthropic_thinking_keeps_answer_room_and_rejects_tiny_context() {
        let mut adaptive = ModelInfo::new(
            "adaptive",
            "Adaptive",
            WireApi::AnthropicMessages,
            ModelCapabilities::new(32_768, 32_768).with_thinking(true),
        );
        if let opi_ai::WireCompat::AnthropicMessages(compat) = &mut adaptive.compat {
            compat.force_adaptive_thinking = true;
        }
        let provider = MockProvider::new_with_models(
            "budget",
            vec![
                ModelInfo::new(
                    "anthropic",
                    "Anthropic",
                    WireApi::AnthropicMessages,
                    ModelCapabilities::new(32_768, 32_768).with_thinking(true),
                ),
                adaptive,
            ],
            (0..6).map(|_| text_response("summary")).collect(),
        );
        let calls = provider.call_log_handle();
        let resolutions = Arc::new(AtomicUsize::new(0));
        let mut collection = Models::new();
        collection
            .register_route(
                Box::new(provider),
                Arc::new(CountingResolver {
                    count: resolutions.clone(),
                    secret: SecretString::from("test"),
                }),
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .unwrap();
        for budget in [Some(8_192), None] {
            let mut request = minimal_request("budget:anthropic");
            request.max_tokens = Some(9_000);
            request.thinking = ThinkingConfig {
                enabled: true,
                budget_tokens: budget,
                level: opi_ai::ThinkingLevel::Medium,
            };
            let prepared = collection
                .prepare_call("budget:anthropic", request)
                .await
                .unwrap();
            drain_to_completion(prepared.start_attempt().unwrap())
                .await
                .unwrap();
        }
        {
            let calls = calls.lock().unwrap();
            assert_eq!(calls[0].thinking.budget_tokens, Some(7_976));
            assert_eq!(calls[1].thinking.budget_tokens, Some(7_976));
        }
        // Task 20.39 replacement proof (annex row 165): the sub-1024 typed
        // preflight is withdrawn — clamped zero budgets flow to the wire
        // (where the Anthropic `|| 1024` default applies) and nonzero
        // sub-1024 budgets pass through.
        for (limit, clamped) in [(1, 0), (1_024, 0), (2_000, 976)] {
            let mut request = minimal_request("budget:anthropic");
            request.max_tokens = Some(limit);
            request.thinking = ThinkingConfig {
                enabled: true,
                budget_tokens: Some(8_192),
                level: opi_ai::ThinkingLevel::Medium,
            };
            let prepared = collection
                .prepare_call("budget:anthropic", request)
                .await
                .unwrap();
            drain_to_completion(prepared.start_attempt().unwrap())
                .await
                .unwrap();
            let calls = calls.lock().unwrap();
            let recorded = calls.last().expect("recorded dispatch");
            assert_eq!(recorded.thinking.budget_tokens, Some(clamped));
        }
        // Task 20.39: the withdrawn-preflight cases now dispatch too.
        assert_eq!(resolutions.load(Ordering::SeqCst), 5);
        assert_eq!(calls.lock().unwrap().len(), 5);
        let mut request = minimal_request("budget:adaptive");
        request.max_tokens = Some(1_024);
        request.thinking = ThinkingConfig {
            enabled: true,
            budget_tokens: None,
            level: opi_ai::ThinkingLevel::Medium,
        };
        let prepared = collection
            .prepare_call("budget:adaptive", request)
            .await
            .unwrap();
        drain_to_completion(prepared.start_attempt().unwrap())
            .await
            .unwrap();
        assert_eq!(
            calls.lock().unwrap().last().unwrap().thinking.budget_tokens,
            None,
            "adaptive thinking uses effort rather than a numeric minimum"
        );
    }

    #[tokio::test]
    async fn prepare_call_rejects_default_thinking_budget_before_auth_or_dispatch() {
        let provider = MockProvider::new_with_models(
            "budget",
            vec![
                ModelInfo::new(
                    "wide",
                    "Wide",
                    WireApi::OpenAiCompletions,
                    ModelCapabilities::new(200_000, 131_072).with_thinking(true),
                ),
                ModelInfo::new(
                    "narrow",
                    "Narrow",
                    WireApi::OpenAiCompletions,
                    ModelCapabilities::new(200_000, 8_192).with_thinking(true),
                ),
            ],
            vec![text_response("done")],
        );
        let calls = provider.call_log_handle();
        let resolve_hits = Arc::new(AtomicUsize::new(0));
        let mut collection = Models::new();
        collection
            .register_route(
                Box::new(provider),
                Arc::new(CountingResolver {
                    count: resolve_hits.clone(),
                    secret: SecretString::from(SECRET_VALUE),
                }),
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .unwrap();

        for (spec, budget) in [("budget:wide", 131_072), ("budget:narrow", 20_000)] {
            let mut request = minimal_request(spec);
            request.thinking = ThinkingConfig {
                enabled: true,
                budget_tokens: Some(budget),
                level: opi_ai::ThinkingLevel::Medium,
            };
            let error = match collection.prepare_call(spec, request).await {
                Ok(_) => panic!("an incompatible default thinking budget was prepared"),
                Err(error) => error,
            };
            assert!(matches!(
                error,
                ModelsError::Provider(ProviderError::UnsupportedCapability(_))
            ));
            assert!(!error.to_string().contains(SECRET_VALUE));
        }
        assert_eq!(resolve_hits.load(Ordering::SeqCst), 0);
        assert!(calls.lock().unwrap().is_empty());

        let mut request = minimal_request("budget:narrow");
        request.thinking = ThinkingConfig {
            enabled: true,
            budget_tokens: Some(7_168),
            level: opi_ai::ThinkingLevel::Medium,
        };
        let prepared = collection
            .prepare_call("budget:narrow", request)
            .await
            .unwrap();
        drain_to_completion(prepared.start_attempt().unwrap())
            .await
            .unwrap();
        assert_eq!(resolve_hits.load(Ordering::SeqCst), 1);
        let calls = calls.lock().unwrap();
        assert_eq!(calls[0].max_tokens, Some(8_192));
        assert_eq!(calls[0].thinking.budget_tokens, Some(7_168));
    }

    #[tokio::test]
    async fn prepare_call_resolves_route_and_auth_once_and_streams_via_prepared_seam() {
        let prepared_hits = Arc::new(AtomicUsize::new(0));
        let legacy_hits = Arc::new(AtomicUsize::new(0));
        let resolve_hits = Arc::new(AtomicUsize::new(0));

        let provider = ProbeProvider::new(
            "alpha",
            "model-a",
            prepared_hits.clone(),
            legacy_hits.clone(),
        );
        let resolver = Arc::new(CountingResolver {
            count: resolve_hits.clone(),
            secret: SecretString::from("sk-alpha-canary"),
        });

        let mut collection = Models::new();
        collection
            .register_route(
                Box::new(provider),
                resolver,
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .expect("register dispatchable route");

        let prepared = collection
            .prepare_call("alpha:model-a", minimal_request("alpha:model-a"))
            .await
            .expect("prepare_call resolves route + auth");

        // Redacted route facts identify the resolved route; the secret never
        // appears in the route's debug output.
        let route = prepared.route();
        assert_eq!(route.provider_id, "alpha");
        assert_eq!(route.model_id, "model-a");
        assert_eq!(
            prepared.auth_provenance().source,
            AuthProvenanceSource::Static
        );
        let route_dbg = format!("{route:?}");
        assert!(
            !route_dbg.contains("sk-alpha-canary"),
            "secret leaked into route facts: {route_dbg}"
        );

        // One attempt streams through the prepared seam; auth was resolved once
        // and the legacy stream() entry was not used.
        let mut stream = prepared.start_attempt().expect("start_attempt");
        let first = stream.next().await;
        assert!(first.is_some(), "prepared attempt produced no events");
        drop(stream);

        assert_eq!(
            resolve_hits.load(Ordering::SeqCst),
            1,
            "auth resolved more than once"
        );
        assert_eq!(
            prepared_hits.load(Ordering::SeqCst),
            1,
            "prepared seam not used"
        );
        assert_eq!(
            legacy_hits.load(Ordering::SeqCst),
            0,
            "legacy stream() used"
        );
    }

    #[tokio::test]
    async fn metadata_registration_replaces_and_removes_prior_dispatch_state() {
        let resolve_hits = Arc::new(AtomicUsize::new(0));
        let mut collection = Models::new();
        collection
            .register_route(
                Box::new(ProbeProvider::new(
                    "alpha",
                    "model-a",
                    Arc::new(AtomicUsize::new(0)),
                    Arc::new(AtomicUsize::new(0)),
                )),
                Arc::new(CountingResolver {
                    count: resolve_hits.clone(),
                    secret: SecretString::from("stale-secret"),
                }),
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .unwrap();
        collection
            .register(
                Box::new(ProbeProvider::new(
                    "alpha",
                    "model-a",
                    Arc::new(AtomicUsize::new(0)),
                    Arc::new(AtomicUsize::new(0)),
                )),
                AuthDescriptor::Resolved {
                    source: "metadata only".to_owned(),
                },
                CompatMetadata::default(),
            )
            .unwrap();

        let error = collection
            .prepare_call("alpha:model-a", minimal_request("alpha:model-a"))
            .await
            .expect_err("metadata-only replacement must not retain stale resolver");
        assert!(matches!(error, ModelsError::RouteNotDispatchable { .. }));
        assert_eq!(resolve_hits.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn dispatch_registration_replaces_and_removes_prior_metadata_state() {
        let mut collection = Models::new();
        collection
            .register(
                Box::new(ProbeProvider::new(
                    "alpha",
                    "model-a",
                    Arc::new(AtomicUsize::new(0)),
                    Arc::new(AtomicUsize::new(0)),
                )),
                AuthDescriptor::Resolved {
                    source: "old metadata".to_owned(),
                },
                CompatMetadata::default(),
            )
            .unwrap();
        collection.set_probe("alpha", opi_ai::credential::CredentialSource::Absent);
        collection
            .register_route(
                Box::new(ProbeProvider::new(
                    "alpha",
                    "model-a",
                    Arc::new(AtomicUsize::new(0)),
                    Arc::new(AtomicUsize::new(0)),
                )),
                Arc::new(CountingResolver {
                    count: Arc::new(AtomicUsize::new(0)),
                    secret: SecretString::from("new-secret"),
                }),
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .unwrap();

        assert!(collection.auth_descriptor("alpha").is_none());
        assert!(collection.probe("alpha").is_none());
    }

    #[tokio::test]
    async fn prepare_call_preserves_resolver_reported_source_and_fallback() {
        let mut collection = Models::new();
        collection
            .register_route(
                Box::new(ProbeProvider::new(
                    "alpha",
                    "model-a",
                    Arc::new(AtomicUsize::new(0)),
                    Arc::new(AtomicUsize::new(0)),
                )),
                Arc::new(ProvenanceResolver),
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .unwrap();

        let prepared = collection
            .prepare_call("alpha:model-a", minimal_request("alpha:model-a"))
            .await
            .unwrap();
        assert_eq!(
            prepared.auth_provenance(),
            &AuthProvenance {
                source: AuthProvenanceSource::Environment {
                    name: "ALPHA_API_KEY".to_owned(),
                },
                fallback: AuthFallback::Used {
                    from: AuthProvenanceSource::CredentialStore {
                        kind: "native-keyring".to_owned(),
                    },
                    to: AuthProvenanceSource::Environment {
                        name: "ALPHA_API_KEY".to_owned(),
                    },
                    reason: "credential store unavailable".to_owned(),
                },
            }
        );
    }

    /// Drain a stream until it yields a terminal event or an error.
    async fn drain_to_terminal(stream: &mut EventStream) {
        while let Some(item) = stream.next().await {
            match item {
                Ok(event) if event.is_terminal() => break,
                Err(_) => break,
                _ => {}
            }
        }
    }

    fn collection_with_probe(
        prepared: Arc<AtomicUsize>,
        legacy: Arc<AtomicUsize>,
        resolved: Arc<AtomicUsize>,
    ) -> Models {
        let mut collection = Models::new();
        collection
            .register_route(
                Box::new(ProbeProvider::new("alpha", "model-a", prepared, legacy)),
                Arc::new(CountingResolver {
                    count: resolved,
                    secret: SecretString::from("sk-alpha-canary"),
                }),
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .expect("register dispatchable route");
        collection
    }

    #[tokio::test]
    async fn start_attempt_rejects_a_second_active_attempt() {
        let prepared_hits = Arc::new(AtomicUsize::new(0));
        let legacy_hits = Arc::new(AtomicUsize::new(0));
        let resolve_hits = Arc::new(AtomicUsize::new(0));
        let collection = collection_with_probe(
            prepared_hits.clone(),
            legacy_hits.clone(),
            resolve_hits.clone(),
        );
        let prepared = collection
            .prepare_call("alpha:model-a", minimal_request("alpha:model-a"))
            .await
            .expect("prepare_call");

        // Held (not drained) so it remains the active attempt.
        let attempt = prepared.start_attempt().expect("first attempt");
        let err = match prepared.start_attempt() {
            Err(error) => error,
            Ok(_) => panic!("second concurrent attempt must be rejected"),
        };
        assert!(
            matches!(err, ModelsError::AttemptAlreadyActive),
            "got {err:?}"
        );
        drop(attempt);
    }

    #[tokio::test]
    async fn dropping_an_unpolled_attempt_releases_the_active_slot() {
        let collection = collection_with_probe(
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
        );
        let prepared = collection
            .prepare_call("alpha:model-a", minimal_request("alpha:model-a"))
            .await
            .unwrap();

        let attempt = prepared.start_attempt().unwrap();
        drop(attempt);
        let retry = prepared
            .start_attempt()
            .expect("dropping the attempt stream releases its lease");
        drop(retry);
    }

    #[tokio::test]
    async fn cancelling_during_auth_resolution_stops_preparation() {
        let resolve_hits = Arc::new(AtomicUsize::new(0));
        let resolver_entered = Arc::new(tokio::sync::Notify::new());
        let mut collection = Models::new();
        collection
            .register_route(
                Box::new(ProbeProvider::new(
                    "alpha",
                    "model-a",
                    Arc::new(AtomicUsize::new(0)),
                    Arc::new(AtomicUsize::new(0)),
                )),
                Arc::new(PendingResolver {
                    count: resolve_hits.clone(),
                    entered: resolver_entered.clone(),
                }),
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .unwrap();
        let collection = Arc::new(collection);
        let request = minimal_request("alpha:model-a");
        let cancel = request.cancel.clone();
        let task = tokio::spawn({
            let collection = collection.clone();
            async move { collection.prepare_call("alpha:model-a", request).await }
        });

        wait_for_resolver_readiness(&resolver_entered, Duration::from_secs(1))
            .await
            .expect("resolver must be entered before cancellation");
        assert_eq!(resolve_hits.load(Ordering::SeqCst), 1);
        cancel.cancel();
        let error = tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .expect("cancellation must interrupt pending auth resolution")
            .expect("prepare task panicked")
            .expect_err("cancelled preparation must fail");
        assert!(matches!(error, ModelsError::CallCancelled));
    }

    #[tokio::test]
    async fn admitted_retry_after_failure_resolves_auth_once_and_releases_the_slot() {
        let prepared_hits = Arc::new(AtomicUsize::new(0));
        let legacy_hits = Arc::new(AtomicUsize::new(0));
        let resolve_hits = Arc::new(AtomicUsize::new(0));
        let mut collection = Models::new();
        collection
            .register_route(
                Box::new(
                    ProbeProvider::new(
                        "alpha",
                        "model-a",
                        prepared_hits.clone(),
                        legacy_hits.clone(),
                    )
                    .with_first_error(ProviderError::RateLimited {
                        retry_after_ms: Some(0),
                    }),
                ),
                Arc::new(CountingResolver {
                    count: resolve_hits.clone(),
                    secret: SecretString::from("sk-alpha-canary"),
                }),
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .expect("register dispatchable route");
        let prepared = collection
            .prepare_call_with_policy(
                "alpha:model-a",
                minimal_request("alpha:model-a"),
                opi_ai::retry::AcquisitionPolicy::with_max_retries(1),
            )
            .await
            .expect("prepare_call");

        let mut first = prepared.start_attempt().expect("first attempt");
        drain_to_terminal(&mut first).await;
        let admission = prepared
            .retry_admission()
            .expect("retryable failure with budget admits the next attempt");
        assert_eq!(admission.retry_index, 1);
        let mut retry = prepared.start_attempt().expect("admitted retry dispatches");
        drain_to_terminal(&mut retry).await;

        assert_eq!(
            resolve_hits.load(Ordering::SeqCst),
            1,
            "auth resolved more than once across retries"
        );
        assert_eq!(
            prepared_hits.load(Ordering::SeqCst),
            2,
            "two prepared attempts: the failed one and the admitted retry"
        );
    }

    #[tokio::test]
    async fn cancelling_the_shared_token_forbids_any_attempt() {
        let prepared_hits = Arc::new(AtomicUsize::new(0));
        let legacy_hits = Arc::new(AtomicUsize::new(0));
        let resolve_hits = Arc::new(AtomicUsize::new(0));
        let collection = collection_with_probe(
            prepared_hits.clone(),
            legacy_hits.clone(),
            resolve_hits.clone(),
        );

        let request = minimal_request("alpha:model-a");
        let token = request.cancel.clone();
        let prepared = collection
            .prepare_call("alpha:model-a", request)
            .await
            .expect("prepare_call");

        token.cancel();
        let err = match prepared.start_attempt() {
            Err(error) => error,
            Ok(_) => panic!("cancelled call must reject attempts"),
        };
        assert!(matches!(err, ModelsError::CallCancelled), "got {err:?}");
        assert_eq!(
            prepared_hits.load(Ordering::SeqCst),
            0,
            "provider dispatched despite cancellation"
        );
    }

    #[tokio::test]
    async fn cancelling_during_an_active_attempt_is_observed_once_and_forbids_retry() {
        let dispatches = Arc::new(AtomicUsize::new(0));
        let cancellations = Arc::new(AtomicUsize::new(0));
        let resolver_hits = Arc::new(AtomicUsize::new(0));
        let mut collection = Models::new();
        collection
            .register_route(
                Box::new(CancelAwareProvider {
                    models: vec![ModelInfo::new(
                        "model-a",
                        "model-a",
                        WireApi::OpenAiCompletions,
                        ModelCapabilities::new(100_000, 4_096),
                    )],
                    dispatches: dispatches.clone(),
                    cancellations: cancellations.clone(),
                }),
                Arc::new(CountingResolver {
                    count: resolver_hits.clone(),
                    secret: SecretString::from("sk-cancel-test"),
                }),
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .unwrap();

        let request = minimal_request("cancel-aware:model-a");
        let token = request.cancel.clone();
        let prepared = collection
            .prepare_call("cancel-aware:model-a", request)
            .await
            .expect("prepare call");
        let mut stream = prepared.start_attempt().expect("start active attempt");
        assert!(futures_util::poll!(stream.next()).is_pending());
        assert_eq!(dispatches.load(Ordering::SeqCst), 1);
        let task = tokio::spawn(async move { stream.next().await });
        token.cancel();

        let terminal = tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .expect("active attempt observes cancellation")
            .expect("attempt task did not panic");
        assert!(matches!(terminal, Some(Err(ProviderError::Cancelled))));
        assert_eq!(resolver_hits.load(Ordering::SeqCst), 1);
        assert_eq!(dispatches.load(Ordering::SeqCst), 1);
        assert_eq!(cancellations.load(Ordering::SeqCst), 1);
        assert!(matches!(
            prepared.start_attempt(),
            Err(ModelsError::CallCancelled)
        ));
        assert_eq!(dispatches.load(Ordering::SeqCst), 1, "no retry dispatch");
    }

    #[tokio::test]
    async fn drain_retains_wire_cancellation_and_forbids_redispatch() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind stalled collection server");
        let addr = listener
            .local_addr()
            .expect("stalled collection server addr");
        let request_received = Arc::new(tokio::sync::Notify::new());
        let server_request_received = request_received.clone();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept Azure request");
            let mut request = Vec::new();
            let mut buf = [0_u8; 1024];
            loop {
                let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                    .await
                    .expect("read Azure request");
                if read == 0 {
                    return;
                }
                request.extend_from_slice(&buf[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            server_request_received.notify_one();
            std::future::pending::<()>().await;
        });

        let dispatches = Arc::new(AtomicUsize::new(0));
        let base_url = format!("http://127.0.0.1:{}", addr.port());
        let provider =
            AzureOpenAIProvider::new(Some(base_url), "my-gpt4o".into(), Some("2024-06-01".into()))
                .expect("construct Azure provider");
        let mut collection = Models::new();
        collection
            .register_route(
                Box::new(CountingAzureProvider {
                    inner: provider,
                    dispatches: dispatches.clone(),
                }),
                Arc::new(StaticAuthResolver::new(
                    AuthScheme::ApiKey,
                    SecretString::from("sk-collection-cancel"),
                )),
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .expect("register Azure route");

        let request = minimal_request("azure:my-gpt4o");
        let token = request.cancel.clone();
        let prepared = collection
            .prepare_call("azure:my-gpt4o", request)
            .await
            .expect("prepare Azure call");
        let attempt = prepared.start_attempt().expect("start Azure attempt");
        let drain = tokio::spawn(drain_to_completion(attempt));
        tokio::time::timeout(Duration::from_secs(1), request_received.notified())
            .await
            .expect("Azure request must reach the stalled server");
        token.cancel();

        let error = tokio::time::timeout(Duration::from_secs(1), drain)
            .await
            .expect("attempt drain must not wait for HTTP after cancellation")
            .expect("attempt drain task must not panic")
            .expect_err("cancelled attempt must drain to Err(CallFailure)");
        assert!(
            matches!(error.cause(), ProviderError::Cancelled),
            "got {:?}",
            error
        );
        assert_eq!(
            error.message().stop_reason,
            opi_ai::stream::StopReason::Aborted,
            "cancellation maps to Aborted"
        );
        assert!(matches!(
            prepared.start_attempt(),
            Err(ModelsError::CallCancelled)
        ));
        assert_eq!(dispatches.load(Ordering::SeqCst), 1, "no redispatch");
    }

    /// A resolver that always fails preparation with a typed credential error.
    struct FailingResolver {
        kind: FailKind,
        provider_id: &'static str,
    }

    #[derive(Debug, Clone, Copy)]
    enum FailKind {
        Revoked,
        Needed,
    }

    struct CredentialRejectingProvider {
        kind: FailKind,
        models: Vec<ModelInfo>,
        dispatches: Arc<AtomicUsize>,
    }

    impl Provider for CredentialRejectingProvider {
        fn id(&self) -> &str {
            "alpha"
        }

        fn models(&self) -> &[ModelInfo] {
            &self.models
        }

        fn stream_prepared(&self, _request: Request, _auth: ResolvedAuth) -> EventStream {
            self.dispatches.fetch_add(1, Ordering::SeqCst);
            let first = text_response("started")
                .into_iter()
                .next()
                .expect("text response starts with a non-terminal event");
            let error = match self.kind {
                FailKind::Revoked => ProviderError::CredentialRevoked {
                    provider_id: "alpha".to_owned(),
                },
                FailKind::Needed => ProviderError::CredentialNeeded {
                    provider_id: "alpha".to_owned(),
                },
            };
            Box::pin(futures_util::stream::iter([Ok(first), Err(error)]))
        }
    }

    impl AuthResolver for FailingResolver {
        fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
            let kind = self.kind;
            let provider_id = self.provider_id.to_owned();
            Box::pin(async move {
                Err(match kind {
                    FailKind::Revoked => ProviderError::CredentialRevoked { provider_id },
                    FailKind::Needed => ProviderError::CredentialNeeded { provider_id },
                })
            })
        }
    }

    #[tokio::test]
    async fn rejected_or_expired_credential_terminates_the_call_without_dispatch() {
        for kind in [FailKind::Revoked, FailKind::Needed] {
            let prepared_hits = Arc::new(AtomicUsize::new(0));
            let legacy_hits = Arc::new(AtomicUsize::new(0));
            let provider = ProbeProvider::new(
                "alpha",
                "model-a",
                prepared_hits.clone(),
                legacy_hits.clone(),
            );
            let mut collection = Models::new();
            collection
                .register_route(
                    Box::new(provider),
                    Arc::new(FailingResolver {
                        kind,
                        provider_id: "alpha",
                    }),
                    AuthProvenanceSource::Static,
                    CompatMetadata::default(),
                )
                .expect("register route");

            let err = collection
                .prepare_call("alpha:model-a", minimal_request("alpha:model-a"))
                .await
                .expect_err("credential failure must terminate the call");
            assert!(
                matches!(
                    err,
                    ModelsError::Provider(
                        ProviderError::CredentialRevoked { .. }
                            | ProviderError::CredentialNeeded { .. }
                    )
                ),
                "got {err:?}"
            );
            assert_eq!(
                prepared_hits.load(Ordering::SeqCst),
                0,
                "provider dispatched despite credential failure ({kind:?})"
            );
        }
    }

    #[tokio::test]
    async fn stream_time_credential_failure_forbids_redispatch() {
        for kind in [FailKind::Revoked, FailKind::Needed] {
            let dispatches = Arc::new(AtomicUsize::new(0));
            let mut collection = Models::new();
            collection
                .register_route(
                    Box::new(CredentialRejectingProvider {
                        kind,
                        models: vec![ModelInfo::new(
                            "model-a",
                            "model-a",
                            WireApi::OpenAiCompletions,
                            ModelCapabilities::new(100_000, 4_096),
                        )],
                        dispatches: dispatches.clone(),
                    }),
                    Arc::new(CountingResolver {
                        count: Arc::new(AtomicUsize::new(0)),
                        secret: SecretString::from("sk-rejected"),
                    }),
                    AuthProvenanceSource::Static,
                    CompatMetadata::default(),
                )
                .expect("register route");
            let prepared = collection
                .prepare_call("alpha:model-a", minimal_request("alpha:model-a"))
                .await
                .expect("prepare call");

            let mut attempt = prepared.start_attempt().expect("start attempt");
            let first = attempt
                .next()
                .await
                .expect("stream starts")
                .expect("first event succeeds");
            assert!(!first.is_terminal());
            assert!(matches!(
                attempt.next().await,
                Some(Err(ProviderError::CredentialRevoked { .. }
                    | ProviderError::CredentialNeeded { .. }))
            ));

            let error = match prepared.start_attempt() {
                Err(error) => error,
                Ok(_) => panic!("credential-terminal call must reject redispatch"),
            };
            assert!(
                matches!(
                    &error,
                    ModelsError::CredentialTerminated { provider }
                        if provider.as_str() == "alpha"
                ),
                "got {error:?}"
            );
            assert_eq!(dispatches.load(Ordering::SeqCst), 1, "no redispatch");
        }
    }

    #[tokio::test]
    async fn prepared_call_debug_redacts_the_resolved_secret() {
        let prepared_hits = Arc::new(AtomicUsize::new(0));
        let legacy_hits = Arc::new(AtomicUsize::new(0));
        let resolve_hits = Arc::new(AtomicUsize::new(0));
        let collection = collection_with_probe(
            prepared_hits.clone(),
            legacy_hits.clone(),
            resolve_hits.clone(),
        );
        let prepared = collection
            .prepare_call("alpha:model-a", minimal_request("alpha:model-a"))
            .await
            .expect("prepare_call");

        let call_dbg = format!("{prepared:?}");
        assert!(
            !call_dbg.contains("sk-alpha-canary"),
            "secret leaked via PreparedProviderCall debug: {call_dbg}"
        );
        let route_dbg = format!("{:?}", prepared.route());
        assert!(
            !route_dbg.contains("sk-alpha-canary"),
            "secret leaked via route debug: {route_dbg}"
        );
    }

    use opi_ai::message::{ImageSource, InputContent, MediaType, Message, UserMessage};

    /// A request carrying image content (rejected by text-only models).
    fn image_request(model: &str) -> Request {
        let mut request = minimal_request(model);
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Image {
                    source: ImageSource::Bytes {
                        data: vec![0x89, 0x50, 0x4E, 0x47],
                    },
                    media_type: MediaType::Png,
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations");
        request
    }

    #[tokio::test]
    async fn unknown_route_returns_typed_error_without_dispatch() {
        let prepared_hits = Arc::new(AtomicUsize::new(0));
        let legacy_hits = Arc::new(AtomicUsize::new(0));
        let resolve_hits = Arc::new(AtomicUsize::new(0));
        let collection = collection_with_probe(
            prepared_hits.clone(),
            legacy_hits.clone(),
            resolve_hits.clone(),
        );

        let err = collection
            .prepare_call("nope:missing", minimal_request("nope:missing"))
            .await
            .expect_err("unknown route must fail");
        assert!(matches!(err, ModelsError::Registry(_)), "got {err:?}");
        assert_eq!(
            prepared_hits.load(Ordering::SeqCst),
            0,
            "dispatched despite unknown route"
        );
    }

    #[tokio::test]
    async fn unknown_model_returns_typed_registry_error_without_dispatch() {
        let prepared_hits = Arc::new(AtomicUsize::new(0));
        let legacy_hits = Arc::new(AtomicUsize::new(0));
        let resolve_hits = Arc::new(AtomicUsize::new(0));
        let collection = collection_with_probe(
            prepared_hits.clone(),
            legacy_hits.clone(),
            resolve_hits.clone(),
        );

        let err = collection
            .prepare_call(
                "alpha:no-such-model",
                minimal_request("alpha:no-such-model"),
            )
            .await
            .expect_err("unknown model must fail");
        assert!(
            matches!(
                err,
                ModelsError::Registry(opi_ai::registry::RegistryError::UnknownModel { .. })
            ),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn request_route_mismatch_and_noncanonical_identity_fail_before_auth_or_dispatch() {
        let alpha_dispatches = Arc::new(AtomicUsize::new(0));
        let alpha_resolutions = Arc::new(AtomicUsize::new(0));
        let beta_dispatches = Arc::new(AtomicUsize::new(0));
        let beta_resolutions = Arc::new(AtomicUsize::new(0));
        let mut collection = Models::new();
        for (provider, dispatches, resolutions) in [
            ("alpha", alpha_dispatches.clone(), alpha_resolutions.clone()),
            ("beta", beta_dispatches.clone(), beta_resolutions.clone()),
        ] {
            collection
                .register_route(
                    Box::new(ProbeProvider::new(
                        provider,
                        "model-a",
                        dispatches,
                        Arc::new(AtomicUsize::new(0)),
                    )),
                    Arc::new(CountingResolver {
                        count: resolutions,
                        secret: SecretString::from("sk-canary"),
                    }),
                    AuthProvenanceSource::Static,
                    CompatMetadata::default(),
                )
                .expect("register route");
        }

        for request_model in [
            "beta:model-a",
            "alpha:model-a ",
            " alpha:model-a",
            "alpha: model-a",
        ] {
            let error = collection
                .prepare_call("alpha:model-a", minimal_request(request_model))
                .await
                .expect_err("mismatched or noncanonical request identity must fail");
            assert!(
                matches!(
                    &error,
                    ModelsError::RequestRouteMismatch {
                        request_model: actual_request_model,
                        route_provider,
                        route_model,
                    } if actual_request_model == request_model
                        && route_provider.as_str() == "alpha"
                        && route_model.as_str() == "model-a"
                ),
                "got {error:?} for {request_model:?}"
            );
        }
        assert_eq!(alpha_resolutions.load(Ordering::SeqCst), 0);
        assert_eq!(beta_resolutions.load(Ordering::SeqCst), 0);
        assert_eq!(alpha_dispatches.load(Ordering::SeqCst), 0);
        assert_eq!(beta_dispatches.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn auth_failure_does_not_silently_fall_back_to_another_provider() {
        // alpha's auth fails; beta is healthy and must NOT be tried.
        let beta_prepared = Arc::new(AtomicUsize::new(0));
        let mut collection = Models::new();
        collection
            .register_route(
                Box::new(ProbeProvider::new(
                    "alpha",
                    "model-a",
                    Arc::new(AtomicUsize::new(0)),
                    Arc::new(AtomicUsize::new(0)),
                )),
                Arc::new(FailingResolver {
                    kind: FailKind::Revoked,
                    provider_id: "alpha",
                }),
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .expect("register alpha");
        collection
            .register_route(
                Box::new(ProbeProvider::new(
                    "beta",
                    "model-b",
                    beta_prepared.clone(),
                    Arc::new(AtomicUsize::new(0)),
                )),
                Arc::new(CountingResolver {
                    count: Arc::new(AtomicUsize::new(0)),
                    secret: SecretString::from("sk-beta-canary"),
                }),
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .expect("register beta");

        let err = collection
            .prepare_call("alpha:model-a", minimal_request("alpha:model-a"))
            .await
            .expect_err("alpha auth must fail");
        assert!(
            matches!(
                err,
                ModelsError::Provider(ProviderError::CredentialRevoked { .. })
            ),
            "got {err:?}"
        );
        assert_eq!(
            beta_prepared.load(Ordering::SeqCst),
            0,
            "silently fell back to beta"
        );
    }

    #[tokio::test]
    async fn capability_mismatch_returns_typed_error_before_auth_or_dispatch() {
        let prepared_hits = Arc::new(AtomicUsize::new(0));
        let legacy_hits = Arc::new(AtomicUsize::new(0));
        let resolve_hits = Arc::new(AtomicUsize::new(0));
        let collection = collection_with_probe(
            prepared_hits.clone(),
            legacy_hits.clone(),
            resolve_hits.clone(),
        );

        // alpha's model is text-only; an image request is rejected before dispatch.
        let err = collection
            .prepare_call("alpha:model-a", image_request("alpha:model-a"))
            .await
            .expect_err("image to text-only model must fail");
        assert!(
            matches!(
                err,
                ModelsError::Provider(ProviderError::UnsupportedCapability(_))
            ),
            "got {err:?}"
        );
        assert_eq!(
            prepared_hits.load(Ordering::SeqCst),
            0,
            "dispatched despite capability mismatch"
        );
        assert_eq!(
            resolve_hits.load(Ordering::SeqCst),
            0,
            "auth resolved despite earlier capability failure"
        );
    }

    #[tokio::test]
    async fn prepare_call_surfaces_each_registered_auth_source_on_the_route() {
        // Behavioral: every registered source classification must round-trip
        // through the real prepare_call path onto route().provenance.source.
        // (Replaces an earlier construct-then-assert-equals-self test.)
        let sources: [(&str, AuthProvenanceSource); 4] = [
            ("static-prov", AuthProvenanceSource::Static),
            (
                "env-prov",
                AuthProvenanceSource::Environment {
                    name: "PROVIDER_KEY".to_owned(),
                },
            ),
            (
                "store-prov",
                AuthProvenanceSource::CredentialStore {
                    kind: "keychain".to_owned(),
                },
            ),
            (
                "oauth-prov",
                AuthProvenanceSource::OAuth {
                    kind: "github-copilot".to_owned(),
                },
            ),
        ];
        for (id, source) in sources {
            let mut collection = Models::new();
            collection
                .register_route(
                    Box::new(ProbeProvider::new(
                        id,
                        "model-a",
                        Arc::new(AtomicUsize::new(0)),
                        Arc::new(AtomicUsize::new(0)),
                    )),
                    Arc::new(CountingResolver {
                        count: Arc::new(AtomicUsize::new(0)),
                        secret: SecretString::from("sk-canary"),
                    }),
                    source.clone(),
                    CompatMetadata::default(),
                )
                .expect("register route");
            let spec = format!("{id}:model-a");
            let prepared = collection
                .prepare_call(&spec, minimal_request(&spec))
                .await
                .expect("prepare_call");
            assert_eq!(
                prepared.auth_provenance().source,
                source,
                "source not surfaced on route for {id}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Task 20.3: simple and advanced entries observe the same route and outcome
// ---------------------------------------------------------------------------

#[tokio::test]
async fn models_simple_and_advanced_observe_same_route_and_outcome() {
    // One dispatchable route; the caller uses the neutral entry and advanced
    // preparation and must observe the same selected route and final message.
    let mut models = Models::new();
    models
        .register_route(
            text_mock_repeated("parity", "same answer", 2),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                secret("configured"),
            )) as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();

    // The complete record comes from the sync catalog read.
    let info = models
        .get_model("parity", "mock-model")
        .expect("catalog record");
    assert_eq!(info.id, "mock-model");
    let record = opi_ai::models::ChatModelRecord::new("parity", info);

    // Neutral entry.
    let simple_message = models
        .complete_simple(
            record,
            minimal_request("parity:mock-model"),
            Default::default(),
        )
        .await
        .unwrap();
    assert_eq!(assistant_text(&simple_message), "same answer");

    // Advanced preparation through the same state machine.
    let prepared = models
        .prepare_call("parity:mock-model", minimal_request("parity:mock-model"))
        .await
        .unwrap();
    assert_eq!(prepared.route().provider_id, "parity");
    assert_eq!(prepared.route().model_id, "mock-model");
    let attempt = prepared.start_attempt().unwrap();
    let advanced_message = opi_ai::stream::ChatStream::new(attempt)
        .result()
        .await
        .unwrap();
    assert_eq!(assistant_text(&advanced_message), "same answer");

    // Both paths resolve the same route facts and canonical model identity.
    assert_eq!(simple_message.model, advanced_message.model);
    assert_eq!(simple_message.provider, advanced_message.provider);
}

mod registry_preflight_and_refresh {
    use super::minimal_request as request;
    use std::sync::{Arc, Mutex};

    use opi_ai::credential::BoxAuthFuture;
    use opi_ai::model_info::ThinkingLevel;
    use opi_ai::models::{AuthDescriptor, CompatMetadata, Models, RefreshOutcome, SecretKey};
    use opi_ai::provider::{Provider, ProviderError, Request, ThinkingConfig};
    use opi_ai::{ModelCapabilities, ModelInfo, WireApi};

    struct CountingRoute {
        model: Vec<ModelInfo>,
        calls: Arc<std::sync::atomic::AtomicUsize>,
    }

    impl Provider for CountingRoute {
        fn id(&self) -> &str {
            "mapped"
        }

        fn models(&self) -> &[ModelInfo] {
            &self.model
        }

        fn stream_prepared(
            &self,
            _request: Request,
            _auth: opi_ai::auth::ResolvedAuth,
        ) -> opi_ai::provider::EventStream {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Box::pin(futures_util::stream::empty())
        }

        fn replace_model_catalog(&mut self, models: Vec<ModelInfo>) -> Result<(), ProviderError> {
            self.model = models;
            Ok(())
        }
    }

    fn unsupported_xhigh_model() -> ModelInfo {
        ModelInfo::new(
            "reasoning",
            "Reasoning",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(1000, 100).with_thinking(true),
        )
    }
    #[tokio::test]
    async fn collection_preflights_registry_model_before_provider_dispatch() {
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut collection = Models::new();
        collection
            .register(
                Box::new(CountingRoute {
                    model: vec![unsupported_xhigh_model()],
                    calls: calls.clone(),
                }),
                AuthDescriptor::StaticApiKey {
                    value: SecretKey::new("unused"),
                },
                CompatMetadata::default(),
            )
            .unwrap();
        let mut request = request("mapped:reasoning");
        request.thinking = ThinkingConfig {
            enabled: true,
            budget_tokens: None,
            level: ThinkingLevel::XHigh,
        };

        let error = collection
            .prepare_call("mapped:reasoning", request)
            .await
            .expect_err("unsupported thinking must fail");

        assert!(error.to_string().contains("unsupported"));
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    }
    struct LoggedRefreshProvider {
        id: &'static str,
        log: Arc<Mutex<Vec<String>>>,
        result: Result<Option<Vec<ModelInfo>>, &'static str>,
    }

    impl Provider for LoggedRefreshProvider {
        fn id(&self) -> &str {
            self.id
        }

        fn models(&self) -> &[ModelInfo] {
            &[]
        }

        fn stream_prepared(
            &self,
            _request: Request,
            _auth: opi_ai::auth::ResolvedAuth,
        ) -> opi_ai::provider::EventStream {
            Box::pin(futures_util::stream::empty())
        }

        fn refresh_models(
            &self,
        ) -> BoxAuthFuture<'_, Result<Option<Vec<ModelInfo>>, ProviderError>> {
            self.log.lock().unwrap().push(self.id.into());
            let result = self.result.clone();
            Box::pin(async move {
                result.map_err(|message| {
                    ProviderError::ProviderSide(
                        opi_ai::provider::ProviderErrorSummary::from_untrusted(message),
                    )
                })
            })
        }
    }
    #[tokio::test]
    async fn refresh_collects_every_provider_after_the_first_error() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut collection = Models::new();
        for provider in [
            LoggedRefreshProvider {
                id: "a-error",
                log: log.clone(),
                result: Err("first"),
            },
            LoggedRefreshProvider {
                id: "b-success",
                log: log.clone(),
                result: Ok(Some(vec![ModelInfo::new(
                    "fresh",
                    "Fresh",
                    WireApi::OpenAiCompletions,
                    ModelCapabilities::new(1000, 100),
                )])),
            },
            LoggedRefreshProvider {
                id: "c-error",
                log: log.clone(),
                result: Err("later"),
            },
        ] {
            collection
                .register(
                    Box::new(provider),
                    AuthDescriptor::StaticApiKey {
                        value: SecretKey::new("unused"),
                    },
                    CompatMetadata::default(),
                )
                .unwrap();
        }

        // Refresh reports per-provider outcomes and still evaluates every
        // provider in deterministic (sorted) id order after an error.
        let report = collection.refresh(true).await.unwrap();
        assert_eq!(
            *log.lock().unwrap(),
            ["a-error", "b-success", "c-error"],
            "refresh must call every provider in sorted order"
        );
        // Successful providers publish despite sibling failures.
        assert!(collection.resolve("b-success:fresh").is_ok());
        assert!(report.find("a-error").unwrap().outcome.is_failure());
        assert!(report.find("c-error").unwrap().outcome.is_failure());
        assert!(matches!(
            report.find("b-success").unwrap().outcome,
            RefreshOutcome::Published { models: 1 }
        ));
    }
}

// ---------------------------------------------------------------------------
// ModelsStore-backed refresh: restore, atomic publication, late/superseded
// writes, cancellation, catalog sources, and binding stability (task 20.33)
// ---------------------------------------------------------------------------

mod catalog_store_refresh_20_33 {
    use super::*;
    use opi_ai::RefreshStatus;
    use opi_ai::catalog::{DynamicCatalogSource, HttpCatalogSource, ModelsStore};
    use opi_ai::models::RefreshOutcome;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Provider whose `refresh_models` signals entry and blocks until released.
    struct BarrierRefreshProvider {
        id: &'static str,
        builtin: Vec<ModelInfo>,
        fresh: Vec<ModelInfo>,
        entered: Arc<tokio::sync::Notify>,
        release: Mutex<Option<tokio::sync::oneshot::Receiver<()>>>,
    }

    impl BarrierRefreshProvider {
        fn new(
            id: &'static str,
            fresh: Vec<ModelInfo>,
            entered: Arc<tokio::sync::Notify>,
            release: tokio::sync::oneshot::Receiver<()>,
        ) -> Self {
            Self {
                id,
                builtin: Vec::new(),
                fresh,
                entered,
                release: Mutex::new(Some(release)),
            }
        }
    }

    impl Provider for BarrierRefreshProvider {
        fn id(&self) -> &str {
            self.id
        }
        fn models(&self) -> &[ModelInfo] {
            Box::leak(self.builtin.clone().into_boxed_slice())
        }
        fn stream_prepared(
            &self,
            _request: Request,
            _auth: opi_ai::auth::ResolvedAuth,
        ) -> opi_ai::provider::EventStream {
            Box::pin(futures_util::stream::empty())
        }
        fn refresh_models(
            &self,
        ) -> BoxAuthFuture<'_, Result<Option<Vec<ModelInfo>>, ProviderError>> {
            self.entered.notify_one();
            let release = self.release.lock().unwrap().take();
            let fresh = self.fresh.clone();
            Box::pin(async move {
                if let Some(release) = release {
                    let _ = release.await;
                }
                Ok(Some(fresh))
            })
        }
    }

    /// Provider that counts refresh fetches, proving offline refresh never
    /// touches the network path.
    struct CountingRefreshProvider {
        id: &'static str,
        fresh: Vec<ModelInfo>,
        fetches: Arc<AtomicUsize>,
    }

    struct CountingOAuthResolver(Arc<AtomicUsize>);

    impl AuthResolver for CountingOAuthResolver {
        fn resolve(&self) -> BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Box::pin(async {
                Err(ProviderError::CredentialNeeded {
                    provider_id: "dyn".to_owned(),
                })
            })
        }
    }

    impl Provider for CountingRefreshProvider {
        fn id(&self) -> &str {
            self.id
        }
        fn models(&self) -> &[ModelInfo] {
            Box::leak(self.fresh.clone().into_boxed_slice())
        }
        fn stream_prepared(
            &self,
            _request: Request,
            _auth: opi_ai::auth::ResolvedAuth,
        ) -> opi_ai::provider::EventStream {
            Box::pin(futures_util::stream::empty())
        }
        fn refresh_models(
            &self,
        ) -> BoxAuthFuture<'_, Result<Option<Vec<ModelInfo>>, ProviderError>> {
            self.fetches.fetch_add(1, Ordering::SeqCst);
            let fresh = self.fresh.clone();
            Box::pin(async move { Ok(Some(fresh)) })
        }
    }

    /// User-plugged dynamic catalog source with fixed output.
    struct FixedCatalogSource {
        id: String,
        catalogs: HashMap<String, opi_ai::catalog::ModelsStoreEntry>,
    }

    enum CancelAtCatalogResult {
        External {
            entered: Arc<std::sync::Barrier>,
            released: Arc<std::sync::Barrier>,
        },
        Source,
        BeforeRefresh,
    }

    struct CancellingCatalogSource {
        fetches: Arc<AtomicUsize>,
        cancellation: Arc<CancelAtCatalogResult>,
    }

    impl DynamicCatalogSource for CancellingCatalogSource {
        fn id(&self) -> &str {
            "cancel-at-result"
        }

        fn fetch(
            &self,
            token: CancellationToken,
        ) -> futures_util::future::BoxFuture<
            'static,
            Result<HashMap<String, opi_ai::catalog::ModelsStoreEntry>, ProviderError>,
        > {
            let later = self.fetches.fetch_add(1, Ordering::SeqCst) > 0;
            let cancellation = self.cancellation.clone();
            Box::pin(async move {
                if later {
                    match cancellation.as_ref() {
                        CancelAtCatalogResult::External { entered, released } => {
                            // Keep this poll pending synchronously until the external
                            // thread cancels, then return a successful late result.
                            entered.wait();
                            released.wait();
                        }
                        CancelAtCatalogResult::Source => token.cancel(),
                        CancelAtCatalogResult::BeforeRefresh => {}
                    }
                }
                let id = if later { "late" } else { "old" };
                Ok(catalog_fixture::catalogs(HashMap::from([(
                    "dyn".to_owned(),
                    vec![model_info(id, id)],
                )])))
            })
        }
    }

    async fn assert_cancelled_catalog_retains_publication(
        with_store: bool,
        cancellation: CancelAtCatalogResult,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("models-store.json");
        let mut collection = Models::new();
        register(
            &mut collection,
            Box::new(RefreshProvider::new("dyn", vec![])),
        );
        if with_store {
            collection.set_models_store(ModelsStore::open(&path).unwrap());
        }
        let fetches = Arc::new(AtomicUsize::new(0));
        let cancellation = Arc::new(cancellation);
        collection.register_catalog_source(Arc::new(CancellingCatalogSource {
            fetches: fetches.clone(),
            cancellation: cancellation.clone(),
        }));
        collection.refresh(true).await.unwrap();
        assert!(collection.resolve("dyn:old").is_ok());
        let persisted = with_store.then(|| std::fs::read(&path).unwrap());

        let token = CancellationToken::new();
        let external = match cancellation.as_ref() {
            CancelAtCatalogResult::External { entered, released } => {
                let (entered, released) = (entered.clone(), released.clone());
                let token = token.clone();
                Some(std::thread::spawn(move || {
                    entered.wait();
                    token.cancel();
                    released.wait();
                }))
            }
            CancelAtCatalogResult::BeforeRefresh => {
                token.cancel();
                None
            }
            CancelAtCatalogResult::Source => None,
        };
        let report = collection.refresh_with(true, &token).await.unwrap();
        if let Some(external) = external {
            external.join().unwrap();
        }
        assert!(token.is_cancelled());
        assert_eq!(
            report.status(),
            RefreshStatus::Cancelled,
            "store={with_store}"
        );
        assert!(collection.resolve("dyn:old").is_ok(), "store={with_store}");
        assert!(
            collection.resolve("dyn:late").is_err(),
            "store={with_store}"
        );
        let expected_fetches =
            if matches!(cancellation.as_ref(), CancelAtCatalogResult::BeforeRefresh) {
                1
            } else {
                2
            };
        assert_eq!(fetches.load(Ordering::SeqCst), expected_fetches);
        if let Some(persisted) = persisted {
            assert_eq!(std::fs::read(&path).unwrap(), persisted);
            let reopened = ModelsStore::open(&path).unwrap();
            assert_eq!(reopened.revision(), 1);
            assert_eq!(reopened.snapshot().catalogs["dyn"].models[0].id(), "old");
        }
    }

    #[tokio::test]
    async fn cancelled_catalog_external_result_preserves_old_revision() {
        for with_store in [true, false] {
            assert_cancelled_catalog_retains_publication(
                with_store,
                CancelAtCatalogResult::External {
                    entered: Arc::new(std::sync::Barrier::new(2)),
                    released: Arc::new(std::sync::Barrier::new(2)),
                },
            )
            .await;
        }
    }

    #[tokio::test]
    async fn cancelled_catalog_source_result_preserves_old_revision() {
        for with_store in [true, false] {
            assert_cancelled_catalog_retains_publication(with_store, CancelAtCatalogResult::Source)
                .await;
        }
    }

    #[tokio::test]
    async fn cancelled_catalog_before_refresh_preserves_old_revision_without_fetch() {
        for with_store in [true, false] {
            assert_cancelled_catalog_retains_publication(
                with_store,
                CancelAtCatalogResult::BeforeRefresh,
            )
            .await;
        }
    }

    impl DynamicCatalogSource for FixedCatalogSource {
        fn id(&self) -> &str {
            &self.id
        }
        fn fetch(
            &self,
            _token: CancellationToken,
        ) -> futures_util::future::BoxFuture<
            'static,
            Result<HashMap<String, opi_ai::catalog::ModelsStoreEntry>, ProviderError>,
        > {
            let catalogs = self.catalogs.clone();
            Box::pin(async move { Ok(catalogs) })
        }
    }

    /// User-plugged dynamic catalog source that always fails.
    struct FailingCatalogSource {
        id: String,
    }

    impl DynamicCatalogSource for FailingCatalogSource {
        fn id(&self) -> &str {
            &self.id
        }
        fn fetch(
            &self,
            _token: CancellationToken,
        ) -> futures_util::future::BoxFuture<
            'static,
            Result<HashMap<String, opi_ai::catalog::ModelsStoreEntry>, ProviderError>,
        > {
            Box::pin(async move {
                Err(ProviderError::ProviderSide(
                    opi_ai::provider::ProviderErrorSummary::from_untrusted("source down"),
                ))
            })
        }
    }

    fn register(collection: &mut Models, provider: Box<dyn Provider>) {
        collection
            .register(
                provider,
                AuthDescriptor::StaticApiKey {
                    value: SecretKey::new("key"),
                },
                CompatMetadata::default(),
            )
            .unwrap();
    }

    #[tokio::test]
    async fn refresh_false_performs_no_network_and_restores_store_first() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("models-store.json");

        // A prior publication exists in the store.
        let store = ModelsStore::open(&path).unwrap();
        store
            .begin()
            .commit(catalog_fixture::catalogs(HashMap::from([(
                "dyn".to_owned(),
                vec![model_info("stored-1", "Stored 1")],
            )])))
            .await
            .unwrap();

        let fetches = Arc::new(AtomicUsize::new(0));
        let auth_resolutions = Arc::new(AtomicUsize::new(0));
        let source_fetches = Arc::new(AtomicUsize::new(0));
        let mut collection = Models::new();
        collection
            .register_route(
                Box::new(CountingRefreshProvider {
                    id: "dyn",
                    fresh: vec![model_info("fresh-1", "Fresh 1")],
                    fetches: fetches.clone(),
                }),
                Arc::new(CountingOAuthResolver(auth_resolutions.clone())),
                AuthProvenanceSource::OAuth {
                    kind: "offline-fixture".to_owned(),
                },
                CompatMetadata::default(),
            )
            .unwrap();
        collection.register_catalog_source(Arc::new(CancellingCatalogSource {
            fetches: source_fetches.clone(),
            cancellation: Arc::new(CancelAtCatalogResult::BeforeRefresh),
        }));
        collection.set_models_store(store);

        // Offline refresh: restore the persisted cache, never fetch.
        let report = collection.refresh(false).await.unwrap();
        assert_eq!(
            report.status(),
            RefreshStatus::OfflineRestored,
            "refresh(false) restores without network"
        );
        assert_eq!(fetches.load(Ordering::SeqCst), 0, "no provider fetch");
        assert_eq!(source_fetches.load(Ordering::SeqCst), 0, "no source fetch");
        assert_eq!(auth_resolutions.load(Ordering::SeqCst), 0, "no OAuth work");
        assert!(collection.resolve("dyn:stored-1").is_ok());
        assert!(
            collection.resolve("dyn:fresh-1").is_err(),
            "offline refresh must not publish unfetched models"
        );
        assert!(matches!(
            report.find("dyn").map(|entry| &entry.outcome),
            Some(RefreshOutcome::Restored { models: 1 })
        ));
    }

    #[tokio::test]
    async fn late_superseded_refresh_cannot_publish_or_rollback() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("models-store.json");

        let mut collection = Models::new();
        let store = ModelsStore::open(&path).unwrap();
        store
            .begin()
            .commit(catalog_fixture::catalogs(HashMap::from([(
                "winner".to_owned(),
                vec![model_info("w1", "W1")],
            )])))
            .await
            .unwrap();
        collection.set_models_store(store.clone());

        // Refresh A hangs inside its fetch; while it hangs, a second publisher
        // commits a newer revision directly through the store.
        let entered = Arc::new(tokio::sync::Notify::new());
        let (release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
        register(
            &mut collection,
            Box::new(BarrierRefreshProvider::new(
                "racer",
                vec![model_info("racer-fresh", "Racer Fresh")],
                entered.clone(),
                release_rx,
            )),
        );

        let mut refresh_a = collection;
        let task = tokio::spawn(async move { refresh_a.refresh(true).await.unwrap() });
        entered.notified().await;

        // Publisher B commits revision 2 while A is suspended in fetch.
        store
            .begin()
            .commit(catalog_fixture::catalogs(HashMap::from([(
                "winner".to_owned(),
                vec![model_info("w2", "W2")],
            )])))
            .await
            .unwrap();

        release_tx.send(()).unwrap();
        let report = task.await.unwrap();
        assert_eq!(
            report.status(),
            RefreshStatus::Superseded,
            "the late refresh must be dropped, not published"
        );

        // Store: B's revision wins; A's racer write never lands.
        assert_eq!(store.revision(), 2);
        assert!(
            !store.snapshot().catalogs.contains_key("racer"),
            "superseded write must not persist"
        );

        // Reopened store shows B's publication (no rollback).
        let reopened = ModelsStore::open(&path).unwrap();
        assert_eq!(reopened.revision(), 2);
        assert!(reopened.snapshot().catalogs["winner"].models[0].id() == "w2");

        // A fresh collection restoring from the store sees B's world.
        let mut collection_b = Models::new();
        register(
            &mut collection_b,
            Box::new(RefreshProvider::new("winner", vec![])),
        );
        collection_b.set_models_store(ModelsStore::open(&path).unwrap());
        collection_b.refresh(false).await.unwrap();
        assert!(collection_b.resolve("winner:w2").is_ok());
        assert!(collection_b.resolve("racer:racer-fresh").is_err());
    }

    #[tokio::test]
    async fn cancelled_refresh_never_publishes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("models-store.json");
        let mut collection = Models::new();
        collection.set_models_store(ModelsStore::open(&path).unwrap());

        let entered = Arc::new(tokio::sync::Notify::new());
        let (release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
        register(
            &mut collection,
            Box::new(BarrierRefreshProvider::new(
                "racer",
                vec![model_info("racer-fresh", "Racer Fresh")],
                entered.clone(),
                release_rx,
            )),
        );
        // Hold the sender for the whole test: the fetch stays suspended until
        // cancellation drops it, and a released-late write cannot publish.
        let _hold_release = release_tx;

        let token = CancellationToken::new();
        let task_token = token.clone();
        let task = tokio::spawn(async move {
            let report = collection.refresh_with(true, &task_token).await.unwrap();
            (report, collection)
        });
        entered.notified().await;
        token.cancel();
        let (report, collection) = task.await.unwrap();
        assert_eq!(report.status(), RefreshStatus::Cancelled);

        // Nothing persisted, nothing published, and a later release of the
        // suspended fetch cannot publish after cancellation.
        assert!(!path.exists(), "cancelled refresh persists nothing");
        assert!(
            collection.resolve("racer:racer-fresh").is_err(),
            "cancelled work must never appear in the in-memory catalog"
        );
    }

    #[tokio::test]
    async fn custom_catalog_source_publishes_and_retains_failed_sources() {
        let mut collection = Models::new();
        // Catalog sources publish dynamic catalogs for registered providers.
        register(
            &mut collection,
            Box::new(RefreshProvider::new("custom-prov", vec![])),
        );
        collection.register_catalog_source(Arc::new(FixedCatalogSource {
            id: "custom-src".to_owned(),
            catalogs: catalog_fixture::catalogs(HashMap::from([(
                "custom-prov".to_owned(),
                vec![model_info("src-model", "Source Model")],
            )])),
        }));
        collection.register_catalog_source(Arc::new(FailingCatalogSource {
            id: "broken-src".to_owned(),
        }));

        let report = collection.refresh(true).await.unwrap();
        assert_eq!(report.status(), RefreshStatus::Published);
        assert!(
            collection.resolve("custom-prov:src-model").is_ok(),
            "custom source models publish through refresh"
        );
        assert!(matches!(
            report.find("custom-src").unwrap().outcome,
            RefreshOutcome::Published { models: 1 }
        ));
        assert!(
            report.find("broken-src").unwrap().outcome.is_failure(),
            "failing source reports its error"
        );
    }

    /// Minimal raw HTTP server returning queued responses, one per request.
    async fn spawn_catalog_server(
        responses: Vec<(u16, String)>,
    ) -> (std::net::SocketAddr, Arc<Mutex<VecDeque<(u16, String)>>>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind catalog server");
        let addr = listener.local_addr().expect("catalog server addr");
        let queue = Arc::new(Mutex::new(responses.into_iter().collect::<VecDeque<_>>()));
        let server_queue = queue.clone();
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut request = Vec::new();
                let mut buf = [0_u8; 1024];
                loop {
                    let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                        .await
                        .expect("read catalog request");
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buf[..read]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                let (code, body) = server_queue
                    .lock()
                    .unwrap()
                    .pop_front()
                    .unwrap_or((200, "{}".to_owned()));
                let reason = match code {
                    200 => "OK",
                    500 => "Internal Server Error",
                    _ => "Status",
                };
                let response = format!(
                    "HTTP/1.1 {code} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                use tokio::io::AsyncWriteExt;
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });
        (addr, queue)
    }

    #[tokio::test]
    async fn http_catalog_source_publishes_and_retains_on_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("models-store.json");

        let document = serde_json::json!({
            "providers": {
                "http-prov": {
                    "models": [
                        {
                            "id": "s1",
                            "name": "Server One",
                            "api": "openai-completions",
                            "contextWindow": 32_000,
                            "maxTokens": 2_048
                        }
                    ]
                }
            }
        })
        .to_string();
        let (addr, _queue) =
            spawn_catalog_server(vec![(200, document), (500, "boom".to_owned())]).await;
        let url = format!("http://127.0.0.1:{}", addr.port());

        let mut collection = Models::new();
        let store = ModelsStore::open(&path).unwrap();
        collection.set_models_store(store);
        // The provider exposes no native listing endpoint (its refresh fails);
        // the HTTP catalog source owns its dynamic catalog, and failures on
        // either side retain prior data.
        register(
            &mut collection,
            Box::new(ErrorRefreshProvider {
                id: "http-prov",
                error_msg: "no native models endpoint",
            }),
        );
        let source = HttpCatalogSource::new(url);
        collection.register_catalog_source(Arc::new(source));

        // Round 1: the controlled HTTP source publishes and persists.
        let report = collection.refresh(true).await.unwrap();
        assert_eq!(report.status(), RefreshStatus::Published);
        assert!(collection.resolve("http-prov:s1").is_ok());
        let persisted = ModelsStore::open(&path).unwrap();
        assert!(
            persisted.snapshot().catalogs.contains_key("http-prov"),
            "published source catalog persists through the store"
        );

        // Round 2: the server fails; prior data is retained, error reported.
        let report = collection.refresh(true).await.unwrap();
        assert!(
            collection.resolve("http-prov:s1").is_ok(),
            "failed source refresh retains prior valid data"
        );
        assert!(report.find("http-catalog").unwrap().outcome.is_failure());
    }

    #[tokio::test]
    async fn prepared_call_keeps_binding_across_refresh_replacement() {
        let mut collection = Models::new();
        let mut bound = RefreshProvider::new("dyn", vec![model_info("m1", "M1")]);
        bound.events = Mutex::new(Some(
            text_response("bound response")
                .into_iter()
                .map(Ok)
                .collect(),
        ));
        collection
            .register_route(
                Box::new(bound),
                Arc::new(StaticAuthResolver::new(
                    AuthScheme::ApiKey,
                    secrecy::SecretString::from("sk-bound"),
                )),
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .expect("register dispatchable route");

        // Prepare against the current catalog, then replace it wholesale.
        let prepared = collection
            .prepare_call("dyn:m1", minimal_request("dyn:m1"))
            .await
            .expect("prepare binds while the model exists");
        collection
            .register_route(
                Box::new(
                    RefreshProvider::new("dyn", vec![]).with_refresh(vec![model_info("m2", "M2")]),
                ),
                Arc::new(StaticAuthResolver::new(
                    AuthScheme::ApiKey,
                    secrecy::SecretString::from("sk-bound"),
                )),
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .unwrap();
        collection.refresh(true).await.unwrap();
        assert!(
            collection.resolve("dyn:m1").is_err(),
            "refresh replaced the catalog wholesale"
        );

        // The already-prepared call keeps its old binding and still drains.
        let attempt = prepared.start_attempt().expect("old binding survives");
        let message = drain_to_completion(attempt)
            .await
            .expect("prepared call completes on the frozen binding");
        assert_eq!(assistant_text(&message), "bound response");
    }
}

// ---------------------------------------------------------------------------
// Builtin provider factory layer (task 20.34)
// ---------------------------------------------------------------------------

use opi_ai::catalog::{Catalog, CatalogOperation};
use opi_ai::provider_collection::{BuiltinEnvAuthResolver, BuiltinOverrides, register_builtin};
use opi_ai::providers::{BuiltinEndpoint, builtin_profile, builtin_provider_profiles};

/// The exact canonical builtin provider set, sorted (pi `builtinProviders`).
const ALL_42_CANONICAL: [&str; 42] = [
    "amazon-bedrock",
    "ant-ling",
    "anthropic",
    "azure",
    "baseten",
    "cerebras",
    "cloudflare-ai-gateway",
    "cloudflare-workers-ai",
    "deepseek",
    "fireworks",
    "github-copilot",
    "google",
    "google-vertex",
    "groq",
    "huggingface",
    "kimi-coding",
    "meta",
    "minimax",
    "minimax-cn",
    "mistral",
    "moonshotai",
    "moonshotai-cn",
    "nvidia",
    "openai",
    "openai-codex",
    "opencode",
    "opencode-go",
    "openrouter",
    "qwen-token-plan",
    "qwen-token-plan-cn",
    "qwen-token-plan-individual",
    "radius",
    "together",
    "typesafe",
    "vercel-ai-gateway",
    "xai",
    "xiaomi",
    "xiaomi-token-plan-ams",
    "xiaomi-token-plan-cn",
    "xiaomi-token-plan-sgp",
    "zai",
    "zai-coding-cn",
];

/// Injected auth context answering from a fixed map: no process environment.
struct MapAuthContext(std::collections::BTreeMap<&'static str, &'static str>);

impl opi_ai::auth::AuthContext for MapAuthContext {
    fn env(&self, name: &str) -> Result<Option<String>, opi_ai::auth::AuthContextError> {
        Ok(self.0.get(name).map(|value| (*value).to_owned()))
    }
    fn file_exists(&self, _path: &str) -> Result<bool, opi_ai::auth::AuthContextError> {
        Ok(false)
    }
}

fn map_context(
    pairs: &[(&'static str, &'static str)],
) -> std::sync::Arc<dyn opi_ai::auth::AuthContext> {
    std::sync::Arc::new(MapAuthContext(pairs.iter().copied().collect()))
}

fn first_catalog_chat_model(provider: &str) -> String {
    let catalog = Catalog::builtin().expect("builtin catalog loads");
    catalog
        .models(provider, CatalogOperation::Chat)
        .expect("provider has chat entries")
        .next()
        .expect("non-empty chat catalog")
        .id
        .clone()
}

#[test]
fn builtin_profiles_cover_exactly_the_42_canonical_providers() {
    let profiles = builtin_provider_profiles();
    assert_eq!(profiles.len(), 42, "the pinned capability inventory count");
    let ids: Vec<&str> = profiles.iter().map(|profile| profile.id).collect();
    assert_eq!(ids, ALL_42_CANONICAL.to_vec(), "table is sorted canonical");
    // No permanent legacy aliases.
    for legacy in ["gemini", "vertex", "bedrock", "openai-responses"] {
        assert!(builtin_profile(legacy).is_none(), "{legacy} must not exist");
    }
}

#[test]
fn builtin_profiles_pin_source_qualified_metadata() {
    assert_eq!(
        builtin_profile("amazon-bedrock").unwrap().name,
        "Amazon Bedrock"
    );
    assert_eq!(builtin_profile("anthropic").unwrap().name, "Anthropic");
    assert_eq!(
        builtin_profile("google-vertex").unwrap().name,
        "Google Vertex AI"
    );
    let openai = builtin_profile("openai").expect("openai profile");
    assert_eq!(openai.name, "OpenAI");
    assert_eq!(openai.base_url, Some("https://api.openai.com/v1"));
    assert_eq!(openai.api_key_env, Some("OPENAI_API_KEY"));
    assert_eq!(openai.auth_scheme, AuthScheme::Bearer);
    assert!(openai.oauth, "ChatGPT subscription OAuth exists");
    assert!(
        openai.hides_classifiers_on_oauth,
        "ChatGPT tokens never reach the Decisions API"
    );

    let google = builtin_profile("google").expect("google profile");
    assert_eq!(
        google.base_url,
        Some("https://generativelanguage.googleapis.com/v1beta")
    );
    assert_eq!(google.api_key_env, Some("GEMINI_API_KEY"));
    assert_eq!(google.auth_scheme, AuthScheme::ApiKey);

    let azure = builtin_profile("azure").expect("azure profile");
    assert_eq!(azure.base_url, None, "the endpoint is deployment-derived");
    assert_eq!(azure.api_key_env, Some("AZURE_OPENAI_API_KEY"));
    assert!(matches!(azure.endpoint, BuiltinEndpoint::AzureDeployment));

    let vertex = builtin_profile("google-vertex").expect("google-vertex profile");
    assert_eq!(vertex.api_key_env, None, "Google Cloud credentials chain");
    assert!(matches!(vertex.endpoint, BuiltinEndpoint::VertexLocation));

    let bedrock = builtin_profile("amazon-bedrock").expect("amazon-bedrock profile");
    assert_eq!(bedrock.api_key_env, None, "AWS SigV4 credential chain");
    assert!(matches!(bedrock.endpoint, BuiltinEndpoint::BedrockRegion));

    let deepseek = builtin_profile("deepseek").expect("deepseek profile");
    assert_eq!(deepseek.base_url, Some("https://api.deepseek.com"));
    assert_eq!(deepseek.api_key_env, Some("DEEPSEEK_API_KEY"));
    assert_eq!(deepseek.auth_scheme, AuthScheme::Bearer);

    let copilot = builtin_profile("github-copilot").expect("github-copilot profile");
    assert_eq!(
        copilot.base_url,
        Some("https://api.individual.githubcopilot.com")
    );
    assert_eq!(copilot.api_key_env, Some("COPILOT_GITHUB_TOKEN"));
    assert!(copilot.oauth);

    let workers = builtin_profile("cloudflare-workers-ai").expect("cloudflare-workers-ai");
    assert_eq!(workers.api_key_env, Some("CLOUDFLARE_API_KEY"));

    let radius = builtin_profile("radius").expect("radius profile");
    assert_eq!(radius.api_key_env, Some("RADIUS_API_KEY"));
    assert!(matches!(radius.endpoint, BuiltinEndpoint::Dynamic));

    let typesafe = builtin_profile("typesafe").expect("typesafe profile");
    assert!(typesafe.classifier_only, "typesafe ships only classifiers");
}

#[tokio::test]
async fn builtin_env_resolver_reads_through_injected_context() {
    let resolver = BuiltinEnvAuthResolver::new(
        map_context(&[("DEEPSEEK_API_KEY", "sk-deepseek")]),
        "DEEPSEEK_API_KEY",
        AuthScheme::Bearer,
    );
    let resolved = resolver.resolve().await.expect("mapped key resolves");
    assert_eq!(resolved.scheme, AuthScheme::Bearer);
    assert_eq!(resolved.base_url, None);

    let missing =
        BuiltinEnvAuthResolver::new(map_context(&[]), "DEEPSEEK_API_KEY", AuthScheme::Bearer);
    assert!(matches!(
        missing.resolve().await,
        Err(ProviderError::CredentialNeeded { .. })
    ));
}

#[tokio::test]
async fn register_builtin_registers_the_canonical_collection() {
    let mut models = Models::new();
    let report = register_builtin(
        &mut models,
        map_context(&[]),
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .expect("builtin registration succeeds");

    // Vertex source selection is lazy and remains registered without ambient reads.
    for expected in [
        "google",
        "google-vertex",
        "amazon-bedrock",
        "anthropic",
        "openai",
        "deepseek",
        "openrouter",
    ] {
        assert!(
            report.registered.iter().any(|id| id == expected),
            "{expected} must register; got {:?}",
            report.registered
        );
    }
    // Endpoint and credentials are lazy; typesafe is classifier-only.
    assert!(report.registered.iter().any(|id| id == "azure"));
    assert!(!report.registered.iter().any(|id| id == "typesafe"));
    let skipped_ids: Vec<&str> = report.skipped.iter().map(|(id, _)| id.as_str()).collect();
    assert!(!skipped_ids.contains(&"azure"));
    assert!(!skipped_ids.contains(&"google-vertex"));
    assert!(skipped_ids.contains(&"typesafe"));

    // The bundled catalog backs canonical resolution.
    let google_model = first_catalog_chat_model("google");
    assert!(models.resolve(&format!("google:{google_model}")).is_ok());
    // No permanent legacy aliases in the collection.
    assert!(models.resolve("gemini:anything").is_err());
    let provider_ids = models.provider_ids();
    assert!(!provider_ids.contains(&"gemini"));
    assert!(!provider_ids.contains(&"bedrock"));
    assert!(!provider_ids.contains(&"vertex"));
}

#[tokio::test]
async fn azure_builtin_registers_with_configured_endpoint() {
    let mut models = Models::new();
    // Fixture host per the acceptance-source hermeticity contract.
    let overrides = BuiltinOverrides {
        azure_endpoint: Some("https://azure.example.com".into()),
        ..BuiltinOverrides::default()
    };
    let report = register_builtin(
        &mut models,
        map_context(&[("AZURE_OPENAI_API_KEY", "azure-key")]),
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
        &overrides,
    )
    .expect("builtin registration succeeds");
    assert!(report.registered.iter().any(|id| id == "azure"));
    let provider_ids = models.provider_ids();
    assert!(provider_ids.contains(&"azure"), "provider stays `azure`");
}

#[tokio::test]
async fn real_public_call_through_a_groq_completions_builtin_profile() {
    use wiremock::matchers;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    let sse = concat!(
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hi\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n",
        "data: [DONE]\n\n",
    );
    Mock::given(matchers::method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let mut models = Models::new();
    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert("groq".into(), server.uri());
    register_builtin(
        &mut models,
        map_context(&[("GROQ_API_KEY", "gsk-groq")]),
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
        &overrides,
    )
    .unwrap();

    // groq's first catalog model is a plain non-thinking completions model.
    let model = first_catalog_chat_model("groq");
    let spec = format!("groq:{model}");
    let prepared = models
        .prepare_call(&spec, minimal_request(&spec))
        .await
        .expect("canonical builtin dispatches");
    let message = drain_to_completion(prepared.start_attempt().unwrap())
        .await
        .expect("stream completes");
    assert_eq!(assistant_text(&message), "hi");

    let requests = server.received_requests().await.expect("captured");
    let request = &requests[0];
    assert!(
        request.url.path().ends_with("chat/completions"),
        "openai-completions wire, got {}",
        request.url.path()
    );
    assert_eq!(
        request.headers.get("authorization").unwrap(),
        "Bearer gsk-groq"
    );
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body["model"], model, "the wire model is the catalog id");
}

#[tokio::test]
async fn real_public_call_through_openai_defaults_to_responses_wire() {
    use wiremock::matchers;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    let sse = concat!(
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hi\"}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"r\",\"usage\":{\"input_tokens\":1,\"output_tokens\":1,\"total_tokens\":2}}}\n\n",
        "data: [DONE]\n\n",
    );
    Mock::given(matchers::method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let mut models = Models::new();
    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert("openai".into(), server.uri());
    register_builtin(
        &mut models,
        map_context(&[("OPENAI_API_KEY", "sk-openai")]),
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
        &overrides,
    )
    .unwrap();

    let model = first_catalog_chat_model("openai");
    let spec = format!("openai:{model}");
    let prepared = models
        .prepare_call(&spec, minimal_request(&spec))
        .await
        .expect("openai builtin dispatches");
    let message = drain_to_completion(prepared.start_attempt().unwrap())
        .await
        .expect("stream completes");
    assert_eq!(assistant_text(&message), "hi");

    let requests = server.received_requests().await.expect("captured");
    let request = &requests[0];
    assert!(
        request.url.path().ends_with("/responses"),
        "openai defaults to the Responses wire, got {}",
        request.url.path()
    );
    assert_eq!(
        request.headers.get("authorization").unwrap(),
        "Bearer sk-openai"
    );
}

#[tokio::test]
async fn catalog_tiered_pricing_flows_into_registered_models() {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[]),
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();

    // Find one tiered chat entry in the bundled catalog.
    let catalog = Catalog::builtin().unwrap();
    let mut tiered: Option<(String, String, serde_json::Value)> = None;
    for provider in catalog.providers() {
        for entry in catalog.models(provider, CatalogOperation::Chat).unwrap() {
            if entry.raw.get("cost").and_then(|c| c.get("tiers")).is_some() {
                tiered = Some((provider.to_owned(), entry.id.clone(), entry.raw.clone()));
                break;
            }
        }
        if tiered.is_some() {
            break;
        }
    }
    let Some((provider, id, raw)) = tiered else {
        panic!("bundled catalog has no tiered pricing entry");
    };
    let (_, model) = models
        .resolve(&format!("{provider}:{id}"))
        .expect("tiered model registers");
    let pricing = model.pricing.as_ref().expect("pricing attached");
    let cost = &raw["cost"];
    assert_eq!(
        pricing.base.input_cost_per_mtok,
        cost["input"].as_f64().unwrap()
    );
    let raw_tier = &cost["tiers"][0];
    let tier = &pricing.tiers[0];
    assert_eq!(
        tier.input_tokens_above,
        raw_tier["inputTokensAbove"].as_number().unwrap().clone()
    );
    assert_eq!(
        tier.pricing.input_cost_per_mtok,
        raw_tier["input"].as_f64().unwrap()
    );
}

#[tokio::test]
async fn builtin_registration_keeps_custom_record_upsert() {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[]),
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();

    // A custom registration replaces the builtin provider wholesale.
    models
        .register_route(
            text_mock("google", "custom wins"),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                secret(SECRET_VALUE),
            )) as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();
    let prepared = models
        .prepare_call("google:mock-model", minimal_request("google:mock-model"))
        .await
        .expect("custom record dispatches after builtin");
    let message = drain_to_completion(prepared.start_attempt().unwrap())
        .await
        .expect("custom mock completes");
    assert_eq!(assistant_text(&message), "custom wins");
}

#[tokio::test]
async fn classifier_and_image_builtins_register_from_the_catalog() {
    let mut models = Models::new();
    let report = register_builtin(
        &mut models,
        map_context(&[
            ("TYPESAFE_API_KEY", "ts-key"),
            ("OPENAI_API_KEY", "sk-openai"),
            ("CLOUDFLARE_API_KEY", "cf-key"),
        ]),
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    assert!(
        report
            .classifier_apis
            .iter()
            .any(|api| api == "typesafe-system-one")
    );
    assert!(
        report
            .classifier_apis
            .iter()
            .any(|api| api == "openai-decisions")
    );
    assert!(
        report
            .image_apis
            .iter()
            .any(|api| api == "openrouter-images")
    );

    let typesafe_record = models
        .classifier_model("typesafe", "jev-latest")
        .expect("catalog classifier record registers");
    assert_eq!(typesafe_record.provider_id, "typesafe");
    let catalog = Catalog::builtin().unwrap();
    let openrouter_image = catalog
        .models("openrouter", CatalogOperation::Image)
        .unwrap()
        .next()
        .unwrap()
        .id
        .clone();
    let image = models
        .image_model("openrouter", &openrouter_image)
        .expect("catalog image record registers");
    assert_eq!(image.provider_id, "openrouter");
}

#[tokio::test]
async fn classifier_same_api_keeps_provider_auth_bindings() {
    use opi_ai::api::system_one::{TypesafeSystemOneApi, default_typesafe_classifier_record};
    use opi_ai::classifier::{ClassifierCall, ClassifierContext, ClassifierStopReason};
    use std::sync::Arc;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, method},
    };
    let mut models = Models::new();
    let mut records = Vec::new();
    let mut servers = Vec::new();
    for (provider, key) in [("provider-a", "fixture-a"), ("provider-b", "fixture-b")] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(header("authorization", format!("Bearer {key}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"answers":{}})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let mut record = default_typesafe_classifier_record();
        record.provider_id = provider.into();
        record.base_url = server.uri();
        models
            .register_classifier_api(
                Arc::new(TypesafeSystemOneApi::new(
                    opi_ai::http::HttpClient::new(),
                    record.clone(),
                )),
                Arc::new(StaticAuthResolver::new(
                    AuthScheme::Bearer,
                    secrecy::SecretString::from(key),
                )),
                AuthProvenanceSource::Static,
            )
            .unwrap();
        records.push(record);
        servers.push(server);
    }
    for record in records {
        assert!(
            models
                .classifier_model(&record.provider_id, &record.id)
                .is_ok()
        );
        let result = models
            .classify(
                record,
                ClassifierContext::default(),
                ClassifierCall::default(),
            )
            .await;
        assert_eq!(
            result.stop_reason,
            ClassifierStopReason::Stop,
            "{:?}",
            result.error
        );
    }
    for server in servers {
        server.verify().await;
    }
}

#[tokio::test]
async fn classifier_model_modality_is_rejected_before_authentication() {
    use opi_ai::api::decisions::{OpenAiDecisionsApi, default_decisions_classifier_record};
    use opi_ai::classifier::{
        ClassifierCall, ClassifierContext, ClassifierImage, ClassifierInputModality,
        ClassifierStopReason,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    struct Probe(Arc<AtomicUsize>);
    impl AuthResolver for Probe {
        fn resolve<'a>(
            &'a self,
        ) -> opi_ai::credential::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Box::pin(async {
                Err(ProviderError::CredentialNeeded {
                    provider_id: "fixture".into(),
                })
            })
        }
    }
    let count = Arc::new(AtomicUsize::new(0));
    let mut record = default_decisions_classifier_record();
    record.input = vec![ClassifierInputModality::Text];
    let mut models = Models::new();
    models
        .register_classifier_api(
            Arc::new(OpenAiDecisionsApi::new(
                opi_ai::http::HttpClient::new(),
                record.clone(),
            )),
            Arc::new(Probe(count.clone())),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let result = models
        .classify(
            record,
            ClassifierContext {
                images: Some(vec![ClassifierImage {
                    mime_type: "image/png".into(),
                    data: "eA==".into(),
                }]),
                ..Default::default()
            },
            ClassifierCall::default(),
        )
        .await;
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(matches!(
        result.error,
        Some(ProviderError::UnsupportedCapability(_))
    ));
}

#[test]
fn builtin_full_record_keeps_pinned_model_compatibility() {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[]),
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    let model = models.get_model("opencode", "big-pickle").unwrap();
    let opi_ai::model_info::WireCompat::OpenAiCompletions(compat) = model.compat else {
        panic!("chat wire")
    };
    assert_eq!(compat.max_tokens_field.as_deref(), Some("max_tokens"));
    assert_eq!(
        compat.instruction_role,
        Some(opi_ai::model_info::ChatInstructionRole::System)
    );
}

#[tokio::test]
async fn builtin_stored_key_wins_and_store_replacement_only_changes_future_calls() {
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    let server = MockServer::start().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(200).set_body_string(
        "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n"
    )).expect(2).mount(&server).await;
    let store = |key: &'static str| {
        Arc::new(FakeStore {
            entries: Mutex::new(HashMap::from([(
                "groq".into(),
                Credential::ApiKey(secret(key).into()),
            )])),
        }) as Arc<dyn CredentialStore>
    };
    let mut models = Models::new();
    models.set_credential_store(store("stored-first"));
    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert("groq".into(), server.uri());
    register_builtin(
        &mut models,
        map_context(&[("GROQ_API_KEY", "ambient-unused")]),
        Arc::new(opi_ai::http::HttpClient::new()),
        &overrides,
    )
    .unwrap();
    let id = first_catalog_chat_model("groq");
    let record = opi_ai::ChatModelRecord::new("groq", models.get_model("groq", &id).unwrap());
    let request = minimal_request(&format!("groq:{id}"));
    let frozen = models.complete_simple(record.clone(), request.clone(), Default::default());
    assert!(server.received_requests().await.unwrap().is_empty());
    models.set_credential_store(store("stored-second"));
    frozen.await.expect("old call keeps its source binding");
    models
        .complete_simple(record, request, Default::default())
        .await
        .unwrap();
    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent[0].headers["authorization"], "Bearer stored-first");
    assert_eq!(sent[1].headers["authorization"], "Bearer stored-second");
}

#[test]
fn builtin_all_operation_records_are_available_for_selection() {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[]),
        Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    let catalog = Catalog::builtin().unwrap();
    let mut missing = Vec::new();
    for provider in catalog.providers() {
        for operation in [CatalogOperation::Classifier, CatalogOperation::Image] {
            for entry in catalog.models(provider, operation).unwrap() {
                let found = match operation {
                    CatalogOperation::Classifier => {
                        models.classifier_model(provider, &entry.id).is_ok()
                    }
                    CatalogOperation::Image => models.image_model(provider, &entry.id).is_ok(),
                    CatalogOperation::Chat => unreachable!(),
                };
                if !found {
                    missing.push(format!("{provider}:{}:{operation:?}", entry.id));
                }
            }
        }
    }
    assert!(missing.is_empty(), "missing operation records: {missing:?}");
}

fn node5_reply(wire: WireApi) -> &'static str {
    match wire {
        WireApi::OpenAiCompletions | WireApi::MistralConversations => concat!(
            "data: {\"id\":\"x\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hi\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"x\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n",
            "data: [DONE]\n\n"
        ),
        WireApi::OpenAiResponses
        | WireApi::AzureOpenAiResponses
        | WireApi::OpenAiCodexResponses => concat!(
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hi\"}\n\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"r\",\"usage\":{\"input_tokens\":1,\"output_tokens\":1,\"total_tokens\":2}}}\n\n"
        ),
        WireApi::AnthropicMessages => concat!(
            "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"model\":\"fixture\",\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\n",
            "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hi\"}}\n\n",
            "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
            "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":1}}\n\n",
            "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n"
        ),
        WireApi::PiMessages => concat!(
            "data: {\"type\":\"start\"}\n\n",
            "data: {\"type\":\"text_start\",\"contentIndex\":0}\n\n",
            "data: {\"type\":\"text_delta\",\"contentIndex\":0,\"delta\":\"hi\"}\n\n",
            "data: {\"type\":\"text_end\",\"contentIndex\":0,\"content\":\"hi\"}\n\n",
            "data: {\"type\":\"done\",\"reason\":\"stop\",\"usage\":{\"input\":1,\"output\":1,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":2}}\n\n"
        ),
        WireApi::GoogleGenerativeAi => {
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"hi\"}],\"role\":\"model\"},\"finishReason\":\"STOP\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":1,\"candidatesTokenCount\":1,\"totalTokenCount\":2}}\n\n"
        }
        _ => panic!("fixture missing for {wire}"),
    }
}

#[tokio::test]
async fn builtin_mapped_calls_use_owned_custom_records_and_header_precedence() {
    use std::sync::Arc;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    for (provider, env, wire) in [
        ("groq", "GROQ_API_KEY", WireApi::OpenAiCompletions),
        ("openai", "OPENAI_API_KEY", WireApi::OpenAiResponses),
        ("anthropic", "ANTHROPIC_API_KEY", WireApi::AnthropicMessages),
        ("azure", "AZURE_OPENAI_API_KEY", WireApi::OpenAiCompletions),
        (
            "azure",
            "AZURE_OPENAI_API_KEY",
            WireApi::AzureOpenAiResponses,
        ),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(node5_reply(wire)),
            )
            .expect(1)
            .mount(&server)
            .await;
        let mut models = Models::new();
        register_builtin(
            &mut models,
            map_context(&[(env, "fixture-key")]),
            Arc::new(opi_ai::http::HttpClient::new()),
            &BuiltinOverrides::default(),
        )
        .unwrap();
        let mut info = ModelInfo::new(
            "custom-owned",
            "Custom owned",
            wire,
            ModelCapabilities::new(8192, 129),
        )
        .with_base_url(format!("{}/owned", server.uri()));
        info.headers = vec![
            ("x-model".into(), Some("record".into())),
            ("x-override".into(), Some("record".into())),
            ("x-absent".into(), None),
        ];
        info.sampling_params
            .insert("top_p".into(), serde_json::json!(0.34));
        let mut request = minimal_request("ignored-request-model");
        request.extra_headers = vec![("x-override".into(), "request".into())];
        let output = models
            .complete_simple(
                opi_ai::ChatModelRecord::new(provider, info.clone()),
                request,
                Default::default(),
            )
            .await
            .unwrap_or_else(|error| panic!("{provider}/{wire}: {error:?}"));
        assert_eq!(assistant_text(&output), "hi");
        assert_eq!(output.provider, provider);
        assert_eq!(output.model, "custom-owned");
        let sent = server.received_requests().await.unwrap();
        let sent = &sent[0];
        assert!(
            sent.url.path().starts_with("/owned/"),
            "{provider}: {}",
            sent.url
        );
        assert_eq!(sent.headers["x-model"], "record");
        assert_eq!(sent.headers["x-override"], "request");
        assert!(!sent.headers.contains_key("x-absent"));
        let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
        assert_eq!(body["model"], "custom-owned");
        if wire == WireApi::AnthropicMessages {
            assert!(body.get("top_p").is_none());
        } else {
            assert_eq!(body["top_p"], serde_json::json!(0.34), "{provider}/{wire}");
        }
        info.headers
            .push(("authorization".into(), Some("forbidden".into())));
        assert!(
            models
                .complete_simple(
                    opi_ai::ChatModelRecord::new(provider, info),
                    minimal_request("ignored"),
                    Default::default()
                )
                .await
                .is_err()
        );
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn builtin_gateway_all_three_wires_use_only_source_auth_header() {
    use std::sync::Arc;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    for wire in [
        WireApi::OpenAiCompletions,
        WireApi::OpenAiResponses,
        WireApi::AnthropicMessages,
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(node5_reply(wire)),
            )
            .expect(1)
            .mount(&server)
            .await;
        let mut models = Models::new();
        register_builtin(
            &mut models,
            map_context(&[
                ("CLOUDFLARE_API_KEY", "gateway-fixture"),
                ("CLOUDFLARE_ACCOUNT_ID", "account"),
                ("CLOUDFLARE_GATEWAY_ID", "gateway"),
            ]),
            Arc::new(opi_ai::http::HttpClient::new()),
            &BuiltinOverrides::default(),
        )
        .unwrap();
        let info = ModelInfo::new(
            "custom-gateway",
            "Gateway",
            wire,
            ModelCapabilities::new(8192, 128),
        )
        .with_base_url(format!("{}/selected", server.uri()));
        let output = models
            .complete_simple(
                opi_ai::ChatModelRecord::new("cloudflare-ai-gateway", info),
                minimal_request("ignored"),
                Default::default(),
            )
            .await
            .unwrap();
        assert_eq!(assistant_text(&output), "hi");
        let sent = server.received_requests().await.unwrap();
        assert_eq!(
            sent[0].headers["cf-aig-authorization"],
            "Bearer gateway-fixture"
        );
        assert!(!sent[0].headers.contains_key("authorization"));
        assert!(!sent[0].headers.contains_key("x-api-key"));
        assert!(sent[0].url.path().starts_with("/selected/"));
    }
}

#[tokio::test]
async fn builtin_source_profiles_reach_their_registered_wires() {
    use std::collections::BTreeSet;
    use std::sync::Arc;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    let catalog = opi_ai::catalog::Catalog::builtin().unwrap();
    let mut checked = BTreeSet::new();
    for profile in opi_ai::providers::builtin_provider_profiles() {
        // Native crypto/ADC transports and classifier-only operation have their
        // own real builtin fixtures; this corpus covers every other profile/wire.
        if ["amazon-bedrock", "google", "google-vertex", "typesafe"].contains(&profile.id) {
            continue;
        }
        let mut wires = BTreeSet::new();
        for entry in catalog
            .models(profile.id, opi_ai::catalog::CatalogOperation::Chat)
            .unwrap()
        {
            if !wires.insert(entry.api.clone()) {
                continue;
            }
            let server = MockServer::start().await;
            let source_path = if profile.id == "azure" {
                "/openai/v1".to_owned()
            } else {
                url::Url::parse(&entry.base_url)
                    .unwrap()
                    .path()
                    .trim_end_matches('/')
                    .to_owned()
            };
            let controlled_base = format!("{}{source_path}", server.uri());
            let mut overrides = BuiltinOverrides::default();
            overrides
                .base_urls
                .insert(profile.id.into(), controlled_base.clone());
            if profile.id == "azure" {
                overrides.azure_endpoint = Some(controlled_base);
            }
            let credential = if profile.id == "openai-codex" {
                Credential::OAuthToken {
                    access: "fixture-codex".into(),
                    refresh: "unused".into(),
                    expires_at: Some(time::OffsetDateTime::now_utc() + time::Duration::hours(1)),
                    base_url: None,
                    account_id: Some("fixture-account".into()),
                    client_id: None,
                    enterprise_domain: None,
                    available_model_ids: None,
                    scopes: None,
                }
            } else {
                Credential::ApiKey("fixture-source-key".into())
            };
            let mut models = Models::new();
            models.set_credential_store(Arc::new(FakeStore {
                entries: Mutex::new(HashMap::from([(profile.id.into(), credential)])),
            }));
            register_builtin(
                &mut models,
                map_context(&[
                    ("CLOUDFLARE_ACCOUNT_ID", "fixture-account"),
                    ("CLOUDFLARE_GATEWAY_ID", "fixture-gateway"),
                ]),
                Arc::new(opi_ai::http::HttpClient::new()),
                &overrides,
            )
            .unwrap();
            let info = models.get_model(profile.id, &entry.id).unwrap();
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(node5_reply(info.wire_api)),
                )
                .expect(1)
                .mount(&server)
                .await;
            let mut request = minimal_request("owned model selects routing");
            request.transport = opi_ai::provider::TransportPreference::Sse;
            request.max_tokens = Some(32);
            request.session_id = Some("profile-session".into());
            request
                .extra_headers
                .push(("x-call-proof".into(), "profile-wire".into()));
            let output = models
                .complete_simple(
                    opi_ai::ChatModelRecord::new(profile.id, info.clone()),
                    request,
                    Default::default(),
                )
                .await
                .unwrap_or_else(|error| panic!("{}/{}: {error:?}", profile.id, entry.api));
            assert_eq!(output.provider, profile.id, "{}/{}", profile.id, entry.api);
            assert_eq!(output.model, entry.id);
            assert_eq!(assistant_text(&output), "hi");
            let sent = server.received_requests().await.unwrap();
            assert_eq!(sent.len(), 1, "{}/{}", profile.id, entry.api);
            let suffix = match info.wire_api {
                WireApi::OpenAiCompletions => Some("/chat/completions"),
                WireApi::MistralConversations => Some("/v1/chat/completions"),
                WireApi::OpenAiResponses | WireApi::AzureOpenAiResponses => Some("/responses"),
                WireApi::OpenAiCodexResponses => Some("/codex/responses"),
                WireApi::AnthropicMessages => Some("/v1/messages"),
                WireApi::PiMessages => Some("/messages"),
                _ => None,
            };
            if let Some(suffix) = suffix {
                assert_eq!(
                    sent[0].url.path(),
                    format!("{source_path}{suffix}"),
                    "{}/{}",
                    profile.id,
                    entry.api
                );
            }

            assert_eq!(sent[0].headers["x-call-proof"], "profile-wire");
            let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
            if info.wire_api == WireApi::GoogleGenerativeAi {
                assert!(sent[0].url.path().contains(&entry.id));
            } else {
                assert_eq!(body["model"], entry.id);
            }
            if ["opencode", "opencode-go"].contains(&profile.id) {
                assert_eq!(sent[0].headers["x-opencode-session"], "profile-session");
            }
            let auth_header = if profile.id == "cloudflare-ai-gateway" {
                "cf-aig-authorization"
            } else if info.wire_api == WireApi::AzureOpenAiResponses {
                "api-key"
            } else if info.wire_api == WireApi::GoogleGenerativeAi {
                "x-goog-api-key"
            } else if info.wire_api == WireApi::AnthropicMessages && profile.id != "github-copilot"
            {
                "x-api-key"
            } else {
                "authorization"
            };
            assert!(
                sent[0].headers.contains_key(auth_header),
                "{}/{} lacks {auth_header}",
                profile.id,
                entry.api
            );
            checked.insert(profile.id);
        }
    }
    assert_eq!(checked.len(), 38);
}

#[tokio::test]
async fn builtin_classifier_profiles_send_the_selected_record_with_stored_auth() {
    use opi_ai::classifier::{ClassifierCall, ClassifierContext, ClassifierStopReason};
    use std::sync::Arc;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    let catalog = Catalog::builtin().unwrap();
    for provider in [
        "typesafe",
        "openrouter",
        "opencode",
        "vercel-ai-gateway",
        "cloudflare-workers-ai",
        "openai",
    ] {
        let server = MockServer::start().await;
        let response = match provider {
            "cloudflare-workers-ai" => {
                serde_json::json!({"success":true,"result":{"answers":{},"usage":{"input_tokens":2,"output_tokens":1}}})
            }
            "openai" => {
                serde_json::json!({"answers":[],"usage":{"input_tokens":2,"output_tokens":1}})
            }
            _ => serde_json::json!({"answers":{},"usage":{"input_tokens":2,"output_tokens":1}}),
        };
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
        let store = Arc::new(FakeStore {
            entries: Mutex::new(HashMap::from([(
                provider.into(),
                Credential::ApiKey("stored-classifier".into()),
            )])),
        });
        let mut models = Models::new();
        models.set_credential_store(store);
        register_builtin(
            &mut models,
            map_context(&[("CLOUDFLARE_ACCOUNT_ID", "fixture")]),
            Arc::new(opi_ai::http::HttpClient::new()),
            &BuiltinOverrides::default(),
        )
        .unwrap();
        let entry = catalog
            .models(provider, CatalogOperation::Classifier)
            .unwrap()
            .last()
            .unwrap();
        let mut record = models.classifier_model(provider, &entry.id).unwrap();
        record.base_url = format!("{}/custom-classifier", server.uri());
        let result = models
            .classify(
                record,
                ClassifierContext::default(),
                ClassifierCall::default(),
            )
            .await;
        assert_eq!(
            result.stop_reason,
            ClassifierStopReason::Stop,
            "{provider}: {:?}",
            result.error
        );
        assert_eq!(result.provider, provider);
        assert_eq!(result.model, entry.id);
        assert_eq!(result.usage.unwrap().input_tokens, 2);
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent[0].headers["authorization"], "Bearer stored-classifier");
        assert!(sent[0].url.path().starts_with("/custom-classifier/"));
        assert!(
            models
                .get_auth(provider, Default::default())
                .await
                .unwrap()
                .is_some(),
            "{provider} management auth must expose its configured source"
        );
    }
}

#[test]
fn builtin_construction_and_unpolled_calls_do_not_read_sources() {
    struct NoSources;
    impl opi_ai::auth::AuthContext for NoSources {
        fn env(&self, _: &str) -> Result<Option<String>, opi_ai::auth::AuthContextError> {
            panic!("construction read environment")
        }
        fn file_exists(&self, _: &str) -> Result<bool, opi_ai::auth::AuthContextError> {
            panic!("construction inspected a file")
        }
    }
    let mut models = Models::new();
    let report = register_builtin(
        &mut models,
        std::sync::Arc::new(NoSources),
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    assert_eq!(report.registered.len(), 41);
    let model = models
        .get_model("groq", &first_catalog_chat_model("groq"))
        .unwrap();
    let call = models.complete_simple(
        opi_ai::ChatModelRecord::new("groq", model),
        minimal_request("ignored"),
        Default::default(),
    );
    drop(call);
}

#[tokio::test]
async fn builtin_openai_oauth_excludes_decisions_without_falling_back_to_env_key() {
    use opi_ai::classifier::{ClassifierCall, ClassifierContext, ClassifierStopReason};
    use std::sync::Arc;
    let credential = Credential::OAuthToken {
        access: "fixture-oauth".into(),
        refresh: "unused".into(),
        expires_at: Some(time::OffsetDateTime::now_utc() + time::Duration::hours(1)),
        base_url: None,
        account_id: None,
        client_id: None,
        enterprise_domain: None,
        available_model_ids: None,
        scopes: None,
    };
    let mut models = Models::new();
    models.set_credential_store(Arc::new(FakeStore {
        entries: Mutex::new(HashMap::from([("openai".into(), credential)])),
    }));
    register_builtin(
        &mut models,
        map_context(&[("OPENAI_API_KEY", "must-not-fallback")]),
        Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    assert_eq!(
        models.check_auth("openai").await.unwrap().unwrap().kind,
        AuthCheckKind::OAuth
    );
    assert!(
        models
            .get_available_classifiers(Some("openai"))
            .await
            .unwrap()
            .is_empty()
    );
    let entry = Catalog::builtin()
        .unwrap()
        .models("openai", CatalogOperation::Classifier)
        .unwrap()
        .next()
        .unwrap();
    let mut record = models.classifier_model("openai", &entry.id).unwrap();
    let server = wiremock::MockServer::start().await;
    record.base_url = server.uri();
    let result = models
        .classify(
            record,
            ClassifierContext::default(),
            ClassifierCall::default(),
        )
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn builtin_nonstream_operations_reuse_the_injected_redirect_policy() {
    use opi_ai::classifier::{ClassifierCall, ClassifierContext, ClassifierStopReason};
    use opi_ai::images::{ImagesCall, ImagesContext, ImagesStopReason};
    use std::sync::Arc;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    let origin = MockServer::start().await;
    let redirected = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(307)
                .insert_header("location", format!("{}/destination", redirected.uri())),
        )
        .mount(&origin)
        .await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"answers":{},"choices":[]})),
        )
        .mount(&redirected)
        .await;
    let client = opi_ai::http::HttpClientBuilder::new()
        .redirects(opi_ai::http::RedirectPolicy::None)
        .build()
        .unwrap();
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[
            ("TYPESAFE_API_KEY", "fixture"),
            ("OPENROUTER_API_KEY", "fixture"),
        ]),
        Arc::new(client),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    let mut classifier = models.classifier_model("typesafe", "jev-latest").unwrap();
    classifier.base_url = origin.uri();
    let classified = models
        .classify(
            classifier,
            ClassifierContext::default(),
            ClassifierCall::default(),
        )
        .await;
    assert_eq!(classified.stop_reason, ClassifierStopReason::Error);
    let entry = Catalog::builtin()
        .unwrap()
        .models("openrouter", CatalogOperation::Image)
        .unwrap()
        .last()
        .unwrap();
    let mut record = models.image_model("openrouter", &entry.id).unwrap();
    record.base_url = origin.uri();
    let generated = models
        .generate_images(record, ImagesContext::default(), ImagesCall::default())
        .await;
    assert_eq!(generated.stop_reason, ImagesStopReason::Error);
    assert_eq!(origin.received_requests().await.unwrap().len(), 2);
    assert!(
        redirected.received_requests().await.unwrap().is_empty(),
        "source policy forbids redirect transport sends"
    );
}

#[tokio::test]
async fn builtin_oauth_flow_replacement_changes_future_calls_only() {
    use opi_ai::auth::{LoginPresenter, OAuthCredential, OAuthProvider};
    use std::sync::Arc;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    struct Flow(&'static str);
    impl OAuthProvider for Flow {
        fn id(&self) -> &str {
            "openai"
        }
        fn to_auth(&self, _: &OAuthCredential) -> ResolvedAuth {
            ResolvedAuth {
                env: Default::default(),
                scheme: AuthScheme::Bearer,
                secret: self.0.into(),
                base_url: None,
                account_id: None,
                provenance: opi_ai::auth::AuthProvenance {
                    source: AuthProvenanceSource::OAuth {
                        kind: "openai".into(),
                    },
                    fallback: opi_ai::auth::AuthFallback::NotAttempted,
                },
            }
        }
        fn login<'a>(
            &'a self,
            _: &'a dyn LoginPresenter,
        ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
            panic!("login must not start")
        }
        fn refresh<'a>(
            &'a self,
            _: &'a OAuthCredential,
        ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
            panic!("fresh fixture must not refresh")
        }
    }
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(node5_reply(WireApi::OpenAiResponses)),
        )
        .expect(2)
        .mount(&server)
        .await;
    let credential = Credential::OAuthToken {
        access: "acknowledged".into(),
        refresh: "unused".into(),
        expires_at: Some(time::OffsetDateTime::now_utc() + time::Duration::hours(1)),
        base_url: None,
        account_id: None,
        client_id: None,
        enterprise_domain: None,
        available_model_ids: None,
        scopes: None,
    };
    let mut models = Models::new();
    models.set_credential_store(Arc::new(FakeStore {
        entries: Mutex::new(HashMap::from([("openai".into(), credential)])),
    }));
    models.register_oauth_flow("openai", Arc::new(Flow("old-flow")));
    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert("openai".into(), server.uri());
    register_builtin(
        &mut models,
        map_context(&[]),
        Arc::new(opi_ai::http::HttpClient::new()),
        &overrides,
    )
    .unwrap();
    let info = models
        .get_model("openai", &first_catalog_chat_model("openai"))
        .unwrap();
    let record = opi_ai::ChatModelRecord::new("openai", info);
    let old = models.complete_simple(
        record.clone(),
        minimal_request("ignored"),
        Default::default(),
    );
    models.register_oauth_flow("openai", Arc::new(Flow("new-flow")));
    old.await.unwrap();
    models
        .complete_simple(record, minimal_request("ignored"), Default::default())
        .await
        .unwrap();
    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent[0].headers["authorization"], "Bearer old-flow");
    assert_eq!(sent[1].headers["authorization"], "Bearer new-flow");
}

#[tokio::test]
async fn builtin_anthropic_header_owned_token_is_not_an_oauth_identity() {
    use std::sync::Arc;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(node5_reply(WireApi::AnthropicMessages)),
        )
        .expect(1)
        .mount(&server)
        .await;
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[
            ("ANTHROPIC_AUTH_TOKEN", "sk-ant-oat-header-owned-fixture"),
            ("ANTHROPIC_OAUTH_TOKEN", "must-not-use"),
            ("ANTHROPIC_API_KEY", "must-not-use"),
        ]),
        Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    let model = ModelInfo::new(
        "owned",
        "Owned",
        WireApi::AnthropicMessages,
        ModelCapabilities::new(2048, 128),
    )
    .with_base_url(server.uri());
    models
        .complete_simple(
            opi_ai::ChatModelRecord::new("anthropic", model),
            minimal_request("ignored"),
            Default::default(),
        )
        .await
        .unwrap();
    let sent = server.received_requests().await.unwrap();
    assert_eq!(
        sent[0].headers["authorization"],
        "Bearer sk-ant-oat-header-owned-fixture"
    );
    assert!(!sent[0].headers.contains_key("x-api-key"));
    assert!(!sent[0].headers.contains_key("anthropic-beta"));
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
    assert!(body.get("system").is_none());
}

#[tokio::test]
async fn builtin_azure_chat_and_responses_resolve_source_endpoint_and_deployment() {
    use std::sync::Arc;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    struct AzureEnv(String);
    impl opi_ai::auth::AuthContext for AzureEnv {
        fn env(&self, name: &str) -> Result<Option<String>, opi_ai::auth::AuthContextError> {
            Ok(match name {
                "AZURE_OPENAI_API_KEY" => Some("azure-fixture".into()),
                "AZURE_OPENAI_BASE_URL" => Some(self.0.clone()),
                "AZURE_OPENAI_API_VERSION" => Some("fixture-version".into()),
                "AZURE_OPENAI_DEPLOYMENT_NAME_MAP" => Some("catalog-id=actual-deployment".into()),
                _ => None,
            })
        }
        fn file_exists(&self, _: &str) -> Result<bool, opi_ai::auth::AuthContextError> {
            Ok(false)
        }
    }
    for wire in [WireApi::OpenAiCompletions, WireApi::AzureOpenAiResponses] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(node5_reply(wire)),
            )
            .expect(1)
            .mount(&server)
            .await;
        let unused = MockServer::start().await;
        let mut models = Models::new();
        register_builtin(
            &mut models,
            Arc::new(AzureEnv(format!("{}/openai/v1", server.uri()))),
            Arc::new(opi_ai::http::HttpClient::new()),
            &BuiltinOverrides::default(),
        )
        .unwrap();
        let model = ModelInfo::new(
            "catalog-id",
            "Catalog identity",
            wire,
            ModelCapabilities::new(2048, 128),
        )
        .with_base_url(unused.uri());
        let output = models
            .complete_simple(
                opi_ai::ChatModelRecord::new("azure", model),
                minimal_request("ignored"),
                Default::default(),
            )
            .await
            .unwrap();
        assert_eq!(output.provider, "azure");
        assert_eq!(output.model, "catalog-id");
        assert!(unused.received_requests().await.unwrap().is_empty());
        let sent = server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
        assert_eq!(body["model"], "actual-deployment");
        if wire == WireApi::OpenAiCompletions {
            assert_eq!(sent[0].url.path(), "/openai/v1/chat/completions");
            assert_eq!(sent[0].headers["authorization"], "Bearer azure-fixture");
        } else {
            assert_eq!(sent[0].url.path(), "/openai/v1/responses");
            assert!(
                sent[0]
                    .url
                    .query_pairs()
                    .any(|(key, value)| key == "api-version" && value == "fixture-version")
            );
            assert_eq!(sent[0].headers["api-key"], "azure-fixture");
        }
    }
}

#[tokio::test]
async fn builtin_simple_tool_choice_reaches_each_chat_wire() {
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    for (provider, env, wire) in [
        ("groq", "GROQ_API_KEY", WireApi::OpenAiCompletions),
        ("openai", "OPENAI_API_KEY", WireApi::OpenAiResponses),
        ("anthropic", "ANTHROPIC_API_KEY", WireApi::AnthropicMessages),
        ("azure", "AZURE_OPENAI_API_KEY", WireApi::OpenAiCompletions),
        (
            "azure",
            "AZURE_OPENAI_API_KEY",
            WireApi::AzureOpenAiResponses,
        ),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(node5_reply(wire)),
            )
            .expect(2)
            .mount(&server)
            .await;
        let mut models = Models::new();
        register_builtin(
            &mut models,
            map_context(&[(env, "fixture-key")]),
            Arc::new(opi_ai::http::HttpClient::new()),
            &BuiltinOverrides::default(),
        )
        .unwrap();
        let record = opi_ai::ChatModelRecord::new(
            provider,
            ModelInfo::new(
                "choice-fixture",
                "Choice fixture",
                wire,
                ModelCapabilities::new(8192, 64),
            )
            .with_base_url(server.uri()),
        );
        for choice in [
            opi_ai::SimpleToolChoice::Auto,
            opi_ai::SimpleToolChoice::None,
        ] {
            models
                .complete_simple(
                    record.clone(),
                    minimal_request("ignored"),
                    opi_ai::SimpleOptions {
                        tool_choice: Some(choice),
                        ..Default::default()
                    },
                )
                .await
                .unwrap_or_else(|error| panic!("{provider}/{wire}/{choice:?}: {error:?}"));
        }
        for (request, expected) in server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .zip(["auto", "none"])
        {
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            let choice = if wire == WireApi::AnthropicMessages {
                &body["tool_choice"]["type"]
            } else {
                &body["tool_choice"]
            };
            assert_eq!(choice, expected, "{provider}/{wire}");
        }
    }
}

#[tokio::test]
async fn builtin_azure_freezes_endpoint_deployment_and_payload_across_retries() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    struct ChangingSource {
        bases: [String; 2],
        reads: AtomicUsize,
        model: String,
    }
    impl opi_ai::auth::AuthContext for ChangingSource {
        fn env(&self, name: &str) -> Result<Option<String>, opi_ai::auth::AuthContextError> {
            Ok(match name {
                "AZURE_OPENAI_API_KEY" => Some("isolated-key".into()),
                "AZURE_OPENAI_BASE_URL" => {
                    Some(self.bases[self.reads.fetch_add(1, Ordering::SeqCst).min(1)].clone())
                }
                "AZURE_OPENAI_DEPLOYMENT_NAME_MAP" => Some(format!(
                    "{}=deployment-{}",
                    self.model,
                    self.reads.load(Ordering::SeqCst)
                )),
                _ => None,
            })
        }
        fn file_exists(&self, _: &str) -> Result<bool, opi_ai::auth::AuthContextError> {
            panic!("unselected source")
        }
    }
    for wire in [WireApi::OpenAiCompletions, WireApi::AzureOpenAiResponses] {
        for advanced in [false, true] {
            let first = MockServer::start().await;
            let second = MockServer::start().await;
            let sends = Arc::new(AtomicUsize::new(0));
            let count = sends.clone();
            Mock::given(method("POST"))
                .respond_with(move |_: &wiremock::Request| {
                    if count.fetch_add(1, Ordering::SeqCst) == 0 && advanced {
                        ResponseTemplate::new(429).insert_header("retry-after", "0")
                    } else {
                        ResponseTemplate::new(200)
                            .insert_header("content-type", "text/event-stream")
                            .set_body_string(node5_reply(wire))
                    }
                })
                .expect(if advanced { 2 } else { 1 })
                .mount(&first)
                .await;
            let catalog = Catalog::builtin().unwrap();
            let entry = catalog
                .models("azure", CatalogOperation::Chat)
                .unwrap()
                .find(|entry| entry.api == wire.to_string())
                .unwrap();
            let source = Arc::new(ChangingSource {
                bases: [first.uri(), second.uri()],
                reads: AtomicUsize::new(0),
                model: entry.id.clone(),
            });
            let mut models = Models::new();
            let overrides = BuiltinOverrides {
                azure_endpoint: Some(first.uri()),
                ..Default::default()
            };
            register_builtin(
                &mut models,
                source.clone(),
                Arc::new(opi_ai::http::HttpClient::new()),
                &overrides,
            )
            .unwrap();
            let spec = format!("azure:{}", entry.id);
            let mut request = minimal_request(&spec);
            let payloads = Arc::new(AtomicUsize::new(0));
            let calls = payloads.clone();
            request.callbacks =
                opi_ai::callbacks::RequestCallbacks::new().with_async_on_payload(move |_| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    async { Ok(None) }
                });
            if advanced {
                let prepared = models
                    .prepare_call_with_policy(
                        &spec,
                        request,
                        opi_ai::retry::AcquisitionPolicy {
                            max_retries: 1,
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();
                assert!(
                    opi_ai::ChatStream::new(prepared.start_attempt().unwrap())
                        .result()
                        .await
                        .is_err()
                );
                assert!(prepared.retry_admission().is_some());
                opi_ai::ChatStream::new(prepared.start_attempt().unwrap())
                    .result()
                    .await
                    .unwrap();
            } else {
                let model = models.get_model("azure", &entry.id).unwrap();
                models
                    .complete_simple(
                        opi_ai::ChatModelRecord::new("azure", model),
                        request,
                        Default::default(),
                    )
                    .await
                    .unwrap();
            }
            assert_eq!(source.reads.load(Ordering::SeqCst), 1, "{wire}/{advanced}");
            assert_eq!(payloads.load(Ordering::SeqCst), 1);
            assert!(second.received_requests().await.unwrap().is_empty());
            let sent = first.received_requests().await.unwrap();
            for request in &sent {
                let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
                assert_eq!(body["model"], "deployment-1");
            }
            if advanced {
                assert_eq!(sent[0].body, sent[1].body);
            }
        }
    }
}

#[tokio::test]
async fn builtin_profile_api_base_paths_reach_exact_endpoints() {
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    for (provider, env, wire, azure_base, suffix) in [
        (
            "groq",
            "GROQ_API_KEY",
            WireApi::OpenAiCompletions,
            false,
            "/chat/completions",
        ),
        (
            "openai",
            "OPENAI_API_KEY",
            WireApi::OpenAiResponses,
            false,
            "/responses",
        ),
        (
            "azure",
            "AZURE_OPENAI_API_KEY",
            WireApi::OpenAiCompletions,
            true,
            "/chat/completions",
        ),
        (
            "azure",
            "AZURE_OPENAI_API_KEY",
            WireApi::AzureOpenAiResponses,
            true,
            "/responses",
        ),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(node5_reply(wire)),
            )
            .expect(1)
            .mount(&server)
            .await;
        let mut models = Models::new();
        register_builtin(
            &mut models,
            map_context(&[(env, "fixture-key")]),
            Arc::new(opi_ai::http::HttpClient::new()),
            &BuiltinOverrides::default(),
        )
        .unwrap();
        let catalog = Catalog::builtin().unwrap();
        let entry = catalog
            .models(provider, CatalogOperation::Chat)
            .unwrap()
            .find(|entry| entry.api == wire.to_string())
            .unwrap();
        let mut model = models.get_model(provider, &entry.id).unwrap();
        let source_path = if azure_base {
            "/openai/v1".into()
        } else {
            url::Url::parse(model.base_url.as_deref().unwrap())
                .unwrap()
                .path()
                .trim_end_matches('/')
                .to_owned()
        };
        model.base_url = Some(format!("{}{source_path}", server.uri()));
        models
            .complete_simple(
                opi_ai::ChatModelRecord::new(provider, model),
                minimal_request("ignored"),
                Default::default(),
            )
            .await
            .unwrap();
        assert_eq!(
            server.received_requests().await.unwrap()[0].url.path(),
            format!("{source_path}{suffix}"),
            "{provider}/{wire}"
        );
    }
}
