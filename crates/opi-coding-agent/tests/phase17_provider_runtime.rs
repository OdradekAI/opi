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

/// An extension contributing one custom provider. Product assembly registers
/// extension providers LOOKUP-ONLY (registry entry without an
/// auth-resolver-backed route), so `ext:ext-model` resolves in the registry
/// but is not dispatchable.
struct ExtProviderExtension;

impl opi_agent::extension::Extension for ExtProviderExtension {
    fn name(&self) -> &str {
        "ext-providers"
    }

    fn providers(&self) -> Vec<Box<dyn Provider>> {
        use opi_ai::test_support::{MockProvider, text_response};
        vec![Box::new(MockProvider::new_with_models(
            "ext",
            vec![model_info("ext-model")],
            vec![text_response("ext-response")],
        ))]
    }
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
        "a1".into(),
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
    assert_eq!(
        harness.set_model_validated("a1".into()).unwrap(),
        "alpha:a1",
        "a bare selection normalizes only when exactly one dispatchable route serves it (here: alpha)"
    );
    let unknown = harness
        .set_model_validated("missing".into())
        .expect_err("an unknown bare selection is a typed failure before any write");
    assert_eq!(
        unknown, "bare model 'missing' matches no dispatchable route",
        "an unknown bare selection fails with typed remediation, not a parse error"
    );
    assert_eq!(
        harness.model_spec(),
        "alpha:a1",
        "a rejected bare selection keeps the active route"
    );
    harness.prompt("hi from alpha").await.unwrap();

    let (_, entries) = opi_agent::session::SessionReader::read_all(
        harness
            .session()
            .expect("session remains active")
            .session_path(),
    )
    .unwrap();
    assert!(matches!(
        entries
            .iter()
            .filter_map(|entry| match entry {
                opi_agent::session::SessionEntry::ModelChange(change) => Some(change),
                _ => None,
            })
            .next_back(),
        Some(change)
            if change.model == "alpha:a1"
                && change.input_source
                    == Some(opi_agent::session::ModelInputSource::BareNormalized)
    ));

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

/// P17-PRV-002 write side: a bare model served by MORE THAN ONE dispatchable
/// route is ambiguous and must fail with typed remediation naming every
/// candidate BEFORE anything is persisted. The active route is kept and no
/// provider is dispatched.
#[tokio::test]
#[allow(clippy::await_holding_lock)] // serialized OPI_SESSIONS_DIR mutation; not re-acquired in awaited dispatch.
async fn phase17_bare_selection_ambiguous_across_dispatchable_routes_fails_typed() {
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
        vec![model_info("shared")],
        vec![text_response("alpha-shared")],
    );
    let alpha_calls = alpha_provider.call_log_handle();
    let beta_provider = MockProvider::new_with_models(
        "beta",
        vec![model_info("shared")],
        vec![text_response("beta-shared")],
    );
    let beta_calls = beta_provider.call_log_handle();

