//! Phase 7 task 7.2 — runtime diagnostic classification (opi-agent layer).
//!
//! These tests pin the classification bridges that map provider and agent-loop
//! error families into the shared [`opi_agent::Diagnostic`] vocabulary with
//! stable `code`/`severity`/`source` tuples, redactable details, and optional
//! next actions. They run without any network access.
//!
//! The provider taxonomy itself lives in `opi-ai` (`ProviderError::category`);
//! see `crates/opi-ai/tests/provider_diagnostics.rs`. Runtime emission through
//! production call sites (agent loop retry/cancellation/tool execution, session
//! recovery, compaction) is covered by the wiring cycles.

use opi_agent::diagnostic::code::*;
use opi_agent::diagnostic::{Diagnostic, SOURCE_AGENT, SOURCE_PROVIDER, SOURCE_TOOL, Severity};
use opi_agent::loop_types::AgentError;
use opi_ai::provider::ProviderError;

// ===========================================================================
// From<&ProviderError>: each category maps to a stable (severity, code, source)
// ===========================================================================

#[test]
fn rate_limited_classifies_as_warning_rate_limit_diagnostic() {
    let diag: Diagnostic = (&ProviderError::RateLimited {
        retry_after_ms: Some(5_000),
    })
        .into();
    assert_eq!(diag.severity, Severity::Warning);
    assert_eq!(diag.code, CODE_PROVIDER_RATE_LIMITED);
    assert_eq!(diag.code, "provider_rate_limited");
    assert_eq!(diag.source, SOURCE_PROVIDER);
    // retry_after_ms is benign metadata (not content-sensitive) and is exposed
    // in details so consumers can pace retries.
    let details = diag.details.expect("rate-limited carries retry_after_ms");
    assert_eq!(details["retry_after_ms"], 5_000);
}

#[test]
fn rate_limited_without_retry_after_has_no_details() {
    let diag: Diagnostic = (&ProviderError::RateLimited {
        retry_after_ms: None,
    })
        .into();
    assert_eq!(diag.severity, Severity::Warning);
    assert!(diag.details.is_none(), "no retry_after_ms -> no details");
}

#[test]
fn timeout_classifies_as_warning_timeout_diagnostic() {
    let diag: Diagnostic = (&ProviderError::Timeout).into();
    assert_eq!(diag.severity, Severity::Warning);
    assert_eq!(diag.code, CODE_PROVIDER_TIMEOUT);
    assert_eq!(diag.source, SOURCE_PROVIDER);
}

#[test]
fn request_failed_classifies_as_error_diagnostic() {
    let diag: Diagnostic = (&ProviderError::RequestFailed("internal error".into())).into();
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.code, CODE_PROVIDER_REQUEST_FAILED);
    assert_eq!(diag.source, SOURCE_PROVIDER);
    assert_eq!(diag.message, "provider request failed");
    assert_eq!(
        diag.details.as_ref().unwrap()["provider_error"],
        "internal error"
    );
}

#[test]
fn stream_error_classifies_as_error_diagnostic() {
    let diag: Diagnostic = (&ProviderError::StreamError("connection reset".into())).into();
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.code, CODE_PROVIDER_STREAM_ERROR);
    assert_eq!(diag.source, SOURCE_PROVIDER);
    assert_eq!(diag.message, "provider stream failed");
    assert_eq!(
        diag.details.as_ref().unwrap()["provider_error"],
        "connection reset"
    );
}

#[test]
fn auth_failed_classifies_as_error_with_action() {
    let diag: Diagnostic = (&ProviderError::AuthFailed("invalid api key".into())).into();
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.code, CODE_PROVIDER_AUTH_FAILED);
    assert_eq!(diag.source, SOURCE_PROVIDER);
    let action = diag
        .action
        .as_deref()
        .expect("auth failures carry a remediation action");
    assert!(
        action.to_lowercase().contains("credential") || action.to_lowercase().contains("api key"),
        "action should guide credential fix: {action}"
    );
}

