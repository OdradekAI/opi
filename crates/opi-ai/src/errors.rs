//! Shared provider errors, redacted diagnostics and deterministic classification.
//! Retry scheduling and request-state construction remain with their runtime owners.

use crate::types::model::WireApi;
mod redaction;
mod service_fault;
pub use redaction::safe_excerpt;
pub use service_fault::{SmithyErrorClass, classify_service_failure};

/// Redaction-safe public summary carried by free-form [`ProviderError`] classes.
///
/// The inner string is private so a custom [`crate::Provider`] cannot put a dynamic
/// upstream payload into `ProviderError` by constructing a variant directly.
/// Dynamic untrusted text can only enter through [`Self::from_untrusted`],
/// which intentionally discards it. Closed no-input constructors expose the
/// limited summaries custom providers can retain.
///
/// ```compile_fail
/// use opi_ai::provider::ProviderError;
///
/// let raw_upstream = String::from("arbitrary provider response");
/// let _ = ProviderError::ProviderSide(raw_upstream);
/// ```
///
/// Static and deliberately leaked provider text are not safe conversions:
///
/// ```compile_fail
/// use opi_ai::provider::ProviderError;
///
/// const PROVIDER_CANARY: &str = "static-provider-canary";
/// let _ = ProviderError::ProviderSide(PROVIDER_CANARY.into());
/// ```
///
/// ```compile_fail
/// use opi_ai::provider::ProviderError;
///
/// let leaked: &'static str = Box::leak(String::from("leaked-provider-canary").into_boxed_str());
/// let _ = ProviderError::ProviderSide(leaked.into());
/// ```
#[derive(Clone, serde::Serialize)]
#[serde(transparent)]
pub struct ProviderErrorSummary(
    Box<str>,
    #[serde(skip)] Option<std::sync::Arc<crate::types::transcript::TranscriptError>>,
);

impl PartialEq for ProviderErrorSummary {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for ProviderErrorSummary {}
impl std::hash::Hash for ProviderErrorSummary {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.0, state);
    }
}

impl std::error::Error for ProviderErrorSummary {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.1.as_deref().map(|source| source as _)
    }
}

impl ProviderErrorSummary {
    const REDACTED: &'static str = "[REDACTED]";

    /// Construct the closed redacted-detail summary without accepting text.
    pub fn redacted() -> Self {
        Self(Self::REDACTED.into(), None)
    }

    /// Construct the closed summary for credentials rejected by a provider.
    pub fn authentication_rejected() -> Self {
        Self("provider rejected credentials".into(), None)
    }

    /// Replace an arbitrary upstream error payload with a stable safe summary.
    ///
    /// The input is accepted so custom providers have an explicit producer
    /// boundary, but it is never retained in public `Display`, `Debug`, or
    /// serialized diagnostics.
    pub fn from_untrusted(_payload: impl AsRef<str>) -> Self {
        Self::redacted()
    }

    /// Return the safe summary text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Construct an in-crate summary after applying the shared defense-in-depth
    /// redactor. Raw provider response bodies use [`Self::from_untrusted`]
    /// instead; this path is for locally-produced context and transport errors.
    /// Build a summary with the canonical credential redaction applied.
    ///
    /// Public since task 20.51: functional-callback failures and advanced
    /// error constructors need the same redaction pipeline as adapters.
    pub fn sanitized(summary: impl AsRef<str>) -> Self {
        Self(safe_excerpt(summary.as_ref()).into_boxed_str(), None)
    }

    /// Construct from crate-owned, caller-attested static diagnostic text.
    pub(crate) fn attested_static(summary: &'static str) -> Self {
        Self(summary.into(), None)
    }

    /// Construct a safe HTTP failure summary from status alone.
    ///
    /// Callers must not read a provider-controlled response body solely to
    /// construct this summary.
    pub(crate) fn from_http_response(status: u16) -> Self {
        Self(
            format!("HTTP {status}: {}", Self::REDACTED).into_boxed_str(),
            None,
        )
    }

