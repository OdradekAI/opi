//! Native auth helper boundaries exercised through the public ADC source.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opi_ai::auth::adc::native::{
    AdcAuthSelection, AdcNativeOwner, AdcResolutionError, AdcSourceHttpError, AdcSourceRequest,
    AdcSourceResponse, AdcSourceTransport, FixedAdcClock,
};
use opi_ai::auth::{AuthContext, AuthContextError, AuthProcessSnapshot};
use opi_ai::credential::BoxAuthFuture;

struct HelperContext {
    cwd: PathBuf,
}

fn helper_environment(cwd: &Path) -> std::collections::BTreeMap<String, String> {
    let mut env = std::collections::BTreeMap::from([
        ("HOME".to_owned(), cwd.to_string_lossy().into_owned()),
        ("TEMP".to_owned(), cwd.to_string_lossy().into_owned()),
        ("TMP".to_owned(), cwd.to_string_lossy().into_owned()),
    ]);
    #[cfg(windows)]
    {
        let system = std::env::var("SystemRoot").expect("Windows system root");
        env.insert("SystemRoot".to_owned(), system.clone());
        env.insert("WINDIR".to_owned(), system.clone());
        env.insert("USERPROFILE".to_owned(), cwd.to_string_lossy().into_owned());
        // A spawned PowerShell on Windows Server burns its whole budget in
        // module-analysis/cache path resolution when these ordinary system
        // variables are missing from an Exact environment; any real Windows
        // credential process inherits them.
        env.insert(
            "LOCALAPPDATA".to_owned(),
            cwd.to_string_lossy().into_owned(),
        );
        env.insert(
            "PSModulePath".to_owned(),
            format!("{system}\\System32\\WindowsPowerShell\\v1.0\\Modules"),
        );
        env.insert("ComSpec".to_owned(), format!("{system}\\System32\\cmd.exe"));
        env.insert(
            "PATH".to_owned(),
            format!("{system}\\System32;{system}\\System32\\WindowsPowerShell\\v1.0"),
        );
    }
    #[cfg(unix)]
    env.insert("PATH".to_owned(), "/usr/bin:/bin".to_owned());
    env
}

impl AuthContext for HelperContext {
    fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
        Ok((name == "GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES").then(|| "1".to_owned()))
    }

    fn file_exists(&self, path: &str) -> Result<bool, AuthContextError> {
        Ok(Path::new(path).exists())
    }

    fn process_snapshot(&self) -> Result<AuthProcessSnapshot, AuthContextError> {
        Ok(AuthProcessSnapshot {
            env: helper_environment(&self.cwd),
            cwd: self.cwd.clone(),
        })
    }
}

#[derive(Default)]
struct TokenTransport {
    sends: AtomicUsize,
    bodies: Mutex<Vec<String>>,
}

impl AdcSourceTransport for TokenTransport {
    fn send(
        &self,
        request: AdcSourceRequest,
    ) -> BoxAuthFuture<'_, Result<AdcSourceResponse, AdcSourceHttpError>> {
        assert_eq!(request.method, "POST");
        assert_eq!(request.url, "https://sts.googleapis.com/v1/token");
        self.sends.fetch_add(1, Ordering::SeqCst);
        self.bodies
            .lock()
            .unwrap()
            .push(request.body.unwrap_or_default());
        Box::pin(async {
            Ok(AdcSourceResponse {
                status: 200,
                headers: Vec::new(),
                body: br#"{"access_token":"synthetic-access","expires_in":3600}"#.to_vec(),
            })
        })
    }
}

fn shell_path() -> PathBuf {
    #[cfg(windows)]
    {
        PathBuf::from(std::env::var_os("SystemRoot").expect("Windows system root"))
            .join("System32/WindowsPowerShell/v1.0/powershell.exe")
    }
    #[cfg(unix)]
    {
        PathBuf::from("/bin/sh")
    }
}

