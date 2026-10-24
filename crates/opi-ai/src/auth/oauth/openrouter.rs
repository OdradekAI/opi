//! OpenRouter OAuth PKCE flow, pinned to pi 1.3.0
//! `packages/ai/src/auth/oauth/openrouter.ts` (sdk-contracts row 143).
//!
//! OpenRouter exchanges an authorization code for a permanent,
//! user-controlled API key rather than an expiring access/refresh pair. The
//! authorize URL carries `callback_url` + S256 PKCE but no `state` and no
//! `client_id`; the CSRF defense is a random per-login callback path that the
//! loopback handler validates (404 + keep waiting on any other route). The
//! callback is an ephemeral loopback port raced against a manual paste of the
//! redirect URL or the bare code. `refresh` is the identity — a permanent key
//! is never rotated, never re-exchanged, and the flow never issues the
//! personalization request (`/models/user` belongs to the explicit,
//! separately-selected catalog helper).

use std::time::Duration;

use crate::auth::oauth::{
    AuthorizeStyle, AuthzInput, CallbackBindFallback, ParsedInputError, PkceInputKind,
    PkceLoginConfig, TokenRequestEncoding, json_request_deadline, no_redirect_client,
    provider_config_error, provider_network_error, run_pkce_login_with, token_endpoint_error,
    unwrap_response,
};
use crate::auth::{BoxAuthFuture, LoginPresenter, OAuthCredential, OAuthProvider};
use crate::provider::ProviderError;
use secrecy::SecretString;
use time::OffsetDateTime;
use uuid::Uuid;

/// Production authorization endpoint (pi `AUTHORIZE_URL`).
pub const OPENROUTER_AUTHORIZE_URL: &str = "https://openrouter.ai/auth";
/// Production key-exchange endpoint (pi `TOKEN_URL`).
pub const OPENROUTER_TOKEN_URL: &str = "https://openrouter.ai/api/v1/auth/keys";
/// Total login budget (pi `LOGIN_TIMEOUT_MS`, five minutes). The key exchange
/// additionally carries pi's 30-second per-request JSON deadline
/// (`TOKEN_EXCHANGE_TIMEOUT_MS`) via the shared runner deadline helper.
pub const OPENROUTER_LOGIN_TIMEOUT: Duration = Duration::from_secs(300);
/// Per-login random callback route prefix (pi `/oauth/callback/` plus a
/// random UUID); the whole path is validated against every callback request.
pub const OPENROUTER_CALLBACK_PATH_PREFIX: &str = "/oauth/callback/";
/// Loopback host named in the callback URL (pi `getCallbackHost` default).
pub const OPENROUTER_REDIRECT_HOST: &str = "127.0.0.1";

/// Expiry recorded for a permanent OpenRouter key. pi stores
/// `Number.MAX_SAFE_INTEGER`; `OffsetDateTime` tops out at year 9999, so the
/// Rust encoding is the last representable instant — far outside every
/// actionable refresh window, and never refreshed ([`OAuthProvider::refresh`]
/// is the identity for this flow).
pub const OPENROUTER_PERMANENT_KEY_UNIX_TS: i64 = 253_402_300_799; // 9999-12-31T23:59:59Z

/// The permanent-key expiry sentinel.
pub fn openrouter_permanent_key_expiry() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(OPENROUTER_PERMANENT_KEY_UNIX_TS)
        .expect("9999-12-31T23:59:59Z is within the representable range")
}

/// Progress message template: `{}` becomes the bound redirect URI
/// (pi `Listening for OpenRouter OAuth callback on …`).
pub const OPENROUTER_LISTENING_PROGRESS: &str = "Listening for OpenRouter OAuth callback on {}";

/// OpenRouter OAuth provider. One flow: browser PKCE against
/// `openrouter.ai/auth` with a random per-login loopback callback path, raced
/// against pasting the redirect URL or bare code; the code exchanges for a
/// permanent API key. Endpoints are configurable so tests point them at a
/// loopback server; the callback host override pi reads from the environment
/// (`PI_OAUTH_CALLBACK_HOST`) is host policy expressed through construction.
#[derive(Clone)]
pub struct OpenRouterOAuthProvider {
    authorize_url: String,
    token_url: String,
    client: reqwest::Client,
    timeout: Duration,
}

