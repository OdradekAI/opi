//! Shared AWS SSO, login and refresh fixtures.

#![allow(unused_imports)]

use super::*;

/// Fixed clock for refresh-window tests.
pub(crate) struct FakeClock(pub(crate) time::OffsetDateTime);

impl SourceClock for FakeClock {
    fn now(&self) -> time::OffsetDateTime {
        self.0
    }
}

pub(crate) struct AdjustableSourceClock(pub(crate) std::sync::Mutex<time::OffsetDateTime>);

impl SourceClock for AdjustableSourceClock {
    fn now(&self) -> time::OffsetDateTime {
        *self.0.lock().unwrap()
    }
}

/// Deterministic DPoP `jti` source.
pub(crate) struct CountingJti(pub(crate) std::sync::atomic::AtomicUsize);

impl DpopJtiSource for CountingJti {
    fn next_jti(&self) -> String {
        let value = self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("jti-{value}")
    }
}

pub(crate) fn cache_with_owned_refresh(transport: Arc<dyn AwsSourceTransport>) -> AwsSourceCache {
    AwsSourceCache::with_mutation_queue(Arc::new(opi_ai::mutation::MutationQueue::new()))
        .with_passive_deps(Arc::new(PassiveRefreshDeps {
            transport,
            clock: Arc::new(FakeClock(rfc3339("2030-01-01T00:00:00Z"))),
            jti: Arc::new(CountingJti(AtomicUsize::new(0))),
        }))
}

pub(crate) fn modern_sso_token_file(dir: &std::path::Path, expires_at: &str) {
    write_sso_token_file(
        dir,
        "sess-a",
        &SsoTokenSnapshot {
            access_token: secrecy::SecretString::from("SSO_TOKEN"),
            expires_at: rfc3339(expires_at),
            refresh: Some(SsoRefreshMaterial {
                client_id: "oidc-client".to_owned(),
                client_secret: secrecy::SecretString::from("oidc-secret"),
                refresh_token: secrecy::SecretString::from("oidc-refresh"),
            }),
            modern: true,
        },
    )
    .unwrap();
}

pub(crate) fn sso_scoped(start_url: &str, session: Option<&str>) -> ScopedAwsEnv {
    ScopedAwsEnv {
        sso_start_url: Some(start_url.to_owned()),
        sso_account_id: Some("123456789012".to_owned()),
        sso_region: Some("us-east-1".to_owned()),
        sso_role_name: Some("Engineering".to_owned()),
        sso_session: session.map(str::to_owned),
        ..ScopedAwsEnv::empty()
    }
}

pub(crate) fn role_credentials_body(expiration_ms: i64) -> String {
    format!(
        r#"{{"roleCredentials":{{"accessKeyId":"SSO_AKID","secretAccessKey":"SSO_SAK","sessionToken":"SSO_SESSION","expiration":{expiration_ms}}}}}"#
    )
}

pub(crate) fn bearer_of(request: &SourceHttpRequest) -> String {
    request
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("x-amz-sso_bearer_token"))
        .map(|(_, value)| value.clone())
        .expect("bearer header present")
}

pub(crate) fn login_cache_key_pair() -> (p256::SecretKey, String) {
    let secret = p256::SecretKey::random(&mut rand::rngs::OsRng);
    use p256::elliptic_curve::pkcs8::EncodePrivateKey;
    let pem = secret
        .to_pkcs8_pem(p256::pkcs8::LineEnding::LF)
        .unwrap()
        .to_string();
    (secret, pem)
}

pub(crate) fn write_login_cache(
    dir: &std::path::Path,
    session: &str,
    pem: &str,
    access_token: Option<&str>,
    expires_at: Option<&str>,
) {
    write_login_cache_file(
        dir,
        session,
        &opi_ai::auth::aws::cache::LoginTokenSnapshot {
            client_id: "signin-client".to_owned(),
            refresh_token: secrecy::SecretString::from("signin-refresh"),
            signing_key: secrecy::SecretString::from(pem.to_owned()),
            account: Some("123456789012".to_owned()),
            access_token: access_token.map(secrecy::SecretString::from),
            expires_at: expires_at.map(rfc3339),
        },
    )
    .unwrap();
}

