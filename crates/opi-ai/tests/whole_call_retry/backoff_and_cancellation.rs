//! Capped backoff, caller cancellation and transport release while reading an error body.

use crate::fixtures::{Finals, logical_request, message, transient_policy};
use opi_ai::message::AssistantContent;
use opi_ai::provider::{Provider, ProviderError};
use opi_ai::retry::{
    DEFAULT_WHOLE_CALL_RETRY_MAX_DELAY_MS, WholeCallRetryCallbacks, WholeCallRetryPolicy,
};
use opi_ai::retry_assistant_call;
use opi_ai::stream::{ChatStream, StopReason};

#[test]
fn delay_grows_exponentially_and_caps_at_the_default_60s() {
    let policy = transient_policy(8, 2_000);
    assert_eq!(opi_ai::retry::whole_call_retry_delay_ms(&policy, 1), 2_000);
    assert_eq!(opi_ai::retry::whole_call_retry_delay_ms(&policy, 2), 4_000);
    assert_eq!(opi_ai::retry::whole_call_retry_delay_ms(&policy, 3), 8_000);
    assert_eq!(opi_ai::retry::whole_call_retry_delay_ms(&policy, 4), 16_000);
    // Explicit caller caps replace the default.
    let capped = WholeCallRetryPolicy {
        max_delay_ms: Some(5_000),
        ..transient_policy(8, 30_000)
    };
    assert_eq!(opi_ai::retry::whole_call_retry_delay_ms(&capped, 2), 5_000);
    // Huge exponents saturate at the cap, never overflowing.
    let huge = transient_policy(8, u64::MAX / 2);
    assert_eq!(
        opi_ai::retry::whole_call_retry_delay_ms(&huge, 5),
        DEFAULT_WHOLE_CALL_RETRY_MAX_DELAY_MS
    );
}

#[tokio::test]
async fn cancelled_backoff_normalizes_to_aborted_preserving_partial_and_usage() {
    let policy = transient_policy(3, 120);
    let cancel = tokio_util::sync::CancellationToken::new();
    let cancel_for_loop = cancel.clone();
    let finished: Finals = Default::default();
    let finished_sink = finished.clone();

    let mut callbacks = WholeCallRetryCallbacks::new();
    callbacks.on_retry_scheduled = Some(Box::new(move |_, _, _, _| {
        cancel_for_loop.cancel();
        Ok(())
    }));
    callbacks.on_retry_finished = Some(Box::new(move |success, attempt, error| {
        finished_sink
            .lock()
            .unwrap()
            .push((success, attempt, error.map(str::to_string)));
        Ok(())
    }));

    let calls = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let calls_for_produce = calls.clone();
    let result = retry_assistant_call(
        move || {
            calls_for_produce.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            std::future::ready(message(
                StopReason::Error,
                Some("connection refused while streaming"),
            ))
        },
        Some(&policy),
        cancel,
        Some(callbacks),
    )
    .await
    .unwrap();

    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(result.stop_reason, StopReason::Aborted);
    assert_eq!(result.error_message, None, "aborted drops the error text");
    assert_eq!(result.usage.input_tokens, 3, "usage is preserved");
    assert_eq!(
        result.content,
        vec![AssistantContent::Text {
            text: "partial".into(),
            signature: None,
        }],
        "partial output is preserved"
    );
    assert_eq!(
        *finished.lock().unwrap(),
        vec![(false, 1, Some("connection refused while streaming".into()))]
    );
}

#[tokio::test]
async fn cancelling_pending_429_body_aborts_without_retry_or_raw_text() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    for responses in [false, true] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let (headers_sent, received) = tokio::sync::oneshot::channel();
        let peer = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0; 8192];
            assert!(socket.read(&mut buffer).await.unwrap() > 0);
            socket.write_all(b"HTTP/1.1 429 Too Many Requests\r\nContent-Length: 4096\r\nConnection: close\r\n\r\n{").await.unwrap();
            headers_sent.send(()).unwrap();
            loop {
                if socket.read(&mut buffer).await.unwrap_or(0) == 0 {
                    break;
                }
            }
        });
        let provider: Box<dyn Provider> = if responses {
            Box::new(
                opi_ai::providers::openai_responses::OpenAiResponsesProvider::new(Some(endpoint)),
            )
        } else {
            Box::new(opi_ai::providers::openai::OpenAiChatProvider::new(Some(
                endpoint,
            )))
        };
        let request = logical_request();
        let cancel = request.cancel.clone();
        let result = tokio::spawn(
            ChatStream::new(
                provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()),
            )
            .result(),
        );
        received.await.unwrap();
        cancel.cancel();
        let failure = tokio::time::timeout(std::time::Duration::from_secs(2), result)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err();
        assert!(matches!(failure.cause(), ProviderError::Cancelled));
        assert_eq!(failure.message().stop_reason, StopReason::Aborted);
        tokio::time::timeout(std::time::Duration::from_secs(2), peer)
            .await
            .unwrap()
            .unwrap();
    }
}
