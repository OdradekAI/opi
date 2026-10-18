//! Compaction integration tests (task 2.8).
//!
//! DoD: "manual/threshold/overflow triggers, summary record with
//! first_kept_entry_id and tokens before/after, hook extensibility tested"

use opi_agent::compaction::{
    CompactionConfig, CompactionEngine, CompactionHooks, DefaultCompactionHooks, Entry,
    SummarySource,
};
use opi_agent::message::AgentMessage;
use opi_agent::session_event::CompactionReason;
use opi_ai::message::{
    AssistantContent, AssistantMessage, ImageSource, InputContent, MediaType, Message,
    OutputContent, ToolResultMessage, UserMessage,
};
use opi_ai::stream::{StopReason, Usage};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn user_text(id: &str, text: &str) -> Entry {
    Entry {
        id: id.into(),
        message: AgentMessage::Llm(Message::User(UserMessage {
            content: vec![InputContent::Text { text: text.into() }],
            timestamp_ms: 0,
        })),
    }
}

fn assistant_text(id: &str, text: &str) -> Entry {
    Entry {
        id: id.into(),
        message: AgentMessage::Llm(Message::Assistant(AssistantMessage {
            content: vec![AssistantContent::Text { text: text.into() }],
            api: opi_ai::ApiKind::Anthropic,
            provider: "mock".into(),
            model: "mock-model".into(),
            response_model: None,
            wire: None,
            response_id: None,
            usage: Usage::default(),
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp_ms: 0,
        })),
    }
}

// ---------------------------------------------------------------------------
// Trigger tests
// ---------------------------------------------------------------------------

#[test]
fn manual_trigger_always_compacts() {
    let engine = CompactionEngine::new(CompactionConfig::default());
    assert!(engine.should_compact(0, CompactionReason::Manual));
    assert!(engine.should_compact(100_000, CompactionReason::Manual));
}

#[test]
fn overflow_trigger_always_compacts() {
    let engine = CompactionEngine::new(CompactionConfig::default());
    assert!(engine.should_compact(0, CompactionReason::Overflow));
    assert!(engine.should_compact(100_000, CompactionReason::Overflow));
}

#[test]
fn threshold_trigger_compacts_above_threshold() {
    let engine = CompactionEngine::new(CompactionConfig {
        threshold_tokens: 1000,
        ..Default::default()
    });
    assert!(
        engine.should_compact(1500, CompactionReason::Threshold),
        "should compact when tokens exceed threshold"
    );
}

#[test]
fn threshold_trigger_does_not_compact_below_threshold() {
    let engine = CompactionEngine::new(CompactionConfig {
        threshold_tokens: 1000,
        ..Default::default()
    });
    assert!(
        !engine.should_compact(500, CompactionReason::Threshold),
        "should not compact when tokens below threshold"
    );
}

#[test]
fn disabled_engine_no_automatic_compaction() {
    let engine = CompactionEngine::new(CompactionConfig {
        enabled: false,
        ..Default::default()
    });
    assert!(
        !engine.should_compact(1_000_000, CompactionReason::Threshold),
        "disabled should not threshold-compact"
    );
    assert!(
        !engine.should_compact(1_000_000, CompactionReason::Overflow),
        "disabled should not overflow-compact"
    );
}

#[test]
fn disabled_engine_manual_still_works() {
    let engine = CompactionEngine::new(CompactionConfig {
        enabled: false,
        ..Default::default()
    });
    assert!(
        engine.should_compact(0, CompactionReason::Manual),
        "manual should always work even when disabled"
    );
}

// ---------------------------------------------------------------------------
// Summary record tests
// ---------------------------------------------------------------------------

#[test]
fn compact_produces_summary_with_first_kept_entry_id() {
    let engine = CompactionEngine::new(CompactionConfig::default());
    let entries = vec![
        user_text("e1", "Hello, this is a test message with some content"),
        assistant_text(
            "e2",
            "I received your message and here is my response with more content",
        ),
        user_text("e3", "Short"),
    ];

    let result = engine
        .compact(&entries, CompactionReason::Manual, &DefaultCompactionHooks)
        .unwrap();

    assert!(
        !result.summary_text.is_empty(),
        "summary should not be empty"
    );
    assert_eq!(
        result.first_kept_entry_id, "e3",
        "first_kept_entry_id should be the last entry"
    );
}

