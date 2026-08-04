//! Adapter process host: spawns and manages a child process adapter.
//!
//! The host starts a child process, performs the initialize/capabilities
//! handshake over JSONL, sends correlated requests with id-based response
//! matching, times out requests, sends best-effort cancel, drops event
//! messages under backpressure, reports adapter crash/unavailable states,
//! and reaps the child on shutdown.
//!
//! # Architecture
//!
//! ```text
//! AdapterHost
//!   ├── stdin writer (Arc<Mutex<ChildStdin>>)
//!   ├── reader task (spawned tokio task)
//!   │   └── reads stdout lines → routes to pending oneshot channels
//!   ├── pending map (Arc<std::sync::Mutex<HashMap<String, oneshot::Sender>>>)
//!   └── id counter (AtomicU64)
//! ```
//!
//! # Unstable
//!
//! This module is part of the **unstable 0.x extension API**. Breaking changes
//! may occur between minor versions without a major version bump.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{Mutex, oneshot};

use opi_agent::Diagnostic;

use crate::adapter_protocol::{
    AdapterCommandCapability, AdapterHostMessage, AdapterModelOverride, AdapterProcessMessage,
    AdapterToolCapability, PROTOCOL_VERSION,
};
use crate::diagnostic_bridge::diagnostic_for_adapter_host_message;
use crate::tool::{AttachError, TreeGuard};

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors from the adapter process host.
#[derive(Debug, thiserror::Error)]
pub enum AdapterHostError {
    /// The adapter process could not be spawned.
    #[error("adapter spawn failed for '{package}': {reason}")]
    SpawnFailed { package: String, reason: String },

    /// The initialize handshake timed out.
    #[error("initialize handshake timed out after {timeout:?}")]
    InitializeTimeout { timeout: Duration },

    /// A request timed out waiting for a response.
    #[error("request '{id}' timed out after {timeout:?}")]
    RequestTimeout { id: String, timeout: Duration },

    /// The adapter process exited unexpectedly during the handshake.
    #[error("adapter '{package}' exited unexpectedly (exit code: {exit_code:?})")]
    AdapterExited {
        package: String,
        exit_code: Option<i32>,
    },

    /// The adapter is no longer available (crashed or shut down).
    #[error("adapter '{package}' is unavailable")]
    AdapterUnavailable { package: String },

    /// IO error communicating with the adapter.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for spawning an adapter child process.
#[derive(Debug, Clone)]
pub struct AdapterProcessConfig {
    /// Path to the adapter binary.
    pub command: PathBuf,
    /// Arguments to pass to the adapter binary.
    pub args: Vec<String>,
    /// Working directory for the child process.
    pub working_dir: PathBuf,
    /// Additional environment variables to set for the child process.
    pub env: Vec<(String, String)>,
}

// ---------------------------------------------------------------------------
// Capabilities
// ---------------------------------------------------------------------------

/// Capabilities advertised by the adapter during initialization.
#[derive(Debug, Clone)]
pub struct AdapterCapabilities {
    /// Tools this adapter provides.
    pub tools: Vec<AdapterToolCapability>,
    /// Commands this adapter handles.
    pub commands: Vec<AdapterCommandCapability>,
    /// Hook names this adapter implements.
    pub hooks: Vec<String>,
    /// Model overrides for specific tools.
    pub model_overrides: Vec<AdapterModelOverride>,
}

// ---------------------------------------------------------------------------
// Pending response map
// ---------------------------------------------------------------------------

type PendingMap = Arc<
    std::sync::Mutex<
        HashMap<String, oneshot::Sender<Result<AdapterProcessMessage, AdapterHostError>>>,
    >,
>;

// ---------------------------------------------------------------------------
// AdapterHost
// ---------------------------------------------------------------------------

/// Manages a child process adapter with JSONL communication.
///
/// The host owns the child process, a reader task that routes responses to
/// pending requests, and shared state for request correlation.
pub struct AdapterHost {
    package_name: String,
    capabilities: AdapterCapabilities,
    stdin_writer: Arc<Mutex<ChildStdin>>,
    pending: PendingMap,
    diagnostics: Arc<std::sync::Mutex<Vec<Diagnostic>>>,
    child: Option<Child>,
    reader_handle: Option<tokio::task::JoinHandle<()>>,
    id_counter: AtomicU64,
    /// Phase 15.4 L0 tree guard retained for the complete host lifetime. On
    /// Windows this owns the kill-on-close Job Object; on Unix it owns the
    /// configured process group. Best-effort attach failures leave this `None`
    /// after recording one redacted degradation diagnostic.
    l0_guard: Option<TreeGuard>,
}

impl std::fmt::Debug for AdapterHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdapterHost")
            .field("package_name", &self.package_name)
            .field("capabilities", &self.capabilities)
            .field("child_pid", &self.child_pid())
            .finish_non_exhaustive()
    }
}

