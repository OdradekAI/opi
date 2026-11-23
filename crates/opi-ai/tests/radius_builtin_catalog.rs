//! Builtin Radius registration, authenticated discovery and store publication.

#[path = "support/catalog_store.rs"]
mod catalog_fixture;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use opi_ai::auth::{AuthContext, AuthContextError};
use opi_ai::catalog::{Catalog, CatalogOperation};
use opi_ai::http::HttpClient;
use opi_ai::provider::Request;
use opi_ai::providers::collection::{BuiltinOverrides, register_builtin};
use opi_ai::transcript::{Context as ShorthandContext, normalize_context};
use opi_ai::{
    ChatModelRecord, Credential, CredentialStore, InMemoryCredentialStore, ModelCapabilities,
    ModelInfo, Models, ModelsStore, RefreshOutcome, RefreshStatus, ThinkingLevel, WireApi,
};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const RADIUS: &str = "radius";
const KEY: &str = "fixture-radius-key";

#[derive(Default)]
struct RadiusAuthContext {
    key: Option<&'static str>,
    reads: AtomicUsize,
}

impl AuthContext for RadiusAuthContext {
    fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok((name == "RADIUS_API_KEY")
            .then_some(self.key)
            .flatten()
            .map(str::to_owned))
    }

    fn file_exists(&self, _: &str) -> Result<bool, AuthContextError> {
        panic!("Radius does not need filesystem auth probes")
    }
}

fn builtin(
    gateway: Option<&str>,
    store: Option<ModelsStore>,
    context: Arc<RadiusAuthContext>,
) -> Models {
    let mut models = Models::new();
    if let Some(store) = store {
        models.set_models_store(store);
    }
    register_at(&mut models, gateway, context);
    models
}

fn register_at(models: &mut Models, gateway: Option<&str>, context: Arc<RadiusAuthContext>) {
    let mut options = BuiltinOverrides::default();
    if let Some(gateway) = gateway {
        options.base_urls.insert(RADIUS.into(), gateway.to_owned());
    }
    let report = register_builtin(models, context, Arc::new(HttpClient::new()), &options)
        .expect("register builtins");
    assert!(report.registered.iter().any(|id| id == RADIUS));
}

fn key_context() -> Arc<RadiusAuthContext> {
    Arc::new(RadiusAuthContext {
        key: Some(KEY),
        ..Default::default()
    })
}

fn remote_model(id: &str) -> Value {
    json!({
        "id": id,
        "name": "Gateway model",
        "reasoning": true,
        "thinkingLevelMap": { "low": "gateway-low", "medium": null },
        "input": ["text", "image"],
        "cost": { "input": 1.25, "output": 4, "cacheRead": 0.25, "cacheWrite": 0 },
        "contextWindow": 65536,
        "maxTokens": 8192,
    })
}

async fn mount_config(server: &MockServer, key: &str, response: ResponseTemplate) {
    Mock::given(method("GET"))
        .and(path("/v1/config"))
        .and(header("accept", "application/json"))
        .and(header("authorization", format!("Bearer {key}")))
        .respond_with(response)
        .expect(1)
        .mount(server)
        .await;
}

fn config(base_url: &str, models: Vec<Value>) -> Value {
    json!({ "baseUrl": base_url, "models": models })
}

async fn refresh(models: &mut Models, allow_network: bool) -> opi_ai::RefreshReport {
    models
        .refresh(catalog_fixture::refresh_options(
            &[RADIUS],
            allow_network,
            None,
        ))
        .await
        .expect("refresh")
}

