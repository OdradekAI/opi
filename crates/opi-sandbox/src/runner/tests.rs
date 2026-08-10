use super::gated::{StartProbe, acknowledgement_path};
#[cfg(windows)]
use super::preparation::resolve_program;
use super::supervision::{SupervisionControl, remove_temp_root_until, supervise};
use super::*;

const TEST_START_TOKEN: &[u8] = b"test-start-token-0123456789abcdef";

#[cfg(unix)]
struct PassThroughLauncher;

#[cfg(unix)]
impl Restriction for PassThroughLauncher {
    fn launcher(
        &self,
        _ctx: &RestrictionCtx<'_>,
    ) -> Result<Option<LauncherSpec>, crate::policy::RestrictionSetupError> {
        Ok(Some(LauncherSpec {
            program: PathBuf::from("/usr/bin/env"),
            prefix: Vec::new(),
        }))
    }

    fn prepare(
        &self,
        _cmd: &mut Command,
        _ctx: &RestrictionCtx<'_>,
    ) -> Result<crate::policy::AppliedRestriction, crate::policy::RestrictionSetupError> {
        Ok(crate::policy::AppliedRestriction {
            mechanism: Mechanism::Seatbelt,
            contract: ContractStatus::Restricted,
        })
    }
}

fn fake_seatbelt_run(script: &str) -> (SandboxRun, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().expect("temp root");
    let temp_root = temp.path().to_path_buf();
    let release_gate = temp_root.join("release.armed");
    std::fs::write(&release_gate, b"").expect("create release gate");
    let probe = acknowledgement_path(&release_gate);
    let marker_root = tempfile::tempdir().expect("marker root").keep();
    let marker = marker_root.join("released");
    let mut cmd = if cfg!(windows) {
        let mut cmd = Command::new("powershell");
        cmd.args(["-NoProfile", "-Command", script]);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", script]);
        cmd
    };
    cmd.env("OPI_TEST_GATE", &release_gate)
        .env("OPI_TEST_PROBE", &probe)
        .env(
            "OPI_TEST_TOKEN",
            std::str::from_utf8(TEST_START_TOKEN).expect("ASCII test token"),
        )
        .env("OPI_TEST_MARKER", &marker)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    configure_tree(&mut cmd);
    let child = cmd.spawn().expect("spawn fake launcher");
    let child_pid = child.id();
    let tree = TreeGuard::attach_child(child_pid).expect("attach fake launcher");
    #[cfg(windows)]
    let tree = {
        let tree = tree;
        resume_child(child_pid.expect("child pid")).expect("resume fake launcher");
        tree
    };
    let now = Instant::now();
    let deadlines = RunDeadlines::new(
        now + Duration::from_secs(3),
        now + Duration::from_secs(4),
        Duration::from_secs(3),
    );
    let deadline_cell = Arc::new(OnceLock::new());
    deadline_cell.set(deadlines).expect("set deadlines");
    let cancel = CancellationToken::new();
    let (event_tx, event_rx) = mpsc::channel(OUTPUT_EVENT_CAPACITY);
    let inner = Box::pin(supervise(
        child,
        tree,
        temp,
        None,
        temp_root.clone(),
        SupervisionControl {
            deadline_cell: Arc::clone(&deadline_cell),
            cancel: cancel.clone(),
            faults: FaultInjection::default(),
            event_tx,
        },
    ));
    let run = SandboxRun {
        started_emitted: false,
        completed: false,
        auto_release: true,
        temp_root,
        child_pid,
        mechanism: Mechanism::Seatbelt,
        contract: ContractStatus::Restricted,
        release_gate: Some(release_gate),
        start_probe: Some(StartProbe {
            path: probe.clone(),
            token: TEST_START_TOKEN.to_vec(),
        }),
        start_probe_poll: None,
        start_probe_rejected: false,
        prestart_result: None,
        cancel,
        deadline_plan: RunDeadlinePlan::Fixed(deadlines),
        deadline_cell,
        event_rx,
        terminal_result: None,
        inner: Some(inner),
    };
    (run, probe, marker)
}

async fn next(run: &mut SandboxRun) -> Option<SandboxEvent> {
    std::future::poll_fn(|cx| Pin::new(&mut *run).poll_next(cx)).await
}

