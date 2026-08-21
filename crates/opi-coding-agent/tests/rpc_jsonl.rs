//! RPC JSONL mode behavioral tests (task 4.1).
//!
//! Tests cover:
//! - Command parsing (valid/invalid JSON)
//! - Response format (success/error/data)
//! - ID correlation
//! - Malformed frame rejection
//! - Prompt/continue/abort with mock provider
//! - Session info, set_model, compact commands
//! - stdout framing (one JSON object per line)
//! - startup diagnostics (rpc_ready header and session_info)
//! - Exit behavior
//! - Interleaved async events
//! - Compatibility with existing JSON mode event semantics

mod common;

use std::future::Future;
use std::io::Write;
use std::pin::Pin;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::Duration;

use futures_util::stream;
use opi_agent::diagnostic::{Diagnostic, SOURCE_PACKAGE, Severity, code};
use opi_agent::extension::{Extension, ExtensionCommand, ExtensionError, ExtensionRegistry};
use opi_ai::provider::{
    EventStream, ModelInfo, Provider, ProviderError, ProviderErrorSummary, Request,
};
use opi_ai::registry::ModelCapabilities;
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use opi_ai::test_support::{MockProvider, MockResponse, base_assistant, text_response};
use opi_coding_agent::adapter_extension::ProcessAdapter;
use opi_coding_agent::adapter_host::{AdapterHost, AdapterProcessConfig};
use opi_coding_agent::config::{ExecutionStrategy, OpiConfig, PermissionDecision};
use opi_coding_agent::policy::ToolSelection;
use opi_coding_agent::rpc::{RPC_SCHEMA_VERSION, RpcCommand, RpcRunner};
use opi_coding_agent::runtime_packages::RuntimePackageStartup;

// ---------------------------------------------------------------------------
// Command parsing
// ---------------------------------------------------------------------------

