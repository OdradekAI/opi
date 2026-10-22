//! Owned AWS SSO source family (task 20.19.3): modern `sso_session` and
//! legacy inline profiles, the standard SSO token cache, modern OIDC token
//! refresh, and the GetRoleCredentials federation exchange.
//!
//! Wire contracts follow the fixed corpus (`pi-1.3.0-rust-backends.md` §3.3
//! and `sdk-target-proposal.md` B-1/B-2/B-3):
//!
//! * GetRoleCredentials: `GET /federation/credentials?role_name=…&account_id=…`
//!   with the `x-amz-sso_bearer_token` header; the response's
//!   `roleCredentials.expiration` is epoch **milliseconds**. Missing values,
//!   invalid payloads, and service denials are typed terminal failures, not
//!   "unconfigured".
//! * Modern session tokens refresh through SSO OIDC `POST /token`
//!   (`clientId`/`clientSecret`/`grantType=refresh_token`/`refreshToken`);
//!   OAuth errors (including slowdown/throttle) deny the attempt. A failed
//!   refresh with an unexpired token keeps the old token; a failed refresh
//!   of an expired token fails. A successful refresh writes the standard
//!   SSO cache file, and that write failure is ignored.
//! * Legacy tokens have no refresh path: expiry requires a fresh login.
//! * opi never performs an automatic login and never extends an expired
//!   token (no static-stability stretching for SSO credentials).
//!
//! All sends run on the injected [`AwsSourceTransport`] seam; cache-file
//! access happens only through injected directories (never ambient home
//! discovery inside this module).

use std::path::{Path, PathBuf};

use secrecy::{ExposeSecret, SecretString};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

use crate::auth::AwsSigV4Credentials;
use crate::auth::aws::cache::{
    AwsSourceCache, CacheIdentity, REFRESH_WINDOW, SourceCacheValue, SsoRefreshMaterial,
    SsoTokenSnapshot,
};
use crate::auth::aws::endpoint::{oidc_endpoint_url, sso_endpoint_url};
use crate::auth::aws::local::{
    AwsLocalChainInput, CredentialResolutionError, CredentialSource, TerminalReason,
};
use crate::auth::aws::source_http::{AwsSourceTransport, SourceHttpMethod, SourceHttpRequest};

/// Validated SSO selection (explicit init fields or a shared-config
/// profile plus its `sso-session` section).
#[derive(Debug, Clone)]
pub struct SsoProfileConfig {
    /// SSO start URL (from the session section or legacy inline property).
    pub start_url: String,
    /// SSO region (from the session section or legacy `sso_region`).
    pub region: String,
    /// Target account id.
    pub account_id: String,
    /// Target role name.
    pub role_name: String,
    /// Modern `sso_session` name when present.
    pub session: Option<String>,
}

impl SsoProfileConfig {
    /// Whether this selection is modern (session-based).
    pub fn modern(&self) -> bool {
        self.session.is_some()
    }

    /// Standard SSO cache key: the session name for modern profiles, the
    /// upper-cased start URL for legacy profiles. The file name is the
    /// SHA-1 hex of this key (standard cache-name compatibility).
    pub fn cache_key(&self) -> String {
        match &self.session {
            Some(session) => session.clone(),
            None => self.start_url.to_uppercase(),
        }
    }
}

/// SHA-1 hex of the standard SSO cache key (file-name compatibility hash
/// only; never used for authentication).
pub fn sso_cache_file_name(cache_key: &str) -> String {
    use sha1::{Digest, Sha1};
    hex::encode(Sha1::digest(cache_key.as_bytes()))
}

/// Result of reading the standard SSO token cache file for one key.
#[derive(Debug, Clone)]
pub struct SsoTokenFile {
    /// Parsed token snapshot.
    pub snapshot: SsoTokenSnapshot,
}

