use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::oauth_fixtures::ScriptedPresenter;

#[allow(unused_imports)]
pub use opi_ai::auth::oauth::radius::{
    RADIUS_CALLBACK_PORT, RADIUS_DEFAULT_GATEWAY, RADIUS_OAUTH_CLIENT_ID, RADIUS_OAUTH_SCOPE,
    RadiusOAuthProvider, parse_radius_device_authorization,
};

pub static RADIUS_PORT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub async fn radius_port_guard() -> tokio::sync::MutexGuard<'static, ()> {
    RADIUS_PORT_LOCK.lock().await
}

pub async fn mount_radius_discovery(server: &MockServer, endpoint: &str) {
    Mock::given(method("GET"))
        .and(path("/v1/oauth"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "authorizationEndpoint": endpoint })),
        )
        .mount(server)
        .await;
}

pub async fn mount_radius_token(server: &MockServer, status: u16, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .mount(server)
        .await;
}

pub fn radius_token_success() -> serde_json::Value {
    serde_json::json!({
        "access_token": "radius-access-1",
        "refresh_token": "radius-refresh-1",
        "expires_in": 3600,
    })
}

pub fn radius_device_start(interval: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "device_code": "radius-device-1",
        "user_code": "RAD-CODE",
        "verification_uri": "https://radius.example/device",
        "interval": interval,
        "expires_in": 900,
    })
}

pub async fn mount_radius_device_start(server: &MockServer, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/v1/oauth/device"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

pub fn radius_flow(server: &MockServer) -> RadiusOAuthProvider {
    RadiusOAuthProvider::new_with_gateway(server.uri())
}

pub async fn radius_token_requests(server: &MockServer) -> Vec<wiremock::Request> {
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|request| request.url.path() == "/v1/oauth/token")
        .collect()
}

#[allow(dead_code)]
pub async fn drive_radius_callback(presenter: &ScriptedPresenter, code: &str) {
    let mut url = None;
    for _ in 0..300 {
        url = presenter.authorize_urls.lock().unwrap().first().cloned();
        if url.is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let url = url.expect("authorize url never presented");
    let state = ScriptedPresenter::query_param(&url, "state").unwrap();
    let callback =
        format!("http://127.0.0.1:{RADIUS_CALLBACK_PORT}/oauth/callback?code={code}&state={state}");
    let response = reqwest::get(&callback).await.expect("callback request");
    assert_eq!(response.status(), 200);
}

#[allow(dead_code)]
pub fn custom_radius_flow(gateway: String, id: &str, name: &str) -> RadiusOAuthProvider {
    RadiusOAuthProvider::new_with_gateway(gateway).with_identity(id, name)
}