#[test]
fn compact_records_tokens_before_and_after() {
    let engine = CompactionEngine::new(CompactionConfig {
        threshold_tokens: 10,
        ..Default::default()
    });

    // Create enough entries that some will be compacted
    let entries: Vec<Entry> = (0..20)
        .flat_map(|i| {
            vec![
                user_text(
                    &format!("u{}", i),
                    &format!("User message number {} with substantial content", i),
                ),
                assistant_text(
                    &format!("a{}", i),
                    &format!(
                        "Assistant response number {} with substantial content back",
                        i
                    ),
                ),
            ]
        })
        .collect();

    let result = engine
        .compact(
            &entries,
            CompactionReason::Threshold,
            &DefaultCompactionHooks,
        )
        .unwrap();

    assert!(result.tokens_before > 0, "tokens_before should be positive");
    assert!(result.tokens_after > 0, "tokens_after should be positive");
    assert!(
        result.tokens_after < result.tokens_before,
        "tokens_after should be less than tokens_before"
    );
}

#[test]
fn compact_keeps_recent_entries() {
    let engine = CompactionEngine::new(CompactionConfig {
        threshold_tokens: 10,
        ..Default::default()
    });

    let entries = vec![
        user_text("e1", "Old message 1 with enough text"),
        assistant_text("e2", "Old response 1 with enough text"),
        user_text("e3", "Old message 2 with enough text"),
        assistant_text("e4", "Old response 2 with enough text"),
        user_text("e5", "Recent message with enough text"),
        assistant_text("e6", "Recent response with enough text"),
    ];

    let result = engine
        .compact(&entries, CompactionReason::Manual, &DefaultCompactionHooks)
        .unwrap();

    assert!(
        !result.kept_entries.is_empty(),
        "should keep at least one entry"
    );
    assert_eq!(
        result.first_kept_entry_id, result.kept_entries[0].id,
        "first_kept_entry_id should match first kept entry"
    );
}

#[test]
fn compact_summary_contains_reasonable_text() {
    let engine = CompactionEngine::new(CompactionConfig::default());
    let entries = vec![
        user_text("e1", "Please read the file src/main.rs"),
        assistant_text("e2", "The file contains a hello world program"),
        user_text("e3", "Now update it"),
    ];

    let result = engine
        .compact(&entries, CompactionReason::Manual, &DefaultCompactionHooks)
        .unwrap();

    // Core summary should reference the compacted content
    assert!(
        result.summary_text.len() > 10,
        "summary should have meaningful content, got: {:?}",
        result.summary_text
    );
}

// ---------------------------------------------------------------------------
// Hook extensibility tests
// ---------------------------------------------------------------------------

struct CustomSummaryHook;

impl CompactionHooks for CustomSummaryHook {
    fn generate_summary(&self, _messages: &[AgentMessage]) -> Option<String> {
        Some("Custom hook generated this summary".into())
    }
}

#[test]
fn compact_custom_hook_provides_summary() {
    let engine = CompactionEngine::new(CompactionConfig::default());
    let entries = vec![
        user_text("e1", "Message one"),
        assistant_text("e2", "Response one"),
        user_text("e3", "Message two"),
    ];

    let result = engine
        .compact(&entries, CompactionReason::Manual, &CustomSummaryHook)
        .unwrap();

    assert_eq!(
        result.summary_text, "Custom hook generated this summary",
        "custom hook summary should be used"
    );
    assert_eq!(
        result.summary_source,
        SummarySource::Hook,
        "source should indicate hook"
    );
}

#[test]
fn compact_default_hook_uses_core_summary() {
    let engine = CompactionEngine::new(CompactionConfig::default());
    let entries = vec![
        user_text("e1", "Hello world"),
        assistant_text("e2", "Hi there"),
        user_text("e3", "How are you?"),
    ];

    let result = engine
        .compact(&entries, CompactionReason::Manual, &DefaultCompactionHooks)
        .unwrap();

    assert_eq!(
        result.summary_source,
        SummarySource::Core,
        "source should indicate core"
    );
    assert!(
        !result.summary_text.is_empty(),
        "core summary should not be empty"
    );
}

struct NoSummaryHook;

impl CompactionHooks for NoSummaryHook {
    fn generate_summary(&self, _messages: &[AgentMessage]) -> Option<String> {
        None
    }
}

#[test]
fn compact_hook_returns_none_falls_back_to_core() {
    let engine = CompactionEngine::new(CompactionConfig::default());
    let entries = vec![
        user_text("e1", "First message"),
        assistant_text("e2", "First response"),
        user_text("e3", "Second message"),
    ];

    let result = engine
        .compact(&entries, CompactionReason::Manual, &NoSummaryHook)
        .unwrap();

    assert_eq!(
        result.summary_source,
        SummarySource::Core,
        "should fall back to core when hook returns None"
    );
}

// ---------------------------------------------------------------------------
// Edge case tests
// ---------------------------------------------------------------------------