/// Read and parse the standard SSO token cache file for `cache_key`.
/// `Ok(None)` when the file is absent; `Err(())` when present but unusable.
#[allow(clippy::result_unit_err)]
pub fn read_sso_token_file(dir: &Path, cache_key: &str) -> Result<Option<SsoTokenFile>, ()> {
    let path: PathBuf = dir.join(format!("{}.json", sso_cache_file_name(cache_key)));
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return Ok(None);
    };
    let value: Value = serde_json::from_str(&contents).map_err(|_| ())?;
    let access_token = value
        .get("accessToken")
        .and_then(Value::as_str)
        .filter(|token| !token.trim().is_empty())
        .ok_or(())?
        .to_owned();
    let expires_at_raw = value
        .get("expiresAt")
        .and_then(Value::as_str)
        .ok_or(())?
        .to_owned();
    let expires_at = OffsetDateTime::parse(&expires_at_raw, &Rfc3339).map_err(|_| ())?;
    let refresh = match (
        value.get("clientId").and_then(Value::as_str),
        value
            .get("clientSecret")
            .and_then(Value::as_str)
            .filter(|secret| !secret.trim().is_empty()),
        value
            .get("refreshToken")
            .and_then(Value::as_str)
            .filter(|token| !token.trim().is_empty()),
    ) {
        (Some(client_id), Some(client_secret), Some(refresh_token)) => Some(SsoRefreshMaterial {
            client_id: client_id.to_owned(),
            client_secret: SecretString::from(client_secret),
            refresh_token: SecretString::from(refresh_token),
        }),
        _ => None,
    };
    let modern = refresh.is_some();
    Ok(Some(SsoTokenFile {
        snapshot: SsoTokenSnapshot {
            access_token: SecretString::from(access_token),
            expires_at,
            refresh,
            modern,
        },
    }))
}

/// Write the standard SSO cache file. The corpus pins that a failed
/// post-refresh write is ignored by the refresh itself; callers treat the
/// `Err` as ignorable.
pub fn write_sso_token_file(
    dir: &Path,
    cache_key: &str,
    snapshot: &SsoTokenSnapshot,
) -> std::io::Result<()> {
    let expires_at = snapshot
        .expires_at
        .format(&Rfc3339)
        .map_err(std::io::Error::other)?;
    let mut value = serde_json::json!({
        "accessToken": snapshot.access_token.expose_secret(),
        "expiresAt": expires_at,
    });
    if let Some(material) = &snapshot.refresh {
        value["clientId"] = Value::String(material.client_id.clone());
        value["clientSecret"] = Value::String(material.client_secret.expose_secret().to_owned());
        value["refreshToken"] = Value::String(material.refresh_token.expose_secret().to_owned());
    }
    let path = dir.join(format!("{}.json", sso_cache_file_name(cache_key)));
    std::fs::write(
        path,
        serde_json::to_vec(&value).map_err(std::io::Error::other)?,
    )
}

/// One modern SSO OIDC token refresh exchange (`POST /token`).
pub(crate) async fn sso_oidc_refresh(
    transport: &dyn AwsSourceTransport,
    endpoint: &str,
    material: &SsoRefreshMaterial,
    now: OffsetDateTime,
) -> Result<SsoTokenSnapshot, SsoExchangeError> {
    let body = serde_json::json!({
        "clientId": material.client_id,
        "clientSecret": material.client_secret.expose_secret(),
        "grantType": "refresh_token",
        "refreshToken": material.refresh_token.expose_secret(),
    });
    let request = SourceHttpRequest {
        method: SourceHttpMethod::Post,
        url: format!("{endpoint}/token"),
        headers: vec![
            ("content-type".to_owned(), "application/json".to_owned()),
            ("accept".to_owned(), "application/json".to_owned()),
        ],
        body: Some(serde_json::to_vec(&body).expect("serializable refresh body")),
        timeout: Some(std::time::Duration::from_secs(1)),
    };
    let response = transport
        .send(request)
        .await
        .map_err(|_| SsoExchangeError::Transport)?;
    if response.status != 200 {
        let code = serde_json::from_slice::<Value>(&response.body)
            .ok()
            .and_then(|value| {
                value
                    .get("error")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            });
        return Err(SsoExchangeError::Denied {
            status: Some(response.status),
            code,
        });
    }
    let value: Value =
        serde_json::from_slice(&response.body).map_err(|_| SsoExchangeError::InvalidResponse)?;
    let access_token = value
        .get("accessToken")
        .and_then(Value::as_str)
        .filter(|token| !token.trim().is_empty())
        .ok_or(SsoExchangeError::InvalidResponse)?
        .to_owned();
    let expires_in = value
        .get("expiresIn")
        .and_then(Value::as_i64)
        .ok_or(SsoExchangeError::InvalidResponse)?;
    let refresh_token = value
        .get("refreshToken")
        .and_then(Value::as_str)
        .filter(|token| !token.trim().is_empty());
    Ok(SsoTokenSnapshot {
        access_token: SecretString::from(access_token),
        expires_at: now + Duration::seconds(expires_in),
        refresh: Some(SsoRefreshMaterial {
            client_id: material.client_id.clone(),
            client_secret: material.client_secret.clone(),
            refresh_token: SecretString::from(
                refresh_token.unwrap_or_else(|| material.refresh_token.expose_secret()),
            ),
        }),
        modern: true,
    })
}

