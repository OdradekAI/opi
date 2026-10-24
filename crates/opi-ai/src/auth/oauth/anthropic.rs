//! Anthropic OAuth flow (Claude Pro/Max), pinned to pi 1.3.0
//! `packages/ai/src/auth/oauth/anthropic.ts`.
//!
//! Wire contract: JSON token requests with a 30-second per-request timeout,
//! the fixed 53692 preferred callback port (`/callback` path, `localhost`
//! redirect host) falling back to a free loopback port and finally to
//! paste-back, a copy-code login method whose redirect URI is the hosted
//! `oauth/code/callback` page, and the exact production scopes. Expiry is
//! stored as the real service expiry; the single five-minute near-expiry
//! correction is [`OAuthCredential::needs_refresh`].

use std::time::Duration;

use crate::auth::oauth::{
    AuthorizeStyle, CallbackBindFallback, PkceLoginConfig, RefreshFieldPolicy,
    TokenRequestEncoding, code_challenge_s256, generate_code_verifier, generate_state,
    json_request_deadline, no_redirect_client, provider_config_error, provider_network_error,
    refresh_oauth_token, run_pkce_login, token_endpoint_error, url_encode,
};
use crate::auth::{
    BoxAuthFuture, LoginPresenter, OAuthCredential, OAuthLoginMethod, OAuthProvider,
};
use crate::provider::ProviderError;
use secrecy::SecretString;
use time::OffsetDateTime;

/// Production authorization endpoint (pi `AUTHORIZE_URL`).
pub const ANTHROPIC_AUTHORIZE_URL: &str = "https://claude.ai/oauth/authorize";
/// Production token endpoint (pi `TOKEN_URL`).
pub const ANTHROPIC_TOKEN_URL: &str = "https://platform.claude.com/v1/oauth/token";
/// Production OAuth client id (pi `CLIENT_ID`, base64-decoded).
pub const ANTHROPIC_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
/// Requested scopes (pi `SCOPES`).
pub const ANTHROPIC_SCOPES: &str = "org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers user:file_upload";
/// Preferred callback port so it can be forwarded into containers or over
/// SSH; Anthropic accepts any loopback port, so login falls back to a free
/// port when this one cannot be bound (pi #10571).
pub const ANTHROPIC_CALLBACK_PORT: u16 = 53692;
/// Callback path (pi `CALLBACK_PATH`).
pub const ANTHROPIC_CALLBACK_PATH: &str = "/callback";
/// Redirect host named in the redirect URI (pi binds `127.0.0.1` but names
/// `localhost`, per RFC 8252).
pub const ANTHROPIC_REDIRECT_HOST: &str = "localhost";
/// Fixed redirect URI when no callback server could be bound (manual-only).
pub const ANTHROPIC_MANUAL_REDIRECT_URI: &str = "http://localhost:53692/callback";
/// Copy-code redirect URI: the provider's hosted page shows the code
/// (pi `COPY_CODE_REDIRECT_URI`).
pub const ANTHROPIC_COPY_CODE_REDIRECT_URI: &str =
    "https://platform.claude.com/oauth/code/callback";

/// Anthropic OAuth provider using PKCE authorization-code with a loopback
/// callback (browser method) or a hosted copy-code page. Endpoints are
/// configurable so tests point them at a loopback server. The factory
/// supplies a lazy credential source; `AnthropicProvider` applies the OAuth
/// beta header for that auth scheme.
#[derive(Clone)]
pub struct AnthropicOAuthProvider {
    authorize_url: String,
    token_url: String,
    client_id: String,
    client: reqwest::Client,
    timeout: Duration,
}

impl AnthropicOAuthProvider {
    /// Construct with configurable endpoints and a login timeout. Builds a
    /// redirect-`none` HTTP client for the token exchange.
    pub fn new(
        authorize_url: String,
        token_url: String,
        client_id: String,
        timeout: Duration,
    ) -> Self {
        Self {
            authorize_url,
            token_url,
            client_id,
            client: no_redirect_client(),
            timeout,
        }
    }

    /// Construct with an injected HTTP client (redirects must be disabled
    /// by the caller — see [`no_redirect_client`]).
    pub fn with_http_client(
        authorize_url: String,
        token_url: String,
        client_id: String,
        timeout: Duration,
        client: reqwest::Client,
    ) -> Self {
        Self {
            authorize_url,
            token_url,
            client_id,
            client,
            timeout,
        }
    }

    /// Production endpoints and client id.
    pub fn production() -> Self {
        Self::new(
            ANTHROPIC_AUTHORIZE_URL.to_owned(),
            ANTHROPIC_TOKEN_URL.to_owned(),
            ANTHROPIC_CLIENT_ID.to_owned(),
            Duration::from_secs(300),
        )
    }

