//! Frozen telemetry across Deferred submit, fetch, cancel and handle serialization.

use std::sync::Arc;
use std::task::Context as TaskContext;

use opi_ai::api::contract::{DeferredApi, DeferredCancelCall, DeferredFetchCall, SimpleOptions};
use opi_ai::auth::{AuthProvenanceSource, ResolvedAuth};
use opi_ai::models::Models;
use opi_ai::provider::{EventStream, ProviderError};
use opi_ai::telemetry::TelemetryContext;
use opi_ai::types::ApiId;
use tracing::Level;
use tracing_core::dispatcher;

use super::fixtures::{
    AdapterObservations, MarkerChatApi, MarkerStream, MarkerTerminal, StaticBearer,
    assert_all_parented, assert_no_canaries, assert_no_telemetry_keys, await_under_ambient,
    deferred_handle, marker_record, marker_request, message_text, probe_spans, recorder,
};

struct MarkerDeferredApi;

impl DeferredApi<()> for MarkerDeferredApi {
    fn api_id(&self) -> &ApiId {
        static API: std::sync::OnceLock<ApiId> = std::sync::OnceLock::new();
        API.get_or_init(|| ApiId::new("tele-deferred"))
    }
    fn provider_id(&self) -> &str {
        "telemetry"
    }
    fn model_id(&self) -> &str {
        "marker-model"
    }
    fn default_options(&self) {}
    fn fetch_prepared(
        &self,
        _handle: &opi_ai::types::deferred::DeferredHandle,
        _options: &(),
        _auth: ResolvedAuth,
        call: DeferredFetchCall,
    ) -> EventStream {
        Box::pin(MarkerStream::new(
            call.telemetry_context,
            opi_ai::test_support::text_response("deferred final"),
        ))
    }
    fn cancel_prepared<'a>(
        &'a self,
        _handle: &'a opi_ai::types::deferred::DeferredHandle,
        _options: &'a (),
        _auth: ResolvedAuth,
        call: DeferredCancelCall,
    ) -> opi_ai::credential::BoxAuthFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            if let Some(context) = &call.telemetry_context {
                context.in_scope(|| {
                    tracing::span!(Level::INFO, "deferred_cancel_marker").in_scope(|| ());
                });
            }
            Ok(())
        })
    }
}

#[tokio::test]
async fn deferred_submit_fetch_and_cancel_freeze_the_context_and_the_handle_stays_serializable() {
    let (a_recording, a_dispatch) = recorder();
    let (b_recording, b_dispatch) = recorder();
    let observed = Arc::new(AdapterObservations::default());
    let mut models = Models::new();
    models
        .register_chat_api(
            Arc::new(MarkerChatApi {
                observed,
                terminal: MarkerTerminal::Deferred,
            }),
            Arc::new(StaticBearer),
            AuthProvenanceSource::Static,
        )
        .expect("register deferred-submit chat api");
    models
        .register_deferred_api(
            Arc::new(MarkerDeferredApi),
            Arc::new(StaticBearer),
            AuthProvenanceSource::Static,
        )
        .expect("register marker deferred backend");

    let parent =
        dispatcher::with_default(&a_dispatch, || tracing::span!(Level::INFO, "caller_parent"));
    let parent_id = parent.id().unwrap().into_u64();
    let context = dispatcher::with_default(&b_dispatch, || TelemetryContext::from_span(parent));

    // Submit: a normal chat call whose terminal is the deferred handle.
    let submitted = await_under_ambient(
        &b_dispatch,
        models.complete_simple(
            marker_record(),
            marker_request(Some(context.clone())),
            SimpleOptions::default(),
        ),
    )
    .await
    .expect("deferred submission succeeds");
    assert_eq!(submitted.stop_reason, opi_ai::stream::StopReason::Deferred);
    let handle = submitted
        .deferred
        .clone()
        .expect("the terminal carries the deferred handle");
    assert_eq!(&*handle, &deferred_handle());

    // The serialized handle carries exactly its source-owned identity
    // fields; no telemetry or trace artifact rides along.
    let serialized = serde_json::to_value(&*handle).expect("handle serializes");
    assert_no_telemetry_keys(&serialized);
    let mut keys: Vec<&str> = serialized
        .as_object()
        .expect("handle serializes to an object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, vec!["api", "id", "modelId", "provider"]);

    // The whole submitted terminal message (including the embedded handle)
    // serializes without telemetry artifacts.
    let message_json = serde_json::to_value(&submitted).expect("message serializes");
    assert_no_telemetry_keys(&message_json);

    // Fetch: the backend's stream sections observe the frozen context.
    let fetched = await_under_ambient(
        &b_dispatch,
        models.fetch_deferred(
            &handle,
            DeferredFetchCall {
                telemetry_context: Some(context.clone()),
                ..Default::default()
            },
        ),
    )
    .await
    .expect("deferred fetch succeeds");
    assert_eq!(message_text(&fetched), "deferred final");
    assert_eq!(fetched.stop_reason, opi_ai::stream::StopReason::Stop);

    // Cancel: the backend's synchronous section observes the same context.
    models
        .cancel_deferred(
            &handle,
            DeferredCancelCall {
                telemetry_context: Some(context.clone()),
                ..Default::default()
            },
        )
        .await
        .expect("deferred cancel succeeds");

    // Abandonment: a fetched stream polled once then dropped runs one
    // scoped drop section in the backend.
    let mut stream = models.stream_deferred(
        &handle,
        DeferredFetchCall {
            telemetry_context: Some(context),
            ..Default::default()
        },
    );
    let waker = futures_util::task::noop_waker();
    let mut cx = TaskContext::from_waker(&waker);
    let first = dispatcher::with_default(&b_dispatch, || {
        futures_util::StreamExt::poll_next_unpin(&mut stream, &mut cx)
    });
    assert!(first.is_ready());
    drop(stream);

    let spans = probe_spans(&a_recording);
    assert_all_parented(&spans, parent_id);
    for expected in [
        "chat_poll_marker",
        "deferred_cancel_marker",
        "chat_drop_marker",
    ] {
        assert!(
            spans.iter().any(|(name, _)| *name == expected),
            "expected a parented '{expected}' marker"
        );
    }
    assert!(b_recording.lock().unwrap().spans.is_empty());
    assert_no_canaries(&a_recording);
}
