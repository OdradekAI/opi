//! Single `Models` front door: catalog, redacted auth status, and chat calls.
//!
//! [`Models`] consolidates provider/model lookup, the provider-side auth
//! contract, OpenAI-compatible compatibility metadata, and every chat-call
//! entry into one owner (AIR-045). Ordinary calls — native
//! [`stream`](Models::stream)/[`complete`](Models::complete), neutral
//! [`stream_simple`](Models::stream_simple)/[`complete_simple`](Models::complete_simple),
//! auth-injected [`stream_direct`](Models::stream_direct), and advanced
//! [`prepare_call`](Models::prepare_call) — share the sole request compiler,
//! auth resolution, attempt seam, and `ChatStream`
//! collector; there is no parallel facade or router.
//!
//! # Auth descriptor, resolver, and store split
//!
//! [`AuthDescriptor`] carries redaction-safe metadata and supports
//! non-live dispatch gates. Concrete routes use [`crate::AuthResolver`] to
//! resolve live credentials once per logical call, while
//! [`crate::CredentialStore`] owns persisted credential IO and redacted
//! availability probes. OAuth flows populate or refresh stored credentials
//! outside this driver; the driver does not perform login.
//!
//! # Typed custom chat APIs
//!
//! Custom chat adapters register as an object-safe [`crate::ChatApi<O>`] with one
//! fixed options type `O` and an exact [`crate::ApiId`]. Registration installs a
//! catalog-invisible provider route (the record never appears in catalog
//! reads) plus the typed dispatch entry. A call created through any entry
//! freezes the caller's complete [`ChatModelRecord`], the captured adapter,
//! and the owned input at construction; authentication starts on the first
//! poll. Later registration changes never mutate an already-created call.
//!
//! # Unstable
//!
//! This surface is part of the **unstable 0.x extension substrate**. Breaking
//! changes may occur between minor versions without a major version bump.

pub(crate) use std::collections::{BTreeSet, HashMap};
pub(crate) use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

#[cfg(any(feature = "builtins", feature = "provider-protocol-common"))]
pub(crate) use futures_util::StreamExt;
pub(crate) use futures_util::future::BoxFuture;
pub(crate) use futures_util::stream;
use tokio_util::sync::CancellationToken;

pub(crate) use crate::api::contract::{
    ChatApiHandle, DeferredApiHandle, NativeOptions, SimpleOptions,
};
pub(crate) use crate::auth::{
    AuthFallback, AuthProvenance, AuthProvenanceSource, AuthResolver, AuthScheme,
    AwsSigV4Credentials, ResolvedAuth,
};
pub(crate) use crate::credential::{Credential, CredentialStore, CredentialStoreError};
pub(crate) use crate::message::AssistantMessage;
pub(crate) use crate::mutation::{MutationError, MutationOutcome};
pub(crate) use crate::provider::{
    EventStream, ModelInfo, Provider, ProviderError, ProviderErrorSummary, Request,
};
pub(crate) use crate::registry::{
    ModelCapabilities, ProviderRegistry, RegistrationError, RegistryError,
};
pub(crate) use crate::stream::{AssistantStreamEvent, CallFailure};
#[cfg(feature = "builtins")]
pub(crate) use crate::types::ApiId;
pub use crate::types::ChatModelRecord;
use crate::types::deferred::DeferredError;

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

mod contracts;
pub use contracts::*;
#[cfg(feature = "builtins")]
pub(crate) use contracts::{BuiltinAuthBindings, BuiltinAuthStatus};
pub(crate) use contracts::{BuiltinAuthCheck, BuiltinAuthFactory};

mod registry_owner;
pub(crate) use registry_owner::RegistryOwner;

mod auth_owner;
pub(crate) use auth_owner::AuthOwner;

mod catalog_owner;
pub(crate) use catalog_owner::CatalogOwner;

mod operation_owner;
pub(crate) use operation_owner::OperationOwner;

mod dispatch;
mod facade;

/// A single driver of providers/models that owns provider+model lookup, auth
/// resolution, compatibility metadata, and every chat-call entry.
///
/// Wraps a [`ProviderRegistry`] and layers the auth/call contract on top so
/// every entry — native, simple, direct, and advanced — shares one state
/// machine. See the [module-level documentation](self) for the full seam
/// description.
pub struct Models {
    registry: RegistryOwner,
    auth_owner: AuthOwner,
    compat: HashMap<String, CompatMetadata>,
    catalog_owner: CatalogOwner,
    operation_owner: OperationOwner,
}

/// Validate a candidate dynamic catalog before it can replace the live
/// publication: a malformed or duplicate refresh must preserve the
/// last-known catalog (mirrors `register_model` checks).
fn validate_dynamic_catalog(provider_id: &str, models: &[ModelInfo]) -> Result<(), ProviderError> {
    let config = |message: String| ProviderError::Config(ProviderErrorSummary::sanitized(message));
    let mut seen = std::collections::HashSet::new();
    for model in models {
        if model.id.is_empty() {
            return Err(config(format!(
                "dynamic catalog for provider '{provider_id}' contains a model with an empty id"
            )));
        }
        if !seen.insert(model.id.as_str()) {
            return Err(config(format!(
                "dynamic catalog for provider '{provider_id}' has duplicate model id '{}'",
                model.id
            )));
        }
        if let Err(source) = model.validate() {
            return Err(config(format!(
                "dynamic catalog for provider '{provider_id}' has invalid model '{}': {source}",
                model.id
            )));
        }
    }
    Ok(())
}

