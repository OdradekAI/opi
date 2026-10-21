use crate::ws::io::{CloseState, Driver, Fault};
use std::{
    future::{Future, poll_fn},
    pin::Pin,
    task::Poll,
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn pending<F: Future>(future: &mut Pin<Box<F>>) {
    assert!(poll_fn(|cx| Poll::Ready(future.as_mut().poll(cx).is_pending())).await);
}
fn masked(opcode: u8, payload: &[u8], mask: [u8; 4]) -> Vec<u8> {
    let mut wire = vec![0x80 | opcode];
    if payload.len() < 126 {
        wire.push(0x80 | payload.len() as u8);
    } else {
        wire.push(0xfe);
        wire.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    }
    wire.extend_from_slice(&mask);
    wire.extend(payload.iter().enumerate().map(|(i, b)| *b ^ mask[i % 4]));
    wire
}
fn peer_close(payload: &[u8]) -> Vec<u8> {
    let mut wire = vec![0x88, payload.len() as u8];
    wire.extend_from_slice(payload);
    wire
}

#[tokio::test]
async fn close_timeout_discards_retained_receive_event_permanently() {
    let (client, mut peer) = tokio::io::duplex(64);
    let mut driver = Driver::new(client, None).unwrap();
    driver.queue(8, &[], false, [1; 4]).unwrap();
    peer.write_all(&[0x81, 1, b'x']).await.unwrap();
    assert!(matches!(
        driver.finish_close(None).await,
        Err(Fault::CloseReceivePending)
    ));
    assert!(matches!(
        driver.finish_close(Some(tokio::time::Instant::now())).await,
        Err(Fault::CloseTimeout)
    ));
    assert_eq!(driver.close_state(), CloseState::Failed);
    for _ in 0..2 {
        assert!(driver.next().await.unwrap().is_none());
    }
    let mut sent_close = [0; 6];
    peer.read_exact(&mut sent_close).await.unwrap();
    assert_eq!(sent_close.as_slice(), masked(8, &[], [1; 4]));
    assert_eq!(peer.read(&mut [0]).await.unwrap(), 0);
    assert!(matches!(
        driver.finish_close(None).await,
        Err(Fault::Closed)
    ));
}

#[tokio::test]
async fn peer_close_requires_explicit_masked_response_empty_stays_empty() {
    let (client, mut peer) = tokio::io::duplex(16);
    let mut driver = Driver::new(client, None).unwrap();
    peer.write_all(&[0x88, 0]).await.unwrap();
    let close = driver.next().await.unwrap().unwrap();
    assert!(close.payload.is_empty());
    assert_eq!(driver.close_state(), CloseState::PeerCloseReceived);
    assert!(!driver.is_terminal());
    assert!(driver.next().await.unwrap().is_none());
    assert!(matches!(
        driver.finish_close(None).await,
        Err(Fault::CloseResponseRequired)
    ));
    assert!(!driver.is_terminal());
    let mask = [5, 1, 9, 2];
    driver.queue(8, &close.payload, false, mask).unwrap();
    assert_eq!(driver.close_state(), CloseState::ResponseQueued);
    assert!(matches!(
        driver.queue(8, &[], false, [2; 4]),
        Err(Fault::Closed)
    ));
    let server = async {
        let mut got = [0; 6];
        peer.read_exact(&mut got).await.unwrap();
        assert_eq!(got.as_slice(), masked(8, &[], mask));
        peer.shutdown().await.unwrap();
        assert_eq!(peer.read(&mut [0]).await.unwrap(), 0);
    };
    let ((), result) = tokio::join!(server, driver.finish_close(None));
    result.unwrap();
    assert_eq!(driver.close_state(), CloseState::Closed);
    assert!(driver.is_terminal());
    assert!(driver.next().await.unwrap().is_none());
    driver.finish_close(None).await.unwrap();
}

#[tokio::test]
async fn response_preserves_close_code_reason_and_does_not_encode_status_1005() {
    let payload = [3, 232, b'o', b'k'];
    let mask = [4, 8, 2, 6];
    let (client, mut peer) = tokio::io::duplex(32);
    let mut driver = Driver::new(client, None).unwrap();
    peer.write_all(&peer_close(&payload)).await.unwrap();
    let event = driver.next().await.unwrap().unwrap();
    assert_eq!(event.payload, payload);
    driver.queue(8, &event.payload, false, mask).unwrap();
    let server = async {
        let mut wire = [0; 10];
        peer.read_exact(&mut wire).await.unwrap();
        assert_eq!(wire.as_slice(), masked(8, &payload, mask));
        peer.shutdown().await.unwrap();
    };
    let ((), result) = tokio::join!(server, driver.finish_close(None));
    result.unwrap();
}

#[tokio::test]
async fn local_close_waits_for_peer_close_and_transport_eof_across_cancellation() {
    let (client, mut peer) = tokio::io::duplex(32);
    let mut driver = Driver::new(client, None).unwrap();
    let mask = [7, 6, 5, 4];
    assert!(matches!(
        driver.finish_close(None).await,
        Err(Fault::CloseNotStarted)
    ));
    driver.queue(8, &[], false, mask).unwrap();
    let mut future = Box::pin(driver.finish_close(None));
    pending(&mut future).await;
    drop(future);
    assert_eq!(driver.close_state(), CloseState::LocalCloseSent);
    let mut wire = [0; 6];
    peer.read_exact(&mut wire).await.unwrap();
    assert_eq!(wire.as_slice(), masked(8, &[], mask));
    peer.write_all(&[0x88, 0]).await.unwrap();
    assert_eq!(driver.next().await.unwrap().unwrap().opcode, 8);
    let mut future = Box::pin(driver.finish_close(None));
    pending(&mut future).await;
    drop(future);
    assert_eq!(driver.close_state(), CloseState::HandshakeComplete);
    assert!(!driver.is_terminal());
    assert_eq!(
        peer.read(&mut [0]).await.unwrap(),
        0,
        "write-half shutdown already retained"
    );
    peer.shutdown().await.unwrap();
    driver.finish_close(None).await.unwrap();
    assert_eq!(driver.close_state(), CloseState::Closed);
    assert_eq!(
        peer.read(&mut [0]).await.unwrap(),
        0,
        "no duplicate Close on resume"
    );
}

#[tokio::test]
async fn simultaneous_close_sends_exactly_one_frame() {
    let (client, mut peer) = tokio::io::duplex(32);
    let mut driver = Driver::new(client, None).unwrap();
    let payload = [3, 232];
    let mask = [1, 3, 5, 7];
    driver.queue(8, &payload, false, mask).unwrap();
    peer.write_all(&peer_close(&[3, 233])).await.unwrap();
    assert_eq!(driver.next().await.unwrap().unwrap().payload, [3, 233]);
    assert!(matches!(
        driver.queue(8, &[3, 233], false, [9; 4]),
        Err(Fault::Closed)
    ));
    let server = async {
        let mut bytes = [0; 8];
        peer.read_exact(&mut bytes).await.unwrap();
        assert_eq!(bytes.as_slice(), masked(8, &payload, mask));
        peer.shutdown().await.unwrap();
        assert_eq!(peer.read(&mut [0]).await.unwrap(), 0);
    };
    let ((), result) = tokio::join!(server, driver.finish_close(None));
    result.unwrap();
}

#[tokio::test]
async fn peer_close_discards_only_completely_unsent_application_frame() {
    let (client, mut peer) = tokio::io::duplex(16);
    let mut driver = Driver::new(client, None).unwrap();
    driver
        .queue(2, b"must never be sent", false, [8; 4])
        .unwrap();
    peer.write_all(&[0x88, 0]).await.unwrap();
    driver.next().await.unwrap();
    assert_eq!(driver.pending_write_len(), 0);
    driver.queue(8, &[], false, [3; 4]).unwrap();
    let server = async {
        let mut wire = [0; 6];
        peer.read_exact(&mut wire).await.unwrap();
        assert_eq!(wire.as_slice(), masked(8, &[], [3; 4]));
        peer.shutdown().await.unwrap();
    };
    let ((), result) = tokio::join!(server, driver.finish_close(None));
    result.unwrap();
}

#[tokio::test]
async fn peer_close_finishes_partial_application_frame_then_exact_response() {
    let (client, mut peer) = tokio::io::duplex(16);
    let mut driver = Driver::new(client, None).unwrap();
    let body = vec![73; 200];
    let mask = [1, 2, 3, 4];
    driver.queue(2, &body, false, mask).unwrap();
    let mut future = Box::pin(driver.flush());
    pending(&mut future).await;
    drop(future);
    assert_eq!(driver.write_offset(), 16);
    peer.write_all(&[0x88, 0]).await.unwrap();
    driver.next().await.unwrap();
    driver.queue(8, &[], false, [4, 3, 2, 1]).unwrap();
    let retained = driver.pending_write_len();
    assert!(matches!(
        driver.queue(2, b"late", false, [0; 4]),
        Err(Fault::Closed)
    ));
    assert_eq!(driver.pending_write_len(), retained);
    assert_eq!(driver.write_offset(), 16);
    let mut expected = masked(2, &body, mask);
    expected.extend_from_slice(&masked(8, &[], [4, 3, 2, 1]));
    let server = async {
        let mut actual = vec![0; expected.len()];
        peer.read_exact(&mut actual).await.unwrap();
        assert_eq!(actual, expected);
        peer.shutdown().await.unwrap();
    };
    let ((), result) = tokio::join!(server, driver.finish_close(None));
    result.unwrap();
}

#[tokio::test]
async fn peer_eof_before_response_is_abnormal_not_close_success() {
    let (client, mut peer) = tokio::io::duplex(16);
    let mut driver = Driver::new(client, None).unwrap();
    peer.write_all(&[0x88, 0]).await.unwrap();
    peer.shutdown().await.unwrap();
    driver.next().await.unwrap();
    driver.queue(8, &[], false, [7; 4]).unwrap();
    assert!(matches!(
        driver.finish_close(None).await,
        Err(Fault::PeerEofBeforeCloseSent)
    ));
    assert_eq!(driver.close_state(), CloseState::Failed);
    assert!(driver.next().await.unwrap().is_none());
    assert_eq!(
        peer.read(&mut [0]).await.unwrap(),
        0,
        "no falsely completed response"
    );
}

#[tokio::test(start_paused = true)]
async fn caller_deadline_bounds_missing_peer_and_is_absorbing() {
    let (client, mut peer) = tokio::io::duplex(16);
    let mut driver = Driver::new(client, None).unwrap();
    driver.queue(8, &[], false, [2; 4]).unwrap();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(7);
    assert!(matches!(
        driver.finish_close(Some(deadline)).await,
        Err(Fault::CloseTimeout)
    ));
    assert!(driver.is_terminal());
    assert_eq!(driver.close_state(), CloseState::Failed);
    assert!(driver.next().await.unwrap().is_none());
    assert!(driver.next().await.unwrap().is_none());
    let mut wire = Vec::new();
    peer.read_to_end(&mut wire).await.unwrap();
    assert_eq!(wire, masked(8, &[], [2; 4]));
}

#[tokio::test(start_paused = true)]
async fn caller_deadline_bounds_response_backpressure_without_hidden_remainder() {
    let (client, mut peer) = tokio::io::duplex(8);
    let mut driver = Driver::new(client, None).unwrap();
    peer.write_all(&[0x88, 0]).await.unwrap();
    driver.next().await.unwrap();
    let mut payload = vec![3, 232];
    payload.extend_from_slice(&[b'x'; 100]);
    driver.queue(8, &payload, false, [6; 4]).unwrap();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    assert!(matches!(
        driver.finish_close(Some(deadline)).await,
        Err(Fault::CloseTimeout)
    ));
    let mut wire = Vec::new();
    peer.read_to_end(&mut wire).await.unwrap();
    assert_eq!(wire, masked(8, &payload, [6; 4])[..8]);
    assert_eq!(driver.pending_write_len(), 0);
    assert!(driver.next().await.unwrap().is_none());
}

#[tokio::test]
async fn malformed_close_is_error_once_then_permanent_eof() {
    for wire in [
        vec![0x88, 1, 1],
        vec![0x88, 2, 3, 237],
        vec![0x88, 3, 3, 232, 255],
        vec![0x08, 0],
        vec![0xc8, 0],
    ] {
        let (client, mut peer) = tokio::io::duplex(16);
        let mut driver = Driver::new(client, Some(Default::default())).unwrap();
        peer.write_all(&wire).await.unwrap();
        assert!(driver.next().await.is_err(), "accepted {wire:x?}");
        assert_eq!(driver.close_state(), CloseState::Failed);
        assert!(driver.next().await.unwrap().is_none());
        assert!(driver.next().await.unwrap().is_none());
        assert_eq!(peer.read(&mut [0]).await.unwrap(), 0);
    }
}

#[tokio::test]
async fn post_close_data_cannot_be_delivered_or_turn_into_false_completion() {
    let (client, mut peer) = tokio::io::duplex(32);
    let mut driver = Driver::new(client, None).unwrap();
    driver.queue(8, &[], false, [0; 4]).unwrap();
    driver.flush().await.unwrap();
    peer.write_all(&[0x88, 0, 0x81, 1, b'x']).await.unwrap();
    assert_eq!(driver.next().await.unwrap().unwrap().opcode, 8);
    assert!(driver.next().await.unwrap().is_none());
    assert!(driver.finish_close(None).await.is_err());
    assert_eq!(driver.close_state(), CloseState::Failed);
}

#[tokio::test]
async fn owner_drop_after_peer_close_does_not_send_implicit_ack() {
    let (client, mut peer) = tokio::io::duplex(16);
    let mut driver = Driver::new(client, None).unwrap();
    peer.write_all(&[0x88, 0]).await.unwrap();
    driver.next().await.unwrap();
    drop(driver);
    assert_eq!(peer.read(&mut [0]).await.unwrap(), 0);
}

#[tokio::test]
async fn finish_close_retains_inflight_application_and_close_events() {
    let (client, mut peer) = tokio::io::duplex(32);
    let mut driver = Driver::new(client, None).unwrap();
    driver.queue(8, &[], false, [9; 4]).unwrap();
    peer.write_all(&[0x81, 1, b'x', 0x88, 0]).await.unwrap();
    peer.shutdown().await.unwrap();
    assert!(matches!(
        driver.finish_close(None).await,
        Err(Fault::CloseReceivePending)
    ));
    assert!(!driver.is_terminal());
    assert!(matches!(
        driver.finish_close(None).await,
        Err(Fault::CloseReceivePending)
    ));
    assert_eq!(driver.next().await.unwrap().unwrap().payload, b"x");
    driver.finish_close(None).await.unwrap();
    assert_eq!(driver.next().await.unwrap().unwrap().opcode, 8);
    assert!(driver.next().await.unwrap().is_none());
}

#[tokio::test]
async fn cancelled_pending_flush_with_all_close_bytes_written_accepts_peer_reply_eof() {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        task::Context,
    };
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
    struct Gate {
        inner: tokio::io::DuplexStream,
        blocked: Arc<AtomicBool>,
    }
    impl AsyncRead for Gate {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.inner).poll_read(cx, buf)
        }
    }
    impl AsyncWrite for Gate {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            Pin::new(&mut self.inner).poll_write(cx, buf)
        }
        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            if self.blocked.load(Ordering::SeqCst) {
                Poll::Pending
            } else {
                Pin::new(&mut self.inner).poll_flush(cx)
            }
        }
        fn poll_shutdown(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.inner).poll_shutdown(cx)
        }
    }
    let (client, mut peer) = tokio::io::duplex(16);
    let blocked = Arc::new(AtomicBool::new(true));
    let mut driver = Driver::new(
        Gate {
            inner: client,
            blocked: blocked.clone(),
        },
        None,
    )
    .unwrap();
    driver.queue(8, &[], false, [4; 4]).unwrap();
    let mut future = Box::pin(driver.flush());
    pending(&mut future).await;
    drop(future);
    assert_eq!(driver.write_offset(), 6);
    assert_eq!(driver.close_state(), CloseState::LocalCloseQueued);
    let mut wire = [0; 6];
    peer.read_exact(&mut wire).await.unwrap();
    assert_eq!(wire.as_slice(), masked(8, &[], [4; 4]));
    peer.write_all(&[0x88, 0]).await.unwrap();
    peer.shutdown().await.unwrap();
    assert_eq!(driver.next().await.unwrap().unwrap().opcode, 8);
    blocked.store(false, Ordering::SeqCst);
    driver.finish_close(None).await.unwrap();
    assert_eq!(driver.close_state(), CloseState::Closed);
    assert_eq!(peer.read(&mut [0]).await.unwrap(), 0);
}
