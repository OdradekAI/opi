//! Lazy builtin Vertex source selection over the existing ADC owner.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use secrecy::{ExposeSecret, SecretString};

use super::adc::native::{
    AdcAuthSelection, AdcNativeOwner, AdcSourceHttpError, AdcSourceRequest, AdcSourceResponse,
    AdcSourceTransport, ReqwestAdcTransport, SystemAdcClock,
};
use super::{
    AuthContext, AuthProvenance, AuthProvenanceSource, AuthResolver, AuthScheme, ResolvedAuth,
};
use crate::credential::{BoxAuthFuture, CredentialStore};
use crate::models::{
    AuthCheck, AuthCheckKind, BuiltinAuthBindings, BuiltinAuthFactory, BuiltinAuthStatus,
};
use crate::provider::{ProviderError, ProviderErrorSummary};

/// Source and routing inputs for the builtin Vertex profile.
///
/// All fields are owned at registration. Environment reads, credential-store
/// access and ADC exchanges start only when an operation demands them.
#[derive(Clone, Default)]
pub struct BuiltinVertexConfig {
    /// Project override; otherwise GOOGLE_CLOUD_PROJECT then GCLOUD_PROJECT.
    pub project: Option<String>,
    /// Location override; otherwise GOOGLE_CLOUD_LOCATION.
    pub location: Option<String>,
    /// Generation host override, including any deployment prefix.
    pub base_url: Option<String>,
    /// Existing ADC owner with caller-selected source transport and clock.
    pub adc: Option<AdcNativeOwner>,
}

impl std::fmt::Debug for BuiltinVertexConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuiltinVertexConfig")
            .field("project", &self.project)
            .field("location", &self.location)
            .field("base_url", &self.base_url)
            .field("has_adc_owner", &self.adc.is_some())
            .finish()
    }
}

#[derive(Default)]
struct LazyTransport(OnceLock<ReqwestAdcTransport>);

impl AdcSourceTransport for LazyTransport {
    fn send(
        &self,
        request: AdcSourceRequest,
    ) -> BoxAuthFuture<'_, Result<AdcSourceResponse, AdcSourceHttpError>> {
        Box::pin(async move {
            self.0
                .get_or_init(ReqwestAdcTransport::new)
                .send(request)
                .await
        })
    }
}

#[derive(Clone)]
struct Resolver {
    context: Arc<dyn AuthContext>,
    store: Option<Arc<dyn CredentialStore>>,
    config: BuiltinVertexConfig,
    adc: AdcNativeOwner,
}

fn config_error(message: impl Into<String>) -> ProviderError {
    ProviderError::Config(ProviderErrorSummary::from_untrusted(message.into()))
}

struct Selection {
    context: Arc<dyn AuthContext>,
    env: BTreeMap<String, SecretString>,
    api_key: Option<(SecretString, AuthProvenanceSource)>,
}

impl Resolver {
    async fn selection(&self) -> Result<Selection, ProviderError> {
        let stored = super::builtin::stored_api_key(self.store.as_ref(), "google-vertex").await?;
        let env = stored
            .as_ref()
            .map(|stored| stored.env().clone())
            .unwrap_or_default();
        let context = super::builtin::scoped_context(self.context.clone(), &env);
        let candidate = match stored.as_ref().and_then(|stored| stored.key()) {
            Some(key) => Some((
                key.clone(),
                AuthProvenanceSource::CredentialStore {
                    kind: "google-vertex".into(),
                },
            )),
            None => context
                .env("GOOGLE_CLOUD_API_KEY")
                .map_err(|error| config_error(error.to_string()))?
                .map(|key| {
                    (
                        key.into(),
                        AuthProvenanceSource::Environment {
                            name: "GOOGLE_CLOUD_API_KEY".into(),
                        },
                    )
                }),
        };
        // These are the pinned provider's ADC markers, not usable API keys.
        let api_key = candidate.filter(|(key, _)| {
            let key = key.expose_secret().trim();
            let marker = key
                .strip_prefix('<')
                .and_then(|key| key.strip_suffix('>'))
                .is_some_and(|key| !key.is_empty() && !key.contains('>'));
            !key.is_empty() && key != "gcp-vertex-credentials" && !marker
        });
        Ok(Selection {
            context,
            env,
            api_key,
        })
    }

