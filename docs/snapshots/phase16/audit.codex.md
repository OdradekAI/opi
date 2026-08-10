# Phase 16 Audit — Codex

## Audit metadata

- Phase: `16`
- Task scope: `16.1` through `16.16.3` (21 ledger tasks)
- Current-state authority: `21dfcd8836974cd7e12454774156b3aefa97f2b5`
- Recorded task-commit span: `1021842c937653de545cd335450df985f822bd06` through `f8aff0237221fbf7d56b58abb5dce02833344bfc`
- Normative sources:
  - `docs/opi-spec.md`
  - `docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md`
  - `docs/snapshots/phase16/opi-impl-state.json`
- Review axes: Standards and Spec, plus correctness, security/redaction, test quality, invariants, integration, and residuals
- Dirty-tree isolation: the unrelated deletion of `docs/snapshots/phase16/remediation-plan.md` was excluded. Static reads used committed objects; commands ran in an isolated detached worktree at the pinned HEAD.
- Contamination isolation: no prior `audit.*.md` or evaluator report was read or searched.

## Verdict

**PASS-WITH-FINDINGS**

No Blocker was found. Three Major gaps remain:

1. cancellation during the pre-`ready` handshake is classified as a protocol violation rather than completing the required cancellation/cleanup flow;
2. normal fixed/rules startup removes missing, untrusted, and disabled named adapters before routing, collapsing their distinct lifecycle failures into `no_eligible_adapter`;
3. the phase-exit native/six-target evidence bundle claimed as preserved is absent from both committed HEAD and the current checkout, so critical platform claims cannot be independently re-audited.

The runtime remains fail-closed in the reviewed paths. Protocol bounds, redaction, Minimal Runtime isolation, no-local-fallback behavior, package executable binding, crate boundaries, Windows L0 posture, and documentation contracts have strong focused coverage.

## Findings summary

| ID | Axis | Severity | Finding |
|---|---|---:|---|
| AUD-P16-001 | Spec | Major | Pre-`ready` cancellation cannot complete the specified cleanup flow |
| AUD-P16-002 | Spec / integration | Major | Fixed/rules startup collapses three lifecycle failures into `no_eligible_adapter` |
| AUD-P16-003 | Test quality | Major | Claimed phase-exit platform evidence is not preserved at current HEAD |
| AUD-P16-004 | Test quality | Minor | `package_cli` subprocess tests hard-code the in-tree Cargo target directory |
| AUD-P16-005 | Residuals | Minor | Seven acceptance scenarios remain `open` after phase exit is marked complete |
| AUD-P16-006 | Standards | Minor | `runner.rs` has accumulated divergent responsibilities in one 2,967-line module |
| AUD-P16-007 | Standards | Minor | Backend timeout branches duplicate security-sensitive cancellation cleanup |
| AUD-P16-008 | Standards | Info | Trust invalidation is duplicated between enable and activation paths |
| AUD-P16-009 | Standards | Info | `AdapterNotSelected` retains unused model-controlled text |

## Spec and integration findings

### AUD-P16-001 Major: Pre-`ready` cancellation cannot complete the specified cleanup flow

**Files:** `crates/opi-coding-agent/src/execution/protocol_host.rs`, `crates/opi-sandbox/src/backend.rs`

**Cause:** `finish_with_cancel` marks the host state as cancelling before reading grace-period frames. The transition table then rejects `Ready` while cancelling and rejects every pre-`started` `Failed` frame. A conforming backend that was already processing `initialize` can therefore send its normal `ready` milestone or a pre-start terminal failure after receiving `cancel`, but the host converts either response into `protocol_violation`. The shipped backend itself sends `ready` before reading the next host frame, so a cancel racing the handshake enters this path.

**Impact:** A valid user cancellation or deadline during adapter startup is surfaced as protocol corruption (or, for a silent backend, `cleanup_unconfirmed`) instead of the bounded cancellation result required by the Phase 16 cancellation contract. The failure code and remediation are wrong for a normal control-flow event.

**Fix:** Define an explicit pre-start cancellation state. It must accept only the bounded post-cancel sequence needed to prove no target escaped, then return cancellation/timeout with confirmed cleanup. Add a production-host test that races cancellation with the shipped backend's `ready` emission; do not encode `protocol_violation` as the expected cancellation result.

