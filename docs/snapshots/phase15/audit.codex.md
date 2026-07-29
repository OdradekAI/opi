# Phase 15 Safety & Sandbox -- Independent Code Audit

**Auditor:** codex (independent, no prior audit reports consulted)

**Date:** 2026-07-29

**Scope:** Tasks 15.1--15.9, task verification commits
`11d4c28e8d3d4d0f85ff0d53f2bdf9795c95cf4c..d88980f8eb703ceb0ce22a39bad25f42fd21c80c`

**Method:** Full reads of the Phase 15 implementation state, normative spec,
design, corrective research, relevant production modules, tests, manifests, and
CI workflows. Source behavior was audited at current `main` HEAD
`13953dedb9b3c7b4572e68af7232af529013f186`, which includes post-phase
remediation commits. Operations/sandbox, trust/startup, and tests/docs/CI were
also reviewed as independent parallel file groups. Existing Phase 15 audit
reports were not opened or searched.

---

## 1. Executive Summary

**Verdict: PASS-WITH-FINDINGS**

| Severity | Count |
|---|---:|
| Blocker | 0 |
| Major | 6 |
| Minor | 6 |
| Info | 0 |

The phase has substantial production wiring and broad tests: Operations
injection, L0 process-tree cleanup, trust-store concurrency, normal-session
resource gating, Windows fallback, paired documentation, and CI platform gates
are all present. The main concerns are cross-layer integration defects: trusted
startup reverses config precedence, early config-consuming commands bypass
trust, Linux ABI 1--3 loses the promised seccomp socket gate, and macOS does not
retain the helper identity it probed. File-tool path checks remain vulnerable to
an ancestor-swap race, and dropped bash futures can leave intermediate output
spills behind.

No finding is a blocker under Phase 15's documented defense-in-depth threat
model, but the Major findings should be corrected before relying on the phase's
safety and trust claims.

### Per-task summary

| Task | Title | Verdict |
|---|---|---|
| 15.1 | Operations contracts and local file backend | Pass |
| 15.2 | Operations injection and local bash execution path | Major findings |
| 15.3 | Sandbox configuration and fallback diagnostic contract | Minor finding |
| 15.4 | Always-on L0 subprocess-tree lifecycle | Pass with output-cleanup finding |
| 15.5.1 | Strict sandbox policy and production dispatch | Pass |
| 15.5.2 | Linux seccomp backend feasibility and policy contract | Minor finding |
| 15.5.3 | Linux strict runtime and L2/L3 policy | Major finding |
| 15.5.4 | macOS sandbox-exec strict backend | Major finding |
| 15.5.5 | Windows strict capability fallback | Pass |
| 15.5.6 | Strict sandbox matrix integration and acceptance | Major/minor findings |
| 15.6 | Project trust store and resolver substrate | Minor finding |
| 15.7 | Trust-gated project resource discovery | Major finding |
| 15.8.1 | Project trust startup policy and headless resolution | Major finding |
| 15.8.2 | Interactive project trust prompt and state transition | Minor finding |
| 15.9 | Safety and trust documentation, guards, and phase acceptance | Minor findings |

---

## 2. Correctness and Cross-Task Integration

### 2.1 MAJOR: Trusted two-stage config resolution reverses layer precedence

**Files:** `crates/opi-coding-agent/src/config.rs:1574-1643`,
`crates/opi-coding-agent/src/main.rs:234-288`,
`crates/opi-coding-agent/src/main.rs:320-387`

**Cause:** `resolve_pre_trust_config` resolves user config, explicit `--config`,
`OPI_MODEL`, and `--model`. For a trusted project, both startup paths then call
`merge_project_config`, which merges the project TOML onto that already-finished
configuration. The normal resolver instead orders the layers user -> project
-> explicit config -> environment/CLI. The staged path therefore is not
equivalent to `resolve_config`, despite its documentation.

The same implementation validates project `[providers.custom]` fragments in a
fresh map. A valid partial project override of a user custom provider can fail,
whereas the normal resolver merges raw fragments and validates once.

**Impact:** A trusted project can override `--model`, `OPI_MODEL`, explicit
`--config` values, and an explicit config's `[sandbox] require = true` unless
the corresponding sandbox CLI flag is also supplied. Valid layered custom
provider configurations can also fail only on the trust-gated startup path.

