//! Frozen telemetry through Chat entry points, task progress, poll and Drop.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::task::{Context as TaskContext, Poll};

use opi_ai::api::contract::{ChatApi, SimpleOptions};
use opi_ai::auth::AuthProvenanceSource;
use opi_ai::models::Models;
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::telemetry::TelemetryContext;
use tracing::Level;
use tracing_core::{Dispatch, dispatcher};

use super::fixtures::{
    AdapterObservations, MarkerChatApi, MarkerTerminal, Recording, StaticBearer,
    assert_all_parented, assert_no_canaries, await_under_ambient, marker_record, marker_request,
    message_text, probe_spans, recorder,
};

fn marker_models(
    observed: Arc<AdapterObservations>,
) -> (Models, opi_ai::api::contract::ChatApiHandle<()>) {
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            Arc::new(MarkerChatApi {
                observed,
                terminal: MarkerTerminal::Text,
            }),
            Arc::new(StaticBearer),
            AuthProvenanceSource::Static,
        )
        .expect("register marker chat api");
    (models, handle)
}

#[tokio::test]
async fn default_and_noop_contexts_run_business_once_without_sdk_observation() {
    let (a_recording, a_dispatch) = recorder();
    let observed = Arc::new(AdapterObservations::default());
    let (models, _handle) = marker_models(Arc::clone(&observed));

    // Omitted context: business runs, and even a recording ambient default
    // observes no SDK-emitted span or event.
    let message = await_under_ambient(
        &a_dispatch,
        models.complete_simple(
            marker_record(),
            marker_request(None),
            SimpleOptions::default(),
        ),
    )
    .await
    .expect("omitted context still completes business work");
    assert_eq!(message_text(&message), "marker response");
    assert_eq!(observed.dispatches.load(Ordering::SeqCst), 1);
    assert_eq!(*observed.telemetry_present.lock().unwrap(), vec![false]);
    assert!(
        a_recording.lock().unwrap().spans.is_empty(),
        "the SDK must not emit automatic call/attempt spans"
    );
    assert!(a_recording.lock().unwrap().events.is_empty());

    // Explicit noop: business runs exactly once with identical output, the
    // consumer's ambient dispatcher is not adopted by the execution task.
    // The raw carrier tests below still prove noop preserves execution-local tracing.
    let message = await_under_ambient(
        &a_dispatch,
        models.complete_simple(
            marker_record(),
            marker_request(Some(TelemetryContext::noop())),
            SimpleOptions::default(),
        ),
    )
    .await
    .expect("noop context still completes business work");
    assert_eq!(message_text(&message), "marker response");
    assert_eq!(observed.dispatches.load(Ordering::SeqCst), 2);
    assert_eq!(
        *observed.telemetry_present.lock().unwrap(),
        vec![false, true]
    );
    assert!(a_recording.lock().unwrap().spans.is_empty());
    assert_no_canaries(&a_recording);
}

#[tokio::test]
async fn abandoned_chat_call_scopes_poll_and_drop_to_the_frozen_parent() {
    let (a_recording, a_dispatch) = recorder();
    let (b_recording, b_dispatch) = recorder();
    let observed = Arc::new(AdapterObservations::default());
    let (models, handle) = marker_models(Arc::clone(&observed));

    let parent =
        dispatcher::with_default(&a_dispatch, || tracing::span!(Level::INFO, "caller_parent"));
    let parent_id = parent.id().unwrap().into_u64();
    let context = dispatcher::with_default(&b_dispatch, || TelemetryContext::from_span(parent));

    let mut call = models.stream(
        marker_record(),
        marker_request(Some(context)),
        handle.options(()),
    );
    let waker = futures_util::task::noop_waker();
    let mut cx = TaskContext::from_waker(&waker);
    let first = dispatcher::with_default(&b_dispatch, || {
        futures_util::StreamExt::poll_next_unpin(&mut call, &mut cx)
    });
    assert!(
        first.is_pending(),
        "consumer does not poll the source inline"
    );
    drop(call);
    wait_chat_release(&observed).await;

    let spans = probe_spans(&a_recording);
    let polls = spans
        .iter()
        .filter(|(name, _)| *name == "chat_poll_marker")
        .count();
    let drops = spans
        .iter()
        .filter(|(name, _)| *name == "chat_drop_marker")
        .count();
    assert_eq!(
        polls, 4,
        "execution drains all scripted events and source EOF after consumer Drop"
    );
    assert_eq!(
        drops, 1,
        "execution releases the source in its frozen scope"
    );
    assert_all_parented(&spans, parent_id);
    assert!(b_recording.lock().unwrap().spans.is_empty());
}

