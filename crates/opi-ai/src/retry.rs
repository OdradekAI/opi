//! Retry/backoff/rate-limit support.
//!
//! Task 2.15 introduced the header parsing, exponential backoff calculation,
//! and product-facing [`RetryConfig`]. Task 20.4 makes this module the one
//! acquisition-retry owner (AIR-007 / P20-ACQUISITION):
//! [`PreparedProviderCall`](crate::Models::prepare_call) freezes an
//! [`AcquisitionPolicy`] per logical call, and the crate-private acquisition
//! core owns the per-payload-variant send counters, retry admission,
//! readiness deadline, and the sealed states that forbid further attempts.
//! Consumers schedule attempts through
//! [`start_attempt`](crate::models::PreparedProviderCall::start_attempt) and
//! read [`RetryAdmission`] decisions instead of re-classifying failures.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::provider::ProviderError;

/// Configuration for automatic retry of retryable provider errors.
#[derive(Debug, Clone, PartialEq)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (not counting the initial request).
    pub max_attempts: u32,
    /// Base delay in milliseconds for exponential backoff.
    pub initial_delay_ms: u64,
    /// Upper bound on delay in milliseconds.
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 60_000,
        }
    }
}

impl RetryConfig {
    /// Calculate the delay for a given retry attempt.
    ///
    /// If `retry_after_ms` is provided (from provider headers), it is used
    /// (capped to `max_delay_ms`). Otherwise, exponential backoff is applied:
    /// `initial_delay_ms * 2^attempt`, capped at `max_delay_ms`.
    pub fn delay_for_attempt(&self, attempt: u32, retry_after_ms: Option<u64>) -> u64 {
        let raw = match retry_after_ms {
            Some(ms) => ms,
            None => calculate_backoff_delay(attempt, self.initial_delay_ms, self.max_delay_ms),
        };
        raw.min(self.max_delay_ms)
    }

    /// Whether a retry should be attempted at the given attempt number.
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }
}

/// Calculate exponential backoff delay: `base_ms * 2^attempt`, capped at `max_delay_ms`.
pub fn calculate_backoff_delay(attempt: u32, base_ms: u64, max_delay_ms: u64) -> u64 {
    let multiplier = 1u64.checked_shl(attempt).unwrap_or(u64::MAX);
    base_ms.saturating_mul(multiplier).min(max_delay_ms)
}

/// Parse retry delay from HTTP response headers.
///
/// Checks headers in priority order:
/// 1. `retry-after` — seconds value or HTTP-date
/// 2. `x-ratelimit-reset` — Unix timestamp
///
/// Returns delay in milliseconds, or `None` if no usable header is found.
pub fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    // Priority 1: Retry-After header
    if let Some(val) = headers.get("retry-after")
        && let Ok(s) = val.to_str()
    {
        // Try seconds first
        if let Ok(secs) = s.parse::<f64>() {
            return Some((secs * 1000.0) as u64);
        }
        // Try HTTP-date
        if let Some(delay_ms) = parse_http_date_delay(s) {
            return Some(delay_ms);
        }
    }

    // Priority 2: x-ratelimit-reset (Unix timestamp)
    if let Some(val) = headers.get("x-ratelimit-reset")
        && let Ok(s) = val.to_str()
        && let Ok(timestamp) = s.parse::<f64>()
    {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        let delay_secs = timestamp - now;
        if delay_secs > 0.0 {
            return Some((delay_secs * 1000.0) as u64);
        }
    }

    None
}

/// Parse an HTTP-date string and return the delay in milliseconds from now.
///
/// Supports IMF-fixdate format: `Day, DD Mon YYYY HH:MM:SS GMT`
/// Example: `Fri, 23 May 2026 12:00:00 GMT`
///
/// Returns `None` if the date cannot be parsed or is in the past.
pub fn parse_http_date_delay(s: &str) -> Option<u64> {
    let ts = parse_imf_fixdate(s)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    if ts > now {
        Some((ts - now) * 1000)
    } else {
        None
    }
}

/// Parse IMF-fixdate format: `Day, DD Mon YYYY HH:MM:SS GMT`
pub(crate) fn parse_imf_fixdate(s: &str) -> Option<u64> {
    // Expected format: "Fri, 23 May 2026 12:00:00 GMT"
    let s = s.trim();
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 2 {
        return None;
    }
    let rest = parts[1].trim();
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    // Expected: ["DD", "Mon", "YYYY", "HH:MM:SS", "GMT"]
    if tokens.len() != 5 {
        return None;
    }
    let day: u32 = tokens[0].parse().ok()?;
    let month = parse_month(tokens[1])?;
    let year: u32 = tokens[2].parse().ok()?;
    let time_parts: Vec<&str> = tokens[3].split(':').collect();
    if time_parts.len() != 3 {
        return None;
    }
    let hour: u32 = time_parts[0].parse().ok()?;
    let minute: u32 = time_parts[1].parse().ok()?;
    let second: u32 = time_parts[2].parse().ok()?;
    if tokens[4] != "GMT" {
        return None;
    }

    // Convert to Unix timestamp using civil date calculation
    Some(date_to_unix(year, month, day, hour, minute, second))
}

/// Map three-letter month abbreviation to month number (1-12).
fn parse_month(s: &str) -> Option<u32> {
    match s {
        "Jan" => Some(1),
        "Feb" => Some(2),
        "Mar" => Some(3),
        "Apr" => Some(4),
        "May" => Some(5),
        "Jun" => Some(6),
        "Jul" => Some(7),
        "Aug" => Some(8),
        "Sep" => Some(9),
        "Oct" => Some(10),
        "Nov" => Some(11),
        "Dec" => Some(12),
        _ => None,
    }
}

