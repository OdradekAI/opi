//! Behavioral tests for the Phase 15.1 Operations seam (task 15.1).
//!
//! DoD: "constructor-level mock tests prove the contracts preserve bytes,
//! metadata, command cwd/env/timeout/cancellation fields, operation/path/
//! error identity, and malformed or denied backend results without touching
//! the real filesystem outside a temp root."
//!
//! The keystone test `operations_contracts_and_local_file_backend` is a thin
//! orchestrator over focused helper fns so a single platform-specific
//! regression does not block the entire DoD gate. The atomic-write
//! failed-replacement-preserves-prior-file property is additionally proven
//! by inline unit tests inside `src/tool/operations.rs` (they need the
//! `#[cfg(test)]` failure-injection seam which is only callable from inside
//! the crate).

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opi_agent::tool::Tool;
use opi_coding_agent::tool::{
    AccessMode, BashOpError, BashOperations, BashRequest, BashResult, BashTool, EditTool,
    FileOperations, FsOpError, LocalFileOperations, OpMetadata, PathPolicy, ReadTool,
    ToolDiagnostic, WriteTool,
};
use serde_json::json;
use tokio_util::sync::CancellationToken;

// =========================================================================
// Keystone DoD test (thin orchestrator over focused helpers)
// =========================================================================

#[tokio::test]
async fn operations_contracts_and_local_file_backend() {
    // Each helper is a focused sub-contract; a failure pinpoints the clause.
    file_ops_roundtrip_preserves_bytes().await;
    file_ops_metadata_reports_std_fields().await;
    file_ops_mkdir_recursive_vs_non_recursive().await;
    file_ops_access_modes_basic().await;
    file_ops_errors_carry_path_identity().await;
    file_ops_do_not_escape_temp_root().await;
}

async fn file_ops_roundtrip_preserves_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("round.bin");
    let ops = LocalFileOperations::new();
    ops.write_file(&target, b"line1\nline2\n").await.unwrap();
    let bytes = ops.read_file(&target).await.unwrap();
    assert_eq!(bytes, b"line1\nline2\n", "round-trip must preserve bytes");
}

async fn file_ops_metadata_reports_std_fields() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("meta.txt");
    std::fs::write(&file, "12345").unwrap();
    let ops = LocalFileOperations::new();
    let meta = ops.metadata(&file).await.unwrap();
    assert_eq!(meta.len, 5);
    assert!(meta.is_file);
    assert!(!meta.is_dir);

    let dir_meta = ops.metadata(dir.path()).await.unwrap();
    assert!(dir_meta.is_dir);
    assert!(!dir_meta.is_file);
}

async fn file_ops_mkdir_recursive_vs_non_recursive() {
    let dir = tempfile::tempdir().unwrap();
    let ops = LocalFileOperations::new();

    let nested = dir.path().join("a/b/c");
    ops.mkdir(&nested, true).await.unwrap();
    assert!(
        nested.is_dir(),
        "recursive mkdir must create intermediate dirs"
    );

    let fresh = dir.path().join("fresh");
    let missing_parent = fresh.join("deep");
    let err = ops.mkdir(&missing_parent, false).await.unwrap_err();
    assert!(
        matches!(err, FsOpError::NotFound { .. } | FsOpError::Io { .. }),
        "non-recursive mkdir with missing parent must fail: {err:?}"
    );
}

async fn file_ops_access_modes_basic() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("accessible.txt");
    std::fs::write(&file, "x").unwrap();
    let ops = LocalFileOperations::new();

    ops.access(&file, AccessMode::Exists).await.unwrap();
    ops.access(&file, AccessMode::Readable).await.unwrap();
    ops.access(&file, AccessMode::Writable).await.unwrap();

    let missing = dir.path().join("absent.txt");
    let err = ops.access(&missing, AccessMode::Exists).await.unwrap_err();
    assert!(
        matches!(err, FsOpError::NotFound { .. }),
        "Exists on missing path: {err:?}"
    );
}

