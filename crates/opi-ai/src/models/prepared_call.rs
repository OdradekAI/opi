//! Prepared provider execution ownership for the [`Models`](crate::models::Models) facade.

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};

use futures_util::{Stream, StreamExt, stream};
use tokio_util::sync::CancellationToken;

use super::ModelsError;
use crate::api::contract::SimpleOptions;
use crate::auth::{AuthProvenance, ResolvedAuth};
use crate::provider::{EventStream, ModelInfo, Provider, ProviderError, Request};
use crate::stream::AssistantStreamEvent;

/// Immutable, redacted facts about a resolved provider route.
///
/// Contains no secret material; the resolved secret and its non-secret
/// provenance both stay private to [`PreparedProviderCall`].
#[derive(Debug, Clone)]
pub struct PreparedRoute {
    /// Projection accepted for the frozen canonical input before authentication.
    pub projection: crate::transcript::ProjectionFacts,
    /// Canonical provider id of the resolved route.
    pub provider_id: String,
    /// Canonical model id of the resolved route.
    pub model_id: String,
    /// Provider wire/API kind of the resolved route.
    pub wire_api: crate::model_info::WireApi,
}

/// An opaque prepared provider call.
///
/// Privately freezes the resolved provider, the request, and the secret-bearing
/// resolved authentication. `start_attempt` reuses the
/// An admitted strict-downgrade variant (task 20.11): exactly one rebuilt
/// payload variant after a source-qualified Anthropic strict-limit
/// rejection, with its machine-readable reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrictVariantAdmission {
    /// The recorded downgrade reason (e.g. `strict_tool_limits_exceeded`).
    pub reason: &'static str,
}

/// frozen route, request, and authentication for each permitted sequential
/// retry without repeating route/auth preparation. The secret never enters
/// [`PreparedRoute`], Agent-visible state, evidence, diagnostics, or
/// model-visible state.
pub struct PreparedProviderCall {
    provider: Arc<dyn Provider>,
    request: Box<Request>,
    auth: ResolvedAuth,
    route: PreparedRoute,
    /// Set when an attempt reports that the frozen credential is unavailable
    /// or revoked; later attempts fail closed instead of redispatching it.
    credential_terminal: Arc<AtomicBool>,
    /// The acquisition-retry owner: per-variant send counters, retry
    /// admission, readiness deadline, and sealed states.
    acquisition: Arc<crate::retry::AcquisitionCore>,
    /// The logical call's stable retry-invocation identifier (the wire
    /// `amz-sdk-invocation-id`; task 20.20).
    invocation_id: String,
    resources: Option<crate::codex_resources::CallResourceLease>,
    resource_owner: Option<Arc<dyn crate::codex_resources::SessionResourceOwner>>,
    // Retain the complete record without inflating callers' inline async state.
    selected_model: Option<Box<ModelInfo>>,
    simple_options: SimpleOptions,
}

struct AttemptStream {
    inner: EventStream,
    cancel: CancellationToken,
    credential_terminal: Arc<AtomicBool>,
    acquisition: Arc<crate::retry::AcquisitionCore>,
    released: bool,
    /// A business terminal was delivered. The acquisition lease is released,
    /// but the provider source remains available for `ChatStream::result` to
    /// drain source-owned tail work before the handle is dropped.
    terminal_seen: bool,
    /// Whether this attempt was ever polled (dispatched). Distinguishes a
    /// lease released before send from a dropped dispatch with unknown
    /// effect.
    polled: bool,
    _resource_owner: Option<Arc<dyn crate::codex_resources::SessionResourceOwner>>,
}

impl AttemptStream {
    fn release(&mut self) {
        if !self.released {
            self.acquisition.release_attempt();
            self.released = true;
        }
    }
}

