//! HTTP/1 response-head, chunk, and trailer acceptance through public Chat calls.

use std::sync::Arc;

use opi_ai::http::RedirectPolicy;
use opi_ai::provider::ProviderError;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use super::fixtures::{
    Delivery, RecordedRequest, SSE_BODY, Script, accepted_failure_cause, chat_call, chat_error,
    chat_request, chunk, is_post_acceptance_stream_failure, plain_client, read_request, sse_head,
    start_raw,
};

/// Chunked SSE response with `pad` extra header lines in the head.
fn sse_response_with_head_pad(pad: &[String]) -> Vec<u8> {
    let mut out = sse_head(pad);
    out.extend_from_slice(
        format!("{}{}\r\n0\r\n\r\n", chunk(SSE_BODY.as_bytes()), SSE_BODY).as_bytes(),
    );
    out
}

/// Chunked SSE response carrying `trailer_lines` after the final chunk.
fn sse_response_with_trailers(trailer_lines: &[String]) -> Vec<u8> {
    let mut out = sse_head(&[]);
    // Without a protocol sentinel, success requires a valid HTTP EOF, so
    // this fixture exercises trailer validation even after finish and usage.
    let body = SSE_BODY
        .strip_suffix("data: [DONE]\n\n")
        .expect("fixture ends with the protocol sentinel");
    out.extend_from_slice(format!("{}{}\r\n0\r\n", chunk(body.as_bytes()), body).as_bytes());
    for line in trailer_lines {
        out.extend_from_slice(line.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"\r\n");
    out
}

/// Chunked response whose single chunk carries `extension` after the size
/// and whose payload is the SSE body.
fn sse_response_with_chunk_extension(extension: &str) -> Vec<u8> {
    let mut out = sse_head(&[]);
    out.extend_from_slice(
        format!(
            "{:x};{}\r\n{}\r\n0\r\n\r\n",
            SSE_BODY.len(),
            extension,
            SSE_BODY
        )
        .as_bytes(),
    );
    out
}

/// Chunked response with `chunks` data chunks, each carrying `ext` bytes of
/// chunk extension; the payload streams the SSE body split across chunks.
fn sse_multi_chunk_with_extensions(chunks: usize, ext: &str) -> Vec<u8> {
    let mut out = sse_head(&[]);
    let body = SSE_BODY.as_bytes();
    let per = body.len().div_ceil(chunks).max(1);
    for piece in body.chunks(per) {
        out.extend_from_slice(format!("{:x};{}\r\n", piece.len(), ext).as_bytes());
        out.extend_from_slice(piece);
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"0\r\n\r\n");
    out
}

/// Build `count` pad header lines sized so the response head totals
/// `total` bytes including the terminating blank line.
fn head_pad_for_total(total: usize, count: usize) -> Vec<String> {
    let base =
        b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n"
            .len();
    let mut lines: Vec<String> = (0..count).map(|i| format!("x-pad-{i}: p")).collect();
    let used: usize = base + lines.iter().map(|l| l.len() + 2).sum::<usize>() + 2;
    let pad = total.saturating_sub(used);
    if let Some(last) = lines.last_mut() {
        last.push_str(&"p".repeat(pad));
    }
    lines
}

async fn assert_head_size_accepted(total: usize, expect_success: bool, delivery: Delivery) {
    let pad = head_pad_for_total(total, 2);
    let response = sse_response_with_head_pad(&pad);
    let wrote = response.len();
    let server = start_raw(Arc::new(move |_: usize, _: &RecordedRequest| {
        Script::delivered(response.clone(), delivery)
    }))
    .await;
    let client = plain_client(RedirectPolicy::default());
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        chat_call(&server.url, client),
    )
    .await;
    if expect_success {
        let message = outcome
            .expect("head-size case terminates")
            .expect("round trip");
        assert_eq!(message.response_id.as_deref(), Some("chatcmpl-acc"));
    } else {
        let error = outcome
            .expect("oversized head terminates")
            .expect_err("head rejected");
        assert!(
            matches!(error, ProviderError::Network(_)),
            "head parse failure must surface at acquisition: {error:?}"
        );
    }
    assert_eq!(
        server.stats.count(),
        1,
        "one send for a {wrote}-byte scripted response"
    );
    server.stop().await;
}