/// Convert a civil date/time to a Unix timestamp (seconds since epoch, UTC).
fn date_to_unix(year: u32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> u64 {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let y = if month <= 2 { year - 1 } else { year } as i64;
    let m = if month <= 2 { month + 9 } else { month - 3 } as i64;
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    let secs = days * 86400 + hour as i64 * 3600 + minute as i64 * 60 + second as i64;
    secs as u64
}

// ---------------------------------------------------------------------------
// Acquisition owner (task 20.4, AIR-007 / P20-ACQUISITION)
// ---------------------------------------------------------------------------

/// Which retry-family rules the acquisition owner applies (task 20.20).
///
/// `Helper` keeps the AIR-007 helper-family behavior (408/409/429/>=500 &
/// `x-should-retry`, `min(500ms * 2^i, 8s)` backoff with `[0.75, 1.0)`
/// jitter, 60-second advisory-hint bound). `Smithy` applies the pinned AWS
/// Smithy V2 modes for Bedrock through
/// [`SmithyRetryConfig`](crate::bedrock::retry::SmithyRetryConfig):
/// exact error-trait/network classification, the per-call quota, the
/// bounded `+5s` hint rule, and (adaptive) the CUBIC client-side rate
/// limiter. The helper 60-second hint bound and helper jitter are never
/// applied to Smithy modes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetrySourcePolicy {
    /// Helper/Codex family rules (the default for every non-AWS wire).
    Helper,
    /// AWS Smithy V2 rules with the frozen mode/flavor/attempt budget.
    Smithy(crate::bedrock::retry::SmithyRetryConfig),
}

/// Per-logical-call acquisition policy frozen at
/// [`Models::prepare_call_with_policy`](crate::Models::prepare_call_with_policy).
///
/// `max_retries` counts **extra sends per payload-variant acquisition
/// episode**: an ordinary HTTP acquisition performs at most `1 + max_retries`
/// sends of the frozen variant. The override order is per-call, then protocol
/// default (helper/Codex adapters default to `0`, Bedrock to its frozen
/// Smithy configuration); provider profiles are a later concern.
/// `max_retry_delay_ms` bounds only the server-advisory hint (default 60
/// seconds; helper sources only — Smithy sources use the source's bounded
/// `+5s` rule); an explicit zero disables that upper-bound check but still
/// honors hints. `jitter_seed` pins the backoff jitter for deterministic
/// tests; production leaves it `None` for per-call entropy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcquisitionPolicy {
    /// Extra sends admitted per payload variant (not counting the initial
    /// request).
    pub max_retries: u32,
    /// Upper bound applied to server-advisory retry hints, in milliseconds.
    /// `0` disables the bound (hints are used as-is). Helper sources only;
    /// Smithy sources always use the source's bounded `+5s` hint rule.
    pub max_retry_delay_ms: u64,
    /// Deterministic jitter seed for tests; `None` uses per-call entropy.
    pub jitter_seed: Option<u64>,
    /// Which retry-family rules apply (task 20.20). Defaults to the helper
    /// family; Bedrock providers freeze their resolved
    /// [`SmithyRetryConfig`](crate::bedrock::retry::SmithyRetryConfig)
    /// here.
    pub source: RetrySourcePolicy,
    /// Time source for the adaptive rate limiter when `source` is an
    /// adaptive Smithy config. Production uses the wall clock; deterministic
    /// tests script manual ticks.
    pub smithy_clock: crate::bedrock::retry::SmithyClock,
}

impl Default for AcquisitionPolicy {
    fn default() -> Self {
        Self {
            max_retries: 0,
            max_retry_delay_ms: 60_000,
            jitter_seed: None,
            source: RetrySourcePolicy::Helper,
            smithy_clock: crate::bedrock::retry::SmithyClock::default(),
        }
    }
}

impl AcquisitionPolicy {
    /// A policy admitting `max_retries` extra sends with the protocol-default
    /// hint bound and production entropy.
    pub fn with_max_retries(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Self::default()
        }
    }
}

/// The SDK-owned decision that another acquisition attempt is admitted.
///
/// Emitted by the acquisition owner after an observed retryable attempt failure
/// while the frozen per-variant budget still has room. `retry_index` is
/// 1-based (the first retry is `1`); `delay_ms` is the source-qualified wait
/// the caller must observe before the next
/// [`start_attempt`](crate::models::PreparedProviderCall::start_attempt)
/// becomes ready.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryAdmission {
    /// 1-based index of the admitted extra send.
    pub retry_index: u32,
    /// Milliseconds to wait before the next attempt start is ready.
    pub delay_ms: u64,
}

/// Why a prepared call no longer admits acquisition attempts.
///
/// Observable through
/// [`acquisition_seal`](crate::models::PreparedProviderCall::acquisition_seal)
/// and the [`crate::ModelsError::CallSealed`] start refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AcquisitionSeal {
    /// A streaming response was successfully handed off (at least one stream
    /// event reached the caller). Body/EOF/parser failures after handoff are
    /// final and never re-enter acquisition retry.
    PostHandoff,
    /// An attempt was dispatched but dropped without an observed closed
    /// outcome, so its effect is unknown and further scheduling is refused.
    UnknownDispatch,
    /// The frozen per-variant retry budget is spent.
    Exhausted,
    /// The attempt reached a terminal closed outcome (success, a classified
    /// non-retryable failure, or end-of-stream) that admits no further send.
    Finished,
}