    pub(crate) fn unsupported_temporal(count: usize) -> Self {
        let source = std::sync::Arc::new(
            crate::types::transcript::TranscriptError::UnsupportedTemporalFidelity {
                mid_convo_system_messages: count,
            },
        );
        let mut summary = Self::sanitized(source.to_string());
        summary.1 = Some(source);
        summary
    }
}

impl std::ops::Deref for ProviderErrorSummary {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl std::fmt::Display for ProviderErrorSummary {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl std::fmt::Debug for ProviderErrorSummary {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("ProviderErrorSummary")
            .field(&self.as_str())
            .finish()
    }
}

impl PartialEq<str> for ProviderErrorSummary {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for ProviderErrorSummary {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

/// Errors that can occur during provider streaming.
///
/// The [`ProviderError::category`] taxonomy exposes nine closed classes
/// (auth, config, request, network, rate_limit, provider, stream, capability,
/// cancelled). `Timeout` is retained as a distinct variant but classifies as
/// `Network`, since the spec defines the network class as "DNS, TLS, proxy,
/// timeout".
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ProviderError {
    /// Adapter-owned canonical state at failure, consumed by `ChatStream`.
    #[doc(hidden)]
    #[error("{0}")]
    StreamFailure(#[source] Box<ProviderStreamFailure>),
    #[error("rate limited")]
    RateLimited { retry_after_ms: Option<u64> },
    #[error("request timed out")]
    Timeout,
    #[error("request failed: {0}")]
    RequestFailed(ProviderErrorSummary),
    /// A functional callback failed (task 20.51): header transform, payload,
    /// response, or raw observer. A functional failure of the call, never
    /// retryable, never swallowed, and never triggering a new call.
    #[error("callback phase failed: {0}")]
    CallbackPhase(ProviderErrorSummary),
    /// A source-qualified strict-limit rejection (task 20.11, V-34): the
    /// Anthropic wire answered 400 `invalid_request_error` with one of the
    /// documented strict-tool limit messages on a request that actually
    /// carried prefer-strict tools. Never an ordinary transport retry: the
    /// acquisition owner may admit exactly one rebuilt downgrade variant,
    /// and a second occurrence (or one without prefer-strict tools) is a
    /// terminal failure.
    #[error("strict tool limits exceeded: {detail}")]
    StrictToolLimits { detail: ProviderErrorSummary },
    #[error("stream error: {0}")]
    StreamError(ProviderErrorSummary),
    #[error("authentication failed: {0}")]
    AuthFailed(ProviderErrorSummary),
    /// No credential is available for the provider. Non-retryable: the caller
    /// must obtain a credential (interactive login or a typed non-interactive
    /// diagnostic) before retrying. Distinct from [`AuthFailed`](Self::AuthFailed)
    /// because it routes to the login/credential-needed path, not retry. It
    /// never starts login automatically.
    #[error("credential needed for provider '{provider_id}'; run `/login {provider_id}`")]
    CredentialNeeded { provider_id: String },
    /// A previously valid credential was rejected by the provider (e.g. 401 on
    /// an OAuth token). Non-retryable and never auto-relogs-in: the current
    /// turn ends and a later explicit `/login` is required.
    #[error("credential revoked for provider '{provider_id}'; re-login required")]
    CredentialRevoked { provider_id: String },
    /// The provider requires a non-secret account identifier alongside its
    /// bearer token, but the credential did not contain one.
    #[error("credential for provider '{provider_id}' is missing its account id; re-login required")]
    AccountIdMissing { provider_id: String },
    /// The user cancelled login-method selection before any credential flow
    /// or persistence operation began.
    #[error("login cancelled for provider '{provider_id}'")]
    LoginCancelled { provider_id: String },
    #[error("network error: {0}")]
    Network(ProviderErrorSummary),
    #[error("invalid provider configuration: {0}")]
    Config(#[source] ProviderErrorSummary),
    #[error("unknown model '{model_id}' for provider '{provider_id}'")]
    UnknownModel {
        provider_id: String,
        model_id: String,
    },
    #[error("provider '{provider_id}' has no route for wire API '{wire_api}'")]
    MissingWireRoute {
        provider_id: String,
        wire_api: WireApi,
    },
    #[error(
        "model '{model_id}' wire API '{wire_api}' does not match compatibility wire '{compat_wire}'"
    )]
    WireCompatMismatch {
        model_id: String,
        wire_api: WireApi,
        compat_wire: WireApi,
    },
    #[error("provider error: {0}")]
    ProviderSide(ProviderErrorSummary),
    /// A structured service-model failure carrying the wire status and error
    /// name (task 20.20). AWS wires surface this so the acquisition owner can
    /// classify through the source's exact trait/name rules; other consumers
    /// see the same taxonomy category as [`ProviderSide`](Self::ProviderSide).
    #[error("service fault {status}: {detail}")]
    ServiceFault {
        /// The HTTP status code of the failed response.
        status: u16,
        /// The wire service error type name, when exposed.
        #[doc(hidden)]
        error_name: Option<String>,
        /// Server-advised retry delay in milliseconds, when present.
        retry_after_ms: Option<u64>,
        /// Redacted human-readable detail.
        detail: ProviderErrorSummary,
    },
    #[error("unsupported capability: {0}")]
    UnsupportedCapability(ProviderErrorSummary),
    #[error("cancelled")]
    Cancelled,
}

/// Retained call state and its typed, redacted cause.
///
/// The fields and constructor are private. Advanced preparation failures may
/// carry this through `ModelsError`; the ordinary stream collector converts it
/// into `CallFailure`. Diagnostic formatting exposes only the typed cause.
#[doc(hidden)]
#[derive(Clone, PartialEq)]
pub struct ProviderStreamFailure {
    message: crate::types::message::AssistantMessage,
    cause: ProviderError,
    response_received: bool,
    // Preserve preparation cancellation origin independently of the caller token.
    call_cancelled: bool,
}

impl ProviderStreamFailure {
    pub(crate) fn set_projection(&mut self, facts: &crate::types::transcript::ProjectionFacts) {
        self.message.projection = Some(facts.clone());
    }