impl Stream for AttemptStream {
    type Item = Result<AssistantStreamEvent, ProviderError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.released {
            if !this.terminal_seen {
                return Poll::Ready(None);
            }
            // The lease is already released, so tail polling must not update
            // retry/admission state. Only `ChatStream::result` reaches this
            // branch after terminal delivery; consumer iteration remains
            // permanent EOF at the outer ChatStream seam.
            loop {
                match this.inner.as_mut().poll_next(cx) {
                    Poll::Ready(Some(_)) => continue,
                    Poll::Ready(None) => return Poll::Ready(None),
                    Poll::Pending => return Poll::Pending,
                }
            }
        }
        if !this.polled {
            if this.cancel.is_cancelled() {
                this.acquisition.note_failure(&ProviderError::Cancelled);
                this.release();
                return Poll::Ready(Some(Err(ProviderError::Cancelled)));
            }
            this.polled = true;
            this.acquisition.note_dispatch();
        }
        let item = this.inner.as_mut().poll_next(cx);
        match &item {
            Poll::Ready(Some(Err(cause))) => {
                // Projection metadata preserves the underlying typed failure.
                // A response handoff still seals acquisition; pre-handoff
                // failures retain strict-variant and credential classification.
                let classified = match cause {
                    ProviderError::StreamFailure(failure) if !cause.response_received() => {
                        failure.cause()
                    }
                    _ => cause,
                };
                if matches!(
                    classified,
                    ProviderError::CredentialNeeded { .. }
                        | ProviderError::CredentialRevoked { .. }
                ) {
                    this.credential_terminal.store(true, Ordering::Release);
                }
                this.acquisition.note_failure(classified);
            }
            Poll::Ready(Some(Ok(_))) => {
                // The first delivered stream item is the streaming-response
                // handoff; it seals acquisition retry for this logical call.
                this.acquisition.note_handoff();
            }
            Poll::Ready(None) => {
                this.acquisition.note_finished();
            }
            Poll::Pending => {}
        }
        let terminal = match &item {
            Poll::Ready(None) => true,
            Poll::Ready(Some(Err(_))) => true,
            Poll::Ready(Some(Ok(event))) => event.is_terminal(),
            Poll::Pending => false,
        };
        if terminal {
            let business_terminal = matches!(
                &item,
                Poll::Ready(Some(Ok(event))) if event.is_terminal()
            );
            this.terminal_seen = business_terminal;
            this.release();
        }
        item
    }
}

impl Drop for AttemptStream {
    fn drop(&mut self) {
        if !self.released {
            if self.polled {
                // Dispatched but dropped without an observed closed outcome:
                // the effect is unknown and further scheduling is refused.
                self.acquisition.note_unknown_dispatch();
            }
            self.release();
        }
    }
}

impl std::fmt::Debug for PreparedProviderCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never surface the secret-bearing auth; only the redacted route facts.
        f.debug_struct("PreparedProviderCall")
            .field("route", &self.route)
            .field("auth", &"<redacted>")
            .finish()
    }
}

impl PreparedProviderCall {
    pub(super) fn new(
        provider: Arc<dyn Provider>,
        request: Request,
        auth: ResolvedAuth,
        route: PreparedRoute,
        policy: crate::retry::AcquisitionPolicy,
        resources: Option<crate::codex_resources::CallResourceLease>,
        resource_owner: Option<Arc<dyn crate::codex_resources::SessionResourceOwner>>,
        selected_model: Option<ModelInfo>,
        simple_options: SimpleOptions,
    ) -> Self {
        Self {
            provider,
            request: Box::new(request),
            auth,
            route,
            credential_terminal: Arc::new(AtomicBool::new(false)),
            acquisition: Arc::new(crate::retry::AcquisitionCore::new(policy)),
            invocation_id: uuid::Uuid::new_v4().to_string(),
            resources,
            resource_owner,
            selected_model: selected_model.map(Box::new),
            simple_options,
        }
    }

    pub(super) fn cancellation_token(&self) -> CancellationToken {
        self.request.cancel.clone()
    }

    /// The immutable, redacted route facts for this prepared call.
    pub fn route(&self) -> &PreparedRoute {
        &self.route
    }

