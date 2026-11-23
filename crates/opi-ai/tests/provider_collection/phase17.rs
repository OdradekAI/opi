use super::*;
use futures_util::StreamExt;
use opi_ai::auth::{
    AuthFallback, AuthProvenance, AuthProvenanceSource, AuthResolver, AuthScheme, ResolvedAuth,
};
use opi_ai::credential::BoxAuthFuture;
use opi_ai::models::CompatMetadata;
use opi_ai::provider::{EventStream, Provider, ProviderError, Request};
use opi_ai::providers::azure_openai::AzureOpenAIProvider;
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
    fn new(id: &str, model: &str, prepared: Arc<AtomicUsize>, legacy: Arc<AtomicUsize>) -> Self {
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
        let original_request = request.clone();
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
                    matches!(
                        accepted_failure_cause(&error, &original_request),
                        ProviderError::RateLimited {
                            retry_after_ms: Some(0)
                        }
                    ),
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
    let mut records = vec![
        ModelInfo::new(
            "anthropic",
            "Anthropic",
            WireApi::AnthropicMessages,
            ModelCapabilities::new(32_768, 32_768).with_thinking(true),
        ),
        adaptive,
    ];
    records.extend([1, 1_024, 2_000].map(|limit| {
        ModelInfo::new(
            format!("anthropic-{limit}"),
            "Small-cap Anthropic",
            WireApi::AnthropicMessages,
            ModelCapabilities::new(200_000, limit).with_thinking(true),
        )
    }));
    let provider = MockProvider::new_with_models(
        "budget",
        records,
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
        for call in &calls[..2] {
            assert_eq!(call.max_tokens, Some(17_192));
            assert_eq!(call.thinking.budget_tokens, Some(8_192));
        }
    }
    // Actual final model caps constrain the combined answer/thinking
    // ceiling. Zero and sub-1024 budgets remain valid compiler results.
    for (limit, clamped) in [(1, 0), (1_024, 0), (2_000, 976)] {
        let spec = format!("budget:anthropic-{limit}");
        let mut request = minimal_request(&spec);
        request.max_tokens = Some(4_096);
        request.thinking = ThinkingConfig {
            enabled: true,
            budget_tokens: Some(8_192),
            level: opi_ai::ThinkingLevel::Medium,
        };
        let prepared = collection.prepare_call(&spec, request).await.unwrap();
        drain_to_completion(prepared.start_attempt().unwrap())
            .await
            .unwrap();
        let calls = calls.lock().unwrap();
        let recorded = calls.last().expect("recorded dispatch");
        assert_eq!(recorded.max_tokens, Some(limit));
        assert_eq!(recorded.thinking.budget_tokens, Some(clamped));
    }
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
    assert_eq!(
        calls.lock().unwrap().last().unwrap().max_tokens,
        Some(1_024)
    );
}

#[tokio::test]
async fn prepare_call_adds_bedrock_thinking_only_for_numeric_claude() {
    for (id, name, expected_limit) in [
        ("anthropic.claude-sonnet-4-5", "Numeric model", 6_144),
        ("anthropic/claude-sonnet-4-5", "Numeric model", 6_144),
        ("application-profile", "Claude Sonnet", 6_144),
        ("openai.gpt-oss-120b", "GPT OSS", 4_096),
        ("anthropic.claude-sonnet-4-6", "Claude Sonnet 4.6", 4_096),
    ] {
        let provider = MockProvider::new_with_models(
            "budget",
            vec![ModelInfo::new(
                id,
                name,
                WireApi::BedrockConverseStream,
                ModelCapabilities::new(200_000, 64_000).with_thinking(true),
            )],
            vec![text_response("done")],
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
        let spec = format!("budget:{id}");
        let mut request = minimal_request(&spec);
        request.max_tokens = Some(4_096);
        request.thinking = ThinkingConfig {
            enabled: true,
            budget_tokens: Some(2_048),
            level: opi_ai::ThinkingLevel::Low,
        };
        let prepared = collection.prepare_call(&spec, request).await.unwrap();
        drain_to_completion(prepared.start_attempt().unwrap())
            .await
            .unwrap();
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].max_tokens, Some(expected_limit), "model {id}");
        assert_eq!(calls[0].thinking.budget_tokens, Some(2_048));
    }
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
    let original_request = request.clone();
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
    assert!(
        error.is_call_cancelled(),
        "logical-call cancellation origin remains known"
    );
    assert_eq!(
        accepted_models_failure_cause(&error, &original_request),
        &ProviderError::Cancelled
    );
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
        .prepare_call("cancel-aware:model-a", request.clone())
        .await
        .expect("prepare call");
    let mut stream = prepared.start_attempt().expect("start active attempt");
    assert!(futures_util::poll!(stream.next()).is_pending());
    assert_eq!(dispatches.load(Ordering::SeqCst), 1);
    let task = tokio::spawn(async move {
        let terminal = stream.next().await;
        assert!(
            stream.next().await.is_none(),
            "cancelled attempt must end permanently"
        );
        terminal
    });
    token.cancel();

    let terminal = tokio::time::timeout(Duration::from_secs(1), task)
        .await
        .expect("active attempt observes cancellation")
        .expect("attempt task did not panic");
    let error = terminal
        .expect("one cancellation")
        .expect_err("active cancellation must fail closed");
    assert_eq!(
        accepted_failure_cause(&error, &request),
        &ProviderError::Cancelled
    );
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
            accepted_models_failure_cause(&err, &minimal_request("alpha:model-a"))
                == &match kind {
                    FailKind::Revoked => ProviderError::CredentialRevoked {
                        provider_id: "alpha".into()
                    },
                    FailKind::Needed => ProviderError::CredentialNeeded {
                        provider_id: "alpha".into()
                    },
                },
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
        let error = attempt
            .next()
            .await
            .expect("one credential failure")
            .expect_err("credential invalidation must fail closed");
        assert_eq!(
            accepted_failure_cause(&error, &minimal_request("alpha:model-a")),
            &match kind {
                FailKind::Revoked => ProviderError::CredentialRevoked {
                    provider_id: "alpha".into()
                },
                FailKind::Needed => ProviderError::CredentialNeeded {
                    provider_id: "alpha".into()
                },
            }
        );
        assert!(
            attempt.next().await.is_none(),
            "credential failure is terminal"
        );

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
            accepted_models_failure_cause(&err, &minimal_request("alpha:model-a")),
            ProviderError::CredentialRevoked { provider_id } if provider_id == "alpha"
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