/// Drain interim Output/Diagnostic events until the terminal Completed event.
/// Stdout-producing targets (for example `/usr/bin/env`) emit Output chunks
/// between Started and Completed; on macOS the inherited environment adds
/// extra stdout, so callers that need only the final result must drain rather
/// than assert the immediately-following event is Completed.
#[cfg(unix)]
async fn next_completed(run: &mut SandboxRun) -> SandboxResult {
    loop {
        match next(run).await {
            Some(SandboxEvent::Completed(result)) => return result,
            Some(_) => {}
            None => panic!("run ended without a Completed event"),
        }
    }
}

struct ProbeAndExitTogether {
    probe: PathBuf,
    token: Vec<u8>,
    first_poll: bool,
    result: Option<SandboxResult>,
}

impl std::future::Future for ProbeAndExitTogether {
    type Output = SandboxResult;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.first_poll {
            self.first_poll = false;
            std::fs::write(&self.probe, &self.token).expect("write simultaneous probe");
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        Poll::Ready(self.result.take().expect("one terminal result"))
    }
}

#[tokio::test]
async fn launcher_exit_wins_when_probe_and_exit_become_ready_together() {
    let temp = tempfile::tempdir().expect("temp root");
    let temp_root = temp.path().to_path_buf();
    let release_gate = temp_root.join("release.armed");
    std::fs::write(&release_gate, b"").expect("create release gate");
    let probe = acknowledgement_path(&release_gate);
    let now = Instant::now();
    let deadlines = RunDeadlines::new(
        now + Duration::from_secs(3),
        now + Duration::from_secs(4),
        Duration::from_secs(3),
    );
    let deadline_cell = Arc::new(OnceLock::new());
    deadline_cell.set(deadlines).expect("set deadlines");
    let (event_tx, event_rx) = mpsc::channel(OUTPUT_EVENT_CAPACITY);
    drop(event_tx);
    let result = SandboxResult {
        outcome: SandboxOutcome::Exited { code: Some(68) },
        cleanup: CleanupState::Confirmed,
        stdout: Vec::new(),
        stderr: Vec::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        temp_root: temp_root.clone(),
    };
    let mut run = SandboxRun {
        started_emitted: false,
        completed: false,
        auto_release: true,
        temp_root,
        child_pid: None,
        mechanism: Mechanism::Seatbelt,
        contract: ContractStatus::Restricted,
        release_gate: Some(release_gate),
        start_probe: Some(StartProbe {
            path: probe.clone(),
            token: TEST_START_TOKEN.to_vec(),
        }),
        start_probe_poll: None,
        start_probe_rejected: false,
        prestart_result: None,
        cancel: CancellationToken::new(),
        deadline_plan: RunDeadlinePlan::Fixed(deadlines),
        deadline_cell,
        event_rx,
        terminal_result: None,
        inner: Some(Box::pin(ProbeAndExitTogether {
            probe,
            token: TEST_START_TOKEN.to_vec(),
            first_poll: true,
            result: Some(result),
        })),
    };

    let event = next(&mut run).await;

    assert!(matches!(event, Some(SandboxEvent::Completed(_))));
}

#[tokio::test]
async fn fake_profile_rejection_emits_no_started_event() {
    let (mut run, probe, marker) = fake_seatbelt_run("exit 65");

    let event = next(&mut run).await;

    assert!(matches!(event, Some(SandboxEvent::Completed(_))));
    assert!(!probe.exists(), "rejected profile emitted no proof");
    assert!(!marker.exists(), "rejected launcher never released target");
}

#[tokio::test]
async fn fake_launcher_early_exit_emits_no_started_event() {
    let script = if cfg!(windows) {
        "Start-Sleep -Milliseconds 50; exit 66"
    } else {
        "sleep 0.05; exit 66"
    };
    let (mut run, probe, marker) = fake_seatbelt_run(script);

    let event = next(&mut run).await;

    assert!(matches!(event, Some(SandboxEvent::Completed(_))));
    assert!(!probe.exists(), "early launcher exit emitted no proof");
    assert!(
        !marker.exists(),
        "early launcher exit never released target"
    );
}