fn helper(dir: &Path, name: &str, unix: &str, windows: &str) -> String {
    #[cfg(windows)]
    let (extension, body, options) = ("ps1", windows, "-NoProfile -File ");
    #[cfg(unix)]
    let (extension, body, options) = ("sh", unix, "");
    let _ = (unix, windows);
    let path = dir.join(format!("{name}.{extension}"));
    std::fs::write(&path, body).unwrap();
    format!(
        "\"{}\" {options}\"{}\"",
        shell_path().display(),
        path.display()
    )
}

/// Backslash-separator PowerShell path for .NET `ProcessStartInfo.FileName`
/// and `Start-Process -FilePath`: Win32 `lpApplicationName` does not normalize
/// forward slashes, so a `System32/...` path makes the spawned child die
/// before running any script line (observed both on CI runners and locally).
/// Fixture descendants must stay PowerShell: they hold the supplier's stdout
/// pipe write-end, which is the behavior under test.
#[cfg(windows)]
fn powershell_exe() -> PathBuf {
    let system = std::env::var_os("SystemRoot").expect("Windows system root");
    let mut path = PathBuf::from(&system);
    path.push("System32");
    path.push("WindowsPowerShell");
    path.push("v1.0");
    path.push("powershell.exe");
    path
}

/// Pay the runner's PowerShell cold-start cost (Defender scan, .NET init)
/// once before a fixture chain relies on bounded spawn latency. Warm hosts
/// return in well under a second; the budget only covers a cold CI runner.
#[cfg(windows)]
fn warm_powershell() {
    let ok = std::process::Command::new(powershell_exe())
        .args(["-NoProfile", "-NonInteractive", "-Command", "exit 0"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    assert!(ok, "PowerShell warm-up spawn failed");
}

fn owner(
    dir: &Path,
    name: &str,
    command: String,
    timeout: u64,
) -> (AdcNativeOwner, Arc<TokenTransport>) {
    let credentials = dir.join(format!("{name}.json"));
    std::fs::write(
        &credentials,
        serde_json::to_vec(&serde_json::json!({
            "type": "external_account",
            "audience": "//iam.googleapis.com/projects/123/locations/global/workloadIdentityPools/p/providers/o",
            "subject_token_type": "urn:ietf:params:oauth:token-type:jwt",
            "credential_source": {"executable": {"command": command, "timeout_millis": timeout}},
        }))
        .unwrap(),
    )
    .unwrap();
    let transport = Arc::new(TokenTransport::default());
    let owner = AdcNativeOwner::new(
        AdcAuthSelection::CredentialsFile(credentials),
        transport.clone(),
        Arc::new(FixedAdcClock::new(
            time::OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
        )),
    )
    .with_ambient(Arc::new(HelperContext {
        cwd: dir.to_path_buf(),
    }));
    (owner, transport)
}

#[tokio::test]
async fn executable_failure_details_never_enter_public_diagnostics() {
    let dir = tempfile::tempdir().unwrap();
    for (name, unix, windows) in [
        (
            "exit",
            "printf '%s' 'EXECUTABLE-OUTPUT-SECRET' >&2\nexit 3",
            "[Console]::Error.Write('EXECUTABLE-OUTPUT-SECRET')\nexit 3",
        ),
        (
            "reported",
            "printf '%s' '{\"version\":1,\"success\":false,\"code\":\"EXECUTABLE-CODE-SECRET\",\"message\":\"EXECUTABLE-MESSAGE-SECRET\"}'",
            "[Console]::Out.Write('{\"version\":1,\"success\":false,\"code\":\"EXECUTABLE-CODE-SECRET\",\"message\":\"EXECUTABLE-MESSAGE-SECRET\"}')",
        ),
    ] {
        let command = helper(dir.path(), name, unix, windows);
        let (owner, transport) = owner(dir.path(), name, command, 30_000);
        let error = owner.resolve_token().await.unwrap_err();
        match &error {
            AdcResolutionError::ExecutableFailed { code, .. } => {
                assert_eq!(
                    code.as_str(),
                    if name == "exit" { "3" } else { "[REDACTED]" },
                    "the intended helper failure must be reached"
                );
            }
            other => panic!("helper failed before the intended failure: {other:?}"),
        }
        for rendered in [
            error.to_string(),
            format!("{error:?}"),
            error.safe_summary(),
        ] {
            assert!(
                !rendered.contains("SECRET"),
                "secret in diagnostic: {rendered}"
            );
        }
        assert_eq!(transport.sends.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn executable_collects_large_stderr_without_blocking_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let response = r#"{"version":1,"success":true,"token_type":"urn:ietf:params:oauth:token-type:jwt","id_token":"synthetic-subject","expiration_time":2000000000}"#;
    let unix = format!("head -c 131072 /dev/zero | tr '\\0' ' ' >&2\nprintf '%s' '{response}' >&2");
    let windows =
        format!("[Console]::Error.Write((' ' * 131072))\n[Console]::Error.Write('{response}')");
    let command = helper(dir.path(), "large-stderr", &unix, &windows);
    let (owner, transport) = owner(dir.path(), "large-stderr", command, 5_000);
    owner
        .resolve_token()
        .await
        .expect("valid merged helper output");
    assert_eq!(transport.sends.load(Ordering::SeqCst), 1);
}

struct FrozenContext {
    cwd: PathBuf,
}

impl AuthContext for FrozenContext {
    fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
        Ok((name == "GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES").then(|| "1".to_owned()))
    }

    fn file_exists(&self, path: &str) -> Result<bool, AuthContextError> {
        Ok(Path::new(path).exists())
    }

    fn process_snapshot(&self) -> Result<AuthProcessSnapshot, AuthContextError> {
        let mut env = helper_environment(&self.cwd);
        env.insert(
            "OPI_AUTH_PROCESS_VALUE".to_owned(),
            "selectedenv-".to_owned(),
        );
        env.insert(
            "google_external_account_audience".to_owned(),
            "unselected-audience".to_owned(),
        );
        Ok(AuthProcessSnapshot {
            env,
            cwd: self.cwd.clone(),
        })
    }
}

#[tokio::test]
async fn executable_subject_consumes_frozen_environment_and_cwd() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path().join("selected working directory");
    std::fs::create_dir(&cwd).unwrap();
    std::fs::write(cwd.join("identity.txt"), "selectedcwd").unwrap();
    let unix = "value=\"${OPI_AUTH_PROCESS_VALUE}$(/bin/cat identity.txt)\"\nprintf '{\"version\":1,\"success\":true,\"token_type\":\"urn:ietf:params:oauth:token-type:jwt\",\"id_token\":\"%s\",\"expiration_time\":2000000000}' \"$value\"";
    let windows = "if ($env:GOOGLE_EXTERNAL_ACCOUNT_AUDIENCE -ne '//iam.googleapis.com/projects/123/locations/global/workloadIdentityPools/p/providers/o') { exit 4 }\n$value = $env:OPI_AUTH_PROCESS_VALUE + [System.IO.File]::ReadAllText('identity.txt')\n[Console]::Out.Write(('{\"version\":1,\"success\":true,\"token_type\":\"urn:ietf:params:oauth:token-type:jwt\",\"id_token\":\"' + $value + '\",\"expiration_time\":2000000000}'))";
    let command = helper(dir.path(), "snapshot", unix, windows);
    let (owner, transport) = owner(dir.path(), "snapshot", command, 30_000);
    owner
        .with_ambient(Arc::new(FrozenContext { cwd }))
        .resolve_token()
        .await
        .expect("helper uses the selected context");
    assert_eq!(transport.sends.load(Ordering::SeqCst), 1);
    assert!(
        transport.bodies.lock().unwrap()[0].contains("subject_token=selectedenv-selectedcwd"),
        "STS must receive the selected helper identity"
    );
}

struct NoProcessContext;

impl AuthContext for NoProcessContext {
    fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
        Ok((name == "GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES").then(|| "1".to_owned()))
    }

    fn file_exists(&self, path: &str) -> Result<bool, AuthContextError> {
        Ok(Path::new(path).exists())
    }

    fn process_snapshot(&self) -> Result<AuthProcessSnapshot, AuthContextError> {
        Err(AuthContextError::ProcessSnapshot {
            reason: "no native helper context selected",
        })
    }
}

#[tokio::test]
async fn executable_fresh_cache_does_not_require_native_process_context() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cached-subject.json");
    std::fs::write(
        &cache,
        serde_json::to_vec(&serde_json::json!({
            "version": 1,
            "success": true,
            "token_type": "urn:ietf:params:oauth:token-type:jwt",
            "id_token": "cached-subject",
            "expiration_time": 2_000_000_000,
        }))
        .unwrap(),
    )
    .unwrap();
    let credentials = dir.path().join("credential.json");
    std::fs::write(&credentials, serde_json::to_vec(&serde_json::json!({
        "type": "external_account",
        "audience": "//iam.googleapis.com/projects/123/locations/global/workloadIdentityPools/p/providers/o",
        "subject_token_type": "urn:ietf:params:oauth:token-type:jwt",
        "credential_source": {"executable": {"command": "never-spawn-this-helper", "output_file": cache}},
    })).unwrap()).unwrap();
    let transport = Arc::new(TokenTransport::default());
    AdcNativeOwner::new(
        AdcAuthSelection::CredentialsFile(credentials),
        transport.clone(),
        Arc::new(FixedAdcClock::new(
            time::OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
        )),
    )
    .with_ambient(Arc::new(NoProcessContext))
    .resolve_token()
    .await
    .expect("fresh cache needs no native helper");
    assert_eq!(transport.sends.load(Ordering::SeqCst), 1);
    assert!(transport.bodies.lock().unwrap()[0].contains("subject_token=cached-subject"));
}

