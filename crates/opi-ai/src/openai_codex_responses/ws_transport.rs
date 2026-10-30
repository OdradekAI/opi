//! Real Codex WebSocket transport (task 20.16, DEL-07 / RDF-24).
//!
//! This module owns the production path from the Codex adapter through the
//! source-qualified strict Upgrade bridge ([`crate::ws`]) into the private
//! frame/PMD/IO protocol owner ([`crate::ws::io::Driver`]): framing, PMD with
//! the negotiated window, the `response.create` text frame, the shared
//! Responses event mapper (identical public events to the SSE transport),
//! explicit client Close, and active/idle cancellation.
//!
//! Transport policy mirrors the reviewed source: the effective transport is
//! resolved first (omitted and explicit `Auto` follow the same delta rules);
//! a delta request is only sent when the non-input fields are unchanged and
//! the previous full input plus the actual response items form an exact
//! prefix of the new input, otherwise the full input is sent; a continuation
//! is published only after a successful terminal with a response id.
//! `previous_response_not_found` allows exactly one full-input retry before
//! public output, `websocket_connection_limit_reached` before generation
//! starts allows exactly one reconnect, and the WS→SSE fallback is permitted
//! only for transport-class failures before the first effective provider
//! event. Abort, API, auth and protocol failures never masquerade as
//! transport loss; an output-delivered error ends that stream.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::Duration;

use futures_util::FutureExt;
use futures_util::future::{BoxFuture, Shared};
use secrecy::ExposeSecret;
use tokio_util::sync::CancellationToken;

use crate::auth::{AuthScheme, ResolvedAuth};
use crate::callbacks::RequestCallbacks;
use crate::codex_resources::{GenerationGuard, GenerationPhase};
use crate::http::HttpClient;
use crate::message::AssistantMessage;
use crate::openai_responses_shared::{
    ParsedEvent, ResponsesEvent, ResponsesMapper, ResponsesWireTools,
};
use crate::provider::{ProviderError, ProviderErrorSummary, TransportPreference};
use crate::replay::ReplayRoute;
use crate::stream::{AssistantStreamEvent, ProducerSnapshot, send_or_cancel};
use crate::ws::{self, WsUpgradeError};

use super::{PROVIDER_ID, RESPONSES_PATH, UPSTREAM_STREAM_ERROR};

type Driver = crate::ws::io::Driver<reqwest::Upgraded>;

/// Source-pinned WebSocket beta header (replaces the SSE
/// `responses=experimental` value on the WebSocket handshake).
const OPENAI_BETA_RESPONSES_WEBSOCKETS: &str = "responses_websockets=2026-02-06";
/// Source-pinned WebSocket connect timeout.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// Source-pinned maximum pooled connection age.
const MAX_CONNECTION_AGE: Duration = Duration::from_secs(55 * 60);
/// Source-pinned idle lifetime, checked only on the next acquire.
const MAX_IDLE_TIME: Duration = Duration::from_secs(5 * 60);
/// Budget for the explicit client Close after a completed temporary
/// connection. The ws owner accepts only absolute deadlines; this bounds the
/// handshake without waiting on an unresponsive peer forever.
const CLOSE_BUDGET: Duration = Duration::from_secs(30);
const MISSING_RESPONSE_CODE: &str = "previous_response_not_found";
const CONNECTION_LIMIT_CODE: &str = "websocket_connection_limit_reached";

/// Provider-owned WebSocket transport state: session-scoped pooled
/// connections with their continuation state, and the per-session WS→SSE
/// fallback marker.
#[derive(Default)]
pub(crate) struct WsTransportState {
    connections: Mutex<HashMap<(String, String), PoolSlot>>,
    fallback_sessions: Mutex<HashSet<String>>,
    pending_closes: Mutex<PendingCloses>,
    /// Resource generation registry (task 20.17): every WS-family call
    /// registers one generation before wire work; cleanup cutoffs cancel
    /// the matching old work and await its local exits.
    resources: std::sync::Arc<crate::codex_resources::ResourceRegistry>,
}

type SharedClose = Shared<BoxFuture<'static, Result<(), String>>>;

#[derive(Default)]
struct PendingCloses {
    next_id: u64,
    jobs: HashMap<u64, (String, SharedClose)>,
}

pub(super) struct CleanupClose {
    pub(super) id: u64,
    future: SharedClose,
}

impl CleanupClose {
    pub(super) async fn resolve(self) -> Result<(), String> {
        self.future.await
    }
}

impl WsTransportState {
    /// Called under the generation cutoff lock. The state retains each Close
    /// separately from its waiters, so dropping one barrier cannot discard
    /// unfinished release or make another session wait for it.
    pub(super) fn prepare_cleanup(&self, session: Option<&str>) -> (Vec<CleanupClose>, usize) {
        let (drivers, cleared) = self.take_cleanup_connections(session);
        let mut pending = self.pending_closes.lock().unwrap();
        for (session, mut driver) in drivers {
            pending.next_id += 1;
            let id = pending.next_id;
            let future = async move { silent_close(&mut driver).await }
                .boxed()
                .shared();
            pending.jobs.insert(id, (session, future));
        }
        let jobs = pending
            .jobs
            .iter()
            .filter(|(_, (stored_session, _))| {
                session.is_none_or(|target| stored_session == target)
            })
            .map(|(&id, (_, future))| CleanupClose {
                id,
                future: future.clone(),
            })
            .collect();
        (jobs, cleared)
    }