**Fix:** Preserve raw layer state and finalize in the documented order after
the trust decision, or rerun a full resolver that is authorized to load the
project layer. Validate custom providers once after all raw fragments merge.
Add staged/full-equivalence tests for CLI model, environment model, explicit
config, sandbox fields, and partial custom-provider overrides.

### 2.2 MAJOR: Early config-consuming commands ignore `--no-trust`

**Files:** `crates/opi-coding-agent/src/main.rs:39-43`,
`crates/opi-coding-agent/src/main.rs:61-77`,
`crates/opi-coding-agent/src/main.rs:530-560`,
`crates/opi-coding-agent/src/cli.rs:111-122`

**Cause:** `opi doctor` and `--list-models` dispatch before project-trust
preflight and call the full `resolve_config` with the current directory as the
project layer. Model listing then builds its provider catalog and native
credential-presence probes from that config. This contradicts the CLI contract
that `--no-trust` skips project resources, including `.opi/config.toml`.

**Impact:** `opi --no-trust --list-models` still parses and uses an untrusted
project config; malformed project TOML can fail the command, and
project-configured providers affect listing and credential probes. Doctor also
consumes the explicitly denied project layer. Normal interactive,
non-interactive, and RPC session startup does perform the trust gate correctly.

**Fix:** Route config-consuming early commands through reusable headless
trust-gated resolution while continuing to honor explicit `--config`. Add
subprocess tests for `doctor` and `--list-models` with a denied, malformed, and
provider-defining project config.

### 2.3 MAJOR: Workspace-only file operations are vulnerable to an ancestor-swap race

**Files:** `crates/opi-coding-agent/src/tool/mod.rs:64-109`,
`crates/opi-coding-agent/src/tool/read.rs:131-147`,
`crates/opi-coding-agent/src/tool/write.rs:84-101`,
`crates/opi-coding-agent/src/tool/edit.rs:110-128`,
`crates/opi-coding-agent/src/tool/operations.rs:499-519`

**Cause:** `PathPolicy` canonicalizes and checks the path once, returns a plain
`PathBuf`, and the asynchronous backend later opens, creates, or renames by
pathname. A concurrent process can replace a checked ancestor directory with a
symlink or junction between those steps. Existing escape tests create the link
before resolution and therefore do not exercise the race.

**Impact:** A raced read, write, or edit can be redirected outside the harness
workspace despite the workspace-only policy. Phase 15 correctly disclaims an
OS security boundary, so this is not a Blocker, but it is a real escape from the
documented file-tool policy.

**Fix:** Carry a verified parent-directory handle into the backend and perform
no-follow, descriptor-relative operations and same-directory replacement
relative to that handle (`openat2` with `RESOLVE_BENEATH`/`NO_SYMLINKS` on
Linux and platform equivalents). Add deterministic ancestor-replacement tests
for read, write, and edit.

### 2.4 MAJOR: Dropping a bash execution future can orphan private spill files

**File:** `crates/opi-coding-agent/src/tool/operations.rs:785-790`,
`crates/opi-coding-agent/src/tool/operations.rs:952-982`,
`crates/opi-coding-agent/src/tool/operations.rs:1083-1165`

**Cause:** Each stream-capture task owns a `StreamCapture` containing an
optional spill file. Cleanup is explicit only after the execution future awaits
the capture handles. Dropping the execution future detaches those Tokio tasks;
when they later finish, their `StreamCapture` outputs are dropped without
calling `cleanup_spill` because `StreamCapture` has no `Drop` implementation.
The capture-abort/error branches have the same ownership hazard.

**Impact:** Cancelling or tearing down an execution after more than the in-memory
capture limit can leave command output in the OS temp directory. Unix creation
is private, but the data persists unexpectedly and can contain credentials or
other sensitive output. The existing dropped-future L0 test emits too little
output to detect this.

**Fix:** Make spill ownership RAII-based: remove the intermediate path from
`Drop`, and explicitly disarm only for an intentionally retained merged-output
artifact. Add a dropped-future test that exceeds the capture limit and asserts
that no per-stream spill remains after process-tree cleanup.

---

## 3. Platform Sandbox Findings

### 3.1 MAJOR: Linux ABI 1--3 fail-open drops the seccomp new-socket baseline

**Files:** `crates/opi-coding-agent/src/sandbox/linux.rs:673-726`,
`crates/opi-coding-agent/src/sandbox.rs:313-337`,
`crates/opi-coding-agent/src/sandbox.rs:397-408`,
`crates/opi-coding-agent/src/sandbox/linux.rs:870-895`