#[tokio::test]
async fn cancelling_executable_supplier_reaps_its_descendants() {
    let dir = tempfile::tempdir().unwrap();
    let started = dir.path().join("started");
    let survived = dir.path().join("survived");
    #[cfg(unix)]
    let parent = {
        let child = helper(
            dir.path(),
            "descendant",
            &format!(
                "printf started > '{}'\nsleep 8\nprintf survived > '{}'",
                started.display(),
                survived.display()
            ),
            "",
        );
        format!("{child} & wait")
    };
    #[cfg(windows)]
    let parent = {
        warm_powershell();
        let child = helper(
            dir.path(),
            "descendant",
            "",
            &format!(
                "Set-Content -LiteralPath '{}' -Value started\nStart-Sleep -Seconds 8\nSet-Content -LiteralPath '{}' -Value survived",
                started.display(),
                survived.display()
            ),
        );
        let argv = opi_ai::auth::adc::executable::parse_command(&child).unwrap();
        format!(
            "$child = Start-Process -FilePath '{}' -WindowStyle Hidden -PassThru -ArgumentList @('-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File','\"{}\"'); Wait-Process -Id $child.Id",
            powershell_exe().display(),
            argv[3]
        )
    };
    let command = helper(dir.path(), "parent", &parent, &parent);
    let (owner, transport) = owner(dir.path(), "parent", command, 30_000);
    let worker = tokio::spawn(async move { owner.resolve_token().await });
    // A cold CI runner can spend most of this window in PowerShell startup
    // (parent plus descendant) before the descendant script runs at all.
    tokio::time::timeout(Duration::from_secs(45), async {
        while !started.exists() {
            assert!(
                !worker.is_finished(),
                "helper failed before descendant startup"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("descendant started");
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    // The descendant lives for 8s after signalling `started`; checking at
    // 8.5s still catches a descendant that was never reaped while tolerating
    // multi-second tree-kill latency on a loaded cold runner.
    tokio::time::sleep(Duration::from_millis(8500)).await;
    assert!(
        !survived.exists(),
        "descendant survived supplier cancellation"
    );
    assert_eq!(transport.sends.load(Ordering::SeqCst), 0);
}

#[cfg(windows)]
fn windows_probe_status(dir: &Path, script: &str) -> Option<std::process::ExitStatus> {
    use std::process::{Command, Stdio};
    let mut process = Command::new(shell_path())
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env_clear()
        .envs(helper_environment(dir))
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .ok()?;
    let end = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        match process.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) if std::time::Instant::now() < end => {
                std::thread::sleep(Duration::from_millis(10));
            }
            _ => {
                let _ = process.kill();
                let _ = process.wait();
                return None;
            }
        }
    }
}

#[cfg(windows)]
struct OwnedWindowsProbe {
    dir: PathBuf,
    pid_file: PathBuf,
}

#[cfg(windows)]
impl OwnedWindowsProbe {
    fn cleanup(&self) {
        let Some((pid, started)) = std::fs::read_to_string(&self.pid_file)
            .ok()
            .and_then(|value| {
                let (pid, started) = value.trim().split_once(',')?;
                Some((pid.parse::<u32>().ok()?, started.parse::<u64>().ok()?))
            })
            .filter(|(pid, _)| *pid > 4)
        else {
            return;
        };
        let taskkill =
            PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32/taskkill.exe");
        // Check only the probe's exact PID and creation time. Holding the
        // process handle prevents PID reuse between the check and termination.
        let script = format!(
            "$ErrorActionPreference = 'Stop'; Write-Output 'cleanup_expected={pid},{started}'; $p = Get-Process -Id {pid} -ErrorAction SilentlyContinue; if ($p) {{ $null = $p.Handle; $actual = $p.StartTime.ToUniversalTime().Ticks; Write-Output \"cleanup_actual={pid},$actual\"; if ($actual -eq {started}) {{ & '{}' /PID {pid} /T /F; Write-Output \"taskkill_exit=$LASTEXITCODE\"; Write-Output \"child_exited=$($p.WaitForExit(2000))\" }} else {{ Write-Output 'cleanup_identity_mismatch'; exit 2 }} }} else {{ Write-Output 'child_already_exited' }}",
            taskkill.display(),
        );
        let status = windows_probe_status(&self.dir, &script);
        eprintln!("cleanup_status={status:?}");
    }
}

#[cfg(windows)]
impl Drop for OwnedWindowsProbe {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[cfg(windows)]
#[tokio::test]
async fn executable_timeout_reaps_a_descendant_after_its_parent_has_exited() {
    let dir = tempfile::tempdir().unwrap();
    let child_pid_file = dir.path().join("child-pid");
    let parent_pid_file = dir.path().join("parent-pid");
    let ready = dir.path().join("child-ready");
    warm_powershell();
    // The descendant is PowerShell spawned through .NET ProcessStartInfo: it
    // inherits and holds the supplier's stdout pipe while sleeping, which is
    // exactly the condition the timeout-reap contract covers. The .NET
    // FileName must use backslash separators (see powershell_exe). A spawned
    // child powershell on a cold CI runner can spend tens of seconds in
    // startup, hence the warm-up and the wide windows below.
    let child = helper(
        dir.path(),
        "orphan-child",
        "",
        &format!(
            "Set-Content -LiteralPath '{}' -Value $PID\nWrite-Output 'held-by-child'\nStart-Sleep -Seconds 90",
            ready.display(),
        ),
    );
    let argv = opi_ai::auth::adc::executable::parse_command(&child).unwrap();
    let child_script = PathBuf::from(&argv[3]);
    let guard = OwnedWindowsProbe {
        dir: dir.path().to_path_buf(),
        pid_file: child_pid_file.clone(),
    };
    let command = helper(
        dir.path(),
        "exiting-parent",
        "",
        &format!(
            "Set-Content -LiteralPath '{}' -Value $PID\n$i = New-Object System.Diagnostics.ProcessStartInfo\n$i.FileName = '{}'\n$i.Arguments = '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File \"{}\"'\n$i.UseShellExecute = $false\n$i.CreateNoWindow = $true\n$p = [System.Diagnostics.Process]::Start($i)\nSet-Content -LiteralPath '{}' -Value \"$($p.Id),$($p.StartTime.ToUniversalTime().Ticks)\"\nexit 0",
            parent_pid_file.display(),
            powershell_exe().display(),
            child_script.display(),
            child_pid_file.display(),
        ),
    );
    // The deadline must exceed the parent's single PowerShell cold start on
    // a CI runner; with the old 5s budget the deadline reaped the parent
    // before it wrote any marker file, so readiness could never succeed.
    let (owner, transport) = owner(dir.path(), "exiting-parent", command, 45_000);
    let worker = tokio::spawn(async move { owner.resolve_token().await });
    // Covers the parent PowerShell cold start plus the batch child's
    // immediate marker writes.
    let readiness = tokio::time::timeout(Duration::from_secs(45), async {
        while !ready.exists() || !child_pid_file.exists() || !parent_pid_file.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    if readiness.is_err() {
        eprintln!(
            "readiness markers: ready={} child_pid={} parent_pid={} worker_finished={}",
            ready.exists(),
            child_pid_file.exists(),
            parent_pid_file.exists(),
            worker.is_finished()
        );
        if let Ok(content) = std::fs::read_to_string(&child_pid_file) {
            let pid = content.trim().split_once(',').map(|x| x.0).unwrap_or("?");
            eprintln!("child pid file: {content}");
            let probe = format!(
                "$p = Get-Process -Id {pid} -ErrorAction SilentlyContinue; if ($p) {{ Write-Output \"child_alive=$($p.Id) cpu=$($p.CPU) threads=$($p.Threads.Count)\"; $wmi = Get-CimInstance Win32_Process -Filter \"ProcessId={pid}\"; Write-Output \"cmdline=$($wmi.CommandLine)\" }} else {{ Write-Output 'child_not_found' }}"
            );
            let _ = windows_probe_status(dir.path(), &probe);
        }
        if let Ok(script) = std::fs::read_to_string(dir.path().join("exiting-parent.ps1")) {
            eprintln!("parent script:\n{script}");
        }
        if let Ok(batch) = std::fs::read_to_string(dir.path().join("orphan-child.ps1")) {
            eprintln!("child script:\n{batch}");
        }
    }
    readiness.expect("the synthetic child must start");
    let child_pid = std::fs::read_to_string(&child_pid_file)
        .unwrap()
        .trim()
        .split_once(',')
        .unwrap()
        .0
        .parse::<u32>()
        .unwrap();
    let parent_pid = std::fs::read_to_string(&parent_pid_file)
        .unwrap()
        .trim()
        .parse::<u32>()
        .unwrap();
    let parent_exited = windows_probe_status(
        dir.path(),
        &format!("$p = Get-Process -Id {parent_pid} -ErrorAction SilentlyContinue; if ($p -and !$p.HasExited) {{ exit 1 }} else {{ exit 0 }}"),
    )
    .expect("inspect only the probe parent PID")
    .success();
    assert!(parent_exited, "fixture parent has not exited");
    eprintln!("parent_exited=true parent_pid={parent_pid} child_pid={child_pid}");
    // The 45s supplier deadline must already have elapsed by now. A child
    // holding stdout keeps the supplier pending until cleanup really kills it.
    tokio::time::sleep(Duration::from_secs(50)).await;
    let child_alive_after_timeout = windows_probe_status(
        dir.path(),
        &format!("$p = Get-Process -Id {child_pid} -ErrorAction SilentlyContinue; if ($p) {{ Write-Output \"owned_child_has_exited=$($p.HasExited)\" }}; if ($p -and !$p.HasExited) {{ exit 0 }} else {{ exit 1 }}"),
    )
    .expect("inspect only the owned child PID")
    .success();
    eprintln!("owned_child_alive_after_source_timeout={child_alive_after_timeout}");
    // Cleanup precedes the assertion, including on the expected failing
    // branch. Drop repeats this exact-owned-PID cleanup after early failures.
    guard.cleanup();
    let result = tokio::time::timeout(Duration::from_secs(30), worker)
        .await
        .expect("supplier settles after the probe has reclaimed its child")
        .unwrap();
    assert!(matches!(result, Err(AdcResolutionError::ExecutableTimeout)));
    assert_eq!(transport.sends.load(Ordering::SeqCst), 0);
    assert!(
        !child_alive_after_timeout,
        "owned descendant PID {child_pid} survived timeout after parent PID {parent_pid} exited"
    );
}
