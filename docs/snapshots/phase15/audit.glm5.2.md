# Phase 15 Safety & Sandbox — Independent Code Audit

**Auditor**: glm5.2 (independent; no prior audit reports consulted)
**Date**: 2026-07-29
**Scope**: Tasks 15.1–15.9 (15 tasks), Phase-15 verified commit range `11d4c28..d88980f`
**Pin audited**: `d88980f` (phase-exit commit; see Method for why HEAD was not used)
**Method**: Independent read of the Phase-15 spec (`docs/opi-spec.md` §15, the
`2026-07-11-phase15-safety-sandbox-design.md` design doc) plus first-hand full reads of the
T4/T5/T6 source and tests, cross-checked by a 7-subsystem fan-out of reader/auditor agents and a
per-finding adversarial verify pass — every read pinned to `d88980f`. No `audit.*.md` or
evaluator/D.2 transcript was read.

---

## 1. Executive Summary

**Verdict: PASS-WITH-FINDINGS**

| Severity | Count |
|----------|-------|
| Blocker  | 0     |
| Major    | 1     |
| Minor    | 14    |
| Info     | 2     |

Phase 15 is a substantial, well-engineered safety/sandbox cluster. The load-bearing security
invariants hold under independent inspection: `#![forbid(unsafe_code)]` is present in
`sandbox.rs`, `tool/operations.rs`, `sandbox/windows.rs`, and `sandbox/linux.rs` (the only Opi-side
`unsafe` is the audited helpers in `tool/process_tree.rs`); the seccomp filter is a correct
default-allow/match-deny overlay that denies new `socket(AF_INET/INET6/NETLINK)` while preserving
`AF_UNIX` and carries the exact L3 danger blocklist (`clone`/`unshare` allowed); Landlock engagement
is correctly keyed to the *observed* ABI, not a kernel string; the macOS seatbelt profile is
deterministically escaped against string-literal and `${var}` injection, correctly ordered for
last-match-wins, and canonicalized for `/var`→`/private/var`; Windows truthfully reports L0-only
with a kill-on-close / no-breakaway Job Object; fail-closed refuses spawn before any side effect;
diagnostics carry only a redacted `{layer, reason}`; and the trust gate skips every project layer
(config/skills/fragments/themes/extensions/project-scope adapter declarations/context files) for an
`Untrusted` project while an untrusted project's native adapter children provably never start.

The one **Major** is a cross-platform L0 consistency bug: on Windows, disarming the tree on a clean
bash exit drops the Job-Object handle and kills backgrounded survivors — the opposite of the
documented `disarm` intent and of Unix behavior. The Minors cluster around (a) the strict-sandbox
*default* silently providing no confinement on macOS and on Linux <6.7 because confinement attaches
only on a fully-`Engaged` outcome, together with spec prose that overclaims the seccomp layer is
"always engaged"; (b) toggle-fidelity (per-layer `Some(false)` opt-outs honored at the decision
layer but ignored at enforcement); and (c) test-quality gaps where guards assert on dead/stale
surfaces (a backwards seatbelt comment, a vacuous temp-residue filter, a dead macOS argv function,
no behavioral L3-danger denial test). None of the Minors is a security hole; most are truthfulness
or coverage gaps in a phase whose acceptance criteria explicitly weight both.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 15.1   | Operations contracts and local file backend            | PASS |
| 15.2   | Operations injection and local bash execution path      | PASS-WITH-FINDINGS (m8) |
| 15.3   | Sandbox configuration and fallback diagnostic contract  | PASS |
| 15.4   | Always-on L0 subprocess-tree lifecycle                  | PASS-WITH-FINDINGS (**M1**, m8) |
| 15.5.1 | Strict sandbox policy and production dispatch           | PASS-WITH-FINDINGS (m3) |
| 15.5.2 | Linux seccomp backend feasibility and policy contract   | PASS-WITH-FINDINGS (m1, m2, m4) |
| 15.5.3 | Linux strict runtime and L2/L3 policy                   | PASS-WITH-FINDINGS (m2, m3, m4) |
| 15.5.4 | macOS sandbox-exec strict backend                       | PASS-WITH-FINDINGS (m3, m5, m10) |
| 15.5.5 | Windows strict capability fallback                      | PASS |
| 15.5.6 | Strict sandbox matrix integration and acceptance        | PASS |
| 15.6   | Project trust store and resolver substrate              | PASS-WITH-FINDINGS (m11) |
| 15.7   | Trust-gated project resource discovery                  | PASS-WITH-FINDINGS (m9) |
| 15.8.1 | Project trust startup policy and headless resolution    | PASS |
| 15.8.2 | Interactive project trust prompt and state transition   | PASS-WITH-FINDINGS (m12) |
| 15.9   | Safety and trust documentation, guards, and acceptance  | PASS-WITH-FINDINGS (m3, m13, m14) |