    /// The non-secret auth source classification plus fallback decision for the
    /// resolved authentication.
    ///
    /// Provenance is carried beside the secret on the private
    /// [`ResolvedAuth`]; this redacted accessor is the only public read path,
    /// so callers and evidence can distinguish auth sources without seeing the
    /// secret.
    pub fn auth_provenance(&self) -> &AuthProvenance {
        &self.auth.provenance
    }

    /// The SDK-owned admission for the next attempt, if the last observed
    /// failure was retryable and the frozen per-variant budget still has
    /// room.
    ///
    /// The caller reads the [`RetryAdmission`](crate::retry::RetryAdmission), waits `delay_ms`
    /// (cancellably, through its own token — the shared
    /// [`Request::cancel`] token also forbids any further attempt once
    /// cancelled), and then calls [`start_attempt`](Self::start_attempt)
    /// again. Starting before the deadline returns
    /// [`ModelsError::RetryNotReady`] with zero dispatch.
    pub fn retry_admission(&self) -> Option<crate::retry::RetryAdmission> {
        self.acquisition.admission()
    }

    /// The sealed state, if this prepared call no longer admits acquisition
    /// attempts (post-handoff, unknown-dispatch, exhausted budget, or a
    /// terminal outcome).
    pub fn acquisition_seal(&self) -> Option<crate::retry::AcquisitionSeal> {
        self.acquisition.seal()
    }

    /// The pending strict-downgrade variant admission, if the last observed
    /// failure was a source-qualified Anthropic strict-limit rejection on
    /// the first variant (task 20.11, V-34/V-36).
    ///
    /// The caller dispatches it immediately through
    /// [`start_attempt`](Self::start_attempt) — no readiness wait: the
    /// rebuilt variant removes only prefer-strict, re-runs the payload
    /// callback under its own freeze slot, and keeps the frozen
    /// headers/auth/model. At most one exists per logical call and nothing
    /// resets the counters to gain another.
    pub fn strict_variant_admission(&self) -> Option<StrictVariantAdmission> {
        self.acquisition
            .strict_variant_admission()
            .map(|reason| StrictVariantAdmission { reason })
    }

