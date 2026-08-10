# Phase 16 Pluggable Extensions and Command Execution -- Independent Code Audit

**Auditor**: glm5.2 (independent, fresh-context re-audit; no prior audit report consulted)
**Date**: 2026-08-10
**Scope**: Phase 16 registered requirements (16 exit criteria C1--C16, 21 tasks 16.1--16.16.3, scenarios SC16-01..SC16-15b) and the complete relevant implementation at `audit_head`
**Implementation target**: `c5de89216b316529d1c8c1c182fe496a3103f42f` (current committed implementation; working tree clean, so working copies equal committed objects)
**Phase exit commit**: `f8aff02` (last task 16.16.3 `verified_at_commit`; provenance only)
**History use**: provenance and discovery only; no diff coverage boundary
**Independence**: fresh-context-same-family (the auditor and the prior `audit.glm5.2.md` are the same model family; this run is a fresh, uncontaminated re-audit)
**Method**: 6-reader -> 8-dimension adversarial-verify workflow (63 agents: 6 deep-readers by file group -> 8 dimension reviewers each reading committed source + maps -> per-finding refute-default verification reading the cited code), followed by the auditor's own independent re-verification of every headline and unverified finding against committed code at `c5de892`.

## Contamination disclosure

Two prior Phase 16 audits are committed at `audit_head` (`audit.codex.md`, `audit.glm5.2.md`) plus `remediation-plan.md`. **None was read.** The prior `audit.glm5.2.md` working-tree copy was removed solely to let this report be written fresh; it remains recoverable via git. The auditor's auto-loaded memory index references that a prior re-audit existed and was PASS-WITH-FINDINGS with CI-integration Majors; that one-line index entry was treated as background only, and every conclusion below is derived independently from source/tests/spec/config/CI at `c5de892`. A prior "tempfile dev-dep used in lib" concern referenced by memory was independently re-checked and is **resolved** (`tempfile` is correctly under `[dependencies]` at `crates/opi-coding-agent/Cargo.toml:62`; `contribution.rs:502` is production code backed by that dependency).

---

## 1. Executive Summary

**Verdict: PASS-WITH-FINDINGS**

| Severity | Count |
|----------|-------|
| Blocker  | 0     |
| Major    | 1     |
| Minor    | 6     |
| Info     | 18    |

Phase 16 is a large, well-structured, heavily-tested implementation. After a 63-agent deep audit, independent re-verification of the headline and unverified findings, and an independent cross-check of the divergent findings raised by a concurrent `codex` audit (see Appendix A), **no Blocker was found and one Major defect was confirmed.** The five-gate lifecycle, fail-closed no-fallback invariant, the 14-code redacted failure envelope, the crate-boundary isolation between `opi-coding-agent` and `opi-sandbox`, the protocol state machine, and the Phase 15 migration (rejection without aliases) all hold in the committed code.

The single **Major (4.3)** is an L0 process-tree-cleanup gap for external adapters: the host's Unix tree-kill is process-group-scoped (`kill(-backend_pgid)`), but the backend places its target in a *separate* process group (`process_group(0)`), so when an unresponsive backend is SIGKILL'd past the cancel grace period the target descendant is not reached and the backend's own kill-on-drop guard cannot fire (SIGKILL bypasses `Drop`). The system reports this honestly via `cleanup_unconfirmed`, but the L0 "kill child **and descendants**" guarantee (C7) is not mechanically enforced for external-adapter targets. This was surfaced by the concurrent codex audit and **independently confirmed** by reading the committed spawn/kill code; codex's other headline Major (an exact-cap CRLF line-reader divergence) was **independently refuted** at `c5de892` (the two readers agree).

The remaining findings are low-risk and mostly fail-safe: the most substantive Minor (3.1) is a manifest-hash normalization inconsistency that can spuriously disable a package under a Windows CRLF re-checkout, but it fails closed and never spawns an untrusted/wrong target. Several Info findings are documented design trade-offs (crate-boundary-driven code duplication, Windows as a permanently-unsupported restriction platform, env-var confidentiality as an explicit Non-Goal). Two CI/hygiene residuals are worth noting before release: the audited `HEAD` is 21 commits ahead of `origin/main`.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 16.1  | Pin the Phase 16 documentation contract | PASS |
| 16.2  | Pin L0 supervision and define the policy-neutral seam | FAIL (4.3 -- L0 descendant-kill guarantee not enforced for external-adapter targets) |
| 16.3  | Add opi-protocol::execution::v1 | PASS |
| 16.4  | Parse and hard-gate executable contributions | PASS |
| 16.5  | Add Package Trust and enable/disable lifecycle | PASS-WITH-MINOR (3.1 manifest-hash inconsistency on the resolver lock path) |
| 16.6  | Add execution configuration, failures, routing, and permission policy | PASS-WITH-MINOR (3.2 startup vs doctor diagnostic-code surface) |
| 16.7  | Implement the one-shot execution protocol host | FAIL (4.3 -- host pgrp-kill cannot reach the separately-grouped adapter target) |
| 16.8  | Build the deep Execution Runtime assembly | PASS |
| 16.9  | Wire Execution Runtime, dynamic bash schema, and public surfaces | PASS |
| 16.10 | Add the interactive permission broker and TUI prompt | PASS |
| 16.11.1 | Build the standalone opi-sandbox SDK and runner | PASS-WITH-MINOR (9.1 TMPDIR override) |
| 16.11.2 | Build the human opi-sandbox CLI and direct smoke | PASS |
| 16.12 | Add the atomic helper gate and protocol backend | PASS |
| 16.13 | Port the Linux native restriction contract | PASS |
| 16.14.1 | Port the macOS native restriction contract | PASS |
| 16.14.2 | Pin the Windows unsupported execution posture | PASS |
| 16.15.1 | Build host-neutral opi-sandbox packaging | PASS |
| 16.15.2 | Wire native package CI, release, and artifact audit | PASS-WITH-MINOR (9.3 unpushed HEAD) |
| 16.16.1 | Remove core native sandbox and enforce migration boundaries | PASS |
| 16.16.2 | Prove install-to-execute and cross-surface diagnostics | PASS-WITH-MINOR (3.2 cross-surface code) |
| 16.16.3 | Synchronize documentation and close Phase 16 repository gates | PASS |

---

## 2. Standards Findings

### 2.1 INFO: Windows Job-Object FFI duplicated across opi-coding-agent and opi-sandbox

**File:** `crates/opi-coding-agent/src/tool/process_tree.rs:449-565, 65-109`; `crates/opi-sandbox/src/process_tree.rs:311-391, 115-143`
**Cause:** ~150 lines of Windows Job-Object FFI (`CreateJobObjectW` + `SetInformationJobObject` with `KILL_ON_JOB_CLOSE`, `AssignProcessToJobObject`, `TerminateJobObject`, `CloseHandle`) and the `Toolhelp32Snapshot` thread-resume loop are near-duplicated between the two crates' `process_tree.rs`. The two diverge only in their error enum and the coding-agent's `terminate_with` test-injection seam.
**Impact:** A Fowler Duplicated Code smell. It is a **documented architectural override**: the Phase 16 crate-boundary requirement mandates `opi-coding-agent` MUST NOT depend on `opi-sandbox` and `opi-sandbox` depends only on `opi-protocol`. Removing the duplication would require a third shared crate. Both `phase16_crate_boundaries.rs` and `opi-sandbox/tests/crate_boundaries.rs` prove (via `cargo tree`) that no sharing edge exists.
**Fix:** None required. If desired later, extract `opi-process-tree` as a standalone crate consumed by both.

