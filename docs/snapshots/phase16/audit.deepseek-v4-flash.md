# Phase 16 Pluggable Extensions and Command Execution -- Independent Code Audit

**Auditor**: deepseek-v4-flash (independent; no prior audit reports consulted)
**Date**: 2026-08-06
**Scope**: Tasks 16.1-16.16.3, commits `1021842..edd8d91` (phase tasks through `f8aff02`, plus the two post-archive remediation commits `2b23010` and `edd8d91` on branch `codex/phase16-remediation-2`)
**Method**: Full read of the canonical design spec and `docs/opi-spec.md` Phase 16 section; personal deep reads of the execution substrate (`protocol_host`, `runtime`, `failure`, `permission`, `router`, `contribution`, `supervision`, `process_tree`, `bash`, config/validation, harness wiring, CI/release workflows, opi-sandbox `helper`/`backend`, opi-protocol `session`/`bounds`); four parallel file-group subagents for full reads of opi-protocol, opi-sandbox, the opi-coding-agent test suite, and package/startup/diagnostics sources; git-history review of the remediation commits. The branch carries two remediation commits after the archived phase-exit; this audit assesses the current post-remediation tree at HEAD. The pre-existing `audit.codex.md` / `audit.deepseek-v4-flash.md` were not consulted (contamination isolation); the stale `audit.deepseek-v4-flash.md` was superseded.

---

## 1. Executive Summary

**Verdict: PASS-WITH-FINDINGS**

| Severity | Count |
|----------|-------|
| Blocker  | 0     |
| Major    | 1     |
| Minor    | 18    |
| Info     | 16    |

The phase is functionally sound and the remediation has substantially hardened the protocol-host/runtime/package surfaces. The core DoD invariants -- five independent lifecycle gates, no-fallback, no-degraded-success, request-id correlation, L0 supervision, atomic start gate, protocol-stdin isolation, honest `started`/doctor vocabulary, redacted failure envelope, and crate boundaries -- all hold in the shipped code and are behaviorally proven through production call sites. The single Major is a demonstrable arithmetic error in the `opi-protocol` `Bounds::validate` base64-inflation formula; it does not affect the shipped `Bounds::DEFAULT` (huge slack), but it breaks the documented sizing contract for third-party hosts, which the design explicitly anticipates. The Minors are concentrated in opi-sandbox protocol-edge semantics (initialize wait not deadline-bounded, `accepted` emitted before validation, `cleanup_unconfirmed` reported without polling for confirmation), a cross-surface TUI gap (startup execution diagnostics never rendered), and test-suite robustness. None of the findings are Blocker-level; the phase meets its exit criteria with fixable residual risk.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 16.1 | Pin the Phase 16 documentation contract | PASS |
| 16.2 | Pin L0 supervision and define the policy-neutral seam | PASS |
| 16.3 | Add opi-protocol::execution::v1 | PASS-WITH-FINDINGS (1 Major + 6 Minor) |
| 16.4 | Parse and hard-gate executable contributions | PASS |
| 16.5 | Add Package Trust and enable/disable lifecycle | PASS |
| 16.6 | Add execution configuration, failures, routing, and permission policy | PASS |
| 16.7 | Implement the one-shot execution protocol host | PASS |
| 16.8 | Build the deep Execution Runtime assembly | PASS |
| 16.9 | Wire Execution Runtime, dynamic bash schema, and public surfaces | PASS-WITH-FINDINGS |
| 16.10 | Add the interactive permission broker and TUI prompt | PASS-WITH-FINDINGS |
| 16.11.1 | Build the standalone opi-sandbox SDK and runner | PASS |
| 16.11.2 | Build the human opi-sandbox CLI and direct smoke | PASS-WITH-FINDINGS |
| 16.12 | Add the atomic helper gate and protocol backend | PASS-WITH-FINDINGS |
| 16.13 | Port the Linux native restriction contract | PASS-WITH-FINDINGS |
| 16.14.1 | Port the macOS native restriction contract | PASS-WITH-FINDINGS |
| 16.14.2 | Pin the Windows unsupported execution posture | PASS |
| 16.15.1 | Build host-neutral opi-sandbox packaging | PASS |
| 16.15.2 | Wire native package CI, release, and artifact audit | PASS |
| 16.16.1 | Remove core native sandbox and enforce migration boundaries | PASS |
| 16.16.2 | Prove install-to-execute and cross-surface diagnostics | PASS-WITH-FINDINGS |
| 16.16.3 | Synchronize documentation and close Phase 16 repository gates | PASS-WITH-FINDINGS |

