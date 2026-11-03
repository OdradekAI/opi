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
        // PowerShell command discovery cannot resolve a bare `cmd`/`ping`
        // without the standard executable-extension list.
        env.insert(
            "PATHEXT".to_owned(),
            ".COM;.EXE;.BAT;.CMD;.VBS;.JS;.WS".to_owned(),
        );
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

/// Backslash-separator cmd.exe path: Win32 `lpApplicationName` does not
/// normalize forward slashes, so a `System32/...` path makes a spawned
/// child die before running any script line (observed on CI and locally).
/// Fixture descendants are cmd.exe batch files rather than PowerShell: a
/// spawned powershell on a Windows Server CI runner can burn tens of seconds
/// of CPU before executing its first script line, while cmd starts in
/// milliseconds and both kinds of child provably inherit and hold the
/// supplier's stdout pipe write-end, which is the behavior under test.
#[cfg(windows)]
fn cmd_exe() -> PathBuf {
    let system = std::env::var_os("SystemRoot").expect("Windows system root");
    let mut path = PathBuf::from(&system);
    path.push("System32");
    path.push("cmd.exe");
    path
}

#[cfg(windows)]
fn cmd_helper(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(format!("{name}.cmd"));
    std::fs::write(&path, body).unwrap();
    path
}

/// Run one PowerShell probe and capture its stdout (the status-based variant
/// above is for pure side effects).
#[cfg(windows)]
fn windows_probe_output(dir: &Path, script: &str) -> Option<String> {
    use std::process::{Command, Stdio};
    let mut process = Command::new(shell_path())
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env_clear()
        .envs(helper_environment(dir))
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let end = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        match process.try_wait() {
            Ok(Some(_)) => {
                let mut out = String::new();
                use std::io::Read;
                process.stdout.take()?.read_to_string(&mut out).ok()?;
                return Some(out);
            }
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

/// Resolve the PIDs of this test's `ping -n <count>` descendant holder. The
/// fixture embeds a per-test ping count so concurrent tests never match each
/// other's holders.
#[cfg(windows)]
fn descendant_pids(dir: &Path, ping_count: u16) -> Vec<u32> {
    let script = format!(
        "Get-CimInstance Win32_Process -Filter \"Name='PING.EXE'\" | Where-Object {{ $_.CommandLine -like '*-n {ping_count}*' }} | ForEach-Object {{ Write-Output $_.ProcessId }}"
    );
    windows_probe_output(dir, &script)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.trim().parse::<u32>().ok())
        .collect()
}

/// Force-kill any straggler holder left by an early test failure so it cannot
/// hold a pipe or survive into later tests.
#[cfg(windows)]
fn cleanup_stragglers(dir: &Path, ping_count: u16) {
    let taskkill = cmd_exe().with_file_name("taskkill.exe");
    for pid in descendant_pids(dir, ping_count) {
        let _ = std::process::Command::new(&taskkill)
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}

/// Drop guard that always cleans this test's stragglers.
#[cfg(windows)]
struct StragglerGuard {
    dir: PathBuf,
    ping_count: u16,
}

#[cfg(windows)]
impl Drop for StragglerGuard {
    fn drop(&mut self) {
        cleanup_stragglers(&self.dir, self.ping_count);
    }
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
        // Synchronous console chain (no .NET spawn and no `start /b`: neither
        // passes the supplier's pipe handles to the child): the parent waits
        // on the batch, so the descendant stays alive exactly until the
        // supplier is aborted and the job kill reaps the whole chain.
        let batch = cmd_helper(
            dir.path(),
            "descendant",
            &format!(
                "@echo off\r\necho started> \"{}\"\r\nping -n 9 127.0.0.1 >nul\r\necho survived> \"{}\"",
                started.display(),
                survived.display()
            ),
        );
        format!(
            "$ErrorActionPreference='Stop'\ntry {{ cmd /c \"{}\" 2> \"{}\" }} catch {{ Set-Content -LiteralPath \"{}\" $_.Exception.Message; throw }}",
            batch.display(),
            dir.path().join("cmderr.txt").display(),
            dir.path().join("pserr.txt").display()
        )
    };
    let command = helper(dir.path(), "parent", &parent, &parent);
    let (owner, transport) = owner(dir.path(), "parent", command, 30_000);
    let mut worker = Some(tokio::spawn(async move { owner.resolve_token().await }));
    // A cold CI runner can spend most of this window in PowerShell startup
    // (parent plus descendant) before the descendant script runs at all.
    tokio::time::timeout(Duration::from_secs(45), async {
        while !started.exists() {
            if let Some(handle) = worker.as_mut().filter(|h| h.is_finished()) {
                let result = handle.await.expect("join helper worker");
                for name in ["cmderr.txt", "pserr.txt", "parent.ps1"] {
                    if let Ok(content) = std::fs::read_to_string(dir.path().join(name)) {
                        eprintln!("{name}:\n{content}");
                    }
                }
                panic!("helper failed before descendant startup: {result:?}");
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("descendant started");
    let worker = worker.take().expect("worker still alive");
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
#[tokio::test]
async fn executable_timeout_reaps_a_descendant_after_its_parent_has_exited() {
    const HOLDER_PINGS: u16 = 91;
    let dir = tempfile::tempdir().unwrap();
    let parent_pid_file = dir.path().join("parent-pid");
    let ready = dir.path().join("child-ready");
    let _stragglers = StragglerGuard {
        dir: dir.path().to_path_buf(),
        ping_count: HOLDER_PINGS,
    };
    // Console chain with one detached holder: every higher-level spawn
    // mechanism on Windows loses the supplier's pipe handles (.NET
    // Process.Start either spins PowerShell engine init on Server runners or
    // drops the write-end; `start /b` hands the child console handles), so
    // the parent script itself P/Invokes CreateProcessW with
    // bInheritHandles and STARTF_USESTDHANDLES — the same native semantics
    // the supplier uses — to launch a long ping that keeps the stdout
    // write-end alive after the parent exits. The WMI probes below find the
    // holder by its per-test ping count.
    let parent_script = r#"Set-Content -LiteralPath '<PARENT_PID>' -Value $PID
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class DetachedHolder {
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    struct STARTUPINFO { public int cb; public string lpReserved, lpDesktop, lpTitle; public int dwX, dwY, dwXSize, dwYSize, dwXCountChars, dwYCountChars, dwFillAttribute, dwFlags; public short wShowWindow, cbReserved2; public IntPtr lpReserved2, hStdInput, hStdOutput, hStdError; }
    [StructLayout(LayoutKind.Sequential)]
    struct PROCESS_INFORMATION { public IntPtr hProcess, hThread; public int dwProcessId, dwThreadId; }
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern bool CreateProcess(string app, string cmd, IntPtr pa, IntPtr ta, bool inherit, uint flags, IntPtr env, string cwd, ref STARTUPINFO si, out PROCESS_INFORMATION pi);
    [DllImport("kernel32.dll")]
    static extern IntPtr GetStdHandle(int which);
    [DllImport("kernel32.dll")]
    static extern bool CloseHandle(IntPtr h);
    public static void Start(string commandLine) {
        var si = new STARTUPINFO();
        si.cb = Marshal.SizeOf(typeof(STARTUPINFO));
        si.dwFlags = 0x100; // STARTF_USESTDHANDLES
        si.hStdInput = GetStdHandle(-10);
        si.hStdOutput = GetStdHandle(-11);
        si.hStdError = GetStdHandle(-12);
        PROCESS_INFORMATION pi;
        if (!CreateProcess(null, commandLine, IntPtr.Zero, IntPtr.Zero, true, 0x08000000u, IntPtr.Zero, null, ref si, out pi)) {
            throw new Exception("CreateProcess failed: " + Marshal.GetLastWin32Error());
        }
        CloseHandle(pi.hProcess);
        CloseHandle(pi.hThread);
    }
}
'@
[DetachedHolder]::Start('ping -n <PINGS> 127.0.0.1')
Set-Content -LiteralPath '<READY>' -Value ok
exit 0
"#
    .replace("<PARENT_PID>", &parent_pid_file.display().to_string())
    .replace("<PINGS>", &HOLDER_PINGS.to_string())
    .replace("<READY>", &ready.display().to_string());
    let parent_script_path = dir.path().join("exiting-parent.ps1");
    std::fs::write(&parent_script_path, parent_script).unwrap();
    let command = format!(
        "\"{}\" -NoProfile -File \"{}\"",
        shell_path().display(),
        parent_script_path.display()
    );
    // The deadline must exceed the parent's single PowerShell cold start on
    // a CI runner; with the old 5s budget the deadline reaped the parent
    // before it wrote any marker file, so readiness could never succeed.
    let (owner, transport) = owner(dir.path(), "exiting-parent", command, 45_000);
    let worker = tokio::spawn(async move { owner.resolve_token().await });
    // Covers the parent PowerShell cold start plus the wrapper's immediate
    // holder launch.
    let readiness = tokio::time::timeout(Duration::from_secs(45), async {
        while !ready.exists() || !parent_pid_file.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    if readiness.is_err() {
        eprintln!(
            "readiness markers: ready={} parent_pid={} worker_finished={} holder_pids={:?}",
            ready.exists(),
            parent_pid_file.exists(),
            worker.is_finished(),
            descendant_pids(dir.path(), HOLDER_PINGS),
        );
        if let Ok(script) = std::fs::read_to_string(dir.path().join("exiting-parent.ps1")) {
            eprintln!("parent script:\n{script}");
        }
    }
    readiness.expect("the synthetic holder must start");
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
    eprintln!(
        "parent_exited=true parent_pid={parent_pid} holder_pids={:?}",
        descendant_pids(dir.path(), HOLDER_PINGS)
    );
    // The 45s supplier deadline must already have elapsed by now. A holder
    // with the stdout write-end keeps the supplier pending until cleanup
    // really kills it.
    tokio::time::sleep(Duration::from_secs(50)).await;
    let child_alive_after_timeout = !descendant_pids(dir.path(), HOLDER_PINGS).is_empty();
    eprintln!("owned_child_alive_after_source_timeout={child_alive_after_timeout}");
    // Cleanup precedes the assertion, including on the expected failing
    // branch; the Drop guard repeats it after early failures.
    cleanup_stragglers(dir.path(), HOLDER_PINGS);
    let result = tokio::time::timeout(Duration::from_secs(30), worker)
        .await
        .expect("supplier settles after the probe has reclaimed its child")
        .unwrap();
    eprintln!("supplier result: {result:?}");
    assert!(matches!(result, Err(AdcResolutionError::ExecutableTimeout)));
    assert_eq!(transport.sends.load(Ordering::SeqCst), 0);
    assert!(
        !child_alive_after_timeout,
        "descendant holder survived timeout after parent PID {parent_pid} exited"
    );
}

#[tokio::test]
async fn cancelling_default_aws_process_without_policy_reaps_the_helper() {
    use opi_ai::auth::aws::process::{
        CredentialProcessRunner, CredentialProcessSpec, SpawnedCredentialProcess,
    };
    let dir = tempfile::tempdir().unwrap();
    let started = dir.path().join("aws-started");
    let survived = dir.path().join("aws-survived");
    let unix = format!(
        "printf started > '{}'; sleep 2; printf survived > '{}'",
        started.display(),
        survived.display()
    );
    let windows = format!(
        "[System.IO.File]::WriteAllText('{}', 'started'); Start-Sleep -Seconds 2; [System.IO.File]::WriteAllText('{}', 'survived')",
        started.display(),
        survived.display()
    );
    let command = helper(dir.path(), "aws-unbounded-cancel", &unix, &windows);
    let command = if cfg!(windows) {
        format!("& {command}")
    } else {
        command
    };
    let spec = CredentialProcessSpec {
        command,
        cwd: Some(dir.path().to_path_buf()),
        env: opi_ai::auth::aws::process::ProcessEnv::Exact(helper_environment(dir.path())),
    };
    let runner = SpawnedCredentialProcess::default();
    assert_eq!(runner.policy(), None);
    let task = tokio::spawn(async move { runner.run(&spec).await });
    tokio::time::timeout(Duration::from_secs(45), async {
        while !started.exists() {
            assert!(
                !task.is_finished(),
                "helper exited before its readiness marker"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("owned AWS helper started");
    task.abort();
    let _ = task.await;
    tokio::time::sleep(Duration::from_millis(2600)).await;
    assert!(
        !survived.exists(),
        "helper survived cancellation of the unbounded SDK runner"
    );
}
