//! Lazy source selection for builtin profiles. Construction never reads sources.

use std::collections::BTreeMap;
use std::sync::Arc;

use secrecy::{ExposeSecret, SecretString};

use super::{
    AuthContext, AuthProvenance, AuthProvenanceSource, AuthResolver, AuthScheme, OAuthProvider,
    ResolvedAuth,
};
use crate::credential::{ApiKeyCredential, BoxAuthFuture, Credential, CredentialStore};
use crate::models::{
    AuthCheck, AuthCheckKind, BuiltinAuthBindings, BuiltinAuthFactory, BuiltinAuthStatus,
};
use crate::provider::{ProviderError, ProviderErrorSummary};
use crate::providers::BuiltinProviderProfile;

fn config(message: impl Into<String>) -> ProviderError {
    ProviderError::Config(ProviderErrorSummary::from_untrusted(message.into()))
}

async fn stored(
    store: Option<&Arc<dyn CredentialStore>>,
    id: &str,
) -> Result<Option<Credential>, ProviderError> {
    match store {
        Some(store) => store
            .read(id)
            .await
            .map_err(|error| config(error.to_string())),
        None => Ok(None),
    }
}

pub(crate) async fn stored_api_key(
    store: Option<&Arc<dyn CredentialStore>>,
    id: &str,
) -> Result<Option<ApiKeyCredential>, ProviderError> {
    match stored(store, id).await? {
        Some(Credential::ApiKey(key)) => Ok(Some(key)),
        Some(Credential::OAuthToken { .. }) => {
            Err(config(format!("{id} requires an API key credential")))
        }
        None => Ok(None),
    }
}

/// Capture credential-scoped configuration without reading ambient sources.
pub(crate) fn scoped_context(
    base: Arc<dyn AuthContext>,
    env: &BTreeMap<String, SecretString>,
) -> Arc<dyn AuthContext> {
    Arc::new(ScopedContext {
        base,
        env: env.clone(),
    })
}

struct ScopedContext {
    base: Arc<dyn AuthContext>,
    env: BTreeMap<String, SecretString>,
}
impl AuthContext for ScopedContext {
    fn home_dir(&self) -> Result<Option<std::path::PathBuf>, super::AuthContextError> {
        self.base.home_dir()
    }

    fn env(&self, name: &str) -> Result<Option<String>, super::AuthContextError> {
        match self.env.get(name) {
            Some(value) => Ok(Some(value.expose_secret().to_owned())),
            None => self.base.env(name),
        }
    }
    fn file_exists(&self, path: &str) -> Result<bool, super::AuthContextError> {
        self.base.file_exists(path)
    }
    fn process_snapshot(&self) -> Result<super::AuthProcessSnapshot, super::AuthContextError> {
        let mut snapshot = self.base.process_snapshot()?;
        snapshot.env.extend(
            self.env
                .iter()
                .map(|(name, value)| (name.clone(), value.expose_secret().to_owned())),
        );
        Ok(snapshot)
    }
}

#[derive(Clone)]
struct BuiltinResolver {
    profile: BuiltinProviderProfile,
    context: Arc<dyn AuthContext>,
    store: Option<Arc<dyn CredentialStore>>,
    flow: Option<Arc<dyn OAuthProvider>>,
    classifier: bool,
}

enum Selection {
    Key(Box<ResolvedAuth>),
    OAuth {
        available_model_ids: Option<Vec<String>>,
    },
}

impl BuiltinResolver {
    fn env(&self, name: &str) -> Result<Option<String>, ProviderError> {
        self.context
            .env(name)
            .map(|value| value.filter(|value| !value.is_empty()))
            .map_err(|error| config(error.to_string()))
    }