    /// Remove completed results only after the whole barrier has awaited its
    /// selected work. A dropped waiter leaves both successes and failures
    /// available for the next cleanup.
    pub(super) fn finish_cleanup(&self, completed: &[u64]) {
        let mut pending = self.pending_closes.lock().unwrap();
        for id in completed {
            pending.jobs.remove(id);
        }
    }

    pub(super) fn drop_pending_closes(&self) {
        self.pending_closes.lock().unwrap().jobs.clear();
    }
    pub(crate) fn registry(&self) -> &std::sync::Arc<crate::codex_resources::ResourceRegistry> {
        &self.resources
    }

    /// Remove the pooled connections and sticky fallback markers matched by
    /// the cleanup selector. Returns the taken drivers (to close) and the
    /// number of cleared fallback markers. `None` matches everything.
    pub(crate) fn take_cleanup_connections(
        &self,
        session: Option<&str>,
    ) -> (Vec<(String, Driver)>, usize) {
        let mut drivers = Vec::new();
        {
            let mut connections = self.connections.lock().unwrap();
            let matched_keys: Vec<(String, String)> = connections
                .keys()
                .filter(|(pooled_session, _)| match session {
                    Some(target) => pooled_session == target,
                    None => true,
                })
                .cloned()
                .collect();
            for key in matched_keys {
                if let Some(slot) = connections.remove(&key)
                    && let Some(entry) = slot.idle
                {
                    drivers.push((key.0, entry.driver));
                }
            }
        }
        let cleared = match session {
            Some(target) => usize::from(self.fallback_sessions.lock().unwrap().remove(target)),
            None => self.fallback_sessions.lock().unwrap().drain().count(),
        };
        (drivers, cleared)
    }
}

struct PooledConnection {
    endpoint: String,
    driver: Driver,
    compression: bool,
    created_at: tokio::time::Instant,
    released_at: tokio::time::Instant,
    continuation: Option<WsContinuation>,
}

/// A busy reservation remains visible while its driver is outside the map.
/// Overlapping calls use temporary connections rather than replacing it.
struct PoolSlot {
    marker: std::sync::Arc<()>,
    idle: Option<PooledConnection>,
}

struct PoolReservation<'a> {
    connections: &'a Mutex<HashMap<(String, String), PoolSlot>>,
    key: (String, String),
    marker: std::sync::Arc<()>,
}

impl PoolReservation<'_> {
    fn publish(&self, connection: &mut Option<PooledConnection>) -> bool {
        let mut connections = self.connections.lock().unwrap();
        if let Some(slot) = connections.get_mut(&self.key)
            && std::sync::Arc::ptr_eq(&slot.marker, &self.marker)
        {
            slot.idle = connection.take();
            true
        } else {
            false
        }
    }
}

impl Drop for PoolReservation<'_> {
    fn drop(&mut self) {
        let mut connections = self.connections.lock().unwrap();
        if connections.get(&self.key).is_some_and(|slot| {
            slot.idle.is_none() && std::sync::Arc::ptr_eq(&slot.marker, &self.marker)
        }) {
            connections.remove(&self.key);
        }
    }
}

#[derive(Clone)]
struct WsContinuation {
    last_full_body: serde_json::Value,
    last_input: Vec<serde_json::Value>,
    last_response_id: String,
    last_response_items: Vec<serde_json::Value>,
}

/// What the bounded WebSocket attempt loop should do next.
pub(super) enum WsLoopOutcome {
    /// The public stream finished (terminal or delivered error).
    Done,
    /// Transport-class failure before the first provider event: continue on SSE
    /// with a fixed, redacted-safe diagnostic reason.
    Fallback(&'static str),
    /// Typed terminal failure to publish (partial preserved).
    Terminal(ProviderError),
}

/// How a failed WebSocket attempt classifies for the bounded loop.
enum AttemptFlow {
    RetryFullInput(ProviderError),
    Reconnect(ProviderError),
    Fallback(&'static str),
    Terminal(ProviderError),
}

/// One acquired connection: pooled entries carry their slot identity and
/// return to the map on success; temporary connections complete an explicit
/// client Close.
struct Acquired<'a> {
    reservation: Option<PoolReservation<'a>>,
    compression: bool,
    created_at: Option<tokio::time::Instant>,
    driver: Driver,
}

/// Extract the account id and bearer token shared by both transports.
pub(super) fn bearer_parts(resolved: &ResolvedAuth) -> Result<(String, String), ProviderError> {
    let account_id = resolved
        .account_id
        .as_deref()
        .filter(|account_id| !account_id.trim().is_empty())
        .ok_or_else(|| ProviderError::AccountIdMissing {
            provider_id: PROVIDER_ID.into(),
        })?
        .to_owned();
    let secret = resolved.secret.expose_secret();
    let bearer = match resolved.scheme {
        AuthScheme::Bearer => format!("Bearer {secret}"),
        AuthScheme::ApiKey | AuthScheme::AwsSigV4(_) | AuthScheme::AnthropicFederation => {
            return Err(ProviderError::Config(
                ProviderErrorSummary::attested_static(
                    "OpenAI Codex Responses accepts only Bearer authentication",
                ),
            ));
        }
    };
    Ok((account_id, bearer))
}

fn ws_url(base_url: &str) -> String {
    let url = crate::endpoint::join_endpoint(base_url, RESPONSES_PATH);
    if let Some(rest) = url.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = url.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        url
    }
}

