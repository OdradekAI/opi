use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use futures_util::StreamExt;
use opi_ai::CollectionError;
use opi_ai::message::{
    AssistantContent, InputContent, Message, ToolCall, ToolResultMessage, UserMessage,
};
use opi_ai::provider::{CacheRetention, ProviderError, Request};
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::diagnostic::code::*;
use crate::diagnostic::{Diagnostic, SOURCE_AGENT, SOURCE_PROVIDER, SOURCE_TOOL, Severity};
use crate::diagnostic_sink::DiagnosticSink;
use crate::event::{AgentEvent, AgentEventSink};
use crate::hooks::{
    AfterToolCallContext, AfterToolCallResult, AgentHooks, BeforeToolCallContext,
    BeforeToolCallResult, PrepareNextTurnContext, ShouldStopAfterTurnContext,
};
use crate::loop_types::{
    AgentError, AgentLoopConfig, AgentLoopContext, NextTurnState, validate_next_turn_candidate,
};
use crate::message::AgentMessage;
use crate::tool::{ExecutionMode, ToolDiagnostic, ToolResult};
use crate::validation;

/// Run the agent loop until completion or cancellation.
///
/// The loop operates on one complete [`NextTurnState`] (context, canonical
/// provider:model selection, inference) owned durably by the Agent.
/// Each turn prepares one logical model call through the registered
/// [`opi_ai::ProviderCollection`] (route lookup + auth resolution + validation) before
/// the retry loop, then opens every sequential attempt from that same opaque
/// prepared call. After a turn reaches a terminal outcome the loop finalizes
/// it, builds and validates a candidate next-turn state away from live state,
/// atomically applies it, and only then lets `should_stop_after_turn` observe
/// the applied state before any steering/follow-up polling.
///
/// When `context.evidence_sink` is `Some`, the loop emits ordered, redacted
/// evidence records (provider/tool/retry) over stable run/turn/call identities.
/// An emission failure advances the run's versioned evidence
/// health (fail-open for the run; the authorizer then fails closed under a
/// complete-evidence policy). Evidence setup and run finalization are the
/// caller's responsibility.
pub async fn agent_loop(
    context: AgentLoopContext,
    config: AgentLoopConfig,
    hooks: &dyn AgentHooks,
    events: AgentEventSink,
    cancel: CancellationToken,
) -> AgentLoopResult {
    let events = crate::event::redacting_event_sink(events);
    agent_loop_with_identities(context, config, hooks, events, cancel).await
}

/// Complete core-owned result of one low-level loop execution.
///
/// The actual state, terminal outcome, and final evidence health remain
/// composable even when execution fails. Discarding this value would also
/// discard evidence-health and uncertain-side-effect truth.
///
/// ```compile_fail
/// #![deny(unused_must_use)]
/// use opi_agent::AgentLoopResult;
/// fn result() -> AgentLoopResult { unimplemented!() }
/// fn discard() { result(); }
/// ```
#[must_use = "inspect execution outcome and evidence health before discarding a loop result"]
pub struct AgentLoopResult {
    pub(crate) state: NextTurnState,
    pub(crate) identities: crate::evidence::IdentityAllocator,
    pub(crate) evidence_health: crate::evidence::EvidenceHealth,
    pub(crate) terminal_outcome: crate::evidence::TerminalOutcome,
    pub(crate) error: Option<AgentError>,
}

impl std::fmt::Debug for AgentLoopResult {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentLoopResult")
            .field("state", &self.state)
            .field("error", &self.error)
            .field("terminal_outcome", &self.terminal_outcome)
            .field("evidence_health", &self.evidence_health)
            .finish_non_exhaustive()
    }
}

impl AgentLoopResult {
    /// State retained at the loop boundary, including any actual terminal tool
    /// result produced before a partial-side-effect or cleanup-unknown error.
    pub fn state(&self) -> &NextTurnState {
        &self.state
    }

    /// The owning execution error, if the loop did not complete normally.
    pub fn error(&self) -> Option<&AgentError> {
        self.error.as_ref()
    }

    /// Exact terminal execution outcome.
    pub fn terminal_outcome(&self) -> &crate::evidence::TerminalOutcome {
        &self.terminal_outcome
    }

    /// Final core-owned evidence-health snapshot.
    pub fn evidence_health(&self) -> &crate::evidence::EvidenceHealth {
        &self.evidence_health
    }

    /// Consume the lifecycle result into the traditional execution result.
    pub fn into_execution_result(self) -> Result<NextTurnState, AgentError> {
        match self.error {
            Some(error) => Err(error),
            None => Ok(self.state),
        }
    }
}

pub(crate) fn terminal_outcome_for_error(error: &AgentError) -> crate::evidence::TerminalOutcome {
    match error {
        AgentError::Cancelled | AgentError::Tool(crate::tool::ToolError::Cancelled) => {
            crate::evidence::TerminalOutcome::Cancelled
        }
        AgentError::Tool(crate::tool::ToolError::PartialSideEffect(_)) => {
            crate::evidence::TerminalOutcome::PartialSideEffect
        }
        AgentError::Tool(crate::tool::ToolError::CleanupUnknown(_)) => {
            crate::evidence::TerminalOutcome::CleanupUnknown
        }
        AgentError::Provider(_)
        | AgentError::InvalidArmedRun
        | AgentError::InvalidModelSpec { .. }
        | AgentError::UnknownProvider { .. }
        | AgentError::UnknownModel { .. }
        | AgentError::AuthFailed(_)
        | AgentError::CredentialNeeded { .. }
        | AgentError::CredentialRevoked { .. }
        | AgentError::AccountIdMissing { .. }
        | AgentError::Hook(_)
        | AgentError::Tool(crate::tool::ToolError::ExecutionFailed(_))
        | AgentError::MaxTurnsExceeded(_)
        | AgentError::EvidenceSetup(_)
        | AgentError::EvidenceFinalization(_)
        | AgentError::SessionResume(_)
        | AgentError::SessionPersist(_)
        | AgentError::RouteNotDispatchable { .. }
        | AgentError::RequestRouteMismatch { .. }
        | AgentError::CredentialTerminated { .. }
        | AgentError::AttemptAlreadyActive
        | AgentError::ProviderProtocol { .. }
        | AgentError::InvalidNextTurnCandidate(_)
        | AgentError::InvalidToolRegistration(_) => crate::evidence::TerminalOutcome::Failed,
    }
}