---

## 2. Correctness Findings

### 2.1 MAJOR: `Bounds::validate` undercounts base64 inflation; the documented sizing formula is wrong

**File:** `crates/opi-protocol/src/execution/v1/bounds.rs`
**Lines:** 55-68 (formula); error/doc text at 34-35
**Cause:** The consistency check computes `chunk_required = (max_decoded_chunk_size * 4 + 2) / 3 + 64`, i.e. `floor((4n+2)/3)`. Standard base64 of `n` bytes is `4 * ceil(n/3)` characters, which is **larger by 2 for `n mod 3 == 1` and by 1 for `n mod 3 == 2`**. Verified numerically: for `n = 1_048_576` (the shipped 1 MiB default chunk), the formula reserves `1_398_102` but real base64 is `1_398_104`; for `n = 4` it reserves 6, real is 8. Both the doc comment (`"max_line_size must be >= ceil(max_decoded_chunk_size * 4/3) + framing"`) and the `BoundsError` display repeat the wrong formula.
**Impact:** A host that sizes `max_line_size` to exactly the documented formula (which `Bounds::validate` accepts) will hit a spurious `CodecError::OversizedLine` when `encode_backend` serializes a chunk of exactly `max_decoded_chunk_size` bytes with `n mod 3 != 0`. The shipped product uses `Bounds::DEFAULT` (2 MiB line vs ~1.40 MiB worst-case chunk), so no shipped-path impact; but `opi-protocol` is designed for third-party consumption ("other agents ---+--> opi-protocol::execution::v1"), and the documented sizing rule fails closed for them. The formula is a load-bearing numeric invariant of a public crate.
**Fix:** Compute the true base64 length with checked arithmetic: `let b64 = n.checked_add(2)?.checked_div(3)?.checked_mul(4)?; let chunk_required = b64.checked_add(64)?;` and correct the doc/error text.

### 2.2 MINOR: `Session` does not enforce a single terminal frame (`completed` XOR `failed`)

**File:** `crates/opi-protocol/src/execution/v1/session.rs`
**Lines:** 121-130
**Cause:** `check_duplicate` keys on the `kind()` string; `completed` and `failed` are distinct kinds, so a backend emitting both is accepted by the session (each seen exactly once). The spec state machine has exactly one terminal frame.
**Impact:** The shipped enforcement substrate does not catch a real protocol violation. The 16.7 host is unaffected (it returns immediately after the first terminal `Action`), but any other `Session` consumer (the backend, third-party hosts) relying on `Session` as sole enforcement misses it.
**Fix:** Track a `terminal: bool` and reject any `completed`/`failed` after a terminal frame has been observed, or document the gap explicitly in the rustdoc.

### 2.3 MINOR: `Session::account_output` mutates cumulative state before the bound check

**File:** `crates/opi-protocol/src/execution/v1/session.rs`
**Lines:** 132-144
**Cause:** `self.cumulative = self.cumulative.saturating_add(bytes);` runs before the `> max_cumulative_output` check; on error the counter retains the inflated value.
**Impact:** State corruption on the error path if a caller treats the error as recoverable. Protocol violations are normally terminal, so impact is low.
**Fix:** Check `if self.cumulative.saturating_add(bytes) > limit` before assigning.

### 2.4 MINOR: `LineReader` CRLF asymmetry -- a CRLF ending consumes one byte of the line cap

**File:** `crates/opi-protocol/src/execution/v1/codec.rs`
**Lines:** 77-88
**Cause:** The newline branch (`byte == b'\n'`) returns before the cap check, but the `\r` of a CRLF pair is pushed and counted. A line of exactly `cap` data bytes + `\n` is accepted; the same `cap` bytes + `\r\n` is rejected as `OversizedLine`.
**Impact:** The effective data allowance depends on line ending (contradicts the "max wire bytes per JSONL line" description). Negligible at the 2 MiB default; off-by-one for custom bounds.
**Fix:** Exempt a `\r` that is immediately followed by `\n` from the cap, or document that CRLF consumes one cap byte.