impl AcquisitionSeal {
    /// Stable, redacted machine-readable name for diagnostics.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PostHandoff => "post-handoff",
            Self::UnknownDispatch => "unknown-dispatch",
            Self::Exhausted => "exhausted",
            Self::Finished => "finished",
        }
    }
}

/// Refusal reason returned by [`AcquisitionCore::check_start`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetryStartError {
    /// A prior attempt still owns the call's execution lease.
    AlreadyActive,
    /// The call is sealed; no further attempt may be dispatched.
    Sealed(AcquisitionSeal),
    /// The admitted retry is not ready yet; the caller must keep waiting.
    NotReady { retry_ready_in_ms: u64 },
}

/// The per-logical-call acquisition state machine (crate-private).
///
/// One instance lives inside each `PreparedProviderCall` behind an `Arc` and
/// is updated synchronously from the attempt stream's `poll_next` and from
/// `start_attempt`. It owns the payload-variant counters, the retry admission
/// decision (classification + hint-aware delay with jitter), the readiness
/// deadline that enforces the caller's wait, and the sticky sealed state.
/// No method performs IO.
pub(crate) struct AcquisitionCore {
    inner: Mutex<AcquisitionInner>,
}

/// Per-attempt Smithy mode state (task 20.20). One instance per logical
/// call; the quota and rate limiter never outlive the call.
struct SmithyAttemptState {
    config: crate::bedrock::retry::SmithyRetryConfig,
    quota: crate::bedrock::retry::SmithyRetryQuota,
    limiter: crate::bedrock::retry::AdaptiveRateLimiter,
    completed_retries: u32,
    spent: Option<u32>,
    refunded: bool,
}

impl SmithyAttemptState {
    fn new(
        config: crate::bedrock::retry::SmithyRetryConfig,
        clock: crate::bedrock::retry::SmithyClock,
    ) -> Self {
        // The limiter only gates sends in adaptive mode; a dormant instance
        // keeps the state shape uniform for standard mode.
        let limiter = crate::bedrock::retry::AdaptiveRateLimiter::new(clock);
        Self {
            config,
            quota: crate::bedrock::retry::SmithyRetryQuota::default(),
            limiter,
            completed_retries: 0,
            spent: None,
            refunded: false,
        }
    }

    /// The limiter's current wall-clock seconds (used to schedule the
    /// post-backoff send-token wait).
    fn limiter_wall_secs(&self) -> f64 {
        self.limiter.wall_secs()
    }
}

struct AcquisitionInner {
    policy: AcquisitionPolicy,
    /// The single execution lease shares the lock with admission and sealing.
    active: bool,
    /// Index of the current payload variant: 0 ordinary, 1 the single
    /// rebuilt strict downgrade (task 20.11).
    variant_index: u32,
    /// Total payload variants dispatched at least once (0 before the first
    /// dispatch, 1 afterwards). Shared across variants; never reset.
    variants_used: u32,
    /// A pending strict-downgrade variant admission: set by a
    /// source-qualified `StrictToolLimits` failure on the first variant,
    /// consumed by the next `start_attempt` (V-34: at most one rebuilt
    /// variant per logical call).
    pending_strict_variant: Option<&'static str>,
    /// 1-based sends within the current variant's episode (the
    /// `amz-sdk-request` attempt number); resets with the variant.
    variant_dispatches: u32,
    /// Extra sends admitted in the current variant. The shared counter is
    /// what callers cannot reset.
    retries_admitted: u32,
    /// Attempts whose start was reserved (1-based wire attempt numbers).
    dispatches: u32,
    /// Deadline before which `start_attempt` refuses dispatch.
    ready_at: Option<Instant>,
    /// The admission awaiting the caller's next start, if any.
    admission: Option<RetryAdmission>,
    /// Sticky sealed state; the first seal wins.
    sealed: Option<AcquisitionSeal>,
    jitter: JitterRng,
    /// Smithy mode state when the frozen policy selects the AWS family.
    smithy: Option<SmithyAttemptState>,
}

impl AcquisitionCore {
    pub(crate) fn new(policy: AcquisitionPolicy) -> Self {
        let jitter = match policy.jitter_seed {
            Some(seed) => JitterRng(seed),
            None => JitterRng::from_entropy(),
        };
        let smithy = match &policy.source {
            RetrySourcePolicy::Helper => None,
            RetrySourcePolicy::Smithy(config) => Some(SmithyAttemptState::new(
                config.clone(),
                policy.smithy_clock.clone(),
            )),
        };
        Self {
            inner: Mutex::new(AcquisitionInner {
                policy,
                active: false,
                variant_index: 0,
                variants_used: 0,
                pending_strict_variant: None,
                variant_dispatches: 0,
                retries_admitted: 0,
                dispatches: 0,
                ready_at: None,
                admission: None,
                sealed: None,
                jitter,
                smithy,
            }),
        }
    }