```yaml
id: P16-standards-01
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Info
title: Windows Job-Object FFI duplicated across opi-coding-agent and opi-sandbox
claim: ~150 lines of Windows Job-Object FFI and thread-resume logic are near-duplicated between the two crates' process_tree.rs, removable only via a third shared crate.
evidence:
  - location: crates/opi-coding-agent/src/tool/process_tree.rs:449-565
    detail: JobGuard new/assign/terminate/Drop + resume_child mirror opi-sandbox/src/process_tree.rs:311-391,115-143
  - location: phase16_crate_boundaries.rs + opi-sandbox/tests/crate_boundaries.rs
    detail: cargo tree proves no opi-coding-agent -> opi-sandbox edge, so the duplication cannot be removed without a third crate
criterion_source: Phase 16 CRATE BOUNDARIES (documented override of the Fowler Duplicated Code smell)
reproduction: []
confidence: high
status: unverified
```

### 2.2 INFO: opi-sandbox has no README; opi-protocol README is EN-only

**File:** `crates/opi-sandbox/Cargo.toml`; `crates/opi-protocol/README.md`
**Cause:** `opi-sandbox` has no README at all, and `opi-protocol/README.md` has no `.zh.md` counterpart.
**Impact:** CLAUDE.md's localized-counterpart lockstep rule ("update the localized counterpart in the same change") is not satisfied for these two new standalone crates. Defensible for freshly-published standalone crates, but the divergence from the repo-wide EN/ZH lockstep convention is worth a deliberate decision.
**Fix:** Add `crates/opi-sandbox/README{,.zh}.md` and `crates/opi-protocol/README.zh.md`, or document the standalone-crate exception.

```yaml
id: P16-standards-02
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Info
title: opi-sandbox has no README and opi-protocol README is EN-only
claim: Two new standalone crates lack the repo's EN+ZH README lockstep.
evidence:
  - location: crates/opi-sandbox/Cargo.toml
    detail: no README field / no README.md present
  - location: crates/opi-protocol/README.md
    detail: EN-only; no README.zh.md
criterion_source: CLAUDE.md localized-counterpart lockstep rule
reproduction: []
confidence: high
status: unverified
```

---

## 3. Spec Findings

### 3.1 MINOR: Two manifest-hash normalization bases for the same package.toml (raw vs LF-normalized)

**File:** `crates/opi-coding-agent/src/package_resolver.rs:210-214, 365-400`; `crates/opi-coding-agent/src/execution/contribution.rs:277, 631-644`; `crates/opi-coding-agent/src/package_store.rs:308`
**Cause:** The same conceptual quantity (a SHA-256 over `package.toml`) is computed with two incompatible normalization bases, and both are persisted in the same `package-lock.toml`. The resolver-level `PackageLockEntry.manifest_sha256` is SHA-256 over **raw** bytes (`package_resolver.rs:210-214` `sha2::Sha256::digest(&bytes)` on `std::fs::read`), and the runtime `lock_drifted` check compares raw-to-raw (`:388`). The contribution/trust-level `LockMaterial.manifest_hash` is SHA-256 over **LF-normalized** bytes (`contribution.rs:277` `sha256_hex(&lf_normalize(raw_manifest_bytes))`), where `lf_normalize` (`:631-644`) strips CRLF->LF with a doc comment "CRLF -> LF so the manifest hash is reproducible across checkouts (git may materialize LF as CRLF on Windows)." The contribution path deliberately anticipated CRLF; the resolver path did not.
**Impact:** Under a CRLF<->LF flip (Windows `git autocrlf` re-checkout), the resolver's `lock_drifted` fires and **disables the package at runtime** (returns `None`, excluding it from enabled identities) with a confusing "manifest hash does not match the lock file" diagnostic, while the contribution pre-spawn revalidation path (LF-normalized) would consider the locked material unchanged. This is a false-disable of a valid, semantically-unchanged package. It **fails safe** (disable, not a wrong-target spawn) and the security-critical pre-spawn trust path uses the robust LF-normalized hash, so it is not a trust bypass. Narrow surface (Windows + autocrlf + re-checkout).
**Fix:** Make `package_resolver::manifest_sha256` use the same `lf_normalize` basis as `contribution.rs` (or unify the two hash computations behind one helper), so both persisted hashes agree across line-ending flips. Add a CRLF-stability test for the resolver lock path mirroring the contribution path's coverage.

```yaml
id: P16-spec-01
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Minor
title: Resolver manifest hash uses raw bytes while activation trust uses LF-normalized
claim: The resolver persists and runtime-compares a raw-byte manifest SHA-256 while the contribution/trust path uses an LF-normalized SHA-256, so a CRLF line-ending flip spuriously disables an otherwise-valid package via lock_drifted.
evidence:
  - location: crates/opi-coding-agent/src/package_resolver.rs:210-214
    detail: manifest_sha256 hashes raw std::fs::read bytes with no normalization
  - location: crates/opi-coding-agent/src/package_resolver.rs:388
    detail: lock_drifted compares lock.manifest_sha256 != actual_hash (raw vs raw), disabling the package on mismatch
  - location: crates/opi-coding-agent/src/execution/contribution.rs:277,631-644
    detail: LockMaterial.manifest_hash = sha256_hex(lf_normalize(raw)); lf_normalize strips CRLF->LF for checkout reproducibility
criterion_source: PACKAGE LIFECYCLE ("If the manifest, lock, or executable changes, Package Trust no longer matches"); consistency of the two lock systems
reproduction:
  - Install an execution package whose package.toml is materialized with CRLF on a Windows autocrlf checkout; flip to LF (or re-checkout under a different autocrlf) and observe lock_drifted disables the package at runtime while the contribution revalidation path treats it as unchanged.
confidence: high
status: unverified
```

### 3.2 MINOR: Startup diagnostic surface emits a 15th envelope code while doctor uses the stable 14-code directly

**File:** `crates/opi-coding-agent/src/diagnostic_bridge.rs:301-319` vs `:325-338`
**Cause:** `diagnostic_from_execution_failure` (the startup/`ExecutionRuntime::build` surface across text/NDJSON/RPC) sets the `Diagnostic.code` to `CODE_ADAPTER_STARTUP_FAILED` ("adapter_startup_failed", a 15th envelope code) and nests the stable `failure.code()` (one of the 14) inside `details.code`. `diagnostic_from_execution_package_failure` (the doctor surface) sets the `Diagnostic.code` **directly** to the stable 14-code. Both preserve remediation and the stable code is present in `details.code` for both, but the top-level `Diagnostic.code` differs across surfaces.
**Impact:** An embedder matching on `Diagnostic.code` for the stable 14 codes would catch a doctor/package-doctor failure (e.g. `package_not_installed`) but miss the same logical failure at startup (where `.code` is `adapter_startup_failed`). The spec says the surfaces "preserve the same codes." This is a cross-surface consistency gap, not a redaction or correctness break.
**Fix:** Either expose the stable 14-code as the startup `Diagnostic.code` (nesting the startup envelope code in `details`), or document that startup-tool-construction failures carry a distinct envelope code with the stable code in `details.code`, and assert the convention in a test.

