//! Source-scoped HTTP/H1 acquisition, redirect/send-rule, proxy-tunnel and
//! upstream acceptance-boundary contract tests for the public `HttpClient`
//! and Models -> Chat injection path (task 20.12, RDF-22).
//!
//! Every case drives the real production seams: clients are built through
//! `opi_ai::http::HttpClientBuilder` (redirect policy, explicit proxy
//! configuration, source-scoped TLS options) and injected into
//! `OpenAiChatProvider::with_client` -> `Models::register_route` ->
//! `prepare_call` -> `start_attempt`. The loopback peers are scripted raw
//! TCP servers (plain and TLS) so the exact bytes hyper 1.9.0 accepts or
//! rejects are observable, including response-head growth past the initial
//! read buffer, trailer count/bytes limits, cumulative chunk-extension
//! limits, bytewise versus coalesced delivery, redirect chains with exact
//! lawful-send counts and cross-origin protected-header stripping, HTTPS
//! proxy outer TLS -> CONNECT -> inner TLS, scoped NO_PROXY bypass,
//! acquisition failure versus failure after an accepted partial response,
//! and single-terminal EOF with no hidden retries or sends. No raw
//! reqwest/rustls client is constructed by the tests; the unmodified
//! crates.io reqwest 0.12.28/Hyper 1.9.0 backend is exercised only through
//! the public `HttpClient` surface.

#![forbid(unsafe_code)]
// Only the CA and one server identity are needed here; the shared fixture
// module carries the full corpus for the TLS contract binary.
mod support {
    #[allow(dead_code)]
    pub(crate) mod tls_fixtures;
}
use support::tls_fixtures;

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use futures_util::StreamExt;
use opi_ai::auth::{AuthProvenanceSource, AuthScheme, StaticAuthResolver};
use opi_ai::http::{HttpClient, HttpClientBuilder, ProxyConfig, RedirectPolicy, TlsOptions};
use opi_ai::message::AssistantMessage;
use opi_ai::models::Models;
use opi_ai::openai_chat::OpenAiChatProvider;
use opi_ai::provider::{CacheRetention, EventStream, ProviderError, Request, ThinkingConfig};
use opi_ai::stream::ChatStream;
use opi_ai::transcript;
use rustls::{
    ServerConfig,
    crypto::aws_lc_rs,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
};
use secrecy::SecretString;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio_rustls::TlsAcceptor;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Scripted raw loopback peers
// ---------------------------------------------------------------------------

/// How a scripted response is written to the socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Delivery {
    /// One write of the full response bytes.
    Coalesced,
    /// One byte per write with a flush, exercising the parser at every
    /// possible split point.
    Bytewise,
}

/// A recorded inbound request (client side).
#[derive(Debug, Clone)]
struct RecordedRequest {
    method: String,
    /// Request-target exactly as received (origin- or absolute-form).
    target: String,
    authorization: Option<String>,
    content_length: usize,
}

/// A scripted response to write back.
struct Script {
    bytes: Vec<u8>,
    delivery: Delivery,
    /// Close the connection after the response instead of keep-alive.
    close: bool,
}

impl Script {
    fn coalesced(bytes: Vec<u8>) -> Self {
        Script {
            bytes,
            delivery: Delivery::Coalesced,
            close: true,
        }
    }
    fn delivered(bytes: Vec<u8>, delivery: Delivery) -> Self {
        Script {
            bytes,
            delivery,
            close: true,
        }
    }
}

type Responder = Arc<dyn Fn(usize, &RecordedRequest) -> Script + Send + Sync>;

#[derive(Default)]
struct RawStats {
    requests: Mutex<Vec<RecordedRequest>>,
    connections: AtomicUsize,
}

impl RawStats {
    fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().unwrap().clone()
    }
    fn count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

