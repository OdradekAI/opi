use super::*;

pub(super) fn stored_api_key(value: &str) -> Credential {
    Credential::ApiKey(SecretString::from(value.to_owned()).into())
}

/// Synthetic service-account key shared by native and impersonated ADC exchanges.
pub(super) const SERVICE_ACCOUNT_PEM: &str = r###"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQCgvXz9l8JQ6kC1
E5/ZBuDBqumYD03J6pq9twlaSZ4cDCSOhadPVn8y9yayRNxEDZ25eWObVr5SrQvJ
QnlnKmN4bIXKThvkFxzGWTX96ESZbBBg2Fbv3EjkaRaE+H8WBUtc/Hfn5av8CfKj
x9GOwSzW3VkBPW4LhBSCbXLB07f4hgQ6tMyTaejx5nNBzYtD8SsTW9LnpW4GE2GX
z+U1axSU8WFLfgUzVIXZMSrHY0rwOSwCcSicgbNGd8uIEIv0juOyakaRrPlIk5Ce
s+BYC2dT/SEDJd7VqH98qAw9KxOMmGw9rD7odHafXIB8VmQsTsm53N0KdrsufhJO
32YWIsWFAgMBAAECggEACR6ES+Z99kq6mztO3zMXcbgBUtM9ccuO9gfqGSv3iHZS
OvQPXajV2b2b/ahP9HFu+xHitNz7OmQJuOQ9+ycfEmWFF/ON/Ab+ptcdVndvWuKU
BvaHGh+90pg5WEJj2As6rxQTYRM5NQ5DFzJz1lzpxGfgOo8yYidfECuRmOMAJd0s
3k8FZ/y6nshQgfQ0Lw96aFGfCu8Ycu+3K7p8mh+wH1Bq5oVeXgExlcdO6DLXjtQO
vq0H52mBEHKTG7uiJRadPwoYL8gNhyMhg8hDMUX0w6cUIUchJkLygEaa6NElisMZ
jnSo+gcvHDl2LGQrknineuwgJT12SQ1mtnhltp//eQKBgQDQR5ssNInh/sMOxD8o
ZbuT1iRYMQYeNZqLZeJjstMtQHy3Ul1NH2REImStR1Y5dJRO3d1vRiGIZZx++SEp
44QXdfKin0IfW9w89+2KrZrxfk6eraWHPbYX6FRCob2cfm84RgI+P9YQ7jNLoiix
264MjWJ59Qoyl+ZxTFKMh6637QKBgQDFkYJQSeOApt2OUqtbozJ6NuiyzqpTJVDd
tx4/gGGwKtNm5c1FhYBF6n3WEq261l3drvDC6ucnoN2f7eItcS7Owv5Vylzx1oAM
8KPr59WM96phpnugAWBrOYa4UyJYUxn1eVyjx9l986IeaSIXdhXa32bVfd220Ouu
CC7Lv/1g+QKBgQCugA/mDW6oRcYvDQTNV868oeF5cjlp/gcpW4c9eImtlCXpgbxh
7TcYTqiC9Q2svZu0J/ZqUBuwJ1Hin9/FNLVq4osS7NN9an0vJ5mBYso6IlTEZUAV
+wIhLanK2a/++X89aopelSVIsgmQCKCHw0IjiiFDhKWylcmgiw1uaA9eUQKBgAaE
gFoJtfmeh5yUWZbSUW36tb6AVtobVwmZauQJZi+bmhHA7v6mIs8rpy8Suqx5OeuA
Q+0KNsn14pnODTIWk8a6ZA2MeXh47QyAO8gig8b8vKAwD8/9e6LN6QXXqEBlrWuN
r2Kvyxr+mJCMQ1WmzZ3E1O2iE1hemAfLuU97a+5xAoGBAKoGwyrOChBroJmIcMc7
RYpC88yq0FXkN4rXD1o/3n0CUzeeb1pad2wlt0efNCWRU5E2dM13aBxdQvg0kGNq
f1JdzC/2b2W3wiseeVHZn7MSsny0JpfbveYtQ9xbrmmRY2amtuMzS/jbnrIKQ+wv
7VCl8xqVmjOIpkF7O/B26nBb
-----END PRIVATE KEY-----"###;

// --- Mock OAuthProvider + LoginPresenter proving object safety + callability ---

pub(crate) struct MockLoginPresenter {
    pub(crate) manual_code: String,
    pub(crate) url_calls: AtomicUsize,
}

impl LoginPresenter for MockLoginPresenter {
    fn present_auth_url<'a>(
        &'a self,
        _url: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        let count = &self.url_calls;
        Box::pin(async move {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }

    fn present_device_code<'a>(
        &'a self,
        _user_code: &'a str,
        _uri: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        Box::pin(async { Ok(()) })
    }

    fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
        let code = self.manual_code.clone();
        Box::pin(async move { Ok(code) })
    }

    fn notify_success(&self) {}

    fn notify_failure(&self, _reason: &str) {}
}

pub(crate) struct MockOAuthProvider {
    pub(crate) id: String,
}

impl OAuthProvider for MockOAuthProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn login<'a>(
        &'a self,
        presenter: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async move {
            presenter
                .present_auth_url("https://example/oauth/authorize")
                .await?;
            let code = presenter.await_manual_code().await?;
            presenter.notify_success();
            Ok(OAuthCredential {
                access: SecretString::from(format!("access-{code}")),
                refresh: SecretString::from("refresh-fixed"),
                expires_at: None,
                base_url: None,
                account_id: None,
                client_id: None,
                enterprise_domain: None,
                available_model_ids: None,
                scopes: None,
            })
        })
    }

    fn refresh<'a>(
        &'a self,
        cred: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async move {
            Ok(OAuthCredential {
                access: SecretString::from("access-refreshed"),
                refresh: cred.refresh.clone(),
                expires_at: cred.expires_at,
                base_url: cred.base_url.clone(),
                account_id: None,
                client_id: None,
                enterprise_domain: None,
                available_model_ids: None,
                scopes: None,
            })
        })
    }
}