    /// Known canonical partial, including accepted projection facts. This
    /// association does not establish dispatch or external acknowledgement.
    pub fn message(&self) -> &crate::types::message::AssistantMessage {
        &self.message
    }

    /// The original typed classification, without the adapter-state envelope.
    pub fn cause(&self) -> &ProviderError {
        &self.cause
    }
}

impl std::error::Error for ProviderStreamFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}

impl std::fmt::Display for ProviderStreamFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.cause.fmt(formatter)
    }
}

impl std::fmt::Debug for ProviderStreamFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.cause, formatter)
    }
}

impl ProviderError {
    pub(crate) fn with_partial(self, message: crate::types::message::AssistantMessage) -> Self {
        match self {
            Self::StreamFailure(_) => self,
            cause => Self::StreamFailure(Box::new(ProviderStreamFailure {
                message,
                cause,
                response_received: false,
                call_cancelled: false,
            })),
        }
    }

    pub(crate) fn with_call_cancellation(mut self) -> Self {
        if let Self::StreamFailure(failure) = &mut self
            && matches!(failure.cause, Self::Cancelled)
        {
            failure.call_cancelled = true;
        }
        self
    }

    pub(crate) fn is_call_cancelled(&self) -> bool {
        matches!(self, Self::StreamFailure(failure)
            if failure.call_cancelled && matches!(failure.cause, Self::Cancelled))
    }

    /// Carry a transport handoff fact without changing the public cause or
    /// canonical partial. Only the private producer snapshot sets this flag.
    pub(crate) fn after_response(mut self) -> Self {
        if let Self::StreamFailure(failure) = &mut self {
            failure.response_received = true;
        }
        self
    }

    pub(crate) fn response_received(&self) -> bool {
        matches!(self, Self::StreamFailure(failure) if failure.response_received)
    }

    pub(crate) fn into_stream_parts(
        self,
    ) -> (
        Option<crate::types::message::AssistantMessage>,
        ProviderError,
    ) {
        match self {
            Self::StreamFailure(failure) => (Some(failure.message), failure.cause),
            cause => (None, cause),
        }
    }