async fn assert_chat_dispatch(
    server: &MockServer,
    models: &Models,
    model: ModelInfo,
    endpoint: &str,
    key: &str,
) {
    Mock::given(method("POST"))
        .and(path(endpoint))
        .and(header("authorization", format!("Bearer {key}")))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(concat!(
                    "data: {\"type\":\"start\"}\n\n",
                    r#"data: {"type":"done","reason":"stop","usage":{"input":3,"output":2,"cacheRead":0,"cacheWrite":0,"totalTokens":5,"cost":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"total":0}}}"#,
                    "\n\n",
                )),
        )
        .expect(1)
        .mount(server)
        .await;
    let id = model.id.clone();
    let request = Request {
        projection: Default::default(),
        model: format!("{RADIUS}:{id}"),
        context: normalize_context(ShorthandContext::default()).unwrap(),
        max_tokens: Some(32),
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: Default::default(),
        timeout: None,
        extra_headers: vec![],
        transport: Default::default(),
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };
    let output = models
        .complete_simple(
            ChatModelRecord::new(RADIUS, model),
            request,
            Default::default(),
        )
        .await
        .unwrap();
    assert_eq!(output.model, id);
    let sent = server.received_requests().await.unwrap();
    assert_eq!(
        sent.last().unwrap().body_json::<Value>().unwrap()["model"],
        id
    );
}

