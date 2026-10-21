//! Workspace-external consumer of the packaged `opi-ws` companion (RDF-24).
//!
//! Runs a fixed named-check battery against the real cargo package artifact
//! and prints a JSON report on stdout. The checks are backend-agnostic: the
//! decoder is the direct libz-rs-sys Rust path in every feature graph, and
//! every encoder round trip uses the consumer's own encoder instance, so the
//! ordinary / union-default / union-c-zlib profiles prove the same public
//! behavior with different encoder feature unions.

use std::future::poll_fn;
use std::task::Poll;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

mod deflate_audit;

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

fn server_frame(first: u8, payload: &[u8]) -> Vec<u8> {
    let mut wire = vec![first];
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

fn masked(opcode: u8, payload: &[u8], mask: [u8; 4]) -> Vec<u8> {
    let mut wire = vec![0x80 | opcode, 0x80 | payload.len() as u8];
    wire.extend_from_slice(&mask);
    wire.extend(
        payload
            .iter()
            .enumerate()
            .map(|(i, b)| *b ^ mask[i % 4]),
    );
    wire
}

/// Decode the RFC 7692-style hello vector (tail omitted by the sender).
async fn rfc_hello_decodes() -> Result<(), String> {
    let mut decoder = opi_ws::pmd::Decoder::new(false).map_err(|e| e.to_string())?;
    let output = decoder
        .decode(&hex("f248cdc9c90700"))
        .map_err(|e| format!("decode failed: {e:?}"))?;
    if output == b"Hello" {
        Ok(())
    } else {
        Err(format!("unexpected output {:?}", output))
    }
}

/// Two compressed messages through one encoder/decoder pair: the second
/// payload must actually depend on retained history.
async fn context_takeover_across_messages() -> Result<(), String> {
    let mut encoder = opi_ws::pmd::Encoder::new(15, false).map_err(|e| e.to_string())?;
    let first = encoder.encode(b"history-payload ").map_err(|e| e.to_string())?;
    let second = encoder.encode(b"history-payload ").map_err(|e| e.to_string())?;
    let mut decoder = opi_ws::pmd::Decoder::new(false).map_err(|e| e.to_string())?;
    if decoder.decode(&first).map_err(|e| e.to_string())? != b"history-payload " {
        return Err("first message corrupted".into());
    }
    let mut independent = opi_ws::pmd::Decoder::new(false).map_err(|e| e.to_string())?;
    if independent.decode(&second).is_ok() {
        return Err("second message did not depend on retained history".into());
    }
    match decoder.decode(&second) {
        Ok(output) if output == b"history-payload " => Ok(()),
        other => Err(format!("takeover decode failed: {other:?}")),
    }
}

/// Compressed text/plain binary plus independently audited window-8 wire
/// in both directions, with and without context takeover.
async fn driver_echo_roundtrip() -> Result<(), String> {
    let (client, mut peer) = tokio::io::duplex(64);
    let mut driver = opi_ws::io::Driver::new(client, Some(Default::default())).map_err(|e| e.to_string())?;
    let mut encoder = opi_ws::pmd::Encoder::new(15, false).map_err(|e| e.to_string())?;
    let compressed = encoder.encode("回环 canary".as_bytes()).map_err(|e| e.to_string())?;
    peer.write_all(&server_frame(0xc1, &compressed)).await.map_err(|e| e.to_string())?;
    let message = driver
        .next()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("no message")?;
    if message.opcode != 1 || message.payload != "回环 canary".as_bytes() {
        return Err("compressed text corrupted".into());
    }
    peer.write_all(&server_frame(0x82, b"binary")).await.map_err(|e| e.to_string())?;
    let message = driver
        .next()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("no message")?;
    if message.opcode != 2 || message.payload != b"binary" {
        return Err("binary corrupted".into());
    }
    window8_roundtrip().await
}

async fn window8_roundtrip() -> Result<(), String> {
    let mut seed = 7_u32;
    let pattern: Vec<_> = (0..200)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed as u8
        })
        .collect();
    let expected = pattern.repeat(257);
    for no_context in [false, true] {
        let (client, mut peer) = tokio::io::duplex(8192);
        let parameters = opi_ws::io::PmdParameters {
            server_no_context_takeover: no_context,
            client_no_context_takeover: no_context,
            server_max_window_bits: 8,
            client_max_window_bits: 8,
        };
        let mut driver =
            opi_ws::io::Driver::new(client, Some(parameters)).map_err(|e| e.to_string())?;
        let mut encoder = opi_ws::pmd::Encoder::new(8, no_context).map_err(|e| e.to_string())?;
        let (mut inbound_history, mut outbound_history) = (Vec::new(), Vec::new());
        for message in 0..2 {
            let compressed = encoder.encode(&expected).map_err(|e| e.to_string())?;
            if no_context {
                inbound_history.clear();
                outbound_history.clear();
            }
            let (decoded, maximum) =
                deflate_audit::decode_pmd(&compressed, &mut inbound_history, 8)?;
            if decoded != expected || !(1..=256).contains(&maximum) {
                return Err("inbound window8 wire content/distance mismatch".into());
            }
            let wire = server_frame(0xc2, &compressed);
            // The real encoder may produce more bytes than the duplex can
            // hold. Exercise backpressure by driving both ends together.
            let (written, received) = tokio::join!(peer.write_all(&wire), driver.next());
            written.map_err(|e| e.to_string())?;
            let received = received
                .map_err(|e| e.to_string())?
                .ok_or("window8 message missing")?;
            if received.payload != expected {
                return Err("window8 receive corrupted".into());
            }
            driver
                .queue(2, &expected, true, [3, 5, 7, 9])
                .map_err(|e| e.to_string())?;
            let receive = async {
                let mut head = [0; 2];
                peer.read_exact(&mut head)
                    .await
                    .map_err(|e| e.to_string())?;
                if head[0] != 0xc2 || head[1] & 128 == 0 {
                    return Err("window8 output was not compressed and masked".to_string());
                }
                let length = match head[1] & 127 {
                    126 => usize::from(peer.read_u16().await.map_err(|e| e.to_string())?),
                    127 => usize::try_from(peer.read_u64().await.map_err(|e| e.to_string())?)
                        .map_err(|e| e.to_string())?,
                    n => usize::from(n),
                };
                let mut mask = [0; 4];
                peer.read_exact(&mut mask)
                    .await
                    .map_err(|e| e.to_string())?;
                let mut payload = vec![0; length];
                peer.read_exact(&mut payload)
                    .await
                    .map_err(|e| e.to_string())?;
                for (i, byte) in payload.iter_mut().enumerate() {
                    *byte ^= mask[i % 4];
                }
                Ok::<_, String>(payload)
            };
            let flush = async { driver.flush().await.map_err(|e| e.to_string()) };
            let ((), payload) = tokio::try_join!(flush, receive)?;
            let (decoded, maximum) = deflate_audit::decode_pmd(&payload, &mut outbound_history, 8)?;
            if decoded != expected || !(1..=256).contains(&maximum) {
                return Err("outbound window8 wire content/distance mismatch".into());
            }
            if !no_context
                && message == 1
                && deflate_audit::decode_pmd(&payload, &mut Vec::new(), 8).is_ok()
            {
                return Err("window8 takeover fixture did not depend on retained history".into());
            }
        }
    }
    Ok(())
}