### 2.5 MINOR: opi-sandbox backend waits for `initialize` with no deadline bound

**File:** `crates/opi-sandbox/src/backend.rs`
**Lines:** 150-161
**Cause:** `recv_host_frame(&mut rx, ...)` awaits `rx.recv()` with no `timeout_at` before the exchange deadline is known (`init.deadline_ms` arrives with the frame). Every later phase is deadline-bounded; the initial wait is not.
**Impact:** A standalone `opi-sandbox backend --stdio` that never receives `initialize` blocks forever, violating the one-shot "processes one invocation, exits after its terminal result" contract. The host's L0 supervision bounds it in integration, so production impact is low.
**Fix:** Wrap the initialize read in a bounded timeout (`INITIALIZE_WAIT_TIMEOUT`) emitting `failed{ExecutionTimedOut, Handshake}` on expiry.

### 2.6 MINOR: `accepted` is emitted before the execute request is validated

**File:** `crates/opi-sandbox/src/backend.rs`
**Lines:** 253-261 (emit `accepted`); 265-297 (validation in `helper::build_request`/`start`/`runner.run`)
**Cause:** The backend flushes `accepted` immediately after decoding the `execute` frame, then performs semantic validation (zero timeout, empty/nonexistent workspace, cwd outside workspace) afterwards. A zero-timeout execute yields `accepted` then `failed{ProtocolViolation, Handshake}`, contradicting the spec's definition ("`accepted` means the request is valid and the target has not started").
**Impact:** Hosts treating `accepted` as a validity assertion receive a false claim for semantically malformed requests. State-machine ordering is not violated (pre-start `failed` after `accepted` is legal), so impact is low.
**Fix:** Perform cheap request validation (nonzero `timeout_ms`, non-empty program/workspace/cwd) in `helper::build_request` and emit `failed` before `accepted` when invalid.

### 2.7 MINOR: Drain-loop deadline reports `cleanup_unconfirmed` without polling for confirmation

**File:** `crates/opi-sandbox/src/backend.rs`
**Lines:** 396-404
**Cause:** On deadline expiry the backend cancels, drops the run, and immediately emits `failed{CleanupUnconfirmed, Cleanup}` without observing the run's own best-effort cleanup (kill_on_drop + TreeGuard + TempDir). A run whose cleanup actually confirmed is misreported as unconfirmed.
**Impact:** Conservative and fail-closed, never over-claims confirmation, but a scheduling-dependent false `cleanup_unconfirmed`. The host surfaces it as `ExecutionFailure::CleanupUnconfirmed`, i.e. a spurious failure on a timed-out command whose cleanup succeeded.
**Fix:** After `cancel.cancel()`, poll the run under a bounded grace (as `drain_cancelled_run` does) and only fall back to `CleanupUnconfirmed` when that drain fails to confirm.

### 2.8 MINOR: The request deadline does not bound synchronous restriction setup

**File:** `crates/opi-sandbox/src/backend.rs` (also `helper.rs`)
**Lines:** 282-291
**Cause:** `helper::start` -> `runner.run` is fully synchronous and calls the injected `Restriction::prepare`/`launcher`; the deadline is only checked *after* setup returns. A blocking `prepare` can overrun the deadline (the `DelayedFailingRestriction` test sleeps 1500 ms against a 1000 ms deadline).
**Impact:** The "deadline covers startup/handshake/setup" guarantee is violated for a slow restriction. The shipped Linux/macOS restrictions are fast, so this is currently theoretical; the spawn still happens behind the gate and is killed on drop.
**Fix:** Document that `prepare` must be non-blocking, or race setup against the deadline on a `spawn_blocking` boundary.

---

## 3. Security / Redaction Findings

The redaction contract is strong and was verified end-to-end: `ExecutionFailure::remediation`/`Display` interpolate only package/adapter/strategy/mode labels (failure.rs:162-252); `From<ActivationError>` drops the untrusted `detail` string and the opaque `Store` display (failure.rs:262-278); `AdapterNotSelected` replaces raw model input with `<unavailable>` (failure.rs:36, 74); `bash.rs` sets `command_included: false` and `values_included: false`; `protocol_host.rs` redacts backend diagnostics with `RedactionMode::Summary` and never surfaces process stderr into the envelope; `contribution.rs` redacts the L0 layer/reason pair. A canary test (`unselectable_model_backend_is_redacted_from_public_text`, failure.rs:364-381) pins the raw-model-input redaction. No path interpolates command text, env values, credentials, unnecessary absolute paths, or PIDs into the stable codes.

