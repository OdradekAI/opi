//! Ordinary chat-call orchestration for the [`Models`](crate::models::Models) facade.

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::Stream;
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

use super::{
    ChatApiEntry, Models, compile_prepared_request, drive_resolved_call_registered,
    models_error_cause, resolve_route_auth, setup_config_error,
};
use crate::api::contract::CallOptions;
use crate::auth::{AuthProvenanceSource, AuthResolver, ResolvedAuth};
use crate::message::AssistantMessage;
use crate::provider::{EventStream, Provider, ProviderError, ProviderErrorSummary, Request};
use crate::stream::{AssistantStreamEvent, CallFailure, ChatStream};
use crate::types::{ApiId, ChatModelRecord};

// ChatCall — independently executing ordinary Models entry
// ---------------------------------------------------------------------------

/// The frozen binding captured at call creation (AIR-045).
pub(super) struct FrozenChatBinding {
    pub(super) record: ChatModelRecord,
    pub(super) route: FrozenRoute,
    pub(super) auth_plan: AuthPlan,
    pub(super) resource_owner: Option<Arc<dyn crate::codex_resources::SessionResourceOwner>>,
    pub(super) registration_gate: Arc<std::sync::Mutex<()>>,
}

pub(super) enum FrozenRoute {
    ChatApi {
        api_id: ApiId,
        entry: Option<ChatApiEntry>,
    },
    Provider {
        provider_id: String,
        provider: Option<Arc<dyn Provider>>,
    },
}

pub(super) enum AuthPlan {
    Route {
        provider_id: String,
        resolver: Option<Arc<dyn AuthResolver>>,
        source: AuthProvenanceSource,
    },
    Injected(Box<ResolvedAuth>),
}

impl AuthPlan {
    pub(super) fn default_for(models: &Models, provider_id: &str) -> Self {
        let (resolver, source) = models.route_auth_parts(provider_id);
        Self::Route {
            provider_id: provider_id.to_owned(),
            resolver,
            source,
        }
    }
}

/// The consumer of one ordinary [`Models`] chat call: native, simple, or direct.
///
/// Construction freezes the owned input, registration and auth plan, then
/// schedules setup and source execution on the current Tokio runtime. No
/// consumer poll is needed for progress. Dropping this handle or its result
/// future does not cancel execution. Retain the request's cancellation token
/// or use [`abort`](Self::abort) for explicit cancellation.
///
/// Events, typed setup failures, buffering, panic and runtime-shutdown behavior
/// follow [`StartedChatStream`](crate::stream::StartedChatStream). Registration,
/// type and route failures still precede authentication. Scheduling does not
/// promise setup has already run when construction returns; the runtime must
/// keep running until completion. An unfinished call retains an unbounded event
/// queue until it is consumed, discarded by `result`, or dropped.
/// After a business terminal, execution drains the source to EOF before forwarding
/// that terminal, settling telemetry and completing `result`. Tail events cannot
/// replace the first outcome or trigger another attempt. An unending source tail
/// therefore keeps the call pending; cooperative request cancellation or explicit
/// [`abort`](Self::abort) retains its existing interruption contract.
pub struct ChatCall {
    execution: crate::stream::StartedChatStream,
}

fn chat_binding_telemetry_attributes(
    binding: &FrozenChatBinding,
    request: &Request,
) -> crate::telemetry::AiRequestAttributes {
    let api = match &binding.route {
        FrozenRoute::ChatApi { api_id, .. } => api_id.as_str().to_owned(),
        FrozenRoute::Provider { .. } => binding.record.wire_api().as_str().to_owned(),
    };
    let mut attributes = crate::telemetry::AiRequestAttributes::new(
        binding.record.provider_id(),
        binding.record.model_id(),
        api,
    );
    if request.thinking.enabled
        && let Some(reasoning) = request.thinking.level.wire_name()
    {
        attributes = attributes.with_reasoning(reasoning);
    }
    if let Some(pricing) = binding.record.info().pricing.as_ref() {
        attributes = attributes.with_pricing(pricing.base);
    }
    attributes
}