```yaml
id: AUD-P16-001
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: spec
severity: Major
title: Pre-ready cancellation cannot complete the specified cleanup flow
claim: A cancellation received while the protocol host is AwaitingReady cannot be acknowledged through either Ready or a pre-start Failed terminal; the host returns protocol_violation or cleanup_unconfirmed instead of a bounded cancellation result.
evidence:
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:718-753
    detail: Ready advances only when cancelling is false, and Failed is accepted during cancellation only in Draining after Started.
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:1142-1231
    detail: finish_with_cancel sets cancelling before reading grace-period frames and maps rejected frames to ProtocolViolation.
  - location: crates/opi-sandbox/src/backend.rs:249-283
    detail: The shipped backend emits Ready before it reads the next host frame, where a racing Cancel is treated as a non-Execute protocol failure.
  - location: crates/opi-coding-agent/tests/execution_protocol_host.rs:856-905
    detail: Existing tests explicitly pin protocol_violation for pre-milestone cancellation rather than the normative cancellation outcome.
criterion_source: docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md:516-529
reproduction:
  - cargo test -p opi-coding-agent --test execution_protocol_host cancellation_pre_ready_rejects_subsequent_negotiation_sequence -- --exact
  - Race the production host cancellation token immediately after initialize reaches the shipped opi-sandbox backend and observe protocol_violation instead of a cancelled or timed-out terminal with confirmed cleanup.
confidence: high
status: unverified
```

### AUD-P16-002 Major: Fixed/rules startup collapses three lifecycle failures into `no_eligible_adapter`

**Files:** `crates/opi-coding-agent/src/harness.rs`, `crates/opi-coding-agent/src/package_activation.rs`, `crates/opi-coding-agent/src/execution/router.rs`

**Cause:** Fixed/rules startup asks `usable_enabled_identities_for` for the selected adapter. That function starts from `enabled_identities`, which keeps only `trusted && enabled` records and omits absent matches. The resulting eligibility catalog therefore contains no selected external identity for a missing, untrusted, or disabled package. `select_named_candidate` maps all three cases to `NoEligibleAdapter` before invocation-time activation can return `NotInstalled`, `Untrusted`, or `Disabled`.

**Impact:** Ordinary startup with a configured external adapter produces the wrong stable code and remediation for three user-actionable lifecycle states. The more precise codes are currently reachable mainly through stale/injected identity snapshots, so focused runtime tests do not prove the normal production startup path promised by the five-gate model.

**Fix:** Preserve the selected named identity and its lifecycle state through startup, or perform a selected-package lookup that returns the precise lifecycle error before building routing eligibility. Add full harness tests for fixed and rules configurations where the package is absent, untrusted, and disabled before startup.

```yaml
id: AUD-P16-002
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: integration
severity: Major
title: Fixed and rules startup collapse lifecycle failures into no_eligible_adapter
claim: When a fixed or rules configuration names an external adapter whose package is absent, untrusted, or disabled before harness startup, the production wiring omits the identity and routing returns no_eligible_adapter rather than package_not_installed, package_untrusted, or contribution_disabled.
evidence:
  - location: crates/opi-coding-agent/src/harness.rs:208-219
    detail: Fixed and rules wiring resolves only the concrete selected id through usable_enabled_identities_for.
  - location: crates/opi-coding-agent/src/package_activation.rs:329-343
    detail: enabled_identities filters records to trusted and enabled before exposing adapter identities.
  - location: crates/opi-coding-agent/src/package_activation.rs:389-439
    detail: Requested ids with no enabled match are silently omitted instead of returning a lifecycle error.
  - location: crates/opi-coding-agent/src/execution/router.rs:164-180
    detail: A named backend missing from eligibility is mapped to NoEligibleAdapter.
  - location: crates/opi-coding-agent/tests/windows_execution_posture.rs:30-41
    detail: The precise absent-package test injects an EnabledIdentity directly into ExecutionRuntime and does not exercise production harness startup.
criterion_source: docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md:50-61,740-762
reproduction:
  - Start a production harness with strategy=fixed and backend=<external-id> while the corresponding global package is installed but untrusted before startup; invoke bash and inspect the stable diagnostic code.
  - Repeat with the package disabled and absent; compare the result to package_untrusted, contribution_disabled, and package_not_installed respectively.
confidence: high
status: unverified
```

## Test-quality and residual findings

### AUD-P16-003 Major: Claimed phase-exit platform evidence is not preserved at current HEAD

**Files:** `docs/snapshots/phase16/opi-impl-state.json`, `.gitignore`