async fn wait_chat_release(observed: &AdapterObservations) {
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while observed.drops.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("execution releases the scripted source");
}

fn business_counts(observed: &AdapterObservations) -> [usize; 4] {
    [
        observed.dispatches.load(Ordering::SeqCst),
        observed.polls.load(Ordering::SeqCst),
        observed.events.load(Ordering::SeqCst),
        observed.drops.load(Ordering::SeqCst),
    ]
}

#[tokio::test]
async fn ordinary_chat_keeps_frozen_parent_on_the_execution_task() {
    let (recording, dispatch) = recorder();
    let observed = Arc::new(AdapterObservations::default());
    let (models, handle) = marker_models(Arc::clone(&observed));
    let parent =
        dispatcher::with_default(&dispatch, || tracing::span!(Level::INFO, "caller_parent"));
    let parent_id = parent.id().unwrap().into_u64();
    let context = TelemetryContext::from_span(parent);
    let started = models.stream(
        marker_record(),
        marker_request(Some(context)),
        handle.options(()),
    );
    drop(models);
    let message = started.result().await.unwrap();
    assert_eq!(message_text(&message), "marker response");
    assert_eq!(business_counts(&observed), [1, 4, 3, 1]);
    assert_all_parented(&probe_spans(&recording), parent_id);
    assert_no_canaries(&recording);
}

/// Observe the caller immediately after one adapter section, while the caller's
/// own parent and dispatch are still installed.
fn ambient_section<T>(ambient: &Dispatch, parent: &tracing::Span, body: impl FnOnce() -> T) -> T {
    dispatcher::with_default(ambient, || {
        parent.in_scope(|| {
            let output = body();
            assert_eq!(tracing::Span::current().id(), parent.id());
            tracing::span!(Level::INFO, "ambient_restore_marker").in_scope(|| ());
            output
        })
    })
}

fn assert_ambient_restored(recording: &Mutex<Recording>, parents: &[u64]) {
    let state = recording.lock().unwrap();
    let restored: Vec<_> = state
        .spans
        .iter()
        .filter(|(name, _)| *name == "ambient_restore_marker")
        .map(|(_, parent)| *parent)
        .collect();
    assert_eq!(
        restored,
        parents.iter().copied().map(Some).collect::<Vec<_>>()
    );
    assert!(
        state
            .spans
            .iter()
            .all(|(name, _)| *name == "ambient_parent" || *name == "ambient_restore_marker")
    );
    assert!(state.entered.values().all(Vec::is_empty));
}