#[test]
fn compact_empty_entries_returns_error() {
    let engine = CompactionEngine::new(CompactionConfig::default());
    let result = engine.compact(&[], CompactionReason::Manual, &DefaultCompactionHooks);
    assert!(result.is_err(), "empty entries should fail");
}

#[test]
fn compact_single_entry_returns_error() {
    let engine = CompactionEngine::new(CompactionConfig::default());
    let entries = vec![user_text("e1", "Only one message")];
    let result = engine.compact(&entries, CompactionReason::Manual, &DefaultCompactionHooks);
    assert!(
        result.is_err(),
        "single entry should fail — nothing to compact"
    );
}

#[test]
fn compact_two_entries_succeeds() {
    let engine = CompactionEngine::new(CompactionConfig::default());
    let entries = vec![user_text("e1", "First"), assistant_text("e2", "Second")];
    let result = engine.compact(&entries, CompactionReason::Manual, &DefaultCompactionHooks);
    assert!(result.is_ok(), "two entries should compact successfully");
}

#[test]
fn compact_output_messages_include_summary_and_kept() {
    let engine = CompactionEngine::new(CompactionConfig::default());
    let entries = vec![
        user_text("e1", "Old message that will be compacted away"),
        assistant_text("e2", "Old response that will be compacted away"),
        user_text("e3", "This should be kept"),
    ];

    let result = engine
        .compact(&entries, CompactionReason::Manual, &DefaultCompactionHooks)
        .unwrap();

    // Kept entries should contain the last entry
    let kept_ids: Vec<&str> = result.kept_entries.iter().map(|e| e.id.as_str()).collect();
    assert!(
        kept_ids.contains(&"e3"),
        "kept entries should contain e3, got: {:?}",
        kept_ids
    );
    // The compacted entries should NOT be in kept
    assert!(
        !kept_ids.contains(&"e1"),
        "e1 should have been compacted away"
    );
}

// ---------------------------------------------------------------------------
// Config default tests
// ---------------------------------------------------------------------------

#[test]
fn default_config_has_reasonable_values() {
    let config = CompactionConfig::default();
    assert!(config.enabled, "compaction should be enabled by default");
    assert!(config.threshold_tokens > 0, "threshold should be positive");
}

// ---------------------------------------------------------------------------
// Image content in compaction summary
// ---------------------------------------------------------------------------

fn user_with_image(id: &str, text: &str) -> Entry {
    Entry {
        id: id.into(),
        message: AgentMessage::Llm(Message::User(UserMessage {
            content: vec![
                InputContent::Text { text: text.into() },
                InputContent::Image {
                    source: ImageSource::Base64 {
                        data: "iVBORw0KGgo=".into(),
                    },
                    media_type: MediaType::Png,
                },
            ],
            timestamp_ms: 0,
        })),
    }
}

fn tool_result_with_image(id: &str, text: &str) -> Entry {
    Entry {
        id: id.into(),
        message: AgentMessage::Llm(Message::ToolResult(ToolResultMessage {
            tool_call_id: "tc_1".into(),
            tool_name: "screenshot".into(),
            content: vec![
                OutputContent::Text { text: text.into() },
                OutputContent::Image {
                    source: ImageSource::Bytes {
                        data: vec![0x89, 0x50, 0x4e, 0x47],
                    },
                    media_type: MediaType::Png,
                },
            ],
            details: None,
            is_error: false,
            truncated: false,
            timestamp_ms: 0,
        })),
    }
}

#[test]
fn compaction_summary_includes_image_placeholder_for_user_images() {
    let engine = CompactionEngine::new(CompactionConfig::default());
    let entries = vec![
        user_with_image("e1", "Here is a screenshot"),
        assistant_text("e2", "I see the screenshot"),
    ];

    let result = engine
        .compact(&entries, CompactionReason::Manual, &DefaultCompactionHooks)
        .unwrap();

    let summary = &result.summary_text;
    assert!(
        summary.contains("[image: image/png]"),
        "summary should contain image placeholder, got: {summary}"
    );
}

#[test]
fn compaction_summary_includes_image_placeholder_for_tool_results() {
    let engine = CompactionEngine::new(CompactionConfig::default());
    let entries = vec![
        tool_result_with_image("e1", "Tool captured a screenshot"),
        assistant_text("e2", "I analyzed the screenshot"),
    ];

    let result = engine
        .compact(&entries, CompactionReason::Manual, &DefaultCompactionHooks)
        .unwrap();

    let summary = &result.summary_text;
    assert!(
        summary.contains("[image: image/png]"),
        "summary should contain image placeholder for tool result, got: {summary}"
    );
}
