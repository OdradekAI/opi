//! AWS Bedrock provider profile.
//!
//! Endpoint, route identity, model catalog and immutable dispatch
//! configuration live here. Bedrock Converse request projection, SigV4
//! signing, event-stream decoding and the direct Chat API live in
//! [`crate::api::bedrock`].

use std::sync::Arc;

use secrecy::SecretString;

use crate::api::bedrock::BedrockOptions;
use crate::auth::aws::sigv4::AwsCredentials;
use crate::http::HttpClient;
use crate::model_info::WireApi;
use crate::provider::{ModelInfo, ProviderError, ProviderErrorSummary};
use crate::registry::ModelCapabilities;
use crate::retry::smithy as retry;

/// Model families supported by this provider. `openai` exists for the
/// Bedrock-hosted GPT-OSS/GPT models (pi's effort engine carries dedicated
/// `gpt-oss`/`gpt-` arms keyed on the model id).
pub(crate) const SUPPORTED_FAMILIES: &[&str] =
    &["anthropic", "meta", "mistral", "amazon", "cohere", "openai"];

/// Fixed nonsecret placeholder identity for the explicit
/// proxy-placeholder-signing mode. These well-known values carry no
/// AWS account authority; the mode exists to reach a caller-supplied local
/// proxy without reading any real credential source, and it keeps a real
/// SigV4 signature instead of claiming unsigned `NoAuth`.
pub const PROXY_PLACEHOLDER_ACCESS_KEY_ID: &str = "dummy-access-key";
/// Fixed nonsecret placeholder secret key (see
/// [`PROXY_PLACEHOLDER_ACCESS_KEY_ID`]).
pub const PROXY_PLACEHOLDER_SECRET_ACCESS_KEY: &str = "dummy-secret-key";

/// Fixed nonsecret placeholder SigV4 credentials for a frozen region.
///
/// Part of the explicit proxy-placeholder-signing mode (see
/// [`proxy_placeholder_auth`]).
pub fn proxy_placeholder_credentials(region: &str) -> AwsCredentials {
    AwsCredentials {
        access_key_id: SecretString::from(PROXY_PLACEHOLDER_ACCESS_KEY_ID),
        secret_access_key: SecretString::from(PROXY_PLACEHOLDER_SECRET_ACCESS_KEY),
        session_token: None,
        region: region.to_owned(),
        expires_at: None,
    }
}

/// Whether `base_url` is a standard AWS Bedrock runtime endpoint
/// (`bedrock-runtime[.-fips].<region>.amazonaws.com[.cn]`).
pub fn is_standard_bedrock_endpoint(base_url: &str) -> bool {
    let Ok(url) = url::Url::parse(base_url) else {
        return false;
    };
    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host.to_ascii_lowercase();
    let Some(rest) = host.strip_prefix("bedrock-runtime") else {
        return false;
    };
    let rest = rest.strip_prefix("-fips").unwrap_or(rest);
    let Some(rest) = rest.strip_prefix('.') else {
        return false;
    };
    let Some((region, suffix)) = rest.split_once('.') else {
        return false;
    };
    !region.is_empty()
        && !region.contains('.')
        && (suffix == "amazonaws.com" || suffix == "amazonaws.com.cn")
}

/// Reject the explicit proxy-placeholder mode for standard AWS endpoints.
pub(crate) fn validate_proxy_placeholder_endpoint(base_url: &str) -> Result<(), ProviderError> {
    if is_standard_bedrock_endpoint(base_url) {
        return Err(ProviderError::Config(ProviderErrorSummary::sanitized(
            "Bedrock ProxyPlaceholderSigning requires a caller-supplied nonstandard endpoint; \
             standard AWS endpoints reject this mode",
        )));
    }
    Ok(())
}

/// Build the explicit no-real-credential proxy-placeholder prepared
/// authentication for a caller-supplied nonstandard endpoint and frozen
/// region.
pub fn proxy_placeholder_auth(
    endpoint: &str,
    region: &str,
) -> Result<crate::auth::ResolvedAuth, ProviderError> {
    validate_proxy_placeholder_endpoint(endpoint)?;
    Ok(crate::auth::ResolvedAuth {
        env: Default::default(),
        scheme: crate::auth::AuthScheme::AwsSigV4(proxy_placeholder_credentials(region)),
        secret: SecretString::from(""),
        base_url: Some(endpoint.to_owned()),
        account_id: None,
        provenance: crate::auth::AuthProvenance {
            source: crate::auth::AuthProvenanceSource::AwsSigV4 {
                source: crate::auth::AwsCredentialSource::ProxyPlaceholder,
            },
            fallback: crate::auth::AuthFallback::NotAttempted,
        },
    })
}

