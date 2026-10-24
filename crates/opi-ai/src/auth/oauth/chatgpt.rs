//! OpenAI "Sign in with ChatGPT" OAuth (Responses API token sharing).
//!
//! Pinned to pi 1.3.0 `openai-chatgpt.ts`: a public-client flow (no client
//! secret) that registers a dynamic client per login (`client_id=
//! dynamic_agent_client` at the authorize endpoint), receives OpenAI's
//! issued client id on the callback, and sends the resulting user access
//! token directly to api.openai.com. The fixed loopback redirect
//! `http://127.0.0.1:1455/auth/callback` is shared with other sessions and
//! the Codex CLI, so a failed bind is a typed terminal error — never the
//! Codex manual-only fallback, never a free port — while the bound server
//! races a strictly validated pasted-URL input.

use std::time::Duration;

use secrecy::{ExposeSecret, SecretString};
use time::OffsetDateTime;

use crate::auth::oauth::{
    AuthorizeStyle, CallbackBindFallback, FlowBudget, PkceInputError, PkceInputKind,
    PkceLoginConfig, TokenRequestEncoding, generate_state, no_redirect_client, provider_auth_error,
    provider_config_error, run_pkce_login_with, token_endpoint_error, unwrap_response,
};
use crate::auth::{BoxAuthFuture, LoginPresenter, OAuthCredential, OAuthProvider};
use crate::provider::ProviderError;

use super::{AuthzInput, ParsedInputError};

// Every login registers a new client with this id; OpenAI returns the issued
// client ID in the callback (and it must be reused for the exchange and every
// later refresh).
pub const CHATGPT_DYNAMIC_CLIENT_ID: &str = "dynamic_agent_client";
pub const CHATGPT_AGENT_NAME_HINT: &str = "opi";
pub const CHATGPT_AUTHORIZE_URL: &str = "https://auth.openai.com/api/accounts/authorize";
pub const CHATGPT_TOKEN_URL: &str = "https://auth.openai.com/api/accounts/oauth/token";
pub const CHATGPT_RESOURCE: &str = "https://api.openai.com/v1";
/// RFC 8707 resource indicator carried by the authorize URL and both token
/// requests; pins the token audience to the Responses API.
pub const CHATGPT_CALLBACK_PORT: u16 = 1455;
pub const CHATGPT_CALLBACK_PATH: &str = "/auth/callback";
pub const CHATGPT_REDIRECT_URI: &str = "http://127.0.0.1:1455/auth/callback";
/// The direct-token grant flag: the token response scope must contain it.
pub const CHATGPT_DIRECT_TOKEN_SCOPE: &str = "chatgpt.tokens.use.direct";
pub const CHATGPT_SCOPE: &str =
    "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";

/// Provider id used for typed errors, flow registration, and store keys.
pub const CHATGPT_PROVIDER_ID: &str = "openai-chatgpt";

type ChatGptTokenResponseJson = serde_json::Value;

#[derive(Default)]
struct ChatGptTokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<f64>,
    scope: Option<String>,
    id_token: Option<String>,
}

impl ChatGptTokenResponse {
    fn parse(raw: &ChatGptTokenResponseJson) -> Self {
        let text = |field: &str| {
            raw.get(field)
                .and_then(|value| value.as_str())
                .map(str::to_owned)
        };
        Self {
            access_token: text("access_token"),
            refresh_token: text("refresh_token"),
            expires_in: raw.get("expires_in").and_then(|v| v.as_f64()),
            scope: text("scope"),
            id_token: text("id_token"),
        }
    }
}

