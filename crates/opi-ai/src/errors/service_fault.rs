//! Pure classification of the existing Smithy/Bedrock service-fault values.
//! Protocol retry scheduling, quota and backoff are not part of this module.

/// Names the source classifies as THROTTLING regardless of status.
const THROTTLING_ERROR_CODES: &[&str] = &[
    "BandwidthLimitExceeded",
    "EC2ThrottledException",
    "LimitExceededException",
    "PriorRequestNotComplete",
    "ProvisionedThroughputExceededException",
    "RequestLimitExceeded",
    "RequestThrottled",
    "RequestThrottledException",
    "SlowDown",
    "ThrottledException",
    "Throttling",
    "ThrottlingException",
    "TooManyRequestsException",
    "TransactionInProgressException",
];

/// Names the source classifies as TRANSIENT regardless of status.
const TRANSIENT_ERROR_CODES: &[&str] =
    &["TimeoutError", "RequestTimeout", "RequestTimeoutException"];

/// Statuses the source classifies as TRANSIENT when present.
const TRANSIENT_ERROR_STATUS_CODES: &[u16] = &[500, 502, 503, 504];

/// Bedrock-modeled errors carrying the `$retryable` trait
/// (`ModelNotReadyException` models `$retryable = {}`, which the source
/// classifies TRANSIENT; `ModelTimeoutException` models no trait and stays
/// CLIENT_ERROR).
const BEDROCK_RETRYABLE_TRAIT_NAMES: &[&str] = &["ModelNotReadyException"];

/// The source's four-way classification of one observed failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithyErrorClass {
    /// Throttling: 429, a throttling error name, or a modeled throttling
    /// trait. Retry-eligible with the throttling delay base and cost.
    Throttling,
    /// Transient: a retryable trait, a transient name, a transport
    /// timeout/network failure, or one of the transient statuses.
    Transient,
    /// A 5xx that is not transient: never retried (no blanket >=500 retry).
    ServerError,
    /// Everything else, including 4xx client errors: never retried.
    ClientError,
}

impl SmithyErrorClass {
    /// Whether the class is retry-eligible at all.
    pub fn is_retryable(self) -> bool {
        matches!(self, Self::Throttling | Self::Transient)
    }
}

/// Classify a structured service failure (HTTP status plus the wire error
/// name from `x-amzn-errortype`) exactly as the source does.
pub fn classify_service_failure(status: u16, name: Option<&str>) -> SmithyErrorClass {
    let name = name.map(str::trim).filter(|name| !name.is_empty());
    if status == 429 || name.is_some_and(|name| THROTTLING_ERROR_CODES.contains(&name)) {
        return SmithyErrorClass::Throttling;
    }
    if name.is_some_and(|name| BEDROCK_RETRYABLE_TRAIT_NAMES.contains(&name))
        || name.is_some_and(|name| TRANSIENT_ERROR_CODES.contains(&name))
        || TRANSIENT_ERROR_STATUS_CODES.contains(&status)
    {
        return SmithyErrorClass::Transient;
    }
    if (500..=599).contains(&status) {
        return SmithyErrorClass::ServerError;
    }
    SmithyErrorClass::ClientError
}
