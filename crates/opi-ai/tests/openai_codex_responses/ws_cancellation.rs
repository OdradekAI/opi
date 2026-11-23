fn request() -> Request {
    let mut request = super::request();
    request.transport = opi_ai::provider::TransportPreference::Auto;
    request
}

use super::ws_path::{CodexPeer, read_frame, response_101, write_frame};
use super::*;
use opi_ai::http::{HttpClientBuilder, RedirectPolicy};
use opi_ai::provider::TransportPreference;
use std::io::Write;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

fn ws_provider(base_url: String) -> OpenAiCodexResponsesProvider {
    OpenAiCodexResponsesProvider::new(
        Some(base_url),
        vec![model(None)],
        Arc::new(
            HttpClientBuilder::new()
                .redirects(RedirectPolicy::None)
                .build()
                .unwrap(),
        ),
    )
}

const DELTA: &str = r#"{"type":"response.output_text.delta","delta":"Hello"}"#;

#[tokio::test]
async fn active_cancellation_mid_stream_ends_with_cancelled() {
    let peer = CodexPeer::spawn(
        |mut stream: TcpStream,
         lines: Vec<String>,
         _capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
            stream.write_all(&response_101(&lines, &[])).unwrap();
            let (opcode, _) = read_frame(&mut stream).unwrap();
            assert_eq!(opcode, 1);
            write_frame(&mut stream, 1, DELTA.as_bytes());
            // Hold the connection open; the client cancels mid-stream.
            std::thread::sleep(std::time::Duration::from_millis(800));
            Ok(())
        },
    );
    let provider = ws_provider(peer.base_url());
    let mut call = request();
    call.transport = TransportPreference::WebsocketCached;
    let cancel = call.cancel.clone();
    let mut stream = provider.stream_prepared(call, auth("s", None, Some("a".into())));
    let mut error = None;
    while let Some(result) = stream.next().await {
        match result {
            Ok(_) => cancel.cancel(),
            Err(found) => {
                error = Some(found);
                break;
            }
        }
    }
    let cancelled = matches!(&error, Some(ProviderError::Cancelled))
        || matches!(&error, Some(ProviderError::StreamFailure(failure))
            if matches!(failure.cause(), ProviderError::Cancelled));
    assert!(
        cancelled,
        "active cancellation ends the stream as Cancelled, got {error:?}"
    );
    peer.finish().expect("peer script");
}

#[tokio::test]
async fn idle_cancellation_before_events_ends_with_cancelled() {
    let peer = CodexPeer::spawn(
        |mut stream: TcpStream,
         lines: Vec<String>,
         _capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
            stream.write_all(&response_101(&lines, &[])).unwrap();
            let (opcode, _) = read_frame(&mut stream).unwrap();
            assert_eq!(opcode, 1);
            // No events: the client cancels while idle.
            std::thread::sleep(std::time::Duration::from_millis(800));
            Ok(())
        },
    );
    let provider = ws_provider(peer.base_url());
    let mut call = request();
    call.transport = TransportPreference::WebsocketCached;
    let cancel = call.cancel.clone();
    let mut stream = provider.stream_prepared(call, auth("s", None, Some("a".into())));
    // First poll starts this raw provider call. Dropping only this borrowed
    // wait leaves the call and its idle connection alive for cancellation.
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), stream.next())
            .await
            .is_err()
    );
    cancel.cancel();
    let mut error = None;
    while let Some(result) = stream.next().await {
        if let Err(found) = result {
            error = Some(found);
            break;
        }
    }
    let cancelled = matches!(&error, Some(ProviderError::Cancelled))
        || matches!(&error, Some(ProviderError::StreamFailure(failure))
            if matches!(failure.cause(), ProviderError::Cancelled));
    assert!(
        cancelled,
        "idle cancellation ends the stream as Cancelled, got {error:?}"
    );
    peer.finish().expect("peer script");
}