impl OpenRouterOAuthProvider {
    /// Construct with configurable endpoints and a login timeout, using a
    /// redirect-`none` HTTP client for the key exchange.
    pub fn new(authorize_url: String, token_url: String, timeout: Duration) -> Self {
        Self {
            authorize_url,
            token_url,
            client: no_redirect_client(),
            timeout,
        }
    }

    /// Construct with an injected HTTP client (redirects must be disabled by
    /// the caller — see [`no_redirect_client`]).
    pub fn with_http_client(
        authorize_url: String,
        token_url: String,
        timeout: Duration,
        client: reqwest::Client,
    ) -> Self {
        Self {
            authorize_url,
            token_url,
            client,
            timeout,
        }
    }

    /// Production endpoints and budget.
    pub fn production() -> Self {
        Self::new(
            OPENROUTER_AUTHORIZE_URL.to_owned(),
            OPENROUTER_TOKEN_URL.to_owned(),
            OPENROUTER_LOGIN_TIMEOUT,
        )
    }

    /// Build the per-login PKCE config: the callback path is a fresh random
    /// UUID route each call (pi `crypto.randomUUID()`), the port is
    /// ephemeral, and the authorize URL takes the `callback_url` form with
    /// no `state`.
    fn login_config(&self) -> PkceLoginConfig {
        PkceLoginConfig {
            provider_id: "openrouter",
            authorize_url: self.authorize_url.clone(),
            token_url: self.token_url.clone(),
            client_id: String::new(),
            authorize_params: Vec::new(),
            client: self.client.clone(),
            timeout: self.timeout,
            token_encoding: TokenRequestEncoding::Json,
            preferred_callback_port: None,
            callback_path: format!("{OPENROUTER_CALLBACK_PATH_PREFIX}{}", Uuid::new_v4()),
            redirect_host: OPENROUTER_REDIRECT_HOST.to_owned(),
            manual_only_redirect_uri: None,
            bind_fallback: CallbackBindFallback::FreePort,
            authorize_style: AuthorizeStyle::CallbackUrlNoState,
            listening_progress: Some(OPENROUTER_LISTENING_PROGRESS),
        }
    }
}

impl OAuthProvider for OpenRouterOAuthProvider {
    fn id(&self) -> &str {
        "openrouter"
    }

    fn login<'a>(
        &'a self,
        presenter: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async move {
            let config = self.login_config();
            run_pkce_login_with(
                config,
                presenter,
                parse_openrouter_input,
                &exchange_authorization_code_for_key,
                accept_permanent_key_credential,
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
        // Identity refresh (pi `refresh`): a permanent key is never rotated,
        // never re-exchanged, and refresh performs no network I/O.
        let cred = cred.clone();
        Box::pin(async move { Ok(cred) })
    }
}

/// Flow input parser. OpenRouter sends no `state`, so the CSRF token is
/// ignored; a callback or manual input carrying the OAuth `error` parameter
/// is a typed denial, and the manual formats follow pi
/// `parseAuthorizationInput`: an absolute URL yields its `code` query
/// parameter, a `code=` fragment parses as a query string, and anything
/// else is the bare code itself.
fn parse_openrouter_input(
    input: &str,
    _expected_state: &str,
    kind: PkceInputKind,
) -> Result<AuthzInput, ParsedInputError> {
    let trimmed = input.trim();
    let query = callback_query(trimmed);
    // The OAuth `error` parameter is a typed denial on the callback arm (pi
    // `callback-server.ts` finishes with `authorization failed`); a manual
    // paste of an error URL keeps pi's fall-through to the missing-code
    // failure. The untrusted denial detail is never echoed.
    if kind == PkceInputKind::Callback
        && query.is_some_and(|q| first_query_value(q, "error").is_some())
    {
        return Err(ParsedInputError {
            error: provider_config_error("OpenRouter authorization failed"),
            reason: "authorization denied",
        });
    }
    if let Some(code) = query
        .and_then(|q| first_query_value(q, "code"))
        .filter(|code| !code.is_empty())
    {
        return Ok(AuthzInput {
            code,
            client_id: None,
        });
    }
    match kind {
        // A callback without a code is a malformed route hit: typed failure
        // (the standing fail-fast deviation; pi keeps waiting).
        PkceInputKind::Callback => Err(ParsedInputError {
            error: provider_config_error("OpenRouter authorization missing code"),
            reason: "callback parse error",
        }),
        PkceInputKind::Manual => parse_manual_paste(trimmed)
            .map(|code| AuthzInput {
                code,
                client_id: None,
            })
            .map_err(|error| ParsedInputError {
                error,
                reason: "manual redirect error",
            }),
    }
}

/// Extract the query string of a request target or absolute URL, if any.
fn callback_query(trimmed: &str) -> Option<&str> {
    if let Some((_, query)) = trimmed.split_once('?') {
        return Some(query);
    }
    None
}

/// First non-empty value of `name` in a `&`-separated query string, if any.
fn first_query_value(query: &str, name: &str) -> Option<String> {
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if key == name {
            return Some(value.to_owned());
        }
    }
    None
}