struct RawServer {
    url: String,
    stats: Arc<RawStats>,
    stop: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl RawServer {
    async fn stop(self) {
        let _ = self.stop.send(());
        let _ = self.task.await;
    }
}

/// Serve one plain-TCP scripted connection: parse one request, record it,
/// write the scripted response, and keep looping for keep-alive requests.
async fn serve_scripted(
    mut stream: TcpStream,
    stats: Arc<RawStats>,
    responder: Responder,
) -> std::io::Result<()> {
    loop {
        let recorded = match read_request(&mut stream).await? {
            Some(request) => request,
            None => return Ok(()), // peer closed between requests
        };
        let index = stats.count();
        stats.requests.lock().unwrap().push(recorded.clone());
        let script = responder(index, &recorded);
        deliver(&mut stream, &script.bytes, script.delivery).await?;
        if script.close {
            let _ = stream.shutdown().await;
            return Ok(());
        }
    }
}

/// Read one HTTP/1 request head (and body when Content-Length is present).
/// Returns `None` on a clean close before any bytes.
async fn read_request(stream: &mut TcpStream) -> std::io::Result<Option<RecordedRequest>> {
    let mut buf: Vec<u8> = Vec::with_capacity(2048);
    let mut scratch = [0u8; 4096];
    let head_end = loop {
        if let Some(pos) = find_head_end(&buf) {
            break pos;
        }
        let n = stream.read(&mut scratch).await?;
        if n == 0 {
            if buf.is_empty() {
                return Ok(None);
            }
            return Err(std::io::Error::other("request head truncated"));
        }
        buf.extend_from_slice(&scratch[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or_default().to_owned();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_owned();
    let target = parts.next().unwrap_or_default().to_owned();
    let mut authorization = None;
    let mut content_length = 0usize;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        match name.as_str() {
            "authorization" => authorization = Some(value.to_owned()),
            "content-length" => content_length = value.parse().unwrap_or(0),
            _ => {}
        }
    }
    // Drain exactly the declared body.
    let mut taken = buf.len() - head_end;
    while taken < content_length {
        let n = stream.read(&mut scratch).await?;
        if n == 0 {
            return Err(std::io::Error::other("request body truncated"));
        }
        taken += n;
    }
    Ok(Some(RecordedRequest {
        method,
        target,
        authorization,
        content_length,
    }))
}

fn find_head_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
}

async fn deliver(stream: &mut TcpStream, bytes: &[u8], delivery: Delivery) -> std::io::Result<()> {
    match delivery {
        Delivery::Coalesced => stream.write_all(bytes).await?,
        Delivery::Bytewise => {
            for byte in bytes {
                stream.write_all(std::slice::from_ref(byte)).await?;
                stream.flush().await?;
            }
        }
    }
    stream.flush().await
}

/// Start a plain scripted loopback server.
async fn start_raw(responder: Responder) -> RawServer {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind raw server");
    let addr = listener.local_addr().expect("raw server address");
    let stats = Arc::new(RawStats::default());
    let server_stats = stats.clone();
    let (stop, mut stop_rx) = oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        let mut connections = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                _ = &mut stop_rx => break,
                accepted = listener.accept() => {
                    let Ok((stream, _)) = accepted else { break };
                    server_stats.connections.fetch_add(1, Ordering::SeqCst);
                    let stats = server_stats.clone();
                    let responder = responder.clone();
                    connections.spawn(async move {
                        let _ = serve_scripted(stream, stats, responder).await;
                    });
                }
                Some(_) = connections.join_next(), if !connections.is_empty() => {}
            }
        }
        connections.abort_all();
        while connections.join_next().await.is_some() {}
    });
    RawServer {
        url: format!("http://{addr}"),
        stats,
        stop,
        task,
    }
}

// ---------------------------------------------------------------------------
// Response scripts
// ---------------------------------------------------------------------------

const SSE_BODY: &str = concat!(
    "data: {\"id\":\"chatcmpl-acc\",\"object\":\"chat.completion.chunk\",\"created\":1,",
    "\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},",
    "\"finish_reason\":null}]}\n\n",
    "data: {\"id\":\"chatcmpl-acc\",\"object\":\"chat.completion.chunk\",\"created\":1,",
    "\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],",
    "\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n",
    "data: [DONE]\n\n",
);

