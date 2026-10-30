//! Anthropic's selected OIDC file exchange and in-memory token cache.
//!
//! This owner is used only by the Messages adapter after the complete model
//! endpoint and transport have been frozen. It never discovers credentials,
//! installs a background service, or writes token files.

use std::sync::{Arc, Mutex};

use futures_util::future::{BoxFuture, Shared};
use futures_util::{FutureExt, StreamExt};
use secrecy::{ExposeSecret, SecretString};

use crate::auth::ResolvedAuth;
use crate::http::HttpClient;
use crate::provider::{ProviderError, ProviderErrorSummary};
use crate::schema::ecma_number::{js_to_number, js_trim, js_value_to_string};

type Clock = Arc<dyn Fn() -> f64 + Send + Sync>;
type Refresh = Shared<BoxFuture<'static, Result<Token, ProviderError>>>;

const FIELDS: [&str; 5] = [
    "ANTHROPIC_FEDERATION_RULE_ID",
    "ANTHROPIC_ORGANIZATION_ID",
    "ANTHROPIC_IDENTITY_TOKEN_FILE",
    "ANTHROPIC_SERVICE_ACCOUNT_ID",
    "ANTHROPIC_WORKSPACE_ID",
];

#[derive(Clone)]
struct Config {
    base: String,
    values: [Option<SecretString>; 5],
    transport: usize,
}

impl Config {
    fn new(
        base: &str,
        auth: &ResolvedAuth,
        client: &Arc<HttpClient>,
    ) -> Result<Self, ProviderError> {
        let url =
            url::Url::parse(base).map_err(|_| config_error("Invalid federation API base URL"))?;
        let secure = url.scheme() == "https"
            || (url.scheme() == "http"
                && matches!(
                    url.host_str(),
                    Some("localhost" | "127.0.0.1" | "[::1]" | "::1")
                ));
        if !secure || url.host_str().is_none() {
            return Err(config_error(
                "Federation requires HTTPS or a loopback HTTP API base",
            ));
        }
        let values = FIELDS.map(|name| auth.env.get(name).cloned());
        if values[..3]
            .iter()
            .any(|v| v.as_ref().is_none_or(|v| v.expose_secret().is_empty()))
        {
            return Err(config_error(
                "Federation requires rule, organization, and identity token file",
            ));
        }
        Ok(Self {
            base: base.to_owned(),
            values,
            transport: Arc::as_ptr(client) as usize,
        })
    }

    fn same_binding(&self, other: &Self) -> bool {
        self.base == other.base
            && self.transport == other.transport
            && self.values.iter().zip(&other.values).all(|(a, b)| {
                a.as_ref().map(ExposeSecret::expose_secret)
                    == b.as_ref().map(ExposeSecret::expose_secret)
            })
    }

    fn value(&self, index: usize) -> &str {
        self.values[index]
            .as_ref()
            .map(ExposeSecret::expose_secret)
            .unwrap_or_default()
    }
}

#[derive(Clone)]
struct Token {
    value: SecretString,
    expires_at: f64,
}

#[derive(Default)]
struct State {
    cached: Option<Token>,
    pending: Option<(u64, Refresh)>,
    generation: u64,
    force: bool,
    last_advisory_error: Option<f64>,
    background: Vec<tokio::task::AbortHandle>,
}

pub(crate) struct FederationEntry {
    config: Config,
    client: Arc<HttpClient>,
    clock: Clock,
    state: Mutex<State>,
}

impl Drop for FederationEntry {
    fn drop(&mut self) {
        for task in self
            .state
            .get_mut()
            .unwrap_or_else(|e| e.into_inner())
            .background
            .drain(..)
        {
            task.abort();
        }
    }
}

/// Provider clones share their last selected config and token owner. A changed
/// binding replaces the entry; already-running calls retain their owned entry.
#[derive(Clone)]
pub(crate) struct FederationCache {
    current: Arc<Mutex<Option<Arc<FederationEntry>>>>,
    clock: Clock,
}