#[tokio::test]
async fn fake_launcher_rejection_classifies_as_prestart_restriction_setup() {
    let (mut run, _probe, marker) = fake_seatbelt_run("exit 67");

    let failure = run
        .confirm_start_until(
            Instant::now() + Duration::from_secs(2),
            Instant::now() + Duration::from_secs(3),
        )
        .await
        .expect_err("missing acknowledgement is a pre-start failure");

    assert!(matches!(
        failure,
        StartConfirmationFailure::RestrictionSetup {
            cleanup: CleanupState::Confirmed
        }
    ));
    assert!(!marker.exists(), "restriction failure keeps target gated");
}

#[tokio::test]
async fn forged_probe_content_is_rejected_immediately() {
    let script = if cfg!(windows) {
        "$tmp = \"$env:OPI_TEST_PROBE.tmp\"; Set-Content -NoNewline -LiteralPath $tmp -Value forged; Move-Item -LiteralPath $tmp -Destination $env:OPI_TEST_PROBE; while (Test-Path -LiteralPath $env:OPI_TEST_GATE) { Start-Sleep -Milliseconds 10 }"
    } else {
        "tmp=\"${OPI_TEST_PROBE}.tmp\"; printf forged > \"$tmp\"; ln \"$tmp\" \"$OPI_TEST_PROBE\"; rm -f \"$tmp\"; while [ -e \"$OPI_TEST_GATE\" ]; do sleep 0.01; done"
    };
    let (mut run, _probe, marker) = fake_seatbelt_run(script);

    let event = tokio::time::timeout(Duration::from_millis(500), next(&mut run))
        .await
        .expect("wrong probe content must fail without waiting for the deadline");

    assert!(matches!(event, Some(SandboxEvent::Completed(_))));
    assert!(!marker.exists(), "forged proof never releases the target");
}

#[tokio::test]
async fn non_regular_probe_is_rejected_immediately() {
    let script = if cfg!(windows) {
        "New-Item -ItemType Directory -Path $env:OPI_TEST_PROBE | Out-Null; while (Test-Path -LiteralPath $env:OPI_TEST_GATE) { Start-Sleep -Milliseconds 10 }"
    } else {
        "mkdir \"$OPI_TEST_PROBE\"; while [ -e \"$OPI_TEST_GATE\" ]; do sleep 0.01; done"
    };
    let (mut run, _probe, marker) = fake_seatbelt_run(script);

    let event = tokio::time::timeout(Duration::from_millis(500), next(&mut run))
        .await
        .expect("non-regular probe must fail without waiting for the deadline");

    assert!(matches!(event, Some(SandboxEvent::Completed(_))));
    assert!(!marker.exists(), "non-regular proof never releases target");
}

#[cfg(unix)]
#[tokio::test]
async fn symlink_probe_is_rejected_immediately() {
    let script = "printf '%s' \"$OPI_TEST_TOKEN\" > \"${OPI_TEST_PROBE}.target\"; ln -s \"${OPI_TEST_PROBE}.target\" \"$OPI_TEST_PROBE\"; while [ -e \"$OPI_TEST_GATE\" ]; do sleep 0.01; done";
    let (mut run, _probe, marker) = fake_seatbelt_run(script);

    let event = tokio::time::timeout(Duration::from_millis(500), next(&mut run))
        .await
        .expect("symlink probe must fail without waiting for the deadline");

    assert!(matches!(event, Some(SandboxEvent::Completed(_))));
    assert!(!marker.exists(), "symlink proof never releases target");
}

