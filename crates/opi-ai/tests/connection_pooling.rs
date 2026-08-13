//! Connection pooling tests (task 3.13).
//!
//! Verifies shared reqwest Client construction with tuned pool settings,
//! no per-request client allocation, and provider-level client reuse.

use std::sync::Arc;
use std::time::Duration;

use opi_ai::anthropic::AnthropicProvider;
use opi_ai::gemini::GeminiProvider;
use opi_ai::http::HttpClient;
use opi_ai::openai_chat::OpenAiChatProvider;
use opi_ai::openai_responses::OpenAiResponsesProvider;

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