impl AdapterHost {
    /// Start an adapter child process and perform the initialize/capabilities
    /// handshake.
    ///
    /// If the handshake does not complete within `timeout`, returns
    /// [`AdapterHostError::InitializeTimeout`]. If the child exits during
    /// the handshake, returns [`AdapterHostError::AdapterExited`].
    pub async fn start(
        package_name: impl Into<String>,
        config: AdapterProcessConfig,
        timeout: Duration,
    ) -> Result<Self, AdapterHostError> {
        Self::start_with_tree_attach(package_name, config, timeout, TreeGuard::attach).await
    }

    async fn start_with_tree_attach<F>(
        package_name: impl Into<String>,
        config: AdapterProcessConfig,
        timeout: Duration,
        tree_attach: F,
    ) -> Result<Self, AdapterHostError>
    where
        F: FnOnce(u32) -> Result<TreeGuard, AttachError>,
    {
        let package_name = package_name.into();

        // Spawn the child process
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .current_dir(&config.working_dir)
            .envs(config.env.iter().map(|(k, v)| (k, v)))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Create a new process group so we can kill the tree on shutdown.
        // On Unix this is process_group(0); on Windows use CREATE_NEW_PROCESS_GROUP.
        // tokio::process::Command provides `process_group` as an inherent method,
        // so no std `CommandExt` trait import is needed.
        #[cfg(unix)]
        {
            cmd.process_group(0);
        }
        #[cfg(windows)]
        {
            cmd.creation_flags(0x00000200); // CREATE_NEW_PROCESS_GROUP
        }

        let mut child = cmd.spawn().map_err(|e| AdapterHostError::SpawnFailed {
            package: package_name.clone(),
            reason: e.to_string(),
        })?;

        // Retain the L0 guard on every supported host. Missing PID and OS
        // assignment errors are fail-open, but the exact redacted AttachError
        // survives until diagnostics storage exists below.
        let l0_attach = match child.id() {
            Some(pid) if pid != 0 => tree_attach(pid),
            _ => TreeGuard::attach_child(None),
        };
        let (l0_guard, l0_attach_error) = match l0_attach {
            Ok(guard) => (Some(guard), None),
            Err(error) => (None, Some(error)),
        };

        let stdin = child.stdin.take().expect("stdin should be piped");
        let stdout = child.stdout.take().expect("stdout should be piped");
        let stderr = child.stderr.take().expect("stderr should be piped");

        // Drain stderr in the background to prevent pipe buffer deadlock.
        let stderr_package = package_name.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::debug!(
                    target: "adapter_stderr",
                    package = %stderr_package,
                    %line,
                    "adapter stderr"
                );
            }
        });

        let stdin_writer = Arc::new(Mutex::new(stdin));
        let pending: PendingMap = Arc::new(std::sync::Mutex::new(HashMap::new()));
        let diagnostics = Arc::new(std::sync::Mutex::new(Vec::new()));

        // Start the reader task
        let reader_pending = pending.clone();
        let reader_package = package_name.clone();
        let reader_handle = tokio::spawn(async move {
            reader_loop(stdout, reader_pending, reader_package).await;
        });

        let mut host = Self {
            package_name,
            capabilities: AdapterCapabilities {
                tools: vec![],
                commands: vec![],
                hooks: vec![],
                model_overrides: vec![],
            },
            stdin_writer,
            pending,
            diagnostics,
            child: Some(child),
            reader_handle: Some(reader_handle),
            id_counter: AtomicU64::new(1),
            l0_guard,
        };
        if let Some(error) = l0_attach_error {
            host.diagnostics.lock().unwrap().push(
                crate::diagnostics::process_tree_degraded_diagnostic(error.layer, error.reason),
            );
        }

        // Perform the initialize handshake
        let init_id = host.next_id();
        let init_msg = AdapterHostMessage::Initialize {
            id: init_id.clone(),
            protocol: PROTOCOL_VERSION.to_string(),
            package: host.package_name.clone(),
        };

        let capabilities_result =
            tokio::time::timeout(timeout, async { host.send_request_inner(init_msg).await }).await;

        match capabilities_result {
            Ok(Ok(response)) => {
                match response {
                    AdapterProcessMessage::Capabilities {
                        tools,
                        commands,
                        hooks,
                        model_overrides,
                        ..
                    } => {
                        host.capabilities = AdapterCapabilities {
                            tools,
                            commands,
                            hooks,
                            model_overrides,
                        };
                        Ok(host)
                    }
                    AdapterProcessMessage::Error { message, .. } => {
                        // Adapter responded with error during handshake
                        let _ = host.shutdown_inner("handshake_error").await;
                        Err(AdapterHostError::SpawnFailed {
                            package: host.package_name.clone(),
                            reason: format!("adapter error during handshake: {message}"),
                        })
                    }
                    other => {
                        let _ = host.shutdown_inner("unexpected_response").await;
                        Err(AdapterHostError::SpawnFailed {
                            package: host.package_name.clone(),
                            reason: format!(
                                "expected capabilities response, got: {:?}",
                                other_type(&other)
                            ),
                        })
                    }
                }
            }
            Ok(Err(e)) => {
                // Check if the child actually exited — if so, report as AdapterExited
                let exit_code = host
                    .child
                    .as_mut()
                    .and_then(|c| c.try_wait().ok().flatten())
                    .and_then(|s| s.code());
                let _ = host.shutdown_inner("handshake_failure").await;
                if exit_code.is_some() {
                    Err(AdapterHostError::AdapterExited {
                        package: host.package_name.clone(),
                        exit_code,
                    })
                } else {
                    Err(e)
                }
            }
            Err(_) => {
                // Timeout
                let _ = host.shutdown_inner("initialize_timeout").await;
                Err(AdapterHostError::InitializeTimeout { timeout })
            }
        }
    }

    /// Returns the capabilities advertised by the adapter.
    pub fn capabilities(&self) -> &AdapterCapabilities {
        &self.capabilities
    }

    /// Generate a unique request id.
    pub fn next_id(&self) -> String {
        self.id_counter.fetch_add(1, Ordering::Relaxed).to_string()
    }

    /// Returns the child process ID.
    pub fn child_pid(&self) -> Option<u32> {
        self.child.as_ref().and_then(|child| child.id())
    }

    pub fn take_diagnostics(&self) -> Vec<Diagnostic> {
        std::mem::take(&mut *self.diagnostics.lock().unwrap())
    }

    /// Send a request and await the response with a timeout.
    ///
    /// The request `id` field is used for correlation. The caller should use
    /// [`next_id`](Self::next_id) to generate a unique id.
    pub async fn send_request(
        &self,
        message: AdapterHostMessage,
        timeout: Duration,
    ) -> Result<AdapterProcessMessage, AdapterHostError> {
        let id = extract_id(&message);
        let result = tokio::time::timeout(timeout, self.send_request_inner(message)).await;

        match result {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                // Timeout — clean up the pending entry
                if let Some(id) = id.as_ref() {
                    self.pending.lock().unwrap().remove(id);
                }
                Err(AdapterHostError::RequestTimeout {
                    id: id.unwrap_or_default(),
                    timeout,
                })
            }
        }
    }

    /// Send a fire-and-forget event. Returns immediately.
    ///
    /// If the adapter's stdin is backpressured, the event is dropped and a
    /// diagnostic is recorded.
    pub async fn send_event(&self, event: serde_json::Value) {
        let msg = AdapterHostMessage::Event { event };
        let json = match serde_json::to_string(&msg) {
            Ok(j) => j,
            Err(_) => return,
        };

        // Try to write with a very short timeout; drop on backpressure
        let write_result = tokio::time::timeout(Duration::from_millis(100), async {
            let mut stdin = self.stdin_writer.lock().await;
            stdin.write_all(json.as_bytes()).await.ok()?;
            stdin.write_all(b"\n").await.ok()?;
            stdin.flush().await.ok()?;
            Some(())
        })
        .await;

        match write_result {
            Ok(Some(())) => {}
            Ok(None) => self.record_diagnostic("event delivery failed"),
            Err(_) => self.record_diagnostic("event delivery timed out after 100ms"),
        }
    }

    /// Send a best-effort cancel for an in-flight request.
    ///
    /// Does not wait for a response. If the cancel message cannot be written,
    /// it is silently ignored.
    pub async fn cancel(&self, id: &str, reason: &str) -> Result<(), AdapterHostError> {
        let msg = AdapterHostMessage::Cancel {
            id: id.to_string(),
            reason: reason.to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap_or_default();

        let _ = tokio::time::timeout(Duration::from_millis(100), async {
            let mut stdin = self.stdin_writer.lock().await;
            let _ = stdin.write_all(json.as_bytes()).await;
            let _ = stdin.write_all(b"\n").await;
            let _ = stdin.flush().await;
        })
        .await;

        Ok(())
    }

    /// Send a shutdown message and reap the child process.
    pub async fn shutdown(mut self, reason: &str) -> Result<(), AdapterHostError> {
        self.shutdown_inner(reason).await
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Internal request send: register pending, write to stdin, await response.
    async fn send_request_inner(
        &self,
        message: AdapterHostMessage,
    ) -> Result<AdapterProcessMessage, AdapterHostError> {
        let id = extract_id(&message).unwrap_or_default();

        // Register the pending response channel
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id.clone(), tx);

        // Serialize and write the message
        let json = serde_json::to_string(&message).map_err(|e| AdapterHostError::SpawnFailed {
            package: self.package_name.clone(),
            reason: format!("serialize error: {e}"),
        })?;

        {
            let mut stdin = self.stdin_writer.lock().await;
            if stdin.write_all(json.as_bytes()).await.is_err() {
                self.pending.lock().unwrap().remove(&id);
                return Err(AdapterHostError::AdapterUnavailable {
                    package: self.package_name.clone(),
                });
            }
            if stdin.write_all(b"\n").await.is_err() {
                self.pending.lock().unwrap().remove(&id);
                return Err(AdapterHostError::AdapterUnavailable {
                    package: self.package_name.clone(),
                });
            }
            let _ = stdin.flush().await;
        }

        // Await the response
        rx.await.map_err(|_| {
            // The sender was dropped — the reader task detected adapter exit
            AdapterHostError::AdapterUnavailable {
                package: self.package_name.clone(),
            }
        })?
    }

    /// Internal shutdown: send shutdown message, kill child, join reader.
    async fn shutdown_inner(&mut self, reason: &str) -> Result<(), AdapterHostError> {
        // Send shutdown message (best-effort)
        let shutdown_msg = AdapterHostMessage::Shutdown {
            id: "shutdown".to_string(),
            reason: reason.to_string(),
        };
        if let Ok(json) = serde_json::to_string(&shutdown_msg) {
            let _ = tokio::time::timeout(Duration::from_millis(200), async {
                let mut stdin = self.stdin_writer.lock().await;
                let _ = stdin.write_all(json.as_bytes()).await;
                let _ = stdin.write_all(b"\n").await;
                let _ = stdin.flush().await;
            })
            .await;
        }

        if let Some(ref mut child) = self.child {
            match tokio::time::timeout(Duration::from_secs(5), child.wait()).await {
                Ok(_) => {}
                Err(_) => {
                    // Phase 15.4 L0: the graceful wait timed out, so tear down
                    // the whole adapter subprocess tree (kill-on-close on
                    // Windows) before reaping the direct child. On the graceful
                    // path the guard is dropped later when the host is dropped,
                    // cleaning up any orphaned grandchildren.
                    self.l0_guard.take();
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                }
            }
        }

        // Abort the reader task
        if let Some(handle) = self.reader_handle.take() {
            handle.abort();
        }

        // Fail all pending requests
        let mut map = self.pending.lock().unwrap();
        for (_, tx) in map.drain() {
            let _ = tx.send(Err(AdapterHostError::AdapterUnavailable {
                package: self.package_name.clone(),
            }));
        }

        Ok(())
    }

    fn record_diagnostic(&self, message: impl Into<String>) {
        self.diagnostics
            .lock()
            .unwrap()
            .push(diagnostic_for_adapter_host_message(
                &self.package_name,
                message.into(),
            ));
    }
}