// Serialize ambient-environment tests (AmbientAuthContext reads std::env).
pub(crate) static AMBIENT_ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Injected AuthContext backed by a map, counting every access so tests can
/// prove which operations consult a source at all.
pub(crate) struct CountingMapContext {
    pub(crate) env: std::collections::HashMap<String, String>,
    pub(crate) files: std::collections::HashMap<String, bool>,
    pub(crate) env_reads: AtomicUsize,
    pub(crate) file_probes: AtomicUsize,
}

impl CountingMapContext {
    pub(crate) fn new(env: impl IntoIterator<Item = (&'static str, &'static str)>) -> Self {
        Self {
            env: env
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect(),
            files: std::collections::HashMap::new(),
            env_reads: AtomicUsize::new(0),
            file_probes: AtomicUsize::new(0),
        }
    }
}

impl opi_ai::auth::AuthContext for CountingMapContext {
    fn env(&self, name: &str) -> Result<Option<String>, opi_ai::auth::AuthContextError> {
        self.env_reads.fetch_add(1, Ordering::SeqCst);
        Ok(self.env.get(name).cloned())
    }

    fn file_exists(&self, path: &str) -> Result<bool, opi_ai::auth::AuthContextError> {
        self.file_probes.fetch_add(1, Ordering::SeqCst);
        Ok(self.files.get(path).copied().unwrap_or(false))
    }
}

/// In-memory CredentialStore that records every operation.
pub(crate) struct RecordingStore {
    pub(crate) entries: std::collections::HashMap<String, opi_ai::Credential>,
    pub(crate) reads: AtomicUsize,
    pub(crate) writes: AtomicUsize,
    pub(crate) deletes: std::sync::Mutex<Vec<String>>,
    pub(crate) fail_reads: bool,
}

impl RecordingStore {
    pub(crate) fn new(
        entries: impl IntoIterator<Item = (&'static str, opi_ai::Credential)>,
    ) -> Self {
        Self {
            entries: entries
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect(),
            reads: AtomicUsize::new(0),
            writes: AtomicUsize::new(0),
            deletes: std::sync::Mutex::new(Vec::new()),
            fail_reads: false,
        }
    }
}

impl opi_ai::CredentialStore for RecordingStore {
    fn read<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> opi_ai::BoxAuthFuture<'a, Result<Option<opi_ai::Credential>, opi_ai::CredentialStoreError>>
    {
        Box::pin(async move {
            self.reads.fetch_add(1, Ordering::SeqCst);
            if self.fail_reads {
                return Err(opi_ai::CredentialStoreError::Backend {
                    provider: provider_id.to_owned(),
                    reason: "keychain daemon unreachable".to_owned(),
                });
            }
            Ok(self.entries.get(provider_id).cloned())
        })
    }

    fn write<'a>(
        &'a self,
        _provider_id: &'a str,
        _cred: &'a opi_ai::Credential,
    ) -> opi_ai::BoxAuthFuture<'a, Result<(), opi_ai::CredentialStoreError>> {
        Box::pin(async move {
            self.writes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }

    fn delete<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> opi_ai::BoxAuthFuture<'a, Result<(), opi_ai::CredentialStoreError>> {
        Box::pin(async move {
            self.deletes.lock().unwrap().push(provider_id.to_owned());
            Ok(())
        })
    }

    fn probe<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> opi_ai::BoxAuthFuture<'a, opi_ai::CredentialSource> {
        Box::pin(async move {
            if self.entries.contains_key(provider_id) {
                opi_ai::CredentialSource::Present {
                    label: "keychain test".to_owned(),
                }
            } else {
                opi_ai::CredentialSource::Absent
            }
        })
    }
}

/// One-model metadata provider over the shared non-network test support.
pub(crate) fn provider_with_model(id: &str) -> Box<dyn opi_ai::Provider> {
    Box::new(opi_ai::test_support::MockProvider::new_with_models(
        id,
        vec![opi_ai::ModelInfo::new(
            format!("{id}-model"),
            format!("{id} model"),
            opi_ai::WireApi::OpenAiCompletions,
            opi_ai::ModelCapabilities::new(128_000, 4096),
        )],
        vec![],
    ))
}

/// Resolver that reports no credential available (typed CredentialNeeded).
pub(crate) struct NeededResolver;

impl opi_ai::AuthResolver for NeededResolver {
    fn resolve<'a>(&'a self) -> opi_ai::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        Box::pin(async {
            Err(ProviderError::CredentialNeeded {
                provider_id: "prov-needed".to_owned(),
            })
        })
    }
}

/// Resolver that always fails with a non-CredentialNeeded auth error.
pub(crate) struct FailingResolver {
    pub(crate) provider_id: &'static str,
    pub(crate) calls: AtomicUsize,
}

impl opi_ai::AuthResolver for FailingResolver {
    fn resolve<'a>(&'a self) -> opi_ai::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        let calls = &self.calls;
        let provider_id = self.provider_id;
        Box::pin(async move {
            calls.fetch_add(1, Ordering::SeqCst);
            Err(ProviderError::AuthFailed(
                opi_ai::provider::ProviderErrorSummary::from_untrusted(format!(
                    "selected source failed for {provider_id}"
                )),
            ))
        })
    }
}