**Spec ref:** `docs/opi-spec.md:1878-1892`

**Cause:** On Landlock ABI below 4, `availability(Network)` returns
`TemporarilyUnavailable`. The shared resolver excludes unavailable layers from
`engaged_layers`, and the Linux builder enables network seccomp rules only when
that list contains `Network`. Consequently ABI 1--3 retains filesystem and
possibly L3 syscall rules, but not the ABI-independent socket-creation gate.
The inline ABI V1/V3 test explicitly locks in only `[Fs, Syscalls]`.

**Impact:** With `require = false`, strict network mode on Linux kernels with
Landlock ABI 1--3 allows new AF_INET, AF_INET6, and AF_NETLINK sockets. This
directly contradicts the normative statement that the seccomp socket denial is
always engaged even while Landlock TCP bind/connect is reported as a temporary
gap.

**Fix:** Model the seccomp socket gate and Landlock TCP rights as separate
sublayers, retaining seccomp under fail-open while reporting the TCP gap.
Add an ABI V1/V3 production-path child probe proving `socket(AF_INET)` returns
the selected errno with `require = false`, plus the pre-spawn fail-closed case
for `require = true`.

### 3.2 MAJOR: macOS probes one helper path but launches a newly resolved bare name

**Files:** `crates/opi-coding-agent/src/sandbox/macos.rs:233-270`,
`crates/opi-coding-agent/src/sandbox/macos.rs:273-300`,
`crates/opi-coding-agent/src/tool/operations.rs:649-663`

**Cause:** The capability probe resolves and executes the first
`sandbox-exec` on `PATH`, but `SandboxExecStatus::Available` retains no path.
The confinement plan stores only the literal `"sandbox-exec"`, so every real
command performs a second PATH lookup. A shim can also pass the no-op probe by
accepting `-p` and executing the tail without applying a profile.

**Impact:** Strict mode can report L1/L2 as engaged while executing the shell
without confinement if PATH selects an untrusted shim or helper identity
changes between probe and spawn.

**Fix:** Probe and retain the absolute trusted helper path, preferably
`/usr/bin/sandbox-exec`, and execute exactly that path. Add tests for a
pass-through PATH shim and helper replacement/reordering after the probe.

### 3.3 MINOR: riscv64 can report an unreviewed Linux seccomp backend as engaged

**Files:** `crates/opi-coding-agent/src/sandbox/linux.rs:238-248`,
`crates/opi-coding-agent/src/sandbox/linux.rs:609-618`,
`crates/opi-coding-agent/tests/sandbox_linux_backend.rs:212-217`

**Spec ref:** `docs/opi-spec.md:1897-1902`

**Cause:** Architecture selection delegates directly to seccompiler's
`TargetArch` conversion. The pinned seccompiler version accepts riscv64, so the
backend can report L2/L3 engaged there even though Phase 15 limits verified
engagement to x86_64 and aarch64. The negative test covers only `mips64`.

**Impact:** A non-release riscv64 build claims policy coverage outside the
reviewed and tested target matrix.

**Fix:** Add an explicit x86_64/aarch64 whitelist before conversion and a
riscv64 negative capability test, or add riscv64 to the normative build/runtime
acceptance matrix before claiming engagement.

---

## 4. Security, Redaction, and Trust Edge Cases

### 4.1 MINOR: Sandbox diagnostic redaction accepts arbitrary secret-bearing reasons

**Files:** `crates/opi-coding-agent/src/diagnostics.rs:29-66`,
`crates/opi-coding-agent/tests/sandbox_config.rs:366-380`

**Cause:** The public diagnostic helpers accept any `Into<String>` reason and
copy it verbatim into details. The test named
`sandbox_degraded_redacts_untrusted_reason_text` supplies a credential/path
canary but asserts only that the object has two keys; the canary remains intact
inside `reason`.

**Impact:** Current production call sites use curated reason classes, so no
present leak was found. The stated no-command/no-env/no-path/no-credential
invariant is nevertheless convention-based and can regress through a new
backend or embedder call.

**Fix:** Use typed/static reason categories or sanitize centrally. Assert that
credential and absolute-path canaries are absent from the serialized
diagnostic, not merely that no extra keys exist.

