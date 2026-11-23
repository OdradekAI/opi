//! pi-messages Provider profile.
//!
//! Radius endpoint selection, its gateway catalog and client ownership live
//! here. Pi Messages request framing, event mapping and typed Chat calls live
//! in [`crate::api::pi_messages`].

use std::sync::Arc;

use crate::http::HttpClient;
use crate::model_info::WireApi;
use crate::provider::ModelInfo;
use crate::provider::routes::PiMessagesRouteContext;
use crate::registry::ModelCapabilities;

/// Default Radius gateway.
pub const BASE_URL: &str = "https://radius.pi.dev";

/// Canonical provider-facing name for the neutral Pi Messages route context.
pub type PiMessagesProvider = PiMessagesRouteContext;

impl std::fmt::Debug for PiMessagesRouteContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PiMessagesProvider")
            .field("base_url", &self.base_url)
            .field("models", &self.models)
            .finish_non_exhaustive()
    }
}

impl PiMessagesRouteContext {
    pub fn new(base_url: Option<String>) -> Self {
        let models = if base_url.as_deref().is_none_or(|base| base == BASE_URL) {
            model_catalog().to_vec()
        } else {
            Vec::new()
        };
        Self {
            base_url: base_url.unwrap_or_else(|| BASE_URL.into()),
            client: Arc::new(HttpClient::new()),
            models,
        }
    }

    /// Replace the profile's advertised model records.
    pub fn with_models(mut self, models: Vec<ModelInfo>) -> Self {
        self.models = models;
        self
    }
}

/// Fallback chat catalog for a Radius provider constructed with default
/// model metadata.
pub fn model_catalog() -> Vec<ModelInfo> {
    vec![ModelInfo::new(
        "claude-sonnet-4-5",
        "claude-sonnet-4-5",
        WireApi::PiMessages,
        ModelCapabilities::new(200_000, 16_384)
            .with_streaming(true)
            .with_thinking(true)
            .with_images(true),
    )]
}

#[cfg(feature = "builtins")]
pub(crate) use gateway::RadiusCatalog;

#[cfg(feature = "builtins")]
mod gateway {
    use super::{Arc, BASE_URL, HttpClient, ModelInfo, PiMessagesRouteContext};
    use crate::auth::{AuthScheme, OAuthProvider, ResolvedAuth};
    use crate::catalog::AuthenticatedCatalog;
    use crate::credential::BoxAuthFuture;
    use crate::provider::{ProviderError, ProviderErrorSummary};
    use secrecy::ExposeSecret;
    use serde::{Deserialize, Serialize};
    use url::Url;

    pub(crate) struct RadiusCatalog {
        base_url: String,
        origin: String,
        client: Arc<HttpClient>,
    }

    impl RadiusCatalog {
        pub(crate) fn new(gateway: &str, client: Arc<HttpClient>) -> Result<Self, ProviderError> {
            let gateway = if gateway.contains("://") {
                gateway.to_owned()
            } else {
                format!("https://{gateway}")
            };
            let gateway = gateway_url(&gateway).ok_or_else(|| {
                ProviderError::Config(ProviderErrorSummary::attested_static(
                    "invalid Radius gateway URL",
                ))
            })?;
            Ok(Self {
                base_url: gateway.as_str().trim_end_matches('/').to_owned(),
                origin: gateway.origin().ascii_serialization(),
                client,
            })
        }

        pub(crate) fn is_default(&self) -> bool {
            self.base_url == BASE_URL
        }

        pub(crate) fn route(&self, models: Vec<ModelInfo>) -> PiMessagesRouteContext {
            PiMessagesRouteContext {
                base_url: self.base_url.clone(),
                client: self.client.clone(),
                models,
            }
        }

        pub(crate) fn oauth_flow(&self) -> Arc<dyn OAuthProvider> {
            Arc::new(
                crate::auth::oauth::radius::RadiusOAuthProvider::with_gateway_client(
                    &self.origin,
                    self.client.client().clone(),
                ),
            )
        }
    }

    impl AuthenticatedCatalog for RadiusCatalog {
        fn scope(&self) -> &str {
            &self.origin
        }

        fn fetch(
            &self,
            auth: ResolvedAuth,
        ) -> BoxAuthFuture<'_, Result<Vec<ModelInfo>, ProviderError>> {
            Box::pin(async move {
                if auth.scheme != AuthScheme::Bearer {
                    return Err(ProviderError::Config(
                        ProviderErrorSummary::attested_static(
                            "Radius catalog refresh requires Bearer authentication",
                        ),
                    ));
                }
                let response = self
                    .client
                    .client()
                    .get(format!("{}/v1/config", self.origin))
                    .header(reqwest::header::ACCEPT, "application/json")
                    .bearer_auth(auth.secret.expose_secret())
                    .timeout(std::time::Duration::from_secs(30))
                    .send()
                    .await
                    .map_err(|error| {
                        if error.is_timeout() {
                            ProviderError::Timeout
                        } else {
                            ProviderError::Network(ProviderErrorSummary::redacted())
                        }
                    })?;
                if !response.status().is_success() {
                    return Err(ProviderError::ServiceFault {
                        status: response.status().as_u16(),
                        error_name: None,
                        retry_after_ms: None,
                        detail: ProviderErrorSummary::attested_static(
                            "Radius config request failed",
                        ),
                    });
                }
                if response.url().origin().ascii_serialization() != self.origin {
                    return Err(invalid_config());
                }
                let config: GatewayConfig = response.json().await.map_err(|error| {
                    if error.is_timeout() {
                        ProviderError::Timeout
                    } else {
                        invalid_config()
                    }
                })?;
                let base = gateway_url(&config.base_url)
                    .filter(|url| url.origin().ascii_serialization() == self.origin)
                    .ok_or_else(invalid_config)?;
                let base = base.as_str().trim_end_matches('/');

                config
                    .models
                    .into_iter()
                    .map(|model| model.into_record(base))
                    .collect()
            })
        }
    }

    fn gateway_url(value: &str) -> Option<Url> {
        Url::parse(value).ok().filter(|url| {
            matches!(url.scheme(), "http" | "https")
                && url.has_host()
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none()
        })
    }

    fn invalid_config() -> ProviderError {
        ProviderError::RequestFailed(ProviderErrorSummary::attested_static(
            "invalid Radius gateway config",
        ))
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct GatewayConfig {
        base_url: String,
        models: Vec<GatewayModel>,
    }

    #[derive(Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct GatewayModel {
        id: String,
        name: String,
        reasoning: bool,
        thinking_level_map: Option<serde_json::Map<String, serde_json::Value>>,
        input: Vec<InputModality>,
        cost: serde_json::Map<String, serde_json::Value>,
        context_window: u64,
        max_tokens: u64,
        input_limits: Option<serde_json::Value>,
    }

    #[derive(Deserialize, Serialize)]
    #[serde(rename_all = "lowercase")]
    enum InputModality {
        Text,
        Image,
    }

    impl GatewayModel {
        fn into_record(self, base: &str) -> Result<ModelInfo, ProviderError> {
            if self.input.is_empty() {
                return Err(invalid_config());
            }
            let mut entry = serde_json::to_value(self).map_err(|_| invalid_config())?;
            entry["api"] = "pi-messages".into();
            entry["baseUrl"] = base.into();
            let mut record =
                crate::catalog::wire::decode_model_entry(&entry).map_err(|_| invalid_config())?;
            record.capabilities.supports_streaming = true;
            Ok(record)
        }
    }
}
