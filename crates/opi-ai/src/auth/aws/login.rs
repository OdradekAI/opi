//! Owned AWS login-session source family (task 20.19.3): the standard
//! login cache, the SignIn `CreateOAuth2Token` refresh, and an owned ES256
//! DPoP proof (RFC 9449).
//!
//! Fixed corpus contracts (`pi-1.3.0-rust-backends.md` §3.3,
//! `sdk-target-proposal.md` B-2/B-3):
//!
//! * The cache file name is the SHA-256 hex of the trimmed session name.
//! * Refresh: `POST /v1/token` JSON `{clientId, grantType:"refresh_token",
//!   refreshToken}` with an owned ES256 DPoP proof header (`typ:dpop+JWT`,
//!   `alg:ES256`, JWK `x`/`y`, `jti`, `iat`, `htu`, `htm:POST`).
//! * The response `accessToken` itself carries the three AWS credentials
//!   (`accessKeyId`/`secretAccessKey`/`sessionToken`); `expiresIn` sets the
//!   expiry; `refreshToken` may rotate. Account provenance stays frozen to
//!   the cache/session ARN and is never re-derived from the response.
//! * Failure classification: a missing login_session continues the chain; a
//!   selected session with a bad/missing cache, a DPoP failure, or the
//!   specific `AccessDeniedException` family terminates; other refresh
//!   failures keep a still-valid token and, once expired, raise the default
//!   continue-eligible error.
//! * A rotated refresh token is never lost: the in-memory cache is updated
//!   even when the cache-file write fails.
//!
//! opi never initiates a login and never signs SignIn requests with source
//! SigV4; every send runs on the injected transport seam.

use std::path::Path;

use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use p256::pkcs8::DecodePrivateKey;
use secrecy::{ExposeSecret, SecretString};
use serde_json::Value;
use time::{Duration, OffsetDateTime};

use crate::auth::AwsSigV4Credentials;
use crate::auth::aws::cache::{
    AwsSourceCache, CacheIdentity, LoginTokenSnapshot, OwnedRefreshError, REFRESH_WINDOW,
    SourceCacheValue,
};
use crate::auth::aws::endpoint::signin_endpoint_url;
use crate::auth::aws::local::{
    AwsLocalChainInput, CredentialResolutionError, CredentialSource, TerminalReason,
};
use crate::auth::aws::source_http::{AwsSourceTransport, SourceHttpMethod, SourceHttpRequest};

/// Standard login cache file name: SHA-256 hex of the trimmed session name.
pub fn login_cache_file_name(session: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(session.trim().as_bytes());
    hex::encode(digest)
}

/// Typed login refresh failures (secret-free).
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum LoginRefreshError {
    /// Transport-level failure.
    #[error("the SignIn exchange failed at the transport level")]
    Transport,
    /// The service denied the exchange with a code in the terminal
    /// AccessDenied family.
    #[error("SignIn denied the refresh (status {}, code {})",
        status.map(|s| s.to_string()).unwrap_or_else(|| "unknown".into()),
        code.as_deref().unwrap_or("unknown"))]
    Denied {
        /// HTTP status code when observed.
        status: Option<u16>,
        /// Error code when present.
        code: Option<String>,
    },
    /// A 2xx response did not satisfy the fixed shape.
    #[error("the SignIn response was invalid")]
    InvalidResponse,
}

/// The specific terminal AccessDenied family from the fixed chain rules.
const TERMINAL_LOGIN_CODES: [&str; 4] = [
    "AccessDeniedException",
    "TokenExpired",
    "UserCredentialsChanged",
    "InsufficientPermissions",
];

