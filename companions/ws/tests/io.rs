use opi_ws::{
    frame::Message,
    io::{Core, Driver, Fault},
    pmd,
};
use std::{
    future::{Future, poll_fn},
    task::Poll,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn driver_outbound_respects_negotiated_window() {
    // A repeated, otherwise poorly compressible span tempts a 15-bit encoder
    // to use distances outside the negotiated 10-bit dictionary.
    let mut state = 1u32;
    let mut plain = Vec::new();
    for _ in 0..1800 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        plain.push(state as u8);
    }
    plain.extend_from_within(..);
    let (client, mut peer) = tokio::io::duplex(8192);
    let mut driver = Driver::new(
        client,
        Some(opi_ws::io::PmdParameters {
            client_max_window_bits: 10,
            ..Default::default()
        }),
    )
    .unwrap();
    driver.queue(2, &plain, true, [3; 4]).unwrap();
    driver.flush().await.unwrap();
    let mut head = [0; 4];
    peer.read_exact(&mut head).await.unwrap();
    assert_eq!(&head[..2], &[0xc2, 0xfe]);
    let length = u16::from_be_bytes(head[2..].try_into().unwrap()) as usize;
    let mut mask = [0; 4];
    peer.read_exact(&mut mask).await.unwrap();
    let mut wire = vec![0; length];
    peer.read_exact(&mut wire).await.unwrap();
    for (i, byte) in wire.iter_mut().enumerate() {
        *byte ^= mask[i % 4];
    }
    wire.extend_from_slice(&[0, 0, 255, 255]);
    // Independent raw inflater, with small output slices forcing dictionary
    // lookups through the negotiated window rather than one large output slice.
    let mut inflater = flate2::Decompress::new_with_window_bits(false, 10);
    let mut decoded = Vec::new();
    while inflater.total_in() < wire.len() as u64 {
        let mut chunk = [0; 128];
        let before = inflater.total_out();
        inflater
            .decompress(
                &wire[inflater.total_in() as usize..],
                &mut chunk,
                flate2::FlushDecompress::Sync,
            )
            .expect("outbound stream exceeded the negotiated 10-bit window");
        decoded.extend_from_slice(&chunk[..(inflater.total_out() - before) as usize]);
    }
    assert_eq!(decoded, plain);
}

fn server_frame(first: u8, payload: &[u8]) -> Vec<u8> {
    assert!(payload.len() < 126);
    let mut out = vec![first, payload.len() as u8];
    out.extend_from_slice(payload);
    out
}

async fn poll_pending<F: Future>(future: &mut std::pin::Pin<Box<F>>) {
    assert!(poll_fn(|cx| Poll::Ready(future.as_mut().poll(cx).is_pending())).await);
}

#[tokio::test]
async fn cancelled_read_future_resumes_header_payload_and_pmd() {
    let mut encoder = pmd::Encoder::new(15, false).unwrap();
    let compressed = encoder.encode("分片恢复 canary".as_bytes()).unwrap();
    let compressed_wire = server_frame(0xc1, &compressed);
    for (wire, prefix, expected, pmd) in [
        (server_frame(0x81, b"header"), 1, b"header".to_vec(), false),
        (
            server_frame(0x81, b"payload"),
            5,
            b"payload".to_vec(),
            false,
        ),
        (
            compressed_wire,
            5,
            "分片恢复 canary".as_bytes().to_vec(),
            true,
        ),
    ] {
        let (client, mut peer) = tokio::io::duplex(16);
        let mut driver = Driver::new(client, pmd.then_some(Default::default())).unwrap();
        peer.write_all(&wire[..prefix]).await.unwrap();
        let mut future = Box::pin(driver.next());
        poll_pending(&mut future).await;
        drop(future);
        let (sent, received) = tokio::join!(peer.write_all(&wire[prefix..]), driver.next());
        sent.unwrap();
        let message = received.unwrap().unwrap();
        assert_eq!(message.opcode, 1);
        assert!(!message.compressed);
        assert_eq!(message.payload, expected);
        peer.write_all(&[0x88, 0]).await.unwrap();
        assert_eq!(driver.next().await.unwrap().unwrap().opcode, 8);
        assert!(driver.next().await.unwrap().is_none());
    }
}