/// Handshake headers for the WebSocket transport: the SSE `OpenAI-Beta` is
/// replaced by the websocket beta value, `accept`/`content-type` are dropped,
/// and `session-id`/`x-client-request-id` carry the websocket request id.
fn handshake_headers(
    bearer: &str,
    account_id: &str,
    request_id: &str,
    extra_headers: &[(String, String)],
) -> Vec<(String, String)> {
    let mut headers = vec![
        ("authorization".to_owned(), bearer.to_owned()),
        ("chatgpt-account-id".to_owned(), account_id.to_owned()),
        ("originator".to_owned(), "opi".to_owned()),
        (
            "OpenAI-Beta".to_owned(),
            OPENAI_BETA_RESPONSES_WEBSOCKETS.to_owned(),
        ),
        ("session-id".to_owned(), request_id.to_owned()),
        ("x-client-request-id".to_owned(), request_id.to_owned()),
    ];
    headers.extend(extra_headers.iter().cloned());
    headers
}

fn classify_upgrade_error(error: &WsUpgradeError) -> AttemptFlow {
    match error {
        WsUpgradeError::Transport(cause) => {
            if cause.is_timeout() {
                AttemptFlow::Terminal(ProviderError::Timeout)
            } else {
                AttemptFlow::Fallback("upgrade_transport_error")
            }
        }
        // Authentication and rate-limit statuses stay typed terminal
        // failures; any other non-101 response means the endpoint did not
        // speak WebSocket for this attempt, which is a transport mismatch.
        WsUpgradeError::Status(status) if !matches!(status, 401 | 403 | 429 | 408 | 504) => {
            AttemptFlow::Fallback("upgrade_status")
        }
        WsUpgradeError::Status(status) => {
            AttemptFlow::Terminal(super::map_http_upgrade_status(*status))
        }
        // Strict handshake validation failures are protocol problems: they
        // never masquerade as transport loss.
        _ => AttemptFlow::Terminal(ProviderError::StreamError(
            ProviderErrorSummary::attested_static("WebSocket upgrade failed strict validation"),
        )),
    }
}

/// Only transport loss can fall back before the first provider event.
/// Frame/PMD validation, allocation and owner-state failures are terminal;
/// resending them over another transport would repeat the logical request.
fn classify_transport_fault(
    fault: &crate::ws::io::Fault,
    received_event: bool,
    mapper: &ResponsesMapper,
) -> AttemptFlow {
    let terminal = |summary: &'static str| {
        AttemptFlow::Terminal(mapper.failure(ProviderError::StreamError(
            ProviderErrorSummary::attested_static(summary),
        )))
    };
    if !matches!(
        fault,
        crate::ws::io::Fault::Io(_)
            | crate::ws::io::Fault::TransportEof
            | crate::ws::io::Fault::Closed
            | crate::ws::io::Fault::CloseTimeout
    ) {
        return terminal("OpenAI Codex returned malformed streaming data");
    }
    if received_event {
        terminal("WebSocket transport failed after stream start")
    } else {
        AttemptFlow::Fallback(match fault {
            crate::ws::io::Fault::TransportEof | crate::ws::io::Fault::Closed => "transport_eof",
            crate::ws::io::Fault::CloseTimeout => "idle_timeout",
            _ => "transport_error",
        })
    }
}

/// The transport ended (clean Close, close frame or EOF) without a terminal
/// event: EOF is never success. Before the first provider event this is a transport
/// loss eligible for the bounded SSE fallback; after that event it ends the
/// stream terminally.
fn websocket_closed_before_completion(
    received_event: bool,
    mapper: &ResponsesMapper,
) -> AttemptFlow {
    if received_event {
        AttemptFlow::Terminal(mapper.failure(ProviderError::StreamError(
            ProviderErrorSummary::attested_static(
                "WebSocket stream closed before response.completed",
            ),
        )))
    } else {
        AttemptFlow::Fallback("transport_eof")
    }
}