/// Typed SSO exchange failures (secret-free).
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum SsoExchangeError {
    /// Transport-level failure.
    #[error("the SSO exchange failed at the transport level")]
    Transport,
    /// The service denied the exchange; the standard error code is kept
    /// (OAuth `error` values including slowdown/throttle).
    #[error("SSO denied the exchange (status {}, code {})",
        status.map(|s| s.to_string()).unwrap_or_else(|| "unknown".into()),
        code.as_deref().unwrap_or("unknown"))]
    Denied {
        /// HTTP status code when observed.
        status: Option<u16>,
        /// Standard error code when present.
        code: Option<String>,
    },
    /// A 2xx response did not satisfy the fixed shape.
    #[error("the SSO response was invalid")]
    InvalidResponse,
}

/// Federation credentials parsed from a GetRoleCredentials response.
#[derive(Debug, Clone)]
pub struct SsoRoleCredentials {
    /// Temporary access key id.
    pub access_key_id: SecretString,
    /// Temporary secret access key.
    pub secret_access_key: SecretString,
    /// Temporary session token.
    pub session_token: SecretString,
    /// Absolute expiry reported by the service (epoch milliseconds input).
    pub expires_at: OffsetDateTime,
}

impl SsoRoleCredentials {
    /// Expiry in whole milliseconds for wire assertions.
    pub fn expires_at_unix_millis(&self) -> i64 {
        (self.expires_at.unix_timestamp_nanos() / 1_000_000) as i64
    }
}

/// One GetRoleCredentials exchange. Missing values, invalid payloads, and
/// non-200 statuses are typed failures (the caller classifies them
/// terminal per the corpus).
pub(crate) async fn sso_get_role_credentials(
    transport: &dyn AwsSourceTransport,
    endpoint: &str,
    access_token: &SecretString,
    account_id: &str,
    role_name: &str,
) -> Result<SsoRoleCredentials, SsoExchangeError> {
    let url = format!(
        "{endpoint}/federation/credentials?role_name={}&account_id={}",
        urlencode(role_name),
        urlencode(account_id),
    );
    let request = SourceHttpRequest {
        method: SourceHttpMethod::Get,
        url,
        headers: vec![(
            "x-amz-sso_bearer_token".to_owned(),
            access_token.expose_secret().to_owned(),
        )],
        body: None,
        timeout: Some(std::time::Duration::from_secs(1)),
    };
    let response = transport
        .send(request)
        .await
        .map_err(|_| SsoExchangeError::Transport)?;
    if response.status != 200 {
        return Err(SsoExchangeError::Denied {
            status: Some(response.status),
            code: None,
        });
    }
    let value: Value =
        serde_json::from_slice(&response.body).map_err(|_| SsoExchangeError::InvalidResponse)?;
    let credentials = value
        .get("roleCredentials")
        .ok_or(SsoExchangeError::InvalidResponse)?;
    let access_key_id = string_field(credentials, "accessKeyId")?;
    let secret_access_key = string_field(credentials, "secretAccessKey")?;
    let session_token = string_field(credentials, "sessionToken")?;
    let expiration_ms = credentials
        .get("expiration")
        .and_then(Value::as_i64)
        .ok_or(SsoExchangeError::InvalidResponse)?;
    let expires_at =
        epoch_millis_to_datetime(expiration_ms).ok_or(SsoExchangeError::InvalidResponse)?;
    Ok(SsoRoleCredentials {
        access_key_id: SecretString::from(access_key_id),
        secret_access_key: SecretString::from(secret_access_key),
        session_token: SecretString::from(session_token),
        expires_at,
    })
}