### 4.2 MINOR: “Trust parent” silently becomes session-only at a filesystem root

**Files:** `crates/opi-coding-agent/src/project_trust.rs:710-737`,
`crates/opi-tui/src/trust_prompt.rs:19-35`

**Cause:** `TrustParent` persists through
`project_root.parent().map(...)` but always returns `Trusted`. At `/` or a
Windows drive root there is no parent, so no record and no error are produced
even though the UI defines the choice as durable.

**Impact:** The current session is trusted, but a later session can prompt
again despite the durable-choice label.

**Fix:** Disable the option when there is no distinct parent, return a named
error, or explicitly persist the root itself. Add Unix-root and Windows
drive-root tests.

---

## 5. Test Quality Findings

### 5.1 MINOR: Linux native L2 acceptance does not execute the full required socket matrix

**Files:** `crates/opi-coding-agent/tests/sandbox_strict.rs:881-943`,
`crates/opi-coding-agent/tests/sandbox_strict.rs:1049-1087`,
`.github/workflows/ci.yml:85-100`

**Spec ref:**
`docs/research/2026-07-24-phase15-linux-l2-feasibility.md:135-141`

**Cause:** Native product coverage executes an AF_UNIX stream round-trip and a
TCP bind denial. It does not exercise AF_UNIX datagram IPC or a distinct TCP
connect denial, although CI treats the named test as the L2 product gate.
Structural tests do include both Landlock TCP access rights.

**Impact:** Regressions specific to `ConnectTcp` wiring or Unix datagram
preservation can pass native acceptance.

**Fix:** Add AF_UNIX datagram create/bind/connect/send/receive coverage and a
TCP connect probe whose expected denial is distinguishable from baseline
network failure.

### 5.2 MINOR: Linux native acceptance omits temp-write and L3 runtime assertions

**Files:** `crates/opi-coding-agent/tests/sandbox_strict.rs:1090-1129`,
`crates/opi-coding-agent/tests/sandbox_linux_backend.rs:22-175`

**Cause:** Linux native L1 coverage proves outside-write denial and
workspace-write allowance but not the documented temp-directory carve-out.
L3 has strong structural filter assertions but no selected syscall is executed
under the production child confinement. The temp-write product probe is only
used on macOS.

**Impact:** A Linux-only temp exception or runtime L3 attachment regression can
remain green while plan-shape tests continue to pass.

**Fix:** Add a Linux temp-write allowance to the native product test and a safe,
discriminating L3 syscall probe if the baseline result can be separated from
the sandbox's selected errno.

### 5.3 MINOR: Phase 15 documentation guards search globally rather than by section

**File:** `crates/opi-coding-agent/tests/phase15_safety_sandbox_docs.rs:166-264`

**Cause:** Most English/Chinese spec and README assertions search whole files
for markers. A claim moved out of the Phase 15 or sandbox/trust section would
still satisfy the guard.

**Impact:** Documentation can become structurally misplaced while the
phase-specific guard remains green. The current English and Chinese documents
are synchronized and correctly placed.

**Fix:** Extract the relevant heading-bounded slices first, then run presence
and stale-claim assertions against those slices.

---

## 6. Invariant Verification

