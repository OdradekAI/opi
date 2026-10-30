//! Packaged-HTTP public consumer proof (task 20.12, RDF-22).
//!
//! An ordinary workspace-external Cargo consumer exercising the real public
//! `opi-ai` HTTP/Chat path from the extracted `cargo package` artifact:
//! `HttpClientBuilder::redirects/proxy` -> `OpenAiChatProvider::with_client`
//! -> `Models::register_route` -> `prepare_call` -> `start_attempt`, against
//! scripted loopback TCP peers. No raw reqwest client construction for the
//! SDK path, no privileged helper, no test-only SDK API.
//!
//! With `caller-union` the consumer's own graph additionally enables the
//! reqwest gzip feature, proving the recorded acceptance profile is
//! unchanged under a caller feature union.
//!
//! Prints one JSON report: {"verdict", "profile", "total", "failed",
//! "checks": [...]}.

#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}};

use futures_util::StreamExt;
use opi_ai::auth::{AuthProvenanceSource, AuthScheme, StaticAuthResolver};
use opi_ai::http::{HttpClient, HttpClientBuilder, ProxyConfig, RedirectPolicy};
use opi_ai::message::AssistantMessage;
use opi_ai::models::Models;
use opi_ai::openai_chat::OpenAiChatProvider;
use opi_ai::provider::{CacheRetention, EventStream, ProviderError, Request, ThinkingConfig};
use opi_ai::stream::ChatStream;
use opi_ai::transcript;
use secrecy::SecretString;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const SSE_BODY: &str = concat!(
    "data: {\"id\":\"chatcmpl-pkg\",\"object\":\"chat.completion.chunk\",\"created\":1,",
    "\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},",
    "\"finish_reason\":null}]}\n\n",
    "data: {\"id\":\"chatcmpl-pkg\",\"object\":\"chat.completion.chunk\",\"created\":1,",
    "\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],",
    "\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n",
    "data: [DONE]\n\n",
);

/// One SSE data event without the terminal DONE (partial-body case).
const SSE_PARTIAL: &str = concat!(
    "data: {\"id\":\"chatcmpl-pkg\",\"object\":\"chat.completion.chunk\",\"created\":1,",
    "\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"partial\"},",
    "\"finish_reason\":null}]}\n\n",
);

fn check(report: &mut Vec<serde_json::Value>, name: &str, passed: bool, detail: String) {
    report.push(serde_json::json!({"name": name, "passed": passed, "detail": detail}));
}

// ---------------------------------------------------------------------------
// Scripted raw loopback peers (plain TCP)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct RecordedRequest {
    method: String,
    target: String,
    authorization: Option<String>,
    content_length: usize,
}

struct Script {
    bytes: Vec<u8>,
    close: bool,
}

impl Script {
    fn coalesced(bytes: Vec<u8>) -> Self {
        Script { bytes, close: true }
    }
}

type Responder = Arc<dyn Fn(usize, &RecordedRequest) -> Script + Send + Sync>;

#[derive(Default)]
struct RawStats {
    requests: Mutex<Vec<RecordedRequest>>,
}

impl RawStats {
    fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().unwrap().clone()
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

async fn serve_scripted(mut stream: TcpStream, stats: Arc<RawStats>, responder: Responder) -> std::io::Result<()> {
    loop {
        let recorded = match read_request(&mut stream).await? {
            Some(request) => request,
            None => return Ok(()),
        };
        let index = stats.requests.lock().unwrap().len();
        stats.requests.lock().unwrap().push(recorded.clone());
        let script = responder(index, &recorded);
        stream.write_all(&script.bytes).await?;
        stream.flush().await?;
        if script.close {
            let _ = stream.shutdown().await;
            return Ok(());
        }
    }
}

async fn read_request(stream: &mut TcpStream) -> std::io::Result<Option<RecordedRequest>> {
    let mut buf: Vec<u8> = Vec::with_capacity(2048);
    let mut scratch = [0u8; 4096];
    let head_end = loop {
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos + 4;
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
        let Some((name, value)) = line.split_once(':') else { continue };
        match name.trim().to_ascii_lowercase().as_str() {
            "authorization" => authorization = Some(value.trim().to_owned()),
            "content-length" => content_length = value.trim().parse().unwrap_or(0),
            _ => {}
        }
    }
    let mut taken = buf.len() - head_end;
    while taken < content_length {
        let n = stream.read(&mut scratch).await?;
        if n == 0 {
            return Err(std::io::Error::other("request body truncated"));
        }
        taken += n;
    }
    Ok(Some(RecordedRequest { method, target, authorization, content_length }))
}

async fn start_raw(responder: Responder) -> RawServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind raw server");
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
    RawServer { url: format!("http://{addr}"), stats, stop, task }
}

