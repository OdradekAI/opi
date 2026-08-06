# Phase 16 Pluggable Extensions and Command Execution -- Independent Code Audit

**Auditor**: gpt5-codex (independent, no prior audit reports consulted)
**Date**: 2026-08-06
**Scope**: Tasks 16.1--16.16.3; task implementation commits `1021842c937653de545cd335450df985f822bd06` through `f8aff0237221fbf7d56b58abb5dce02833344bfc` (task-graph baseline `6f51761b6cde3eb309fca63935229412cccef209`); current implementation inspected at `8b547dae65ea1143dde68501fb15683ac20823dc`
**Method**: Read the Phase 16 snapshot, both normative specifications, project guidance, relevant production modules, tests, scripts, and CI/release definitions. Traced the lifecycle and protocol invariants across `opi-protocol`, `opi-coding-agent`, and `opi-sandbox`; inspected the original task range and current remediation state; and ran focused current-HEAD tests and repository gates on Windows. Native Linux/macOS policy behavior was source-audited but could not be executed on this host.

---

## 1. Executive Summary

**Verdict: FAIL**

| Severity | Count |
|----------|-------|
| Blocker  | 0     |
| Major    | 10    |
| Minor    | 6     |
| Info     | 1     |

Phase 16 has a substantial and generally well-tested implementation, including fail-closed routing, immutable executable launch material, bounded protocol codecs, process-tree cleanup, and redacted public diagnostics. It is not ready to close, however: ten major findings remain across protocol deadlines and state semantics, selected-package isolation, native CLI fidelity, macOS restriction establishment, artifact architecture verification, and cross-platform acceptance coverage. Several recorded phase-exit and documentation claims also contradict the current repository.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 16.1 | Pin the Phase 16 documentation contract | PASS |
| 16.2 | Pin L0 supervision and define the policy-neutral seam | PASS |
| 16.3 | Add `opi-protocol::execution::v1` | PASS |
| 16.4 | Parse and hard-gate executable contributions | PASS |
| 16.5 | Add Package Trust and enable/disable lifecycle | PARTIAL -- startup touches unselected packages |
| 16.6 | Add execution configuration, failures, routing, and permission policy | PASS |
| 16.7 | Implement the one-shot execution protocol host | FAIL -- deadline and effective-contract defects |
| 16.8 | Build the deep Execution Runtime assembly | PARTIAL -- host deadline boundary and duplicated dispatch |
| 16.9 | Wire Execution Runtime, dynamic bash schema, and public surfaces | PARTIAL -- eager all-package activation |
| 16.10 | Add the interactive permission broker and TUI prompt | PASS |
| 16.11.1 | Build the standalone `opi-sandbox` SDK and runner | PASS |
| 16.11.2 | Build the human `opi-sandbox` CLI and direct smoke | FAIL -- native argv loss and stale help |
| 16.12 | Add the atomic helper gate and protocol backend | FAIL -- premature `accepted` and macOS setup proof |
| 16.13 | Port the Linux native restriction contract | PARTIAL -- inherited-descriptor proof is weak |
| 16.14.1 | Port the macOS native restriction contract | FAIL -- setup acknowledgement and path fidelity defects |
| 16.14.2 | Pin the Windows unsupported execution posture | PASS |
| 16.15.1 | Build host-neutral `opi-sandbox` packaging | PARTIAL -- executable architecture is unchecked |
| 16.15.2 | Wire native package CI, release, and artifact audit | FAIL -- strict audit accepts mislabeled binary bytes |
| 16.16.1 | Remove core native sandbox and enforce migration boundaries | PASS |
| 16.16.2 | Prove install-to-execute and cross-surface diagnostics | PARTIAL -- protocol and OS acceptance gaps remain |
| 16.16.3 | Synchronize documentation and close Phase 16 repository gates | PARTIAL -- stale docs and contradictory snapshot state |

---

## 2. Correctness Findings

### 2.1 MAJOR: Deadline cancellation is reported as user cancellation

