//! AWS Smithy V2 retry modes for Bedrock generation sends (task 20.20).
//!
//! The pinned source is `@smithy/core@3.33.3`
//! `dist-cjs/submodules/retry/index.js` with the Bedrock error model from
//! `@aws-sdk/client-bedrock-runtime@3.1127.0` (`x-amzn-errortype` names and
//! the modeled `$retryable` traits). opi owns the algorithm; the crate's
//! single acquisition owner (the private core behind the public
//! [`AcquisitionPolicy`](crate::retry::AcquisitionPolicy)) applies it
//! through
//! [`RetrySourcePolicy::Smithy`](crate::retry::RetrySourcePolicy::Smithy)
//! when a [`SmithyRetryConfig`] is frozen into the per-call policy.
//!
//! * Two known modes exist: `standard` and `adaptive`. An unknown mode is a
//!   typed configuration failure, never `Unsupported` and never silently
//!   substituted with standard.
//! * The Smithy2026 flavor is a fixed constants branch. The source selects it
//!   from `SMITHY_NEW_RETRIES_2026` at module load; opi never samples ambient
//!   process state, so the flavor is an explicit construction field.
//! * The retry quota starts at 500 tokens per logical call and is never
//!   shared across calls or instances; auth-source exchanges never draw from
//!   it (source exchanges and generation sends are counted separately by the
//!   source transports and the acquisition owner).
//! * Server hints (`Retry-After`/`x-amz-retry-after`) are bounded by the
//!   Smithy rule `max(delay, min(hint, delay + 5s))`; the helper family's
//!   60-second advisory bound is never applied to Smithy modes.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::provider::ProviderError;

/// Maximum backoff delay (fixed by the source).
pub const SMITHY_MAX_RETRY_DELAY_MS: u64 = 20_000;
/// Initial retry-quota capacity per logical call (fixed by the source).
pub const SMITHY_INITIAL_RETRY_TOKENS: u32 = 500;
/// Hint headroom applied on top of the computed backoff delay.
const SMITHY_HINT_HEADROOM_MS: u64 = 5_000;

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

/// A known AWS retry mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithyRetryMode {
    /// The standard quota/backoff mode (default).
    Standard,
    /// Standard plus the client-side adaptive rate limiter.
    Adaptive,
}

impl SmithyRetryMode {
    /// Parse one configured mode value. Only the two published lowercase
    /// names are known; anything else is a typed failure (the source never
    /// substitutes standard for an unknown mode).
    pub fn parse(value: &str) -> Result<Self, AwsRetryConfigError> {
        match value.trim() {
            "standard" => Ok(Self::Standard),
            "adaptive" => Ok(Self::Adaptive),
            other => Err(AwsRetryConfigError::UnknownMode(other.to_owned())),
        }
    }

    /// The configured mode name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Adaptive => "adaptive",
        }
    }
}

/// A scoped retry configuration failure. Both variants keep the offending
/// value; neither silently falls back to a default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AwsRetryConfigError {
    /// `AWS_RETRY_MODE`/`retry_mode` named no published mode.
    UnknownMode(String),
    /// `AWS_MAX_ATTEMPTS`/`max_attempts` did not parse as a number.
    InvalidMaxAttempts {
        /// Where the unparsable value came from.
        source: MaxAttemptsSource,
        /// The raw configured value.
        value: String,
    },
}

/// Which input supplied a `max_attempts` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaxAttemptsSource {
    /// The scoped `AWS_MAX_ATTEMPTS` environment value.
    ScopedEnvironment,
    /// The shared-config profile `max_attempts` entry.
    SharedProfile,
}

impl std::fmt::Display for MaxAttemptsSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ScopedEnvironment => write!(f, "environment variable AWS_MAX_ATTEMPTS"),
            Self::SharedProfile => write!(f, "shared config file entry max_attempts"),
        }
    }
}