/// Drain an event stream to the single final outcome of the logical call.
///
/// A general stream-drain utility sharing the public seam with
/// [`ChatStream::result`](crate::stream::ChatStream::result): drives any
/// [`EventStream`] (for example the one returned by
/// [`PreparedProviderCall::start_attempt`]) through the same terminal
/// absorption, permanent-EOF and failure rules. `Done` terminals complete
/// `Ok` with the canonical message; every failure (in-band error terminal,
/// transport/protocol error, or a stream that ends without a terminal) drains
/// to exactly one `Err(CallFailure)` carrying the preserved partial and a
/// typed, redacted cause.
pub async fn drain_to_completion(
    stream: EventStream,
) -> Result<AssistantMessage, crate::stream::CallFailure> {
    crate::stream::ChatStream::new(stream).result().await
}

// ---------------------------------------------------------------------------
// Prepared call
// ---------------------------------------------------------------------------

mod prepared_call;
use prepared_call::PreparedDispatch;
pub use prepared_call::{PreparedProviderCall, PreparedRoute};

/// Drive ordinary and direct calls with the same per-call acquisition owner
/// used by advanced preparation. Admitted retries consume their existing
/// counters; only the source-qualified strict admission creates a new variant.
#[cfg(any(feature = "builtins", feature = "provider-protocol-common"))]
pub(crate) fn drive_resolved_call(
    provider: Arc<dyn Provider>,
    request: Request,
    auth: ResolvedAuth,
) -> EventStream {
    if let Some(attempt) = request.callbacks.inherited_attempt() {
        return provider.stream_attempt(request, auth, attempt);
    }
    // Direct adapters are lazy too. Retain their registration in the public
    // stream, rather than only in a producer that can finish before delivery.
    Box::pin(
        stream::once(async move {
            let mut request = request;
            request.callbacks = request.callbacks.for_call();
            let resources = provider
                .session_resources()
                .and_then(|owner| owner.register_call(&request));
            drive_resolved_call_registered(
                provider,
                request,
                auth,
                resources,
                None,
                None,
                SimpleOptions::default(),
            )
        })
        .flatten(),
    )
}

fn drive_resolved_call_registered(
    provider: Arc<dyn Provider>,
    mut request: Request,
    auth: ResolvedAuth,
    resources: Option<crate::codex_resources::CallResourceLease>,
    resource_owner: Option<Arc<dyn crate::codex_resources::SessionResourceOwner>>,
    selected_model: Option<ModelInfo>,
    simple_options: SimpleOptions,
) -> EventStream {
    request.callbacks = request.callbacks.ensure_call_scope();
    request.cancel = resources
        .as_ref()
        .map_or_else(|| request.cancel.child_token(), |lease| lease.token());
    let record = selected_model.as_ref().or_else(|| {
        provider.models().iter().find(|model| {
            model.id
                == request
                    .model
                    .split_once(':')
                    .map_or(request.model.as_str(), |(_, id)| id)
        })
    });
    let wire_api = record
        .map(|model| model.wire_api)
        .or_else(|| provider.models().first().map(|model| model.wire_api))
        .unwrap_or_else(|| provider.default_wire_api());
    let projection = match crate::transcript::compile_request_projection(
        &request,
        provider.id(),
        record,
        wire_api,
    ) {
        Ok(projection) => projection.facts,
        Err(error) => return Box::pin(futures_util::stream::once(async move { Err(error) })),
    };
    let route = PreparedRoute {
        projection,
        provider_id: provider.id().to_owned(),
        model_id: request
            .model
            .split_once(':')
            .map_or(request.model.as_str(), |(_, id)| id)
            .to_owned(),
        wire_api,
    };
    let policy = provider.acquisition_policy_default();
    let prepared = PreparedProviderCall::new(
        PreparedDispatch {
            provider,
            selected_model: selected_model.map(Box::new),
            simple_options,
        },
        request,
        auth,
        route,
        policy,
        resources,
        resource_owner,
    );
    Box::pin(stream::unfold(
        (prepared, None::<EventStream>, false),
        |(prepared, mut current, finished)| async move {
            use futures_util::StreamExt;
            if finished {
                // A business terminal seals delivery, not source settlement.
                // Drain only this attempt; tail items must never re-enter retry
                // admission or replace the collector's first terminal outcome.
                if let Some(mut source) = current {
                    while source.next().await.is_some() {}
                }
                return None;
            }
            loop {
                if current.is_none() {
                    current = match prepared.start_attempt() {
                        Ok(attempt) => Some(attempt),
                        Err(error) => {
                            return Some((Err(models_error_cause(error)), (prepared, None, true)));
                        }
                    };
                }
                match current.as_mut().expect("attempt initialized").next().await {
                    Some(Err(error)) => {
                        if prepared.strict_variant_admission().is_some() {
                            current = None;
                            continue;
                        }
                        if let Some(admission) = prepared.retry_admission() {
                            let cancellation = prepared.cancellation_token();
                            tokio::select! {
                                biased;
                                _ = cancellation.cancelled() => {
                                    return Some((Err(ProviderError::Cancelled), (prepared, None, true)));
                                }
                                _ = tokio::time::sleep(Duration::from_millis(admission.delay_ms)) => {}
                            }
                            current = None;
                            continue;
                        }
                        return Some((Err(error), (prepared, None, true)));
                    }
                    Some(Ok(event)) => {
                        let terminal = event.is_terminal();
                        return Some((Ok(event), (prepared, current, terminal)));
                    }
                    None => return None,
                }
            }
        },
    ))
}

