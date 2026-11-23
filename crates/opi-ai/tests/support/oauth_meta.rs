//! Meta OAuth fixtures shared by the dedicated and cross-provider targets.

use opi_ai::auth::OAuthCredential;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[allow(unused_imports)]
pub use opi_ai::auth::oauth::meta::{
    META_API_KEY_MINT_URL, META_AUTH_HOST, META_CLIENT_ID, MetaOAuthProvider,
    parse_meta_device_authorization,
};

pub fn meta_start_body(
    interval: serde_json::Value,
    expires_in: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "device_code": "meta-device-1",
        "user_code": "META-CODE",
        "verification_uri": "https://auth.meta.com/device",
        "verification_uri_complete": "https://auth.meta.com/device?user_code=META-CODE",
        "interval": interval,
        "expires_in": expires_in,
    })
}

pub async fn mount_meta_start(server: &MockServer, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/oidc/device/authorization/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

pub async fn mount_meta_token(server: &MockServer, status: u16, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/oidc/device/token/"))
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .mount(server)
        .await;
}

pub async fn mount_meta_mint(server: &MockServer, status: u16, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/muse-code/key"))
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .mount(server)
        .await;
}

pub fn meta_identity_success() -> serde_json::Value {
    serde_json::json!({"access_token": "meta-identity-1"})
}

pub fn meta_mint_success() -> serde_json::Value {
    serde_json::json!({"api_key": "meta-api-key-1"})
}

pub fn meta_flow(server: &MockServer) -> MetaOAuthProvider {
    MetaOAuthProvider::new_with_endpoints(server.uri(), format!("{}/muse-code/key", server.uri()))
}

pub async fn requests_at(server: &MockServer, path: &str) -> Vec<wiremock::Request> {
    server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.url.path() == path)
        .collect()
}

pub fn meta_stored_credential() -> OAuthCredential {
    OAuthCredential {
        access: secrecy::SecretString::new("meta-old-api-key".to_owned().into_boxed_str()),
        refresh: secrecy::SecretString::new("meta-identity-1".to_owned().into_boxed_str()),
        expires_at: Some(time::OffsetDateTime::now_utc() - time::Duration::minutes(1)),
        base_url: None,
        account_id: None,
        client_id: None,
        scopes: None,
        enterprise_domain: None,
        available_model_ids: None,
    }
}
