//! Isolated research only: safe pinned wrappers, not an opi-ai implementation.
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, atomic::{AtomicU64, AtomicUsize, Ordering}},
    task::{Context, Poll, Wake, Waker},
    thread::ThreadId,
};
use futures_core::Stream;
use tracing::{Dispatch, Span, Subscriber};
use tracing_core::{span::{Attributes, Current, Id, Record}, Event, Metadata};

#[derive(Clone)]
struct Carrier(Option<(Span, Dispatch)>);
impl Carrier {
    fn from_span(span: Span) -> Self {
        let dispatch = span.with_subscriber(|(_, dispatch)| dispatch.clone());
        Self(dispatch.map(|dispatch| (span, dispatch)))
    }
    fn noop() -> Self { Self(None) }
    fn scope<T>(&self, body: impl FnOnce() -> T) -> T {
        match &self.0 {
            Some((parent, dispatch)) => tracing::dispatcher::with_default(dispatch, || {
                let _entered = parent.enter();
                body()
            }),
            None => tracing::dispatcher::with_default(&Dispatch::none(), body),
        }
    }
    fn wrap<T>(&self, inner: T) -> Scoped<T> {
        Scoped { inner: Some(Box::pin(inner)), carrier: self.clone() }
    }
}
struct Scoped<T> { inner: Option<Pin<Box<T>>>, carrier: Carrier }
impl<T: Future> Future for Scoped<T> {
    type Output = T::Output;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T::Output> {
        let this = &mut *self;
        this.carrier.scope(|| this.inner.as_mut().expect("owned inner").as_mut().poll(cx))
    }
}
impl<T: Stream> Stream for Scoped<T> {
    type Item = T::Item;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T::Item>> {
        let this = &mut *self;
        this.carrier.scope(|| this.inner.as_mut().expect("owned inner").as_mut().poll_next(cx))
    }
}
impl<T> Drop for Scoped<T> {
    fn drop(&mut self) {
        let inner = self.inner.take();
        self.carrier.scope(|| drop(inner));
    }
}