**Cause:** Phase-exit criteria C9-C12 and C16 cite evidence under `target/opi-artifacts/phase16-phase-exit`, and C16 explicitly says the evidence is preserved. `/target` is ignored, the cited `evidence.json` is not a committed object, and the bundle is absent from the current checkout and from a clean detached checkout at the audited HEAD.

**Impact:** The Linux Landlock/seccomp run, macOS Seatbelt run, Windows posture smoke, six-target logs, and final phase-exit artifact audit cannot be independently rerun or authenticated from current repository state. These are critical platform/security claims that cannot be replaced by a green ledger assertion.

**Fix:** Preserve an immutable, repository-addressable evidence index and artifacts, or record durable external artifact URLs plus hashes and provenance sufficient for `opi-artifact-audit.py` to reacquire and verify them. Update C9-C12/C16 only after the preserved bundle passes the audit from a clean checkout.

```yaml
id: AUD-P16-003
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: test-quality
severity: Major
title: Claimed phase-exit platform evidence is not preserved at current HEAD
claim: The evidence bundle required by phase-exit criteria C9 through C12 and C16 cannot be obtained from current committed HEAD or the current checkout, so those native and six-target claims are not independently verifiable.
evidence:
  - location: docs/snapshots/phase16/opi-impl-state.json:3440-3550
    detail: C9-C12 and C16 cite preserved files under target/opi-artifacts/phase16-phase-exit and mark each criterion met.
  - location: .gitignore:1
    detail: /target is ignored.
  - location: command git cat-file -e HEAD:target/opi-artifacts/phase16-phase-exit/evidence.json
    detail: The command exits 128 because the claimed evidence index is not committed.
  - location: clean detached worktree at 21dfcd8836974cd7e12454774156b3aefa97f2b5
    detail: Test-Path target/opi-artifacts/phase16-phase-exit/evidence.json returned False.
criterion_source: docs/snapshots/phase16/opi-impl-state.json:3538-3548
reproduction:
  - git cat-file -e HEAD:target/opi-artifacts/phase16-phase-exit/evidence.json
  - Test-Path -LiteralPath target/opi-artifacts/phase16-phase-exit/evidence.json
  - python scripts/opi-artifact-audit.py target/opi-artifacts/phase16-phase-exit --workspace-root . --phase-exit --json
confidence: high
status: unverified
```

### AUD-P16-004 Minor: `package_cli` subprocess tests hard-code the in-tree Cargo target directory

**File:** `crates/opi-coding-agent/tests/package_cli.rs`

**Cause:** `opi_binary` ignores Cargo's configured target directory and constructs `<workspace>/target/debug/opi(.exe)`. This repository uses a persistent external Cargo cache, so even a successful `cargo build -p opi-coding-agent` places the binary elsewhere.

**Impact:** Eight subprocess tests fail before exercising behavior in the canonical cached build environment. Focused task verification is not hermetic and produces a false red despite a successfully built binary.

**Fix:** Use Cargo's `CARGO_BIN_EXE_opi` integration-test path (or a shared target-path resolver that honors `CARGO_TARGET_DIR`) and remove the manual pre-build assertion.

```yaml
id: AUD-P16-004
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: test-quality
severity: Minor
title: package_cli subprocess tests hard-code the in-tree Cargo target directory
claim: In the repository's configured external Cargo target environment, cargo build succeeds but eight package_cli subprocess tests fail because opi_binary checks only workspace/target/debug/opi.
evidence:
  - location: crates/opi-coding-agent/tests/package_cli.rs:1048-1061
    detail: The helper constructs workspace_root/target/debug/opi and asserts that path exists.
  - location: command cargo build -p opi-coding-agent then cargo test -p opi-coding-agent --test package_cli
    detail: Build succeeded; 28 tests passed and 8 subprocess tests failed with 'opi binary must be built' while the binary existed in the configured external target directory.
criterion_source: AGENTS.md#Implementation workflow
reproduction:
  - cargo build -p opi-coding-agent
  - cargo test -p opi-coding-agent --test package_cli
confidence: high
status: unverified
```

### AUD-P16-005 Minor: Seven acceptance scenarios remain `open` after phase exit is complete

**File:** `docs/snapshots/phase16/opi-impl-state.json`

**Cause:** Acceptance scenarios for tasks 16.13, 16.14.1, 16.14.2, 16.15.2, and 16.16.1 retain `status: open`, while the same snapshot marks every task `passing`, sets `exit_criteria_met: true`, and marks the corresponding exit criteria met.