/// Compute the delta request body for a reusable continuation, or `None` when
/// the non-input fields changed or the previous full input plus actual
/// response items are not an exact prefix of the new input.
fn delta_body(
    body: &serde_json::Value,
    continuation: &WsContinuation,
) -> Option<serde_json::Value> {
    let without = |value: &serde_json::Value| -> Option<serde_json::Value> {
        let mut stripped = value.clone();
        stripped.as_object_mut()?.remove("input");
        stripped.as_object_mut()?.remove("previous_response_id");
        Some(stripped)
    };
    if without(body)? != without(&continuation.last_full_body)? {
        return None;
    }
    let current = body.get("input")?.as_array()?;
    let mut baseline = continuation.last_input.clone();
    baseline.extend(continuation.last_response_items.iter().cloned());
    if current.len() < baseline.len() || current[..baseline.len()] != baseline[..] {
        return None;
    }
    let mut delta = body.clone();
    delta["previous_response_id"] =
        serde_json::Value::String(continuation.last_response_id.clone());
    delta["input"] = serde_json::Value::Array(current[baseline.len()..].to_vec());
    Some(delta)
}

/// Wire input items for the assistant turn that just completed, excluding
/// tool-call outputs (mirrors the reviewed source's continuation baseline).
fn assistant_response_items(
    message: &AssistantMessage,
    route: &ReplayRoute,
    grammar_input_properties: &std::collections::HashMap<String, String>,
) -> Vec<serde_json::Value> {
    let projected = crate::message::Message::Assistant(message.clone());
    let Ok(replay) = crate::replay::project_replay(
        std::slice::from_ref(&projected),
        route,
        crate::replay::ReplayProjection::NativeOrCollapse,
    ) else {
        return Vec::new();
    };
    crate::openai_responses_shared::convert_messages(
        &replay.messages,
        route,
        grammar_input_properties,
        None,
    )
    .into_iter()
    .filter(|item| {
        let item_type = item.get("type").and_then(serde_json::Value::as_str);
        item_type != Some("function_call_output") && item_type != Some("custom_tool_call_output")
    })
    .collect()
}

fn successful_terminal(message: &AssistantMessage) -> bool {
    matches!(
        message.stop_reason,
        crate::stream::StopReason::Stop
            | crate::stream::StopReason::Length
            | crate::stream::StopReason::ToolUse
    )
}

/// The bounded WebSocket attempt loop with the SSE handoff decision.
#[allow(clippy::too_many_arguments)]
pub(super) async fn drive(
    state: &WsTransportState,
    client: &HttpClient,
    transport: TransportPreference,
    session_key: Option<String>,
    base_url: &str,
    resolved: &ResolvedAuth,
    full_body: serde_json::Value,
    requested_service_tier: Option<crate::stream::ServiceTier>,
    extra_headers: Vec<(String, String)>,
    wire_facts: Option<ResponsesWireTools>,
    route: ReplayRoute,
    timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    lease: &GenerationGuard,
    callbacks: RequestCallbacks,
    snapshot: ProducerSnapshot,
    tx: tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
) -> WsLoopOutcome {
    // The generation token carries the cleanup cutoff and the linked caller
    // parent cancellation; every select arm below observes it.
    let cancel = lease.token();
    // Prepared-phase cutoff: a cleanup that fired before this producer
    // started never reaches the wire.
    if cancel.is_cancelled() {
        return WsLoopOutcome::Terminal(ProviderError::Cancelled);
    }
    let (account_id, bearer) = match bearer_parts(resolved) {
        Ok(parts) => parts,
        Err(error) => return WsLoopOutcome::Terminal(error),
    };
    // Per-session sticky fallback: once recorded, later calls in the same
    // session resolve to SSE without touching WebSocket again.
    if let Some(session) = session_key.as_deref()
        && state.fallback_sessions.lock().unwrap().contains(session)
    {
        return WsLoopOutcome::Fallback("session_fallback");
    }

    let mut continuation: Option<WsContinuation> = None;
    let mut retried_missing = false;
    let mut retried_limit = false;
    loop {
        // Cutoff forbids late retries: a cancelled generation never starts
        // another attempt.
        if cancel.is_cancelled() {
            return WsLoopOutcome::Terminal(ProviderError::Cancelled);
        }
        let outcome = attempt(
            state,
            client,
            transport,
            session_key.as_deref(),
            &account_id,
            &bearer,
            base_url,
            &full_body,
            requested_service_tier,
            &mut continuation,
            &extra_headers,
            wire_facts.as_ref(),
            &route,
            timeout,
            connect_timeout,
            lease,
            &callbacks,
            &snapshot,
            &tx,
        )
        .await;
        match outcome.flow {
            AttemptFlowResult::Done => return WsLoopOutcome::Done,
            AttemptFlowResult::RetryFullInput(_error) if !retried_missing => {
                retried_missing = true;
                continuation = None;
            }
            AttemptFlowResult::Reconnect(_error) if !retried_limit => {
                retried_limit = true;
                continuation = outcome.continuation;
            }
            AttemptFlowResult::RetryFullInput(error) | AttemptFlowResult::Reconnect(error) => {
                return WsLoopOutcome::Terminal(error);
            }
            AttemptFlowResult::Fallback(reason) => {
                if let Some(session) = session_key.as_deref()
                    && lease
                        .with_live(|| {
                            state
                                .fallback_sessions
                                .lock()
                                .unwrap()
                                .insert(session.to_owned())
                        })
                        .is_none()
                {
                    return WsLoopOutcome::Terminal(ProviderError::Cancelled);
                }
                return WsLoopOutcome::Fallback(reason);
            }
            AttemptFlowResult::Terminal(error) => {
                if let Some(session) = session_key.as_deref()
                    && matches!(error, ProviderError::Network(_) | ProviderError::Timeout)
                    && lease
                        .with_live(|| {
                            state
                                .fallback_sessions
                                .lock()
                                .unwrap()
                                .insert(session.to_owned())
                        })
                        .is_none()
                {
                    return WsLoopOutcome::Terminal(ProviderError::Cancelled);
                }
                return WsLoopOutcome::Terminal(error);
            }
        }
    }
}

