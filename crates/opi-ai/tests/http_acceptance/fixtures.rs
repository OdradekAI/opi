//! Scripted loopback HTTP/1 peers, request capture, and the Models Chat driver.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use futures_util::StreamExt;
use opi_ai::auth::{AuthProvenanceSource, AuthScheme, StaticAuthResolver};
use opi_ai::http::{HttpClient, HttpClientBuilder, RedirectPolicy};
use opi_ai::message::AssistantMessage;
use opi_ai::models::Models;
use opi_ai::provider::{CacheRetention, EventStream, ProviderError, Request, ThinkingConfig};
use opi_ai::providers::openai::OpenAiChatProvider;
use opi_ai::stream::ChatStream;
use opi_ai::transcript;
use secrecy::SecretString;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// How a scripted response is written to the socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Delivery {
    /// One write of the full response bytes.
    Coalesced,
    /// One byte per write with a flush, exercising the parser at every
    /// possible split point.
    Bytewise,
}

/// A recorded inbound request (client side).
#[derive(Debug, Clone)]
pub(super) struct RecordedRequest {
    pub(super) method: String,
    /// Request-target exactly as received (origin- or absolute-form).
    pub(super) target: String,
    authorization: Option<String>,
    pub(super) referer: Option<String>,
    pub(super) content_length: usize,
    pub(super) body: Vec<u8>,
}

/// A scripted response to write back.
pub(super) struct Script {
    bytes: Vec<u8>,
    delivery: Delivery,
    /// Close the connection after the response instead of keep-alive.
    close: bool,
}

impl Script {
    pub(super) fn coalesced(bytes: Vec<u8>) -> Self {
        Script {
            bytes,
            delivery: Delivery::Coalesced,
            close: true,
        }
    }
    pub(super) fn delivered(bytes: Vec<u8>, delivery: Delivery) -> Self {
        Script {
            bytes,
            delivery,
            close: true,
        }
    }
}

pub(super) type Responder = Arc<dyn Fn(usize, &RecordedRequest) -> Script + Send + Sync>;

#[derive(Default)]
pub(super) struct RawStats {
    requests: Mutex<Vec<RecordedRequest>>,
    connections: AtomicUsize,
}

impl RawStats {
    pub(super) fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().unwrap().clone()
    }
    pub(super) fn count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

pub(super) struct RawServer {
    pub(super) url: String,
    pub(super) stats: Arc<RawStats>,
    stop: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl RawServer {
    pub(super) async fn stop(self) {
        let _ = self.stop.send(());
        let _ = self.task.await;
    }
}

/// Serve scripted HTTP/1 on an established TCP or TLS stream.
pub(super) async fn serve_scripted(
    stream: &mut (impl AsyncRead + AsyncWrite + Unpin),
    stats: Arc<RawStats>,
    responder: Responder,
) -> std::io::Result<()> {
    loop {
        let recorded = match read_request(stream).await? {
            Some(request) => request,
            None => return Ok(()), // peer closed between requests
        };
        let index = stats.count();
        stats.requests.lock().unwrap().push(recorded.clone());
        let script = responder(index, &recorded);
        deliver(stream, &script.bytes, script.delivery).await?;
        if script.close {
            let _ = stream.shutdown().await;
            return Ok(());
        }
    }
}

/// Read one HTTP/1 request head (and body when Content-Length is present).
/// Returns `None` on a clean close before any bytes.
pub(super) async fn read_request(
    stream: &mut (impl AsyncRead + Unpin),
) -> std::io::Result<Option<RecordedRequest>> {
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
    let mut referer = None;
    let mut content_length = 0usize;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        match name.as_str() {
            "authorization" => authorization = Some(value.to_owned()),
            "referer" => referer = Some(value.to_owned()),
            "content-length" => content_length = value.parse().unwrap_or(0),
            _ => {}
        }
    }
    let mut body = buf[head_end..].to_vec();
    body.truncate(content_length);
    while body.len() < content_length {
        let n = stream.read(&mut scratch).await?;
        if n == 0 {
            return Err(std::io::Error::other("request body truncated"));
        }
        let remaining = content_length - body.len();
        body.extend_from_slice(&scratch[..n.min(remaining)]);
    }
    Ok(Some(RecordedRequest {
        method,
        target,
        authorization,
        referer,
        content_length,
        body,
    }))
}

pub(super) fn find_head_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
}

