//! Offline product assembly checks over the real keychain mutation owner.

use std::sync::Arc;
use std::time::Duration;

use opi_ai::credential::CredentialStore;
use opi_ai::mutation::{BarrierEntry, CredentialMutationOp, MutationApplied, MutationDecision};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::credential_store::{
    CredentialResolver, FakeKeyringBackend, KeychainCredentialStore,
};
use opi_coding_agent::provider_factory::{build_harness_collection, build_runtime_route_for_tests};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// This binary's environment-dependent case is run with --test-threads=1.
struct ScopedSourceEnv(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl ScopedSourceEnv {
    fn new(credentials: &std::path::Path) -> Self {
        let names = [
            "GOOGLE_APPLICATION_CREDENTIALS",
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "NO_PROXY",
            #[cfg(unix)]
            "http_proxy",
            #[cfg(unix)]
            "https_proxy",
            #[cfg(unix)]
            "all_proxy",
            #[cfg(unix)]
            "no_proxy",
        ];
        let original = names
            .iter()
            .map(|name| (*name, std::env::var_os(name)))
            .collect();
        for name in names {
            // SAFETY: this isolated test binary runs its cases serially;
            // no auth owner or HTTP client exists yet.
            unsafe {
                if name == "GOOGLE_APPLICATION_CREDENTIALS" {
                    std::env::set_var(name, credentials);
                } else if name.eq_ignore_ascii_case("NO_PROXY") {
                    std::env::set_var(name, "*");
                } else {
                    std::env::remove_var(name);
                }
            }
        }
        Self(original)
    }
}

impl Drop for ScopedSourceEnv {
    fn drop(&mut self) {
        for (name, value) in self.0.drain(..) {
            // SAFETY: every source request and owner has settled before this
            // test's environment guard is dropped.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
    }
}

fn request(model: &str) -> opi_ai::provider::Request {
    opi_ai::provider::Request {
        projection: Default::default(),
        transport: Default::default(),
        model: model.to_owned(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .unwrap(),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: tokio_util::sync::CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: opi_ai::provider::CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

#[tokio::test]
async fn vertex_factory_source_rotation_remains_in_the_shared_store_barrier() {
    let temp = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let token_url = format!("http://{}/rotate", listener.local_addr().unwrap());
    let credentials = temp.path().join("source.json");
    std::fs::write(
        &credentials,
        serde_json::to_vec(&serde_json::json!({
            "type": "external_account_authorized_user",
            "client_id": "synthetic-client",
            "client_secret": "synthetic-client-secret",
            "refresh_token": "synthetic-refresh",
            "token_url": token_url,
        }))
        .unwrap(),
    )
    .unwrap();
    let _env = ScopedSourceEnv::new(&credentials);
    let (observed, received) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        loop {
            let mut block = [0_u8; 1024];
            let count = socket.read(&mut block).await.unwrap();
            assert_ne!(count, 0, "source request ended before the form body");
            bytes.extend_from_slice(&block[..count]);
            assert!(bytes.len() < 16_384, "unexpected source request size");
            if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                let header = String::from_utf8_lossy(&bytes[..end]);
                let size = header
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap();
                if bytes.len() >= end + 4 + size {
                    break;
                }
            }
        }
        observed.send(String::from_utf8(bytes).unwrap()).unwrap();
        released.await.unwrap();
        let body = r#"{"access_token":"synthetic-access","refresh_token":"synthetic-rotated","expires_in":3600,"token_type":"Bearer"}"#;
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
    });
    let store = Arc::new(KeychainCredentialStore::new(
        Box::new(FakeKeyringBackend::new()),
        temp.path().to_path_buf(),
    ));
    let resolver = CredentialResolver::new(store.clone(), Arc::new(|_| None));
    let mut config = OpiConfig::default();
    config.providers.google_vertex.project = Some("synthetic-project".into());
    config.providers.google_vertex.location = Some("us-central1".into());
    let route = build_runtime_route_for_tests(&config, "google-vertex", &resolver).unwrap();
    let model = format!("google-vertex:{}", route.0.models()[0].id);
    let first = build_harness_collection(
        vec![route],
        vec![],
        vec![],
        "google-vertex",
        Some(store.clone()),
    );
    let second = build_harness_collection(
        vec![build_runtime_route_for_tests(&config, "google-vertex", &resolver).unwrap()],
        vec![],
        vec![],
        "google-vertex",
        Some(store.clone()),
    );
    let call_owner = first.clone();
    let call_model = model.clone();
    let waiter = tokio::spawn(async move {
        call_owner
            .prepare_call(&call_model, request(&call_model))
            .await
    });
    let wire = tokio::time::timeout(Duration::from_secs(10), received)
        .await
        .unwrap()
        .unwrap();
    assert!(wire.starts_with("POST /rotate HTTP/1.1"));
    assert!(wire.to_ascii_lowercase().contains("authorization: basic "));
    assert!(wire.contains("refresh_token=synthetic-refresh"));
    let first_barrier = first.mutation_barrier().unwrap();
    let second_barrier = second.mutation_barrier().unwrap();
    assert_eq!(first_barrier.cutoff(), second_barrier.cutoff());
    assert_eq!(first_barrier.cutoff(), store.mutation_barrier().cutoff());
    assert!(matches!(
        first_barrier.report().captured.as_slice(),
        [BarrierEntry::Pending { .. }]
    ));
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    assert!(!second_barrier.report().is_complete());
    release.send(()).unwrap();
    let report = tokio::time::timeout(Duration::from_secs(10), second_barrier.wait())
        .await
        .unwrap();
    assert!(report.is_complete());
    assert!(
        matches!(report.captured.as_slice(), [BarrierEntry::Settled(receipt)] if matches!(receipt.outcome, opi_ai::mutation::MutationOutcome::Written { .. }))
    );
    server.await.unwrap();
    // The original route consumes its published memory ACK without another
    // source exchange. Preparing never dispatches a generation attempt.
    let prepared = first.prepare_call(&model, request(&model)).await.unwrap();
    assert!(matches!(
        prepared.auth_provenance().source,
        opi_ai::AuthProvenanceSource::Adc {
            source: opi_ai::auth::AdcCredentialSource::ExternalAccountAuthorizedUser { .. }
        }
    ));
}

#[tokio::test]
async fn product_collections_observe_the_existing_store_barrier() {
    let temp = tempfile::tempdir().unwrap();
    let store = Arc::new(KeychainCredentialStore::new(
        Box::new(FakeKeyringBackend::new()),
        temp.path().to_path_buf(),
    ));
    let resolver = CredentialResolver::new(store.clone(), Arc::new(|_| None));
    let config = OpiConfig::default();
    let first = build_harness_collection(
        vec![build_runtime_route_for_tests(&config, "openai", &resolver).unwrap()],
        vec![],
        vec![],
        "openai",
        Some(store.clone()),
    );
    let second = build_harness_collection(
        vec![build_runtime_route_for_tests(&config, "openai", &resolver).unwrap()],
        vec![],
        vec![],
        "openai",
        Some(store.clone()),
    );
    let (release, released) = tokio::sync::oneshot::channel();
    let ack = store.modify(
        "openai",
        CredentialMutationOp::Update(Box::new(move |_, _| {
            Box::pin(async move {
                released.await.unwrap();
                Ok(MutationDecision::Keep)
            })
        })),
    );
    let first_barrier = first
        .mutation_barrier()
        .expect("the product collection must attach its existing credential store");
    let second_barrier = second
        .mutation_barrier()
        .expect("a second collection must observe the same store");
    assert_eq!(first_barrier.cutoff(), second_barrier.cutoff());
    assert_eq!(first_barrier.cutoff(), store.mutation_barrier().cutoff());
    assert!(matches!(
        first_barrier.report().captured.as_slice(),
        [BarrierEntry::Pending { provider_id, .. }] if provider_id == "openai"
    ));
    release.send(()).unwrap();
    ack.await.unwrap();
    assert!(first_barrier.wait().await.is_complete());
    assert!(second_barrier.wait().await.is_complete());
}

#[tokio::test]
async fn keychain_source_queue_and_credential_mutations_share_one_cutoff() {
    let temp = tempfile::tempdir().unwrap();
    let store = KeychainCredentialStore::new(
        Box::new(FakeKeyringBackend::new()),
        temp.path().to_path_buf(),
    );
    let queue = store
        .mutation_queue()
        .expect("rotating source owners must be able to join the keychain store queue");
    let (release, released) = tokio::sync::oneshot::channel();
    let source_ack = queue.submit(
        "adc:synthetic-source",
        Box::new(move || {
            Box::pin(async move {
                released.await.unwrap();
                Ok(MutationApplied::Keep)
            })
        }),
    );
    let barrier = store.mutation_barrier();
    let credential_ack = store.delete("adc:synthetic-source");
    let report = barrier.report();
    assert!(matches!(
        report.captured.as_slice(),
        [BarrierEntry::Pending { provider_id, .. }] if provider_id == "adc:synthetic-source"
    ));
    assert!(matches!(
        report.late.as_slice(),
        [BarrierEntry::Pending { provider_id, .. }] if provider_id == "adc:synthetic-source"
    ));
    release.send(()).unwrap();
    source_ack.await.unwrap();
    credential_ack.await.unwrap();
    assert!(barrier.wait().await.is_complete());
    assert_eq!(barrier.report().late.len(), 1);
}