// ---------------------------------------------------------------------------
// Shared compiler and auth resolution (the sole state machine)
// ---------------------------------------------------------------------------

/// Request preparation for neutral calls and the advanced entry: output and
/// thinking resolution, model/context clamping, and capability/wire validation,
/// without mutating the caller's original `Request` (the compiler owns a copy).
fn compile_prepared_request(
    provider: &dyn Provider,
    model: &ModelInfo,
    request: &mut Request,
) -> Result<(), ProviderError> {
    if provider.skips_shared_request_preparation(model) {
        return crate::provider::validate_request_structure_for_model(
            provider.id(),
            Some(model),
            request,
        );
    }
    if request.max_tokens.is_none() {
        let limit = model.capabilities.max_output_tokens;
        if request.thinking.enabled
            && request
                .thinking
                .budget_tokens
                .is_some_and(|budget| budget >= limit)
        {
            return Err(ProviderError::UnsupportedCapability(
                ProviderErrorSummary::sanitized(
                    "thinking budget must be below the resolved model's default output limit",
                ),
            ));
        }
        request.max_tokens = Some(limit);
    }
    let context_limit = model.capabilities.context_window;
    let context_available = if context_limit > 0 {
        // Canonical system/tool declarations carry chronology and timestamp
        // facts needed to validate an anchor and estimate its tail. Passing
        // replayed current values separately would lose those facts or count
        // the same declaration twice.
        let occupied =
            crate::context::estimate_context_tokens(None, request.context.messages(), &[]).tokens;
        let available = context_limit
            .saturating_sub(occupied)
            .saturating_sub(4_096)
            .max(1);
        request.max_tokens = request.max_tokens.map(|limit| limit.min(available));
        Some(available)
    } else {
        None
    };
    provider.prepare_thinking_request(model, request, context_available);
    crate::provider::validate_request_for_model(provider.id(), Some(model), request)
}

/// The one auth resolution shared by advanced preparation and every ordinary
/// entry: cancellation has priority over resolution, and a resolver that
/// reported no provenance decision inherits the registered route source.
async fn resolve_route_auth(
    resolver: &Arc<dyn AuthResolver>,
    source: AuthProvenanceSource,
    cancel: &CancellationToken,
) -> Result<ResolvedAuth, ModelsError> {
    if cancel.is_cancelled() {
        return Err(ModelsError::CallCancelled);
    }
    let mut auth = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(ModelsError::CallCancelled),
        resolved = resolver.resolve() => resolved?,
    };
    // Static/legacy resolvers return the default and rely on route assembly
    // for their non-secret source classification. A resolver that made a
    // real source/fallback decision reports it directly and must not have
    // that truthful result overwritten by route defaults.
    if auth.provenance == AuthProvenance::default() {
        auth.provenance = AuthProvenance {
            source,
            fallback: AuthFallback::NotAttempted,
        };
    }
    Ok(auth)
}

/// Map a preparation failure onto the typed cause a [`CallFailure`] carries
/// when the same step fails inside an ordinary [`ChatCall`].
fn models_error_cause(error: ModelsError) -> ProviderError {
    match error {
        ModelsError::CallCancelled => ProviderError::Cancelled,
        ModelsError::RouteNotDispatchable { provider } => {
            ProviderError::Config(ProviderErrorSummary::sanitized(format!(
                "provider '{provider}' has no dispatchable route"
            )))
        }
        ModelsError::Provider(error) => error,
        ModelsError::Registry(error) => {
            ProviderError::Config(ProviderErrorSummary::sanitized(error.to_string()))
        }
        other => ProviderError::Config(ProviderErrorSummary::sanitized(other.to_string())),
    }
}

fn setup_config_error(message: String) -> ProviderError {
    ProviderError::Config(ProviderErrorSummary::sanitized(message))
}

// ---------------------------------------------------------------------------
mod registration;
pub use registration::{
    ClassifierApiHandle, ClassifierNativeOptions, ImageApiHandle, ImageNativeOptions,
};

// ---------------------------------------------------------------------------
mod chat_call;
pub use chat_call::ChatCall;