#[tokio::test]
async fn started_waits_for_in_profile_acknowledgement_before_release() {
    let script = if cfg!(windows) {
        "$tmp = \"$env:OPI_TEST_PROBE.tmp\"; Start-Sleep -Milliseconds 150; Set-Content -NoNewline -LiteralPath $tmp -Value $env:OPI_TEST_TOKEN; Move-Item -LiteralPath $tmp -Destination $env:OPI_TEST_PROBE; Remove-Item Env:OPI_TEST_TOKEN; while (Test-Path -LiteralPath $env:OPI_TEST_GATE) { Start-Sleep -Milliseconds 10 }; Set-Content -LiteralPath $env:OPI_TEST_MARKER -Value released"
    } else {
        "tmp=\"${OPI_TEST_PROBE}.tmp\"; sleep 0.15; printf '%s' \"$OPI_TEST_TOKEN\" > \"$tmp\"; ln \"$tmp\" \"$OPI_TEST_PROBE\"; rm -f \"$tmp\"; unset OPI_TEST_TOKEN; while [ -e \"$OPI_TEST_GATE\" ]; do sleep 0.01; done; printf released > \"$OPI_TEST_MARKER\""
    };
    let (mut run, probe, marker) = fake_seatbelt_run(script);

    assert!(
        tokio::time::timeout(Duration::from_millis(50), next(&mut run))
            .await
            .is_err(),
        "Started must remain pending before the in-profile proof"
    );
    assert!(matches!(
        next(&mut run).await,
        Some(SandboxEvent::Started { .. })
    ));
    assert_eq!(
        std::fs::read(&probe).expect("read proof"),
        TEST_START_TOKEN,
        "proof content must match the per-run token before Started"
    );
    assert!(
        !marker.exists(),
        "target stays gated until explicit release"
    );
    run.release().expect("release target");
    assert!(matches!(
        next(&mut run).await,
        Some(SandboxEvent::Completed(_))
    ));
    assert!(marker.exists(), "target ran only after release");
}

#[test]
fn background_setup_path_has_no_spawn_capability() {
    let helper_source = include_str!("../helper.rs");
    let gated_source = include_str!("gated.rs");
    let helper_production = helper_source
        .split("\n#[cfg(test)]\nmod tests")
        .next()
        .expect("helper production source");
    let start_body = helper_production
        .split("pub(crate) async fn start(")
        .nth(1)
        .expect("helper start function");

    assert!(
        start_body.contains("spawn_blocking") && start_body.contains("prepare_validated_until"),
        "helper setup workers must call the preparation-only runner path"
    );
    assert!(
        !start_body.contains("run_validated_until") && !start_body.contains("cmd.spawn"),
        "a background setup worker must not own a path that can spawn"
    );
    assert!(
        gated_source.contains("pub(crate) fn spawn_prepared("),
        "actual spawn must be a distinct awaiting-path operation"
    );
}

#[cfg(windows)]
#[test]
fn windows_program_resolution_uses_request_path_case_insensitively() {
    let cwd = tempfile::tempdir().expect("cwd");
    let tools = tempfile::tempdir().expect("tools");
    let executable = tools.path().join("phase16-path-probe.exe");
    std::fs::write(&executable, b"fixture").expect("write fixture executable");
    let additions = [(
        OsString::from("Path"),
        tools.path().as_os_str().to_os_string(),
    )]
    .into_iter()
    .collect();

    assert_eq!(
        resolve_program(
            Path::new("phase16-path-probe"),
            cwd.path(),
            EnvInherit::Clear,
            &additions,
        ),
        Some(executable),
    );
}

#[cfg(windows)]
#[test]
fn windows_clear_environment_does_not_search_ambient_path() {
    let cwd = tempfile::tempdir().expect("cwd");
    assert_eq!(
        resolve_program(
            Path::new("cmd"),
            cwd.path(),
            EnvInherit::Clear,
            &BTreeMap::new(),
        ),
        None,
    );
}

fn request(program: PathBuf, args: Vec<OsString>) -> (SandboxRequest, tempfile::TempDir) {
    let workspace = tempfile::tempdir().expect("workspace");
    (
        SandboxRequest {
            program,
            args,
            workspace: workspace.path().to_path_buf(),
            cwd: workspace.path().to_path_buf(),
            timeout: Duration::from_secs(5),
            env_inherit: EnvInherit::Inherit,
            env_additions: BTreeMap::new(),
            stdin: StdinPolicy::Null,
            cancel: None,
        },
        workspace,
    )
}

fn exit_request() -> (SandboxRequest, tempfile::TempDir) {
    if cfg!(windows) {
        request(
            PathBuf::from("cmd"),
            vec![OsString::from("/C"), OsString::from("exit 0")],
        )
    } else {
        request(
            PathBuf::from("sh"),
            vec![OsString::from("-c"), OsString::from("exit 0")],
        )
    }
}

