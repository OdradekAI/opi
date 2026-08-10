# Phase 16 Remediation Plan

**Date**: 2026-08-10

**Finding sources**: `docs/snapshots/phase16/audit.codex.md` (audit, model `codex`); `docs/snapshots/phase16/audit.glm5.2.md` (audit, model `glm5.2`)

**Commit range**: `1021842c937653de545cd335450df985f822bd06..f8aff0237221fbf7d56b58abb5dce02833344bfc`

**Verification HEAD**: `21dfcd8836974cd7e12454774156b3aefa97f2b5`

**Design specs**: `docs/opi-spec.md`; `docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md`

This plan consumes the two current Phase 16 audit reports. Neither report is
an `independent-family` source: Codex reports `unknown` independence and GLM
reports `fresh-context-same-family`. No cluster therefore has full independent
overlap. Source coverage below is descriptive only; every disposition comes
from current-code, current-artifact, or recorded-commit verification.

---

## Finding cross-reference summary

| Cluster | Theme | Sources | Independence | Coverage | Source severity range | Final severity + rationale | Verification |
|---|---|---|---|---|---|---|---|
| C1 | Pre-`ready` cancellation is classified as protocol corruption | Codex `AUD-P16-001` | unknown | Single source (unknown independence) | Major | Major: a normal cancellation/deadline produces the wrong stable outcome | Confirmed |
| C2 | Fixed/rules startup drops lifecycle state before routing | Codex `AUD-P16-002` | unknown | Single source (unknown independence) | Major | Major: missing, untrusted, and disabled selections collapse to `no_eligible_adapter` | Confirmed |
| C3 | Phase-exit native/six-target evidence is not durable | Codex `AUD-P16-003` | unknown | Single source (unknown independence) | Major | Major: the claimed critical evidence bundle is absent from HEAD and the checkout | Confirmed |
| C4 | `package_cli` hard-codes the in-tree Cargo target | Codex `AUD-P16-004` | unknown | Single source (unknown independence) | Minor | Minor: tests can false-fail or execute a stale binary under the canonical external cache | Confirmed |
| C5 | Seven task scenarios remain `open` after phase exit | Codex `AUD-P16-005`; GLM narrative disputes the conclusion | unknown / fresh-context-same-family | Single normalized source with narrative contradiction | Minor | Minor: the snapshot records seven `open` scenarios while tasks and matching exit criteria are `passing`/`met` | Confirmed |
| C6 | `opi-sandbox` runner owns divergent responsibilities | Codex `AUD-P16-006` | unknown | Single source (unknown independence) | Minor | Minor: 2,966 lines combine validation, spawn, release, capture, supervision, and cleanup despite the normative split requirement | Confirmed |
| C7 | Backend timeout branches repeat cleanup orchestration | Codex `AUD-P16-007` | unknown | Single source (unknown independence) | Minor | Info: repetition exists, but drain/terminal behavior is already centralized and no divergence was found | Confirmed; reranked |
| C8 | Trust invalidation is duplicated | Codex `AUD-P16-008` | unknown | Single source (unknown independence) | Info | Info: paths are similar but operate on different record lifecycles; no defect | Confirmed |
| C9 | `AdapterNotSelected` retains unused requested text | Codex `AUD-P16-009` | unknown | Single source (unknown independence) | Info | Info: unused internal state exists, but no public/logging leak path was found | Confirmed |
| C10 | Current HEAD lacks a green CI run | GLM `M1-repo-gates-unverified-at-head` | fresh-context-same-family | Single degraded source | Major | Major: current HEAD remains unpushed/unverified; the reported E0063 explains only part of the prior CI failure | Partially confirmed |
| C11 | Exit-code schema is wider than codec semantics | GLM `I1-exit-schema-range-wider-than-codec` | fresh-context-same-family | Single degraded source | Info | Info: schema lacks `maximum: 255`, but the authoritative codec rejects invalid values | Confirmed |
| C12 | Session substrate permits two different terminal kinds | GLM `I2-session-no-terminal-exclusivity` | fresh-context-same-family | Single degraded source | Info | Info: this is an explicit substrate/runtime responsibility split; production host enforces exclusivity | Confirmed, intentional |
| C13 | NativeString rustdoc overstates substrate mapping | GLM `I3-nativestring-rustdoc-overstates-mapping` | fresh-context-same-family | Single degraded source | Info | Minor: public compatibility rustdoc is inaccurate; substrate emits codec/session errors and runtime performs the protocol mapping | Confirmed; reranked |
| C14 | Doctor `target` uses OS family rather than target triple | GLM `I4-doctor-target-os-family-vs-ready-triple` | fresh-context-same-family | Single degraded source | Info | Info: documented implementation choice; the spec does not constrain this field's representation | Confirmed |
| C15 | Doctor JSON tests do not parse the JSON | GLM `I5-doctor-json-substring-only-validation` | fresh-context-same-family | Single degraded source | Info | Info: output parses successfully and current values are controlled; parser coverage is optional hardening | Confirmed |
| C16 | Linux pure-model tests compile only on Linux | GLM `I6-linux-pure-model-file-level-cfg` | fresh-context-same-family | Single degraded source | Info | Info: Linux CI exercises them and cross-host extraction would require target-dependency restructuring | Confirmed |
| C17 | Snapshot misstates manifest hashing | GLM `I7-manifest-hash-exact-bytes-ledger-mischar` | fresh-context-same-family | Single degraded source | Info | No defect: task commit `6b24fe1` did use and test LF normalization; the snapshot is accurate historical evidence | Refuted |
| C18 | Snapshot cites deleted documentation test targets | GLM `I8-ledger-stale-docguard-paths` | fresh-context-same-family | Single degraded source | Info | No defect: the cited targets existed at their task commits and phase-exit commit; later deletion does not stale historical evidence | Refuted |
| C19 | Bash ToolResult details carries command metadata | GLM `I9-toolresult-details-carries-command` | fresh-context-same-family | Single degraded source | Info | Info: raw in-memory metadata exists, but first-party events, sessions, diagnostics, traces, and provider wires redact or omit it | Partially confirmed |