    /// Start one attempt using the frozen request and authentication.
    ///
    /// Reserves the active lease immediately. The adapter and its producer
    /// are constructed on the stream's first poll; dropping an unpolled
    /// lease sends nothing and leaves it available for a later start.
    ///
    /// Reuses the prepared route and authentication; does not repeat preparation.
    /// Every attempt shares the frozen [`Request::cancel`] token: cancelling it
    /// terminates the logical call and forbids any further attempt. At most one
    /// attempt stream may be active at a time; a sequential retry must wait until
    /// the prior attempt's stream reaches a terminal event or error, which
    /// releases the active slot. A credential-needed or credential-revoked
    /// stream error additionally terminates the prepared call and forbids retry.
    /// The acquisition owner refuses dispatch while the call is sealed
    /// (post-handoff, unknown-dispatch, exhausted budget, or a terminal
    /// outcome) and before an admitted retry's readiness deadline.
    ///
    /// # Errors
    ///
    /// - [`ModelsError::CallCancelled`] if the shared cancellation token has
    ///   been cancelled.
    /// - [`ModelsError::CredentialTerminated`] if a prior attempt rejected
    ///   the frozen credential.
    /// - [`ModelsError::CallSealed`] if the acquisition owner sealed the call.
    /// - [`ModelsError::RetryNotReady`] if the admitted retry's wait has not
    ///   elapsed yet.
    /// - [`ModelsError::AttemptAlreadyActive`] if a prior attempt stream is
    ///   still active.
    pub fn start_attempt(&self) -> Result<EventStream, ModelsError> {
        if self.request.cancel.is_cancelled() {
            return Err(ModelsError::CallCancelled);
        }
        if self.credential_terminal.load(Ordering::Acquire) {
            return Err(ModelsError::CredentialTerminated {
                provider: self.route.provider_id.clone(),
            });
        }
        if let Err(error) = self.acquisition.check_start() {
            return Err(match error {
                crate::retry::RetryStartError::AlreadyActive => ModelsError::AttemptAlreadyActive,
                crate::retry::RetryStartError::Sealed(seal) => ModelsError::CallSealed { seal },
                crate::retry::RetryStartError::NotReady { retry_ready_in_ms } => {
                    ModelsError::RetryNotReady { retry_ready_in_ms }
                }
            });
        }
        if self.request.cancel.is_cancelled() {
            self.acquisition.release_attempt();
            return Err(ModelsError::CallCancelled);
        }
        // Recheck revocation after atomically reserving the execution lease.
        if self.credential_terminal.load(Ordering::Acquire) {
            self.acquisition.release_attempt();
            return Err(ModelsError::CredentialTerminated {
                provider: self.route.provider_id.clone(),
            });
        }
        // Adapters may eagerly spawn their producer in `stream_prepared`.
        // Defer that invocation too, so an unpolled lease has no effects.
        let provider = self.provider.clone();
        let mut request = (*self.request).clone();
        let auth = self.auth.clone();
        let attempt = self.acquisition.attempt_context(&self.invocation_id);
        request.callbacks = request.callbacks.with_attempt(attempt.clone());
        let resources = self.resources.clone();
        let selected_model = self.selected_model.clone();
        let simple_options = self.simple_options.clone();
        let stream = Box::pin(
            stream::once(async move {
                match selected_model {
                    Some(model) => provider.stream_model_resource_attempt(
                        *model,
                        simple_options,
                        request,
                        auth,
                        attempt,
                        resources,
                    ),
                    None => provider.stream_resource_attempt(request, auth, attempt, resources),
                }
            })
            .flatten(),
        );
        Ok(crate::stream::with_projection(
            Box::pin(AttemptStream {
                inner: stream,
                cancel: self.request.cancel.clone(),
                credential_terminal: self.credential_terminal.clone(),
                acquisition: self.acquisition.clone(),
                released: false,
                terminal_seen: false,
                polled: false,
                _resource_owner: self.resource_owner.clone(),
            }),
            self.route.projection.clone(),
        ))
    }

    /// Start one prepared attempt behind the standard [`ChatStream`] result
    /// contract.
    ///
    /// This is the observed convenience entry for independent consumers. It
    /// keeps [`start_attempt`](Self::start_attempt) available as the raw
    /// attempt seam used by retry/admission controllers, while exposing the
    /// same terminal absorption, source-tail settlement and optional
    /// `pi.ai.request` observer as ordinary `Models` chat calls. Each invocation
    /// owns one attempt span; sequential retry attempts are not implicitly
    /// aggregated. An explicit
    /// [`Request::telemetry_context`](crate::provider::Request::telemetry_context)
    /// enables observation; omission keeps the stream business-only.
    pub fn start_attempt_with_telemetry(&self) -> Result<crate::stream::ChatStream, ModelsError> {
        let source = self.start_attempt()?;
        Ok(crate::stream::ChatStream::new_with_telemetry(
            source,
            self.request.telemetry_context.as_ref(),
            self.ai_telemetry_attributes(),
        ))
    }

    fn ai_telemetry_attributes(&self) -> crate::telemetry::AiRequestAttributes {
        let mut attributes = crate::telemetry::AiRequestAttributes::new(
            self.route.provider_id.clone(),
            self.route.model_id.clone(),
            self.route.wire_api.as_str(),
        );
        if self.request.thinking.enabled
            && let Some(reasoning) = self.request.thinking.level.wire_name()
        {
            attributes = attributes.with_reasoning(reasoning);
        }
        if let Some(pricing) = self
            .selected_model
            .as_deref()
            .and_then(|model| model.pricing.as_ref())
        {
            attributes = attributes.with_pricing(pricing.base);
        }
        attributes
    }
}