/// Loop entry that also returns the run's identity allocator and final
/// evidence health for post-loop lifecycle work.
pub(crate) async fn agent_loop_with_identities(
    context: AgentLoopContext,
    config: AgentLoopConfig,
    hooks: &dyn AgentHooks,
    events: AgentEventSink,
    cancel: CancellationToken,
) -> AgentLoopResult {
    // Clone the sink/collector/collection handles up front (before any partial
    // move out of `context`) so every failure path below can record an
    // observation and prepare the next route. `None` means emission is disabled
    // and nothing below observes any behavior change.
    let diagnostic_sink = context.diagnostic_sink.clone();
    let collection = context.collection.clone();
    let registry = context.registry.clone();
    let authorizer = context.authorizer.clone();
    let mut evidence_health = context.evidence_health;
    let evidence_sink = context.evidence_sink.clone();
    let invocation_context =
        crate::authority::InvocationContext::from_session_id(context.session_id.as_deref());
    // One identity allocator per run mints opaque, non-reused run/turn/call
    // identities and a monotonic run-local sequence. Identities are minted
    // immediately before the corresponding lifecycle evidence is emitted, so
    // correlation precedes any external effect.
    let mut identities = crate::evidence::IdentityAllocator::new();
    let run = identities.run_id();
    let mut state = context.state;

    macro_rules! finish_with_error {
        ($error:expr) => {{
            let error = $error;
            if matches!(&error, AgentError::Cancelled) {
                let diagnostic_call = identities.next_call();
                emit_evidence(
                    &evidence_sink,
                    &mut identities,
                    &mut evidence_health,
                    run,
                    None,
                    diagnostic_call,
                    None,
                    crate::evidence::CallKind::Diagnostic,
                    crate::evidence::EvidencePayload::Diagnostic(
                        crate::evidence::RedactedDiagnostic {
                            severity: Severity::Info,
                            code: CODE_AGENT_CANCELLED,
                        },
                    ),
                );
            }
            return AgentLoopResult {
                state,
                identities,
                evidence_health,
                terminal_outcome: terminal_outcome_for_error(&error),
                error: Some(error),
            };
        }};
    }

    macro_rules! finish_success {
        () => {{
            return AgentLoopResult {
                state,
                identities,
                evidence_health,
                terminal_outcome: crate::evidence::TerminalOutcome::Success,
                error: None,
            };
        }};
    }

    emit_public_event(&events, AgentEvent::AgentStart);

    let mut has_tools_pending = false;
    for turn_idx in 0..config.max_turns {
        let turn = identities.next_turn();
        if cancel.is_cancelled() {
            observe(&diagnostic_sink, cancelled_diagnostic("before_turn"));
            emit_agent_end(&events, &state.context);
            finish_with_error!(AgentError::Cancelled);
        }

        emit_public_event(&events, AgentEvent::TurnStart);

        let transformed = match hooks
            .transform_context(state.context.clone(), cancel.clone())
            .await
        {
            Ok(transformed) => transformed,
            Err(error) => {
                emit_agent_end(&events, &state.context);
                finish_with_error!(error);
            }
        };

        let llm_messages = match hooks.convert_to_llm(&transformed) {
            Ok(messages) => messages,
            Err(error) => {
                emit_agent_end(&events, &state.context);
                finish_with_error!(error);
            }
        };

        let mut assistant_content: Vec<AssistantContent> = Vec::new();
        has_tools_pending = false;
        let mut retry_attempt: u32 = 0;
        let max_attempts = config.retry.as_ref().map(|r| r.max_attempts).unwrap_or(0);

        // Build the request ONCE per turn from the applied state's inference +
        // canonical model selection. Retries reuse the same prepared call, so
        // the request is not rebuilt inside the retry loop.
        let tool_defs = registry.definitions();
        let request = Request {
            model: state.model_selection.to_spec(),
            system: context.system.clone(),
            messages: llm_messages,
            tools: tool_defs,
            max_tokens: state.inference.max_tokens,
            temperature: state.inference.temperature,
            thinking: state.inference.thinking.clone(),
            stop_sequences: vec![],
            metadata: None,
            cancel: cancel.clone(),
            timeout: None,
            extra_headers: vec![],
            cache_retention: CacheRetention::None,
            session_id: context.session_id.clone(),
        };

        // Prepare one logical model call: route lookup, capability/wire
        // validation, and auth resolution all happen here, once, before any
        // model-request dispatch. A failure returns a typed error without
        // selecting another provider, model, wire, or credential policy.
        let prepared = match collection
            .prepare_call(&state.model_selection.to_spec(), request)
            .await
        {
            Ok(prepared) => prepared,
            Err(e) => {
                let err = AgentError::from(e);
                if matches!(&err, AgentError::Cancelled) {
                    observe(&diagnostic_sink, cancelled_diagnostic("during_prepare"));
                    emit_agent_end(&events, &state.context);
                    finish_with_error!(err);
                }
                let diagnostic: Diagnostic = (&err).into();
                let diagnostic_call = identities.next_call();
                emit_evidence(
                    &evidence_sink,
                    &mut identities,
                    &mut evidence_health,
                    run,
                    Some(turn),
                    diagnostic_call,
                    None,
                    crate::evidence::CallKind::Diagnostic,
                    crate::evidence::EvidencePayload::Diagnostic(
                        crate::evidence::RedactedDiagnostic {
                            severity: diagnostic.severity,
                            code: diagnostic.code,
                        },
                    ),
                );
                observe_provider_failure(&diagnostic_sink, diagnostic);
                emit_agent_end(&events, &state.context);
                finish_with_error!(err);
            }
        };

        // The prepared call resolves the one immutable provider route reused
        // across retries. Allocate its call identity and emit a Provider record
        // carrying the requested/resolved route facts. The actual route is
        // provider-reported and only known after
        // the response; the pre-dispatch record marks it unknown rather than
        // copying `resolved`, so a real divergence (if the provider ever reports
        // a different model/wire) is not silently normalized away.
        let resolved_route = prepared.route();
        let provider_call = identities.next_call();
        let mut provider_evidence = match crate::evidence::ProviderEvidenceFacts::from_prepared(
            &state.model_selection.to_spec(),
            resolved_route,
            prepared.auth_provenance(),
        ) {
            Ok(facts) => {
                emit_evidence(
                    &evidence_sink,
                    &mut identities,
                    &mut evidence_health,
                    run,
                    Some(turn),
                    provider_call,
                    None,
                    crate::evidence::CallKind::Provider,
                    crate::evidence::EvidencePayload::Provider(facts.clone()),
                );
                Some(facts)
            }
            Err(_) if evidence_sink.is_some() => {
                // A malformed fact must not be normalized into configured or
                // default provenance. Mark capture incomplete before dispatch;
                // complete-evidence policy will reject later side effects.
                evidence_health.advance_on_failure(crate::evidence::EvidenceFailureCode::Emission);
                None
            }
            Err(_) => None,
        };

        // Outcome of the turn's provider/tool work, collected inside the stream
        // loop and consumed by the post-stream finalize → prepare → stop path.
        let mut turn_tool_results: Vec<ToolResultMessage> = Vec::new();
        let mut turn_terminate = false;

        'stream: loop {
            let mut stream = match prepared.start_attempt() {
                Ok(stream) => stream,
                Err(CollectionError::CallCancelled) => {
                    observe(&diagnostic_sink, cancelled_diagnostic("during_prepare"));
                    emit_agent_end(&events, &state.context);
                    finish_with_error!(AgentError::Cancelled);
                }
                Err(e) => {
                    let err = AgentError::from(e);
                    let diagnostic = Diagnostic::from(&err);
                    observe_provider_failure(&diagnostic_sink, diagnostic);
                    emit_agent_end(&events, &state.context);
                    finish_with_error!(err);
                }
            };
            assistant_content.clear();
            // Track whether the provider has already delivered any stream item
            // for this attempt. Once content has been emitted to the caller, a
            // subsequent mid-stream retryable error must NOT trigger a retry:
            // re-invoking the provider would emit a second Start plus duplicated
            // content.
            let mut stream_delivered_content = false;

            while let Some(item) = {
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => {
                        observe(
                            &diagnostic_sink,
                            cancelled_diagnostic("during_stream"),
                        );
                        emit_agent_end(&events, &state.context);
                        finish_with_error!(AgentError::Cancelled);
                    }
                    item = stream.next() => item,
                }
            } {
                match item {
                    Ok(event) => {
                        // Any successfully delivered stream item means the
                        // caller has observed output for this attempt.
                        stream_delivered_content = true;
                        if let Some(msg) =
                            process_stream_event(&event, &mut assistant_content, &events)
                        {
                            // Use the provider's complete content, which can include thinking,
                            // instead of the local accumulator that tracks text and tool calls.
                            let agent_msg = AgentMessage::Llm(Message::Assistant(msg));

                            emit_public_event(
                                &events,
                                AgentEvent::MessageEnd {
                                    message: agent_msg.clone(),
                                },
                            );

                            state.context.push(agent_msg.clone());

                            if let (
                                Some(provider_evidence),
                                AgentMessage::Llm(Message::Assistant(assistant)),
                            ) = (provider_evidence.take(), &agent_msg)
                            {
                                let actual_model = assistant
                                    .response_model
                                    .as_deref()
                                    .unwrap_or(&assistant.model);
                                let actual =
                                    crate::evidence::ActualRoute::from_reported_provider_model(
                                        assistant.provider.clone(),
                                        actual_model.to_owned(),
                                        crate::evidence::UnknownReason::NotReported,
                                    );
                                match actual {
                                    Ok(actual) => {
                                        emit_evidence(
                                            &evidence_sink,
                                            &mut identities,
                                            &mut evidence_health,
                                            run,
                                            Some(turn),
                                            provider_call,
                                            None,
                                            crate::evidence::CallKind::Provider,
                                            crate::evidence::EvidencePayload::Provider(
                                                provider_evidence.with_actual(actual),
                                            ),
                                        );
                                    }
                                    Err(_) if evidence_sink.is_some() => evidence_health
                                        .advance_on_failure(
                                            crate::evidence::EvidenceFailureCode::Emission,
                                        ),
                                    Err(_) => {}
                                }
                            }

                            let content = match &agent_msg {
                                AgentMessage::Llm(Message::Assistant(a)) => &a.content,
                                _ => &Vec::new(),
                            };
                            let tool_calls: Vec<_> = content
                                .iter()
                                .filter_map(|c| match c {
                                    AssistantContent::ToolCall { tool_call } => {
                                        Some(tool_call.clone())
                                    }
                                    _ => None,
                                })
                                .collect();

                            if !tool_calls.is_empty() {
                                has_tools_pending = true;
                                // Pre-mint one Tool call identity per proposed tool call,
                                // before any executes, so correlation precedes the tool's
                                // external effect.
                                let tool_call_ids: Vec<crate::evidence::CallId> = (0..tool_calls
                                    .len())
                                    .map(|_| identities.next_call())
                                    .collect();
                                let mut tool_results = Vec::new();
                                let mut terminate_flags = Vec::new();
                                let mut tool_terminal_error = None;

                                let batch_is_sequential = tool_calls.iter().any(|tc| {
                                    registry
                                        .get(tc.name.as_str())
                                        .map(|r| {
                                            r.implementation.execution_mode()
                                                == ExecutionMode::Sequential
                                        })
                                        .unwrap_or(true)
                                });

                                if batch_is_sequential {
                                    for (tool_index, tc) in tool_calls.iter().enumerate() {
                                        let parsed = parse_tool_call_arguments(tc.clone());

                                        emit_public_event(
                                            &events,
                                            AgentEvent::ToolExecutionStart {
                                                tool_call_id: parsed.tool_call.id.clone(),
                                                tool_name: parsed.tool_call.name.clone(),
                                                args: parsed.args_for_event.clone(),
                                            },
                                        );

                                        let executed = match parsed.parsed_args {
                                            Ok(args) => {
                                                let mut tool_evidence = ToolEvidenceContext {
                                                    sink: &evidence_sink,
                                                    identities: &mut identities,
                                                    run,
                                                    turn,
                                                    call: tool_call_ids[tool_index],
                                                    parent: provider_call,
                                                    invocation: invocation_binding(
                                                        &invocation_context,
                                                    ),
                                                };
                                                match execute_tool(
                                                    &parsed.tool_call.id,
                                                    &parsed.tool_call.name,
                                                    &args,
                                                    run,
                                                    turn,
                                                    tool_call_ids[tool_index],
                                                    invocation_context.clone(),
                                                    &registry,
                                                    authorizer.as_ref(),
                                                    &mut evidence_health,
                                                    Some(&mut tool_evidence),
                                                    hooks,
                                                    &state.context,
                                                    cancel.clone(),
                                                    &diagnostic_sink,
                                                )
                                                .await
                                                {
                                                    Ok(result) => result,
                                                    Err(err) => {
                                                        if matches!(err, AgentError::Cancelled) {
                                                            observe(
                                                                &diagnostic_sink,
                                                                cancelled_diagnostic(
                                                                    "during_tool_preflight",
                                                                ),
                                                            );
                                                            emit_agent_end(&events, &state.context);
                                                        }
                                                        finish_with_error!(err);
                                                    }
                                                }
                                            }
                                            Err(parse_error) => ExecutedTool::ordinary(
                                                malformed_tool_arguments_result(
                                                    &parsed.tool_call.name,
                                                    &parse_error,
                                                    &diagnostic_sink,
                                                ),
                                            ),
                                        };

                                        let ExecutedTool {
                                            result,
                                            outcome,
                                            terminal_error,
                                        } = executed;
                                        let stops_sequential_batch = terminal_error.is_some();
                                        if let Some(error) = terminal_error {
                                            retain_strongest_terminal_error(
                                                &mut tool_terminal_error,
                                                error,
                                            );
                                        }

                                        let is_error = result.is_error;
                                        let truncated = result.truncated;
                                        terminate_flags.push(result.terminate);
                                        emit_public_event(
                                            &events,
                                            AgentEvent::ToolExecutionEnd {
                                                tool_call_id: parsed.tool_call.id.clone(),
                                                tool_name: parsed.tool_call.name.clone(),
                                                result: serde_json::json!(&result.content),
                                                details: result.details.clone(),
                                                truncated,
                                                is_error,
                                                diagnostics: result.diagnostics.clone(),
                                            },
                                        );

                                        let trm = ToolResultMessage {
                                            tool_call_id: parsed.tool_call.id,
                                            tool_name: parsed.tool_call.name.clone(),
                                            content: result.content,
                                            details: result.details,
                                            is_error,
                                            truncated,
                                            timestamp_ms: opi_ai::time::now_ms(),
                                        };
                                        let mut tool_evidence = ToolEvidenceContext {
                                            sink: &evidence_sink,
                                            identities: &mut identities,
                                            run,
                                            turn,
                                            call: tool_call_ids[tool_index],
                                            parent: provider_call,
                                            invocation: invocation_binding(&invocation_context),
                                        };
                                        emit_tool_outcome_evidence(
                                            &mut tool_evidence,
                                            &mut evidence_health,
                                            &registry,
                                            &trm,
                                            outcome,
                                        );
                                        tool_results.push(trm.clone());
                                        state
                                            .context
                                            .push(AgentMessage::Llm(Message::ToolResult(trm)));
                                        if stops_sequential_batch {
                                            // The remaining source-ordered calls never cross
                                            // preflight or execution after cancellation or an
                                            // uncertain external effect. Controlled results keep
                                            // the assistant/tool-result structure complete without
                                            // fabricating execution evidence for calls that did not
                                            // launch.
                                            for skipped in tool_calls.iter().skip(tool_index + 1) {
                                                let skipped =
                                                    skipped_after_terminal_result(skipped);
                                                tool_results.push(skipped.clone());
                                                state.context.push(AgentMessage::Llm(
                                                    Message::ToolResult(skipped),
                                                ));
                                            }
                                            break;
                                        }
                                    }
                                } else {
                                    let parsed_calls: Vec<_> = tool_calls
                                        .iter()
                                        .map(|tc| {
                                            let parsed = parse_tool_call_arguments(tc.clone());
                                            emit_public_event(
                                                &events,
                                                AgentEvent::ToolExecutionStart {
                                                    tool_call_id: parsed.tool_call.id.clone(),
                                                    tool_name: parsed.tool_call.name.clone(),
                                                    args: parsed.args_for_event.clone(),
                                                },
                                            );
                                            parsed
                                        })
                                        .collect();

                                    // Resolve, hook, schema-check, authorize, emit, and
                                    // freshness-check every call in deterministic source
                                    // order before any parallel launch. Call-local rejections
                                    // retain their results; evidence/freshness invalidation
                                    // aborts the entire not-yet-launched batch.
                                    let mut preflights = Vec::with_capacity(parsed_calls.len());
                                    for (tool_index, parsed) in parsed_calls.iter().enumerate() {
                                        let preflight = match &parsed.parsed_args {
                                            Ok(args) => {
                                                let mut tool_evidence = ToolEvidenceContext {
                                                    sink: &evidence_sink,
                                                    identities: &mut identities,
                                                    run,
                                                    turn,
                                                    call: tool_call_ids[tool_index],
                                                    parent: provider_call,
                                                    invocation: invocation_binding(
                                                        &invocation_context,
                                                    ),
                                                };
                                                match preflight_tool(
                                                    &parsed.tool_call.id,
                                                    &parsed.tool_call.name,
                                                    args,
                                                    run,
                                                    turn,
                                                    tool_call_ids[tool_index],
                                                    invocation_context.clone(),
                                                    &registry,
                                                    authorizer.as_ref(),
                                                    &mut evidence_health,
                                                    Some(&mut tool_evidence),
                                                    hooks,
                                                    &state.context,
                                                    cancel.clone(),
                                                    &diagnostic_sink,
                                                )
                                                .await
                                                {
                                                    Ok(preflight) => preflight,
                                                    Err(err) => {
                                                        if matches!(err, AgentError::Cancelled) {
                                                            observe(
                                                                &diagnostic_sink,
                                                                cancelled_diagnostic(
                                                                    "during_tool_preflight",
                                                                ),
                                                            );
                                                            emit_agent_end(&events, &state.context);
                                                        }
                                                        finish_with_error!(err);
                                                    }
                                                }
                                            }
                                            Err(parse_error) => ToolPreflight::Rejected(
                                                malformed_tool_arguments_result(
                                                    &parsed.tool_call.name,
                                                    parse_error,
                                                    &diagnostic_sink,
                                                ),
                                            ),
                                        };
                                        preflights.push(preflight);
                                    }

                                    let launch_generation = evidence_health.generation();
                                    let batch_is_invalid = preflights.iter().any(|preflight| {
                                        matches!(preflight, ToolPreflight::BatchInvalid(_))
                                            || matches!(
                                                preflight,
                                                ToolPreflight::Ready(prepared)
                                                    if prepared.evidence_health_generation
                                                        != launch_generation
                                            )
                                    });

                                    if cancel.is_cancelled() {
                                        observe(
                                            &diagnostic_sink,
                                            cancelled_diagnostic("before_parallel_tool_launch"),
                                        );
                                        emit_agent_end(&events, &state.context);
                                        finish_with_error!(AgentError::Cancelled);
                                    }

                                    let results = if !batch_is_invalid {
                                        let futures = preflights.into_iter().zip(&parsed_calls).map(
                                            |(preflight, parsed)| {
                                                let registry = registry.clone();
                                                let cancel = cancel.clone();
                                                let diagnostic_sink = diagnostic_sink.clone();
                                                async move {
                                                    match preflight {
                                                        ToolPreflight::Ready(prepared) => {
                                                            execute_prepared_tool(
                                                                &parsed.tool_call.id,
                                                                &parsed.tool_call.name,
                                                                prepared,
                                                                &registry,
                                                                hooks,
                                                                cancel,
                                                                &diagnostic_sink,
                                                            )
                                                            .await
                                                        }
                                                        ToolPreflight::Rejected(result) => {
                                                            ExecutedTool::ordinary(result)
                                                        }
                                                        ToolPreflight::BatchInvalid(_) => {
                                                            unreachable!(
                                                                "valid parallel batch contains no batch-invalid call"
                                                            )
                                                        }
                                                    }
                                                }
                                            },
                                        );
                                        futures_util::future::join_all(futures).await
                                    } else {
                                        preflights
                                            .into_iter()
                                            .map(|preflight| match preflight {
                                                ToolPreflight::Ready(_) => ExecutedTool::ordinary(
                                                    denial_result(
                                                        "parallel_preflight_aborted",
                                                        "parallel tool batch failed preflight; execution denied",
                                                    ),
                                                ),
                                                ToolPreflight::Rejected(result) => {
                                                    ExecutedTool::ordinary(result)
                                                }
                                                ToolPreflight::BatchInvalid(result) => {
                                                    ExecutedTool::ordinary(result)
                                                }
                                            })
                                            .collect()
                                    };

                                    for (tool_index, (parsed, executed)) in
                                        parsed_calls.iter().zip(results).enumerate()
                                    {
                                        let ExecutedTool {
                                            result,
                                            outcome,
                                            terminal_error,
                                        } = executed;
                                        if let Some(error) = terminal_error {
                                            retain_strongest_terminal_error(
                                                &mut tool_terminal_error,
                                                error,
                                            );
                                        }
                                        let is_error = result.is_error;
                                        let truncated = result.truncated;
                                        terminate_flags.push(result.terminate);
                                        emit_public_event(
                                            &events,
                                            AgentEvent::ToolExecutionEnd {
                                                tool_call_id: parsed.tool_call.id.clone(),
                                                tool_name: parsed.tool_call.name.clone(),
                                                result: serde_json::json!(&result.content),
                                                details: result.details.clone(),
                                                truncated,
                                                is_error,
                                                diagnostics: result.diagnostics.clone(),
                                            },
                                        );
                                        let trm = ToolResultMessage {
                                            tool_call_id: parsed.tool_call.id.clone(),
                                            tool_name: parsed.tool_call.name.clone(),
                                            content: result.content,
                                            details: result.details,
                                            is_error,
                                            truncated,
                                            timestamp_ms: opi_ai::time::now_ms(),
                                        };
                                        let mut tool_evidence = ToolEvidenceContext {
                                            sink: &evidence_sink,
                                            identities: &mut identities,
                                            run,
                                            turn,
                                            call: tool_call_ids[tool_index],
                                            parent: provider_call,
                                            invocation: invocation_binding(&invocation_context),
                                        };
                                        emit_tool_outcome_evidence(
                                            &mut tool_evidence,
                                            &mut evidence_health,
                                            &registry,
                                            &trm,
                                            outcome,
                                        );
                                        tool_results.push(trm.clone());
                                        state
                                            .context
                                            .push(AgentMessage::Llm(Message::ToolResult(trm)));
                                    }
                                }

                                let all_terminate = !terminate_flags.is_empty()
                                    && terminate_flags.iter().all(|t| *t);

                                emit_public_event(
                                    &events,
                                    AgentEvent::TurnEnd {
                                        message: agent_msg.clone(),
                                        tool_results: tool_results.clone(),
                                    },
                                );

                                if let Some(error) = tool_terminal_error {
                                    emit_agent_end(&events, &state.context);
                                    finish_with_error!(error);
                                }

                                turn_tool_results = tool_results;
                                turn_terminate = all_terminate;
                                break 'stream;
                            }

                            emit_public_event(
                                &events,
                                AgentEvent::TurnEnd {
                                    message: agent_msg.clone(),
                                    tool_results: vec![],
                                },
                            );

                            break 'stream;
                        }
                    }
                    Err(e) => {
                        if e.is_retryable()
                            && retry_attempt < max_attempts
                            && !stream_delivered_content
                            && let Some(ref rc) = config.retry
                        {
                            let retry_after_ms = match &e {
                                ProviderError::RateLimited { retry_after_ms } => *retry_after_ms,
                                _ => None,
                            };
                            let delay_ms = rc.delay_for_attempt(retry_attempt, retry_after_ms);
                            retry_attempt += 1;

                            emit_public_event(
                                &events,
                                AgentEvent::AutoRetryStart {
                                    attempt: retry_attempt,
                                    max_attempts: rc.max_attempts,
                                    delay_ms,
                                    error_message: e.to_string(),
                                },
                            );
                            observe(
                                &diagnostic_sink,
                                Diagnostic::new(
                                    Severity::Warning,
                                    CODE_PROVIDER_RETRY_ATTEMPT,
                                    SOURCE_PROVIDER,
                                    "retrying after retryable provider error",
                                )
                                .details(json!({
                                    "attempt": retry_attempt,
                                    "max_attempts": rc.max_attempts,
                                    "delay_ms": delay_ms,
                                })),
                            );

                            tokio::select! {
                                biased;
                                _ = cancel.cancelled() => {
                                    observe(
                                        &diagnostic_sink,
                                        cancelled_diagnostic("during_retry_sleep"),
                                    );
                                    emit_agent_end(&events, &state.context);
                                    finish_with_error!(AgentError::Cancelled);
                                }
                                _ = tokio::time::sleep(
                                    std::time::Duration::from_millis(delay_ms)
                                ) => {}
                            }
                            // Re-open the next attempt from the SAME prepared
                            // call: route and auth are not re-resolved. Emit a
                            // Retry record parented to the provider call: the
                            // attempt re-opens the one immutable route (no
                            // re-resolution), so it is a child of the provider
                            // call.
                            let retry_call = identities.next_call();
                            emit_evidence(
                                &evidence_sink,
                                &mut identities,
                                &mut evidence_health,
                                run,
                                Some(turn),
                                retry_call,
                                Some(provider_call),
                                crate::evidence::CallKind::Retry,
                                crate::evidence::EvidencePayload::Structured(
                                    crate::evidence::RedactedValue::redacted(
                                        json!({ "attempt": retry_attempt }),
                                        crate::diagnostic::RedactionMode::Summary,
                                    ),
                                ),
                            );
                            continue 'stream;
                        }

                        if retry_attempt > 0 {
                            let retry_suppressed_after_partial_output =
                                e.is_retryable() && stream_delivered_content;

                            emit_public_event(
                                &events,
                                AgentEvent::AutoRetryEnd {
                                    success: false,
                                    attempt: retry_attempt,
                                    final_error: Some(e.to_string()),
                                },
                            );
                            if retry_suppressed_after_partial_output {
                                observe(
                                    &diagnostic_sink,
                                    Diagnostic::new(
                                        Severity::Warning,
                                        CODE_PROVIDER_RETRY_SUPPRESSED_AFTER_PARTIAL_OUTPUT,
                                        SOURCE_PROVIDER,
                                        "retry suppressed after partial provider output",
                                    )
                                    .details(json!({
                                        "attempts": retry_attempt,
                                        "max_attempts": max_attempts,
                                    })),
                                );
                            } else {
                                observe(
                                    &diagnostic_sink,
                                    Diagnostic::new(
                                        Severity::Error,
                                        CODE_PROVIDER_RETRY_EXHAUSTED,
                                        SOURCE_PROVIDER,
                                        "provider retries exhausted",
                                    )
                                    .details(json!({
                                        "attempts": retry_attempt,
                                        "max_attempts": max_attempts,
                                    }))
                                    .action("reduce request frequency or check model availability"),
                                );
                            }
                        }

                        // The underlying provider error is classified regardless of whether
                        // retries were attempted, so callers see what actually failed.
                        let err = AgentError::from(e);
                        let diagnostic = Diagnostic::from(&err);
                        observe_provider_failure(&diagnostic_sink, diagnostic);
                        emit_agent_end(&events, &state.context);
                        finish_with_error!(err);
                    }
                }
            }

            let err = AgentError::ProviderProtocol {
                detail: opi_ai::provider::ProviderErrorSummary::redacted(),
            };
            let diagnostic = Diagnostic::from(&err);
            observe_provider_failure(&diagnostic_sink, diagnostic);
            emit_agent_end(&events, &state.context);
            finish_with_error!(err);
        }

        // A retried turn that reached a terminal outcome emits the retry-success
        // event here (runs for every 'stream exit path, including the Done-event
        // break that skips the post-while-loop emission above).
        if retry_attempt > 0 {
            emit_public_event(
                &events,
                AgentEvent::AutoRetryEnd {
                    success: true,
                    attempt: retry_attempt,
                    final_error: None,
                },
            );
            observe(
                &diagnostic_sink,
                Diagnostic::new(
                    Severity::Info,
                    CODE_PROVIDER_RETRY_SUCCEEDED,
                    SOURCE_PROVIDER,
                    "provider request succeeded after retry",
                )
                .details(json!({ "attempts": retry_attempt })),
            );
        }

        // Construct the candidate next-turn state away from live state, validate
        // it as a unit, and atomically apply it. None retains the state; an
        // error or cancellation leaves the prior state intact.
        let prep_ctx = PrepareNextTurnContext {
            state: state.clone(),
            tool_results: turn_tool_results.clone(),
            terminate: turn_terminate,
            turn: turn_idx + 1,
        };
        let mut did_prepare = false;
        let mut prior_state = None;
        let prepared_next_turn = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                observe(&diagnostic_sink, cancelled_diagnostic("during_prepare_next_turn"));
                emit_agent_end(&events, &state.context);
                finish_with_error!(AgentError::Cancelled);
            }
            prepared = hooks.prepare_next_turn(prep_ctx) => prepared,
        };
        match prepared_next_turn {
            Ok(Some(candidate)) => {
                if let Err(err) = validate_next_turn_candidate(&collection, &candidate) {
                    emit_agent_end(&events, &state.context);
                    finish_with_error!(err);
                }
                if cancel.is_cancelled() {
                    observe(
                        &diagnostic_sink,
                        cancelled_diagnostic("after_prepare_next_turn_validation"),
                    );
                    emit_agent_end(&events, &state.context);
                    finish_with_error!(AgentError::Cancelled);
                }
                prior_state = Some(std::mem::replace(&mut state, candidate));
                did_prepare = true;
            }
            Ok(None) => {}
            Err(e) => {
                emit_agent_end(&events, &state.context);
                finish_with_error!(e);
            }
        }
        // should_stop_after_turn observes the APPLIED state, after preparation.
        // A tool-driven terminate flag forces the stop.
        let stop_ctx = ShouldStopAfterTurnContext {
            state: state.clone(),
            tool_results: turn_tool_results,
        };
        let should_stop = if turn_terminate {
            true
        } else {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => {
                    if let Some(prior) = prior_state.take() {
                        state = prior;
                    }
                    observe(
                        &diagnostic_sink,
                        cancelled_diagnostic("during_should_stop_after_turn"),
                    );
                    emit_agent_end(&events, &state.context);
                    finish_with_error!(AgentError::Cancelled);
                }
                stop = hooks.should_stop_after_turn(stop_ctx) => stop,
            }
        };
        if cancel.is_cancelled() {
            if let Some(prior) = prior_state.take() {
                state = prior;
            }
            observe(
                &diagnostic_sink,
                cancelled_diagnostic("after_should_stop_after_turn"),
            );
            emit_agent_end(&events, &state.context);
            finish_with_error!(AgentError::Cancelled);
        }
        if should_stop {
            emit_agent_end(&events, &state.context);
            finish_success!();
        }

        // The cancellation-sensitive stop boundary completed. The applied
        // candidate is now committed before queue polling begins.
        drop(prior_state.take());

        // Queue input is applied only after the stop decision permits polling;
        // it cannot resurrect a transition that already failed or was cancelled.
        let steering = drain_queue(&context.steering_queue);
        if !steering.is_empty() {
            emit_public_event(
                &events,
                AgentEvent::QueueUpdate {
                    steering: steering.clone(),
                    follow_up: vec![],
                },
            );
            for msg in steering {
                state.context.push(user_text_message(msg));
            }
            continue;
        }

        // prepare_next_turn applied a fresh candidate (new context for the next
        // provider call). It gets its own next turn, polled before follow-up, so
        // a prepared message is not bundled with a queued follow-up.
        if did_prepare {
            continue;
        }

        if !has_tools_pending {
            let follow_up = pop_follow_up(&context.follow_up_queue);
            if !follow_up.is_empty() {
                emit_public_event(
                    &events,
                    AgentEvent::QueueUpdate {
                        steering: vec![],
                        follow_up: follow_up.clone(),
                    },
                );
                for msg in follow_up {
                    state.context.push(user_text_message(msg));
                }
                continue;
            }
            break;
        }
    }

    // The for-loop only falls through when `turn_idx` reached
    // `config.max_turns`. If tools were still pending on the final turn the run
    // hit the turn cap mid-work — emit a max-turns warning diagnostic
    // (BEFORE AgentEnd, mirroring the cancellation ordering at the top of the
    // loop) and return `AgentError::MaxTurnsExceeded` instead of a silent Ok. A
    // zero-turn run, or exhaustion without pending tools (e.g. steering kept the
    // loop alive), completes normally.
    if has_tools_pending {
        let err = AgentError::MaxTurnsExceeded(config.max_turns);
        observe(&diagnostic_sink, Diagnostic::from(&err));
        emit_agent_end(&events, &state.context);
        finish_with_error!(err);
    }
    emit_agent_end(&events, &state.context);
    AgentLoopResult {
        state,
        identities,
        evidence_health,
        terminal_outcome: crate::evidence::TerminalOutcome::Success,
        error: None,
    }
}