struct AttemptOutcome {
    flow: AttemptFlowResult,
    continuation: Option<WsContinuation>,
}

enum AttemptFlowResult {
    Done,
    RetryFullInput(ProviderError),
    Reconnect(ProviderError),
    Fallback(&'static str),
    Terminal(ProviderError),
}

/// Acquire or open one WebSocket connection for this call.
#[allow(clippy::too_many_arguments)]
async fn acquire<'a>(
    state: &'a WsTransportState,
    client: &HttpClient,
    url: &str,
    headers: &[(String, String)],
    pooled_key: Option<(&str, &str)>,
    cancel: &CancellationToken,
    connect_timeout: Option<Duration>,
    lease: &GenerationGuard,
) -> Result<(Acquired<'a>, Option<WsContinuation>), AttemptFlow> {
    let mut previous = None;
    let reservation = pooled_key.and_then(|(session, account)| {
        let key = (session.to_owned(), account.to_owned());
        let mut connections = state.connections.lock().unwrap();
        if connections
            .get(&key)
            .is_some_and(|slot| slot.idle.is_none())
        {
            return None;
        }
        let marker = std::sync::Arc::new(());
        previous = connections
            .insert(
                key.clone(),
                PoolSlot {
                    marker: marker.clone(),
                    idle: None,
                },
            )
            .and_then(|slot| slot.idle);
        Some(PoolReservation {
            connections: &state.connections,
            key,
            marker,
        })
    });
    if let Some(mut entry) = previous {
        if entry.endpoint == url
            && entry.created_at.elapsed() < MAX_CONNECTION_AGE
            && entry.released_at.elapsed() < MAX_IDLE_TIME
        {
            return Ok((
                Acquired {
                    reservation,
                    created_at: Some(entry.created_at),
                    driver: entry.driver,
                    compression: entry.compression,
                },
                entry.continuation,
            ));
        }
        // Check idle and absolute age only on acquire: neither interrupts a
        // busy generation. Expiry drops the continuation with its socket.
        // Release the ineligible connection before opening its replacement.
        if let Err(detail) = silent_close(&mut entry.driver).await {
            lease.record_unconfirmed(detail);
        }
    }
    let uri = match url::Url::parse(url) {
        Ok(uri) => uri,
        Err(_) => {
            return Err(AttemptFlow::Terminal(ProviderError::Config(
                ProviderErrorSummary::attested_static("invalid Codex WebSocket URL"),
            )));
        }
    };
    let upgrade = ws::WsUpgradeRequest::new(uri)
        .headers(headers.to_vec())
        .permessage_deflate(ws::PmdOffer {
            client_no_context_takeover: true,
            client_max_window_bits: Some(None),
        });
    let connect = async {
        let budget = connect_timeout.unwrap_or(CONNECT_TIMEOUT);
        if budget.is_zero() {
            ws::upgrade(client, upgrade)
                .await
                .map_err(|error| classify_upgrade_error(&error))
        } else {
            match tokio::time::timeout(budget, ws::upgrade(client, upgrade)).await {
                Ok(result) => result.map_err(|error| classify_upgrade_error(&error)),
                Err(_) => Err(AttemptFlow::Terminal(ProviderError::Timeout)),
            }
        }
    };
    let upgraded = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(AttemptFlow::Terminal(ProviderError::Cancelled)),
        result = connect => result?,
    };
    let parameters =
        upgraded
            .negotiation
            .driver_parameters()
            .map(|pmd| crate::ws::io::PmdParameters {
                server_no_context_takeover: pmd.server_no_context_takeover,
                client_no_context_takeover: pmd.client_no_context_takeover,
                server_max_window_bits: pmd.server_max_window_bits,
                client_max_window_bits: pmd.client_max_window_bits.unwrap_or(15),
            });
    let compression = parameters.is_some();
    let driver = crate::ws::io::Driver::new(upgraded.stream, parameters).map_err(|_| {
        AttemptFlow::Terminal(ProviderError::StreamError(
            ProviderErrorSummary::attested_static("WebSocket owner initialization failed"),
        ))
    })?;
    Ok((
        Acquired {
            reservation,
            compression,
            created_at: Some(tokio::time::Instant::now()),
            driver,
        },
        None,
    ))
}