/// Read the standard login cache file. `Ok(None)` when absent;
/// `Err(())` when present but unusable (terminal for a selected session).
#[allow(clippy::result_unit_err)]
pub fn read_login_cache_file(dir: &Path, session: &str) -> Result<Option<LoginTokenSnapshot>, ()> {
    let path = dir.join(format!("{}.json", login_cache_file_name(session)));
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return Ok(None);
    };
    let value: Value = serde_json::from_str(&contents).map_err(|_| ())?;
    let client_id = value
        .get("clientId")
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .ok_or(())?
        .to_owned();
    let refresh_token = value
        .get("refreshToken")
        .and_then(Value::as_str)
        .filter(|token| !token.trim().is_empty())
        .ok_or(())?
        .to_owned();
    let signing_key = value
        .get("privateKey")
        .and_then(Value::as_str)
        .filter(|key| !key.trim().is_empty())
        .ok_or(())?
        .to_owned();
    let account = value
        .get("account")
        .and_then(Value::as_str)
        .filter(|account| !account.trim().is_empty())
        .map(str::to_owned);
    let access_token = value
        .get("accessToken")
        .and_then(Value::as_str)
        .filter(|token| !token.trim().is_empty())
        .map(SecretString::from);
    let expires_at = value
        .get("expiresAt")
        .and_then(Value::as_str)
        .and_then(|raw| {
            OffsetDateTime::parse(raw, &time::format_description::well_known::Rfc3339).ok()
        });
    Ok(Some(LoginTokenSnapshot {
        client_id,
        refresh_token: SecretString::from(refresh_token),
        signing_key: SecretString::from(signing_key),
        account,
        access_token,
        expires_at,
    }))
}

/// Write the standard login cache file (rotated refresh token, current
/// access token). The in-memory cache keeps the rotated token even when
/// this write fails, so the unique refresh token is never lost.
pub fn write_login_cache_file(
    dir: &Path,
    session: &str,
    snapshot: &LoginTokenSnapshot,
) -> std::io::Result<()> {
    let expires_at = snapshot
        .expires_at
        .map(|at| {
            at.format(&time::format_description::well_known::Rfc3339)
                .map_err(std::io::Error::other)
        })
        .transpose()?;
    let mut value = serde_json::json!({
        "clientId": snapshot.client_id,
        "refreshToken": snapshot.refresh_token.expose_secret(),
        "privateKey": snapshot.signing_key.expose_secret(),
    });
    if let Some(account) = &snapshot.account {
        value["account"] = Value::String(account.clone());
    }
    if let Some(token) = &snapshot.access_token {
        value["accessToken"] = Value::String(token.expose_secret().to_owned());
    }
    if let Some(expires_at) = expires_at {
        value["expiresAt"] = Value::String(expires_at);
    }
    let path = dir.join(format!("{}.json", login_cache_file_name(session)));
    std::fs::write(
        path,
        serde_json::to_vec(&value).map_err(std::io::Error::other)?,
    )
}

