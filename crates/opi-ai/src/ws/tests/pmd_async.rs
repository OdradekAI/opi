use crate::ws::pmd::{Decoder, Encoder, Fault};
use std::{
    future::{Future, poll_fn},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

struct CountWake(AtomicUsize);
impl Wake for CountWake {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
fn counter() -> (Arc<CountWake>, Waker) {
    let counter = Arc::new(CountWake(AtomicUsize::new(0)));
    let waker = Waker::from(counter.clone());
    (counter, waker)
}
fn encoded(input: &[u8]) -> Vec<u8> {
    Encoder::new(15, false).unwrap().encode(input).unwrap()
}
fn assert_send<T: Send>(_: T) {}

#[test]
fn decoder_and_borrowed_decode_future_are_send() {
    assert_send(Decoder::new(false).unwrap());
    let mut decoder = Decoder::new(false).unwrap();
    decoder.start_decode(encoded(b"hello"), None).unwrap();
    assert_send(poll_fn(|cx| decoder.poll_decode(cx)));
}

#[test]
fn pending_job_moves_between_os_threads_and_retains_dictionary() {
    let plain = b"retained-history:".repeat(300_000);
    let mut encoder = Encoder::new(15, false).unwrap();
    let first = encoder.encode(&plain).unwrap();
    let next = encoder.encode(&plain).unwrap();
    let mut decoder = Decoder::new(false).unwrap();
    decoder.start_decode(first, None).unwrap();
    let (_, waker) = counter();
    assert!(
        decoder
            .poll_decode(&mut Context::from_waker(&waker))
            .is_pending()
    );
    assert!(decoder.buffered_output() > 0);
    let (mut decoder, decoded) = std::thread::spawn(move || {
        let (_, waker) = counter();
        let mut cx = Context::from_waker(&waker);
        let decoded = loop {
            if let Poll::Ready(value) = decoder.poll_decode(&mut cx) {
                break value.unwrap();
            }
        };
        (decoder, decoded)
    })
    .join()
    .unwrap();
    assert_eq!(decoded, plain);
    assert_eq!(decoder.decode(&next).unwrap(), plain);
    assert!(decoder.yields > 0);
    assert!(decoder.max_steps_per_poll <= 16);
    std::thread::spawn(move || drop(decoder)).join().unwrap();
}

#[test]
fn bfinal_pending_job_moves_threads_without_iteration_cap() {
    let mut wire = [1, 0, 0, 255, 255].repeat(100_001);
    wire.extend([1, 4, 0, 251, 255]);
    wire.extend(b"TAIL");
    wire.push(0);
    let mut decoder = Decoder::new(false).unwrap();
    decoder.start_decode(wire, None).unwrap();
    let (wake, waker) = counter();
    assert!(
        decoder
            .poll_decode(&mut Context::from_waker(&waker))
            .is_pending()
    );
    assert!(wake.0.load(Ordering::SeqCst) > 0);
    let decoder = std::thread::spawn(move || {
        let (_, waker) = counter();
        let mut cx = Context::from_waker(&waker);
        loop {
            if let Poll::Ready(value) = decoder.poll_decode(&mut cx) {
                assert_eq!(value.unwrap(), b"TAIL");
                break;
            }
        }
        assert_eq!(decoder.tail_injections, 1);
        assert!(decoder.resets >= 100_001);
        assert!(decoder.max_steps_per_poll <= 16);
        decoder
    })
    .join()
    .unwrap();
    assert!(!decoder.failed());
}

#[test]
fn final_stream_normalization_retains_history_across_thread_migration() {
    let plain = b"history-window:".repeat(1_000);
    let mut encoder = Encoder::new(15, false).unwrap();
    let _ = encoder.encode(&plain).unwrap();
    let dependent = encoder.encode(&plain).unwrap();
    assert!(
        Decoder::new(false).unwrap().decode(&dependent).is_err(),
        "control must actually depend on prior message history"
    );
    let mut wire = [1, 0, 0, 255, 255].repeat(100);
    wire.push(1);
    let n = plain.len() as u16;
    wire.extend(n.to_le_bytes());
    wire.extend((!n).to_le_bytes());
    wire.extend(&plain);
    wire.push(0);
    let mut decoder = Decoder::new(false).unwrap();
    decoder.start_decode(wire, None).unwrap();
    let (_, waker) = counter();
    assert!(
        decoder
            .poll_decode(&mut Context::from_waker(&waker))
            .is_pending()
    );
    let (mut decoder, first) = std::thread::spawn(move || {
        let (_, waker) = counter();
        let mut cx = Context::from_waker(&waker);
        let first = loop {
            if let Poll::Ready(value) = decoder.poll_decode(&mut cx) {
                break value.unwrap();
            }
        };
        (decoder, first)
    })
    .join()
    .unwrap();
    assert_eq!(first, plain);
    assert_eq!(decoder.decode(&dependent).unwrap(), plain);
    assert!(decoder.resets >= 101);
}

#[test]
fn explicit_cancel_drops_partial_output_and_stream_and_absorbs() {
    for wire in [
        encoded(&vec![b'x'; 4_000_000]),
        [1, 0, 0, 255, 255].repeat(100_001),
    ] {
        let mut decoder = Decoder::new(false).unwrap();
        decoder.start_decode(wire, None).unwrap();
        let (_, waker) = counter();
        let mut cx = Context::from_waker(&waker);
        assert!(decoder.poll_decode(&mut cx).is_pending());
        assert!(decoder.job_pending());
        decoder.cancel();
        assert!(!decoder.job_pending());
        assert_eq!(decoder.buffered_output(), 0);
        assert!(decoder.released());
        assert!(decoder.failed());
        assert!(matches!(
            decoder.poll_decode(&mut cx),
            Poll::Ready(Err(Fault::Terminal))
        ));
        assert!(matches!(
            decoder.start_decode(vec![0], None),
            Err(Fault::Terminal)
        ));
        decoder.cancel();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn dropping_borrowed_future_leaves_resumable_owned_job() {
    let plain = vec![b'x'; 3_000_000];
    let mut decoder = Decoder::new(false).unwrap();
    decoder.start_decode(encoded(&plain), None).unwrap();
    let (_, waker) = counter();
    {
        let mut future = Box::pin(poll_fn(|cx| decoder.poll_decode(cx)));
        assert!(
            future
                .as_mut()
                .poll(&mut Context::from_waker(&waker))
                .is_pending()
        );
    }
    assert!(decoder.job_pending());
    assert!(matches!(
        decoder.start_decode(vec![0], None),
        Err(Fault::Busy)
    ));
    assert!(!decoder.failed());
    let result = poll_fn(|cx| decoder.poll_decode(cx)).await.unwrap();
    assert_eq!(result, plain);
    assert!(!decoder.job_pending());
    assert_eq!(decoder.decode(&[0]).unwrap(), b"");
}

#[tokio::test(flavor = "current_thread")]
async fn decompression_yields_to_another_ready_task() {
    let plain = vec![b'z'; 16_000_000];
    let mut decoder = Decoder::new(false).unwrap();
    decoder.start_decode(encoded(&plain), None).unwrap();
    let ticks = Arc::new(AtomicUsize::new(0));
    let observed = ticks.clone();
    let ticker = tokio::spawn(async move {
        loop {
            observed.fetch_add(1, Ordering::SeqCst);
            tokio::task::yield_now().await;
        }
    });
    let result = poll_fn(|cx| decoder.poll_decode(cx)).await.unwrap();
    ticker.abort();
    assert_eq!(result, plain);
    assert!(
        ticks.load(Ordering::SeqCst) > 1,
        "one poll must not monopolize the executor"
    );
    assert!(decoder.yields > 1);
    assert!(decoder.max_steps_per_poll <= 16);
}

#[tokio::test(flavor = "current_thread")]
async fn another_task_cancels_active_decompression_before_publication() {
    let mut decoder = Decoder::new(false).unwrap();
    decoder
        .start_decode(encoded(&vec![b'x'; 16_000_000]), None)
        .unwrap();
    let cancellation = decoder.cancellation_flag();
    let cancel = tokio::spawn(async move {
        tokio::task::yield_now().await;
        cancellation.store(true, Ordering::Release);
    });
    assert!(matches!(
        poll_fn(|cx| decoder.poll_decode(cx)).await,
        Err(Fault::Cancelled)
    ));
    cancel.await.unwrap();
    assert!(decoder.yields > 0);
    assert!(decoder.released());
    assert_eq!(decoder.buffered_output(), 0);
    assert!(!decoder.job_pending());
    assert!(matches!(decoder.decode(&[0]), Err(Fault::Terminal)));
}

#[test]
fn malformed_tail_after_many_polls_poisoning_and_limits_remain_distinct() {
    let mut wire = [1, 0, 0, 255, 255].repeat(1_000);
    wire.extend([0, 0]);
    let mut decoder = Decoder::new(false).unwrap();
    decoder.start_decode(wire, None).unwrap();
    let (_, waker) = counter();
    let mut cx = Context::from_waker(&waker);
    let failure = loop {
        if let Poll::Ready(value) = decoder.poll_decode(&mut cx) {
            break value.unwrap_err();
        }
    };
    assert!(matches!(failure, Fault::Boundary));
    assert!(decoder.yields > 1);
    assert_eq!(decoder.tail_injections, 0);
    assert!(decoder.released());
    let plain = vec![b'z'; 3_000_000];
    let mut limited = Decoder::new(false).unwrap();
    limited
        .start_decode(encoded(&plain), Some(2_000_000))
        .unwrap();
    let failure = loop {
        if let Poll::Ready(value) = limited.poll_decode(&mut cx) {
            break value.unwrap_err();
        }
    };
    assert!(matches!(failure, Fault::Limit));
    assert!(limited.yields > 1);
    assert!(limited.released());
}

#[test]
fn input_quantum_edges_preserve_large_incompressible_payload_and_reject_truncation() {
    let mut seed = 0x1234_5678u32;
    let plain: Vec<u8> = (0..1_000_000)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed as u8
        })
        .collect();
    let wire = encoded(&plain);
    assert!(wire.len() > 900_000);
    let mut decoder = Decoder::new(false).unwrap();
    decoder.start_decode(wire.clone(), None).unwrap();
    let (_, waker) = counter();
    let mut cx = Context::from_waker(&waker);
    let actual = loop {
        if let Poll::Ready(value) = decoder.poll_decode(&mut cx) {
            break value.unwrap();
        }
    };
    assert_eq!(actual, plain);
    assert!(decoder.yields > 200);
    assert!(decoder.max_steps_per_poll <= 16);
    for n in [256usize, 257, 4095, 4096, wire.len() - 1] {
        assert!(Decoder::new(false).unwrap().decode(&wire[..n]).is_err());
    }
}
