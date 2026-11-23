//! Public collection dispatch and stream-terminal behavior.

use super::*;

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
            accepted_models_failure_cause(&err, &minimal_request("noauth:mock-model")),
            ProviderError::CredentialNeeded { provider_id } if provider_id == "noauth"
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
        projection: None,
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