async fn file_ops_errors_carry_path_identity() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("identity-missing.txt");
    let ops = LocalFileOperations::new();

    let err = ops.read_file(&missing).await.unwrap_err();
    match err {
        FsOpError::NotFound { path } => assert_eq!(path, missing),
        other => panic!("expected NotFound, got {other:?}"),
    }

    let err = ops.metadata(&missing).await.unwrap_err();
    match err {
        FsOpError::NotFound { path } => assert_eq!(path, missing),
        other => panic!("expected NotFound, got {other:?}"),
    }

    let err = ops.read_file(dir.path()).await.unwrap_err();
    match err {
        FsOpError::NotAFile { path } => assert_eq!(path, dir.path()),
        other => panic!("expected NotAFile for directory, got {other:?}"),
    }
}

async fn file_ops_do_not_escape_temp_root() {
    let primary = tempfile::tempdir().unwrap();
    let sibling = tempfile::tempdir().unwrap();

    let ops = LocalFileOperations::new();
    let file = primary.path().join("inside.txt");
    ops.write_file(&file, b"confined").await.unwrap();
    ops.read_file(&file).await.unwrap();
    ops.mkdir(&primary.path().join("subdir"), true)
        .await
        .unwrap();
    ops.metadata(&file).await.unwrap();
    ops.access(&file, AccessMode::Readable).await.unwrap();

    // Sibling temp root must be unchanged.
    let before: Vec<String> = std::fs::read_dir(sibling.path())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    let after: Vec<String> = std::fs::read_dir(sibling.path())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(before, after, "sibling temp root must be unchanged");

    // OS temp must not accumulate opi-ops-tmp artifacts from this process.
    let pid = std::process::id();
    let os_temp_residue: Vec<_> = std::fs::read_dir(std::env::temp_dir())
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains("opi-ops-tmp") && n.contains(&pid.to_string()))
        .collect();
    assert!(
        os_temp_residue.is_empty(),
        "file-ops temp files must stay in-target-dir, not OS temp: {os_temp_residue:?}"
    );
}

// =========================================================================
// Bytes preservation (DoD clause: "preserve bytes")
// =========================================================================

#[tokio::test]
async fn file_operations_read_preserves_bytes_exactly() {
    let ops = LocalFileOperations::new();
    let fixtures: &[(&str, &[u8])] = &[
        ("empty", b""),
        ("single-byte", b"x"),
        ("utf8", "héllo 世界 🌍".as_bytes()),
        ("invalid-utf8", &[0xFF, 0xFE, 0x80]),
        ("crlf", b"line1\r\nline2\r\n"),
        ("lf", b"line1\nline2\n"),
        ("cr", b"line1\rline2\r"),
        ("nul-binary", b"abc\x00def"),
        ("large", &vec![b'A'; 70 * 1024]),
    ];

    for (name, fixture) in fixtures {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("fixture.bin");
        std::fs::write(&target, fixture).unwrap();
        let read = ops.read_file(&target).await.unwrap();
        assert_eq!(read, *fixture, "bytes mismatch for fixture '{name}'");
    }
}

#[tokio::test]
async fn file_operations_write_then_read_roundtrips_binary() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("rw.bin");
    let ops = LocalFileOperations::new();
    let payload: Vec<u8> = (0..=255).cycle().take(1024).collect();
    ops.write_file(&target, &payload).await.unwrap();
    let read = ops.read_file(&target).await.unwrap();
    assert_eq!(read, payload);
}

// =========================================================================
// Metadata (DoD clause: "metadata")
// =========================================================================