/// Observe a provider failure using its preclassified diagnostic.
fn observe_provider_failure(sink: &Option<Arc<dyn DiagnosticSink>>, diagnostic: Diagnostic) {
    observe(sink, diagnostic);
}

fn process_stream_event(
    event: &opi_ai::stream::AssistantStreamEvent,
    content: &mut Vec<AssistantContent>,
    events: &AgentEventSink,
) -> Option<opi_ai::message::AssistantMessage> {
    use opi_ai::stream::AssistantStreamEvent::*;

    match event {
        Start { partial } => {
            let msg = AgentMessage::Llm(Message::Assistant(partial.clone()));
            emit_public_event(events, AgentEvent::MessageStart { message: msg });
            None
        }
        TextDelta { delta, partial, .. } => {
            match content.last_mut() {
                Some(AssistantContent::Text { text }) => {
                    text.push_str(delta);
                }
                _ => {
                    content.push(AssistantContent::Text {
                        text: delta.clone(),
                    });
                }
            }
            let msg = AgentMessage::Llm(Message::Assistant(partial.clone()));
            emit_public_event(
                events,
                AgentEvent::MessageUpdate {
                    message: msg,
                    assistant_event: Box::new(event.clone()),
                },
            );
            None
        }
        ToolCallEnd { tool_call, .. } => {
            content.push(AssistantContent::ToolCall {
                tool_call: tool_call.clone(),
            });
            None
        }
        ThinkingStart { partial, .. }
        | ThinkingDelta { partial, .. }
        | ThinkingEnd { partial, .. } => {
            let msg = AgentMessage::Llm(Message::Assistant(partial.clone()));
            emit_public_event(
                events,
                AgentEvent::MessageUpdate {
                    message: msg,
                    assistant_event: Box::new(event.clone()),
                },
            );
            None
        }
        Done { message, .. } => Some(message.clone()),
        Error { message, .. } => Some(message.clone()),
        _ => None,
    }
}

