use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};

use opi_ai::auth::adc::native::{
    AdcAuthSelection, AdcNativeOwner, AdcSourceHttpError, AdcSourceRequest, AdcSourceResponse,
    AdcSourceTransport, FixedAdcClock,
};
use opi_ai::credential::BoxAuthFuture;

struct Transport {
    requests: Mutex<Vec<AdcSourceRequest>>,
    replies: Mutex<VecDeque<&'static str>>,
    entered: tokio::sync::Semaphore,
    release: tokio::sync::Semaphore,
    gated: bool,
}

impl Transport {
    fn new(replies: impl IntoIterator<Item = &'static str>, gated: bool) -> Arc<Self> {
        Arc::new(Self {
            requests: Mutex::new(Vec::new()),
            replies: Mutex::new(replies.into_iter().collect()),
            entered: tokio::sync::Semaphore::new(0),
            release: tokio::sync::Semaphore::new(0),
            gated,
        })
    }

    fn body(&self, index: usize) -> String {
        self.requests.lock().unwrap()[index].body.clone().unwrap()
    }
}

impl AdcSourceTransport for Transport {
    fn send(
        &self,
        request: AdcSourceRequest,
    ) -> BoxAuthFuture<'_, Result<AdcSourceResponse, AdcSourceHttpError>> {
        let first = {
            let mut requests = self.requests.lock().unwrap();
            requests.push(request);
            requests.len() == 1
        };
        let reply = self
            .replies
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected source exchange");
        Box::pin(async move {
            self.entered.add_permits(1);
            if first && self.gated {
                self.release.acquire().await.unwrap().forget();
            }
            Ok(AdcSourceResponse {
                status: 200,
                headers: vec![],
                body: reply.as_bytes().to_vec(),
            })
        })
    }
}

fn write_source(path: &Path, client: &str, refresh: &str) {
    std::fs::write(
        path,
        serde_json::to_vec(&serde_json::json!({
            "type": "external_account_authorized_user", "client_id": client,
            "client_secret": format!("{client}-secret"), "refresh_token": refresh,
        }))
        .unwrap(),
    )
    .unwrap();
}

fn owner(path: &Path, transport: Arc<Transport>) -> Arc<AdcNativeOwner> {
    Arc::new(AdcNativeOwner::new(
        AdcAuthSelection::CredentialsFile(path.to_owned()),
        transport,
        Arc::new(FixedAdcClock::new(
            time::OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
        )),
    ))
}

const ROTATED: &str =
    r#"{"access_token":"access-A","expires_in":3600,"refresh_token":"rotated-A"}"#;
const FRESH: &str = r#"{"access_token":"access-B","expires_in":3600}"#;

#[tokio::test]
async fn default_url_supplier_get_follows_redirects_and_strips_cross_origin_auth() {
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::path};
    let source = MockServer::start().await;
    let target = MockServer::start().await;
    Mock::given(path("/subject"))
        .respond_with(ResponseTemplate::new(200).set_body_string("subject"))
        .mount(&target)
        .await;
    Mock::given(path("/redirect"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("location", format!("{}/subject", target.uri())),
        )
        .mount(&source)
        .await;
    let reply = opi_ai::auth::adc::native::ReqwestAdcTransport::new()
        .send(AdcSourceRequest {
            method: "GET".into(),
            url: format!("{}/redirect", source.uri()),
            headers: vec![("authorization".into(), "Bearer synthetic-source".into())],
            body: None,
            timeout: Some(std::time::Duration::from_secs(2)),
        })
        .await
        .unwrap();
    assert_eq!(reply.body, b"subject");
    assert!(
        !target.received_requests().await.unwrap()[0]
            .headers
            .contains_key("authorization")
    );
}

#[tokio::test]
async fn source_replacement_never_combines_old_refresh_with_new_client() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("adc.json");
    write_source(&path, "client-A", "refresh-A");
    let transport = Transport::new([ROTATED, FRESH], false);
    let owner = owner(&path, transport.clone());
    owner.resolve_token().await.unwrap();
    write_source(&path, "client-B", "refresh-B");
    owner.invalidate();
    owner.resolve_token().await.unwrap();
    assert!(transport.body(1).contains("refresh_token=refresh-B"));
    assert!(!transport.body(1).contains("rotated-A"));
}

#[tokio::test]
async fn same_source_invalidation_preserves_its_unique_rotated_refresh() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("adc.json");
    write_source(&path, "client-A", "refresh-A");
    let transport = Transport::new([ROTATED, FRESH], false);
    let owner = owner(&path, transport.clone());
    owner.resolve_token().await.unwrap();
    owner.invalidate();
    owner.resolve_token().await.unwrap();
    assert!(transport.body(1).contains("refresh_token=rotated-A"));
}

#[tokio::test]
async fn same_source_invalidation_during_rotation_retains_the_unique_refresh() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("adc.json");
    write_source(&path, "client-A", "refresh-A");
    let transport = Transport::new([ROTATED, FRESH], true);
    let owner = owner(&path, transport.clone());
    let first = {
        let owner = owner.clone();
        tokio::spawn(async move { owner.resolve_token().await })
    };
    transport.entered.acquire().await.unwrap().forget();
    owner.invalidate();
    transport.release.add_permits(1);
    assert!(first.await.unwrap().is_err());
    owner.resolve_token().await.unwrap();
    assert!(transport.body(1).contains("refresh_token=rotated-A"));
}

