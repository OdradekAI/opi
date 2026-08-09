# Phase 16 Pluggable Extensions and Command Execution — Independent Code Audit

**Auditor**: glm5.2 (independent, fresh context; no prior audit reports consulted)
**Date**: 2026-08-09
**Scope**: Tasks 16.1–16.16.3, commits `6f51761..26613ac` (current HEAD, **including the three post-exit remediation passes** `2b23010`, `edd8d91`, `458736f`)
**Method**: Matt `code-review` (separate Standards + Spec axes) → 6-dimension adversarial workflow (6 finders + 6 skeptic verifiers, 12 agents) → independent invariant tracing → native verification (WSL2 Linux real Landlock/seccomp + GHA `macos-latest` sandbox-exec) → CI reproduction on the remediated HEAD (draft PR #3, run 31319356200)
**Independence note**: This is a fresh-context re-audit by the same model family that produced the earlier `audit.glm5.2.md`. The earlier report was **not** read. Per the finding contract, independence is `fresh-context-same-family`. The earlier report predated the final remediation pass and never executed CI on this code.

---

## 1. Executive Summary

**Verdict: PASS-WITH-FINDINGS**

| Severity | Count |
|----------|-------|
| Blocker  | 0     |
| Major    | 4     |
| Minor    | 7     |
| Info     | 7     |

The Phase 16 **design and core implementation are strong**: the deep audit (Spec axis + 6-dimension adversarial workflow over the full source) produced **zero Blockers and zero code-level Majors**. The fail-closed protocol host, redaction envelope, deterministic router, independent lifecycle gates, TOCTOU-safe contribution validator, crate boundaries, and Minimal Runtime all verify against the spec. C8/C9 (Linux Landlock+seccomp) and C10 (macOS sandbox-exec) were confirmed by **real native execution**, not stubs.

The **Majors are all integration/CI-hygiene defects** on a branch that is **18 commits ahead of `origin/main` and has never been CI-verified before this audit**. The most important is a **macOS build break** (`tempfile` declared as a dev-dependency but used in non-test library code) that none of the prior pre-remediation audits caught — because it only manifests on macOS and the developer's host is Windows. The remediated HEAD is **CI-red on all three OSes** for distinct, fixable reasons. None of these indicate a systemic problem with the Phase 16 architecture; they indicate the remediation was not validated against CI before being left unpushed.

Two agent-flagged findings were **refuted by independent tracing** (documented in §9): the "no-degraded-success" C5 concern and an "ungated test seam" claim both fail at the tool layer / are `#[cfg(test)]`-gated. This validates the adversarial-verification approach.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 16.1 | Pin Phase 16 documentation contract | PASS |
| 16.2 | Pin L0 supervision and policy-neutral seam | PASS |
| 16.3 | Add opi-protocol::execution::v1 | PASS |
| 16.4 | Parse and hard-gate executable contributions | PASS-WITH-FINDINGS (owns the macOS `tempfile` build break — Major A1) |
| 16.5 | Add Package Trust and enable/disable lifecycle | PASS |
| 16.6 | Execution config, failures, routing, permission policy | PASS |
| 16.7 | One-shot execution protocol host | PASS-WITH-FINDINGS (owns Windows cancellation test failure — Major A3; source-text pin tests — Minor D1) |
| 16.8 | Deep Execution Runtime assembly | PASS |
| 16.9 | Wire Execution Runtime, dynamic bash schema, public surfaces | PASS |
| 16.10 | Interactive permission broker and TUI prompt | PASS |
| 16.11.1/.2 | Standalone opi-sandbox SDK and human CLI | PASS (C8 verified natively) |
| 16.12 | Atomic helper gate and protocol backend | PASS |
| 16.13 | Port Linux native restriction contract | PASS-WITH-FINDINGS (C9 verified; Landlock-TCP test gap — Minor D4; clippy lint — Minor M1) |
| 16.14.1 | Port macOS native restriction contract | PASS (C10 verified via GHA) |
| 16.14.2 | Pin Windows unsupported execution posture | PASS (C11 verified) |
| 16.15.1 | Host-neutral opi-sandbox packaging | PASS |
| 16.15.2 | Native package CI, release, artifact audit | PASS-WITH-FINDINGS (CI topology present; CI is RED — Major A4; artifact-audit test failures — Major A2) |
| 16.16.1 | Remove core native sandbox, enforce migration | PASS (C13/C14 verified) |
| 16.16.2 | Install-to-execute and cross-surface diagnostics | PASS |
| 16.16.3 | Synchronize docs and close repository gates | PASS-WITH-FINDINGS (docs lockstep OK; repository gates RED) |

---

## 2. CI Health Findings (Majors — highest priority)

The remediated HEAD was run through CI via draft PR #3 (run `31319356200`, OdradekAI/opi). Result: **`conclusion: failure`**, red on all three OSes. The Phase 16 product tests that *did* run are overwhelmingly green; the failures are specific and characterized below.

### A1 — MAJOR: `tempfile` is a dev-dependency but is used in non-test library code → opi-coding-agent does not compile on macOS

**Files:** `crates/opi-coding-agent/Cargo.toml:77` (`tempfile` under `[dev-dependencies]`); `crates/opi-coding-agent/src/execution/contribution.rs:502`

`tempfile` is declared only in `[dev-dependencies]`, but the `#[cfg(unix)] fn bind_launch_material` references it on its non-Linux (macOS) branch:

```rust
#[cfg(not(target_os = "linux"))]
let mut snapshot = tempfile::tempfile()?;
```

Dev-dependencies are not in scope when the **library** crate is compiled. On macOS (unix ∧ ¬linux) this line is active in the lib, so `cargo check/build -p opi-coding-agent` fails with `error[E0433]: cannot find module or crate 'tempfile' in this scope`. It is invisible on the developer's Windows host (the `#[cfg(not(unix))]` clone path is used) and on Linux (the memfd branch is used), which is why it escaped notice.

**Impact:** Blocks every macOS build of the Opi binary and library — a documented six-target release platform (C12). Concretely it failed `Target check (x86_64-apple-darwin)`, `Target check (aarch64-apple-darwin)`, and `execution_acceptance (macos-latest)` in CI.

**Fix:** Move `tempfile` from `[dev-dependencies]` to `[dependencies]` in `crates/opi-coding-agent/Cargo.toml` (it is legitimately needed by the macOS launch-material path).

```yaml
id: glm5.2-A1
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: integration
severity: Major
title: tempfile dev-dependency used in non-test lib code breaks macOS compilation
claim: opi-coding-agent's library does not compile on macOS because tempfile (declared only in [dev-dependencies]) is referenced in non-test code at contribution.rs:502 on the macOS (unix non-linux) launch-material branch.
evidence:
  - location: crates/opi-coding-agent/Cargo.toml:77
    detail: "tempfile = { workspace = true }" is under [dev-dependencies], not [dependencies].
  - location: crates/opi-coding-agent/src/execution/contribution.rs:502
    detail: inside #[cfg(unix)] fn bind_launch_material, `#[cfg(not(target_os = "linux"))] let mut snapshot = tempfile::tempfile()?;` — active on macOS, in library (non-test) code.
  - location: CI run 31319356200 jobs 93259723562 / 93259723529 / 93259723490
    detail: "error[E0433]: cannot find module or crate `tempfile` in this scope ... could not compile `opi-coding-agent` (lib)" on x86_64-apple-darwin, aarch64-apple-darwin, and execution_acceptance(macos).
criterion_source: docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md §Repository gates (six target check; macOS release target); C12
reproduction:
  - cargo check --target x86_64-apple-darwin -p opi-coding-agent --all-targets
confidence: high
status: unverified
```

### A2 — MAJOR: four `artifact_audit_script` "reparse seam" tests fail on Linux/macOS (the C16 evidence-truthfulness guard is itself red on unix)

**File:** `crates/opi-coding-agent/tests/artifact_audit_script.rs:1080, 2548, 2563`

`cargo test --workspace --all-targets` on ubuntu/macos fails:
- `phase_exit_audit_rejects_bundle_root_identity_change` (panic `:2563`)
- `phase_exit_audit_rejects_reparse_bundle_root_seam` (panic `:2548`)
- `release_audit_rejects_bundle_root_reparse_and_identity_change_seams` (panic `:1080`)
- `release_audit_rejects_native_scalar_reparse_seams`

These tests guard that re-parsing artifact-audit bundles/sems is deterministic (no identity drift) — the C16 evidence-truthfulness machinery. They pass on Windows and fail on unix.

**Impact:** The suite that is supposed to *reject* bad/overclaimed evidence is itself failing on two of three OSes. Root cause is unconfirmed by this audit (path-separator/normalization in the reparse fixtures is the most likely cause; real reparse non-determinism is the worse possibility). Either way it must be green before merge.

```yaml
id: glm5.2-A2
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Major
title: artifact_audit_script reparse-seam tests fail on Linux/macOS (C16 guard red on unix)
claim: Four tests in tests/artifact_audit_script.rs that assert deterministic reparse of artifact-audit bundles/seams panic on ubuntu-latest and macos-latest CI, so the C16 evidence-truthfulness guard does not pass on unix.
evidence:
  - location: CI run 31319356200 job 93259723470 (test ubuntu)
    detail: "phase_exit_audit_rejects_bundle_root_identity_change ... FAILED" and three siblings; panics at artifact_audit_script.rs:2563/2548/1080; test (macos-latest) also failed.
criterion_source: design §Repository gates (workspace tests pass); C16 (artifact-audit evidence truthfulness)
reproduction:
  - gh run view --job 93259723470 --log | grep FAILED
confidence: high (failure observed); medium (root cause unconfirmed)
status: unverified
```

### A3 — MAJOR: `cancellation_diagnostic_frame_count_is_bounded` fails on Windows

**File:** `crates/opi-coding-agent/tests/execution_protocol_host.rs:183`

`execution_acceptance (windows-latest)`: `Run execution protocol host acceptance` → `test cancellation_diagnostic_frame_count_is_bounded ... FAILED` (52 passed, 1 failed, panic at `execution_protocol_host.rs:183`). ubuntu acceptance passed.

**Impact:** A C6/C7 protocol-host cancellation test is red on a release OS. Root cause unconfirmed (likely a timing/boundedness assertion sensitive to Windows scheduling vs. a real bound defect). Must be green before merge.

```yaml
id: glm5.2-A3
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Major
title: execution_protocol_host cancellation-diagnostic-bound test fails on Windows
claim: tests/execution_protocol_host.rs::cancellation_diagnostic_frame_count_is_bounded fails on windows-latest CI (panic at :183) while passing on ubuntu, so protocol-host cancellation acceptance is red on a release OS.
evidence:
  - location: CI run 31319356200 job 93259723485 (execution_acceptance windows)
    detail: "test cancellation_diagnostic_frame_count_is_bounded ... FAILED ... panicked at crates\opi-coding-agent\tests\execution_protocol_host.rs:183:5 ... 52 passed; 1 failed".
criterion_source: design §Protocol contract / L0 supervision; C6, C7
reproduction:
  - gh run view --job 93259723485 --log
confidence: high (failure observed); medium (root cause unconfirmed)
status: unverified
```

### A4 — MAJOR: remediated Phase 16 is CI-red and 18 commits ahead of origin/main (process gate)

**File:** `.github/workflows/ci.yml`; git history (`origin/main` at `53bc40c`, HEAD at `26613ac`)

HEAD is 18 commits ahead of `origin/main` with the three remediation passes + the workflow refactor never pushed. Triggering CI on this audit's draft PR surfaced A1–A3 plus a clippy failure (M1). The Phase 16 repository gates (C12/C15/C16) are not satisfied on the remediated state.

**Impact:** The branch cannot merge/ship green. This is the umbrella finding under which A1–A3 + M1 sit; it is recorded separately because the *process* (remediation performed without CI validation, then left unpushed) is itself the systemic risk, even though each code defect is isolated.

```yaml
id: glm5.2-A4
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: integration
severity: Major
title: Remediated Phase 16 HEAD is 18 commits ahead of origin/main and CI-red on all three OSes
claim: HEAD (26613ac) is 18 commits ahead of origin/main (53bc40c); running ci.yml on it fails clippy on ubuntu/macos, test on ubuntu/macos, execution_acceptance on windows/macos, and target_check on apple-darwin, so the Phase 16 repository gates are not met.
evidence:
  - location: git rev-list --count origin/main..HEAD
    detail: 18
  - location: CI run 31319356200 (conclusion: failure)
    detail: red jobs = clippy(ubuntu/macos), test(ubuntu/macos), execution_acceptance(windows/macos), Target check(apple-darwin ×2).
criterion_source: design §Repository gates; C12, C16
reproduction:
  - gh run view 31319356200
confidence: high
status: unverified
```

---

## 3. Standards Findings (Matt code-review Standards axis)

### S1 — MINOR: manual `impl Display + Error` instead of thiserror on a library error type

**File:** `crates/opi-coding-agent/src/execution/protocol_host.rs:188-194`

`ExecutionProtocolFailure` hand-implements `Display`/`Error` by delegating to `self.failure`. CLAUDE.md/AGENTS.md "Code quality" states: "Prefer thiserror for library error types… If a file uses thiserror, do not switch to manual impl Display + Error." Every sibling error in this diff (`ExecutionFailure`, `ContributionValidationError`, `ActivationError`, the opi-protocol set) uses thiserror; this is the lone exception. A `#[derive(thiserror::Error)] #[error("{failure}")]` would satisfy it.

```yaml
id: glm5.2-S1
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Minor
title: ExecutionProtocolFailure uses manual impl Display+Error instead of thiserror
claim: crates/opi-coding-agent/src/execution/protocol_host.rs:188-194 manually implements Display/Error for a library error type, contrary to the repo's documented "prefer thiserror for library error types" rule and the module's own thiserror convention.
evidence:
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:188-194
    detail: "impl std::fmt::Display … impl std::error::Error {}" hand-rolled, delegating to self.failure.
criterion_source: CLAUDE.md / AGENTS.md "Code quality" (prefer thiserror; match module style)
reproduction:
  - grep -n "impl std::fmt::Display for ExecutionProtocolFailure" crates/opi-coding-agent/src/execution/protocol_host.rs
confidence: high
status: unverified
```

### S2 — MINOR (smell): teardown bundle duplicated across six error arms (Shotgun Surgery / Data Clump)

`protocol_host.rs::execute` calls `terminate_and_fail(child, guard, stderr_handle, stdin, <code>, hard_deadline).await` at lines 276-284, 369-377, 402-410, 523-530, 551-558, 582-590 (plus the `terminate_failed_transmission` sibling at 1268). The same five-handle bundle threads through every error arm. Bundling into one `BackendHandles` struct would collapse it. Judgement call.

### S3 — MINOR (smell): `#[allow(clippy::too_many_arguments)]` data clumps

`finalize_terminal` (13 params, `protocol_host.rs:995`) and `finish_with_cancel` (16 params, `:1113`); `runtime.rs:306 ExecutionRuntime::build` (11 params). The accumulators + handle bundle travel together. `too_many_arguments` is endemic in this crate (harness/rpc), which partly endorses the pattern, but the execution core is greenfield and could bundle.

### S4 — MINOR (smell): duplicated `CLEANUP_REPORT_GRACE` literal

`Duration::from_millis(1500)` exists at both `protocol_host.rs:53` and `runtime.rs:168`. The runtime.rs comment justifies the mirror on phase-ownership grounds ("owned by 16.7, not edited here"), but two literals can drift; a `pub(crate)` re-export from `failure.rs` would be safer.

### S6 — INFO (smell): repeated switches over the 14-variant failure enum

`failure.rs:140-157 code()`, `:162-252 remediation()`, `diagnostic_bridge.rs`, and `runtime.rs:777` all switch on the same closed set. Inherent to the closed-enum design; noted for completeness.

*(S5, "test-seam `routed_store_factory_override` ships in release", was REFUTED — see §9.)*

---

## 4. Spec Findings (Matt code-review Spec axis)

**Net: all 11 spec areas (five gates, routing, permission, protocol host, fail-closed, contribution gates, minimal runtime, opi-sandbox standalone, crate boundaries, migration, CI) verify as IMPLEMENTED.** Notably robust:
- **TOCTOU-safe SHA-256**: `contribution.rs:413-543` opens with `O_NOFOLLOW`, copies into a sealed memfd (Linux) / private fd (macOS) / `FILE_SHARE_READ`-only handle (Windows), hashes the snapshot, spawns from `/proc/self/fd/{fd}`, and revalidates per-spawn (`package_activation.rs`).
- **Minimal Runtime**: `harness.rs` constructs `LocalBashOperations` directly with zero router/permission/protocol/store state; default bash schema byte-identical (pinned by `execution_minimal_runtime.rs`).
- **Five gates + non-TTY enable refusal**, drift fail-closed, project-layer `[execution.permissions]` rejected unconditionally, session grants memory-only and reset on resume/fork/branch.

### Spec-1 — EVALUATED, REFUTED: claimed C5 "no-degraded-success" violation in the cancel-grace race

The Spec agent flagged `finish_with_cancel` (`protocol_host.rs:1182-1208`) returning `Ok(CompletedOutcome{ exit: Some(0), timed_out: true })` when a clean `Completed` races the cancel grace, arguing a cancelled/timed-out command is reported as success. **Refuted by tracing through the tool layer**: `completed_outcome_to_bash_result` (`runtime.rs:693`) carries `exit_code: Some(0)` with `timed_out` only in the operation-context diagnostic, but `bash.rs:269` computes `let is_error = timed_out || cancelled || signal.is_some() || exit_code != Some(0)`, so the production **tool result** is `is_error = true`. C5 holds at the layer that matters. (Residual: the intermediate `BashResult` carries `exit_code:Some(0)`; a non-`BashTool` consumer that gates on `exit_code` alone could misread it — Info, not a C5 violation.) Recorded for transparency; **not a defect**.

---

## 5. Correctness / Test-quality / Invariants / Integration Findings (opi-dimensions workflow, adversarially verified)

*(6 finder agents + 6 skeptic verifiers; 8 findings, all confirmed/Info, none Blocker/Major in the code itself.)*

### D1 — MINOR (C16): source-text structural pin tests reproduce the repo's known degenerate-test pattern (3 sites)

**File:** `crates/opi-coding-agent/tests/execution_protocol_host.rs:1196` (+ 2 more sites)

Three Phase 16 tests assert invariants by matching substrings of production source (`src.contains("…")` / `include_str!`) rather than by driving behavior — the exact pattern the repo documents as a prior failure mode. They are honestly scoped as secondary guards and the real behavioral coverage exists alongside, but the pattern is fragile (a rename passes the test while breaking nothing it checks). Confirmed by the skeptic.

### D2 — INFO (C7/C16): external protocol-host dropped-future reap is argued by transitivity, not a direct drop-the-future test

**File:** `crates/opi-coding-agent/tests/execution_protocol_host.rs:1123`

The C7 invariant "dropping the execution future kills child AND descendants" is directly tested for the local bash future and the AdapterHost, but for the external `ExecutionProtocolHost::execute` future it is argued by transitivity from the cancel path (same `TreeGuard`). Sound, but a direct drop-the-future test would close the gap.

### D3 — INFO (C4): backend-diagnostic redaction is defense-in-depth; no canary test for backend-injected diagnostics

**File:** `crates/opi-coding-agent/src/execution/protocol_host.rs:644`

A trusted backend emitting a `Diagnostic{message}` with command text / non-secret env substrings would have secret/absolute-path substrings scrubbed by `redact_text`, but residual command text / non-secret env substrings can survive into the retained diagnostics. This matches the spec's "backend diagnostics are message-only; redaction is the backend's responsibility per the v1 wire" contract, so it is not a violation — but there is no canary test injecting a hostile backend diagnostic to prove the scrubbing that *does* happen. Confirmed.

### D4 — MINOR (C9): Landlock TCP bind/connect layer is wired but never independently exercised — seccomp `socket()` gate masks it in every network=deny probe

**File:** `crates/opi-sandbox/tests/linux_policy.rs:441`; build path `crates/opi-sandbox/src/platform/linux.rs:234-240, 300-304`

The C9-required Landlock TCP bind/connect confinement is built, wired, and fail-closed-gated on ABI ≥ 4, but no test can observe its *independent* enforcement: in every network=deny end-to-end probe the seccomp `socket()`-creation gate fires first (the probe's `TcpListener::bind` calls `socket(AF_INET,…)` before `bind()`), so the Landlock-TCP-policed path is never reached. A regression disabling Landlock TCP bind/connect would not fail any test. The overall network=deny contract still holds because seccomp holds the line, and the Landlock **fs** layer *is* independently proven (`outside_write_denied`/`workspace_write_allowed`). Confirmed; defense-in-depth coverage gap, not a correctness defect.

### D5 — INFO (C11/C4): Windows bootstrap exposes `OPI_SANDBOX_*` control env vars to the target

**File:** `crates/opi-sandbox/src/runner.rs:1349`

On Windows the gated bootstrap passes the target program, per-index args, the release-gate path, and the backend PID to the PowerShell bootstrap via `OPI_SANDBOX_*` env vars, which the target then inherits (even with `env_inherit=Clear`). Acceptable under the documented Windows posture (L0-only, no confinement claim, target runs only after the release gate is removed) — but the control metadata is visible to the target. Re-evaluate if Windows confinement is ever added. Confirmed.

### D6 — INFO (C12/C16): `release.yml` marks `aarch64-pc-windows-msvc` tier-2 with `continue-on-error`, making the six-target release best-effort for that leg

**File:** `.github/workflows/release.yml:60`

`continue-on-error: ${{ matrix.tier2 == true }}` for the arm64-windows target means a build break on that leg omits the artifact and the build job still succeeds — the release can publish five instead of six artifacts without failing. Defensible as tier-2 policy, but it weakens the "six-target" claim for that one leg.

### D7 — INFO (C2): Windows absolute path (`C:\foo`) is classified `DriveRelativeCommand` in the contribution validator

**File:** `crates/opi-coding-agent/src/execution/contribution.rs:564`

`validate_command_path` maps `Component::Prefix` to `DriveRelativeCommand`, so a Windows absolute path like `C:\foo` (Prefix + RootDir + Normal) is reported as "drive-relative" rather than "absolute" — a misleading error variant/message, though the path is **correctly rejected**. Cosmetic; both shapes are rejected.

---

## 6. Minor tooling finding

### M1 — MINOR: new clippy lint `manual_is_multiple_of` denied by `-D warnings` on linux/macos

**File:** `crates/opi-sandbox/src/platform/linux.rs:473` (`if relative % 8 == 0`) and one sibling site

CI `clippy` runs on a newer `stable` than the code was written against; the `manual_is_multiple_of` lint fires (2 sites) and `-D warnings` turns it into an error. Tooling-version drift, not a logic defect. Mechanical fix: `relative.is_multiple_of(8)`. (Observed in CI run 31319356200, `clippy (ubuntu-latest)`/`(macos-latest)`.)

---

## 7. Invariant Verification (C1–C16)

| Criterion | Status | Code evidence | Test / native evidence |
|---|---|---|---|
| C1 Minimal Runtime | PASS | `harness.rs` direct `LocalBashOperations`; default schema byte-identical | `execution_minimal_runtime.rs` (schema pin); construction_probe counts |
| C2 Five independent gates | PASS | `package_activation.rs`, `contribution.rs` | `execution_package_lifecycle.rs`, `package_store.rs` (D7 cosmetic only) |
| C3 Routing/permission | PASS | `router.rs`, `permission.rs` | `execution_routing.rs`, `execution_selected_routing.rs`, `execution_permission.rs` |
| C4 Redacted failure envelope | PASS | `failure.rs`, `protocol_host.rs:644` | `execution_failures.rs`; D3 (no backend-diagnostic canary — Info) |
| C5 No degraded success / no fallback | PASS | `protocol_host.rs` cancel path; `runtime.rs` no-fallback; **`bash.rs:269` is_error** | Spec-1 refuted; traced end-to-end |
| C6 Protocol contract | PASS (win test red — A3) | `protocol_host.rs`, opi-protocol `v1` | `execution_protocol_host.rs`; `execution_v1_contract.rs`; A3 (1 win test fails) |
| C7 L0 supervision | PASS | `process_tree.rs`, `TreeGuard` | `sandbox_l0.rs`; D2 (drop-future by transitivity — Info) |
| C8 opi-sandbox SDK+CLI | PASS | `runner.rs`, `cli.rs`, `backend.rs` | **WSL2 native**: `--version`, `doctor --json`, `run` exit mapping all verified |
| C9 Linux native | PASS | `platform/linux.rs` (Landlock+seccomp) | **WSL2 native**: `cargo test -p opi-sandbox` RC=0; doctor reports landlock+seccomp; **outside-write denied at kernel level**; D4 (Landlock-TCP not independently exercised — Minor) |
| C10 macOS native | PASS | `platform/macos.rs` (sandbox-exec) | **GHA `opi-sandbox package (macos-latest)` → SUCCESS** |
| C11 Windows posture | PASS | `platform/windows.rs` (Job Objects L0) | `windows_execution_posture.rs`; D5 (env-var exposure — Info) |
| C12 Six-target CI | **FAIL** | ci.yml topology present | **CI red**: apple-darwin target_check (A1), clippy linux/macos (M1) |
| C13 Migration (no aliases) | PASS | `config.rs` `LegacySandboxSection`; no `--sandbox` Arg | grep confirms; `execution_migration.rs` |
| C14 Crate boundaries | PASS | opi-sandbox → opi-protocol only; opi-coding-agent ≠deps opi-sandbox | `crate_boundaries.rs` |
| C15 Doc lockstep EN/ZH | PASS | README/README.zh, opi-spec/opi-spec.zh | 16.16.3 docs task; `docs_contract` CI green |
| C16 Artifact-audit evidence | **AT RISK** | `artifact_audit_script.rs` | **A2: the guard itself fails on unix**; M1 clippy |

---

## 8. Native Verification Evidence (real execution, not stubs)

**WSL2 Linux (kernel 6.18 → real seccomp + Landlock):**
- `cargo test -p opi-protocol` → **RC=0**
- `cargo test -p opi-sandbox` → **RC=0** (includes `linux_policy.rs` exercising real Landlock fs + seccomp)
- `doctor --json` → `{"supported":true,"target":"linux","mechanisms":["landlock","seccomp"],"profiles":["workspace-write"],…}`
- Direct CLI `run`: in-workspace write **allowed** (rc 0); **outside-workspace write denied at the kernel** (`/bin/sh: cannot create /tmp/…: Permission denied`) — genuine Landlock restriction engaged, not a stub; `network=allow` control succeeded. (The `network=deny` ad-hoc probe was inconclusive — `/dev/tcp` is bash-only and the probe used dash — but `linux_policy.rs` covers network denial.)

**GHA macOS (run 31319356200):**
- `opi-sandbox package (macos-latest)` → **SUCCESS** (release build → native archive → verify → standalone smoke on the extracted binary). **C10 confirmed.**
- `opi-sandbox package (ubuntu-latest)` → **SUCCESS**.

---

## 9. Refuted Findings (adversarial-verification value)

- **Standards S5** ("test-seam `routed_store_factory_override` ships in release") — **REFUTED**. Both the call site (`harness.rs:203`) and the module definition (`harness.rs:260`) carry `#[cfg(test)]`; the seam does not compile into release.
- **Spec-1** (C5 "no-degraded-success" violated by cancel-grace clean-exit race) — **REFUTED**. The agent stopped at the `BashResult` intermediate; the production tool-result path (`bash.rs:269` `is_error = timed_out || …`) makes it an error regardless of `exit_code`. C5 holds.

Both were independently refuted by tracing the actual code paths rather than trusting the agent summaries.

---

## 10. Residuals and Recommendations

### Priority recommendations
1. **Fix A1 before any push/merge** — move `tempfile` to `[dependencies]` in `crates/opi-coding-agent/Cargo.toml`. This unblocks macOS/`apple-darwin` compilation. Highest single-action value; prior audits missed it because it is macOS-only and the dev host is Windows.
2. **Triage A2 and A3** — root-cause the four `artifact_audit_script` reparse failures on unix and the Windows `cancellation_diagnostic_frame_count_is_bounded` failure. Determine whether they are test defects (path normalization / timing) or real; either way they must be green.
3. **Fix M1** — `relative.is_multiple_of(8)` at `linux.rs:473` (+ sibling); or pin the CI clippy baseline.
4. **Push and require green CI before merge** (A4) — the 18-commit unpushed remediation accrued CI debt that this audit had to surface by triggering a draft PR. The branch should not merge until A1–A3 + M1 are resolved and `ci.yml` is green on all six targets.
5. **Close the temporary audit PR #3 and delete branch `audit-phase16-native`** after the evidence above is no longer needed (it was opened solely to trigger CI for this audit).

### Lower-priority (Minor/Info)
- Replace the 3 source-text structural pin tests (D1) with behavioral assertions, or document them strictly as supplemental.
- Add a Landlock-TCP isolated test (D4) and a backend-diagnostic redaction canary (D3) when the native test harness is next touched.
- Re-export `CLEANUP_REPORT_GRACE` (S4) and consider a `BackendHandles` bundle (S2/S3) to reduce remediation-churn debt in the protocol host.
- Switch `ExecutionProtocolFailure` to thiserror (S1).

### Notable strengths
- The fail-closed protocol state machine, the no-fallback/no-degraded-success guarantees, redaction, TOCTOU-safe contribution validation, deterministic routing, and crate boundaries are all spec-faithful and well-tested at the code level (0 code-level Majors).
- Native restriction is genuinely enforced (real Landlock outside-write denial; real seccomp; real macOS sandbox-exec via GHA), not stubbed.