/// Validate the token response per pi `credentialFromTokenResponse`:
/// non-empty `access_token`/`refresh_token`/`scope`, the scope must contain
/// the direct-token grant, `expires_in` must be a positive finite number, and
/// (login only) a non-empty `id_token` must be present — its contents are
/// never parsed for profile data. The stored expiry is the real service
/// expiry; the single near-expiry correction is
/// [`OAuthCredential::needs_refresh`](crate::auth::OAuthCredential::needs_refresh).
fn chatgpt_credential_from(
    token: ChatGptTokenResponse,
    client_id: &str,
    require_id_token: bool,
) -> Result<OAuthCredential, ProviderError> {
    let invalid =
        |field: &str| provider_config_error(format!("token response has invalid {field}"));
    let access = token
        .access_token
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| invalid("access_token"))?;
    let refresh = token
        .refresh_token
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| invalid("refresh_token"))?;
    let scope = token
        .scope
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| invalid("scope"))?;
    let scopes: Vec<&str> = scope.split_whitespace().collect();
    if !scopes.contains(&CHATGPT_DIRECT_TOKEN_SCOPE) {
        return Err(provider_config_error(format!(
            "token grant did not include {CHATGPT_DIRECT_TOKEN_SCOPE}"
        )));
    }
    let expires_in = token
        .expires_in
        .filter(|secs| secs.is_finite() && *secs > 0.0)
        .ok_or_else(|| invalid("expires_in"))?;
    if require_id_token
        && !token
            .id_token
            .as_deref()
            .is_some_and(|id| !id.trim().is_empty())
    {
        return Err(provider_config_error(
            "token response did not contain an id_token",
        ));
    }
    let expires_at = OffsetDateTime::now_utc()
        .checked_add(time::Duration::seconds(expires_in.max(1.0) as i64))
        .ok_or_else(|| invalid("expires_in"))?;
    Ok(OAuthCredential {
        access: SecretString::new(access.into_boxed_str()),
        refresh: SecretString::new(refresh.into_boxed_str()),
        expires_at: Some(expires_at),
        base_url: None,
        account_id: None,
        client_id: Some(client_id.to_owned()),
        enterprise_domain: None,
        available_model_ids: None,
    })
}

/// OpenAI identifies each installation ("agent host") by a stable URI such as
/// `urn:uuid:<uuid>`. A missing or malformed device id fails typed before any
/// bind or HTTP traffic.
pub(crate) fn agent_host_id(device_id: Option<&str>) -> Result<String, ProviderError> {
    let require = || {
        provider_config_error(
            "sign in with ChatGPT requires a device id (UUID) for this installation",
        )
    };
    let Some(id) = device_id else {
        return Err(require());
    };
    let bytes = id.as_bytes();
    if bytes.len() != 36
        || !matches!(bytes[8], b'-')
        || !matches!(bytes[13], b'-')
        || !matches!(bytes[18], b'-')
        || !matches!(bytes[23], b'-')
        || !bytes
            .iter()
            .enumerate()
            .all(|(i, b)| matches!(i, 8 | 13 | 18 | 23) || b.is_ascii_hexdigit())
    {
        return Err(require());
    }
    Ok(format!("urn:uuid:{}", id.to_ascii_lowercase()))
}

/// Parse and validate a callback target or a pasted redirect URL. Unlike the
/// fixed-client base parser, the input must additionally carry OpenAI's
/// issued `client_id`, a pasted input must be a full URL with exactly the
/// fixed redirect origin and path (never a bare code), and an `error`
/// parameter is a typed authorization failure. The error parameter value is
/// provider-controlled text and is never echoed.
fn chatgpt_parse_input(
    input: &str,
    expected_state: &str,
    kind: PkceInputKind,
) -> Result<AuthzInput, ParsedInputError> {
    let reason = |error: ProviderError, reason: &'static str| ParsedInputError { error, reason };
    let parse_error = |message: &str, kind: PkceInputKind| ParsedInputError {
        error: provider_config_error(message),
        reason: match kind {
            PkceInputKind::Callback => "callback parse error",
            PkceInputKind::Manual => "manual redirect error",
        },
    };
    let input = input.trim();
    let url = if matches!(kind, PkceInputKind::Callback) {
        let target = if input.starts_with('/') {
            format!("http://127.0.0.1{input}")
        } else {
            input.to_owned()
        };
        reqwest::Url::parse(&target).map_err(|_| parse_error("oauth redirect URL malformed", kind))
    } else {
        reqwest::Url::parse(input)
            .map_err(|_| parse_error("paste the full callback URL from the browser", kind))
    }?;
    if matches!(kind, PkceInputKind::Manual) {
        let expected = reqwest::Url::parse(CHATGPT_REDIRECT_URI).expect("static redirect URI");
        if url.origin() != expected.origin() || url.path() != expected.path() {
            return Err(parse_error(
                "the pasted callback URL must start with the fixed redirect URI",
                kind,
            ));
        }
    }
    let mut code = None;
    let mut state = None;
    let mut issued_client_id = None;
    for (name, value) in url.query_pairs() {
        match name.as_ref() {
            "error" if !value.is_empty() => {
                return Err(reason(
                    provider_auth_error("chatgpt authorization failed"),
                    "authorization denied",
                ));
            }
            "code" if code.is_none() => code = Some(value.into_owned()),
            "state" if state.is_none() => state = Some(value.into_owned()),
            "client_id" if issued_client_id.is_none() => {
                let trimmed = value.trim().to_owned();
                if !trimmed.is_empty() {
                    issued_client_id = Some(trimmed);
                }
            }
            _ => {}
        }
    }
    let code = code
        .filter(|c| !c.is_empty())
        .ok_or_else(|| parse_error("oauth redirect missing code", kind))?;
    let state = state
        .filter(|s| !s.is_empty())
        .ok_or_else(|| parse_error("oauth redirect missing state", kind))?;
    if state != expected_state {
        return Err(reason(
            PkceInputError::StateMismatch.provider_error(),
            "state mismatch",
        ));
    }
    let client_id = issued_client_id
        .ok_or_else(|| parse_error("oauth callback did not contain an issued client id", kind))?;
    Ok(AuthzInput {
        code,
        client_id: Some(client_id),
    })
}