```yaml
id: P16-spec-02
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Minor
title: Startup surface uses a 15th envelope code as Diagnostic.code; doctor uses the stable 14-code directly
claim: The startup execution-failure surface emits CODE_ADAPTER_STARTUP_FAILED as the top-level Diagnostic.code (stable code nested in details.code), while the doctor surface emits the stable 14-code directly as Diagnostic.code, so the code field is not uniform across surfaces.
evidence:
  - location: crates/opi-coding-agent/src/diagnostic_bridge.rs:301-319
    detail: diagnostic_from_execution_failure sets Diagnostic.code=CODE_ADAPTER_STARTUP_FAILED; failure.code() goes into details.code
  - location: crates/opi-coding-agent/src/diagnostic_bridge.rs:335
    detail: diagnostic_from_execution_package_failure sets Diagnostic.code = failure.code() (stable 14-code) directly
criterion_source: FAILURE & DIAGNOSTICS ("Text, TUI, NDJSON, RPC, package doctor, and top-level doctor preserve the same codes")
reproduction: []
confidence: high
status: unverified
```

### 3.3 INFO: validate_rules enforces catch-all ordering for all strategies

**File:** `crates/opi-coding-agent/src/config.rs:1217-1264`
**Cause:** `validate_rules` enforces the catch-all ordering invariant (exactly one final catch-all, last) for **all** strategies, so a malformed `[[execution.rules]]` table is rejected even when the selected strategy is `fixed` or `model` and the rules are unused.
**Impact:** Over-eager validation. A user with a stray malformed rule table and `strategy = "fixed"` gets a config error for a table that has no effect. Defensible (fail-fast on malformed config) and arguably desirable, but stricter than the spec requires.
**Fix:** Optional -- skip rules validation when the selected strategy cannot consume rules, or document the eager-validation choice.

```yaml
id: P16-spec-03
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Info
title: validate_rules enforces catch-all ordering for all strategies, even when rules routing is unselected
claim: Malformed execution.rules tables are rejected under fixed/model strategies where rules are never consulted.
evidence:
  - location: crates/opi-coding-agent/src/config.rs:1217-1264
    detail: validate_rules runs unconditionally of the selected strategy
criterion_source: ROUTING (rules require exactly one final catch-all last) -- applied more eagerly than required
reproduction: []
confidence: high
status: unverified
```

### 3.4 INFO: install unconditionally resets trusted+enabled even for byte-identical re-add

**File:** `crates/opi-coding-agent/src/package_activation.rs:466-501`
**Cause:** `PackageActivationStore::install` unconditionally writes a fresh disabled+untrusted record, resetting any prior trust/enablement even when the re-added bytes are identical to the installed artifact. The CLI layer re-adds only when content changed, so the reset is normally masked, but a direct store-level re-add would discard trust silently.
**Impact:** The store layer does not preserve trust across a byte-identical re-add; reliance on the CLI layer to gate this is a minor layering assumption. Fail-safe (trust is only ever dropped, never falsely granted).
**Fix:** Optional -- have `install` short-circuit to a no-op (or preserve trust) when the locked material is byte-identical, so the store layer is self-consistent without the CLI guard.

```yaml
id: P16-spec-04
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Info
title: install resets trusted+enabled even for byte-identical re-add (CLI layer preserves trust)
claim: The store-level install unconditionally resets trust/enablement on re-add; only the CLI layer prevents byte-identical re-adds from discarding trust.
evidence:
  - location: crates/opi-coding-agent/src/package_activation.rs:466-501
    detail: install writes a fresh untrusted+disabled record without comparing to prior locked material
criterion_source: PACKAGE LIFECYCLE (disable preserves trust; trust binds exact material)
reproduction: []
confidence: high
status: unverified
```

---

## 4. Correctness Findings

### 4.1 MINOR: Bounds::validate() skips the max_diagnostics_size cross-check against max_line_size

**File:** `crates/opi-protocol/src/execution/v1/bounds.rs:57-83` (esp. `:80-81`)
**Cause:** `Bounds::validate()` cross-checks `max_line_size` against the base64-inflated chunk (`LineTooSmallForChunk`) and against `max_configuration_size` (`LineTooSmallForConfig`), but explicitly discards `max_diagnostics_size` and `max_cumulative_output` (`let _ = self.max_diagnostics_size;`). If a custom `Bounds` set `max_diagnostics_size` larger than `max_line_size - framing`, a serialized diagnostic frame could not actually be delivered (the line reader would reject it), yet `validate()` would return `Ok(())`.
**Impact:** A declared bound (`max_diagnostics_size`) that `validate()` does not make realizable. It **fails safe** -- the line reader still bounds any frame at `max_line_size`, so there is no unboundedness -- and the shipped `Bounds::DEFAULT` (64 KB diagnostics vs 2 MB line) is unaffected. The gap is validation completeness, not a live misbehavior.
**Fix:** Add a `max_line_size >= max_diagnostics_size + framing` check to `validate()` (a `LineTooSmallForDiagnostics` variant), mirroring the configuration check.

```yaml
id: P16-correctness-01
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: invariants
severity: Minor
title: Bounds::validate() skips the max_diagnostics_size cross-check against max_line_size
claim: validate() certifies a Bounds as consistent without ensuring max_diagnostics_size fits under max_line_size, so a declared bound can be unrealizable.
evidence:
  - location: crates/opi-protocol/src/execution/v1/bounds.rs:80-81
    detail: "let _ = self.max_diagnostics_size; let _ = self.max_cumulative_output;" -- diagnostics size never compared to max_line_size
  - location: crates/opi-protocol/src/execution/v1/bounds.rs:70-79
    detail: chunk and configuration ARE cross-checked against max_line_size; diagnostics is not
criterion_source: PROTOCOL (frames are bounded); INVARIANT #8 (bounds are internally consistent)
reproduction: []
confidence: high
status: unverified
```

### 4.2 MINOR: CompletedPayload exit/signal not enforced mutually exclusive or at-least-one

**File:** `crates/opi-protocol/src/execution/v1/frames.rs:254-270`
**Cause:** `CompletedPayload` carries `exit: Option<u32>` and `signal: Option<u32>` as independent options with no substrate-level validation. The codec accepts a `completed` frame carrying both `Some(exit)` and `Some(signal)`, or neither. The spec says "`completed` is terminal and reports exit/signal."
**Impact:** A non-conformant backend could send a `completed` with both or neither set, and the protocol types would deserialize it without objection; correctness then depends entirely on the host/runtime interpreting the combination. There is no unboundedness or crash, but the substrate does not enforce the documented terminal contract. The host path maps these into `ExecutionFailure`/results conservatively, so a live defect requires a misbehaving backend.
**Fix:** Add a validation (in the codec or a `CompletedPayload::validate`) requiring exactly one of exit/signal be set on a non-timed-out/non-cancelled completion, or document that the host treats ambiguity as failure.