/// Explicit Close: peer Close, masked response, write shutdown, peer EOF.
async fn close_handshake_masked_response() -> Result<(), String> {
    let (client, mut peer) = tokio::io::duplex(64);
    let mut driver = opi_ws::io::Driver::new(client, None).map_err(|e| e.to_string())?;
    peer.write_all(&[0x88, 0]).await.map_err(|e| e.to_string())?;
    let close = driver
        .next()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("no close")?;
    if close.opcode != 8 {
        return Err("expected close frame".into());
    }
    let mask = [9, 2, 7, 4];
    driver
        .queue(8, &close.payload, false, mask)
        .map_err(|e| e.to_string())?;
    // The server reads the exact masked response, then shuts down its write
    // half so the client's finish_close observes a lawful transport EOF.
    let server = async {
        let mut wire = [0; 6];
        peer.read_exact(&mut wire).await.map_err(|e| e.to_string())?;
        if wire.as_slice() != masked(8, &[], mask).as_slice() {
            return Err("close response not exactly the masked frame".to_string());
        }
        peer.shutdown().await.map_err(|e| e.to_string())
    };
    let (server, finished) = tokio::join!(server, driver.finish_close(None));
    server.map_err(|e| e.to_string())?;
    finished.map_err(|e| format!("finish_close failed: {e:?}"))
}