#[tokio::test]
async fn direct_run_cancelled_after_spawn_stays_gated_and_cleans_up() {
    let marker_dir = tempfile::tempdir().expect("marker directory");
    let marker = marker_dir.path().join("must-not-exist");
    let (program, args) = if cfg!(windows) {
        (
            PathBuf::from("powershell"),
            vec![
                OsString::from("-NoProfile"),
                OsString::from("-Command"),
                OsString::from(format!(
                    "Set-Content -LiteralPath '{}' -Value x",
                    marker.display()
                )),
            ],
        )
    } else {
        (
            PathBuf::from("sh"),
            vec![
                OsString::from("-c"),
                OsString::from(format!("printf x > '{}'", marker.display())),
            ],
        )
    };
    let cancel = CancellationToken::new();
    let post_spawn_gate: &'static PostSpawnGate = Box::leak(Box::new(PostSpawnGate::new()));
    let cancel_after_spawn = cancel.clone();
    let gate_worker = std::thread::spawn(move || {
        post_spawn_gate.cancel_after_spawn(&cancel_after_spawn);
    });
    let (mut request, _workspace) = request(program, args);
    request.cancel = Some(cancel);
    let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(crate::NoRestriction))
        .with_faults(FaultInjection {
            post_spawn_gate: Some(post_spawn_gate),
            ..FaultInjection::default()
        });

    let mut run = runner
        .run(request)
        .expect("post-spawn cancellation returns a guarded run");
    gate_worker.join().expect("post-spawn cancellation worker");
    let temp_root = run.temp_root().to_path_buf();
    let result = tokio::time::timeout(Duration::from_secs(3), async {
        assert!(matches!(
            next(&mut run).await,
            Some(SandboxEvent::Started { .. })
        ));
        match next(&mut run).await {
            Some(SandboxEvent::Completed(result)) => result,
            other => panic!("expected cancelled completion, got {other:?}"),
        }
    })
    .await
    .expect("post-spawn cancellation cleanup is bounded");

    assert_eq!(result.outcome, SandboxOutcome::Cancelled);
    assert_eq!(result.cleanup, CleanupState::Confirmed);
    assert!(
        !marker.exists(),
        "cancelled target crossed its release gate"
    );
    assert!(!temp_root.exists(), "cancelled run removed its temp root");
}