#[derive(Clone)]
struct ParsedToolCall {
    tool_call: ToolCall,
    args_for_event: serde_json::Value,
    parsed_args: Result<serde_json::Value, String>,
}

fn parse_tool_call_arguments(tool_call: ToolCall) -> ParsedToolCall {
    match serde_json::from_str::<serde_json::Value>(&tool_call.arguments) {
        Ok(args) => ParsedToolCall {
            tool_call,
            args_for_event: args.clone(),
            parsed_args: Ok(args),
        },
        Err(err) => ParsedToolCall {
            tool_call,
            args_for_event: serde_json::Value::Null,
            parsed_args: Err(err.to_string()),
        },
    }
}

fn malformed_tool_arguments_result(
    tool_name: &str,
    parse_error: &str,
    sink: &Option<Arc<dyn DiagnosticSink>>,
) -> ToolResult {
    observe(
        sink,
        tool_diagnostic(
            CODE_TOOL_VALIDATION_FAILED,
            tool_name,
            "tool arguments were not valid JSON",
        ),
    );
    ToolResult {
        content: vec![opi_ai::message::OutputContent::Text {
            text: format!("tool arguments were not valid JSON: {parse_error}"),
        }],
        details: None,
        is_error: true,
        terminate: false,
        truncated: false,
        diagnostics: vec![],
    }
}