/// Resolve one validated SSO selection end to end: token cache (with the
/// fixed refresh windows), then GetRoleCredentials. Returns the resolved
/// SigV4 credentials plus the SSO region.
///
/// `profile` names the owning shared-config profile for provenance
/// (`None` for the explicit-init segment).
pub async fn resolve_sso_source(
    input: &AwsLocalChainInput<'_>,
    config: &SsoProfileConfig,
    profile: Option<&str>,
) -> Result<(AwsSigV4Credentials, String), CredentialResolutionError> {
    let cache: &AwsSourceCache = input.source_cache.expect("SSO requires a source cache");
    let now = input.clock.now();
    let cache_key = config.cache_key();
    let identity = CacheIdentity::SsoToken {
        cache_key: cache_key.clone(),
    };

    // Fresh hit (or near-expiry hit: the old token stays usable and a
    // single passive refresh may be registered).
    if let Some(SourceCacheValue::SsoToken(snapshot)) = cache.peek(&identity, now) {
        maybe_passive_refresh(input, &identity, &snapshot, now, config);
        return federation_credentials(input, config, &snapshot, profile).await;
    }

    // Active single-flight refresh.
    let guard = cache.refresh_lock(&identity).await;
    if let Some(SourceCacheValue::SsoToken(snapshot)) = cache.peek(&identity, now) {
        drop(guard);
        maybe_passive_refresh(input, &identity, &snapshot, now, config);
        return federation_credentials(input, config, &snapshot, profile).await;
    }
    let _ = guard;

    let Some(cache_dir) = input.sso_cache_dir else {
        return Err(CredentialResolutionError::Terminal {
            credential_source: CredentialSource::Sso {
                profile: profile.map(str::to_owned),
            },
            reason: TerminalReason::InvalidSsoToken,
        });
    };
    let file = match read_sso_token_file(cache_dir, &cache_key) {
        Ok(file) => file,
        Err(()) => {
            return Err(CredentialResolutionError::Terminal {
                credential_source: CredentialSource::Sso {
                    profile: profile.map(str::to_owned),
                },
                reason: TerminalReason::InvalidSsoToken,
            });
        }
    };
    let mut snapshot = match file {
        Some(file) => file.snapshot,
        None => {
            // No cached token: modern refresh material also lives in the
            // cache file, so there is nothing to refresh from.
            return Err(CredentialResolutionError::Terminal {
                credential_source: CredentialSource::Sso {
                    profile: profile.map(str::to_owned),
                },
                reason: TerminalReason::InvalidSsoToken,
            });
        }
    };
    snapshot.modern = snapshot.modern && config.modern();

    let remaining = snapshot.expires_at - now;
    if remaining > Duration::ZERO && remaining <= REFRESH_WINDOW {
        // Near expiry but still valid: return the old value and register a
        // single throttled passive refresh (the corpus pins no blocking
        // active refresh inside the window).
        maybe_passive_refresh(input, &identity, &snapshot, now, config);
        cache.store(identity, SourceCacheValue::SsoToken(snapshot.clone()));
        drop(guard);
        return federation_credentials(input, config, &snapshot, profile).await;
    }
    if remaining <= Duration::ZERO {
        // Expired: an active refresh is the only path (modern tokens only).
        if let Some(material) = snapshot.refresh.clone() {
            let endpoint = oidc_endpoint_url(&config.region, input.oidc_endpoint);
            match sso_oidc_refresh(input.transport, &endpoint, &material, now).await {
                Ok(refreshed) => {
                    // Standard cache write; the corpus pins that a failed
                    // write is ignored by the refresh itself.
                    if let Some(dir) = input.sso_cache_dir {
                        let _ = write_sso_token_file(dir, &cache_key, &refreshed);
                    }
                    snapshot = refreshed;
                }
                Err(SsoExchangeError::Denied { status, code }) if remaining <= Duration::ZERO => {
                    return Err(CredentialResolutionError::Terminal {
                        credential_source: CredentialSource::Sso {
                            profile: profile.map(str::to_owned),
                        },
                        reason: TerminalReason::SsoDenied {
                            status,
                            code,
                            request_id: None,
                        },
                    });
                }
                Err(SsoExchangeError::Transport) if remaining <= Duration::ZERO => {
                    return Err(CredentialResolutionError::Terminal {
                        credential_source: CredentialSource::Sso {
                            profile: profile.map(str::to_owned),
                        },
                        reason: TerminalReason::SsoTransport,
                    });
                }
                Err(SsoExchangeError::InvalidResponse) if remaining <= Duration::ZERO => {
                    return Err(CredentialResolutionError::Terminal {
                        credential_source: CredentialSource::Sso {
                            profile: profile.map(str::to_owned),
                        },
                        reason: TerminalReason::InvalidSsoResponse,
                    });
                }
                Err(_) => unreachable!("expired refresh failures return above"),
            }
        } else if remaining <= Duration::ZERO {
            // Legacy (or material-less) expired token: re-login required.
            return Err(CredentialResolutionError::Terminal {
                credential_source: CredentialSource::Sso {
                    profile: profile.map(str::to_owned),
                },
                reason: TerminalReason::InvalidSsoToken,
            });
        }
    }

    cache.store(identity, SourceCacheValue::SsoToken(snapshot.clone()));
    drop(guard);
    federation_credentials(input, config, &snapshot, profile).await
}