---

## Method, scope pin, and contamination disclosure

- **Commit pin.** Between phase-exit (`d88980f`) and current HEAD (`13953de`), committed **Phase 16**
  work modified the Phase-15 source substantially (`tool/operations.rs` +722, `sandbox/linux.rs`
  +610, `sandbox.rs` +403, `tool/process_tree.rs` +303, `interactive.rs` +287, …). To audit Phase 15
  rather than the in-flight Phase 16, every source/test read was taken at `d88980f` via
  `git show d88980f:<path>`. Normative docs were read at HEAD (stable for §15). The findings below
  cite line numbers at `d88980f`.
- **Contamination.** I did not read `docs/snapshots/phase15/audit.glm5.2.md`,
  `audit.gpt5-codex.md`, any evaluator/D.2 transcript, or the skill files (the stale prior file was
  removed unread to enable a fresh write). I do carry prior-session memory of Phase-15 *outcomes*;
  every finding below was re-derived from the `d88980f` code and spec, not from memory. Two of the 29
  workflow verify agents died on a transient API 429 (`verify:7:macos.rs`,
  `verify:13:tool_operations.rs`); I adjudicated both findings myself from first-hand reads (they
  became m10 and m8).
- **No `cargo`** was run during the audit (host disk-fill risk); findings are read-derived.

---

## 2. Security / Sandbox Findings

### M1 — MAJOR: Windows L0 `disarm()` on clean exit kills backgrounded survivors (contradicts doc intent and Unix)