/// Exchange the authorization code, verifier, and issued client id for
/// tokens against the flow's token endpoint (form-encoded, with the resource
/// parameter). Login requires the `id_token` presence check.
fn chatgpt_exchange<'x>(
    config: &'x PkceLoginConfig,
    input: &'x AuthzInput,
    _state: &'x str,
    redirect_uri: &'x str,
    verifier: &'x str,
    budget: FlowBudget,
) -> BoxAuthFuture<'x, Result<OAuthCredential, ProviderError>> {
    Box::pin(async move {
        let issued_client_id = input.client_id.as_deref().ok_or_else(|| {
            provider_config_error("oauth callback did not contain an issued client id")
        })?;
        let params = [
            ("grant_type", "authorization_code"),
            ("client_id", issued_client_id),
            ("code", input.code.as_str()),
            ("code_verifier", verifier),
            ("redirect_uri", redirect_uri),
            ("resource", CHATGPT_RESOURCE),
        ];
        let send = config.client.post(&config.token_url).form(&params).send();
        let resp = unwrap_response(budget.wait(send).await, "token exchange failed").await?;
        let status = resp.status();
        if !status.is_success() {
            let body = budget.wait(resp.text()).await?.unwrap_or_default();
            return Err(token_endpoint_error(status, &body));
        }
        let raw: ChatGptTokenResponseJson = budget
            .wait(resp.json())
            .await?
            .map_err(|e| provider_config_error(format!("token response parse failed: {e}")))?;
        chatgpt_credential_from(ChatGptTokenResponse::parse(&raw), issued_client_id, true)
    })
}

/// The OpenAI ChatGPT direct OAuth provider.
///
/// `agent_name` overrides the `agent_name_hint` sent to the authorize
/// endpoint (default `"opi"`); `device_id` is the installation's stable UUID
/// (required: a missing or malformed id is a typed login failure).
#[derive(Clone)]
pub struct OpenAiChatGptOAuthProvider {
    authorize_url: String,
    token_url: String,
    client: reqwest::Client,
    timeout: Duration,
    agent_name: String,
    device_id: Option<String>,
}

impl OpenAiChatGptOAuthProvider {
    /// Construct with production endpoints, the default agent-name hint, and
    /// an injected HTTP client (redirects disabled by the caller's policy).
    pub fn with_http_client(
        client: reqwest::Client,
        agent_name: Option<String>,
        device_id: Option<String>,
    ) -> Self {
        Self {
            authorize_url: CHATGPT_AUTHORIZE_URL.to_owned(),
            token_url: CHATGPT_TOKEN_URL.to_owned(),
            client,
            timeout: Duration::from_secs(300),
            agent_name: agent_name.unwrap_or_else(|| CHATGPT_AGENT_NAME_HINT.to_owned()),
            device_id,
        }
    }