#[tokio::test]
async fn late_rotation_cannot_rebind_a_replaced_source() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("adc.json");
    write_source(&path, "client-A", "refresh-A");
    let transport = Transport::new([ROTATED, FRESH], true);
    let owner = owner(&path, transport.clone());
    let pending = {
        let owner = owner.clone();
        tokio::spawn(async move { owner.resolve_token().await })
    };
    transport.entered.acquire().await.unwrap().forget();
    write_source(&path, "client-B", "refresh-B");
    owner.invalidate();
    transport.release.add_permits(1);
    assert!(matches!(
        pending.await.unwrap(),
        Err(opi_ai::auth::adc::native::AdcResolutionError::Superseded)
    ));
    owner.resolve_token().await.unwrap();
    assert!(transport.body(1).contains("refresh_token=refresh-B"));
}

#[tokio::test]
async fn concurrent_short_ttl_waiters_share_the_completed_acquisition() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("adc.json");
    write_source(&path, "client-A", "refresh-A");
    let transport = Transport::new(
        [r#"{"access_token":"short","expires_in":200,"refresh_token":"rotated-A"}"#],
        true,
    );
    let owner = owner(&path, transport.clone());
    let first = {
        let owner = owner.clone();
        tokio::spawn(async move { owner.resolve_token().await })
    };
    transport.entered.acquire().await.unwrap().forget();
    let mut second = {
        let owner = owner.clone();
        Box::pin(async move { owner.resolve_token().await })
    };
    assert!(futures_util::poll!(&mut second).is_pending());
    transport.release.add_permits(1);
    first.await.unwrap().unwrap();
    second.await.unwrap();
    assert_eq!(transport.requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn rotating_adc_survives_waiter_drop_and_shares_the_store_barrier() {
    use opi_ai::credential::CredentialStore;
    use opi_ai::mutation::{BarrierEntry, InMemoryCredentialStore, MutationOutcome};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("adc.json");
    write_source(&path, "client-A", "refresh-A");
    let transport = Transport::new([ROTATED, FRESH], true);
    let store = InMemoryCredentialStore::new();
    let owner = Arc::new(
        AdcNativeOwner::new(
            AdcAuthSelection::CredentialsFile(path),
            transport.clone(),
            Arc::new(FixedAdcClock::new(
                time::OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
            )),
        )
        .with_mutation_queue(store.mutation_queue().unwrap()),
    );
    let first = {
        let owner = owner.clone();
        tokio::spawn(async move { owner.resolve_token().await })
    };
    transport.entered.acquire().await.unwrap().forget();
    let barrier = store.mutation_barrier();
    assert!(matches!(
        barrier.report().captured.as_slice(),
        [BarrierEntry::Pending { .. }]
    ));
    first.abort();
    transport.release.add_permits(1);
    let report = barrier.wait().await;
    assert!(
        matches!(report.captured.as_slice(),[BarrierEntry::Settled(receipt)] if matches!(receipt.outcome,MutationOutcome::Written{..}))
    );
    owner.invalidate();
    owner.resolve_token().await.unwrap();
    assert!(transport.body(1).contains("refresh_token=rotated-A"));
}

#[tokio::test]
async fn ordinary_adc_acquisition_can_be_cancelled_with_its_waiter() {
    use opi_ai::mutation::{BarrierEntry, MutationOutcome};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("adc.json");
    let subject = dir.path().join("subject.txt");
    std::fs::write(&subject, "synthetic-subject").unwrap();
    std::fs::write(&path,serde_json::to_vec(&serde_json::json!({"type":"external_account","audience":"synthetic-audience","subject_token_type":"urn:ietf:params:oauth:token-type:jwt","token_url":"https://sts.googleapis.com/v1/token","credential_source":{"file":subject}})).unwrap()).unwrap();
    let transport = Transport::new([FRESH], true);
    let owner = owner(&path, transport.clone());
    let first = {
        let owner = owner.clone();
        tokio::spawn(async move { owner.resolve_token().await })
    };
    transport.entered.acquire().await.unwrap().forget();
    let barrier = owner.mutation_barrier();
    first.abort();
    let report = tokio::time::timeout(std::time::Duration::from_secs(2), barrier.wait())
        .await
        .unwrap();
    assert!(
        matches!(report.captured.as_slice(),[BarrierEntry::Settled(receipt)] if matches!(receipt.outcome,MutationOutcome::Failed{..}))
    );
}

#[tokio::test]
async fn a_failed_refresh_cannot_return_access_that_expired_during_the_exchange() {
    struct MovingClockTransport {
        clock: Arc<FixedAdcClock>,
        calls: std::sync::atomic::AtomicUsize,
    }
    impl AdcSourceTransport for MovingClockTransport {
        fn send(
            &self,
            _: AdcSourceRequest,
        ) -> BoxAuthFuture<'_, Result<AdcSourceResponse, AdcSourceHttpError>> {
            let first = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0;
            if !first {
                self.clock
                    .set(time::OffsetDateTime::from_unix_timestamp(1_700_000_201).unwrap());
            }
            Box::pin(async move {
                Ok(AdcSourceResponse {
                    status: if first { 200 } else { 400 },
                    headers: vec![],
                    body: if first {
                        br#"{"access_token":"short-lived","expires_in":200}"#.to_vec()
                    } else {
                        br#"{"error":"invalid_grant"}"#.to_vec()
                    },
                })
            })
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("adc.json");
    write_source(&path, "client-A", "refresh-A");
    let clock = Arc::new(FixedAdcClock::new(
        time::OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
    ));
    let transport = Arc::new(MovingClockTransport {
        clock: clock.clone(),
        calls: std::sync::atomic::AtomicUsize::new(0),
    });
    let owner = AdcNativeOwner::new(AdcAuthSelection::CredentialsFile(path), transport, clock);
    owner.resolve_token().await.unwrap();
    assert!(
        owner.resolve_token().await.is_err(),
        "an expired fallback cannot authorize the waiting call"
    );
}

struct ConfigurationContext {
    root: std::path::PathBuf,
    explicit: Option<String>,
    present: bool,
    probe_failure: bool,
    probes: std::sync::atomic::AtomicUsize,
}

impl opi_ai::auth::AuthContext for ConfigurationContext {
    fn env(&self, name: &str) -> Result<Option<String>, opi_ai::auth::AuthContextError> {
        Ok(match name {
            "GOOGLE_APPLICATION_CREDENTIALS" => self.explicit.clone(),
            "HOME" | "APPDATA" => Some(self.root.to_string_lossy().into_owned()),
            _ => None,
        })
    }
    fn file_exists(&self, path: &str) -> Result<bool, opi_ai::auth::AuthContextError> {
        assert!(std::path::Path::new(path).is_absolute());
        self.probes
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.probe_failure {
            Err(opi_ai::auth::AuthContextError::FileProbe {
                path: path.to_owned(),
                reason: "controlled existence failure".into(),
            })
        } else {
            Ok(self.present)
        }
    }
}

fn configuration_models(
    context: Arc<ConfigurationContext>,
    transport: Arc<Transport>,
) -> opi_ai::Models {
    let owner = Arc::new(
        AdcNativeOwner::new(
            AdcAuthSelection::Auto,
            transport,
            Arc::new(FixedAdcClock::new(
                time::OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
            )),
        )
        .with_ambient(context.clone()),
    );
    let mut models = opi_ai::Models::new();
    models.set_auth_context(context);
    models
        .register_route(
            Box::new(opi_ai::vertex::VertexProvider::new(
                "p1".into(),
                "global".into(),
                Some("http://127.0.0.1:1".into()),
            )),
            owner,
            opi_ai::auth::AuthProvenanceSource::Adc {
                source: opi_ai::auth::AdcCredentialSource::ServiceAccount,
            },
            Default::default(),
        )
        .unwrap();
    models
}

#[tokio::test]
async fn adc_configuration_check_matches_the_same_unconfigured_owner() {
    let dir = tempfile::tempdir().unwrap();
    let context = Arc::new(ConfigurationContext {
        root: dir.path().to_owned(),
        explicit: None,
        present: false,
        probe_failure: false,
        probes: Default::default(),
    });
    let transport = Transport::new([], false);
    let models = configuration_models(context.clone(), transport.clone());
    assert!(
        models
            .get_auth("vertex", Default::default())
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        models.check_auth("vertex").await.unwrap().is_none(),
        "the same source has no configured file"
    );
    assert!(transport.requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn adc_configuration_check_only_probes_a_present_invalid_json_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("invalid.json");
    std::fs::write(
        &path,
        "invalid JSON must not be parsed by a configuration check",
    )
    .unwrap();
    let context = Arc::new(ConfigurationContext {
        root: dir.path().to_owned(),
        explicit: Some(path.to_string_lossy().into_owned()),
        present: true,
        probe_failure: false,
        probes: Default::default(),
    });
    let transport = Transport::new([], false);
    let models = configuration_models(context.clone(), transport.clone());
    assert!(models.check_auth("vertex").await.unwrap().is_some());
    assert_eq!(
        context.probes.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "configuration presence must be checked through the selected context"
    );
    assert!(
        models.get_auth("vertex", Default::default()).await.is_err(),
        "actual resolution parses the selected file"
    );
    assert!(transport.requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn adc_configuration_check_propagates_the_selected_file_probe_failure() {
    let dir = tempfile::tempdir().unwrap();
    let context = Arc::new(ConfigurationContext {
        root: dir.path().to_owned(),
        explicit: None,
        present: false,
        probe_failure: true,
        probes: Default::default(),
    });
    let transport = Transport::new([], false);
    let models = configuration_models(context.clone(), transport.clone());
    assert!(
        models.check_auth("vertex").await.is_err(),
        "a selected configuration probe failure is not configured or absent"
    );
    assert_eq!(context.probes.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(transport.requests.lock().unwrap().is_empty());
}