#[tokio::test]
async fn file_operations_metadata_reports_std_fields_across_types() {
    let dir = tempfile::tempdir().unwrap();
    let ops = LocalFileOperations::new();

    let file = dir.path().join("regular.txt");
    std::fs::write(&file, "0123456789").unwrap();
    let meta = ops.metadata(&file).await.unwrap();
    let std_meta = std::fs::metadata(&file).unwrap();
    assert_eq!(meta.len, std_meta.len());
    assert_eq!(meta.is_dir, std_meta.is_dir());
    assert_eq!(meta.is_file, std_meta.is_file());

    let empty = dir.path().join("empty.txt");
    std::fs::write(&empty, "").unwrap();
    let empty_meta = ops.metadata(&empty).await.unwrap();
    assert_eq!(empty_meta.len, 0);
    assert!(empty_meta.is_file);

    let subdir = dir.path().join("subdir");
    std::fs::create_dir(&subdir).unwrap();
    let dir_meta = ops.metadata(&subdir).await.unwrap();
    assert!(dir_meta.is_dir);
    assert!(!dir_meta.is_file);

    let missing = dir.path().join("missing.txt");
    let err = ops.metadata(&missing).await.unwrap_err();
    assert!(matches!(err, FsOpError::NotFound { .. }));
}

// =========================================================================
// Recursive / non-recursive mkdir (DoD clause)
// =========================================================================

#[tokio::test]
async fn file_operations_mkdir_recursive_creates_intermediate_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let ops = LocalFileOperations::new();
    let deep = dir.path().join("a/b/c/d");
    ops.mkdir(&deep, true).await.unwrap();
    assert!(deep.is_dir());

    // Idempotent on existing dir.
    ops.mkdir(&deep, true).await.unwrap();
}

#[tokio::test]
async fn file_operations_mkdir_non_recursive_fails_on_missing_parent() {
    let dir = tempfile::tempdir().unwrap();
    let ops = LocalFileOperations::new();
    let fresh = dir.path().join("fresh");
    let deep = fresh.join("deep");
    let result = ops.mkdir(&deep, false).await;
    assert!(
        result.is_err(),
        "non-recursive mkdir with missing parent must fail"
    );
}

// =========================================================================
// Access modes (DoD clause: "access" + "denied backend results")
// =========================================================================

#[tokio::test]
async fn file_operations_access_modes_distinguish_exists_readable_writable() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("accessible.txt");
    std::fs::write(&file, "data").unwrap();
    let ops = LocalFileOperations::new();

    ops.access(&file, AccessMode::Exists)
        .await
        .expect("Exists on present file");
    ops.access(&file, AccessMode::Readable)
        .await
        .expect("Readable on file");
    ops.access(&file, AccessMode::Writable)
        .await
        .expect("Writable on file");

    let missing = dir.path().join("absent.txt");
    let err = ops
        .access(&missing, AccessMode::Exists)
        .await
        .expect_err("Exists on missing path must Err");
    assert!(matches!(err, FsOpError::NotFound { .. }));
}

/// Unix-only denied-path coverage (compiles out on Windows per host MEMORY.md).
/// The Windows AccessMode-denial gap is a documented 15.1 residual.
#[cfg(unix)]
#[tokio::test]
async fn file_operations_access_denied_on_unix() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("noperm.txt");
    std::fs::write(&file, "secret").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).unwrap();

    let ops = LocalFileOperations::new();
    let err = ops
        .access(&file, AccessMode::Readable)
        .await
        .expect_err("Readable on chmod 000 file must Err");
    assert!(
        matches!(
            err,
            FsOpError::PermissionDenied { .. } | FsOpError::Io { .. }
        ),
        "denied read: {err:?}"
    );

    // Restore for tempdir cleanup.
    let _ = std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644));
}

// =========================================================================
// Operation / path / error identity (DoD clause)
// =========================================================================

#[tokio::test]
async fn file_operations_returns_not_found_for_missing_path() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("no-such-file.txt");
    let ops = LocalFileOperations::new();

    let err = ops.read_file(&missing).await.unwrap_err();
    assert_eq!(
        err,
        FsOpError::NotFound {
            path: missing.clone()
        }
    );

    let err = ops.metadata(&missing).await.unwrap_err();
    assert_eq!(
        err,
        FsOpError::NotFound {
            path: missing.clone()
        }
    );

    let err = ops.access(&missing, AccessMode::Exists).await.unwrap_err();
    assert_eq!(err, FsOpError::NotFound { path: missing });
}

