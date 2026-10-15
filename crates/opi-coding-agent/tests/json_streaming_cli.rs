//! Real CLI regression for JSON events while a provider response is still open.
//! The only provider is a task-owned loopback SSE fixture with a synthetic key.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

const CANARY: &str = "opi-live-json-delta-canary";

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn isolated_command(workspace: &Path, config: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_opi"));
    command
        .env_clear()
        .current_dir(workspace)
        .args(["--config", config.to_str().unwrap()])
        .args(["--model", "streamfixture:stream-model"])
        .args(["--no-trust", "--non-interactive", "--json"]);
    for name in [
        "HOME",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_CACHE_HOME",
        "OPI_SESSIONS_DIR",
        "TMPDIR",
        "TMP",
        "TEMP",
    ] {
        let path = workspace.join(name.to_ascii_lowercase());
        std::fs::create_dir_all(&path).unwrap();
        command.env(name, path);
    }
    // Windows process startup needs SystemRoot; this is not user configuration.
    if cfg!(windows)
        && let Some(root) = std::env::var_os("SystemRoot")
    {
        command.env("SystemRoot", root);
    }
    #[cfg(target_os = "linux")]
    {
        use sha2::Digest;

        // The unique workspace remains alive until the child settles. An
        // unoccupied abstract bus isolates Secret Service and permits the
        // synthetic environment key through the connection-refused fallback.
        let bus = hex::encode(sha2::Sha256::digest(
            workspace.as_os_str().as_encoded_bytes(),
        ));
        command.env(
            "DBUS_SESSION_BUS_ADDRESS",
            format!(
                "unix:abstract=opi-json-stream-{}-{}",
                std::process::id(),
                &bus[..32]
            ),
        );
    }
    command.env("OPI_TEST_STREAM_KEY", "synthetic-loopback-only-key");
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    command
}

fn fixture_config(workspace: &Path, port: u16) -> std::path::PathBuf {
    // main calls dotenvy::dotenv(); do not let this test traverse into an
    // ambient .env while resolving its explicit, isolated configuration.
    assert!(
        workspace
            .ancestors()
            .all(|parent| !parent.join(".env").exists()),
        "test workspace has an ambient .env ancestor"
    );
    let config = workspace.join("config.toml");
    std::fs::write(
        &config,
        format!(
            "[defaults]\nmodel = \"streamfixture:stream-model\"\n\
             [providers.custom.streamfixture]\nbase_url = \"http://127.0.0.1:{port}\"\n\
             api_key_env = \"OPI_TEST_STREAM_KEY\"\nauth_scheme = \"bearer\"\n\
             [[providers.custom.streamfixture.models]]\nid = \"stream-model\"\n\
             api = \"openai-completions\"\ncontext_window = 8192\n\
             max_output_tokens = 1024\nsupports_images = true\n"
        ),
    )
    .unwrap();
    config
}

fn fixture_workspace() -> (tempfile::TempDir, TcpListener, std::path::PathBuf) {
    let workspace = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let config = fixture_config(workspace.path(), listener.local_addr().unwrap().port());
    (workspace, listener, config)
}

fn accept_fixture_request(listener: TcpListener) -> Result<(TcpStream, Vec<u8>), String> {
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err("no loopback request".to_owned());
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(format!("loopback accept failed: {error}")),
        }
    };
    stream.set_nonblocking(false).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    let mut request = Vec::new();
    let mut bytes = [0u8; 4096];
    while !request.windows(4).any(|part| part == b"\r\n\r\n") {
        let count = stream.read(&mut bytes).map_err(|e| e.to_string())?;
        if count == 0 || request.len() > 64 * 1024 {
            return Err("incomplete or oversized provider request header".to_owned());
        }
        request.extend_from_slice(&bytes[..count]);
    }
    let header_end = request
        .windows(4)
        .position(|part| part == b"\r\n\r\n")
        .unwrap()
        + 4;
    let header = String::from_utf8_lossy(&request[..header_end]);
    if !header.starts_with("POST /v1/chat/completions HTTP/1.1")
        || !header.contains("synthetic-loopback-only-key")
    {
        return Err("unexpected local provider request identity".to_owned());
    }
    Ok((stream, request))
}