#[test]
fn rpc_parse_prompt_command() {
    let cmd: RpcCommand = serde_json::from_str(r#"{"type":"prompt","message":"hello"}"#).unwrap();
    assert!(matches!(cmd, RpcCommand::prompt { .. }));
    assert_eq!(cmd.command_name(), "prompt");
    assert!(cmd.id().is_none());
}

#[test]
fn rpc_parse_prompt_with_id() {
    let cmd: RpcCommand =
        serde_json::from_str(r#"{"type":"prompt","id":"req-1","message":"hello"}"#).unwrap();
    assert_eq!(cmd.id(), Some("req-1"));
}

#[test]
fn rpc_parse_continue_command() {
    let cmd: RpcCommand =
        serde_json::from_str(r#"{"type":"continue","message":"more text"}"#).unwrap();
    assert!(matches!(cmd, RpcCommand::continue_ { .. }));
    assert_eq!(cmd.command_name(), "continue");
}

#[test]
fn rpc_parse_abort_command() {
    let cmd: RpcCommand = serde_json::from_str(r#"{"type":"abort"}"#).unwrap();
    assert!(matches!(cmd, RpcCommand::abort { .. }));
    assert!(cmd.id().is_none());
}

#[test]
fn rpc_parse_abort_with_id() {
    let cmd: RpcCommand = serde_json::from_str(r#"{"type":"abort","id":"a1"}"#).unwrap();
    assert_eq!(cmd.id(), Some("a1"));
}

#[test]
fn rpc_parse_set_model_command() {
    let cmd: RpcCommand =
        serde_json::from_str(r#"{"type":"set_model","model":"anthropic:claude-sonnet-4"}"#)
            .unwrap();
    assert!(matches!(cmd, RpcCommand::set_model { .. }));
}

#[test]
fn rpc_parse_set_thinking_level_command() {
    let cmd: RpcCommand =
        serde_json::from_str(r#"{"type":"set_thinking_level","level":"high"}"#).unwrap();
    assert!(matches!(cmd, RpcCommand::set_thinking_level { .. }));
}

#[test]
fn rpc_parse_compact_command() {
    let cmd: RpcCommand = serde_json::from_str(r#"{"type":"compact"}"#).unwrap();
    assert!(matches!(cmd, RpcCommand::compact { .. }));
}

#[test]
fn rpc_parse_session_info_command() {
    let cmd: RpcCommand = serde_json::from_str(r#"{"type":"session_info"}"#).unwrap();
    assert!(matches!(cmd, RpcCommand::session_info { .. }));
}

#[test]
fn rpc_parse_quit_command() {
    let cmd: RpcCommand = serde_json::from_str(r#"{"type":"quit"}"#).unwrap();
    assert!(cmd.is_quit());
}

#[test]
fn rpc_parse_steer_command() {
    let cmd: RpcCommand = serde_json::from_str(r#"{"type":"steer","message":"do this"}"#).unwrap();
    assert!(matches!(cmd, RpcCommand::steer { .. }));
}

#[test]
fn rpc_parse_follow_up_command() {
    let cmd: RpcCommand =
        serde_json::from_str(r#"{"type":"follow_up","message":"then that"}"#).unwrap();
    assert!(matches!(cmd, RpcCommand::follow_up { .. }));
}

#[test]
fn rpc_parse_extension_command() {
    let cmd: RpcCommand = serde_json::from_str(
        r#"{"type":"extension_command","id":"ext-1","name":"echo/upper","args":{"text":"hello"}}"#,
    )
    .unwrap();

    assert!(matches!(cmd, RpcCommand::extension_command { .. }));
    assert_eq!(cmd.id(), Some("ext-1"));
    assert_eq!(cmd.command_name(), "extension_command");
}

#[test]
fn rpc_parse_malformed_json_returns_error() {
    let result = serde_json::from_str::<RpcCommand>("not json at all");
    assert!(result.is_err());
}

#[test]
fn rpc_parse_unknown_type_returns_error() {
    let result = serde_json::from_str::<RpcCommand>(r#"{"type":"unknown_command"}"#);
    assert!(result.is_err());
}

#[test]
fn rpc_parse_missing_required_field_returns_error() {
    // prompt without message
    let result = serde_json::from_str::<RpcCommand>(r#"{"type":"prompt"}"#);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Subprocess tests (spawn the actual opi binary)
// ---------------------------------------------------------------------------

fn test_binary_path(name: &str) -> std::path::PathBuf {
    let current = std::env::current_exe().expect("current exe path");
    let deps_dir = current.parent().expect("deps directory");
    let exact_name = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    let exact_path = deps_dir.join(exact_name);
    if exact_path.exists() {
        return exact_path;
    }

    let exe_suffix = if cfg!(windows) { ".exe" } else { "" };
    let prefix = format!("{name}-");
    let mut best: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
    if let Ok(entries) = std::fs::read_dir(deps_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with(&prefix)
                && name_str.ends_with(exe_suffix)
                && !name_str.ends_with(".d")
                && let Ok(meta) = entry.metadata()
                && let Ok(modified) = meta.modified()
                && best.as_ref().is_none_or(|(t, _)| modified > *t)
            {
                best = Some((modified, entry.path()));
            }
        }
    }

    best.map(|(_, p)| p)
        .unwrap_or_else(|| panic!("Could not find {name} binary in deps directory"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_set_model_returns_model_data() {
    let provider = MockProvider::new_with_models(
        "mock",
        vec![
            rpc_model_info("mock-model", true),
            rpc_model_info("next-model", true),
        ],
        Vec::new(),
    );
    let (command_tx, mut output_rx, task) = custom_provider_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::set_model {
            id: Some("set-1".into()),
            model: "mock:next-model".into(),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "set_model").await;
    assert_eq!(resp["type"], "response");
    assert_eq!(resp["command"], "set_model");
    assert_eq!(resp["id"], "set-1");
    assert_eq!(resp["success"], true);
    assert_eq!(resp["data"]["model"], "mock:next-model");

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_set_model_accepts_model_ids_containing_colons() {
    let provider = MockProvider::new_with_models(
        "bedrock",
        vec![
            rpc_model_info("anthropic.claude-sonnet-4-20250514-v2:0", true),
            rpc_model_info("anthropic.claude-haiku-4-20250514-v1:0", true),
        ],
        Vec::new(),
    );
    let (command_tx, mut output_rx, task) = custom_provider_runner_with_model(
        provider,
        "bedrock:anthropic.claude-sonnet-4-20250514-v2:0",
    );

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::set_model {
            id: Some("set-colon".into()),
            model: "bedrock:anthropic.claude-haiku-4-20250514-v1:0".into(),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "set_model").await;
    assert_eq!(resp["success"], true);
    assert_eq!(
        resp["data"]["model"],
        "bedrock:anthropic.claude-haiku-4-20250514-v1:0"
    );

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_set_model_rejects_invalid_spec() {
    let provider = MockProvider::new("mock", Vec::new());
    let (command_tx, mut output_rx, task) = custom_provider_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::set_model {
            id: Some("set-1".into()),
            model: "not-a-model-spec".into(),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "set_model").await;
    assert_eq!(resp["success"], false);
    // A bare model resolves only through the unique-dispatchable-route proof,
    // so a no-match input returns typed remediation rather than a parse error.
    assert_eq!(
        resp["error"],
        "bare model 'not-a-model-spec' matches no dispatchable route"
    );

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_set_model_rejects_empty_provider_and_empty_model_specs() {
    let provider = MockProvider::new("mock", Vec::new());
    let (command_tx, mut output_rx, task) = custom_provider_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::set_model {
            id: Some("empty-provider".into()),
            model: ":mock-model".into(),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "set_model").await;
    assert_eq!(resp["success"], false);
    assert_eq!(
        resp["error"],
        "invalid model spec: spec must be 'provider:model', got: \":mock-model\""
    );

    command_tx
        .send(RpcCommand::set_model {
            id: Some("empty-model".into()),
            model: "mock:".into(),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "set_model").await;
    assert_eq!(resp["success"], false);
    assert_eq!(
        resp["error"],
        "invalid model spec: spec must be 'provider:model', got: \"mock:\""
    );

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_set_model_rejects_unknown_same_provider_model() {
    let provider = MockProvider::new("mock", Vec::new());
    let (command_tx, mut output_rx, task) = custom_provider_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::set_model {
            id: Some("set-1".into()),
            model: "mock:bad-model".into(),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "set_model").await;
    assert_eq!(resp["success"], false);
    assert_eq!(
        resp["error"],
        "unknown model 'bad-model' for provider 'mock'"
    );

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[test]
#[should_panic(expected = "startup model selection must resolve to a dispatchable route")]
fn rpc_startup_rejects_unadvertised_current_model() {
    let provider = MockProvider::new_with_models(
        "mock",
        vec![rpc_model_info("known-model", true)],
        Vec::new(),
    );
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let _runner = RpcRunner::new(
        Box::new(provider),
        "mock:legacy-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        ToolSelection::Disabled,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_set_model_updates_subsequent_provider_request() {
    let provider = MockProvider::new_with_models(
        "mock",
        vec![
            rpc_model_info("mock-model", true),
            rpc_model_info("next-model", true),
        ],
        vec![text_response("ok")],
    );
    let call_log = provider.call_log_handle();
    let (command_tx, mut output_rx, task) = custom_provider_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::set_model {
            id: Some("set-1".into()),
            model: "mock:next-model".into(),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "set_model").await;
    assert_eq!(resp["success"], true);

    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-1".into()),
            message: "use the updated model".into(),
        })
        .unwrap();
    let prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(prompt["success"], true);
    recv_until_agent_end(&mut output_rx).await;

    {
        let calls = call_log.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].model, "mock:next-model");
    }

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_set_model_rejects_while_running_without_mutating_model() {
    let provider = HeldRequestProvider::new();
    let call_log = provider.call_log();
    let release = provider.release_handle();
    let (command_tx, mut output_rx, task) = custom_provider_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-1".into()),
            message: "start".into(),
        })
        .unwrap();
    let prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(prompt["success"], true);
    loop {
        let line = recv_rpc_line(&mut output_rx).await;
        if line["type"] == "MessageStart" {
            break;
        }
    }

    command_tx
        .send(RpcCommand::set_model {
            id: Some("set-while-running".into()),
            model: "mock:next-model".into(),
        })
        .unwrap();
    let rejected = recv_response(&mut output_rx, "set_model").await;
    assert_eq!(rejected["success"], false);
    assert_eq!(
        rejected["error"],
        "cannot change model while agent is running"
    );
    {
        let calls = call_log.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].model, "mock:mock-model");
    }

    release.notify_one();
    recv_until_agent_end(&mut output_rx).await;

    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-2".into()),
            message: "model should still be unchanged".into(),
        })
        .unwrap();
    let prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(prompt["success"], true);
    loop {
        let line = recv_rpc_line(&mut output_rx).await;
        if line["type"] == "MessageStart" {
            break;
        }
    }
    {
        let calls = call_log.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[1].model, "mock:mock-model");
    }

    release.notify_one();
    recv_until_agent_end(&mut output_rx).await;

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_set_model_revalidates_existing_thinking_before_switching() {
    let provider = MockProvider::new_with_models(
        "mock",
        vec![
            rpc_model_info_with_max_output("large-model", true, 100_000),
            rpc_model_info_with_max_output("small-model", true, 20_000),
        ],
        vec![text_response("still large")],
    );
    let call_log = provider.call_log_handle();
    let (command_tx, mut output_rx, task) =
        custom_provider_runner_with_model(provider, "mock:large-model");

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::set_thinking_level {
            id: Some("think-high".into()),
            level: "high".into(),
        })
        .unwrap();
    let thinking = recv_response(&mut output_rx, "set_thinking_level").await;
    assert_eq!(thinking["success"], true);
    assert_eq!(thinking["data"]["budget_tokens"], 20_000);

    command_tx
        .send(RpcCommand::set_model {
            id: Some("set-small".into()),
            model: "mock:small-model".into(),
        })
        .unwrap();
    let rejected = recv_response(&mut output_rx, "set_model").await;
    assert_eq!(rejected["success"], false);
    assert_eq!(
        rejected["error"],
        "thinking budget 20000 requires max_tokens 20001, exceeding max output tokens 20000 for model 'small-model'"
    );

    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-1".into()),
            message: "model should remain large".into(),
        })
        .unwrap();
    let prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(prompt["success"], true);
    recv_until_agent_end(&mut output_rx).await;

    {
        let calls = call_log.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].model, "mock:large-model");
        assert_eq!(calls[0].thinking.budget_tokens, Some(20_000));
        assert_eq!(calls[0].max_tokens, Some(20_001));
    }

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_set_model_rejects_switch_to_non_thinking_model_with_active_thinking() {
    let provider = MockProvider::new_with_models(
        "mock",
        vec![
            rpc_model_info("thinking-model", true),
            rpc_model_info("plain-model", false),
        ],
        vec![text_response("still thinking")],
    );
    let call_log = provider.call_log_handle();
    let (command_tx, mut output_rx, task) =
        custom_provider_runner_with_model(provider, "mock:thinking-model");

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::set_thinking_level {
            id: Some("think-low".into()),
            level: "low".into(),
        })
        .unwrap();
    let thinking = recv_response(&mut output_rx, "set_thinking_level").await;
    assert_eq!(thinking["success"], true);
    assert_eq!(thinking["data"]["budget_tokens"], 2048);

    command_tx
        .send(RpcCommand::set_model {
            id: Some("set-plain".into()),
            model: "mock:plain-model".into(),
        })
        .unwrap();
    let rejected = recv_response(&mut output_rx, "set_model").await;
    assert_eq!(rejected["success"], false);
    assert_eq!(
        rejected["error"],
        "model 'plain-model' does not support thinking"
    );

    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-1".into()),
            message: "model should remain thinking capable".into(),
        })
        .unwrap();
    let prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(prompt["success"], true);
    recv_until_agent_end(&mut output_rx).await;

    {
        let calls = call_log.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].model, "mock:thinking-model");
        assert!(calls[0].thinking.enabled);
        assert_eq!(calls[0].thinking.budget_tokens, Some(2048));
        assert_eq!(calls[0].max_tokens, Some(2049));
    }

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_set_thinking_level_changes_runtime_config() {
    let provider = MockProvider::new_with_models(
        "mock",
        vec![rpc_model_info("mock-model", true)],
        vec![text_response("ok")],
    );
    let call_log = provider.call_log_handle();
    let (command_tx, mut output_rx, task) = custom_provider_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::set_thinking_level {
            id: Some("think-1".into()),
            level: "low".into(),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "set_thinking_level").await;
    assert_eq!(resp["success"], true);
    assert_eq!(resp["data"]["level"], "low");
    assert_eq!(resp["data"]["enabled"], true);
    assert_eq!(resp["data"]["budget_tokens"], 2048);

    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-1".into()),
            message: "use low thinking".into(),
        })
        .unwrap();
    let prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(prompt["success"], true);
    recv_until_agent_end(&mut output_rx).await;

    {
        let calls = call_log.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].thinking.enabled);
        assert_eq!(calls[0].thinking.budget_tokens, Some(2048));
    }

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_set_thinking_level_off_medium_high_change_runtime_config() {
    let mut config = OpiConfig::default();
    config.thinking.enabled = true;
    config.thinking.budget_tokens = 12_345;

    let provider = MockProvider::new_with_models(
        "mock",
        vec![rpc_model_info("mock-model", true)],
        vec![
            text_response("off"),
            text_response("medium"),
            text_response("high"),
        ],
    );
    let call_log = provider.call_log_handle();
    let (command_tx, mut output_rx, task) = custom_provider_runner_with_config(provider, config);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::set_thinking_level {
            id: Some("think-off".into()),
            level: "off".into(),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "set_thinking_level").await;
    assert_eq!(resp["success"], true);
    assert_eq!(resp["data"]["level"], "off");
    assert_eq!(resp["data"]["enabled"], false);
    assert!(resp["data"]["budget_tokens"].is_null());
    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-off".into()),
            message: "off".into(),
        })
        .unwrap();
    let prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(prompt["success"], true);
    recv_until_agent_end(&mut output_rx).await;

    command_tx
        .send(RpcCommand::set_thinking_level {
            id: Some("think-medium".into()),
            level: "medium".into(),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "set_thinking_level").await;
    assert_eq!(resp["success"], true);
    assert_eq!(resp["data"]["level"], "medium");
    assert_eq!(resp["data"]["enabled"], true);
    assert_eq!(resp["data"]["budget_tokens"], 12_345);
    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-medium".into()),
            message: "medium".into(),
        })
        .unwrap();
    let prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(prompt["success"], true);
    recv_until_agent_end(&mut output_rx).await;

    command_tx
        .send(RpcCommand::set_thinking_level {
            id: Some("think-high".into()),
            level: "high".into(),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "set_thinking_level").await;
    assert_eq!(resp["success"], true);
    assert_eq!(resp["data"]["level"], "high");
    assert_eq!(resp["data"]["enabled"], true);
    assert_eq!(resp["data"]["budget_tokens"], 20_000);
    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-high".into()),
            message: "high".into(),
        })
        .unwrap();
    let prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(prompt["success"], true);
    recv_until_agent_end(&mut output_rx).await;

    {
        let calls = call_log.lock().unwrap();
        assert_eq!(calls.len(), 3);
        assert!(!calls[0].thinking.enabled);
        assert_eq!(calls[0].thinking.budget_tokens, None);
        assert!(calls[1].thinking.enabled);
        assert_eq!(calls[1].thinking.budget_tokens, Some(12_345));
        assert!(calls[2].thinking.enabled);
        assert_eq!(calls[2].thinking.budget_tokens, Some(20_000));
    }

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_set_thinking_level_medium_and_high_keep_request_token_budget_valid() {
    let mut config = OpiConfig::default();
    config.thinking.enabled = true;
    config.thinking.budget_tokens = 12_345;

    let provider = MockProvider::new_with_models(
        "mock",
        vec![rpc_model_info("mock-model", true)],
        vec![text_response("medium"), text_response("high")],
    );
    let call_log = provider.call_log_handle();
    let (command_tx, mut output_rx, task) = custom_provider_runner_with_config(provider, config);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::set_thinking_level {
            id: Some("think-medium".into()),
            level: "medium".into(),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "set_thinking_level").await;
    assert_eq!(resp["success"], true);
    assert_eq!(resp["data"]["budget_tokens"], 12_345);
    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-medium".into()),
            message: "medium".into(),
        })
        .unwrap();
    let prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(prompt["success"], true);
    recv_until_agent_end(&mut output_rx).await;

    command_tx
        .send(RpcCommand::set_thinking_level {
            id: Some("think-high".into()),
            level: "high".into(),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "set_thinking_level").await;
    assert_eq!(resp["success"], true);
    assert_eq!(resp["data"]["budget_tokens"], 20_000);
    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-high".into()),
            message: "high".into(),
        })
        .unwrap();
    let prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(prompt["success"], true);
    recv_until_agent_end(&mut output_rx).await;

    {
        let calls = call_log.lock().unwrap();
        assert_eq!(calls.len(), 2);
        for request in calls.iter() {
            let budget = request
                .thinking
                .budget_tokens
                .expect("thinking request should carry a budget");
            let max_tokens = request
                .max_tokens
                .expect("thinking request should carry a coherent max token limit");
            assert!(
                budget < max_tokens,
                "thinking budget {budget} must be less than request max_tokens {max_tokens}"
            );
        }
        assert_eq!(calls[0].thinking.budget_tokens, Some(12_345));
        assert_eq!(calls[0].max_tokens, Some(12_346));
        assert_eq!(calls[1].thinking.budget_tokens, Some(20_000));
        assert_eq!(calls[1].max_tokens, Some(20_001));
    }

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_startup_thinking_config_sets_valid_first_request_token_budget() {
    let mut config = OpiConfig::default();
    config.thinking.enabled = true;
    config.thinking.budget_tokens = 12_345;

    let provider = MockProvider::new_with_models(
        "mock",
        vec![rpc_model_info_with_max_output("mock-model", true, 12_346)],
        vec![text_response("startup thinking")],
    );
    let call_log = provider.call_log_handle();
    let (command_tx, mut output_rx, task) = custom_provider_runner_with_config(provider, config);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-1".into()),
            message: "first request should have valid thinking config".into(),
        })
        .unwrap();
    let prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(prompt["success"], true);
    recv_until_agent_end(&mut output_rx).await;

    {
        let calls = call_log.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].thinking.enabled);
        assert_eq!(calls[0].thinking.budget_tokens, Some(12_345));
        assert_eq!(calls[0].max_tokens, Some(12_346));
    }

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_startup_thinking_config_disables_known_non_thinking_model() {
    let mut config = OpiConfig::default();
    config.thinking.enabled = true;
    config.thinking.budget_tokens = 2_048;

    let provider = MockProvider::new_with_models(
        "mock",
        vec![rpc_model_info("mock-model", false)],
        vec![text_response("startup thinking disabled")],
    );
    let call_log = provider.call_log_handle();
    let (command_tx, mut output_rx, task) = custom_provider_runner_with_config(provider, config);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-1".into()),
            message: "first request should not enable unsupported thinking".into(),
        })
        .unwrap();
    let prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(prompt["success"], true);
    recv_until_agent_end(&mut output_rx).await;

    {
        let calls = call_log.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert!(!calls[0].thinking.enabled);
        assert_eq!(calls[0].thinking.budget_tokens, None);
        assert_eq!(calls[0].max_tokens, None);
    }

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_set_thinking_level_rejects_budget_above_known_model_limit() {
    let provider = MockProvider::new_with_models(
        "mock",
        vec![rpc_model_info_with_max_output("mock-model", true, 8_192)],
        Vec::new(),
    );
    let (command_tx, mut output_rx, task) = custom_provider_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::set_thinking_level {
            id: Some("think-high".into()),
            level: "high".into(),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "set_thinking_level").await;
    assert_eq!(resp["success"], false);
    assert_eq!(
        resp["error"],
        "thinking budget 20000 requires max_tokens 20001, exceeding max output tokens 8192 for model 'mock-model'"
    );

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_set_thinking_level_rejects_known_non_thinking_model() {
    let provider = MockProvider::new_with_models(
        "mock",
        vec![rpc_model_info("mock-model", false)],
        vec![text_response("thinking stays off")],
    );
    let call_log = provider.call_log_handle();
    let (command_tx, mut output_rx, task) = custom_provider_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::set_thinking_level {
            id: Some("think-low".into()),
            level: "low".into(),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "set_thinking_level").await;
    assert_eq!(resp["success"], false);
    assert_eq!(
        resp["error"],
        "model 'mock-model' does not support thinking"
    );

    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-1".into()),
            message: "rejected thinking should not mutate runtime config".into(),
        })
        .unwrap();
    let prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(prompt["success"], true);
    recv_until_agent_end(&mut output_rx).await;

    {
        let calls = call_log.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert!(!calls[0].thinking.enabled);
        assert_eq!(calls[0].thinking.budget_tokens, None);
        assert_eq!(calls[0].max_tokens, None);
    }

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_set_thinking_level_rejects_invalid_level() {
    let provider = MockProvider::new("mock", Vec::new());
    let (command_tx, mut output_rx, task) = custom_provider_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::set_thinking_level {
            id: Some("think-1".into()),
            level: "maximum".into(),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "set_thinking_level").await;
    assert_eq!(resp["success"], false);
    assert_eq!(
        resp["error"],
        "invalid thinking level 'maximum': expected off, minimal, low, medium, high, xhigh, or max"
    );

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_set_thinking_level_rejects_while_running() {
    let provider = ControlledProvider::new();
    let (command_tx, mut output_rx, task) = rpc_test_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-1".into()),
            message: "start".into(),
        })
        .unwrap();
    let prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(prompt["success"], true);

    command_tx
        .send(RpcCommand::set_thinking_level {
            id: Some("think-1".into()),
            level: "high".into(),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "set_thinking_level").await;
    assert_eq!(resp["success"], false);
    assert_eq!(
        resp["error"],
        "cannot change thinking level while agent is running"
    );

    command_tx.send(RpcCommand::abort { id: None }).unwrap();
    let _abort = recv_response(&mut output_rx, "abort").await;
    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[test]
fn rpc_tool_selection_respects_no_tools() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let provider = MockProvider::new("mock", vec![text_response("ok")]);

    let runner = RpcRunner::new(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        ToolSelection::Disabled,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .expect("rpc runner should construct");

    let system = runner.system_prompt().expect("runner should be idle");
    assert!(
        !system.contains("Available tools:"),
        "RPC --no-tools should remove built-in tool definitions from the system prompt"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_session_info_includes_discovered_resource_metadata() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let skill_dir = workspace
        .path()
        .join(".opi")
        .join("skills")
        .join("rpc-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: rpc-skill
description: RPC visible skill.
---
Body should remain undisclosed.
"#,
    )
    .unwrap();

    let provider = MockProvider::new("mock", Vec::new());
    let runner = RpcRunner::new(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        ToolSelection::Disabled,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .expect("rpc runner should construct");

    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut runner = runner;
        runner.run_with_channels(command_rx, output_tx).await
    });

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::session_info {
            id: Some("resources-1".into()),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "session_info").await;
    assert_eq!(resp["success"], true);
    assert_eq!(resp["id"], "resources-1");

    let skills = resp["data"]["resources"]["skills"]
        .as_array()
        .expect("skills metadata should be an array");
    assert!(
        skills.iter().any(|name| name.as_str() == Some("rpc-skill")),
        "session_info resources should include workspace skill names: {skills:?}"
    );

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase13_rpc_session_info_returns_metadata() {
    //! Phase 13.4 acceptance: RPC `session_info` returns the live session
    //! metadata (name, labels, active branch, model, thinking) for a resumed
    //! session, alongside the existing model/resources/session_id fields. This
    //! is the RPC half of DoD scenario `phase13-session-name-label-commands`.

    use opi_agent::message::AgentMessage;
    use opi_agent::session::{LabelAction, SessionReader};
    use opi_ai::message::{InputContent, Message, UserMessage};
    use opi_coding_agent::harness::ResumeInfo;
    use opi_coding_agent::session_coordinator::SessionCoordinator;

    let workspace = tempfile::tempdir().expect("workspace tempdir");

    // Pre-populate a session file with a content tip + typed metadata via the
    // production coordinator (avoids hand-rolling entry fixtures).
    let mut coord = SessionCoordinator::new(
        workspace.path(),
        &workspace.path().display().to_string(),
        opi_agent::compaction::CompactionConfig::default(),
        "mock:mock-model",
    )
    .expect("coordinator");
    coord
        .on_turn_end_simple(
            &[AgentMessage::Llm(Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "seed".into(),
                }],
                timestamp_ms: 0,
            }))],
            &opi_ai::stream::Usage::default(),
        )
        .expect("seed turn");
    coord
        .append_session_info("rpc-demo".into())
        .expect("session_info");
    coord
        .append_label("bug".into(), LabelAction::Add)
        .expect("label add bug");
    coord
        .append_label("draft".into(), LabelAction::Add)
        .expect("label add draft");
    coord
        .append_label("bug".into(), LabelAction::Remove)
        .expect("label remove bug");

    let session_path = coord.session_path().to_path_buf();
    let session_id = coord.session_id().to_owned();
    let (_h, entries) = SessionReader::read_all(&session_path).expect("read back");
    drop(coord);

    let resume_info = ResumeInfo {
        path: session_path,
        session_id,
        entries,
        original_cwd: workspace.path().to_path_buf(),
        diagnostics: Vec::new(),
        recorded_model: None,
        recorded_thinking: None,
    };

    let provider = MockProvider::new("mock", Vec::new());
    let runner = RpcRunner::new_with_runtime_packages(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        ToolSelection::Disabled,
        None,
        Vec::new(),
        RuntimePackageStartup {
            extension_registry: ExtensionRegistry::new(),
            installed_packages: Vec::new(),
            diagnostics: Vec::new(),
            trust_decision: opi_coding_agent::project_trust::TrustDecision::Trusted,
        },
        Some(resume_info),
        Vec::new(),
    )
    .expect("rpc runner should construct");

    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut runner = runner;
        runner.run_with_channels(command_rx, output_tx).await
    });

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::session_info {
            id: Some("si-meta".into()),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "session_info").await;
    assert_eq!(resp["success"], true, "session_info should succeed: {resp}");
    assert_eq!(resp["id"], "si-meta");

    // Name + labels are the typed metadata persisted above.
    assert_eq!(resp["data"]["name"], "rpc-demo", "name surfaced: {resp}");
    let labels: Vec<String> = resp["data"]["labels"]
        .as_array()
        .expect("labels is an array")
        .iter()
        .map(|v| v.as_str().expect("label string").to_owned())
        .collect();
    assert_eq!(
        labels,
        vec!["draft".to_string()],
        "labels reflect Add bug + Add draft + Remove bug: {resp}"
    );

    // Active branch, model, thinking are surfaced from the live runtime state.
    assert!(
        resp["data"]["active_branch"].is_string(),
        "active_branch is surfaced: {resp}"
    );
    assert_eq!(resp["data"]["model"], "mock:mock-model");
    assert_eq!(resp["data"]["thinking"]["enabled"], false);

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase13_rpc_session_metadata_shape() {
    //! Phase 13.6 acceptance (RPC half): `session_info` exposes the stable
    //! session/tree handoff metadata Phase 14 needs -- active-branch
    //! `entry_count` and `branch_summary`, plus a `branches` array carrying
    //! `{tip, summary, entry_count, depth, active}` per branch in stable tree
    //! order. Summaries are redacted through Phase 7 `redact_text` so secrets
    //! embedded in user/assistant tip text cannot leak to embedders. No
    //! provider-auth, web/share, or package-ecosystem keys are emitted.

    use opi_agent::session::{
        LeafEntry, MessageEntry, SessionEntry, SessionHeader, SessionReader, SessionWriter,
    };
    use opi_ai::message::{AssistantContent, AssistantMessage, InputContent, Message, UserMessage};
    use opi_coding_agent::harness::ResumeInfo;

    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let sessions = tempfile::tempdir().expect("sessions tempdir");

    // A canary Anthropic-style API key (>=20 chars after the prefix) so the
    // Phase 7 SecretRedactor `sk-ant-[a-zA-Z0-9-]{20,}` pattern matches it.
    const KEY_CANARY: &str = "sk-ant-FAKE1234567890abcdefghijklmnop";

    // Build a forked session file directly through SessionWriter:
    //   m1 (user, canary) -> m2 (assistant, trunk tip)
    //   m1 (user)         -> m3 (assistant, sibling tip, REDACTED text)
    // Active tip = m3 (last Leaf). The active tip m3 carries the canary, so a
    // summary derived from its first 80 chars would leak without redaction.
    let session_id = "rpc-tree";
    let session_path = sessions.path().join(format!("{session_id}.jsonl"));
    let ts = "2026-07-06T00:00:00Z";
    {
        let header = SessionHeader::new(
            session_id.into(),
            ts.into(),
            workspace.path().display().to_string(),
            None,
        );
        let mut writer = SessionWriter::create(&session_path, header).expect("create session");

        let m1 = MessageEntry {
            id: "m1".into(),
            parent_id: None,
            timestamp: ts.into(),
            message: Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: format!("plain trunk prompt {KEY_CANARY}"),
                }],
                timestamp_ms: 0,
            }),
        };
        let m2 = MessageEntry {
            id: "m2".into(),
            parent_id: Some("m1".into()),
            timestamp: ts.into(),
            message: Message::Assistant(AssistantMessage {
                content: vec![AssistantContent::Text {
                    text: "trunk reply".into(),
                }],
                api: opi_ai::ApiKind::Anthropic,
                provider: "anthropic".into(),
                model: "claude-sonnet-4".into(),
                response_model: None,
                response_id: None,
                usage: Default::default(),
                stop_reason: opi_ai::stream::StopReason::Stop,
                error_message: None,
                timestamp_ms: 0,
            }),
        };
        let m3 = MessageEntry {
            id: "m3".into(),
            parent_id: Some("m1".into()),
            timestamp: ts.into(),
            message: Message::Assistant(AssistantMessage {
                content: vec![AssistantContent::Text {
                    text: KEY_CANARY.into(),
                }],
                api: opi_ai::ApiKind::Anthropic,
                provider: "anthropic".into(),
                model: "claude-sonnet-4".into(),
                response_model: None,
                response_id: None,
                usage: Default::default(),
                stop_reason: opi_ai::stream::StopReason::Stop,
                error_message: None,
                timestamp_ms: 0,
            }),
        };
        let leaf = LeafEntry {
            id: "leaf-1".into(),
            parent_id: Some("m3".into()),
            timestamp: ts.into(),
            entry_id: "m3".into(),
        };
        writer
            .append(&SessionEntry::Message(m1))
            .expect("append m1");
        writer
            .append(&SessionEntry::Message(m2))
            .expect("append m2");
        writer
            .append(&SessionEntry::Message(m3))
            .expect("append m3");
        writer
            .append(&SessionEntry::Leaf(leaf))
            .expect("append leaf");
    }

    let (_h, entries) = SessionReader::read_all(&session_path).expect("read back entries");

    let resume_info = ResumeInfo {
        path: session_path,
        session_id: session_id.into(),
        entries,
        original_cwd: workspace.path().to_path_buf(),
        diagnostics: Vec::new(),
        recorded_model: None,
        recorded_thinking: None,
    };

    let provider = MockProvider::new("mock", Vec::new());
    let runner = RpcRunner::new_with_runtime_packages(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        ToolSelection::Disabled,
        None,
        Vec::new(),
        RuntimePackageStartup {
            extension_registry: ExtensionRegistry::new(),
            installed_packages: Vec::new(),
            diagnostics: Vec::new(),
            trust_decision: opi_coding_agent::project_trust::TrustDecision::Trusted,
        },
        Some(resume_info),
        Vec::new(),
    )
    .expect("rpc runner should construct");

    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut runner = runner;
        runner.run_with_channels(command_rx, output_tx).await
    });

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::session_info {
            id: Some("si-shape".into()),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "session_info").await;
    assert_eq!(resp["success"], true, "session_info should succeed: {resp}");
    assert_eq!(resp["id"], "si-shape");

    // Active-branch scalars: tip id, entry count, and redacted summary.
    assert_eq!(
        resp["data"]["active_branch"], "m3",
        "active tip is m3 (last Leaf): {resp}"
    );
    assert_eq!(
        resp["data"]["entry_count"], 1,
        "SessionTree active segment m1->m3 owns 1 entry (m3): {resp}"
    );
    assert_eq!(
        resp["data"]["branch_summary"], "[REDACTED]",
        "active tip m3 carries the canary; summary must be redacted: {resp}"
    );

    // branches[] array, stable tree order, exactly one active, tip matches
    // active_branch. SessionTree represents a fork-at-root as three segments:
    // the trunk stub (m1, depth 0) plus each child branch (m2, m3).
    let branches = resp["data"]["branches"]
        .as_array()
        .expect("branches is an array");
    assert_eq!(branches.len(), 3, "three branch segments in tree: {resp}");
    let tips: Vec<&str> = branches
        .iter()
        .map(|b| b["tip"].as_str().expect("tip string"))
        .collect();
    assert_eq!(tips, vec!["m1", "m2", "m3"], "stable tree order: {resp}");
    for b in branches {
        assert!(b.get("tip").is_some(), "branch has tip: {b}");
        assert!(b.get("summary").is_some(), "branch has summary: {b}");
        assert!(
            b.get("entry_count").is_some(),
            "branch has entry_count: {b}"
        );
        assert!(b.get("depth").is_some(), "branch has depth: {b}");
        assert!(b.get("active").is_some(), "branch has active: {b}");
    }
    let active_count = branches.iter().filter(|b| b["active"] == true).count();
    assert_eq!(active_count, 1, "exactly one active branch: {resp}");
    let active_obj = branches
        .iter()
        .find(|b| b["active"] == true)
        .expect("active branch present");
    assert_eq!(active_obj["tip"], "m3", "active branch tip matches: {resp}");
    assert_eq!(
        active_obj["entry_count"], 1,
        "active branch entry_count matches scalar: {resp}"
    );

    // resources.diagnostics is wired through (redaction itself is pinned by
    // Phase 7's diagnostic-serializer tests).
    assert!(
        resp["data"]["resources"]["diagnostics"].is_array(),
        "diagnostics array surfaced: {resp}"
    );

    // Redaction: the canary planted in m3's tip text must not leak through
    // any of the new summary/branch fields.
    let serialized = resp.to_string();
    assert!(
        !serialized.contains(KEY_CANARY),
        "API key must be redacted from session_info handoff metadata: {serialized}"
    );

    // Forbidden Phase 13 non-goal keys absent from the handoff payload.
    for forbidden in [
        "auth_token",
        "share_url",
        "publish",
        "sync_url",
        "cloud",
        "package_market",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "forbidden non-goal key '{forbidden}' must not appear: {serialized}"
        );
    }

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase13_rpc_session_info_reports_tree_recovery_diagnostics() {
    use opi_agent::session::{
        MessageEntry, SessionEntry, SessionHeader, SessionReader, SessionWriter,
    };
    use opi_ai::message::{InputContent, Message, UserMessage};
    use opi_coding_agent::harness::ResumeInfo;

    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let sessions = tempfile::tempdir().expect("sessions tempdir");
    let session_id = "rpc-tree-recovery";
    let session_path = sessions.path().join(format!("{session_id}.jsonl"));
    let ts = "2026-07-06T00:00:00Z";

    {
        let header = SessionHeader::new(
            session_id.into(),
            ts.into(),
            workspace.path().display().to_string(),
            None,
        );
        let mut writer = SessionWriter::create(&session_path, header).expect("create session");
        writer
            .append(&SessionEntry::Message(MessageEntry {
                id: "m1".into(),
                parent_id: None,
                timestamp: ts.into(),
                message: Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "seed".into(),
                    }],
                    timestamp_ms: 0,
                }),
            }))
            .expect("append message");
    }
    {
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&session_path)
            .expect("open session for corrupt fixture");
        writeln!(file, "{{not valid json}}").expect("write corrupt line");
    }

    let (_header, entries, recovery) =
        SessionReader::read_with_recovery(&session_path).expect("read with recovery");
    assert!(
        recovery.corrupt_count > 0,
        "fixture must carry corrupt recovery"
    );
    let resume_info = ResumeInfo {
        path: session_path,
        session_id: session_id.into(),
        entries,
        original_cwd: workspace.path().to_path_buf(),
        diagnostics: recovery.diagnostics(),
        recorded_model: None,
        recorded_thinking: None,
    };

    let provider = MockProvider::new("mock", Vec::new());
    let runner = RpcRunner::new_with_runtime_packages(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        ToolSelection::Disabled,
        None,
        Vec::new(),
        RuntimePackageStartup {
            extension_registry: ExtensionRegistry::new(),
            installed_packages: Vec::new(),
            diagnostics: Vec::new(),
            trust_decision: opi_coding_agent::project_trust::TrustDecision::Trusted,
        },
        Some(resume_info),
        Vec::new(),
    )
    .expect("rpc runner should construct");

    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut runner = runner;
        runner.run_with_channels(command_rx, output_tx).await
    });

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::session_info {
            id: Some("si-tree-recovery".into()),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "session_info").await;
    assert_eq!(resp["success"], true, "session_info should succeed: {resp}");

    let recovery = resp["data"]["tree_recovery"]
        .as_array()
        .expect("tree_recovery diagnostics are surfaced");
    assert!(
        recovery
            .iter()
            .any(|diag| diag["code"] == code::CODE_SESSION_CORRUPT_ENTRIES),
        "tree_recovery should include corrupt-entry diagnostic: {resp}"
    );

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase13_rpc_session_info_reports_tree_read_error() {
    use opi_agent::session::{
        MessageEntry, SessionEntry, SessionHeader, SessionReader, SessionWriter,
    };
    use opi_ai::message::{InputContent, Message, UserMessage};
    use opi_coding_agent::harness::ResumeInfo;

    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let sessions = tempfile::tempdir().expect("sessions tempdir");
    let session_id = "rpc-tree-read-error";
    let session_path = sessions.path().join(format!("{session_id}.jsonl"));
    let ts = "2026-07-06T00:00:00Z";

    {
        let header = SessionHeader::new(
            session_id.into(),
            ts.into(),
            workspace.path().display().to_string(),
            None,
        );
        let mut writer = SessionWriter::create(&session_path, header).expect("create session");
        writer
            .append(&SessionEntry::Message(MessageEntry {
                id: "m1".into(),
                parent_id: None,
                timestamp: ts.into(),
                message: Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "seed".into(),
                    }],
                    timestamp_ms: 0,
                }),
            }))
            .expect("append message");
    }

    let (_header, entries) = SessionReader::read_all(&session_path).expect("read valid session");
    let resume_info = ResumeInfo {
        path: session_path.clone(),
        session_id: session_id.into(),
        entries,
        original_cwd: workspace.path().to_path_buf(),
        diagnostics: Vec::new(),
        recorded_model: None,
        recorded_thinking: None,
    };

    let provider = MockProvider::new("mock", Vec::new());
    let runner = RpcRunner::new_with_runtime_packages(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        ToolSelection::Disabled,
        None,
        Vec::new(),
        RuntimePackageStartup {
            extension_registry: ExtensionRegistry::new(),
            installed_packages: Vec::new(),
            diagnostics: Vec::new(),
            trust_decision: opi_coding_agent::project_trust::TrustDecision::Trusted,
        },
        Some(resume_info),
        Vec::new(),
    )
    .expect("rpc runner should construct");

    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut runner = runner;
        runner.run_with_channels(command_rx, output_tx).await
    });

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    std::fs::write(&session_path, "not a valid session header\n")
        .expect("corrupt live session file");

    command_tx
        .send(RpcCommand::session_info {
            id: Some("si-tree-read-error".into()),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "session_info").await;
    assert_eq!(resp["success"], true, "session_info should succeed: {resp}");
    // The tree read failure is still surfaced as a structured field, but the
    // message is redacted like its sibling fields: the raw session location
    // must not cross into the RPC response.
    assert!(
        resp["data"]["tree_read_error"]
            .as_str()
            .is_some_and(|m| !m.is_empty()),
        "tree read failure should be surfaced without failing session_info: {resp}"
    );
    let resp_serialized = serde_json::to_string(&resp).unwrap();
    assert!(
        !resp_serialized.contains(&session_path.display().to_string()),
        "tree_read_error must not leak the raw session path: {resp_serialized}"
    );
    assert!(
        !resp_serialized.contains(&format!("{session_id}.jsonl")),
        "tree_read_error must not leak the session file name: {resp_serialized}"
    );

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_extension_command_dispatches_to_registry_with_correlated_response_id() {
    struct RpcCommandExtension;

    impl Extension for RpcCommandExtension {
        fn name(&self) -> &str {
            "rpc-command-extension"
        }

        fn on_command(
            &self,
            command: &ExtensionCommand,
        ) -> Pin<Box<dyn Future<Output = Result<Option<serde_json::Value>, ExtensionError>> + Send>>
        {
            let command = command.clone();
            Box::pin(async move {
                if command.name != "test/echo" {
                    return Ok(None);
                }
                Ok(Some(serde_json::json!({
                    "handled": command.name,
                    "args": command.args,
                })))
            })
        }
    }

    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(RpcCommandExtension)).unwrap();
    let (command_tx, mut output_rx, task) = custom_provider_runner_with_extension_registry(
        MockProvider::new("mock", Vec::new()),
        registry,
    );

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::extension_command {
            id: Some("ext-42".into()),
            name: "test/echo".into(),
            args: serde_json::json!({ "value": 7 }),
        })
        .unwrap();
    let response = recv_response(&mut output_rx, "extension_command").await;

    assert_eq!(response["success"], true);
    assert_eq!(response["id"], "ext-42");
    assert_eq!(response["data"]["handled"], "test/echo");
    assert_eq!(response["data"]["args"]["value"], 7);

    command_tx
        .send(RpcCommand::extension_command {
            id: Some("ext-missing".into()),
            name: "missing/command".into(),
            args: serde_json::json!({}),
        })
        .unwrap();
    let missing = recv_response(&mut output_rx, "extension_command").await;

    assert_eq!(missing["success"], false);
    assert_eq!(missing["id"], "ext-missing");
    assert_eq!(
        missing["error"],
        "extension command not handled: missing/command"
    );

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