impl ChatCall {
    pub(super) fn new(binding: FrozenChatBinding, request: Request, options: CallOptions) -> Self {
        // Pi installs the request observer when `stream()` is constructed, before
        // lazy setup/authentication begins. Keep the same boundary here so an
        // unpolled call and an early setup failure still have a request span.
        let observer = crate::telemetry::SharedAiRequestObserver::start(
            request.telemetry_context.as_ref(),
            chat_binding_telemetry_attributes(&binding, &request),
        );
        let driver = ChatCallDriver::new(binding, request, options, observer.clone());
        Self {
            execution: crate::stream::StartedChatStream::spawn(
                driver,
                ChatCallDriver::result,
                crate::stream::ProducerSnapshot::default(),
                Some(observer),
                None,
            ),
        }
    }

    /// Take the already-started execution handle without resubmitting work.
    ///
    /// Partial consumption and the final outcome are preserved. This method
    /// does not require a current runtime; construction already scheduled work.
    pub fn start(self) -> crate::stream::StartedChatStream {
        self.execution
    }

    /// Discard queued events and await the same execution's final outcome.
    ///
    /// Dropping this future does not cancel the request. No second request is
    /// made, including after partial or complete iteration.
    pub async fn result(self) -> Result<AssistantMessage, CallFailure> {
        self.execution.result().await
    }

    /// Explicitly stop and join the execution task, preserving its observed
    /// partial or already-completed outcome. See
    /// [`StartedChatStream::abort`](crate::stream::StartedChatStream::abort)
    /// for provider-task cleanup limits and cancellation of this wait. When
    /// the request carries an explicit telemetry context, the ordinary
    /// `pi.ai.request` observer is settled with the returned `aborted` outcome
    /// after Tokio confirms that this execution task was cancelled.
    pub async fn abort(&mut self) -> Result<AssistantMessage, CallFailure> {
        self.execution.abort().await
    }
}

impl Stream for ChatCall {
    type Item = Result<AssistantStreamEvent, CallFailure>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().execution).poll_next(cx)
    }
}

// The execution task owns setup and the canonical collector. Keeping this
// driver private prevents consumers from accidentally taking over execution.
struct ChatCallDriver {
    state: ChatCallState,
    terminal: Option<Result<AssistantMessage, CallFailure>>,
    observer: crate::telemetry::SharedAiRequestObserver,
}

/// The owned setup payload captured at construction.
struct ChatCallSetup {
    binding: FrozenChatBinding,
    request: Request,
    options: CallOptions,
}

struct ResourceApiStream {
    inner: EventStream,
    _resources: Option<crate::codex_resources::CallResourceLease>,
    _owner: Option<Arc<dyn crate::codex_resources::SessionResourceOwner>>,
}

impl Stream for ResourceApiStream {
    type Item = Result<AssistantStreamEvent, ProviderError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().inner.as_mut().poll_next(cx)
    }
}

enum ChatCallState {
    NotStarted(Box<ChatCallSetup>),
    Starting(BoxFuture<'static, Result<ChatStream, CallFailure>>),
    Active(Box<ChatStream>),
    Done,
}

impl ChatCallDriver {
    fn new(
        binding: FrozenChatBinding,
        mut request: Request,
        options: CallOptions,
        observer: crate::telemetry::SharedAiRequestObserver,
    ) -> Self {
        request.callbacks = request.callbacks.for_call();
        Self {
            state: ChatCallState::NotStarted(Box::new(ChatCallSetup {
                binding,
                request,
                options,
            })),
            terminal: None,
            observer,
        }
    }