**File:** `crates/opi-coding-agent/src/execution/protocol_host.rs`
**Lines:** 929--1006
**Cause:** `finish_with_cancel` receives `CancelReason`, but a confirmed `Completed` terminal is normalized only by setting `cancelled = true`. It neither sets `timed_out` for `CancelReason::Deadline` nor clears an inconsistent backend-supplied flag.
**Impact:** A command stopped because its deadline elapsed can surface as `cancelled=true, timed_out=false`. Public tool results and diagnostics therefore describe the wrong terminal cause, and external cancellation can retain an unrelated timeout bit supplied by the adapter.
**Fix:** Normalize both fields from the host-owned reason before finalization: deadline means `timed_out=true, cancelled=false`; external cancellation means `cancelled=true, timed_out=false`. Alternatively reject inconsistent terminal flags, but do not trust the backend to classify the host event.
**Spec ref:** `crates/opi-protocol/src/execution/v1/frames.rs:256--265`; Phase 16 design, “Cancellation and cleanup.”
**Test gap:** Add confirmed-cleanup cases for both deadline and external cancellation and assert the exact public flags.

### 2.2 MAJOR: Human CLI cannot preserve native Unix paths or arguments

**File:** `crates/opi-sandbox/src/main.rs`; `crates/opi-sandbox/src/cli.rs`
**Lines:** `main.rs:9--12`; `cli.rs:108--139,441--452`
**Cause:** The binary collects `std::env::args()` into `Vec<String>`, and the parser stores target arguments in `Vec<String>`. On Unix, `args()` panics when any argv element is not valid UTF-8.
**Impact:** The SDK and protocol preserve platform-native strings, but the required standalone `run` surface cannot execute valid non-UTF-8 programs, workspaces, or arguments. This makes the two public execution surfaces semantically inconsistent.
**Fix:** Collect `args_os`, retain `OsString`/`PathBuf` for workspace, program, and target arguments, and convert only the fixed option names and closed option values to UTF-8.
**Spec ref:** Phase 16 design, “Human CLI” and “Native strings.”
**Test gap:** Add a Unix process-level test with a non-UTF-8 program argument and workspace/path component.

### 2.3 MAJOR: Backend emits `accepted` before the request is semantically valid

**File:** `crates/opi-sandbox/src/backend.rs`
**Lines:** 253--268
**Cause:** `Accepted` is emitted and flushed before `helper::build_request` validates the `ExecutePayload`. A zero timeout and malformed Windows native-string encoding therefore receive `accepted` followed by `failed{protocol_violation,handshake}`.
**Impact:** The implementation violates the wire meaning of `accepted`, so hosts cannot rely on that milestone to mean that the request is valid. This weakens the one-shot protocol state contract at an untrusted process boundary.
**Fix:** Perform all side-effect-free semantic validation before emitting `Accepted`; keep platform/restriction setup failures after `Accepted` where appropriate.
**Spec ref:** Phase 16 design line 499: “`accepted` means the request is valid and the target has not started.”
**Test gap:** `crates/opi-sandbox/tests/protocol_conformance.rs:1067--1082` calls zero timeout semantically invalid but asserts only the later failure. Assert that no `Accepted` frame precedes invalid-request failure.

---

## 3. Security and Redaction Findings

### 3.1 MAJOR: macOS profile paths are constructed with lossy conversion

**File:** `crates/opi-sandbox/src/platform/macos.rs`
**Lines:** 131--139, 310--323
**Cause:** Canonical workspace and invocation-temp paths are converted with `to_string_lossy()` before being embedded in the Seatbelt profile.
**Impact:** A non-UTF-8 path is changed rather than represented exactly or rejected. The resulting profile can deny the real workspace while granting a different path containing replacement characters, yet the invocation reports `restricted`.
**Fix:** Use an exact Seatbelt-compatible encoding if available. If the profile language cannot represent the native path losslessly, reject the invocation before spawn and before `Started`.
**Spec ref:** Phase 16 design, “macOS” and the effective `workspace-write` restriction contract.
**Test gap:** Add a native macOS test using a non-UTF-8 workspace component and assert exact confinement or pre-start refusal.

### 3.2 MAJOR: macOS reports `Started/restricted` before proving the invocation profile was accepted