/// One full WebSocket attempt: acquire, delta resolution, `response.create`,
/// the shared event mapper, completion, continuation publication and release.
#[allow(clippy::too_many_arguments)]
async fn attempt(
    state: &WsTransportState,
    client: &HttpClient,
    transport: TransportPreference,
    session_key: Option<&str>,
    account_id: &str,
    bearer: &str,
    base_url: &str,
    full_body: &serde_json::Value,
    requested_service_tier: Option<crate::stream::ServiceTier>,
    continuation: &mut Option<WsContinuation>,
    extra_headers: &[(String, String)],
    wire_facts: Option<&ResponsesWireTools>,
    route: &ReplayRoute,
    timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    lease: &GenerationGuard,
    callbacks: &RequestCallbacks,
    snapshot: &ProducerSnapshot,
    tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
) -> AttemptOutcome {
    let cancel = lease.token();
    let url = ws_url(base_url);
    let websocket_request_id = session_key
        .map(str::to_owned)
        .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
    let headers = handshake_headers(bearer, account_id, &websocket_request_id, extra_headers);
    let use_cached_context = matches!(
        transport,
        TransportPreference::WebsocketCached | TransportPreference::Auto
    ) && session_key.is_some();

    lease.set_phase(GenerationPhase::Connecting);
    let (mut acquired, pooled_continuation) = match acquire(
        state,
        client,
        &url,
        &headers,
        session_key.map(|session| (session, account_id)),
        &cancel,
        connect_timeout,
        lease,
    )
    .await
    {
        Ok(acquired) => acquired,
        Err(flow) => {
            return AttemptOutcome {
                flow: flow.into(),
                continuation: continuation.clone(),
            };
        }
    };
    // The attached WS transport seals generic acquisition. Its own bounded
    // reconnect/full-input/fallback branches retain their separate budgets.
    snapshot.received_response();
    lease.set_phase(if acquired.reservation.is_some() {
        GenerationPhase::Busy
    } else {
        GenerationPhase::Temporary
    });
    if use_cached_context && pooled_continuation.is_some() {
        *continuation = pooled_continuation;
    }

    let send_body = if use_cached_context {
        continuation
            .as_ref()
            .and_then(|cont| delta_body(full_body, cont))
            .unwrap_or_else(|| full_body.clone())
    } else {
        full_body.clone()
    };
    let mut frame_body = send_body;
    frame_body["type"] = serde_json::Value::String("response.create".into());

    let mut mapper = ResponsesMapper::new(
        PROVIDER_ID,
        crate::model_info::WireApi::OpenAiCodexResponses,
        &route.model,
    )
    .with_requested_service_tier(requested_service_tier);
    if let Some(wire) = wire_facts {
        mapper.set_grammar_tools(wire.grammar_input_properties.clone());
        mapper.set_strict_tools(wire.strict_tools.clone());
        mapper.seed_diagnostics(wire.diagnostics.clone());
    }
    mapper.set_snapshot(snapshot.clone());

    let outcome = run_stream(
        &mut acquired.driver,
        &frame_body,
        acquired.compression,
        &mut mapper,
        timeout,
        &cancel,
        callbacks,
        tx,
    )
    .await;
    match outcome {
        Ok((delivered_error, published)) => {
            let final_continuation = if published && !delivered_error {
                mapper
                    .partial()
                    .response_id
                    .clone()
                    .map(|response_id| WsContinuation {
                        last_full_body: full_body.clone(),
                        last_input: full_body
                            .get("input")
                            .and_then(serde_json::Value::as_array)
                            .cloned()
                            .unwrap_or_default(),
                        last_response_id: response_id,
                        last_response_items: assistant_response_items(
                            mapper.partial(),
                            route,
                            wire_facts
                                .map(|wire| &wire.grammar_input_properties)
                                .unwrap_or(&Default::default()),
                        ),
                    })
            } else {
                None
            };
            // Output-delivered errors end that stream: the connection does
            // not return to the pool.
            let mut returning = Some(PooledConnection {
                endpoint: url,
                driver: acquired.driver,
                compression: acquired.compression,
                created_at: acquired
                    .created_at
                    .unwrap_or_else(tokio::time::Instant::now),
                released_at: tokio::time::Instant::now(),
                continuation: final_continuation.clone(),
            });
            if !delivered_error && let Some(reservation) = acquired.reservation.as_ref() {
                let _ = lease.with_live(|| reservation.publish(&mut returning));
            }
            if let Some(mut temporary) = returning {
                let closed = tokio::select! {
                    biased;
                    _ = cancel.cancelled() => silent_close(&mut temporary.driver).await,
                    result = close_temporary(&mut temporary.driver) => result,
                };
                if let Err(detail) = closed {
                    lease.record_unconfirmed(detail);
                }
            }
            AttemptOutcome {
                flow: AttemptFlowResult::Done,
                continuation: final_continuation,
            }
        }
        Err(flow) => {
            // Failed attempts never return a connection to the pool. A
            // cancelled generation completes an explicit prompt Close (the
            // source's silent close); other failures release IO through the
            // owner's Drop without protocol output.
            if matches!(flow, AttemptFlow::Terminal(ProviderError::Cancelled))
                && let Err(detail) = silent_close(&mut acquired.driver).await
            {
                lease.record_unconfirmed(detail);
            }
            AttemptOutcome {
                flow: flow.into(),
                continuation: None,
            }
        }
    }
}