**Impact:** The canonical phase snapshot contradicts itself, making automated and human consumers unable to tell whether those scenarios were actually closed or merely summarized as complete later.

**Fix:** Reconcile the canonical ledger through the guarded ledger workflow so each scenario status matches its independently verified criterion; do not hand-edit the snapshot.

```yaml
id: AUD-P16-005
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: residuals
severity: Minor
title: Seven acceptance scenarios remain open after phase exit is complete
claim: The Phase 16 snapshot contains seven acceptance_scenarios with status open even though phase_exit.exit_criteria_met is true and the corresponding tasks and criteria are recorded as passing or met.
evidence:
  - location: docs/snapshots/phase16/opi-impl-state.json:2135,2151,2272,2288,2409,2618,2754
    detail: Seven scenario records retain status open.
  - location: docs/snapshots/phase16/opi-impl-state.json:3190-3195
    detail: Phase exit is marked completed with exit_criteria_met true and all 21 tasks D.2-clean.
  - location: docs/snapshots/phase16/opi-impl-state.json:3440-3507
    detail: The matching Linux, macOS, Windows, release-topology, and migration criteria are marked met.
criterion_source: docs/snapshots/phase16/opi-impl-state.json
reproduction:
  - git show HEAD:docs/snapshots/phase16/opi-impl-state.json | Select-String '"status": "open"'
confidence: high
status: unverified
```

## Standards findings

### AUD-P16-006 Minor: `runner.rs` has divergent responsibilities

**File:** `crates/opi-sandbox/src/runner.rs`

**Cause:** The 2,967-line module owns request validation, temporary-root preparation, command construction and spawning, release-gate state, platform bootstrap, process supervision, output capture, and cleanup outcomes. These concerns change for different reasons despite the spec's explicit large-module split requirement.

**Impact:** Security-sensitive lifecycle changes require navigating and modifying one broad module, increasing review cost and the chance that preparation, start-gate, or cleanup invariants drift.

**Fix:** Keep `SandboxRunner` as the facade and split validation/preparation, gated spawn, supervision/cleanup, and output collection into responsibility-focused private modules.

```yaml
id: AUD-P16-006
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: standards
severity: Minor
title: Sandbox runner has divergent responsibilities in one large module
claim: crates/opi-sandbox/src/runner.rs combines at least six independently changing responsibilities in 2967 lines, contrary to the repository requirement to split large modules by responsibility.
evidence:
  - location: crates/opi-sandbox/src/runner.rs:176-1905
    detail: Production definitions cover validation, deadline planning, prepared/spawned run state, release gates, process control, capture, and cleanup in one module; tests continue through line 2967.
  - location: docs/opi-spec.md:2234-2239
    detail: Maintainability requirements explicitly require splitting large modules by responsibility.
criterion_source: docs/opi-spec.md:2234-2239
reproduction:
  - (git show HEAD:crates/opi-sandbox/src/runner.rs | Measure-Object -Line).Lines
  - Inspect the production type and function inventory in crates/opi-sandbox/src/runner.rs.
confidence: high
status: unverified
```

### AUD-P16-007 Minor: Backend timeout branches duplicate cancellation cleanup

**File:** `crates/opi-sandbox/src/backend.rs`

**Cause:** Multiple handshake/start-gate timeout checks independently cancel, retain the gate, drain until the same deadline, classify cleanup, and emit a terminal failure. The repeated branches differ only in phase or surrounding milestone.

**Impact:** A future correction to timeout or cleanup semantics can be applied to one branch but missed in another. This is the same security-sensitive state area implicated by AUD-P16-001.

**Fix:** Extract one fail-closed cancellation-and-cleanup classifier parameterized by failure phase and target-start state, then use it from all four branches.

```yaml
id: AUD-P16-007
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: standards
severity: Minor
title: Backend timeout branches duplicate cancellation cleanup
claim: Four backend timeout branches independently implement the same cancel, keep-gated, bounded-drain, cleanup-classification, and terminal-emission sequence.
evidence:
  - location: crates/opi-sandbox/src/backend.rs:339-354
    detail: Expired start outcome performs cancellation, gated drain, cleanup classification, and timeout emission.
  - location: crates/opi-sandbox/src/backend.rs:371-387
    detail: Started-event timeout repeats the same sequence.
  - location: crates/opi-sandbox/src/backend.rs:408-423
    detail: Post-start-event deadline check repeats the sequence.
  - location: crates/opi-sandbox/src/backend.rs:436-451
    detail: Pre-release deadline check repeats the sequence with only the phase changed.
criterion_source: null
reproduction:
  - Inspect crates/opi-sandbox/src/backend.rs:330-451 and compare the four timeout branches.
confidence: high
status: unverified
```