/// One SSE data event without the terminal DONE (for partial-body cases).
const SSE_PARTIAL: &str = concat!(
    "data: {\"id\":\"chatcmpl-acc\",\"object\":\"chat.completion.chunk\",\"created\":1,",
    "\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"partial\"},",
    "\"finish_reason\":null}]}\n\n",
);

fn chunk(bytes: &[u8]) -> String {
    format!("{:x}\r\n", bytes.len())
}

fn sse_head(pad_headers: &[String]) -> Vec<u8> {
    let mut head = String::from(
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n",
    );
    for pad in pad_headers {
        head.push_str(pad);
        head.push_str("\r\n");
    }
    // Terminate the header section with the blank line.
    head.push_str("\r\n");
    head.into_bytes()
}

/// Chunked SSE response (single data chunk) terminated by the final chunk.
fn sse_chunked_response() -> Vec<u8> {
    let mut out = sse_head(&[]);
    out.extend_from_slice(
        format!("{}{}\r\n0\r\n\r\n", chunk(SSE_BODY.as_bytes()), SSE_BODY).as_bytes(),
    );
    out
}

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
    // Without terminal usage the mapper retains its finish reason until
    // transport EOF, so the public result necessarily observes the trailers.
    let body = SSE_BODY.replace(
        ",\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}",
        "",
    );
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

fn redirect_response(status: u16, location: &str) -> Vec<u8> {
    format!("HTTP/1.1 {status} Moved\r\nlocation: {location}\r\ncontent-length: 0\r\n\r\n")
        .into_bytes()
}

// ---------------------------------------------------------------------------
// Models -> Chat driver over the real injection path
// ---------------------------------------------------------------------------