fn skipped_after_terminal_result(tool_call: &ToolCall) -> ToolResultMessage {
    ToolResultMessage {
        tool_call_id: tool_call.id.clone(),
        tool_name: tool_call.name.clone(),
        content: vec![opi_ai::message::OutputContent::Text {
            text: "tool was not executed because a prior sequential call ended the run".to_owned(),
        }],
        details: None,
        is_error: true,
        truncated: false,
        timestamp_ms: opi_ai::time::now_ms(),
    }
}

/// Outcome of mandatory trusted authorization for one resolved, validated call.
#[derive(Clone)]
enum Authorized {
    /// A fresh `Allow` whose registration, capability, and evidence-health
    /// generation all match the current call.
    AllowFresh {
        authorization: crate::tool::ToolExecutionAuthorization,
        evidence_health_generation: crate::evidence::EvidenceGeneration,
    },
    /// Zero-execution denial (authorizer denial, stale generation, or authorizer
    /// error/unavailability). `stable_code` and `redacted_reason` are secret-free.
    Deny {
        stable_code: String,
        redacted_reason: String,
        scope: AuthorizationRejectionScope,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AuthorizationRejectionScope {
    Call,
    Batch,
}

struct ToolEvidenceContext<'a> {
    sink: &'a Option<Arc<dyn crate::evidence::EvidenceSink>>,
    identities: &'a mut crate::evidence::IdentityAllocator,
    run: crate::evidence::RunId,
    turn: crate::evidence::TurnId,
    call: crate::evidence::CallId,
    parent: crate::evidence::CallId,
    invocation: Result<crate::evidence::InvocationBinding, crate::evidence::OpaqueIdentityError>,
}

fn emit_tool_authorization_evidence(
    evidence: &mut ToolEvidenceContext<'_>,
    health: &mut crate::evidence::EvidenceHealth,
    registration: &crate::authority::RegisteredTool,
    decision: &Authorized,
) -> bool {
    let facts = evidence
        .invocation
        .as_ref()
        .map_err(|error| crate::evidence::ToolEvidenceFactError::from(*error))
        .and_then(|invocation| {
            crate::evidence::ToolEvidenceFacts::authorization(
                registration.provider_visible_name.clone(),
                registration.registration_id.as_str().to_owned(),
                registration.capability.clone(),
                invocation.clone(),
                authorization_evidence_facts(decision)?,
            )
        });
    emit_tool_facts(evidence, health, facts)
}

fn emit_tool_outcome_evidence(
    evidence: &mut ToolEvidenceContext<'_>,
    health: &mut crate::evidence::EvidenceHealth,
    registry: &crate::authority::ToolRegistry,
    result: &ToolResultMessage,
    outcome: crate::evidence::ToolExecutionOutcome,
) {
    let registration = registry.get(result.tool_name.as_str());
    let facts = evidence
        .invocation
        .as_ref()
        .map_err(|error| crate::evidence::ToolEvidenceFactError::from(*error))
        .and_then(|invocation| {
            crate::evidence::ToolEvidenceFacts::outcome(
                result.tool_name.clone(),
                registration.map(|item| item.registration_id.as_str()),
                registration.map(|item| item.capability.clone()),
                invocation.clone(),
                outcome,
            )
        });
    emit_tool_facts(evidence, health, facts);
}

fn emit_tool_facts(
    evidence: &mut ToolEvidenceContext<'_>,
    health: &mut crate::evidence::EvidenceHealth,
    facts: Result<crate::evidence::ToolEvidenceFacts, crate::evidence::ToolEvidenceFactError>,
) -> bool {
    match facts {
        Ok(facts) => emit_evidence(
            evidence.sink,
            evidence.identities,
            health,
            evidence.run,
            Some(evidence.turn),
            evidence.call,
            Some(evidence.parent),
            crate::evidence::CallKind::Tool,
            crate::evidence::EvidencePayload::Tool(facts),
        ),
        Err(_) if evidence.sink.is_some() => {
            health.advance_on_failure(crate::evidence::EvidenceFailureCode::Emission);
            false
        }
        Err(_) => true,
    }
}

fn invocation_binding(
    context: &crate::authority::InvocationContext,
) -> Result<crate::evidence::InvocationBinding, crate::evidence::OpaqueIdentityError> {
    match context {
        crate::authority::InvocationContext::NoSession => {
            Ok(crate::evidence::InvocationBinding::NoSession)
        }
        crate::authority::InvocationContext::Session(reference) => {
            crate::evidence::InvocationBinding::session(reference.clone())
        }
    }
}

fn authorization_evidence_facts(
    decision: &Authorized,
) -> Result<crate::evidence::ToolAuthorizationFacts, crate::evidence::OpaqueIdentityError> {
    match decision {
        Authorized::AllowFresh { authorization, .. } => {
            Ok(crate::evidence::ToolAuthorizationFacts::Allowed {
                policy_ref: authorization.policy_ref.clone(),
                permission_ref: authorization.permission_ref.clone(),
                permission_scope: authorization.permission_scope.clone(),
                scoped_grant_ref: authorization.scoped_grant_ref.clone(),
            })
        }
        Authorized::Deny {
            stable_code,
            redacted_reason,
            ..
        } => crate::evidence::ToolAuthorizationFacts::denied(stable_code.clone(), redacted_reason),
    }
}

/// Resolve a current trusted `Allow` for one call: authorize against the
/// current evidence-health snapshot, verify the returned `Allow` still matches
/// the registration/capability/generation, and reauthorize once if it is stale.
/// An authorizer error, denial, or a persistently stale generation all yield
/// [`Authorized::Deny`] with zero execution (AUT-003/005).
///
/// `evidence_health` is the loop's current run-local snapshot. A stale decision
/// is requested once more against that snapshot; the caller separately emits
/// authorization evidence and verifies the live generation before launch.
#[allow(clippy::too_many_arguments)] // one immutable authorization request assembled at this boundary
async fn authorize_and_verify(
    authorizer: &dyn crate::authority::ToolAuthorizer,
    registration: &crate::authority::RegisteredTool,
    args: &serde_json::Value,
    evidence_health: crate::evidence::EvidenceHealth,
    run_id: crate::evidence::RunId,
    turn_id: crate::evidence::TurnId,
    call_id: crate::evidence::CallId,
    invocation_context: crate::authority::InvocationContext,
    cancel: CancellationToken,
) -> Result<Authorized, AgentError> {
    for attempt in 0..2u8 {
        if cancel.is_cancelled() {
            return Err(AgentError::Cancelled);
        }
        let request = crate::authority::ToolAuthorizationRequest {
            run_id,
            turn_id,
            call_id,
            invocation_context: invocation_context.clone(),
            registration_id: registration.registration_id.clone(),
            capability: registration.capability.clone(),
            arguments: args.clone(),
            evidence_health: evidence_health.clone(),
        };
        let pending_decision = authorizer.authorize(request, cancel.clone());
        let decision = match tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(AgentError::Cancelled),
            decision = pending_decision => decision,
        } {
            Ok(decision) => decision,
            Err(_) => {
                return Ok(Authorized::Deny {
                    stable_code: "authorization_unavailable".to_owned(),
                    redacted_reason: "authorizer failed to reach a decision; execution denied"
                        .to_owned(),
                    scope: AuthorizationRejectionScope::Call,
                });
            }
        };
        match decision {
            crate::authority::AuthorizationDecision::Deny {
                stable_code,
                redacted_reason,
            } => {
                return Ok(Authorized::Deny {
                    stable_code,
                    redacted_reason,
                    scope: AuthorizationRejectionScope::Call,
                });
            }
            crate::authority::AuthorizationDecision::Allow {
                policy_ref,
                permission_ref,
                permission_scope,
                scoped_grant_ref,
                registration_id,
                capability,
                evidence_health_generation,
            } => {
                if registration_id == registration.registration_id
                    && capability == registration.capability
                    && evidence_health_generation == evidence_health.generation()
                {
                    return Ok(Authorized::AllowFresh {
                        authorization: crate::tool::ToolExecutionAuthorization {
                            policy_ref,
                            permission_ref,
                            permission_scope,
                            scoped_grant_ref,
                        },
                        evidence_health_generation,
                    });
                }
                // Stale: the Allow does not match the current registration,
                // capability, or evidence-health snapshot. Reauthorize once;
                // a still-stale second decision denies with zero execution.
                if attempt > 0 {
                    let generation_mismatch =
                        evidence_health_generation != evidence_health.generation();
                    let identity_mismatch = registration_id != registration.registration_id
                        || capability != registration.capability;
                    return Ok(if generation_mismatch {
                        Authorized::Deny {
                            stable_code: "authorization_stale".to_owned(),
                            redacted_reason: "authorization Allow was stale; execution denied"
                                .to_owned(),
                            scope: AuthorizationRejectionScope::Batch,
                        }
                    } else if identity_mismatch {
                        Authorized::Deny {
                            stable_code: "authorization_mismatch".to_owned(),
                            redacted_reason:
                                "authorization Allow did not match the resolved tool; execution denied"
                                    .to_owned(),
                            scope: AuthorizationRejectionScope::Call,
                        }
                    } else {
                        unreachable!("a matching Allow returns before stale classification")
                    });
                }
            }
        }
    }
    unreachable!("authorize_and_verify loops at most twice before returning")
}