impl Default for FederationCache {
    fn default() -> Self {
        Self {
            current: Arc::new(Mutex::new(None)),
            clock: Arc::new(|| {
                (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as f64 / 1000.0
            }),
        }
    }
}

impl FederationCache {
    pub(crate) fn bind(
        &self,
        base: &str,
        auth: &ResolvedAuth,
        client: &Arc<HttpClient>,
    ) -> Result<Arc<FederationEntry>, ProviderError> {
        let config = Config::new(base, auth, client)?;
        let mut current = self.current.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = &*current
            && entry.config.same_binding(&config)
        {
            return Ok(entry.clone());
        }
        let entry = Arc::new(FederationEntry {
            config,
            client: client.clone(),
            clock: self.clock.clone(),
            state: Mutex::new(State::default()),
        });
        *current = Some(entry.clone());
        Ok(entry)
    }
}

impl FederationEntry {
    /// A 401 invalidates only this binding. The current generation request is
    /// never resent here; a subsequent call must exchange a fresh assertion.
    pub(crate) fn invalidate(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.cached = None;
        state.force = true;
        state.generation += 1;
    }

    pub(crate) async fn token(self: &Arc<Self>) -> Result<SecretString, ProviderError> {
        let refresh = {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let force = std::mem::take(&mut state.force);
            if !force && let Some(token) = state.cached.clone() {
                let now = (self.clock)();
                let remaining = token.expires_at - now;
                if remaining > 120.0 {
                    return Ok(token.value);
                }
                if remaining > 30.0 {
                    if state.pending.is_none()
                        && state
                            .last_advisory_error
                            .is_none_or(|last| now - last >= 5.0)
                    {
                        drop(self.start_refresh(&mut state, true));
                    }
                    return Ok(token.value);
                }
            }
            if !force && let Some((_, pending)) = &state.pending {
                pending.clone()
            } else {
                self.start_refresh(&mut state, false)
            }
        };
        refresh.await.map(|token| token.value)
    }