```yaml
id: P16-correctness-02
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: correctness
severity: Minor
title: CompletedPayload exit/signal not enforced mutually exclusive or at-least-one
claim: The protocol substrate accepts a completed frame with both exit and signal set, or neither, without enforcing the documented terminal exit/signal contract.
evidence:
  - location: crates/opi-protocol/src/execution/v1/frames.rs:254-270
    detail: exit: Option<u32> and signal: Option<u32> are independent; no validate enforces mutual exclusivity or at-least-one
criterion_source: PROTOCOL ("completed is terminal and reports exit/signal")
reproduction: []
confidence: high
status: unverified
```

### 4.3 MAJOR: Host L0 tree-kill is pgrp-scoped and cannot reach the external adapter's separately-grouped target

**File:** `crates/opi-coding-agent/src/tool/process_tree.rs:381` (`libc::kill(-*pgid, libc::SIGKILL)`); `crates/opi-coding-agent/src/execution/protocol_host.rs:230` (host `configure_tree`); `crates/opi-sandbox/src/process_tree.rs:92` and `crates/opi-coding-agent/src/tool/process_tree.rs:51` (`process_group(0)`)
**Cause:** Both the host (spawning the backend) and the backend (spawning the target) call `configure_tree`, which on Unix runs `cmd.process_group(0)` -- each child is placed in a **brand-new process group** (`pgid == child pid`). The host's `TreeGuard::terminate` then tears down the backend with `libc::kill(-backend_pgid, SIGKILL)` (a process-group kill). Because the target lives in its **own** process group (`-target_pgid`, not `-backend_pgid`), the host's kill does not reach it. The only thing that would kill the target is the backend's *own* kill-on-drop `TreeGuard` -- but the host uses `SIGKILL`, which bypasses `Drop`, so if the backend has not already cooperatively cancelled its target before the host's grace expires, the backend's cleanup guard never runs and the target (and any descendants it spawned) is orphaned.
**Impact:** The L0 invariant C7 ("timeout/cancel/drop kill child **and descendants**") is **not mechanically enforced for external-adapter targets**. It holds for the local path (the target is the host's direct child and shares the killed group) but not for the external path, where the target is a separately-grouped grandchild. The orphan is reachable whenever the backend is unresponsive past the cancel grace window -- e.g. the backend's `async fn drive()` performs **synchronous** `stdout` writes (see 4.4); if its stdout pipe stalls during host teardown, it cannot read the `cancel` frame, cannot kill its target, and is then SIGKILL'd by the host without having cleaned up. The system reports this honestly via `cleanup_unconfirmed`, so it is not a silent-success or data-loss defect, but a mandatory cleanup guarantee has a real hole. This was raised by the concurrent codex audit and **independently confirmed** here by reading the spawn + kill code.
**Fix:** Make the host's external-adapter teardown actually reach the target: either (a) have the backend NOT place its target in a new group (drop `process_group(0)` on the backend->target edge so the target shares the backend's group, which the host already kills), or (b) have the host walk/signals the backend's descendant groups on teardown rather than only `-backend_pgid`, or (c) send `SIGTERM` first with a short grace to let the backend's `Drop`/cancel handler kill the target before `SIGKILL`. Add a Linux test that orphans a target behind a stub backend that ignores `cancel`, then asserts the target process group is gone after host teardown (the current `sandbox_l0` evidence covers local L0, not this cross-process grandchild case).

```yaml
id: P16-correctness-03
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: invariants
severity: Major
title: Host L0 tree-kill is pgrp-scoped and cannot reach the external adapter's separately-grouped target
claim: The host SIGKILLs only the backend's process group; the backend places its target in a separate process group, so on an unresponsive backend the target descendant is orphaned and the L0 kill-descendants guarantee is not enforced for external adapters.
evidence:
  - location: crates/opi-coding-agent/src/tool/process_tree.rs:381
    detail: "libc::kill(-*pgid, libc::SIGKILL) -- pgrp-scoped kill of the backend's group only"
  - location: crates/opi-coding-agent/src/tool/process_tree.rs:51 and crates/opi-sandbox/src/process_tree.rs:92
    detail: "configure_tree runs cmd.process_group(0); both host->backend and backend->target edges create a NEW process group, so the target's pgid != backend's pgid"
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:230, 235
    detail: host configure_tree + TreeGuard::attach_child cover the backend only; the target is a separately-grouped grandchild
criterion_source: L0 SUPERVISION / INVARIANT C7 (timeout/cancel/drop kill child AND descendants)
reproduction:
  - On Linux: run a stub execution backend that spawns a long-lived target in its own pgrp and then ignores the cancel frame; let the host deadline fire; observe the target process survives host teardown (cleanup_unconfirmed is reported).
confidence: high
status: unverified
```

### 4.4 MINOR: Backend drive() does synchronous stdout writes on the async runtime, blocking cancel processing

**File:** `crates/opi-sandbox/src/backend.rs:132` (`pub async fn drive`) writing through `stdout: &mut dyn std::io::Write`; `crates/opi-sandbox/src/main.rs:18` (`#[tokio::main]`)
**Cause:** `drive()` is `async` and runs on the tokio runtime, but it emits frames via synchronous `Write` calls to `stdout`. If the host is not draining the backend's stdout fast enough (notably during teardown, when the host stops reading to enter its cancel/grace/kill sequence), a blocking `write` stalls the executor thread running `drive()`, preventing it from reading the `cancel` frame and killing its target.
**Impact:** The end-to-end deadline is still host-enforced (the host reads with `timeout_at` and SIGKILLs after grace), so this is not itself a deadline violation. But it is the trigger that makes the 4.3 orphan reachable: a backend blocked on a sync stdout write cannot cooperate with cancel, so the host falls back to a pgrp-kill that misses the target. Liveness/robustness defect independent of 4.3 as well (any async task that does unbounded blocking I/O on the runtime).
**Fix:** Write frames via the async `tokio::io::AsyncWriteExt` API (or `spawn_blocking` around the sync writes), so `drive()` stays cancellable while blocked on output.

```yaml
id: P16-correctness-04
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: correctness
severity: Minor
title: Backend drive() does synchronous stdout writes on the async runtime
claim: The async backend drive() emits frames via blocking std::io::Write calls, so a stalled stdout pipe blocks the executor and prevents cancel-frame processing.
evidence:
  - location: crates/opi-sandbox/src/backend.rs:132,109,115
    detail: "pub async fn drive(..., stdout: &mut dyn Write, ...); emit_frame/emit_failed_or_silent write synchronously"
  - location: crates/opi-sandbox/src/main.rs:18
    detail: "#[tokio::main] async fn main -- drive() runs on the async runtime"
criterion_source: L0 SUPERVISION (cooperative cancel must remain reachable); async-runtime liveness
reproduction: []
confidence: high
status: unverified
```