/// Register one passive near-expiry OIDC refresh with the mutation owner
/// when attached (throttled per session; failures settle into the queue
/// receipt and the old token stays usable).
fn maybe_passive_refresh(
    input: &AwsLocalChainInput<'_>,
    identity: &CacheIdentity,
    snapshot: &SsoTokenSnapshot,
    now: OffsetDateTime,
    config: &SsoProfileConfig,
) {
    let Some(material) = &snapshot.refresh else {
        return;
    };
    let remaining = snapshot.expires_at - now;
    if remaining <= Duration::ZERO || remaining > REFRESH_WINDOW {
        return;
    }
    let Some(cache) = input.source_cache else {
        return;
    };
    let endpoint = oidc_endpoint_url(&config.region, input.oidc_endpoint).to_owned();
    let cache_key = config.cache_key();
    let material = material.clone();
    let cache_dir = input.sso_cache_dir.map(Path::to_path_buf);
    let identity = identity.clone();
    let transport = cache.passive_transport();
    let clock = cache.passive_clock();
    let (Some(transport), Some(clock)) = (transport, clock) else {
        return;
    };
    let slot = Box::new(move || {
        Box::pin(async move {
            let refreshed =
                match sso_oidc_refresh(transport.as_ref(), &endpoint, &material, clock.now()).await
                {
                    Ok(refreshed) => refreshed,
                    Err(_) => return Ok(crate::mutation::MutationApplied::Keep),
                };
            if let Some(dir) = &cache_dir {
                let _ = write_sso_token_file(dir, &cache_key, &refreshed);
            }
            Ok(crate::mutation::MutationApplied::Write)
        })
            as crate::credential::BoxAuthFuture<
                'static,
                Result<crate::mutation::MutationApplied, crate::mutation::MutationError>,
            >
    });
    cache.maybe_spawn_passive(identity, now, slot);
}

/// Exchange the (valid) token for federation credentials and build the
/// resolved temporary credentials bundle.
async fn federation_credentials(
    input: &AwsLocalChainInput<'_>,
    config: &SsoProfileConfig,
    snapshot: &SsoTokenSnapshot,
    profile: Option<&str>,
) -> Result<(AwsSigV4Credentials, String), CredentialResolutionError> {
    let source = CredentialSource::Sso {
        profile: profile.map(str::to_owned),
    };
    let terminal = |reason: TerminalReason| CredentialResolutionError::Terminal {
        credential_source: source.clone(),
        reason,
    };
    // Fresh federation credentials from a prior exchange are reused; a
    // near-expiry bundle still in its validity is returned (the token
    // refresh above governs renewal).
    let temporary_identity = CacheIdentity::Temporary {
        source: "sso".to_owned(),
        account: Some(config.account_id.clone()),
    };
    let now = input.clock.now();
    if let Some(cache) = input.source_cache
        && let Some(SourceCacheValue::Temporary(credentials)) = cache.peek(&temporary_identity, now)
    {
        return Ok((credentials, config.region.clone()));
    }
    let endpoint = sso_endpoint_url(&config.region, input.sso_endpoint);
    match sso_get_role_credentials(
        input.transport,
        &endpoint,
        &snapshot.access_token,
        &config.account_id,
        &config.role_name,
    )
    .await
    {
        Ok(role) => {
            let credentials = AwsSigV4Credentials {
                access_key_id: role.access_key_id,
                secret_access_key: role.secret_access_key,
                session_token: Some(role.session_token),
                region: config.region.clone(),
                expires_at: Some(role.expires_at),
            };
            if let Some(cache) = input.source_cache {
                cache.store(
                    temporary_identity,
                    SourceCacheValue::Temporary(credentials.clone()),
                );
            }
            Ok((credentials, config.region.clone()))
        }
        Err(SsoExchangeError::Transport) => Err(terminal(TerminalReason::SsoTransport)),
        Err(SsoExchangeError::Denied { status, code }) => {
            Err(terminal(TerminalReason::SsoDenied {
                status,
                code,
                request_id: None,
            }))
        }
        Err(SsoExchangeError::InvalidResponse) => Err(terminal(TerminalReason::InvalidSsoResponse)),
    }
}

