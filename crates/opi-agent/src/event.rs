//! Agent event protocol (S7.4).

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RedactionMode, redact_public_value, redact_text};
use crate::message::AgentMessage;
use crate::session_event::{CompactionReason, CompactionResult};
use crate::tool::ToolDiagnostic;

/// Callback type for emitting agent events to subscribers.
pub type AgentEventSink = Box<dyn Fn(AgentEvent) + Send + Sync>;

/// Wrap a low-level event sink in the public redaction boundary.
///
/// The public [`crate::agent_loop`] entry point uses this once around its raw
/// producer sink. Stateful [`crate::Agent`] runs instead route raw loop events
/// into the Agent-owned subscriber fan-out, which applies the same transform.
pub(crate) fn redacting_event_sink(sink: AgentEventSink) -> AgentEventSink {
    Box::new(move |event| sink(event.redacted_for_public()))
}

/// Events emitted during the agent loop lifecycle.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentEvent {
    /// The agent loop has started.
    AgentStart,
    /// The agent loop has ended. No more loop events will be emitted.
    AgentEnd { messages: Vec<AgentMessage> },
    /// A new turn (provider request/response cycle) has started.
    TurnStart,
    /// A turn has ended with the assistant message and any tool results.
    TurnEnd {
        message: AgentMessage,
        tool_results: Vec<opi_ai::message::ToolResultMessage>,
    },
    /// An assistant message has started streaming.
    MessageStart { message: AgentMessage },
    /// An assistant message has been updated with a stream event.
    MessageUpdate {
        message: AgentMessage,
        #[serde(
            serialize_with = "serde_json_bridge::serialize",
            deserialize_with = "deserialize_boxed_stream_event"
        )]
        assistant_event: Box<opi_ai::stream::AssistantStreamEvent>,
    },
    /// An assistant message has finished streaming.
    MessageEnd { message: AgentMessage },
    /// Tool execution is about to begin.
    ToolExecutionStart {
        tool_call_id: String,
        tool_name: String,
        args: serde_json::Value,
    },
    /// Tool execution has produced a progress update.
    ToolExecutionUpdate {
        tool_call_id: String,
        tool_name: String,
        args: serde_json::Value,
        partial_result: serde_json::Value,
    },
    /// Tool execution has completed.
    ToolExecutionEnd {
        tool_call_id: String,
        tool_name: String,
        result: serde_json::Value,
        details: Option<serde_json::Value>,
        is_error: bool,
        #[serde(default)]
        truncated: bool,
        /// Tool-owned structured failure context lifted from
        /// `ToolResult::diagnostics`. Public event emission redacts this context together
        /// with `details`; it is NOT carried on the provider-facing
        /// `ToolResultMessage`. Empty for success/no-diagnostics results and
        /// omitted from the wire when empty (`skip_serializing_if`).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        diagnostics: Vec<ToolDiagnostic>,
    },
    /// Queue messages were delivered to the conversation.
    QueueUpdate {
        steering: Vec<String>,
        follow_up: Vec<String>,
    },
    /// A retryable provider error occurred; a retry attempt is starting.
    AutoRetryStart {
        attempt: u32,
        max_attempts: u32,
        delay_ms: u64,
        error_message: String,
    },
    /// A retry attempt concluded (either successfully or after exhausting attempts).
    AutoRetryEnd {
        success: bool,
        attempt: u32,
        final_error: Option<String>,
    },
    /// Compaction has started. Emitted by the harness outside the agent loop.
    CompactionStart { reason: CompactionReason },
    /// Compaction has finished. Emitted by the harness outside the agent loop.
    CompactionEnd {
        reason: CompactionReason,
        result: Option<CompactionResult>,
        aborted: bool,
        error_message: Option<String>,
    },
    /// Session persistence failed (disk full, permissions, etc.).
    SessionPersistError { message: String },
}