#[tokio::test]
async fn response_head_below_at_and_above_initial_read_buffer() {
    // hyper 1.9.0 reads responses through an 8 KiB initial buffer that
    // grows adaptively (default maximum 8 KiB + 100 x 4 KiB): heads below,
    // exactly at, and above the initial size, and well into the adaptive
    // range (300 KiB), are accepted deterministically. No response-head
    // byte cap below the adaptive maximum is enforced through the public
    // surface; the enforced head boundary is the 100-field header count
    // in the next test.
    assert_head_size_accepted(1_024, true, Delivery::Coalesced).await;
    assert_head_size_accepted(8_192, true, Delivery::Coalesced).await;
    assert_head_size_accepted(16_384, true, Delivery::Coalesced).await;
    assert_head_size_accepted(8_192, true, Delivery::Bytewise).await;
    assert_head_size_accepted(300 * 1024, true, Delivery::Coalesced).await;
}

fn head_pad_headers(count: usize) -> Vec<String> {
    (0..count).map(|i| format!("x-h{i}: v")).collect()
}

#[tokio::test]
async fn response_head_header_count_boundary() {
    // hyper 1.9.0 parses at most 100 header fields per message head,
    // counting the two standard fields this response carries, so 98 pad
    // fields (100 total) still parse and 99 (101 total) are rejected at
    // acquisition.
    for delivery in [Delivery::Coalesced, Delivery::Bytewise] {
        for (count, expect_success) in [
            (97usize, true),
            (98, true),
            (99, false),
            (123, false),
            (198, false),
        ] {
            let pad = head_pad_headers(count);
            let response = sse_response_with_head_pad(&pad);
            let server = start_raw(Arc::new(move |_: usize, _: &RecordedRequest| {
                Script::delivered(response.clone(), delivery)
            }))
            .await;
            let client = plain_client(RedirectPolicy::default());
            let outcome = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                chat_call(&server.url, client),
            )
            .await
            .expect("header-count case terminates");
            if expect_success {
                assert_eq!(
                    outcome
                        .expect("100-or-fewer headers accepted")
                        .response_id
                        .as_deref(),
                    Some("chatcmpl-acc")
                );
            } else {
                let error = outcome.expect_err("101 headers rejected");
                assert!(
                    matches!(error, ProviderError::Network(_)),
                    "header overflow must surface at acquisition: {error:?}"
                );
            }
            assert_eq!(server.stats.count(), 1);
            server.stop().await;
        }
    }
}

#[tokio::test]
async fn response_head_near_adaptive_buffer_boundary_has_typed_outcome() {
    // TCP coalescing and the backend's spare buffer capacity affect this
    // boundary: record actual acceptance without promising a universal
    // equality threshold. Every outcome must terminate at the public seam.
    for delivery in [Delivery::Coalesced, Delivery::Bytewise] {
        for total in [417_791, 417_792, 417_793] {
            let response = sse_response_with_head_pad(&head_pad_for_total(total, 2));
            let server = start_raw(Arc::new(move |_, _: &RecordedRequest| {
                Script::delivered(response.clone(), delivery)
            }))
            .await;
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                chat_call(&server.url, plain_client(RedirectPolicy::default())),
            )
            .await
            .unwrap();
            eprintln!(
                "head_bytes={total} delivery={delivery:?} accepted={}",
                result.is_ok()
            );
            match result {
                Ok(message) => assert_eq!(message.response_id.as_deref(), Some("chatcmpl-acc")),
                Err(error) => assert!(matches!(error, ProviderError::Network(_)), "{error:?}"),
            }
            assert_eq!(server.stats.count(), 1);
            server.stop().await;
        }
    }
}