    /// Consume the handle and actively drive the call to the single final
    /// outcome (the first poll, and therefore auth, starts here if it had not
    /// happened yet).
    pub async fn result(mut self) -> Result<AssistantMessage, CallFailure> {
        use futures_util::StreamExt as _;
        while self.terminal.is_none() {
            if self.next().await.is_none() {
                break;
            }
        }
        if let ChatCallState::Active(stream) =
            std::mem::replace(&mut self.state, ChatCallState::Done)
        {
            let outcome = stream.result().await;
            if self.terminal.is_none() {
                self.terminal = Some(outcome);
            }
        }
        match self.terminal.take() {
            Some(outcome) => outcome,
            None => {
                let failure = CallFailure::from_partial(
                    None,
                    ProviderError::StreamError(ProviderErrorSummary::attested_static(
                        "chat call ended without a terminal event",
                    )),
                );
                self.observer.finish(Some(failure.message()));
                Err(failure)
            }
        }
    }
}

impl Stream for ChatCallDriver {
    type Item = Result<AssistantStreamEvent, CallFailure>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            if let ChatCallState::Active(stream) = &mut this.state {
                return match Stream::poll_next(Pin::new(stream), cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(Some(Ok(event))) => {
                        if let AssistantStreamEvent::Done { message, .. } = &event {
                            this.terminal = Some(Ok(message.clone()));
                        }
                        Poll::Ready(Some(Ok(event)))
                    }
                    Poll::Ready(Some(Err(failure))) => {
                        this.terminal = Some(Err(failure.clone()));
                        Poll::Ready(Some(Err(failure)))
                    }
                    Poll::Ready(None) => {
                        this.state = ChatCallState::Done;
                        Poll::Ready(None)
                    }
                };
            }
            if matches!(this.state, ChatCallState::Done) {
                return Poll::Ready(None);
            }
            match std::mem::replace(&mut this.state, ChatCallState::Done) {
                ChatCallState::NotStarted(setup) => {
                    // First poll: setup (and therefore auth) starts here.
                    let ChatCallSetup {
                        binding,
                        request,
                        options,
                    } = *setup;
                    this.state = ChatCallState::Starting(Box::pin(materialize_chat_call(
                        binding,
                        request,
                        options,
                        this.observer.clone(),
                    )));
                }
                ChatCallState::Starting(mut future) => {
                    match std::future::Future::poll(future.as_mut(), cx) {
                        Poll::Pending => {
                            this.state = ChatCallState::Starting(future);
                            return Poll::Pending;
                        }
                        Poll::Ready(Ok(stream)) => {
                            this.state = ChatCallState::Active(Box::new(stream));
                        }
                        Poll::Ready(Err(failure)) => {
                            this.observer.on_delivery(Err(&failure));
                            this.observer.finish(Some(failure.message()));
                            this.terminal = Some(Err(failure.clone()));
                            this.state = ChatCallState::Done;
                            return Poll::Ready(Some(Err(failure)));
                        }
                    }
                }
                ChatCallState::Active(_) | ChatCallState::Done => {
                    unreachable!("active and done states are handled above")
                }
            }
        }
    }
}

