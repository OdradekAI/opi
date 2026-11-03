//! Builtin Bedrock authentication through the source-qualified local chain.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use secrecy::{ExposeSecret, SecretString};

use super::aws::cache::{AwsSourceCache, PassiveRefreshDeps, RandomDpopJti, SystemSourceClock};
use super::aws::local::{
    AwsAuthSelection, AwsLocalChainInput, CredentialResolutionError, ScopedAwsEnv,
    resolve_local_aws_auth,
};
use super::aws::process::{CredentialProcessRunner, SpawnedCredentialProcess};
use super::aws::source_http::{
    AwsSourceTransport, ReqwestSourceTransport, SourceHttpError, SourceHttpRequest,
    SourceHttpResponse,
};
use super::{AuthContext, AuthProvenanceSource, AuthResolver, ResolvedAuth};
use crate::credential::{BoxAuthFuture, CredentialStore};
use crate::models::{
    AuthCheck, AuthCheckKind, BuiltinAuthBindings, BuiltinAuthFactory, BuiltinAuthStatus,
};
use crate::provider::{ProviderError, ProviderErrorSummary};

/// Owned inputs for the builtin Bedrock source chain. No source is read during
/// construction; the existing native chain retains its selection and failure rules.
#[derive(Clone, Default)]
pub struct BuiltinAwsConfig {
    /// Explicit source selection; omission uses the native Auto chain.
    pub selection: Option<AwsAuthSelection>,
    /// Source-scoped values, with the native no-mixing rules for credential pairs.
    pub scoped: ScopedAwsEnv,
    /// Explicit signing region.
    pub region: Option<String>,
    /// Generation endpoint override, independent of auth-source HTTP.
    pub base_url: Option<String>,
    /// Injected transport for source exchanges (STS, metadata, SSO and login).
    pub transport: Option<Arc<dyn AwsSourceTransport>>,
    /// Injected credential-process owner. Omission uses the SDK runner without
    /// a selected timeout/stdout policy; process cleanup remains owned.
    pub process: Option<Arc<dyn CredentialProcessRunner>>,
}

impl std::fmt::Debug for BuiltinAwsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuiltinAwsConfig")
            .field("selection", &self.selection)
            .field("scoped", &self.scoped)
            .field("region", &self.region)
            .field("base_url", &self.base_url)
            .field("has_transport", &self.transport.is_some())
            .field("has_process", &self.process.is_some())
            .finish()
    }
}

#[derive(Default)]
struct LazyTransport(OnceLock<ReqwestSourceTransport>);

impl AwsSourceTransport for LazyTransport {
    fn send(
        &self,
        request: SourceHttpRequest,
    ) -> BoxAuthFuture<'_, Result<SourceHttpResponse, SourceHttpError>> {
        Box::pin(async move {
            self.0
                .get_or_init(ReqwestSourceTransport::new)
                .send(request)
                .await
        })
    }
}

#[derive(Clone)]
struct Resolver {
    context: Arc<dyn AuthContext>,
    store: Option<Arc<dyn CredentialStore>>,
    config: BuiltinAwsConfig,
    transport: Arc<dyn AwsSourceTransport>,
    process: Arc<dyn CredentialProcessRunner>,
    cache: AwsSourceCache,
}

fn config_error(message: impl Into<String>) -> ProviderError {
    ProviderError::Config(ProviderErrorSummary::from_untrusted(message.into()))
}

struct Selection {
    selection: AwsAuthSelection,
    scoped: ScopedAwsEnv,
    context: Arc<dyn AuthContext>,
    env: BTreeMap<String, SecretString>,
    stored_key: bool,
}

fn scoped_env(configured: &ScopedAwsEnv, env: &BTreeMap<String, SecretString>) -> ScopedAwsEnv {
    let mut scoped = configured.clone();
    macro_rules! secret {
        ($field:ident, $name:literal) => {
            if scoped.$field.is_none() {
                scoped.$field = env.get($name).cloned();
            }
        };
    }
    macro_rules! text {
        ($field:ident, $name:literal) => {
            if scoped.$field.is_none() {
                scoped.$field = env.get($name).map(|value| value.expose_secret().to_owned());
            }
        };
    }
    // Preserve attempted scoped pairs, including empty/partial values, so the
    // native chain cannot borrow a credential half from ambient configuration.
    secret!(access_key_id, "AWS_ACCESS_KEY_ID");
    secret!(secret_access_key, "AWS_SECRET_ACCESS_KEY");
    secret!(session_token, "AWS_SESSION_TOKEN");
    secret!(bearer_token_bedrock, "AWS_BEARER_TOKEN_BEDROCK");
    secret!(
        container_authorization_token,
        "AWS_CONTAINER_AUTHORIZATION_TOKEN"
    );
    text!(profile, "AWS_PROFILE");
    text!(region, "AWS_REGION");
    text!(default_region, "AWS_DEFAULT_REGION");
    text!(web_identity_token_file, "AWS_WEB_IDENTITY_TOKEN_FILE");
    text!(role_arn, "AWS_ROLE_ARN");
    text!(role_session_name, "AWS_ROLE_SESSION_NAME");
    text!(
        container_credentials_relative_uri,
        "AWS_CONTAINER_CREDENTIALS_RELATIVE_URI"
    );
    text!(
        container_credentials_full_uri,
        "AWS_CONTAINER_CREDENTIALS_FULL_URI"
    );
    text!(
        container_authorization_token_file,
        "AWS_CONTAINER_AUTHORIZATION_TOKEN_FILE"
    );
    text!(metadata_service_endpoint, "AWS_METADATA_SERVICE_ENDPOINT");
    text!(
        metadata_service_endpoint_mode,
        "AWS_EC2_METADATA_SERVICE_ENDPOINT_MODE"
    );
    if scoped.ec2_metadata_disabled.is_none() {
        scoped.ec2_metadata_disabled = env.get("AWS_EC2_METADATA_DISABLED").map(|value| {
            super::aws::metadata::parse_metadata_disabled(Some(value.expose_secret()))
        });
    }
    scoped
}

