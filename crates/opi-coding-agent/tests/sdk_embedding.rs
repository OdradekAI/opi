//! SDK embedding surface integration tests (task 4.2).
//!
//! Tests that opi-coding-agent re-uses SDK types from opi-agent::sdk
//! without duplicating protocol logic. Verifies type compatibility
//! between RPC mode and the SDK surface.

use opi_agent::sdk::{SDK_SCHEMA_VERSION, SdkCommand, SdkResponse};
use opi_agent::session_event::CompactionReason;
use opi_ai::test_support::MockProvider;
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::harness::CodingHarness;
use opi_coding_agent::rpc::{RPC_SCHEMA_VERSION, RpcCommand};

// ---------------------------------------------------------------------------
// Type compatibility: rpc.rs uses sdk types
// ---------------------------------------------------------------------------

#[test]
fn rpc_command_is_sdk_command() {
    // Verify that RpcCommand is a re-export of SdkCommand
    // by parsing the same JSON through both.
    let json = r#"{"type":"prompt","message":"hello","id":"1"}"#;

    let sdk_cmd: SdkCommand = serde_json::from_str(json).unwrap();
    let rpc_cmd: RpcCommand = serde_json::from_str(json).unwrap();

    assert_eq!(sdk_cmd.id(), rpc_cmd.id());
    assert_eq!(sdk_cmd.command_name(), rpc_cmd.command_name());
}

#[test]
fn rpc_schema_version_matches_sdk() {
    assert_eq!(
        SDK_SCHEMA_VERSION, RPC_SCHEMA_VERSION,
        "RPC and SDK must share the same schema version"
    );
}

#[test]
fn sdk_response_compatible_with_rpc_format() {
    // Build an SDK response and verify it matches the RPC wire format
    let resp = SdkResponse::success(Some("1"), "prompt");
    let val = serde_json::to_value(&resp).unwrap();
    assert_eq!(val["type"], "response");
    assert_eq!(val["success"], true);
    assert_eq!(val["id"], "1");
    assert_eq!(val["command"], "prompt");
}

#[test]
fn sdk_error_response_compatible_with_rpc_format() {
    let resp = SdkResponse::error(None, "parse", "invalid json");
    let val = serde_json::to_value(&resp).unwrap();
    assert_eq!(val["type"], "response");
    assert_eq!(val["success"], false);
    assert_eq!(val["error"], "invalid json");
}

#[test]
fn all_rpc_commands_parse_as_sdk() {
    let commands = vec![
        r#"{"type":"prompt","message":"hi"}"#,
        r#"{"type":"continue","message":"more"}"#,
        r#"{"type":"steer","message":"s"}"#,
        r#"{"type":"follow_up","message":"f"}"#,
        r#"{"type":"abort"}"#,
        r#"{"type":"set_model","model":"m"}"#,
        r#"{"type":"set_thinking_level","level":"low"}"#,
        r#"{"type":"compact"}"#,
        r#"{"type":"session_info"}"#,
        r#"{"type":"quit"}"#,
    ];
    for json in commands {
        let sdk: SdkCommand = serde_json::from_str(json).unwrap();
        let rpc: RpcCommand = serde_json::from_str(json).unwrap();
        assert_eq!(sdk.command_name(), rpc.command_name());
    }
}

// ---------------------------------------------------------------------------
// Phase 8 (task 8.5): harness command-state contract.
//
// The RPC runner delegates every non-run command to CodingHarness methods that
// return structured Results. These tests pin those harness-layer contracts
// (capability validation, extension dispatch, compaction no-op) directly,
// independent of the RPC wire. They construct a CodingHarness, which creates a
// session in OPI_SESSIONS_DIR, so each test isolates the dir under a serial
// lock (test binaries are separate processes; a per-file mutex suffices).
// ---------------------------------------------------------------------------

static SESSION_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn session_lock() -> std::sync::MutexGuard<'static, ()> {
    SESSION_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn isolate_sessions_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("sessions tempdir");
    // SAFETY: test-only env var mutation serialized by SESSION_TEST_LOCK; no
    // other thread reads this var concurrently during the test.
    unsafe {
        std::env::set_var("OPI_SESSIONS_DIR", dir.path());
    }
    dir
}

fn clear_sessions_dir() {
    // SAFETY: test-only env var mutation serialized by SESSION_TEST_LOCK.
    unsafe {
        std::env::remove_var("OPI_SESSIONS_DIR");
    }
}