    fn browser_config(&self) -> PkceLoginConfig {
        PkceLoginConfig {
            provider_id: "anthropic",
            authorize_url: self.authorize_url.clone(),
            token_url: self.token_url.clone(),
            client_id: self.client_id.clone(),
            authorize_params: vec![
                ("code".into(), "true".into()),
                ("scope".into(), ANTHROPIC_SCOPES.into()),
            ],
            client: self.client.clone(),
            timeout: self.timeout,
            token_encoding: TokenRequestEncoding::Json,
            preferred_callback_port: Some(ANTHROPIC_CALLBACK_PORT),
            callback_path: ANTHROPIC_CALLBACK_PATH.to_owned(),
            redirect_host: ANTHROPIC_REDIRECT_HOST.to_owned(),
            manual_only_redirect_uri: Some(ANTHROPIC_MANUAL_REDIRECT_URI.to_owned()),
            bind_fallback: CallbackBindFallback::FreePort,
            authorize_style: AuthorizeStyle::Standard,
            listening_progress: None,
        }
    }
}

impl OAuthProvider for AnthropicOAuthProvider {
    fn id(&self) -> &str {
        "anthropic"
    }

    fn login<'a>(
        &'a self,
        presenter: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async move {
            let methods = [OAuthLoginMethod::Browser, OAuthLoginMethod::CopyCode];
            // Method selection races cancellation and the total budget: a
            // presenter that never answers cannot hang the login.
            let budget = crate::auth::oauth::FlowBudget::new(self.timeout)?;
            let mut cancellation = presenter.await_login_cancelled();
            let method = tokio::select! {
                biased;
                cancelled = cancellation.as_mut() => {
                    cancelled?;
                    presenter.notify_failure("login cancelled");
                    return Err(ProviderError::LoginCancelled {
                        provider_id: "anthropic".to_owned(),
                    });
                }
                method = presenter
                    .select_login_method("anthropic", &methods, OAuthLoginMethod::Browser) => method?,
                _ = budget.elapsed() => {
                    if let Err(error) = presenter.cancel_manual_code().await {
                        presenter.notify_failure("manual code cleanup failed");
                        return Err(error);
                    }
                    presenter.notify_failure("login method selection timed out");
                    return Err(ProviderError::Timeout);
                }
            };
            match method {
                OAuthLoginMethod::Browser => {
                    let config = self.browser_config();
                    // The same cancellation future threads through selection
                    // and the login race: a presenter that fired cancellation
                    // during selection must not have the wake lost when the
                    // runner creates a fresh waiter.
                    run_pkce_login(
                        config,
                        presenter,
                        accept_oauth_credential,
                        Some(cancellation),
                        Some(budget),
                    )
                    .await
                }
                OAuthLoginMethod::CopyCode => {
                    let config = self.browser_config();
                    run_copy_code_login(&config, presenter).await
                }
                OAuthLoginMethod::DeviceCode => Err(provider_config_error(
                    "OAuth provider 'anthropic' supplied an invalid login method",
                )),
            }
        })
    }

    fn refresh<'a>(
        &'a self,
        cred: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        let token_url = self.token_url.clone();
        let client_id = self.client_id.clone();
        let client = self.client.clone();
        Box::pin(async move {
            refresh_oauth_token(
                &client,
                &token_url,
                &client_id,
                cred,
                "anthropic",
                cred.base_url.clone(),
                TokenRequestEncoding::Json,
                RefreshFieldPolicy::Lenient,
            )
            .await
        })
    }
}

fn accept_oauth_credential(credential: OAuthCredential) -> Result<OAuthCredential, ProviderError> {
    Ok(credential)
}

/// Copy-code login (pi `loginAnthropicCopyCode`): authorize against the
/// hosted `oauth/code/callback` page, have the user paste the shown code
/// (optionally `code#state`), then exchange it with the fixed redirect URI.
async fn run_copy_code_login(
    config: &PkceLoginConfig,
    presenter: &dyn LoginPresenter,
) -> Result<OAuthCredential, ProviderError> {
    let verifier = generate_code_verifier();
    let challenge = code_challenge_s256(&verifier);
    let state = generate_state();
    let redirect_uri = ANTHROPIC_COPY_CODE_REDIRECT_URI.to_owned();

    let mut authorize_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&code_challenge={}&code_challenge_method=S256&state={}",
        config.authorize_url,
        url_encode(&config.client_id),
        url_encode(&redirect_uri),
        challenge,
        state,
    );
    for (name, value) in &config.authorize_params {
        authorize_url.push('&');
        authorize_url.push_str(name);
        authorize_url.push('=');
        authorize_url.push_str(&url_encode(value));
    }

    presenter.present_auth_url(&authorize_url).await?;

    let mut cancellation = presenter.await_login_cancelled();
    let pasted = tokio::select! {
        biased;
        cancelled = cancellation.as_mut() => {
            cancelled?;
            presenter.notify_failure("login cancelled");
            return Err(ProviderError::LoginCancelled { provider_id: config.provider_id.to_owned() });
        }
        res = presenter.await_manual_code() => match res {
            Ok(input) => input,
            Err(e) => {
                presenter.notify_failure("manual code error");
                return Err(e);
            }
        },
    };
    let (code, sent_state) = parse_copy_code_paste(&pasted, &state)?;
    exchange_copy_code(config, &code, &sent_state, &redirect_uri, &verifier)
        .await
        .inspect(|_| presenter.notify_success())
        .inspect_err(|_| presenter.notify_failure("token exchange failed"))
}