    /// Check admission and reserve the single execution lease in one critical
    /// section. Sealing/readiness cannot change between admission and taking
    /// the lease. Refusals consume neither the lease nor a send.
    pub(crate) fn check_start(&self) -> Result<(), RetryStartError> {
        let mut inner = self.inner.lock().expect("acquisition core poisoned");
        if let Some(seal) = inner.sealed {
            return Err(RetryStartError::Sealed(seal));
        }
        if inner.active {
            return Err(RetryStartError::AlreadyActive);
        }
        if inner.pending_strict_variant.is_some() {
            // Consume the single strict-downgrade admission (V-34/V-36):
            // the rebuilt variant dispatches immediately with its own fresh
            // per-variant budget; the total bound stays 2x(1+N) because
            // `variants_used` caps at 2 and each variant admits at most
            // `max_retries` extra sends.
            inner.pending_strict_variant = None;
            inner.variant_index = 1;
            inner.variants_used = inner.variants_used.max(2);
            inner.retries_admitted = 0;
            inner.ready_at = None;
            inner.admission = None;
            inner.variant_dispatches = 0;
        }
        if let Some(ready_at) = inner.ready_at {
            let now = Instant::now();
            if now < ready_at {
                return Err(RetryStartError::NotReady {
                    retry_ready_in_ms: (ready_at - now).as_millis() as u64,
                });
            }
            inner.ready_at = None;
        }
        inner.dispatches += 1;
        inner.variant_dispatches += 1;
        inner.active = true;
        Ok(())
    }

    /// The wire-visible attempt context for the reserved attempt: 1-based
    /// attempt number, the frozen total budget, and the episode's invocation
    /// id (task 20.20, `amz-sdk-request`/`amz-sdk-invocation-id`).
    pub(crate) fn attempt_context(&self, invocation_id: &str) -> crate::provider::AttemptContext {
        let inner = self.inner.lock().expect("acquisition core poisoned");
        let max_attempts = match &inner.policy.source {
            RetrySourcePolicy::Helper => inner.policy.max_retries.saturating_add(1),
            RetrySourcePolicy::Smithy(config) => config.max_attempts,
        };
        let payload_variant = match inner.variant_index {
            0 => crate::provider::PayloadVariant::Ordinary,
            _ => crate::provider::PayloadVariant::AnthropicStrictDowngrade,
        };
        crate::provider::AttemptContext {
            invocation_id: invocation_id.to_owned(),
            attempt: inner.variant_dispatches.max(1),
            max_attempts,
            payload_variant,
        }
    }

    /// Release an observed terminal or an unpolled lease. Terminal state is
    /// recorded before this release, under the same acquisition owner.
    pub(crate) fn release_attempt(&self) {
        self.inner.lock().expect("acquisition core poisoned").active = false;
    }

    /// Record that an attempt was dispatched (the first dispatch marks the
    /// payload variant as used).
    pub(crate) fn note_dispatch(&self) {
        let mut inner = self.inner.lock().expect("acquisition core poisoned");
        if inner.variants_used == inner.variant_index {
            inner.variants_used += 1;
        }
    }

    /// Record a successful streaming-response handoff: the first delivered
    /// stream item seals acquisition retry, even before the first public
    /// content event. In Smithy mode the success also refunds the retry
    /// quota and (adaptive) updates the measured/CUBIC send rate.
    pub(crate) fn note_handoff(&self) {
        let mut inner = self.inner.lock().expect("acquisition core poisoned");
        if let Some(state) = inner.smithy.as_mut()
            && !state.refunded
        {
            state.refunded = true;
            if state.config.mode == crate::bedrock::retry::SmithyRetryMode::Adaptive {
                state.limiter.update_client_sending_rate(false);
            }
            state.quota.refund(state.spent);
        }
        if inner.sealed.is_none() {
            inner.sealed = Some(AcquisitionSeal::PostHandoff);
        }
    }

    /// Record an attempt dropped after dispatch without an observed closed
    /// outcome: the effect is unknown and further scheduling is refused.
    pub(crate) fn note_unknown_dispatch(&self) {
        let mut inner = self.inner.lock().expect("acquisition core poisoned");
        if inner.sealed.is_none() {
            inner.sealed = Some(AcquisitionSeal::UnknownDispatch);
        }
    }

    /// Record a terminal closed outcome that admits no further send (success,
    /// end-of-stream, or a failure classified below).
    pub(crate) fn note_finished(&self) {
        let mut inner = self.inner.lock().expect("acquisition core poisoned");
        if inner.sealed.is_none() {
            inner.sealed = Some(AcquisitionSeal::Finished);
        }
    }

    /// Classify an observed attempt failure. A retryable cause with remaining
    /// per-variant budget produces the next [`RetryAdmission`] and sets the
    /// readiness deadline; anything else seals the call (`Exhausted` when the
    /// budget is spent, `Finished` for a classified final failure).
    pub(crate) fn note_failure(&self, cause: &ProviderError) {
        let mut inner = self.inner.lock().expect("acquisition core poisoned");
        if cause.response_received() && inner.sealed.is_none() {
            inner.sealed = Some(AcquisitionSeal::PostHandoff);
        }
        if inner.sealed.is_some() {
            // Already sealed (for example post-handoff): the failure is final.
            inner.admission = None;
            return;
        }
        if inner.smithy.is_some() {
            Self::note_smithy_failure(&mut inner, cause);
            return;
        }
        if matches!(cause, ProviderError::StrictToolLimits { .. }) {
            // The source-qualified strict-limit rejection: on the first
            // variant it admits exactly one rebuilt downgrade variant
            // (dispatched without a readiness wait); anywhere else it is a
            // terminal failure.
            inner.admission = None;
            if inner.variant_index == 0 && inner.variants_used <= 1 {
                inner.pending_strict_variant = Some("strict_tool_limits_exceeded");
            } else {
                inner.sealed = Some(AcquisitionSeal::Finished);
            }
            return;
        }
        if !cause.is_retryable() {
            inner.admission = None;
            inner.sealed = Some(AcquisitionSeal::Finished);
            return;
        }
        if inner.retries_admitted >= inner.policy.max_retries {
            inner.admission = None;
            inner.sealed = Some(AcquisitionSeal::Exhausted);
            return;
        }
        let hint = cause.retry_after_ms();
        let policy = inner.policy.clone();
        let delay_ms = retry_delay_ms(&policy, inner.retries_admitted + 1, hint, &mut inner.jitter);
        inner.retries_admitted += 1;
        inner.ready_at = Some(Instant::now() + Duration::from_millis(delay_ms));
        inner.admission = Some(RetryAdmission {
            retry_index: inner.retries_admitted,
            delay_ms,
        });
    }