struct PreparedToolExecution {
    args: serde_json::Value,
    authorization: crate::tool::ToolExecutionAuthorization,
    evidence_health_generation: crate::evidence::EvidenceGeneration,
}

struct ExecutedTool {
    result: ToolResult,
    outcome: crate::evidence::ToolExecutionOutcome,
    terminal_error: Option<AgentError>,
}

impl ExecutedTool {
    fn ordinary(result: ToolResult) -> Self {
        let outcome = if result.is_error {
            crate::evidence::ToolExecutionOutcome::Failed
        } else {
            crate::evidence::ToolExecutionOutcome::Succeeded
        };
        Self {
            result,
            outcome,
            terminal_error: None,
        }
    }
}

fn retain_strongest_terminal_error(slot: &mut Option<AgentError>, candidate: AgentError) {
    fn rank(error: &AgentError) -> u8 {
        match error {
            AgentError::Tool(crate::tool::ToolError::CleanupUnknown(_)) => 3,
            AgentError::Tool(crate::tool::ToolError::PartialSideEffect(_)) => 2,
            AgentError::Cancelled | AgentError::Tool(crate::tool::ToolError::Cancelled) => 1,
            _ => 0,
        }
    }

    if slot
        .as_ref()
        .is_none_or(|current| rank(&candidate) > rank(current))
    {
        *slot = Some(candidate);
    }
}

enum ToolPreflight {
    Ready(PreparedToolExecution),
    Rejected(ToolResult),
    BatchInvalid(ToolResult),
}