fn chat_request() -> Request {
    Request {
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
        context: transcript::normalize_context(transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

fn registered_models(client: Arc<HttpClient>, base_url: &str) -> Models {
    let provider = OpenAiChatProvider::with_client(
        Some(base_url.to_owned()),
        "openai".to_owned(),
        vec![],
        client,
    );
    let mut models = Models::new();
    models
        .register_route(
            Box::new(provider),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("acceptance-contract-key"),
            )),
            AuthProvenanceSource::Static,
            opi_ai::models::CompatMetadata {
                openai_compatible: true,
                profile: None,
            },
        )
        .expect("route registration");
    models
}

/// Drive one call to its terminal, returning the assistant message.
async fn chat_call(
    server_url: &str,
    client: Arc<HttpClient>,
) -> Result<AssistantMessage, ProviderError> {
    let models = registered_models(client, server_url);
    let prepared = models
        .prepare_call("openai:gpt-4o", chat_request())
        .await
        .expect("call preparation");
    let stream: EventStream = prepared.start_attempt().expect("attempt start");
    ChatStream::new(stream)
        .result()
        .await
        .map_err(|failure| failure.into_parts().1)
}

/// Drive one call until its first error terminal and return it.
async fn chat_error(server_url: &str, client: Arc<HttpClient>) -> ProviderError {
    let models = registered_models(client, server_url);
    let prepared = models
        .prepare_call("openai:gpt-4o", chat_request())
        .await
        .expect("call preparation");
    let mut stream: EventStream = prepared.start_attempt().expect("attempt start");
    while let Some(item) = stream.next().await {
        if let Err(error) = item {
            return error;
        }
    }
    panic!("negative case completed without a stream error");
}

fn plain_client(policy: RedirectPolicy) -> Arc<HttpClient> {
    Arc::new(
        HttpClientBuilder::new()
            .max_idle_per_host(0)
            .redirects(policy)
            .build()
            .expect("plain client build"),
    )
}

/// Post-acceptance stream failures surface through the adapter's
/// `StreamFailure` envelope (partial assistant state plus the typed
/// `StreamError` cause); before any content is accepted the bare
/// `StreamError` variant is produced.
fn is_post_acceptance_stream_failure(error: &ProviderError) -> bool {
    match error {
        ProviderError::StreamFailure(failure) => {
            matches!(failure.cause(), ProviderError::StreamError(_))
        }
        ProviderError::StreamError(_) => true,
        _ => false,
    }
}

fn bearer_of(request: &RecordedRequest) -> Option<&str> {
    request.authorization.as_deref()
}

// ---------------------------------------------------------------------------
// Redirect policy: defaults, same-origin, cross-origin, limits
// ---------------------------------------------------------------------------

#[test]
fn redirect_policy_defaults_to_limited_ten_and_is_configurable() {
    assert_eq!(
        HttpClient::new().redirect_policy(),
        RedirectPolicy::Limited(10)
    );
    assert_eq!(
        RedirectPolicy::default(),
        RedirectPolicy::Limited(10),
        "the source-qualified default must not silently track the backend"
    );
    let client = HttpClientBuilder::new()
        .redirects(RedirectPolicy::None)
        .build()
        .unwrap();
    assert_eq!(client.redirect_policy(), RedirectPolicy::None);
    let client = HttpClientBuilder::new()
        .redirects(RedirectPolicy::Limited(2))
        .build()
        .unwrap();
    assert_eq!(client.redirect_policy(), RedirectPolicy::Limited(2));
}

#[tokio::test]
async fn same_origin_redirect_keeps_authorization_with_exact_sends() {
    let server = start_raw(Arc::new(|index: usize, request: &RecordedRequest| {
        if index == 0 {
            assert_eq!(request.method, "POST", "original request is the POST");
            Script::coalesced(redirect_response(307, "/final"))
        } else {
            Script::coalesced(sse_chunked_response())
        }
    }))
    .await;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        chat_call(&server.url, plain_client(RedirectPolicy::default())),
    )
    .await
    .expect("same-origin redirect completes")
    .expect("same-origin redirect round trip");
    assert_eq!(result.response_id.as_deref(), Some("chatcmpl-acc"));
    let requests = server.stats.requests();
    assert_eq!(
        requests.len(),
        2,
        "exactly the original plus one hop: {requests:?}"
    );
    assert_eq!(requests[0].target, "/v1/chat/completions");
    assert_eq!(
        requests[1].target, "/final",
        "the hop follows the Location target"
    );
    for request in &requests {
        assert_eq!(
            bearer_of(request),
            Some("Bearer acceptance-contract-key"),
            "same-origin redirect must preserve the protected authorization header"
        );
        assert_eq!(
            request.content_length, 65,
            "307 preserves the POST method and body across the hop"
        );
    }
    server.stop().await;
}

#[tokio::test]
async fn cross_origin_redirect_strips_protected_authorization() {
    let origin = start_raw(Arc::new(|_: usize, _: &RecordedRequest| {
        Script::coalesced(redirect_response(307, "@@TARGET@@/v1/chat/completions"))
    }))
    .await;
    let target = start_raw(Arc::new(|_: usize, _: &RecordedRequest| {
        Script::coalesced(sse_chunked_response())
    }))
    .await;
    let redirect = redirect_response(307, &format!("{}/v1/chat/completions", target.url));
    let origin_server = origin;
    // Re-point the origin responder at the real target port by serving a
    // second scripted server whose bytes embed the target URL.
    origin_server.stop().await;
    let origin = start_raw(Arc::new(move |_: usize, _: &RecordedRequest| {
        Script::coalesced(redirect.clone())
    }))
    .await;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        chat_call(&origin.url, plain_client(RedirectPolicy::default())),
    )
    .await
    .expect("cross-origin redirect completes")
    .expect("cross-origin redirect round trip");
    assert_eq!(result.response_id.as_deref(), Some("chatcmpl-acc"));
    let origin_requests = origin.stats.requests();
    let target_requests = target.stats.requests();
    assert_eq!(origin_requests.len(), 1);
    assert_eq!(target_requests.len(), 1);
    assert_eq!(
        bearer_of(&origin_requests[0]),
        Some("Bearer acceptance-contract-key"),
        "the original request carries the authorization header"
    );
    assert_eq!(
        bearer_of(&target_requests[0]),
        None,
        "cross-origin redirect must strip the protected authorization header"
    );
    origin.stop().await;
    target.stop().await;
}