/// Setup for one independently scheduled call: canonical identity from the frozen record,
/// pre-auth route/type checks, the shared compiler, the shared auth
/// resolution (or the injected auth), and one dispatch through the captured
/// adapter/provider into the shared [`ChatStream`] collector.
async fn materialize_chat_call(
    binding: FrozenChatBinding,
    mut request: Request,
    options: CallOptions,
    observer: crate::telemetry::SharedAiRequestObserver,
) -> Result<ChatStream, CallFailure> {
    // Canonical model identity comes from the frozen complete record.
    request.model = binding.record.spec();
    let setup_failure = |cause| CallFailure::from_setup(&binding.record, cause);
    let setup_config_failure = |message| setup_failure(setup_config_error(message));
    let resources = {
        let _registration = binding.registration_gate.lock().unwrap();
        binding
            .resource_owner
            .as_ref()
            .and_then(|owner| owner.register_call(&request))
    };
    request.cancel = resources
        .as_ref()
        .map_or_else(|| request.cancel.child_token(), |lease| lease.token());
    if request.cancel.is_cancelled() {
        return Err(setup_failure(ProviderError::Cancelled));
    }

    let (event_stream, projection) = match binding.route {
        FrozenRoute::ChatApi { api_id, entry } => {
            let Some(entry) = entry else {
                return Err(setup_config_failure(format!(
                    "chat api '{}' is not registered",
                    api_id.as_str()
                )));
            };
            if entry.provider_id != binding.record.provider_id() {
                return Err(setup_config_failure(format!(
                    "record provider '{}' does not match the registered chat api provider '{}'",
                    binding.record.provider_id(),
                    entry.provider_id
                )));
            }
            let CallOptions::Native { type_id, value } = options else {
                unreachable!("simple entries dispatch through provider routes")
            };
            if type_id != entry.options_type {
                return Err(setup_config_failure(format!(
                    "native options type does not match the registered chat api '{}'",
                    api_id.as_str()
                )));
            }
            crate::provider::validate_request_structure_for_model(
                binding.record.provider_id(),
                Some(binding.record.info()),
                &request,
            )
            .map_err(setup_failure)?;
            entry
                .dispatch
                .validate_native_erased(binding.record.info(), &request, value.as_ref())
                .map_err(setup_failure)?;
            let projection = crate::transcript::compile_request_projection(
                &request,
                binding.record.provider_id(),
                Some(binding.record.info()),
                binding.record.wire_api(),
            )
            .map_err(setup_failure)?
            .facts;
            let auth = resolve_binding_auth(&binding.auth_plan, &request.cancel)
                .await
                .map_err(|cause| setup_failure(cause).with_projection(&projection))?;
            let retained_resources = resources.clone();
            let stream = entry.dispatch.stream_prepared_erased(
                binding.record.info().clone(),
                request,
                value,
                auth,
                resources,
            );
            (
                Box::pin(ResourceApiStream {
                    inner: stream,
                    _resources: retained_resources,
                    _owner: binding.resource_owner,
                }) as EventStream,
                projection,
            )
        }
        FrozenRoute::Provider {
            provider_id,
            provider,
        } => {
            let Some(provider) = provider else {
                return Err(setup_config_failure(format!(
                    "unknown provider: {provider_id}"
                )));
            };
            let CallOptions::Simple(simple_options) = options else {
                unreachable!("native entries dispatch through chat APIs")
            };
            provider
                .validate_simple_options(binding.record.info(), &simple_options)
                .map_err(setup_failure)?;
            compile_prepared_request(provider.as_ref(), binding.record.info(), &mut request)
                .map_err(setup_failure)?;
            let projection = crate::transcript::compile_request_projection(
                &request,
                binding.record.provider_id(),
                Some(binding.record.info()),
                binding.record.wire_api(),
            )
            .map_err(setup_failure)?
            .facts;
            let mut auth = resolve_binding_auth(&binding.auth_plan, &request.cancel)
                .await
                .map_err(|cause| setup_failure(cause).with_projection(&projection))?;
            let mut selected_model = binding.record.info().clone();
            let provider = provider
                .prepare_model_call(&mut selected_model, &mut auth)
                .map_err(|cause| setup_failure(cause).with_projection(&projection))?
                .unwrap_or(provider);
            (
                drive_resolved_call_registered(
                    provider,
                    request,
                    auth,
                    resources,
                    binding.resource_owner,
                    Some(selected_model),
                    simple_options,
                ),
                projection,
            )
        }
    };
    Ok(ChatStream::with_shared_telemetry(
        crate::stream::with_projection(event_stream, projection),
        observer,
    ))
}

async fn resolve_binding_auth(
    plan: &AuthPlan,
    cancel: &CancellationToken,
) -> Result<ResolvedAuth, ProviderError> {
    match plan {
        AuthPlan::Injected(auth) => Ok((**auth).clone()),
        AuthPlan::Route {
            provider_id,
            resolver: None,
            ..
        } => Err(setup_config_error(format!(
            "provider '{provider_id}' has no dispatchable route"
        ))),
        AuthPlan::Route {
            resolver: Some(resolver),
            source,
            ..
        } => resolve_route_auth(resolver, source.clone(), cancel)
            .await
            .map_err(models_error_cause),
    }
}