---

## 5. Security / Redaction Findings

### 5.1 INFO: AdapterUnavailable{Store} drops the failing adapter identity in public diagnostics

**File:** `crates/opi-coding-agent/src/execution/failure.rs:272-275, 216-235`
**Cause:** `From<ActivationError>` maps `ActivationError::Store(_)` to `AdapterUnavailable { adapter_id: None, detail: UnavailableDetail::Store }`. With `adapter_id = None`, `remediation()` resolves to "An adapter could not be activated (a package-store error)" -- a generic diagnostic that does not name the failing package.
**Impact:** This is **intentional redaction**: a `Store` error is an I/O/integrity failure with no specific validated adapter identity to expose (the store display may carry store internals), so the conversion drops it. Actionability is slightly reduced for the `Store` sub-case only (the other `UnavailableDetail` variants do name the adapter). No leak; no incorrect behavior.
**Fix:** None required for safety. If a future store error carries a safe package name, thread it into `adapter_id`.

```yaml
id: P16-security-01
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: security
severity: Info
title: AdapterUnavailable{Store} drops the failing adapter identity in public diagnostics
claim: The Store sub-case of AdapterUnavailable carries adapter_id=None, so the public diagnostic cannot name the failing package for that one sub-case.
evidence:
  - location: crates/opi-coding-agent/src/execution/failure.rs:272-275
    detail: ActivationError::Store(_) maps to AdapterUnavailable { adapter_id: None, detail: Store }
  - location: crates/opi-coding-agent/src/execution/failure.rs:216-235
    detail: remediation() with adapter_id=None renders "An adapter could not be activated (a package-store error)"
criterion_source: FAILURE & DIAGNOSTICS redaction contract (intentional drop)
reproduction: []
confidence: high
status: unverified
```

### 5.2 INFO: Windows bootstrap sets OPI_SANDBOX_* invocation-metadata env vars on the child

**File:** `crates/opi-sandbox/src/runner.rs:1370-1384, 1352-1368`
**Cause:** `apply_windows_bootstrap_env` sets `OPI_SANDBOX_RELEASE_GATE`, `OPI_SANDBOX_TARGET_PROGRAM`, `OPI_SANDBOX_TARGET_ARG_*`, `OPI_SANDBOX_BACKEND_PID` on the child `Command` (after `apply_env`, which may `env_clear`). A direct-SDK Windows target process can therefore observe these invocation-metadata variables in its own environment.
**Impact:** Not a defect against the Phase 16 contract: Windows is a **permanently-unsupported** restriction platform (Job Objects provide L0 supervision only; no confinement claim exists), and **environment-variable confidentiality is an explicit Non-Goal** ("Providing host-read or environment-variable confidentiality"). The variables are invocation metadata, not credentials. Informational only.
**Fix:** None required for the Phase 16 contract. If a future Windows restriction mechanism is added, revisit how the bootstrap passes the release-gate/argv without env leakage.

```yaml
id: P16-security-02
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: security
severity: Info
title: Windows bootstrap exposes OPI_SANDBOX_* invocation-metadata env vars to the target
claim: On the unsupported Windows SDK path, bootstrap metadata env vars are visible to the target process after env_clear.
evidence:
  - location: crates/opi-sandbox/src/runner.rs:1370-1384
    detail: apply_windows_bootstrap_env sets OPI_SANDBOX_* on cmd after apply_env/env_clear
criterion_source: Non-Goal (env-var confidentiality) + Windows unsupported restriction
reproduction: []
confidence: high
status: unverified
```

### 5.3 INFO: ToolExecutionEnd event redacts context but clones the diagnostic message raw

**File:** `crates/opi-agent/src/event.rs:152-174`
**Cause:** The `ToolExecutionEnd` event redacts the `details` and `diagnostics` context but clones the diagnostic `message` field without a redaction pass.
**Impact:** Pre-existing `opi-agent` infrastructure (not introduced by Phase 16), intersecting the Phase 16 redaction surface only where a backend/execution diagnostic message reaches the event stream. Phase 16's own diagnostic surfaces go through `diagnostic_bridge` summary redaction, and the stable `ExecutionFailure` messages are constructed from safe identifiers, so the practical exposure is low. Medium confidence pending a full event-stream audit, which is outside Phase 16's scope.
**Fix:** Apply `redact_text(.., Summary)` to the cloned diagnostic `message` in the event path, or confirm message fields are already redacted at production time.

```yaml
id: P16-security-03
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: security
severity: Info
title: ToolExecutionEnd event clones the diagnostic message raw (context is redacted)
claim: The event redacts details and diagnostics context but not the diagnostic message string itself.
evidence:
  - location: crates/opi-agent/src/event.rs:152-174
    detail: details/diagnostics context redacted; diagnostic.message cloned without redact_text
criterion_source: SECURITY/REDACTION (surfaces omit command text/env/credentials) -- pre-existing opi-agent infra
reproduction: []
confidence: medium
status: unverified
```

### 5.4 Examined and dismissed: ToolResult.details carries the raw command string

The model-callable `bash` tool puts `"command"` into the in-memory `ToolResult.details` (`bash_operation_metadata`, `crates/opi-agent/src/tool/result.rs:104-113`), including for routed-external results (`crates/opi-coding-agent/src/tool/bash.rs:258-268`). This was raised and re-checked. It is **not a defect**: `details.command` is the model's *own* command surfaced back to the model/user (standard agent feedback, not a credential leak); it is the **pre-existing Phase 11 contract** that Phase 16 explicitly preserves byte-for-byte (SC16-01); and the security-critical **diagnostic** path correctly excludes raw command text (`bash_operation_diagnostic` sets `command_included: false` with a test asserting `context.get("command").is_none()`, `bash.rs:458-491, 585-588`). No finding.

---

## 6. Invariant Verification