fn mock_harness() -> CodingHarness {
    let provider = MockProvider::new("mock", Vec::new());
    CodingHarness::new(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().expect("cwd"),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
}

/// `set_model_validated` accepts a valid same-provider model and returns
/// structured error strings for cross-provider, malformed, and unknown specs.
#[test]
fn phase8_harness_command_contract_set_model_validated_returns_structured_errors() {
    let _lock = session_lock();
    let _dir = isolate_sessions_dir();
    let mut harness = mock_harness();

    // Valid same-provider model resolves and returns the active model.
    assert_eq!(
        harness
            .set_model_validated("mock:mock-model".into())
            .unwrap(),
        "mock:mock-model"
    );

    // A cross-provider switch to an UNREGISTERED provider still fails, now at
    // route resolution (unknown model/provider) rather than "cannot switch".
    let err = harness
        .set_model_validated("openai:gpt-4o".into())
        .expect_err("unregistered cross-provider model must be rejected");
    assert!(err.contains("unknown model"), "cross-provider error: {err}");

    // Malformed spec is rejected.
    let err = harness
        .set_model_validated("not-a-spec".into())
        .expect_err("malformed spec must be rejected");
    assert!(
        err.contains("invalid model spec"),
        "invalid spec error: {err}"
    );

    // Unknown same-provider model is rejected.
    let err = harness
        .set_model_validated("mock:does-not-exist".into())
        .expect_err("unknown model must be rejected");
    assert!(err.contains("unknown model"), "unknown model error: {err}");

    clear_sessions_dir();
}

/// `set_thinking_level` accepts "off" on a non-thinking model and returns
/// structured error strings for an invalid level and an unsupported model.
#[test]
fn phase8_harness_command_contract_set_thinking_level_returns_structured_errors() {
    let _lock = session_lock();
    let _dir = isolate_sessions_dir();
    let mut harness = mock_harness();

    // "off" succeeds even on a non-thinking model.
    let state = harness.set_thinking_level("off").expect("off succeeds");
    assert_eq!(state.level, "off");
    assert!(!state.enabled);

    // Invalid level is rejected.
    let err = harness
        .set_thinking_level("maximum")
        .err()
        .expect("invalid level must be rejected");
    assert!(
        err.contains("invalid thinking level"),
        "invalid level error: {err}"
    );

    // The mock model does not support thinking; "low" is rejected.
    let err = harness
        .set_thinking_level("low")
        .err()
        .expect("non-thinking model must reject a thinking level");
    assert!(
        err.contains("does not support thinking"),
        "non-thinking model error: {err}"
    );

    clear_sessions_dir();
}

/// `dispatch_extension_command` with no registered extension returns Ok(None),
/// which the RPC layer surfaces as the documented "extension command not
/// handled" error — no panic, no partial mutation.
#[tokio::test]
#[allow(clippy::await_holding_lock)] // SESSION_TEST_LOCK serializes OPI_SESSIONS_DIR mutation; it is not re-acquired within the awaited dispatch.
async fn phase8_harness_command_contract_extension_dispatch_unhandled_returns_none() {
    let _lock = session_lock();
    let _dir = isolate_sessions_dir();
    let mut harness = mock_harness();

    let result = harness
        .dispatch_extension_command("anything", Some("x"), serde_json::json!({}))
        .await;
    assert!(
        result.is_ok(),
        "dispatch without a registry is Ok: {result:?}"
    );
    assert_eq!(
        result.unwrap(),
        None,
        "unhandled extension command yields Ok(None)"
    );

    clear_sessions_dir();
}

/// `compact_with_diagnostic` on a fresh session with no turns returns Ok with
/// no compaction result plus a structured diagnostic (the documented no-op /
/// failure response), not an error.
#[test]
fn phase8_harness_command_contract_compact_noop_returns_structured_diagnostic() {
    let _lock = session_lock();
    let _dir = isolate_sessions_dir();
    let mut harness = mock_harness();

    let result = harness.compact_with_diagnostic(CompactionReason::Manual);
    assert!(result.is_ok(), "compaction no-op is Ok: {result:?}");
    let (compacted, diagnostic) = result.unwrap();
    assert!(
        compacted.is_none(),
        "nothing-to-compact yields no compaction result"
    );
    assert!(
        !diagnostic.code.is_empty(),
        "compaction diagnostic must carry a structured code"
    );

    clear_sessions_dir();
}