impl std::fmt::Display for AwsRetryConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownMode(mode) => write!(f, "unknown AWS retry mode '{mode}'"),
            Self::InvalidMaxAttempts { source, value } => {
                write!(f, "{source} must be a number, got \"{value}\"")
            }
        }
    }
}

impl std::error::Error for AwsRetryConfigError {}

/// The frozen AWS retry configuration for one Bedrock provider bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithyRetryConfig {
    /// The retry mode.
    pub mode: SmithyRetryMode,
    /// The fixed Smithy2026 constants branch. The source reads
    /// `SMITHY_NEW_RETRIES_2026` at module load; opi exposes the branch as
    /// explicit construction input and never samples ambient state.
    pub smithy_2026: bool,
    /// The total attempt budget (initial send included). The source default
    /// is 3 (two extra sends).
    pub max_attempts: u32,
}

impl Default for SmithyRetryConfig {
    fn default() -> Self {
        Self {
            mode: SmithyRetryMode::Standard,
            smithy_2026: false,
            max_attempts: 3,
        }
    }
}

impl SmithyRetryConfig {
    /// The acquisition policy freezing this configuration for one logical
    /// call. `max_retries` mirrors the total budget as extra sends so the
    /// shared counters stay coherent; admission itself is governed by
    /// `max_attempts`.
    pub fn acquisition_policy(&self) -> crate::retry::AcquisitionPolicy {
        crate::retry::AcquisitionPolicy {
            max_retries: self.max_attempts.saturating_sub(1),
            source: crate::retry::RetrySourcePolicy::Smithy(self.clone()),
            ..crate::retry::AcquisitionPolicy::default()
        }
    }
}

/// The source's scoped retry inputs: environment values win over the
/// shared-config profile, which wins over the defaults. opi never derives
/// these from ambient process state; the caller supplies the scoped values.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScopedRetryInputs<'a> {
    /// Scoped `AWS_RETRY_MODE`.
    pub env_retry_mode: Option<&'a str>,
    /// Scoped `AWS_MAX_ATTEMPTS` (raw text; parsed with the source's
    /// leading-integer semantics).
    pub env_max_attempts: Option<&'a str>,
    /// Shared-config profile `retry_mode` entry.
    pub profile_retry_mode: Option<&'a str>,
    /// Shared-config profile `max_attempts` entry.
    pub profile_max_attempts: Option<&'a str>,
}

/// Resolve the retry configuration from scoped inputs using the source's
/// precedence: environment, then shared-config profile, then default
/// (standard, legacy constants, 3 total attempts). Empty values are skipped
/// like the source's falsy checks; an unknown mode or unparsable attempt
/// count is a typed failure.
pub fn resolve_smithy_retry_config(
    inputs: ScopedRetryInputs<'_>,
) -> Result<SmithyRetryConfig, AwsRetryConfigError> {
    let mode_value = non_empty(inputs.env_retry_mode).or(non_empty(inputs.profile_retry_mode));
    let mode = match mode_value {
        Some(value) => SmithyRetryMode::parse(value)?,
        None => SmithyRetryMode::Standard,
    };
    let attempts_value = non_empty(inputs.env_max_attempts)
        .map(|value| (value, MaxAttemptsSource::ScopedEnvironment))
        .or(non_empty(inputs.profile_max_attempts)
            .map(|value| (value, MaxAttemptsSource::SharedProfile)));
    let max_attempts = match attempts_value {
        Some((value, source)) => {
            parse_max_attempts(value).ok_or(AwsRetryConfigError::InvalidMaxAttempts {
                source,
                value: value.to_owned(),
            })?
        }
        None => 3,
    };
    Ok(SmithyRetryConfig {
        mode,
        smithy_2026: false,
        max_attempts,
    })
}