### AUD-P16-008 Info: Trust invalidation is duplicated between enable and activation

**File:** `crates/opi-coding-agent/src/package_activation.rs`

**Cause:** `enable` and `activate` separately re-read records, clear `trusted` and `enabled`, persist the records, and construct an `Untrusted` error when lock revalidation fails.

**Impact:** The lifecycle is currently correct, but future changes to the security-sensitive drift transition must be kept synchronized in two implementations.

**Fix:** Centralize durable drift invalidation and error construction in one private helper.

```yaml
id: AUD-P16-008
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: standards
severity: Info
title: Trust invalidation is duplicated between enable and activation
claim: Both PackageActivationStore::enable and PackageActivationStore::activate independently clear trusted and enabled state, write records, and return an Untrusted error after revalidation failure.
evidence:
  - location: crates/opi-coding-agent/src/package_activation.rs:525-536
    detail: enable implements the drift invalidation transition inline.
  - location: crates/opi-coding-agent/src/package_activation.rs:634-647
    detail: activate independently implements the same transition and error construction.
criterion_source: null
reproduction:
  - Inspect and compare the revalidate_lock error arms in PackageActivationStore::enable and PackageActivationStore::activate.
confidence: high
status: unverified
```

### AUD-P16-009 Info: `AdapterNotSelected` retains unused model-controlled text

**Files:** `crates/opi-coding-agent/src/execution/failure.rs`, `crates/opi-coding-agent/src/execution/router.rs`

**Cause:** The router stores the omitted or rejected model-provided backend string in `AdapterNotSelected.requested`, but the Display implementation substitutes `<unavailable>` and production code/remediation matches discard the field. Derived `Debug` still retains the original value.

**Impact:** This adds unused state and keeps attacker/model-controlled text alive in an error type whose public contract intentionally redacts it. No current public leak was found.

**Fix:** Remove `requested` from the variant and construct only the strategy, unless a concrete non-public consumer is added with an explicit redaction boundary.

```yaml
id: AUD-P16-009
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: standards
severity: Info
title: AdapterNotSelected retains unused model-controlled text
claim: AdapterNotSelected.requested is populated from model input but is discarded by every production display, code, and remediation match while remaining present in derived Debug output.
evidence:
  - location: crates/opi-coding-agent/src/execution/failure.rs:74-78
    detail: The variant stores requested while its Display string uses the constant <unavailable>.
  - location: crates/opi-coding-agent/src/execution/failure.rs:149,211-214
    detail: Stable-code and remediation paths match requested with .. and never consume it.
  - location: crates/opi-coding-agent/src/execution/router.rs:208-229
    detail: The field is populated from omitted or rejected model backend input.
criterion_source: AGENTS.md#Operating principles
reproduction:
  - git grep -n 'AdapterNotSelected' HEAD -- crates/opi-coding-agent
confidence: high
status: unverified
```

## Invariant assessment

| Invariant | Code evidence | Test / audit result |
|---|---|---|
| Default fixed-local allow stays in Minimal Runtime | `harness.rs:195-257`; `runtime.rs:294-380` | Focused minimal-runtime and routing suites passed |
| External selection never falls back to local | `runtime.rs:407-665` | Runtime/product/no-fallback tests passed |
| Installed, Trusted, Enabled, Selected, Permitted remain separate | Trust records and permission/router types are separate | **Partial:** AUD-P16-002 collapses normal startup diagnostics |
| One request deadline covers handshake through cleanup | Absolute deadlines in protocol host and sandbox runner | **Partial:** AUD-P16-001 breaks pre-`ready` cancellation semantics |
| Wire messages are bounded and stateful | `opi-protocol::execution::v1` bounds, codec, session | Protocol all-target tests passed |
| Public failures redact command/env/path/backend text | Failure mapping and backend diagnostic redactor | Failure/redaction/product tests passed; no leak found |
| `started` is an atomic target-release gate | Sandbox helper/runner release-gate design | Sandbox all-target tests passed on Windows; native macOS/Linux runtime not rerun |
| Opi does not link `opi-sandbox` or native restriction policy | Cargo graph and crate-boundary guards | Crate-boundary tests passed |
| Windows reports L0 supervision only | Windows posture and local contract | Windows posture and L0 tests passed |
| Platform evidence is durable and independently auditable | Phase snapshot C9-C12/C16 | **Not met:** AUD-P16-003 |
| EN/ZH/product documentation stays synchronized | Documentation contract script and guards | `opi-doc-check.py` passed |

