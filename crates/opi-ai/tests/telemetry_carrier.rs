//! Closure test for the P20-TELEMETRY shared decision (task 20.54, DEL-12).
//!
//! Proves the TelemetryContext carrier substrate on the pinned
//! tracing 0.1.44 / tracing-core 0.1.36 semantics: frozen parent plus source
//! Dispatch (A-create, never poll-time ambient), scoped poll/drop sections
//! with no enter guard crossing await and no extra poll (B-poll, Pending,
//! unpolled drop), cross-thread and producer propagation, and noop
//! business-output preservation under a disabled scoped dispatcher. This
//! substrate does not close actual SDK propagation.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use futures_util::task::noop_waker;
use opi_ai::telemetry::TelemetryContext;
use tracing::Level;
use tracing_core::dispatcher::{self, Dispatch};
use tracing_core::span::{Attributes, Id, Record};
use tracing_core::{Event, Subscriber};

// ---------------------------------------------------------------------------
// In-memory recording subscriber: no exporter, no global subscriber.
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Recorder {
    spans: Mutex<Vec<&'static str>>,
}

struct RecordingSubscriber {
    recorder: Arc<Recorder>,
}

impl Subscriber for RecordingSubscriber {
    fn enabled(&self, _metadata: &tracing_core::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, attributes: &Attributes<'_>) -> Id {
        // The carrier-captured parent span (named "parent") is created under
        // the recording dispatch by the capture fixture itself; it is not a
        // propagation probe, so the fixture records only other spans.
        let name = attributes.metadata().name();
        if name != "parent" {
            self.recorder.spans.lock().unwrap().push(name);
        }
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, _event: &Event<'_>) {}

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

fn recorder() -> (Arc<Recorder>, Dispatch) {
    let recorder = Arc::new(Recorder::default());
    let dispatch = Dispatch::new(RecordingSubscriber {
        recorder: recorder.clone(),
    });
    (recorder, dispatch)
}

/// Freeze a context by creating it under a dedicated recording dispatch.
fn captured_context() -> (Arc<Recorder>, TelemetryContext) {
    let (recorder, dispatch) = recorder();
    let context = dispatcher::with_default(&dispatch, || {
        TelemetryContext::from_span(tracing::span!(Level::INFO, "parent"))
    });
    (recorder, context)
}

/// Future probe: counts polls, creates a span each poll, pends until the
/// `ready_after`-th poll, then yields 7.
struct Counting {
    polls: Arc<AtomicUsize>,
    ready_after: usize,
}

impl Future for Counting {
    type Output = u8;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<u8> {
        let this = self.get_mut();
        let seen = this.polls.fetch_add(1, Ordering::SeqCst) + 1;
        tracing::span!(Level::INFO, "poll_probe").in_scope(|| ());
        if seen >= this.ready_after {
            Poll::Ready(7)
        } else {
            Poll::Pending
        }
    }
}

/// Future probe: always pends; fires a span from Drop.
struct Droppy;

impl Future for Droppy {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        Poll::Pending
    }
}

impl Drop for Droppy {
    fn drop(&mut self) {
        tracing::span!(Level::INFO, "drop_probe").in_scope(|| ());
    }
}

// ---------------------------------------------------------------------------
// A-create: the dispatch is frozen at from_span time, never at later poll.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn from_span_captures_dispatch_at_creation_not_at_later_poll() {
    let (recorder, context) = captured_context();
    // Outside any with_default: the ambient default here routes nowhere, so
    // reaching the recorder proves the frozen dispatch, not poll-time ambient.
    let output = context
        .instrument(async {
            tracing::span!(Level::INFO, "inner_probe").in_scope(|| ());
            41
        })
        .await;
    assert_eq!(output, 41);
    assert_eq!(*recorder.spans.lock().unwrap(), vec!["inner_probe"]);
}

#[test]
fn in_scope_routes_to_frozen_dispatch_and_restores_caller() {
    let (frozen_recorder, context) = captured_context();
    let (ambient_recorder, ambient) = recorder();
    dispatcher::with_default(&ambient, || {
        context.in_scope(|| {
            tracing::span!(Level::INFO, "scoped_probe").in_scope(|| ());
        });
        // The caller context is restored: this later probe routes ambient.
        tracing::span!(Level::INFO, "caller_probe").in_scope(|| ());
    });
    assert_eq!(*frozen_recorder.spans.lock().unwrap(), vec!["scoped_probe"]);
    assert_eq!(
        *ambient_recorder.spans.lock().unwrap(),
        vec!["caller_probe"]
    );
}

// ---------------------------------------------------------------------------
// B-poll: one inner poll per outer poll, Pending preserved, scoped sections.
// ---------------------------------------------------------------------------