#[tokio::test]
async fn dropping_owner_after_cancelled_read_closes_peer() {
    let (client, mut peer) = tokio::io::duplex(16);
    let mut driver = Driver::new(client, Some(Default::default())).unwrap();
    peer.write_all(&[0x41, 3, 0xca]).await.unwrap();
    let mut future = Box::pin(driver.next());
    poll_pending(&mut future).await;
    drop(future);
    drop(driver);
    assert_eq!(peer.read(&mut [0; 1]).await.unwrap(), 0);
}

#[tokio::test]
async fn malformed_frame_pmd_and_early_eof_error_once_then_permanent_none() {
    for wire in [
        vec![0xc9, 0],
        vec![0x81],
        vec![0x81, 3, b'a'],
        vec![0xc1, 1, 0xca],
    ] {
        let (client, mut peer) = tokio::io::duplex(16);
        let mut driver = Driver::new(client, Some(Default::default())).unwrap();
        peer.write_all(&wire).await.unwrap();
        peer.shutdown().await.unwrap();
        assert!(driver.next().await.is_err(), "accepted {wire:02x?}");
        assert!(driver.next().await.unwrap().is_none());
        assert!(driver.next().await.unwrap().is_none());
        assert!(driver.is_terminal());
        assert_eq!(peer.read(&mut [0; 1]).await.unwrap(), 0);
    }
}

fn masked(first: u8, payload: &[u8], mask: [u8; 4]) -> Vec<u8> {
    let mut out = vec![first];
    match payload.len() {
        0..=125 => out.push(0x80 | payload.len() as u8),
        126..=65535 => {
            out.push(0xfe);
            out.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        }
        _ => {
            out.push(0xff);
            out.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }
    }
    out.extend_from_slice(&mask);
    out.extend(
        payload
            .iter()
            .enumerate()
            .map(|(i, byte)| *byte ^ mask[i % 4]),
    );
    out
}

#[tokio::test]
async fn cancelled_backpressure_flush_resumes_exact_offset_without_reencoding() {
    let payload: Vec<u8> = (0..1024).map(|n| (n % 251) as u8).collect();
    let mask = [1, 7, 19, 231];
    let mut independent = pmd::Encoder::new(15, false).unwrap();
    let expected = masked(0xc2, &independent.encode(&payload).unwrap(), mask);
    assert!(expected.len() > 16);
    let (client, mut peer) = tokio::io::duplex(16);
    let mut driver = Driver::new(client, Some(Default::default())).unwrap();
    driver.queue(2, &payload, true, mask).unwrap();
    assert_eq!(driver.pending_write_len(), expected.len());
    let mut future = Box::pin(driver.flush());
    poll_pending(&mut future).await;
    drop(future);
    assert_eq!(driver.write_offset(), 16);
    let mut actual = vec![0; expected.len()];
    let (read, flushed) = tokio::join!(peer.read_exact(&mut actual), driver.flush());
    read.unwrap();
    flushed.unwrap();
    assert_eq!(actual, expected);
    assert_eq!(driver.write_offset(), 0);
    assert_eq!(driver.pending_write_len(), 0);
    let second_expected = masked(0xc2, &independent.encode(&payload).unwrap(), mask);
    driver.queue(2, &payload, true, mask).unwrap();
    let mut second = vec![0; second_expected.len()];
    let (read, flushed) = tokio::join!(peer.read_exact(&mut second), driver.flush());
    read.unwrap();
    flushed.unwrap();
    assert_eq!(
        second, second_expected,
        "retained PMD context advanced exactly once per queue"
    );
}

#[tokio::test]
async fn dropping_owner_with_pending_write_sends_no_hidden_remainder() {
    let (client, mut peer) = tokio::io::duplex(16);
    let mut driver = Driver::new(client, None).unwrap();
    let payload = vec![42; 1024];
    driver.queue(2, &payload, false, [9, 8, 7, 6]).unwrap();
    let mut future = Box::pin(driver.flush());
    poll_pending(&mut future).await;
    drop(future);
    assert_eq!(driver.write_offset(), 16);
    drop(driver);
    let mut bytes = Vec::new();
    peer.read_to_end(&mut bytes).await.unwrap();
    assert_eq!(bytes.len(), 16);
    assert_eq!(bytes, masked(0x82, &payload, [9, 8, 7, 6])[..16]);
}