#[tokio::test]
async fn cancel_after_started_wins_before_auto_release() {
    let marker_dir = tempfile::tempdir().expect("marker directory");
    let marker = marker_dir.path().join("must-not-exist");
    let (program, args) = if cfg!(windows) {
        (
            PathBuf::from("powershell"),
            vec![
                OsString::from("-NoProfile"),
                OsString::from("-Command"),
                OsString::from(format!(
                    "Set-Content -LiteralPath '{}' -Value x",
                    marker.display()
                )),
            ],
        )
    } else {
        (
            PathBuf::from("sh"),
            vec![
                OsString::from("-c"),
                OsString::from(format!("printf x > '{}'", marker.display())),
            ],
        )
    };
    let cancel = CancellationToken::new();
    let cancel_cleanup_gate: &'static PostSpawnGate = Box::leak(Box::new(PostSpawnGate::new()));
    let (mut request, _workspace) = request(program, args);
    request.cancel = Some(cancel.clone());
    let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(crate::NoRestriction))
        .with_faults(FaultInjection {
            cancel_cleanup_gate: Some(cancel_cleanup_gate),
            ..FaultInjection::default()
        });
    let mut run = runner.run(request).expect("run starts behind its gate");
    assert!(matches!(
        next(&mut run).await,
        Some(SandboxEvent::Started { .. })
    ));

    let observed_marker = marker.clone();
    let observer = std::thread::spawn(move || {
        cancel_cleanup_gate.observe_before_cleanup(|| {
            let deadline = std::time::Instant::now() + Duration::from_millis(500);
            while !observed_marker.exists() && std::time::Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(5));
            }
        });
    });
    cancel.cancel();
    let result = tokio::time::timeout(Duration::from_secs(3), async {
        match next(&mut run).await {
            Some(SandboxEvent::Completed(result)) => result,
            other => panic!("expected cancelled completion, got {other:?}"),
        }
    })
    .await
    .expect("post-Started cancellation cleanup is bounded");
    observer.join().expect("cancellation cleanup observer");

    assert_eq!(result.outcome, SandboxOutcome::Cancelled);
    assert_eq!(result.cleanup, CleanupState::Confirmed);
    assert!(
        !marker.exists(),
        "cancelled target crossed its release gate"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn unix_bootstrap_preserves_caller_token_environment_variables() {
    let (mut request, _workspace) = request(PathBuf::from("/usr/bin/env"), Vec::new());
    request.env_inherit = EnvInherit::Clear;
    for (key, value) in [
        ("token", "sentinel-token"),
        ("gate", "sentinel-gate"),
        ("backend", "sentinel-backend"),
        ("mode", "sentinel-mode"),
        ("leader", "sentinel-leader"),
        ("token_peer", "sentinel-peer"),
        ("PATH", "/usr/bin:/bin"),
    ] {
        request
            .env_additions
            .insert(OsString::from(key), OsString::from(value));
    }
    let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(crate::NoRestriction));
    let mut run = runner.run(request).expect("run starts");

    assert!(matches!(
        next(&mut run).await,
        Some(SandboxEvent::Started { .. })
    ));
    let result = next_completed(&mut run).await;

    assert_eq!(
        result.outcome,
        SandboxOutcome::Exited { code: Some(0) },
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let environment = String::from_utf8(result.stdout).expect("target environment is UTF-8");
    for (key, value) in [
        ("token", "sentinel-token"),
        ("gate", "sentinel-gate"),
        ("backend", "sentinel-backend"),
        ("mode", "sentinel-mode"),
        ("leader", "sentinel-leader"),
        ("token_peer", "sentinel-peer"),
    ] {
        assert!(
            environment
                .lines()
                .any(|line| line == format!("{key}={value}")),
            "caller environment entry {key} was changed: {environment:?}"
        );
    }
}

#[cfg(unix)]
#[tokio::test]
async fn unix_acknowledgement_does_not_require_request_path() {
    let (mut request, _workspace) = request(PathBuf::from("/usr/bin/env"), Vec::new());
    request.env_inherit = EnvInherit::Clear;
    request
        .env_additions
        .insert(OsString::from("PATH"), OsString::new());
    let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(PassThroughLauncher));
    let mut run = runner.run(request).expect("run starts");

    assert!(matches!(
        next(&mut run).await,
        Some(SandboxEvent::Started { .. })
    ));
    let result = next_completed(&mut run).await;

    assert_eq!(
        result.outcome,
        SandboxOutcome::Exited { code: Some(0) },
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        String::from_utf8(result.stdout)
            .expect("target environment is UTF-8")
            .lines()
            .any(|line| line == "PATH="),
        "target must receive the caller's empty PATH"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn unix_acknowledgement_ignores_hostile_request_path_utilities() {
    use std::os::unix::fs::PermissionsExt;

    let hostile = tempfile::tempdir().expect("hostile PATH directory");
    let marker_root = tempfile::tempdir().expect("marker directory");
    let marker = marker_root.path().join("hostile-utility-ran");
    for utility in ["ln", "rm", "sleep"] {
        let shim = hostile.path().join(utility);
        std::fs::write(
                &shim,
                format!(
                    "#!/bin/sh\nprintf '%s\\n' '{utility}' >> \"$OPI_TEST_HOSTILE_MARKER\"\nexec /bin/{utility} \"$@\"\n"
                ),
            )
            .expect("write hostile utility shim");
        std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755))
            .expect("make hostile utility shim executable");
    }
    let (mut request, _workspace) = request(PathBuf::from("/usr/bin/env"), Vec::new());
    request.env_inherit = EnvInherit::Clear;
    request.env_additions.insert(
        OsString::from("PATH"),
        hostile.path().as_os_str().to_os_string(),
    );
    request.env_additions.insert(
        OsString::from("OPI_TEST_HOSTILE_MARKER"),
        marker.as_os_str().to_os_string(),
    );
    let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(PassThroughLauncher));
    let mut run = runner.run(request).expect("run starts");

    assert!(matches!(
        next(&mut run).await,
        Some(SandboxEvent::Started { .. })
    ));
    assert!(
        !marker.exists(),
        "request-controlled utilities ran before Started"
    );
    run.release().expect("release target");
    let result = next_completed(&mut run).await;

    assert_eq!(result.outcome, SandboxOutcome::Exited { code: Some(0) });
    assert!(!marker.exists(), "request-controlled utilities ran");
    let environment = String::from_utf8(result.stdout).expect("target environment is UTF-8");
    let expected_path = format!("PATH={}", hostile.path().display());
    assert!(
        environment.lines().any(|line| line == expected_path),
        "target must receive its caller-provided PATH: {environment:?}"
    );
}