**File:** `crates/opi-sandbox/src/runner.rs`; `crates/opi-sandbox/src/platform/macos.rs`
**Lines:** `runner.rs:367--423,475--496,629--666`; `macos.rs:300--344`
**Cause:** The runner trusts the earlier generic `sandbox-exec` availability probe, spawns the per-invocation launcher, and immediately exposes a restricted `SandboxRun`. The inner bootstrap creates `${gate}.probe` only after `sandbox-exec` has accepted the rendered profile, but the runner never waits for that probe before emitting `Started`.
**Impact:** A rejected or malformed per-invocation profile can produce `Started{guarantee:"restricted"}` before the launcher exits. The target remains fail-closed behind the release gate, but protocol consumers receive a false setup milestone and the failure is misclassified as post-start.
**Fix:** Add a bounded in-profile acknowledgement: wait for the probe, detect early launcher exit, and map rejection to pre-start `RestrictionSetup`. Only then expose the `Started` event.
**Spec ref:** Phase 16 design lines 500--502 and 711--713; macOS contract requiring failure before target execution when the profile cannot be established.
**Test gap:** Add fake-launcher early-exit/rejected-profile cases plus native macOS coverage proving the acknowledgement occurs inside Seatbelt.

No command, environment, backend stderr, or credential leak was found in the reviewed public failure surfaces. Hostile-backend redaction tests passed, and backend process stderr remains a bounded tracing-only sink.

---

## 4. Test Quality Findings

### 4.1 MAJOR: Feature-gated execution acceptance runs only on Ubuntu

**File:** `.github/workflows/ci.yml`
**Lines:** 44--74
**Cause:** The ordinary workspace test job uses Linux, Windows, and macOS, but the feature-gated product/protocol/runtime acceptance job is fixed to `ubuntu-latest`.
**Impact:** The deepest real-process host acceptance never exercises macOS descriptor launch or Windows suspended-process/Job-Object paths on their native CI operating systems. A critical platform-specific regression can pass the release gates.
**Fix:** Matrix the feature-gated execution-acceptance job over Ubuntu, macOS, and Windows; build the mock peer and run the same product, protocol-host, routing, and runtime suites on each.
**Spec ref:** Phase 16 design, “Required acceptance matrix” and native platform contract.

### 4.2 MINOR: Linux inherited-descriptor test does not deliberately inherit one

**File:** `crates/opi-sandbox/tests/linux_policy.rs`
**Lines:** 271--294
**Cause:** The test counts target descriptors but does not create a known inheritable high-numbered descriptor or `AF_INET` socket. Runtime-owned descriptors are normally `CLOEXEC`, so the count can stay low even if the explicit closure step regresses.
**Impact:** The test does not directly prove the stated descriptor-closure invariant, especially the network-socket case.
**Fix:** Create a non-`CLOEXEC` file descriptor and an inheritable INET socket in the parent, execute the target, and assert both exact descriptors are absent.
**Spec ref:** Phase 16 Linux contract requiring closure of inherited nonessential INET descriptors.

---

## 5. Spec Compliance Findings

### 5.1 MAJOR: Manifest handshake timeout does not bound the Initialize write

**File:** `crates/opi-coding-agent/src/execution/protocol_host.rs`
**Lines:** 201--216, 274--285, 738--762
**Cause:** The host computes `handshake_deadline`, but sends and flushes `Initialize` using the overall hard deadline. `write_frame` may therefore spend up to its 500 ms write allowance after a much shorter manifest handshake timeout has expired.
**Impact:** A non-reading or pipe-blocked adapter can violate its locked `handshake_timeout_ms` before negotiation even begins. The advertised per-adapter handshake bound is not enforced over the full handshake.
**Fix:** Bound the Initialize write, and any pre-read spawn/attach elapsed check, by `handshake_deadline` rather than `hard_deadline`.
**Spec ref:** Phase 16 contribution manifest and protocol-host handshake timeout requirements.
**Test gap:** Use a non-reading mock, a sufficiently large bounded adapter configuration, and a 1 ms handshake timeout; assert the elapsed bound.

### 5.2 MAJOR: Host accepts empty or meaningless effective-contract fields