#[test]
fn provider_error_diagnostic_uses_static_message_and_redacted_body_details() {
    let err = ProviderError::RequestFailed(
        "HTTP 500: body carried sk-proj-1234567890abcdefghijklmnopqrstuv".into(),
    );
    let diag = Diagnostic::from(&err);

    assert_eq!(diag.message, "provider request failed");
    assert_eq!(
        diag.details.as_ref().unwrap()["provider_error"]
            .as_str()
            .unwrap(),
        "HTTP 500: body carried sk-proj-1234567890abcdefghijklmnopqrstuv"
    );

    let payload = diag.redacted_payload(opi_agent::diagnostic::RedactionMode::Summary);
    assert_eq!(payload.message, "provider request failed");
    assert_eq!(
        payload.details.as_ref().unwrap()["provider_error"],
        "[REDACTED]"
    );
}

#[test]
fn rate_limited_carries_a_remediation_action() {
    let diag: Diagnostic = (&ProviderError::RateLimited {
        retry_after_ms: Some(1_000),
    })
        .into();
    assert!(diag.action.is_some(), "rate-limited carries an action");
}

// ===========================================================================
// From<&AgentError>: each variant maps to a stable (severity, code, source)
// ===========================================================================

#[test]
fn agent_provider_error_classifies_as_provider_error() {
    let diag: Diagnostic = (&AgentError::Provider("upstream blew up".into())).into();
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.code, CODE_PROVIDER_ERROR);
    assert_eq!(diag.source, SOURCE_PROVIDER);
}

#[test]
fn agent_auth_failed_classifies_as_provider_auth() {
    let diag: Diagnostic = (&AgentError::AuthFailed("expired token".into())).into();
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.code, CODE_PROVIDER_AUTH_FAILED);
    assert_eq!(diag.source, SOURCE_PROVIDER);
}

#[test]
fn agent_tool_error_classifies_as_tool_failure() {
    let diag: Diagnostic = (&AgentError::Tool("write failed".into())).into();
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.code, CODE_TOOL_FAILED);
    assert_eq!(diag.source, SOURCE_TOOL);
}

#[test]
fn agent_hook_error_classifies_as_agent_hook_failure() {
    let diag: Diagnostic = (&AgentError::Hook("before_tool_call blocked".into())).into();
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.code, CODE_HOOK_FAILED);
    assert_eq!(diag.source, SOURCE_AGENT);
}

#[test]
fn agent_cancelled_classifies_as_info_not_error() {
    // Cancellation is harness/user-initiated, not a failure, so it is Info.
    let diag: Diagnostic = (&AgentError::Cancelled).into();
    assert_eq!(diag.severity, Severity::Info);
    assert_eq!(diag.code, CODE_AGENT_CANCELLED);
    assert_eq!(diag.source, SOURCE_AGENT);
}

#[test]
fn agent_max_turns_classifies_as_warning() {
    let diag: Diagnostic = (&AgentError::MaxTurnsExceeded(50)).into();
    assert_eq!(diag.severity, Severity::Warning);
    assert_eq!(diag.code, CODE_AGENT_MAX_TURNS_EXCEEDED);
    assert_eq!(diag.source, SOURCE_AGENT);
    let details = diag
        .details
        .as_ref()
        .expect("max-turns carries the limit in details");
    assert_eq!(details["max_turns"], 50);
}

// ===========================================================================
// Code + source vocabulary stability (typos become compile/literal errors)
// ===========================================================================

#[test]
fn provider_code_constants_are_stable_literals() {
    assert_eq!(CODE_PROVIDER_AUTH_FAILED, "provider_auth_failed");
    assert_eq!(CODE_PROVIDER_RATE_LIMITED, "provider_rate_limited");
    assert_eq!(CODE_PROVIDER_TIMEOUT, "provider_timeout");
    assert_eq!(CODE_PROVIDER_REQUEST_FAILED, "provider_request_failed");
    assert_eq!(CODE_PROVIDER_STREAM_ERROR, "provider_stream_error");
    assert_eq!(CODE_PROVIDER_ERROR, "provider_error");
}

#[test]
fn agent_code_constants_are_stable_literals() {
    assert_eq!(CODE_TOOL_FAILED, "tool_failed");
    assert_eq!(CODE_HOOK_FAILED, "hook_failed");
    assert_eq!(CODE_AGENT_CANCELLED, "agent_cancelled");
    assert_eq!(CODE_AGENT_MAX_TURNS_EXCEEDED, "agent_max_turns_exceeded");
}

#[test]
fn source_agent_is_stable_literal() {
    assert_eq!(SOURCE_AGENT, "agent");
}

// ===========================================================================
// Details redaction: provider retry_after_ms survives; sensitive keys scrub
// ===========================================================================