    /// Construct with production endpoints and a redirect-disabled client.
    pub fn production(agent_name: Option<String>, device_id: Option<String>) -> Self {
        Self::with_http_client(no_redirect_client(), agent_name, device_id)
    }

    /// Construct with configurable endpoints for offline wire tests.
    #[doc(hidden)]
    pub fn new_with_endpoints(
        authorize_url: String,
        token_url: String,
        agent_name: Option<String>,
        device_id: Option<String>,
        timeout: Duration,
    ) -> Self {
        Self {
            authorize_url,
            token_url,
            client: no_redirect_client(),
            timeout,
            agent_name: agent_name.unwrap_or_else(|| CHATGPT_AGENT_NAME_HINT.to_owned()),
            device_id,
        }
    }

    fn login_config(&self, host_id: String) -> PkceLoginConfig {
        // The nonce is an independent one-time value; it shares the state
        // generator (32 random bytes, base64url).
        let nonce = generate_state();
        PkceLoginConfig {
            provider_id: CHATGPT_PROVIDER_ID,
            authorize_url: self.authorize_url.clone(),
            token_url: self.token_url.clone(),
            client_id: CHATGPT_DYNAMIC_CLIENT_ID.to_owned(),
            authorize_params: vec![
                ("scope".to_owned(), CHATGPT_SCOPE.to_owned()),
                ("resource".to_owned(), CHATGPT_RESOURCE.to_owned()),
                ("agent_name_hint".to_owned(), self.agent_name.clone()),
                ("ext_agent_host_id".to_owned(), host_id),
                ("nonce".to_owned(), nonce),
            ],
            client: self.client.clone(),
            timeout: self.timeout,
            token_encoding: TokenRequestEncoding::Form,
            preferred_callback_port: Some(CHATGPT_CALLBACK_PORT),
            callback_path: CHATGPT_CALLBACK_PATH.to_owned(),
            redirect_host: "127.0.0.1".to_owned(),
            manual_only_redirect_uri: None,
            bind_fallback: CallbackBindFallback::Required,
            authorize_style: AuthorizeStyle::Standard,
            listening_progress: None,
        }
    }
}

impl OAuthProvider for OpenAiChatGptOAuthProvider {
    fn id(&self) -> &str {
        CHATGPT_PROVIDER_ID
    }

    fn login<'a>(
        &'a self,
        presenter: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async move {
            // A missing or malformed device id fails before any bind or HTTP
            // traffic.
            let host_id = agent_host_id(self.device_id.as_deref())?;
            let config = self.login_config(host_id);
            run_pkce_login_with(
                config,
                presenter,
                chatgpt_parse_input,
                &chatgpt_exchange,
                Ok,
                None,
                None,
            )
            .await
        })
    }

    fn refresh<'a>(
        &'a self,
        cred: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        let token_url = self.token_url.clone();
        let client = self.client.clone();
        Box::pin(async move {
            // Refresh must reuse the issued client id stored with the
            // credential; without it the only honest outcome is a typed
            // reconnect error.
            let client_id = cred
                .client_id
                .as_deref()
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .ok_or_else(|| {
                    provider_auth_error(
                        "stored chatgpt credential has no issued client id; sign in again",
                    )
                })?
                .to_owned();
            let budget = FlowBudget::new(Duration::from_secs(30))?;
            let params = [
                ("grant_type", "refresh_token"),
                ("client_id", client_id.as_str()),
                ("refresh_token", cred.refresh.expose_secret()),
                ("resource", CHATGPT_RESOURCE),
            ];
            let send = client.post(&token_url).form(&params).send();
            let resp = unwrap_response(budget.wait(send).await, "token refresh failed").await?;
            let status = resp.status();
            if !status.is_success() {
                let body = budget.wait(resp.text()).await?.unwrap_or_default();
                return Err(token_endpoint_error(status, &body));
            }
            let raw: ChatGptTokenResponseJson = budget
                .wait(resp.json())
                .await?
                .map_err(|e| provider_config_error(format!("token response parse failed: {e}")))?;
            // The id_token presence check is a login-only contract; refresh
            // re-validates everything else.
            chatgpt_credential_from(ChatGptTokenResponse::parse(&raw), &client_id, false)
        })
    }
}