impl Drop for AdapterHost {
    fn drop(&mut self) {
        // Best-effort cleanup: kill child and abort reader on drop
        if let Some(ref mut child) = self.child {
            // start_kill is non-blocking
            let _ = child.start_kill();
        }
        if let Some(handle) = self.reader_handle.take() {
            handle.abort();
        }
    }
}

// ---------------------------------------------------------------------------
// Reader task
// ---------------------------------------------------------------------------

/// Background task that reads lines from the adapter's stdout and routes
/// responses to the appropriate pending request channels.
async fn reader_loop(stdout: ChildStdout, pending: PendingMap, package_name: String) {
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        if line.is_empty() {
            continue;
        }

        let msg: AdapterProcessMessage = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let id = extract_response_id(&msg);

        if let Some(id) = id {
            let tx = pending.lock().unwrap().remove(&id);
            if let Some(tx) = tx {
                let _ = tx.send(Ok(msg));
            }
            // If no pending request for this id, the response is dropped
        }
        // Unsolicited messages (Error with id: None) are logged and ignored
    }

    // EOF reached — adapter exited. Fail all pending requests.
    let mut map = pending.lock().unwrap();
    for (_, tx) in map.drain() {
        let _ = tx.send(Err(AdapterHostError::AdapterUnavailable {
            package: package_name.clone(),
        }));
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract the id from an AdapterHostMessage.
fn extract_id(msg: &AdapterHostMessage) -> Option<String> {
    match msg {
        AdapterHostMessage::Initialize { id, .. } => Some(id.clone()),
        AdapterHostMessage::ToolCall { id, .. } => Some(id.clone()),
        AdapterHostMessage::Command { id, .. } => Some(id.clone()),
        AdapterHostMessage::Hook { id, .. } => Some(id.clone()),
        AdapterHostMessage::StateSerialize { id } => Some(id.clone()),
        AdapterHostMessage::StateRestore { id, .. } => Some(id.clone()),
        AdapterHostMessage::Cancel { id, .. } => Some(id.clone()),
        AdapterHostMessage::Shutdown { id, .. } => Some(id.clone()),
        AdapterHostMessage::Event { .. } => None,
    }
}

/// Extract the id from an AdapterProcessMessage.
fn extract_response_id(msg: &AdapterProcessMessage) -> Option<String> {
    match msg {
        AdapterProcessMessage::Capabilities { id, .. } => Some(id.clone()),
        AdapterProcessMessage::ToolResult { id, .. } => Some(id.clone()),
        AdapterProcessMessage::CommandResult { id, .. } => Some(id.clone()),
        AdapterProcessMessage::HookResult { id, .. } => Some(id.clone()),
        AdapterProcessMessage::StateResult { id, .. } => Some(id.clone()),
        AdapterProcessMessage::Error { id, .. } => id.clone(),
    }
}

/// Get the type name of an AdapterProcessMessage for error messages.
fn other_type(msg: &AdapterProcessMessage) -> &'static str {
    match msg {
        AdapterProcessMessage::Capabilities { .. } => "capabilities",
        AdapterProcessMessage::ToolResult { .. } => "tool_result",
        AdapterProcessMessage::CommandResult { .. } => "command_result",
        AdapterProcessMessage::HookResult { .. } => "hook_result",
        AdapterProcessMessage::StateResult { .. } => "state_result",
        AdapterProcessMessage::Error { .. } => "error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::CODE_PROCESS_TREE_DEGRADED;

    fn one_handshake_adapter() -> AdapterProcessConfig {
        let response = r#"{"type":"capabilities","id":"1","tools":[],"commands":[],"hooks":[],"model_overrides":[]}"#;
        #[cfg(windows)]
        {
            AdapterProcessConfig {
                command: PathBuf::from("powershell"),
                args: vec![
                    "-NoProfile".to_string(),
                    "-Command".to_string(),
                    format!(
                        "$null = [Console]::In.ReadLine(); [Console]::Out.WriteLine('{response}'); [Console]::Out.Flush(); $null = [Console]::In.ReadLine()"
                    ),
                ],
                working_dir: std::env::current_dir().unwrap(),
                env: vec![],
            }
        }
        #[cfg(unix)]
        {
            AdapterProcessConfig {
                command: PathBuf::from("sh"),
                args: vec![
                    "-c".to_string(),
                    format!("read init; printf '%s\\n' '{response}'; read shutdown"),
                ],
                working_dir: std::env::current_dir().unwrap(),
                env: vec![],
            }
        }
    }

    #[tokio::test]
    async fn attach_failure_records_one_redacted_diagnostic_and_continues() {
        let host = AdapterHost::start_with_tree_attach(
            "faulted-adapter",
            one_handshake_adapter(),
            Duration::from_secs(5),
            |_pid| TreeGuard::attach(0),
        )
        .await
        .expect("L0 attach failure is fail-open");

        let diagnostics = host.take_diagnostics();
        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.code, CODE_PROCESS_TREE_DEGRADED);
        assert_eq!(
            diagnostic.details,
            Some(serde_json::json!({
                "layer": if cfg!(windows) { "windows-job" } else { "unix-pgroup" },
                "reason": "missing child process id",
            }))
        );

        host.shutdown("test_end").await.unwrap();
    }
}