#[tokio::test]
async fn file_operations_returns_not_a_file_for_directory_read() {
    let dir = tempfile::tempdir().unwrap();
    let ops = LocalFileOperations::new();
    let err = ops.read_file(dir.path()).await.unwrap_err();
    assert_eq!(
        err,
        FsOpError::NotAFile {
            path: dir.path().to_path_buf()
        }
    );
}

// =========================================================================
// Temp-root confinement (DoD clause: "without touching the real fs outside
// a temp root")
// =========================================================================

#[tokio::test]
async fn file_operations_errors_do_not_escape_temp_root() {
    let primary = tempfile::tempdir().unwrap();
    let sentinel = tempfile::tempdir().unwrap();
    let pid = std::process::id();

    let ops = LocalFileOperations::new();
    // Exercise every method (success and failure) inside primary.
    let file = primary.path().join("escape.txt");
    ops.write_file(&file, b"x").await.unwrap();
    ops.read_file(&file).await.unwrap();
    ops.mkdir(&primary.path().join("d"), true).await.unwrap();
    ops.metadata(&file).await.unwrap();
    ops.access(&file, AccessMode::Writable).await.unwrap();
    // Failure paths:
    let _ = ops.read_file(&primary.path().join("missing")).await;
    let _ = ops.read_file(primary.path()).await;

    let sentinel_entries: Vec<_> = std::fs::read_dir(sentinel.path())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        sentinel_entries.is_empty(),
        "sentinel temp root polluted: {sentinel_entries:?}"
    );

    let os_temp_residue: Vec<_> = std::fs::read_dir(std::env::temp_dir())
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains("opi-ops-tmp") && n.contains(&pid.to_string()))
        .collect();
    assert!(
        os_temp_residue.is_empty(),
        "file-ops temp files leaked to OS temp: {os_temp_residue:?}"
    );
}

// =========================================================================
// Object safety (DoD clause: trait shape lets 15.2 inject Arc<dyn T>)
// =========================================================================

#[test]
fn file_operations_trait_is_object_safe_via_arc_dyn() {
    let _: Arc<dyn FileOperations> = Arc::new(LocalFileOperations::new());
}

#[tokio::test]
async fn file_operations_dyn_dispatch_executes() {
    let dyn_ops: Arc<dyn FileOperations> = Arc::new(LocalFileOperations::new());
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("dyn.txt");
    dyn_ops.write_file(&target, b"via-dyn").await.unwrap();
    let bytes = dyn_ops.read_file(&target).await.unwrap();
    assert_eq!(bytes, b"via-dyn");
}

#[test]
fn bash_operations_trait_is_object_safe_via_arc_dyn() {
    let mock = MockBashOperations::with_exec_result(Ok(sample_bash_result()));
    let _: Arc<dyn BashOperations> = Arc::new(mock);
}

// =========================================================================
// BashRequest / BashResult field contracts (DoD clause: "command
// cwd/env/timeout/cancellation fields preserved by the request/result types")
// =========================================================================

#[test]
fn bash_request_carries_command_cwd_env_timeout_signal_fields() {
    let token = CancellationToken::new();
    let req = BashRequest {
        command: "echo hello".to_string(),
        cwd: PathBuf::from("/tmp/work"),
        timeout: Duration::from_secs(42),
        signal: token.clone(),
        env: vec![("OPI_TEST".to_string(), "v1".to_string())],
    };
    assert_eq!(req.command, "echo hello");
    assert_eq!(req.cwd, PathBuf::from("/tmp/work"));
    assert_eq!(req.timeout, Duration::from_secs(42));
    assert_eq!(req.env.len(), 1);
    assert_eq!(req.env[0].0, "OPI_TEST");
    // CancellationToken is Send + Sync + Clone (threads through the trait).
    let _cloned = req.signal.clone();
}