#[tokio::test]
async fn redirect_limit_enforced_with_exact_lawful_sends_and_typed_cause() {
    let server = start_raw(Arc::new(|_: usize, _: &RecordedRequest| {
        Script::coalesced(redirect_response(307, "/loop"))
    }))
    .await;
    let error = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        chat_error(&server.url, plain_client(RedirectPolicy::Limited(2))),
    )
    .await
    .expect("redirect loop terminates");
    match error {
        ProviderError::Network(summary) => {
            assert_eq!(
                summary.as_str(),
                "too many redirects",
                "typed redacted cause, no redirect target or URL leaks"
            );
        }
        other => panic!("expected Network, got {other:?}"),
    }
    let requests = server.stats.requests();
    assert_eq!(
        requests.len(),
        3,
        "1 original + 2 lawful hops, then refusal: {requests:?}"
    );
    server.stop().await;
}

#[tokio::test]
async fn redirect_none_returns_status_to_caller_with_single_send() {
    let server = start_raw(Arc::new(|_: usize, _: &RecordedRequest| {
        Script::coalesced(redirect_response(307, "/elsewhere"))
    }))
    .await;
    let error = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        chat_error(&server.url, plain_client(RedirectPolicy::None)),
    )
    .await
    .expect("no-redirect policy returns the status");
    match error {
        ProviderError::ProviderSide(summary) => {
            let text = summary.as_str();
            assert!(
                text.contains("307") || text.contains("3"),
                "status surfaces: {text}"
            );
        }
        other => panic!("expected ProviderSide, got {other:?}"),
    }
    assert_eq!(server.stats.count(), 1, "exactly one send, no follow");
    server.stop().await;
}

// ---------------------------------------------------------------------------
// Acquisition failure versus failure after accepted partial usage
// ---------------------------------------------------------------------------

#[tokio::test]
async fn acquisition_failure_before_any_request_is_typed_network() {
    // Reserve a port and close the listener so the target is unreachable.
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("reserve port");
    let addr = listener.local_addr().expect("reserved address");
    drop(listener);
    let error = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        chat_error(
            &format!("http://{addr}"),
            plain_client(RedirectPolicy::default()),
        ),
    )
    .await
    .expect("acquisition failure terminates");
    assert!(
        matches!(error, ProviderError::Network(_)),
        "acquisition-stage failure must stay typed Network: {error:?}"
    );
}

#[tokio::test]
async fn server_close_after_accepted_head_is_single_terminal_without_retry() {
    // Complete chunked body without the terminal SSE DONE: the response was
    // accepted (200 head + a content event), so the failure is a
    // post-acquisition stream error with one terminal and no retry.
    let body = format!(
        "{}{}\r\n0\r\n\r\n",
        chunk(SSE_PARTIAL.as_bytes()),
        SSE_PARTIAL
    );
    let mut script = sse_head(&[]);
    script.extend_from_slice(body.as_bytes());
    let server = start_raw(Arc::new(move |_: usize, _: &RecordedRequest| {
        Script::coalesced(script.clone())
    }))
    .await;
    let models = registered_models(plain_client(RedirectPolicy::default()), &server.url);
    let prepared = models
        .prepare_call("openai:gpt-4o", chat_request())
        .await
        .expect("prepare");
    let mut stream: EventStream = prepared.start_attempt().expect("attempt");
    let mut terminals = 0usize;
    let mut saw_error = false;
    let drained = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(item) = stream.next().await {
            match item {
                Ok(event) => assert!(!event.is_terminal(), "no success terminal expected"),
                Err(error) => {
                    saw_error = true;
                    assert!(
                        is_post_acceptance_stream_failure(&error),
                        "post-acceptance EOF must be typed StreamError: {error:?}"
                    );
                    terminals += 1;
                }
            }
        }
    })
    .await;
    assert!(drained.is_ok(), "stream reaches EOF");
    assert!(
        saw_error,
        "exactly one error terminal for the permanent EOF"
    );
    assert_eq!(terminals, 1);
    assert_eq!(
        server.stats.count(),
        1,
        "no hidden retry or extra send after EOF"
    );
    server.stop().await;
}