#[test]
fn core_preserves_prefix_rejects_decoded_utf8_and_absorbs_close_tail() {
    let mut core = Core::new(true, false).unwrap();
    let mut wire = server_frame(0x81, b"accepted");
    wire.extend_from_slice(&[0xc9, 0]);
    let mut messages = Vec::new();
    assert!(core.ingest(&wire, &mut messages).is_err());
    assert_eq!(
        messages,
        [Message {
            opcode: 1,
            compressed: false,
            payload: b"accepted".to_vec()
        }]
    );
    assert!(
        core.ingest(&server_frame(0x81, b"late"), &mut messages)
            .is_err()
    );
    assert_eq!(messages.len(), 1);
    let mut core = Core::new(true, false).unwrap();
    let mut encoder = pmd::Encoder::new(15, false).unwrap();
    let invalid = encoder.encode(&[0xff]).unwrap();
    assert!(
        core.ingest(&server_frame(0xc1, &invalid), &mut Vec::new())
            .is_err()
    );
    let mut core = Core::new(false, false).unwrap();
    let mut messages = Vec::new();
    core.ingest(&[0x88, 0, 0xff, 0xff], &mut messages).unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].opcode, 8);
    assert!(core.eof().is_ok());
}

#[tokio::test]
async fn queued_close_survives_late_queue_rejection_and_cancelled_flush() {
    let mut payload = 1000_u16.to_be_bytes().to_vec();
    payload.extend_from_slice(&[b'x'; 64]);
    let mask = [3, 1, 4, 1];
    let expected = masked(0x88, &payload, mask);
    let (client, mut peer) = tokio::io::duplex(16);
    let mut driver = Driver::new(client, Some(Default::default())).unwrap();
    driver.queue(8, &payload, false, mask).unwrap();
    assert!(matches!(
        driver.queue(1, b"late", false, [0; 4]),
        Err(Fault::Closed)
    ));
    assert_eq!(driver.pending_write_len(), expected.len());
    assert!(
        !driver.is_terminal(),
        "local Close still waits for peer response"
    );
    let mut future = Box::pin(driver.flush());
    poll_pending(&mut future).await;
    drop(future);
    assert_eq!(driver.write_offset(), 16);
    assert!(matches!(
        driver.queue(1, b"later", false, [0; 4]),
        Err(Fault::Closed)
    ));
    assert_eq!(driver.pending_write_len(), expected.len());
    assert_eq!(driver.write_offset(), 16);
    let mut actual = vec![0; expected.len()];
    let (read, flushed) = tokio::join!(peer.read_exact(&mut actual), driver.flush());
    read.unwrap();
    flushed.unwrap();
    assert_eq!(actual, expected);
    assert!(matches!(
        driver.queue(9, b"late ping", false, [0; 4]),
        Err(Fault::Closed)
    ));
    assert!(!driver.is_terminal());
    peer.write_all(&[0x88, 0]).await.unwrap();
    assert_eq!(driver.next().await.unwrap().unwrap().opcode, 8);
    assert!(driver.next().await.unwrap().is_none());
    assert!(driver.next().await.unwrap().is_none());
    peer.shutdown().await.unwrap();
    driver.finish_close(None).await.unwrap();
    assert_eq!(
        peer.read(&mut [0; 1]).await.unwrap(),
        0,
        "only one local Close was sent"
    );
}

#[tokio::test]
async fn busy_queue_preserves_first_frame_and_does_not_advance_pmd_context() {
    let payload: Vec<u8> = (0..1024).map(|n| (n % 251) as u8).collect();
    let mask = [6, 2, 6, 4];
    let mut independent = pmd::Encoder::new(15, false).unwrap();
    let first_expected = masked(0xc2, &independent.encode(&payload).unwrap(), mask);
    let (client, mut peer) = tokio::io::duplex(16);
    let mut driver = Driver::new(client, Some(Default::default())).unwrap();
    driver.queue(2, &payload, true, mask).unwrap();
    assert!(matches!(
        driver.queue(2, b"must never enter encoder", true, mask),
        Err(Fault::Busy)
    ));
    assert!(!driver.is_terminal());
    assert_eq!(driver.pending_write_len(), first_expected.len());
    assert_eq!(driver.write_offset(), 0);
    let mut first = vec![0; first_expected.len()];
    let (read, flushed) = tokio::join!(peer.read_exact(&mut first), driver.flush());
    read.unwrap();
    flushed.unwrap();
    assert_eq!(first, first_expected);
    let second_expected = masked(0xc2, &independent.encode(&payload).unwrap(), mask);
    driver.queue(2, &payload, true, mask).unwrap();
    let mut second = vec![0; second_expected.len()];
    let (read, flushed) = tokio::join!(peer.read_exact(&mut second), driver.flush());
    read.unwrap();
    flushed.unwrap();
    assert_eq!(
        second, second_expected,
        "Busy did not advance compressor context"
    );
}