fn run_streaming_case(compact: bool, image: bool) {
    let (workspace, listener, config) = fixture_workspace();

    let (sent_tx, sent_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let server = thread::spawn(move || {
        let (mut stream, mut request) = match accept_fixture_request(listener) {
            Ok(pair) => pair,
            Err(error) => {
                sent_tx.send(Err(error)).unwrap();
                return;
            }
        };
        let mut bytes = [0u8; 4096];
        let header_end = request
            .windows(4)
            .position(|part| part == b"\r\n\r\n")
            .unwrap()
            + 4;
        let header = String::from_utf8_lossy(&request[..header_end]);
        if image {
            let content_length: usize = header
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse().unwrap())
                })
                .expect("JSON provider request has a content length");
            assert!(content_length < 1024 * 1024);
            while request.len() < header_end + content_length {
                let count = stream.read(&mut bytes).unwrap();
                assert!(count > 0, "provider request body ended early");
                request.extend_from_slice(&bytes[..count]);
            }
            assert!(
                String::from_utf8_lossy(&request[header_end..header_end + content_length])
                    .contains("data:image/png;base64,"),
                "--image did not reach the loopback provider request"
            );
        }
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n",
            )
            .unwrap();
        let role = serde_json::json!({"id":"stream-1","object":"chat.completion.chunk","created":1,"model":"stream-model","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]});
        let text = serde_json::json!({"id":"stream-1","object":"chat.completion.chunk","created":1,"model":"stream-model","choices":[{"index":0,"delta":{"content":CANARY},"finish_reason":null}]});
        write!(stream, "data: {role}\n\ndata: {text}\n\n").unwrap();
        stream.flush().unwrap();
        sent_tx.send(Ok(())).unwrap();
        // No terminal SSE event or EOF before the parent has observed stdout.
        let _ = release_rx.recv_timeout(Duration::from_secs(20));
    });

    let mut command = isolated_command(workspace.path(), &config);
    if compact {
        command.arg("--json-compact");
    }
    if image {
        let image_path = workspace.path().join("pixel.png");
        std::fs::write(&image_path, b"\x89PNG\r\n\x1a\n").unwrap();
        command.arg("--image").arg(image_path);
    }
    command.arg("stream a response");
    let mut child = ChildGuard(command.spawn().unwrap());
    let stdout = child.0.stdout.take().unwrap();
    let stderr = child.0.stderr.take().unwrap();
    let (line_tx, line_rx) = mpsc::channel();
    let reader = thread::spawn(move || {
        let mut all = Vec::new();
        let mut stdout = BufReader::new(stdout);
        loop {
            let mut line = String::new();
            if stdout.read_line(&mut line).unwrap() == 0 {
                break;
            }
            let complete = line.ends_with('\n');
            if complete {
                line_tx.send(line.clone()).unwrap();
            }
            all.push((line, complete));
            if !complete {
                break;
            }
        }
        all
    });
    let errors = thread::spawn(move || {
        let mut bytes = Vec::new();
        BufReader::new(stderr).read_to_end(&mut bytes).unwrap();
        String::from_utf8_lossy(&bytes).into_owned()
    });
    let request = sent_rx.recv_timeout(Duration::from_secs(15));
    let mut prefix = Vec::new();
    let mut saw_canary = false;
    if matches!(&request, Ok(Ok(()))) {
        let deadline = Instant::now() + Duration::from_secs(8);
        while Instant::now() < deadline {
            if let Ok(line) = line_rx.recv_timeout(Duration::from_millis(100)) {
                let event: Value = serde_json::from_str(&line).unwrap();
                saw_canary |= event["type"] == "Agent"
                    && event["event"]["type"] == "MessageUpdate"
                    && event["event"]["assistant_event"]["type"] == "text_delta"
                    && event["event"]["assistant_event"]["delta"] == CANARY;
                prefix.push(line);
                if saw_canary {
                    break;
                }
            }
        }
    }
    let live_at_canary = child.0.try_wait().unwrap().is_none();
    let _ = child.0.kill();
    let _ = child.0.wait();
    let _ = release_tx.send(());
    server.join().unwrap();
    let all = reader.join().unwrap();
    let stderr = errors.join().unwrap();
    assert!(
        matches!(&request, Ok(Ok(()))),
        "provider request failed: {request:?}; stderr={stderr}"
    );
    assert!(
        saw_canary,
        "no live text delta before SSE completion; stdout lines={}, stderr={stderr}",
        all.len()
    );
    assert!(
        live_at_canary,
        "child exited before the held provider response ended"
    );
    assert!(
        all.len() >= prefix.len()
            && all
                .iter()
                .zip(&prefix)
                .all(|((line, complete), expected)| *complete && line == expected),
        "already-read complete NDJSON prefix was lost"
    );
    assert!(
        all.iter()
            .enumerate()
            .all(|(index, (_, complete))| *complete || index + 1 == all.len()),
        "only the final post-kill stdout line may be incomplete"
    );
    assert!(
        all.iter()
            .filter(|(_, complete)| *complete)
            .all(|(line, _)| {
                let event: Value = serde_json::from_str(line).unwrap();
                !(event["type"] == "Agent" && event["event"]["type"] == "AgentEnd")
            }),
        "a killed in-flight Agent must not fabricate AgentEnd"
    );
}