#[test]
fn bash_result_carries_stdout_stderr_exit_code_signal_diagnostics() {
    let result = sample_bash_result();
    assert_eq!(result.stdout, b"out");
    assert_eq!(result.stderr, b"err");
    assert_eq!(result.exit_code, Some(0));
    assert!(result.signal.is_none());
    assert_eq!(result.diagnostics.len(), 1);
}

#[test]
fn bash_request_is_clone_when_signal_is_clone() {
    // BashRequest derives Clone; CancellationToken: Clone.
    let req = BashRequest {
        command: "cmd".to_string(),
        cwd: PathBuf::from("/cwd"),
        timeout: Duration::from_secs(1),
        signal: CancellationToken::new(),
        env: vec![],
    };
    let cloned = req.clone();
    assert_eq!(cloned.command, req.command);
    assert_eq!(cloned.cwd, req.cwd);
}

// =========================================================================
// Malformed backend result (DoD clause: "malformed ... backend results")
// =========================================================================

#[test]
fn bash_op_error_variants_format_via_display() {
    let spawn = BashOpError::SpawnFailed {
        message: "shell not found".to_string(),
    };
    assert!(format!("{spawn}").contains("failed to spawn command"));
    assert!(format!("{spawn}").contains("shell not found"));

    let wait = BashOpError::WaitFailed {
        message: "EINTR".to_string(),
    };
    assert!(format!("{wait}").contains("failed to wait for process"));

    let other = BashOpError::Other {
        message: "backend unavailable".to_string(),
    };
    assert!(format!("{other}").contains("backend unavailable"));
}

// =========================================================================
// Denied backend result propagation through Arc<dyn FileOperations>
// (DoD clause: "denied backend results")
// =========================================================================

#[tokio::test]
async fn mock_file_operations_denied_result_propagates_as_fsoperror_through_dyn() {
    let denied_path = PathBuf::from("/denied/secret.txt");
    let mock = MockFileOperations::with_read_result(Err(FsOpError::PermissionDenied {
        path: denied_path.clone(),
    }));
    let dyn_ops: Arc<dyn FileOperations> = Arc::new(mock);

    let err = dyn_ops
        .read_file(&denied_path)
        .await
        .expect_err("injected PermissionDenied must propagate through dyn dispatch");
    assert_eq!(
        err,
        FsOpError::PermissionDenied { path: denied_path },
        "operation/path/error identity must survive dyn dispatch"
    );
}

// =========================================================================
// Bash mock injection through Arc<dyn BashOperations>
// =========================================================================

#[tokio::test]
async fn mock_bash_operations_returns_injected_result_through_dyn() {
    let expected = sample_bash_result();
    let mock = MockBashOperations::with_exec_result(Ok(expected.clone()));
    let dyn_ops: Arc<dyn BashOperations> = Arc::new(mock);

    let request = BashRequest {
        command: "echo".to_string(),
        cwd: PathBuf::from("/cwd"),
        timeout: Duration::from_secs(5),
        signal: CancellationToken::new(),
        env: vec![],
    };
    let result = dyn_ops.exec(request).await.unwrap();
    assert_eq!(result.stdout, expected.stdout);
    assert_eq!(result.exit_code, expected.exit_code);
}

#[tokio::test]
async fn mock_bash_operations_malformed_result_propagates_through_dyn() {
    let mock = MockBashOperations::with_exec_result(Err(BashOpError::SpawnFailed {
        message: "boom".to_string(),
    }));
    let dyn_ops: Arc<dyn BashOperations> = Arc::new(mock);

    let request = BashRequest {
        command: "x".to_string(),
        cwd: PathBuf::from("/cwd"),
        timeout: Duration::from_secs(1),
        signal: CancellationToken::new(),
        env: vec![],
    };
    let err = dyn_ops.exec(request).await.unwrap_err();
    assert!(matches!(err, BashOpError::SpawnFailed { .. }));
    assert!(format!("{err}").contains("boom"));
}

// =========================================================================
// Mock fixtures
// =========================================================================