    let mut harness = CodingHarness::builder(
        Box::new(alpha_provider),
        "alpha:shared".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .extra_routes(vec![(Box::new(beta_provider), static_resolver())])
    .build();
    harness.prompt("seed").await.expect("seed prompt runs");

    let err = harness
        .set_model_validated("shared".into())
        .expect_err("an ambiguous bare selection must fail before any write");
    assert_eq!(
        err,
        "bare model 'shared' matches more than one dispatchable route: alpha:shared, beta:shared",
        "ambiguity names every candidate route"
    );
    assert_eq!(
        harness.model_spec(),
        "alpha:shared",
        "a rejected ambiguous selection keeps the active route"
    );

    // The durable session records no model_change for the rejected selection.
    let (_, entries) = opi_agent::session::SessionReader::read_all(
        harness
            .session()
            .expect("session remains active")
            .session_path(),
    )
    .unwrap();
    assert!(
        entries
            .iter()
            .all(|entry| { !matches!(entry, opi_agent::session::SessionEntry::ModelChange(_)) }),
        "no model_change entry may be persisted for an ambiguous bare selection"
    );
    assert_eq!(
        alpha_calls.lock().unwrap().len(),
        1,
        "alpha dispatched only for the seed prompt"
    );
    assert_eq!(
        beta_calls.lock().unwrap().len(),
        0,
        "beta was never dispatched"
    );

    phase17::clear_sessions_dir();
}

/// Lookup-only extension providers are registry-resolvable but hold no
/// dispatchable route: a model-change write to such a spec must fail BEFORE
/// the durable `model_change` entry is appended, leaving the session and the
/// live route unchanged instead of persisting a route the runtime would
/// reject at the next dispatch.
#[tokio::test]
#[allow(clippy::await_holding_lock)] // serialized OPI_SESSIONS_DIR mutation; not re-acquired in awaited dispatch.
async fn phase17_set_model_to_lookup_only_extension_route_fails_without_persisting() {
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
    let mut registry = opi_agent::extension::ExtensionRegistry::new();
    registry
        .register(Box::new(ExtProviderExtension))
        .expect("extension registers");

    let mut harness = CodingHarness::builder(
        Box::new(alpha_provider),
        "a1".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .extension_registry(registry)
    .build();
    assert_eq!(harness.model_spec(), "alpha:a1");
    harness.prompt("seed").await.expect("seed prompt runs");

    let err = harness
        .set_model_validated("ext:ext-model".into())
        .expect_err("a lookup-only route must fail before the durable write");
    assert_eq!(
        err, "provider 'ext' has no dispatchable route",
        "the failure is the collection's typed route error"
    );
    assert_eq!(
        harness.model_spec(),
        "alpha:a1",
        "a rejected model change keeps the live route"
    );

    // The durable session records no model_change for the rejected selection:
    // validation must precede the append, not trail it.
    let (_, entries) = opi_agent::session::SessionReader::read_all(
        harness
            .session()
            .expect("session remains active")
            .session_path(),
    )
    .unwrap();
    assert!(
        entries
            .iter()
            .all(|entry| { !matches!(entry, opi_agent::session::SessionEntry::ModelChange(_)) }),
        "no model_change entry may be persisted for a rejected lookup-only route"
    );

    phase17::clear_sessions_dir();
}

/// Resuming a session whose latest `model_change` records a lookup-only
/// (registry-resolvable, resolver-less) route keeps the CLI/config model and
/// emits the typed model-incompatible diagnostic instead of panicking at the
/// application step.
#[tokio::test]
#[allow(clippy::await_holding_lock)] // serialized OPI_SESSIONS_DIR mutation; not re-acquired in awaited dispatch.
async fn phase17_resume_of_lookup_only_recorded_route_is_typed_not_panicking() {
    use opi_agent::diagnostic::code::CODE_SESSION_RESUME_MODEL_INCOMPATIBLE;
    use opi_agent::session::{
        LeafEntry, MessageEntry, ModelChangeEntry, SessionEntry, SessionHeader, SessionWriter,
    };
    use opi_ai::message::{InputContent, Message, UserMessage};
    use opi_ai::test_support::{MockProvider, text_response};
    use opi_coding_agent::config::OpiConfig;
    use opi_coding_agent::harness::CodingHarness;
    use opi_coding_agent::project_trust::TrustDecision;

    let _lock = phase17::session_lock();
    let sessions = tempfile::tempdir().expect("sessions tempdir");
    phase17::set_sessions_dir(sessions.path());
    let workspace = tempfile::tempdir().expect("workspace tempdir");

    // Session fixture: one user message, then a canonical model_change naming
    // the lookup-only extension route, with the leaf on the message.
    let path = sessions.path().join("s-ext.jsonl");
    let header = SessionHeader::new(
        "s-ext".into(),
        "2026-08-21T12:00:00Z".into(),
        "/repo".into(),
        None,
    );
    let mut writer = SessionWriter::create(&path, header).unwrap();
    writer
        .append(&SessionEntry::Message(MessageEntry {
            id: "msg-1".into(),
            parent_id: None,
            timestamp: "2026-08-21T12:00:01Z".into(),
            message: Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "seed".into(),
                }],
                timestamp_ms: 0,
            }),
        }))
        .unwrap();
    writer
        .append(&SessionEntry::ModelChange(ModelChangeEntry {
            id: "model-1".into(),
            parent_id: Some("msg-1".into()),
            timestamp: "2026-08-21T12:00:02Z".into(),
            model: "ext:ext-model".into(),
            input_source: None,
        }))
        .unwrap();
    writer
        .append(&SessionEntry::Leaf(LeafEntry {
            id: "leaf-1".into(),
            parent_id: Some("msg-1".into()),
            timestamp: "2026-08-21T12:00:03Z".into(),
            entry_id: "msg-1".into(),
        }))
        .unwrap();
    drop(writer);

    let alpha = MockProvider::new_with_models(
        "alpha",
        vec![model_info("a1")],
        vec![text_response("alpha-response")],
    );
    let mut registry = opi_agent::extension::ExtensionRegistry::new();
    registry
        .register(Box::new(ExtProviderExtension))
        .expect("extension registers");
    let mut harness = CodingHarness::builder(
        Box::new(alpha),
        "a1".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .extension_registry(registry)
    .record_diagnostics(true)
    .build();

    let resumed = harness
        .resume_session_id("s-ext")
        .expect("resume keeps the CLI/config model instead of panicking");
    assert_eq!(resumed, 1, "one user message is reconstructed");
    assert_eq!(
        harness.model_spec(),
        "alpha:a1",
        "the lookup-only recorded route is never applied"
    );
    assert!(
        harness
            .recorded_diagnostics()
            .iter()
            .any(|d| d.code == CODE_SESSION_RESUME_MODEL_INCOMPATIBLE),
        "the lookup-only recorded route emits the typed model-incompatible diagnostic"
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