impl AgentEvent {
    pub fn redacted_for_public(&self) -> Self {
        match self {
            AgentEvent::AgentEnd { messages } => AgentEvent::AgentEnd {
                messages: messages.iter().map(redact_agent_message).collect(),
            },
            AgentEvent::TurnEnd {
                message,
                tool_results,
            } => AgentEvent::TurnEnd {
                message: redact_agent_message(message),
                tool_results: tool_results
                    .iter()
                    .map(redact_tool_result_message)
                    .collect(),
            },
            AgentEvent::MessageStart { message } => AgentEvent::MessageStart {
                message: redact_agent_message(message),
            },
            AgentEvent::MessageUpdate {
                message,
                assistant_event,
            } => AgentEvent::MessageUpdate {
                message: redact_agent_message(message),
                assistant_event: Box::new(redact_assistant_stream_event(assistant_event)),
            },
            AgentEvent::MessageEnd { message } => AgentEvent::MessageEnd {
                message: redact_agent_message(message),
            },
            AgentEvent::ToolExecutionStart {
                tool_call_id,
                tool_name,
                args,
            } => AgentEvent::ToolExecutionStart {
                tool_call_id: tool_call_id.clone(),
                tool_name: tool_name.clone(),
                args: redact_public_value(args),
            },
            AgentEvent::ToolExecutionUpdate {
                tool_call_id,
                tool_name,
                args,
                partial_result,
            } => AgentEvent::ToolExecutionUpdate {
                tool_call_id: tool_call_id.clone(),
                tool_name: tool_name.clone(),
                args: redact_public_value(args),
                partial_result: redact_public_value(partial_result),
            },
            AgentEvent::ToolExecutionEnd {
                tool_call_id,
                tool_name,
                result,
                details,
                is_error,
                truncated,
                diagnostics,
            } => AgentEvent::ToolExecutionEnd {
                tool_call_id: tool_call_id.clone(),
                tool_name: tool_name.clone(),
                result: result.clone(),
                details: details.as_ref().map(redact_public_value),
                is_error: *is_error,
                truncated: *truncated,
                diagnostics: diagnostics
                    .iter()
                    .map(|d| crate::tool::ToolDiagnostic {
                        code: d.code.clone(),
                        message: redact_text(&d.message, RedactionMode::Summary),
                        context: redact_public_value(&d.context),
                    })
                    .collect(),
            },
            AgentEvent::AutoRetryStart {
                attempt,
                max_attempts,
                delay_ms,
                error_message,
            } => AgentEvent::AutoRetryStart {
                attempt: *attempt,
                max_attempts: *max_attempts,
                delay_ms: *delay_ms,
                error_message: redact_text(error_message, RedactionMode::Summary),
            },
            AgentEvent::AutoRetryEnd {
                success,
                attempt,
                final_error,
            } => AgentEvent::AutoRetryEnd {
                success: *success,
                attempt: *attempt,
                final_error: final_error
                    .as_ref()
                    .map(|error| redact_text(error, RedactionMode::Summary)),
            },
            AgentEvent::CompactionEnd {
                reason,
                result,
                aborted,
                error_message,
            } => AgentEvent::CompactionEnd {
                reason: *reason,
                result: result.as_ref().map(redact_compaction_result),
                aborted: *aborted,
                error_message: error_message
                    .as_ref()
                    .map(|message| classify_compaction_error(message).into_public()),
            },
            AgentEvent::SessionPersistError { .. } => AgentEvent::SessionPersistError {
                message: ClassifiedPublicText::Untrusted.into_public(),
            },
            other => other.clone(),
        }
    }
}