#[tokio::test]
async fn malformed_content_lengths_and_chunk_extension_never_succeed() {
    for delivery in [Delivery::Coalesced, Delivery::Bytewise] {
        for (head, acquisition) in [
            ("content-length: 1\r\ncontent-length: 2\r\n", true),
            ("content-length: -1\r\n", true),
            ("content-length: 1, 2\r\n", true),
            // TE takes precedence over CL; the non-chunked bytes must then
            // fail in the accepted body's decoder, not at acquisition.
            (
                "content-length: invalid\r\ntransfer-encoding: chunked\r\n",
                false,
            ),
        ] {
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n{head}\r\n{}",
                SSE_BODY
            )
            .into_bytes();
            let server = start_raw(Arc::new(move |_, _: &RecordedRequest| {
                Script::delivered(response.clone(), delivery)
            }))
            .await;
            let error = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                chat_error(&server.url, plain_client(RedirectPolicy::default())),
            )
            .await
            .unwrap();
            if acquisition {
                assert!(
                    matches!(
                        accepted_failure_cause(&error, &chat_request()),
                        ProviderError::Network(_)
                    ),
                    "{error:?}"
                );
            } else {
                assert!(is_post_acceptance_stream_failure(&error), "{error:?}");
            }
            assert_eq!(server.stats.count(), 1);
            server.stop().await;
        }
        let response = sse_response_with_chunk_extension("invalid\nextension");
        let server = start_raw(Arc::new(move |_, _: &RecordedRequest| {
            Script::delivered(response.clone(), delivery)
        }))
        .await;
        let error = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            chat_error(&server.url, plain_client(RedirectPolicy::default())),
        )
        .await
        .unwrap();
        assert!(is_post_acceptance_stream_failure(&error), "{error:?}");
        assert_eq!(server.stats.count(), 1);
        server.stop().await;
    }
}

fn trailer_lines(count: usize) -> Vec<String> {
    (0..count).map(|i| format!("x-t{i}: trailer")).collect()
}

async fn assert_trailers_accepted(lines: Vec<String>, expect_success: bool, delivery: Delivery) {
    let count = lines.len();
    let bytes = lines.iter().map(|line| line.len() + 2).sum::<usize>() + 2;
    let response = sse_response_with_trailers(&lines);
    let server = start_raw(Arc::new(move |_: usize, _: &RecordedRequest| {
        Script::delivered(response.clone(), delivery)
    }))
    .await;
    let client = plain_client(RedirectPolicy::default());
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        chat_call(&server.url, client),
    )
    .await;
    eprintln!(
        "trailer_count={count} trailer_bytes={bytes} delivery={delivery:?} accepted={}",
        matches!(&outcome, Ok(Ok(_)))
    );
    if expect_success {
        assert_eq!(
            outcome
                .expect("trailer case terminates")
                .expect("trailers accepted")
                .response_id
                .as_deref(),
            Some("chatcmpl-acc")
        );
    } else {
        let error = outcome
            .expect("trailer rejection terminates")
            .expect_err("trailers rejected");
        assert!(
            is_post_acceptance_stream_failure(&error),
            "trailer overflow must surface as a body stream error: {error:?}"
        );
    }
    assert_eq!(server.stats.count(), 1);
    server.stop().await;
}

#[tokio::test]
async fn trailer_count_and_bytes_boundaries() {
    // These fixtures delay the public terminal until HTTP EOF. The pinned
    // decoder rejects a count of 100 and a buffered trailer size of 16 KiB;
    // no SDK quota is involved. Include both sides and the exact values.
    for delivery in [Delivery::Coalesced, Delivery::Bytewise] {
        for count in [20, 99, 100, 101, 125, 200, 400] {
            assert_trailers_accepted(trailer_lines(count), count < 100, delivery).await;
        }
        for total in [16_383, 16_384, 16_385, 65_536] {
            let prefix = "x-pad: ";
            let line = format!("{prefix}{}", "p".repeat(total - prefix.len() - 4));
            assert_trailers_accepted(vec![line], total < 16_384, delivery).await;
        }
    }
}

#[tokio::test]
async fn malformed_and_truncated_trailers_do_not_publish_success() {
    for delivery in [Delivery::Coalesced, Delivery::Bytewise] {
        for tail in [
            b"invalid trailer\r\n\r\n".as_slice(),
            b"x-tail: unfinished".as_slice(),
        ] {
            let mut response = sse_response_with_trailers(&[]);
            response.truncate(response.len() - 2);
            response.extend_from_slice(tail);
            let server = start_raw(Arc::new(move |_, _: &RecordedRequest| {
                Script::delivered(response.clone(), delivery)
            }))
            .await;
            let error = chat_error(&server.url, plain_client(RedirectPolicy::default())).await;
            assert!(is_post_acceptance_stream_failure(&error), "{error:?}");
            assert_eq!(server.stats.count(), 1);
            server.stop().await;
        }
    }
}

