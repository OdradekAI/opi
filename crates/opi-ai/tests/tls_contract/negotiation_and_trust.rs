//! TLS versions, ALPN and server trust over the public HTTP client.

use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use opi_ai::http::{HttpClient, TlsConfigError, TlsOptions};
use opi_ai::provider::ProviderError;
use tokio::time::timeout;

use super::fixtures::{
    KxProfile, Trust, accepted_failure_cause, assert_negative_zero_http, assert_round_trip,
    build_tls_client, chat_models_error, chat_request, start_server,
};
use super::tls_fixtures::{
    CERT_CA, CERT_CLIENT_RSA2048, CERT_OTHER_CA, CERT_SERVER_EXPIRED, CERT_SERVER_RSA,
    CERT_SERVER_WRONGHOST, KEY_PEM_RSA1024, KEY_SERVER_EXPIRED, KEY_SERVER_RSA,
    KEY_SERVER_WRONGHOST,
};

#[tokio::test]
async fn chat_tls13_h2_negotiation_with_explicit_roots() {
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        true,
        false,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let outcome = assert_round_trip(&server, Trust::Explicit(CERT_CA), None, false, "h2").await;
    server.finish().await.expect("server finish");
    let message = outcome.expect("round trip");
    assert_eq!(
        message.response_id.as_deref(),
        Some("chatcmpl-tls"),
        "chat terminal must carry the loopback response id"
    );
}

#[tokio::test]
async fn chat_tls13_h1_explicit_selection_with_extra_roots() {
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        false,
        false,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let outcome = assert_round_trip(
        &server,
        Trust::PinnedDefaultWithExtra(CERT_CA),
        None,
        true,
        "http/1.1",
    )
    .await;
    server.finish().await.expect("server finish");
    outcome.expect("round trip");
}

#[tokio::test]
async fn chat_tls12_h2_negotiation() {
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        false,
        true,
        false,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let outcome = assert_round_trip(&server, Trust::Explicit(CERT_CA), None, false, "h2").await;
    let stats = server.stats.clone();
    server.finish().await.expect("server finish");
    let message = outcome.expect("round trip");
    let peers = stats.peers.lock().unwrap().clone();
    assert_eq!(
        peers[0]["version"].as_str(),
        Some("Some(TLSv1_2)"),
        "server must observe TLS 1.2"
    );
    assert!(message.response_id.is_some());
}

#[tokio::test]
async fn untrusted_server_root_fails_zero_http() {
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        true,
        false,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let outcome =
        assert_negative_zero_http(&server, Trust::Explicit(CERT_OTHER_CA), None, false).await;
    let stats = server.stats.clone();
    server.finish().await.expect("server finish");
    outcome.expect("negative");
    assert_eq!(stats.rejected.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn wrong_hostname_and_expired_server_fail_zero_http() {
    for (label, cert, key) in [
        (
            "wrong-hostname",
            CERT_SERVER_WRONGHOST,
            KEY_SERVER_WRONGHOST,
        ),
        ("expired", CERT_SERVER_EXPIRED, KEY_SERVER_EXPIRED),
    ] {
        let server = start_server(cert, key, true, true, false, KxProfile::Declared)
            .await
            .expect("server start");
        let outcome =
            assert_negative_zero_http(&server, Trust::Explicit(CERT_CA), None, false).await;
        server.finish().await.expect("server finish");
        outcome.unwrap_or_else(|e| panic!("{label}: {e}"));
    }
}

#[tokio::test]
async fn host_system_roots_do_not_trust_the_loopback_ca() {
    // Opt-in host-root mode is scoped to this client only. The synthetic
    // loopback CA is not installed in the host store, so verification must
    // fail closed with zero business HTTP (no fallback to another trust
    // mode). Positive host-root trust stays with real host CAs and is not
    // claimed here.
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        true,
        false,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let outcome = assert_negative_zero_http(&server, Trust::HostSystem, None, false).await;
    server.finish().await.expect("server finish");
    outcome.expect("negative");
}

#[tokio::test]
async fn alpn_no_overlap_fails_zero_http() {
    // The client is explicitly H1-only; the server offers only h2.
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        true,
        false,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let outcome = assert_negative_zero_http(&server, Trust::Explicit(CERT_CA), None, true).await;
    let stats = server.stats.clone();
    server.finish().await.expect("server finish");
    outcome.expect("negative");
    assert!(
        stats.handshakes.load(Ordering::SeqCst) == 0 || stats.http.load(Ordering::SeqCst) == 0,
        "no negotiated connection may carry business HTTP"
    );
}

#[tokio::test]
async fn p521_only_key_exchange_fails_zero_http() {
    // The server offers only P-521 key exchange, which the frozen client
    // profile does not include: the handshake must fail with zero business
    // HTTP (typed redacted transport failure), never downgrade.
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        true,
        false,
        KxProfile::P521Only,
    )
    .await
    .expect("server start");
    let outcome = assert_negative_zero_http(&server, Trust::Explicit(CERT_CA), None, false).await;
    let stats = server.stats.clone();
    server.finish().await.expect("server finish");
    outcome.expect("negative");
    assert_eq!(
        stats.handshakes.load(Ordering::SeqCst),
        0,
        "no handshake may complete against a no-common-group peer"
    );
}

#[test]
fn root_input_negatives_fail_typed() {
    assert_eq!(
        TlsOptions::new()
            .with_explicit_roots_pem(b"not a pem")
            .unwrap_err(),
        TlsConfigError::RootDecode
    );
    assert_eq!(
        TlsOptions::new().with_explicit_roots_pem(b"").unwrap_err(),
        TlsConfigError::RootDecode
    );
    assert_eq!(
        TlsOptions::new().with_extra_roots_pem(b"").unwrap_err(),
        TlsConfigError::RootDecode
    );
}

#[test]
fn build_error_displays_redacted() {
    let error = build_tls_client(
        Trust::Explicit("not a pem"),
        Some((CERT_CLIENT_RSA2048, KEY_PEM_RSA1024)),
        false,
    )
    .expect_err("invalid inputs");
    let rendered = format!("{error}");
    assert!(
        rendered.contains("TLS configuration rejected"),
        "{rendered}"
    );
    assert!(!rendered.contains("PRIVATE"), "{rendered}");
}

#[tokio::test]
async fn default_roots_reject_unconfigured_loopback_ca_without_business_http() {
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        true,
        false,
        KxProfile::Declared,
    )
    .await
    .unwrap();
    let error = timeout(
        Duration::from_secs(5),
        chat_models_error(&server.url, Arc::new(HttpClient::new())),
    )
    .await
    .unwrap();
    assert!(
        matches!(
            accepted_failure_cause(&error, &chat_request()),
            ProviderError::Network(_)
        ),
        "{error:?}"
    );
    assert_eq!(server.stats.http.load(Ordering::SeqCst), 0);
    assert_eq!(server.stats.accepts.load(Ordering::SeqCst), 1);
    server.finish().await.unwrap();
}