/// Parse `max_attempts` with the source's `parseInt` semantics: a leading
/// (optionally signed) run of digits, so `"3x"` is 3 and `"x"` fails.
/// Negative values clamp to zero extra sends (the source's negative totals
/// never admit a retry either; opi is a typed configuration, not a string).
fn parse_max_attempts(value: &str) -> Option<u32> {
    let trimmed = value.trim();
    let mut chars = trimmed.char_indices();
    let mut end = None;
    if let Some((_, first)) = chars.next() {
        if !first.is_ascii_digit() && first != '+' && first != '-' {
            return None;
        }
    } else {
        return None;
    }
    for (index, ch) in chars {
        if !ch.is_ascii_digit() {
            end = Some(index);
            break;
        }
    }
    let digits = &trimmed[..end.unwrap_or(trimmed.len())];
    let parsed: i64 = digits.parse().ok()?;
    parsed.clamp(0, u32::MAX as i64).try_into().ok()
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.trim().is_empty())
}

/// Extract the `retry_mode` and `max_attempts` entries of one shared-config
/// profile section (`[default]` when no profile is selected). This is the
/// minimal reader for retry knobs; it does not interpret auth entries.
pub fn read_profile_retry_entries(
    config_text: &str,
    profile: Option<&str>,
) -> (Option<String>, Option<String>) {
    let section = profile
        .map(|name| format!("[profile {name}]"))
        .unwrap_or_else(|| "[default]".to_owned());
    let mut retry_mode = None;
    let mut max_attempts = None;
    let mut in_section = false;
    for line in config_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = trimmed == section;
            continue;
        }
        if !in_section {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "retry_mode" if retry_mode.is_none() => retry_mode = Some(value.to_owned()),
            "max_attempts" if max_attempts.is_none() => max_attempts = Some(value.to_owned()),
            _ => {}
        }
    }
    (retry_mode, max_attempts)
}

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

/// Classify one observed attempt failure for a Smithy source. Structured
/// service faults classify exactly; the neutral transport taxonomy maps to
/// the source's transient families; everything else is client-terminal.
pub fn classify_attempt_failure(cause: &ProviderError) -> SmithyErrorClass {
    match cause {
        ProviderError::ServiceFault {
            status, error_name, ..
        } => classify_service_failure(*status, error_name.as_deref()),
        ProviderError::RateLimited { .. } => SmithyErrorClass::Throttling,
        ProviderError::Timeout => SmithyErrorClass::Transient,
        ProviderError::Network(_) => SmithyErrorClass::Transient,
        _ => SmithyErrorClass::ClientError,
    }
}

/// The token cost one retry of the given class consumes.
///
/// Legacy constants charge TRANSIENT 10 and THROTTLING 5 (the modified cost
/// type is TRANSIENT); the 2026 constants charge THROTTLING 5 and every
/// other retryable class 14.
pub fn smithy_capacity_cost(class: SmithyErrorClass, smithy_2026: bool) -> u32 {
    match (class, smithy_2026) {
        (SmithyErrorClass::Throttling, true) => 5,
        (_, true) => 14,
        (SmithyErrorClass::Transient, false) => 10,
        (_, false) => 5,
    }
}

/// The exponential base for the given class and flavor: 500ms for throttling
/// (1000ms in 2026), 100ms otherwise (50ms in 2026).
fn smithy_delay_base(class: SmithyErrorClass, smithy_2026: bool) -> u64 {
    match (class, smithy_2026) {
        (SmithyErrorClass::Throttling, true) => 1_000,
        (SmithyErrorClass::Throttling, false) => 500,
        (_, true) => 50,
        (_, false) => 100,
    }
}