| Invariant | Code evidence | Test coverage |
|-----------|--------------|---------------|
| Five gates independent (Installed/Trusted/Enabled/Selected/Permitted) | `router.rs` `resolve_selection` gates on eligibility + non-denied permission; `permission.rs` separate from routing; model cannot mutate gates | `execution_routing`, `execution_package_lifecycle`, `execution_product` |
| Fail-closed: selected external adapter never falls back to local | `RoutedBashOperations::exec` resolves selection and fails; no local retry on external `Err` | `windows_execution_posture` (absent/target/version mismatch, `call_count()==0`), `execution_product`, `execution_runtime` |
| No degraded success (timeout/cancel are errors even on clean exit) | `bash.rs:277` `is_error = timed_out \|\| cancelled \|\| signal.is_some() \|\| exit_code != Some(0)` | `execution_product::timed_out_in_band_completed_is_not_a_success`, `cancelled_in_band_completed_is_not_a_success` |
| One backend process, one execution, no second execution | `protocol_host.rs` one-shot drive; backend `--stdio` processes one invocation | `execution_protocol_host`, `protocol_conformance` |
| Every frame carries one host request id | `protocol_host.rs` / `backend.rs` stamp request id; `opi-protocol` fixtures reject missing/mismatched/cross-request ids | `execution_v1_contract`, `execution_v1_schema` |
| Cleanup-on-drop / temp-root + child-tree removal on all terminal paths | LOCAL: `supervision.rs` biased cancel>timeout>wait + pgrp kill of the direct child; `SandboxRun` owns guards. **EXTERNAL: host kill is `-backend_pgid` only and cannot reach the separately-grouped target -- see 4.3 (Major)** | `sandbox_l0` (LOCAL path), `sdk_contract`; **no test covers the external grandchild-target orphan (4.3)** |
| Invocation-stateful, cross-invocation stateless SDK | `runner.rs` no shared invocation state; each call owns temp root | `sdk_contract` |
| Bounds internally consistent | `bounds.rs::validate` (chunk, config) -- **gap: diagnostics not cross-checked (4.1)** | `defaults_are_consistent`; missing diagnostics cross-check test |
| Crate boundaries: opi-coding-agent links neither opi-sandbox nor native policy | `Cargo.toml` deps + `cargo tree` proofs | `phase16_crate_boundaries`, `crate_boundaries` |

### 6.1 INFO: EnvInherit hardcoded to Inherit for all external adapter executions

**File:** `crates/opi-coding-agent/src/execution/runtime.rs:659`
**Cause:** External adapter executions hardcode `EnvInherit::Inherit`; the wire type carries an inheritance policy but the host never makes it configurable.
**Impact:** The spec is satisfied by presence ("execute carries ... env-inheritance policy") and the bounded-additions path is honored, so no behavior is wrong. The policy is simply not user-configurable for external adapters (it is always inherit). Informational.
**Fix:** Optional -- surface an env-inheritance configuration knob, or document that external adapters always inherit.

```yaml
id: P16-invariants-01
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: invariants
severity: Info
title: EnvInherit hardcoded to Inherit for all external adapter executions
claim: The wire env-inheritance policy is carried but the host always uses Inherit for external adapters; it is not configurable.
evidence:
  - location: crates/opi-coding-agent/src/execution/runtime.rs:659
    detail: EnvInherit::Inherit used unconditionally for external adapter execution
criterion_source: PROTOCOL ("execute carries ... env-inheritance policy") -- presence satisfied, configurability not
reproduction: []
confidence: high
status: unverified
```

---

## 7. Integration Findings

### 7.1 MINOR: Shell program+flag mapping duplicated between local and protocol paths

**File:** `crates/opi-coding-agent/src/execution/protocol_host.rs:594-612`
**Cause:** The mapping of the `bash` shell string to an explicit platform shell program and argument vector (`sh -c`/`cmd /C`) is implemented separately for the local path and the protocol-host path with no shared constant or helper.
**Impact:** Fowler Duplicated Code with a divergence risk: if the shell invocation convention changes, both sites must be updated in lockstep or the local and external backends could disagree on how a command is invoked.
**Fix:** Extract one `fn shell_program_and_args() -> (&str, Vec<&str>)` (or constant) shared by both paths.

```yaml
id: P16-integration-01
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: integration
severity: Minor
title: Shell program+flag mapping duplicated between local and protocol paths
claim: The bash-to-shell-program mapping is implemented independently in two places with no shared constant, risking divergence.
evidence:
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:594-612
    detail: protocol host maps the shell string to a program+argvec separately from the local path
criterion_source: PROTOCOL ("the host maps the bash shell string to an explicit platform shell program and argument vector")
reproduction: []
confidence: high
status: unverified
```

### 7.2 INFO: Cap-before-materialize line reader triplicated across crates

**File:** `crates/opi-protocol/src/execution/v1/codec.rs:70-108`
**Cause:** The "read up to a line cap, then materialize" async line-framing reader is implemented independently in `opi-protocol` (codec), the `opi-coding-agent` host, and the `opi-sandbox` backend.
**Impact:** Duplicated framing logic that could diverge on edge cases (CRLF handling, truncation semantics). Crate boundaries (the host/backend cannot depend on each other, and `opi-protocol` owns the codec) prevent trivial sharing.
**Fix:** Optional -- publish the canonical line reader from `opi-protocol` and consume it in both the host and backend.

```yaml
id: P16-integration-02
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: integration
severity: Info
title: Cap-before-materialize line reader triplicated across three crates/modules
claim: The bounded async line-framing reader is implemented independently in the protocol codec, the host, and the backend.
evidence:
  - location: crates/opi-protocol/src/execution/v1/codec.rs:70-108
    detail: line reader present; mirrored independently in the host and backend
criterion_source: INTEGRATION (host/backend agreement on frame shapes and bounds)
reproduction: []
confidence: high
status: unverified
```

### 7.3 INFO: Backend masks exit to 0..255 but host and runtime pass u32 through

**File:** `crates/opi-coding-agent/src/execution/runtime.rs:696`
**Cause:** The backend masks the target exit to `0..255`, but the host and runtime pass the wire `u32` through without re-masking.
**Impact:** The exit-masking contract is enforced only by the sender (the backend). A conformant backend masks; a non-conformant backend sending a value >255 would pass through unchanged. No live defect with a conformant backend; defensive only.
**Fix:** Optional -- re-mask on the host/runtime side, or assert the backend contract in a test.

```yaml
id: P16-integration-03
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: integration
severity: Info
title: Backend masks exit to 0..255 but host/runtime pass u32 through without masking
claim: Exit-code masking is sender-enforced only; the host/runtime do not re-mask the wire u32.
evidence:
  - location: crates/opi-coding-agent/src/execution/runtime.rs:696
    detail: runtime passes the exit value through without masking to 0..255
criterion_source: PROTOCOL/INTEGRATION (exit-code masking contract)
reproduction: []
confidence: high
status: unverified
```

---

## 8. Test-Quality Findings

### 8.1 INFO: Several structural tests are source-text greps, not behavioral assertions

**File:** `crates/opi-coding-agent/tests/execution_protocol_host.rs:1196-1203` (and similar across the execution test suite)
**Cause:** A number of structural/crate-boundary/documentation tests assert via `include_str!`/`read_repo_file!` + `.contains(...)` over source text rather than driving the behavior.
**Impact:** These tests pin source-text presence (tokens, comments, guard phrases) rather than observable behavior; they pass even if the pinned string is dead code, and they can rot silently. They are legitimate for documentation/guard contracts where no runtime entry point exists, but several would be stronger as behavioral checks.
**Fix:** Where a production entry point exists, prefer driving it (as `windows_execution_posture.rs` and `execution_product.rs` already do) over grepping source text.

```yaml
id: P16-testquality-01
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: Several structural tests are source-text greps rather than behavioral assertions
claim: Some execution/crate-boundary/docs tests assert on stringly-typed source internals via include_str/read_repo_file + .contains instead of observable behavior.
evidence:
  - location: crates/opi-coding-agent/tests/execution_protocol_host.rs:1196-1203
    detail: representative source-text assertion pattern
criterion_source: TEST QUALITY (assertion strength)
reproduction: []
confidence: high
status: unverified
```