    /// Whether this error is retryable (rate-limited, timeout, or transient
    /// network failure). Retry timing/backoff behavior is owned by the agent
    /// runtime; this is the taxonomy-layer retryability signal.
    pub fn is_retryable(&self) -> bool {
        if let Self::StreamFailure(failure) = self {
            return failure.cause.is_retryable();
        }
        if let Self::ServiceFault {
            status, error_name, ..
        } = self
        {
            // The taxonomy-layer projection of the source's classification;
            // the acquisition owner re-classifies exactly through the frozen
            // mode rules.
            return classify_service_failure(*status, error_name.as_deref()).is_retryable();
        }
        matches!(
            self,
            ProviderError::RateLimited { .. } | ProviderError::Timeout | ProviderError::Network(_)
        )
    }

    /// Stable diagnostic category for this provider error.
    ///
    /// `opi-ai` cannot depend on `opi-agent`'s shared `Diagnostic` model, so
    /// the provider-side classification surface is this small taxonomy. The
    /// `opi-agent` diagnostic layer maps each [`ProviderErrorCategory`] into a
    /// diagnostic `code`/`severity`/`source` triple.
    pub fn category(&self) -> ProviderErrorCategory {
        match self {
            ProviderError::StreamFailure(failure) => failure.cause.category(),
            ProviderError::AuthFailed(_)
            | ProviderError::CredentialNeeded { .. }
            | ProviderError::CredentialRevoked { .. }
            | ProviderError::AccountIdMissing { .. } => ProviderErrorCategory::Auth,
            ProviderError::Config(_)
            | ProviderError::MissingWireRoute { .. }
            | ProviderError::WireCompatMismatch { .. } => ProviderErrorCategory::Config,
            ProviderError::RequestFailed(_)
            | ProviderError::CallbackPhase(_)
            | ProviderError::StrictToolLimits { .. }
            | ProviderError::UnknownModel { .. } => ProviderErrorCategory::Request,
            ProviderError::Timeout | ProviderError::Network(_) => ProviderErrorCategory::Network,
            ProviderError::RateLimited { .. } => ProviderErrorCategory::RateLimit,
            ProviderError::ServiceFault { .. } => ProviderErrorCategory::Provider,
            ProviderError::ProviderSide(_) => ProviderErrorCategory::Provider,
            ProviderError::StreamError(_) => ProviderErrorCategory::Stream,
            ProviderError::UnsupportedCapability(_) => ProviderErrorCategory::Capability,
            ProviderError::Cancelled | ProviderError::LoginCancelled { .. } => {
                ProviderErrorCategory::Cancelled
            }
        }
    }

    /// Server-advised delay before retrying, in milliseconds.
    ///
    /// A [`ProviderError::RateLimited`] cause carries `retry_after_ms`;
    /// an adapter-state envelope preserves that delay unchanged.
    pub fn retry_after_ms(&self) -> Option<u64> {
        match self {
            ProviderError::StreamFailure(failure) => failure.cause.retry_after_ms(),
            ProviderError::RateLimited { retry_after_ms } => *retry_after_ms,
            ProviderError::ServiceFault { retry_after_ms, .. } => *retry_after_ms,
            _ => None,
        }
    }
}

/// Diagnostic category for a [`ProviderError`].
///
/// This is the opi-ai-owned classification substrate consumed by the
/// `opi-agent` diagnostic layer. Keeping it here means provider error
/// classification can be tested without any network access and without a
/// dependency on the shared `Diagnostic` model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderErrorCategory {
    /// Authentication failed (missing/invalid/expired credentials).
    Auth,
    /// Invalid provider configuration (bad endpoint, unsupported model, bad profile).
    Config,
    /// Local pre-request validation/schema failure (before the wire call).
    Request,
    /// Network/transport failure (DNS, TLS, proxy, timeout, connection).
    Network,
    /// Rate limited; a retry may succeed after a delay.
    RateLimit,
    /// Provider-side 4xx/5xx response with a safe body excerpt.
    Provider,
    /// Streaming response failed mid-flight.
    Stream,
    /// Unsupported image/tool/thinking capability rejected before the call.
    Capability,
    /// User/runtime cancellation. The retry loop owns timing, backoff, and
    /// stream-abort behavior; this is the taxonomy/diagnostic-layer slot.
    Cancelled,
}

