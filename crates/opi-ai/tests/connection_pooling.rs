//! Connection pooling tests (task 3.13).
//!
//! Verifies tuned pool settings, H1 socket reuse after consuming responses,
//! and provider-level client reuse.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, body::Incoming, service::service_fn};
use hyper_util::rt::TokioIo;
use opi_ai::anthropic::AnthropicProvider;
use opi_ai::gemini::GeminiProvider;
use opi_ai::http::{HttpClient, HttpClientBuilder};
use opi_ai::openai_chat::OpenAiChatProvider;
use opi_ai::openai_responses::OpenAiResponsesProvider;
use tokio::{net::TcpListener, sync::oneshot, task::JoinSet, time::timeout};

// ---------------------------------------------------------------------------
// HttpClient defaults
// ---------------------------------------------------------------------------

#[test]
fn http_client_default_pool_settings() {
    let client = HttpClient::new();
    let (max_idle, idle_timeout) = client.pool_config();
    assert_eq!(max_idle, 10, "pool_max_idle_per_host should be 10");
    assert_eq!(idle_timeout, Duration::from_secs(90));
}

#[test]
fn http_client_builder_custom_settings() {
    let client = opi_ai::http::HttpClientBuilder::new()
        .max_idle_per_host(5)
        .idle_timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let (max_idle, idle_timeout) = client.pool_config();
    assert_eq!(max_idle, 5);
    assert_eq!(idle_timeout, Duration::from_secs(30));
}

// ---------------------------------------------------------------------------
// H1 socket reuse through the public client
// ---------------------------------------------------------------------------

async fn two_consumed_h1_calls(client: &HttpClient) -> (usize, usize) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(AtomicUsize::new(0));
    let server_requests = requests.clone();
    let (stop, mut stopped) = oneshot::channel();
    let server = tokio::spawn(async move {
        let mut connections = JoinSet::new();
        let mut accepted = 0;
        loop {
            tokio::select! {
                _ = &mut stopped => break,
                connection = listener.accept() => {
                    let (stream, _) = connection.unwrap();
                    accepted += 1;
                    let requests = server_requests.clone();
                    connections.spawn(async move {
                        let service = service_fn(move |request: Request<Incoming>| {
                            let requests = requests.clone();
                            async move {
                                let path = request.uri().path().to_owned();
                                let body = request.into_body().collect().await.unwrap();
                                assert!(body.to_bytes().is_empty());
                                requests.fetch_add(1, Ordering::SeqCst);
                                Ok::<_, std::convert::Infallible>(Response::new(Full::new(
                                    hyper::body::Bytes::from(path),
                                )))
                            }
                        });
                        hyper::server::conn::http1::Builder::new()
                            .keep_alive(true)
                            .serve_connection(TokioIo::new(stream), service)
                            .await
                            .unwrap();
                    });
                }
                Some(result) = connections.join_next(), if !connections.is_empty() => {
                    result.unwrap();
                }
            }
        }
        connections.abort_all();
        while let Some(result) = connections.join_next().await {
            if let Err(error) = result {
                assert!(error.is_cancelled(), "H1 connection task failed: {error}");
            }
        }
        accepted
    });

    timeout(Duration::from_secs(5), async {
        for path in ["/first", "/second"] {
            let response = client
                .client()
                .get(format!("{url}{path}"))
                .send()
                .await
                .unwrap();
            assert_eq!(response.version(), reqwest::Version::HTTP_11);
            assert_eq!(response.status(), reqwest::StatusCode::OK);
            assert_eq!(response.bytes().await.unwrap().as_ref(), path.as_bytes());
        }
    })
    .await
    .expect("two fully consumed H1 calls should complete");

    stop.send(()).unwrap();
    let accepted = timeout(Duration::from_secs(5), server)
        .await
        .expect("H1 peer should stop and release its sockets")
        .unwrap();
    (requests.load(Ordering::SeqCst), accepted)
}

#[tokio::test]
async fn http_client_reuses_h1_socket_after_fully_consumed_calls() {
    let client = HttpClient::new();
    assert_eq!(
        two_consumed_h1_calls(&client).await,
        (2, 1),
        "two completed requests on one shared client should reuse one socket"
    );

    // The same peer accepts a second socket when the public builder disables
    // idle pooling, so the reuse assertion does not follow from server setup.
    let no_idle_pool = HttpClientBuilder::new()
        .max_idle_per_host(0)
        .build()
        .unwrap();
    assert_eq!(
        two_consumed_h1_calls(&no_idle_pool).await,
        (2, 2),
        "disabling idle pooling should require one socket per request"
    );
}

// ---------------------------------------------------------------------------
// Provider holds shared client
// ---------------------------------------------------------------------------

#[test]
fn anthropic_provider_reuses_shared_client() {
    let client = Arc::new(HttpClient::new());
    let client_ptr = Arc::as_ptr(&client);

    let provider = AnthropicProvider::with_client(None, client.clone());
    assert!(
        Arc::ptr_eq(&client, provider.http_client()),
        "provider should hold the same Arc<HttpClient> instance"
    );
    assert_eq!(
        Arc::as_ptr(&client),
        client_ptr,
        "no new Arc should be created"
    );
}

#[test]
fn openai_chat_provider_reuses_shared_client() {
    let client = Arc::new(HttpClient::new());
    let provider = OpenAiChatProvider::with_client(None, "openai".into(), vec![], client.clone());
    assert!(Arc::ptr_eq(&client, provider.http_client()));
}

#[test]
fn openai_responses_provider_reuses_shared_client() {
    let client = Arc::new(HttpClient::new());
    let provider = OpenAiResponsesProvider::with_client(None, client.clone());
    assert!(Arc::ptr_eq(&client, provider.http_client()));
}

#[test]
fn gemini_provider_reuses_shared_client() {
    let client = Arc::new(HttpClient::new());
    let provider = GeminiProvider::with_client(None, client.clone());
    assert!(Arc::ptr_eq(&client, provider.http_client()));
}

// ---------------------------------------------------------------------------
// No per-request allocation
// ---------------------------------------------------------------------------

#[test]
fn provider_new_creates_single_client() {
    // Providers created with new() should internally create exactly one
    // HttpClient. Two providers create two separate clients.
    let p1 = AnthropicProvider::new(None);
    let p2 = AnthropicProvider::new(None);

    // Each provider has its own client (not shared unless explicitly shared).
    assert!(
        !Arc::ptr_eq(p1.http_client(), p2.http_client()),
        "separate new() calls should create separate clients"
    );
}

#[test]
fn shared_client_across_providers() {
    // A single Arc<HttpClient> can be shared across multiple providers.
    let client = Arc::new(HttpClient::new());
    let p1 = AnthropicProvider::with_client(None, client.clone());
    let p2 = AnthropicProvider::with_client(None, client.clone());

    assert!(Arc::ptr_eq(p1.http_client(), p2.http_client()));
    assert_eq!(Arc::strong_count(&client), 3); // original + 2 providers
}

// ---------------------------------------------------------------------------
// Pool config is visible through provider
// ---------------------------------------------------------------------------

#[test]
fn provider_client_has_tuned_pool_config() {
    let provider = AnthropicProvider::new(None);
    let (max_idle, idle_timeout) = provider.http_client().pool_config();
    assert_eq!(max_idle, 10);
    assert_eq!(idle_timeout, Duration::from_secs(90));
}