#[tokio::test]
async fn cancelled_next_after_inbound_quantum_preserves_unconsumed_read_bytes() {
    let payload: Vec<u8> = (0..8192).map(|n| (n % 251) as u8).collect();
    let mut wire = vec![0x82, 126];
    wire.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    wire.extend_from_slice(&payload);
    let (client, mut peer) = tokio::io::duplex(16384);
    let mut driver = Driver::new(client, None).unwrap();
    peer.write_all(&wire).await.unwrap();
    let mut future = Box::pin(driver.next());
    poll_pending(&mut future).await;
    drop(future);
    assert!(!driver.is_terminal());
    assert_eq!(driver.next().await.unwrap().unwrap().payload, payload);
}

#[tokio::test]
async fn cancelled_next_during_expansion_preserves_job_and_dictionary() {
    let payload = vec![b'a'; 1024 * 1024];
    let mut encoder = pmd::Encoder::new(15, false).unwrap();
    let first = encoder.encode(&payload).unwrap();
    let second = encoder.encode(&payload).unwrap();
    let wire = |payload: &[u8]| {
        let mut wire = vec![0xc2, 126];
        wire.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        wire.extend_from_slice(payload);
        wire
    };
    let (client, mut peer) = tokio::io::duplex(16384);
    let mut driver = Driver::new(client, Some(Default::default())).unwrap();
    peer.write_all(&wire(&first)).await.unwrap();
    for _ in 0..2 {
        let mut future = Box::pin(driver.next());
        poll_pending(&mut future).await;
        drop(future);
    }
    assert_eq!(driver.next().await.unwrap().unwrap().payload, payload);
    peer.write_all(&wire(&second)).await.unwrap();
    assert_eq!(driver.next().await.unwrap().unwrap().payload, payload);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn initialized_driver_and_next_future_move_between_executor_threads() {
    fn assert_send<T: Send>(_: &T) {}
    let payload = vec![b'z'; 512 * 1024];
    let compressed = pmd::Encoder::new(15, false)
        .unwrap()
        .encode(&payload)
        .unwrap();
    let mut wire = vec![0xc2, 126];
    wire.extend_from_slice(&(compressed.len() as u16).to_be_bytes());
    wire.extend_from_slice(&compressed);
    let (client, mut peer) = tokio::io::duplex(4096);
    let mut driver = Driver::new(client, Some(Default::default())).unwrap();
    assert_send(&driver);
    let task = async move {
        let future = driver.next();
        assert_send(&future);
        future.await.unwrap().unwrap().payload
    };
    assert_send(&task);
    let task = tokio::spawn(task);
    peer.write_all(&wire).await.unwrap();
    assert_eq!(task.await.unwrap(), payload);
}

#[tokio::test]
async fn owner_drop_during_pmd_job_releases_transport_without_late_result() {
    let payload = vec![b'a'; 1024 * 1024];
    let compressed = pmd::Encoder::new(15, false)
        .unwrap()
        .encode(&payload)
        .unwrap();
    let mut wire = vec![0xc2, 126];
    wire.extend_from_slice(&(compressed.len() as u16).to_be_bytes());
    wire.extend_from_slice(&compressed);
    let (client, mut peer) = tokio::io::duplex(8192);
    let mut driver = Driver::new(client, Some(Default::default())).unwrap();
    peer.write_all(&wire).await.unwrap();
    let mut future = Box::pin(driver.next());
    poll_pending(&mut future).await;
    drop(future);
    drop(driver);
    assert_eq!(peer.read(&mut [0]).await.unwrap(), 0);
}

#[tokio::test]
async fn async_driver_preserves_accepted_prefix_before_coalesced_failure() {
    let (client, mut peer) = tokio::io::duplex(32);
    let mut driver = Driver::new(client, Some(Default::default())).unwrap();
    peer.write_all(&[0x81, 1, b'x', 0xc9, 0]).await.unwrap();
    assert_eq!(driver.next().await.unwrap().unwrap().payload, b"x");
    assert!(driver.next().await.is_err());
    assert!(driver.next().await.unwrap().is_none());
    assert!(driver.next().await.unwrap().is_none());
}

fn text_wire(payload: &[u8], compressed: bool) -> Vec<u8> {
    let mut wire = vec![if compressed { 0xc1 } else { 0x81 }];
    match payload.len() {
        0..=125 => wire.push(payload.len() as u8),
        126..=65535 => {
            wire.push(126);
            wire.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        }
        _ => {
            wire.push(127);
            wire.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }
    }
    wire.extend_from_slice(payload);
    wire
}
struct ReadyInput {
    wire: Vec<u8>,
    offset: usize,
}
impl tokio::io::AsyncRead for ReadyInput {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        out: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let n = out.remaining().min(self.wire.len() - self.offset);
        out.put_slice(&self.wire[self.offset..self.offset + n]);
        self.offset += n;
        Poll::Ready(Ok(()))
    }
}
impl tokio::io::AsyncWrite for ReadyInput {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(bytes.len()))
    }
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
async fn drive_text_in_owned_quanta(
    wire: Vec<u8>,
    expected: &[u8],
    pmd_enabled: bool,
    valid: bool,
) -> usize {
    let mut driver = Driver::new(
        ReadyInput { wire, offset: 0 },
        pmd_enabled.then_some(Default::default()),
    )
    .unwrap();
    let (mut validator_polls, mut previous) = (0, 0);
    loop {
        // Deliberately cancel every Pending borrowed future, including UTF8 validation.
        let mut future = Box::pin(driver.next());
        let result = poll_fn(|cx| Poll::Ready(future.as_mut().poll(cx))).await;
        drop(future);
        match result {
            Poll::Pending => {
                if let Some((offset, total)) = driver.text_validation_progress() {
                    assert_eq!(total, expected.len());
                    assert!(offset > previous);
                    assert!(offset - previous <= 8192, "validation exceeded one quantum");
                    previous = offset;
                    validator_polls += 1;
                }
                tokio::task::yield_now().await;
            }
            Poll::Ready(Ok(Some(message))) => {
                assert!(valid, "published invalid final UTF8");
                assert_eq!(message.opcode, 1);
                assert!(!message.compressed);
                assert_eq!(message.payload, expected);
                break;
            }
            Poll::Ready(Err(error)) => {
                assert!(!valid, "unexpected validation failure: {error}");
                assert!(matches!(error, Fault::Frame(opi_ws::frame::Fault::Utf8)));
                assert!(driver.next().await.unwrap().is_none());
                assert!(driver.next().await.unwrap().is_none());
                break;
            }
            Poll::Ready(Ok(None)) => panic!("text disappeared before validation"),
        }
    }
    validator_polls
}