#[test]
fn diagnostic_details_round_trip_through_redaction() {
    use opi_agent::diagnostic::RedactionMode;
    let diag =
        Diagnostic::from(&ProviderError::RequestFailed("boom".into())).details(serde_json::json!({
            "retry_after_ms": 2000,
            "prompt": "hidden system prompt",
            "api_key": "sk-ant-1234567890abcdefghijklmnopqrstuv"
        }));
    let redacted = diag.redacted_details(RedactionMode::Summary).unwrap();
    assert_eq!(redacted["retry_after_ms"], 2000);
    assert_eq!(redacted["prompt"], "[REDACTED]");
    assert_eq!(redacted["api_key"], "[REDACTED]");
}

// ===========================================================================
// DiagnosticSink: minimal emission substrate (RecordingSink / NullSink)
//
// This is the in-process observation channel Phase 7 task 7.2 wires failure
// paths to. It is intentionally NOT the durable trace envelope (a later task);
// here we only need somewhere to record a Diagnostic and read it back.
// ===========================================================================

#[test]
fn recording_sink_captures_diagnostics_in_emission_order() {
    use opi_agent::diagnostic::{Diagnostic, Severity};
    use opi_agent::{DiagnosticSink, RecordingSink};
    let sink = RecordingSink::new();
    assert!(sink.is_empty());
    sink.record(Diagnostic::new(Severity::Info, "first", "agent", "started"));
    sink.record(Diagnostic::new(
        Severity::Error,
        "second",
        "agent",
        "failed",
    ));
    let snapshot = sink.snapshot();
    assert_eq!(sink.len(), 2);
    assert_eq!(snapshot.len(), 2);
    assert_eq!(snapshot[0].code, "first");
    assert_eq!(snapshot[1].code, "second");
}

#[test]
fn null_sink_silently_discards_diagnostics() {
    use opi_agent::diagnostic::{Diagnostic, Severity};
    use opi_agent::{DiagnosticSink, NullSink};
    // Contract: discards silently, never panics.
    NullSink.record(Diagnostic::new(
        Severity::Info,
        "dropped",
        "agent",
        "ignored",
    ));
}

#[test]
fn recording_sink_is_shareable_as_dyn_diagnostic_sink() {
    use opi_agent::diagnostic::{Diagnostic, Severity};
    use opi_agent::{DiagnosticSink, RecordingSink};
    use std::sync::Arc;
    // The runtime holds the sink behind an `Arc<dyn DiagnosticSink>` so the
    // concrete type (RecordingSink in tests, a trace sink later) can vary.
    let sink: Arc<dyn DiagnosticSink> = Arc::new(RecordingSink::new());
    sink.record(Diagnostic::new(
        Severity::Warning,
        "shared",
        "agent",
        "via dyn",
    ));
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Arc<dyn DiagnosticSink>>();
}

// ===========================================================================
// Session corrupt-line recovery classification
//
// SessionReader returns a CrashRecovery; the harness inspects it and records
// the matching diagnostic(s). The mapping is pure and lives on CrashRecovery so
// it can be tested here without driving the harness.
// ===========================================================================

mod session_recovery_classification {
    use opi_agent::diagnostic::code::*;
    use opi_agent::diagnostic::{SOURCE_SESSION, Severity};
    use opi_agent::session::CrashRecovery;

    #[test]
    fn clean_recovery_emits_no_diagnostics() {
        assert!(CrashRecovery::default().diagnostics().is_empty());
    }

