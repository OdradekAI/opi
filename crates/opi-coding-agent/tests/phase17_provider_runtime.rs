//! Phase 17.5 provider-runtime acceptance (P17-A02 + PRV-001/002/004, OUT-001).
//!
//! These tests prove the dispatchable-provider-collection contract at the
//! `ProviderCollection` seam: route lookup and per-call auth preparation go
//! through the registered runtime collection (not a startup-selected provider
//! object), a missing / ambiguous / unauthenticated route or auth-preparation
//! failure yields a typed error with ZERO model dispatch and NO silent fallback
//! to another provider/model/wire/credential, and successive model calls route
//! to different registered providers through one collection without rebuilding
//! it.
//!
//! P17-A02 (the 17.5-owned acceptance scenario) is
//! [`phase17_route_and_auth_failures_do_not_dispatch_model_http`].

#[path = "common/phase17.rs"]
mod phase17;

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use opi_ai::auth::{
    AuthProvenanceSource, AuthResolver, AuthScheme, ResolvedAuth, StaticAuthResolver,
};
use opi_ai::credential::BoxAuthFuture;
use opi_ai::model_info::WireApi;
use opi_ai::provider::{EventStream, ModelInfo, Provider, ProviderError, Request};
use opi_ai::provider_collection::drain_to_completion;
use opi_ai::registry::ModelCapabilities;
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use opi_ai::test_support::base_assistant;
use opi_ai::{CompatMetadata, ProviderCollection};
use tokio_util::sync::CancellationToken;

/// A provider that counts how many times `stream_prepared` is invoked, so a
/// test can prove model dispatch did (or did not) occur. It advertises one model
/// and returns a single terminal `Done` event on dispatch.
struct CountingProvider {
    id: String,
    model: ModelInfo,
    dispatches: Arc<AtomicU32>,
}

impl CountingProvider {
    fn new(id: &str, model_id: &str, dispatches: Arc<AtomicU32>) -> Self {
        Self {
            id: id.to_owned(),
            model: model_info(model_id),
            dispatches,
        }
    }
}

impl Provider for CountingProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn models(&self) -> &[ModelInfo] {
        std::slice::from_ref(&self.model)
    }

    fn stream_prepared(&self, _request: Request, _auth: ResolvedAuth) -> EventStream {
        self.dispatches.fetch_add(1, Ordering::SeqCst);
        let message = base_assistant();
        Box::pin(futures_util::stream::once(async move {
            Ok(AssistantStreamEvent::Done {
                reason: StopReason::Stop,
                message,
            })
        }))
    }
}

/// An auth resolver that always fails with `CredentialNeeded`, for the
/// unauthenticated-route case.
struct FailingResolver {
    provider_id: String,
}

struct CountingResolver {
    calls: Arc<AtomicU32>,
}

impl AuthResolver for CountingResolver {
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            Ok(ResolvedAuth {
                scheme: AuthScheme::ApiKey,
                secret: secrecy::SecretString::from("active-route-key"),
                base_url: None,
                account_id: None,
                provenance: Default::default(),
            })
        })
    }
}

impl AuthResolver for FailingResolver {
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        let provider_id = self.provider_id.clone();
        Box::pin(async move { Err(ProviderError::CredentialNeeded { provider_id }) })
    }
}

fn model_info(id: &str) -> ModelInfo {
    ModelInfo::new(
        id,
        id,
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(8192, 4096).with_streaming(true),
    )
}

fn request_for(model_spec: &str) -> Request {
    Request {
        model: model_spec.to_owned(),
        system: None,
        messages: Vec::new(),
        tools: Vec::new(),
        max_tokens: None,
        temperature: None,
        thinking: opi_ai::provider::ThinkingConfig::default(),
        stop_sequences: Vec::new(),
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: Vec::new(),
        cache_retention: opi_ai::provider::CacheRetention::None,
        session_id: None,
    }
}