// ---------------------------------------------------------------------------
// Public typed diagnostic extraction (task 20.53.3, P20-S53-3).
// Pinned source: `.repo/pi-1.3.0/packages/ai/src/utils/diagnostics.ts`
// (`DiagnosticErrorInfo`/`extractDiagnosticError`).

/// Typed, secret-safe diagnostic details extracted from a [`ProviderError`]
/// (pinned `DiagnosticErrorInfo`; task 20.53.3).
///
/// `name` is the type-level error name (the [`ProviderError`] variant
/// discriminant), `message` is the [`ProviderError`] display text whose
/// detail carrier is a [`ProviderErrorSummary`] - secret-safe by
/// construction - and the typed code members preserve the structured
/// carriers a [`ProviderError::ServiceFault`] holds: its HTTP status and the
/// wire service error type name. The pinned JavaScript shape also carries a
/// `stack`; Rust has no equivalent execution-stack source, so no stack field
/// exists here.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DiagnosticErrorInfo {
    /// Type-level error name: the `ProviderError` variant discriminant.
    pub name: &'static str,
    /// Secret-safe human message (the error's display text).
    pub message: String,
    /// Typed HTTP status carried by a service-model fault.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
    /// Wire service error type name exposed by a service-model fault.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_name: Option<String>,
}

/// Extract typed diagnostic details from a [`ProviderError`] (pinned
/// `extractDiagnosticError`; task 20.53.3, P20-S53-3).
///
/// Preserves the typed code carriers (`ServiceFault` HTTP status and wire
/// error name) and the secret-safe source detail text; never retains raw
/// provider payloads (those never enter [`ProviderErrorSummary`]).
pub fn extract_diagnostic_error(error: &ProviderError) -> DiagnosticErrorInfo {
    let (name, http_status, error_name) = match error {
        ProviderError::StreamFailure(_) => ("StreamFailure", None, None),
        ProviderError::RateLimited { .. } => ("RateLimited", None, None),
        ProviderError::Timeout => ("Timeout", None, None),
        ProviderError::RequestFailed(_) => ("RequestFailed", None, None),
        ProviderError::CallbackPhase(_) => ("CallbackPhase", None, None),
        ProviderError::StrictToolLimits { .. } => ("StrictToolLimits", None, None),
        ProviderError::StreamError(_) => ("StreamError", None, None),
        ProviderError::AuthFailed(_) => ("AuthFailed", None, None),
        ProviderError::CredentialNeeded { .. } => ("CredentialNeeded", None, None),
        ProviderError::CredentialRevoked { .. } => ("CredentialRevoked", None, None),
        ProviderError::AccountIdMissing { .. } => ("AccountIdMissing", None, None),
        ProviderError::LoginCancelled { .. } => ("LoginCancelled", None, None),
        ProviderError::Network(_) => ("Network", None, None),
        ProviderError::Config(_) => ("Config", None, None),
        ProviderError::UnknownModel { .. } => ("UnknownModel", None, None),
        ProviderError::MissingWireRoute { .. } => ("MissingWireRoute", None, None),
        ProviderError::WireCompatMismatch { .. } => ("WireCompatMismatch", None, None),
        ProviderError::ProviderSide(_) => ("ProviderSide", None, None),
        ProviderError::ServiceFault {
            status, error_name, ..
        } => ("ServiceFault", Some(*status), error_name.clone()),
        ProviderError::UnsupportedCapability(_) => ("UnsupportedCapability", None, None),
        ProviderError::Cancelled => ("Cancelled", None, None),
    };
    DiagnosticErrorInfo {
        name,
        message: error.to_string(),
        http_status,
        error_name,
    }
}