impl Resolver {
    fn explicit_selection(&self) -> Option<AwsAuthSelection> {
        self.config
            .selection
            .clone()
            .filter(|selection| !matches!(selection, AwsAuthSelection::Auto))
    }

    async fn selection(&self) -> Result<Selection, ProviderError> {
        // An explicit source must not consult an unrelated credential store.
        if let Some(selection) = self.explicit_selection() {
            return Ok(Selection {
                selection,
                scoped: self.config.scoped.clone(),
                context: self.context.clone(),
                env: BTreeMap::new(),
                stored_key: false,
            });
        }
        let stored = super::builtin::stored_api_key(self.store.as_ref(), "amazon-bedrock").await?;
        let env = stored
            .as_ref()
            .map(|stored| stored.env().clone())
            .unwrap_or_default();
        let key = stored.as_ref().and_then(|stored| stored.key()).cloned();
        Ok(Selection {
            selection: key
                .clone()
                .map(AwsAuthSelection::Bearer)
                .unwrap_or(AwsAuthSelection::Auto),
            scoped: scoped_env(&self.config.scoped, &env),
            context: super::builtin::scoped_context(self.context.clone(), &env),
            env,
            stored_key: key.is_some(),
        })
    }

    async fn status(&self) -> Result<Option<BuiltinAuthStatus>, ProviderError> {
        let selection = self.selection().await?;
        let env = |name| {
            selection
                .context
                .env(name)
                .map_err(|error| config_error(error.to_string()))
        };
        let scoped = &selection.scoped;
        let pair_attempted = scoped.access_key_id.is_some()
            || scoped.secret_access_key.is_some()
            || scoped.session_token.is_some();
        let configured = !matches!(selection.selection, AwsAuthSelection::Auto)
            || scoped.bearer_token_bedrock.is_some()
            || env("AWS_BEARER_TOKEN_BEDROCK")?.is_some()
            || scoped.profile.is_some()
            || env("AWS_PROFILE")?.is_some()
            || (scoped.access_key_id.is_some() && scoped.secret_access_key.is_some())
            || (!pair_attempted
                && env("AWS_ACCESS_KEY_ID")?.is_some()
                && env("AWS_SECRET_ACCESS_KEY")?.is_some())
            || scoped.container_credentials_relative_uri.is_some()
            || scoped.container_credentials_full_uri.is_some()
            || scoped.web_identity_token_file.is_some()
            || env("AWS_CONTAINER_CREDENTIALS_RELATIVE_URI")?.is_some()
            || env("AWS_CONTAINER_CREDENTIALS_FULL_URI")?.is_some()
            || env("AWS_WEB_IDENTITY_TOKEN_FILE")?.is_some();
        Ok(configured.then(|| BuiltinAuthStatus {
            check: AuthCheck {
                source: Some("AWS credential chain".into()),
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
            let selected = self.selection().await?;
            let input = AwsLocalChainInput {
                selection: selected.selection,
                scoped: selected.scoped,
                ambient: selected.context.as_ref(),
                credentials_file: None,
                config_file: None,
                process: self.process.as_ref(),
                transport: self.transport.as_ref(),
                mfa: None,
                sts_endpoint: None,
                source_cache: Some(&self.cache),
                clock: &SystemSourceClock,
                dpop_jti: &RandomDpopJti,
                sso_cache_dir: None,
                login_cache_dir: None,
                sso_endpoint: None,
                oidc_endpoint: None,
                signin_endpoint: None,
                region: self.config.region.as_deref(),
                default_region: "us-east-1",
            };
            let mut auth = resolve_local_aws_auth(&input)
                .await
                .map_err(|error| match error {
                    CredentialResolutionError::Exhausted => ProviderError::CredentialNeeded {
                        provider_id: "amazon-bedrock".into(),
                    },
                    error => ProviderError::Config(error.into_safe_provider_summary()),
                })?;
            if selected.stored_key {
                auth.provenance.source = AuthProvenanceSource::CredentialStore {
                    kind: "amazon-bedrock".into(),
                };
            }
            auth.env = selected.env;
            if let Some(base_url) = &self.config.base_url {
                auth.base_url = Some(base_url.clone());
            }
            Ok(auth)
        })
    }
}

pub(crate) fn factory(
    context: Arc<dyn AuthContext>,
    config: BuiltinAwsConfig,
) -> BuiltinAuthFactory {
    Arc::new(move |store, _flow| {
        let transport: Arc<dyn AwsSourceTransport> = config
            .transport
            .clone()
            .unwrap_or_else(|| Arc::new(LazyTransport::default()));
        let process: Arc<dyn CredentialProcessRunner> = config
            .process
            .clone()
            .unwrap_or_else(|| Arc::new(SpawnedCredentialProcess::default()));
        let queue = store
            .as_ref()
            .and_then(|store| store.mutation_queue())
            .unwrap_or_else(|| Arc::new(crate::mutation::MutationQueue::new()));
        let cache = AwsSourceCache::with_mutation_queue(queue).with_passive_deps(Arc::new(
            PassiveRefreshDeps {
                transport: transport.clone(),
                clock: Arc::new(SystemSourceClock),
                jti: Arc::new(RandomDpopJti),
            },
        ));
        let resolver = Arc::new(Resolver {
            context: context.clone(),
            store,
            config: config.clone(),
            transport,
            process,
            cache,
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