**File:** `crates/opi-protocol/src/execution/v1/frames.rs`; `crates/opi-coding-agent/src/execution/protocol_host.rs`
**Lines:** `frames.rs:216--227`; `protocol_host.rs:495--504,649--667,969--977`
**Cause:** `StartedPayload` carries raw `String` values for placement, guarantee, and policy. The host state machine accepts every `Started` payload and copies these values verbatim into a successful outcome.
**Impact:** An external adapter can complete successfully while reporting empty or whitespace-only effective placement, guarantee, or policy. This defeats the requirement that guarantees come from each invocation’s established effective contract rather than adapter identity.
**Fix:** Validate required `Started` fields before entering `Draining`; reject empty/whitespace values as `protocol_violation`. Consider constrained protocol types for universally required non-empty fields while retaining adapter-defined vocabulary.
**Spec ref:** Phase 16 design lines 152--154 and 500--502.
**Test gap:** Add normal and cancellation-path tests for empty and whitespace-only `Started` fields.

### 5.3 MAJOR: Artifact verification checks labels and hashes, not executable architecture

**File:** `scripts/opi-sandbox-package.py`; `scripts/opi-artifact-audit.py`
**Lines:** `opi-sandbox-package.py:242--315`; `opi-artifact-audit.py:820--951`
**Cause:** Verification reconciles the target text, filename, manifest, lock, member layout, and hashes, but never parses the extracted executable’s ELF or Mach-O architecture.
**Impact:** A text fixture or a binary built for a different CPU can be labeled with the expected target, hashed consistently, and pass the “strict” packaging and release audit. Wrong-target release evidence is therefore not actually rejected.
**Fix:** Parse ELF class/`e_machine` and Mach-O magic/`cputype` from the extracted executable and compare it with the declared target; reject unknown formats and mismatches.
**Spec ref:** Phase 16 artifact audit requirement to reject wrong-target evidence.
**Test gap:** `crates/opi-coding-agent/tests/artifact_audit_script.rs:246--250,602--610` and `opi_sandbox_packaging.rs:287--299` use arbitrary fixture bytes as successful binaries. Replace or supplement them with minimal valid ELF/Mach-O headers and negative architecture cases.

---

## 6. Cross-task Integration Findings

### 6.1 MAJOR: Startup revalidates and can mutate every enabled package, not only the selected package

**File:** `crates/opi-coding-agent/src/harness.rs`; `crates/opi-coding-agent/src/package_activation.rs`
**Lines:** `harness.rs:151--205`; `package_activation.rs:344--373,537--580`
**Cause:** Routed startup always calls `usable_enabled_identities`, which loops over every trusted and enabled record and invokes `activate`. `activate` performs full lock/hash revalidation and durably clears trust and enablement on drift, even for packages unrelated to a fixed backend or the rule selected for the current mode.
**Impact:** Starting Opi with one named adapter can incur unrelated package I/O and mutate trust state for packages that were not selected. This contradicts the selected-only discovery contract and creates cross-package side effects during ordinary startup.
**Fix:** Make discovery strategy-aware. Model routing may validate the candidates it exposes; fixed routing should resolve only its configured adapter/package; rules should resolve only the first matching selected identity for the run mode. Preserve invocation-time revalidation of the actual selected package.
**Spec ref:** Phase 16 design lines 368--380; task 16.5 definition of done.
**Test gap:** Configure one selected valid package and one unrelated drifted package, then assert fixed/rules startup neither activates nor invalidates the unrelated record.

### 6.2 MINOR: Maintainer workspace graph omits both Phase 16 crates and an internal dependency

**File:** `AGENTS.md`; `CLAUDE.md`; `crates/opi-coding-agent/tests/phase16_extension_docs.rs`
**Lines:** `AGENTS.md:120--130`; `CLAUDE.md:120--130`; `phase16_extension_docs.rs:384--448`
**Cause:** Both guidance files still show the original four-crate graph and omit `opi-protocol`, `opi-sandbox`, and the `opi-coding-agent -> opi-protocol` edge. The documentation guard checks only broad markers anywhere in each file, so this stale graph still passes.
**Impact:** Maintainers and agents receive incorrect architecture and dependency guidance, increasing the chance of invalid workspace dependency changes.
**Fix:** Update both guidance files in lockstep and make the docs test assert the six-crate graph and the `opi-protocol` dependency edge explicitly.