struct AcknowledgedExpiryClock {
    queue: Arc<opi_ai::mutation::MutationQueue>,
    advance_after_ack: bool,
}

impl SourceClock for AcknowledgedExpiryClock {
    fn now(&self) -> time::OffsetDateTime {
        let written = self.advance_after_ack && self.queue.barrier().report().captured.iter().any(|entry| {
            matches!(entry, opi_ai::mutation::BarrierEntry::Settled(receipt) if matches!(receipt.outcome, opi_ai::mutation::MutationOutcome::Written { .. }))
        });
        rfc3339(if written {
            "2030-01-01T00:00:02Z"
        } else {
            "2030-01-01T00:00:00Z"
        })
    }
}

struct ExpiryProbeTransport {
    body: Vec<u8>,
}

impl AwsSourceTransport for ExpiryProbeTransport {
    fn send(
        &self,
        _request: SourceHttpRequest,
    ) -> futures_util::future::BoxFuture<'_, Result<SourceHttpResponse, SourceHttpError>> {
        Box::pin(async move {
            Ok(SourceHttpResponse {
                status: 200,
                body: self.body.clone(),
            })
        })
    }
}

pub(crate) async fn source_refresh_expiry_probe(
    login: bool,
    ttl: i64,
    advance: bool,
) -> (Result<ResolvedAuth, CredentialResolutionError>, usize) {
    let dir = tempfile::tempdir().unwrap();
    let queue = Arc::new(opi_ai::mutation::MutationQueue::new());
    let clock = Arc::new(AcknowledgedExpiryClock {
        queue: queue.clone(),
        advance_after_ack: advance,
    });
    let access = if login {
        r#"{"accessKeyId":"LOGIN_AKID","secretAccessKey":"LOGIN_SAK","sessionToken":"LOGIN_SESSION"}"#
    } else {
        "SSO_TOKEN_2"
    };
    let owner = Arc::new(ExpiryProbeTransport { body: serde_json::to_vec(&serde_json::json!({"accessToken":access,"expiresIn":ttl,"refreshToken":"rotated-refresh"})).unwrap() });
    let cache = AwsSourceCache::with_mutation_queue(queue).with_passive_deps(Arc::new(
        PassiveRefreshDeps {
            transport: owner,
            clock: clock.clone(),
            jti: Arc::new(CountingJti(AtomicUsize::new(0))),
        },
    ));
    let config = dir.path().join("config");
    std::fs::write(&config, "[profile corp]\nlogin_session=my-session\n").unwrap();
    if login {
        let (_secret, pem) = login_cache_key_pair();
        cache.store(
            CacheIdentity::LoginSession {
                session: "my-session".to_owned(),
            },
            SourceCacheValue::LoginSession(opi_ai::auth::aws::cache::LoginTokenSnapshot {
                client_id: "signin-client".to_owned(),
                refresh_token: SecretString::from("signin-refresh"),
                signing_key: SecretString::from(pem),
                account: Some("123456789012".to_owned()),
                access_token: None,
                expires_at: None,
            }),
        );
    } else {
        cache.store(
            CacheIdentity::SsoToken {
                cache_key: "sess-a".to_owned(),
            },
            SourceCacheValue::SsoToken(SsoTokenSnapshot {
                access_token: SecretString::from("OLD"),
                expires_at: rfc3339("2029-12-31T23:00:00Z"),
                refresh: Some(SsoRefreshMaterial {
                    client_id: "oidc-client".to_owned(),
                    client_secret: SecretString::from("oidc-secret"),
                    refresh_token: SecretString::from("oidc-refresh"),
                }),
                modern: true,
            }),
        );
    }
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "true",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let federation = FakeSourceTransport::new(vec![source_response(
        200,
        role_credentials_body(1_900_000_000_000),
    )]);
    let selection = if login {
        AwsAuthSelection::Profile("corp".to_owned())
    } else {
        AwsAuthSelection::Auto
    };
    let scoped = if login {
        ScopedAwsEnv::empty()
    } else {
        sso_scoped("https://start.example", Some("sess-a"))
    };
    let input = chain_input(&ambient, None, Some(&config), &runner, selection, scoped)
        .with_source_transport(&federation)
        .with_source_cache(Some(&cache))
        .with_clock(clock.as_ref())
        .with_login_cache_dir(Some(dir.path()))
        .with_sso_cache_dir(Some(dir.path()));
    let result = resolve_local_aws_auth(&input).await;
    if advance {
        assert_eq!(clock.now(), rfc3339("2030-01-01T00:00:02Z"));
    }
    (result, federation.send_count())
}

