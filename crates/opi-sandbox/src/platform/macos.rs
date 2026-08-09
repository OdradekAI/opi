//! macOS native restriction leaf (Phase 16 task 16.14.1).
//!
//! Safe confinement that ports the audited Phase 15 sandbox-exec/Seatbelt
//! behavior onto the opi-sandbox [`Restriction`](crate::policy::Restriction)
//! seam. This module is under `platform/mod.rs`'s `#![forbid(unsafe_code)]`
//! (which propagates and cannot be overridden). macOS confinement is a
//! LAUNCHER, not a `pre_exec` hook: `sandbox-exec` must be the parent process
//! that applies the rendered profile to its child (the target). There is
//! therefore NO `unsafe` and NO `process_tree` FFI entry here (unlike the Linux
//! leaf, whose Landlock/seccomp FFI lives in `crate::process_tree`).
//!
//! opi-sandbox is FAIL-CLOSED (no `require` flag): [`MacosRestriction`] is
//! constructed only inside [`posture`] after [`probe_sandbox_exec`] returns
//! [`SandboxExecStatus::Available`]; a missing or rejected `sandbox-exec`
//! reports `supported == false`, so the CLI `run` and the backend `--stdio`
//! gate refuse BEFORE target start (exit 125 / `failed{Unavailable, Handshake}`).
//!
//! # Structure (why this is NOT file-level cfg-gated)
//!
//! The pure profile model (`render_profile`, `escape_path`,
//! `canonicalize_for_profile`, [`SandboxExecStatus`], [`macos_posture_fields`])
//! is host-independent and is gated to `cfg(any(target_os = "macos", test))` so
//! it compiles + is exercised by the inline `#[cfg(test)]` invariants on EVERY
//! host (the Windows-verification half of the DoD), while the runtime items
//! (`probe_sandbox_exec`, [`MacosRestriction`], [`posture`]) are individually
//! `cfg(target_os = "macos")`. A file-level `#![cfg(target_os = "macos")]`
//! (mirroring `linux.rs`) would compile the pure model out on Windows and
//! silently report `0 tests` — a false green on the host-verifiable half. This
//! mirrors the audited 15.5.4 substrate (`crates/opi-coding-agent/src/sandbox/
//! macos.rs`), which compiles its pure model on every target.
//!
//! # Profile shape (deny-overlay on a seatbelt allow-default base)
//!
//! ```text
//! (version 1)
//! (allow default)                              ; seatbelt default is DENY; allow-all base
//! (deny file-write* (subpath "/"))             ; fs engaged (deny first; last-match-wins)
//! (allow file-write* (subpath "<workspace>"))  ; fs engaged (exceptions AFTER the deny)
//! (allow file-write* (subpath "<temp>"))       ; fs engaged
//! (deny network*)                               ; network engaged (deny) only
//! ```
//!
//! `(deny network*)` blocks bind/connect/inbound/outbound but NOT `socket()`
//! creation itself, so the macOS network sentinel exercises `bind` (not bare
//! `socket()`), unlike the Linux seccomp twin which gates `socket()` at `arg[0]`.

#![forbid(unsafe_code)]

// ---------------------------------------------------------------------------
// Pure profile model — host-independent. Gated to macOS (consumed by the
// runtime) and to `cfg(test)` (consumed by the inline invariants below), so it
// never triggers dead_code under a non-macOS `cargo build` while still compiling
// + testing on every host.
// ---------------------------------------------------------------------------

#[cfg(any(target_os = "macos", test))]
use std::path::{Path, PathBuf};

#[cfg(any(target_os = "macos", test))]
use crate::policy::Mechanism;