#[test]
fn unix_bootstrap_has_no_out_of_profile_filesystem_redirection() {
    let source = include_str!("gated.rs");
    let script = source
        .split_once("const SCRIPT: &str = r#\"")
        .expect("Unix bootstrap start")
        .1
        .split_once("\"#;")
        .expect("Unix bootstrap end")
        .0;

    for line in script.lines().filter(|line| line.contains('>')) {
        assert!(
            line.contains("2>&-") || line.contains("> \"$2.probe.tmp.$$\""),
            "in-profile bootstrap redirects to a filesystem path outside the invocation root: {line}"
        );
    }
}

async fn complete_with_faults(faults: FaultInjection) -> SandboxResult {
    let (request, _workspace) = exit_request();
    let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(crate::NoRestriction))
        .with_faults(faults);
    let mut run = runner.run(request).expect("run starts");
    assert!(matches!(
        std::future::poll_fn(|cx| Pin::new(&mut run).poll_next(cx)).await,
        Some(SandboxEvent::Started { .. })
    ));
    match std::future::poll_fn(|cx| Pin::new(&mut run).poll_next(cx)).await {
        Some(SandboxEvent::Completed(result)) => result,
        other => panic!("expected Completed, got {other:?}"),
    }
}

#[tokio::test]
async fn injected_attach_failure_refuses_before_target_release() {
    let marker_dir = tempfile::tempdir().expect("marker dir");
    let marker = marker_dir.path().join("must-not-exist");
    let (program, args) = if cfg!(windows) {
        (
            PathBuf::from("powershell"),
            vec![
                OsString::from("-NoProfile"),
                OsString::from("-Command"),
                OsString::from(format!(
                    "Set-Content -LiteralPath '{}' -Value x",
                    marker.display()
                )),
            ],
        )
    } else {
        (
            PathBuf::from("sh"),
            vec![
                OsString::from("-c"),
                OsString::from(format!("printf x > '{}'", marker.display())),
            ],
        )
    };
    let (request, _workspace) = request(program, args);
    let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(crate::NoRestriction))
        .with_faults(FaultInjection {
            attach: true,
            ..FaultInjection::default()
        });
    let failure = match runner.run(request) {
        Ok(run) => {
            drop(run);
            panic!("attach failure must refuse")
        }
        Err(failure) => failure,
    };
    assert_eq!(failure.reason, SetupFailureReason::SpawnFailed);
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert!(!marker.exists(), "target crossed a failed attach gate");
}

#[tokio::test]
async fn injected_cleanup_failures_are_reported_unconfirmed() {
    for faults in [
        FaultInjection {
            terminate: true,
            ..FaultInjection::default()
        },
        FaultInjection {
            wait: true,
            ..FaultInjection::default()
        },
        FaultInjection {
            temp: true,
            ..FaultInjection::default()
        },
    ] {
        let result = complete_with_faults(faults).await;
        assert_eq!(result.cleanup, CleanupState::Unconfirmed);
    }
}