    /// The Smithy branch of [`AcquisitionCore::note_failure`]: exact
    /// classification, the source's attempt/quota admission pair, the
    /// bounded hint rule, and (adaptive) the send-token wait folded into the
    /// readiness deadline.
    fn note_smithy_failure(inner: &mut AcquisitionInner, cause: &ProviderError) {
        use crate::bedrock::retry::SmithyErrorClass;

        let class = crate::bedrock::retry::classify_attempt_failure(cause);
        if !class.is_retryable() {
            inner.admission = None;
            inner.sealed = Some(AcquisitionSeal::Finished);
            return;
        }
        let state = inner
            .smithy
            .as_mut()
            .expect("smithy branch requires smithy state");
        let next = state.completed_retries + 1;
        if next >= state.config.max_attempts {
            inner.admission = None;
            inner.sealed = Some(AcquisitionSeal::Exhausted);
            return;
        }
        let cost = crate::bedrock::retry::smithy_capacity_cost(class, state.config.smithy_2026);
        if !state.quota.has_capacity(cost) {
            // The source refuses the refresh token here; the spent budget
            // surfaces with the last failure.
            inner.admission = None;
            inner.sealed = Some(AcquisitionSeal::Exhausted);
            return;
        }
        state.quota.acquire(cost);
        state.spent = Some(cost);
        let adaptive = state.config.mode == crate::bedrock::retry::SmithyRetryMode::Adaptive;
        if adaptive {
            state
                .limiter
                .update_client_sending_rate(class == SmithyErrorClass::Throttling);
        }
        let hint = cause.retry_after_ms();
        let unit = inner.jitter.fraction_unit();
        let mut delay_ms = crate::bedrock::retry::smithy_backoff_delay_ms(
            state.completed_retries,
            class,
            state.config.smithy_2026,
            hint,
            unit,
        );
        if adaptive {
            // The source sleeps the backoff first, then waits for a send
            // token with the bucket refilled at that later moment.
            let at_secs = state.limiter_wall_secs() + delay_ms as f64 / 1000.0;
            delay_ms += state.limiter.send_token_wait_ms(at_secs);
        }
        state.completed_retries += 1;
        inner.retries_admitted = state.completed_retries;
        inner.ready_at = Some(Instant::now() + Duration::from_millis(delay_ms));
        inner.admission = Some(RetryAdmission {
            retry_index: state.completed_retries,
            delay_ms,
        });
    }

    /// The admission awaiting the caller's next start, if any.
    pub(crate) fn admission(&self) -> Option<RetryAdmission> {
        self.inner
            .lock()
            .expect("acquisition core poisoned")
            .admission
    }

    /// The pending strict-downgrade variant admission, if the last observed
    /// failure was a source-qualified `StrictToolLimits` on the first
    /// variant (task 20.11). The caller dispatches it immediately through
    /// `start_attempt`; at most one exists per logical call.
    pub(crate) fn strict_variant_admission(&self) -> Option<&'static str> {
        let inner = self.inner.lock().expect("acquisition core poisoned");
        if inner.sealed.is_some() {
            return None;
        }
        inner.pending_strict_variant
    }

    /// The sticky sealed state, if the call no longer admits attempts.
    pub(crate) fn seal(&self) -> Option<AcquisitionSeal> {
        self.inner.lock().expect("acquisition core poisoned").sealed
    }
}

/// Compute the source-qualified wait before the admitted retry.
///
/// Server advisory hints win when present, bounded by
/// `policy.max_retry_delay_ms` unless that bound is explicitly zero
/// (zero disables only the upper-bound check; hints are still honored).
/// Without a hint the helper-family backoff runs: `min(500ms * 2^i, 8000ms)`
/// for zero-based retry index `i`, multiplied by a jitter factor drawn from
/// `[0.75, 1.0)`.
fn retry_delay_ms(
    policy: &AcquisitionPolicy,
    one_based_retry_index: u32,
    hint_ms: Option<u64>,
    jitter: &mut JitterRng,
) -> u64 {
    let Some(hint) = hint_ms else {
        let i = one_based_retry_index.saturating_sub(1);
        let base = 500u64
            .saturating_mul(1u64.checked_shl(i).unwrap_or(u64::MAX))
            .min(8_000);
        let percent = 75 + jitter.fraction_percent(); // [75, 99]
        return base.saturating_mul(percent) / 100;
    };
    if policy.max_retry_delay_ms == 0 {
        hint
    } else {
        hint.min(policy.max_retry_delay_ms)
    }
}

/// SplitMix64 jitter source. Not cryptographic; it only decorrelates backoff
/// waits. Seeded deterministically for tests, from process entropy otherwise.
/// Test-support preview: the first full-jitter unit draw (`[0, 1)`) a
/// seeded Smithy policy will consume. Mirrors the private [`JitterRng`]
/// sequence so fixtures can assert exact source delays.
#[doc(hidden)]
pub fn smithy_unit_jitter_preview(seed: u64) -> f64 {
    smithy_unit_jitter_draw(seed, 0)
}