    fn key(
        &self,
        secret: SecretString,
        scheme: AuthScheme,
        source: AuthProvenanceSource,
        env: BTreeMap<String, SecretString>,
    ) -> Result<ResolvedAuth, ProviderError> {
        if secret.expose_secret().is_empty() {
            return Err(ProviderError::CredentialNeeded {
                provider_id: self.profile.id.into(),
            });
        }
        let ctx = scoped_context(self.context.clone(), &env);
        let field = |name: &str| ctx.env(name).map_err(|error| config(error.to_string()));
        let base_url = match self.profile.id {
            "cloudflare-workers-ai" | "cloudflare-ai-gateway" => {
                let account = field("CLOUDFLARE_ACCOUNT_ID")?
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| ProviderError::CredentialNeeded {
                        provider_id: self.profile.id.into(),
                    })?;
                if self.profile.id == "cloudflare-ai-gateway" {
                    let gateway = field("CLOUDFLARE_GATEWAY_ID")?
                        .filter(|v| !v.is_empty())
                        .ok_or_else(|| ProviderError::CredentialNeeded {
                            provider_id: self.profile.id.into(),
                        })?;
                    Some(format!(
                        "https://gateway.ai.cloudflare.com/v1/{account}/{gateway}"
                    ))
                } else {
                    Some(format!(
                        "https://api.cloudflare.com/client/v4/accounts/{account}"
                    ))
                }
            }
            _ => None,
        };
        Ok(ResolvedAuth {
            env,
            scheme,
            secret,
            base_url,
            account_id: None,
            provenance: AuthProvenance {
                source,
                fallback: super::AuthFallback::NotAttempted,
            },
        })
    }

    async fn select(&self) -> Result<Option<Selection>, ProviderError> {
        let credential = stored(self.store.as_ref(), self.profile.id).await?;
        let scoped = match credential {
            Some(Credential::OAuthToken {
                available_model_ids,
                ..
            }) => {
                if self.flow.is_none()
                    || (self.classifier && self.profile.hides_classifiers_on_oauth)
                {
                    return Err(config(format!(
                        "{} does not accept OAuth for this operation",
                        self.profile.id
                    )));
                }
                return Ok(Some(Selection::OAuth {
                    available_model_ids,
                }));
            }
            Some(Credential::ApiKey(credential)) => {
                if self.profile.id == "openai-codex" {
                    return Err(config("openai-codex requires OAuth credentials"));
                }
                if let Some(secret) = credential.key() {
                    return self
                        .key(
                            secret.clone(),
                            self.profile.auth_scheme.clone(),
                            AuthProvenanceSource::CredentialStore {
                                kind: self.profile.id.into(),
                            },
                            credential.env().clone(),
                        )
                        .map(|auth| Some(Selection::Key(Box::new(auth))));
                }
                credential.env().clone()
            }
            None => BTreeMap::new(),
        };
        let candidates: Vec<(&str, AuthScheme)> = match self.profile.id {
            "anthropic" => vec![
                ("ANTHROPIC_AUTH_TOKEN", AuthScheme::Bearer),
                ("ANTHROPIC_OAUTH_TOKEN", AuthScheme::ApiKey),
                ("ANTHROPIC_API_KEY", AuthScheme::ApiKey),
            ],
            "cloudflare-ai-gateway" => vec![("CLOUDFLARE_API_KEY", AuthScheme::ApiKey)],
            _ => self
                .profile
                .api_key_env
                .map(|name| vec![(name, self.profile.auth_scheme.clone())])
                .unwrap_or_default(),
        };
        for (name, scheme) in candidates {
            if let Some(secret) = self.env(name)? {
                return self
                    .key(
                        secret.into(),
                        scheme,
                        AuthProvenanceSource::Environment { name: name.into() },
                        scoped,
                    )
                    .map(|auth| Some(Selection::Key(Box::new(auth))));
            }
        }
        if self.profile.id == "anthropic" {
            let ctx = scoped_context(self.context.clone(), &scoped);
            let mut env = scoped;
            for name in [
                "ANTHROPIC_FEDERATION_RULE_ID",
                "ANTHROPIC_ORGANIZATION_ID",
                "ANTHROPIC_IDENTITY_TOKEN_FILE",
            ] {
                let Some(value) = ctx
                    .env(name)
                    .map_err(|error| config(error.to_string()))?
                    .filter(|value| !value.is_empty())
                else {
                    return Ok(None);
                };
                env.insert(name.into(), value.into());
            }
            for name in ["ANTHROPIC_SERVICE_ACCOUNT_ID", "ANTHROPIC_WORKSPACE_ID"] {
                if let Some(value) = ctx
                    .env(name)
                    .map_err(|error| config(error.to_string()))?
                    .filter(|value| !value.is_empty())
                {
                    env.insert(name.into(), value.into());
                }
            }
            return Ok(Some(Selection::Key(Box::new(ResolvedAuth {
                scheme: AuthScheme::AnthropicFederation,
                secret: "".into(),
                base_url: None,
                account_id: None,
                env,
                provenance: AuthProvenance {
                    source: AuthProvenanceSource::Environment {
                        name: "ANTHROPIC_IDENTITY_TOKEN_FILE".into(),
                    },
                    fallback: super::AuthFallback::NotAttempted,
                },
            }))));
        }
        Ok(None)
    }

    async fn status(&self) -> Result<Option<BuiltinAuthStatus>, ProviderError> {
        let selection = match self.select().await {
            Ok(selection) => selection,
            Err(ProviderError::CredentialNeeded { .. }) => None,
            Err(error) => return Err(error),
        };
        let Some(selection) = selection else {
            return Ok(None);
        };
        let (source, kind, available_model_ids, oauth) = match selection {
            Selection::Key(auth) => {
                let source = if auth.scheme == AuthScheme::AnthropicFederation {
                    "workload identity federation".to_owned()
                } else {
                    match auth.provenance.source {
                        AuthProvenanceSource::Environment { name } => name,
                        _ => "credential store".into(),
                    }
                };
                (source, AuthCheckKind::ApiKey, None, false)
            }
            Selection::OAuth {
                available_model_ids,
            } => (
                "OAuth".into(),
                AuthCheckKind::OAuth,
                available_model_ids,
                true,
            ),
        };
        Ok(Some(BuiltinAuthStatus {
            check: AuthCheck {
                source: Some(source),
                kind,
            },
            available_model_ids,
            responses_only: oauth && self.profile.id == "openai",
            classifiers_allowed: !(oauth && self.profile.hides_classifiers_on_oauth),
        }))
    }
}