fn rpc_model_info(id: &str, supports_thinking: bool) -> ModelInfo {
    rpc_model_info_with_max_output(id, supports_thinking, 100_000)
}

fn rpc_model_info_with_max_output(
    id: &str,
    supports_thinking: bool,
    max_output_tokens: u64,
) -> ModelInfo {
    ModelInfo::new(
        id,
        id,
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(100_000, max_output_tokens)
            .with_images(true)
            .with_streaming(true)
            .with_thinking(supports_thinking),
    )
}
#[derive(Clone)]
struct BlockingCleanupProvider {
    cleanup_finished: Arc<AtomicBool>,
}

impl BlockingCleanupProvider {
    fn new(cleanup_finished: Arc<AtomicBool>) -> Self {
        Self { cleanup_finished }
    }
}

impl Provider for BlockingCleanupProvider {
    fn id(&self) -> &str {
        "mock"
    }

    fn models(&self) -> &[ModelInfo] {
        static MODELS: std::sync::LazyLock<Vec<ModelInfo>> =
            std::sync::LazyLock::new(|| vec![rpc_model_info("mock-model", false)]);
        &MODELS
    }

    fn stream_prepared(&self, _request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
        let stream = stream::unfold(0, move |step| async move {
            match step {
                0 => Some((
                    Ok(AssistantStreamEvent::Start {
                        partial: base_assistant(),
                    }),
                    1,
                )),
                1 => {
                    std::future::pending::<()>().await;
                    None
                }
                _ => None,
            }
        });
        Box::pin(stream)
    }
}

impl Drop for BlockingCleanupProvider {
    fn drop(&mut self) {
        std::thread::sleep(Duration::from_millis(150));
        self.cleanup_finished.store(true, Ordering::SeqCst);
    }
}

#[derive(Clone)]
struct ControlledEmitCleanupProvider {
    continue_streaming: Arc<tokio::sync::Notify>,
    cleanup_finished: Arc<AtomicBool>,
}

impl ControlledEmitCleanupProvider {
    fn new(
        continue_streaming: Arc<tokio::sync::Notify>,
        cleanup_finished: Arc<AtomicBool>,
    ) -> Self {
        Self {
            continue_streaming,
            cleanup_finished,
        }
    }
}

impl Provider for ControlledEmitCleanupProvider {
    fn id(&self) -> &str {
        "mock"
    }

    fn models(&self) -> &[ModelInfo] {
        static MODELS: std::sync::LazyLock<Vec<ModelInfo>> =
            std::sync::LazyLock::new(|| vec![rpc_model_info("mock-model", false)]);
        &MODELS
    }

