//! Client identity admission, verification and connection scope.

use std::sync::{Arc, atomic::Ordering};

use opi_ai::http::{HttpClientBuildError, TlsConfigError, TlsKeyError, TlsOptions};
use rustls::pki_types::{CertificateDer, pem::PemObject};

use super::fixtures::{
    KxProfile, Trust, assert_negative_zero_http, assert_round_trip, build_tls_client,
    chat_models_call, fingerprint, start_server,
};
use super::tls_fixtures::{
    CERT_CA, CERT_CLIENT_EXPIRED, CERT_CLIENT_P521, CERT_CLIENT_RSA_E3, CERT_CLIENT_RSA2048,
    CERT_CLIENT_RSA8192, CERT_CLIENT_UNTRUSTED, CERT_CLIENT_WRONG_EKU, CERT_SERVER_P521,
    CERT_SERVER_RSA, KEY_CLIENT_EXPIRED, KEY_CLIENT_P521, KEY_CLIENT_RSA_E3, KEY_CLIENT_RSA2048,
    KEY_CLIENT_RSA8192, KEY_CLIENT_UNTRUSTED, KEY_CLIENT_WRONG_EKU, KEY_PEM_ENCRYPTED,
    KEY_PEM_MULTIPRIME, KEY_PEM_RSA1024, KEY_SERVER_P521, KEY_SERVER_RSA,
};

#[tokio::test]
async fn admitted_client_identity_profiles_over_mtls() {
    for (label, cert, key) in [
        ("rsa2048", CERT_CLIENT_RSA2048, KEY_CLIENT_RSA2048),
        ("rsa-e3", CERT_CLIENT_RSA_E3, KEY_CLIENT_RSA_E3),
        ("rsa8192", CERT_CLIENT_RSA8192, KEY_CLIENT_RSA8192),
        ("p521", CERT_CLIENT_P521, KEY_CLIENT_P521),
    ] {
        let server = start_server(
            CERT_SERVER_RSA,
            KEY_SERVER_RSA,
            true,
            true,
            true,
            KxProfile::Declared,
        )
        .await
        .expect("server start");
        let outcome = assert_round_trip(
            &server,
            Trust::Explicit(CERT_CA),
            Some((cert, key)),
            false,
            "h2",
        )
        .await;
        let stats = server.stats.clone();
        server.finish().await.expect("server finish");
        outcome.unwrap_or_else(|e| panic!("{label} identity: {e}"));
        let peers = stats.peers.lock().unwrap().clone();
        let expected = fingerprint(
            CertificateDer::from_pem_slice(cert.as_bytes())
                .expect("identity cert")
                .as_ref(),
        );
        assert_eq!(
            peers[0]["fingerprint"].as_str(),
            Some(expected.as_str()),
            "{label} peer certificate fingerprint mismatch"
        );
    }
}

