#!/usr/bin/env python3
"""Execute the three package-shipped onboarding sources against controlled peers.

Cargo creates the actual opi-ai artifact; a fresh workspace-external ordinary
consumer compiles its exact examples and runs their shared setup, call
construction and execution functions. Recording injected auth, memory store and
loopback peers prove source-free setup/static selection, actual pre-poll
laziness, final response/stream delivery and the tool-result second request.
No live provider, real credential, root patch or dependency is introduced.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
import tomllib
from pathlib import Path
from typing import Any

import run_early_package_proof as shared
from package_slice import check_cargo_configs, controlled_environment

HARNESS = shared.HARNESS
REPO = shared.REPO

EXAMPLES = ("quick_start.rs", "quick_start_stream.rs", "tool_loop.rs")

EXPECTED_CASES = ("quick_start", "quick_start_stream", "tool_loop")
EXPECTED_TEXT = {
    "quick_start": "Hello world",
    "quick_start_stream": "Hello world",
    "tool_loop": "Tokyo: 22 C, clear.",
}
PROOF_PREFIX = "OPI_README_PROOF="



def execution_environment(environment: dict[str, str]) -> tuple[dict[str, str], dict[str, Any]]:
    """Isolate the controlled peer subprocess without mutating host settings."""
    proxy_names = {"HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "NO_PROXY"}
    removed = sorted(name for name in environment if name.upper() in proxy_names)
    controlled = {name: value for name, value in environment.items() if name.upper() not in proxy_names}
    bypass = "localhost,127.0.0.1,::1"
    controlled.update(NO_PROXY=bypass, no_proxy=bypass)
    return controlled, {"removed_proxy_variable_names": removed,
                        "loopback_bypass": {"NO_PROXY": bypass, "no_proxy": bypass}}


def check_packaged_sources(repo: Path, package: Path) -> tuple[dict[str, str], dict[str, bool]]:
    """Bind executed sources to the artifact and both exact README embeds."""
    examples = {}
    for name in EXAMPLES:
        source = (package / "examples" / name).read_bytes()
        if source != (repo / "crates/opi-ai/examples" / name).read_bytes():
            raise ValueError(f"packaged example {name} differs from the workspace source")
        examples[name] = hashlib.sha256(source).hexdigest()
    manuals = {}
    for manual in ("README.md", "README.zh.md"):
        text = (package / manual).read_text(encoding="utf-8")
        embedded = all((package / "examples" / name).read_text(encoding="utf-8").rstrip("\n") in text
                       for name in EXAMPLES)
        manuals[manual] = embedded
        if not embedded:
            raise ValueError(f"{manual} does not embed the exact packaged example sources")
    return examples, manuals


def check_report(report: dict[str, Any]) -> None:
    """A compiled source or omitted continuation is never executed acceptance."""
    checks = report.get("checks") if isinstance(report, dict) else None
    if (not isinstance(checks, list) or len(checks) != len(EXPECTED_CASES)
            or [item.get("name") for item in checks if isinstance(item, dict)] != list(EXPECTED_CASES)
            or report.get("schema") != "opi-readme-execution/1"
            or report.get("verdict") != "PASS" or report.get("total") != 3 or report.get("failed") != 0):
        raise ValueError("consumer did not execute exactly the three named onboarding flows")
    idle = {"source_reads": 0, "store_reads": 0, "requests": 0}
    for case in checks:
        if case.get("passed") is not True or case.get("setup") != idle or case.get("unpolled") != idle:
            raise ValueError(f"{case['name']} lacks zero-I/O setup and actual unpolled-call proof")
        observed = case.get("observed", {})
        expected_requests = 2 if case["name"] == "tool_loop" else 1
        requests = case.get("requests")
        if (not isinstance(observed, dict)
                or not isinstance(observed.get("source_reads"), int) or observed["source_reads"] <= 0
                or not isinstance(observed.get("store_reads"), int) or observed["store_reads"] <= 0
                or observed.get("requests") != expected_requests
                or not isinstance(requests, list) or len(requests) != expected_requests
                or case.get("final_text") != EXPECTED_TEXT[case["name"]]):
            raise ValueError(f"{case['name']} lacks actual auth, sends and expected final outcome")
        for request in requests:
            if (not isinstance(request, dict) or request.get("method") != "POST"
                    or request.get("target") != "/v1/messages"
                    or request.get("headers", {}).get("x-api-key") != "readme-synthetic-key"
                    or request.get("body", {}).get("model") != "claude-sonnet-4-5"):
                raise ValueError(f"{case['name']} has an unexpected controlled request")
        if case["name"] == "tool_loop":
            if case.get("continuation") is not True:
                raise ValueError("tool_loop did not execute its tool-result continuation")
            messages = requests[1].get("body", {}).get("messages", [])
            blocks = [block for message in messages if isinstance(message, dict)
                      for block in message.get("content", []) if isinstance(block, dict)]
            call = next((block for block in blocks if block.get("type") == "tool_use"), {})
            result = next((block for block in blocks if block.get("type") == "tool_result"), {})
            if (call.get("id") != "weather-1" or call.get("name") != "get_weather"
                    or call.get("input") != {"city": "Tokyo"}
                    or result.get("tool_use_id") != "weather-1"
                    or result.get("content") != "22 C, clear" or result.get("is_error") is True):
                raise ValueError("tool_loop second request lost the admitted call or tool result")


def parse_execution(stdout: str) -> dict[str, Any]:
    reports = [line[len(PROOF_PREFIX):] for line in stdout.splitlines() if line.startswith(PROOF_PREFIX)]
    if len(reports) != 1:
        raise ValueError("consumer did not emit exactly one executed onboarding report")
    report = json.loads(reports[0])
    check_report(report)
    return report


CONSUMER_SOURCE = r'''
#[allow(dead_code)]
#[path = "bin/quick_start.rs"]
mod quick_start;
#[allow(dead_code)]
#[path = "bin/quick_start_stream.rs"]
mod quick_start_stream;
#[allow(dead_code)]
#[path = "bin/tool_loop.rs"]
mod tool_loop;

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use opi_ai::http::HttpClient;
use opi_ai::message::{AssistantContent, AssistantMessage};
use opi_ai::mutation::InMemoryCredentialStore;
use opi_ai::provider_collection::BuiltinOverrides;
use opi_ai::stream::StopReason;
use opi_ai::{AuthContext, AuthContextError};
use serde_json::{Value, json};

#[derive(Default)]
struct RecordingContext {
    reads: AtomicUsize,
}

impl AuthContext for RecordingContext {
    fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok((name == "ANTHROPIC_API_KEY").then(|| "readme-synthetic-key".into()))
    }
    fn file_exists(&self, _path: &str) -> Result<bool, AuthContextError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(false)
    }
    fn home_dir(&self) -> Result<Option<std::path::PathBuf>, AuthContextError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(None)
    }
}

#[derive(Default)]
struct PeerState {
    requests: Vec<Value>,
    errors: Vec<String>,
}

struct Peer {
    url: String,
    state: Arc<Mutex<PeerState>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Peer {
    fn new(responses: Vec<String>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind controlled peer");
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let state = Arc::new(Mutex::new(PeerState::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_state = state.clone();
        let worker_stop = stop.clone();
        let worker = thread::spawn(move || {
            while !worker_stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut socket, _)) => {
                        socket
                            .set_read_timeout(Some(Duration::from_secs(10)))
                            .unwrap();
                        socket
                            .set_write_timeout(Some(Duration::from_secs(10)))
                            .unwrap();
                        match read_request(&mut socket) {
                            Ok(request) => {
                                let ordinal = {
                                    let mut state = worker_state.lock().unwrap();
                                    let ordinal = state.requests.len();
                                    state.requests.push(request);
                                    ordinal
                                };
                                let (status, body) = match responses.get(ordinal) {
                                    Some(body) => ("200 OK", body.as_str()),
                                    None => ("500 Unexpected Request", "unexpected extra send"),
                                };
                                let response = format!(
                                    "HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                                    body.len()
                                );
                                if let Err(error) = socket.write_all(response.as_bytes()) {
                                    worker_state.lock().unwrap().errors.push(error.to_string());
                                }
                            }
                            Err(error) => {
                                worker_state.lock().unwrap().errors.push(error.to_string())
                            }
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(error) => {
                        worker_state.lock().unwrap().errors.push(error.to_string());
                        return;
                    }
                }
            }
        });
        Self {
            url,
            state,
            stop,
            worker: Some(worker),
        }
    }

    fn requests(&self) -> Vec<Value> {
        let state = self.state.lock().unwrap();
        assert!(
            state.errors.is_empty(),
            "controlled peer errors: {:?}",
            state.errors
        );
        state.requests.clone()
    }

    fn overrides(&self) -> BuiltinOverrides {
        let mut overrides = BuiltinOverrides::default();
        overrides
            .base_urls
            .insert("anthropic".into(), self.url.clone());
        overrides
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            worker.join().expect("controlled peer thread");
        }
    }
}

fn read_request(socket: &mut TcpStream) -> std::io::Result<Value> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    let header_end = loop {
        if let Some(index) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            break index + 4;
        }
        let size = socket.read(&mut buffer)?;
        if size == 0 || bytes.len() > 1024 * 1024 {
            return Err(std::io::Error::other("missing controlled request headers"));
        }
        bytes.extend_from_slice(&buffer[..size]);
    };
    let head = String::from_utf8(bytes[..header_end].to_vec()).map_err(std::io::Error::other)?;
    let mut lines = head.split("\r\n");
    let line = lines.next().unwrap_or_default();
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or_default();
    let headers: BTreeMap<String, String> = lines
        .filter_map(|line| {
            line.split_once(':')
                .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().into()))
        })
        .collect();
    let length: usize = headers
        .get("content-length")
        .ok_or_else(|| std::io::Error::other("missing content length"))?
        .parse()
        .map_err(std::io::Error::other)?;
    if length > 1024 * 1024 {
        return Err(std::io::Error::other(
            "unexpected oversized fixture request",
        ));
    }
    while bytes.len() < header_end + length {
        let size = socket.read(&mut buffer)?;
        if size == 0 {
            return Err(std::io::Error::other("truncated controlled request"));
        }
        bytes.extend_from_slice(&buffer[..size]);
    }
    let body: Value = serde_json::from_slice(&bytes[header_end..header_end + length])
        .map_err(std::io::Error::other)?;
    Ok(json!({"method": method, "target": target, "headers": headers, "body": body}))
}

fn event(kind: &str, data: Value) -> String {
    format!("event: {kind}\ndata: {data}\n\n")
}

fn response(tool: bool, text: &str) -> String {
    let mut output = event(
        "message_start",
        json!({"type":"message_start","message":{
        "id":"readme-message","type":"message","role":"assistant","content":[],
        "model":"claude-sonnet-4-5","stop_reason":null,
        "usage":{"input_tokens":7,"output_tokens":0}}}),
    );
    if tool {
        output += &event(
            "content_block_start",
            json!({"type":"content_block_start","index":0,
            "content_block":{"type":"tool_use","id":"weather-1","name":"get_weather","input":{}}}),
        );
        output += &event(
            "content_block_delta",
            json!({"type":"content_block_delta","index":0,
            "delta":{"type":"input_json_delta","partial_json":"{\"city\":\"Tokyo\"}"}}),
        );
    } else {
        output += &event(
            "content_block_start",
            json!({"type":"content_block_start","index":0,
            "content_block":{"type":"text","text":""}}),
        );
        output += &event(
            "content_block_delta",
            json!({"type":"content_block_delta","index":0,
            "delta":{"type":"text_delta","text":text}}),
        );
    }
    output += &event(
        "content_block_stop",
        json!({"type":"content_block_stop","index":0}),
    );
    output += &event(
        "message_delta",
        json!({"type":"message_delta",
        "delta":{"stop_reason":if tool {"tool_use"} else {"end_turn"},"stop_sequence":null},
        "usage":{"output_tokens":5}}),
    );
    output += &event("message_stop", json!({"type":"message_stop"}));
    output
}

fn snapshot(context: &RecordingContext, store: &InMemoryCredentialStore, peer: &Peer) -> Value {
    json!({"source_reads":context.reads.load(Ordering::SeqCst),
        "store_reads":store.read_count(),"requests":peer.requests().len()})
}

fn assert_idle(snapshot: &Value) {
    assert_eq!(
        snapshot,
        &json!({"source_reads":0,"store_reads":0,"requests":0})
    );
}

fn finish_check(
    name: &str,
    setup: Value,
    unpolled: Value,
    message: AssistantMessage,
    expected_text: &str,
    expected_requests: usize,
    context: &RecordingContext,
    store: &InMemoryCredentialStore,
    peer: &Peer,
) -> Value {
    let observed = snapshot(context, store, peer);
    assert!(observed["source_reads"].as_u64().unwrap() > 0);
    assert!(observed["store_reads"].as_u64().unwrap() > 0);
    let requests = peer.requests();
    assert_eq!(requests.len(), expected_requests);
    for request in &requests {
        assert_eq!(request["method"], "POST");
        assert_eq!(request["target"], "/v1/messages");
        assert_eq!(request["headers"]["x-api-key"], "readme-synthetic-key");
        assert_eq!(request["body"]["model"], "claude-sonnet-4-5");
    }
    assert_eq!(message.stop_reason, StopReason::Stop);
    assert_eq!(message.provider, "anthropic");
    assert_eq!(message.model, "claude-sonnet-4-5");
    let text: String = message
        .content
        .iter()
        .filter_map(|part| match part {
            AssistantContent::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text, expected_text);
    let continuation = if name == "tool_loop" {
        let messages = requests[1]["body"]["messages"].as_array().unwrap();
        let assistant = messages
            .iter()
            .find(|message| message["role"] == "assistant")
            .unwrap();
        let call = assistant["content"]
            .as_array()
            .unwrap()
            .iter()
            .find(|block| block["type"] == "tool_use")
            .unwrap();
        assert_eq!(call["id"], "weather-1");
        assert_eq!(call["name"], "get_weather");
        assert_eq!(call["input"], json!({"city":"Tokyo"}));
        let result = messages
            .iter()
            .filter(|message| message["role"] == "user")
            .flat_map(|message| message["content"].as_array().unwrap())
            .find(|block| block["type"] == "tool_result")
            .unwrap();
        assert_eq!(result["tool_use_id"], "weather-1");
        assert_eq!(result["content"], "22 C, clear");
        assert_ne!(result["is_error"], true);
        true
    } else {
        false
    };
    json!({"name":name,"passed":true,"setup":setup,"unpolled":unpolled,
        "observed":observed,"final_text":text,"continuation":continuation,"requests":requests})
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut checks = Vec::new();
    {
        let peer = Peer::new(vec![response(false, "Hello world")]);
        let context = Arc::new(RecordingContext::default());
        let store = Arc::new(InMemoryCredentialStore::new());
        let (mut models, record) = quick_start::setup(
            context.clone(),
            Arc::new(HttpClient::new()),
            &peer.overrides(),
        )?;
        models.set_credential_store(store.clone());
        let setup = snapshot(&context, &store, &peer);
        assert_idle(&setup);
        let call = quick_start::start(&models, record)?;
        let unpolled = snapshot(&context, &store, &peer);
        assert_idle(&unpolled);
        let message = quick_start::finish(call).await?;
        checks.push(finish_check(
            "quick_start",
            setup,
            unpolled,
            message,
            "Hello world",
            1,
            &context,
            &store,
            &peer,
        ));
    }
    {
        let peer = Peer::new(vec![response(false, "Hello world")]);
        let context = Arc::new(RecordingContext::default());
        let store = Arc::new(InMemoryCredentialStore::new());
        let (mut models, record) = quick_start_stream::setup(
            context.clone(),
            Arc::new(HttpClient::new()),
            &peer.overrides(),
        )?;
        models.set_credential_store(store.clone());
        let setup = snapshot(&context, &store, &peer);
        assert_idle(&setup);
        let call = quick_start_stream::start(&models, record)?;
        let unpolled = snapshot(&context, &store, &peer);
        assert_idle(&unpolled);
        let message = quick_start_stream::finish(call).await?;
        checks.push(finish_check(
            "quick_start_stream",
            setup,
            unpolled,
            message,
            "Hello world",
            1,
            &context,
            &store,
            &peer,
        ));
    }
    {
        let peer = Peer::new(vec![
            response(true, ""),
            response(false, "Tokyo: 22 C, clear."),
        ]);
        let context = Arc::new(RecordingContext::default());
        let store = Arc::new(InMemoryCredentialStore::new());
        let mut state = tool_loop::setup(
            context.clone(),
            Arc::new(HttpClient::new()),
            &peer.overrides(),
        )?;
        state.models.set_credential_store(store.clone());
        let setup = snapshot(&context, &store, &peer);
        assert_idle(&setup);
        let call = state.start_round()?;
        let unpolled = snapshot(&context, &store, &peer);
        assert_idle(&unpolled);
        let message = state.finish(call).await?;
        checks.push(finish_check(
            "tool_loop",
            setup,
            unpolled,
            message,
            "Tokyo: 22 C, clear.",
            2,
            &context,
            &store,
            &peer,
        ));
    }
    println!(
        "OPI_README_PROOF={}",
        json!({"schema":"opi-readme-execution/1","verdict":"PASS","total":3,"failed":0,"checks":checks})
    );
    Ok(())
}
'''


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--offline", action="store_true",
                        help="required offline mode marker")
    parser.add_argument("--scratch", required=True,
                        help="new workspace-external scratch directory")
    parser.add_argument("--target-dir", default=os.environ.get("CARGO_TARGET_DIR"),
                        help="existing marked workspace cache (or CARGO_TARGET_DIR)")
    args = parser.parse_args()
    if not args.offline:
        parser.error("--offline is required; this proof never resolves from the network")
    if not args.target_dir:
        parser.error("--target-dir or CARGO_TARGET_DIR must identify the existing workspace cache")

    scratch = Path(args.scratch).expanduser().resolve()
    target = Path(args.target_dir).expanduser().resolve()
    try:
        env = controlled_environment(dict(os.environ), target)
        configs = check_cargo_configs(env, [REPO, scratch / "consumer", scratch / "package"])
        shared.validate_paths(REPO, scratch, target)
    except (OSError, ValueError) as exc:
        parser.error(str(exc))
    scratch.mkdir(parents=True)
    env.setdefault("CARGO_BUILD_JOBS", "4")

    evidence: dict[str, Any] = {
        "family": "readme-examples",
        "scope": ("three exact packaged onboarding sources execute lazy builtin setup, "
                  "unpolled calls, final responses/stream and tool-result continuation "
                  "as an ordinary workspace-external consumer (ONB-02)"),
        "scratch": str(scratch),
        "inputs": {"runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                   "consumer_source_sha256": hashlib.sha256(CONSUMER_SOURCE.encode()).hexdigest()},
        "target": str(target),
        "cargo_configs": configs,
        "steps": [],
        "verdict": "FAIL",
    }

    def run(name: str, command: list[str], cwd: Path,
            environment: dict[str, str] | None = None) -> str:
        started = time.monotonic()
        result = subprocess.run(
            command, cwd=cwd, env=env if environment is None else environment,
            capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=3600,
        )
        (scratch / f"{name}.stdout").write_text(result.stdout, encoding="utf-8")
        (scratch / f"{name}.stderr").write_text(result.stderr, encoding="utf-8")
        evidence["steps"].append({
            "name": name, "command": command, "exit": result.returncode,
            "elapsed_seconds": round(time.monotonic() - started, 2),
            "stdout_sha256": hashlib.sha256((scratch / f"{name}.stdout").read_bytes()).hexdigest(),
            "stderr_sha256": hashlib.sha256((scratch / f"{name}.stderr").read_bytes()).hexdigest(),
        })
        if result.returncode:
            raise ValueError(f"{name} failed with exit {result.returncode}; see scratch logs")
        return result.stdout

    lease = [sys.executable, str(REPO / "scripts/opi-cargo-cache.py"), "lease"]
    leased = False
    try:
        evidence["rustc"] = run("rustc-version", ["rustc", "-vV"], REPO)
        evidence["cargo"] = run("cargo-version", ["cargo", "-vV"], REPO)
        run("lease-start", lease + ["start", "--target", str(target), "--pid", str(os.getpid())], REPO)
        leased = True
        version = tomllib.loads(
            (REPO / "Cargo.toml").read_text(encoding="utf-8")
        )["workspace"]["package"]["version"]
        prefix = f"opi-ai-{version}"
        run("package", ["cargo", "package", "-p", "opi-ai", "--allow-dirty", "--no-verify",
                        "--offline", "--locked"], REPO)
        archive = scratch / f"{prefix}.crate"
        archive.write_bytes((target / "package" / archive.name).read_bytes())
        evidence["package"] = {
            "file": archive.name,
            "sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
        }
        package = shared.unpack_package(archive, scratch / "package", prefix)

        examples, manuals = check_packaged_sources(REPO, package)
        evidence["packaged_examples"] = examples
        evidence["manuals_embed_exact_sources"] = manuals

        consumer = scratch / "consumer"
        shutil.copytree(
            HARNESS / "consumer-readme",
            consumer,
            ignore=shutil.ignore_patterns("target", "Cargo.lock"),
        )
        manifest = consumer / "Cargo.toml"
        text = manifest.read_text(encoding="utf-8")
        manifest.write_text(
            text.replace('"@PACKAGE@"', json.dumps(package.as_posix())),
            encoding="utf-8", newline="\n",
        )
        # Modules and runnable standalone bins use the exact packaged sources.
        # The harness calls their shared setup/start/finish functions.
        (consumer / "src" / "bin").mkdir(parents=True)
        for name in EXAMPLES:
            shutil.copyfile(package / "examples" / name, consumer / "src" / "bin" / name)
        (consumer / "src" / "main.rs").write_text(CONSUMER_SOURCE, encoding="utf-8", newline="\n")
        run("consumer-lock", ["cargo", "generate-lockfile", "--offline"], consumer)
        lock = tomllib.loads((consumer / "Cargo.lock").read_text(encoding="utf-8"))
        root_names = {
            entry.get("name") for entry in lock.get("package", [])
        }
        for forbidden in ("opi-agent", "opi-coding-agent", "opi-protocol", "opi-tui"):
            if forbidden in root_names:
                raise ValueError(f"consumer graph must not contain {forbidden}")
        run("consumer-check", ["cargo", "check", "--offline", "--locked", "--bins"], consumer)
        metadata = json.loads(run("consumer-metadata", ["cargo", "metadata", "--offline", "--locked",
                                                       "--format-version", "1"], consumer))
        sdk = [item for item in metadata["packages"] if item["name"] == "opi-ai"]
        if len(sdk) != 1 or Path(sdk[0]["manifest_path"]).resolve() != package / "Cargo.toml":
            raise ValueError("consumer must execute the actual extracted opi-ai package")
        evidence["consumer_dependencies"] = sorted(root_names)
        execution_env, evidence["execution_environment"] = execution_environment(env)
        evidence["execution"] = parse_execution(run(
            "consumer-execute", ["cargo", "run", "--offline", "--locked", "--quiet",
                                 "--bin", "consumer-readme"], consumer, execution_env))
        evidence["verdict"] = "PASS"
        return 0
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        print(f"readme example proof failed: {exc}", file=sys.stderr)
        return 1
    finally:
        if leased:
            try:
                run("lease-end", lease + ["end", "--target", str(target), "--pid", str(os.getpid())], REPO)
            except (OSError, ValueError, subprocess.SubprocessError) as exc:
                evidence["verdict"] = "FAIL"
                evidence["lease_end_error"] = str(exc)
                print(f"readme example lease release failed: {exc}", file=sys.stderr)
        out = HARNESS / "evidence" / "readme-examples-proof.json"
        serialized = json.dumps(evidence, indent=2) + "\n"
        (scratch / "evidence.json").write_text(serialized, encoding="utf-8", newline="\n")
        out.write_text(serialized, encoding="utf-8", newline="\n")
        print(f"evidence: {out}")
        if "lease_end_error" in evidence:
            return 1


if __name__ == "__main__":
    raise SystemExit(main())