/// Compute the Smithy V2 backoff delay for one admitted retry.
///
/// `completed_retries` is the zero-based count of retries already performed.
/// The delay is `floor(u * min(base * 2^completed, 20s))` with full jitter
/// `u in [0, 1)`. A server hint adjusts it with the bounded rule
/// `max(delay, min(hint, delay + 5s))` — the helper family's 60-second
/// advisory bound is never applied here.
pub fn smithy_backoff_delay_ms(
    completed_retries: u32,
    class: SmithyErrorClass,
    smithy_2026: bool,
    hint_ms: Option<u64>,
    unit_jitter: f64,
) -> u64 {
    let base = smithy_delay_base(class, smithy_2026);
    let raw = base.saturating_mul(1u64.checked_shl(completed_retries).unwrap_or(u64::MAX));
    let capped = raw.min(SMITHY_MAX_RETRY_DELAY_MS);
    let delay = (unit_jitter.clamp(0.0, 1.0) * capped as f64).floor() as u64;
    match hint_ms {
        Some(hint) => delay.max(hint.min(delay + SMITHY_HINT_HEADROOM_MS)),
        None => delay,
    }
}

/// Parse the Smithy retry-after hint from one response's headers, in
/// milliseconds.
///
/// `retry-after` accepts numeric seconds, an IMF-fixdate, or the source's
/// trailing ` GMT, <seconds>` form; `x-amz-retry-after` is milliseconds.
/// The source scans headers in its response-object enumeration order; a
/// Rust header map carries no stable order, so opi checks `retry-after`
/// first and `x-amz-retry-after` second (documented divergence). An
/// unparsable value falls through to the other candidate like the source's
/// per-header loop.
pub fn parse_smithy_retry_after(
    headers: &reqwest::header::HeaderMap,
    now_unix: u64,
) -> Option<u64> {
    if let Some(value) = headers.get("retry-after").and_then(|v| v.to_str().ok())
        && let Some(ms) = parse_retry_after_value(value, now_unix)
    {
        return Some(ms);
    }
    if let Some(value) = headers
        .get("x-amz-retry-after")
        .and_then(|v| v.to_str().ok())
        && let Ok(ms) = value.trim().parse::<u64>()
    {
        return Some(ms);
    }
    None
}

fn parse_retry_after_value(value: &str, now_unix: u64) -> Option<u64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    // The source's trailing " GMT, <seconds>" form is checked only when the
    // value does not end with "GMT" itself.
    if !value.ends_with("GMT") {
        if let Some((_, seconds)) = value.rsplit_once(" GMT, ")
            && seconds.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
            && !seconds.is_empty()
        {
            let seconds: f64 = seconds.parse().ok()?;
            return Some((seconds * 1000.0) as u64);
        }
        if let Ok(seconds) = value.parse::<f64>() {
            return Some((seconds * 1000.0) as u64);
        }
        return None;
    }
    let date = crate::retry::parse_imf_fixdate(value)?;
    date.checked_sub(now_unix).map(|delta| delta * 1000)
}

/// The per-logical-call retry quota (fixed 500-token lifetime).
#[derive(Debug, Clone)]
pub struct SmithyRetryQuota {
    capacity: u32,
}

impl Default for SmithyRetryQuota {
    fn default() -> Self {
        Self {
            capacity: SMITHY_INITIAL_RETRY_TOKENS,
        }
    }
}

impl SmithyRetryQuota {
    /// The remaining capacity.
    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    /// Whether one retry of the given cost fits in the remaining capacity.
    pub fn has_capacity(&self, cost: u32) -> bool {
        self.capacity >= cost
    }

    /// Consume one retry's cost. Callers check
    /// [`has_capacity`](Self::has_capacity) first.
    pub fn acquire(&mut self, cost: u32) {
        self.capacity = self.capacity.saturating_sub(cost);
    }

    /// Refund after a successful outcome: the last spent cost, or the
    /// no-retry increment (1) when nothing was spent this call. Capped at
    /// the initial capacity.
    pub fn refund(&mut self, spent: Option<u32>) {
        let increment = spent.unwrap_or(1);
        self.capacity = self
            .capacity
            .saturating_add(increment)
            .min(SMITHY_INITIAL_RETRY_TOKENS);
    }
}

