use std::sync::Arc;
use std::time::Duration;

#[allow(unused_imports)]
pub use opi_ai::auth::oauth::codex::{
    CODEX_DEVICE_REDIRECT_URI, CODEX_REDIRECT_URI, CodexOAuthProvider,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::oauth_fixtures::ScriptedPresenter;

pub const CODEX_DEVICE_VERIFICATION: &str = "https://auth.openai.com/codex/device";

pub fn codex_flow(
    server: &MockServer,
    browser_timeout: Duration,
    device_timeout: Duration,
) -> CodexOAuthProvider {
    let base = server.uri();
    CodexOAuthProvider::new_with_device_endpoints(
        format!("{base}/oauth/authorize"),
        format!("{base}/oauth/token"),
        format!("{base}/deviceauth/usercode"),
        format!("{base}/deviceauth/token"),
        CODEX_DEVICE_VERIFICATION.to_owned(),
        CODEX_DEVICE_REDIRECT_URI.to_owned(),
        "codex-client-id".to_owned(),
        browser_timeout,
        device_timeout,
    )
}

pub fn codex_jwt(account_id: Option<&str>) -> String {
    use base64::Engine as _;
    let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header = engine.encode(br#"{"alg":"RS256"}"#);
    let payload = match account_id {
        Some(id) => format!(r#"{{"https://api.openai.com/auth":{{"chatgpt_account_id":"{id}"}}}}"#),
        None => "{}".to_owned(),
    };
    let payload = engine.encode(payload.as_bytes());
    format!("{header}.{payload}.signature")
}

pub fn codex_presenter() -> Arc<ScriptedPresenter> {
    let presenter = Arc::new(ScriptedPresenter::default());
    let hook = ScriptedPresenter::echo_redirect_hook();
    *presenter.on_present.lock().unwrap() = Some(hook);
    presenter
}

pub async fn mount_codex_token_success(server: &MockServer, access: &str) {
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": access,
            "refresh_token": "codex-refresh-1",
            "expires_in": 3600,
        })))
        .mount(server)
        .await;
}

pub async fn mount_codex_device_start(server: &MockServer, interval: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_auth_id": "device-auth-1",
            "user_code": "PUBLIC-CODE",
            "interval": interval,
        })))
        .mount(server)
        .await;
}

pub async fn mount_codex_device_poll_complete(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "authorization_code": "device-code-1",
            "code_verifier": "device-verifier-from-server",
        })))
        .mount(server)
        .await;
}

pub async fn mount_codex_device_poll_status(
    server: &MockServer,
    status: u16,
    body: serde_json::Value,
) {
    Mock::given(method("POST"))
        .and(path("/deviceauth/token"))
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .mount(server)
        .await;
}

pub fn urlencode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}