/// Test-support: the zero-based `index`th full-jitter unit draw of a seeded
/// Smithy policy (the first retry consumes draw 0, the second draw 1, ...).
#[doc(hidden)]
pub fn smithy_unit_jitter_draw(seed: u64, index: u32) -> f64 {
    let mut rng = JitterRng(seed);
    let mut draw = 0.0;
    for _ in 0..=index {
        draw = rng.fraction_unit();
    }
    draw
}

struct JitterRng(u64);

impl JitterRng {
    fn from_entropy() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        Self(nanos ^ ((std::process::id() as u64) << 32))
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform jitter percentage in `[0, 24]`, so `75 + fraction_percent()`
    /// covers the source's `[0.75, 1.0)` backoff factor.
    fn fraction_percent(&mut self) -> u64 {
        self.next_u64() % 25
    }

    /// Uniform unit jitter in `[0, 1)` for the Smithy full-jitter backoff.
    fn fraction_unit(&mut self) -> f64 {
        // 53 random mantissa bits over 2^53 covers f64's exact integer
        // range, matching the source's Math.random() span.
        (self.next_u64() >> 11) as f64 / 9_007_199_254_740_992.0
    }
}

#[cfg(test)]
mod acquisition_tests {
    use super::{AcquisitionCore, AcquisitionPolicy, AcquisitionSeal, RetryStartError};
    use crate::provider::ProviderError;
    use std::sync::{Arc, Barrier};

    #[test]
    fn concurrent_start_checks_reserve_exactly_one_attempt() {
        let core = Arc::new(AcquisitionCore::new(AcquisitionPolicy::default()));
        let start = Arc::new(Barrier::new(3));
        let checked = Arc::new(Barrier::new(3));
        let workers = (0..2)
            .map(|_| {
                let core = core.clone();
                let start = start.clone();
                let checked = checked.clone();
                std::thread::spawn(move || {
                    start.wait();
                    let admitted = core.check_start().is_ok();
                    checked.wait();
                    admitted
                })
            })
            .collect::<Vec<_>>();
        start.wait();
        checked.wait();
        let admitted = workers
            .into_iter()
            .map(|worker| usize::from(worker.join().unwrap()))
            .sum::<usize>();
        assert_eq!(
            admitted, 1,
            "the acquisition check must reserve its lease atomically"
        );
    }

    #[test]
    fn released_attempt_cannot_bypass_the_recorded_failure_deadline_or_seal() {
        let core = AcquisitionCore::new(AcquisitionPolicy::with_max_retries(1));
        core.check_start().unwrap();
        assert!(matches!(
            core.check_start(),
            Err(RetryStartError::AlreadyActive)
        ));
        core.note_failure(&ProviderError::RateLimited {
            retry_after_ms: Some(10_000),
        });
        core.release_attempt();
        assert!(matches!(
            core.check_start(),
            Err(RetryStartError::NotReady { .. })
        ));
        core.note_handoff();
        assert!(matches!(
            core.check_start(),
            Err(RetryStartError::Sealed(AcquisitionSeal::PostHandoff))
        ));
    }
}

// ============================================================================
// Explicit whole-call retry helper (AIR-065 / V-57, task 20.52)
// ============================================================================
//
// This section is deliberately independent of the acquisition core above:
// the low-level machinery owns attempts *within* one logical call, while
// this helper owns the caller-opted restart of *whole* logical calls. Each
// invocation of the caller's `produce` closure is a new logical call (the
// helper never re-dispatches an old one, never mutates transcripts, never
// re-executes tools and never resets a prepared call's budget); omission or
// a disabled policy means exactly one call. Classification follows the
// pinned source pattern lists: quota/billing/usage-limit exhaustion vetoes
// first, then the transient families (provider load, gateway forwarding,
// network/transport, premature stream endings, explicit retry guidance).

/// Default cap for whole-call retry backoff delays (60s). Product recovery
/// policy stays with the host; the SDK helper only bounds a caller-supplied
/// schedule.
pub const DEFAULT_WHOLE_CALL_RETRY_MAX_DELAY_MS: u64 = 60_000;

/// Caller-owned whole-call retry schedule. `max_retries` counts EXTRA
/// complete calls; the initial call never counts as a retry.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WholeCallRetryPolicy {
    /// Opt-in switch; `false` (or omitting the policy) makes one call.
    pub enabled: bool,
    /// Extra complete calls after the initial one (0 = never retry).
    pub max_retries: u32,
    /// Base delay in ms; per-attempt delay is `base * 2^(attempt-1)`.
    pub base_delay_ms: u64,
    /// Optional per-call cap in ms; defaults to
    /// [`DEFAULT_WHOLE_CALL_RETRY_MAX_DELAY_MS`].
    pub max_delay_ms: Option<u64>,
}

/// Exponential whole-call backoff: `base_delay_ms * 2^(attempt-1)`
/// (saturating) capped at `max_delay_ms` or the 60s default.
pub fn whole_call_retry_delay_ms(policy: &WholeCallRetryPolicy, attempt: u32) -> u64 {
    let exponent = attempt.saturating_sub(1).min(63);
    let delay = policy.base_delay_ms.saturating_mul(1u64 << exponent);
    delay.min(
        policy
            .max_delay_ms
            .unwrap_or(DEFAULT_WHOLE_CALL_RETRY_MAX_DELAY_MS),
    )
}

