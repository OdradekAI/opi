//! xAI OAuth fixtures shared by the dedicated and cross-provider targets.

use opi_ai::auth::OAuthCredential;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[allow(unused_imports)]
pub use opi_ai::auth::oauth::xai::{
    XAI_CLIENT_ID, XAI_DEVICE_CODE_URL, XAI_SCOPE, XAI_TOKEN_URL, XaiOAuthProvider,
    parse_xai_device_authorization,
};

pub fn xai_start_body(
    interval: serde_json::Value,
    expires_in: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "device_code": "xai-device-1",
        "user_code": "XAI-CODE",
        "verification_uri": "https://auth.x.ai/device",
        "verification_uri_complete": "https://auth.x.ai/device?user_code=XAI-CODE",
        "interval": interval,
        "expires_in": expires_in,
    })
}

pub async fn mount_xai_start(server: &MockServer, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/oauth2/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

pub async fn mount_xai_token(server: &MockServer, status: u16, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .mount(server)
        .await;
}

pub fn xai_token_success() -> serde_json::Value {
    serde_json::json!({
        "access_token": "xai-access-1",
        "refresh_token": "xai-refresh-1",
        "expires_in": 3600,
    })
}

pub fn xai_flow(server: &MockServer) -> XaiOAuthProvider {
    XaiOAuthProvider::new_with_endpoints(
        format!("{}/oauth2/device/code", server.uri()),
        format!("{}/oauth2/token", server.uri()),
    )
}

pub async fn xai_token_requests(server: &MockServer) -> Vec<wiremock::Request> {
    server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.url.path() == "/oauth2/token")
        .collect()
}

pub fn xai_stored_credential() -> OAuthCredential {
    OAuthCredential {
        access: secrecy::SecretString::new("xai-old-access".to_owned().into_boxed_str()),
        refresh: secrecy::SecretString::new("xai-old-refresh".to_owned().into_boxed_str()),
        expires_at: Some(time::OffsetDateTime::now_utc() - time::Duration::minutes(1)),
        base_url: None,
        account_id: None,
        client_id: None,
        scopes: None,
        enterprise_domain: None,
        available_model_ids: None,
    }
}

pub fn merge_json(base: serde_json::Value, extra: serde_json::Value) -> serde_json::Value {
    let mut base = base;
    let map = base.as_object_mut().unwrap();
    for (k, v) in extra.as_object().unwrap() {
        map.insert(k.clone(), v.clone());
    }
    base
}