    fn start_refresh(self: &Arc<Self>, state: &mut State, advisory: bool) -> Refresh {
        state.generation += 1;
        let generation = state.generation;
        let owner = Arc::downgrade(self);
        let config = self.config.clone();
        let client = self.client.client().clone();
        let clock = self.clock.clone();
        let refresh = async move {
            let result = exchange(&config, &client, &clock).await;
            if advisory && let Err(error) = &result {
                tracing::debug!(error = %error, "Anthropic federation advisory refresh failed; serving cached token");
            }
            if let Some(owner) = owner.upgrade() {
                let mut state = owner.state.lock().unwrap_or_else(|e| e.into_inner());
                if generation == state.generation {
                    match &result {
                        Ok(token) => state.cached = Some(token.clone()),
                        Err(_) if advisory => state.last_advisory_error = Some(clock()),
                        Err(_) => (),
                    }
                    state.pending = None;
                }
            }
            result
        }
        .boxed()
        .shared();
        state.pending = Some((generation, refresh.clone()));
        // As with the source's eager Promise, refresh completion and expiry
        // accounting belong to this cache, even if its last waiter cancels.
        // The task holds only a weak owner; dropping the owner aborts it.
        state.background.retain(|task| !task.is_finished());
        let driven = refresh.clone();
        state.background.push(
            tokio::spawn(async move {
                let _ = driven.await;
            })
            .abort_handle(),
        );
        refresh
    }
}

fn config_error(message: &'static str) -> ProviderError {
    ProviderError::Config(ProviderErrorSummary::attested_static(message))
}

fn auth_error(message: &'static str) -> ProviderError {
    ProviderError::AuthFailed(ProviderErrorSummary::attested_static(message))
}

async fn exchange(
    config: &Config,
    client: &reqwest::Client,
    clock: &Clock,
) -> Result<Token, ProviderError> {
    let bytes = tokio::fs::read(config.value(2))
        .await
        .map_err(|_| auth_error("Federation identity token file could not be read"))?;
    let content = String::from_utf8_lossy(&bytes);
    let assertion = js_trim(&content);
    if assertion.is_empty() {
        return Err(auth_error("Federation identity token file is empty"));
    }
    if assertion.encode_utf16().count() > 16 * 1024 {
        return Err(auth_error(
            "Federation identity token exceeds the source 16 KiB assertion limit",
        ));
    }
    let mut body = serde_json::json!({
        "grant_type":"urn:ietf:params:oauth:grant-type:jwt-bearer",
        "assertion":assertion,
        "federation_rule_id":config.value(0),
        "organization_id":config.value(1),
    });
    for (index, field) in [(3, "service_account_id"), (4, "workspace_id")] {
        if !config.value(index).is_empty() {
            body[field] = config.value(index).into();
        }
    }
    let response = client
        .post(format!(
            "{}/v1/oauth/token",
            config.base.trim_end_matches('/')
        ))
        .header("content-type", "application/json")
        .header(
            "anthropic-beta",
            "oauth-2025-04-20,oidc-federation-2026-04-01",
        )
        .header(
            "user-agent",
            concat!(
                "opi-ai/",
                env!("CARGO_PKG_VERSION"),
                " oidcFederationProvider"
            ),
        )
        .json(&body)
        .send()
        .await
        .map_err(|_| auth_error("Federation token endpoint could not be reached"))?;
    let status = response.status().as_u16();
    let request_id = response
        .headers()
        .get("request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    // Retain response identity and category, never the credential endpoint body.
    let response_error = |message: &str| {
        let mut summary = format!("HTTP {status}: {message}");
        if let Some(id) = &request_id {
            summary.push_str(" (request-id: ");
            summary.push_str(id);
            summary.push(')');
        }
        ProviderError::AuthFailed(ProviderErrorSummary::sanitized(summary))
    };
    if !response.status().is_success() {
        return Err(response_error(
            "Federation token endpoint rejected the exchange",
        ));
    }
    // The locked token parser reads only its first 1 MiB, then closes the body.
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|_| response_error("Federation token response could not be read"))?;
        let remaining = (1 << 20) - bytes.len();
        bytes.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
        if chunk.len() >= remaining {
            break;
        }
    }
    let text = String::from_utf8_lossy(&bytes);
    // The source TextDecoder replaces invalid UTF-8 and consumes one BOM.
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|_| response_error("Federation token endpoint returned invalid JSON"))?;
    let token = value
        .get("access_token")
        .and_then(serde_json::Value::as_str)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            response_error("Federation token response requires a nonempty access token")
        })?;
    if let Some(kind) = value.get("token_type")
        && !kind.is_null()
        && !kind
            .as_str()
            .is_some_and(|s| s.is_empty() || s.eq_ignore_ascii_case("bearer"))
    {
        return Err(response_error(
            "Federation token response has an unsupported token type",
        ));
    }
    let ttl = number(value.get("expires_in"))
        .filter(|value| value.is_finite())
        .ok_or_else(|| response_error("Federation token response requires finite expires_in"))?;
    Ok(Token {
        value: token.into(),
        expires_at: clock() + ttl,
    })
}

fn number(value: Option<&serde_json::Value>) -> Option<f64> {
    use serde_json::Value;
    match value? {
        Value::Number(value) => value.as_f64(),
        Value::Null => Some(0.0),
        Value::Bool(value) => Some(u8::from(*value) as f64),
        Value::String(value) => Some(js_to_number(value)),
        value @ Value::Array(_) => Some(js_to_number(&array_primitive(value))),
        Value::Object(_) => Some(f64::NAN),
    }
}

