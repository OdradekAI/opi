use opi_ai::http::{HttpClient, HttpClientBuilder, RedirectPolicy};
use opi_ai::ws::{self, PmdOffer, WsUpgradeRequest};
use url::Url;

#[test]
fn websocket_compression_profile_exposes_public_upgrade_owner() {
    let nonce = ws::fresh_nonce();
    assert_eq!(nonce.len(), 16);
    assert!(!ws::accept_token(&nonce).is_empty());

    let request = WsUpgradeRequest::new(Url::parse("ws://127.0.0.1:1/socket").unwrap())
        .subprotocols(["opi-test".to_owned()])
        .permessage_deflate(PmdOffer {
            client_no_context_takeover: true,
            client_max_window_bits: Some(Some(12)),
        });
    let _ = request;
}

#[test]
fn websocket_compression_profile_reuses_source_scoped_http_owner() {
    let client = HttpClient::new();
    assert_eq!(client.redirect_policy(), RedirectPolicy::Limited(20));
    let client = HttpClientBuilder::new()
        .redirects(RedirectPolicy::None)
        .build()
        .expect("WebSocket owner must build its HTTP bridge");
    assert_eq!(client.redirect_policy(), RedirectPolicy::None);
}