#[tokio::test]
async fn mid_body_abort_after_partial_events_is_stage_provable() {
    // Write the head plus half a chunk frame, then hard-close: the response
    // was accepted, partial content was delivered, and the truncated body
    // fails as a stream error rather than an acquisition failure.
    let body = format!("{}{}", chunk(SSE_PARTIAL.as_bytes()), SSE_PARTIAL);
    let mut script = sse_head(&[]);
    script.extend_from_slice(body.as_bytes());
    script.extend_from_slice(b"5\r\npar"); // truncated follow-up chunk
    let server = start_raw(Arc::new(move |_: usize, _: &RecordedRequest| {
        Script::coalesced(script.clone())
    }))
    .await;
    let error = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        chat_error(&server.url, plain_client(RedirectPolicy::default())),
    )
    .await
    .expect("mid-body abort terminates");
    assert!(
        is_post_acceptance_stream_failure(&error),
        "post-acceptance truncation must be typed StreamError: {error:?}"
    );
    assert_eq!(server.stats.count(), 1, "no retry after the aborted stream");
    server.stop().await;
}

// ---------------------------------------------------------------------------
// RDF-22 acceptance boundaries: response head, trailers, chunk extensions
// ---------------------------------------------------------------------------

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
                assert!(matches!(error, ProviderError::Network(_)), "{error:?}");
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

// ---------------------------------------------------------------------------
// HTTPS proxy: outer TLS -> CONNECT tunnel -> inner TLS target
// ---------------------------------------------------------------------------

fn tls_server_config(cert: &str, key: &str, alpn_h1: bool) -> ServerConfig {
    let cert = CertificateDer::from_pem_slice(cert.as_bytes()).expect("cert pem");
    let key = PrivateKeyDer::from_pem_slice(key.as_bytes()).expect("key pem");
    let provider = Arc::new(aws_lc_rs::default_provider());
    let mut config = ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])
        .expect("server versions")
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .expect("server identity");
    // The proxy layer sees no ALPN offer (reqwest clears proxy-connection
    // ALPN); the inner target selects HTTP/1.1 from the client's offer.
    config.alpn_protocols = if alpn_h1 {
        vec![b"http/1.1".to_vec()]
    } else {
        vec![]
    };
    config
}

/// A TLS-fronted scripted SSE target server.
async fn start_tls_target() -> (String, Arc<RawStats>, oneshot::Sender<()>, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind tls target");
    let addr = listener.local_addr().expect("tls target address");
    let acceptor = TlsAcceptor::from(Arc::new(tls_server_config(
        tls_fixtures::CERT_SERVER_RSA,
        tls_fixtures::KEY_SERVER_RSA,
        true,
    )));
    let stats = Arc::new(RawStats::default());
    let server_stats = stats.clone();
    let (stop, mut stop_rx) = oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        let mut connections = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                _ = &mut stop_rx => break,
                accepted = listener.accept() => {
                    let Ok((stream, _)) = accepted else { break };
                    let acceptor = acceptor.clone();
                    let stats = server_stats.clone();
                    connections.spawn(async move {
                        let Ok(tls) = acceptor.accept(stream).await else { return };
                        let mut tls = tls;
                        let responder: Responder = Arc::new(|_: usize, _: &RecordedRequest| {
                            Script::coalesced(sse_chunked_response())
                        });
                        let _ = serve_scripted_tls(&mut tls, stats, responder).await;
                    });
                }
                Some(_) = connections.join_next(), if !connections.is_empty() => {}
            }
        }
        connections.abort_all();
        while connections.join_next().await.is_some() {}
    });
    (format!("https://{addr}"), stats, stop, task)
}

