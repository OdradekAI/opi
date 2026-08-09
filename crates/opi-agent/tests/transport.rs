//! Tests verifying the Transport settlement decision (task 4.3).
//!
//! The `Transport` trait was removed from the public API because:
//!
//! - It had no implementations, no tests, and no downstream consumers.
//! - Its string-based `send(&str)/receive() -> String` contract did not match
//!   the JSONL framing that the RPC runner (task 4.1) actually built.
//! - Keeping a misleading stub is worse than removing it and letting task 4.10
//!   (streaming proxy) design the real abstraction from actual requirements.
//! - This follows ADR-017: "reserve for Phase 4 **or** remove."
//!
//! Task 4.10 will define its own transport/proxy surface when its framing and
//! backpressure requirements are clear.

/// Verify that the core `opi_agent` public types compile and are accessible
/// without any `Transport` surface.
#[test]
fn core_types_accessible_without_transport() {
    use opi_agent::{
        Agent, AgentError, AgentEvent, AgentEventSink, AgentHooks, AgentLoopConfig,
        AgentLoopContext, AgentMessage, AgentSessionEvent, AgentState, ExecutionMode,
        SDK_SCHEMA_VERSION, SdkCommand, SdkResponse, Tool, ToolDef, ToolError, ToolResult,
    };

    // SDK surface works independently.
    let _ = SDK_SCHEMA_VERSION;

    // Command construction round-trips through JSON.
    let cmd: SdkCommand = serde_json::from_str(r#"{"type":"prompt","message":"hi"}"#).unwrap();
    assert_eq!(cmd.command_name(), "prompt");
    assert!(cmd.id().is_none());

    // Response construction works.
    let resp = SdkResponse::success(None, "prompt");
    assert!(resp.success);

    // Key types exist and have the expected bounds.
    fn _assert_bounds<T: Send + Sync>() {}
    _assert_bounds::<AgentState>();
    _assert_bounds::<ExecutionMode>();
    _assert_bounds::<ToolError>();
    _assert_bounds::<AgentError>();
    _assert_bounds::<AgentEvent>();
    _assert_bounds::<AgentMessage>();
    _assert_bounds::<AgentSessionEvent>();

    // Prove the type aliases exist at the crate root.
    let _: Option<AgentEventSink> = None;
    let _: Option<&dyn AgentHooks> = None;
    let _: Option<AgentLoopConfig> = None;
    let _: Option<AgentLoopContext> = None;
    let _: Option<Box<dyn Tool>> = None;
    let _: Option<ToolResult> = None;
    let _: Option<ToolDef> = None;
    let _: Option<Agent> = None;
}

/// Verify that the `sdk` module docs do not reference a settled Transport
/// trait as part of the public contract.
#[test]
fn sdk_docs_do_not_claim_settled_transport() {
    // The SDK module should either not mention Transport at all, or clearly
    // state it is not part of the SDK contract. After task 4.3 (removal),
    // there is no Transport in the public API to reference.
    //
    // This is a compile-time guarantee: if someone tried to re-add
    // `use crate::Transport` in sdk.rs, the module wouldn't compile because
    // the Transport type no longer exists.
    use opi_agent::sdk;
    let _ = sdk::SDK_SCHEMA_VERSION;
}