#[test]
fn pending_chat_stream_preserves_frozen_context_across_threads() {
    let (a_recording, a_dispatch) = recorder();
    let (b_recording, b_dispatch) = recorder();
    let observed = Arc::new(AdapterObservations {
        pending_first_poll: true,
        ..Default::default()
    });
    let api = MarkerChatApi {
        observed: Arc::clone(&observed),
        terminal: MarkerTerminal::Text,
    };
    let parent =
        dispatcher::with_default(&a_dispatch, || tracing::span!(Level::INFO, "caller_parent"));
    let parent_id = parent.id().unwrap().into_u64();
    let context = dispatcher::with_default(&b_dispatch, || TelemetryContext::from_span(parent));
    let ambient_parent = dispatcher::with_default(&b_dispatch, || {
        tracing::span!(Level::INFO, "ambient_parent")
    });
    let first_ambient_id = ambient_parent.id().unwrap().into_u64();
    let mut call = opi_ai::ChatStream::new(api.stream_prepared(
        marker_request(Some(context)),
        (),
        opi_ai::test_support::resolved_bearer_auth(),
    ));
    assert_eq!(business_counts(&observed), [1, 0, 0, 0]);

    let waker = futures_util::task::noop_waker();
    let mut cx = TaskContext::from_waker(&waker);
    let pending = ambient_section(&b_dispatch, &ambient_parent, || {
        futures_util::StreamExt::poll_next_unpin(&mut call, &mut cx)
    });
    assert!(pending.is_pending());
    assert_eq!(business_counts(&observed), [1, 1, 0, 0]);

    let resumed = std::thread::spawn(move || {
        let ambient_parent = dispatcher::with_default(&b_dispatch, || {
            tracing::span!(Level::INFO, "ambient_parent")
        });
        let ambient_id = ambient_parent.id().unwrap().into_u64();
        let waker = futures_util::task::noop_waker();
        let mut cx = TaskContext::from_waker(&waker);
        let mut events = Vec::new();
        for _ in 0..3 {
            match ambient_section(&b_dispatch, &ambient_parent, || {
                futures_util::StreamExt::poll_next_unpin(&mut call, &mut cx)
            }) {
                Poll::Ready(Some(Ok(event))) => events.push(event),
                other => panic!("expected one scripted event per poll, got {other:?}"),
            }
        }
        assert!(matches!(
            ambient_section(&b_dispatch, &ambient_parent, || {
                futures_util::StreamExt::poll_next_unpin(&mut call, &mut cx)
            }),
            Poll::Ready(None)
        ));
        (events, ambient_id)
    })
    .join()
    .expect("stream resumes on another thread");

    let (events, resumed_ambient_id) = resumed;
    assert!(matches!(events[0], AssistantStreamEvent::Start { .. }));
    assert!(matches!(
        &events[1],
        AssistantStreamEvent::TextDelta { delta, .. } if delta == "marker response"
    ));
    match &events[2] {
        AssistantStreamEvent::Done { reason, message } => {
            assert_eq!(*reason, opi_ai::stream::StopReason::Stop);
            assert_eq!(message_text(message), "marker response");
        }
        other => panic!("expected the original terminal, got {other:?}"),
    }
    assert_eq!(business_counts(&observed), [1, 4, 3, 1]);
    let spans = probe_spans(&a_recording);
    assert_eq!(
        spans
            .iter()
            .filter(|(name, _)| *name == "chat_poll_marker")
            .count(),
        4
    );
    assert_eq!(
        spans
            .iter()
            .filter(|(name, _)| *name == "chat_drop_marker")
            .count(),
        1
    );
    assert_all_parented(&spans, parent_id);
    assert!(
        a_recording
            .lock()
            .unwrap()
            .entered
            .values()
            .all(Vec::is_empty)
    );
    assert_ambient_restored(
        &b_recording,
        &[
            first_ambient_id,
            resumed_ambient_id,
            resumed_ambient_id,
            resumed_ambient_id,
            resumed_ambient_id,
        ],
    );
}

#[test]
fn constructed_unpolled_event_stream_scopes_drop() {
    let (a_recording, a_dispatch) = recorder();
    let (b_recording, b_dispatch) = recorder();
    let observed = Arc::new(AdapterObservations::default());
    let api = MarkerChatApi {
        observed: Arc::clone(&observed),
        terminal: MarkerTerminal::Text,
    };
    let parent =
        dispatcher::with_default(&a_dispatch, || tracing::span!(Level::INFO, "caller_parent"));
    let parent_id = parent.id().unwrap().into_u64();
    let context = dispatcher::with_default(&b_dispatch, || TelemetryContext::from_span(parent));
    let stream = api.stream_prepared(
        marker_request(Some(context)),
        (),
        opi_ai::test_support::resolved_bearer_auth(),
    );
    assert_eq!(business_counts(&observed), [1, 0, 0, 0]);
    assert!(probe_spans(&a_recording).is_empty());
    let ambient_parent = dispatcher::with_default(&b_dispatch, || {
        tracing::span!(Level::INFO, "ambient_parent")
    });
    let ambient_id = ambient_parent.id().unwrap().into_u64();
    ambient_section(&b_dispatch, &ambient_parent, || drop(stream));
    assert_eq!(business_counts(&observed), [1, 0, 0, 1]);
    assert_eq!(
        probe_spans(&a_recording),
        vec![("chat_drop_marker", Some(parent_id))]
    );
    assert!(
        a_recording
            .lock()
            .unwrap()
            .entered
            .values()
            .all(Vec::is_empty)
    );
    assert_ambient_restored(&b_recording, &[ambient_id]);
}