fn static_resolver() -> Arc<dyn AuthResolver> {
    Arc::new(StaticAuthResolver::new(
        AuthScheme::ApiKey,
        secrecy::SecretString::from("test-key"),
    ))
}

/// Register a dispatchable counting route and return its dispatch counter.
fn register_counting_route(
    collection: &mut ProviderCollection,
    provider_id: &str,
    model_id: &str,
    resolver: Arc<dyn AuthResolver>,
) -> Arc<AtomicU32> {
    let dispatches = Arc::new(AtomicU32::new(0));
    let provider = CountingProvider::new(provider_id, model_id, Arc::clone(&dispatches));
    collection
        .register_route(
            Box::new(provider),
            resolver,
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .expect("register_route must succeed for a fresh provider id");
    dispatches
}

/// P17-A02 (owned acceptance): an unknown, ambiguous, or unauthenticated
/// provider:model returns the owning typed failure before model dispatch, with
/// no silent provider/credential fallback. Every dispatch counter stays at 0.
#[tokio::test]
async fn phase17_route_and_auth_failures_do_not_dispatch_model_http() {
    let mut collection = ProviderCollection::new();
    let alpha_dispatches =
        register_counting_route(&mut collection, "alpha", "a1", static_resolver());

    // Unknown provider: "beta:b1" has no registered route.
    let err = collection
        .prepare_call("beta:b1", request_for("beta:b1"))
        .await
        .expect_err("unknown provider must fail before dispatch");
    assert!(
        matches!(err, opi_ai::CollectionError::Registry(_)),
        "unknown provider must be a registry typed error, got {err:?}"
    );

    // Unknown model: provider "alpha" exists but model "zzz" does not.
    let err = collection
        .prepare_call("alpha:zzz", request_for("alpha:zzz"))
        .await
        .expect_err("unknown model must fail before dispatch");
    assert!(
        matches!(err, opi_ai::CollectionError::Registry(_)),
        "unknown model must be a registry typed error, got {err:?}"
    );

    // Ambiguous bare model: no provider separator, so it cannot identify one
    // canonical route (Phase 17 adds no alias registry).
    let err = collection
        .prepare_call("a1", request_for("a1"))
        .await
        .expect_err("ambiguous bare model must fail before dispatch");
    assert!(
        matches!(err, opi_ai::CollectionError::Registry(_)),
        "bare model must be a registry typed error, got {err:?}"
    );

    // Unauthenticated route: "beta:b1" is registered but its resolver fails with
    // CredentialNeeded. The collection must NOT silently fall back to "alpha".
    let beta_dispatches = register_counting_route(
        &mut collection,
        "beta",
        "b1",
        Arc::new(FailingResolver {
            provider_id: "beta".into(),
        }),
    );
    let err = collection
        .prepare_call("beta:b1", request_for("beta:b1"))
        .await
        .expect_err("unauthenticated route must fail before dispatch");
    assert!(
        matches!(
            err,
            opi_ai::CollectionError::Provider(ProviderError::CredentialNeeded { .. })
        ),
        "unauthenticated route must surface the owning CredentialNeeded failure, got {err:?}"
    );

    // No model dispatch occurred in any failure case, and no fallback reached alpha.
    assert_eq!(
        alpha_dispatches.load(Ordering::SeqCst),
        0,
        "alpha must not dispatch for failures on other routes (no silent fallback)"
    );
    assert_eq!(
        beta_dispatches.load(Ordering::SeqCst),
        0,
        "beta must not dispatch when auth preparation fails"
    );
}

/// P17-PRV-001 + P17-OUT-001: model calls perform route lookup and dispatch
/// through the registered collection; successive calls route to two different
/// providers through ONE collection without reconstructing it.
#[tokio::test]
async fn phase17_two_providers_dispatch_through_one_collection_without_rebuild() {
    let mut collection = ProviderCollection::new();
    let alpha = register_counting_route(&mut collection, "alpha", "a1", static_resolver());
    let beta = register_counting_route(&mut collection, "beta", "b1", static_resolver());

    // First logical call routes to alpha.
    let prepared = collection
        .prepare_call("alpha:a1", request_for("alpha:a1"))
        .await
        .expect("alpha route must prepare");
    assert_eq!(prepared.route().provider_id, "alpha");
    assert_eq!(prepared.route().model_id, "a1");
    let _ = drain_to_completion(prepared.start_attempt().expect("attempt starts"))
        .await
        .expect("alpha stream drains");

    // Second logical call routes to beta through the SAME collection.
    let prepared = collection
        .prepare_call("beta:b1", request_for("beta:b1"))
        .await
        .expect("beta route must prepare");
    assert_eq!(prepared.route().provider_id, "beta");
    assert_eq!(prepared.route().model_id, "b1");
    let _ = drain_to_completion(prepared.start_attempt().expect("attempt starts"))
        .await
        .expect("beta stream drains");

    assert_eq!(
        alpha.load(Ordering::SeqCst),
        1,
        "alpha dispatched exactly once"
    );
    assert_eq!(
        beta.load(Ordering::SeqCst),
        1,
        "beta dispatched exactly once"
    );
}

/// P17-PRV-002: the canonical `provider:model` selection resolves; an ambiguous
/// bare selection fails before the provider is touched. No alias registry exists.
#[tokio::test]
async fn phase17_canonical_selection_resolves_and_bare_is_ambiguous() {
    let mut collection = ProviderCollection::new();
    let alpha = register_counting_route(&mut collection, "alpha", "a1", static_resolver());

    // Canonical spec resolves and dispatches.
    let prepared = collection
        .prepare_call("alpha:a1", request_for("alpha:a1"))
        .await
        .expect("canonical spec must resolve");
    let _ = drain_to_completion(prepared.start_attempt().expect("attempt starts"))
        .await
        .expect("canonical stream drains");
    assert_eq!(alpha.load(Ordering::SeqCst), 1);

    // Bare model id is ambiguous (no provider) and fails before dispatch.
    collection
        .prepare_call("a1", request_for("a1"))
        .await
        .expect_err("bare model must not resolve without an alias registry");
    assert_eq!(
        alpha.load(Ordering::SeqCst),
        1,
        "failed bare selection must not add a dispatch"
    );
}

/// P17-PRV-004: a route or auth-preparation failure does not silently fall back
/// to another registered provider; the typed failure is returned.
#[tokio::test]
async fn phase17_auth_failure_does_not_silently_fallback() {
    let mut collection = ProviderCollection::new();
    let alpha = register_counting_route(&mut collection, "alpha", "a1", static_resolver());
    let _gamma = register_counting_route(
        &mut collection,
        "gamma",
        "g1",
        Arc::new(FailingResolver {
            provider_id: "gamma".into(),
        }),
    );

    // gamma's auth fails; the collection must return the owning failure rather
    // than retry against alpha.
    let err = collection
        .prepare_call("gamma:g1", request_for("gamma:g1"))
        .await
        .expect_err("gamma auth failure must surface");
    assert!(
        matches!(
            err,
            opi_ai::CollectionError::Provider(ProviderError::CredentialNeeded { .. })
        ),
        "got {err:?}"
    );
    assert_eq!(
        alpha.load(Ordering::SeqCst),
        0,
        "no silent fallback reached alpha"
    );
}

/// P17-A01 / P17-OUT-001 (Phase 17.5 acceptance): a `CodingHarness` whose
/// dispatch collection holds TWO real routes switches provider via
/// `set_model_validated` without reconstructing the Agent, and both providers
/// dispatch through one collection.
///
/// The active `alpha` route and the `extra_routes` `beta` route are both
/// registered dispatchable providers; a cross-provider switch resolves at the
/// next `prepare_call` instead of being rejected at configuration time.
#[tokio::test]
#[allow(clippy::await_holding_lock)] // serialized OPI_SESSIONS_DIR mutation; not re-acquired in awaited dispatch.
async fn phase17_coding_harness_cross_provider_switch_dispatches_both_providers() {
    use opi_ai::test_support::{MockProvider, text_response};
    use opi_coding_agent::config::OpiConfig;
    use opi_coding_agent::harness::CodingHarness;
    use opi_coding_agent::project_trust::TrustDecision;

    let _lock = phase17::session_lock();
    let sessions = tempfile::tempdir().expect("sessions tempdir");
    phase17::set_sessions_dir(sessions.path());
    let workspace = tempfile::tempdir().expect("workspace tempdir");

    let alpha_provider = MockProvider::new_with_models(
        "alpha",
        vec![model_info("a1")],
        vec![text_response("alpha-response")],
    );
    let alpha_calls = alpha_provider.call_log_handle();
    let beta_provider = MockProvider::new_with_models(
        "beta",
        vec![model_info("b1")],
        vec![text_response("beta-response")],
    );
    let beta_calls = beta_provider.call_log_handle();

    let mut harness = CodingHarness::builder(
        Box::new(alpha_provider),
        "alpha:a1".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .extra_routes(vec![(Box::new(beta_provider), static_resolver())])
    .build();

    assert_eq!(harness.model_spec(), "alpha:a1");

    // Cross-provider switch resolves through the dispatch collection without
    // reconstructing the Agent.
    assert_eq!(
        harness.set_model_validated("beta:b1".into()).unwrap(),
        "beta:b1"
    );
    assert_eq!(harness.model_spec(), "beta:b1");
    harness.prompt("hi from beta").await.unwrap();

    // Switch back to alpha through the SAME harness instance.
    assert_eq!(
        harness.set_model_validated("alpha:a1".into()).unwrap(),
        "alpha:a1"
    );
    harness.prompt("hi from alpha").await.unwrap();

    assert_eq!(
        alpha_calls.lock().unwrap().len(),
        1,
        "alpha dispatched exactly once"
    );
    assert_eq!(
        beta_calls.lock().unwrap().len(),
        1,
        "beta dispatched exactly once"
    );

    phase17::clear_sessions_dir();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // serialized OPI_SESSIONS_DIR mutation; not re-acquired in awaited dispatch.
async fn phase17_noninteractive_runtime_preserves_the_active_route_resolver() {
    use opi_agent::extension::ExtensionRegistry;
    use opi_ai::test_support::{MockProvider, text_response};
    use opi_coding_agent::config::OpiConfig;
    use opi_coding_agent::policy::ToolSelection;
    use opi_coding_agent::project_trust::TrustDecision;
    use opi_coding_agent::runner::NonInteractiveRunner;
    use opi_coding_agent::runtime_packages::RuntimePackageStartup;

    let _lock = phase17::session_lock();
    let sessions = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let resolver_calls = Arc::new(AtomicU32::new(0));
    let resolver: Arc<dyn AuthResolver> = Arc::new(CountingResolver {
        calls: resolver_calls.clone(),
    });
    let provider = MockProvider::new_with_models(
        "active",
        vec![model_info("model")],
        vec![text_response("done")],
    );
    phase17::set_sessions_dir(sessions.path());
    let startup = RuntimePackageStartup {
        extension_registry: ExtensionRegistry::new(),
        installed_packages: Vec::new(),
        diagnostics: Vec::new(),
        trust_decision: TrustDecision::Trusted,
    };
    let mut runner = NonInteractiveRunner::new_with_resume_runtime_packages_and_auth(
        Box::new(provider),
        "active:model".to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        None,
        Vec::new(),
        None,
        ToolSelection::Default,
        startup,
        None,
        resolver,
        Vec::new(),
    )
    .unwrap();

    let result = runner.run("hello").await;

    assert_eq!(result.exit_code, 0, "{}", result.stderr);
    assert_eq!(resolver_calls.load(Ordering::SeqCst), 1);
    phase17::clear_sessions_dir();
}