#[test]
fn default_json_emits_text_delta_before_provider_response_finishes() {
    run_streaming_case(false, false);
}

#[test]
fn compact_json_with_image_emits_text_delta_before_provider_response_finishes() {
    run_streaming_case(true, true);
}

#[test]
fn json_stdout_broken_pipe_exits_with_failure_without_panicking() {
    let (workspace, listener, config) = fixture_workspace();
    let (sent_tx, sent_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let server = thread::spawn(move || -> Result<(), String> {
        let (mut stream, _) = accept_fixture_request(listener)?;
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n",
            )
            .map_err(|e| e.to_string())?;
        let role = serde_json::json!({"id":"stream-1","object":"chat.completion.chunk","created":1,"model":"stream-model","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]});
        let text = serde_json::json!({"id":"stream-1","object":"chat.completion.chunk","created":1,"model":"stream-model","choices":[{"index":0,"delta":{"content":CANARY},"finish_reason":null}]});
        write!(stream, "data: {role}\n\ndata: {text}\n\n").map_err(|e| e.to_string())?;
        stream.flush().map_err(|e| e.to_string())?;
        sent_tx.send(()).map_err(|e| e.to_string())?;
        // The parent closes the child's stdout reader only after the provider
        // has delivered a real delta. Then this valid response can complete.
        release_rx
            .recv_timeout(Duration::from_secs(15))
            .map_err(|e| e.to_string())?;
        let finish = serde_json::json!({"id":"stream-1","object":"chat.completion.chunk","created":1,"model":"stream-model","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}});
        write!(stream, "data: {finish}\n\ndata: [DONE]\n\n").map_err(|e| e.to_string())?;
        stream.flush().map_err(|e| e.to_string())
    });

    let mut command = isolated_command(workspace.path(), &config);
    command.arg("finish after stdout disconnects");
    let mut child = ChildGuard(command.spawn().unwrap());
    let stdout = child.0.stdout.take().unwrap();
    let stderr = child.0.stderr.take().unwrap();
    let errors = thread::spawn(move || {
        let mut bytes = Vec::new();
        BufReader::new(stderr).read_to_end(&mut bytes).unwrap();
        String::from_utf8_lossy(&bytes).into_owned()
    });
    let provider_started = sent_rx.recv_timeout(Duration::from_secs(15));
    drop(stdout);
    let _ = release_tx.send(());
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut settled = None;
    while Instant::now() < deadline {
        if let Some(status) = child.0.try_wait().unwrap() {
            settled = Some(status);
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    if settled.is_none() {
        let _ = child.0.kill();
        let _ = child.0.wait();
    }
    let server_result = server.join().unwrap();
    let stderr = errors.join().unwrap();
    assert!(
        provider_started.is_ok(),
        "no local provider delta before stdout disconnect: {provider_started:?}; stderr={stderr}"
    );
    // The CLI deliberately cancels the provider request once JSON stdout
    // breaks (json_output fail-fast). Whether the fixture server still sees
    // its final write through depends on the platform's socket close
    // semantics: Linux closes gracefully (FIN) so the write succeeds, while
    // Windows aborts (RST) and the write fails with a peer-abort error. Both
    // prove the CLI settled the run without panicking; any other fixture
    // failure is a real regression.
    let server_settled_or_cancel_aborted = match &server_result {
        Ok(()) => true,
        Err(reason) => {
            let reason = reason.to_ascii_lowercase();
            reason.contains("aborted")
                || reason.contains("connection reset")
                || reason.contains("broken pipe")
                || reason.contains("os error 10053")
        }
    };
    assert!(
        server_settled_or_cancel_aborted,
        "local provider neither completed nor was aborted by the CLI's broken-stdout cancellation: {server_result:?}; stderr={stderr}"
    );
    let status = settled.expect("CLI did not settle after stdout broke");
    assert_eq!(
        status.code(),
        Some(1),
        "broken stdout must be a failed CLI run, not provider success; stderr={stderr}"
    );
    assert!(
        !stderr.contains("panicked") && !stderr.contains("stack backtrace"),
        "broken stdout must not panic: {stderr}"
    );
}
