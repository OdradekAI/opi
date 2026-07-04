use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use futures_util::StreamExt;
use opi_ai::message::{
    AssistantContent, InputContent, Message, ToolCall, ToolResultMessage, UserMessage,
};
use opi_ai::provider::{Request, validate_request_capabilities};
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
use crate::loop_types::{AgentError, AgentLoopConfig, AgentLoopContext};
use crate::message::AgentMessage;
use crate::tool::{ExecutionMode, Tool, ToolDiagnostic, ToolResult};
use crate::trace::{TraceCollector, TraceKind};
use crate::validation;

/// Run the agent loop until completion or cancellation.
///
/// The loop iterates: provider request, stream response, detect tool calls,
/// validate and execute tools, send tool results back, and repeat until no
/// tool calls or stop condition.
///
/// When `context.trace` is `Some`, the loop emits versioned, redacted trace
/// records (run/turn/provider/tool/diagnostic-linked). Tracing is fail-open: a
/// trace sink write failure never aborts the run. The collector must be
/// prepared by the caller before the loop runs (fail-closed is the caller's
/// responsibility).
pub async fn agent_loop(
    context: AgentLoopContext,
    config: AgentLoopConfig,
    hooks: &dyn AgentHooks,
    events: AgentEventSink,
    cancel: CancellationToken,
) -> Result<Vec<AgentMessage>, AgentError> {
    // Clone the sink/collector handles up front (before any partial move out
    // of `context`) so every failure path below can record an observation.
    // `None` means emission is disabled and nothing below observes any
    // behavior change.
    let diagnostic_sink = context.diagnostic_sink.clone();
    let trace = context.trace.clone();
    let tools_map: HashMap<String, &dyn Tool> = context
        .tools
        .iter()
        .map(|t| (t.definition().name.clone(), t.as_ref()))
        .collect();
    let tool_defs: Vec<_> = context.tools.iter().map(|t| t.definition()).collect();

    let mut messages = context.messages;

    emit_public_event(&events, AgentEvent::AgentStart);
    trace_run(&trace, TraceKind::RunStarted);

    let mut has_tools_pending = false;
    for turn_idx in 0..config.max_turns {
        let turn_id = format!("t{turn_idx}");
        if cancel.is_cancelled() {
            observe(
                &diagnostic_sink,
                &trace,
                cancelled_diagnostic("before_turn"),
            );
            emit_agent_end(&events, &trace, &messages);
            return Err(AgentError::Cancelled);
        }

        emit_public_event(&events, AgentEvent::TurnStart);
        trace_turn(&trace, TraceKind::TurnStarted, &turn_id);

        let transformed = hooks
            .transform_context(messages.clone(), cancel.clone())
            .await?;

        let llm_messages = hooks.convert_to_llm(&transformed)?;

        let mut assistant_content: Vec<AssistantContent> = Vec::new();
        has_tools_pending = false;
        let mut retry_attempt: u32 = 0;
        let max_attempts = config.retry.as_ref().map(|r| r.max_attempts).unwrap_or(0);

        'stream: loop {
            let request = Request {
                model: context.model.clone(),
                system: context.system.clone(),
                messages: llm_messages.clone(),
                tools: tool_defs.clone(),
                max_tokens: config.max_tokens,
                temperature: config.temperature,
                thinking: config.thinking.clone().unwrap_or_default(),
                stop_sequences: vec![],
                metadata: None,
                cancel: cancel.clone(),
            };
            if let Err(e) = validate_request_capabilities(context.provider.as_ref(), &request) {
                observe(
                    &diagnostic_sink,
                    &trace,
                    Diagnostic::new(
                        Severity::Error,
                        CODE_PROVIDER_CAPABILITY_INVALID,
                        SOURCE_PROVIDER,
                        e.to_string(),
                    ),
                );
                trace_provider(&trace, TraceKind::ProviderFailure, &turn_id);
                emit_agent_end(&events, &trace, &messages);
                return Err(AgentError::Provider(e.to_string()));
            }
            trace_provider(&trace, TraceKind::ProviderRequest, &turn_id);
            let mut stream = context.provider.stream(request);
            assistant_content.clear();
            // Track whether the provider has already delivered any stream item
            // for this attempt. Once content has been emitted to the caller, a
            // subsequent mid-stream retryable error must NOT trigger a retry:
            // re-invoking the provider would emit a second Start plus duplicated
            // content. (Phase 12 task 12.7 DoD clause 5.)
            let mut stream_delivered_content = false;

            while let Some(item) = {
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => {
                        observe(
                            &diagnostic_sink,
                            &trace,
                            cancelled_diagnostic("during_stream"),
                        );
                        emit_agent_end(&events, &trace, &messages);
                        return Err(AgentError::Cancelled);
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
                            trace_provider(&trace, TraceKind::ProviderStreamCompletion, &turn_id);

                            messages.push(agent_msg.clone());

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
                                let mut tool_results = Vec::new();
                                let mut terminate_flags = Vec::new();

                                let batch_is_sequential = tool_calls.iter().any(|tc| {
                                    tools_map
                                        .get(tc.name.as_str())
                                        .map(|t| t.execution_mode() == ExecutionMode::Sequential)
                                        .unwrap_or(true)
                                });

                                if batch_is_sequential {
                                    for tc in &tool_calls {
                                        let parsed = parse_tool_call_arguments(tc.clone());

                                        emit_public_event(
                                            &events,
                                            AgentEvent::ToolExecutionStart {
                                                tool_call_id: parsed.tool_call.id.clone(),
                                                tool_name: parsed.tool_call.name.clone(),
                                                args: crate::diagnostic::redact_public_value(
                                                    &parsed.args_for_event,
                                                ),
                                            },
                                        );

                                        let result = match parsed.parsed_args {
                                            Ok(args) => {
                                                execute_tool(
                                                    &parsed.tool_call.id,
                                                    &parsed.tool_call.name,
                                                    &args,
                                                    &tools_map,
                                                    hooks,
                                                    &messages,
                                                    cancel.clone(),
                                                    &diagnostic_sink,
                                                    &trace,
                                                    &turn_id,
                                                )
                                                .await
                                            }
                                            Err(parse_error) => malformed_tool_arguments_result(
                                                &parsed.tool_call.name,
                                                &parse_error,
                                                &diagnostic_sink,
                                                &trace,
                                                &turn_id,
                                            ),
                                        };

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
                                            tool_name: parsed.tool_call.name,
                                            content: result.content,
                                            details: result.details,
                                            is_error,
                                            truncated,
                                            timestamp_ms: 0,
                                        };
                                        tool_results.push(trm.clone());
                                        messages.push(AgentMessage::Llm(Message::ToolResult(trm)));
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
                                                    args: crate::diagnostic::redact_public_value(
                                                        &parsed.args_for_event,
                                                    ),
                                                },
                                            );
                                            parsed
                                        })
                                        .collect();

                                    let futures: Vec<_> = parsed_calls
                                        .iter()
                                        .map(|parsed| {
                                            let tools_map = &tools_map;
                                            let messages = &messages;
                                            let cancel = cancel.clone();
                                            let diagnostic_sink = diagnostic_sink.clone();
                                            let trace = trace.clone();
                                            let turn_id = turn_id.clone();
                                            async move {
                                                match parsed.parsed_args.clone() {
                                                    Ok(args) => {
                                                        execute_tool(
                                                            &parsed.tool_call.id,
                                                            &parsed.tool_call.name,
                                                            &args,
                                                            tools_map,
                                                            hooks,
                                                            messages,
                                                            cancel,
                                                            &diagnostic_sink,
                                                            &trace,
                                                            &turn_id,
                                                        )
                                                        .await
                                                    }
                                                    Err(parse_error) => {
                                                        malformed_tool_arguments_result(
                                                            &parsed.tool_call.name,
                                                            &parse_error,
                                                            &diagnostic_sink,
                                                            &trace,
                                                            &turn_id,
                                                        )
                                                    }
                                                }
                                            }
                                        })
                                        .collect();
                                    let results = futures_util::future::join_all(futures).await;
                                    for (parsed, result) in parsed_calls.iter().zip(results) {
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
                                            timestamp_ms: 0,
                                        };
                                        tool_results.push(trm.clone());
                                        messages.push(AgentMessage::Llm(Message::ToolResult(trm)));
                                    }
                                }

                                let all_terminate = !terminate_flags.is_empty()
                                    && terminate_flags.iter().all(|t| *t);

                                emit_public_event(
                                    &events,
                                    AgentEvent::TurnEnd {
                                        message: agent_msg,
                                        tool_results: tool_results.clone(),
                                    },
                                );
                                trace_turn(&trace, TraceKind::TurnEnded, &turn_id);

                                if all_terminate {
                                    emit_agent_end(&events, &trace, &messages);
                                    return Ok(messages);
                                }

                                let stop_ctx = ShouldStopAfterTurnContext {
                                    messages: messages.clone(),
                                    tool_results,
                                };
                                if hooks.should_stop_after_turn(stop_ctx).await {
                                    emit_agent_end(&events, &trace, &messages);
                                    return Ok(messages);
                                }

                                break 'stream;
                            }

                            emit_public_event(
                                &events,
                                AgentEvent::TurnEnd {
                                    message: agent_msg.clone(),
                                    tool_results: vec![],
                                },
                            );
                            trace_turn(&trace, TraceKind::TurnEnded, &turn_id);

                            let stop_ctx = ShouldStopAfterTurnContext {
                                messages: messages.clone(),
                                tool_results: vec![],
                            };
                            if hooks.should_stop_after_turn(stop_ctx).await {
                                emit_agent_end(&events, &trace, &messages);
                                return Ok(messages);
                            }
                        }
                    }
                    Err(e) => {
                        if e.is_retryable()
                            && retry_attempt < max_attempts
                            && !stream_delivered_content
                            && let Some(ref rc) = config.retry
                        {
                            let retry_after_ms = match &e {
                                opi_ai::provider::ProviderError::RateLimited { retry_after_ms } => {
                                    *retry_after_ms
                                }
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
                            trace_provider(&trace, TraceKind::ProviderRetry, &turn_id);
                            observe(
                                &diagnostic_sink,
                                &trace,
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
                                        &trace,
                                        cancelled_diagnostic("during_retry_sleep"),
                                    );
                                    emit_agent_end(&events, &trace, &messages);
                                    return Err(AgentError::Cancelled);
                                }
                                _ = tokio::time::sleep(
                                    std::time::Duration::from_millis(delay_ms)
                                ) => {}
                            }
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
                                    &trace,
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
                                    &trace,
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
                        observe(&diagnostic_sink, &trace, Diagnostic::from(&e));
                        trace_provider(&trace, TraceKind::ProviderFailure, &turn_id);
                        emit_agent_end(&events, &trace, &messages);
                        return Err(match &e {
                            opi_ai::provider::ProviderError::AuthFailed(msg) => {
                                AgentError::AuthFailed(msg.clone())
                            }
                            opi_ai::provider::ProviderError::Cancelled => AgentError::Cancelled,
                            _ => AgentError::Provider(e.to_string()),
                        });
                    }
                }
            }

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
                    &trace,
                    Diagnostic::new(
                        Severity::Info,
                        CODE_PROVIDER_RETRY_SUCCEEDED,
                        SOURCE_PROVIDER,
                        "provider request succeeded after retry",
                    )
                    .details(json!({ "attempts": retry_attempt })),
                );
            }
            break 'stream;
        }

        let next_turn_ctx = PrepareNextTurnContext {
            messages: messages.clone(),
            turn: turn_idx + 1,
        };
        let mut hook_injected = false;
        if let Some(update) = hooks.prepare_next_turn(next_turn_ctx).await
            && !update.extra_messages.is_empty()
        {
            hook_injected = true;
            messages.extend(update.extra_messages);
        }

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
                messages.push(user_text_message(msg));
            }
            continue;
        }

        if hook_injected {
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
                    messages.push(user_text_message(msg));
                }
                continue;
            }
            break;
        }
    }

    // Phase 11.8 (S2): the for-loop only falls through when `turn_idx` reached
    // `config.max_turns`. If tools were still pending on the final turn the run
    // hit the turn cap mid-work — emit a max-turns warning diagnostic + trace
    // (BEFORE AgentEnd, mirroring the cancellation ordering at the top of the
    // loop) and return `AgentError::MaxTurnsExceeded` instead of a silent Ok. A
    // zero-turn run, or exhaustion without pending tools (e.g. steering kept the
    // loop alive), completes normally.
    if has_tools_pending {
        let err = AgentError::MaxTurnsExceeded(config.max_turns);
        observe(&diagnostic_sink, &trace, Diagnostic::from(&err));
        emit_agent_end(&events, &trace, &messages);
        return Err(err);
    }
    emit_agent_end(&events, &trace, &messages);
    Ok(messages)
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
    trace: &Option<Arc<TraceCollector>>,
    turn_id: &str,
) -> ToolResult {
    trace_tool(trace, TraceKind::ToolCallStarted, tool_name, turn_id);
    observe(
        sink,
        trace,
        tool_diagnostic(
            CODE_TOOL_VALIDATION_FAILED,
            tool_name,
            "tool arguments were not valid JSON",
        ),
    );
    trace_tool(trace, TraceKind::ToolCallFailed, tool_name, turn_id);
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

#[allow(clippy::too_many_arguments)] // private helper threading sinks + turn id alongside existing call context
async fn execute_tool(
    call_id: &str,
    tool_name: &str,
    args: &serde_json::Value,
    tools_map: &HashMap<String, &dyn Tool>,
    hooks: &dyn AgentHooks,
    messages: &[AgentMessage],
    cancel: CancellationToken,
    sink: &Option<Arc<dyn DiagnosticSink>>,
    trace: &Option<Arc<TraceCollector>>,
    turn_id: &str,
) -> ToolResult {
    // Tool call boundary record; emitted for every path below (completed,
    // failed, cancelled) so the trace always brackets a tool execution.
    trace_tool(trace, TraceKind::ToolCallStarted, tool_name, turn_id);

    let tool = match tools_map.get(tool_name) {
        Some(t) => *t,
        None => {
            observe(
                sink,
                trace,
                tool_diagnostic(CODE_TOOL_UNKNOWN, tool_name, "unknown tool requested"),
            );
            trace_tool(trace, TraceKind::ToolCallFailed, tool_name, turn_id);
            return ToolResult {
                content: vec![opi_ai::message::OutputContent::Text {
                    text: format!("unknown tool: {tool_name}"),
                }],
                details: None,
                is_error: true,
                terminate: false,
                truncated: false,
                diagnostics: vec![],
            };
        }
    };

    let schema = &tool.definition().input_schema;
    if let Err(err) = validation::validate(schema, args) {
        observe(
            sink,
            trace,
            tool_diagnostic(
                CODE_TOOL_VALIDATION_FAILED,
                tool_name,
                "tool arguments failed schema validation",
            ),
        );
        trace_tool(trace, TraceKind::ToolCallFailed, tool_name, turn_id);
        return ToolResult::from_validation_error(err);
    }

    let ctx = BeforeToolCallContext {
        tool_call_id: call_id.to_owned(),
        tool_name: tool_name.to_owned(),
        args: args.clone(),
        messages: messages.to_vec(),
    };
    match hooks.before_tool_call(ctx).await {
        BeforeToolCallResult::Allow => {}
        BeforeToolCallResult::Deny { reason } => {
            observe(
                sink,
                trace,
                tool_diagnostic(
                    CODE_TOOL_EXECUTION_FAILED,
                    tool_name,
                    "tool call denied by hook",
                ),
            );
            trace_tool(trace, TraceKind::ToolCallFailed, tool_name, turn_id);
            return ToolResult {
                content: vec![opi_ai::message::OutputContent::Text { text: reason }],
                details: None,
                is_error: true,
                terminate: false,
                truncated: false,
                diagnostics: vec![],
            };
        }
    }

    match tool
        .execute(call_id, args.clone(), cancel.clone(), None)
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
                // Phase 11.8 (S1): lift tool-owned structured failure context from
                // `final_result.diagnostics` into Phase 7 Diagnostics (per-cause
                // code + structured context), mirrored as DiagnosticLinked trace
                // records. The lift reads diagnostics AFTER `after_tool_call`, so a
                // hook `Replace` owns the lifted set. When the tool reported a bare
                // is_error result with no diagnostics, fall back to the generic
                // execution-failed diagnostic so a failure is always observed.
                if final_result.diagnostics.is_empty() {
                    observe(
                        sink,
                        trace,
                        tool_diagnostic(
                            CODE_TOOL_EXECUTION_FAILED,
                            tool_name,
                            "tool returned an error result",
                        ),
                    );
                } else {
                    for tool_diag in &final_result.diagnostics {
                        observe(sink, trace, tool_owned_diagnostic(tool_diag, tool_name));
                    }
                }
                trace_tool(trace, TraceKind::ToolCallFailed, tool_name, turn_id);
            } else {
                trace_tool(trace, TraceKind::ToolCallCompleted, tool_name, turn_id);
            }
            final_result
        }
        Err(e) => {
            observe(
                sink,
                trace,
                tool_diagnostic(
                    CODE_TOOL_EXECUTION_FAILED,
                    tool_name,
                    "tool execution failed",
                ),
            );
            // Distinguish a cancellation from a real failure when the token is
            // set; otherwise this is a genuine tool error. Best-effort: the
            // token may be set just after execute returned.
            let kind = if cancel.is_cancelled() {
                TraceKind::ToolCallCancelled
            } else {
                TraceKind::ToolCallFailed
            };
            trace_tool(trace, kind, tool_name, turn_id);
            ToolResult {
                content: vec![opi_ai::message::OutputContent::Text {
                    text: e.to_string(),
                }],
                details: None,
                is_error: true,
                terminate: false,
                truncated: false,
                diagnostics: vec![],
            }
        }
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
        timestamp_ms: 0,
    }))
}