    fn stream_prepared(&self, _request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
        let continue_streaming = self.continue_streaming.clone();
        let stream = stream::unfold(0, move |step| {
            let continue_streaming = continue_streaming.clone();
            async move {
                match step {
                    0 => Some((
                        Ok(AssistantStreamEvent::Start {
                            partial: base_assistant(),
                        }),
                        1,
                    )),
                    1 => {
                        continue_streaming.notified().await;
                        let mut partial = base_assistant();
                        partial
                            .content
                            .push(opi_ai::message::AssistantContent::Text {
                                text: "after drop".into(),
                            });
                        Some((
                            Ok(AssistantStreamEvent::TextDelta {
                                content_index: 0,
                                delta: "after drop".into(),
                                partial,
                            }),
                            2,
                        ))
                    }
                    2 => {
                        std::future::pending::<()>().await;
                        None
                    }
                    _ => None,
                }
            }
        });
        Box::pin(stream)
    }
}

impl Drop for ControlledEmitCleanupProvider {
    fn drop(&mut self) {
        self.cleanup_finished.store(true, Ordering::SeqCst);
    }
}

#[derive(Clone)]
struct HeldRequestProvider {
    calls: Arc<Mutex<Vec<Request>>>,
    release: Arc<tokio::sync::Notify>,
}

impl HeldRequestProvider {
    fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            release: Arc::new(tokio::sync::Notify::new()),
        }
    }

    fn call_log(&self) -> Arc<Mutex<Vec<Request>>> {
        Arc::clone(&self.calls)
    }

    fn release_handle(&self) -> Arc<tokio::sync::Notify> {
        Arc::clone(&self.release)
    }
}

impl Provider for HeldRequestProvider {
    fn id(&self) -> &str {
        "mock"
    }

    fn models(&self) -> &[ModelInfo] {
        static MODELS: std::sync::LazyLock<Vec<ModelInfo>> = std::sync::LazyLock::new(|| {
            vec![
                rpc_model_info("mock-model", false),
                rpc_model_info("next-model", false),
            ]
        });
        &MODELS
    }

    fn stream_prepared(&self, request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
        let cancel = request.cancel.clone();
        let release = self.release.clone();
        self.calls.lock().unwrap().push(request);

        let stream = stream::unfold(0, move |step| {
            let cancel = cancel.clone();
            let release = release.clone();
            async move {
                match step {
                    0 => Some((
                        Ok(AssistantStreamEvent::Start {
                            partial: base_assistant(),
                        }),
                        1,
                    )),
                    1 => {
                        tokio::select! {
                            _ = cancel.cancelled() => None,
                            _ = release.notified() => {
                                let mut message = base_assistant();
                                message.content.push(opi_ai::message::AssistantContent::Text {
                                    text: "released".into(),
                                });
                                Some((Ok(AssistantStreamEvent::Done {
                                    reason: StopReason::Stop,
                                    message,
                                }), 2))
                            }
                        }
                    }
                    _ => None,
                }
            }
        });
        Box::pin(stream)
    }
}

#[derive(Clone)]
struct PanickingProvider;

impl Provider for PanickingProvider {
    fn id(&self) -> &str {
        "mock"
    }

    fn models(&self) -> &[ModelInfo] {
        static MODELS: std::sync::LazyLock<Vec<ModelInfo>> =
            std::sync::LazyLock::new(|| vec![rpc_model_info("mock-model", false)]);
        &MODELS
    }

    fn stream_prepared(&self, _request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
        let stream = stream::unfold(0, move |step| async move {
            match step {
                0 => Some((
                    Ok(AssistantStreamEvent::Start {
                        partial: base_assistant(),
                    }),
                    1,
                )),
                1 => panic!("forced active run panic"),
                _ => None,
            }
        });
        Box::pin(stream)
    }
}

fn blocking_cleanup_runner(
    provider: BlockingCleanupProvider,
) -> (
    tokio::sync::mpsc::UnboundedSender<RpcCommand>,
    tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>,
    tokio::task::JoinHandle<i32>,
) {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let runner = RpcRunner::new(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        ToolSelection::Disabled,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .expect("rpc runner should construct");

    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut runner = runner;
        runner.run_with_channels(command_rx, output_tx).await
    });
    (command_tx, output_rx, task)
}

fn custom_provider_runner<P>(
    provider: P,
) -> (
    tokio::sync::mpsc::UnboundedSender<RpcCommand>,
    tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>,
    tokio::task::JoinHandle<i32>,
)
where
    P: Provider + 'static,
{
    custom_provider_runner_with_config(provider, OpiConfig::default())
}

fn custom_provider_runner_with_model<P>(
    provider: P,
    model: impl Into<String>,
) -> (
    tokio::sync::mpsc::UnboundedSender<RpcCommand>,
    tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>,
    tokio::task::JoinHandle<i32>,
)
where
    P: Provider + 'static,
{
    custom_provider_runner_with_config_and_model(provider, OpiConfig::default(), model)
}

fn custom_provider_runner_with_config<P>(
    provider: P,
    config: OpiConfig,
) -> (
    tokio::sync::mpsc::UnboundedSender<RpcCommand>,
    tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>,
    tokio::task::JoinHandle<i32>,
)
where
    P: Provider + 'static,
{
    custom_provider_runner_with_config_and_model(provider, config, "mock:mock-model")
}

fn custom_provider_runner_with_config_and_model<P>(
    provider: P,
    config: OpiConfig,
    model: impl Into<String>,
) -> (
    tokio::sync::mpsc::UnboundedSender<RpcCommand>,
    tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>,
    tokio::task::JoinHandle<i32>,
)
where
    P: Provider + 'static,
{
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let runner = RpcRunner::new(
        Box::new(provider),
        model.into(),
        config,
        workspace.path().to_path_buf(),
        false,
        ToolSelection::Disabled,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .expect("rpc runner should construct");

    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut runner = runner;
        runner.run_with_channels(command_rx, output_tx).await
    });
    (command_tx, output_rx, task)
}

#[derive(Clone)]
struct ControlledProvider {
    calls: Arc<Mutex<Vec<Request>>>,
}

impl ControlledProvider {
    fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn call_log(&self) -> Arc<Mutex<Vec<Request>>> {
        Arc::clone(&self.calls)
    }
}

impl Provider for ControlledProvider {
    fn id(&self) -> &str {
        "mock"
    }

    fn models(&self) -> &[ModelInfo] {
        static MODELS: std::sync::LazyLock<Vec<ModelInfo>> =
            std::sync::LazyLock::new(|| vec![rpc_model_info("mock-model", false)]);
        &MODELS
    }

    fn stream_prepared(&self, request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
        let cancel = request.cancel.clone();
        self.calls.lock().unwrap().push(request);

        let stream = stream::unfold(0, move |step| {
            let cancel = cancel.clone();
            async move {
                match step {
                    0 => Some((
                        Ok(AssistantStreamEvent::Start {
                            partial: base_assistant(),
                        }),
                        1,
                    )),
                    1 => {
                        tokio::select! {
                            _ = cancel.cancelled() => None,
                            _ = tokio::time::sleep(Duration::from_millis(50)) => {
                                let mut partial = base_assistant();
                                partial.content.push(opi_ai::message::AssistantContent::Text {
                                    text: "partial".into(),
                                });
                                Some((Ok(AssistantStreamEvent::TextDelta {
                                    content_index: 0,
                                    delta: "partial".into(),
                                    partial,
                                }), 2))
                            }
                        }
                    }
                    2 => {
                        tokio::select! {
                            _ = cancel.cancelled() => None,
                            _ = tokio::time::sleep(Duration::from_millis(250)) => {
                                let mut message = base_assistant();
                                message.content.push(opi_ai::message::AssistantContent::Text {
                                    text: "partial".into(),
                                });
                                Some((Ok(AssistantStreamEvent::Done {
                                    reason: StopReason::Stop,
                                    message,
                                }), 3))
                            }
                        }
                    }
                    _ => None,
                }
            }
        });
        Box::pin(stream)
    }
}

#[derive(Clone)]
struct SecondTurnGatedDeltaProvider {
    run_count: Arc<AtomicUsize>,
    second_delta_parked: Arc<tokio::sync::Notify>,
    release_second_delta: Arc<tokio::sync::Notify>,
    second_delta_emitted: Arc<AtomicBool>,
}

impl SecondTurnGatedDeltaProvider {
    fn new() -> Self {
        Self {
            run_count: Arc::new(AtomicUsize::new(0)),
            second_delta_parked: Arc::new(tokio::sync::Notify::new()),
            release_second_delta: Arc::new(tokio::sync::Notify::new()),
            second_delta_emitted: Arc::new(AtomicBool::new(false)),
        }
    }

    async fn wait_for_second_delta_gate(&self) {
        tokio::time::timeout(Duration::from_secs(2), self.second_delta_parked.notified())
            .await
            .expect("timed out waiting for second run to park before delta");
    }

    fn release_second_delta_gate(&self) {
        self.release_second_delta.notify_one();
    }

    fn second_delta_emitted(&self) -> bool {
        self.second_delta_emitted.load(Ordering::SeqCst)
    }
}

impl Provider for SecondTurnGatedDeltaProvider {
    fn id(&self) -> &str {
        "mock"
    }

    fn models(&self) -> &[ModelInfo] {
        static MODELS: std::sync::LazyLock<Vec<ModelInfo>> =
            std::sync::LazyLock::new(|| vec![rpc_model_info("mock-model", false)]);
        &MODELS
    }

    fn stream_prepared(&self, request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
        let run_number = self.run_count.fetch_add(1, Ordering::SeqCst) + 1;
        let cancel = request.cancel.clone();
        let second_delta_parked = self.second_delta_parked.clone();
        let release_second_delta = self.release_second_delta.clone();
        let second_delta_emitted = self.second_delta_emitted.clone();

        let stream = stream::unfold(0, move |step| {
            let cancel = cancel.clone();
            let second_delta_parked = second_delta_parked.clone();
            let release_second_delta = release_second_delta.clone();
            let second_delta_emitted = second_delta_emitted.clone();
            async move {
                match step {
                    0 => Some((
                        Ok(AssistantStreamEvent::Start {
                            partial: base_assistant(),
                        }),
                        1,
                    )),
                    1 if run_number == 2 => {
                        second_delta_parked.notify_one();
                        release_second_delta.notified().await;
                        if cancel.is_cancelled() {
                            None
                        } else {
                            second_delta_emitted.store(true, Ordering::SeqCst);
                            let mut partial = base_assistant();
                            partial
                                .content
                                .push(opi_ai::message::AssistantContent::Text {
                                    text: "partial".into(),
                                });
                            Some((
                                Ok(AssistantStreamEvent::TextDelta {
                                    content_index: 0,
                                    delta: "partial".into(),
                                    partial,
                                }),
                                2,
                            ))
                        }
                    }
                    1 => {
                        cancel.cancelled().await;
                        None
                    }
                    2 => {
                        let mut message = base_assistant();
                        message
                            .content
                            .push(opi_ai::message::AssistantContent::Text {
                                text: "partial".into(),
                            });
                        Some((
                            Ok(AssistantStreamEvent::Done {
                                reason: StopReason::Stop,
                                message,
                            }),
                            3,
                        ))
                    }
                    _ => None,
                }
            }
        });
        Box::pin(stream)
    }
}

fn rpc_test_runner(
    provider: ControlledProvider,
) -> (
    tokio::sync::mpsc::UnboundedSender<RpcCommand>,
    tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>,
    tokio::task::JoinHandle<i32>,
) {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let runner = RpcRunner::new(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        ToolSelection::Disabled,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .expect("rpc runner should construct");

    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut runner = runner;
        runner.run_with_channels(command_rx, output_tx).await
    });
    (command_tx, output_rx, task)
}

fn custom_provider_runner_with_extension_registry<P>(
    provider: P,
    registry: ExtensionRegistry,
) -> (
    tokio::sync::mpsc::UnboundedSender<RpcCommand>,
    tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>,
    tokio::task::JoinHandle<i32>,
)
where
    P: Provider + 'static,
{
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let runner = RpcRunner::new_with_extension_registry(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        ToolSelection::Disabled,
        None,
        Vec::new(),
        registry,
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .expect("rpc runner should construct");

    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut runner = runner;
        runner.run_with_channels(command_rx, output_tx).await
    });
    (command_tx, output_rx, task)
}

/// Phase 11.4: write audit details cross the RPC JSONL output boundary. The
/// ToolExecutionEnd event carries public-safe tool-owned `details` fields
/// such as action and bytes_written.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn write_tool_result_carries_write_audit_details() {
    let provider = MockProvider::new(
        "mock",
        vec![
            opi_ai::test_support::tool_call_response(
                "tc-write",
                "write",
                r#"{"path":"out.txt","content":"payload"}"#,
            ),
            opi_ai::test_support::text_response("done"),
        ],
    );

    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let runner = RpcRunner::new(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        true, // allow_mutating: write must be executable
        ToolSelection::Allowlist(vec!["write".into()]),
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .expect("rpc runner with write tool");

    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut runner = runner;
        runner.run_with_channels(command_rx, output_tx).await
    });

    let _ready = recv_rpc_line(&mut output_rx).await; // rpc_ready
    command_tx
        .send(RpcCommand::prompt {
            id: Some("w-rpc".into()),
            message: "write out.txt".into(),
        })
        .unwrap();
    let accepted = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(accepted["success"], true);

    // Drain events until the write ToolExecutionEnd is observed (bounded).
    let mut write_end: Option<serde_json::Value> = None;
    for _ in 0..64 {
        let line = recv_rpc_line(&mut output_rx).await;
        if line["type"] == "ToolExecutionEnd" && line["tool_name"] == "write" {
            write_end = Some(line);
            break;
        }
        if line["type"] == "AgentEnd" {
            break;
        }
    }
    let end = write_end.expect("expected a write ToolExecutionEnd event");
    assert_eq!(end["is_error"], false);
    assert_eq!(end["truncated"], false);
    let details = end.get("details").expect("details present");
    assert!(
        details.is_object(),
        "details must be an object, got: {details}"
    );
    assert_eq!(details["action"], "created");
    assert_eq!(details["bytes_written"], 7); // "payload" == 7 bytes

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _ = recv_response(&mut output_rx, "quit").await;
    let _ = task.await;
    let _ = std::fs::remove_file(workspace.path().join("out.txt"));
}

/// Phase 12 task 12.2 — provider-error events on the RPC JSONL surface must
/// not leak credentials. A retryable `Network` error carrying a secret triggers
/// `AutoRetryStart` whose `error_message` is the provider-error string; the
/// public emit boundary scrubs it before it reaches the JSONL stream.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn provider_error_events_are_redacted() {
    let secret = "sk-proj-1234567890abcdefghijklmnopqrstuv";
    let make_err = || {
        MockResponse::Error(ProviderError::Network(
            ProviderErrorSummary::from_untrusted(format!("conn reset; echoed key {secret}")),
        ))
    };
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            make_err(),
            make_err(),
            make_err(),
            make_err(),
            make_err(),
            make_err(),
        ],
    );
    let (command_tx, mut output_rx, task) = custom_provider_runner(provider);

    let _ready = recv_rpc_line(&mut output_rx).await; // rpc_ready
    command_tx
        .send(RpcCommand::prompt {
            id: Some("redact-rpc".into()),
            message: "go".into(),
        })
        .unwrap();
    let _accepted = recv_response(&mut output_rx, "prompt").await;

    // Drain every event line the run emits and assert the secret never leaks.
    let mut leaked = false;
    let mut saw_auto_retry = false;
    for _ in 0..256 {
        let Ok(maybe_line) =
            tokio::time::timeout(std::time::Duration::from_secs(2), output_rx.recv()).await
        else {
            break;
        };
        let Some(line) = maybe_line else {
            break;
        };
        if line.to_string().contains(secret) {
            leaked = true;
        }
        if line["type"] == "AutoRetryStart" {
            saw_auto_retry = true;
        }
        if line["type"] == "AgentEnd" {
            break;
        }
    }
    assert!(
        saw_auto_retry,
        "test should have triggered an AutoRetryStart event"
    );
    assert!(
        !leaked,
        "RPC JSONL event stream leaked provider secret across the public boundary"
    );

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _ = recv_response(&mut output_rx, "quit").await;
    let _ = task.await;
}