### 3.1 MINOR: Interactive TUI never surfaces startup execution-wiring diagnostics

**File:** `crates/opi-coding-agent/src/interactive.rs` (`run_interactive_tui`); contrast `harness.rs:1169`
**Lines:** interactive.rs 849-1148 (no `resource_metadata`/`diagnostic` render path; grep confirms zero references)
**Cause:** A startup execution refusal (e.g. explicit `[execution.permissions] local = "deny"`, or fixed-external unavailable) causes `build_refused_execution_tools` to omit the bash tool and return `diagnostic_from_execution_failure`, appended to `resources.metadata.diagnostics`. The TUI event loop renders only `AgentEvent` messages and never reads `harness.resource_metadata()`, so the refusal is invisible.
**Impact:** In interactive mode the bash tool silently disappears with no code/remediation, while text (`startup_diagnostics_stderr_prefix`), NDJSON (`StartupDiagnostics`), RPC (`rpc_ready.startup_diagnostics`), and both doctor surfaces emit the same stable code. This violates the 16.9/16.16.2 cross-surface requirement ("Text, TUI, NDJSON, RPC, package doctor, and top-level doctor preserve stable redacted routing/permission codes and remediation") for the TUI surface.
**Fix:** Render `harness.resource_metadata().diagnostic_payloads(RedactionMode::Summary)` as initial system messages before the first frame in `run_interactive_tui`.

### 3.2 MINOR: A drifted fixed-selected backend is reclassified to `no_eligible_adapter` instead of `package_untrusted`

**File:** `crates/opi-coding-agent/src/package_activation.rs:360`; `execution/router.rs:110-113`
**Cause:** `usable_enabled_identities` silently `continue`s over any trusted+enabled record whose `activate()` fails (drift/static-gate re-failure). Under `strategy = "fixed", backend = "opi-sandbox"` with a drifted executable, the identity is dropped, eligibility contains only `local`, and the first bash call fails with `NoEligibleAdapter` -- whose remediation ("Install, trust, and enable an adapter, or select a different backend") does not mention drift/review.
**Impact:** The spec's `package_untrusted` code (with "manifest/lock/executable drifted ... re-confirm trust" remediation) is the intended drift signal, but the runtime point of failure misattributes the cause. Fail-closed and durable trust invalidation are satisfied; the classification/remediation is imprecise.
**Fix:** Surface a startup diagnostic (e.g. `package_untrusted`) when `usable_enabled_identities` drops a trusted+enabled record, and/or report the underlying activation reason on the fixed path rather than falling to generic `no_eligible_adapter`.

### 3.3 INFO: Schema cannot enforce base64 validity (`contentEncoding` is annotation-only in JSON Schema 2020-12)

**File:** `crates/opi-protocol/src/execution/v1/frames.rs:372-386`; `tests/execution_v1_schema.rs:176-190`; `tests/fixtures/invalid_base64.json`
**Cause/Impact:** `Base64Bytes` schema is `{"type":"string","contentEncoding":"base64"}`; under 2020-12 `contentEncoding` must not be used as an assertion, so the schema cannot reject invalid base64. `invalid_base64.json` is deliberately excluded from the schema's `invalid` list; the codec path does reject it. A non-Rust client validating against `schema()` can emit frames that pass schema but fail the codec -- a mild spec/impl divergence in the base64 contract. The implementation is correct (the schema physically cannot do better); the split should be documented.
**Fix:** Document in `schema.rs` that base64 validity is codec-enforced only.

---

## 4. Test Quality Findings

The suite is generally strong: the ledger gate counts are met or exceeded on every claimed target; the mock peer (`tests/fixtures/execution_backend_mock.rs`) is a genuine `harness=false` `[[test]]` binary speaking the real wire via typed `opi_protocol` frames; the 14 stable codes reach `ToolResult.diagnostics` via real production paths; no-vacuous assertions (no `assert_eq!(x,x)`, no `assert!(true)`) were found; isolation is strong (`tempfile::tempdir()` everywhere except the documented exceptions below); env-mutating tests are serialized; no-degraded-success and no-fallback are behaviorally pinned.