### 6.3 MINOR: Routed dispatch duplicates an “impossible” error path

**File:** `crates/opi-coding-agent/src/execution/runtime.rs`
**Lines:** 425--440, 492--510, 575--595
**Cause:** The runtime declares a missing non-local adapter provably unreachable, then implements the same defensive missing-adapter error in two dispatch paths.
**Impact:** The duplicated eligibility/adapter coupling can diverge, and it conflicts with the repository rule against defensive handling for cases represented as impossible.
**Fix:** Represent selection and dispatch with one construction-time validated type or shared dispatch function so an eligible external identity necessarily carries its adapter.

---

## 7. Residual and Standards Findings

### 7.1 MINOR: Phase-exit snapshot contradicts its acceptance-scenario state

**File:** `docs/snapshots/phase16/opi-impl-state.json`
**Lines:** 2125--2151, 2262--2288, 2396--2409, 2606--2618, 2739--2754, 3190--3194
**Cause:** Seven acceptance scenarios remain `open` (`SC16-10`, both `SC16-09b` variants, `SC16-11`, `SC16-12a`, `SC16-12b`, and `SC16-15a`), while `phase_exit.exit_criteria_met` is true and its summary claims 16/16 criteria met.
**Impact:** The canonical audit ledger gives two incompatible answers about whether native and artifact acceptance was completed.
**Fix:** Regenerate or correct the canonical snapshot through the implementation-state workflow so scenario status and phase exit agree.

### 7.2 MINOR: Shipped help and platform module still claim macOS restriction is future work

**File:** `crates/opi-sandbox/src/cli.rs`; `crates/opi-sandbox/src/platform/mod.rs`
**Lines:** `cli.rs:537--547`; `platform/mod.rs:1--14`
**Cause:** Help says native restriction “lands in later tasks,” while the platform module says macOS remains unsupported and direct run refuses there.
**Impact:** User-facing and maintainer-facing behavior claims contradict the shipped Linux/macOS posture.
**Fix:** Describe current Linux/macOS native support and Windows refusal accurately; add exact assertions to the documentation/help tests.

### 7.3 MINOR: Public library errors bypass the repository’s `thiserror` convention

**File:** `crates/opi-coding-agent/src/tool/process_tree.rs`; `crates/opi-sandbox/src/cli.rs`; `crates/opi-sandbox/src/process_tree.rs`
**Lines:** `tool/process_tree.rs:115--147`; `cli.rs:66--106`; `process_tree.rs:51--76`
**Cause:** These public error types manually implement `Display` and `Error` even though project guidance prefers `thiserror` for library errors.
**Impact:** The code is correct, but error definitions are more verbose and easier to make inconsistent than neighboring library error types.
**Fix:** Derive `thiserror::Error` while preserving the exact redacted messages and public fields.

### 7.4 INFO: Adapter identity remains an unconstrained primitive across trust and routing

**File:** `crates/opi-coding-agent/src/config.rs`; `crates/opi-coding-agent/src/execution/contribution.rs`; `crates/opi-coding-agent/src/execution/router.rs`; `crates/opi-coding-agent/src/execution/runtime.rs`; `crates/opi-coding-agent/src/package_activation.rs`
**Lines:** `config.rs:139--159`; `contribution.rs:108--126`; `router.rs:72`; `runtime.rs:216--217,527,616--617`; `package_activation.rs:157--170,204`
**Cause:** Security-relevant adapter IDs are validated at some boundaries but then carried as plain `String` values through configuration, trust, permission, routing, and dispatch.
**Impact:** This is not a demonstrated behavior defect, but it makes invalid identity states representable and forces cross-module string conventions.
**Fix:** Introduce a validated `AdapterId` newtype at configuration and manifest boundaries and carry it through lifecycle and routing types.

---

## 8. Invariant Verification