**File:** `crates/opi-coding-agent/src/tool/process_tree.rs`
**Lines:** `disarm` doc 163–170 / body 171–173; `JobGuard::Drop` 373–384; flag 309; call site `crates/opi-coding-agent/src/tool/operations.rs:707–709`
**Cause:** On a clean child exit `LocalBashOperations::exec` calls `l0_tree.disarm()` (operations.rs
Done arm). `TreeGuard::disarm` overwrites `self.inner` with `TreeGuardInner::Disabled`
(process_tree.rs:172). On Windows the previous inner is `Job(Some(job))`; overwriting it **drops the
`JobGuard`**, whose `Drop` runs `CloseHandle` (process_tree.rs:380). Because the job was created with
`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` (process_tree.rs:309), closing the last handle kills every
process still in the job. The `disarm` doc comment (process_tree.rs:163–170) explicitly states this
path exists so "the tree is NOT torn down … matching pre-15.4 behavior for backgrounded survivors,"
and on Unix `disarm` is a true no-op (the `Disabled` variant's `terminate`/`Drop` does nothing), so
Unix correctly preserves a backgrounded child (`sleep 100 &`) across a clean bash exit.
**Impact:** On Windows, any backgrounded descendant is killed when its parent bash command exits
cleanly — a cross-platform behavioral inconsistency that contradicts the documented L0 contract and
the 15.4 DoD (which scopes tree termination to "timeout, cancellation, and dropped execution", not
clean exit). Not a crash or security hole, but incorrect behavior on a normal path.
**Fix:** Make `disarm` not drop the `JobGuard` on Windows. Add a `JobGuard::disarm` (or transition
the handle out without closing it, e.g. `mem::replace` into a `ManuallyDrop`/leaked handle kept for
the lifetime `disarm` promises) so the kill-on-close safety net fires only on the genuine teardown
paths (timeout/cancel/drop). Add a Windows behavioral test mirroring the Unix
backgrounded-survivor-preserves-on-clean-exit property.

### m3 — MINOR: Default `--sandbox strict` attaches no confinement on macOS and on Linux ABI<4; spec overclaims the seccomp layer is "always engaged"

**File:** `crates/opi-coding-agent/src/sandbox.rs:391–395` (confinement attached only on `Engaged`); `sandbox/linux.rs:508–519` (Network `TemporarilyUnavailable` on ABI<4); `sandbox/macos.rs:117–149` + `sandbox.rs:408–410` (macOS Syscalls permanent gap); spec `docs/opi-spec.md:1884–1892`
**Cause:** `prepare_production` attaches the confinement plan **only when the outcome is `Engaged`**
(sandbox.rs:391–395). A single requested-but-unavailable layer forces `FailOpen` (or `FailClosed`
under `require=true`), so no confinement attaches. With the default config (`fs/network/syscalls`
all `None` = requested):
- **macOS** advertises `Syscalls` as `PermanentlyUnavailable` (`sandbox-exec` is L1/L2 only), so a
  default strict request fail-opens to L0 — the `sandbox-exec` deny overlay never engages unless the
  user explicitly sets `syscalls = false`.
- **Linux on Landlock ABI<4** (kernels <6.7 — Ubuntu 22.04's 5.15, RHEL 9's 5.14, Debian 12's 6.1)
  advertises `Network` as `TemporarilyUnavailable` (no Landlock TCP), so a default strict request
  fail-opens to L0 — neither the seccomp socket gate, the danger blocklist, nor Landlock-FS engages,
  even though the seccomp filter is ABI-independent.

This fail-open *behavior* is spec-sanctioned (§15: "a strict `network`-requested config on an ABI-3
kernel fails open"). The defect is the **truthfulness/UX gap**: (a) §15 prose states "the seccomp
socket-creation denial is always engaged", which is literally false whenever the outcome is not
`Engaged`; and (b) no user-facing doc (README/spec) tells users they must set `syscalls = false`
(macOS) or accept L0-only / opt out of `network` (Linux <6.7) to actually obtain confinement.
**Impact:** A user who sets `--sandbox strict` on macOS or a common LTS Linux and assumes they have
confinement gets L0 only, silently. For a defense-in-depth feature this is a meaningful
expectation gap, and the `phase15_safety_sandbox_docs` truthfulness guard does not catch the
"always engaged" overclaim.
**Fix:** (1) Correct §15 to state the seccomp filter is *built* unconditionally but *applied* only
when the strict outcome is `Engaged` (i.e., when no requested layer is unavailable). (2) Document in
the README/spec flag table that macOS requires `syscalls = false` and that Linux <6.7 confines only
with `network = false` (or upgrade to ≥6.7). (3) Consider attaching the ABI-independent seccomp
danger-blocklist independently of the Landlock-TCP layer so the L3 blocklist engages even on ABI<4 —
this is a design change, so scope it deliberately.

### m2 — MINOR: Per-layer `Some(false)` opt-outs are honored at the decision layer but ignored at enforcement

**File:** `crates/opi-coding-agent/src/sandbox/linux.rs:367–382` (`build_linux_confinement`); `crates/opi-coding-agent/src/sandbox/macos.rs:309–327` (`build_macos_confinement` always `render_profile(true, true)`)
**Cause:** `SandboxConfig.{fs,network,syscalls}` are `Option<bool>` "so the dispatcher can
distinguish an explicit opt-out from an unset value." `prepare` honors `Some(false)` (skips querying
the layer), but once the outcome is `Engaged`, `build_linux_confinement` / `build_macos_confinement`
build the **full** plan regardless of which layers the user opted out — e.g., a macOS user who sets
`network = false` still gets `(deny network*)` in the rendered profile.
**Impact:** Over-confinement (the safer direction), so not a security gap; but it makes the
per-layer toggles misleading at the enforcement layer. The 15.5.3 D.2 review already recorded this
as an accepted hardening deferral.
**Fix:** Thread the resolved toggles into `build_*_confinement` so an opted-out layer is omitted from
the plan (macOS: pass `fs_enabled`/`network_enabled`; Linux: conditionally add the Landlock net
rights). Low priority.

### m5 — MINOR: macOS `build_wrapped_argv` has zero production callers; a host-independent test asserts on this dead function

**File:** `crates/opi-coding-agent/src/sandbox/macos.rs:227–235` (`build_wrapped_argv`); test `crates/opi-coding-agent/tests/sandbox_strict.rs:697`
**Cause:** `git grep build_wrapped_argv d88980f` returns two hits: the definition and one test. The
production macOS path builds its argv through `MacosStrictBackend::build_confinement` →
`Confinement::launcher("sandbox-exec", vec!["-p", profile])` and the spawn-site rebuild in
`operations.rs:608–626`, never calling `build_wrapped_argv`.
**Impact:** The host-independent "argv preservation" test exercises a function that is not on the
production path, so it gives false confidence about the real launcher argv.
**Fix:** Either delete `build_wrapped_argv` or retarget the test at the real
`Confinement::launcher_prefix` → `operations.rs` rebuild (assert the rebuilt `Command` argv is
`sandbox-exec -p <profile> sh -c <command>`).

---

## 3. Correctness / Invariant Findings

### m1 — MINOR: Linux `Syscalls` availability reports `Engaged` without an arch check while confinement build can fail on unsupported arches

**File:** `crates/opi-coding-agent/src/sandbox/linux.rs:491` (`Syscalls => Engaged`) vs `:368` (`std::env::consts::ARCH.try_into().ok()?`)
**Cause:** `LinuxStrictBackend::availability(Syscalls)` returns `Engaged` unconditionally
(linux.rs:491), but `build_linux_confinement` resolves `TargetArch` via
`std::env::consts::ARCH.try_into().ok()?` and returns `None` on any arch without a verified seccomp
backend (linux.rs:368). On such an arch the resolver would report syscall confinement as engaged
while `build_confinement` produces no plan, so an `Engaged` decision could carry `confinement =
None`.
**Impact:** Not reachable on the shipped release matrix (x86_64/aarch64 only), so practical impact
is low; but it is a capability/reporting inconsistency that could mislead on a future arch addition.
**Fix:** Gate `availability(Syscalls)` on the same arch check (`TargetArch` resolution) so an
unsupported arch reports `PermanentlyUnavailable` rather than `Engaged`.

### m11 — MINOR: `resolve_project_trust_decision` maps `Undecided → Trusted` (permissive 15.7 default), opposite to the authoritative headless policy

**File:** `crates/opi-coding-agent/src/project_trust.rs:483–495` (esp. :494)
**Cause:** The superseded `pub fn resolve_project_trust_decision` maps `Undecided => Trusted`
(project_trust.rs:494) so pre-15.8.1 single-layer projects keep loading. The authoritative policy
(`ProjectStartupPlan::headless_decision`, project_trust.rs:572–577) maps `Undecided => Untrusted`.
`git grep` shows `resolve_project_trust_decision` has no production caller at `d88980f` (15.8.1/15.8.2
route through `prepare_project_startup`), so it is dead but still `pub` and divergent.
**Impact:** No live behavior change, but a `pub` function exporting the opposite default from the
real policy is a footgun for embedders who discover and call it.
**Fix:** Delete `resolve_project_trust_decision`, or document that it is a 15.7-compatibility shim
and warn against its use.

### m12 — MINOR: `run_trust_prompt_terminal` leaks raw mode / alt-screen on error and can discard a sent Trust choice

**File:** `crates/opi-coding-agent/src/interactive.rs:144–147`
**Cause:** The prompt setup does `enable_raw_mode()? → EnterAlternateScreen()? → Terminal::new(backend)?`
with bare `?` and **no RAII/Drop guard**. If `EnterAlternateScreen` or `Terminal::new` fails after
raw mode is enabled, the function returns early leaving the terminal in raw mode / alternate screen.
The same function is not panic-safe (an unwind mid-prompt would skip cleanup), and a Trust choice
already sent over the oneshot before a late failure can be discarded.
**Impact:** Terminal state corruption on a setup failure path (rare), and a non-panic-safe prompt.
Not reached on the happy path.
**Fix:** Wrap the setup in an RAII guard whose `Drop` runs `disable_raw_mode` +
`LeaveAlternateScreen` on error, or use a scope guard; ensure the oneshot receive cannot lose an
already-sent choice.

---

## 4. Spec-Compliance / Documentation Findings

### m14 — MINOR: §15 acceptance trace (SC1) cites `adapter_host_mock::adapter_process_group_contract`, but the test lives in `sandbox_l0.rs`

**File:** `docs/opi-spec.md:2011` (SC1 row); actual test `crates/opi-coding-agent/tests/sandbox_l0.rs:520`
**Cause:** `git grep -n 'fn adapter_process_group_contract' d88980f` returns exactly one hit,
`sandbox_l0.rs:520`, not `adapter_host_mock`. The SC1 acceptance-trace row names the wrong test
module. (This matches the low-severity citation flag already recorded in the phase-exit
`audit_notes` for SC1.)
**Impact:** Documentation only; the criterion is met by the real test, so no status change.
**Fix:** Correct the SC1 row to `sandbox_l0::adapter_process_group_contract`.

### m13 — MINOR: CHANGELOG Breaking entry omits that `AppState` lost `Copy/Clone/PartialEq/Eq`

**File:** `CHANGELOG.md` (`[Unreleased]` Phase 15 entry); `crates/opi-tui/src/lib.rs:145`
**Cause:** Adding `AppState::AwaitingTrust(AwaitingTrustState)` reduced `AppState` from
`#[derive(Debug, Clone, Copy, PartialEq, Eq)]` to `#[derive(Debug)]` only — a real breaking change
for a published 0.x crate (`opi-tui`). The CHANGELOG `[Unreleased]` Phase 15 Breaking entry notes
the new variant but does not call out the lost derives.
**Impact:** Downstream embedders matching/comparing/cloning `AppState` break without warning.
**Fix:** Add a Breaking line: "`AppState` loses `Copy`/`Clone`/`PartialEq`/`Eq` (now carries
`AwaitingTrustState`)."

### m10 — MINOR: macOS sandbox module doc still says the runtime is "deferred to a native macOS runner" though it is landed

**File:** `crates/opi-coding-agent/src/sandbox/macos.rs:10–24`
**Cause:** The module-level doc (written during the substrate-only 15.5.4 iteration) states the
macOS runtime — `sandbox-exec` probe, `MacosStrictBackend`, `Confinement::launcher`, dispatcher
wiring — is deferred. At `d88980f` all of that is present (`probe_sandbox_exec`,
`MacosStrictBackend`, `build_macos_confinement`, selected by `production_sandbox_backend` on macOS)
and verified on GHA `macos-latest`.
**Impact:** Stale internal doc; misleading to anyone reading the module.
**Fix:** Update the module doc to reflect that the runtime landed and is CI-verified on macOS.

---

## 5. Test-Quality Findings

### m4 — MINOR: L3 danger blocklist is never behaviorally exercised — only structurally pinned

**File:** `crates/opi-coding-agent/src/sandbox/linux.rs:77–101` (`danger_syscalls`); tests `sandbox_linux_backend.rs` / `sandbox_strict.rs`
**Cause:** The danger blocklist is verified by asserting the syscall *set* and rule shape, but no
engaged test drives a confined child to actually trigger a denied syscall (e.g., attempt `bpf`/
`ptrace` and assert `EPERM`). The substrate test's own doc comment promises a behavioral proof that
15.5.3 did not deliver.
**Impact:** A future regression that compiles the wrong syscall numbers, or drops the filter from
the applied plan, would not be caught by an engaged denial. Given m3 (the filter is only applied on
`Engaged`), this coverage gap is more material than it appears.
**Fix:** Add an engaged Linux test that runs a denied danger syscall under strict mode and asserts
the stable `EPERM`.

### m7 — MINOR: Atomic-write residue-leak guard filters for a stale temp tag (`.opi-write-tmp`) production never emits

**File:** `crates/opi-coding-agent/tests/tools_read_write_edit_bash.rs:565` (filter) vs `crates/opi-coding-agent/src/tool/operations.rs:303/435` (tag `opi-ops-tmp`)
**Cause:** The "no temp residue left behind" assertion filters `read_dir` entries with
`.contains(".opi-write-tmp")`, but production `atomic_write_bytes` stages temps as
`.{file}.opi-ops-tmp-{pid}-{nanos}`. The filter never matches production temps, so the guard is
vacuously true.
**Impact:** A future temp-leak regression under the real tag would not be caught.
**Fix:** Change the filter to `.contains("opi-ops-tmp")` (the production tag).

### m8 — MINOR: PathPolicy-before-backend ordering is proven only for the read tool; write/edit lack the outside-workspace-rejects-before-backend test the DoD claims

**File:** `crates/opi-coding-agent/tests/tool_operations.rs`
**Cause:** The DoD for 15.2 claims coverage that "policy rejection occurs before backend invocation"
across read/write/edit, but the mock-backend "no call on policy reject" assertion exists only for
the read tool. The mutating write and edit tools have no equivalent test proving an outside-workspace
path is rejected before the `FileOperations` backend is touched.
**Impact:** A regression that re-ordered policy vs. backend for write/edit would not be caught.
**Fix:** Add mock-backend tests for `WriteTool`/`EditTool` asserting the backend records zero calls
on an outside-workspace path.

### m9 — MINOR: Untrusted `.opi/extensions` and `.opi/themes` gating is not behaviorally exercised

**File:** `crates/opi-coding-agent/tests/trust_resource_gating.rs` (`write_project_resources` 42–74; `untrusted_project_skips_every_gated_layer` 271–333); production retain loop `crates/opi-coding-agent/src/harness.rs:2162–2176`
**Cause:** The "untrusted skips every gated layer" fixture writes project resources but the
behavioral assertions do not separately confirm that `.opi/extensions` and `.opi/themes` are filtered
for an untrusted project. Other gated layers (config/skills/fragments/packages/context) are
exercised.
**Impact:** A regression un-filtering extensions/themes for untrusted projects would not be caught.
Extensions is the most security-relevant gap (a project extension could otherwise load before/around
consent).
**Fix:** Extend the fixture/assertions to cover `.opi/extensions` and `.opi/themes` non-loading for
untrusted projects.

### m6 — MINOR: Stale seatbelt semantics comment in `sandbox_strict.rs` ("first-match-wins" — it is last-match-wins)

**File:** `crates/opi-coding-agent/tests/sandbox_strict.rs:619–620`
**Cause:** The comment reads "Seatbelt is first-match-wins: the workspace/temp exceptions MUST
precede the root deny…". Seatbelt is **last-match-wins** (the production `render_profile` correctly
emits the root deny *first* then the exceptions, per macos.rs:200–214). The assertion immediately
below is correct; only the comment is backwards.
**Impact:** Misleading comment; the assertion happens to match production order, so no false pass
today, but the comment would mislead a future editor into reordering wrongly.
**Fix:** Correct the comment to "last-match-wins."