fn string_field(value: &Value, key: &str) -> Result<String, SsoExchangeError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|field| !field.trim().is_empty())
        .map(str::to_owned)
        .ok_or(SsoExchangeError::InvalidResponse)
}

fn epoch_millis_to_datetime(millis: i64) -> Option<OffsetDateTime> {
    let seconds = millis.div_euclid(1000);
    let nanos = millis.rem_euclid(1000) * 1_000_000;
    OffsetDateTime::from_unix_timestamp_nanos((seconds as i128) * 1_000_000_000 + nanos as i128)
        .ok()
}

/// Minimal percent-encoding for query components (unreserved set stays).
fn urlencode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_and_file_name_follow_the_standard_shapes() {
        let modern = SsoProfileConfig {
            start_url: "https://start.example".to_owned(),
            region: "us-east-1".to_owned(),
            account_id: "123".to_owned(),
            role_name: "r".to_owned(),
            session: Some("my-session".to_owned()),
        };
        assert_eq!(modern.cache_key(), "my-session");
        assert_eq!(sso_cache_file_name("my-session").len(), 40);

        let legacy = SsoProfileConfig {
            start_url: "https://start.example".to_owned(),
            region: "us-east-1".to_owned(),
            account_id: "123".to_owned(),
            role_name: "r".to_owned(),
            session: None,
        };
        assert_eq!(legacy.cache_key(), "HTTPS://START.EXAMPLE");
        assert_ne!(
            sso_cache_file_name(&legacy.cache_key()),
            sso_cache_file_name(&modern.cache_key())
        );
    }

    #[test]
    fn epoch_millis_conversion_keeps_millisecond_precision() {
        let datetime = epoch_millis_to_datetime(1_700_000_000_123).unwrap();
        assert_eq!(
            datetime.unix_timestamp_nanos() / 1_000_000,
            1_700_000_000_123
        );
        assert!(epoch_millis_to_datetime(i64::MAX).is_none());
    }

    #[test]
    fn token_file_roundtrip_keeps_refresh_material() {
        let dir = tempfile::tempdir().unwrap();
        let snapshot = SsoTokenSnapshot {
            access_token: SecretString::from("ACCESS"),
            expires_at: OffsetDateTime::parse("2030-01-01T00:00:00Z", &Rfc3339).unwrap(),
            refresh: Some(SsoRefreshMaterial {
                client_id: "client".to_owned(),
                client_secret: SecretString::from("secret"),
                refresh_token: SecretString::from("refresh"),
            }),
            modern: true,
        };
        write_sso_token_file(dir.path(), "KEY", &snapshot).unwrap();
        let read = read_sso_token_file(dir.path(), "KEY").unwrap().unwrap();
        assert!(read.snapshot.modern);
        assert!(read.snapshot.refresh.is_some());
        // Missing file is Ok(None); corrupt file is Err.
        assert!(read_sso_token_file(dir.path(), "OTHER").unwrap().is_none());
        std::fs::write(
            dir.path()
                .join(format!("{}.json", sso_cache_file_name("BAD"))),
            "{",
        )
        .unwrap();
        assert!(read_sso_token_file(dir.path(), "BAD").is_err());
    }

    #[test]
    fn urlencode_keeps_unreserved_and_encodes_the_rest() {
        assert_eq!(urlencode("aBc-_.~"), "aBc-_.~");
        assert_eq!(urlencode("role name/1"), "role%20name%2F1");
    }
}