/// Dynamic model/tool-derived text is not safe merely because it is called a
/// summary. Only exact core-owned closed messages may cross unchanged.
enum ClassifiedPublicText {
    SafeClosed(&'static str),
    Untrusted,
}

impl ClassifiedPublicText {
    fn into_public(self) -> String {
        match self {
            ClassifiedPublicText::SafeClosed(message) => message.to_owned(),
            ClassifiedPublicText::Untrusted => "[REDACTED]".to_owned(),
        }
    }
}

fn classify_compaction_error(message: &str) -> ClassifiedPublicText {
    match message {
        "compaction produced no output" => {
            ClassifiedPublicText::SafeClosed("compaction produced no output")
        }
        _ => ClassifiedPublicText::Untrusted,
    }
}

fn redact_compaction_result(result: &CompactionResult) -> CompactionResult {
    CompactionResult {
        // Core summaries contain prompt/tool text and hook summaries are
        // extension-controlled. Both are untrusted at the public boundary.
        summary: ClassifiedPublicText::Untrusted.into_public(),
        first_kept_entry_id: redact_text(&result.first_kept_entry_id, RedactionMode::Summary),
        tokens_before: result.tokens_before,
        tokens_after: result.tokens_after,
    }
}

fn redact_agent_message(message: &AgentMessage) -> AgentMessage {
    match message {
        AgentMessage::Llm(opi_ai::message::Message::User(user)) => {
            AgentMessage::Llm(opi_ai::message::Message::User(user.clone()))
        }
        AgentMessage::Llm(opi_ai::message::Message::Assistant(assistant)) => AgentMessage::Llm(
            opi_ai::message::Message::Assistant(redact_assistant_message(assistant)),
        ),
        AgentMessage::Llm(opi_ai::message::Message::ToolResult(tool_result)) => AgentMessage::Llm(
            opi_ai::message::Message::ToolResult(redact_tool_result_message(tool_result)),
        ),
        other => other.clone(),
    }
}

fn redact_assistant_message(
    message: &opi_ai::message::AssistantMessage,
) -> opi_ai::message::AssistantMessage {
    opi_ai::message::AssistantMessage {
        content: message
            .content
            .iter()
            .map(redact_assistant_content)
            .collect(),
        api: message.api,
        provider: message.provider.clone(),
        model: message.model.clone(),
        response_model: message.response_model.clone(),
        response_id: message.response_id.clone(),
        usage: message.usage.clone(),
        stop_reason: message.stop_reason,
        error_message: message
            .error_message
            .as_ref()
            .map(|error| redact_text(error, RedactionMode::Summary)),
        timestamp_ms: message.timestamp_ms,
    }
}

fn redact_assistant_content(
    content: &opi_ai::message::AssistantContent,
) -> opi_ai::message::AssistantContent {
    match content {
        opi_ai::message::AssistantContent::Text { text } => {
            opi_ai::message::AssistantContent::Text { text: text.clone() }
        }
        opi_ai::message::AssistantContent::Thinking { thinking } => {
            opi_ai::message::AssistantContent::Thinking {
                thinking: thinking.clone(),
            }
        }
        opi_ai::message::AssistantContent::ToolCall { tool_call } => {
            opi_ai::message::AssistantContent::ToolCall {
                tool_call: redact_tool_call(tool_call),
            }
        }
        other => other.clone(),
    }
}

fn redact_tool_call(tool_call: &opi_ai::message::ToolCall) -> opi_ai::message::ToolCall {
    opi_ai::message::ToolCall {
        id: tool_call.id.clone(),
        name: tool_call.name.clone(),
        arguments: redact_tool_arguments(&tool_call.arguments),
    }
}

fn redact_tool_arguments(arguments: &str) -> String {
    serde_json::from_str::<serde_json::Value>(arguments)
        .map(|value| redact_public_value(&value))
        .and_then(|value| {
            serde_json::to_string(&value)
                .map_err(|_| serde_json::Error::io(std::io::ErrorKind::InvalidData.into()))
        })
        .unwrap_or_else(|_| "\"[REDACTED]\"".to_owned())
}

fn redact_tool_result_message(
    message: &opi_ai::message::ToolResultMessage,
) -> opi_ai::message::ToolResultMessage {
    opi_ai::message::ToolResultMessage {
        tool_call_id: message.tool_call_id.clone(),
        tool_name: message.tool_name.clone(),
        content: message.content.clone(),
        details: message.details.as_ref().map(redact_public_value),
        is_error: message.is_error,
        truncated: message.truncated,
        timestamp_ms: message.timestamp_ms,
    }
}

fn redact_assistant_stream_event(
    event: &opi_ai::stream::AssistantStreamEvent,
) -> opi_ai::stream::AssistantStreamEvent {
    use opi_ai::stream::AssistantStreamEvent;

    match event {
        AssistantStreamEvent::Start { partial } => AssistantStreamEvent::Start {
            partial: redact_assistant_message(partial),
        },
        AssistantStreamEvent::TextStart {
            content_index,
            partial,
        } => AssistantStreamEvent::TextStart {
            content_index: *content_index,
            partial: redact_assistant_message(partial),
        },
        AssistantStreamEvent::TextDelta {
            content_index,
            delta,
            partial,
        } => AssistantStreamEvent::TextDelta {
            content_index: *content_index,
            delta: delta.clone(),
            partial: redact_assistant_message(partial),
        },
        AssistantStreamEvent::TextEnd {
            content_index,
            content,
            partial,
        } => AssistantStreamEvent::TextEnd {
            content_index: *content_index,
            content: content.clone(),
            partial: redact_assistant_message(partial),
        },
        AssistantStreamEvent::ThinkingStart {
            content_index,
            partial,
        } => AssistantStreamEvent::ThinkingStart {
            content_index: *content_index,
            partial: redact_assistant_message(partial),
        },
        AssistantStreamEvent::ThinkingDelta {
            content_index,
            delta,
            partial,
        } => AssistantStreamEvent::ThinkingDelta {
            content_index: *content_index,
            delta: delta.clone(),
            partial: redact_assistant_message(partial),
        },
        AssistantStreamEvent::ThinkingEnd {
            content_index,
            content,
            partial,
        } => AssistantStreamEvent::ThinkingEnd {
            content_index: *content_index,
            content: content.clone(),
            partial: redact_assistant_message(partial),
        },
        AssistantStreamEvent::ToolCallStart {
            content_index,
            partial,
        } => AssistantStreamEvent::ToolCallStart {
            content_index: *content_index,
            partial: redact_assistant_message(partial),
        },
        AssistantStreamEvent::ToolCallDelta {
            content_index,
            partial,
            ..
        } => AssistantStreamEvent::ToolCallDelta {
            content_index: *content_index,
            delta: "[REDACTED]".to_owned(),
            partial: redact_assistant_message(partial),
        },
        AssistantStreamEvent::ToolCallEnd {
            content_index,
            tool_call,
            partial,
        } => AssistantStreamEvent::ToolCallEnd {
            content_index: *content_index,
            tool_call: redact_tool_call(tool_call),
            partial: redact_assistant_message(partial),
        },
        AssistantStreamEvent::Done { reason, message } => AssistantStreamEvent::Done {
            reason: *reason,
            message: redact_assistant_message(message),
        },
        AssistantStreamEvent::Error { reason, message } => AssistantStreamEvent::Error {
            reason: *reason,
            message: redact_assistant_message(message),
        },
        other => other.clone(),
    }
}

fn deserialize_boxed_stream_event<'de, D>(
    deserializer: D,
) -> Result<Box<opi_ai::stream::AssistantStreamEvent>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(deserializer)?;
    let event: opi_ai::stream::AssistantStreamEvent =
        serde_json::from_value(v).map_err(serde::de::Error::custom)?;
    Ok(Box::new(event))
}

mod serde_json_bridge {
    use serde::{Serialize as _, Serializer};

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: serde::Serialize,
        S: Serializer,
    {
        serde_json::to_value(value)
            .map_err(serde::ser::Error::custom)
            .and_then(|v| v.serialize(serializer))
    }
}