#[tokio::test]
async fn p521_server_identity_and_client_identity() {
    let server = start_server(
        CERT_SERVER_P521,
        KEY_SERVER_P521,
        true,
        true,
        true,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let outcome = assert_round_trip(
        &server,
        Trust::Explicit(CERT_CA),
        Some((CERT_CLIENT_RSA2048, KEY_CLIENT_RSA2048)),
        false,
        "h2",
    )
    .await;
    server.finish().await.expect("server finish");
    outcome.expect("round trip");
}

#[tokio::test]
async fn connection_source_identity_isolation() {
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        true,
        true,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let identities = [
        (CERT_CLIENT_RSA2048, KEY_CLIENT_RSA2048),
        (CERT_CLIENT_P521, KEY_CLIENT_P521),
    ];
    for (cert, key) in identities {
        let client = Arc::new(
            build_tls_client(Trust::Explicit(CERT_CA), Some((cert, key)), false)
                .expect("client build"),
        );
        chat_models_call(&server.url, client.clone())
            .await
            .expect("identity round trip");
        drop(client);
        server.wait_idle().await.expect("idle");
    }
    let stats = server.stats.clone();
    server.finish().await.expect("server finish");
    let peers = stats.peers.lock().unwrap().clone();
    let first = fingerprint(
        CertificateDer::from_pem_slice(CERT_CLIENT_RSA2048.as_bytes())
            .expect("cert")
            .as_ref(),
    );
    let second = fingerprint(
        CertificateDer::from_pem_slice(CERT_CLIENT_P521.as_bytes())
            .expect("cert")
            .as_ref(),
    );
    assert_eq!(stats.http.load(Ordering::SeqCst), 2);
    assert_eq!(stats.accepts.load(Ordering::SeqCst), 2);
    assert_eq!(peers.len(), 2);
    assert_eq!(peers[0]["fingerprint"].as_str(), Some(first.as_str()));
    assert_eq!(peers[1]["fingerprint"].as_str(), Some(second.as_str()));
    assert_ne!(
        peers[0]["fingerprint"].as_str(),
        peers[1]["fingerprint"].as_str(),
        "source identities must not mix between connection owners"
    );
}

#[tokio::test]
async fn wrong_client_identities_fail_zero_http() {
    for (label, identity) in [
        ("missing", None),
        (
            "untrusted",
            Some((CERT_CLIENT_UNTRUSTED, KEY_CLIENT_UNTRUSTED)),
        ),
        ("expired", Some((CERT_CLIENT_EXPIRED, KEY_CLIENT_EXPIRED))),
        (
            "wrong-eku",
            Some((CERT_CLIENT_WRONG_EKU, KEY_CLIENT_WRONG_EKU)),
        ),
    ] {
        let server = start_server(
            CERT_SERVER_RSA,
            KEY_SERVER_RSA,
            true,
            true,
            true,
            KxProfile::Declared,
        )
        .await
        .expect("server start");
        let outcome =
            assert_negative_zero_http(&server, Trust::Explicit(CERT_CA), identity, false).await;
        server.finish().await.expect("server finish");
        outcome.unwrap_or_else(|e| panic!("{label} client identity: {e}"));
    }
}

#[test]
fn unsupported_client_keys_fail_typed_at_admission() {
    let chain = CERT_CLIENT_RSA2048;
    let cases = [
        ("rsa-1024", KEY_PEM_RSA1024, TlsKeyError::BackendLoad),
        ("multi-prime", KEY_PEM_MULTIPRIME, TlsKeyError::BackendLoad),
        ("encrypted", KEY_PEM_ENCRYPTED, TlsKeyError::Decode),
    ];
    for (label, key, expected) in cases {
        let error = TlsOptions::new()
            .with_client_identity_pem(chain.as_bytes(), key.as_bytes())
            .expect_err("unsupported key must fail admission");
        assert_eq!(error, TlsConfigError::KeyAdmission(expected), "{label}");
        let rendered = format!("{error:?}");
        assert!(
            !rendered.contains("PRIVATE") && !rendered.contains("BEGIN"),
            "admission failure must stay redacted: {rendered}"
        );
    }
}

#[tokio::test]
async fn mismatched_identity_fails_before_any_socket() {
    // RSA-2048 certificate paired with a P-521 key: the identity must be
    // rejected at build, before any connection exists.
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        true,
        true,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let error = build_tls_client(
        Trust::Explicit(CERT_CA),
        Some((CERT_CLIENT_RSA2048, KEY_CLIENT_P521)),
        false,
    )
    .expect_err("mismatched identity must fail at build");
    assert!(
        matches!(
            error,
            HttpClientBuildError::Tls(TlsConfigError::IdentityAssembly)
        ),
        "unexpected error: {error:?}"
    );
    assert_eq!(
        server.stats.accepts.load(Ordering::SeqCst),
        0,
        "no socket may be opened for a rejected identity"
    );
    server.finish().await.expect("server finish");
}