## Decision record

| ID | Finding cluster(s) | Decision | Rationale | Decided by |
|---|---|---|---|---|
| D1 | C1 | Add an explicit pre-start cancellation state using the existing v1 `Ready`/`Failed` vocabulary; accept an already-in-flight `Ready`, never send `Execute`, require a bounded reason-consistent pre-start terminal and clean backend exit | The normative wire already permits `Failed` before `started`; this closes the cancellation contract without adding fields or changing the wire identity | auto |
| D2 | C2 | Resolve fixed/rules selected adapter IDs from installed lock metadata before filtering by trust/enablement, then activate only matched packages and preserve typed lifecycle failures | `usable_enabled_identities_for` already documents the package lock as the identity index and promises preserved activation errors; this restores its intended contract without touching Minimal Runtime | auto |
| D3 | C3 | Do not rewrite the historical snapshot; route durable evidence reacquisition and storage to a source-owning implementation/evidence task | Platform reruns and the durable medium require external execution/provenance decisions, while remediation is forbidden to edit `.opi-impl-state.json` | auto handoff |
| D4 | C4 | Use Cargo's `CARGO_BIN_EXE_opi` integration-test path and remove the manual in-tree prebuild assertion | This is the standard exact binary produced for the test invocation and honors the configured external target cache | auto |
| D5 | C5 | Do not hand-edit either ledger; route scenario/phase-exit reconciliation through guarded `opi-implement` state handling | The inconsistency is real, but the canonical ledger and archived snapshot are owned exclusively by `opi-implement` | auto handoff |
| D6 | C6 | Keep `SandboxRunner` and its public types as the facade; extract private validation/preparation, gated-spawn/release, and supervision/capture/cleanup modules | These are the independently changing responsibilities established by the verified inventory; the split changes no public API or behavior | auto |
| D7 | C7-C12, C14-C16, C19 | Make no Phase 16 product change | These are intentional boundaries, informational observations, or optional refactors/hardening without a verified defect | auto |
| D8 | C13 | Correct the public rustdoc to distinguish substrate codec/session errors from host/backend runtime `ProtocolViolation` mapping | The observed implementation and required compatibility documentation determine one truthful wording | auto |
| D9 | C17-C18 | Drop both findings | Recorded-commit inspection refutes the claims; changing the snapshot would falsify historical evidence | auto |
| D10 | C10 | Require explicit authorization to push the exact remediation commit, then wait for the complete CI matrix; do not treat prior green local subsets as repository-gate closure | Publication is outside plan-only remediation, and the previous CI root-cause narrative was only partially correct | auto handoff |

## Remediation layers

### Layer 1: `opi-sandbox` (lower affected crate)

`opi-sandbox` depends only on `opi-protocol`; no `opi-protocol` behavior change
is required before this layer.

**Verification**:

    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/opi-impl-smoke.ps1 scoped --crate opi-sandbox --test protocol_conformance --test backend_protocol_smoke --test sdk_contract --test standalone_smoke --test cli_contract --test linux_policy --test macos_policy

#### Fix 1.1: Complete cancellation before command disclosure