/// Serve scripted HTTP/1 over an established TLS stream.
async fn serve_scripted_tls(
    tls: &mut tokio_rustls::server::TlsStream<TcpStream>,
    stats: Arc<RawStats>,
    responder: Responder,
) -> std::io::Result<()> {
    loop {
        let recorded = match read_request_tls(tls).await? {
            Some(request) => request,
            None => return Ok(()),
        };
        let index = stats.count();
        stats.requests.lock().unwrap().push(recorded.clone());
        let script = responder(index, &recorded);
        match script.delivery {
            Delivery::Coalesced => tls.write_all(&script.bytes).await?,
            Delivery::Bytewise => {
                for byte in &script.bytes {
                    tls.write_all(std::slice::from_ref(byte)).await?;
                    tls.flush().await?;
                }
            }
        }
        tls.flush().await?;
        if script.close {
            let _ = tls.shutdown().await;
            return Ok(());
        }
    }
}

/// Read one HTTP/1 request over TLS (same framing as the plain variant).
async fn read_request_tls(
    tls: &mut tokio_rustls::server::TlsStream<TcpStream>,
) -> std::io::Result<Option<RecordedRequest>> {
    let mut buf: Vec<u8> = Vec::with_capacity(2048);
    let mut scratch = [0u8; 4096];
    let head_end = loop {
        if let Some(pos) = find_head_end(&buf) {
            break pos;
        }
        let n = tls.read(&mut scratch).await?;
        if n == 0 {
            if buf.is_empty() {
                return Ok(None);
            }
            return Err(std::io::Error::other("request head truncated"));
        }
        buf.extend_from_slice(&scratch[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or_default().to_owned();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_owned();
    let target = parts.next().unwrap_or_default().to_owned();
    let mut authorization = None;
    let mut content_length = 0usize;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        match name.trim().to_ascii_lowercase().as_str() {
            "authorization" => authorization = Some(value.trim().to_owned()),
            "content-length" => content_length = value.trim().parse().unwrap_or(0),
            _ => {}
        }
    }
    let mut taken = buf.len() - head_end;
    while taken < content_length {
        let n = tls.read(&mut scratch).await?;
        if n == 0 {
            return Err(std::io::Error::other("request body truncated"));
        }
        taken += n;
    }
    Ok(Some(RecordedRequest {
        method,
        target,
        authorization,
        content_length,
    }))
}

/// An HTTPS proxy: outer TLS terminated at the proxy, then CONNECT tunnel.
async fn start_https_proxy(
    target_addr: std::net::SocketAddr,
) -> (
    String,
    Arc<AtomicUsize>,
    oneshot::Sender<()>,
    JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind https proxy");
    let addr = listener.local_addr().expect("https proxy address");
    let acceptor = TlsAcceptor::from(Arc::new(tls_server_config(
        tls_fixtures::CERT_SERVER_RSA,
        tls_fixtures::KEY_SERVER_RSA,
        false,
    )));
    let connects = Arc::new(AtomicUsize::new(0));
    let proxy_connects = connects.clone();
    let (stop, mut stop_rx) = oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        let mut tunnels = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                _ = &mut stop_rx => break,
                accepted = listener.accept() => {
                    let Ok((stream, _)) = accepted else { break };
                    let acceptor = acceptor.clone();
                    let connects = proxy_connects.clone();
                    tunnels.spawn(async move {
                        let Ok(mut tls) = acceptor.accept(stream).await else { return };
                        connects.fetch_add(1, Ordering::SeqCst);
                        // Read the CONNECT head.
                        let mut buf = Vec::with_capacity(512);
                        let mut scratch = [0u8; 1024];
                        let head_end = loop {
                            if let Some(pos) = find_head_end(&buf) { break pos; }
                            let n = match tls.read(&mut scratch).await { Ok(n) => n, Err(_) => return };
                            if n == 0 { return; }
                            buf.extend_from_slice(&scratch[..n]);
                        };
                        let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
                        if !head.starts_with("CONNECT ") { return; }
                        let _ = tls.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n").await;
                        let _ = tls.flush().await;
                        let Ok(mut target) = TcpStream::connect(target_addr).await else { return };
                        let _ = tokio::io::copy_bidirectional(&mut tls, &mut target).await;
                    });
                }
                Some(_) = tunnels.join_next(), if !tunnels.is_empty() => {}
            }
        }
        tunnels.abort_all();
        while tunnels.join_next().await.is_some() {}
    });
    (format!("https://{addr}"), connects, stop, task)
}

