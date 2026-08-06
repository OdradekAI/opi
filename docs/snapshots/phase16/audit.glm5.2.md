# Phase 16 Pluggable Extensions and Command Execution — Independent Code Audit

**Auditor**: glm5.2 (independent; no prior audit reports, evaluator transcripts, or reader notes consulted)
**Date**: 2026-08-06
**Scope**: Tasks 16.1–16.16.3 (21 tasks). Commit range `1021842c` (16.1) → `f8aff02` (16.16.3), plus two post-phase remediation commits `2b23010` and `edd8d91` ("fix(execution): remediate phase 16 audit findings"). Audited at HEAD `8b547da`, which includes both remediation passes.
**Spec**: `docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md` (canonical) + `docs/opi-spec.md` §Phase 16.
**Method**: Full read of the design spec (967 lines) and all 21 task DoDs/exit criteria. Deep read of the affected source/test across `opi-protocol`, `opi-sandbox`, `opi-coding-agent`, and `opi-tui` via 10 parallel independent readers organized by concern area (~1.59M tokens, 377 tool calls), each walled off from prior audit content. Findings were then adversarially verified by 4 fresh cross-cutting agents (a redaction attacker, a fail-closed/no-fallback attacker, a Major-refutation skeptic, and a completeness critic) that had not seen the readers' findings, plus direct inline confirmation of the load-bearing findings against the source. The host is Windows, so `#[cfg(unix)]` / Linux / macOS code was read as source and is noted as host-verifiability-limited where relevant.

---

## 1. Executive Summary

**Verdict: PASS**

| Severity | Count |
|----------|-------|
| Blocker  | 0     |
| Major    | 0     |
| Minor    | 26    |
| Info     | 12    |

Phase 16 is a large, security-sensitive slice (command-execution capability, package trust lifecycle, routing/permission, a versioned wire protocol, a standalone sandbox product, native confinement, and a Phase 15 → 16 migration). It is in strong shape. Every load-bearing invariant I traced holds in code and is test-pinned: the five independent gates, fail-closed-after-selection with **no** local fallback, **no** degraded-success state (timeout/cancellation are errors even on a clean exit code), the 14-code redacted failure envelope, request-id correlation, byte-for-byte Minimal-Runtime schema invariance, invocation-stateful/cross-invocation-stateless cleanup-on-every-path for `opi-sandbox`, honest `restricted`/`supervised` vocabulary, and the crate-boundary + migration contract (native sandbox deleted from core; `[sandbox]`/`--sandbox`/`--sandbox-require` rejected without aliases; `opi-coding-agent` links no `opi-sandbox`/native-policy dependency). No Non-Goal leaked into the implementation. The two remediation commits introduced no new behavioral defect on independent re-examination.

The findings are overwhelmingly **test-quality gaps, doc drift, and defense-in-depth/latent hardening observations** — not live defects. The one initially-promoted Major (git-install crash-safety ordering) was **downgraded to Minor after adversarial verification**: the runtime ordering is correct, the primary (publish-failure) rollback path *is* behaviorally tested, and only a degenerate source-text ordering test plus two hard-to-trigger rollback edges are genuinely uncovered.

### Per-task summary

All 21 tasks pass; residuals are Minor/Info only.

| Task | Title | Verdict |
|------|-------|---------|
| 16.1 | Pin the Phase 16 documentation contract | PASS |
| 16.2 | Pin L0 supervision and define the policy-neutral seam | PASS (Minor T15/C1) |
| 16.3 | Add opi-protocol::execution::v1 | PASS (Minor C3/T5; Info C4/C7) |
| 16.4 | Parse and hard-gate executable contributions | PASS (Minor T3/T4) |
| 16.5 | Add Package Trust and enable/disable lifecycle | PASS (Minor T1/T7; Info I4) |
| 16.6 | Add execution config, failures, routing, permission policy | PASS (Minor S3/S4; Info S5) |
| 16.7 | Implement the one-shot execution protocol host | PASS (Minor C2; Info C4/T19) |
| 16.8 | Build the deep Execution Runtime assembly | PASS (Minor I1; Info I3) |
| 16.9 | Wire Execution Runtime, dynamic bash schema, public surfaces | PASS (Minor T8/T9; Info C5) |
| 16.10 | Add the interactive permission broker and TUI prompt | PASS (Minor T10/T11; Info T20) |
| 16.11.1 | Build the standalone opi-sandbox SDK and runner | PASS |
| 16.11.2 | Build the human opi-sandbox CLI and direct smoke | PASS (Minor T6/T12) |
| 16.12 | Add the atomic helper gate and protocol backend | PASS |
| 16.13 | Port the Linux native restriction contract | PASS (Minor T2/T13/T14; Info C6/SC2) |
| 16.14.1 | Port the macOS native restriction contract | PASS (Minor T13) |
| 16.14.2 | Pin the Windows unsupported execution posture | PASS |
| 16.15.1 | Build host-neutral opi-sandbox packaging | PASS |
| 16.15.2 | Wire native package CI, release, and artifact audit | PASS (Minor T17/T18) |
| 16.16.1 | Remove core native sandbox and enforce migration boundaries | PASS (Minor SC1; Info I2) |
| 16.16.2 | Prove install-to-execute and cross-surface diagnostics | PASS |
| 16.16.3 | Synchronize documentation and close Phase 16 repository gates | PASS |

---

## 2. Correctness findings

### 2.1 MINOR: L0 supervision cancel/timeout arms kill the child but never reap it (asymmetry vs attach/resume arms)