| Invariant | Code evidence | Test coverage / result |
|-----------|---------------|------------------------|
| Minimal Runtime default does not scan package state | `harness.rs` classifies direct local before `execution_wiring`; `ExecutionRuntime::build` direct branch | Covered by `execution_minimal_runtime` and runtime sentinel tests; PASS |
| Selected external failure never falls back to local | Router/runtime return stable failures after selection | Product/runtime `no_local_fallback` tests passed; PASS |
| Model cannot mutate install, trust, enablement, policy, or grants | Model schema exposes only eligible backend selection | Routing/product permission tests passed; PASS |
| Command is undisclosed until validated `Ready` | Host sends `Initialize`, validates identity/version/target, then sends `Execute` | Protocol-host ordering and mismatch tests passed; PASS |
| `accepted` means request valid and target not started | Backend emits it before `build_request` | FAIL -- finding 2.3 |
| `Started` is flushed before target release | Backend emits/flushed `Started`, then calls `run.release`; release-file gate blocks target | Runner/backend tests passed; PASS |
| `Started` reports an established effective contract | Generic host accepts empty fields; macOS does not await in-profile acknowledgement | FAIL -- findings 3.2 and 5.2 |
| One absolute deadline bounds the whole invocation | Host clock starts pre-spawn, but Initialize write uses hard deadline rather than handshake deadline | PARTIAL -- finding 5.1 |
| Deadline and user cancellation remain distinguishable | Wire has separate flags; host rewrites both causes to cancellation | FAIL -- finding 2.1 |
| Target never inherits protocol stdin | Backend pins `StdinPolicy::Null` | Helper and direct CLI tests passed; PASS |
| Cleanup failure never becomes degraded success | Host/backend require confirmed cleanup and fail closed otherwise | Cleanup/cancellation/tree tests passed; PASS |
| Executed adapter bytes match validated bytes | Unix uses immutable bound launch material; Windows keeps no-write/no-delete-sharing handle | Contribution/activation/runtime inspection and tests; PASS |
| Runtime resolves only the named selected package | Startup activates all trusted+enabled records | FAIL -- finding 6.1 |
| Diagnostics and public failures are redacted | Closed failure codes, curated diagnostics, bounded backend stderr excluded from surfaces | Hostile-backend and redaction tests passed; PASS |
| Linux/macOS artifacts match declared platform/architecture | Scripts reconcile labels/hashes but do not parse executable format | FAIL -- finding 5.3 |
| Core does not link native restriction implementation | `opi-sandbox` depends only on `opi-protocol`; core boundary guards reject old sandbox surface | Crate-boundary and migration tests passed; PASS |

---

## 9. Validation Evidence and Recommendations

### Commands run at current HEAD

- `cargo fmt --check --all` -- PASS
- `cargo clippy --workspace --all-targets -- -D warnings` -- PASS
- `cargo test -p opi-protocol -p opi-sandbox` -- PASS (all executed unit, integration, smoke, boundary, conformance, SDK, and doc tests)
- Built `execution_backend_mock`, then ran `execution_product`, `execution_protocol_host`, `execution_routing`, and `execution_runtime` with `execution-backend-test-fixture` -- PASS (22 + 38 + 20 + 14 tests)
- `cargo test -p opi-coding-agent --test phase16_extension_docs` -- PASS (9 tests; finding 6.2 explains the assertion gap)

The core Phase 16 suites were also run against the original terminal task commit `f8aff0237221fbf7d56b58abb5dce02833344bfc` from an isolated source archive and passed. The findings above are based on the current implementation, so issues repaired after that task commit are not carried forward as current defects.

### Validation limits

- Audit host: Windows.
- Native Linux Landlock/seccomp and macOS Seatbelt behavior was not executed locally. Source, tests, scripts, and CI topology were inspected; findings 3.1, 3.2, 4.1, and 4.2 require native confirmation after remediation.
- No network or real model/provider credentials were used.

### Priority recommendations

1. Repair protocol semantics first: bound Initialize by the handshake deadline, normalize terminal timeout/cancel flags, validate effective `Started` fields, and move request validation before `Accepted`.
2. Close the macOS pre-start contract with an in-profile acknowledgement and lossless-or-reject path handling.
3. Make package discovery selected-only for fixed/rules routing and add an unrelated-drift regression test.
4. Verify actual ELF/Mach-O architecture and run feature-gated process acceptance on all supported CI operating systems.
5. Reconcile the phase snapshot and documentation claims, then strengthen their guards before re-running phase exit.