### i1 — INFO: `rpc_never_emits_trust_prompt` builds `RpcRunner` via the non-production `::new` (hardcodes `Trusted`)

**File:** `crates/opi-coding-agent/tests/rpc_trust.rs:167–176` (test) vs `crates/opi-coding-agent/src/rpc.rs:148–158`
**Cause:** The test constructs `RpcRunner::new(...)`, which hardcodes `trust_decision = Trusted`
and passes empty registry/installed-packages, so it does not drive the production
`prepare_project_startup` → headless path. Its docstring therefore overstates coverage of the
production untrusted-RPC path.
**Impact:** The "RPC never emits a trust prompt" property is still true (RPC has no UI request
surface, confirmed in `rpc.rs`), but this particular test is not the proof.
**Fix:** Add a test that drives RPC startup through the production preflight with an untrusted
project and asserts no UI request.

### i2 — INFO: Marker-adapter spawn-absence is proven by input-vector deduction, not by a spawnable `[adapter]` through the real `AdapterHost::start`

**File:** `crates/opi-coding-agent/tests/trust_resource_gating.rs:105–114` (`write_package` writes a bare manifest with no `[adapter]` table)
**Cause:** The untrusted-adapter test proves the filtered package vector never reaches
`start_adapters_from_packages`, but the fixture manifests have `adapter = None` (no `[adapter]`
table), so even the "global adapter still starts" half does not exercise a real spawnable adapter
through `AdapterHost::start`.
**Impact:** The declaration-load gate is proven; the spawnable-adapter realism is not. Low risk.
**Fix:** Give the global adapter fixture a real `[adapter]` table and assert it spawns while the
project one does not.