| Invariant | Code evidence | Test coverage / assessment |
|---|---|---|
| Operations, sandbox, trust, and UI ownership stays out of `opi-agent` | Production types remain in `opi-coding-agent`; prompt widget remains in `opi-tui` | Structural ownership guard passes |
| `PathPolicy` runs before injected file operations | Read/write/edit resolve before calling `FileOperations` | Mock rejection tests pass; **race-safe containment fails** (2.3) |
| L0 process-tree cleanup is always active, including sandbox off | Bash and adapter spawns configure Unix process groups / Windows Job Objects and terminate on completion/drop | Timeout, cancellation, dropped-future, clean-exit descendant, adapter-drop, and Windows tests pass |
| Fail-open retains independently engaged strict layers | Common resolver keeps engaged layers and build-time gaps | Generally covered; **Linux partial L2 retention fails** (3.1) |
| `require = true` refuses known unavailable layers before command side effects | `StrictOutcome::FailClosed` returns before spawn | Injected/platform tests pass |
| Linux L2 preserves AF_UNIX and blocks new external-family sockets | Seccomp filter shape is correct on verified targets; Landlock uses both TCP rights | ABI >=4 structure/native bind coverage passes; **ABI <4 production composition fails** and operation matrix is incomplete |
| macOS engagement uses the helper proven usable | Profile and capability probe exist | **Fails helper-identity continuity** (3.2) |
| Windows truthfully reports L0-only | Windows backend marks L1--L3 permanently unavailable | Native fallback and one-time diagnostic tests pass |
| Sandbox diagnostics contain no command, env, path, or credential | Current production reasons are curated | Field shape passes; **content invariant is not enforced** (4.1) |
| Trust resolves before project resource/provider/package/harness construction | Normal session startup performs pre-trust config and resource filtering | Interactive, non-interactive, and RPC ordering pass; **early config commands bypass it** (2.2) |
| Untrusted project resources and adapters do not load | Project config/resources/packages/context are filtered by structural scope | Resource-gating and real marker-adapter tests pass |
| Trust decision precedence is CLI -> resolver -> store -> global default -> ask | `prepare_project_startup` seals and evaluates the registry in order | Store/startup suites pass |
| Trusted project config preserves normal config precedence | Claimed by `merge_project_config` documentation | **Fails** (2.1); no staged-equivalence matrix |
| Trust gates loading, not tool execution; no built-in `/trust` | Tool selection is independent; source/docs retain the non-goals | Structural and documentation guards pass |
| English/Chinese Phase 15 documentation remains synchronized | Paired spec and README claims are present | Content passes; section-scoping guard is weak (5.3) |

---

## 7. Coverage and Verification

| Area | Positive coverage | Negative/error coverage | Remaining gap |
|---|---|---|---|
| Operations | Binary read/write, metadata, mkdir, injection, atomic replacement, bounded capture | Backend error identity, invalid paths/content, replacement failures | Ancestor-swap race; dropped spill cleanup |
| L0 lifecycle | Bash and adapter process trees on Unix/Windows | Timeout, cancel, dropped future, attach/terminate faults | No material defect found |
| Sandbox policy | Defaults, toggles, precedence, fail-open subsets, fail-closed | Permanent/temporary capability gaps, redacted field shape | Linux partial L2 and reason-content enforcement |
| Linux | Filter/ruleset shape, native L1/L2 product probes, x86_64/aarch64 target checks | Unsupported mips, alternate-surface residual audit | ABI 1--3 composition, riscv64 claim, connect/datagram/temp/L3 runtime coverage |
| macOS | Profile escaping/toggles and native deny/allow tests | Missing/unusable helper paths | Probed helper identity is not retained |
| Windows | Real Job Object lifecycle and strict-to-L0 fallback | One-time unavailable reporting | Accepted suspended-create/assign race remains documented |
| Trust | Store persistence/locking/ancestors, resolver ordering, all gated resource classes, five prompt choices | Malformed store, untrusted adapters, headless ask, prompt cancellation | Early commands, staged config precedence, root parent choice |
| Docs/CI | Paired EN/ZH claims, non-goals, native three-OS product jobs, six target checks | Exact CI filters reject zero-test success | Section-scoped doc assertions and Linux operation matrix |

Focused local execution on Windows passed 149 relevant tests with zero failures.
The cfg-Linux backend integration binary correctly selected zero tests on this
host, so Linux/macOS runtime conclusions come from full source/test review and
native CI evidence rather than pretending Windows output proves them. The
latest Phase 15 remediation commit in current ancestry (`62750b37`) was green
across the three native OS sandbox jobs and six release-target checks in GitHub
Actions run `30324263356`.

---

## 8. Residuals and Recommendations

### Priority recommendations

1. Fix the trust/config integration first: restore config precedence and route
   every config-consuming command through the trust decision.
2. Correct strict-platform truthfulness: retain Linux seccomp on ABI 1--3 and
   bind macOS confinement to the exact helper that passed the probe.
3. Close local data/policy races: use handle-relative file operations and make
   stream-spill cleanup RAII-based.
4. Add the missing native Linux operation cases, enforce diagnostic content
   redaction, and cover root-level `TrustParent`.

### Reviewed residuals that remain accurately documented

- Strict sandboxing confines only bash subprocess trees and is defense in depth,
  not opi-self confinement.
- Linux L2 does not close inherited descriptors, non-TCP traffic, or
  `io_uring` socket/connect paths.
- Windows remains L0-only and retains the documented create/assign race.
- Adapters receive L0 but not strict confinement.
- Trust gates project-resource loading, not tool execution.