**File:** `crates/opi-coding-agent/src/tool/supervision.rs`
**Lines:** 241–249 (cancel/timeout arms), cf. 163–164 and 201–202 (attach/resume arms)
**Cause:** In `supervise_inner`'s biased select, the cancel arm (:241–244) and timeout arm (:246–249) call `child.kill().await` + `push_terminate`, but never `child.wait()`. The attach-failure path (:163–164) and the Windows resume-failure path (:201–202) both explicitly do `child.kill().await; child.wait().await;`, and the `Done` arm reaps via the `child.wait()` future. So the direct child of a cancelled or timed-out local `bash` invocation is killed but not explicitly reaped.
**Impact:** The L0 *terminate* contract (kill the whole process tree) is still satisfied, and the child is eventually reaped by tokio's drop/orphan-reap semantics plus process exit. This is a consistency/defense-in-depth gap rather than a confirmed leak: if reap semantics ever differ (e.g. a current-thread runtime, or a future tokio change), cancelled/timed-out bash calls in a long-running interactive/RPC session could accumulate zombies. The within-file asymmetry reads as an oversight.
**Fix:** After the kill in the cancel/timeout arms (or once before constructing `SupervisionOutcome`), call `let _ = child.wait().await;` (optionally wrapped in `tokio::time::timeout(TERMINATED_PIPE_DRAIN_GRACE, child.wait())`), mirroring the attach-failure arm.

### 2.2 MINOR: Diagnostic-frame count is unbounded in the protocol host

**File:** `crates/opi-coding-agent/src/execution/protocol_host.rs` (with `crates/opi-protocol/src/execution/v1/session.rs`)
**Lines:** protocol_host.rs ~508–510 (Diagnostic accumulation); session.rs `observe_backend`/`account_output`
**Cause:** `Session::account_output` accumulates only `Stdout`/`Stderr` bytes toward `max_cumulative_output`; `validate_backend` caps each `Diagnostic` *message* at `max_diagnostics_size`, but `Diagnostic` frames are excluded from the cumulative counter and have no per-execution count or aggregate-bytes cap. The host pushes every received (redacted) Diagnostic into a `Vec<Diagnostic>` with no upper bound.
**Impact:** A buggy or chatty backend can stream unlimited Diagnostic frames within the deadline; the `Vec` grows without bound until the host deadline fires. Bounded by wall-clock + pipe throughput and by the threat model (adapters are trusted code), so this is a defense-in-depth/asymmetric-DoS surface, not an exploit. It is the one protocol frame class with neither per-frame-amount nor cumulative accounting.
**Fix:** Count Diagnostic frames (and their decoded message bytes) toward `max_cumulative_output` in `account_output`, or add a separate `max_diagnostics_frames`/`max_diagnostics_bytes` bound enforced in `observe_backend`.

### 2.3 MINOR: `feed_host_line`/`feed_backend_line` do not enforce line/message size (rustdoc overclaim + defense-in-depth gap)