- **Finding source**: `docs/snapshots/phase16/audit.codex.md` (audit, model `codex`, `AUD-P16-001`)
- **Cluster**: C1
- **Decision**: D1
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-sandbox/src/backend.rs` ~L249-L283 and pre-execute terminal helpers; protocol conformance/backend smoke tests
- **Change**: After emitting `Ready`, accept a matching `Cancel` as normal pre-start control flow. Emit one bounded `Failed` terminal whose handshake code agrees with deadline versus user cancellation, flush it, and exit cleanly. Never validate, disclose, or release an `Execute` target on this path.
- **Test plan**: Add backend cases for cancel immediately after initialize, cancel racing `Ready`, deadline cancel, wrong request id, and silence past grace; assert no target marker starts and exactly one terminal frame is emitted.

#### Fix 1.2: Split `SandboxRunner` by responsibility

- **Finding source**: `docs/snapshots/phase16/audit.codex.md` (audit, model `codex`, `AUD-P16-006`)
- **Cluster**: C6
- **Decision**: D6
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-sandbox/src/runner.rs` ~L72-L1999; new private modules under `crates/opi-sandbox/src/runner/`
- **Change**: Retain `SandboxRunner` plus public request/event/result types as the facade. Move request/deadline validation and preparation, gated spawn/release/bootstrap construction, and supervision/output/cleanup into private responsibility-focused modules. Preserve ordering, cleanup precedence, cfg boundaries, and error types.
- **Test plan**: Retain existing SDK, CLI, backend, standalone, Linux, and macOS behavior coverage; add no new behavior assertion solely for file layout.

### Layer 2: `opi-coding-agent` (product integration)

**Verification**:

    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/opi-impl-smoke.ps1 scoped --crate opi-coding-agent --test execution_package_lifecycle --test execution_selected_routing --test package_cli
    cargo test -p opi-coding-agent --features execution-backend-test-fixture --test execution_backend_mock --no-run
    cargo clippy -p opi-coding-agent --features execution-backend-test-fixture --test execution_protocol_host -- -D warnings
    cargo test -p opi-coding-agent --features execution-backend-test-fixture --test execution_protocol_host
    cargo test -p opi-coding-agent --features execution-backend-test-fixture --test execution_product

#### Fix 2.1: Accept the bounded pre-start cancellation sequence

- **Finding source**: `docs/snapshots/phase16/audit.codex.md` (audit, model `codex`, `AUD-P16-001`)
- **Cluster**: C1
- **Decision**: D1
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/execution/protocol_host.rs` ~L672-L763, ~L1142-L1231; `crates/opi-coding-agent/tests/execution_protocol_host.rs` ~L856-L905
- **Change**: Represent pre-start cancellation explicitly. During bounded grace, accept an already-in-flight matching `Ready` and the subsequent valid pre-start terminal, never send `Execute`, preserve the host cancellation reason in the returned timeout/cancellation classification, and continue to require terminal validation, EOF, clean backend exit, and L0 teardown. Keep malformed, duplicate, mismatched-id, or post-grace traffic as failures.
- **Test plan**: Replace the test that pins `protocol_violation` with deadline/cancellation success-path coverage, plus wrong-sequence and no-terminal negatives. Add a composition test that launches the explicitly supplied built `opi-sandbox` binary and races cancellation immediately after initialize; assert no command disclosure or target start.

#### Fix 2.2: Preserve selected-package lifecycle failures at startup

- **Finding source**: `docs/snapshots/phase16/audit.codex.md` (audit, model `codex`, `AUD-P16-002`)
- **Cluster**: C2
- **Decision**: D2
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/harness.rs` ~L208-L223; `crates/opi-coding-agent/src/package_activation.rs` ~L329-L439; `crates/opi-coding-agent/src/execution/router.rs` ~L164-L180; lifecycle/selected-routing/product tests
- **Change**: Make `usable_enabled_identities_for` resolve only requested adapter IDs from installed package-lock metadata before trust/enablement filtering, detect duplicate providers, and call `activate` for the matched package. Return `NotInstalled` for an unknown fixed/rules external selection and preserve `Untrusted`/`Disabled`/collision errors. Keep default fixed-local Minimal Runtime store-untouched.
- **Test plan**: Add production-harness fixed and rules cases for absent, untrusted, disabled, colliding, and valid packages; assert stable `package_not_installed`, `package_untrusted`, and `contribution_disabled` codes, no process start, no rule fallthrough, and unchanged fixed-local sentinel behavior.

#### Fix 2.3: Execute the Cargo-provided test binary