#[tokio::test]
async fn sixteen_megabyte_plain_text_validation_is_bounded_and_cancel_resumable() {
    let payload = vec![b'x'; 16 * 1024 * 1024];
    let polls = drive_text_in_owned_quanta(text_wire(&payload, false), &payload, false, true).await;
    assert_eq!(
        polls,
        payload.len() / 8192 - 1,
        "FIN must not do a hidden whole-text scan"
    );
}

#[tokio::test]
async fn sixteen_megabyte_compressed_text_validation_is_bounded_and_cancel_resumable() {
    let payload = vec![b'z'; 16 * 1024 * 1024];
    let compressed = pmd::Encoder::new(15, false)
        .unwrap()
        .encode(&payload)
        .unwrap();
    let polls =
        drive_text_in_owned_quanta(text_wire(&compressed, true), &payload, true, true).await;
    assert_eq!(
        polls,
        payload.len() / 8192 - 1,
        "inflate completion must not hide a whole-text scan"
    );
}

#[tokio::test]
async fn utf8_codepoints_cross_validation_quanta_without_false_rejection() {
    for scalar in ["¢", "中", "🦀"] {
        for before in 1..scalar.len() {
            let mut payload = vec![b'a'; 8192 - before];
            payload.extend_from_slice(scalar.as_bytes());
            payload.extend_from_slice(&vec![b'b'; 16384]);
            for compressed in [false, true] {
                let encoded = if compressed {
                    pmd::Encoder::new(15, false)
                        .unwrap()
                        .encode(&payload)
                        .unwrap()
                } else {
                    payload.clone()
                };
                assert!(
                    drive_text_in_owned_quanta(
                        text_wire(&encoded, compressed),
                        &payload,
                        compressed,
                        true
                    )
                    .await
                        >= 2
                );
            }
        }
    }
}