/// Phase 11.5: edit diff-preview details cross the RPC JSONL output boundary.
/// The ToolExecutionEnd event carries public-safe tool-owned `details` fields,
/// with before/after as STRING values (the contract interactive.rs depends on)
/// plus action/occurrences metadata.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn edit_tool_result_carries_diff_preview_details() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    // edit reads the file, so it must pre-exist in the workspace.
    std::fs::write(workspace.path().join("src.txt"), "hello world").unwrap();

    let provider = MockProvider::new(
        "mock",
        vec![
            opi_ai::test_support::tool_call_response(
                "tc-edit",
                "edit",
                r#"{"path":"src.txt","old_string":"hello","new_string":"HI"}"#,
            ),
            opi_ai::test_support::text_response("done"),
        ],
    );

    let runner = RpcRunner::new(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        true, // allow_mutating: edit must be executable
        ToolSelection::Allowlist(vec!["edit".into()]),
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .expect("rpc runner with edit tool");

    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut runner = runner;
        runner.run_with_channels(command_rx, output_tx).await
    });

    let _ready = recv_rpc_line(&mut output_rx).await; // rpc_ready
    command_tx
        .send(RpcCommand::prompt {
            id: Some("e-rpc".into()),
            message: "edit src.txt".into(),
        })
        .unwrap();
    let accepted = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(accepted["success"], true);

    // Drain events until the edit ToolExecutionEnd is observed (bounded).
    let mut edit_end: Option<serde_json::Value> = None;
    for _ in 0..64 {
        let line = recv_rpc_line(&mut output_rx).await;
        if line["type"] == "ToolExecutionEnd" && line["tool_name"] == "edit" {
            edit_end = Some(line);
            break;
        }
        if line["type"] == "AgentEnd" {
            break;
        }
    }
    let end = edit_end.expect("expected an edit ToolExecutionEnd event");
    assert_eq!(end["is_error"], false);
    assert_eq!(end["truncated"], false);
    let details = end.get("details").expect("details present");
    assert!(
        details.is_object(),
        "details must be an object, got: {details}"
    );
    assert_eq!(details["action"], "edited");
    assert_eq!(details["occurrences"], 1);
    assert!(
        details["before"].is_string(),
        "before must be string-typed at the RPC boundary"
    );
    assert!(
        details["after"].is_string(),
        "after must be string-typed at the RPC boundary"
    );
    assert_eq!(details["before"], "hello world");
    assert_eq!(details["after"], "HI world");

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _ = recv_response(&mut output_rx, "quit").await;
    let _ = task.await;
    let _ = std::fs::remove_file(workspace.path().join("src.txt"));
}

/// Phase 11.8 S1: a tool-failure result's per-cause diagnostic + structured
/// context cross the RPC JSONL output boundary on `ToolExecutionEnd.diagnostics`
/// (read of a missing file -> `tool_path_not_found` with path context).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tool_failure_trace_carries_tool_details() {
    let provider = MockProvider::new(
        "mock",
        vec![
            opi_ai::test_support::tool_call_response(
                "tc-read",
                "read",
                r#"{"path":"no_such_file_xyz.json"}"#,
            ),
            opi_ai::test_support::text_response("done"),
        ],
    );
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let runner = RpcRunner::new(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false, // read is non-mutating
        ToolSelection::Allowlist(vec!["read".into()]),
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .expect("rpc runner with read tool");

    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut runner = runner;
        runner.run_with_channels(command_rx, output_tx).await
    });

    let _ready = recv_rpc_line(&mut output_rx).await; // rpc_ready
    command_tx
        .send(RpcCommand::prompt {
            id: Some("r-rpc".into()),
            message: "read missing".into(),
        })
        .unwrap();
    let accepted = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(accepted["success"], true);

    let mut read_end: Option<serde_json::Value> = None;
    for _ in 0..64 {
        let line = recv_rpc_line(&mut output_rx).await;
        if line["type"] == "ToolExecutionEnd" && line["tool_name"] == "read" {
            read_end = Some(line);
            break;
        }
        if line["type"] == "AgentEnd" {
            break;
        }
    }
    let end = read_end.expect("expected a read ToolExecutionEnd event");
    assert_eq!(end["is_error"], true, "missing-file read is_error: {end}");
    let diags = end["diagnostics"]
        .as_array()
        .expect("diagnostics array on RPC ToolExecutionEnd (Phase 11.8 D2)");
    assert!(
        diags.iter().any(|d| d["code"] == "tool_path_not_found"),
        "per-cause tool_path_not_found diagnostic surfaces over RPC: {diags:?}"
    );

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _ = recv_response(&mut output_rx, "quit").await;
    let _ = task.await;
}

/// Phase 11.8 S4: RPC `ToolExecutionEnd` exposes details, diagnostics,
/// is_error, and truncated consistently for a failing tool result (bash
/// nonzero exit carries operation details AND a tool-owned diagnostic).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tool_result_details_diagnostics_and_truncated_shape() {
    let cmd = if cfg!(windows) {
        "cmd /C exit 1"
    } else {
        "exit 1"
    };
    let args = serde_json::to_string(&serde_json::json!({ "command": cmd })).unwrap();
    let provider = MockProvider::new(
        "mock",
        vec![
            opi_ai::test_support::tool_call_response("tc-bash", "bash", &args),
            opi_ai::test_support::text_response("done"),
        ],
    );
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let runner = RpcRunner::new(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        true, // allow_mutating: bash must be executable
        ToolSelection::Allowlist(vec!["bash".into()]),
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .expect("rpc runner with bash tool");

    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut runner = runner;
        runner.run_with_channels(command_rx, output_tx).await
    });

    let _ready = recv_rpc_line(&mut output_rx).await; // rpc_ready
    command_tx
        .send(RpcCommand::prompt {
            id: Some("b-rpc".into()),
            message: "run failing".into(),
        })
        .unwrap();
    let accepted = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(accepted["success"], true);

    let mut bash_end: Option<serde_json::Value> = None;
    for _ in 0..64 {
        let line = recv_rpc_line(&mut output_rx).await;
        if line["type"] == "ToolExecutionEnd" && line["tool_name"] == "bash" {
            bash_end = Some(line);
            break;
        }
        if line["type"] == "AgentEnd" {
            break;
        }
    }
    let end = bash_end.expect("expected a bash ToolExecutionEnd event");
    assert_eq!(end["is_error"], true, "nonzero exit is_error: {end}");
    assert!(
        end["details"].is_object(),
        "operation details present (command/exit_code/...): {end}"
    );
    assert_eq!(
        end["details"]["command"], "[REDACTED]",
        "bash command should be redacted at the RPC boundary: {end}"
    );
    assert_eq!(end["truncated"], false, "no truncation: {end}");
    let diags = end["diagnostics"]
        .as_array()
        .expect("diagnostics array present (Phase 11.8 D2)");
    assert!(
        diags.iter().any(|d| {
            d["code"] == "tool_execution_failed"
                && d["context"]["exit_code"] == 1
                && d["context"]["command_included"] == false
                && d["context"].get("command").is_none()
        }),
        "bash operation diagnostic (exit_code=1) surfaces without command text over RPC: {diags:?}"
    );

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _ = recv_response(&mut output_rx, "quit").await;
    let _ = task.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_tool_events_redact_bash_command_canary() {
    let command = if cfg!(windows) {
        "set OPI_RPC_COMMAND_SECRET_CANARY=1 && exit /B 1"
    } else {
        "OPI_RPC_COMMAND_SECRET_CANARY=1 false"
    };
    let args = serde_json::to_string(&serde_json::json!({ "command": command })).unwrap();
    let provider = MockProvider::new(
        "mock",
        vec![
            opi_ai::test_support::tool_call_response("tc-bash", "bash", &args),
            text_response("done"),
        ],
    );
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let runner = RpcRunner::new(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        true,
        ToolSelection::Allowlist(vec!["bash".into()]),
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .expect("rpc runner with bash tool");

    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut runner = runner;
        runner.run_with_channels(command_rx, output_tx).await
    });

    let ready = recv_rpc_line(&mut output_rx).await;
    assert!(
        !serde_json::to_string(&ready)
            .unwrap()
            .contains("OPI_RPC_COMMAND_SECRET_CANARY")
    );
    command_tx
        .send(RpcCommand::prompt {
            id: Some("bash-canary".into()),
            message: "run failing command".into(),
        })
        .unwrap();
    let accepted = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(accepted["success"], true);

    let mut saw_bash_end = false;
    for _ in 0..64 {
        let line = recv_rpc_line(&mut output_rx).await;
        let rendered = serde_json::to_string(&line).unwrap();
        assert!(
            !rendered.contains("OPI_RPC_COMMAND_SECRET_CANARY"),
            "{rendered}"
        );
        if line["type"] == "ToolExecutionEnd" && line["tool_name"] == "bash" {
            saw_bash_end = true;
            assert_eq!(line["is_error"], true);
            assert_eq!(line["truncated"], false);
            assert!(
                line["diagnostics"].as_array().is_some_and(|items| items
                    .iter()
                    .any(|d| d["code"] == "tool_execution_failed")),
                "{line}"
            );
        }
        if line["type"] == "AgentEnd" {
            break;
        }
    }
    assert!(saw_bash_end, "expected bash ToolExecutionEnd");

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _ = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

/// Phase 11.6: bash truncation details + the top-level `truncated` flag cross
/// the RPC JSONL output boundary. bash is the first built-in tool whose
/// `ToolResult.truncated` can flip true, and `full_output` is a new details
/// key; this pins that both survive the agent_loop generic lift into
/// ToolExecutionEnd (parity with the 11.4 write / 11.5 edit visibility tests).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bash_tool_result_carries_truncation_details() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let cap = opi_coding_agent::tool::MAX_BASH_OUTPUT_BYTES;
    let n = (cap / 13) + 2000;
    let command = if cfg!(windows) {
        format!("FOR /L %i IN (1,1,{n}) DO @echo AAAAAAAAAAAA")
    } else {
        format!("yes AAAAAAAAAAAA | head -n {n}")
    };
    let args = serde_json::to_string(&serde_json::json!({ "command": command }))
        .expect("encode bash args");

    let provider = MockProvider::new(
        "mock",
        vec![
            opi_ai::test_support::tool_call_response("tc-bash", "bash", &args),
            opi_ai::test_support::text_response("done"),
        ],
    );

    let runner = RpcRunner::new(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        true, // allow_mutating: bash must be executable
        ToolSelection::Allowlist(vec!["bash".into()]),
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .expect("rpc runner with bash tool");

    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut runner = runner;
        runner.run_with_channels(command_rx, output_tx).await
    });

    let _ready = recv_rpc_line(&mut output_rx).await; // rpc_ready
    command_tx
        .send(RpcCommand::prompt {
            id: Some("b-rpc".into()),
            message: "run a noisy command".into(),
        })
        .unwrap();
    let accepted = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(accepted["success"], true);

    // Drain events until the bash ToolExecutionEnd is observed (bounded).
    let mut bash_end: Option<serde_json::Value> = None;
    for _ in 0..64 {
        let line = recv_rpc_line(&mut output_rx).await;
        if line["type"] == "ToolExecutionEnd" && line["tool_name"] == "bash" {
            bash_end = Some(line);
            break;
        }
        if line["type"] == "AgentEnd" {
            break;
        }
    }
    let end = bash_end.expect("expected a bash ToolExecutionEnd event");
    assert_eq!(end["is_error"], false);
    assert_eq!(
        end["truncated"], true,
        "top-level truncated flag must cross the RPC boundary when output is capped"
    );
    let details = end.get("details").expect("details present");
    assert!(
        details.is_object(),
        "details must be an object, got: {details}"
    );
    assert_eq!(
        details["truncated"], true,
        "details.truncated must mirror the flag at the RPC boundary"
    );
    let full = details["full_output"]
        .as_str()
        .expect("details.full_output reference must cross the RPC boundary");
    assert_eq!(
        full, "[REDACTED]",
        "public RPC events must redact the absolute full_output spill path"
    );

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _ = recv_response(&mut output_rx, "quit").await;
    let _ = task.await;
}

async fn recv_rpc_line(
    output_rx: &mut tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>,
) -> serde_json::Value {
    tokio::time::timeout(Duration::from_secs(2), output_rx.recv())
        .await
        .expect("timed out waiting for RPC output")
        .expect("RPC output channel closed")
}

async fn recv_response(
    output_rx: &mut tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>,
    command: &str,
) -> serde_json::Value {
    loop {
        let line = recv_rpc_line(output_rx).await;
        if line["type"] == "response" && line["command"] == command {
            return line;
        }
    }
}

async fn recv_until_agent_end(
    output_rx: &mut tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>,
) {
    loop {
        let line = recv_rpc_line(output_rx).await;
        if line["type"] == "AgentEnd" {
            return;
        }
    }
}

async fn wait_for_idle_session_info(
    command_tx: &tokio::sync::mpsc::UnboundedSender<RpcCommand>,
    output_rx: &mut tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>,
) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    let mut attempt = 0;
    loop {
        if tokio::time::Instant::now() >= deadline {
            panic!("RPC runner did not become idle after active run completed");
        }
        let id = format!("idle-{attempt}");
        command_tx
            .send(RpcCommand::session_info {
                id: Some(id.clone()),
            })
            .unwrap();
        let response = recv_response(output_rx, "session_info").await;
        if response["id"] != id {
            continue;
        }
        if response["success"] == true {
            return;
        }
        assert_eq!(
            response["error"],
            "cannot query session info while agent is running"
        );
        attempt += 1;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_quit_while_running_waits_for_active_task_cleanup() {
    let cleanup_finished = Arc::new(AtomicBool::new(false));
    let provider = BlockingCleanupProvider::new(cleanup_finished.clone());
    let (command_tx, mut output_rx, task) = blocking_cleanup_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-1".into()),
            message: "start".into(),
        })
        .unwrap();
    let _prompt = recv_response(&mut output_rx, "prompt").await;
    loop {
        let line = recv_rpc_line(&mut output_rx).await;
        if line["type"] == "MessageStart" {
            break;
        }
    }

    command_tx
        .send(RpcCommand::quit {
            id: Some("quit-1".into()),
        })
        .unwrap();
    let quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(quit["success"], true);

    assert_eq!(task.await.unwrap(), 0);
    assert!(
        cleanup_finished.load(Ordering::SeqCst),
        "RPC runner returned before the active run task completed cleanup"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_eof_while_running_waits_for_active_task_cleanup() {
    let cleanup_finished = Arc::new(AtomicBool::new(false));
    let provider = BlockingCleanupProvider::new(cleanup_finished.clone());
    let (command_tx, mut output_rx, task) = blocking_cleanup_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-1".into()),
            message: "start".into(),
        })
        .unwrap();
    let _prompt = recv_response(&mut output_rx, "prompt").await;
    loop {
        let line = recv_rpc_line(&mut output_rx).await;
        if line["type"] == "MessageStart" {
            break;
        }
    }

    drop(command_tx);

    assert_eq!(task.await.unwrap(), 0);
    assert!(
        cleanup_finished.load(Ordering::SeqCst),
        "RPC runner returned before the active run task completed cleanup"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_output_drop_while_running_waits_for_active_task_cleanup() {
    let continue_streaming = Arc::new(tokio::sync::Notify::new());
    let cleanup_finished = Arc::new(AtomicBool::new(false));
    let provider =
        ControlledEmitCleanupProvider::new(continue_streaming.clone(), cleanup_finished.clone());
    let (command_tx, mut output_rx, task) = custom_provider_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-1".into()),
            message: "start".into(),
        })
        .unwrap();
    let _prompt = recv_response(&mut output_rx, "prompt").await;
    loop {
        let line = recv_rpc_line(&mut output_rx).await;
        if line["type"] == "MessageStart" {
            break;
        }
    }

    drop(output_rx);
    continue_streaming.notify_one();

    let exit = tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .expect("RPC runner did not exit after output failure")
        .unwrap();
    assert_eq!(exit, 1);
    assert!(
        cleanup_finished.load(Ordering::SeqCst),
        "RPC runner returned after output failure before active run cleanup"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_active_run_join_error_is_fatal() {
    let (command_tx, mut output_rx, task) = custom_provider_runner(PanickingProvider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-1".into()),
            message: "start".into(),
        })
        .unwrap();
    let _prompt = recv_response(&mut output_rx, "prompt").await;

    let exit = tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .expect("RPC runner continued after active run JoinError")
        .unwrap();
    assert_eq!(exit, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_mid_turn_abort_is_processed() {
    let provider = ControlledProvider::new();
    let (command_tx, mut output_rx, task) = rpc_test_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-1".into()),
            message: "start".into(),
        })
        .unwrap();
    let prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(prompt["success"], true);

    command_tx
        .send(RpcCommand::abort {
            id: Some("abort-1".into()),
        })
        .unwrap();
    let abort = recv_response(&mut output_rx, "abort").await;
    assert_eq!(abort["success"], true);
    assert_eq!(abort["id"], "abort-1");

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_second_turn_abort_after_cancel_targets_active_run() {
    let provider = SecondTurnGatedDeltaProvider::new();
    let (command_tx, mut output_rx, task) = custom_provider_runner(provider.clone());

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-1".into()),
            message: "first".into(),
        })
        .unwrap();
    let _prompt = recv_response(&mut output_rx, "prompt").await;
    loop {
        let line = recv_rpc_line(&mut output_rx).await;
        if line["type"] == "MessageStart" {
            break;
        }
    }
    command_tx.send(RpcCommand::abort { id: None }).unwrap();
    let _abort = recv_response(&mut output_rx, "abort").await;
    recv_until_agent_end(&mut output_rx).await;
    wait_for_idle_session_info(&command_tx, &mut output_rx).await;

    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-2".into()),
            message: "second".into(),
        })
        .unwrap();
    let second_prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(second_prompt["id"], "prompt-2");
    assert_eq!(second_prompt["success"], true);

    provider.wait_for_second_delta_gate().await;

    command_tx
        .send(RpcCommand::abort {
            id: Some("abort-2".into()),
        })
        .unwrap();
    let second_abort = recv_response(&mut output_rx, "abort").await;
    assert_eq!(second_abort["success"], true);

    provider.release_second_delta_gate();

    loop {
        let line = recv_rpc_line(&mut output_rx).await;
        assert_ne!(
            line["type"], "MessageUpdate",
            "stale control handle allowed the second run to continue after abort"
        );
        if line["type"] == "AgentEnd" {
            break;
        }
    }
    assert!(
        !provider.second_delta_emitted(),
        "second run emitted a delta after abort released the gate"
    );

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

