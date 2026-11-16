use std::time::Duration;

use opi_ai::http::{HttpClient, HttpClientBuilder, ProxyConfig, RedirectPolicy};

#[test]
fn http_transport_profile_keeps_the_frozen_defaults() {
    let client = HttpClient::new();

    assert_eq!(client.pool_config(), (10, Duration::from_secs(90)));
    assert_eq!(client.redirect_policy(), RedirectPolicy::Limited(20));
    assert_eq!(client.proxy_config().url, None);
    assert_eq!(client.proxy_config().no_proxy, None);
}

#[test]
fn http_transport_profile_normalizes_proxy_and_redacts_credentials() {
    let client = HttpClientBuilder::new()
        .proxy(ProxyConfig {
            url: Some("  ".to_owned()),
            no_proxy: Some("".to_owned()),
        })
        .redirects(RedirectPolicy::None)
        .build()
        .expect("valid source-scoped HTTP client");

    assert_eq!(client.proxy_config().url, None);
    assert_eq!(client.proxy_config().no_proxy, None);
    assert_eq!(client.redirect_policy(), RedirectPolicy::None);
    assert_eq!(
        opi_ai::http::redact_proxy_credentials("https://user:secret@example.test:8443"),
        "https://***:***@example.test:8443"
    );
}

#[test]
fn http_transport_profile_resolves_explicit_proxy_precedence() {
    let proxy = opi_ai::http::resolve_proxy(
        Some("http://http-proxy.test:8080"),
        Some("https://https-proxy.test:8443"),
        Some("localhost,*.internal"),
    );

    assert_eq!(proxy.url.as_deref(), Some("https://https-proxy.test:8443"));
    assert_eq!(proxy.no_proxy.as_deref(), Some("localhost,*.internal"));
}
