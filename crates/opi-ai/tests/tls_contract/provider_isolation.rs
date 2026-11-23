//! Process-global rustls provider isolation for public TLS clients.

use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use opi_ai::http::{HttpClient, HttpClientBuilder, TlsOptions};
use rustls::crypto::CryptoProvider;
use tokio::{io::AsyncReadExt, net::TcpListener, time::timeout};

use super::fixtures::{KxProfile, Trust, assert_round_trip, chat_models_call, start_server};
use super::tls_fixtures::{
    CERT_CA, CERT_CLIENT_RSA2048, CERT_SERVER_RSA, KEY_CLIENT_RSA2048, KEY_SERVER_RSA,
};

#[tokio::test]
async fn instance_tls_never_installs_a_process_global_provider() {
    let before = CryptoProvider::get_default();
    let _default_client = HttpClient::new();
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
    outcome.expect("round trip");
    match (before, CryptoProvider::get_default()) {
        (None, None) => {}
        (Some(a), Some(b)) => assert!(Arc::ptr_eq(a, b), "global provider changed"),
        _ => panic!("global provider appeared or disappeared"),
    }
}

fn ring_control_inner() {
    // Host control: a preinstalled process-global ring provider must not
    // change the instance TLS path.
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("fresh control process");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("control runtime");
    runtime.block_on(async {
        assert!(CryptoProvider::get_default().is_some());
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
            Some((CERT_CLIENT_RSA2048, KEY_CLIENT_RSA2048)),
            false,
            "h2",
        )
        .await;
        let stats = server.stats.clone();
        server.finish().await.expect("server finish");
        outcome.expect("instance TLS under a host ring provider");
        assert_eq!(
            stats.http.load(Ordering::SeqCst),
            1,
            "control round trip must carry business HTTP"
        );
        assert!(CryptoProvider::get_default().is_some());
    });
}

#[test]
fn host_preinstalled_ring_provider_control() {
    if std::env::var("OPI_TLS_RING_CONTROL").as_deref() == Ok("run") {
        ring_control_inner();
        return;
    }
    // Parent: rerun this exact test in a fresh child process with the
    // control marker set. Libtest omits the crate name from its test paths.
    let status = std::process::Command::new(std::env::current_exe().expect("test binary"))
        .args([
            "--exact",
            concat!(module_path!(), "::host_preinstalled_ring_provider_control")
                .split_once("::")
                .expect("crate-qualified test name")
                .1,
            "--nocapture",
        ])
        .env("OPI_TLS_RING_CONTROL", "run")
        .status()
        .expect("control subprocess");
    assert!(status.success(), "host ring control subprocess failed");
}

#[test]
fn default_tls_ignores_host_provider_profile() {
    if std::env::var("OPI_TLS_DEFAULT_CONTROL").as_deref() != Ok("run") {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                concat!(
                    module_path!(),
                    "::default_tls_ignores_host_provider_profile"
                )
                .split_once("::")
                .expect("crate-qualified test name")
                .1,
                "--nocapture",
            ])
            .env("OPI_TLS_DEFAULT_CONTROL", "run")
            .status()
            .unwrap();
        assert!(status.success(), "default TLS profile control failed");
        return;
    }
    let mut host = rustls::crypto::aws_lc_rs::default_provider();
    host.kx_groups = vec![rustls::crypto::aws_lc_rs::kx_group::X25519];
    host.install_default().unwrap();
    let before = CryptoProvider::get_default().unwrap().clone();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            for client in [
                HttpClient::new(),
                HttpClientBuilder::new()
                    .tls(TlsOptions::new())
                    .build()
                    .unwrap(),
            ] {
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let url = format!(
                    "https://localhost:{}",
                    listener.local_addr().unwrap().port()
                );
                let peer = tokio::spawn(async move {
                    let (mut tcp, _) = listener.accept().await.unwrap();
                    let mut acceptor = rustls::server::Acceptor::default();
                    let mut bytes = [0; 8192];
                    loop {
                        let n = tcp.read(&mut bytes).await.unwrap();
                        assert_ne!(n, 0, "ClientHello expected");
                        acceptor
                            .read_tls(&mut std::io::Cursor::new(&bytes[..n]))
                            .unwrap();
                        if let Some(accepted) = acceptor.accept().unwrap() {
                            return accepted.client_hello().named_groups().unwrap().to_vec();
                        }
                    }
                });
                assert!(
                    timeout(
                        Duration::from_secs(5),
                        chat_models_call(&url, Arc::new(client))
                    )
                    .await
                    .unwrap()
                    .is_err()
                );
                assert_eq!(
                    peer.await.unwrap(),
                    vec![
                        rustls::NamedGroup::X25519MLKEM768,
                        rustls::NamedGroup::X25519,
                        rustls::NamedGroup::secp256r1,
                        rustls::NamedGroup::secp384r1,
                    ]
                );
            }
        });
    assert!(Arc::ptr_eq(&before, CryptoProvider::get_default().unwrap()));
}