    fn project_location(
        &self,
        context: &dyn AuthContext,
    ) -> Result<Option<(String, String)>, ProviderError> {
        let env = |name| {
            context
                .env(name)
                .map_err(|error| config_error(error.to_string()))
        };
        let project = match self.config.project.clone() {
            Some(project) => Some(project),
            None => match env("GOOGLE_CLOUD_PROJECT")? {
                Some(project) => Some(project),
                None => env("GCLOUD_PROJECT")?,
            },
        };
        let location = match self.config.location.clone() {
            Some(location) => Some(location),
            None => env("GOOGLE_CLOUD_LOCATION")?,
        };
        Ok(project.zip(location).filter(|(project, location)| {
            !project.trim().is_empty() && !location.trim().is_empty()
        }))
    }

    fn endpoint(&self, project_location: Option<(&str, &str)>) -> String {
        let host = self
            .config
            .base_url
            .clone()
            .unwrap_or_else(|| match project_location {
                Some((_, location)) if location != "global" => {
                    format!("https://{location}-aiplatform.googleapis.com")
                }
                _ => "https://aiplatform.googleapis.com".into(),
            });
        let host = host.trim_end_matches('/');
        match project_location {
            Some((project, location)) => format!(
                "{host}/v1/projects/{project}/locations/{location}/publishers/google/models"
            ),
            None => format!("{host}/v1/publishers/google/models"),
        }
    }

    async fn status(&self) -> Result<Option<BuiltinAuthStatus>, ProviderError> {
        let selection = self.selection().await?;
        let source = if selection.api_key.is_some() {
            "Vertex API key"
        } else if self.project_location(selection.context.as_ref())?.is_some()
            && self
                .adc
                .clone()
                .with_ambient(selection.context)
                .check_configured()
                .map_err(|error| config_error(error.safe_summary()))?
        {
            "Application Default Credentials"
        } else {
            return Ok(None);
        };
        Ok(Some(BuiltinAuthStatus {
            check: AuthCheck {
                source: Some(source.into()),
                kind: AuthCheckKind::ApiKey,
            },
            available_model_ids: None,
            responses_only: false,
            classifiers_allowed: false,
        }))
    }
}

impl AuthResolver for Resolver {
    fn resolve(&self) -> BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
        Box::pin(async move {
            let selection = self.selection().await?;
            if let Some((secret, source)) = selection.api_key {
                return Ok(ResolvedAuth {
                    env: selection.env,
                    scheme: AuthScheme::ApiKey,
                    secret: SecretString::from(secret.expose_secret().trim()),
                    base_url: Some(self.endpoint(None)),
                    account_id: None,
                    provenance: AuthProvenance {
                        source,
                        ..Default::default()
                    },
                });
            }
            let (project, location) = self
                .project_location(selection.context.as_ref())?
                .ok_or_else(|| config_error("Vertex ADC requires a project and location"))?;
            let mut auth = self
                .adc
                .clone()
                .with_ambient(selection.context)
                .resolve()
                .await?;
            auth.env = selection.env;
            auth.base_url = Some(self.endpoint(Some((&project, &location))));
            Ok(auth)
        })
    }
}

pub(crate) fn factory(
    context: Arc<dyn AuthContext>,
    config: BuiltinVertexConfig,
) -> BuiltinAuthFactory {
    Arc::new(move |store, _flow| {
        let mut adc = config
            .adc
            .clone()
            .unwrap_or_else(|| {
                AdcNativeOwner::new(
                    AdcAuthSelection::Auto,
                    Arc::new(LazyTransport::default()),
                    Arc::new(SystemAdcClock),
                )
            })
            .with_ambient(context.clone());
        if let Some(queue) = store.as_ref().and_then(|store| store.mutation_queue()) {
            adc = adc.with_mutation_queue(queue);
        }
        let resolver = Arc::new(Resolver {
            context: context.clone(),
            store,
            config: config.clone(),
            adc,
        });
        let check = resolver.clone();
        BuiltinAuthBindings {
            chat: resolver.clone(),
            classifier: resolver,
            check: Arc::new(move || {
                let check = check.clone();
                Box::pin(async move { check.status().await })
            }),
        }
    })
}