/// A malformed server frame fails once, then the owner is terminal.
async fn strict_rejection_is_terminal() -> Result<(), String> {
    let (client, mut peer) = tokio::io::duplex(64);
    let mut driver = opi_ws::io::Driver::new(client, Some(Default::default())).map_err(|e| e.to_string())?;
    peer.write_all(&[0x41, 0, 0xc0, 0]).await.map_err(|e| e.to_string())?;
    if driver.next().await.is_ok() {
        return Err("RSV1 on continuation accepted".into());
    }
    match driver.next().await {
        Ok(None) | Err(_) => Ok(()),
        Ok(Some(_)) => Err("owner delivered after failure".into()),
    }
}

/// The borrowed receive future is cancellable and resumes from owned state.
async fn cancelled_borrow_resumes() -> Result<(), String> {
    let (client, mut peer) = tokio::io::duplex(64);
    let mut driver = opi_ws::io::Driver::new(client, None).map_err(|e| e.to_string())?;
    let payload = vec![0x5a; 300];
    let mut wire = vec![0x82, 126];
    wire.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    wire.extend_from_slice(&payload);
    peer.write_all(&wire[..64]).await.map_err(|e| e.to_string())?;
    {
        let mut future = Box::pin(driver.next());
        let pending = poll_fn(|cx| Poll::Ready(future.as_mut().poll(cx).is_pending())).await;
        if !pending {
            return Err("partial header should be pending".into());
        }
    }
    // The remainder exceeds the duplex buffer, so the write and the resumed
    // receive must make progress together.
    let (write, message) = tokio::join!(peer.write_all(&wire[64..]), driver.next());
    write.map_err(|e| e.to_string())?;
    let message = message.map_err(|e| e.to_string())?.ok_or("message disappeared after cancellation")?;
    if message.payload != payload {
        return Err("resumed message corrupted".into());
    }
    peer.shutdown().await.map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_time()
        .build()
        .expect("consumer runtime");
    let checks = runtime.block_on(async {
        [
            ("rfc-hello-decodes", rfc_hello_decodes().await),
            ("context-takeover-across-messages", context_takeover_across_messages().await),
            ("driver-echo-roundtrip", driver_echo_roundtrip().await),
            ("close-handshake-masked-response", close_handshake_masked_response().await),
            ("strict-rejection-is-terminal", strict_rejection_is_terminal().await),
            ("cancelled-borrow-resumes", cancelled_borrow_resumes().await),
        ]
        .into_iter()
        .map(|(name, result)| {
            serde_json::json!({
                "name": name,
                "passed": result.is_ok(),
                "detail": result.err().unwrap_or_default(),
            })
        })
        .collect::<Vec<_>>()
    });
    let failed = checks
        .iter()
        .filter(|c| !c["passed"].as_bool().unwrap())
        .count();
    let report = serde_json::json!({
        "verdict": if failed == 0 { "PASS" } else { "FAIL" },
        "failed": failed,
        "total": checks.len(),
        "checks": checks,
    });
    println!("{report}");
}