// ---------------------------------------------------------------------------
// Phase 8: RPC abort cancellation contract (task 8.4)
//
// An abort of an in-flight RPC run must surface the shared observable
// cancellation contract: a correlated success response, a terminal AgentEnd
// event, and a return to idle so a subsequent prompt is accepted.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase8_abort_cancellation_contract() {
    let provider = ControlledProvider::new();
    let (command_tx, mut output_rx, task) = rpc_test_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    // Start a run whose provider stream hangs so the run stays in flight.
    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-1".into()),
            message: "start".into(),
        })
        .unwrap();
    let prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(prompt["success"], true);

    // Abort the active run; the response is correlated with the request id.
    command_tx
        .send(RpcCommand::abort {
            id: Some("abort-1".into()),
        })
        .unwrap();
    let abort = recv_response(&mut output_rx, "abort").await;
    assert_eq!(abort["success"], true);
    assert_eq!(
        abort["id"], "abort-1",
        "abort response carries the correlated id"
    );

    // The cancelled run emits a terminal AgentEnd event on the wire.
    recv_until_agent_end(&mut output_rx).await;

    // The runner returns to idle, then accepts a new prompt.
    wait_for_idle_session_info(&command_tx, &mut output_rx).await;
    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-2".into()),
            message: "again".into(),
        })
        .unwrap();
    let second = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(second["id"], "prompt-2");
    assert_eq!(
        second["success"], true,
        "runner accepts a new prompt after abort (idle)"
    );

    // Abort the second run before quitting so shutdown is prompt.
    command_tx
        .send(RpcCommand::abort {
            id: Some("abort-2".into()),
        })
        .unwrap();
    let _ = recv_response(&mut output_rx, "abort").await;
    recv_until_agent_end(&mut output_rx).await;

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

// ---------------------------------------------------------------------------
// Phase 8 (task 8.5): RPC command-state contract.
//
// The RPC runner accepts prompt/continue only when idle, queues steer/follow_up
// while running, treats abort as a successful no-op when idle, rejects every
// mutating command while busy with a correlated structured error, and never
// silently queues or partially applies a mutation to an in-flight run.
// ---------------------------------------------------------------------------

/// A second prompt while a run is in flight is rejected (not queued) with a
/// correlated error, and no second run is spawned.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase8_rpc_command_contract_second_prompt_rejected_while_busy_no_second_run() {
    let provider = ControlledProvider::new();
    let call_log = provider.call_log();
    let (command_tx, mut output_rx, task) = rpc_test_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::prompt {
            id: Some("p1".into()),
            message: "start".into(),
        })
        .unwrap();
    let first = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(first["success"], true, "first prompt accepted");

    // Wait until the run is provably in flight (provider stream started).
    loop {
        let line = recv_rpc_line(&mut output_rx).await;
        if line["type"] == "MessageStart" {
            break;
        }
    }

    // A second prompt while the run is in flight is rejected, not queued.
    command_tx
        .send(RpcCommand::prompt {
            id: Some("p2".into()),
            message: "second".into(),
        })
        .unwrap();
    let second = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(
        second["id"], "p2",
        "rejected response carries the correlated id"
    );
    assert_eq!(
        second["success"], false,
        "second prompt while busy must be rejected"
    );
    assert_eq!(
        second["error"],
        "agent is already running; use steer or follow_up to queue messages"
    );

    // No second run was spawned: still exactly one provider request.
    assert_eq!(
        call_log.lock().unwrap().len(),
        1,
        "busy rejection must not spawn a second run"
    );

    command_tx.send(RpcCommand::abort { id: None }).unwrap();
    let _ = recv_response(&mut output_rx, "abort").await;
    recv_until_agent_end(&mut output_rx).await;
    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

/// A continue while a run is in flight is rejected (not queued) with a
/// correlated error, and no second run is spawned.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase8_rpc_command_contract_continue_rejected_while_busy() {
    let provider = ControlledProvider::new();
    let call_log = provider.call_log();
    let (command_tx, mut output_rx, task) = rpc_test_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::prompt {
            id: Some("p1".into()),
            message: "start".into(),
        })
        .unwrap();
    let first = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(first["success"], true, "prompt accepted");

    // Wait until the run is provably in flight (provider stream started).
    loop {
        let line = recv_rpc_line(&mut output_rx).await;
        if line["type"] == "MessageStart" {
            break;
        }
    }

    command_tx
        .send(RpcCommand::continue_ {
            id: Some("c2".into()),
            message: "more".into(),
        })
        .unwrap();
    let cont = recv_response(&mut output_rx, "continue").await;
    assert_eq!(
        cont["id"], "c2",
        "rejected response carries the correlated id"
    );
    assert_eq!(
        cont["success"], false,
        "continue while busy must be rejected"
    );
    assert_eq!(
        cont["error"],
        "agent is already running; use steer or follow_up to queue messages"
    );
    assert_eq!(
        call_log.lock().unwrap().len(),
        1,
        "busy rejection must not spawn a second run"
    );

    command_tx.send(RpcCommand::abort { id: None }).unwrap();
    let _ = recv_response(&mut output_rx, "abort").await;
    recv_until_agent_end(&mut output_rx).await;
    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

/// abort while idle is a successful no-op with a correlated id, and leaves the
/// runner able to accept and complete a subsequent prompt.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase8_rpc_command_contract_abort_while_idle_is_successful_noop() {
    let provider = ControlledProvider::new();
    let (command_tx, mut output_rx, task) = rpc_test_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    // abort while idle is a successful no-op with a correlated id.
    command_tx
        .send(RpcCommand::abort {
            id: Some("idle-abort".into()),
        })
        .unwrap();
    let abort = recv_response(&mut output_rx, "abort").await;
    assert_eq!(abort["id"], "idle-abort");
    assert_eq!(
        abort["success"], true,
        "abort while idle is a successful no-op"
    );

    // The idle no-op leaves the runner able to accept and complete a prompt.
    command_tx
        .send(RpcCommand::prompt {
            id: Some("p1".into()),
            message: "after idle abort".into(),
        })
        .unwrap();
    let prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(prompt["success"], true);
    recv_until_agent_end(&mut output_rx).await;

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

/// follow_up sent while a run is in flight is accepted (queued) with a
/// correlated success response, and is delivered as a second provider request
/// once the running turn completes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase8_rpc_command_contract_follow_up_queued_while_running() {
    let provider = ControlledProvider::new();
    let call_log = provider.call_log();
    let (command_tx, mut output_rx, task) = rpc_test_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::prompt {
            id: Some("p1".into()),
            message: "start".into(),
        })
        .unwrap();
    let _prompt = recv_response(&mut output_rx, "prompt").await;

    // follow_up while running is accepted (queued) with a correlated id.
    command_tx
        .send(RpcCommand::follow_up {
            id: Some("fu1".into()),
            message: "queued follow up".into(),
        })
        .unwrap();
    let follow_up = recv_response(&mut output_rx, "follow_up").await;
    assert_eq!(follow_up["id"], "fu1");
    assert_eq!(
        follow_up["success"], true,
        "follow_up while running is queued (success)"
    );

    // The run completes turn 1, then the queued follow-up drives a second turn.
    loop {
        let line = recv_rpc_line(&mut output_rx).await;
        if line["type"] == "AgentEnd" {
            break;
        }
    }

    {
        let calls = call_log.lock().unwrap();
        assert!(
            calls.len() >= 2,
            "queued follow-up must trigger a second provider request"
        );
        let saw_follow_up = calls[1].messages.iter().any(|message| match message {
            opi_ai::message::Message::User(user) => user.content.iter().any(|content| {
                matches!(
                    content,
                    opi_ai::message::InputContent::Text { text } if text == "queued follow up"
                )
            }),
            _ => false,
        });
        assert!(
            saw_follow_up,
            "second provider request should include the follow-up message"
        );
    }

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