    #[test]
    fn truncated_line_emits_warning() {
        let recovery = CrashRecovery {
            truncated_line: true,
            ..Default::default()
        };
        let diags = recovery.diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, CODE_SESSION_TRUNCATED_LINE);
        assert_eq!(diags[0].source, SOURCE_SESSION);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn corrupt_entries_carries_count() {
        let recovery = CrashRecovery {
            corrupt_count: 3,
            ..Default::default()
        };
        let diags = recovery.diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, CODE_SESSION_CORRUPT_ENTRIES);
        assert_eq!(diags[0].severity, Severity::Warning);
        let details = diags[0].details.as_ref().expect("carries count");
        assert_eq!(details["corrupt_count"], 3);
        // Counts only — never entry content.
        assert!(details.get("entries").is_none());
    }

    #[test]
    fn unknown_entries_carries_count() {
        // Phase 13.1: unknown future entry types are reported separately from
        // corruption, never fatal.
        let recovery = CrashRecovery {
            unknown_count: 4,
            ..Default::default()
        };
        let diags = recovery.diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, CODE_SESSION_UNKNOWN_ENTRIES);
        assert_eq!(diags[0].source, SOURCE_SESSION);
        assert_eq!(diags[0].severity, Severity::Warning);
        let details = diags[0].details.as_ref().expect("carries count");
        assert_eq!(details["unknown_count"], 4);
        assert!(details.get("entries").is_none());
    }

    #[test]
    fn corrupt_and_truncation_emit_one_diagnostic_per_observation() {
        // Phase 13.1 splits the historical combined "corrupt with truncation"
        // code into two independent diagnostics so each observation keeps its
        // own stable code.
        let recovery = CrashRecovery {
            truncated_line: true,
            corrupt_count: 2,
            ..Default::default()
        };
        let diags = recovery.diagnostics();
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].code, CODE_SESSION_TRUNCATED_LINE);
        assert_eq!(diags[1].code, CODE_SESSION_CORRUPT_ENTRIES);
        assert_eq!(diags[1].details.as_ref().unwrap()["corrupt_count"], 2);
    }
}

// ===========================================================================
// Compaction classification
//
// CompactionEngine returns Result<CompactionOutput, CompactionError>; the
// harness maps it to a single diagnostic. NothingToCompact is informational
// (a no-op), a successful compaction is an informational observation with
// before/after token counts.
// ===========================================================================

mod compaction_classification {
    use opi_agent::compaction::{CompactionError, CompactionOutput, SummarySource};
    use opi_agent::diagnostic::Diagnostic;
    use opi_agent::diagnostic::code::*;
    use opi_agent::diagnostic::{SOURCE_SESSION, Severity};
    use opi_agent::session_event::CompactionReason;

    #[test]
    fn nothing_to_compact_is_informational() {
        let diag = Diagnostic::from(&CompactionError::NothingToCompact);
        assert_eq!(diag.severity, Severity::Info);
        assert_eq!(diag.code, CODE_COMPACTION_NOTHING_TO_COMPACT);
        assert_eq!(diag.source, SOURCE_SESSION);
    }

    #[test]
    fn successful_compaction_emits_info_with_token_counts() {
        let output = CompactionOutput {
            reason: CompactionReason::Manual,
            summary_text: "compacted".into(),
            first_kept_entry_id: "e8".into(),
            tokens_before: 1000,
            tokens_after: 200,
            kept_entries: vec![],
            summary_source: SummarySource::Core,
        };
        let diag = output.diagnostic();
        assert_eq!(diag.severity, Severity::Info);
        assert_eq!(diag.code, CODE_SESSION_COMPACTED);
        assert_eq!(diag.source, SOURCE_SESSION);
        let details = diag.details.as_ref().expect("carries token counts");
        assert_eq!(details["tokens_before"], 1000);
        assert_eq!(details["tokens_after"], 200);
    }
}

// ===========================================================================
// Phase 8 task 8.6 — session/compaction diagnostics reach the in-process sink
//
// The pure classification bridges above live on CrashRecovery / CompactionOutput
// / CompactionError; this gate pins that their Diagnostic output round-trips
// through the in-process RecordingSink (the trait-object path the harness holds)
// with code/severity/source/details intact. The full end-to-end wiring
// (resume_session_id -> RecordingSink -> diagnostic_counts) is in
// opi-coding-agent and is covered by session_runtime.rs; here we lock the
// substrate the harness depends on.
// ===========================================================================

mod session_compaction_sink_round_trip {
    use std::sync::Arc;

    use opi_agent::compaction::{CompactionError, CompactionOutput, SummarySource};
    use opi_agent::diagnostic::code::*;
    use opi_agent::diagnostic::{Diagnostic, SOURCE_SESSION, Severity};
    use opi_agent::session::CrashRecovery;
    use opi_agent::session_event::CompactionReason;
    use opi_agent::{DiagnosticSink, RecordingSink};

