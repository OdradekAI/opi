//! Unit tests for InteractiveCodingHooks (C3/H8).
//!
//! Directly exercises the before_tool_call hook. Interactive mode delegates
//! tool availability to startup tool selection; the hook itself is pass-through.

mod common;

use opi_agent::hooks::{AgentHooks, BeforeToolCallContext, BeforeToolCallResult};
use opi_coding_agent::harness::InteractiveCodingHooks;

fn make_ctx(tool_name: &str) -> BeforeToolCallContext {
    BeforeToolCallContext {
        tool_call_id: "tc-1".into(),
        tool_name: tool_name.into(),
        args: serde_json::json!({}),
        messages: vec![],
    }
}

async fn assert_allowed(hooks: &InteractiveCodingHooks, tool_name: &str) {
    let result = hooks.before_tool_call(make_ctx(tool_name)).await;
    assert!(
        matches!(result, BeforeToolCallResult::Continue),
        "{tool_name} should be allowed"
    );
}

#[tokio::test]
async fn interactive_write_allowed_by_default() {
    let hooks = InteractiveCodingHooks::new(false);
    assert_allowed(&hooks, "write").await;
}

#[tokio::test]
async fn interactive_edit_allowed_by_default() {
    let hooks = InteractiveCodingHooks::new(false);
    assert_allowed(&hooks, "edit").await;
}

#[tokio::test]
async fn interactive_bash_allowed_by_default() {
    let hooks = InteractiveCodingHooks::new(false);
    assert_allowed(&hooks, "bash").await;
}

#[tokio::test]
async fn interactive_read_allowed_by_default() {
    let hooks = InteractiveCodingHooks::new(false);
    assert_allowed(&hooks, "read").await;
}

#[tokio::test]
async fn interactive_glob_allowed_by_default() {
    let hooks = InteractiveCodingHooks::new(false);
    assert_allowed(&hooks, "glob").await;
}

#[tokio::test]
async fn interactive_grep_allowed_by_default() {
    let hooks = InteractiveCodingHooks::new(false);
    assert_allowed(&hooks, "grep").await;
}

#[tokio::test]
async fn interactive_allow_mutating_flag_is_compatibility_noop() {
    let hooks = InteractiveCodingHooks::new(true);
    assert_allowed(&hooks, "write").await;
    assert_allowed(&hooks, "edit").await;
    assert_allowed(&hooks, "bash").await;
    assert_allowed(&hooks, "read").await;
}

#[tokio::test]
async fn interactive_unknown_tool_allowed_by_default() {
    let hooks = InteractiveCodingHooks::new(false);
    assert_allowed(&hooks, "custom_search").await;
}
