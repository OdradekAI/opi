use std::sync::{Arc, Mutex};
use std::time::Instant;

use opi_ai::auth::{LoginPresenter, OAuthCredential, OAuthLoginMethod};
use opi_ai::credential::BoxAuthFuture;
use opi_ai::provider::ProviderError;

/// Shared scripted presenter for OAuth flow targets.
///
/// The fixture owns interaction recording and manual replies. Provider targets
/// own only their protocol-specific setup and assertions.
pub type PresentHook = Arc<dyn Fn(&str) -> String + Send + Sync>;

#[derive(Default)]
pub struct ScriptedPresenter {
    pub authorize_urls: Mutex<Vec<String>>,
    pub manual_reply: Mutex<Option<String>>,
    pub on_present: Mutex<Option<PresentHook>>,
    pub forced_method: Mutex<Option<OAuthLoginMethod>>,
    pub offered_methods: Mutex<Vec<OAuthLoginMethod>>,
    pub selected_provider_ids: Mutex<Vec<String>>,
    pub device_codes: Mutex<Vec<(String, String)>>,
    pub prompt_answers: Mutex<Vec<String>>,
    pub progress: Mutex<Vec<String>>,
    pub success_at: Mutex<Option<Instant>>,
    pub failures: Mutex<Vec<String>>,
}

impl ScriptedPresenter {
    pub fn with_manual(code: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            manual_reply: Mutex::new(Some(code.into())),
            ..Self::default()
        })
    }

    /// Echo the presented authorize URL back as a pasted redirect URL.
    #[allow(dead_code)]
    pub fn echo_redirect_hook() -> PresentHook {
        Arc::new(|url: &str| {
            let state = Self::query_param(url, "state").unwrap();
            let redirect = Self::query_param(url, "redirect_uri").unwrap();
            format!("{redirect}?code=code-1&state={state}")
        })
    }

    pub fn last_authorize_url(&self) -> String {
        self.authorize_urls
            .lock()
            .unwrap()
            .last()
            .cloned()
            .expect("presenter saw no authorize url")
    }

    pub fn query_param(url: &str, name: &str) -> Option<String> {
        let query = url.split_once('?')?.1;
        for pair in query.split('&') {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            if key == name {
                return Some(percent_decode(value));
            }
        }
        None
    }
}

pub fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                out.push(byte);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Assert the service expiry remains within a small wall-clock tolerance.
pub fn expires_within(expires_at: time::OffsetDateTime, expires_in: i64) {
    let now = time::OffsetDateTime::now_utc();
    let target = now + time::Duration::seconds(expires_in);
    assert!(
        expires_at > now + time::Duration::seconds(expires_in - 30),
        "expires_at {expires_at} too early for +{expires_in}s"
    );
    assert!(
        expires_at < target + time::Duration::seconds(30),
        "expires_at {expires_at} too late for +{expires_in}s"
    );
}

pub fn stored_credential() -> OAuthCredential {
    OAuthCredential {
        access: secrecy::SecretString::from("old-access"),
        refresh: secrecy::SecretString::from("old-refresh"),
        expires_at: Some(time::OffsetDateTime::now_utc() - time::Duration::minutes(10)),
        base_url: None,
        account_id: None,
        client_id: None,
        scopes: None,
        enterprise_domain: None,
        available_model_ids: None,
    }
}

impl LoginPresenter for ScriptedPresenter {
    fn select_login_method<'a>(
        &'a self,
        provider_id: &'a str,
        methods: &'a [OAuthLoginMethod],
        default: OAuthLoginMethod,
    ) -> BoxAuthFuture<'a, Result<OAuthLoginMethod, ProviderError>> {
        let forced = *self.forced_method.lock().unwrap();
        *self.offered_methods.lock().unwrap() = methods.to_vec();
        self.selected_provider_ids
            .lock()
            .unwrap()
            .push(provider_id.to_owned());
        Box::pin(async move {
            if let Some(forced) = forced
                && methods.contains(&forced)
            {
                return Ok(forced);
            }
            if methods.contains(&default) {
                Ok(default)
            } else {
                Err(ProviderError::Config(
                    opi_ai::provider::ProviderErrorSummary::redacted(),
                ))
            }
        })
    }

    fn present_auth_url<'a>(
        &'a self,
        url: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        let url = url.to_owned();
        Box::pin(async move {
            if let Some(hook) = self.on_present.lock().unwrap().clone() {
                *self.manual_reply.lock().unwrap() = Some(hook(&url));
            }
            self.authorize_urls.lock().unwrap().push(url);
            Ok(())
        })
    }

    fn present_device_code<'a>(
        &'a self,
        user_code: &'a str,
        verification_uri: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        self.device_codes
            .lock()
            .unwrap()
            .push((user_code.to_owned(), verification_uri.to_owned()));
        Box::pin(async { Ok(()) })
    }

    fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
        let reply = self.manual_reply.lock().unwrap().clone();
        Box::pin(async move {
            match reply {
                Some(code) => Ok(code),
                None => std::future::pending().await,
            }
        })
    }

    fn prompt_text<'a>(
        &'a self,
        _message: &'a str,
        _placeholder: &'a str,
    ) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
        let answer = self
            .prompt_answers
            .lock()
            .unwrap()
            .pop()
            .unwrap_or_default();
        Box::pin(async move { Ok(answer) })
    }

    fn notify_progress(&self, message: &str) {
        self.progress.lock().unwrap().push(message.to_owned());
    }

    fn notify_success(&self) {
        *self.success_at.lock().unwrap() = Some(Instant::now());
    }

    fn notify_failure(&self, reason: &str) {
        self.failures.lock().unwrap().push(reason.to_owned());
    }
}