/// compact, session_info, and extension_command are each rejected while a run
/// is in flight - each with its own correlated id and the documented error -
/// and none mutates the running turn (still exactly one provider request).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase8_rpc_command_contract_mutating_commands_rejected_while_busy_no_mutation() {
    let provider = ControlledProvider::new();
    let call_log = provider.call_log();
    let (command_tx, mut output_rx, task) = rpc_test_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::prompt {
            id: Some("p1".into()),
            message: "start".into(),
        })
        .unwrap();
    let prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(prompt["success"], true);

    // Wait until the run is provably in flight.
    loop {
        let line = recv_rpc_line(&mut output_rx).await;
        if line["type"] == "MessageStart" {
            break;
        }
    }

    // compact rejected while busy.
    command_tx
        .send(RpcCommand::compact {
            id: Some("cmp".into()),
        })
        .unwrap();
    let compact = recv_response(&mut output_rx, "compact").await;
    assert_eq!(compact["id"], "cmp");
    assert_eq!(compact["success"], false);
    assert_eq!(compact["error"], "cannot compact while agent is running");

    // session_info rejected while busy.
    command_tx
        .send(RpcCommand::session_info {
            id: Some("si".into()),
        })
        .unwrap();
    let session_info = recv_response(&mut output_rx, "session_info").await;
    assert_eq!(session_info["id"], "si");
    assert_eq!(session_info["success"], false);
    assert_eq!(
        session_info["error"],
        "cannot query session info while agent is running"
    );

    // extension_command rejected while busy.
    command_tx
        .send(RpcCommand::extension_command {
            id: Some("ext".into()),
            name: "x/y".into(),
            args: serde_json::json!({}),
        })
        .unwrap();
    let extension = recv_response(&mut output_rx, "extension_command").await;
    assert_eq!(extension["id"], "ext");
    assert_eq!(extension["success"], false);
    assert_eq!(
        extension["error"],
        "cannot dispatch extension command while agent is running"
    );

    // Still exactly one provider request: no partial mutation while running.
    assert_eq!(
        call_log.lock().unwrap().len(),
        1,
        "busy rejections must not mutate the run"
    );

    command_tx.send(RpcCommand::abort { id: None }).unwrap();
    let _ = recv_response(&mut output_rx, "abort").await;
    recv_until_agent_end(&mut output_rx).await;
    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_mid_turn_steer_is_queued() {
    let provider = ControlledProvider::new();
    let call_log = provider.call_log();
    let (command_tx, mut output_rx, task) = rpc_test_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-1".into()),
            message: "start".into(),
        })
        .unwrap();
    let _prompt = recv_response(&mut output_rx, "prompt").await;

    command_tx
        .send(RpcCommand::steer {
            id: Some("steer-1".into()),
            message: "use the queued steering".into(),
        })
        .unwrap();
    let steer = recv_response(&mut output_rx, "steer").await;
    assert_eq!(steer["success"], true);

    loop {
        let line = recv_rpc_line(&mut output_rx).await;
        if line["type"] == "AgentEnd" {
            break;
        }
    }

    {
        let calls = call_log.lock().unwrap();
        assert!(
            calls.len() >= 2,
            "queued steering should trigger a second provider request"
        );
        let second_call = &calls[1];
        let saw_steering = second_call.messages.iter().any(|message| match message {
            opi_ai::message::Message::User(user) => user.content.iter().any(|content| {
                matches!(
                    content,
                    opi_ai::message::InputContent::Text { text }
                        if text == "use the queued steering"
                )
            }),
            _ => false,
        });
        assert!(
            saw_steering,
            "second provider request should include steering message"
        );
    }

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_events_stream_before_turn_end() {
    let provider = ControlledProvider::new();
    let (command_tx, mut output_rx, task) = rpc_test_runner(provider);

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");

    command_tx
        .send(RpcCommand::prompt {
            id: Some("prompt-1".into()),
            message: "start".into(),
        })
        .unwrap();
    let _prompt = recv_response(&mut output_rx, "prompt").await;

    loop {
        let line = recv_rpc_line(&mut output_rx).await;
        assert_ne!(
            line["type"], "AgentEnd",
            "MessageUpdate should stream before the turn ends"
        );
        if line["type"] == "MessageUpdate" {
            break;
        }
    }

    command_tx.send(RpcCommand::abort { id: None }).unwrap();
    let _abort = recv_response(&mut output_rx, "abort").await;
    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

// ---------------------------------------------------------------------------
// Unit tests for response helpers
// ---------------------------------------------------------------------------

#[test]
fn rpc_response_format_success() {
    // Verify the response JSON structure matches the documented format.
    use serde_json::json;

    let resp = json!({
        "type": "response",
        "id": "req-1",
        "command": "prompt",
        "success": true,
    });
    assert_eq!(resp["type"], "response");
    assert_eq!(resp["command"], "prompt");
    assert_eq!(resp["success"], true);
    assert_eq!(resp["id"], "req-1");
}

#[test]
fn rpc_response_format_error() {
    use serde_json::json;

    let resp = json!({
        "type": "response",
        "id": "req-2",
        "command": "set_model",
        "success": false,
        "error": "model not found: invalid/model",
    });
    assert_eq!(resp["success"], false);
    assert!(resp["error"].is_string());
}

#[test]
fn rpc_response_format_with_data() {
    use serde_json::json;

    let resp = json!({
        "type": "response",
        "id": "req-3",
        "command": "session_info",
        "success": true,
        "data": {
            "model": "mock-model",
            "session_id": "abc123",
        },
    });
    assert_eq!(resp["success"], true);
    assert!(resp["data"].is_object());
    assert_eq!(resp["data"]["model"], "mock-model");
}

// ---------------------------------------------------------------------------
// Protocol version check
// ---------------------------------------------------------------------------

#[test]
fn rpc_schema_version_is_3() {
    // RPC mode uses schema version 3 (distinct from JSON mode's version 2).
    assert_eq!(RPC_SCHEMA_VERSION, 3);
}

// ---------------------------------------------------------------------------
// Phase 6 (task 6.5): startup diagnostics availability in RPC mode.
//
// The DoD requires startup diagnostics to be available in RPC mode. They are
// surfaced two ways: proactively in the rpc_ready header's startup_diagnostics
// array (so a headless client sees degraded-path diagnostics the instant the
// session is ready, without polling), and on demand via the session_info
// command's resources.diagnostics. Both flow from RuntimePackageStartup.
// ---------------------------------------------------------------------------

fn runner_with_runtime_packages<P>(
    provider: P,
    registry: ExtensionRegistry,
    diagnostics: Vec<Diagnostic>,
) -> (
    tokio::sync::mpsc::UnboundedSender<RpcCommand>,
    tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>,
    tokio::task::JoinHandle<i32>,
)
where
    P: Provider + 'static,
{
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let runtime_startup = RuntimePackageStartup {
        extension_registry: registry,
        installed_packages: Vec::new(),
        diagnostics,
        trust_decision: opi_coding_agent::project_trust::TrustDecision::Trusted,
    };
    let runner = RpcRunner::new_with_runtime_packages(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        ToolSelection::Disabled,
        None,
        Vec::new(),
        runtime_startup,
        None,
        Vec::new(),
    )
    .expect("rpc runner should construct");

    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut runner = runner;
        runner.run_with_channels(command_rx, output_tx).await
    });
    (command_tx, output_rx, task)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_ready_header_carries_startup_diagnostics() {
    let diagnostic = Diagnostic::new(
        Severity::Warning,
        code::CODE_PACKAGE_DIAGNOSTIC,
        SOURCE_PACKAGE,
        "package disabled at runtime: rpc demo diagnostic",
    );
    let (command_tx, mut output_rx, task) = runner_with_runtime_packages(
        MockProvider::new("mock", Vec::new()),
        ExtensionRegistry::new(),
        vec![diagnostic],
    );

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");
    let diagnostics = header["startup_diagnostics"]
        .as_array()
        .expect("rpc_ready should carry a startup_diagnostics array");
    assert!(
        diagnostics
            .iter()
            .any(|d| d["code"] == code::CODE_PACKAGE_DIAGNOSTIC
                && d["message"] == "package disabled at runtime: rpc demo diagnostic"),
        "rpc_ready startup_diagnostics must include the injected diagnostic: {diagnostics:?}"
    );

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

/// RPC fixed-local `ask` is refused while the harness is built, without a
/// prompt. The ready header carries the stable `permission_required` code and
/// headless-specific remediation (bash omitted, no fallback).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_ready_refuses_local_ask_at_startup_without_prompt() {
    let mut config = OpiConfig::default();
    config.execution.strategy = ExecutionStrategy::Fixed;
    config.execution.backend = "local".into();
    config
        .execution
        .permissions
        .insert("local".into(), PermissionDecision::Ask);
    // RpcRunner resolves this refusal synchronously during construction. The
    // isolated config dir makes the result independent of host package state.
    let provider = MockProvider::new("mock", vec![text_response("ok")]);
    let call_log = provider.call_log_handle();
    let _env_guard = common::empty_user_config_dir();
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let runner = RpcRunner::new(
        Box::new(provider),
        "mock:mock-model".into(),
        config,
        workspace.path().to_path_buf(),
        true,
        ToolSelection::Default,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .expect("rpc runner should construct");
    drop(_env_guard);
    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut runner = runner;
        runner.run_with_channels(command_rx, output_tx).await
    });

    let header = recv_rpc_line(&mut output_rx).await;
    assert_eq!(header["type"], "rpc_ready");
    let diagnostics = header["startup_diagnostics"]
        .as_array()
        .expect("rpc_ready should carry a startup_diagnostics array");
    let permission_required = diagnostics
        .iter()
        .find(|d| d["details"]["code"] == "permission_required");
    assert!(
        permission_required.is_some(),
        "the stable permission_required code must surface in the RPC ready header: {diagnostics:?}"
    );
    let diagnostic = permission_required.expect("found");
    let remediation = diagnostic["details"]["remediation"]
        .as_str()
        .unwrap_or_default();
    assert!(
        !remediation.is_empty(),
        "remediation text must ride along and be actionable: {diagnostic}"
    );
    assert!(
        remediation.contains("cannot be granted non-interactively")
            && remediation.contains("run interactively"),
        "remediation must carry the headless-specific actionable fragment: {remediation}"
    );

    command_tx
        .send(RpcCommand::prompt {
            id: Some("no-bash".into()),
            message: "inspect available tools".into(),
        })
        .unwrap();
    let prompt = recv_response(&mut output_rx, "prompt").await;
    assert_eq!(prompt["success"], true);
    recv_until_agent_end(&mut output_rx).await;
    {
        let calls = call_log.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert!(
            calls[0].tools.iter().all(|tool| tool.name != "bash"),
            "RPC headless ask must omit bash instead of falling back to local: {:?}",
            calls[0]
                .tools
                .iter()
                .map(|tool| tool.name.as_str())
                .collect::<Vec<_>>()
        );
    }

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_session_info_surfaces_startup_diagnostics() {
    let diagnostic = Diagnostic::new(
        Severity::Warning,
        code::CODE_PACKAGE_DIAGNOSTIC,
        SOURCE_PACKAGE,
        "package disabled at runtime: rpc demo diagnostic",
    );
    let (command_tx, mut output_rx, task) = runner_with_runtime_packages(
        MockProvider::new("mock", Vec::new()),
        ExtensionRegistry::new(),
        vec![diagnostic],
    );

    let _header = recv_rpc_line(&mut output_rx).await;
    command_tx
        .send(RpcCommand::session_info {
            id: Some("diag-1".into()),
        })
        .unwrap();
    let resp = recv_response(&mut output_rx, "session_info").await;
    assert_eq!(resp["success"], true);
    let diagnostics = resp["data"]["resources"]["diagnostics"]
        .as_array()
        .expect("session_info resources.diagnostics");
    assert!(
        diagnostics
            .iter()
            .any(|d| d["code"] == code::CODE_PACKAGE_DIAGNOSTIC
                && d["message"] == "package disabled at runtime: rpc demo diagnostic"),
        "session_info resources.diagnostics must include the injected diagnostic: {diagnostics:?}"
    );

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

// ---------------------------------------------------------------------------
// Phase 6 (task 6.5): adapter-backed command consistency through RPC.
//
// Adapter-backed commands route through the shared
// CodingHarness::dispatch_extension_command abstraction (rpc.rs calls it for
// every extension_command). A stateful add->list sequence against a real
// process adapter proves the shared dispatch path maintains adapter state
// consistently through the RPC command path. (NonInteractive/CLI have no
// extension-command dispatch, so consistency is proven at the shared
// abstraction the RPC path relies on.)
// ---------------------------------------------------------------------------

async fn start_todo_registry() -> (Arc<AdapterHost>, ExtensionRegistry) {
    let config = AdapterProcessConfig {
        command: test_binary_path("package_adapter_example"),
        args: vec!["todo".to_string()],
        working_dir: std::env::current_dir().expect("cwd"),
        env: vec![],
    };
    let host = AdapterHost::start("todo", config, Duration::from_secs(10))
        .await
        .expect("start adapter");
    let caps = host.capabilities().clone();
    let host = Arc::new(host);
    let adapter = ProcessAdapter::from_host("todo", host.clone(), caps);
    let mut registry = ExtensionRegistry::new();
    registry.register(Box::new(adapter)).expect("register");
    (host, registry)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_adapter_backed_commands_dispatch_consistently_through_shared_abstraction() {
    let (_host, registry) = start_todo_registry().await;
    let (command_tx, mut output_rx, task) =
        runner_with_runtime_packages(MockProvider::new("mock", Vec::new()), registry, Vec::new());
    let _header = recv_rpc_line(&mut output_rx).await;

    command_tx
        .send(RpcCommand::extension_command {
            id: Some("add-1".into()),
            name: "todo/add".into(),
            args: serde_json::json!({"title": "rpc todo", "description": "consistent"}),
        })
        .unwrap();
    let add_resp = recv_response(&mut output_rx, "extension_command").await;
    assert_eq!(
        add_resp["success"], true,
        "todo/add should succeed: {add_resp}"
    );
    assert_eq!(add_resp["id"], "add-1");

    command_tx
        .send(RpcCommand::extension_command {
            id: Some("list-1".into()),
            name: "todo/list".into(),
            args: serde_json::json!({}),
        })
        .unwrap();
    let list_resp = recv_response(&mut output_rx, "extension_command").await;
    assert_eq!(
        list_resp["success"], true,
        "todo/list should succeed: {list_resp}"
    );
    assert_eq!(list_resp["id"], "list-1");
    let items = list_resp["data"]["items"]
        .as_array()
        .expect("todo/list data.items");
    assert!(
        items.iter().any(|item| item["title"] == "rpc todo"),
        "todo/list through the RPC dispatch abstraction must reflect the prior todo/add: {items:?}"
    );

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _quit = recv_response(&mut output_rx, "quit").await;
    assert_eq!(task.await.unwrap(), 0);
}

// ===========================================================================
// Phase 7 task 7.5 - RPC exposure: startup diagnostics + run-summary counts,
// and the versioned redacted trace envelope (supported + unsupported paths).
// ===========================================================================

mod phase7 {
    use super::{recv_response, recv_rpc_line, recv_until_agent_end, wait_for_idle_session_info};
    use std::sync::Arc;

    use opi_agent::diagnostic::{Diagnostic, SOURCE_PACKAGE, SOURCE_SESSION, Severity, code};
    use opi_agent::evidence::InMemoryEvidenceSink;
    use opi_agent::extension::ExtensionRegistry;
    use opi_agent::session::{SessionHeader, SessionWriter};
    use opi_ai::provider::ProviderError;
    use opi_ai::test_support::{self, MockProvider, MockResponse};
    use opi_coding_agent::config::OpiConfig;
    use opi_coding_agent::harness::ResumeInfo;
    use opi_coding_agent::policy::ToolSelection;
    use opi_coding_agent::rpc::{RpcCommand, RpcRunner};
    use opi_coding_agent::runtime_packages::RuntimePackageStartup;

    use tokio::sync::mpsc::unbounded_channel;

    fn empty_resume_info(workspace: &tempfile::TempDir, session_id: &str) -> ResumeInfo {
        let session_path = workspace.path().join(format!("{session_id}.jsonl"));
        let header = SessionHeader::new(
            session_id.into(),
            "2026-06-26T00:00:00Z".into(),
            workspace.path().display().to_string(),
            None,
        );
        {
            let _writer = SessionWriter::create(&session_path, header)
                .expect("create temporary session file");
        }
        ResumeInfo {
            path: session_path,
            session_id: session_id.into(),
            entries: Vec::new(),
            original_cwd: workspace.path().to_path_buf(),
            diagnostics: Vec::new(),
            recorded_model: None,
            recorded_thinking: None,
        }
    }

    /// rpc_ready carries startup diagnostics before any prompt output, and a
    /// run with a retryable error surfaces structured diagnostic counts in a
    /// run_summary event (clauses 1 + 2).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn phase7_startup_diagnostics_and_counts() {
        let provider = MockProvider::new_with_errors(
            "mock",
            vec![
                MockResponse::Error(ProviderError::RateLimited {
                    retry_after_ms: Some(1),
                }),
                MockResponse::Events(test_support::text_response("ok")),
            ],
        );
        let runtime = RuntimePackageStartup {
            extension_registry: ExtensionRegistry::new(),
            installed_packages: Vec::new(),
            diagnostics: vec![Diagnostic::new(
                Severity::Warning,
                code::CODE_PACKAGE_DIAGNOSTIC,
                SOURCE_PACKAGE,
                "phase7 rpc startup",
            )],
            trust_decision: opi_coding_agent::project_trust::TrustDecision::Trusted,
        };
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let mut runner = RpcRunner::new_with_runtime_packages(
            Box::new(provider),
            "mock:mock-model".into(),
            OpiConfig::default(),
            workspace.path().to_path_buf(),
            false,
            ToolSelection::Disabled,
            None,
            Vec::new(),
            runtime,
            None,
            Vec::new(),
        )
        .expect("rpc runner");

        let (command_tx, command_rx) = unbounded_channel();
        let (output_tx, mut output_rx) = unbounded_channel();
        let task =
            tokio::spawn(async move { runner.run_with_channels(command_rx, output_tx).await });

        // rpc_ready is the first line and carries startup_diagnostics.
        let ready = recv_rpc_line(&mut output_rx).await;
        assert_eq!(ready["type"], "rpc_ready", "first line is rpc_ready");
        let startup = ready["startup_diagnostics"]
            .as_array()
            .expect("startup_diagnostics array");
        assert!(
            startup.iter().any(|d| d["message"] == "phase7 rpc startup"
                && d["code"] == code::CODE_PACKAGE_DIAGNOSTIC),
            "startup diagnostic present before any prompt output: {startup:?}"
        );

        command_tx
            .send(RpcCommand::prompt {
                id: None,
                message: "hi".into(),
            })
            .unwrap();
        let accepted = recv_response(&mut output_rx, "prompt").await;
        assert_eq!(accepted["success"], true, "prompt accepted");

        // The run produces a run_summary event carrying structured counts after
        // the turn completes. Drain lines until we see it.
        let mut saw_counts = false;
        for _ in 0..64 {
            let line = recv_rpc_line(&mut output_rx).await;
            if line["type"] == "run_summary" {
                let diags = line["diagnostics"]
                    .as_object()
                    .expect("run_summary diagnostics object");
                assert!(diags.contains_key("info"), "info count present");
                assert!(diags.contains_key("warning"), "warning count present");
                assert!(diags.contains_key("error"), "error count present");
                let warning = diags["warning"].as_u64().unwrap();
                assert!(
                    warning >= 1,
                    "expected >=1 warning (retry attempt), got {warning}"
                );
                saw_counts = true;
                break;
            }
        }
        assert!(
            saw_counts,
            "expected a run_summary event with structured diagnostic counts"
        );

        command_tx.send(RpcCommand::quit { id: None }).unwrap();
        let _ = task.await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn phase7_rpc_resume_diagnostics_are_startup_diagnostics() {
        let provider = MockProvider::new("mock", Vec::new());
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let resume_info = ResumeInfo {
            path: workspace.path().join("resume.jsonl"),
            session_id: "resume".into(),
            entries: Vec::new(),
            original_cwd: workspace.path().to_path_buf(),
            diagnostics: vec![Diagnostic::new(
                Severity::Warning,
                code::CODE_SESSION_TRUNCATED_LINE,
                SOURCE_SESSION,
                "session file ended with a truncated line",
            )],
            recorded_model: None,
            recorded_thinking: None,
        };
        let mut runner = RpcRunner::new_with_runtime_packages(
            Box::new(provider),
            "mock:mock-model".into(),
            OpiConfig::default(),
            workspace.path().to_path_buf(),
            false,
            ToolSelection::Disabled,
            None,
            Vec::new(),
            RuntimePackageStartup {
                extension_registry: ExtensionRegistry::new(),
                installed_packages: Vec::new(),
                diagnostics: Vec::new(),
                trust_decision: opi_coding_agent::project_trust::TrustDecision::Trusted,
            },
            Some(resume_info),
            Vec::new(),
        )
        .expect("rpc runner");

        let (command_tx, command_rx) = unbounded_channel();
        let (output_tx, mut output_rx) = unbounded_channel();
        let task =
            tokio::spawn(async move { runner.run_with_channels(command_rx, output_tx).await });

        let ready = recv_rpc_line(&mut output_rx).await;
        let startup = ready["startup_diagnostics"]
            .as_array()
            .expect("startup diagnostics array");
        assert!(
            startup
                .iter()
                .any(|d| d["code"] == code::CODE_SESSION_TRUNCATED_LINE
                    && d["source"] == SOURCE_SESSION),
            "resume recovery diagnostic should be emitted in rpc_ready: {startup:?}"
        );

        command_tx.send(RpcCommand::quit { id: None }).unwrap();
        let _ = task.await;
    }

    /// The trace command returns a versioned redacted envelope when tracing is
    /// enabled, and a structured unsupported_trace_request error otherwise
    /// (clauses 3, 4, 6).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn phase7_trace_request_supported_and_unsupported_paths() {
        // --- Supported path: runner built WITH an evidence recorder. ---
        let trace_sink = Arc::new(InMemoryEvidenceSink::new());
        let provider = MockProvider::new("mock", vec![test_support::text_response("hi")]);
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let mut runner = RpcRunner::new_with_trace(
            Box::new(provider),
            "mock:mock-model".into(),
            OpiConfig::default(),
            workspace.path().to_path_buf(),
            false,
            ToolSelection::Disabled,
            None,
            Vec::new(),
            Some(trace_sink.clone()),
            opi_coding_agent::project_trust::TrustDecision::Trusted,
        )
        .expect("rpc runner with trace");

        let (command_tx, command_rx) = unbounded_channel();
        let (output_tx, mut output_rx) = unbounded_channel();
        let task =
            tokio::spawn(async move { runner.run_with_channels(command_rx, output_tx).await });

        let _ready = recv_rpc_line(&mut output_rx).await; // rpc_ready
        command_tx
            .send(RpcCommand::prompt {
                id: None,
                message: "hi".into(),
            })
            .unwrap();
        let _ = recv_response(&mut output_rx, "prompt").await;
        recv_until_agent_end(&mut output_rx).await;

        // The run was traced into the shared sink.
        assert!(
            !trace_sink.records().is_empty(),
            "a traced run must produce trace records"
        );

        // Supported trace request returns a versioned envelope.
        command_tx
            .send(RpcCommand::trace {
                id: Some("t1".into()),
            })
            .unwrap();
        let resp = recv_response(&mut output_rx, "trace").await;
        assert_eq!(resp["success"], true, "trace request succeeds when enabled");
        let records = resp["data"]["records"]
            .as_array()
            .expect("envelope records array");
        assert!(!records.is_empty(), "envelope carries trace records");

        command_tx.send(RpcCommand::quit { id: None }).unwrap();
        let _ = task.await;

        // --- Unsupported path: runner built WITHOUT a trace sink. ---
        let provider2 = MockProvider::new("mock", vec![test_support::text_response("hi")]);
        let workspace2 = tempfile::tempdir().expect("workspace tempdir");
        let mut runner2 = RpcRunner::new_with_trace(
            Box::new(provider2),
            "mock:mock-model".into(),
            OpiConfig::default(),
            workspace2.path().to_path_buf(),
            false,
            ToolSelection::Disabled,
            None,
            Vec::new(),
            None,
            opi_coding_agent::project_trust::TrustDecision::Trusted,
        )
        .expect("rpc runner without trace");

        let (command_tx2, command_rx2) = unbounded_channel();
        let (output_tx2, mut output_rx2) = unbounded_channel();
        let task2 =
            tokio::spawn(async move { runner2.run_with_channels(command_rx2, output_tx2).await });

        let _ready2 = recv_rpc_line(&mut output_rx2).await; // rpc_ready
        command_tx2
            .send(RpcCommand::trace {
                id: Some("t2".into()),
            })
            .unwrap();
        let resp2 = recv_response(&mut output_rx2, "trace").await;
        assert_eq!(resp2["success"], false, "trace fails when not enabled");
        assert_eq!(
            resp2["error_code"], "unsupported_trace_request",
            "structured error code for unsupported trace"
        );

        command_tx2.send(RpcCommand::quit { id: None }).unwrap();
        let _ = task2.await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn phase7_runtime_package_rpc_enables_trace_by_default() {
        let provider = MockProvider::new("mock", vec![test_support::text_response("hi")]);
        let runtime = RuntimePackageStartup {
            extension_registry: ExtensionRegistry::new(),
            installed_packages: Vec::new(),
            diagnostics: Vec::new(),
            trust_decision: opi_coding_agent::project_trust::TrustDecision::Trusted,
        };
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let mut runner = RpcRunner::new_with_runtime_packages(
            Box::new(provider),
            "mock:mock-model".into(),
            OpiConfig::default(),
            workspace.path().to_path_buf(),
            false,
            ToolSelection::Disabled,
            None,
            Vec::new(),
            runtime,
            None,
            Vec::new(),
        )
        .expect("rpc runner");

        let (command_tx, command_rx) = unbounded_channel();
        let (output_tx, mut output_rx) = unbounded_channel();
        let task =
            tokio::spawn(async move { runner.run_with_channels(command_rx, output_tx).await });

        let _ready = recv_rpc_line(&mut output_rx).await;
        command_tx
            .send(RpcCommand::prompt {
                id: None,
                message: "hi".into(),
            })
            .unwrap();
        let _ = recv_response(&mut output_rx, "prompt").await;
        recv_until_agent_end(&mut output_rx).await;

        command_tx
            .send(RpcCommand::trace {
                id: Some("trace-default".into()),
            })
            .unwrap();
        let resp = recv_response(&mut output_rx, "trace").await;
        assert_eq!(
            resp["success"], true,
            "production runtime-package RPC path should support trace"
        );
        assert!(
            resp["data"]["records"]
                .as_array()
                .is_some_and(|records| !records.is_empty()),
            "trace response should carry records"
        );

        command_tx.send(RpcCommand::quit { id: None }).unwrap();
        let _ = task.await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn phase7_rpc_manual_compaction_response_carries_diagnostic() {
        let provider = MockProvider::new("mock", vec![test_support::text_response("hi")]);
        let runtime = RuntimePackageStartup {
            extension_registry: ExtensionRegistry::new(),
            installed_packages: Vec::new(),
            diagnostics: Vec::new(),
            trust_decision: opi_coding_agent::project_trust::TrustDecision::Trusted,
        };
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let resume_info = empty_resume_info(&workspace, "compact-manual");
        let mut runner = RpcRunner::new_with_runtime_packages(
            Box::new(provider),
            "mock:mock-model".into(),
            OpiConfig::default(),
            workspace.path().to_path_buf(),
            false,
            ToolSelection::Disabled,
            None,
            Vec::new(),
            runtime,
            Some(resume_info),
            Vec::new(),
        )
        .expect("rpc runner");

        let (command_tx, command_rx) = unbounded_channel();
        let (output_tx, mut output_rx) = unbounded_channel();
        let task =
            tokio::spawn(async move { runner.run_with_channels(command_rx, output_tx).await });

        let _ready = recv_rpc_line(&mut output_rx).await;
        command_tx
            .send(RpcCommand::prompt {
                id: Some("p1".into()),
                message: "hi".into(),
            })
            .unwrap();
        assert_eq!(
            recv_response(&mut output_rx, "prompt").await["success"],
            true
        );
        recv_until_agent_end(&mut output_rx).await;
        wait_for_idle_session_info(&command_tx, &mut output_rx).await;

        command_tx
            .send(RpcCommand::compact {
                id: Some("compact-1".into()),
            })
            .unwrap();
        let compact = recv_response(&mut output_rx, "compact").await;
        assert_eq!(compact["success"], true, "manual compaction should succeed");
        let diagnostics = compact["data"]["diagnostics"]
            .as_array()
            .expect("compact response should include diagnostics");
        assert!(
            diagnostics
                .iter()
                .any(|d| d["code"] == code::CODE_SESSION_COMPACTED
                    && d["source"] == SOURCE_SESSION),
            "manual compaction diagnostic should be public: {diagnostics:?}"
        );

        command_tx.send(RpcCommand::quit { id: None }).unwrap();
        let _ = task.await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn phase7_rpc_manual_compaction_noop_response_carries_diagnostic() {
        let provider = MockProvider::new("mock", Vec::new());
        let runtime = RuntimePackageStartup {
            extension_registry: ExtensionRegistry::new(),
            installed_packages: Vec::new(),
            diagnostics: Vec::new(),
            trust_decision: opi_coding_agent::project_trust::TrustDecision::Trusted,
        };
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let resume_info = empty_resume_info(&workspace, "compact-empty");
        let mut runner = RpcRunner::new_with_runtime_packages(
            Box::new(provider),
            "mock:mock-model".into(),
            OpiConfig::default(),
            workspace.path().to_path_buf(),
            false,
            ToolSelection::Disabled,
            None,
            Vec::new(),
            runtime,
            Some(resume_info),
            Vec::new(),
        )
        .expect("rpc runner");

        let (command_tx, command_rx) = unbounded_channel();
        let (output_tx, mut output_rx) = unbounded_channel();
        let task =
            tokio::spawn(async move { runner.run_with_channels(command_rx, output_tx).await });

        let _ready = recv_rpc_line(&mut output_rx).await;
        command_tx
            .send(RpcCommand::compact {
                id: Some("compact-empty".into()),
            })
            .unwrap();
        let compact = recv_response(&mut output_rx, "compact").await;
        assert_eq!(
            compact["success"], true,
            "manual compaction no-op should succeed"
        );
        let diagnostics = compact["data"]["diagnostics"]
            .as_array()
            .expect("compact no-op response should include diagnostics");
        assert!(
            diagnostics
                .iter()
                .any(|d| d["code"] == code::CODE_COMPACTION_NOTHING_TO_COMPACT
                    && d["source"] == SOURCE_SESSION),
            "manual compaction no-op diagnostic should be public: {diagnostics:?}"
        );

        command_tx.send(RpcCommand::quit { id: None }).unwrap();
        let _ = task.await;
    }

    /// DoD SC6 (RPC trace surface): a requested trace envelope over a run whose
    /// prompt embeds every sensitive class (API key, GitHub token, credentialed
    /// URL, bearer/JWT) contains none of them. The envelope is serialized via
    /// the shared redaction boundary, so this guards both structural-metadata
    /// emission and any future diagnostic-details leak.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn phase7_rpc_trace_redacts_sensitive_values() {
        let secrets = [
            "sk-ant-1234567890abcdefghijklmnopqrstuv",
            "ghp_01234567890123456789012345678901234567",
            "https://alice:s3cr3t@gitlab.example.com/o/r.git",
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIg.abc123def456",
        ];
        let prompt = format!(
            "rotate now: {} {} {} {}",
            secrets[0], secrets[1], secrets[2], secrets[3]
        );

        let trace_sink = Arc::new(InMemoryEvidenceSink::new());
        let provider = MockProvider::new("mock", vec![test_support::text_response("hi")]);
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let mut runner = RpcRunner::new_with_trace(
            Box::new(provider),
            "mock:mock-model".into(),
            OpiConfig::default(),
            workspace.path().to_path_buf(),
            false,
            ToolSelection::Disabled,
            None,
            Vec::new(),
            Some(trace_sink.clone()),
            opi_coding_agent::project_trust::TrustDecision::Trusted,
        )
        .expect("rpc runner with trace");

        let (command_tx, command_rx) = unbounded_channel();
        let (output_tx, mut output_rx) = unbounded_channel();
        let task =
            tokio::spawn(async move { runner.run_with_channels(command_rx, output_tx).await });

        let _ready = recv_rpc_line(&mut output_rx).await; // rpc_ready
        command_tx
            .send(RpcCommand::prompt {
                id: None,
                message: prompt.clone(),
            })
            .unwrap();
        let _ = recv_response(&mut output_rx, "prompt").await;
        recv_until_agent_end(&mut output_rx).await;

        command_tx
            .send(RpcCommand::trace {
                id: Some("t-redact".into()),
            })
            .unwrap();
        let resp = recv_response(&mut output_rx, "trace").await;
        assert_eq!(resp["success"], true, "trace request succeeds when enabled");
        let envelope = serde_json::to_string(&resp["data"]).unwrap_or_default();
        for secret in secrets {
            assert!(
                !envelope.contains(secret),
                "RPC trace envelope leaked a sensitive value: {secret}\n--- envelope ---\n{envelope}",
            );
        }

        command_tx.send(RpcCommand::quit { id: None }).unwrap();
        let _ = task.await;
    }

    /// Across multiple RPC runs in one session, run_summary counts are scoped
    /// per run (not cumulative) and each run_summary is preceded by AgentEnd.
    /// Pins the two evaluator-confirmed fixes for Phase 7 task 7.5.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn phase7_run_summary_per_run_counts_and_after_agent_end() {
        // Two prompt runs; each consumes a retryable error then a success.
        let provider = MockProvider::new_with_errors(
            "mock",
            vec![
                MockResponse::Error(ProviderError::RateLimited {
                    retry_after_ms: Some(1),
                }),
                MockResponse::Events(test_support::text_response("r1")),
                MockResponse::Error(ProviderError::RateLimited {
                    retry_after_ms: Some(1),
                }),
                MockResponse::Events(test_support::text_response("r2")),
            ],
        );
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let mut runner = RpcRunner::new(
            Box::new(provider),
            "mock:mock-model".into(),
            OpiConfig::default(),
            workspace.path().to_path_buf(),
            false,
            ToolSelection::Disabled,
            None,
            Vec::new(),
            opi_coding_agent::project_trust::TrustDecision::Trusted,
        )
        .expect("rpc runner");
        let (command_tx, command_rx) = unbounded_channel();
        let (output_tx, mut output_rx) = unbounded_channel();
        let task =
            tokio::spawn(async move { runner.run_with_channels(command_rx, output_tx).await });

        let _ready = recv_rpc_line(&mut output_rx).await; // rpc_ready

        for run in 1..=2u32 {
            command_tx
                .send(RpcCommand::prompt {
                    id: None,
                    message: format!("p{run}"),
                })
                .unwrap();
            let _accepted = recv_response(&mut output_rx, "prompt").await;

            // Collect lines until the run_summary; AgentEnd must precede it,
            // and the per-run warning count must be exactly 1 (not accumulated
            // across the two runs).
            let mut saw_agent_end = false;
            let mut summary_warning: Option<u64> = None;
            for _ in 0..64 {
                let line = recv_rpc_line(&mut output_rx).await;
                match line["type"].as_str() {
                    Some("AgentEnd") => saw_agent_end = true,
                    Some("run_summary") => {
                        summary_warning = line["diagnostics"]["warning"].as_u64();
                        break;
                    }
                    _ => {}
                }
            }
            assert!(
                saw_agent_end,
                "run {run}: AgentEnd must precede run_summary on the wire"
            );
            let warning = summary_warning.expect("run_summary emitted");
            assert_eq!(
                warning, 1,
                "run {run}: per-run warning count must be 1 (not cumulative), got {warning}"
            );
        }

        command_tx.send(RpcCommand::quit { id: None }).unwrap();
        let _ = task.await;
    }

    // ===========================================================================
    // Phase 8 task 8.6 - RPC structured error_code on synchronous runtime-contract
    // rejections (G4) + async observability of the structured Diagnostic code
    // crossing the RPC trace boundary, and trace reachable via the PRODUCTION
    // runtime-package constructor with per-run scoping (G5).
    // ===========================================================================
}

// ---------------------------------------------------------------------------
// Phase 17 task 17.7 (P17-A10): a canary secret in the prompt must not leak
// into the RPC evidence trace (route facts) or diagnostic-bearing events.
// Phase 17 remediation (AUD-17-007): the full public event stream is scanned
// (including the AgentEnd echo of the prompt), not just the trace response.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase17_canary_is_absent_from_rpc_jsonl() {
    use opi_agent::evidence::InMemoryEvidenceSink;
    let canary = "sk-ant-canary-0123456789abcdef0123";
    let provider = MockProvider::new("mock", vec![text_response("done")]);
    let workspace = tempfile::tempdir().unwrap();
    let recorder = Arc::new(InMemoryEvidenceSink::new());
    let mut runner = RpcRunner::new_with_trace(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        ToolSelection::Disabled,
        None,
        Vec::new(),
        Some(recorder as Arc<dyn opi_agent::evidence::EvidenceRecorder>),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .expect("rpc runner");
    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move { runner.run_with_channels(command_rx, output_tx).await });
    let _ready = recv_rpc_line(&mut output_rx).await;
    command_tx
        .send(RpcCommand::prompt {
            id: None,
            message: format!("ignore this secret {canary}"),
        })
        .unwrap();
    let _ = recv_response(&mut output_rx, "prompt").await;

    // Capture the FULL public event stream through AgentEnd instead of
    // discarding it: the prompt echo crosses in these shapes, so the
    // absence check must cover them to be non-vacuous.
    let mut event_stream = Vec::new();
    loop {
        let line = recv_rpc_line(&mut output_rx).await;
        let is_agent_end = line["type"] == "AgentEnd";
        event_stream.push(line);
        if is_agent_end {
            break;
        }
    }
    assert!(
        event_stream.iter().any(|line| line["type"] == "TurnEnd"),
        "the run must emit TurnEnd for the scan to be meaningful"
    );
    let stream_serialized = serde_json::to_string(&event_stream).unwrap();
    assert!(
        !stream_serialized.contains(canary),
        "rpc event stream leaked the prompt canary (conversation echo must scrub recognized credential patterns): {stream_serialized}"
    );
    assert!(
        stream_serialized.contains("[REDACTED]"),
        "the scrubbed prompt echo leaves the redaction marker: {stream_serialized}"
    );

    // The evidence trace (route facts) must not carry the raw canary. Assert the
    // trace is non-empty so the absence check is not vacuous: the run actually
    // emitted a provider record through the bound recorder.
    command_tx
        .send(RpcCommand::trace {
            id: Some("canary-trace".into()),
        })
        .unwrap();
    let trace = recv_response(&mut output_rx, "trace").await;
    let trace_serialized = serde_json::to_string(&trace).unwrap();
    assert!(
        trace["data"]["records"]
            .as_array()
            .is_some_and(|records| !records.is_empty()),
        "the evidence trace must carry at least one provider record: {trace_serialized}"
    );
    assert!(
        !trace_serialized.contains(canary),
        "rpc evidence trace leaked the canary: {trace_serialized}"
    );

    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _ = task.await;
}