#[derive(Default)]
struct State {
    spans: Vec<(u64, String, Option<u64>)>,
    entered: HashMap<ThreadId, Vec<Id>>,
}
#[derive(Clone, Default)]
struct Recorder { state: Arc<Mutex<State>>, ids: Arc<AtomicU64> }
impl Subscriber for Recorder {
    fn enabled(&self, _: &Metadata<'_>) -> bool { true }
    fn new_span(&self, attrs: &Attributes<'_>) -> Id {
        let id = self.ids.fetch_add(1, Ordering::SeqCst) + 1;
        let mut state = self.state.lock().unwrap();
        let parent = attrs.parent().cloned().or_else(|| {
            if attrs.is_contextual() {
                state.entered.get(&std::thread::current().id()).and_then(|v| v.last()).cloned()
            } else { None }
        });
        state.spans.push((id, attrs.metadata().name().to_owned(), parent.map(|v| v.into_u64())));
        Id::from_u64(id)
    }
    fn record(&self, _: &Id, _: &Record<'_>) {}
    fn record_follows_from(&self, _: &Id, _: &Id) {}
    fn event(&self, _: &Event<'_>) {}
    fn enter(&self, id: &Id) {
        self.state.lock().unwrap().entered.entry(std::thread::current().id()).or_default().push(id.clone());
    }
    fn exit(&self, id: &Id) {
        let popped = self.state.lock().unwrap().entered.entry(std::thread::current().id()).or_default().pop();
        assert_eq!(popped.as_ref(), Some(id));
    }
    fn current_span(&self) -> Current {
        // The probe's contextual child lookup is in new_span. No ambient capture is used.
        Current::none()
    }
}
fn mark() { let _span = tracing::info_span!("marker"); }
struct WakeNothing;
impl Wake for WakeNothing { fn wake(self: Arc<Self>) {} }
struct Work { polls: Arc<AtomicUsize>, drops: Arc<AtomicUsize>, stream: bool }
impl Future for Work {
    type Output = Result<u32, &'static str>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        mark();
        match self.polls.fetch_add(1, Ordering::SeqCst) {
            0 => Poll::Pending,
            _ => Poll::Ready(Err("functional-error")),
        }
    }
}
impl Stream for Work {
    type Item = Result<u32, &'static str>;
    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        assert!(self.stream);
        mark();
        match self.polls.fetch_add(1, Ordering::SeqCst) {
            0 => Poll::Pending,
            1 => Poll::Ready(Some(Err("functional-error"))),
            _ => Poll::Ready(None),
        }
    }
}
impl Drop for Work {
    fn drop(&mut self) { mark(); self.drops.fetch_add(1, Ordering::SeqCst); }
}
fn work(stream: bool) -> (Work, Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let polls = Arc::new(AtomicUsize::new(0));
    let drops = Arc::new(AtomicUsize::new(0));
    (Work { polls: polls.clone(), drops: drops.clone(), stream }, polls, drops)
}
fn assert_send_sync<T: Send + Sync>() {}
fn main() {
    assert_send_sync::<Carrier>();
    let rec_a = Recorder::default();
    let rec_b = Recorder::default();
    let a = Dispatch::new(rec_a.clone());
    let b = Dispatch::new(rec_b.clone());
    let parent = tracing::dispatcher::with_default(&a, || tracing::info_span!("parent_a"));
    let carrier = tracing::dispatcher::with_default(&b, || Carrier::from_span(parent));
    let parent_id = carrier.0.as_ref().unwrap().0.id().unwrap().into_u64();
    let waker = Waker::from(Arc::new(WakeNothing));
    let mut cx = Context::from_waker(&waker);
    let mut cases = Vec::new();
    tracing::dispatcher::with_default(&b, || {
        let mut calls = 0;
        assert_eq!(carrier.scope(|| { calls += 1; mark(); 17 }), 17);
        assert_eq!(calls, 1);
        cases.push("sync-source-dispatch-and-once");
        mark(); // must be restored to B
    });
    let (f, polls, drops) = work(false);
    let mut f = Box::pin(carrier.wrap(f));
    tracing::dispatcher::with_default(&b, || assert!(Future::poll(f.as_mut(), &mut cx).is_pending()));
    let carrier_for_thread = b.clone();
    let f = std::thread::spawn(move || {
        let w = Waker::from(Arc::new(WakeNothing));
        let mut cx = Context::from_waker(&w);
        tracing::dispatcher::with_default(&carrier_for_thread, || {
            assert_eq!(Future::poll(f.as_mut(), &mut cx), Poll::Ready(Err("functional-error")));
        });
        f
    }).join().unwrap();
    tracing::dispatcher::with_default(&b, || drop(f));
    assert_eq!(polls.load(Ordering::SeqCst), 2);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    cases.push("future-pending-cross-thread-error-drop");
    let (s, polls, drops) = work(true);
    let mut s = Box::pin(carrier.wrap(s));
    tracing::dispatcher::with_default(&b, || {
        assert!(Stream::poll_next(s.as_mut(), &mut cx).is_pending());
        assert_eq!(Stream::poll_next(s.as_mut(), &mut cx), Poll::Ready(Some(Err("functional-error"))));
        assert_eq!(Stream::poll_next(s.as_mut(), &mut cx), Poll::Ready(None));
        drop(s);
    });
    assert_eq!(polls.load(Ordering::SeqCst), 3);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    cases.push("stream-pending-item-error-eof-drop");
    let (f, polls, drops) = work(false);
    tracing::dispatcher::with_default(&b, || drop(carrier.wrap(f)));
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    cases.push("unpolled-drop");
    let before_a = rec_a.state.lock().unwrap().spans.len();
    let before_b = rec_b.state.lock().unwrap().spans.len();
    for noop in [Carrier::noop(), Carrier::from_span(Span::none())] {
        let (f, polls, drops) = work(false);
        let mut f = Box::pin(noop.wrap(f));
        tracing::dispatcher::with_default(&b, || {
            assert!(Future::poll(f.as_mut(), &mut cx).is_pending());
            assert_eq!(Future::poll(f.as_mut(), &mut cx), Poll::Ready(Err("functional-error")));
            drop(f);
        });
        assert_eq!(polls.load(Ordering::SeqCst), 2);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }
    assert_eq!(rec_a.state.lock().unwrap().spans.len(), before_a);
    assert_eq!(rec_b.state.lock().unwrap().spans.len(), before_b);
    cases.push("noop-and-disabled-no-ambient-business-preserved");
    let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(2).build().unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let task_carrier = carrier.clone();
    let handle = tracing::dispatcher::with_default(&b, || runtime.spawn(task_carrier.wrap(async move {
        mark();
        let _ = rx.await;
        mark();
        Err::<(), _>("producer-error")
    })));
    tx.send(()).unwrap();
    assert_eq!(runtime.block_on(handle).unwrap(), Err("producer-error"));
    cases.push("tokio-producer-cross-worker-output");
    let (f, polls, drops) = work(false);
    let (tx, rx) = std::sync::mpsc::channel();
    let carrier_abort = carrier.clone();
    let handle = runtime.spawn(carrier_abort.wrap(async move {
        let _guard = f;
        mark();
        tx.send(()).unwrap();
        std::future::pending::<()>().await;
    }));
    rx.recv().unwrap();
    handle.abort();
    assert!(runtime.block_on(handle).unwrap_err().is_cancelled());
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    cases.push("tokio-producer-abort-drop");
    drop(runtime);
    let records_a = &rec_a.state.lock().unwrap().spans;
    let records_b = &rec_b.state.lock().unwrap().spans;
    assert!(records_a.iter().filter(|(_, n, _)| n == "marker").all(|(_, _, p)| *p == Some(parent_id)));
    assert_eq!(records_b.len(), 1);
    assert!(records_b.iter().all(|(_, n, p)| n == "marker" && p.is_none()));
    println!("{}", serde_json::json!({"cases":cases,"a_spans":records_a,"b_spans":records_b,"unsafe":false,"result":"pass","scope":"isolated carrier mechanism, not product propagation"}));
}