## Per-task summary

| Task | Ledger status | Audit disposition |
|---|---|---|
| 16.1 | passing | Minor residual ledger inconsistency (AUD-P16-005) |
| 16.2 | passing | No task-specific finding |
| 16.3 | passing | Protocol bounds/schema/session suites passed |
| 16.4 | passing | Contribution hard-gate coverage passed |
| 16.5 | passing | Informational duplication in drift invalidation (AUD-P16-008) |
| 16.6 | passing | Lifecycle diagnosis gap and unused redacted payload (AUD-P16-002, AUD-P16-009) |
| 16.7 | passing | Major pre-`ready` cancellation gap (AUD-P16-001) |
| 16.8 | passing | No separate runtime-assembly finding; affected by AUD-P16-002 |
| 16.9 | passing | Production startup loses lifecycle specificity (AUD-P16-002) |
| 16.10 | passing | Interactive permission coverage passed |
| 16.11.1 | passing | Runner maintainability gap (AUD-P16-006) |
| 16.11.2 | passing | Standalone CLI and smoke tests passed on Windows posture |
| 16.12 | passing | Cancellation semantics and duplicated cleanup paths (AUD-P16-001, AUD-P16-007) |
| 16.13 | passing | Native evidence unavailable; scenario status stale (AUD-P16-003, AUD-P16-005) |
| 16.14.1 | passing | Native evidence unavailable; scenario status stale (AUD-P16-003, AUD-P16-005) |
| 16.14.2 | passing | Windows tests passed; preserved evidence absent and scenario stale (AUD-P16-003, AUD-P16-005) |
| 16.15.1 | passing | Packaging structure tests passed |
| 16.15.2 | passing | Artifact-auditor fixtures passed, but real bundle is absent and scenario stale (AUD-P16-003, AUD-P16-005) |
| 16.16.1 | passing | Migration tests passed; scenario status stale (AUD-P16-005) |
| 16.16.2 | passing | Product tests passed; focused package CLI subprocess tests are non-hermetic (AUD-P16-004) |
| 16.16.3 | passing | Documentation check passed; final evidence-preservation claim is not reproducible (AUD-P16-003) |

## Verification performed

All commands below ran against the isolated detached worktree at `21dfcd8836974cd7e12454774156b3aefa97f2b5`.

- `cargo fmt --check --all` — passed.
- `cargo test -p opi-protocol --all-targets` — passed (unit, contract, and schema suites).
- `cargo test -p opi-sandbox --all-targets` — passed on Windows, including protocol, SDK, CLI, policy-model, process-tree, and standalone-smoke coverage.
- Focused `opi-coding-agent` execution suites — passed:
  - `execution_failures`
  - `execution_routing`
  - `execution_minimal_runtime`
  - `phase16_crate_boundaries`
  - `execution_migration`
  - feature-gated `execution_product`
  - `execution_config`
  - `execution_contribution_manifest`
  - `execution_package_lifecycle`
  - `execution_permission`
  - `execution_protocol_host`
  - `execution_runtime`
  - `execution_selected_routing`
  - `artifact_audit_script` (81 tests)
  - `opi_sandbox_packaging`
  - `opi_sandbox_release_topology`
  - `package_adapter_example`
  - `package_store`
  - `sandbox_l0`
  - `windows_execution_posture`
- `cargo test -p opi-coding-agent --test package_cli` — 28 passed, 8 failed at the hard-coded binary-path precondition (AUD-P16-004), even after `cargo build -p opi-coding-agent` succeeded in the configured external target directory.
- `python scripts/opi-doc-check.py` — `opi documentation contracts: PASS`.

## Limits and residual risk

- This host is Windows. Native Linux Landlock/seccomp and macOS Seatbelt enforcement were reviewed statically but not executed locally.
- The phase snapshot's claimed native and six-target evidence bundle was unavailable, which is itself AUD-P16-003.
- The full workspace clippy, all-target workspace test, doctest, and rustdoc release gates were not repeated; focused Phase 16 gates were prioritized.
- Runtime code, tests, specifications, and ledger were not modified. Test impact: `none` (audit artifact only).