### 4.1 MINOR: Vacuous test -- `harness_respects_max_iterations_config` asserts nothing

**File:** `crates/opi-coding-agent/tests/interactive_mock.rs`
**Lines:** 286-305
**Cause:** Builds a `CodingHarness` with `config.defaults.max_iterations = 3` and `drop(harness)`; the only implicit check is that construction did not panic.
**Impact:** The name overclaims an iteration cap that is never driven or asserted; a regression that silently ignores `max_iterations` passes. The file is referenced by the 16.10 ledger gate.
**Fix:** Drive a loop that would exceed 3 iterations and assert termination at 3, or rename to `harness_constructs_with_low_max_iterations` and assert the config value is observed.

### 4.2 MINOR: Two negative config tests do not assert the rejection reason

**File:** `crates/opi-coding-agent/tests/execution_config.rs`
**Lines:** 135-151 (helper 142-144)
**Cause:** `rules_strategy_rejects_missing_catch_all` and `rules_strategy_rejects_empty_rules` both use `expect_invalid_config` which only asserts `result.expect_err(...)` -- a generic `ConfigError`, not the specific `InvalidExecutionConfig{rules...}` the names promise. Sibling tests use the stronger `expect_invalid_exec(result, "rules")`.
**Impact:** A regression collapsing these into a different-but-still-`Parse` error path would not be caught.
**Fix:** Use `expect_invalid_exec(..., "rules")` for both.

### 4.3 MINOR: Temp-file/project-dir leakage in migration tests

**File:** `crates/opi-coding-agent/tests/execution_migration.rs`
**Lines:** 41-54, 185-206
**Cause:** Uses `std::env::temp_dir()` + pid + atomic counter instead of `tempfile::tempdir()`; scratch files/dirs persist across runs.
**Impact:** Accumulating scratch state in the system temp dir; weak isolation vs. the RAII pattern used elsewhere.
**Fix:** Return a `tempfile::TempDir` so the fixture is dropped/cleaned.

### 4.4 MINOR: `std::mem::forget` leaks one temp dir per pack

**File:** `crates/opi-coding-agent/tests/opi_sandbox_packaging.rs`
**Lines:** 305 (inside `pack_fresh()`)
**Cause:** `pack_fresh` intentionally leaks the `TempDir` to keep the artifact tree alive; ~12 tests orphan a temp tree each.
**Impact:** Accumulating `packaging-*` trees on CI hosts and developer machines.
**Fix:** Store the `TempDir` as a field of `Packed` and drop at test end.

### 4.5 MINOR: Negative-path network tests can pass vacuously when `python3` is absent

**File:** `crates/opi-sandbox/tests/linux_policy.rs:195-211`; `crates/opi-sandbox/tests/macos_policy.rs:214-234`
**Cause:** The deny tests run `python3 -c '...socket...'` and assert "no SOCKET_OK" + "nonzero exit". Without `python3`, `/bin/sh -c` fails with exit 127 and empty stdout, so both assertions hold regardless of the sandbox.
**Impact:** The deny evidence is vacuous on python3-less hosts. The positive allow tests would still fail without python3, so the suite does not silently pass today, but the deny path alone is unproven if the allow tests are ever cfg-gated.
**Fix:** Gate on `command -v python3` (skip/fail explicitly) or use a python-free probe.

---

## 5. Spec Compliance Findings

### 5.1 INFO: Production startup still scans the legacy resource-package store and may spawn legacy adapter processes before Minimal-Runtime classification

**File:** `crates/opi-coding-agent/src/main.rs:846-852` (and the RPC/interactive analogues); `runtime_packages.rs:37-82`; `adapter_extension.rs:809-903`
**Cause:** All three startup modes call `start_installed_package_runtime_with_trust` unconditionally, which resolves `packages.toml`/`package-lock.toml` (a per-package scan) and may spawn legacy `[adapter]` process-jsonl adapter processes (via `AdapterHost::start`), regardless of execution routing. The Phase 16 **execution** store (`package-trust.toml`) is correctly never touched and no router/permission/protocol state is created.
**Impact:** The shipped `docs/opi-spec.md` claim ("starts no extension or package adapter process, performs no ... per-package scan") is broader than production behavior; the Minimal-Runtime guarantee is proven at the harness seam, not the `main()` path. The legacy resource runtime is pre-existing and explicitly out of Phase 16 scope (Non-Goal: no `opi-extension-jsonl-v1` migration), so this is a scoping/truthfulness gap, not a functional defect in the new machinery.
**Fix:** Scope the doc claim to the execution store / executable-adapter runtime, or gate the legacy-adapter spawn on execution routing.