/// Time source for the adaptive rate limiter, in Unix seconds.
///
/// `Wall` reads the system clock. `Manual` pops scripted ticks (an empty
/// queue repeats the last tick) so deterministic tests can drive the
/// measured-rate buckets and cubic recovery exactly.
#[derive(Debug, Clone, Default)]
pub enum SmithyClock {
    /// The system clock (production default).
    #[default]
    Wall,
    /// A scripted clock for deterministic tests (nanosecond ticks).
    Manual(Arc<Mutex<VecDeque<u64>>>),
}

impl PartialEq for SmithyClock {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Wall, Self::Wall) => true,
            (Self::Manual(a), Self::Manual(b)) => {
                // Compare the scripted contents, not the Arc identity.
                let a = a.lock().expect("smithy clock poisoned");
                let b = b.lock().expect("smithy clock poisoned");
                *a == *b
            }
            _ => false,
        }
    }
}

impl Eq for SmithyClock {}

impl SmithyClock {
    /// Build a manual clock from scripted ticks given in seconds.
    pub fn manual(ticks_secs: impl IntoIterator<Item = f64>) -> Self {
        let ticks = ticks_secs
            .into_iter()
            .map(|secs| (secs * 1e9) as u64)
            .collect();
        Self::Manual(Arc::new(Mutex::new(ticks)))
    }

    fn now_secs(&self) -> f64 {
        match self {
            Self::Wall => std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or_default(),
            Self::Manual(ticks) => {
                let mut queue = ticks.lock().expect("smithy clock poisoned");
                if queue.len() > 1 {
                    queue.pop_front();
                }
                // An empty script repeats the final tick.
                let nanos = queue.front().copied().unwrap_or(0);
                nanos as f64 / 1e9
            }
        }
    }
}

/// The source's adaptive client-side rate limiter: an EWMA-measured request
/// rate over half-second buckets, a CUBIC send-rate controller that cuts the
/// rate multiplicatively on throttling and grows it cubically afterwards,
/// and a token bucket that gates sends once enabled.
///
/// The limiter is disabled until the first throttling outcome; before that,
/// send tokens are free. All math mirrors the pinned source (beta 0.7,
/// scale constant 0.4, smoothing 0.8, minimum capacity 1, minimum fill rate
/// 0.5, eight-decimal precision).
#[derive(Debug, Clone)]
pub struct AdaptiveRateLimiter {
    clock: SmithyClock,
    beta: f64,
    min_capacity: f64,
    min_fill_rate: f64,
    scale_constant: f64,
    smooth: f64,
    enabled: bool,
    available_tokens: f64,
    last_max_rate: f64,
    measured_tx_rate: f64,
    request_count: f64,
    fill_rate: f64,
    last_throttle_time: f64,
    last_timestamp: f64,
    last_tx_rate_bucket: f64,
    max_capacity: f64,
    time_window: f64,
}

impl AdaptiveRateLimiter {
    /// Construct the limiter with the source's default parameters.
    pub fn new(clock: SmithyClock) -> Self {
        let now = clock.now_secs();
        let bucket_now = clock.now_secs();
        Self {
            clock,
            beta: 0.7,
            min_capacity: 1.0,
            min_fill_rate: 0.5,
            scale_constant: 0.4,
            smooth: 0.8,
            enabled: false,
            available_tokens: 0.0,
            last_max_rate: 0.0,
            measured_tx_rate: 0.0,
            request_count: 0.0,
            fill_rate: 0.5,
            last_throttle_time: now,
            last_timestamp: 0.0,
            last_tx_rate_bucket: bucket_now.floor(),
            max_capacity: 1.0,
            time_window: 0.0,
        }
    }

    /// Whether the token bucket is enabled (after the first throttle).
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// The current fill rate (sends per second).
    pub fn fill_rate(&self) -> f64 {
        self.fill_rate
    }

    /// The currently available send tokens.
    pub fn available_tokens(&self) -> f64 {
        self.available_tokens
    }