#[tokio::test]
async fn dropped_unpolled_models_call_completes_in_its_frozen_scope() {
    let (a_recording, a_dispatch) = recorder();
    let (b_recording, b_dispatch) = recorder();
    let observed = Arc::new(AdapterObservations::default());
    let (models, handle) = marker_models(Arc::clone(&observed));
    let parent =
        dispatcher::with_default(&a_dispatch, || tracing::span!(Level::INFO, "caller_parent"));
    let context = dispatcher::with_default(&b_dispatch, || TelemetryContext::from_span(parent));
    let call = models.stream(
        marker_record(),
        marker_request(Some(context)),
        handle.options(()),
    );
    assert_eq!(business_counts(&observed), [0, 0, 0, 0]);
    let ambient_parent = dispatcher::with_default(&b_dispatch, || {
        tracing::span!(Level::INFO, "ambient_parent")
    });
    let ambient_id = ambient_parent.id().unwrap().into_u64();
    ambient_section(&b_dispatch, &ambient_parent, || drop(call));
    wait_chat_release(&observed).await;
    assert_eq!(business_counts(&observed), [1, 4, 3, 1]);
    assert_eq!(*observed.telemetry_present.lock().unwrap(), vec![true]);
    let spans = probe_spans(&a_recording);
    assert_eq!(spans.len(), 6);
    assert_eq!(
        spans
            .iter()
            .filter(|(name, _)| *name == "opi.telemetry.span")
            .count(),
        1,
        "ordinary Models calls install one AI request observer"
    );
    assert_ambient_restored(&b_recording, &[ambient_id]);
}

#[test]
fn noop_pending_stream_restores_ambient_and_preserves_business() {
    let (b_recording, b_dispatch) = recorder();
    let mut restored_parents = Vec::new();
    let mut outcomes = Vec::new();
    for context in [None, Some(TelemetryContext::noop())] {
        let observed = Arc::new(AdapterObservations {
            pending_first_poll: true,
            ..Default::default()
        });
        let api = MarkerChatApi {
            observed: Arc::clone(&observed),
            terminal: MarkerTerminal::Text,
        };
        let mut call = opi_ai::ChatStream::new(api.stream_prepared(
            marker_request(context),
            (),
            opi_ai::test_support::resolved_bearer_auth(),
        ));
        assert_eq!(business_counts(&observed), [1, 0, 0, 0]);
        let ambient_parent = dispatcher::with_default(&b_dispatch, || {
            tracing::span!(Level::INFO, "ambient_parent")
        });
        let ambient_id = ambient_parent.id().unwrap().into_u64();
        let waker = futures_util::task::noop_waker();
        let mut cx = TaskContext::from_waker(&waker);
        assert!(
            ambient_section(&b_dispatch, &ambient_parent, || {
                futures_util::StreamExt::poll_next_unpin(&mut call, &mut cx)
            })
            .is_pending()
        );
        assert_eq!(business_counts(&observed), [1, 1, 0, 0]);
        for _ in 0..3 {
            match ambient_section(&b_dispatch, &ambient_parent, || {
                futures_util::StreamExt::poll_next_unpin(&mut call, &mut cx)
            }) {
                Poll::Ready(Some(Ok(AssistantStreamEvent::Done { message, .. }))) => {
                    outcomes.push(message_text(&message));
                }
                Poll::Ready(Some(Ok(_))) => {}
                other => panic!("expected a scripted business event, got {other:?}"),
            }
        }
        assert!(matches!(
            ambient_section(&b_dispatch, &ambient_parent, || {
                futures_util::StreamExt::poll_next_unpin(&mut call, &mut cx)
            }),
            Poll::Ready(None)
        ));
        assert_eq!(business_counts(&observed), [1, 4, 3, 0]);
        restored_parents.extend([ambient_id; 6]);
        ambient_section(&b_dispatch, &ambient_parent, || drop(call));
        assert_eq!(observed.drops.load(Ordering::SeqCst), 1);
    }
    assert_eq!(outcomes, vec!["marker response", "marker response"]);
    let state = b_recording.lock().unwrap();
    let restored: Vec<_> = state
        .spans
        .iter()
        .filter(|(name, _)| *name == "ambient_restore_marker")
        .map(|(_, parent)| *parent)
        .collect();
    assert_eq!(
        restored,
        restored_parents
            .iter()
            .copied()
            .map(Some)
            .collect::<Vec<_>>()
    );
    let markers: Vec<_> = state
        .spans
        .iter()
        .filter(|(name, _)| *name != "ambient_parent" && *name != "ambient_restore_marker")
        .collect();
    assert_eq!(
        markers.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
        [
            "chat_poll_marker",
            "chat_poll_marker",
            "chat_poll_marker",
            "chat_poll_marker",
            "chat_drop_marker"
        ]
    );
    assert!(
        markers
            .iter()
            .all(|(_, parent)| *parent == restored_parents.last().copied())
    );
    assert!(state.entered.values().all(Vec::is_empty));
}