    #[test]
    fn phase8_session_compaction_diagnostics_sink() {
        let sink = RecordingSink::new();

        // --- Crash recovery observations round-trip through the sink ---
        for diagnostic in (CrashRecovery {
            truncated_line: true,
            ..Default::default()
        })
        .diagnostics()
        {
            sink.record(diagnostic);
        }
        for diagnostic in (CrashRecovery {
            corrupt_count: 3,
            ..Default::default()
        })
        .diagnostics()
        {
            sink.record(diagnostic);
        }
        // A recovery that combines truncation and corruption now emits two
        // independent diagnostics (Phase 13.1 split the historical combined
        // code so each observation keeps its own stable code).
        for diagnostic in (CrashRecovery {
            truncated_line: true,
            corrupt_count: 2,
            ..Default::default()
        })
        .diagnostics()
        {
            sink.record(diagnostic);
        }
        // Clean recovery emits nothing.
        assert!(CrashRecovery::default().diagnostics().is_empty());

        // --- Compaction success + nothing-to-compact round-trip ---
        let output = CompactionOutput {
            reason: CompactionReason::Manual,
            summary_text: "compacted".into(),
            first_kept_entry_id: "e8".into(),
            tokens_before: 1000,
            tokens_after: 200,
            kept_entries: vec![],
            summary_source: SummarySource::Hook,
        };
        sink.record(output.diagnostic());
        sink.record(Diagnostic::from(&CompactionError::NothingToCompact));

        let snapshot = sink.snapshot();
        // truncated + corrupt(3) + (truncated + corrupt(2)) + compacted + nothing = 6
        assert_eq!(
            snapshot.len(),
            6,
            "expected four recovery diagnostics plus compacted and nothing_to_compact"
        );

        // All four recovery diagnostics are warnings sourced from the session.
        for diag in &snapshot[0..4] {
            assert_eq!(diag.severity, Severity::Warning);
            assert_eq!(diag.source, SOURCE_SESSION);
        }

        // Two truncated + two corrupt (counts 3 and 2) across the observations.
        let truncated_count = snapshot
            .iter()
            .filter(|d| d.code == CODE_SESSION_TRUNCATED_LINE)
            .count();
        let corrupt_with = |n: i64| {
            snapshot
                .iter()
                .filter(|d| {
                    d.code == CODE_SESSION_CORRUPT_ENTRIES
                        && d.details
                            .as_ref()
                            .is_some_and(|dt| dt["corrupt_count"].as_i64() == Some(n))
                })
                .count()
        };
        assert_eq!(truncated_count, 2);
        assert_eq!(corrupt_with(3), 1);
        assert_eq!(corrupt_with(2), 1);
        // No entry content ever leaks into recovery details.
        for diag in &snapshot[0..4] {
            if let Some(details) = &diag.details {
                assert!(details.get("entries").is_none(), "no entry content");
            }
        }

        // Compaction success: Info, session_compacted, token/source details survive.
        let compacted = snapshot
            .iter()
            .find(|d| d.code == CODE_SESSION_COMPACTED)
            .expect("compacted diagnostic present");
        assert_eq!(compacted.severity, Severity::Info);
        assert_eq!(compacted.source, SOURCE_SESSION);
        let compacted_details = compacted
            .details
            .as_ref()
            .expect("compaction carries details");
        assert_eq!(compacted_details["tokens_before"], 1000);
        assert_eq!(compacted_details["tokens_after"], 200);
        assert_eq!(compacted_details["summary_source"], "hook");

        // NothingToCompact: Info, compaction_nothing_to_compact.
        let nothing = snapshot
            .iter()
            .find(|d| d.code == CODE_COMPACTION_NOTHING_TO_COMPACT)
            .expect("nothing_to_compact diagnostic present");
        assert_eq!(nothing.severity, Severity::Info);
        assert_eq!(nothing.source, SOURCE_SESSION);

        // --- Trait-object path the harness holds (Arc<dyn DiagnosticSink>) ---
        // The runtime stores the sink behind `Arc<dyn DiagnosticSink>`; record
        // goes through the trait object while the concrete RecordingSink keeps
        // the buffer so we can snapshot it.
        let concrete = Arc::new(RecordingSink::new());
        let dyn_sink: Arc<dyn DiagnosticSink> = concrete.clone();
        for diagnostic in (CrashRecovery {
            corrupt_count: 7,
            ..Default::default()
        })
        .diagnostics()
        {
            dyn_sink.record(diagnostic);
        }
        let dyn_snapshot = concrete.snapshot();
        assert_eq!(dyn_snapshot.len(), 1);
        assert_eq!(dyn_snapshot[0].code, CODE_SESSION_CORRUPT_ENTRIES);
        assert_eq!(dyn_snapshot[0].severity, Severity::Warning);
        assert_eq!(
            dyn_snapshot[0].details.as_ref().unwrap()["corrupt_count"],
            7
        );
    }
}