/// Manual paste semantics (pi `parseAuthorizationInput`): an absolute URL
/// must carry `?code=` (missing means failure, not fall-through), a `code=`
/// fragment parses as a query string, anything else is the bare code. Empty
/// input is a typed failure.
fn parse_manual_paste(trimmed: &str) -> Result<String, ProviderError> {
    if trimmed.is_empty() {
        return Err(provider_config_error(
            "OpenRouter missing authorization code",
        ));
    }
    if let Ok(url) = reqwest::Url::parse(trimmed) {
        return url
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, value)| value.into_owned())
            .filter(|code| !code.is_empty())
            .ok_or_else(|| provider_config_error("OpenRouter missing authorization code"));
    }
    if trimmed.contains("code=") {
        return first_query_value(trimmed, "code")
            .filter(|code| !code.is_empty())
            .ok_or_else(|| provider_config_error("OpenRouter missing authorization code"));
    }
    Ok(trimmed.to_owned())
}

/// Key exchange (pi `exchangeAuthorizationCode`): a JSON POST of the code,
/// the PKCE verifier and the fixed S256 method against the auth-keys
/// endpoint with a 30-second deadline. A non-ok status is the closed
/// typed token-endpoint error (the body detail pi appends is never echoed);
/// an ok response must be JSON carrying a non-empty `key`.
fn exchange_authorization_code_for_key<'x>(
    config: &'x PkceLoginConfig,
    input: &'x AuthzInput,
    _state: &'x str,
    _redirect_uri: &'x str,
    verifier: &'x str,
    _budget: crate::auth::oauth::FlowBudget,
) -> BoxAuthFuture<'x, Result<OAuthCredential, ProviderError>> {
    Box::pin(async move {
        let body = serde_json::json!({
            "code": input.code,
            "code_verifier": verifier,
            "code_challenge_method": "S256",
        });
        let send = config
            .client
            .post(&config.token_url)
            .header("accept", "application/json")
            .header("content-type", "application/json")
            .json(&body)
            .send();
        let resp =
            unwrap_response(json_request_deadline(send).await, "OpenRouter key exchange").await?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|_| provider_network_error("OpenRouter key exchange body read failed"))?;
        if !status.is_success() {
            return Err(token_endpoint_error(status, &text));
        }
        #[derive(serde::Deserialize)]
        struct KeyResponse {
            key: String,
        }
        let parsed: KeyResponse = serde_json::from_str(&text)
            .map_err(|_| provider_config_error("OpenRouter OAuth response is not valid JSON"))?;
        if parsed.key.is_empty() {
            return Err(provider_config_error(
                "OpenRouter OAuth response carries no \"key\"",
            ));
        }
        Ok(OAuthCredential {
            access: SecretString::new(parsed.key.into_boxed_str()),
            refresh: SecretString::new(String::new().into_boxed_str()),
            expires_at: Some(openrouter_permanent_key_expiry()),
            base_url: None,
            account_id: None,
            client_id: None,
            enterprise_domain: None,
            available_model_ids: None,
        })
    })
}

/// Finalize hook: the credential is complete as exchanged.
fn accept_permanent_key_credential(
    credential: OAuthCredential,
) -> Result<OAuthCredential, ProviderError> {
    Ok(credential)
}