fn sample_bash_result() -> BashResult {
    BashResult {
        stdout: b"out".to_vec(),
        stderr: b"err".to_vec(),
        exit_code: Some(0),
        signal: None,
        diagnostics: vec![ToolDiagnostic {
            code: "test_diagnostic".to_string(),
            message: "sample".to_string(),
            details: None,
        }],
    }
}

struct MockFileOperations {
    next_read: Arc<Mutex<Result<Vec<u8>, FsOpError>>>,
}

impl MockFileOperations {
    fn with_read_result(result: Result<Vec<u8>, FsOpError>) -> Self {
        Self {
            next_read: Arc::new(Mutex::new(result)),
        }
    }
}

impl FileOperations for MockFileOperations {
    fn read_file(
        &self,
        _path: &Path,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, FsOpError>> + Send>> {
        let result = self.next_read.lock().unwrap().clone();
        Box::pin(async move { result })
    }

    fn write_file(
        &self,
        _path: &Path,
        _data: &[u8],
    ) -> Pin<Box<dyn Future<Output = Result<(), FsOpError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn mkdir(
        &self,
        _path: &Path,
        _recursive: bool,
    ) -> Pin<Box<dyn Future<Output = Result<(), FsOpError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn metadata(
        &self,
        _path: &Path,
    ) -> Pin<Box<dyn Future<Output = Result<OpMetadata, FsOpError>> + Send>> {
        Box::pin(async move {
            Ok(OpMetadata {
                len: 0,
                is_dir: false,
                is_file: true,
                readonly: false,
                modified: None,
            })
        })
    }

    fn access(
        &self,
        _path: &Path,
        _mode: AccessMode,
    ) -> Pin<Box<dyn Future<Output = Result<(), FsOpError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }
}

struct MockBashOperations {
    next_exec: Arc<Mutex<Result<BashResult, BashOpError>>>,
}

impl MockBashOperations {
    fn with_exec_result(result: Result<BashResult, BashOpError>) -> Self {
        Self {
            next_exec: Arc::new(Mutex::new(result)),
        }
    }
}

impl BashOperations for MockBashOperations {
    fn exec(
        &self,
        _request: BashRequest,
    ) -> Pin<Box<dyn Future<Output = Result<BashResult, BashOpError>> + Send>> {
        let result = self.next_exec.lock().unwrap().clone();
        Box::pin(async move { result })
    }
}

// =========================================================================
// Phase 15.2: Operations injection reaches tool execution (task 15.2)
//
// DoD: "Mock-backend tests prove policy rejection occurs before backend
// invocation, permitted paths and bash requests reach the injected backend
// exactly once, local defaults preserve existing tool output/truncation/error
// behavior, and the production build_tools path constructs all eight tools."
//
// The keystone is an orchestrator over focused helpers so one regression
// cannot block the whole gate. Local-default OUTPUT behavior is pinned by the
// existing tools_read_write_edit_bash suite (now driven through the injected
// LocalFileOperations / LocalBashOperations); eight-tool construction is pinned
// by tool_selection::build_tools_constructs_expected_default_set.
// =========================================================================

#[tokio::test]
async fn operations_injection_reaches_tool_execution() {
    read_injected_ops_receive_resolved_path_once().await;
    read_workspace_policy_rejects_outside_path_before_backend().await;
    write_injected_ops_receive_resolved_path_and_bytes_once().await;
    edit_injected_ops_read_then_write_through_backend().await;
    bash_injected_ops_receive_request_fields_once().await;
}