### 8.2 INFO: Three contribution security gates compile out on Windows via cfg(unix)

**File:** `crates/opi-coding-agent/tests/execution_contribution_manifest.rs:496-539`
**Cause:** Three executable-contribution security-gate tests (path-containment / executable-shape rejection) are gated `#[cfg(unix)]`, so they are invisible and do not run on a Windows host.
**Impact:** On a Windows dev/CI host these security assertions are silently absent (a known platform-invisibility pattern in this repo). The gates still run on Linux/macOS CI. Coverage gap on Windows only.
**Fix:** Where the assertion logic is platform-neutral (lexical path rejection), remove the `cfg(unix)` gate; keep `cfg(unix)` only for genuinely Unix-only mechanics.

```yaml
id: P16-testquality-02
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: Three contribution security gates compile out on Windows via cfg(unix)
claim: Security-gate tests gated cfg(unix) are invisible on a Windows host.
evidence:
  - location: crates/opi-coding-agent/tests/execution_contribution_manifest.rs:496-539
    detail: cfg(unix)-gated contribution security assertions
criterion_source: TEST QUALITY (platform-invisible coverage)
reproduction: []
confidence: high
status: unverified
```

### 8.3 INFO: Entire protocol-host subprocess suite is behind a feature gate

**File:** `crates/opi-coding-agent/tests/execution_protocol_host.rs:13`
**Cause:** The complete protocol-host subprocess behavioral suite is gated behind the `execution-backend-test-fixture` feature; default `cargo test -p opi-coding-agent` does not exercise it.
**Impact:** The richest protocol-ordering/edge-case coverage is opt-in. A contributor running the default test suite sees none of it. The fixture must also be built first (`--test execution_backend_mock --no-run`), which is a known sequencing requirement.
**Fix:** Optional -- run the feature-gated suite in the default dev profile, or document prominently that protocol-host coverage requires the feature.

```yaml
id: P16-testquality-03
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: Entire protocol-host subprocess suite is behind the execution-backend-test-fixture feature gate
claim: Default cargo test does not exercise the protocol-host behavioral subprocess suite.
evidence:
  - location: crates/opi-coding-agent/tests/execution_protocol_host.rs:13
    detail: suite gated on the execution-backend-test-fixture feature
criterion_source: TEST QUALITY (coverage default-off)
reproduction: []
confidence: high
status: unverified
```

### 8.4 INFO: Bounded-drain-grace test asserts <3s, not the actual 500ms constant

**File:** `crates/opi-coding-agent/tests/sandbox_l0.rs:541, 568`
**Cause:** The bounded-drain-grace integration test asserts completion within ~3s, while the actual drain-grace constant is 500ms.
**Impact:** The assertion is ~6x looser than the real bound; a regression that doubled the grace period would still pass. The invariant is exercised but not tightly pinned.
**Fix:** Tighten the assertion toward the actual constant (with a realistic CI jitter margin).

```yaml
id: P16-testquality-04
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: Bounded-drain-grace test asserts <3s, not the actual 500ms constant
claim: The drain-grace integration test bound is ~6x looser than the real constant.
evidence:
  - location: crates/opi-coding-agent/tests/sandbox_l0.rs:541,568
    detail: asserts a ~3s ceiling vs the 500ms drain-grace constant
criterion_source: TEST QUALITY (assertion strength)
reproduction: []
confidence: high
status: unverified
```

### 8.5 INFO: Model-strategy x target/version-mismatch not exercised end-to-end through production execute

**File:** `crates/opi-coding-agent/tests/windows_execution_posture.rs` (covered under fixed only)
**Cause:** Target-mismatch and version-mismatch are proven through the real activation gate under the **fixed** strategy (`fixed("opi-sandbox")` + `exec_code` + `call_count()==0`). The same mismatch under the **model** strategy is not driven end-to-end through the production execute path.
**Impact:** Low. Target/version mismatch fails at `revalidate_lock` / `validate_executable_contributions` during activation, which is **strategy-independent** (it runs before routing-strategy selection matters), so the fixed-strategy test already proves the gate. The gap is a coverage nicety, not a behavioral hole.
**Fix:** Optional -- add a model-strategy variant, or document that the activation gate is strategy-independent.

```yaml
id: P16-testquality-05
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: Model-strategy x target/version-mismatch not exercised end-to-end
claim: Target/version mismatch is proven under fixed only; the model-strategy combination is not driven through production execute (low value -- the gate is strategy-independent).
evidence:
  - location: crates/opi-coding-agent/tests/windows_execution_posture.rs:390-503
    detail: target/version mismatch tested via fixed("opi-sandbox") through ExecutionRuntime::build
criterion_source: TEST QUALITY (coverage)
reproduction: []
confidence: high
status: unverified
```

---

## 9. Residuals

### 9.1 INFO: opi-sandbox SDK silently overrides user-provided TMPDIR/TMP/TEMP

**File:** `crates/opi-sandbox/src/runner.rs:1362-1367`
**Cause:** `apply_env` unconditionally sets `TMPDIR`/`TMP`/`TEMP` to the invocation temp root, overriding any user-provided value in the additions map.
**Impact:** The SDK advertises "explicit inputs," but a caller-provided `TMPDIR` is silently replaced. Minor API-ergonomics wart; the invocation temp root is the correct value for restriction, so behavior is right -- the issue is silent override rather than rejection.
**Fix:** Either reject a user-provided `TMPDIR`/`TMP`/`TEMP` with a clear error, or document that these are always invocation-owned.

```yaml
id: P16-residuals-01
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: residuals
severity: Info
title: opi-sandbox SDK silently overrides user-provided TMPDIR/TMP/TEMP
claim: apply_env unconditionally overrides caller-provided TMPDIR/TMP/TEMP with the invocation temp root.
evidence:
  - location: crates/opi-sandbox/src/runner.rs:1362-1367
    detail: cmd.env("TMPDIR", temp_root).env("TMP", temp_root).env("TEMP", temp_root) runs after applying user additions
criterion_source: API ERGONOMICS (explicit-inputs contract)
reproduction: []
confidence: high
status: unverified
```

### 9.2 INFO: hash_and_rewind_snapshot reads the full executable into a Vec per spawn

**File:** `crates/opi-coding-agent/src/execution/contribution.rs:545-552`
**Cause:** `hash_and_rewind_snapshot` hashes the executable by `read_to_end` into a `Vec<u8>`, allocating the full executable bytes per adapter invocation (once per process start, during pre-spawn revalidation).
**Impact:** Once-per-spawn allocation bounded by executable size; negligible in practice (a single spawn already forks a process). A streaming hash would avoid the allocation. Informational.
**Fix:** Optional -- hash the snapshot via a chunked `Sha256::update` reader to avoid the full-byte `Vec`.

