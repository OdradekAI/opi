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

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

fn server_frame(first: u8, payload: &[u8]) -> Vec<u8> {
    let mut wire = vec![first, payload.len() as u8];
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

/// Full duplex echo: compressed text and plain binary, then peer Close.
async fn driver_echo_roundtrip() -> Result<(), String> {
    let (client, mut peer) = tokio::io::duplex(64);
    let mut driver = opi_ws::io::Driver::new(client, true, false).map_err(|e| e.to_string())?;
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
    Ok(())
}

/// Explicit Close: peer Close, masked response, write shutdown, peer EOF.
async fn close_handshake_masked_response() -> Result<(), String> {
    let (client, mut peer) = tokio::io::duplex(64);
    let mut driver = opi_ws::io::Driver::new(client, false, false).map_err(|e| e.to_string())?;
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
    let mut driver = opi_ws::io::Driver::new(client, true, false).map_err(|e| e.to_string())?;
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
    let mut driver = opi_ws::io::Driver::new(client, false, false).map_err(|e| e.to_string())?;
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