/// Parse a copy-code paste: a redirect URL, `code#state`, `code=...&state=`,
/// or a bare code. A missing state means the expected one (pi
/// `parsed.state ?? verifier`).
fn parse_copy_code_paste(
    input: &str,
    expected_state: &str,
) -> Result<(String, String), ProviderError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(provider_config_error("oauth redirect missing code"));
    }
    let is_url = trimmed
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
        || trimmed
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"));
    if is_url {
        let url = reqwest::Url::parse(trimmed)
            .map_err(|_| provider_config_error("oauth redirect URL malformed"))?;
        let query = url.query().unwrap_or("");
        return extract_code_state(query, expected_state, false);
    }
    if let Some((code, state)) = trimmed.split_once('#') {
        if state.is_empty() || code.is_empty() {
            return Err(provider_config_error("oauth redirect missing code"));
        }
        if state != expected_state {
            return Err(provider_config_error("oauth state mismatch"));
        }
        return Ok((code.to_owned(), state.to_owned()));
    }
    if trimmed.contains("code=") {
        return extract_code_state(trimmed, expected_state, false);
    }
    Ok((trimmed.to_owned(), expected_state.to_owned()))
}

fn extract_code_state(
    query: &str,
    expected_state: &str,
    require_state: bool,
) -> Result<(String, String), ProviderError> {
    let mut code = None;
    let mut state = None;
    for pair in query.split('&') {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        match name {
            "code" if code.is_some() => {
                return Err(provider_config_error("oauth redirect has duplicate code"));
            }
            "code" => code = Some(value),
            "state" if state.is_some() => {
                return Err(provider_config_error("oauth redirect has duplicate state"));
            }
            "state" => state = Some(value),
            _ => {}
        }
    }
    let code = code
        .filter(|value| !value.is_empty())
        .ok_or_else(|| provider_config_error("oauth redirect missing code"))?;
    let state = match state {
        Some(state) if !state.is_empty() => state.to_owned(),
        _ if require_state => return Err(provider_config_error("oauth redirect missing state")),
        _ => expected_state.to_owned(),
    };
    if state != expected_state {
        return Err(provider_config_error("oauth state mismatch"));
    }
    Ok((code.to_owned(), state))
}

/// JSON token exchange for the copy-code path (pi `exchangeAuthorizationCode`
/// shape: the state travels in the JSON body).
async fn exchange_copy_code(
    config: &PkceLoginConfig,
    code: &str,
    state: &str,
    redirect_uri: &str,
    verifier: &str,
) -> Result<OAuthCredential, ProviderError> {
    #[derive(serde::Deserialize)]
    struct TokenResponse {
        access_token: String,
        refresh_token: String,
        expires_in: i64,
    }
    let body = serde_json::json!({
        "grant_type": "authorization_code",
        "client_id": config.client_id,
        "code": code,
        "state": state,
        "redirect_uri": redirect_uri,
        "code_verifier": verifier,
    });
    let send = config
        .client
        .post(&config.token_url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send();
    let resp = json_request_deadline(send)
        .await
        .map_err(|_| ProviderError::Timeout)?
        .map_err(|_| provider_network_error("token exchange failed"))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(token_endpoint_error(status, &body));
    }
    let token: TokenResponse = resp
        .json()
        .await
        .map_err(|e| provider_config_error(format!("token response parse failed: {e}")))?;
    let expires_at = OffsetDateTime::now_utc()
        .checked_add(time::Duration::seconds(token.expires_in))
        .ok_or_else(|| provider_config_error("token response expires_in out of range"))?;
    Ok(OAuthCredential {
        access: SecretString::new(token.access_token.into_boxed_str()),
        refresh: SecretString::new(token.refresh_token.into_boxed_str()),
        expires_at: Some(expires_at),
        base_url: None,
        account_id: None,
        client_id: None,
        enterprise_domain: None,
        available_model_ids: None,
    })
}