pub(crate) struct GatedLoginRefreshTransport {
    pub(crate) entered: Notify,
    pub(crate) release: Notify,
    pub(crate) sends: AtomicUsize,
}

impl AwsSourceTransport for GatedLoginRefreshTransport {
    fn send(
        &self,
        _request: SourceHttpRequest,
    ) -> futures_util::future::BoxFuture<'_, Result<SourceHttpResponse, SourceHttpError>> {
        self.sends.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            self.entered.notify_one();
            self.release.notified().await;
            Ok(SourceHttpResponse {
                status: 200,
                body: serde_json::to_vec(&serde_json::json!({
                    "accessToken": r#"{"accessKeyId":"LOGIN_AKID","secretAccessKey":"LOGIN_SAK","sessionToken":"LOGIN_SESSION"}"#,
                    "expiresIn": 28800,
                    "refreshToken": "signin-refresh-2",
                }))
                .unwrap(),
            })
        })
    }
}

/// Checks the proof shape and verifies its ES256 signature against the embedded JWK.
pub(crate) fn assert_dpop_proof_shape(proof: &str, jti: &str, iat: i64, htu: &str) {
    use p256::ecdsa::signature::Verifier;
    let parts: Vec<&str> = proof.split('.').collect();
    assert_eq!(parts.len(), 3, "three-segment DPoP JWT");
    let decode = |value: &str| {
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
    };
    let header: serde_json::Value = serde_json::from_slice(&decode(parts[0])).unwrap();
    assert_eq!(header["typ"], "dpop+JWT");
    assert_eq!(header["alg"], "ES256");
    assert_eq!(header["jwk"]["kty"], "EC");
    assert_eq!(header["jwk"]["crv"], "P-256");
    let payload: serde_json::Value = serde_json::from_slice(&decode(parts[1])).unwrap();
    assert_eq!(payload["jti"], jti);
    assert_eq!(payload["iat"], iat);
    assert_eq!(payload["htu"], htu);
    assert_eq!(payload["htm"], "POST");
    use p256::elliptic_curve::sec1::FromEncodedPoint;
    let x = decode(header["jwk"]["x"].as_str().unwrap());
    let y = decode(header["jwk"]["y"].as_str().unwrap());
    let mut sec1 = vec![0x04];
    sec1.extend(x);
    sec1.extend(y);
    let point = p256::EncodedPoint::from_bytes(&sec1).unwrap();
    let public = p256::PublicKey::from_encoded_point(&point).unwrap();
    let verifying = p256::ecdsa::VerifyingKey::from(&public);
    let signature = p256::ecdsa::Signature::from_slice(&decode(parts[2])).unwrap();
    verifying
        .verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &signature)
        .expect("ES256 signature verifies");
}

/// Preserves the typed result for the caller's `expect_err` assertion.
pub(crate) trait UnwrapOrOk<T, E> {
    fn unwrap_err_or_ok(self, message: &str) -> Result<T, E>;
}

impl<T, E> UnwrapOrOk<T, E> for Result<T, E> {
    fn unwrap_err_or_ok(self, _message: &str) -> Result<T, E> {
        self
    }
}