/// JSON arrays use Array.toString before Number coercion. Null elements are
/// empty and nested arrays retain their own commas; objects stringify to the
/// ordinary object tag, which cannot be parsed as a finite number.
fn array_primitive(value: &serde_json::Value) -> String {
    use serde_json::Value;
    enum Part<'a> {
        Value(&'a Value),
        Comma,
    }
    let mut pending = vec![Part::Value(value)];
    let mut output = String::new();
    while let Some(part) = pending.pop() {
        match part {
            Part::Comma => output.push(','),
            Part::Value(Value::Array(values)) => {
                for (index, value) in values.iter().enumerate().rev() {
                    pending.push(Part::Value(value));
                    if index > 0 {
                        pending.push(Part::Comma);
                    }
                }
            }
            Part::Value(Value::Null) => (),
            Part::Value(Value::String(value)) => output.push_str(value),
            Part::Value(Value::Object(_)) => output.push_str("[object Object]"),
            Part::Value(value) => output.push_str(&js_value_to_string(value).unwrap_or_default()),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn token_expiry_coercion_preserves_json_primitive_rules() {
        use serde_json::json;
        for (value, expected) in [
            (json!("\u{feff}0xE10\u{a0}"), 3600.0),
            (json!("0b111000010000"), 3600.0),
            (json!("0o7020"), 3600.0),
            (json!(null), 0.0),
            (json!(false), 0.0),
            (json!(true), 1.0),
            (json!([]), 0.0),
            (json!([null]), 0.0),
            (json!([["3600"]]), 3600.0),
            (json!(-30), -30.0),
        ] {
            assert_eq!(number(Some(&value)), Some(expected), "{value}");
        }
        for value in [json!({}), json!([1, 2]), json!([{}]), json!("bad")] {
            assert!(number(Some(&value)).unwrap().is_nan(), "{value}");
        }
        assert!(number(Some(&json!("Infinity"))).unwrap().is_infinite());
        assert_eq!(number(None), None);
        assert_eq!(array_primitive(&json!([[1, 2], null, [3]])), "1,2,,3");
    }

    async fn fixture() -> (
        tempfile::TempDir,
        MockServer,
        FederationCache,
        Arc<FederationEntry>,
        Arc<AtomicI64>,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("identity");
        std::fs::write(&file, "fixture-jwt").unwrap();
        let server = MockServer::start().await;
        let mut auth = crate::test_support::resolved_auth();
        auth.env = [
            (FIELDS[0], "rule".to_owned()),
            (FIELDS[1], "org".to_owned()),
            (FIELDS[2], file.to_string_lossy().into_owned()),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.into()))
        .collect();
        let now = Arc::new(AtomicI64::new(1000));
        let clock = now.clone();
        let cache = FederationCache {
            clock: Arc::new(move || clock.load(Ordering::SeqCst) as f64),
            ..Default::default()
        };
        let entry = cache
            .bind(&server.uri(), &auth, &Arc::new(HttpClient::new()))
            .unwrap();
        (dir, server, cache, entry, now)
    }

    async fn settle(entry: &FederationEntry) {
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while entry.state.lock().unwrap().pending.is_some() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn wallclock_thresholds_and_concurrent_refresh_follow_source_boundaries() {
        let (_dir, server, _cache, entry, now) = fixture().await;
        let sends = Arc::new(AtomicUsize::new(0));
        let count = sends.clone();
        Mock::given(method("POST"))
            .and(path("/v1/oauth/token"))
            .respond_with(move |_: &wiremock::Request| {
                let n = count.fetch_add(1, Ordering::SeqCst) + 1;
                ResponseTemplate::new(200)
                    .set_delay(std::time::Duration::from_millis(50))
                    .set_body_json(
                        serde_json::json!({"access_token":format!("access-{n}"),"expires_in":3600}),
                    )
            })
            .mount(&server)
            .await;
        let (a, b) = tokio::join!(entry.token(), entry.token());
        assert_eq!(a.unwrap().expose_secret(), "access-1");
        assert_eq!(b.unwrap().expose_secret(), "access-1");
        assert_eq!(sends.load(Ordering::SeqCst), 1);
        now.store(4479, Ordering::SeqCst); // 121 seconds: no refresh.
        assert_eq!(entry.token().await.unwrap().expose_secret(), "access-1");
        assert_eq!(sends.load(Ordering::SeqCst), 1);
        now.store(4480, Ordering::SeqCst); // Exactly 120: stale immediately.
        assert_eq!(entry.token().await.unwrap().expose_secret(), "access-1");
        now.store(4569, Ordering::SeqCst); // 31: joins advisory, still stale.
        assert_eq!(entry.token().await.unwrap().expose_secret(), "access-1");
        now.store(4570, Ordering::SeqCst); // Exactly 30: waits for pending refresh.
        assert_eq!(entry.token().await.unwrap().expose_secret(), "access-2");
        assert_eq!(sends.load(Ordering::SeqCst), 2);
        now.store(100, Ordering::SeqCst); // Wallclock correction changes freshness.
        assert_eq!(entry.token().await.unwrap().expose_secret(), "access-2");
        assert_eq!(sends.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn advisory_failure_backs_off_five_seconds_but_mandatory_failure_is_observable() {
        let (_dir, server, _cache, entry, now) = fixture().await;
        let sends = Arc::new(AtomicUsize::new(0));
        let count = sends.clone();
        Mock::given(method("POST"))
            .and(path("/v1/oauth/token"))
            .respond_with(move |_: &wiremock::Request| {
                if count.fetch_add(1, Ordering::SeqCst) == 0 {
                    ResponseTemplate::new(200).set_body_json(
                        serde_json::json!({"access_token":"old-token","expires_in":100}),
                    )
                } else {
                    ResponseTemplate::new(500).set_body_string("secret-server-canary")
                }
            })
            .mount(&server)
            .await;
        entry.token().await.unwrap();
        assert_eq!(entry.token().await.unwrap().expose_secret(), "old-token");
        settle(&entry).await;
        assert_eq!(sends.load(Ordering::SeqCst), 2);
        now.store(1004, Ordering::SeqCst);
        entry.token().await.unwrap();
        assert_eq!(sends.load(Ordering::SeqCst), 2);
        now.store(1005, Ordering::SeqCst);
        entry.token().await.unwrap();
        settle(&entry).await;
        assert_eq!(sends.load(Ordering::SeqCst), 3);
        now.store(1070, Ordering::SeqCst);
        let error = entry.token().await.unwrap_err();
        assert!(matches!(error, ProviderError::AuthFailed(_)));
        assert!(!format!("{error:?}").contains("secret-server-canary"));
        assert_eq!(sends.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn advisory_refresh_is_aborted_when_its_owner_is_released() {
        let (_dir, server, cache, entry, _now) = fixture().await;
        let sends = Arc::new(AtomicUsize::new(0));
        let count = sends.clone();
        Mock::given(method("POST"))
            .and(path("/v1/oauth/token"))
            .respond_with(move |_: &wiremock::Request| {
                let first = count.fetch_add(1, Ordering::SeqCst) == 0;
                ResponseTemplate::new(200)
                    .set_delay(if first {
                        std::time::Duration::ZERO
                    } else {
                        std::time::Duration::from_secs(60)
                    })
                    .set_body_json(serde_json::json!({"access_token":"token","expires_in":100}))
            })
            .mount(&server)
            .await;
        entry.token().await.unwrap();
        entry.token().await.unwrap();
        let task = entry
            .state
            .lock()
            .unwrap()
            .background
            .last()
            .unwrap()
            .clone();
        drop(entry);
        drop(cache);
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while !task.is_finished() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("owned background task drops with cache");
    }

    #[tokio::test]
    async fn cancelled_mandatory_waiter_does_not_delay_expiry_or_reuse_expired_token() {
        let (_dir, server, _cache, entry, now) = fixture().await;
        let seen = Arc::new(tokio::sync::Notify::new());
        let request_seen = seen.clone();
        let sends = Arc::new(AtomicUsize::new(0));
        let count = sends.clone();
        Mock::given(method("POST"))
            .and(path("/v1/oauth/token"))
            .respond_with(move |_: &wiremock::Request| {
                let n = count.fetch_add(1, Ordering::SeqCst) + 1;
                request_seen.notify_one();
                ResponseTemplate::new(200)
                    .set_delay(std::time::Duration::from_millis(20))
                    .set_body_json(serde_json::json!({
                        "access_token": format!("token-{n}"), "expires_in": 60
                    }))
            })
            .expect(2)
            .mount(&server)
            .await;
        let owned = entry.clone();
        let waiter = tokio::spawn(async move { owned.token().await });
        seen.notified().await;
        waiter.abort();
        assert!(waiter.await.unwrap_err().is_cancelled());

        // The HTTP response completes while no caller waits. As with the
        // source's eager Promise, its expiry must be recorded now, not when a
        // later generation happens to request the cached token.
        let completed = tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while entry.state.lock().unwrap().pending.is_some() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .is_ok();
        now.store(1100, Ordering::SeqCst); // First response expired at 1060.
        let next = entry.token().await.unwrap();
        assert_eq!(
            next.expose_secret(),
            "token-2",
            "completed without waiter: {completed}; cancelled wait cannot defer expiry"
        );
        assert_eq!(sends.load(Ordering::SeqCst), 2);
        assert!(completed, "the exchange is driven by its cache owner");
    }

    #[tokio::test]
    async fn mandatory_refresh_stops_when_the_binding_owner_is_released() {
        let (_dir, server, cache, entry, _now) = fixture().await;
        let seen = Arc::new(tokio::sync::Notify::new());
        let request_seen = seen.clone();
        Mock::given(method("POST"))
            .and(path("/v1/oauth/token"))
            .respond_with(move |_: &wiremock::Request| {
                request_seen.notify_one();
                ResponseTemplate::new(200)
                    .set_delay(std::time::Duration::from_secs(60))
                    .set_body_json(serde_json::json!({"access_token":"token","expires_in":3600}))
            })
            .expect(1)
            .mount(&server)
            .await;
        let owned = entry.clone();
        let waiter = tokio::spawn(async move { owned.token().await });
        seen.notified().await;
        waiter.abort();
        assert!(waiter.await.unwrap_err().is_cancelled());
        let task = entry
            .state
            .lock()
            .unwrap()
            .background
            .last()
            .cloned()
            .expect("mandatory exchange has an owner-driven task");
        let weak = Arc::downgrade(&entry);
        drop(entry);
        drop(cache);
        assert!(
            weak.upgrade().is_none(),
            "task does not keep its owner alive"
        );
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while !task.is_finished() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("releasing the last owner aborts its mandatory exchange");
    }

    #[tokio::test]
    async fn configuration_endpoint_and_transport_each_own_a_token_binding() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("identity");
        std::fs::write(&file, "fixture-identity").unwrap();
        let server = MockServer::start().await;
        let calls = Arc::new(AtomicUsize::new(0));
        let count = calls.clone();
        Mock::given(method("POST"))
            .respond_with(move |_: &wiremock::Request| {
                let n = count.fetch_add(1, Ordering::SeqCst) + 1;
                ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"access_token":format!("token-{n}"),"expires_in":3600}),
                )
            })
            .mount(&server)
            .await;
        let mut auth = crate::test_support::resolved_auth();
        auth.env = [
            (FIELDS[0], "rule".to_owned()),
            (FIELDS[1], "org".to_owned()),
            (FIELDS[2], file.to_string_lossy().into_owned()),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.into()))
        .collect();
        let client = Arc::new(HttpClient::new());
        let cache = FederationCache::default();
        let original = cache.bind(&server.uri(), &auth, &client).unwrap();
        assert_eq!(original.token().await.unwrap().expose_secret(), "token-1");
        assert_eq!(
            cache
                .clone()
                .bind(&server.uri(), &auth, &client)
                .unwrap()
                .token()
                .await
                .unwrap()
                .expose_secret(),
            "token-1"
        );
        auth.env
            .insert(FIELDS[4].into(), "another-workspace".into());
        assert_eq!(
            cache
                .bind(&server.uri(), &auth, &client)
                .unwrap()
                .token()
                .await
                .unwrap()
                .expose_secret(),
            "token-2"
        );
        let base = format!("{}/custom///", server.uri());
        assert_eq!(
            cache
                .bind(&base, &auth, &client)
                .unwrap()
                .token()
                .await
                .unwrap()
                .expose_secret(),
            "token-3"
        );
        let other_client = Arc::new(HttpClient::new());
        assert_eq!(
            cache
                .bind(&base, &auth, &other_client)
                .unwrap()
                .token()
                .await
                .unwrap()
                .expose_secret(),
            "token-4"
        );
        assert_eq!(original.token().await.unwrap().expose_secret(), "token-1");
        assert_eq!(calls.load(Ordering::SeqCst), 4);
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests[2].url.path(), "/custom/v1/oauth/token");
        let body: serde_json::Value = serde_json::from_slice(&requests[1].body).unwrap();
        assert_eq!(body["workspace_id"], "another-workspace");
    }
}