### 5.2 INFO: `with_model_backend_enum` returns `None` when there are no eligible candidates

**File:** `crates/opi-coding-agent/src/tool/bash.rs:134-136`; `harness.rs:124-149`
**Cause/Impact:** Under `strategy = "model"` with zero model-visible adapters, the schema is returned unchanged (no `backend` field), so the model cannot even attempt selection and the router returns `adapter_not_selected`. Fail-closed either way; an empty required enum would be more literally spec-shaped but functionally identical.

### 5.3 INFO: `TargetId` lacks `minLength: 1` while the other wire identifiers reject empty

**File:** `crates/opi-protocol/src/execution/v1/frames.rs:286-298`
**Cause/Impact:** `RequestId`, `ImplementationId`, and `ProtocolId` reject the empty string and carry `minLength: 1`; `TargetId` is a bare transparent `String`. An empty `ready.target` passes deserialization and schema validation. Inconsistent strictness; low impact (the host compares target byte-for-byte against the locked value).
**Fix:** Mirror the other identifiers (`TargetId::new` guard + `minLength: 1`).

---

## 6. Invariant Verification

| Invariant | Code evidence | Test coverage |
|-----------|--------------|---------------|
| No degraded-success state | `bash.rs:269` `is_error = timed_out \|\| cancelled \|\| signal.is_some() \|\| exit != Some(0)`; `ExecutionFailure` has no Degraded variant; `resolve_selection` returns `Result<Selection, ExecutionFailure>` | `execution_product.rs` `timed_out_in_band_completed_is_not_a_success` / `cancelled_in_band_completed_is_not_a_success`; `execution_failures.rs` (type + literal) |
| No fallback after external selection | `RoutedBashOperations::exec` dispatches only the selected adapter; no `local` retry on external failure | `execution_runtime.rs:303` `routed_external_activation_failure_does_not_fall_back_to_local` (`call_count()==0`); `execution_product.rs:243` |
| Five independent lifecycle gates | `package_activation.rs` (Installed/Trusted/Enabled) + `router.rs` (Selected) + `permission.rs` (Permitted); `usable_enabled_identities` revalidates before exposure | `execution_package_lifecycle.rs` (`add_global_execution_package_persists_lock_and_untrusted_disabled_record`, `enable_refuses_without_explicit_confirmation`, `disable_preserves_trust_and_clears_enablement`, `activate_drift_invalidates_trust_durably`) |
| Request-id correlation | `Session::check_id` seeds from first observed frame; host `observe_host` anchors the host id | `execution_v1_contract.rs` missing/mismatched/cross-id fixtures; `execution_protocol_host.rs` cross-id rejection |
| L0 supervision (timeout/cancel/drop kill tree) | `supervision.rs` biased race with `push_terminate` on every branch; `TreeGuard` Drop terminates; `kill_on_drop(true)` | `sandbox_l0.rs` (clean-exit bg-descendant kill, bounded drain, dropped-future kill); `execution_product.rs` packaged-adapter tree-kill |
| Protocol stdin never reaches target | `helper.rs` `build_request` pins `StdinPolicy::Null`; `cli.rs` backend arm | `helper.rs` `build_request_pins_stdin_to_null`; `cli_contract.rs` stdin-EOF |
| Atomic start gate (started flushed before target release) | `helper::start` all-or-nothing; backend flushes `started` before draining the run | `protocol_conformance.rs` ordering; `helper.rs` refusal tests |
| Honest `started`/doctor vocabulary | `helper.rs` `started_payload` maps `Unrestricted->supervised`/`Restricted->restricted`, never `isolated`; doctor `supported=false` on Windows | `helper.rs` `started_payload_vocabulary_is_honest_l0_only` + native variant; `cli_contract.rs` doctor JSON |
| Crate boundaries (opi binary owns no opi-sandbox/native restriction) | `crates/opi-coding-agent/Cargo.toml` no `opi-sandbox` dep; `cargo tree` | `phase16_crate_boundaries.rs` (cargo-tree + filesystem existence + source-tripwire guards) |
| Project-local executable contributions rejected | `contribution.rs:250-252` `ProjectLocalExecutableContribution` | `execution_contribution_manifest.rs` |
| Rules: exactly one final catch-all, no fallthrough | `config.rs:1171-1219` `validate_rules`; `router.rs:119-136` `find_map` first-match then `gate` (no fall-through) | `execution_config.rs` + `execution_routing.rs` (`rules_selected_backend_failure_does_not_fall_through`) |
| Model non-authority | `router.rs:141-159` `resolve_model` gates on `available && !deny`; `apply_execution_overrides` touches only strategy/backend | `execution_routing.rs` (`model_cannot_select_denied_backend`, `model_cannot_mutate_permission_or_trust`); `main.rs` override tests |
| Redaction (no command/env/credential/PID leak) | failure.rs/redaction path traced in §3 | `execution_failures.rs` (`redaction_omits_*`); `execution_product.rs` secret-canary; `bash.rs` `command_included:false` |

