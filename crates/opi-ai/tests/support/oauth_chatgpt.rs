use std::sync::Arc;
use std::time::Duration;

pub use opi_ai::auth::oauth::chatgpt::OpenAiChatGptOAuthProvider;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::oauth_fixtures::{PresentHook, ScriptedPresenter, percent_decode};

pub const CHATGPT_DEVICE_UUID: &str = "0F0E8D9A-1111-4222-8333-444455556666";
pub const CHATGPT_REDIRECT: &str = "http://127.0.0.1:1455/auth/callback";

pub fn chatgpt_flow(
    server: &MockServer,
    device_id: Option<&str>,
    timeout: Duration,
) -> OpenAiChatGptOAuthProvider {
    OpenAiChatGptOAuthProvider::new_with_endpoints(
        format!("{}/api/accounts/authorize", server.uri()),
        format!("{}/api/accounts/oauth/token", server.uri()),
        None,
        device_id.map(str::to_owned),
        timeout,
    )
}

pub fn chatgpt_paste_hook() -> PresentHook {
    Arc::new(|url: &str| {
        let state = ScriptedPresenter::query_param(url, "state").unwrap();
        format!("{CHATGPT_REDIRECT}?code=code-1&state={state}&client_id=issued-client-1")
    })
}

pub async fn mount_chatgpt_token(server: &MockServer, mutate: serde_json::Value) {
    let mut body = serde_json::json!({
        "access_token": "at-1",
        "refresh_token": "rt-1",
        "expires_in": 3600,
        "scope": "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct",
        "id_token": "idt-1",
    });
    if let (Some(map), Some(overrides)) = (body.as_object_mut(), mutate.as_object()) {
        for (key, value) in overrides {
            if value.is_null() {
                map.remove(key);
            } else {
                map.insert(key.clone(), value.clone());
            }
        }
    }
    Mock::given(method("POST"))
        .and(path("/api/accounts/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

pub fn form_pairs(body: &str) -> Vec<(String, String)> {
    body.split('&')
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (percent_decode(key), percent_decode(value))
        })
        .collect()
}

pub async fn chatgpt_callback_get(request_line: &str) -> Option<String> {
    let mut stream = tokio::net::TcpStream::connect("127.0.0.1:1455")
        .await
        .ok()?;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    stream
        .write_all(
            format!("{request_line} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .await
        .ok()?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.ok()?;
    Some(String::from_utf8_lossy(&buf).into_owned())
}

pub async fn chatgpt_wait_for_authorize_url(presenter: &ScriptedPresenter) -> String {
    for _ in 0..200 {
        if let Some(url) = presenter.authorize_urls.lock().unwrap().last() {
            return url.clone();
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("presenter saw no authorize url");
}