#[tokio::test]
async fn default_uses_opi_bundle_and_custom_registration_stays_empty_and_offline() {
    let context = key_context();
    let bundle: Vec<_> = Catalog::builtin()
        .unwrap()
        .models(RADIUS, CatalogOperation::Chat)
        .unwrap()
        .map(|model| model.id.as_str())
        .collect();
    assert!(!bundle.is_empty());
    for gateway in [None, Some("https://radius.pi.dev/"), Some("radius.pi.dev")] {
        let mut models = builtin(gateway, None, context.clone());
        let actual = models.get_models(Some(RADIUS));
        let mut ids: Vec<_> = actual.iter().map(|model| model.id.as_str()).collect();
        let mut expected = bundle.clone();
        ids.sort_unstable();
        expected.sort_unstable();
        assert_eq!(ids, expected);
        assert_eq!(
            refresh(&mut models, false).await.status(),
            RefreshStatus::OfflineRestored
        );
    }
    let server = MockServer::start().await;
    let mut models = builtin(Some(&server.uri()), None, context.clone());
    assert!(models.get_models(Some(RADIUS)).is_empty());
    assert!(models.compat(RADIUS).unwrap().openai_compatible);
    assert_eq!(
        refresh(&mut models, false).await.status(),
        RefreshStatus::OfflineRestored
    );
    assert_eq!(context.reads.load(Ordering::SeqCst), 0);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn caller_owned_model_uses_the_configured_chat_base_path() {
    let server = MockServer::start().await;
    let gateway = format!("{}/custom-prefix/", server.uri());
    let models = builtin(Some(&gateway), None, key_context());
    let model = ModelInfo::new(
        "caller-model",
        "Caller model",
        WireApi::PiMessages,
        ModelCapabilities::new(65536, 8192).with_streaming(true),
    );
    assert_chat_dispatch(&server, &models, model, "/custom-prefix/messages", KEY).await;
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn refresh_reopens_catalog_and_dispatches_a_new_model_through_pi_messages() {
    let server = MockServer::start().await;
    let directory = tempfile::tempdir().unwrap();
    let path_to_store = directory.path().join("models.json");
    let store = ModelsStore::open(&path_to_store).unwrap();
    let api_base = format!("{}/v1", server.uri());
    let mut models = builtin(Some(&server.uri()), Some(store), key_context());
    let keys = Arc::new(InMemoryCredentialStore::new());
    keys.write(RADIUS, &Credential::ApiKey("fixture-stored-key".into()))
        .await
        .unwrap();
    models.set_credential_store(keys.clone());
    mount_config(
        &server,
        "fixture-stored-key",
        ResponseTemplate::new(200)
            .set_body_json(config(&api_base, vec![remote_model("new-model")])),
    )
    .await;
    let report = refresh(&mut models, true).await;
    assert_eq!(report.status(), RefreshStatus::Published);
    assert_eq!(
        report.find(RADIUS).unwrap().outcome,
        RefreshOutcome::Published { models: 1 }
    );
    let model = models.get_model(RADIUS, "new-model").unwrap();
    assert_eq!(model.wire_api, WireApi::PiMessages);
    assert_eq!(model.base_url.as_deref(), Some(api_base.as_str()));
    assert_eq!(model.capabilities.context_window, 65536);
    assert_eq!(model.capabilities.max_output_tokens, 8192);
    assert!(model.capabilities.supports_images && model.capabilities.supports_streaming);
    assert_eq!(
        model
            .thinking_level_map
            .resolve(ThinkingLevel::Low)
            .unwrap(),
        Some("gateway-low".into())
    );
    assert_eq!(
        model.pricing.as_ref().unwrap().base.input_cost_per_mtok,
        1.25
    );
    drop(models);

    let store = ModelsStore::open(&path_to_store).unwrap();
    let mut restored = builtin(Some(&api_base), Some(store.clone()), key_context());
    restored.set_credential_store(keys.clone());
    let reads = keys.read_count();
    let restore = refresh(&mut restored, false).await;
    assert_eq!(restore.status(), RefreshStatus::OfflineRestored);
    assert_eq!(
        restore.find(RADIUS).unwrap().outcome,
        RefreshOutcome::Restored { models: 1 }
    );
    assert_eq!(restored.get_model(RADIUS, "new-model"), Some(model.clone()));
    assert_eq!(keys.read_count(), reads);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);

    let other = MockServer::start().await;
    let mut other_gateway = builtin(Some(&other.uri()), Some(store), key_context());
    assert!(
        refresh(&mut other_gateway, false)
            .await
            .find(RADIUS)
            .is_none()
    );
    assert!(other_gateway.get_models(Some(RADIUS)).is_empty());
    assert!(other.received_requests().await.unwrap().is_empty());

    assert_chat_dispatch(
        &server,
        &restored,
        model,
        "/v1/messages",
        "fixture-stored-key",
    )
    .await;
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn explicit_empty_is_persisted_and_never_restored_on_a_different_gateway() {
    let server = MockServer::start().await;
    let directory = tempfile::tempdir().unwrap();
    let path_to_store = directory.path().join("models.json");
    let store = ModelsStore::open(&path_to_store).unwrap();
    let gateway = server.uri();
    let base = format!("{gateway}/v1");
    let mut models = builtin(Some(&gateway), Some(store), key_context());
    mount_config(
        &server,
        KEY,
        ResponseTemplate::new(200).set_body_json(config(&base, vec![remote_model("before")])),
    )
    .await;
    assert!(
        !refresh(&mut models, true)
            .await
            .find(RADIUS)
            .unwrap()
            .outcome
            .is_failure()
    );
    assert!(models.get_model(RADIUS, "before").is_some());
    server.verify().await;
    server.reset().await;

    mount_config(
        &server,
        KEY,
        ResponseTemplate::new(200).set_body_json(config(&base, vec![])),
    )
    .await;
    assert_eq!(
        refresh(&mut models, true)
            .await
            .entries()
            .last()
            .unwrap()
            .outcome,
        RefreshOutcome::Published { models: 0 }
    );
    assert!(models.get_models(Some(RADIUS)).is_empty());
    drop(models);

    let store = ModelsStore::open(&path_to_store).unwrap();
    assert!(store.snapshot().catalogs[RADIUS].models.is_empty());
    let mut reopened = builtin(Some(&gateway), Some(store.clone()), Default::default());
    assert_eq!(
        refresh(&mut reopened, false)
            .await
            .find(RADIUS)
            .unwrap()
            .outcome,
        RefreshOutcome::Restored { models: 0 }
    );
    let mut default = builtin(None, Some(store), Default::default());
    assert!(refresh(&mut default, false).await.find(RADIUS).is_none());
    assert!(!default.get_models(Some(RADIUS)).is_empty());
    assert_eq!(server.received_requests().await.unwrap().len(), 1);

    // Older native publications have no gateway scope. The current reader
    // still accepts them and preserves an acknowledged empty catalog.
    let old_path = directory.path().join("old-models.json");
    std::fs::write(
        &old_path,
        br#"{"revision":1,"providers":{"radius":{"models":[],"x-opi":{"publisher":{"kind":"native"}}}}}"#,
    )
    .unwrap();
    default.set_models_store(ModelsStore::open(old_path).unwrap());
    assert_eq!(
        refresh(&mut default, false)
            .await
            .find(RADIUS)
            .unwrap()
            .outcome,
        RefreshOutcome::Restored { models: 0 }
    );
    assert!(default.get_models(Some(RADIUS)).is_empty());
}

#[tokio::test]
async fn failed_or_unconfigured_refresh_retains_last_known_catalog() {
    let server = MockServer::start().await;
    let directory = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(directory.path().join("models.json")).unwrap();
    let gateway = server.uri();
    let base = format!("{gateway}/v1");
    let mut models = builtin(Some(&gateway), Some(store.clone()), key_context());
    mount_config(
        &server,
        KEY,
        ResponseTemplate::new(200).set_body_json(config(&base, vec![remote_model("known")])),
    )
    .await;
    assert_eq!(
        refresh(&mut models, true)
            .await
            .find(RADIUS)
            .unwrap()
            .outcome,
        RefreshOutcome::Published { models: 1 }
    );
    let original = store.snapshot().catalogs;
    let broken_row = json!({ "id": "malformed" });
    let wrong_port = MockServer::start().await;
    for response in [
        ResponseTemplate::new(503).set_body_string("fixture upstream body"),
        ResponseTemplate::new(200).set_body_json(config(&base, vec![broken_row])),
        ResponseTemplate::new(200).set_body_json(config(
            &base,
            vec![remote_model("dup"), remote_model("dup")],
        )),
        ResponseTemplate::new(200).set_body_json(config(
            &wrong_port.uri(),
            vec![remote_model("wrong-origin")],
        )),
    ] {
        server.verify().await;
        server.reset().await;
        mount_config(&server, KEY, response).await;
        let report = refresh(&mut models, true).await;
        assert!(report.entries().last().unwrap().outcome.is_failure());
        assert!(models.get_model(RADIUS, "known").is_some());
        assert_eq!(store.snapshot().catalogs, original);
    }
    server.verify().await;
    server.reset().await;
    let mut unconfigured = builtin(Some(&gateway), Some(store.clone()), Default::default());
    refresh(&mut unconfigured, true).await;
    assert!(unconfigured.get_model(RADIUS, "known").is_some());
    assert_eq!(store.snapshot().catalogs, original);
    assert!(server.received_requests().await.unwrap().is_empty());
    assert!(wrong_port.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn expired_oauth_is_refreshed_at_the_configured_gateway_only_when_online() {
    let server = MockServer::start().await;
    let api_base = format!("{}/v1", server.uri());
    let keys = Arc::new(InMemoryCredentialStore::new());
    keys.write(
        RADIUS,
        &Credential::OAuthToken {
            access: "fixture-expired-access".into(),
            refresh: "fixture-refresh".into(),
            expires_at: Some(time::OffsetDateTime::now_utc() - time::Duration::hours(1)),
            base_url: None,
            account_id: None,
            client_id: None,
            enterprise_domain: None,
            available_model_ids: None,
            scopes: None,
        },
    )
    .await
    .unwrap();
    let context = key_context();
    let mut models = builtin(None, None, context.clone());
    models.set_credential_store(keys.clone());
    register_at(&mut models, Some(&api_base), context.clone());
    let reads = keys.read_count();
    refresh(&mut models, false).await;
    assert_eq!(keys.read_count(), reads);
    assert!(server.received_requests().await.unwrap().is_empty());

    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "fixture-new-access",
            "refresh_token": "fixture-new-refresh",
            "expires_in": 3600,
        })))
        .expect(1)
        .mount(&server)
        .await;
    mount_config(
        &server,
        "fixture-new-access",
        ResponseTemplate::new(200)
            .set_body_json(config(&api_base, vec![remote_model("oauth-model")])),
    )
    .await;
    assert_eq!(
        refresh(&mut models, true)
            .await
            .find(RADIUS)
            .unwrap()
            .outcome,
        RefreshOutcome::Published { models: 1 }
    );
    assert!(models.get_model(RADIUS, "oauth-model").is_some());
    assert_eq!(context.reads.load(Ordering::SeqCst), 0);
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests[0].url.path(), "/v1/oauth/token");
    assert_eq!(requests[1].url.path(), "/v1/config");
}