---

## 6. Invariant Verification

| Invariant | Code evidence (`d88980f`) | Test coverage |
|-----------|---------------------------|---------------|
| `#![forbid(unsafe_code)]` in `sandbox.rs`, `tool/operations.rs`, `sandbox/windows.rs`, `sandbox/linux.rs` (under sandbox.rs); only Opi-side `unsafe` is the audited helpers in `tool/process_tree.rs` | `sandbox.rs:40`, `operations.rs:32`, `windows.rs:19`, `linux.rs:38`; `observed_landlock_abi` + `install_child_confinement` + Job-Object FFI in `process_tree.rs:413–482`, `281–384` | `phase15_safety_sandbox_docs` structural grep on the modules; SC6 CI matrix |
| Seccomp filter is default-allow / match-deny; denies new `socket(AF_INET/INET6/NETLINK)`, preserves `AF_UNIX`; exact L3 blocklist; `clone`/`unshare` allowed | `linux.rs:120–154` (mismatch=Allow, match=Errno(EPERM), socket arg[0] rules, empty danger rules), `65–101` | `sandbox_linux_backend` rule-shape tests; **behavioral L3 denial test missing (m4)** |
| Landlock keyed to *observed* ABI (probe, not kernel string); TCP bind/connect ABI≥4; FS writes ABI≥1 | `process_tree.rs:413–433` (`observed_landlock_abi`); `linux.rs:384–391`, `487–521` | `sandbox_linux_backend::landlock_capability_uses_observed_abi…`; engaged `linux_landlock_abi4_denies_tcp_bind_connect` |
| macOS profile escaped + last-match-wins ordered + canonicalized | `macos.rs:158–219` (`escape_path` escapes `\ " $`; deny-then-exceptions), `332–338` (canonicalize) | `macos_profile_and_capability_matrix`; native engaged product tests |
| Windows Job Object kill-on-close + no-breakaway; L0-only truthful | `process_tree.rs:281–384` (flag 309 omits breakaway-OK), `windows.rs:33–39` | `sandbox_l0::windows_bash_and_adapter_use_kill_on_close_job`; `windows_strict_*` |
| L0 always-on in `mode = off`; tree killed on timeout/cancel/drop | `operations.rs:587` (`configure_tree` always), `639–726` (race + terminate), `process_tree.rs:271–275` (Drop=terminate) | `sandbox_l0::bash_l0_kills_process_tree_in_off_mode`; **Windows clean-exit survivor preservation broken (M1)** |
| Fail-closed refuses spawn before any side effect | `operations.rs:568–572` (`SandboxUnavailable` returned pre-spawn) | `sandbox_strict::unavailable_layer_fail_open_and_fail_closed` |
| Diagnostics redacted to `{layer, reason}`; no command/env/path/cred | `diagnostics.rs:35–67`; `operations.rs:881–907` | inline redaction tests; `permanent_gap_diagnostic_is_once_per_startup` |
| Untrusted skips ALL project layers (config/skills/fragments/themes/extensions/packages/context); project adapter children never start | `project_trust.rs:631–672`; `runtime_packages.rs` filter → sole `start_adapters_from_packages`; `context_files.rs`; `harness.rs:2162–2176` | `trust_resource_gating` 11/11; **extensions/themes behavioral gap (m9)** |
| Registry sealed at resolution; standard CLI empty; no CLI `-e` / native loader; no project self-authorization | `project_trust.rs:262–311`, `631–655`; `main.rs` uses `::new()` | `project_trust_startup::standard_cli_passes_empty_resolver_registry`, `no_resource_bypass…` |
| Headless never prompts → Untrusted; RPC emits no UI request | `project_trust.rs:580–613` (`HeadlessPreTrustUi`), `568–578` | `non_interactive_trust`, `rpc_trust` (note i1) |
| Interactive prompt precedes provider/package/adapter/harness construction | `interactive.rs` + `main.rs` (two sequential `block_on`) | `interactive_trust::interactive_prompt_precedes_project_startup_side_effects` |
| Operations layered below `PathPolicy`; nav tools unchanged (4-of-8 widen); `FileOperations` unsandboxed | `operations.rs:206–257`, `tool/mod.rs`; `harness.rs build_tools_with_sandbox` | `tool_operations`, `tool_selection::build_tools_constructs_expected_default_set`; **write/edit PathPolicy-before-backend gap (m8)** |