All invariants hold in the shipped code and are behaviorally proven. The residual risk is the `Bounds::validate` arithmetic (2.1) and the opi-sandbox deadline semantics (2.5-2.8), none of which break an invariant on the shipped default path.

---

## 7. Residuals and Recommendations

### Priority recommendations

1. **Fix the `Bounds::validate` base64 formula (2.1, Major).** One-line checked-arithmetic fix; correct the `BoundsError`/doc text. This is the only Major and it hardens the public `opi-protocol` contract for third-party hosts.
2. **Bound the opi-sandbox backend's `initialize` wait (2.5)** and **emit `failed` before `accepted` for semantically invalid requests (2.6)**. Together these close the standalone one-shot contract ("processes one invocation, exits after terminal result") and the spec definition of `accepted`.
3. **Render startup execution diagnostics in the TUI (3.1)** so interactive mode satisfies the cross-surface code/remediation requirement instead of silently omitting the bash tool.
4. **Report the underlying drift reason on the fixed path (3.2)** instead of falling to `no_eligible_adapter`, so the `package_untrusted` drift signal reaches the runtime point of failure.
5. **Scope the Minimal-Runtime doc claim to the execution store (5.1)** and note the pre-existing legacy resource-package runtime in `docs/opi-spec.md` / the phase guard, or gate the legacy adapter spawn on execution routing.
6. **Fix the test-suite robustness items (4.1-4.5)**: replace the vacuous `max_iterations` test, strengthen the two config negative assertions, use RAII temp dirs, drop the `mem::forget`, and gate the python3-based network tests.

### Lower-priority residuals

- `Session` single-terminal enforcement and cumulative-counter-on-error ordering (2.2, 2.3) -- cheap hardening of the standalone substrate.
- `cleanup_unconfirmed` without polling for confirmation (2.7) and deadline not bounding synchronous setup (2.8) -- scheduling-dependent, fail-closed; consider a bounded post-cancel drain.
- Linux `IoctlDev`/`Truncate` handled-but-never-granted (TTY ioctls get EPERM) is untested and unreported; add a limitation string and a behavioral test.
- `close_nonessential_inherited_fds` iterates the whole fd table with a syscall per fd in the child pre_exec (performance on high-RLIMIT hosts); correctness is fine.
- Info items in §3.3, §5.2, §5.3 and the opi-sandbox Info list (macOS `escape_path` newline, backend reader thread, piped direct-CLI stdout, truncation notice in stderr, magic-number framing reserves, `validate_backend` control flow, `HostPhase::Terminal` dead-code allow, handshake-timeout-to-`cleanup_unconfirmed` semantics, non-atomic store writes, re-add preserving trust/enabled) are quality/doc notes with no shipped-path impact.

### Contamination statement

Prior audit reports (`audit.codex.md`, the stale `audit.deepseek-v4-flash.md`) were not read; all findings derive from the design spec, current source, tests, config, workflows, scripts, and git history. The stale `audit.deepseek-v4-flash.md` (pre-remediation, untracked) was superseded by this report.