/// Record a diagnostic into the optional sink AND mirror it as a
/// diagnostic-linked trace record when a collector is attached. A `None` sink
/// disables diagnostic emission without any other observable effect; the trace
/// mirror is independent and fail-open. Routing every runtime diagnostic
/// through here keeps the two surfaces in lockstep.
fn observe(
    sink: &Option<Arc<dyn DiagnosticSink>>,
    trace: &Option<Arc<TraceCollector>>,
    diagnostic: Diagnostic,
) {
    let source = diagnostic.source;
    let code = diagnostic.code;
    let severity = diagnostic.severity;
    if let Some(sink) = sink {
        sink.record(diagnostic);
    }
    if let Some(trace) = trace {
        trace
            .record(source, TraceKind::DiagnosticLinked)
            .severity(severity)
            .diagnostic_code(code)
            .emit();
    }
}

/// Emit a run-scoped trace record (no turn id).
fn trace_run(trace: &Option<Arc<TraceCollector>>, kind: TraceKind) {
    if let Some(trace) = trace {
        trace.record(SOURCE_AGENT, kind).emit();
    }
}

/// Emit a turn-scoped agent trace record.
fn trace_turn(trace: &Option<Arc<TraceCollector>>, kind: TraceKind, turn_id: &str) {
    if let Some(trace) = trace {
        trace.record(SOURCE_AGENT, kind).turn(turn_id).emit();
    }
}