/// Base64url without padding (RFC 9449 encoding for JWK coordinates, JWT
/// segments, and signatures).
pub fn base64url_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(triple >> 18) as usize & 63] as char);
        out.push(ALPHABET[(triple >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[(triple >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[triple as usize & 63] as char);
        }
    }
    out
}

/// One DPoP proof JWT signed with ES256 (deterministic RFC 6979 signature;
/// `jti` from the injected source, `iat` from the injected clock).
pub fn build_dpop_proof(
    signing_key: &SigningKey,
    jti: &str,
    iat_unix: i64,
    htu: &str,
) -> Option<String> {
    let encoded_point = signing_key.verifying_key().to_encoded_point(false);
    let coordinates = encoded_point.as_bytes();
    if coordinates.len() != 65 {
        return None;
    }
    let (x, y) = coordinates.split_at(33);
    let header = serde_json::json!({
        "typ": "dpop+JWT",
        "alg": "ES256",
        "jwk": {
            "kty": "EC",
            "crv": "P-256",
            "x": base64url_encode(&x[1..]),
            "y": base64url_encode(y),
        },
    });
    let payload = serde_json::json!({
        "jti": jti,
        "iat": iat_unix,
        "htu": htu,
        "htm": "POST",
    });
    let header_b64 = base64url_encode(header.to_string().as_bytes());
    let payload_b64 = base64url_encode(payload.to_string().as_bytes());
    let signing_input = format!("{header_b64}.{payload_b64}");
    let signature: Signature = signing_key.sign(signing_input.as_bytes());
    Some(format!(
        "{signing_input}.{}",
        base64url_encode(&signature.to_bytes())
    ))
}

/// Parse the DPoP signing key from the cached SEC1 (or PKCS#8) PEM.
/// `pkcs8::DecodePrivateKey` covers both PEM forms for p256 secret keys.
pub fn parse_signing_key(pem: &SecretString) -> Option<SigningKey> {
    let pem = pem.expose_secret();
    let secret = p256::SecretKey::from_pkcs8_pem(pem)
        .or_else(|_| p256::SecretKey::from_sec1_pem(pem))
        .ok()?;
    Some(SigningKey::from(&secret))
}

/// Credentials parsed out of a SignIn access token (the token itself is a
/// JSON document carrying the three AWS credential fields).
#[derive(Debug, Clone)]
pub struct LoginCredentials {
    /// Temporary access key id.
    pub access_key_id: SecretString,
    /// Temporary secret access key.
    pub secret_access_key: SecretString,
    /// Temporary session token.
    pub session_token: SecretString,
    /// Absolute expiry derived from `expiresIn`.
    pub expires_at: OffsetDateTime,
}

fn parse_access_token(
    token: &SecretString,
    expires_at: OffsetDateTime,
) -> Option<LoginCredentials> {
    let value: Value = serde_json::from_str(token.expose_secret()).ok()?;
    let access_key_id = value
        .get("accessKeyId")
        .and_then(Value::as_str)
        .filter(|key| !key.trim().is_empty())?
        .to_owned();
    let secret_access_key = value
        .get("secretAccessKey")
        .and_then(Value::as_str)
        .filter(|key| !key.trim().is_empty())?
        .to_owned();
    let session_token = value
        .get("sessionToken")
        .and_then(Value::as_str)
        .filter(|token| !token.trim().is_empty())?
        .to_owned();
    Some(LoginCredentials {
        access_key_id: SecretString::from(access_key_id),
        secret_access_key: SecretString::from(secret_access_key),
        session_token: SecretString::from(session_token),
        expires_at,
    })
}

/// One SignIn refresh exchange with an owned DPoP proof.
pub(crate) async fn signin_refresh(
    transport: &dyn AwsSourceTransport,
    endpoint: &str,
    client_id: &str,
    refresh_token: &SecretString,
    signing_key: &SigningKey,
    jti: &str,
    iat: i64,
) -> Result<(SecretString, Option<SecretString>, i64), LoginRefreshError> {
    let url = format!("{endpoint}/v1/token");
    // A DPoP construction failure is the corpus's terminal DPoP failure.
    let proof = build_dpop_proof(signing_key, jti, iat, &url).ok_or(LoginRefreshError::Denied {
        status: None,
        code: Some("DPoP proof construction failed".to_owned()),
    })?;
    let body = serde_json::json!({
        "clientId": client_id,
        "grantType": "refresh_token",
        "refreshToken": refresh_token.expose_secret(),
    });
    let request = SourceHttpRequest {
        method: SourceHttpMethod::Post,
        url: url.clone(),
        headers: vec![
            ("content-type".to_owned(), "application/json".to_owned()),
            ("accept".to_owned(), "application/json".to_owned()),
            ("dpop".to_owned(), proof),
        ],
        body: Some(serde_json::to_vec(&body).expect("serializable refresh body")),
        timeout: Some(std::time::Duration::from_secs(1)),
    };
    let response = transport
        .send(request)
        .await
        .map_err(|_| LoginRefreshError::Transport)?;
    if response.status != 200 {
        let value = serde_json::from_slice::<Value>(&response.body).ok();
        // The corpus classifies by the standard code names; a body without
        // a code is not the terminal family.
        let code = value
            .as_ref()
            .and_then(|value| {
                value
                    .get("code")
                    .or_else(|| value.get("__type"))
                    .or_else(|| value.get("message"))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .filter(|code| {
                TERMINAL_LOGIN_CODES
                    .iter()
                    .any(|known| code.contains(known))
            });
        return Err(LoginRefreshError::Denied {
            status: Some(response.status),
            code,
        });
    }
    let value: Value =
        serde_json::from_slice(&response.body).map_err(|_| LoginRefreshError::InvalidResponse)?;
    let access_token = value
        .get("accessToken")
        .and_then(Value::as_str)
        .filter(|token| !token.trim().is_empty())
        .ok_or(LoginRefreshError::InvalidResponse)?
        .to_owned();
    let expires_in = value
        .get("expiresIn")
        .and_then(Value::as_i64)
        .ok_or(LoginRefreshError::InvalidResponse)?;
    let rotated_refresh = value
        .get("refreshToken")
        .and_then(Value::as_str)
        .filter(|token| !token.trim().is_empty())
        .map(SecretString::from);
    Ok((
        SecretString::from(access_token),
        rotated_refresh,
        expires_in,
    ))
}

/// Resolve one selected login session. `Ok(None)` means the chain
/// continues (missing session or expired-token refresh failure); terminal
/// failures return `Err`.
pub async fn resolve_login_session(
    input: &AwsLocalChainInput<'_>,
    session: &str,
    profile: &str,
    effective_region: &str,
) -> Result<Option<AwsSigV4Credentials>, CredentialResolutionError> {
    let session = session.trim();
    let source = CredentialSource::LoginSession {
        session: session.to_owned(),
    };
    let terminal = |reason: TerminalReason| CredentialResolutionError::Terminal {
        credential_source: source.clone(),
        reason,
    };
    let cache: &AwsSourceCache = input
        .source_cache
        .expect("login_session requires a source cache");
    let identity = CacheIdentity::LoginSession {
        session: session.to_owned(),
    };
    let now = input.clock.now();

    // Fresh or near-expiry in-memory hit.
    if let Some((SourceCacheValue::LoginSession(snapshot), revision)) =
        cache.peek_versioned(&identity, now)
    {
        return login_from_snapshot(
            input,
            (snapshot, revision),
            session,
            profile,
            effective_region,
            now,
            &terminal,
        )
        .await;
    }

    let guard = cache.refresh_lock(&identity).await;
    let now = input.clock.now();
    if let Some((SourceCacheValue::LoginSession(snapshot), revision)) =
        cache.peek_versioned(&identity, now)
    {
        drop(guard);
        return login_from_snapshot(
            input,
            (snapshot, revision),
            session,
            profile,
            effective_region,
            now,
            &terminal,
        )
        .await;
    }

    let (snapshot, revision) = match cache.source_snapshot(&identity) {
        Some((SourceCacheValue::LoginSession(snapshot), revision)) => (snapshot, revision),
        _ => {
            let Some(cache_dir) = input.login_cache_dir else {
                return Err(terminal(TerminalReason::LoginCacheInvalid));
            };
            let snapshot = match read_login_cache_file(cache_dir, session) {
                Ok(Some(snapshot)) => snapshot,
                _ => return Err(terminal(TerminalReason::LoginCacheInvalid)),
            };
            let revision = cache.store(identity, SourceCacheValue::LoginSession(snapshot.clone()));
            (snapshot, revision)
        }
    };
    drop(guard);
    login_from_snapshot(
        input,
        (snapshot, revision),
        session,
        profile,
        effective_region,
        now,
        &terminal,
    )
    .await
}

/// Use (and if needed refresh) one login snapshot. Shared by the in-memory
/// hit path and the file-read path.
async fn login_from_snapshot(
    input: &AwsLocalChainInput<'_>,
    snapshot: (LoginTokenSnapshot, u64),
    session: &str,
    profile: &str,
    effective_region: &str,
    now: OffsetDateTime,
    terminal: &(dyn Fn(TerminalReason) -> CredentialResolutionError + Send + Sync),
) -> Result<Option<AwsSigV4Credentials>, CredentialResolutionError> {
    let _ = profile;
    let (snapshot, revision) = snapshot;
    // A still-valid cached access token is used directly (a passive
    // refresh is registered when it is inside the window).
    if let Some(token) = &snapshot.access_token {
        if let Some(expires_at) = snapshot.expires_at {
            let remaining = expires_at - now;
            if remaining > Duration::ZERO {
                if remaining <= REFRESH_WINDOW {
                    register_login_passive(
                        input,
                        session,
                        &snapshot,
                        revision,
                        effective_region,
                        now,
                    );
                }
                if let Some(credentials) = parse_access_token(token, expires_at) {
                    return Ok(Some(login_credentials(
                        input,
                        &snapshot,
                        credentials,
                        effective_region,
                    )));
                }
                return Err(terminal(TerminalReason::InvalidLoginResponse));
            }
        } else {
            // No expiry recorded: parse without passive refresh (values
            // without an expiry never refresh passively).
            if let Some(credentials) = parse_access_token(token, now) {
                return Ok(Some(login_credentials(
                    input,
                    &snapshot,
                    credentials,
                    effective_region,
                )));
            }
            return Err(terminal(TerminalReason::InvalidLoginResponse));
        }
    }

    let cache = input.source_cache.expect("login requires a source cache");
    let transport = cache
        .passive_transport()
        .ok_or_else(|| terminal(TerminalReason::SourceRefreshOwnerUnavailable))?;
    let clock = cache
        .passive_clock()
        .ok_or_else(|| terminal(TerminalReason::SourceRefreshOwnerUnavailable))?;
    let jti = cache
        .passive_jti()
        .ok_or_else(|| terminal(TerminalReason::SourceRefreshOwnerUnavailable))?;
    let identity = CacheIdentity::LoginSession {
        session: session.to_owned(),
    };
    let generation = cache.generation(&identity);
    let endpoint = signin_endpoint_url(effective_region, input.signin_endpoint).to_owned();
    let cache_dir = input.login_cache_dir.map(Path::to_path_buf);
    let session = session.to_owned();
    let publisher = cache.clone();
    let refresh_identity = identity.clone();
    let result = cache
        .run_owned_refresh(identity, move || async move {
            let terminal = |reason| CredentialResolutionError::Terminal {
                credential_source: CredentialSource::LoginSession {
                    session: session.clone(),
                },
                reason,
            };
            if publisher.generation(&refresh_identity) != generation {
                return Err(terminal(TerminalReason::SourceRefreshSuperseded));
            }
            let Some((SourceCacheValue::LoginSession(current), revision)) =
                publisher.source_snapshot(&refresh_identity)
            else {
                return Err(terminal(TerminalReason::SourceRefreshSuperseded));
            };
            let now = clock.now();
            if current.access_token.is_some()
                && current.expires_at.is_none_or(|expiry| expiry > now)
            {
                return Ok((current, crate::mutation::MutationApplied::Keep));
            }
            let signing_key = parse_signing_key(&current.signing_key)
                .ok_or_else(|| terminal(TerminalReason::DpopFailed))?;
            let refreshed = signin_refresh(
                transport.as_ref(),
                &endpoint,
                &current.client_id,
                &current.refresh_token,
                &signing_key,
                &jti.next_jti(),
                now.unix_timestamp(),
            )
            .await;
            let (access_token, rotated_refresh, expires_in) = match refreshed {
                Ok(value) => value,
                Err(LoginRefreshError::Denied {
                    status,
                    code: Some(code),
                }) if TERMINAL_LOGIN_CODES
                    .iter()
                    .any(|known| code.contains(known)) =>
                {
                    return Err(terminal(TerminalReason::LoginDenied {
                        status,
                        code: Some(code),
                    }));
                }
                Err(_) => return Err(CredentialResolutionError::Exhausted),
            };
            let expires_at = now
                .checked_add(Duration::seconds(expires_in))
                .ok_or_else(|| terminal(TerminalReason::InvalidLoginResponse))?;
            if parse_access_token(&access_token, expires_at).is_none() {
                return Err(terminal(TerminalReason::InvalidLoginResponse));
            }
            let mut refreshed = current;
            refreshed.access_token = Some(access_token);
            refreshed.expires_at = Some(expires_at);
            if let Some(rotated) = rotated_refresh {
                refreshed.refresh_token = rotated;
            }
            let persisted = refreshed.clone();
            publisher
                .commit_source_rotation(
                    &refresh_identity,
                    revision,
                    SourceCacheValue::LoginSession(refreshed.clone()),
                    || {
                        if let Some(dir) = &cache_dir {
                            let _ = write_login_cache_file(dir, &session, &persisted);
                        }
                    },
                )
                .ok_or_else(|| terminal(TerminalReason::SourceRefreshSuperseded))?;
            Ok((refreshed, crate::mutation::MutationApplied::Write))
        })
        .await;
    let refreshed = match result {
        Ok(snapshot) => snapshot,
        Err(OwnedRefreshError::Operation(CredentialResolutionError::Exhausted)) => return Ok(None),
        Err(OwnedRefreshError::Operation(error)) => return Err(error),
        Err(OwnedRefreshError::Unavailable) => {
            return Err(terminal(TerminalReason::SourceRefreshOwnerUnavailable));
        }
        Err(OwnedRefreshError::Cancelled) => {
            return Err(terminal(TerminalReason::SourceRefreshCancelled));
        }
        Err(OwnedRefreshError::Unknown) => {
            return Err(terminal(TerminalReason::SourceRefreshOutcomeUnknown));
        }
    };
    let access = refreshed
        .access_token
        .as_ref()
        .ok_or_else(|| terminal(TerminalReason::InvalidLoginResponse))?;
    if refreshed
        .expires_at
        .is_some_and(|expires_at| expires_at <= input.clock.now())
    {
        return Err(terminal(TerminalReason::InvalidLoginResponse));
    }
    let credentials = parse_access_token(access, refreshed.expires_at.unwrap_or(now))
        .ok_or_else(|| terminal(TerminalReason::InvalidLoginResponse))?;
    Ok(Some(login_credentials(
        input,
        &refreshed,
        credentials,
        effective_region,
    )))
}

fn login_credentials(
    input: &AwsLocalChainInput<'_>,
    snapshot: &LoginTokenSnapshot,
    credentials: LoginCredentials,
    effective_region: &str,
) -> AwsSigV4Credentials {
    let bundle = AwsSigV4Credentials {
        access_key_id: credentials.access_key_id,
        secret_access_key: credentials.secret_access_key,
        session_token: Some(credentials.session_token),
        region: effective_region.to_owned(),
        expires_at: Some(credentials.expires_at),
    };
    if let Some(cache) = input.source_cache {
        cache.store(
            CacheIdentity::Temporary {
                source: "login_session".to_owned(),
                account: snapshot.account.clone(),
            },
            SourceCacheValue::Temporary(bundle.clone()),
        );
    }
    bundle
}

/// Register one passive near-expiry SignIn refresh with the mutation owner
/// when attached (throttled; failures settle into the queue receipt).
fn register_login_passive(
    input: &AwsLocalChainInput<'_>,
    session: &str,
    snapshot: &LoginTokenSnapshot,
    expected_revision: u64,
    effective_region: &str,
    now: OffsetDateTime,
) {
    let Some(cache) = input.source_cache else {
        return;
    };
    let Some(signing_key) = parse_signing_key(&snapshot.signing_key) else {
        return;
    };
    let transport = cache.passive_transport();
    let jti = cache.passive_jti();
    let clock = cache.passive_clock();
    let (Some(transport), Some(jti), Some(clock)) = (transport, jti, clock) else {
        return;
    };
    let endpoint = signin_endpoint_url(effective_region, input.signin_endpoint).to_owned();
    let session = session.to_owned();
    let snapshot = snapshot.clone();
    let cache_dir = input.login_cache_dir.map(std::path::Path::to_path_buf);
    let identity = CacheIdentity::LoginSession {
        session: session.clone(),
    };
    let refreshed_identity = identity.clone();
    let publisher = cache.clone();
    let slot = Box::new(move || {
        Box::pin(async move {
            let jti = jti.next_jti();
            let iat = clock.now().unix_timestamp();
            let result = signin_refresh(
                transport.as_ref(),
                &endpoint,
                &snapshot.client_id,
                &snapshot.refresh_token,
                &signing_key,
                &jti,
                iat,
            )
            .await;
            let (access_token, rotated_refresh, expires_in) = result.map_err(|_| {
                crate::mutation::MutationError::Operation(
                    crate::provider::ProviderError::AuthFailed(
                        crate::provider::ProviderErrorSummary::attested_static(
                            "AWS login token refresh failed",
                        ),
                    ),
                )
            })?;
            let now = clock.now();
            let expires_at = now
                .checked_add(Duration::seconds(expires_in))
                .ok_or_else(|| {
                    crate::mutation::MutationError::Operation(
                        crate::provider::ProviderError::AuthFailed(
                            crate::provider::ProviderErrorSummary::attested_static(
                                "AWS login token refresh returned an invalid expiry",
                            ),
                        ),
                    )
                })?;
            let mut refreshed = snapshot.clone();
            refreshed.access_token = Some(access_token);
            refreshed.expires_at = Some(expires_at);
            if let Some(rotated) = rotated_refresh {
                refreshed.refresh_token = rotated;
            }
            let persisted = refreshed.clone();
            publisher
                .commit_source_rotation(
                    &refreshed_identity,
                    expected_revision,
                    SourceCacheValue::LoginSession(refreshed),
                    || {
                        if let Some(dir) = &cache_dir {
                            let _ = write_login_cache_file(dir, &session, &persisted);
                        }
                    },
                )
                .ok_or_else(|| {
                    crate::mutation::MutationError::Operation(
                        crate::provider::ProviderError::AuthFailed(
                            crate::provider::ProviderErrorSummary::attested_static(
                                "AWS login token source changed during refresh",
                            ),
                        ),
                    )
                })?;
            Ok(crate::mutation::MutationApplied::Write)
        })
            as crate::credential::BoxAuthFuture<
                'static,
                Result<crate::mutation::MutationApplied, crate::mutation::MutationError>,
            >
    });
    cache.maybe_spawn_passive(identity, now, slot);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_cache_file_name_is_sha256_of_trimmed_session() {
        assert_eq!(login_cache_file_name(" session "), {
            use sha2::{Digest, Sha256};
            hex::encode(Sha256::digest(b"session"))
        });
    }

    #[test]
    fn base64url_encoding_matches_the_reference_alphabet() {
        assert_eq!(base64url_encode(b""), "");
        assert_eq!(base64url_encode(b"f"), "Zg");
        assert_eq!(base64url_encode(b"fo"), "Zm8");
        assert_eq!(base64url_encode(b"foo"), "Zm9v");
        assert_eq!(base64url_encode(&[251, 255, 190]), "-_--");
    }

    #[test]
    fn dpop_proof_carries_the_fixed_header_and_payload() {
        let secret = p256::SecretKey::random(&mut rand::rngs::OsRng);
        let signing_key = SigningKey::from(&secret);
        let proof =
            build_dpop_proof(&signing_key, "jti-1", 1_700_000_000, "https://x/v1/token").unwrap();
        let parts: Vec<&str> = proof.split('.').collect();
        assert_eq!(parts.len(), 3);
        let header: Value = decode_b64url_json(parts[0]);
        assert_eq!(header["typ"], "dpop+JWT");
        assert_eq!(header["alg"], "ES256");
        assert_eq!(header["jwk"]["kty"], "EC");
        assert_eq!(header["jwk"]["crv"], "P-256");
        let payload: Value = decode_b64url_json(parts[1]);
        assert_eq!(payload["jti"], "jti-1");
        assert_eq!(payload["iat"], 1_700_000_000);
        assert_eq!(payload["htu"], "https://x/v1/token");
        assert_eq!(payload["htm"], "POST");
        // The signature verifies against the embedded JWK.
        use p256::ecdsa::signature::Verifier;
        let x = decode_b64url(header["jwk"]["x"].as_str().unwrap());
        let y = decode_b64url(header["jwk"]["y"].as_str().unwrap());
        let mut sec1 = vec![0x04];
        sec1.extend(x);
        sec1.extend(y);
        use p256::elliptic_curve::sec1::FromEncodedPoint;
        let point = p256::EncodedPoint::from_bytes(&sec1).unwrap();
        let public = p256::PublicKey::from_encoded_point(&point).unwrap();
        let verifying = p256::ecdsa::VerifyingKey::from(&public);
        let signature_bytes = decode_b64url(parts[2]);
        let signature = Signature::from_slice(&signature_bytes).unwrap();
        verifying
            .verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &signature)
            .unwrap();
    }

    fn decode_b64url(value: &str) -> Vec<u8> {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut out = Vec::new();
        let mut buffer = 0u32;
        let mut bits = 0u32;
        for ch in value.bytes() {
            let index = ALPHABET.iter().position(|a| *a == ch).unwrap() as u32;
            buffer = (buffer << 6) | index;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((buffer >> bits) as u8);
            }
        }
        out
    }

    fn decode_b64url_json(value: &str) -> Value {
        serde_json::from_slice(&decode_b64url(value)).unwrap()
    }
}