fn tls_proxy_client(proxy_url: &str, no_proxy: Option<&str>) -> Arc<HttpClient> {
    let options = TlsOptions::new()
        .with_explicit_roots_pem(tls_fixtures::CERT_CA.as_bytes())
        .expect("explicit roots");
    Arc::new(
        HttpClientBuilder::new()
            .max_idle_per_host(0)
            .tls(options)
            .proxy(ProxyConfig {
                url: Some(proxy_url.to_owned()),
                no_proxy: no_proxy.map(str::to_owned),
            })
            .build()
            .expect("proxied TLS client build"),
    )
}

#[tokio::test]
async fn https_proxy_outer_tls_connect_tunnel_inner_tls() {
    let (target_url, target_stats, target_stop, target_task) = start_tls_target().await;
    let target_addr: std::net::SocketAddr = target_url
        .trim_start_matches("https://")
        .parse()
        .expect("target address");
    let (proxy_url, connects, proxy_stop, proxy_task) = start_https_proxy(target_addr).await;
    let message = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        chat_call(&target_url, tls_proxy_client(&proxy_url, None)),
    )
    .await
    .expect("tunneled call terminates")
    .expect("outer TLS -> CONNECT -> inner TLS round trip");
    assert_eq!(message.response_id.as_deref(), Some("chatcmpl-acc"));
    assert_eq!(
        connects.load(Ordering::SeqCst),
        1,
        "exactly one CONNECT, no double wrap"
    );
    let requests = target_stats.requests();
    assert_eq!(requests.len(), 1, "one tunneled request to the target");
    assert_eq!(
        bearer_of(&requests[0]),
        Some("Bearer acceptance-contract-key"),
        "the tunneled request keeps its protected authorization header"
    );
    let _ = proxy_stop.send(());
    let _ = proxy_task.await;
    let _ = target_stop.send(());
    let _ = target_task.await;
}

#[tokio::test]
async fn scoped_no_proxy_bypasses_the_https_proxy() {
    let (target_url, target_stats, target_stop, target_task) = start_tls_target().await;
    let target_addr: std::net::SocketAddr = target_url
        .trim_start_matches("https://")
        .parse()
        .expect("target address");
    let (proxy_url, connects, proxy_stop, proxy_task) = start_https_proxy(target_addr).await;
    let message = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        chat_call(&target_url, tls_proxy_client(&proxy_url, Some("127.0.0.1"))),
    )
    .await
    .expect("bypassed call terminates")
    .expect("scoped NO_PROXY keeps the direct TLS path");
    assert_eq!(message.response_id.as_deref(), Some("chatcmpl-acc"));
    assert_eq!(
        connects.load(Ordering::SeqCst),
        0,
        "the scoped NO_PROXY entry bypasses the proxy entirely"
    );
    let requests = target_stats.requests();
    assert_eq!(requests.len(), 1, "direct connection to the target");
    let _ = proxy_stop.send(());
    let _ = proxy_task.await;
    let _ = target_stop.send(());
    let _ = target_task.await;
}