/// Emit a turn-scoped provider trace record.
fn trace_provider(trace: &Option<Arc<TraceCollector>>, kind: TraceKind, turn_id: &str) {
    if let Some(trace) = trace {
        trace.record(SOURCE_PROVIDER, kind).turn(turn_id).emit();
    }
}

/// Emit a turn-scoped tool trace record carrying the tool name in details.
fn trace_tool(
    trace: &Option<Arc<TraceCollector>>,
    kind: TraceKind,
    tool_name: &str,
    turn_id: &str,
) {
    if let Some(trace) = trace {
        trace
            .record(SOURCE_TOOL, kind)
            .turn(turn_id)
            .details(json!({ "tool_name": tool_name }))
            .emit();
    }
}

/// Emit the run-ended trace record (fail-open) followed by the `AgentEnd`
/// event, deduplicating the seven exit paths.
fn emit_agent_end(
    events: &AgentEventSink,
    trace: &Option<Arc<TraceCollector>>,
    messages: &[AgentMessage],
) {
    trace_run(trace, TraceKind::RunEnded);
    emit_public_event(
        events,
        AgentEvent::AgentEnd {
            messages: messages.to_vec(),
        },
    );
}

fn emit_public_event(events: &AgentEventSink, event: AgentEvent) {
    events(event.redacted_for_public());
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

/// Lift one tool-owned [`ToolDiagnostic`] into a Phase 7 [`Diagnostic`] (Phase
/// 11.8 / S1). The tool-owned code is resolved to its stable `&'static str`
/// via [`crate::diagnostic::resolve_tool_code`], and `tool_name` is merged into
/// the structured context so the record carries both the owning tool and the
/// per-cause fields (path for read/write/edit; command/exit_code/cancelled/
/// timed_out/truncated for bash). Routed through [`observe`] so the entry lands
/// in both the diagnostic sink (redacted in Summary mode) and the
/// DiagnosticLinked trace (code + severity only).
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