#[test]
fn instrument_polls_inner_once_per_poll_and_preserves_pending() {
    let (recorder, context) = captured_context();
    let polls = Arc::new(AtomicUsize::new(0));
    let mut future = context.instrument(Counting {
        polls: polls.clone(),
        ready_after: 2,
    });
    // Lazy: constructing the wrapper polls nothing.
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    assert!(recorder.spans.lock().unwrap().is_empty());

    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    assert!(Pin::new(&mut future).poll(&mut cx).is_pending());
    assert_eq!(polls.load(Ordering::SeqCst), 1, "no extra poll on Pending");
    assert_eq!(*recorder.spans.lock().unwrap(), vec!["poll_probe"]);
    match Pin::new(&mut future).poll(&mut cx) {
        Poll::Ready(value) => assert_eq!(value, 7),
        Poll::Pending => panic!("second poll must be ready"),
    }
    assert_eq!(polls.load(Ordering::SeqCst), 2);
}

// ---------------------------------------------------------------------------
// Drop: unpolled wrappers still scope the drop of the wrapped future.
// ---------------------------------------------------------------------------

#[test]
fn unpolled_instrumented_drop_is_scoped() {
    let (recorder, context) = captured_context();
    {
        let future = context.instrument(Droppy);
        let _ = &future; // never polled
    }
    assert_eq!(*recorder.spans.lock().unwrap(), vec!["drop_probe"]);
}

// ---------------------------------------------------------------------------
// Cross-thread and producer propagation.
// ---------------------------------------------------------------------------

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn clone_moves_cross_thread_to_same_frozen_dispatch() {
    assert_send_sync::<TelemetryContext>();
    let (recorder, context) = captured_context();
    let moved = context.clone();
    let handle = std::thread::spawn(move || {
        moved.in_scope(|| {
            tracing::span!(Level::INFO, "cross_probe").in_scope(|| ());
        });
    });
    handle.join().unwrap();
    assert_eq!(*recorder.spans.lock().unwrap(), vec!["cross_probe"]);
}

#[tokio::test]
async fn producer_task_propagates_frozen_context() {
    let (recorder, context) = captured_context();
    let handle = tokio::spawn(context.instrument(async {
        tracing::span!(Level::INFO, "producer_probe").in_scope(|| ());
        5
    }));
    assert_eq!(handle.await.unwrap(), 5);
    assert_eq!(*recorder.spans.lock().unwrap(), vec!["producer_probe"]);
}

// ---------------------------------------------------------------------------
// Noop: business work runs exactly once, output preserved, observation
// shadowed by a disabled scoped dispatcher, caller context untouched.
// ---------------------------------------------------------------------------

#[test]
fn noop_in_scope_runs_business_once_and_shadows_observation() {
    let context = TelemetryContext::noop();
    let calls = Arc::new(AtomicUsize::new(0));
    let probe_calls = calls.clone();
    let (ambient_recorder, ambient) = recorder();
    let output = dispatcher::with_default(&ambient, move || {
        context.in_scope(move || {
            probe_calls.fetch_add(1, Ordering::SeqCst);
            tracing::span!(Level::INFO, "must_not_route").in_scope(|| ());
            3
        })
    });
    assert_eq!(output, 3);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "business work exactly once"
    );
    assert!(
        ambient_recorder.spans.lock().unwrap().is_empty(),
        "noop scope must shadow ambient observation"
    );
}

#[test]
fn noop_instrument_preserves_output_and_shadows_events() {
    let context = TelemetryContext::noop();
    let (ambient_recorder, ambient) = recorder();
    let polls = Arc::new(AtomicUsize::new(0));
    let mut future = context.instrument(Counting {
        polls: polls.clone(),
        ready_after: 1,
    });
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    let polled = dispatcher::with_default(&ambient, || Pin::new(&mut future).poll(&mut cx));
    assert_eq!(polled, Poll::Ready(7));
    assert_eq!(
        polls.load(Ordering::SeqCst),
        1,
        "business work exactly once"
    );
    assert!(
        ambient_recorder.spans.lock().unwrap().is_empty(),
        "noop scope must shadow ambient observation"
    );
}

// ---------------------------------------------------------------------------
// Disabled span degrades to the explicit noop state.
// ---------------------------------------------------------------------------

#[test]
fn disabled_span_degrades_to_noop() {
    let (ambient_recorder, ambient) = recorder();
    let context = dispatcher::with_default(&ambient, || {
        TelemetryContext::from_span(tracing::Span::none())
    });
    let output = context.in_scope(|| {
        tracing::span!(Level::INFO, "must_not_route").in_scope(|| ());
        9
    });
    assert_eq!(output, 9);
    assert!(
        ambient_recorder.spans.lock().unwrap().is_empty(),
        "disabled span must behave as noop"
    );
}