```yaml
id: P16-residuals-02
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: residuals
severity: Info
title: hash_and_rewind_snapshot allocates the full executable into a Vec per spawn
claim: Pre-spawn executable hashing reads the whole file into a Vec rather than streaming.
evidence:
  - location: crates/opi-coding-agent/src/execution/contribution.rs:545-552
    detail: let mut file_bytes = Vec::new(); snapshot.read_to_end(&mut file_bytes)?; sha256_hex(&file_bytes)
criterion_source: PERFORMANCE (avoid avoidable per-invocation allocations)
reproduction: []
confidence: high
status: unverified
```

### 9.3 INFO: Audited HEAD is 21 commits ahead of origin/main

**File:** repository state (`git rev-list --count origin/main..HEAD` == 21)
**Cause:** `audit_head` (`c5de892`) is 21 commits ahead of `origin/main` (0 behind).
**Impact:** Not a code defect. CI/release evidence for the exact audited state is incomplete -- the phase-exit artifact audit and per-task `verified_at_commit` evidence were captured at earlier commits (`f8aff02` and prior), and the 21 unpushed commits include post-phase-exit changes that public CI has not validated. Release hygiene: push and let CI run on the audited HEAD before any release.
**Fix:** Push `main` to `origin` and confirm the full CI matrix is green on the audited HEAD before release.

```yaml
id: P16-residuals-03
source_kind: audit
source_path: docs/snapshots/phase16/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: residuals
severity: Info
title: Audited HEAD is 21 commits ahead of origin/main
claim: The audited commit is unpushed; CI/release evidence for the exact audited state is incomplete.
evidence:
  - location: git rev-list --count origin/main..HEAD
    detail: 21 commits ahead, 0 behind
criterion_source: Release hygiene / repository gates (evidence completeness)
reproduction:
  - git rev-list --count origin/main..HEAD
confidence: high
status: unverified
```

---

## 10. Residuals and Recommendations

### Priority recommendations

1. **(4.3, Major)** Fix the external-adapter L0 descendant-kill gap. The host's `-backend_pgid` SIGKILL cannot reach the target's separate process group, and SIGKILL bypasses the backend's kill-on-drop guard, so an unresponsive backend orphans its target. Either keep the target in the backend's group, have the host signal descendant groups, or `SIGTERM`-with-grace before `SIGKILL`. Add a Linux test that orphans a target behind a cancel-ignoring stub backend and asserts the target group is reaped by host teardown. Pairs with 4.4 (make `drive()` writes async so cancel stays reachable).
2. **(3.1, Minor)** Unify the manifest-hash normalization basis so the resolver lock path uses the same LF-normalization as the contribution/trust path; a Windows autocrlf re-checkout can spuriously disable a globally-installed execution adapter. Add a CRLF-stability test for the resolver lock path.
3. **(3.2, Minor)** Make the startup vs doctor `Diagnostic.code` field uniform (expose the stable 14-code as the top-level code, or document the startup-envelope convention and assert it).
4. **(4.1 / 4.2, Minor)** Tighten protocol self-validation: add the `max_diagnostics_size` cross-check to `Bounds::validate()`, and enforce `CompletedPayload` exit/signal mutual-exclusivity/at-least-one at the substrate.
5. **(7.1, Minor)** Extract the shared shell-program mapping so the local and protocol paths cannot diverge.
6. **(9.3, Info but release-blocking)** Push `main` to `origin` and confirm the full CI matrix is green on the audited HEAD before release.

### Non-blocking observations worth a deliberate decision
- Standalone-crate README lockstep (2.2) and the documented crate-boundary duplication overrides (2.1, 7.2) -- accept or formalize the exceptions.
- Tighten the drain-grace test bound (8.4) and de-feature-gate or document the protocol-host suite (8.3).

### Verdict rationale
No Blocker. One Major (4.3): the external-adapter L0 descendant-kill guarantee is not mechanically enforced (host pgrp-kill misses the separately-grouped target; SIGKILL bypasses the backend's kill-on-drop guard), reachable when a backend is unresponsive past cancel grace; the system reports it honestly via `cleanup_unconfirmed`, and the local L0 path is correct, so this is a cleanup-guarantee hole rather than a security/data-loss/crash defect. The six Minors are fail-safe, narrow, or substrate-validation-completeness gaps. The phase is sound for continuation; address 4.3 (and the contributing 4.4) and 9.3 before release.

---

## Appendix A. Cross-audit divergence and independent re-verification

During this audit a **concurrent `codex` audit** ran in the same working tree and wrote its own deliverable (`audit.codex.md`, verdict **FAIL -- 5 Major**). That file was not read for findings; its divergence surfaced when an unexpected working-tree change was diagnosed. Per the skill's contamination rules, each of codex's headline Major claims was then **independently re-checked against committed code at `c5de892`** rather than adopted. Results:

| Codex Major claim | Independent re-verification at `c5de892` | Outcome |
|---|---|---|
| Host async line reader rejects exact-cap CRLF frames (semantic drift vs the canonical codec) | `codec.rs::LineReader` (97-100) and `protocol_host.rs::CappedReader` (1425-1428) BOTH set `pending_cr` on CR without consuming the cap and accept a `max_line_size`+CRLF line | **Refuted** -- the two readers agree at this commit; not a finding (the duplication is noted as Info 7.2) |
| Host pgrp does not contain the adapter target process group (L0 descendant-kill gap) | `process_tree.rs:381` `kill(-pgid, SIGKILL)`; both spawn edges use `process_group(0)`; SIGKILL bypasses the backend's kill-on-drop | **Confirmed -- now finding 4.3 (Major)** |
| Synchronous backend stdout writes block the async loop past deadlines | `backend.rs:132` `async fn drive` writes via sync `dyn Write`; host reads with `timeout_at` so the deadline is host-enforced, but the backend's own cancel processing can stall | **Partially confirmed -- now finding 4.4 (Minor); it is the trigger for 4.3, not a standalone deadline defeat** |
| Normal in-band timeouts lose the stable `execution_timed_out` code (host deadline maps to `cleanup_unconfirmed`) | `protocol_host.rs:949` maps a host read-deadline to `CleanupUnconfirmed`; backend-reported `Failed{ExecutionTimedOut}` maps to `ExecutionTimedOut`; this is the documented design in `failure.rs` | **Not adopted as a defect** -- documented, defensible mapping; noted here for transparency |
| Public-surface redaction trusts adapter-controlled free text | `failure.rs` interpolates only safe identifiers and drops untrusted detail; adapter diagnostic `message` text is the backend's responsibility per `frames.rs:250` | **Not adopted at Major** -- the redaction contract is upheld on the host side; backend-authored diagnostic free text is a protocol-trust boundary, noted via 5.3 (pre-existing event path) |

This auditor's verdict is **PASS-WITH-FINDINGS (1 Major, 6 Minor, 18 Info)**, differing from codex's FAIL because the most concrete interop claim (CRLF) is refuted in the committed code and the timeout/redaction claims are assessed as documented designs rather than defects, while the genuinely confirmed pgrp-containment gap is adopted as Major 4.3. The divergence itself is the value: the cross-check surfaced a real Major (4.3) that the initial 63-agent pass had classified as a passing invariant, and it was retained only after independent code verification.