    /// The EWMA-measured transmit rate.
    pub fn measured_tx_rate(&self) -> f64 {
        self.measured_tx_rate
    }

    /// The clock's current seconds (used to schedule the post-backoff
    /// send-token wait at the moment the backoff will have elapsed).
    pub fn wall_secs(&self) -> f64 {
        self.clock.now_secs()
    }

    /// The milliseconds to wait for one send token at `at_secs`, consuming
    /// it. A disabled bucket returns zero and consumes nothing.
    pub fn send_token_wait_ms(&mut self, at_secs: f64) -> u64 {
        if !self.enabled {
            return 0;
        }
        self.refill(at_secs);
        if self.available_tokens < 1.0 {
            let deficit = 1.0 - self.available_tokens;
            self.available_tokens -= 1.0;
            return ((deficit / self.fill_rate) * 1000.0).ceil() as u64;
        }
        self.available_tokens -= 1.0;
        0
    }

    /// Record one outcome, updating the measured rate and the CUBIC send
    /// rate. A throttling outcome cuts the rate and enables the bucket; a
    /// success outcome grows the rate cubically.
    pub fn update_client_sending_rate(&mut self, throttled: bool) {
        self.update_measured_rate();
        let now = self.clock.now_secs();
        let calculated_rate = if throttled {
            let rate_to_use = if !self.enabled {
                self.measured_tx_rate
            } else {
                self.measured_tx_rate.min(self.fill_rate)
            };
            self.last_max_rate = rate_to_use;
            self.calculate_time_window();
            self.last_throttle_time = now;
            self.enabled = true;
            self.cubic_throttle(rate_to_use)
        } else {
            self.calculate_time_window();
            self.cubic_success(now)
        };
        let new_rate = calculated_rate.min(2.0 * self.measured_tx_rate);
        self.update_token_bucket_rate(new_rate);
    }

    fn refill(&mut self, timestamp: f64) {
        if self.last_timestamp == 0.0 {
            self.last_timestamp = timestamp;
            return;
        }
        let fill_amount = (timestamp - self.last_timestamp) * self.fill_rate;
        self.available_tokens = self.max_capacity.min(self.available_tokens + fill_amount);
        self.last_timestamp = timestamp;
    }

    fn calculate_time_window(&mut self) {
        self.time_window =
            get_precise(((self.last_max_rate * (1.0 - self.beta)) / self.scale_constant).cbrt());
    }

    fn cubic_throttle(&self, rate_to_use: f64) -> f64 {
        get_precise(rate_to_use * self.beta)
    }

    fn cubic_success(&self, timestamp: f64) -> f64 {
        get_precise(
            self.scale_constant * (timestamp - self.last_throttle_time - self.time_window).powi(3)
                + self.last_max_rate,
        )
    }

    fn update_token_bucket_rate(&mut self, new_rate: f64) {
        let now = self.clock.now_secs();
        self.refill(now);
        self.fill_rate = new_rate.max(self.min_fill_rate);
        self.max_capacity = new_rate.max(self.min_capacity);
        self.available_tokens = self.available_tokens.min(self.max_capacity);
    }

    fn update_measured_rate(&mut self) {
        let t = self.clock.now_secs();
        let time_bucket = (t * 2.0).floor() / 2.0;
        self.request_count += 1.0;
        if time_bucket > self.last_tx_rate_bucket {
            let current_rate = self.request_count / (time_bucket - self.last_tx_rate_bucket);
            self.measured_tx_rate = get_precise(
                current_rate * self.smooth + self.measured_tx_rate * (1.0 - self.smooth),
            );
            self.request_count = 0.0;
            self.last_tx_rate_bucket = time_bucket;
        }
    }
}

/// The source rounds controller values to eight decimals.
fn get_precise(value: f64) -> f64 {
    (value * 1e8).round() / 1e8
}