async fn receive_config(listener: &TcpListener) -> TcpStream {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut reader = BufReader::new(&mut socket);
        let mut request = String::new();
        loop {
            let previous = request.len();
            let read = reader.read_line(&mut request).await.unwrap();
            assert!(read > 0, "expected complete HTTP headers");
            if &request[previous..] == "\r\n" {
                break;
            }
        }
        assert!(request.starts_with("GET /v1/config HTTP/1.1\r\n"));
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer fixture-radius-key")
        );
        socket
    })
    .await
    .expect("builtin sends an authenticated request")
}

async fn respond(mut socket: TcpStream, gateway: &str, id: &str) {
    let body = config(&format!("{gateway}/v1"), vec![remote_model(id)]).to_string();
    socket
        .write_all(
            format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len(),
            )
            .as_bytes(),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn cancellation_while_waiting_for_http_preserves_the_last_store_revision() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gateway = format!("http://{}", listener.local_addr().unwrap());
    let directory = tempfile::tempdir().unwrap();
    let path_to_store = directory.path().join("models.json");
    let store = ModelsStore::open(&path_to_store).unwrap();
    let mut models = builtin(Some(&gateway), Some(store.clone()), key_context());
    let seed = tokio::spawn(async move {
        assert_eq!(
            refresh(&mut models, true).await.status(),
            RefreshStatus::Published
        );
        models
    });
    respond(receive_config(&listener).await, &gateway, "seed").await;
    let mut models = seed.await.unwrap();
    let revision = store.revision();
    let bytes = std::fs::read(&path_to_store).unwrap();
    let cancel = CancellationToken::new();
    let caller_token = cancel.clone();
    let waiting = tokio::spawn(async move {
        let report = models
            .refresh_with(
                catalog_fixture::refresh_options(&[RADIUS], true, None),
                &caller_token,
            )
            .await
            .unwrap();
        (models, report)
    });
    let _held_socket = receive_config(&listener).await;
    cancel.cancel();
    let (models, report) = tokio::time::timeout(Duration::from_secs(2), waiting)
        .await
        .expect("cancellation returns without the gateway responding")
        .unwrap();
    assert_eq!(report.status(), RefreshStatus::Cancelled);
    assert!(models.get_model(RADIUS, "seed").is_some());
    assert_eq!(store.revision(), revision);
    assert_eq!(std::fs::read(&path_to_store).unwrap(), bytes);
}