/// Concrete AWS Bedrock provider profile using the Converse API.
#[derive(Clone)]
pub struct BedrockProvider {
    pub(crate) provider_identity: String,
    pub(crate) base_url: Option<String>,
    pub(crate) client: Arc<HttpClient>,
    /// Dedicated HTTP/1.1 client for the explicit proxy-placeholder mode;
    /// placeholder-authenticated sends refuse to run without it.
    pub(crate) placeholder_client: Option<Arc<HttpClient>>,
    pub(crate) models: Vec<ModelInfo>,
    /// Typed native options for one direct/native dispatch.
    pub(crate) native_options: Option<BedrockOptions>,
    /// Frozen AWS retry configuration.
    pub(crate) retry: retry::SmithyRetryConfig,
}

impl BedrockProvider {
    /// Construct a credential-free Bedrock provider profile.
    pub fn new(base_url: Option<String>, client: Arc<HttpClient>) -> Self {
        let models = model_catalog();
        Self {
            provider_identity: "bedrock".into(),
            base_url,
            models,
            client,
            placeholder_client: None,
            native_options: None,
            retry: retry::SmithyRetryConfig::default(),
        }
    }

    /// Replace the model catalog for a direct adapter or test.
    pub fn with_models(mut self, models: Vec<ModelInfo>) -> Self {
        self.models = models;
        self
    }

    /// Bind capture and replay to the registered canonical provider identity.
    #[cfg(feature = "builtins")]
    pub(crate) fn with_provider_identity(mut self, provider: &str) -> Self {
        self.provider_identity = provider.to_owned();
        self
    }

    /// Bind typed native options for one dispatch.
    pub fn with_native_options(mut self, options: BedrockOptions) -> Self {
        self.native_options = Some(options);
        self
    }

    /// Freeze an explicit AWS retry configuration.
    pub fn with_retry_config(mut self, retry: retry::SmithyRetryConfig) -> Self {
        self.retry = retry;
        self
    }

    /// Attach the dedicated HTTP/1.1 client used only by the explicit
    /// proxy-placeholder-signing mode.
    pub fn with_placeholder_client(mut self, client: Arc<HttpClient>) -> Self {
        self.placeholder_client = Some(client);
        self
    }

    /// Replace the HTTP client with a shared one.
    pub fn with_client(self, client: Arc<HttpClient>) -> Self {
        Self { client, ..self }
    }

    /// Access the shared HTTP client.
    pub fn http_client(&self) -> &Arc<HttpClient> {
        &self.client
    }

    /// Return supported model families.
    pub fn supported_model_families(&self) -> Vec<&str> {
        SUPPORTED_FAMILIES.to_vec()
    }

    /// Validate that a model ID belongs to a supported family.
    pub fn validate_model_id(&self, model_id: &str) -> Result<(), ProviderError> {
        let family = model_id.split('.').next().unwrap_or("");
        if SUPPORTED_FAMILIES.contains(&family) {
            Ok(())
        } else {
            Err(ProviderError::Config(ProviderErrorSummary::sanitized(
                format!(
                    "unsupported Bedrock model family '{family}' in model ID '{model_id}'; supported families: {}",
                    SUPPORTED_FAMILIES.join(", ")
                ),
            )))
        }
    }
}

/// Redact AWS credentials for safe display.
pub fn redact_credentials(_access_key_id: &str, _secret_key: &str) -> String {
    "***".to_string()
}

/// Built-in Bedrock model metadata without credentials or HTTP construction.
pub fn model_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo::new(
            "anthropic.claude-sonnet-4-20250514-v2:0",
            "Claude Sonnet 4 (Bedrock)",
            WireApi::BedrockConverseStream,
            ModelCapabilities::new(200000, 8192)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        ),
        ModelInfo::new(
            "anthropic.claude-opus-4-20250514-v1:0",
            "Claude Opus 4 (Bedrock)",
            WireApi::BedrockConverseStream,
            ModelCapabilities::new(200000, 8192)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        ),
        ModelInfo::new(
            "anthropic.claude-haiku-4-5-20250514-v1:0",
            "Claude Haiku 4.5 (Bedrock)",
            WireApi::BedrockConverseStream,
            ModelCapabilities::new(200000, 8192)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        ),
    ]
}
