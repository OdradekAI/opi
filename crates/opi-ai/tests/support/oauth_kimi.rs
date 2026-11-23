//! Kimi Coding OAuth fixtures shared by the dedicated and cross-provider targets.

use opi_ai::auth::OAuthCredential;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[allow(unused_imports)]
pub use opi_ai::auth::oauth::kimi_coding::{
    KIMI_CLIENT_ID, KIMI_DEFAULT_OAUTH_HOST, KimiCodingOAuthProvider,
    parse_kimi_device_authorization,
};

pub fn kimi_start_body(
    interval: serde_json::Value,
    expires_in: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "device_code": "kimi-device-1",
        "user_code": "KIMI-CODE",
        "verification_uri": "https://auth.kimi.com/device",
        "verification_uri_complete": "https://auth.kimi.com/device?user_code=KIMI-CODE",
        "interval": interval,
        "expires_in": expires_in,
    })
}

pub async fn mount_kimi_start(server: &MockServer, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/api/oauth/device_authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

pub async fn mount_kimi_token(server: &MockServer, status: u16, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/api/oauth/token"))
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .mount(server)
        .await;
}

pub fn kimi_token_success() -> serde_json::Value {
    serde_json::json!({
        "access_token": "kimi-access-1",
        "refresh_token": "kimi-refresh-1",
        "expires_in": 3600,
    })
}

pub fn kimi_flow(server: &MockServer) -> KimiCodingOAuthProvider {
    KimiCodingOAuthProvider::new_with_oauth_host(server.uri())
}

pub async fn kimi_token_requests(server: &MockServer) -> Vec<wiremock::Request> {
    server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.url.path() == "/api/oauth/token")
        .collect()
}

pub fn kimi_stored_credential() -> OAuthCredential {
    OAuthCredential {
        access: secrecy::SecretString::new("kimi-old-access".to_owned().into_boxed_str()),
        refresh: secrecy::SecretString::new("kimi-old-refresh".to_owned().into_boxed_str()),
        expires_at: Some(time::OffsetDateTime::now_utc() - time::Duration::minutes(1)),
        base_url: None,
        account_id: None,
        client_id: None,
        scopes: None,
        enterprise_domain: None,
        available_model_ids: None,
    }
}
