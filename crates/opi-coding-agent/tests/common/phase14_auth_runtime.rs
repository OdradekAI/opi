//! Shared Phase 14 runtime-auth integration-test plumbing.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use opi_ai::provider::ProviderError;
use opi_ai::test_support::{MockProvider, MockResponse};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::policy::ToolSelection;
use opi_coding_agent::rpc::RpcRunner;
use opi_coding_agent::runner::NonInteractiveRunner;

pub fn credential_runner(workspace: &Path) -> NonInteractiveRunner {
    let provider = MockProvider::new_with_errors(
        "anthropic",
        vec![MockResponse::Error(ProviderError::CredentialNeeded {
            provider_id: "anthropic".into(),
        })],
    );
    NonInteractiveRunner::new(
        Box::new(provider),
        "anthropic:claude-sonnet-4-5".into(),
        OpiConfig::default(),
        workspace.to_path_buf(),
        false,
        None,
        Vec::new(),
    )
}

pub async fn run_json_credential_capture(mut runner: NonInteractiveRunner) -> serde_json::Value {
    let result = tokio::time::timeout(Duration::from_secs(2), runner.run_json("hello"))
        .await
        .expect("JSON mode must not prompt or block");
    serde_json::json!({
        "entry_point": "NonInteractiveRunner::run_json",
        "exit_code": result.exit_code,
        "stdout": result.stdout,
        "stderr": result.stderr,
    })
}

pub async fn run_text_credential_capture(mut runner: NonInteractiveRunner) -> serde_json::Value {
    let result = tokio::time::timeout(Duration::from_secs(2), runner.run("hello"))
        .await
        .expect("text mode must not prompt or block");
    serde_json::json!({
        "entry_point": "NonInteractiveRunner::run",
        "exit_code": result.exit_code,
        "stdout": result.stdout,
        "stderr": result.stderr,
    })
}

pub async fn run_rpc_stdio_child() {
    let workspace = tempfile::tempdir().expect("RPC child workspace");
    let provider = MockProvider::new_with_errors(
        "anthropic",
        vec![MockResponse::Error(ProviderError::CredentialNeeded {
            provider_id: "anthropic".into(),
        })],
    );
    let mut runner = RpcRunner::new(
        Box::new(provider),
        "anthropic:claude-sonnet-4-5".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        ToolSelection::Disabled,
        None,
        Vec::new(),
    )
    .expect("construct RPC runner");
    assert_eq!(runner.run().await, 0);
}

struct ChildCleanup {
    child: Child,
}

impl Drop for ChildCleanup {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

pub fn run_rpc_stdio_capture(child_test_name: &str) -> Vec<serde_json::Value> {
    let child = Command::new(std::env::current_exe().expect("current integration test binary"))
        .args([
            "--exact",
            child_test_name,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn RpcRunner::run child");
    let mut child = ChildCleanup { child };
    let stdout = child.child.stdout.take().expect("child stdout");
    let (line_tx, line_rx) = mpsc::channel();
    let stdout_task = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if line_tx.send(line.expect("read RPC child stdout")).is_err() {
                break;
            }
        }
    });
    let stderr = child.child.stderr.take().expect("child stderr");
    let stderr_task = std::thread::spawn(move || {
        BufReader::new(stderr)
            .lines()
            .collect::<Result<Vec<_>, _>>()
            .expect("read RPC child stderr")
    });
    let mut stdin = child.child.stdin.take().expect("child stdin");
    let mut records = Vec::new();
    recv_rpc_until(&line_rx, &mut records, |value| value["type"] == "rpc_ready");
    writeln!(
        stdin,
        "{}",
        serde_json::json!({"type":"prompt","id":"auth-1","message":"hello"})
    )
    .unwrap();
    stdin.flush().unwrap();
    recv_rpc_until(&line_rx, &mut records, |value| {
        value["type"] == "CredentialNeeded"
    });
    writeln!(
        stdin,
        "{}",
        serde_json::json!({"type":"quit","id":"quit-1"})
    )
    .unwrap();
    stdin.flush().unwrap();
    recv_rpc_until(&line_rx, &mut records, |value| {
        value["type"] == "response" && value["command"] == "quit"
    });
    drop(stdin);

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.child.try_wait().expect("poll RPC child") {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            panic!("RpcRunner::run child exceeded five-second bound");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    stdout_task.join().expect("join stdout reader");
    let stderr = stderr_task.join().expect("join stderr reader");
    assert!(status.success(), "RPC child failed: {stderr:?}");
    records
}

fn recv_rpc_until(
    lines: &mpsc::Receiver<String>,
    records: &mut Vec<serde_json::Value>,
    predicate: impl Fn(&serde_json::Value) -> bool,
) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        let line = lines
            .recv_timeout(remaining)
            .unwrap_or_else(|error| panic!("timed out waiting for RPC record: {error}"));
        let Some(start) = line.find('{') else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line[start..]) else {
            continue;
        };
        let done = predicate(&value);
        records.push(value);
        if done {
            return;
        }
    }
}
