use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use opi_ai::auth::oauth::anthropic::AnthropicOAuthProvider;
use opi_ai::auth::{OAuthCredential, OAuthProvider};
use opi_ai::provider::ProviderError;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::oauth_fixtures::ScriptedPresenter;

pub static PREFERRED_PORT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub async fn wait_preferred_port_free() {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match TcpListener::bind("127.0.0.1:53692") {
                Ok(listener) => {
                    drop(listener);
                    return;
                }
                Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                Err(error) => panic!("probe preferred-port fixture: {error}"),
            }
        }
    })
    .await
    .expect("parallel fixtures must release the preferred port");
}

pub async fn occupy_preferred_port_retrying() -> TcpListener {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match TcpListener::bind("127.0.0.1:53692") {
                Ok(listener) => return listener,
                Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                Err(error) => panic!("occupy preferred-port fixture: {error}"),
            }
        }
    })
    .await
    .expect("parallel fixtures must release the preferred port")
}

pub fn anthropic_flow(token_url: String) -> AnthropicOAuthProvider {
    AnthropicOAuthProvider::new(
        "https://authorize.example.test/authorize".to_owned(),
        token_url,
        "test-client-id".to_owned(),
        Duration::from_secs(30),
    )
}

pub fn token_body(access: &str, refresh: &str, expires_in: i64) -> serde_json::Value {
    serde_json::json!({
        "access_token": access,
        "refresh_token": refresh,
        "expires_in": expires_in,
    })
}

pub async fn mount_token_success(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(token_body("acc-1", "ref-1", 3600)))
        .mount(server)
        .await;
}

pub async fn port_guard() -> tokio::sync::MutexGuard<'static, ()> {
    PREFERRED_PORT_LOCK.lock().await
}

pub async fn login_via_echoed_redirect(
    flow: &AnthropicOAuthProvider,
) -> Result<OAuthCredential, ProviderError> {
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.on_present.lock().unwrap() = Some(ScriptedPresenter::echo_redirect_hook());
    flow.login(presenter.as_ref()).await
}