/// Complete the explicit client Close on a temporary connection.
async fn close_temporary(driver: &mut Driver) -> Result<(), String> {
    close_with_budget(driver, CLOSE_BUDGET).await
}

/// Cleanup confirms a peer Close only within this local release budget.
const SILENT_CLOSE_CONFIRM: Duration = Duration::from_millis(250);

pub(super) async fn silent_close(driver: &mut Driver) -> Result<(), String> {
    close_with_budget(driver, SILENT_CLOSE_CONFIRM).await
}

/// A single caller deadline covers the Close write, flush, peer response,
/// transport shutdown and EOF. Dropping the borrow leaves state in Driver.
async fn close_with_budget<S>(
    driver: &mut crate::ws::io::Driver<S>,
    budget: Duration,
) -> Result<(), String>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    if !driver.close_started() {
        driver
            .queue(8, &1000_u16.to_be_bytes(), false, ws::fresh_mask())
            .map_err(|fault| format!("close frame rejected: {fault:?}"))?;
    }
    driver
        .finish_close(Some(tokio::time::Instant::now() + budget))
        .await
        .map_err(|fault| format!("close handshake unconfirmed: {fault:?}"))
}

fn error_code(value: &serde_json::Value) -> Option<&str> {
    let nested = value.get("error").and_then(|error| error.get("code"));
    value
        .get("code")
        .and_then(serde_json::Value::as_str)
        .or_else(|| nested.and_then(serde_json::Value::as_str))
}