// ===========================================================================
// Runtime emission through production agent_loop paths
//
// These drive the real agent_loop with a MockProvider and a RecordingSink
// threaded via AgentLoopContext.diagnostic_sink, then assert each failure path
// emits a shared Diagnostic with the expected severity/code/source. Runtime
// behavior (return values, events, retry timing) is unchanged.
// ===========================================================================

mod runtime_emission {
    use std::sync::Arc;

    use opi_agent::agent_loop;
    use opi_agent::diagnostic::code::*;
    use opi_agent::diagnostic::{SOURCE_AGENT, SOURCE_PROVIDER, SOURCE_TOOL, Severity};
    use opi_agent::event::{AgentEvent, AgentEventSink};
    use opi_agent::hooks::AgentHooks;
    use opi_agent::loop_types::{AgentError, AgentLoopConfig, AgentLoopContext};
    use opi_agent::message::AgentMessage;
    use opi_agent::{DiagnosticSink, RecordingSink};
    use opi_ai::message::{InputContent, Message, UserMessage};
    use opi_ai::provider::ProviderError;
    use opi_ai::retry::RetryConfig;
    use opi_ai::test_support::{self, MockProvider, MockResponse};

    struct NoopHooks;
    impl AgentHooks for NoopHooks {
        fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
            let mut out = Vec::new();
            for msg in messages {
                if let AgentMessage::Llm(m) = msg {
                    out.push(m.clone());
                }
            }
            Ok(out)
        }
    }

    fn user_msg(text: &str) -> AgentMessage {
        AgentMessage::Llm(Message::User(UserMessage {
            content: vec![InputContent::Text { text: text.into() }],
            timestamp_ms: 0,
        }))
    }

    fn ctx(provider: MockProvider, sink: Arc<RecordingSink>) -> AgentLoopContext {
        AgentLoopContext {
            provider: Box::new(provider),
            tools: vec![],
            messages: vec![user_msg("hello")],
            model: "mock-model".into(),
            system: None,
            steering_queue: None,
            follow_up_queue: None,
            diagnostic_sink: Some(sink as Arc<dyn DiagnosticSink>),
            trace: None,
            session_id: None,
        }
    }

    fn config(retry: Option<RetryConfig>) -> AgentLoopConfig {
        AgentLoopConfig {
            max_turns: 10,
            max_tokens: None,
            temperature: None,
            retry,
            ..Default::default()
        }
    }

    fn fast_retry() -> RetryConfig {
        RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 1,
            max_delay_ms: 10,
        }
    }

    fn null_event_sink() -> AgentEventSink {
        Box::new(|_: AgentEvent| {})
    }

    fn codes_of(sink: &RecordingSink) -> Vec<&'static str> {
        let snapshot = sink.snapshot();
        snapshot.iter().map(|d| d.code).collect()
    }

    #[tokio::test]
    async fn retry_then_success_emits_attempt_and_succeeded() {
        let provider = MockProvider::new_with_errors(
            "mock",
            vec![
                MockResponse::Error(ProviderError::RateLimited {
                    retry_after_ms: Some(1),
                }),
                MockResponse::Events(test_support::text_response("ok")),
            ],
        );
        let sink = Arc::new(RecordingSink::new());
        let result = agent_loop(
            ctx(provider, sink.clone()),
            config(Some(fast_retry())),
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(result.is_ok(), "{:?}", result.err());

        let codes = codes_of(&sink);
        assert!(
            codes.contains(&CODE_PROVIDER_RETRY_ATTEMPT),
            "expected a retry-attempt diagnostic, got {codes:?}"
        );
        assert!(codes.contains(&CODE_PROVIDER_RETRY_SUCCEEDED));
        // Every retry diagnostic must be provider-sourced; attempts are
        // recoverable (Warning), success is informational (Info).
        let snap = sink.snapshot();
        let attempt = snap
            .iter()
            .find(|d| d.code == CODE_PROVIDER_RETRY_ATTEMPT)
            .unwrap();
        assert_eq!(attempt.severity, Severity::Warning);
        assert_eq!(attempt.source, SOURCE_PROVIDER);
        let succeeded = snap
            .iter()
            .find(|d| d.code == CODE_PROVIDER_RETRY_SUCCEEDED)
            .unwrap();
        assert_eq!(succeeded.severity, Severity::Info);
    }

    #[tokio::test]
    async fn retry_exhausted_emits_exhausted_error() {
        let provider = MockProvider::new_with_errors(
            "mock",
            vec![
                MockResponse::Error(ProviderError::RateLimited {
                    retry_after_ms: Some(1),
                }),
                MockResponse::Error(ProviderError::RateLimited {
                    retry_after_ms: Some(1),
                }),
                MockResponse::Error(ProviderError::RateLimited {
                    retry_after_ms: Some(1),
                }),
            ],
        );
        let sink = Arc::new(RecordingSink::new());
        let retry = RetryConfig {
            max_attempts: 2,
            initial_delay_ms: 1,
            max_delay_ms: 10,
        };
        let result = agent_loop(
            ctx(provider, sink.clone()),
            config(Some(retry)),
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(result.is_err());

        let snap = sink.snapshot();
        let exhausted = snap
            .iter()
            .find(|d| d.code == CODE_PROVIDER_RETRY_EXHAUSTED)
            .expect("retry exhaustion emits an error diagnostic");
        assert_eq!(exhausted.severity, Severity::Error);
        assert_eq!(exhausted.source, SOURCE_PROVIDER);
    }

    #[tokio::test]
    async fn retry_after_prior_attempt_then_partial_stream_error_emits_suppressed_not_exhausted() {
        let mut partial_events = test_support::text_response("partial after retry");
        partial_events.pop();

        let provider = MockProvider::new_with_errors(
            "mock",
            vec![
                MockResponse::Error(ProviderError::RateLimited {
                    retry_after_ms: Some(1),
                }),
                MockResponse::EventsThenError(
                    partial_events,
                    ProviderError::RateLimited {
                        retry_after_ms: Some(1),
                    },
                ),
            ],
        );
        let sink = Arc::new(RecordingSink::new());
        let result = agent_loop(
            ctx(provider, sink.clone()),
            config(Some(fast_retry())),
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(
            result.is_err(),
            "partial stream error after retry should fail"
        );

        let snap = sink.snapshot();
        let suppressed = snap
            .iter()
            .find(|d| d.code == CODE_PROVIDER_RETRY_SUPPRESSED_AFTER_PARTIAL_OUTPUT)
            .expect("partial-after-retry error must emit the suppression diagnostic");
        assert_eq!(suppressed.severity, Severity::Warning);
        assert_eq!(suppressed.source, SOURCE_PROVIDER);
        assert!(
            snap.iter().all(|d| d.code != CODE_PROVIDER_RETRY_EXHAUSTED),
            "partial-after-retry suppression must not emit retry_exhausted"
        );
    }

    #[tokio::test]
    async fn auth_failure_emits_provider_auth_diagnostic() {
        let provider = MockProvider::new_with_errors(
            "mock",
            vec![MockResponse::Error(ProviderError::AuthFailed(
                "bad key".into(),
            ))],
        );
        let sink = Arc::new(RecordingSink::new());
        let result = agent_loop(
            ctx(provider, sink.clone()),
            config(Some(fast_retry())),
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(result.is_err());

        let snap = sink.snapshot();
        let auth = snap
            .iter()
            .find(|d| d.code == CODE_PROVIDER_AUTH_FAILED)
            .expect("auth failure emits provider_auth_failed");
        assert_eq!(auth.severity, Severity::Error);
        assert_eq!(auth.source, SOURCE_PROVIDER);
        // No retry should have been attempted for a non-retryable auth error.
        assert!(!codes_of(&sink).contains(&CODE_PROVIDER_RETRY_ATTEMPT));
    }

    #[tokio::test]
    async fn cancellation_emits_agent_cancelled_info() {
        let provider =
            MockProvider::new("mock", vec![test_support::text_response("never reached")]);
        let sink = Arc::new(RecordingSink::new());
        let cancel = tokio_util::sync::CancellationToken::new();
        cancel.cancel();
        let result = agent_loop(
            ctx(provider, sink.clone()),
            config(None),
            &NoopHooks,
            null_event_sink(),
            cancel,
        )
        .await;
        assert!(matches!(result, Err(AgentError::Cancelled)));

        let snap = sink.snapshot();
        let cancelled = snap
            .iter()
            .find(|d| d.code == CODE_AGENT_CANCELLED)
            .expect("cancellation emits agent_cancelled");
        // Cancellation is harness/user-initiated, so it is Info, not Error.
        assert_eq!(cancelled.severity, Severity::Info);
        assert_eq!(cancelled.source, SOURCE_AGENT);
    }

    #[tokio::test]
    async fn unknown_tool_call_emits_tool_unknown_diagnostic() {
        let provider = MockProvider::new(
            "mock",
            vec![
                test_support::tool_call_response("c1", "ghost", "{}"),
                test_support::text_response("done"),
            ],
        );
        let sink = Arc::new(RecordingSink::new());
        let result = agent_loop(
            ctx(provider, sink.clone()),
            config(None),
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(result.is_ok(), "{:?}", result.err());

        let snap = sink.snapshot();
        let unknown = snap
            .iter()
            .find(|d| d.code == CODE_TOOL_UNKNOWN)
            .expect("unknown tool emits tool_unknown");
        assert_eq!(unknown.severity, Severity::Error);
        assert_eq!(unknown.source, SOURCE_TOOL);
        let details = unknown.details.as_ref().expect("carries tool_name");
        assert_eq!(details["tool_name"], "ghost");
    }

    #[tokio::test]
    async fn max_turns_exhaustion_emits_warning_diagnostic_and_error() {
        // Phase 11.8 S2: exhausting max_turns with tools still pending emits a
        // CODE_AGENT_MAX_TURNS_EXCEEDED warning diagnostic and returns
        // AgentError::MaxTurnsExceeded (not a silent Ok(messages)).
        use opi_agent::tool::{Tool, ToolError, ToolResult};
        use opi_ai::message::{OutputContent, ToolDef};
        use std::future::Future;
        use std::pin::Pin;

        struct PendingTool;
        impl Tool for PendingTool {
            fn definition(&self) -> ToolDef {
                ToolDef {
                    name: "pending".into(),
                    description: "always succeeds; keeps the loop calling".into(),
                    input_schema: serde_json::json!({"type": "object"}),
                }
            }
            fn execute(
                &self,
                _call_id: &str,
                _arguments: serde_json::Value,
                _signal: tokio_util::sync::CancellationToken,
                _on_update: Option<opi_agent::tool::UpdateCallback>,
            ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
                Box::pin(async {
                    Ok(ToolResult {
                        content: vec![OutputContent::Text { text: "ok".into() }],
                        details: None,
                        is_error: false,
                        terminate: false,
                        truncated: false,
                        diagnostics: vec![],
                    })
                })
            }
        }

        let provider = MockProvider::new(
            "mock",
            vec![
                test_support::tool_call_response("c1", "pending", "{}"),
                test_support::tool_call_response("c2", "pending", "{}"),
            ],
        );
        let sink = Arc::new(RecordingSink::new());
        let loop_ctx = AgentLoopContext {
            provider: Box::new(provider),
            tools: vec![Box::new(PendingTool)],
            messages: vec![user_msg("go")],
            model: "mock-model".into(),
            system: None,
            steering_queue: None,
            follow_up_queue: None,
            diagnostic_sink: Some(sink.clone() as Arc<dyn DiagnosticSink>),
            trace: None,
            session_id: None,
        };
        let cfg = AgentLoopConfig {
            max_turns: 2,
            ..Default::default()
        };
        let result = agent_loop(
            loop_ctx,
            cfg,
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;

        // Returns MaxTurnsExceeded(2), not Ok.
        let err = result.expect_err("max-turns exhaustion must return an error, not Ok");
        assert!(
            matches!(err, AgentError::MaxTurnsExceeded(2)),
            "expected MaxTurnsExceeded(2), got {err:?}"
        );

        // Emits the warning diagnostic with the limit in details.
        let snap = sink.snapshot();
        let mt = snap
            .iter()
            .find(|d| d.code == CODE_AGENT_MAX_TURNS_EXCEEDED)
            .expect("max-turns warning diagnostic must be emitted");
        assert_eq!(mt.severity, Severity::Warning);
        assert_eq!(mt.source, SOURCE_AGENT);
        assert_eq!(mt.details.as_ref().expect("details")["max_turns"], 2);
    }
}