#[tokio::test]
async fn slower_builtin_refresh_cannot_replace_a_shared_store_commit() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gateway = format!("http://{}", listener.local_addr().unwrap());
    let directory = tempfile::tempdir().unwrap();
    let path_to_store = directory.path().join("models.json");
    let store = ModelsStore::open(&path_to_store).unwrap();
    let mut old = builtin(Some(&gateway), Some(store.clone()), key_context());
    let mut new = builtin(Some(&gateway), Some(store.clone()), key_context());
    let old_call = tokio::spawn(async move {
        let report = refresh(&mut old, true).await;
        (old, report)
    });
    let old_peer = receive_config(&listener).await;
    let new_call = tokio::spawn(async move {
        let report = refresh(&mut new, true).await;
        (new, report)
    });
    respond(receive_config(&listener).await, &gateway, "newer").await;
    let (new, report) = new_call.await.unwrap();
    assert_eq!(report.status(), RefreshStatus::Published);
    assert!(new.get_model(RADIUS, "newer").is_some());
    let revision = store.revision();
    respond(old_peer, &gateway, "stale").await;
    let (old, report) = old_call.await.unwrap();
    assert_eq!(report.status(), RefreshStatus::Superseded);
    assert!(old.get_model(RADIUS, "stale").is_none());
    assert_eq!(store.revision(), revision);

    let mut reopened = builtin(
        Some(&gateway),
        Some(ModelsStore::open(path_to_store).unwrap()),
        Default::default(),
    );
    assert_eq!(
        refresh(&mut reopened, false)
            .await
            .find(RADIUS)
            .unwrap()
            .outcome,
        RefreshOutcome::Restored { models: 1 }
    );
    assert!(reopened.get_model(RADIUS, "newer").is_some());
    assert!(reopened.get_model(RADIUS, "stale").is_none());
}