/// Drive the event stream for one attempt. Returns whether a successful
/// completion with a response id was observed (continuation publication is
/// the caller's decision).
#[allow(clippy::too_many_arguments)]
async fn run_stream(
    driver: &mut Driver,
    frame_body: &serde_json::Value,
    compression: bool,
    mapper: &mut ResponsesMapper,
    timeout: Option<Duration>,
    cancel: &CancellationToken,
    callbacks: &RequestCallbacks,
    tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
) -> Result<(bool, bool), AttemptFlow> {
    let payload = serde_json::to_string(frame_body).unwrap_or_default();
    if let Err(fault) = driver.queue(1, payload.as_bytes(), compression, ws::fresh_mask()) {
        return Err(classify_transport_fault(&fault, false, mapper));
    }
    // Active send cancellation: the cutoff is observed before the request
    // frame leaves the local buffer whenever possible.
    let flushed = tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(crate::ws::io::Fault::Closed),
        result = driver.flush() => result,
    };
    if let Err(fault) = flushed {
        if cancel.is_cancelled() {
            return Err(AttemptFlow::Terminal(
                mapper.failure(ProviderError::Cancelled),
            ));
        }
        return Err(classify_transport_fault(&fault, false, mapper));
    }

    let mut emitted_any = false;
    let mut received_event = false;
    let mut delivered_error = false;
    loop {
        if mapper.saw_done {
            return Ok((delivered_error, successful_terminal(mapper.partial())));
        }
        let next = async {
            match timeout.filter(|budget| !budget.is_zero()) {
                Some(budget) => match tokio::time::timeout(budget, driver.next()).await {
                    Ok(next) => next,
                    Err(_) => Err(crate::ws::io::Fault::CloseTimeout),
                },
                None => driver.next().await,
            }
        };
        let message = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                return Err(AttemptFlow::Terminal(
                    mapper.failure(ProviderError::Cancelled),
                ));
            }
            next = next => next,
        };
        let message = match message {
            Ok(message) => message,
            Err(fault) => {
                return Err(classify_transport_fault(&fault, received_event, mapper));
            }
        };
        let Some(message) = message else {
            // Transport ended (clean Close or EOF) without a terminal event:
            // EOF is never success.
            return Err(websocket_closed_before_completion(received_event, mapper));
        };
        match message.opcode {
            8 => return Err(websocket_closed_before_completion(received_event, mapper)),
            9 => {
                if let Err(fault) = driver.queue(10, &message.payload, false, ws::fresh_mask()) {
                    return Err(classify_transport_fault(&fault, received_event, mapper));
                }
                let flushed = tokio::select! {
                    biased;
                    _ = cancel.cancelled() => return Err(AttemptFlow::Terminal(mapper.failure(ProviderError::Cancelled))),
                    result = driver.flush() => result,
                };
                if let Err(fault) = flushed {
                    return Err(classify_transport_fault(&fault, received_event, mapper));
                }
                continue;
            }
            10 => continue,
            _ => {}
        }
        let text = if message.opcode == 2 {
            // The source projects binary messages with replacement decoding;
            // text frames have already passed strict UTF-8 validation.
            String::from_utf8_lossy(&message.payload)
        } else {
            match std::str::from_utf8(&message.payload) {
                Ok(text) => std::borrow::Cow::Borrowed(text),
                Err(_) => {
                    return Err(AttemptFlow::Terminal(mapper.failure(
                        ProviderError::StreamError(ProviderErrorSummary::attested_static(
                            "OpenAI Codex returned malformed streaming data",
                        )),
                    )));
                }
            }
        };
        let value: serde_json::Value = match serde_json::from_str(&text) {
            Ok(value) => value,
            Err(_) => {
                // Invalid JSON is a protocol failure: never fallback.
                return Err(AttemptFlow::Terminal(mapper.failure(
                    ProviderError::StreamError(ProviderErrorSummary::attested_static(
                        "OpenAI Codex returned malformed streaming data",
                    )),
                )));
            }
        };
        tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(AttemptFlow::Terminal(mapper.failure(ProviderError::Cancelled))),
            result = callbacks.notify_raw_async(&value) => {
                if let Err(error) = result { return Err(AttemptFlow::Terminal(mapper.failure(error))); }
            }
        }
        // Missing-previous recovery precedes public output; connection-limit
        // recovery has the stricter first-provider-event boundary.
        if value.get("type").and_then(serde_json::Value::as_str) == Some("error") {
            let upstream = || {
                mapper.failure(ProviderError::StreamError(
                    ProviderErrorSummary::attested_static(UPSTREAM_STREAM_ERROR),
                ))
            };
            match error_code(&value) {
                Some(MISSING_RESPONSE_CODE) if !emitted_any => {
                    return Err(AttemptFlow::RetryFullInput(upstream()));
                }
                Some(CONNECTION_LIMIT_CODE) if !received_event => {
                    return Err(AttemptFlow::Reconnect(upstream()));
                }
                _ => {}
            }
        }
        match ResponsesEvent::try_from_json_str(&text) {
            ParsedEvent::Valid(event) => {
                // Ordinary recovery seals at the provider event, even when its
                // mapper emits no public content (for example message setup).
                if value
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|kind| !kind.is_empty())
                {
                    received_event = true;
                }
                let event = match event {
                    ResponsesEvent::Error { .. } => ResponsesEvent::Error {
                        message: UPSTREAM_STREAM_ERROR.to_owned(),
                    },
                    event => event,
                };
                for item in mapper.process(event) {
                    emitted_any = true;
                    if item.is_err() {
                        delivered_error = true;
                    }
                    match send_or_cancel(cancel, tx, item).await {
                        Ok(true) => {}
                        Ok(false) => return Ok((delivered_error, false)),
                        Err(error) => {
                            return Err(AttemptFlow::Terminal(mapper.failure(error)));
                        }
                    }
                }
            }
            ParsedEvent::UsageError(error) => {
                return Err(AttemptFlow::Terminal(mapper.failure(
                    ProviderError::StreamError(ProviderErrorSummary::sanitized(error)),
                )));
            }
            ParsedEvent::Malformed { .. } => {
                return Err(AttemptFlow::Terminal(mapper.failure(
                    ProviderError::StreamError(ProviderErrorSummary::attested_static(
                        "OpenAI Codex returned malformed streaming data",
                    )),
                )));
            }
        }
    }
}

impl From<AttemptFlow> for AttemptFlowResult {
    fn from(flow: AttemptFlow) -> Self {
        match flow {
            AttemptFlow::RetryFullInput(error) => Self::RetryFullInput(error),
            AttemptFlow::Reconnect(error) => Self::Reconnect(error),
            AttemptFlow::Fallback(reason) => Self::Fallback(reason),
            AttemptFlow::Terminal(error) => Self::Terminal(error),
        }
    }
}

#[cfg(test)]
mod close_deadline_tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn cleanup_deadline_bounds_a_blocked_close_write_and_flush() {
        let (io, _held_peer) = tokio::io::duplex(1);
        let mut driver = crate::ws::io::Driver::new(io, None).unwrap();
        let started = tokio::time::Instant::now();
        let result = tokio::time::timeout(
            SILENT_CLOSE_CONFIRM * 2,
            close_with_budget(&mut driver, SILENT_CLOSE_CONFIRM),
        )
        .await
        .expect("the cleanup deadline must include writing the Close frame");
        assert!(result.is_err(), "an unresponsive peer cannot confirm Close");
        assert_eq!(started.elapsed(), SILENT_CLOSE_CONFIRM);
    }
}