- **Finding source**: `docs/snapshots/phase16/audit.codex.md` (audit, model `codex`, `AUD-P16-004`)
- **Cluster**: C4
- **Decision**: D4
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/tests/package_cli.rs` ~L1048-L1061 and eight subprocess tests through ~L1349
- **Change**: Replace the manually constructed `workspace/target/debug/opi(.exe)` path with Cargo's `CARGO_BIN_EXE_opi` path and remove the instruction to prebuild an unrelated location.
- **Test plan**: Run the complete `package_cli` integration binary with the canonical external `CARGO_TARGET_DIR`; add an assertion that the selected path equals Cargo's supplied binary and does not fall back to workspace `target/`.

### Final layer: public rustdoc

**Verification**:

    $env:RUSTDOCFLAGS = "-D warnings"
    cargo doc -p opi-protocol --no-deps
    Remove-Item Env:RUSTDOCFLAGS
    python scripts/opi-doc-check.py

#### Fix D.1: State NativeString error ownership accurately

- **Finding source**: `docs/snapshots/phase16/audit.glm5.2.md` (audit, model `glm5.2`, `I3-nativestring-rustdoc-overstates-mapping`)
- **Cluster**: C13
- **Decision**: D8
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-protocol/src/execution/v1/native.rs` ~L44-L47; matching compatibility wording in `crates/opi-protocol/src/execution/v1/mod.rs` ~L108-L126
- **Change**: Document that malformed native-string representations surface from the substrate as `CodecError::Json` / `SessionError::Codec`, while the consuming host/backend runtime maps them to a wire/public protocol violation.
- **Test plan**: Existing malformed NativeString codec/session tests cover behavior; build warning-free public rustdoc. Test impact is `retain`.

## Evidence and ledger handoffs

These confirmed findings do not authorize direct remediation edits:

1. **C3 / AUD-P16-003**: create a source-owned evidence task that reruns the
   Linux, macOS, Windows, six-target, workspace-gate, and artifact-audit checks
   at one exact commit, then records a durable digest-bound retrieval index.
   The user must choose repository storage versus a durable external artifact
   service before execution. Do not present new evidence as retroactive proof
   and do not rewrite the Phase 16 snapshot.
2. **C5 / AUD-P16-005**: use the guarded `opi-implement` reconciliation flow to
   decide whether the seven task scenarios should be `met` or the phase-exit
   trace should cease claiming closure. `opi-remediate` must not modify either
   ledger file.
3. **C10 / M1**: after code remediation and explicit push authorization, push
   the exact commit and wait for all ordinary jobs plus all six target checks.
   The prior CI run is not evidence for the new commit.

## Final verification

The executable fixes span `opi-sandbox` and `opi-coding-agent`; after each
layer's scoped gate passes, run the workspace tier once:

    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/opi-impl-smoke.ps1 full
    cargo test --workspace --doc
    python scripts/opi-doc-check.py

Also run the explicit built-backend cancellation composition test on Windows,
Linux, and macOS. The code-remediation gate does not close C3, C5, or C10;
those remain separately owned handoffs until their durable evidence exists.

## Scope exclusions

| Finding | Status | Reason |
|---|---|---|
| C3 / `AUD-P16-003` | Handoff | Durable native/six-target evidence requires source ownership, platform reruns, and a storage/provenance decision |
| C5 / `AUD-P16-005` | Handoff | Only guarded `opi-implement` may reconcile canonical/historical ledger state |
| C7 / `AUD-P16-007` | Info/No action | Cleanup primitives are centralized; helper extraction is optional and could obscure state transitions |
| C8 / `AUD-P16-008` | Info/No action | Similar invalidation branches operate on different record flows and have no verified drift |
| C9 / `AUD-P16-009` | Info/No action | No public leak or required consumer exists |
| C10 / `M1-repo-gates-unverified-at-head` | External handoff | Requires explicit push authority and a new CI run; no local code edit closes it |
| C11 / `I1-exit-schema-range-wider-than-codec` | Info/No action | Codec remains authoritative; adding schema hardening is optional and outside minimum remediation |
| C12 / `I2-session-no-terminal-exclusivity` | Info/No action | Explicit substrate/runtime responsibility boundary |
| C14 / `I4-doctor-target-os-family-vs-ready-triple` | Info/No action | Documented, spec-permitted representation |
| C15 / `I5-doctor-json-substring-only-validation` | Info/No action | Output parses and no malformed-output defect was reproduced |
| C16 / `I6-linux-pure-model-file-level-cfg` | Info/No action | Supported Linux CI runs the tests; cross-host restructuring is optional |
| C17 / `I7-manifest-hash-exact-bytes-ledger-mischar` | Refuted | LF normalization was the actual implementation at recorded task commit `6b24fe1` |
| C18 / `I8-ledger-stale-docguard-paths` | Refuted | Both cited Rust targets existed at the recorded task and phase-exit commits |
| C19 / `I9-toolresult-details-carries-command` | Info/No action | First-party public/persisted/provider surfaces redact or omit the raw metadata |

## Test impact

Plan-only change: `none` for Rust tests and runtime behavior.

If execution is approved: `add` pre-ready backend/host/composition tests and
production-harness lifecycle diagnostics; `update` the existing cancellation
expectation and `package_cli` binary resolver; `retain` runner behavior and
NativeString codec/session coverage. No test deletion is planned.