#[tokio::test]
async fn trailer_fixture_keeps_the_call_pending_until_the_http_tail_arrives() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (sent, ready) = oneshot::channel();
    let (release, tail_ready) = oneshot::channel();
    let peer = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        read_request(&mut socket).await.unwrap().unwrap();
        let mut response = sse_response_with_trailers(&[]);
        response.truncate(response.len() - 2);
        socket.write_all(&response).await.unwrap();
        socket.flush().await.unwrap();
        sent.send(()).unwrap();
        tail_ready.await.unwrap();
        socket.write_all(b"x-tail: observed\r\n\r\n").await.unwrap();
        socket.shutdown().await.unwrap();
    });
    let mut call = Box::pin(chat_call(&url, plain_client(RedirectPolicy::default())));
    tokio::select! {
        result = &mut call => panic!("the unfinished HTTP response completed: {result:?}"),
        _ = ready => {}
    }
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), &mut call)
            .await
            .is_err(),
        "the trailer fixture must not emit its public Done before HTTP EOF"
    );
    release.send(()).unwrap();
    assert_eq!(
        call.await.unwrap().response_id.as_deref(),
        Some("chatcmpl-acc")
    );
    peer.await.unwrap();
}

#[tokio::test]
async fn chunk_extension_exact_boundary_is_stage_provable() {
    for delivery in [Delivery::Coalesced, Delivery::Bytewise] {
        for size in [16_383, 16_384, 16_385] {
            let response = sse_response_with_chunk_extension(&"p".repeat(size));
            let server = start_raw(Arc::new(move |_, _: &RecordedRequest| {
                Script::delivered(response.clone(), delivery)
            }))
            .await;
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                chat_call(&server.url, plain_client(RedirectPolicy::default())),
            )
            .await
            .unwrap();
            if size < 16_384 {
                assert_eq!(result.unwrap().response_id.as_deref(), Some("chatcmpl-acc"));
            } else {
                let error = result.unwrap_err();
                assert!(is_post_acceptance_stream_failure(&error), "{error:?}");
            }
            assert_eq!(server.stats.count(), 1);
            server.stop().await;
        }
    }
}

#[tokio::test]
async fn cumulative_chunk_extension_boundary() {
    // hyper 1.9.0 allows at most 16 KiB of chunk-extension bytes cumulatively
    // across a chunked body; beyond that the body fails as a stream error.
    // Below the limit, delivered both coalesced and bytewise.
    for delivery in [Delivery::Coalesced, Delivery::Bytewise] {
        let ext = "e".repeat(100);
        let response = sse_multi_chunk_with_extensions(8, &ext); // 8 * 100 = 800 bytes
        let server = start_raw(Arc::new(move |_: usize, _: &RecordedRequest| {
            Script::delivered(response.clone(), delivery)
        }))
        .await;
        let message = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            chat_call(&server.url, plain_client(RedirectPolicy::default())),
        )
        .await
        .expect("chunk-extension success terminates")
        .expect("cumulative extensions below the limit are accepted");
        assert_eq!(message.response_id.as_deref(), Some("chatcmpl-acc"));
        assert_eq!(server.stats.count(), 1);
        server.stop().await;
    }
    // Above the limit: 96 chunks x 300 extension bytes = 28800 > 16384.
    let ext = "e".repeat(300);
    let response = sse_multi_chunk_with_extensions(96, &ext);
    let server = start_raw(Arc::new(move |_: usize, _: &RecordedRequest| {
        Script::delivered(response.clone(), Delivery::Coalesced)
    }))
    .await;
    let error = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        chat_error(&server.url, plain_client(RedirectPolicy::default())),
    )
    .await
    .expect("chunk-extension rejection terminates");
    assert!(
        is_post_acceptance_stream_failure(&error),
        "chunk-extension overflow must surface as a body stream error: {error:?}"
    );
    assert_eq!(server.stats.count(), 1);
    server.stop().await;
}

#[tokio::test]
async fn single_extension_chunk_round_trip() {
    // A single small chunk extension is accepted and ignored upstream.
    let response = sse_response_with_chunk_extension("foo=bar");
    let server = start_raw(Arc::new(move |_: usize, _: &RecordedRequest| {
        Script::coalesced(response.clone())
    }))
    .await;
    let message = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        chat_call(&server.url, plain_client(RedirectPolicy::default())),
    )
    .await
    .expect("single extension terminates")
    .expect("single chunk extension accepted");
    assert_eq!(message.response_id.as_deref(), Some("chatcmpl-acc"));
    server.stop().await;
}