#[tokio::test]
async fn native_simple_and_direct_entries_attribute_adapters_to_the_frozen_parent() {
    let (a_recording, a_dispatch) = recorder();
    let (b_recording, b_dispatch) = recorder();
    let observed = Arc::new(AdapterObservations::default());
    let (models, handle) = marker_models(Arc::clone(&observed));

    let parent =
        dispatcher::with_default(&a_dispatch, || tracing::span!(Level::INFO, "caller_parent"));
    let parent_id = parent.id().unwrap().into_u64();
    let context = dispatcher::with_default(&b_dispatch, || TelemetryContext::from_span(parent));

    // Native entry driven to the single final outcome.
    let message = await_under_ambient(
        &b_dispatch,
        models.complete(
            marker_record(),
            marker_request(Some(context.clone())),
            handle.options(()),
        ),
    )
    .await
    .expect("native entry succeeds with an explicit context");
    assert_eq!(message_text(&message), "marker response");

    assert_all_parented(&probe_spans(&a_recording), parent_id);
    assert!(
        b_recording.lock().unwrap().spans.is_empty(),
        "ambient B must never observe the SDK operation"
    );

    // The simple entry's neutral conversion must not drop or replace the
    // frozen carrier.
    let message = await_under_ambient(
        &b_dispatch,
        models.complete_simple(
            marker_record(),
            marker_request(Some(context.clone())),
            SimpleOptions::default(),
        ),
    )
    .await
    .expect("simple entry succeeds with an explicit context");
    assert_eq!(message_text(&message), "marker response");
    assert_all_parented(&probe_spans(&a_recording), parent_id);
    assert!(b_recording.lock().unwrap().spans.is_empty());

    // The direct entry with injected authentication freezes the same carrier.
    let message = await_under_ambient(
        &b_dispatch,
        models.complete_direct(
            marker_record(),
            marker_request(Some(context)),
            handle.options(()),
            opi_ai::test_support::resolved_bearer_auth(),
        ),
    )
    .await
    .expect("direct entry succeeds with an explicit context");
    assert_eq!(message_text(&message), "marker response");
    assert_all_parented(&probe_spans(&a_recording), parent_id);
    assert!(b_recording.lock().unwrap().spans.is_empty());

    // Business invariance: a context-free twin produces the same text with
    // the same dispatch count, and neither recorder observed SDK spans.
    let dispatches_with = observed.dispatches.load(Ordering::SeqCst);
    let message = models
        .complete_simple(
            marker_record(),
            marker_request(None),
            SimpleOptions::default(),
        )
        .await
        .expect("context-free twin succeeds");
    assert_eq!(message_text(&message), "marker response");
    assert_eq!(
        observed.dispatches.load(Ordering::SeqCst),
        dispatches_with + 1,
        "telemetry must not change how often business work runs"
    );
    // A second Models instance registering the same ApiId stays isolated:
    // its markers attribute to its own caller parent, never to the first
    // instance's frozen parent.
    let (c_recording, c_dispatch) = recorder();
    let second_parent = dispatcher::with_default(&c_dispatch, || {
        tracing::span!(Level::INFO, "second_caller_parent")
    });
    let second_parent_id = second_parent.id().unwrap().into_u64();
    let second_context =
        dispatcher::with_default(&b_dispatch, || TelemetryContext::from_span(second_parent));
    let (second_models, _second_handle) = marker_models(Arc::new(AdapterObservations::default()));
    let spans_before_second_instance = probe_spans(&a_recording).len();
    let message = await_under_ambient(
        &b_dispatch,
        second_models.complete_simple(
            marker_record(),
            marker_request(Some(second_context)),
            SimpleOptions::default(),
        ),
    )
    .await
    .expect("second instance succeeds");
    assert_eq!(message_text(&message), "marker response");
    let second_spans: Vec<_> = c_recording
        .lock()
        .unwrap()
        .spans
        .iter()
        .copied()
        .filter(|(name, _)| *name != "second_caller_parent")
        .collect();
    assert_all_parented(&second_spans, second_parent_id);
    assert_eq!(
        probe_spans(&a_recording).len(),
        spans_before_second_instance,
        "the second instance's markers must never reach the first instance's parent"
    );
    assert_no_canaries(&a_recording);
    assert_no_canaries(&b_recording);
    assert_no_canaries(&c_recording);
}