**File:** `crates/opi-protocol/src/execution/v1/session.rs`
**Lines:** 91–105
**Cause:** The rustdoc claims these functions "enforce per-frame codec bounds (line/message/... size)", but they call `decode_*` → `observe_*` → `validate_host/validate_backend`, which (per `validate_host`'s own doc) check only configuration/diagnostics/chunk sizes. Line size is enforced only by `LineReader::read_line` on the input stream and `encode_line` on output.
**Impact:** A caller that bypasses `LineReader` (the codec module doc explicitly warns against `BufRead::read_line`) and feeds a 10 MB line directly to `feed_*_line` gets no line-size rejection. In the documented input path (`LineReader` then `feed_*_line`) all bounds are enforced, so this is doc drift + a defense-in-depth gap on a wire-facing public API.
**Fix:** Add a defensive `if line.len() > self.bounds.max_line_size { return Err(...) }` guard at the top of `feed_host_line`/`feed_backend_line`, or correct the rustdoc to drop "line/message" (matching `validate_host`'s disclaimer).

### 2.4 INFO: `map_failure_code` always maps `Unavailable` to `Handshake`, ignoring `FailedPayload.phase`

**File:** `crates/opi-coding-agent/src/execution/protocol_host.rs`
**Lines:** 692–704
**Cause:** `FailureCode::Unavailable` is mapped to `UnavailableDetail::Handshake` unconditionally, regardless of `FailedPayload.phase`. A (semantically anomalous) post-`started` `Unavailable` would be labeled a pre-start handshake failure in remediation text.
**Impact:** `Unavailable` is by definition a pre-start condition, so no correct backend triggers this; the wire code returned (`adapter_unavailable`) is still correct. Only the human-readable detail sublabel is inaccurate for that anomalous combination, and no test covers `failed_post_started`+`unavailable`.
**Fix:** Add a comment that `Unavailable` is by-definition handshake-phase, or map `Unavailable` with `phase != Handshake` to `ProtocolViolation`.

### 2.5 INFO: `operation_context` diagnostic `signal` field is write-only (dead data for routed adapters)

**File:** `crates/opi-coding-agent/src/tool/operations.rs` (:1077) cross-ref `tool/bash.rs` (:295–340)
**Cause:** The remediation added `signal` to the operation-context payload, but `lift_operation_context` (bash.rs:295–315) and `copy_effective_contract` (bash.rs:317–340) do not read it; the wrapper sources signal from `BashResult.signal`. For an external adapter that reported signal only through this diagnostic (leaving `BashResult.signal=None`), the signal would be dropped from the public `ToolResult`.
**Impact:** No current functional defect observed for `LocalBashOperations` (`BashResult.signal` carries the value). Flagged so the next reviewer confirms routed adapters source signal correctly.
**Fix:** Drop the field if it is purely informational, or have `lift_operation_context` read `signal`/`exit_code` from the diagnostic when the `BashResult` fields are `None`.

### 2.6 INFO: `close_nonessential_inherited_fds` iterates up to the soft `RLIMIT_NOFILE` per spawn

**File:** `crates/opi-sandbox/src/process_tree.rs`
**Lines:** ~539–550
**Cause:** `fd_table_size()` returns the soft `RLIMIT_NOFILE` (`getdtablesize`), and the closure loops fd 3..max calling `getsockopt`+`close` per fd. On hosts that raise the soft limit (tokio runtimes, `systemd DefaultLimitNOFILE=1M`), this is ~1M syscalls per confined spawn under `network=deny`.
**Impact:** Latency tax only; not a correctness or security issue.
**Fix:** Iterate only open fds (scan `/proc/self/fd`) or cap at a sane bound (e.g. 4096); document the trade-off.

### 2.7 INFO: Base64 size formula in `Bounds::validate` underestimates by up to 2 bytes (fail-closed)

**File:** `crates/opi-protocol/src/execution/v1/bounds.rs`
**Lines:** 55–69
**Cause:** Computes `ceil(4N/3)+64` rather than the exact `ceil(N/3)*4+64`, under-counting by up to 2 bytes for non-multiples of 3.
**Impact:** Default bounds are unaffected (`max_line_size` 2 MiB ≫ need). For custom bounds set exactly at the threshold, a maximal valid chunk could be rejected by `encode_line` as `OversizedLine` — fail-**closed**, never over-allocated.
**Fix:** Optional: compute `((max_decoded_chunk_size + 2) / 3) * 4 + 64` exactly, or document the approximation.

---

## 3. Security / redaction findings

No live redaction leak was found. A dedicated cross-cutting attacker traced every hostile source (model-supplied `backend` string, hostile backend `Diagnostic`/`Failed`/`Stdout`/`Stderr` frames, attacker-influenced package name/path/adapter id, command/env/secrets, raw backend *process* stderr) to every public surface (ToolResult content+details/diagnostics, NDJSON, RPC, doctor, TUI, tracing, `Debug`/`{:?}`). All are contained: the failure envelope interpolates only safe identifiers; `From<ActivationError>` drops untrusted detail; backend crash stderr is bounded (`STDERR_CAP`) and discarded (`let _ =`); target stdout/stderr are legitimately in-band command output; embedder boundaries apply `redact(_, Summary)`. The items below are hardening / latent / asymmetry observations.

### 3.1 MINOR: `full_output` spill path carries the opi PID into model-visible `ToolResult.details` (pre-existing; redacted at embedder boundary)

**File:** `crates/opi-coding-agent/src/tool/operations.rs` (:1100–1101, :1241–1251), consumed at `tool/bash.rs` (:226–259)
**Cause:** When merged target stdout+stderr exceeds 64 KiB, the local backend spills to `temp_dir().join(format!("opi-bash-output-{pid}-{nanos}-{counter}.log"))` and surfaces that absolute path + the opi PID in `details.full_output`, lifted into `ToolResult.details` (which crosses to the model before the event-boundary redaction runs).
**Impact:** Host-generated (opi's own PID/temp path), not hostile. It is scrubbed at NDJSON/RPC event boundaries (pinned by `rpc_jsonl.rs:3013`), so it does not reach embedders. It is pre-existing Phase 11 behavior, not a Phase 16 regression. The spec's "omit PIDs and unnecessary absolute paths" guidance is met at public/embedder surfaces but not in the model-visible details block.
**Fix:** Optional hardening — drop the PID from the spill filename (counter+nanos already guarantee uniqueness), or apply `redact_text`/`redact_public_value` to `ToolResult.details` before provider conversion.

### 3.2 MINOR: `adapter_command` reaches `opi package list --json` stdout unredacted (asymmetry vs doctor)

**File:** `crates/opi-coding-agent/src/package_cli.rs`
**Lines:** 884–888 (`list_package_json`)
**Cause:** `list_package_json` emits the manifest-declared `adapter_command` and resolved `adapter_resolved_command`/`package_root` verbatim, with no `redact()`/`redact_public_value()`. A hostile package author controls these strings via the shipped manifest. The sibling `doctor` path *does* redact via `redacted_payload(Summary)` (`doctor.rs:306`).
**Impact:** A user reviewing an installed crafted package sees attacker-controlled text unredacted in `list --json`. Strong mitigations: the user already installed it via `package add`, and `list` is a review/audit tool whose purpose is to show what was installed (redacting would hide the threat the user is evaluating). The list-vs-doctor asymmetry is the real concern.
**Fix:** Route `list_package_json` through `redact_public_value`/`redact_summary_paths` for consistency with `doctor`, or document that the unredacted value is intentional for auditability.

### 3.3 MINOR: Multi-layer `[execution.permissions]` uses whole-map REPLACE — a partial explicit `--config` silently wipes user-layer `deny`

**File:** `crates/opi-coding-agent/src/config.rs`
**Lines:** 1051–1067 (`merge_into`), layers user → project → explicit
**Cause:** `merge_into` applies `permissions` with whole-map REPLACE (`config.execution.permissions = v;`). A user with `local = "deny"` in USER config who runs `opi --config c.toml` where `c.toml` carries any `[execution.permissions]` table (even `{opi-sandbox="allow"}`) ends up with that map replacing the accumulated user map — `local` reverts to its default (`Allow`) with no warning.
**Impact:** A persistent safety guard can be silently unset by a partial explicit-config permissions table the user did not realize was complete-replace. Not a project-layer vector (project permissions are rejected). The behavior is documented ("REPLACE-if-present") but unpinned by a test, and the security relevance of the permissions map makes the footgun sharper than for single-value strategy/backend.
**Fix:** Add a regression test loading user `local="deny"` + explicit `"opi-sandbox"="allow"` through `resolve_config` and pinning the resulting map (documenting REPLACE explicitly). If the design intent is key-level merge for safety, merge entry-by-entry instead.

### 3.4 MINOR: Redaction sweep test uses safe values; does not prove `Display` redacts canaries placed in interpolated `adapter_id`/`name` fields

**File:** `crates/opi-coding-agent/tests/execution_failures.rs`
**Lines:** 269–288
**Cause:** `redaction_safe_across_all_declared_codes` constructs each variant with hard-coded safe values. The `Display` impls for `PolicyDenied`/`PermissionDenied`/`PermissionRequired`/`PackageNotInstalled`/`PackageUntrusted`/`ContributionDisabled` interpolate `{adapter_id:?}`/`{name:?}`. Because the canaries are never placed into these `String` fields, the sweep proves "safe input → safe output", not "canary input → redacted output".
**Impact:** No live leak: production populates these from validated package-store identities. But a future change routing a less-vetted string into `adapter_id`/`name` would leak via `Display` and this test would not catch it.
**Fix:** Inject each `REDACT_CANARY` into the `String` field of each variant that carries one and assert neither `Display` nor `remediation()` contains the canary, matching the strength of the existing `AdapterNotSelected` test.

### 3.5 INFO: `AdapterNotSelected.requested` is a public field with raw model input; `Debug` derive prints it verbatim

**File:** `crates/opi-coding-agent/src/execution/failure.rs`
**Lines:** 40–107 (struct + `#[derive(Debug)]`), 75–78 (`requested: String`)
**Cause:** `Display` and `remediation()` correctly substitute `REDACTED_BACKEND_PLACEHOLDER` and never interpolate `requested`, but the field is public and `Debug` prints it verbatim. `exec_failure_to_bash_op_error` drops the field entirely (only code/Display/remediation/adapter_id are read).
**Impact:** No production leak: a grep found no non-test `{:?}` site debug-prints an `ExecutionFailure`. Latent risk only — a future `tracing::error!("{:?}", failure)` would reintroduce a leak.
**Fix:** Document that the redaction contract covers only `Display`/`remediation`/`code` (not `Debug`/field access), or drop the raw payload in favor of a boolean/placeholder.

---

## 4. Test-quality findings

### 4.1 MINOR (downgraded from Major): git-install crash-safety ordering verified only by a degenerate source-text test; two rollback edges untested

**File:** `crates/opi-coding-agent/tests/package_cli.rs` (:1591–1610); ref `src/package_cli.rs` (`install_git_package` :195–333)
**Cause:** `git_update_invalidates_trust_before_live_cache_swap` uses `include_str!("../src/package_cli.rs")` and string-searches for `"prepare_activation_update("` and `"stage_cache_replacement("` to assert textual ordering — the documented L-D3 degenerate pattern (it would pass on no-op bodies). The runtime ordering is **correct**: `prepare_activation_update` durably writes `trusted=false`/`enabled=false` (`package_activation.rs:644–646`) **before** `stage_cache_replacement` swaps the live cache (`package_cli.rs:262` then `:278`); every crash boundary between/after leaves the package untrusted+disabled, and `PendingCacheReplacement::Drop` backs this. The primary (publish-failure) rollback (`:312–321`) **is** behaviorally tested — `package_add_git_metadata_write_failure_preserves_existing_lock_and_cache` (`:609–656`) sets `packages.toml` read-only between two git installs, forcing the publish to fail after the cache swap, then asserts `git_head(&package_root)==first_commit` (`:654`), proving `replacement.rollback()` restored the cache.
**Impact:** The two remaining rollback edges — `stage_cache_replacement` failure (`:280–283`) and `canonicalize` failure (`:287–294`) — have no dedicated behavioral test, and the ordering invariant is pinned only by a brittle source-text assertion. These are hard-to-trigger paths on correct code. A future refactor could silently break the textual ordering or those edges.
**Fix:** Replace (or augment) the source-text test with a behavioral fault-injection test that fails *inside* `stage_cache_replacement` after the trust-disable write committed and asserts `package-trust.toml` is untrusted+disabled while the live cache still holds the old bytes. Add focused tests for the two untested rollback edges via a stage/canonicalize-failure seam. No runtime code change needed.

### 4.2 MINOR: `danger_blocklist` unit test verifies only 9 of 14 syscalls

**File:** `crates/opi-sandbox/src/platform/linux.rs`
**Lines:** 391–416
**Cause:** `danger_blocklist_is_fixed_and_io_uring_free` iterates a 9-element required list; `danger_syscalls()` returns 14 (also `kexec_load`, `kexec_file_load`, `init_module`, `finit_module`, `delete_module`). Removing any of those 5 would not fail the test.
**Impact:** A future edit dropping the kexec/module syscalls from the seccomp baseline would pass CI silently.
**Fix:** Extend the required list to all 14 entries (or assert length 14 on x86_64).

### 4.3 MINOR: No `LockMaterial` TOML round-trip test for all 8 fields

**File:** `crates/opi-coding-agent/tests/execution_package_lifecycle.rs`
**Lines:** 117–140
**Cause:** `valid_contribution_yields_exact_lock_material` asserts all 8 fields in-memory; no test writes a populated `PackageLockEntry`, reads it back via `read_lock`, and asserts all 8 fields survive TOML serialize/deserialize (with hyphens/dots).
**Impact:** A serde rename/typo or a non-ASCII-deserialization quirk would not be caught; `revalidate_lock` drift comparison relies on `PartialEq` over deserialized bytes.
**Fix:** Add a write→read→assert-field-for-field round-trip test.

### 4.4 MINOR: No negative tests for missing required contribution fields (`adapter_config`, `handshake_timeout_ms`)

**File:** `crates/opi-coding-agent/tests/execution_contribution_manifest.rs`
**Lines:** 386–412
**Cause:** These fields lack `#[serde(default)]`, so omission correctly yields `Malformed`; value-range and unknown-field rejections are tested, but missing-required-field rejection is not.
**Impact:** Adding `#[serde(default)]` later would weaken the gate with no test failure.
**Fix:** Add two manifest tests omitting each field, asserting `ContributionValidationError::Malformed`.

### 4.5 MINOR: No `LineReader` clean-EOF / final-line-without-newline test

**File:** `crates/opi-protocol/tests/execution_v1_contract.rs`
**Lines:** ~301–315
**Cause:** `LineReader::read_line` documents `Ok(false)` (clean EOF) and `Ok(true)` at EOF with a partial line; only the oversized and at-cap paths are tested.
**Fix:** Add tests for empty input (`Ok(false)`), `b"abc"` no newline (`Ok(true)`), and a second read after `b"abc\n"` (`Ok(false)`).

### 4.6 MINOR: `cli.rs` doc claims `InvalidRequest` is "unreachable from the human CLI" — false for nonexistent workspace/cwd

**File:** `crates/opi-sandbox/src/cli.rs`
**Lines:** 299–312
**Cause:** `parse_run` only validates the `--workspace` token is non-empty/flag-shaped; it does not stat the path. `SandboxRunner::run` then calls `workspace.canonicalize()` (`runner.rs:323–328`), which fails for nonexistent paths and returns `SetupFailed{InvalidRequest}`, mapped to exit **2** (usage) not **125** (pre-start).
**Impact:** On a supported platform, a nonexistent `--workspace` returns exit 2, contradicting the docstring's exit table. A script checking 125 for "setup failed" mis-classifies a path-existence failure as usage.
**Fix:** Correct the docstring, or split the variant (e.g. `WorkspaceNotFound`) and map canonicalize failures to 125.

### 4.7 MINOR: `build_trust_display` sets `executable_rel_path` to the absolute canonical path (inconsistent with field name and `list --json`)

**File:** `crates/opi-coding-agent/src/package_activation.rs`
**Lines:** 694–703
**Cause:** Maps `executable_rel_path: v.command.display()` where `v.command` is the canonical *absolute* path; the lock material's `executable_rel_path` is the relative raw command, and `list_package_json` surfaces the relative path.
**Impact:** Inconsistent identity display between `package enable` (absolute) and `package list --json` (relative) under a field named `..._rel_path`. Not a redaction violation.
**Fix:** Set it to `v.lock.executable_rel_path.clone()`, or rename the field and intentionally surface the canonical path.

### 4.8 MINOR: `real_store_wiring` test fixture uses `enabled_identities()` instead of production's `usable_enabled_identities()`

**File:** `crates/opi-coding-agent/tests/execution_product.rs`
**Lines:** 597–624
**Cause:** Production `routed_store_state` filters trusted+enabled records by target/opi-version compatibility (`usable_enabled_identities`); the fixture uses the unfiltered `enabled_identities()`.
**Impact:** The SC16-13 slice does not exercise the startup target/version filter (silent today because the fixture's adapter matches the host). The per-invocation `activate()` gate still catches mismatches at exec time, so this is a realism gap, not a hole.
**Fix:** Switch the fixture to `usable_enabled_identities(host_target_triple(), host_opi_version())` and add a focused test that a target-mismatched enabled adapter is filtered out at startup.

### 4.9 MINOR: Harness startup broker-installation for an external-ask adapter through the real constructor is not exercised end-to-end

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 1122–1140
**Cause:** The local-ask broker-install shape is covered (`interactive_ask_real_constructor_installs_permission_broker`); the external-ask + `GeneralRouted` + `Interactive` shape is covered at the runtime layer (broker injected directly into `ExecutionWiring`), but no test drives the real `CodingHarness::builder().build()` for that shape.
**Fix:** Add a harness unit test with a routed-store override returning one enabled external identity + external ask policy + `Interactive`, asserting `brokers()==1`, `permission_managers()==1`, and the rx/manager survive into the harness fields.

### 4.10 MINOR: Permission-prompt snapshot coverage is asymmetric (local/no-package only at 80×24)

**File:** `crates/opi-tui/tests/permission_prompt_snapshots.rs`
**Lines:** 42–66
**Cause:** The external variant is snapshotted at both 80×24 and 120×40; the local/no-package variant (different render branch) only at 80×24. The DoD calls for deterministic 80×24 and 120×40 snapshots.
**Fix:** Add `permission_prompt_local_no_package_120x40` and commit the `.snap` after review.

### 4.11 MINOR: Prompt widget snapshots render into the full buffer, not the production centered 70%×50% overlay rect

**File:** `crates/opi-tui/tests/permission_prompt_snapshots.rs`
**Lines:** 15–32
**Cause:** Snapshots render the widget at `f.area()`; production renders inside `centered_rect(70, 50, frame.area())` (`interactive.rs:833`) — ~56×12 on 80×24. The committed `.snap`s pin only the widget in isolation.
**Impact:** A regression in the centered overlay/layout under the smaller sub-rect would not be caught.
**Fix:** Add an integration-level snapshot through the production `draw_state` path (or document the snapshots as widget-level + add a `centered_rect` assertion).

### 4.12 MINOR: `crate_boundaries` PATH-read whitelist is a brittle exact-string match

**File:** `crates/opi-sandbox/tests/crate_boundaries.rs`
**Lines:** 79–90
**Cause:** The tripwire permits the legitimate `std::env::var_os("PATH")` read by string-replacing exactly that token before grepping. A different call shape (`use std::env; env::var_os("PATH")`, or a computed key) either false-positives or bypasses.
**Fix:** Add a companion negative test asserting known-bad patterns (`var_os("OPI_SESSIONS_DIR")`) are caught, or move to a `syn`-aware scan.

### 4.13 MINOR: `linux_policy.rs` / `macos_policy.rs` network tests assume `python3` on PATH without documenting it

**File:** `crates/opi-sandbox/tests/linux_policy.rs` (:200–257), `macos_policy.rs`
**Cause:** Several network/io_uring/AF_UNIX sentinels shell out to `python3 -c '...'`; the file preambles document only the Landlock-ABI / sandbox-exec requirement, not python3. The io_uring test also hard-codes syscall 425 (x86_64).
**Impact:** On a supported Linux/macOS host without python3 (minimal containers), these tests fail spuriously with NotFound-style errors that look like confinement failures.
**Fix:** Add a python3 availability guard that skips with a clear message; use `libc::SYS_io_uring_setup` for arch-correctness; update the preamble.

### 4.14 MINOR: `outside_write_denied` test discards run output and asserts only marker non-creation

**File:** `crates/opi-sandbox/tests/linux_policy.rs`
**Lines:** 152–173 (and macOS twin `macos_policy.rs:174–190`)
**Cause:** Binds `let _ = out;` and asserts `!marker.exists()`. Sound (a Landlock regression would create the marker), but does not exercise the denial's exit-code/error-class surface.
**Fix:** Also assert `!out.status.success()` and optionally a permission-denied-class stderr.

### 4.15 MINOR: `OwnedCaptureTask::take_capture` uses `unreachable!()` (panic) if the drain-task `Arc` is not yet unique

**File:** `crates/opi-coding-agent/src/tool/supervision.rs`
**Lines:** 355–361
**Cause:** `Arc::try_unwrap(capture).unwrap_or_else(|_| unreachable!(...))` relies on the drain task having released its clone. That holds today on the Ok/Err and timeout paths (task completed or aborted+awaited).
**Impact:** Forward-looking only: a future refactor adding another clone or a partial-drop path turns this into a runtime panic inside the production supervision path.
**Fix:** Replace with a non-panicking snapshot (lock the mutex, clone the `StreamCapture`, let the `Arc` drop naturally).

### 4.16 MINOR: `release-topology` guard does not pin the `python3` interpreter prefix on the release-audit invocation

**File:** `crates/opi-coding-agent/tests/opi_sandbox_release_topology.rs`
**Lines:** 303–332
**Cause:** Asserts the `sandbox_release_audit` job contains `opi-artifact-audit.py` and `--release` but not the `python3` interpreter token that `release.yml:253` actually uses.
**Fix:** Add `"python3"` to the `assert_present` needle list (and consider pinning the `evidence` positional).

### 4.17 MINOR: Artifact-audit gate/six-target evidence bundles use unbound text markers with no run-identity binding

**File:** `scripts/opi-artifact-audit.py`
**Lines:** 1086–1235 (`_audit_gates_bundle`, `_audit_six_target_bundle`)
**Cause:** Native smoke markers are cryptographically bound to the archive SHA-256 (`DIRECT_SMOKE_RE`/`BACKEND_SMOKE_RE`/`NATIVE_SENTINEL_SMOKE_RE` require `archive_sha256=<digest>`). The gate/six-target bundles accept cargo text markers (`test result: ok. N passed; 0 failed`, `Finished \`...\` profile`, `PASS`) with no binding to a build identity/digest. A text file with those literals passes the gate.
**Impact:** Under the operator-trust model (genuine preserved evidence) this is acceptable, and failure-marker precedence is sound. The native-archive topology is genuinely fail-closed; the workspace-gate half is text-trust only — not "genuinely fail-closed" against a self-deceiving operator.
**Fix:** If fail-closed-ness is a release-gate requirement, bind gate/six-target captures to a run identity (commit SHA or artifact hash) and validate it like the native smokes; otherwise document the asymmetry.

### 4.18 INFO: Headless model-strategy `ask` lacks an end-to-end production-path test asserting `permission_required` in the `ToolResult`

**File:** `crates/opi-coding-agent/tests/execution_product.rs`
**Lines:** 840–890 (`routed_tool_result` hard-codes `RunMode::Interactive`)
**Cause:** The headless (Rpc/NonInteractive) leg of a model-strategy external `ask` is exercised only at the unit level; no single test drives it through `CodingHarness::build_tools` → `RoutedBashOperations::exec` → `resolve_selection` → `PermissionRequired` passthrough and asserts the `ToolResult` carries the stable code.
**Impact:** The invariant holds by composition of unit-tested pieces (no behavioral hole).
**Fix:** Parameterize `routed_tool_result` by `RunMode` and add one assertion for the headless ask path.

### 4.19 INFO: `FixedChoiceBroker` is dead code

**File:** `crates/opi-coding-agent/src/execution/permission.rs`
**Lines:** 179–197
**Cause:** Declared `pub`, impls `InteractivePermissionBroker`, but no production code or test constructs it (tests use `RecordingBroker`; harness installs `TuiPermissionBroker`); not re-exported from `execution/mod.rs`.
**Fix:** Delete it, or re-export + add a test if intended as an embedder utility.

---

## 5. Spec-compliance findings

### 5.1 MINOR: Empty or unknown-field-only `[sandbox]` table is silently accepted (contract says rejected)

**File:** `crates/opi-coding-agent/src/config.rs`
**Lines:** 704–739
**Cause:** `TomlSandbox` is `#[serde(default)]` with no `deny_unknown_fields`; `legacy_sandbox_rejection` rejects only when `is_present()` (any of mode/require/fs/network/syscalls is set). serde cannot distinguish an absent `[sandbox]` from a present-but-empty one for an all-Option all-default struct, and unknown fields are silently dropped. So `[sandbox]` (empty) and `[sandbox]\nunknown="x"` both deserialize to default with `is_present()==false` and are accepted.
**Impact:** Functionally harmless (an empty/unknown-only table configures nothing and reintroduces no sandbox behavior), but it deviates from the strict contract wording ("`[sandbox]` is rejected") and is an untested edge — `execution_migration.rs` tests only non-empty tables.
**Fix:** Add `#[serde(deny_unknown_fields)]` and make `TomlConfig.sandbox` an `Option<TomlSandbox>` so `legacy_sandbox_rejection` rejects any `Some(_)` (covers empty + unknown-field tables); or explicitly accept this and add a test pinning the field-presence-based design.

### 5.2 INFO: Per-run Linux confinement reports only `Mechanism::Landlock` despite both Landlock and seccomp being applied

**File:** `crates/opi-sandbox/src/platform/linux.rs` (:329–332) + `lib.rs:28–29`
**Cause:** `AppliedRestriction` carries a single `Mechanism`; `LinuxRestriction::prepare` returns `Landlock` even though it also installs the seccomp overlay. `lib.rs` documents that a native Linux run "reports `Mechanism::Landlock`/`Mechanism::Seccomp`", overstating the per-run `Started` vocabulary (doctor does list both).
**Impact:** No confinement is weakened — only the per-run reported vocabulary. A consumer observing only `Started` must consult `doctor` to learn seccomp is engaged.
**Fix:** Narrow the `lib.rs` comment to "the doctor report lists both; the per-run Started reports Landlock as the lead mechanism", or extend `AppliedRestriction` to carry both.

### 5.3 INFO: Runtime resolver does not re-validate project-local `[[contributions.adapters]]` (inert contributions silently)

**File:** `crates/opi-coding-agent/src/package_resolver.rs`
**Lines:** 292–432
**Cause:** `resolve_declaration`/`resolve_git_declaration` parse `[[contributions.adapters]]` as raw tables but never call `validate_executable_contributions`. The project-local gate is enforced only at `package add --local`; a hand-edited project package with contribution tables resolves normally and silently ignores them.
**Impact:** Not exploitable: activation is global-only, so a project-local contribution can never activate or be model-selected. But it violates the spirit of the gate (install-time-only) and a user could believe a project contribution is active when it is inert.
**Fix:** Validate during discovery with `PackageSource::ProjectLocal` and surface a diagnostic, or document the install-time-only gate and add a `doctor` diagnostic for project manifests carrying non-empty `adapter_contributions`.

---

## 6. Invariant verification

All Phase 16 invariants hold in code; test-coverage caveats are noted.

| Invariant | Code evidence | Test coverage |
|-----------|--------------|---------------|
| Minimal Runtime: default local direct, no sentinel/package/router/permission/protocol task, byte-identical schema | `execution/runtime.rs` `ExecutionRuntime::build` DirectLocal branch; `harness.rs` `harness_execution`; `tool/bash.rs` schema | `execution_minimal_runtime.rs::default_allow_real_constructor_opens_no_extended_execution_state` (real constructor, 0 managers/brokers/routers + panic-on-call store); `tool_assembly_minimal_runtime_preserves_schema_and_runs_local_backend` (compares vs fresh schema) |
| Five independent gates; model cannot mutate any | `package_activation.rs` (install/enable/disable/remove/activate); `execution/router.rs` `resolve_selection` (pure, non-mutating) | `execution_package_lifecycle.rs` (install→untrusted+disabled, type-the-name enable, disable-preserves-trust, drift-invalidates, collision-rejected); `execution_routing.rs::model_cannot_mutate_permission_or_trust` |
| Fail-closed after selection; no local fallback | `runtime.rs` `RoutedBashOperations::exec` / `ProcessCommandAdapter::exec`; no `Local` variant in `failure.rs`; structural `no_local_fallback_exists` guard | `execution_runtime.rs::routed_external_activation_failure_does_not_fall_back_to_local` (call_count==0); `execution_product.rs::disabled_packaged_adapter_is_contribution_disabled_without_fallback`; `execution_routing.rs::rules_selected_backend_failure_does_not_fall_through` |
| No degraded success (timeout/cancel are errors on clean exit) | `tool/bash.rs:269` `is_error = timed_out \|\| cancelled \|\| signal.is_some() \|\| exit_code != Some(0)`; host force-sets `cancelled=true` on cancel-raced Completed | `execution_product.rs::timed_out_in_band_completed_is_not_a_success`, `cancelled_in_band_completed_is_not_a_success` |
| 14-code redacted failure envelope; no `Degraded` variant | `execution/failure.rs` (14 codes, `remediation()`); `From<ActivationError>` drops detail | `execution_failures.rs::all_14_codes_declared_with_stable_literal`, `remediation_is_distinct_across_all_14_codes`, `redaction_omits_*`; (caveat: S4 — sweep uses safe values) |
| Every frame carries one host-generated request id; cross-id/dup/cumulative enforced | `execution/protocol_host.rs` seeds `Session`; `session.rs` `check_id`/`check_duplicate`/`account_output` | `execution_protocol_host.rs::cross_request_id_is_protocol_violation`; `session.rs` unit tests; `cross_request_output_is_rejected_before_accounting` (remediation-fixed ordering) |
| `started` flushed before target release; command undisclosed until `ready` | `protocol_host.rs` transition ordering + start gate; `helper.rs` (sandbox side) | `ready_identity_version_and_target_must_match_lock`; `target_cannot_act_until_started_has_been_observed`; ten protocol-violation surfaces |
| Backend stderr bounded crash evidence only; never in payload | `protocol_host.rs::drain_stderr` (capped, discarded); `redact_backend_diagnostic` | `backend_stderr_canary_never_surfaces` (Display + Debug); `protocol_conformance.rs::failed_frame_is_redacted` |
| Protocol stdin never inherited as target stdin | `StdinPolicy::Null` for backend; `helper.rs` pins it | `helper.rs::build_request_pins_stdin_to_null`; `cli_contract.rs::execute_stdin_null_target_receives_eof` |
| Cleanup on every terminal path + guard drop (opi-sandbox) | `runner.rs` `SandboxRun` Drop (kill_on_drop + TreeGuard + TempDir); no `.await` between spawn and guard | `sdk_contract.rs` timeout/cancel/drop/hard-kill + grandchild-kill |
| Crate boundaries: core links no opi-sandbox/native-policy; opi-sandbox depends only on opi-protocol | `Cargo.toml` graphs; `src/sandbox*` deleted | `phase16_crate_boundaries.rs::cargo_tree_proves_no_sandbox_or_native_policy_dependency` + `no_legacy_sandbox_symbols_in_production_source` |
| Migration: `[sandbox]`/`--sandbox`/`--sandbox-require` rejected without aliases; L0 stays in core | `cli.rs`/`config.rs` rejection; `supervision.rs`/`process_tree.rs` retained | `execution_migration.rs` (off/strict/require/bare rejected at every layer + stable diagnostic); (caveat: SC1 — empty table accepted) |
| Permission grants memory-only; do not survive resume/fork/branch | `PermissionManager` in-process `HashSet`; `reset_grants` at session-switch boundaries | `interactive_permission.rs` + `harness.rs::session_switches_reset_permission_grants_at_production_call_sites` |
| Native honesty: `restricted` (never `isolated`) for opi-sandbox; `supervised` for local | `helper.rs::started_payload`; `windows.rs` unsupported posture | `started_payload_*` tests; `windows_execution_posture.rs::local_exec_reports_supervised_guarantee`; linux/macos doctor asserts `!isolated` |

---

## 7. Cross-task integration findings

### 7.1 MINOR: `router.rs` module doc contradicts the runtime's `always-available=true` eligibility construction

**File:** `crates/opi-coding-agent/src/execution/router.rs`
**Lines:** 31–35 (cf. `execution/runtime.rs:222–228`)
**Cause:** The `EligibleAdapter` docstring says the runtime builds these "installed + trusted + enabled + target-compatible" and "`available` reflects everything except permission". But `Eligibility::from_enabled` (`runtime.rs:229–244`) hardcodes `available: true` for every entry and defers all availability to per-invocation `IdentitySource::activate()` (documented correctly at `runtime.rs:222–228`).
**Impact:** A reader of the public routing surface is misled into thinking `available` is a meaningful pre-spawn signal; the `gate()` `AdapterUnavailable{Ineligible}` branch is effectively unreachable from production (only from hand-built fixtures). Documentation/code consistency defect, not a behavior defect.
**Fix:** Update `router.rs:31–35` to state that production sets `available=true` and availability is re-gated per invocation by `activate` (cross-reference `runtime.rs:222–228`).

### 7.2 INFO: Two distinct `LEGACY_SANDBOX_REMEDIATION` constants (private `cli.rs` vs public `diagnostics.rs`) with different wording

**File:** `crates/opi-coding-agent/src/cli.rs` (:15–20) vs `src/diagnostics.rs` (:20)
**Cause:** The two constants legitimately name different removed inputs (`--sandbox flag` vs `[sandbox] section`). Only the public one has a byte-identity pin; the private one is pinned only via a substring-needle check.
**Fix:** Optional — extract the shared remediation tail into one constant and compose the per-surface prefix at the call site, or add a test that the `cli.rs` constant also contains `REMEDIATION_NEEDLES`.

### 7.3 INFO: `bash_input_schema` duplicates `Eligibility::model_visible_ids` filter logic

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 124–148
**Cause:** `bash_input_schema` re-implements the `available && !Deny` filter (plus an `Ask` annotation) that `Eligibility::model_visible_ids` (`router.rs:58–64`) already implements. The two agree today.
**Fix:** Extend `model_visible_ids` to return the ask flag and have `bash_input_schema` call it, or document the intentional duplication.

### 7.4 INFO: Outer `PackageLockEntry.manifest_sha256` uses raw bytes while inner `LockMaterial.manifest_hash` uses LF-normalized bytes

**File:** `crates/opi-coding-agent/src/package_resolver.rs` (:210–215) vs `execution/contribution.rs:631–644`
**Cause:** A deliberate two-layer design (outer raw hash for resolver drift; inner LF-normalized for activation drift), each internally consistent.
**Impact:** The outer resolver drift check can false-positive across CRLF/LF checkouts on Windows, disabling a package at runtime until re-lock. No correctness bug in the contribution/trust layer.
**Fix:** Consider LF-normalizing the outer hash too (reuse `contribution::lf_normalize`) for cross-platform stability, or document the two-hash design.

---

## 8. Residuals and recommendations

### Host-verifiability limits (not defects)
This audit ran on Windows. All `#[cfg(unix)]` / Linux / macOS code and tests compiled out locally and were reviewed as source: the Landlock/seccomp FFI in `opi-sandbox/src/process_tree.rs` (install order seccomp → `restrict_self` → fd-closure is correct, with the documented rationale that ruleset fds are consumed before close), the sandbox-exec profile in `platform/macos.rs` (last-match-wins on an allow-default base; `escape_path` neutralizes `\`, `"`, `$`; fail-closed on missing/rejected helper), and the unix process-group SIGKILL path. The **behavioral** native enforcement (Landlock actually denies writes, seccomp returns EPERM, sandbox-exec denies network) was not directly re-verified here; it rests on the phase-exit evidence (`linux_policy`/`macos_policy` on WSL2/GHA per the task ledger) and the artifact audit. Per §4.17, the artifact-audit's gate/six-target half is text-trust; the native-archive half is SHA-bound and fail-closed.

### Pre-existing, out-of-Phase-16-scope items noted
- The `bash` result **details block** carries the raw command text (the model's own input echoed back) and the `full_output` spill temp-path/PID (§3.1). Both are pre-existing Phase 11 behavior; Phase 16's redaction scope targets diagnostics/envelopes (which are clean). Worth a future cross-phase pass on `ToolResult.details` redaction before provider conversion.

### Priority recommendations
1. **(Minor, correctness)** Close the L0 reap asymmetry (§2.1) and bound Diagnostic frames (§2.2) — both are quick, both harden long-running sessions.
2. **(Minor, security-semantics)** Pin the permissions REPLACE contract with a test, or switch to key-level merge (§3.3); strengthen the redaction sweep to inject canaries into the interpolated fields (§3.4).
3. **(Minor, test fidelity)** Replace the degenerate git-install ordering test with a behavioral fault-injection test and cover the two untested rollback edges (§4.1); extend the danger-blocklist test to all 14 syscalls (§4.2).
4. **(Minor, contract)** Decide and pin the `[sandbox]` empty-table behavior (§5.1) and the artifact-audit gate/six-target run-identity binding (§4.17) — both are about whether "fail-closed/rejected" means what the docs imply.
5. **(Info, doc drift)** `router.rs` eligibility doc (§7.1), `feed_*_line` rustdoc (§2.3), `cli.rs` `InvalidRequest` doc (§4.6), `lib.rs` per-run mechanism vocabulary (§5.2).

### Items verified clean (no action)
Redaction across all public surfaces (no leak); fail-closed/no-fallback/no-degraded-success end-to-end (including `assert_invariants` release-safety and TOCTOU mitigation via `/proc/self/fd`, `/dev/fd`, and `FILE_SHARE_READ`-only Windows handle); five-gate independence; request-id correlation and the post-remediation cumulative-accounting ordering; byte-for-byte Minimal-Runtime schema; `opi-sandbox` invocation-stateful/cross-invocation-stateless cleanup; crate boundaries and the migration rejection surface; Non-Goal compliance (no Docker/VM/SSH/Gondolin/remote adapters, no file/navigation routing, no core-tool shadowing, no universal-protocol/RPC/NDJSON/trace migration, no dynamic native-library loading, no multi-adapter composition, no host-read/env confidentiality claim, no extension-process sandboxing, no publisher checksum auth, no project-local executable activation, no Windows AppContainer/restricted-token, no Phase 15 alias preservation).