#[tokio::test]
async fn invalid_or_incomplete_final_utf8_is_never_published_after_many_quanta() {
    for suffix in [&[0xff][..], &[0xf0, 0x9f][..]] {
        let mut payload = vec![b'a'; 256 * 8192];
        payload.extend_from_slice(suffix);
        for compressed in [false, true] {
            let encoded = if compressed {
                pmd::Encoder::new(15, false)
                    .unwrap()
                    .encode(&payload)
                    .unwrap()
            } else {
                payload.clone()
            };
            assert!(
                drive_text_in_owned_quanta(
                    text_wire(&encoded, compressed),
                    &payload,
                    compressed,
                    false
                )
                .await
                    >= 255
            );
        }
    }
}

#[test]
fn public_messages_constructor_still_rejects_invalid_plain_text_at_fin() {
    let mut messages = opi_ws::frame::Messages::new();
    assert!(matches!(
        messages.receive(opi_ws::frame::Frame {
            fin: true,
            opcode: 1,
            compressed: false,
            payload: vec![0xff]
        }),
        Err(opi_ws::frame::Fault::Utf8)
    ));
    assert!(
        messages
            .receive(opi_ws::frame::Frame {
                fin: true,
                opcode: 1,
                compressed: false,
                payload: b"late".to_vec()
            })
            .is_err()
    );
}

fn raw_frame(opcode: u8, fin: bool, compressed: bool, payload: &[u8]) -> Vec<u8> {
    let mut wire = text_wire(payload, compressed);
    wire[0] = opcode | if fin { 0x80 } else { 0 } | if compressed { 0x40 } else { 0 };
    wire
}
async fn assert_owned_continuation_copy(
    wire: Vec<u8>,
    expected: &[u8],
    opcode: u8,
    pmd_enabled: bool,
) {
    let mut driver = Driver::new(
        ReadyInput { wire, offset: 0 },
        pmd_enabled.then_some(Default::default()),
    )
    .unwrap();
    let before = driver.next().await.unwrap().unwrap();
    assert_eq!(before.opcode, 9);
    assert_eq!(before.payload, b"before");
    let (mut last, mut copy_yields) = (0, 0);
    loop {
        let mut future = Box::pin(driver.next());
        let result = poll_fn(|cx| Poll::Ready(future.as_mut().poll(cx))).await;
        drop(future);
        match result {
            Poll::Pending => {
                if let Some((offset, total)) = driver.append_progress() {
                    assert!(total > 8192);
                    assert!(offset > last);
                    assert!(offset - last <= 8192);
                    last = offset;
                    copy_yields += 1;
                }
                tokio::task::yield_now().await;
            }
            Poll::Ready(Ok(Some(message))) => {
                assert_eq!(message.opcode, opcode);
                assert_eq!(message.payload, expected);
                assert!(!message.compressed);
                break;
            }
            Poll::Ready(other) => panic!("continuation not delivered: {other:?}"),
        }
    }
    assert!(
        copy_yields > 0,
        "a completed continuation must not copy its whole payload in one poll"
    );
    let after = driver.next().await.unwrap().unwrap();
    assert_eq!(after.opcode, 9);
    assert_eq!(after.payload, b"after");
}

#[tokio::test]
async fn large_binary_continuation_copy_yields_preserves_prefix_and_control_order() {
    let body: Vec<u8> = (0..4 * 1024 * 1024).map(|n| (n % 251) as u8).collect();
    let mut expected = b"prefix".to_vec();
    expected.extend_from_slice(&body);
    let mut wire = raw_frame(2, false, false, b"prefix");
    wire.extend_from_slice(&raw_frame(9, true, false, b"before"));
    wire.extend_from_slice(&raw_frame(0, true, false, &body));
    wire.extend_from_slice(&raw_frame(9, true, false, b"after"));
    assert_owned_continuation_copy(wire, &expected, 2, false).await;
}

#[tokio::test]
async fn compressed_text_continuation_copy_resumes_then_inflates_validates_once() {
    let mut state = 0x1234_5678_u32;
    let mut payload: Vec<u8> = (0..2 * 1024 * 1024)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            32 + (state % 95) as u8
        })
        .collect();
    payload.extend_from_slice("🦀".as_bytes());
    let encoded = pmd::Encoder::new(15, false)
        .unwrap()
        .encode(&payload)
        .unwrap();
    assert!(encoded.len() > 8192);
    let mut wire = raw_frame(1, false, true, &encoded[..1]);
    wire.extend_from_slice(&raw_frame(9, true, false, b"before"));
    wire.extend_from_slice(&raw_frame(0, true, false, &encoded[1..]));
    wire.extend_from_slice(&raw_frame(9, true, false, b"after"));
    assert_owned_continuation_copy(wire, &payload, 1, true).await;
}