fn sse_response() -> Vec<u8> {
    format!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\r\n{:x}\r\n{}\r\n0\r\n\r\n",
        SSE_BODY.len(),
        SSE_BODY
    )
    .into_bytes()
}

fn sse_partial_response() -> Vec<u8> {
    format!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\r\n{:x}\r\n{}\r\n0\r\n\r\n",
        SSE_PARTIAL.len(),
        SSE_PARTIAL
    )
    .into_bytes()
}

fn redirect_response(status: u16, location: &str) -> Vec<u8> {
    format!("HTTP/1.1 {status} Moved\r\nlocation: {location}\r\ncontent-length: 0\r\n\r\n").into_bytes()
}

// ---------------------------------------------------------------------------
// Models -> Chat driver over the real public injection path
// ---------------------------------------------------------------------------

fn chat_request() -> Request {
    Request {
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
        callbacks: Default::default(),
        telemetry_context: None,
        transport: Default::default(),
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
                SecretString::from("pkg-consumer-key"),
            )),
            AuthProvenanceSource::Static,
            opi_ai::models::CompatMetadata { openai_compatible: true, profile: None },
        )
        .expect("route registration");
    models
}

async fn chat_call(server_url: &str, client: Arc<HttpClient>) -> Result<AssistantMessage, ProviderError> {
    let models = registered_models(client, server_url);
    let prepared = models.prepare_call("openai:gpt-4o", chat_request()).await.expect("call preparation");
    let stream: EventStream = prepared.start_attempt().expect("attempt start");
    ChatStream::new(stream).result().await.map_err(|failure| failure.into_parts().1)
}