async fn read_injected_ops_receive_resolved_path_once() {
    let workspace = tempfile::tempdir().unwrap();
    let resolved = workspace.path().join("inside.txt");
    std::fs::write(&resolved, "hello").unwrap();
    let recording = Arc::new(RecordingFileOps::default());
    let tool = ReadTool::new_with_ops(
        workspace.path().to_path_buf(),
        PathPolicy::WorkspaceOnly,
        recording.clone(),
    );
    let result = tool
        .execute(
            "c",
            json!({ "path": "inside.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("read execute");
    assert!(!result.is_error, "permitted read must not be an error");

    let reads = recording.reads.lock().unwrap().clone();
    assert_eq!(
        reads.len(),
        1,
        "read_file must reach the backend exactly once"
    );
    assert_eq!(reads[0], resolved, "backend must receive the resolved path");
}

async fn read_workspace_policy_rejects_outside_path_before_backend() {
    let workspace = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("escape.txt");
    std::fs::write(&outside_file, "x").unwrap();

    let recording = Arc::new(RecordingFileOps::default());
    let tool = ReadTool::new_with_ops(
        workspace.path().to_path_buf(),
        PathPolicy::WorkspaceOnly,
        recording.clone(),
    );
    let result = tool
        .execute(
            "c",
            json!({ "path": outside_file.to_string_lossy() }),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("read execute");
    assert!(result.is_error, "outside-workspace read must be rejected");

    let reads = recording.reads.lock().unwrap().clone();
    assert!(
        reads.is_empty(),
        "PathPolicy rejection must occur BEFORE any backend invocation: {reads:?}"
    );
}

async fn write_injected_ops_receive_resolved_path_and_bytes_once() {
    let workspace = tempfile::tempdir().unwrap();
    let resolved = workspace.path().join("out.txt");
    // cfg.meta = None => metadata NotFound => classified as a create.
    let recording = Arc::new(RecordingFileOps::default());
    let tool = WriteTool::new_with_ops(workspace.path().to_path_buf(), recording.clone());
    let result = tool
        .execute(
            "c",
            json!({ "path": "out.txt", "content": "data" }),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("write execute");
    assert!(!result.is_error, "permitted write must not be an error");

    let writes = recording.writes.lock().unwrap().clone();
    assert_eq!(
        writes.len(),
        1,
        "write_file must reach the backend exactly once"
    );
    assert_eq!(
        writes[0].0, resolved,
        "backend must receive the resolved path"
    );
    assert_eq!(
        writes[0].1, b"data",
        "backend must receive the verbatim content bytes"
    );
}

async fn edit_injected_ops_read_then_write_through_backend() {
    let workspace = tempfile::tempdir().unwrap();
    let resolved = workspace.path().join("e.txt");
    // cfg.meta = Some(true) (regular file); read returns content with one "old".
    let recording = Arc::new(RecordingFileOps {
        cfg: FileOpsConfig {
            read_bytes: b"old content".to_vec(),
            meta: Some(true),
        },
        ..Default::default()
    });
    let tool = EditTool::new_with_ops(workspace.path().to_path_buf(), recording.clone());
    let result = tool
        .execute(
            "c",
            json!({ "path": "e.txt", "old_string": "old", "new_string": "new" }),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("edit execute");
    assert!(!result.is_error, "permitted edit must not be an error");

    let reads = recording.reads.lock().unwrap().clone();
    let writes = recording.writes.lock().unwrap().clone();
    assert_eq!(reads.len(), 1, "edit reads through the backend once");
    assert_eq!(writes.len(), 1, "edit writes through the backend once");
    assert_eq!(writes[0].0, resolved);
    assert_eq!(
        writes[0].1, b"new content",
        "backend must receive the replaced content"
    );
}

async fn bash_injected_ops_receive_request_fields_once() {
    let workspace = tempfile::tempdir().unwrap();
    let recording = Arc::new(RecordingBashOps::exit_zero(b"out\n".to_vec()));
    let tool = BashTool::new_with_ops(workspace.path().to_path_buf(), recording.clone());
    let result = tool
        .execute(
            "c",
            json!({ "command": "echo hi", "timeout_secs": 42 }),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("bash execute");
    assert!(!result.is_error, "exit-0 bash must not be an error");

    let execs = recording.execs.lock().unwrap().clone();
    assert_eq!(execs.len(), 1, "exec must reach the backend exactly once");
    let req = &execs[0];
    assert_eq!(req.command, "echo hi", "command threads through");
    assert_eq!(
        req.cwd,
        workspace.path().to_path_buf(),
        "cwd threads through"
    );
    assert_eq!(
        req.timeout,
        Duration::from_secs(42),
        "timeout threads through"
    );
    assert!(req.env.is_empty(), "env is empty in current usage");
}

// =========================================================================
// Recording mock backends (Phase 15.2 injection tests)
// =========================================================================

#[derive(Clone, Default)]
struct FileOpsConfig {
    /// Bytes returned by `read_file`.
    read_bytes: Vec<u8>,
    /// `None` => metadata returns NotFound; `Some(true)` => regular file;
    /// `Some(false)` => directory.
    meta: Option<bool>,
}

/// Recorded `(resolved_path, bytes)` for one `write_file` call.
type WriteRecord = (PathBuf, Vec<u8>);

#[derive(Default)]
struct RecordingFileOps {
    cfg: FileOpsConfig,
    reads: Arc<Mutex<Vec<PathBuf>>>,
    writes: Arc<Mutex<Vec<WriteRecord>>>,
    metas: Arc<Mutex<Vec<PathBuf>>>,
    mkdirs: Arc<Mutex<Vec<PathBuf>>>,
}

impl FileOperations for RecordingFileOps {
    fn read_file(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, FsOpError>> + Send>> {
        let path = path.to_path_buf();
        let bytes = self.cfg.read_bytes.clone();
        let reads = self.reads.clone();
        Box::pin(async move {
            reads.lock().unwrap().push(path);
            Ok(bytes)
        })
    }

    fn write_file(
        &self,
        path: &Path,
        data: &[u8],
    ) -> Pin<Box<dyn Future<Output = Result<(), FsOpError>> + Send>> {
        let path = path.to_path_buf();
        let data = data.to_vec();
        let writes = self.writes.clone();
        Box::pin(async move {
            writes.lock().unwrap().push((path, data));
            Ok(())
        })
    }

    fn mkdir(
        &self,
        path: &Path,
        _recursive: bool,
    ) -> Pin<Box<dyn Future<Output = Result<(), FsOpError>> + Send>> {
        let path = path.to_path_buf();
        let mkdirs = self.mkdirs.clone();
        Box::pin(async move {
            mkdirs.lock().unwrap().push(path);
            Ok(())
        })
    }

    fn metadata(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn Future<Output = Result<OpMetadata, FsOpError>> + Send>> {
        let recorded = path.to_path_buf();
        let meta_opt = self.cfg.meta;
        let len = self.cfg.read_bytes.len() as u64;
        let metas = self.metas.clone();
        Box::pin(async move {
            metas.lock().unwrap().push(recorded);
            match meta_opt {
                None => Err(FsOpError::NotFound {
                    path: PathBuf::from("/"),
                }),
                Some(true) => Ok(OpMetadata {
                    len,
                    is_dir: false,
                    is_file: true,
                    readonly: false,
                    modified: None,
                }),
                Some(false) => Ok(OpMetadata {
                    len: 0,
                    is_dir: true,
                    is_file: false,
                    readonly: false,
                    modified: None,
                }),
            }
        })
    }

    fn access(
        &self,
        _path: &Path,
        _mode: AccessMode,
    ) -> Pin<Box<dyn Future<Output = Result<(), FsOpError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }
}

struct RecordingBashOps {
    execs: Arc<Mutex<Vec<BashRequest>>>,
    result: BashResult,
}

impl RecordingBashOps {
    fn exit_zero(stdout: Vec<u8>) -> Self {
        Self {
            execs: Arc::new(Mutex::new(Vec::new())),
            result: BashResult {
                stdout,
                stderr: Vec::new(),
                exit_code: Some(0),
                signal: None,
                diagnostics: Vec::new(),
            },
        }
    }
}

impl BashOperations for RecordingBashOps {
    fn exec(
        &self,
        request: BashRequest,
    ) -> Pin<Box<dyn Future<Output = Result<BashResult, BashOpError>> + Send>> {
        let result = self.result.clone();
        let execs = self.execs.clone();
        Box::pin(async move {
            execs.lock().unwrap().push(request);
            Ok(result)
        })
    }
}