/// Message-level classification for the whole-call helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WholeCallRetryClassification {
    /// A successful stop (Stop/Length/ToolUse/Deferred/...): never retried.
    Valid,
    /// Cancellation: terminal and never successful, never retried.
    Aborted,
    /// Transient provider/transport failure: a new call may help.
    Transient,
    /// Deterministic failure (including quota/billing/usage vetoes).
    Terminal,
}

fn non_retryable_limit_pattern() -> &'static regex::Regex {
    static PATTERN: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    PATTERN.get_or_init(|| {
        regex::RegexBuilder::new(
            [
                // Subscription/account limits surfaced as 429 JSON error types.
                "GoUsageLimitError",
                "FreeUsageLimitError",
                "Monthly usage limit reached",
                "available balance",
                // Generic quota/budget/billing exhaustion.
                "insufficient_quota",
                "out of budget",
                "quota exceeded",
                "billing",
                // Subscription-shared usage limits that reset after hours.
                "subscription_sharing_usage_limit_exceeded",
            ]
            .join("|")
            .as_str(),
        )
        .case_insensitive(true)
        .build()
        .expect("vetted literal pattern set")
    })
}

fn retryable_transient_pattern() -> &'static regex::Regex {
    static PATTERN: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    PATTERN.get_or_init(|| {
        regex::RegexBuilder::new(
            [
                // Provider load, HTTP status and server-side transients.
                "overloaded",
                "server_busy",
                "servers are currently busy",
                "currently experiencing high demand",
                "model is at capacity",
                "rate.?limit",
                "too many requests",
                "429",
                "500",
                "502",
                "503",
                "504",
                "520",
                "524",
                "service.?(?:is.?)?(?:temporarily.?)?unavailable",
                "server.?error",
                "internal.?error",
                // Transient upstream failures relayed by wrappers/gateways.
                "provider.?returned.?(?:an.?)?error",
                "exceeded request buffer limit while retrying upstream",
                "unable to forward upstream",
                // Network, proxy and fetch transport failures.
                "network.?error",
                "connection.?error",
                "connection.?refused",
                "connection.?lost",
                "other side closed",
                "fetch failed",
                "getaddrinfo",
                "ENOTFOUND",
                "EAI_AGAIN",
                "upstream.?connect",
                "reset before headers",
                "socket hang up",
                "socket connection was closed",
                "timed? out",
                "timeout",
                "terminated",
                // WebSocket transports reporting close/error text.
                "websocket.?closed",
                "websocket.?error",
                // Premature stream endings from SDKs and transports.
                "ended without",
                "stream ended before message_stop",
                "stream ended before a terminal response event",
                "http2 request did not get a response",
                "pending stream has been canceled",
                // Provider-requested retry-delay caps flow through the outer
                // policy so callers can surface or abort the backoff.
                "retry delay",
                // Explicit mid-stream retry guidance.
                "you can retry your request",
                "try your request again",
                "please retry your request",
                // gRPC based providers.
                "ResourceExhausted",
                // Subscription usage/user data temporarily unavailable.
                "subscription_sharing_usage_unavailable",
                "subscription_sharing_user_unavailable",
            ]
            .join("|")
            .as_str(),
        )
        .case_insensitive(true)
        .build()
        .expect("vetted literal pattern set")
    })
}

/// Classify a finished message for whole-call retry: quota/billing/usage
/// limits veto first; only then may a transient family classify as
/// retryable. Messages that are not failures are valid; aborts are terminal.
pub fn classify_whole_call_retry(
    message: &crate::message::AssistantMessage,
) -> WholeCallRetryClassification {
    match message.stop_reason {
        crate::stream::StopReason::Aborted => WholeCallRetryClassification::Aborted,
        crate::stream::StopReason::Error => {
            let Some(error_message) = message.error_message.as_deref() else {
                return WholeCallRetryClassification::Terminal;
            };
            if non_retryable_limit_pattern().is_match(error_message) {
                WholeCallRetryClassification::Terminal
            } else if retryable_transient_pattern().is_match(error_message) {
                WholeCallRetryClassification::Transient
            } else {
                WholeCallRetryClassification::Terminal
            }
        }
        _ => WholeCallRetryClassification::Valid,
    }
}

/// A retry-callback failure the caller's closure reported; the helper
/// surfaces it instead of continuing or swallowing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryCallbackFailure {
    /// Caller-supplied, already-sanitized detail.
    pub detail: String,
}

impl std::fmt::Display for RetryCallbackFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for RetryCallbackFailure {}

/// Typed helper failures. The loop itself never fails on provider errors
/// (those return as messages); only callback failures surface here.
#[derive(Debug, thiserror::Error)]
pub enum WholeCallRetryError {
    /// A retry callback failed; the loop stopped without another call.
    #[error("whole-call retry callback failed: {0}")]
    Callback(#[from] RetryCallbackFailure),
}

/// Optional lifecycle callbacks around each whole-call retry, mirroring the
/// pinned source contract: `on_retry_scheduled` fires before the backoff
/// sleep, `on_retry_attempt_start` after it (immediately before the retried
/// call), and `on_retry_finished` exactly once when the loop ends — but only
/// if a retry was ever scheduled. All are optional and may fail explicitly.
pub struct WholeCallRetryCallbacks<'a> {
    /// `(attempt, max_attempts, delay_ms, error_message)` before each sleep.
    pub on_retry_scheduled: Option<OnRetryScheduled<'a>>,
    /// Fired after the backoff sleep, before the retried call starts.
    pub on_retry_attempt_start: Option<OnRetryAttemptStart<'a>>,
    /// `(success, attempt, final_error)` once at loop end, only when a retry
    /// was scheduled (`success` is true only for a valid final message).
    pub on_retry_finished: Option<OnRetryFinished<'a>>,
}