---

## 7. Residuals and Recommendations

### Priority recommendations
1. **(Major M1)** Fix the Windows `disarm`-on-clean-exit path so backgrounded survivors are
   preserved, matching Unix and the documented intent. Add a Windows behavioral test.
2. **(Minor m3)** Correct the §15 "seccomp socket-creation denial is always engaged" prose and
   document the macOS `syscalls = false` / Linux-<6.7 `network = false` requirement in the
   user-facing flag table, so `--sandbox strict` does not silently degrade to L0 on the default
   config. Optionally engage the ABI-independent seccomp danger-blocklist independently of the
   Landlock-TCP layer.
3. **(Minor m4 / m8 / m9)** Close the three security-adjacent test gaps: a behavioral L3
   danger-syscall denial test; PathPolicy-before-backend for write/edit; and behavioral
   `.opi/extensions` + `.opi/themes` untrusted-gating assertions.
4. **(Minor m13)** Note the `AppState` derive loss in the CHANGELOG Breaking entry.

### Carry-forward (already documented, confirmed still open)
- Linux L2 is defense-in-depth, not complete network isolation: inherited/open INET/INET6/NETLINK
  fds, non-TCP traffic, `io_uring` socket/connect/accept, and address-range filtering remain
  explicit residuals (`linux.rs:418–448` alternate-surface audit; §15). Confirmed accurate.
- Windows `CreateProcess-suspended → Assign → Resume` race (post-spawn Job assignment) — accepted,
  documented residual (`process_tree.rs` assign-after-spawn).
- Adapter strict-confinement (L0 only in Phase 15), nav-tool Operations, and SSH/container remote
  Operations backends remain deferred follow-ups (design §Residuals). Confirmed not implemented.
- `resolve_project_trust_decision` (m11) and `build_wrapped_argv` (m5) are dead/divergent surfaces
  worth deleting.

### Notes
- The fail-open *behavior* on macOS default-strict and Linux ABI<4 strict is spec-sanctioned
  (§15 explicitly states ABI-3 fails open). Finding m3 concerns the doc truthfulness and the
  undocumented UX, not the fail-open posture itself.
- All platform CI evidence (GHA run 30207584078, 9/9 owned jobs green; 6-triple target-check matrix)
  was not re-run during this audit (host is Windows; disk-fill risk). The findings are read-derived
  against the `d88980f` source.