/// Escape a path for embedding in a seatbelt `(subpath "...")` string literal.
///
/// Backslash, double-quote, and dollar are backslash-escaped. The dollar escape
/// is load-bearing: seatbelt expands `${var}` inside profile strings, so an
/// unescaped `$` in a workspace path would let a crafted path inject or expand
/// a variable. Every special char is prefixed with `\`; control characters are
/// rejected because they cannot be embedded safely in the profile source.
#[cfg(any(target_os = "macos", test))]
fn escape_path(path: &str) -> Result<String, ProfilePathError> {
    let mut out = String::with_capacity(path.len());
    for ch in path.chars() {
        if ch.is_control() {
            return Err(ProfilePathError);
        }
        match ch {
            '\\' | '"' | '$' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    Ok(out)
}

#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProfilePathError;

#[cfg(any(target_os = "macos", test))]
fn profile_path(path: &Path) -> Result<String, ProfilePathError> {
    let path = path.to_str().ok_or(ProfilePathError)?;
    escape_path(path)
}

/// Render the macOS seatbelt deny-overlay profile string.
///
/// Deterministic and escaped. The profile is a deny-overlay on a seatbelt
/// allow-default base: `(allow default)` is the fallback for every operation no
/// explicit rule matches, so the confined child can still exec its shell and
/// read system libraries. `fs_enabled` denies every file-write under `/` with
/// workspace + temp exceptions; `network_enabled` denies `network*` (bind /
/// connect / inbound / outbound — `socket()` creation itself is NOT a
/// `network*` operation).
///
/// **Seatbelt is last-match-wins:** when several explicit rules match an
/// operation, the LAST one decides it. The workspace/temp allow exceptions
/// therefore MUST follow the root write deny. Order: `(allow default)` base,
/// root write deny, workspace/temp write exceptions, network deny. Pure:
/// returns an error for an unrepresentable native path and never invokes
/// `sandbox-exec`.
#[cfg(any(target_os = "macos", test))]
fn render_profile(
    workspace: &Path,
    temp_dir: &Path,
    fs_enabled: bool,
    network_enabled: bool,
) -> Result<String, ProfilePathError> {
    let mut out = String::from("(version 1)\n");
    // seatbelt's default decision is DENY; an explicit allow-default base is
    // load-bearing — without it the confined target cannot exec or read system
    // files, and even the sandbox-exec probe's own helper exec is rejected.
    out.push_str("(allow default)\n");
    if fs_enabled {
        // Last-match-wins: emit the root write deny FIRST, then the workspace/
        // temp allow exceptions so the exceptions (emitted later) win and punch
        // back through; writes outside both remain denied by the root rule.
        out.push_str("(deny file-write* (subpath \"/\"))\n");
        out.push_str(&format!(
            "(allow file-write* (subpath \"{}\"))\n",
            profile_path(workspace)?
        ));
        out.push_str(&format!(
            "(allow file-write* (subpath \"{}\"))\n",
            profile_path(temp_dir)?
        ));
    }
    if network_enabled {
        out.push_str("(deny network*)\n");
    }
    Ok(out)
}

/// Canonicalize a path for a seatbelt subpath rule (resolve symlinks like
/// `/var` -> `/private/var`). Falls back to the verbatim path if the target
/// does not exist (`canonicalize` requires existence). Either result is
/// rejected unless it is exact UTF-8 with no profile control characters.
#[cfg(any(target_os = "macos", test))]
fn canonicalize_for_profile(p: &Path) -> Result<PathBuf, ProfilePathError> {
    match std::fs::canonicalize(p) {
        Ok(c) => {
            profile_path(&c)?;
            Ok(c)
        }
        Err(_) => {
            profile_path(p)?;
            Ok(p.to_path_buf())
        }
    }
}

/// Status of the `sandbox-exec` helper as discovered by the runtime probe.
#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SandboxExecStatus {
    /// The canonical `/usr/bin/sandbox-exec` answered the probe; confinement
    /// can be installed.
    Available(std::path::PathBuf),
    /// `sandbox-exec` was not found.
    Missing,
    /// `sandbox-exec` is present but the probe rejected it (non-zero exit /
    /// signal / spawn failure). The detail is never exposed in diagnostics.
    Unusable(String),
}

#[cfg(any(target_os = "macos", test))]
impl SandboxExecStatus {
    /// Whether the helper is usable (confinement can be installed).
    pub(crate) fn is_available(&self) -> bool {
        matches!(self, Self::Available(_))
    }
}

/// Honest macOS confinement caveats reported by `doctor` on a SUPPORTED host.
/// Mirrors `linux.rs`'s explicit per-caveat enumeration. The DoD requires
/// "legacy/experimental limitations" to be reported truthfully (Apple
/// soft-deprecated `sandbox-exec`).
#[cfg(any(target_os = "macos", test))]
fn supported_limitations() -> Vec<String> {
    vec![
        "sandbox-exec (Seatbelt) is soft-deprecated by Apple (since macOS Sierra 2016) but remains functional; this is a legacy/experimental confinement surface".to_string(),
        "L3 syscall confinement is unavailable on macOS; sandbox-exec denies filesystem writes and network operations only (no syscall filter)".to_string(),
        "host reads remain unrestricted; this is not a read or environment confidentiality boundary".to_string(),
        "(deny network*) blocks bind/connect/inbound/outbound but not socket() creation itself".to_string(),
    ]
}

/// The limitation reported when the helper is missing or rejected (unsupported
/// posture): an honest, non-permanent gap (the helper exists on stock macOS).
#[cfg(any(target_os = "macos", test))]
fn unsupported_limitation(status: &SandboxExecStatus) -> String {
    match status {
        SandboxExecStatus::Missing => {
            "canonical /usr/bin/sandbox-exec is missing; the requested restriction cannot be established, so the target is refused before start".to_string()
        }
        SandboxExecStatus::Unusable(_) => {
            "canonical /usr/bin/sandbox-exec did not pass the runtime probe; the requested restriction cannot be established, so the target is refused before start".to_string()
        }
        SandboxExecStatus::Available(_) => String::new(),
    }
}

/// The pure, host-independent fields derivable from a `sandbox-exec` probe
/// status. Exposing this as a pure function of [`SandboxExecStatus`] makes the
/// missing/rejected-helper -> unsupported invariant unit-testable on ANY host
/// (the DoD's "fail before target start when sandbox-exec is missing or
/// rejected" clause), without constructing the cfg-gated runtime restriction.
#[cfg(any(target_os = "macos", test))]
pub(crate) struct MacosPostureFields {
    /// Whether confinement can be established.
    pub supported: bool,
    /// The mechanisms a supported posture installs.
    pub mechanisms: Vec<Mechanism>,
    /// Honest per-platform caveats.
    pub limitations: Vec<String>,
}

/// Compute the macOS posture fields from a probe status. Pure: never invokes
/// `sandbox-exec`. `Available` -> supported + `[Seatbelt]` + the honest
/// supported caveats; `Missing`/`Unusable` -> unsupported + the honest
/// missing/rejected caveat.
#[cfg(any(target_os = "macos", test))]
pub(crate) fn macos_posture_fields(status: &SandboxExecStatus) -> MacosPostureFields {
    if status.is_available() {
        MacosPostureFields {
            supported: true,
            mechanisms: vec![Mechanism::Seatbelt],
            limitations: supported_limitations(),
        }
    } else {
        MacosPostureFields {
            supported: false,
            mechanisms: Vec::new(),
            limitations: vec![unsupported_limitation(status)],
        }
    }
}

// ---------------------------------------------------------------------------
// Production runtime (cfg(target_os = "macos")).
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
use std::sync::Arc;
#[cfg(target_os = "macos")]
use tokio::process::Command;

#[cfg(target_os = "macos")]
use crate::policy::{
    AppliedRestriction, ContractStatus, LauncherSpec, NetworkPolicy, Restriction, RestrictionCtx,
    RestrictionSetupError,
};

#[cfg(target_os = "macos")]
const SANDBOX_EXEC_PATH: &str = "/usr/bin/sandbox-exec";

/// Probe the canonical `sandbox-exec`, then launch `/usr/bin/true` under a
/// no-op profile. A usable helper exits 0; an MDM-blocked or broken install
/// exits non-zero or fails to spawn. Diagnostics use only a static failure
/// class; raw stderr and `io::Error` display are deliberately discarded.
#[cfg(target_os = "macos")]
fn probe_sandbox_exec() -> SandboxExecStatus {
    let bin = PathBuf::from(SANDBOX_EXEC_PATH);
    if !bin.is_file() {
        return SandboxExecStatus::Missing;
    }
    let profile = "(version 1)\n(allow default)\n";
    match std::process::Command::new(&bin)
        .arg("-p")
        .arg(profile)
        .arg("/usr/bin/true")
        .output()
    {
        Ok(o) if o.status.success() => SandboxExecStatus::Available(bin),
        Ok(o) if o.status.code().is_some() => {
            SandboxExecStatus::Unusable("probe returned non-zero status".to_string())
        }
        Ok(_) => SandboxExecStatus::Unusable("probe terminated by signal".to_string()),
        Err(_) => SandboxExecStatus::Unusable("probe could not start".to_string()),
    }
}

/// The macOS native [`Restriction`]: the `sandbox-exec`/Seatbelt deny-overlay,
/// installed via a [`Restriction::launcher`] parent program. Caches the
/// construction-time probe; constructed ONLY inside [`posture`] after an
/// [`SandboxExecStatus::Available`] probe (mirrors `LinuxRestriction`'s
/// `pub(crate)` + posture-only construction). [`Restriction::launcher`]
/// rejects paths Seatbelt cannot represent exactly, and
/// [`Restriction::prepare`] reports [`Mechanism::Seatbelt`] /
/// [`ContractStatus::Restricted`]
/// — the two agree by construction (an `Available` probe => the launcher wraps
/// the target => the started frame honestly reports `restricted`).
#[cfg(target_os = "macos")]
pub(crate) struct MacosRestriction {
    status: SandboxExecStatus,
}

#[cfg(target_os = "macos")]
impl MacosRestriction {
    /// Build the restriction from a probe status. The caller (`posture`) passes
    /// an `Available` status; the cached status drives fail-closed
    /// defense-in-depth in [`Restriction::launcher`] / [`Restriction::prepare`].
    fn new(status: SandboxExecStatus) -> Self {
        Self { status }
    }
}

#[cfg(target_os = "macos")]
impl Restriction for MacosRestriction {
    fn launcher(
        &self,
        ctx: &RestrictionCtx<'_>,
    ) -> Result<Option<LauncherSpec>, RestrictionSetupError> {
        if ctx.setup_cancelled() {
            return Err(RestrictionSetupError::Failed("setup-cancelled"));
        }
        // Defense-in-depth: posture() constructs us only when Available, but
        // fail closed (no launcher => no confinement) if the cached probe was
        // not Available, so an intra-crate misuse cannot silently run the
        // target unrestricted while prepare() would report Restricted.
        let SandboxExecStatus::Available(_) = &self.status else {
            return Err(RestrictionSetupError::Failed(
                "seatbelt-sandbox-exec-unavailable",
            ));
        };
        // Canonicalize: seatbelt resolves symlinks in the path the child opens,
        // so on macOS (TMPDIR is under /var -> /private/var) it evaluates the
        // /private/var/... form. The subpath exceptions must match that
        // resolved form. The temp exception is the exact invocation-owned root,
        // never the sibling system-temporary directory.
        let ws = canonicalize_for_profile(ctx.workspace)
            .map_err(|_| RestrictionSetupError::Failed("seatbelt-path-unrepresentable"))?;
        let tmp = canonicalize_for_profile(ctx.temp_root)
            .map_err(|_| RestrictionSetupError::Failed("seatbelt-path-unrepresentable"))?;
        // The WorkspaceWrite profile always engages the fs deny-overlay; the
        // network deny engages iff the request denies network.
        let network_enabled = matches!(ctx.network, NetworkPolicy::Deny);
        let profile = render_profile(&ws, &tmp, true, network_enabled)
            .map_err(|_| RestrictionSetupError::Failed("seatbelt-path-unrepresentable"))?;
        if ctx.setup_cancelled() {
            return Err(RestrictionSetupError::Failed("setup-cancelled"));
        }
        Ok(Some(LauncherSpec {
            program: PathBuf::from(SANDBOX_EXEC_PATH),
            prefix: vec!["-p".to_string(), profile],
        }))
    }

    fn prepare(
        &self,
        _cmd: &mut Command,
        ctx: &RestrictionCtx<'_>,
    ) -> Result<AppliedRestriction, RestrictionSetupError> {
        // The launcher already installed the confinement; prepare is a
        // no-op-on-cmd that only reports the effective mechanism/contract.
        // Fail-closed defense-in-depth: never report Restricted without an
        // Available probe (which would mean no launcher was applied).
        if ctx.setup_cancelled() {
            return Err(RestrictionSetupError::Failed("setup-cancelled"));
        }
        if !self.status.is_available() {
            return Err(RestrictionSetupError::Failed(
                "seatbelt-sandbox-exec-unavailable",
            ));
        }
        Ok(AppliedRestriction {
            mechanism: Mechanism::Seatbelt,
            contract: ContractStatus::Restricted,
        })
    }
}

/// The macOS posture: `Supported` (Seatbelt) when `sandbox-exec` probed
/// usable; otherwise `Unsupported` with an honest limitation. Reports the
/// `Seatbelt` mechanism on a supported host and carries the honest
/// legacy/experimental caveats for `doctor`. Mirrors `linux::posture`.
#[cfg(target_os = "macos")]
pub(crate) fn posture() -> super::Posture {
    posture_from_status(probe_sandbox_exec())
}

#[cfg(target_os = "macos")]
fn posture_from_status(status: SandboxExecStatus) -> super::Posture {
    let fields = macos_posture_fields(&status);
    let restriction = if fields.supported {
        Some(Arc::new(MacosRestriction::new(status)) as Arc<dyn Restriction>)
    } else {
        None
    };
    super::Posture {
        supported: fields.supported,
        mechanisms: fields.mechanisms,
        limitations: fields.limitations,
        restriction,
    }
}

// ---------------------------------------------------------------------------
// Host-independent pure-model invariants (run on EVERY host via
// `cargo test -p opi-sandbox --lib`). These assert string/structure facts about
// the rendered profile and the posture fields — NOT kernel enforcement (that is
// the cfg(target_os = "macos") behavioral half verified on a macOS runner).
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Escaping is the security-critical invariant: every special char is
    /// backslash-doubled, so a crafted path cannot inject seatbelt syntax or
    /// expand a `${var}` (the leading `\` neutralizes the `$`).
    #[test]
    fn escape_path_neutralizes_seatbelt_metacharacters() {
        assert_eq!(escape_path("/clean/path").unwrap(), "/clean/path");
        assert_eq!(escape_path("a\"b").unwrap(), "a\\\"b");
        assert_eq!(escape_path("a$b").unwrap(), "a\\$b");
        assert_eq!(escape_path("a\\b").unwrap(), "a\\\\b");
        // ${var}: the dollar is backslash-escaped, so seatbelt treats it as a
        // literal `$` and does not expand the variable.
        assert_eq!(escape_path("${HOME}").unwrap(), "\\${HOME}");
    }

    /// `canonicalize_for_profile` falls back to the verbatim path when the
    /// target does not exist (`canonicalize` requires existence). Verified on a
    /// clearly-absent path so the assertion is host-independent.
    #[test]
    fn canonicalize_falls_back_to_verbatim_for_missing_path() {
        let missing = Path::new("opi-sandbox-missing-path-xyz-12345");
        assert!(
            !missing.exists(),
            "precondition: the chosen path must not exist on the host"
        );
        let rendered = canonicalize_for_profile(missing);
        assert_eq!(rendered.unwrap(), missing);
    }

    #[test]
    fn profile_paths_with_control_characters_are_refused() {
        let rendered = render_profile(Path::new("/ws\n(injected)"), Path::new("/tmp"), true, false);

        assert!(
            rendered.is_err(),
            "Seatbelt paths containing control characters must be refused"
        );
    }

    #[cfg(unix)]
    #[test]
    fn profile_paths_with_invalid_native_bytes_are_refused_without_replacement() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let invalid =
            std::path::PathBuf::from(OsString::from_vec(vec![b'/', b'w', b's', b'/', 0xff]));

        assert!(
            canonicalize_for_profile(&invalid).is_err(),
            "an unrepresentable native path must be refused rather than changed"
        );
    }

    /// The profile carries the version header + the load-bearing allow-default
    /// base even when no layer engages (without it seatbelt's default is DENY
    /// and the target cannot exec or read system files).
    #[test]
    fn render_profile_always_emits_version_and_allow_default_base() {
        let p = render_profile(Path::new("/ws"), Path::new("/tmp"), false, false).unwrap();
        assert!(
            p.starts_with("(version 1)\n"),
            "version header first: {p:?}"
        );
        assert!(
            p.contains("(allow default)\n"),
            "allow-default base always present: {p:?}"
        );
        assert!(
            !p.contains("(deny file-write*"),
            "fs disabled -> no fs deny: {p:?}"
        );
        assert!(
            !p.contains("(deny network*)"),
            "network disabled -> no network deny: {p:?}"
        );
    }

    /// Last-match-wins ordering: the root write deny MUST precede the
    /// workspace/temp allow exceptions, or the deny (emitted later) would
    /// override them and reject workspace writes too.
    #[test]
    fn render_profile_orders_deny_before_allow_exceptions() {
        let p =
            render_profile(Path::new("/ws"), Path::new("/private/var/tmp"), true, false).unwrap();
        let deny_idx = p
            .find("(deny file-write* (subpath \"/\"))")
            .expect("root write deny present");
        let ws_idx = p
            .find("(allow file-write* (subpath \"/ws\"))")
            .expect("ws allow");
        let tmp_idx = p
            .find("(allow file-write* (subpath \"/private/var/tmp\"))")
            .expect("temp allow");
        assert!(deny_idx < ws_idx, "deny must precede workspace allow");
        assert!(deny_idx < tmp_idx, "deny must precede temp allow");
    }

    /// The network deny is emitted iff engaged; the fs deny is independent.
    #[test]
    fn render_profile_independent_fs_and_network_toggles() {
        let both = render_profile(Path::new("/ws"), Path::new("/tmp"), true, true).unwrap();
        assert!(both.contains("(deny file-write* (subpath \"/\"))"));
        assert!(both.contains("(deny network*)"));
        let fs_only = render_profile(Path::new("/ws"), Path::new("/tmp"), true, false).unwrap();
        assert!(fs_only.contains("(deny file-write*"));
        assert!(!fs_only.contains("(deny network*)"));
        let net_only = render_profile(Path::new("/ws"), Path::new("/tmp"), false, true).unwrap();
        assert!(!net_only.contains("(deny file-write*"));
        assert!(net_only.contains("(deny network*)"));
    }

    /// Special characters in a workspace path are escaped in the rendered
    /// profile: the raw path never appears verbatim, the dollar is backslash-
    /// escaped (no bare `$` survives to expand a seatbelt variable), and the
    /// quote/backslash are escaped too.
    #[test]
    fn render_profile_escapes_paths_in_exceptions() {
        // Input carrying all three specials: $ " and \.
        let patho = "a$b\"c\\d";
        let p = render_profile(Path::new(patho), Path::new("/tmp"), true, false).unwrap();
        // The raw path never appears verbatim inside a subpath rule.
        assert!(
            !p.contains(&format!("(subpath \"{patho}\"")),
            "raw path must not appear verbatim: {p:?}"
        );
        // The dollar is escaped (a\$b in the output).
        assert!(p.contains("a\\$b"), "dollar must be escaped: {p:?}");
        // The quote and backslash are escaped too.
        assert!(p.contains("\\\""), "quote must be escaped: {p:?}");
        assert!(p.contains("\\\\"), "backslash must be escaped: {p:?}");
    }

    /// An Available probe -> supported with the Seatbelt mechanism and the
    /// honest supported caveats (incl. the soft-deprecation honesty the DoD
    /// requires).
    #[test]
    fn available_probe_is_supported_with_seatbelt_and_deprecation_honesty() {
        let f = macos_posture_fields(&SandboxExecStatus::Available(std::path::PathBuf::from(
            "/usr/bin/sandbox-exec",
        )));
        assert!(f.supported);
        assert_eq!(f.mechanisms, vec![Mechanism::Seatbelt]);
        assert!(
            f.limitations.iter().any(|l| l.contains("soft-deprecated")),
            "limitations must report the legacy/experimental deprecation: {:?}",
            f.limitations
        );
        assert!(
            f.limitations.iter().any(|l| l.contains("L3 syscall")),
            "limitations must report L3-unavailable"
        );
        assert!(
            f.limitations.iter().any(|l| l.contains("host reads")),
            "limitations must report host-reads-unrestricted"
        );
    }

    /// A Missing or Unusable probe -> unsupported with NO mechanism and an
    /// honest limitation (the DoD's "fail before target start when sandbox-exec
    /// is missing or rejected" clause, verified host-independently).
    #[test]
    fn missing_or_unusable_probe_is_unsupported() {
        for status in [
            SandboxExecStatus::Missing,
            SandboxExecStatus::Unusable("probe returned non-zero status".to_string()),
        ] {
            let f = macos_posture_fields(&status);
            assert!(!f.supported, "unsupported for {status:?}");
            assert!(f.mechanisms.is_empty(), "no mechanism when unsupported");
            assert_eq!(f.limitations.len(), 1, "one honest limitation");
            assert!(!f.limitations[0].is_empty(), "limitation must be populated");
        }
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn missing_and_unusable_helpers_refuse_cli_and_backend_before_target_start() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStrExt as _;

        use opi_protocol::execution::v1::frames::{ExecutePayload, InitializePayload};
        use opi_protocol::execution::v1::{
            BackendToHost, Bounds, EnvInherit, FailureCode, FailurePhase, HostToBackend,
            NativeString, ProtocolId, RequestId, WIRE_IDENTITY, encode_line,
        };
        use tokio::io::AsyncWriteExt as _;

        use crate::policy::{NetworkPolicy, NoRestriction, Profile};

        for status in [
            SandboxExecStatus::Missing,
            SandboxExecStatus::Unusable("injected probe rejection".to_string()),
        ] {
            let workspace = tempfile::tempdir().expect("workspace");
            let marker = workspace.path().join("must-not-start");
            let target = format!("printf started > '{}'", marker.display());
            let command = crate::cli::RunCommand {
                workspace: workspace.path().to_path_buf(),
                profile: Profile::WorkspaceWrite,
                network: NetworkPolicy::Deny,
                program: PathBuf::from("/bin/sh"),
                args: vec![OsString::from("-c"), OsString::from(&target)],
            };

            let cli_code =
                crate::cli::execute_run_command(command, posture_from_status(status.clone())).await;
            assert_eq!(cli_code, 125, "CLI must refuse {status:?}");
            assert!(!marker.exists(), "CLI refusal must not start the target");

            let request_id =
                RequestId::new("macos-helper-refusal".to_string()).expect("request id");
            let initialize = HostToBackend::Initialize(InitializePayload {
                request_id: request_id.clone(),
                deadline_ms: 10_000,
                adapter_config: serde_json::json!({}),
                supported_protocols: vec![
                    ProtocolId::new(WIRE_IDENTITY).expect("protocol identity"),
                ],
            });
            let execute = HostToBackend::Execute(ExecutePayload {
                request_id,
                program: NativeString::from_bytes(b"/bin/sh".to_vec()),
                args: vec![
                    NativeString::from_bytes(b"-c".to_vec()),
                    NativeString::from_bytes(target.as_bytes().to_vec()),
                ],
                workspace: NativeString::from_bytes(workspace.path().as_os_str().as_bytes()),
                cwd: NativeString::from_bytes(workspace.path().as_os_str().as_bytes()),
                timeout_ms: 5_000,
                env_inherit: EnvInherit::Inherit,
                env_additions: Default::default(),
            });
            let input = format!(
                "{}\n{}\n",
                encode_line(&initialize, &Bounds::DEFAULT).expect("initialize frame"),
                encode_line(&execute, &Bounds::DEFAULT).expect("execute frame"),
            )
            .into_bytes();
            let (mut host, reader) = tokio::io::duplex(input.len());
            host.write_all(&input).await.expect("write backend input");
            drop(host);
            let posture = posture_from_status(status.clone());
            let mut output = Vec::new();
            let code = crate::backend::drive(
                Box::pin(reader),
                &mut output,
                Bounds::DEFAULT,
                posture.supported,
                &posture.limitations,
                posture
                    .restriction
                    .unwrap_or_else(|| Arc::new(NoRestriction)),
            )
            .await;
            assert_eq!(code, 0, "backend emits a terminal refusal");
            let frames = output
                .split(|byte| *byte == b'\n')
                .filter(|line| !line.is_empty())
                .map(|line| {
                    opi_protocol::execution::v1::codec::decode_backend(line)
                        .expect("valid backend frame")
                })
                .collect::<Vec<_>>();
            let BackendToHost::Failed(failed) = frames.last().expect("terminal frame") else {
                panic!("expected terminal failed frame, got {frames:?}");
            };
            assert_eq!(failed.code, FailureCode::Unavailable);
            assert_eq!(failed.phase, FailurePhase::Handshake);
            assert!(
                !marker.exists(),
                "backend refusal must not start the target"
            );
        }
    }

    /// Native CI-only proof: the acknowledgement file is created by the
    /// bootstrap running inside the accepted Seatbelt profile, and the target
    /// remains behind the outer release gate when `Started` is observed.
    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn native_acknowledgement_precedes_started_and_target_release() {
        use std::ffi::OsString;
        use std::time::Duration;

        use futures_util::StreamExt;
        use opi_protocol::execution::v1::EnvInherit;

        use crate::policy::{NetworkPolicy, Profile, SandboxPolicy};
        use crate::runner::{SandboxEvent, SandboxRequest, SandboxRunner, StdinPolicy};

        let workspace = tempfile::tempdir().expect("workspace");
        let marker = workspace.path().join("target-ran");
        let request = SandboxRequest {
            program: PathBuf::from("/bin/sh"),
            args: vec![
                OsString::from("-c"),
                OsString::from(format!("printf released > '{}'", marker.display())),
            ],
            workspace: workspace.path().to_path_buf(),
            cwd: workspace.path().to_path_buf(),
            timeout: Duration::from_secs(5),
            env_inherit: EnvInherit::Inherit,
            env_additions: Default::default(),
            stdin: StdinPolicy::Null,
            cancel: None,
        };
        let restriction = MacosRestriction::new(SandboxExecStatus::Available(PathBuf::from(
            SANDBOX_EXEC_PATH,
        )));
        let runner = SandboxRunner::new(
            SandboxPolicy::new(Profile::WorkspaceWrite, NetworkPolicy::Deny),
            Arc::new(restriction),
        );
        let mut run = runner.run(request).expect("native Seatbelt run");

        let temp_root = match tokio::time::timeout(Duration::from_secs(5), run.next())
            .await
            .expect("acknowledgement is bounded")
        {
            Some(SandboxEvent::Started { temp_root, .. }) => temp_root,
            other => panic!("expected Started after native acknowledgement, got {other:?}"),
        };
        let mut probe_name = temp_root.join("release.armed").into_os_string();
        probe_name.push(".probe");
        let probe = PathBuf::from(probe_name);
        let metadata = std::fs::symlink_metadata(&probe).expect("probe metadata");
        assert!(
            metadata.file_type().is_file() && !metadata.file_type().is_symlink(),
            "Started requires a regular in-profile acknowledgement"
        );
        let content = std::fs::read(&probe).expect("read native acknowledgement");
        assert_eq!(content.len(), 64, "acknowledgement uses a 256-bit token");
        assert!(
            content
                .iter()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
            "acknowledgement token is lowercase hexadecimal"
        );
        assert!(!marker.exists(), "target remains gated at Started");

        run.release().expect("release target");
        assert!(matches!(run.next().await, Some(SandboxEvent::Completed(_))));
        assert!(marker.exists(), "target runs only after release");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_non_utf8_workspace_is_refused_before_spawn() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let base = tempfile::tempdir().expect("workspace parent");
        let workspace = base
            .path()
            .join(OsString::from_vec(vec![b'w', b's', b'-', 0xff]));
        std::fs::create_dir(&workspace).expect("create non-UTF workspace");

        assert!(canonicalize_for_profile(&workspace).is_err());
    }
}