/// Callback fired before the backoff sleep of each retry (1-indexed).
pub type OnRetryScheduled<'a> =
    Box<dyn FnMut(u32, u32, u64, &str) -> Result<(), RetryCallbackFailure> + Send + 'a>;
/// Callback fired after the backoff sleep, before the retried call starts.
pub type OnRetryAttemptStart<'a> = Box<dyn FnMut() -> Result<(), RetryCallbackFailure> + Send + 'a>;
/// Callback fired once at loop end (only when a retry was scheduled).
pub type OnRetryFinished<'a> =
    Box<dyn FnMut(bool, u32, Option<&str>) -> Result<(), RetryCallbackFailure> + Send + 'a>;

impl<'a> WholeCallRetryCallbacks<'a> {
    /// An empty callback set: the loop runs silently.
    pub fn new() -> Self {
        Self {
            on_retry_scheduled: None,
            on_retry_attempt_start: None,
            on_retry_finished: None,
        }
    }
}

impl Default for WholeCallRetryCallbacks<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// Run one assistant-producing closure with bounded whole-call retry on
/// transient failures (AIR-065).
///
/// - Omitted/disabled policy: `produce` runs exactly once and its message
///   returns unchanged (no callbacks fire).
/// - Valid stops and aborts return immediately; aborts are never retried.
/// - Quota/billing/usage-limit errors veto before transient classification
///   and fail fast; exhausted budgets return the final error message.
/// - Each retry is a NEW logical call from `produce` (never a re-dispatch),
///   separated by `on_retry_scheduled` → cancellable backoff sleep →
///   `on_retry_attempt_start`, in that order, with `on_retry_finished` once
///   at the end (only if a retry was scheduled).
/// - Cancellation during the backoff returns the failed message re-stamped
///   `Aborted` (error text dropped, partial content and usage preserved).
/// - A failing callback aborts the loop with
///   [`WholeCallRetryError::Callback`] and no further calls.
///
/// The helper never switches provider/model/auth (that is the caller's
/// `produce` closure), never wires product recovery, never merges streamed
/// output and never resets a prepared call's budget.
pub async fn retry_assistant_call<F, Fut>(
    mut produce: F,
    policy: Option<&WholeCallRetryPolicy>,
    cancel: tokio_util::sync::CancellationToken,
    mut callbacks: Option<WholeCallRetryCallbacks<'_>>,
) -> Result<crate::message::AssistantMessage, WholeCallRetryError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = crate::message::AssistantMessage>,
{
    let max_attempts = match policy {
        Some(policy) if policy.enabled => policy.max_retries,
        _ => 0,
    };

    fn fire_finished<'a>(
        callbacks: &mut Option<WholeCallRetryCallbacks<'a>>,
        success: bool,
        attempt: u32,
        final_error: Option<&str>,
    ) -> Result<(), WholeCallRetryError> {
        if let Some(callbacks) = callbacks
            && let Some(on_retry_finished) = callbacks.on_retry_finished.as_mut()
        {
            on_retry_finished(success, attempt, final_error)?;
        }
        Ok(())
    }

    let mut attempt: u32 = 0;
    let mut last_retry: Option<(u32, String)> = None;
    loop {
        let response = produce().await;
        match response.stop_reason {
            crate::stream::StopReason::Aborted => {
                if let Some((scheduled_attempt, _)) = &last_retry {
                    fire_finished(&mut callbacks, false, *scheduled_attempt, None)?;
                }
                return Ok(response);
            }
            crate::stream::StopReason::Error => {
                let transient =
                    classify_whole_call_retry(&response) == WholeCallRetryClassification::Transient;
                if attempt >= max_attempts || !transient {
                    if let Some((scheduled_attempt, _)) = &last_retry {
                        fire_finished(
                            &mut callbacks,
                            false,
                            *scheduled_attempt,
                            response.error_message.as_deref(),
                        )?;
                    }
                    return Ok(response);
                }
                attempt += 1;
                let error_message = response
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "Unknown error".into());
                last_retry = Some((attempt, error_message.clone()));
                let delay_ms =
                    whole_call_retry_delay_ms(policy.expect("enabled policy with budget"), attempt);
                if let Some(callbacks) = callbacks.as_mut()
                    && let Some(on_retry_scheduled) = callbacks.on_retry_scheduled.as_mut()
                {
                    on_retry_scheduled(attempt, max_attempts, delay_ms, &error_message)?;
                }
                let sleep = tokio::time::sleep(Duration::from_millis(delay_ms));
                tokio::select! {
                    _ = sleep => {}
                    _ = cancel.cancelled() => {
                        fire_finished(&mut callbacks, false, attempt, Some(&error_message))?;
                        // Normalize a cancelled backoff to the same aborted
                        // message shape as provider stream aborts: the error
                        // text goes away, partial content and usage stay.
                        let mut aborted = response;
                        aborted.stop_reason = crate::stream::StopReason::Aborted;
                        aborted.error_message = None;
                        return Ok(aborted);
                    }
                }
                if let Some(callbacks) = callbacks.as_mut()
                    && let Some(on_retry_attempt_start) = callbacks.on_retry_attempt_start.as_mut()
                {
                    on_retry_attempt_start()?;
                }
            }
            _ => {
                if let Some((scheduled_attempt, _)) = &last_retry {
                    fire_finished(&mut callbacks, true, *scheduled_attempt, None)?;
                }
                return Ok(response);
            }
        }
    }
}