#[tokio::test]
async fn delayed_temp_removal_is_bounded_by_the_hard_deadline() {
    let temp = tempfile::tempdir().expect("temp root");
    let temp_root = temp.path().to_path_buf();
    let deadline = Instant::now() + Duration::from_millis(50);

    let confirmed = tokio::time::timeout(
        Duration::from_millis(500),
        remove_temp_root_until(temp, deadline, Duration::from_secs(1)),
    )
    .await
    .expect("hard deadline bounds temp removal");

    assert!(!confirmed, "removal past the deadline is unconfirmed");
    tokio::time::timeout(Duration::from_secs(2), async {
        while temp_root.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("detached remover eventually finishes");
}

#[tokio::test]
async fn pre_spawn_expiry_does_not_implicitly_block_on_prepared_cleanup() {
    let (request, _workspace) = exit_request();
    let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(crate::NoRestriction))
        .with_faults(FaultInjection {
            prepared_temp_remove_delay: Duration::from_secs(1),
            ..FaultInjection::default()
        });
    let request = runner.validate_request(request).expect("valid request");
    let prepared = runner
        .prepare_validated_until(request, None)
        .expect("prepared run");
    let now = Instant::now();
    let deadlines = RunDeadlines::new(
        now - Duration::from_millis(1),
        now + Duration::from_secs(2),
        Duration::from_secs(5),
    );
    let wall_start = std::time::Instant::now();

    let prepared = match runner.spawn_prepared(prepared, RunDeadlinePlan::Fixed(deadlines)) {
        SpawnPreparedOutcome::Expired(prepared) => prepared,
        _ => panic!("expired preparation must be returned without spawning"),
    };
    assert!(
        wall_start.elapsed() < Duration::from_millis(400),
        "pre-spawn expiry implicitly waited for prepared cleanup"
    );
    let cleanup_start = std::time::Instant::now();
    let confirmed =
        cleanup_prepared_until(*prepared, Instant::now() + Duration::from_millis(50)).await;
    assert!(!confirmed, "late prepared cleanup must be unconfirmed");
    assert!(
        cleanup_start.elapsed() < Duration::from_millis(400),
        "prepared cleanup exceeded its hard deadline"
    );
}

#[tokio::test(start_paused = true)]
async fn release_is_idempotent_after_the_execution_deadline() {
    let (mut request, _workspace) = exit_request();
    request.timeout = Duration::from_millis(50);
    let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(crate::NoRestriction));
    let mut run = runner.run(request).expect("run starts gated");

    assert!(matches!(
        std::future::poll_fn(|cx| Pin::new(&mut run).poll_next(cx)).await,
        Some(SandboxEvent::Started { .. })
    ));
    run.release().expect("first release succeeds");
    tokio::time::advance(Duration::from_millis(51)).await;

    run.release()
        .expect("repeated release remains a successful no-op");
}

#[tokio::test(start_paused = true)]
async fn unreleased_run_refuses_release_after_the_execution_deadline() {
    let (mut request, _workspace) = exit_request();
    request.timeout = Duration::from_millis(50);
    let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(crate::NoRestriction));
    let mut run = runner.run(request).expect("run starts gated");

    assert!(matches!(
        std::future::poll_fn(|cx| Pin::new(&mut run).poll_next(cx)).await,
        Some(SandboxEvent::Started { .. })
    ));
    tokio::time::advance(Duration::from_millis(51)).await;

    let error = run.release().expect_err("expired gate must stay closed");
    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
}

#[tokio::test(start_paused = true)]
async fn expired_auto_release_keeps_the_target_behind_its_gate() {
    let marker_dir = tempfile::tempdir().expect("marker dir");
    let marker = marker_dir.path().join("must-not-exist");
    let (program, args) = if cfg!(windows) {
        (
            PathBuf::from("powershell"),
            vec![
                OsString::from("-NoProfile"),
                OsString::from("-Command"),
                OsString::from(format!(
                    "Set-Content -LiteralPath '{}' -Value x",
                    marker.display()
                )),
            ],
        )
    } else {
        (
            PathBuf::from("sh"),
            vec![
                OsString::from("-c"),
                OsString::from(format!("printf x > '{}'", marker.display())),
            ],
        )
    };
    let (mut request, _workspace) = request(program, args);
    request.timeout = Duration::from_millis(50);
    let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(crate::NoRestriction))
        .with_faults(FaultInjection {
            terminate_delay: Duration::from_millis(200),
            ..FaultInjection::default()
        });
    let mut run = runner.run(request).expect("run starts gated");

    assert!(matches!(
        std::future::poll_fn(|cx| Pin::new(&mut run).poll_next(cx)).await,
        Some(SandboxEvent::Started { .. })
    ));
    tokio::time::advance(Duration::from_millis(51)).await;
    let result = match std::future::poll_fn(|cx| Pin::new(&mut run).poll_next(cx)).await {
        Some(SandboxEvent::Completed(result)) => result,
        other => panic!("expected timed-out completion, got {other:?}"),
    };

    assert_eq!(result.outcome, SandboxOutcome::TimedOut);
    assert!(!marker.exists(), "expired auto-release crossed its gate");
}