#[allow(clippy::too_many_arguments)] // private helper threading sinks + turn id alongside existing call context
async fn preflight_tool(
    call_id: &str,
    tool_name: &str,
    args: &serde_json::Value,
    run_id: crate::evidence::RunId,
    evidence_turn_id: crate::evidence::TurnId,
    evidence_call_id: crate::evidence::CallId,
    invocation_context: crate::authority::InvocationContext,
    registry: &crate::authority::ToolRegistry,
    authorizer: Option<&Arc<dyn crate::authority::ToolAuthorizer>>,
    evidence_health: &mut crate::evidence::EvidenceHealth,
    mut tool_evidence: Option<&mut ToolEvidenceContext<'_>>,
    hooks: &dyn AgentHooks,
    messages: &[AgentMessage],
    cancel: CancellationToken,
    sink: &Option<Arc<dyn DiagnosticSink>>,
) -> Result<ToolPreflight, AgentError> {
    if cancel.is_cancelled() {
        return Err(AgentError::Cancelled);
    }
    // 1. Resolve one immutable trusted registration (AUT-001). Unknown tools
    //    never reach hook, validation, authorization, or execution.
    let registration = match registry.get(tool_name) {
        Some(r) => r,
        None => {
            observe(
                sink,
                tool_diagnostic(CODE_TOOL_UNKNOWN, tool_name, "unknown tool requested"),
            );
            return Ok(ToolPreflight::Rejected(error_text_result(format!(
                "unknown tool: {tool_name}"
            ))));
        }
    };

    // 2. Non-authoritative before_tool_call hook: deny or continue (AUT-006).
    //    Runs BEFORE schema validation so the hook
    //    observes the proposed call regardless of argument validity; a denial
    //    still yields zero execution.
    let hook_ctx = BeforeToolCallContext {
        tool_call_id: call_id.to_owned(),
        tool_name: tool_name.to_owned(),
        args: args.clone(),
        messages: messages.to_vec(),
    };
    let hook_result = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(AgentError::Cancelled),
        result = hooks.before_tool_call(hook_ctx) => result,
    };
    match hook_result {
        BeforeToolCallResult::Continue => {}
        BeforeToolCallResult::Deny { reason } => {
            observe(
                sink,
                tool_diagnostic(
                    CODE_TOOL_EXECUTION_FAILED,
                    tool_name,
                    "tool call denied by hook",
                ),
            );
            return Ok(ToolPreflight::Rejected(error_text_result(reason)));
        }
    }

    // 3. Validate the FINAL arguments against the registered schema (AUT-002).
    //    The exact validated value is the value authorized and executed.
    let schema = &registration.definition.input_schema;
    if let Err(err) = validation::validate(schema, args) {
        observe(
            sink,
            tool_diagnostic(
                CODE_TOOL_VALIDATION_FAILED,
                tool_name,
                "tool arguments failed schema validation",
            ),
        );
        return Ok(ToolPreflight::Rejected(ToolResult::from_validation_error(
            err,
        )));
    }

    // 4-5. Mandatory trusted authorization against the current evidence-health
    //      snapshot (AUT-003/005). Missing authorizer, authorizer error, denial,
    //      or a stale generation all yield zero executions and a redacted,
    //      controlled outcome whose owning stable code is visible (A08).
    let authorizer = match authorizer {
        Some(a) => a,
        None => {
            observe(
                sink,
                tool_diagnostic(
                    CODE_TOOL_AUTHORIZATION_UNAVAILABLE,
                    tool_name,
                    "no trusted authorizer bound; execution denied",
                ),
            );
            let decision = Authorized::Deny {
                stable_code: "authorization_unavailable".to_owned(),
                redacted_reason: "no trusted authorizer bound; execution denied".to_owned(),
                scope: AuthorizationRejectionScope::Call,
            };
            let authorization_recorded = if let Some(evidence) = &mut tool_evidence {
                emit_tool_authorization_evidence(evidence, evidence_health, registration, &decision)
            } else {
                true
            };
            let result = denial_result(
                "authorization_unavailable",
                "no trusted authorizer bound; execution denied",
            );
            return Ok(if authorization_recorded {
                ToolPreflight::Rejected(result)
            } else {
                ToolPreflight::BatchInvalid(result)
            });
        }
    };
    let decision = authorize_and_verify(
        authorizer.as_ref(),
        registration,
        args,
        evidence_health.clone(),
        run_id,
        evidence_turn_id,
        evidence_call_id,
        invocation_context.clone(),
        cancel.clone(),
    )
    .await?;
    if cancel.is_cancelled() {
        return Err(AgentError::Cancelled);
    }
    let authorization_recorded = match &mut tool_evidence {
        Some(evidence) => {
            emit_tool_authorization_evidence(evidence, evidence_health, registration, &decision)
        }
        None => true,
    };

    if cancel.is_cancelled() {
        return Err(AgentError::Cancelled);
    }
    if !authorization_recorded {
        return Ok(ToolPreflight::BatchInvalid(denial_result(
            "evidence_incomplete",
            "authorization evidence was incomplete; execution denied",
        )));
    }

    Ok(match &decision {
        Authorized::AllowFresh {
            authorization,
            evidence_health_generation,
        } if *evidence_health_generation == evidence_health.generation() => {
            ToolPreflight::Ready(PreparedToolExecution {
                args: args.clone(),
                authorization: authorization.clone(),
                evidence_health_generation: *evidence_health_generation,
            })
        }
        Authorized::AllowFresh { .. } => {
            observe(
                sink,
                tool_diagnostic(
                    CODE_TOOL_AUTHORIZATION_DENIED,
                    tool_name,
                    "authorization evidence changed before launch",
                ),
            );
            ToolPreflight::BatchInvalid(denial_result(
                "evidence_incomplete",
                "authorization evidence was incomplete; execution denied",
            ))
        }
        Authorized::Deny {
            stable_code,
            redacted_reason,
            scope,
        } => {
            observe(
                sink,
                tool_diagnostic(CODE_TOOL_AUTHORIZATION_DENIED, tool_name, redacted_reason),
            );
            let result = denial_result(stable_code, redacted_reason);
            if *scope == AuthorizationRejectionScope::Batch {
                ToolPreflight::BatchInvalid(result)
            } else {
                ToolPreflight::Rejected(result)
            }
        }
    })
}

#[allow(clippy::too_many_arguments)] // private launch boundary over one fully prepared call
async fn execute_prepared_tool(
    call_id: &str,
    tool_name: &str,
    prepared: PreparedToolExecution,
    registry: &crate::authority::ToolRegistry,
    hooks: &dyn AgentHooks,
    cancel: CancellationToken,
    sink: &Option<Arc<dyn DiagnosticSink>>,
) -> ExecutedTool {
    let registration = registry
        .get(tool_name)
        .expect("a prepared tool retains its immutable registration");

    // 6. Execute: only a current Allow reaches Tool::execute (OUT-003).
    match registration
        .implementation
        .execute_authorized(
            call_id,
            prepared.args,
            prepared.authorization,
            cancel.clone(),
            None,
        )
        .await
    {
        Ok(result) => {
            let ctx = AfterToolCallContext {
                tool_call_id: call_id.to_owned(),
                tool_name: tool_name.to_owned(),
                result: result.clone(),
            };
            let final_result = match hooks.after_tool_call(ctx).await {
                AfterToolCallResult::Keep => result,
                AfterToolCallResult::Replace(replacement) => replacement,
            };
            if final_result.is_error {
                // Lift tool-owned structured failure context from
                // `final_result.diagnostics` into Diagnostics (per-cause code +
                // structured context). The lift reads diagnostics AFTER
                // `after_tool_call`, so a hook `Replace` owns the lifted set. When
                // the tool reported a bare is_error result with no diagnostics,
                // fall back to the generic execution-failed diagnostic so a
                // failure is always observed.
                if final_result.diagnostics.is_empty() {
                    observe(
                        sink,
                        tool_diagnostic(
                            CODE_TOOL_EXECUTION_FAILED,
                            tool_name,
                            "tool returned an error result",
                        ),
                    );
                } else {
                    for tool_diag in &final_result.diagnostics {
                        observe(sink, tool_owned_diagnostic(tool_diag, tool_name));
                    }
                }
            }
            ExecutedTool::ordinary(final_result)
        }
        Err(e) => {
            observe(
                sink,
                tool_diagnostic(
                    CODE_TOOL_EXECUTION_FAILED,
                    tool_name,
                    "tool execution failed",
                ),
            );
            let public_message = e.to_string();
            let outcome = match &e {
                crate::tool::ToolError::ExecutionFailed(_) => {
                    crate::evidence::ToolExecutionOutcome::Failed
                }
                crate::tool::ToolError::Cancelled => {
                    crate::evidence::ToolExecutionOutcome::Cancelled
                }
                crate::tool::ToolError::PartialSideEffect(_) => {
                    crate::evidence::ToolExecutionOutcome::PartialSideEffect
                }
                crate::tool::ToolError::CleanupUnknown(_) => {
                    crate::evidence::ToolExecutionOutcome::CleanupUnknown
                }
            };
            let terminal_error = match e {
                crate::tool::ToolError::ExecutionFailed(_) => None,
                crate::tool::ToolError::Cancelled => Some(AgentError::Cancelled),
                error @ crate::tool::ToolError::PartialSideEffect(_)
                | error @ crate::tool::ToolError::CleanupUnknown(_) => {
                    Some(AgentError::Tool(error))
                }
            };
            ExecutedTool {
                result: error_text_result(match outcome {
                    crate::evidence::ToolExecutionOutcome::Cancelled => {
                        "tool execution cancelled".to_owned()
                    }
                    crate::evidence::ToolExecutionOutcome::PartialSideEffect => {
                        "tool execution ended with a partial side effect".to_owned()
                    }
                    crate::evidence::ToolExecutionOutcome::CleanupUnknown => {
                        "tool execution cleanup could not be confirmed".to_owned()
                    }
                    crate::evidence::ToolExecutionOutcome::Failed => public_message,
                    crate::evidence::ToolExecutionOutcome::Succeeded => {
                        unreachable!("an error cannot be a successful tool outcome")
                    }
                }),
                outcome,
                terminal_error,
            }
        }
    }
}