async fn deliver(
    stream: &mut (impl AsyncWrite + Unpin),
    bytes: &[u8],
    delivery: Delivery,
) -> std::io::Result<()> {
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
pub(super) async fn start_raw(responder: Responder) -> RawServer {
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
                    let Ok((mut stream, _)) = accepted else { break };
                    server_stats.connections.fetch_add(1, Ordering::SeqCst);
                    let stats = server_stats.clone();
                    let responder = responder.clone();
                    connections.spawn(async move {
                        let _ = serve_scripted(&mut stream, stats, responder).await;
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

pub(super) const SSE_BODY: &str = concat!(
    "data: {\"id\":\"chatcmpl-acc\",\"object\":\"chat.completion.chunk\",\"created\":1,",
    "\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},",
    "\"finish_reason\":null}]}\n\n",
    "data: {\"id\":\"chatcmpl-acc\",\"object\":\"chat.completion.chunk\",\"created\":1,",
    "\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],",
    "\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n",
    "data: [DONE]\n\n",
);

pub(super) fn chunk(bytes: &[u8]) -> String {
    format!("{:x}\r\n", bytes.len())
}

pub(super) fn sse_head(pad_headers: &[String]) -> Vec<u8> {
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
pub(super) fn sse_chunked_response() -> Vec<u8> {
    let mut out = sse_head(&[]);
    out.extend_from_slice(
        format!("{}{}\r\n0\r\n\r\n", chunk(SSE_BODY.as_bytes()), SSE_BODY).as_bytes(),
    );
    out
}

pub(super) fn chat_request() -> Request {
    Request {
        projection: Default::default(),
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

pub(super) fn registered_models(client: Arc<HttpClient>, base_url: &str) -> Models {
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
pub(super) async fn chat_call(
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
pub(super) async fn chat_error(server_url: &str, client: Arc<HttpClient>) -> ProviderError {
    let models = registered_models(client, server_url);
    let request = chat_request();
    let prepared = models
        .prepare_call("openai:gpt-4o", request.clone())
        .await
        .expect("call preparation");
    let mut stream: EventStream = prepared.start_attempt().expect("attempt start");
    while let Some(item) = stream.next().await {
        if let Err(error) = item {
            accepted_failure_cause(&error, &request);
            assert!(
                stream.next().await.is_none(),
                "negative call has one terminal failure"
            );
            return error;
        }
    }
    panic!("negative case completed without a stream error");
}

pub(super) fn plain_client(policy: RedirectPolicy) -> Arc<HttpClient> {
    Arc::new(
        HttpClientBuilder::new()
            .max_idle_per_host(0)
            .redirects(policy)
            .build()
            .expect("plain client build"),
    )
}

/// Stream failures after successful projection compilation retain known
/// projection facts in `StreamFailure`, including before content arrives.
/// Collector-returned causes can also be classified directly.
pub(super) fn is_post_acceptance_stream_failure(error: &ProviderError) -> bool {
    match error {
        ProviderError::StreamFailure(failure) => {
            matches!(failure.cause(), ProviderError::StreamError(_))
        }
        ProviderError::StreamError(_) => true,
        _ => false,
    }
}

pub(super) fn bearer_of(request: &RecordedRequest) -> Option<&str> {
    request.authorization.as_deref()
}

pub(super) fn accepted_failure_cause<'a>(
    error: &'a opi_ai::provider::ProviderError,
    request: &opi_ai::provider::Request,
) -> &'a opi_ai::provider::ProviderError {
    use opi_ai::provider::ProviderError;
    use sha2::Digest as _;
    let ProviderError::StreamFailure(failure) = error else {
        panic!("accepted projection must retain its failure carrier: {error:?}");
    };
    let facts = failure
        .message()
        .projection
        .as_ref()
        .expect("accepted projection is known");
    let bytes = serde_json::to_vec(request.context.messages()).unwrap();
    assert_eq!(
        facts.source_prefix.message_count,
        request.context.messages().len()
    );
    assert_eq!(
        facts.source_prefix.sha256,
        format!("{:x}", sha2::Sha256::digest(bytes))
    );
    assert_eq!(
        facts.temporal.mode,
        opi_ai::transcript::ProjectionMode::Lossless
    );
    assert!(facts.temporal.affected_ranges.is_empty());
    assert!(facts.replay.losses.is_empty());
    failure.cause()
}