async fn chat_error(server_url: &str, client: Arc<HttpClient>) -> ProviderError {
    let models = registered_models(client, server_url);
    let prepared = models.prepare_call("openai:gpt-4o", chat_request()).await.expect("call preparation");
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

fn bearer_of(request: &RecordedRequest) -> Option<&str> {
    request.authorization.as_deref()
}

fn is_post_acceptance_stream_failure(error: &ProviderError) -> bool {
    match error {
        ProviderError::StreamFailure(failure) => {
            matches!(failure.cause(), ProviderError::StreamError(_))
        }
        ProviderError::StreamError(_) => true,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Battery
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let profile = if cfg!(feature = "caller-union") { "caller-union" } else { "ordinary" };
    let mut report: Vec<serde_json::Value> = Vec::new();

    // 1. The redirect policy is owned SDK surface with a declared default.
    let owned_default = HttpClient::new().redirect_policy() == RedirectPolicy::Limited(10)
        && HttpClientBuilder::new()
            .redirects(RedirectPolicy::None)
            .build()
            .expect("build none")
            .redirect_policy()
            == RedirectPolicy::None;
    check(
        &mut report,
        "redirect-policy-owned-default",
        owned_default,
        "HttpClient default is Limited(10) and configurable".to_owned(),
    );

    // 2. Same-origin redirect keeps the protected authorization header with
    //    exactly two lawful sends.
    let server = start_raw(Arc::new(|index: usize, request: &RecordedRequest| {
        assert_eq!(request.method, "POST");
        if index == 0 {
            Script::coalesced(redirect_response(307, "/final"))
        } else {
            Script::coalesced(sse_response())
        }
    }))
    .await;
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        chat_call(&server.url, plain_client(RedirectPolicy::default())),
    )
    .await;
    let message = outcome.expect("same-origin terminates").expect("same-origin round trip");
    let requests = server.stats.requests();
    let same_origin_ok = message.response_id.as_deref() == Some("chatcmpl-pkg")
        && requests.len() == 2
        && requests[1].target == "/final"
        && requests.iter().all(|r| {
            bearer_of(r) == Some("Bearer pkg-consumer-key") && r.method == "POST" && r.content_length > 0
        });
    check(
        &mut report,
        "same-origin-redirect-preserves-authorization",
        same_origin_ok,
        format!("sends={} hop2_target={:?}", requests.len(), requests.get(1).map(|r| r.target.clone())),
    );
    server.stop().await;

    // 3. Cross-origin redirect strips the protected authorization header.
    let target = start_raw(Arc::new(|_: usize, _: &RecordedRequest| Script::coalesced(sse_response()))).await;
    let location = redirect_response(307, &format!("{}/v1/chat/completions", target.url));
    let origin = start_raw(Arc::new(move |_: usize, _: &RecordedRequest| {
        Script::coalesced(location.clone())
    }))
    .await;
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        chat_call(&origin.url, plain_client(RedirectPolicy::default())),
    )
    .await;
    let message = outcome.expect("cross-origin terminates").expect("cross-origin round trip");
    let origin_requests = origin.stats.requests();
    let target_requests = target.stats.requests();
    let cross_ok = message.response_id.as_deref() == Some("chatcmpl-pkg")
        && origin_requests.len() == 1
        && target_requests.len() == 1
        && bearer_of(&origin_requests[0]) == Some("Bearer pkg-consumer-key")
        && bearer_of(&target_requests[0]).is_none();
    check(
        &mut report,
        "cross-origin-redirect-strips-authorization",
        cross_ok,
        format!(
            "origin_auth={:?} target_auth={:?} target_sends={}",
            bearer_of(&origin_requests[0]),
            bearer_of(&target_requests[0]),
            target_requests.len()
        ),
    );
    origin.stop().await;
    target.stop().await;

    // 4. The redirect limit fails typed with the exact lawful send count.
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
    let limit_ok = matches!(&error, ProviderError::Network(summary) if summary.as_str() == "too many redirects")
        && server.stats.requests().len() == 3;
    check(
        &mut report,
        "redirect-limit-typed-failure-exact-sends",
        limit_ok,
        format!("error={error:?} sends={}", server.stats.requests().len()),
    );
    server.stop().await;

    // 5. Explicit proxy routing with a scoped bypass: through the proxy the
    //    request arrives in absolute form with the upstream Host; with the
    //    scoped no_proxy entry the proxy sees zero connections.
    let target = start_raw(Arc::new(|_: usize, _: &RecordedRequest| Script::coalesced(sse_response()))).await;
    let target_addr: std::net::SocketAddr = target
        .url
        .trim_start_matches("http://")
        .parse()
        .expect("target address");
    let proxy_hits = Arc::new(AtomicUsize::new(0));
    let absolute_targets: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let hits = proxy_hits.clone();
    let seen = absolute_targets.clone();
    let proxy_listener = TcpListener::bind("127.0.0.1:0").await.expect("bind proxy");
    let proxy_addr = proxy_listener.local_addr().expect("proxy address");
    let (proxy_stop, mut proxy_stop_rx) = oneshot::channel::<()>();
    let proxy_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut proxy_stop_rx => break,
                accepted = proxy_listener.accept() => {
                    let Ok((mut stream, _)) = accepted else { break };
                    let hits = hits.clone();
                    let seen = seen.clone();
                    tokio::spawn(async move {
                        hits.fetch_add(1, Ordering::SeqCst);
                        // Read the full request, then forward it to the
                        // upstream in origin form and relay the response.
                        let mut buf = Vec::with_capacity(2048);
                        let mut scratch = [0u8; 4096];
                        let head_end = loop {
                            if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                                break pos + 4;
                            }
                            let n = match stream.read(&mut scratch).await { Ok(n) => n, Err(_) => return };
                            if n == 0 { return; }
                            buf.extend_from_slice(&scratch[..n]);
                        };
                        let head_text = String::from_utf8_lossy(&buf[..head_end]).into_owned();
                        let mut lines = head_text.split("\r\n");
                        let request_line = lines.next().unwrap_or_default().to_owned();
                        let mut parts = request_line.split_whitespace();
                        let method = parts.next().unwrap_or_default().to_owned();
                        let absolute = parts.next().unwrap_or_default().to_owned();
                        let version = parts.next().unwrap_or_default().to_owned();
                        let mut content_length = 0usize;
                        let mut host = None;
                        for line in lines {
                            let Some((name, value)) = line.split_once(':') else { continue };
                            if name.trim().eq_ignore_ascii_case("content-length") {
                                content_length = value.trim().parse().unwrap_or(0);
                            } else if name.trim().eq_ignore_ascii_case("host") {
                                host = Some(value.trim().to_owned());
                            }
                        }
                        seen.lock().unwrap().push(absolute.clone());
                        let path = absolute
                            .splitn(4, '/')
                            .nth(3)
                            .map(|rest| format!("/{rest}"))
                            .unwrap_or_else(|| "/".to_owned());
                        let mut taken = buf.len() - head_end;
                        while taken < content_length {
                            let n = match stream.read(&mut scratch).await { Ok(n) => n, Err(_) => return };
                            if n == 0 { return; }
                            taken += n;
                            buf.extend_from_slice(&scratch[..n]);
                        }
                        let Ok(mut upstream) = TcpStream::connect(target_addr).await else { return };
                        let mut forwarded = format!("{method} {path} {version}\r\n").into_bytes();
                        if let Some(host) = host {
                            forwarded.extend_from_slice(format!("host: {host}\r\n").as_bytes());
                        }
                        forwarded.extend_from_slice(b"\r\n");
                        forwarded.extend_from_slice(&buf[head_end..]);
                        if upstream.write_all(&forwarded).await.is_err() { return; }
                        let _ = tokio::io::copy_bidirectional(&mut upstream, &mut stream).await;
                    });
                }
            }
        }
    });
    let proxied = Arc::new(
        HttpClientBuilder::new()
            .max_idle_per_host(0)
            .proxy(ProxyConfig { url: Some(format!("http://{proxy_addr}")), no_proxy: None })
            .build()
            .expect("proxied client"),
    );
    let through = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        chat_call(&target.url, proxied),
    )
    .await
    .expect("proxied call terminates")
    .expect("proxied round trip");
    let bypassed = Arc::new(
        HttpClientBuilder::new()
            .max_idle_per_host(0)
            .proxy(ProxyConfig {
                url: Some(format!("http://{proxy_addr}")),
                no_proxy: Some("127.0.0.1".to_owned()),
            })
            .build()
            .expect("bypassing client"),
    );
    let direct = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        chat_call(&target.url, bypassed),
    )
    .await
    .expect("bypassed call terminates")
    .expect("bypassed round trip");
    let proxy_ok = through.response_id.as_deref() == Some("chatcmpl-pkg")
        && direct.response_id.as_deref() == Some("chatcmpl-pkg")
        && proxy_hits.load(Ordering::SeqCst) == 1
        && target.stats.requests().len() == 2
        && absolute_targets
            .lock()
            .unwrap()
            .first()
            .map(|t| t.starts_with("http://"))
            == Some(true);
    check(
        &mut report,
        "proxy-routing-with-scoped-bypass",
        proxy_ok,
        format!(
            "proxy_hits={} absolute_target={:?} target_sends={}",
            proxy_hits.load(Ordering::SeqCst),
            absolute_targets.lock().unwrap().first().cloned(),
            target.stats.requests().len()
        ),
    );
    let _ = proxy_stop.send(());
    let _ = proxy_task.await;
    target.stop().await;

    // 6. Typed acquisition failure and single-terminal EOF without retry.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("reserve closed port");
    let closed = listener.local_addr().expect("closed address");
    drop(listener);
    let acquisition = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        chat_error(&format!("http://{closed}"), plain_client(RedirectPolicy::default())),
    )
    .await
    .expect("acquisition failure terminates");
    let eof_server = start_raw(Arc::new(|_: usize, _: &RecordedRequest| {
        Script::coalesced(sse_partial_response())
    }))
    .await;
    let eof_error = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        chat_error(&eof_server.url, plain_client(RedirectPolicy::default())),
    )
    .await
    .expect("EOF case terminates");
    let stage_ok = matches!(acquisition, ProviderError::Network(_))
        && is_post_acceptance_stream_failure(&eof_error)
        && eof_server.stats.requests().len() == 1;
    check(
        &mut report,
        "acquisition-failure-and-single-terminal-eof",
        stage_ok,
        format!(
            "acquisition={acquisition:?} eof={eof_error:?} sends={}",
            eof_server.stats.requests().len()
        ),
    );
    eof_server.stop().await;

    let failed = report.iter().filter(|c| !c["passed"].as_bool().unwrap_or(false)).count();
    println!(
        "{}",
        serde_json::json!({
            "verdict": if failed == 0 { "PASS" } else { "FAIL" },
            "profile": profile,
            "total": report.len(),
            "failed": failed,
            "checks": report,
        })
    );
    std::process::exit(if failed == 0 { 0 } else { 1 });
}