#[allow(clippy::too_many_arguments)] // sequential compatibility wrapper over preflight + launch
async fn execute_tool(
    call_id: &str,
    tool_name: &str,
    args: &serde_json::Value,
    run_id: crate::evidence::RunId,
    evidence_turn_id: crate::evidence::TurnId,
    evidence_call_id: crate::evidence::CallId,
    invocation_context: crate::authority::InvocationContext,
    registry: &crate::authority::ToolRegistry,
    authorizer: Option<&Arc<dyn crate::authority::ToolAuthorizer>>,
    evidence_health: &mut crate::evidence::EvidenceHealth,
    tool_evidence: Option<&mut ToolEvidenceContext<'_>>,
    hooks: &dyn AgentHooks,
    messages: &[AgentMessage],
    cancel: CancellationToken,
    sink: &Option<Arc<dyn DiagnosticSink>>,
) -> Result<ExecutedTool, AgentError> {
    let preflight = preflight_tool(
        call_id,
        tool_name,
        args,
        run_id,
        evidence_turn_id,
        evidence_call_id,
        invocation_context,
        registry,
        authorizer,
        evidence_health,
        tool_evidence,
        hooks,
        messages,
        cancel.clone(),
        sink,
    )
    .await?;
    if cancel.is_cancelled() {
        return Err(AgentError::Cancelled);
    }
    Ok(match preflight {
        ToolPreflight::Ready(prepared) => {
            execute_prepared_tool(call_id, tool_name, prepared, registry, hooks, cancel, sink).await
        }
        ToolPreflight::Rejected(result) => ExecutedTool::ordinary(result),
        ToolPreflight::BatchInvalid(result) => ExecutedTool::ordinary(result),
    })
}

/// A controlled tool error result carrying a single text message (no details).
fn error_text_result(message: String) -> ToolResult {
    ToolResult {
        content: vec![opi_ai::message::OutputContent::Text { text: message }],
        details: None,
        is_error: true,
        terminate: false,
        truncated: false,
        diagnostics: vec![],
    }
}

/// Controlled, secret-free denial outcome. The owning stable code is surfaced
/// in `details` so the authority source is visible without leaking arguments
/// (A08); `redacted_reason` is the model-facing controlled text.
fn denial_result(stable_code: &str, redacted_reason: &str) -> ToolResult {
    ToolResult {
        content: vec![opi_ai::message::OutputContent::Text {
            text: redacted_reason.to_owned(),
        }],
        details: Some(serde_json::json!({ "stable_code": stable_code })),
        is_error: true,
        terminate: false,
        truncated: false,
        diagnostics: vec![],
    }
}

fn drain_queue(queue: &Option<Arc<Mutex<VecDeque<String>>>>) -> Vec<String> {
    match queue {
        Some(q) => {
            let mut q = q.lock().unwrap();
            q.drain(..).collect()
        }
        None => vec![],
    }
}

fn pop_follow_up(queue: &Option<Arc<Mutex<VecDeque<String>>>>) -> Vec<String> {
    match queue {
        Some(q) => {
            let mut q = q.lock().unwrap();
            match q.pop_front() {
                Some(msg) => vec![msg],
                None => vec![],
            }
        }
        None => vec![],
    }
}

fn user_text_message(text: String) -> AgentMessage {
    AgentMessage::Llm(Message::User(UserMessage {
        content: vec![InputContent::Text { text }],
        timestamp_ms: opi_ai::time::now_ms(),
    }))
}

/// Record a diagnostic into the optional sink. A `None` sink disables
/// diagnostic emission without any other observable effect.
fn observe(sink: &Option<Arc<dyn DiagnosticSink>>, diagnostic: Diagnostic) {
    if let Some(sink) = sink {
        sink.record(diagnostic);
    }
}

/// Mint the next monotonic sequence and emit one evidence record through the
/// bound sink. Identities are minted immediately before emission so correlation
/// precedes any external effect. A `None` sink is the capture-disabled default:
/// nothing is minted or emitted and execution behavior is unchanged. An
/// emission failure advances the run's versioned health; incomplete evidence
/// cannot produce a finalized manifest and must be explicitly abandoned.
///
/// The full run/turn/call/parent/kind/payload correlation tuple is intrinsic
/// to one evidence record, so the argument count is the minimal honest shape.
#[allow(clippy::too_many_arguments)]
fn emit_evidence(
    sink: &Option<Arc<dyn crate::evidence::EvidenceSink>>,
    identities: &mut crate::evidence::IdentityAllocator,
    health: &mut crate::evidence::EvidenceHealth,
    run: crate::evidence::RunId,
    turn: Option<crate::evidence::TurnId>,
    call: crate::evidence::CallId,
    parent: Option<crate::evidence::CallId>,
    kind: crate::evidence::CallKind,
    payload: crate::evidence::EvidencePayload,
) -> bool {
    if let Some(sink) = sink.as_ref() {
        let sequence = identities.next_sequence();
        let record = crate::evidence::EvidenceRecord {
            run,
            turn,
            call,
            parent,
            sequence,
            kind,
            payload,
        };
        if let Err(err) = sink.emit(&record) {
            // An emission failure advances the run's versioned health
            // immediately. The run preserves the actual execution outcome; a
            // finalized manifest is withheld once health is incomplete, and the
            // evidence lifecycle must be explicitly abandoned.
            health.advance_on_failure(err.failure_code());
            return false;
        }
    }
    true
}

/// Emit the `AgentEnd` event, deduplicating the exit paths.
fn emit_agent_end(events: &AgentEventSink, messages: &[AgentMessage]) {
    emit_public_event(
        events,
        AgentEvent::AgentEnd {
            messages: messages.to_vec(),
        },
    );
}

fn emit_public_event(events: &AgentEventSink, event: AgentEvent) {
    // `agent_loop` wraps direct consumers in the public event boundary, while
    // `Agent` binds this raw producer sink to its sole redacting subscriber
    // fan-out. Keeping producers raw ensures each route redacts exactly once.
    events(event);
}

/// Build an informational cancellation diagnostic tagged with the lifecycle
/// phase that observed the cancel. Cancellation is harness/user-initiated, so
/// it is `Info`, not an error.
fn cancelled_diagnostic(phase: &str) -> Diagnostic {
    Diagnostic::new(
        Severity::Info,
        CODE_AGENT_CANCELLED,
        SOURCE_AGENT,
        "agent run cancelled",
    )
    .details(json!({ "phase": phase }))
}

/// Build an error tool-failure diagnostic carrying the tool name in details.
fn tool_diagnostic(code: &'static str, tool_name: &str, message: &str) -> Diagnostic {
    Diagnostic::new(Severity::Error, code, SOURCE_TOOL, message)
        .details(json!({ "tool_name": tool_name }))
}

/// Lift one tool-owned [`ToolDiagnostic`] into a [`Diagnostic`]. The tool-owned
/// code is resolved to its stable `&'static str` via
/// [`crate::diagnostic::resolve_tool_code`], and `tool_name` is merged into the
/// structured context so the record carries both the owning tool and the
/// per-cause fields (path for read/write/edit; command/exit_code/cancelled/
/// timed_out/truncated for bash). [`observe`] records the resulting entry in the
/// optional diagnostic sink; the original structured diagnostic remains on the
/// public tool-completion event.
fn tool_owned_diagnostic(tool_diag: &ToolDiagnostic, tool_name: &str) -> Diagnostic {
    let code = crate::diagnostic::resolve_tool_code(&tool_diag.code);
    let mut context = tool_diag.context.clone();
    if let Some(obj) = context.as_object_mut() {
        obj.insert("tool_name".to_string(), json!(tool_name));
    } else {
        // Non-object context (or null): wrap so tool_name is always present.
        context = json!({ "context": context, "tool_name": tool_name });
    }
    Diagnostic::new(
        Severity::Error,
        code,
        SOURCE_TOOL,
        tool_diag.message.clone(),
    )
    .details(context)
}