impl AuthResolver for BuiltinResolver {
    fn resolve(&self) -> BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
        Box::pin(async move {
            match self.select().await? {
                Some(Selection::Key(auth)) => Ok(*auth),
                Some(Selection::OAuth { .. }) => {
                    super::store_oauth::StoreOAuthResolver::new(
                        self.store.clone().expect("selected stored OAuth"),
                        self.profile.id,
                        self.flow.clone().expect("selected OAuth flow"),
                    )
                    .resolve()
                    .await
                }
                None => Err(ProviderError::CredentialNeeded {
                    provider_id: self.profile.id.into(),
                }),
            }
        })
    }
}

pub(crate) fn factory(
    profile: &BuiltinProviderProfile,
    context: Arc<dyn AuthContext>,
    flow: Option<Arc<dyn OAuthProvider>>,
) -> BuiltinAuthFactory {
    let profile = profile.clone();
    Arc::new(move |store, flow_override| {
        let chat = Arc::new(BuiltinResolver {
            profile: profile.clone(),
            context: context.clone(),
            store,
            flow: flow_override.or_else(|| flow.clone()),
            classifier: false,
        });
        let checker = chat.clone();
        let mut classifier = (*chat).clone();
        classifier.classifier = true;
        BuiltinAuthBindings {
            chat,
            classifier: Arc::new(classifier),
            check: Arc::new(move || {
                let checker = checker.clone();
                Box::pin(async move { checker.status().await })
            }),
        }
    })
}

pub(crate) fn default_oauth_flow(id: &str) -> Option<Arc<dyn OAuthProvider>> {
    use super::oauth;
    Some(match id {
        "anthropic" => Arc::new(oauth::anthropic::AnthropicOAuthProvider::production()),
        "openai" => Arc::new(oauth::chatgpt::OpenAiChatGptOAuthProvider::production(
            None, None,
        )),
        "openai-codex" => Arc::new(oauth::codex::CodexOAuthProvider::production()),
        "github-copilot" => Arc::new(oauth::copilot::GitHubCopilotOAuthProvider::production()),
        "kimi-coding" => Arc::new(oauth::kimi_coding::KimiCodingOAuthProvider::production()),
        "meta" => Arc::new(oauth::meta::MetaOAuthProvider::production()),
        "openrouter" => Arc::new(oauth::openrouter::OpenRouterOAuthProvider::production()),
        "radius" => Arc::new(oauth::radius::RadiusOAuthProvider::production()),
        "xai" => Arc::new(oauth::xai::XaiOAuthProvider::production()),
        _ => return None,
    })
}

/// Resolve only the source-owned template prefix; custom API bases remain owned
/// by the selected model. Copilot owns its selected account's endpoint.
pub(crate) fn effective_base_url(provider: &str, selected: &str, auth: &ResolvedAuth) -> String {
    let Some(resolved) = auth.base_url.as_deref() else {
        return selected.into();
    };
    let template = match provider {
        "cloudflare-workers-ai" => {
            Some("https://api.cloudflare.com/client/v4/accounts/{CLOUDFLARE_ACCOUNT_ID}")
        }
        "cloudflare-ai-gateway" => Some(
            "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}",
        ),
        _ => None,
    };
    if let Some(template) = template {
        return selected
            .strip_prefix(template)
            .map(|suffix| format!("{}{suffix}", resolved.trim_end_matches('/')))
            .unwrap_or_else(|| selected.into());
    }
    if provider == "github-copilot" {
        resolved.into()
    } else {
        selected.into()
    }
}
