# Phase 16 Remediation Plan

**Date**: 2026-08-10
**Finding sources**:

- `docs/snapshots/phase16/audit.codex.md` (`audit`, model `codex`, independence `unknown`)
- `docs/snapshots/phase16/audit.glm5.2.md` (`audit`, model `glm5.2`, independence `fresh-context-same-family`)

**Implementation commit range**: `1021842c937653de545cd335450df985f822bd06..f8aff0237221fbf7d56b58abb5dce02833344bfc`
**Verification head**: `4207dd071e7f0c121f708245710ae3f58d451143`
**Design specs**: `docs/opi-spec.md`; `docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md`
**Mode**: recommended execution approved by the user; unblocked remediation
implemented without commits or ledger mutation

The two sources provide no `independent-family` reviewer. Codex reports
`unknown` independence and GLM reports a fresh context from the same model
family. Source agreement is therefore degraded overlap, never independent
consensus.

The GLM report contains twelve actionable narrative findings without complete
normalized YAML blocks (`S2`, `S3`, `S4`, `S6`, `D1`-`D7`, and `M1`). They are
ingested as `degraded-legacy-input`; their original wording and severities are
preserved. `glm5.2-A2` and `glm5.2-A3` also use compound confidence strings
outside the finding contract and are treated as degraded input without silently
rewriting those fields. The report's already-refuted `S5` and `Spec-1` are
retained in scope exclusions rather than reintroduced as unverified findings.

---

## Finding cross-reference summary

| Cluster | Theme | Sources | Independence | Coverage | Source severity range | Final severity + rationale | Verification |
|---|---|---|---|---|---|---|---|
| C1 | Exact-cap CRLF protocol frame | codex `P16-STD-001` | unknown | Single source; no independent coverage | Major | Major; valid protocol input is rejected | Confirmed |
| C2 | Narrative/source-text assertions in Rust tests | codex `P16-STD-002`; GLM `D1` | unknown + same-family degraded | Degraded thematic overlap | Minor | Minor for exact narrative pins; structural guards are Info | Confirmed / Partially confirmed |
| C3 | Hand-written library error traits | codex `P16-STD-003`; GLM `S1` | unknown + same-family degraded | Correlated/degraded overlap | Minor / Minor | Minor; repository style violation without behavior loss | Confirmed |
| C4 | Activated contribution documentation | codex `P16-STD-004` | unknown | Single source; no independent coverage | Minor | Minor; implementation is correct but documentation is false | Confirmed |
| C5 | Magic bash operation-context JSON | codex `P16-STD-005` | unknown | Single source; no independent coverage | Minor | Minor; typed state is duplicated and silently defaulted | Confirmed |
| C6 | Duplicated cleanup grace | codex `P16-STD-006`; GLM `S4` | unknown + same-family degraded | Correlated/degraded overlap | Minor / Minor | Minor; current values agree but can drift | Confirmed |
| C7 | Public sandbox states without production constructors | codex `P16-STD-007` | unknown | Single source; no independent coverage | Minor | Info; reserved API is not itself an observable defect | Partially confirmed |
| C8 | Unix descendant escape through `setsid` | codex `P16-SPEC-001`; GLM narrative C7 PASS | unknown + same-family degraded | Contradictory degraded coverage | Major | Major; literal C7 is not met | Confirmed, including WSL2 mechanism reproduction |
| C9 | Text/TUI drop failure code and remediation | codex `P16-SPEC-002`; GLM narrative C4 PASS | unknown + same-family degraded | Contradictory degraded coverage | Major | Major; only NDJSON/RPC preserve the envelope | Confirmed |
| C10 | Teardown masks unconfirmed cleanup | codex `P16-SPEC-003` | unknown | Single source; no independent coverage | Major | Major; required cleanup classification is lost | Confirmed |
| C11 | Phase-exit evidence is not reproducible at current HEAD | codex `P16-SPEC-004` | unknown | Single source; no independent coverage | Major | Major for current-head evidence integrity; historical commands existed at phase exit | Confirmed as current-head gap |
| C12 | Passing ledger tasks retain open/null closure state | codex `P16-SPEC-005` | unknown | Single source; no independent coverage | Minor | Minor; phase summary does not record how task fields were superseded | Confirmed |
| C13 | Routed output silently truncated at 64 KiB | codex `P16-INTEG-001` | unknown | Single source; no independent coverage | Major | Major; successful output is irrecoverably lost | Confirmed |
| C14 | Standalone output silently truncated at 1 MiB | codex `P16-INTEG-002` | unknown | Single source; no independent coverage | Major | Major; direct byte pass-through and no-degraded-success fail | Confirmed |
| C15 | Missing macOS helper refusal lacks chained test | codex `P16-TEST-001` | unknown | Single source; no independent coverage | Minor | Minor; specified fail-closed plumbing is not tested end to end | Confirmed |
| C16 | macOS production dependency declared dev-only | GLM `glm5.2-A1` | same-family degraded | Single degraded source | Major | Major; Apple targets cannot compile | Confirmed |
| C17 | Artifact-audit reparse tests fail on Unix CI | GLM `glm5.2-A2` | same-family degraded | Single degraded source | Major | Minor diagnostic pending Linux rerun; production defect not established | Partially confirmed |
| C18 | Windows cancellation diagnostic bound failure | GLM `glm5.2-A3` | same-family degraded | Single degraded source | Major | Info pending reproduction; 11 focused repeats plus the full target passed | Cannot confirm |
| C19 | Audited branch is ahead and CI-red | GLM `glm5.2-A4` | same-family degraded | Single degraded source | Major | Major as a historical gate umbrella; not a separate code defect | Partially confirmed / duplicate |
| C20 | Protocol teardown handle data clump | GLM `S2` | same-family degraded | Single degraded source | Minor | Info; maintainability smell only | Confirmed |
| C21 | `too_many_arguments` execution functions | GLM `S3` | same-family degraded | Single degraded source | Minor | Info; ergonomics without incorrect behavior | Confirmed |
| C22 | Repeated failure-enum switches | GLM `S6` | same-family degraded | Single degraded source | Info | Info; only two distinct exhaustive owner mappings remain | Partially confirmed |
| C23 | Protocol-host dropped future not directly tested | GLM `D2` | same-family degraded | Single degraded source | Info | Info; direct test closes a literal C7 gap | Confirmed |
| C24 | Backend diagnostic redaction lacks a canary | GLM `D3` | same-family degraded | Single degraded source | Info | Info; two direct hostile-diagnostic canaries already exist | Refuted |
| C25 | Landlock TCP enforcement is masked by seccomp in tests | GLM `D4` | same-family degraded | Single degraded source | Minor | Minor; defense-in-depth path can regress unnoticed | Confirmed |
| C26 | Windows bootstrap variables reach the target | GLM `D5` | same-family degraded | Single degraded source | Info | Info; permitted by the documented Windows L0-only posture | Confirmed |
| C27 | Windows ARM64 release leg is non-blocking | GLM `D6` | same-family degraded | Single degraded source | Info | Info; explicit Tier 2 policy, while PR target checks remain strict | Confirmed / no defect |
| C28 | Windows absolute path gets drive-relative error | GLM `D7` | same-family degraded | Single degraded source | Info | Info; rejection is correct and only the message is cosmetic | Confirmed |
| C29 | New Linux clippy lint fails `-D warnings` | GLM `M1` | same-family degraded | Single degraded source | Minor | Minor; repository gate fails on the declared toolchain | Confirmed |

## Verification summary

- The canonical synchronous codec accepts exact-cap LF and CRLF frames, while
  the private async reader rejects the CRLF form before observing LF.
- The protocol-host target passed 53/53 tests on Windows. The historical
  `glm5.2-A3` test also passed ten additional direct repetitions, so its root
  cause remains unconfirmed.
- A WSL2 reproduction matching the current negative-PGID termination path
  demonstrated that a `setsid` descendant survives. No native macOS
  reproduction was available from this host.
- Focused current Windows artifact-audit reparse tests passed. The GLM report
  proves four failures on Ubuntu at its audited head; macOS never reached those
  tests because the dev-dependency error stopped compilation first.
- Current HEAD has no production-code changes after either audit head, so the
  static findings apply directly to the verified tree.

## Execution result

The approved unblocked scope is implemented: Fixes 2.1-2.5, 3.1-3.6,
3.8-3.10, D.1, and D.2. Windows focused gates pass, including the Layer 2
`opi-sandbox` scoped smoke, the expanded Layer 3 `opi-coding-agent` scoped
smoke, the 53-test protocol-host target, the 22-test execution-product target,
the feature-gated execution-runtime target, large binary output recovery,
text/TUI provider-recovery diagnostics, and all 79 current artifact-audit
tests. Warning-free production/test clippy and affected-crate rustdoc are part
of the scoped smoke results. The final Windows-host workspace gate also passes
in full (`fmt`, all-target clippy, workspace rustdoc, and all-target tests), as
do workspace doctests and `scripts/opi-doc-check.py`.

Fix 3.1 is structurally verified for both Apple target graphs: `tempfile` is a
normal dependency. Full Apple cross-checks stop in `ring` before this crate is
compiled because this Windows host has no macOS C cross-compiler. Native
Linux/macOS sandbox behavior, the four historical artifact-audit seams on
those hosts, and exact remediation-commit CI remain platform handoffs rather
than locally claimed evidence.

Fix 3.7 remains blocked by its explicit shaping precondition: no supported
Linux/macOS mechanism has yet been selected and proven to contain descendants
that call `setsid`. The historical Phase 16 evidence/ledger work likewise
remains with `opi-implement`; this remediation did not edit either canonical
ledger. Phase 16 therefore remains partially remediated rather than closed.

## Decision record

| ID | Finding cluster(s) | Decision | Rationale | Decided by |
|---|---|---|---|---|
| D1 | C1 | Give the async reader the canonical pending-CR semantics | One behavior is already fixed by the shared protocol contract | auto |
| D2 | C2 | Move narrative/phase prose checks to `opi-doc-check.py`; retain structural Rust guards only when behavior cannot express the invariant | Matches the repository testing policy without deleting useful architecture coverage | auto |
| D3 | C3, C4, C6 | Derive `thiserror`, correct activation rustdoc, and own cleanup grace once | Behavior-preserving corrections with one clear implementation | auto |
| D4 | C5, C13 | Introduce typed `BashOperationContext`, populate it in local/routed operations, and use it for output finalization | Removes JSON smuggling and gives routed output the established recoverable truncation contract. Alternative rejected: centralize the magic JSON while retaining the public shape | user (`recommended`) |
| D5 | C8, C23 | Preserve the full C7 descendant guarantee and strengthen Unix containment | Narrowing C7 would change product intent. A supported mechanism must be shaped and proven before implementation; do not claim a polling approximation as complete containment | user (`recommended`) |
| D6 | C9 | Project the same redacted diagnostic code/remediation into text stderr and TUI state; do not change recovered-run exit semantics in this remediation | Cross-surface presentation is normative; forced nonzero exit after provider recovery is not | auto |
| D7 | C10 | Centralize teardown confirmation and elevate any unconfirmed termination/reap/drain to `cleanup_unconfirmed` | The partial-transmission path already establishes the required policy | auto |
| D8 | C11, C12 | Do not edit the archived ledger in remediation. Hand evidence recovery or historical-claim correction to the guarded `opi-implement` owner | `opi-remediate` is forbidden from mutating `.opi-impl-state.json`; historical evidence cannot be fabricated | auto |
| D9 | C7, C14 | Emit existing `SandboxEvent::Output`/`Diagnostic` variants with bounded backpressure and preserve a bounded terminal preview | Satisfies byte pass-through and makes the public event surface real. Alternative rejected: private spooling that retains unproducible public events | user (`recommended`) |
| D10 | C15 | Add an injectable helper probe/launcher seam and chained missing/unusable-helper tests on macOS | One behavior-preserving test seam closes the specified pre-start path | auto |
| D11 | C16 | Move `tempfile` to normal `opi-coding-agent` dependencies | The production macOS cfg branch requires it | auto |
| D12 | C17 | Improve subprocess failure diagnostics, then rerun the exact tests on Ubuntu and macOS before changing production auditor logic | Current Windows passes and the historical failure lacks root-cause evidence | auto |
| D13 | C25 | Exercise Landlock TCP rules independently of the seccomp socket gate | Required defense-in-depth behavior needs an observable regression test | auto |
| D14 | C29 | Replace manual remainder tests with `is_multiple_of` | Mechanical current-toolchain correction | auto |
| D15 | C18, C20-C22, C26-C28 | Make no product change for unconfirmed, cosmetic, deliberate-policy, or low-value refactor findings | Minimum-change rule; no verified incorrect behavior | auto |
| D16 | C19 | Treat the historical CI-red finding as the final verification umbrella for C16, C17, C18, and C29 | Ahead-of-origin count is not itself a defect | auto |

## Preconditions and handoffs

### Unix descendant containment

Before Fix 3.7 begins, run a bounded architecture review for the Unix
`ProcessTree` implementation. The accepted behavior is fixed: timeout,
cancellation, future drop, and clean direct-child exit must kill a descendant
that calls `setsid` on every Unix platform for which C7 is claimed. The review
must select a real OS mechanism for Linux and macOS and demonstrate it with a
throwaway native prototype. If macOS cannot provide that guarantee, stop and
return to shaping; do not silently substitute a best-effort poller or narrow
the specification inside remediation.

### Historical evidence and ledger closure

The Phase 16 snapshot is an input, not a remediation-owned ledger. The owning
guarded workflow must:

1. attempt to recover authoritative Phase 16 CI/native artifacts with their
   original run and commit identities;
2. preserve recovered evidence in a tracked or durably retrievable location
   bound by digest;
3. if recovery is impossible, formally correct or withdraw the historical C16
   claim through an `opi-implement` checkpoint rather than reconstructing
   evidence;
4. reconcile the five null task-evidence fields and seven open acceptance
   scenarios, or record explicitly how the phase-exit trace superseded them;
5. replace stale current gate inventory with `opi-doc-check.py` and existing
   Cargo targets while preserving the historical command record.

This handoff is not authorization to edit either the root ledger or the
snapshot during remediation execution.

## Remediation layers

### Layer 2: `opi-sandbox`

**Verification**:

    cargo fmt --all
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/opi-impl-smoke.ps1 scoped --crate opi-sandbox --test cli_native_and_docs --test cli_contract --test sdk_contract --test backend_protocol_smoke --test macos_policy --test linux_policy
    cargo clippy -p opi-sandbox --all-targets -- -D warnings

#### Fix 2.1: Stream complete standalone output with bounded backpressure

- **Finding source**: `docs/snapshots/phase16/audit.codex.md` (`audit`, model `codex`, `P16-INTEG-002`, `P16-STD-007`)
- **Cluster**: C7, C14
- **Decision**: D9
- **Verification status**: Confirmed / Partially confirmed
- **File(s)**: `crates/opi-sandbox/src/runner.rs` ~L58, ~L427-L459, ~L1739-L1816; `crates/opi-sandbox/src/cli.rs` ~L259-L306; `crates/opi-sandbox/src/backend.rs` ~L558-L563, ~L778-L786; `crates/opi-sandbox/tests/sdk_contract.rs` ~L465
- **Change**: Emit stdout/stderr chunks as ordered `SandboxEvent::Output` events through a bounded channel. Emit runner diagnostics through `SandboxEvent::Diagnostic`. Make the direct CLI write event bytes immediately and make the protocol backend map them to stdout/stderr frames without duplicating terminal-preview bytes. Keep only a bounded preview and explicit preview-truncation metadata in the terminal result; never discard the only copy of successful output. Leave `UnsupportedPlatform` reserved rather than removing public API.
- **Test plan**: Add greater-than-1-MiB binary stdout and stderr cases through the isolated standalone executable and backend protocol; assert byte equality, ordering, exit zero, bounded buffering, cancellation under backpressure, and no duplicated bytes.

#### Fix 2.2: Chain macOS missing/unusable helper refusal through production

- **Finding source**: `docs/snapshots/phase16/audit.codex.md` (`audit`, model `codex`, `P16-TEST-001`)
- **Cluster**: C15
- **Decision**: D10
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-sandbox/src/platform/macos.rs`; `crates/opi-sandbox/src/runner.rs`; `crates/opi-sandbox/tests/macos_policy.rs` ~L25-L40
- **Change**: Add the smallest injectable helper-probe/launcher seam needed to drive `Missing` and `Unusable` posture through the real pre-start gate without modifying `/usr/bin`.
- **Test plan**: On macOS, assert both postures return setup/unavailable failure with CLI exit 125, emit the expected structured backend failure, and never start the target sentinel. Retain the available-helper native path.

#### Fix 2.3: Exercise Landlock TCP independently

- **Finding source**: `docs/snapshots/phase16/audit.glm5.2.md` (`audit`, model `glm5.2`, degraded narrative `D4`)
- **Cluster**: C25
- **Decision**: D13
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-sandbox/src/platform/linux.rs` ~L233-L335; `crates/opi-sandbox/tests/linux_policy.rs` ~L441-L465
- **Change**: Add a Linux-only test seam that installs the Landlock network layer without the seccomp socket-creation gate, solely for observing Landlock bind/connect enforcement.
- **Test plan**: On a real Linux host with Landlock ABI >= 4, assert TCP bind/connect denial, an allow control, and AF_UNIX preservation. Keep the existing combined production-policy tests.

#### Fix 2.4: Satisfy the current Linux clippy gate

- **Finding source**: `docs/snapshots/phase16/audit.glm5.2.md` (`audit`, model `glm5.2`, degraded narrative `M1`)
- **Cluster**: C29
- **Decision**: D14
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-sandbox/src/platform/linux.rs` ~L470-L473
- **Change**: Replace the two manual remainder checks with `is_multiple_of(4)` and `is_multiple_of(8)`.
- **Test plan**: Run the focused decoder/unit tests and Linux `cargo clippy -p opi-sandbox --all-targets -- -D warnings` on the declared toolchain.

#### Fix 2.5: Remove narrative prose ownership from Rust tests

- **Finding source**: `docs/snapshots/phase16/audit.codex.md` (`audit`, model `codex`, `P16-STD-002`); `docs/snapshots/phase16/audit.glm5.2.md` (`audit`, model `glm5.2`, degraded narrative `D1`)
- **Cluster**: C2
- **Decision**: D2
- **Verification status**: Confirmed / Partially confirmed
- **File(s)**: `crates/opi-sandbox/tests/cli_native_and_docs.rs` ~L230-L339
- **Change**: Delete exact help-sentence, source-comment, historical phase-token, and negative-prose assertions from this Rust test. Retain executable CLI grammar, exit behavior, and stable machine-contract assertions. Move source-derived documentation contracts to the final documentation layer. Do not delete the separate `opi-coding-agent` structural source guards unless equivalent behavioral coverage is added first.
- **Test plan**: Run `cli_native_and_docs`; demonstrate that harmless prose rewording no longer fails Rust tests while behavioral changes still do.

### Layer 3: `opi-coding-agent`

**Verification**:

    cargo fmt --all
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/opi-impl-smoke.ps1 scoped --crate opi-coding-agent --test sandbox_l0 --test non_interactive --test interactive_permission --test artifact_audit_script --test execution_contribution_manifest --test tools_read_write_edit_bash
    cargo test -p opi-coding-agent --features execution-backend-test-fixture --test execution_backend_mock --no-run
    cargo test -p opi-coding-agent --features execution-backend-test-fixture --test execution_protocol_host
    cargo test -p opi-coding-agent --features execution-backend-test-fixture --test execution_product

#### Fix 3.1: Restore macOS compilation

- **Finding source**: `docs/snapshots/phase16/audit.glm5.2.md` (`audit`, model `glm5.2`, `glm5.2-A1`)
- **Cluster**: C16
- **Decision**: D11
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/Cargo.toml` ~L77; `crates/opi-coding-agent/src/execution/contribution.rs` ~L481-L502
- **Change**: Move `tempfile = { workspace = true }` from `[dev-dependencies]` to normal `[dependencies]`; do not add a direct version.
- **Test plan**: Run library and all-target checks for both `x86_64-apple-darwin` and `aarch64-apple-darwin`, followed by the native macOS acceptance job.

#### Fix 3.2: Add typed bash operation context and recover routed full output

- **Finding source**: `docs/snapshots/phase16/audit.codex.md` (`audit`, model `codex`, `P16-STD-005`, `P16-INTEG-001`)
- **Cluster**: C5, C13
- **Decision**: D4
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/tool/operations.rs` ~L192-L240, ~L1003-L1110; `crates/opi-coding-agent/src/execution/runtime.rs` ~L693-L759; `crates/opi-coding-agent/src/tool/bash.rs` ~L225-L339
- **Change**: Define a typed `BashOperationContext` owned by `tool::operations` for cancellation, timeout, signal, effective contract, truncation, and recoverable full-output metadata. Add it to `BashResult`, populate it in local and routed implementations, and make `BashTool` consume it directly. Build public diagnostics/details from the typed value at the boundary. Centralize preview/finalization so routed output over 64 KiB uses the same capped preview, `truncated=true`, and complete `full_output` spill as local execution. Missing context must be an explicit internal error, not silent `false`/`None` defaults.
- **Test plan**: Add local/routed context parity tests, exact-64-KiB and 64-KiB-plus-one cases, byte-complete full-output recovery, and propagation of `truncated` through `ToolResultMessage`, `ToolExecutionEnd`, NDJSON, and RPC.

#### Fix 3.3: Align async line framing with the canonical codec

- **Finding source**: `docs/snapshots/phase16/audit.codex.md` (`audit`, model `codex`, `P16-STD-001`)
- **Cluster**: C1
- **Decision**: D1
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/execution/protocol_host.rs` ~L1330-L1370; `crates/opi-protocol/src/execution/v1/codec.rs` ~L64-L90
- **Change**: Give `CappedReader` a pending-CR state outside the data-size cap, matching the canonical reader's LF/CRLF delimiter treatment. Do not widen frame bounds.
- **Test plan**: Add async exact-cap LF and CRLF acceptance, cap-plus-one rejection, lone-CR, EOF-after-CR, and production host/mock-peer coverage.

#### Fix 3.4: Preserve cleanup failure classification on every teardown path

- **Finding source**: `docs/snapshots/phase16/audit.codex.md` (`audit`, model `codex`, `P16-SPEC-003`); `docs/snapshots/phase16/audit.glm5.2.md` (`audit`, model `glm5.2`, degraded narrative `S2`)
- **Cluster**: C10, C20
- **Decision**: D7
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/execution/protocol_host.rs` ~L511-L590, ~L995-L1285
- **Change**: Introduce one teardown outcome that records tree termination, child reap, and stderr drain confirmation. Route EOF, codec, transition, diagnostic-overflow, cancellation, and terminal-finalization failures through it. Return `cleanup_unconfirmed` whenever any component is unconfirmed while retaining the original failure as redacted diagnostic context. Bundle handles only where needed to make this correction; do not perform a standalone parameter-count refactor.
- **Test plan**: Inject each teardown-component failure across malformed, out-of-order, EOF, diagnostic-overflow, cancellation, and partial-frame paths; assert exact code precedence and redaction.

#### Fix 3.5: Own cleanup report grace once

- **Finding source**: `docs/snapshots/phase16/audit.codex.md` (`audit`, model `codex`, `P16-STD-006`); `docs/snapshots/phase16/audit.glm5.2.md` (`audit`, model `glm5.2`, degraded narrative `S4`)
- **Cluster**: C6
- **Decision**: D3
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/execution/protocol_host.rs` ~L53; `crates/opi-coding-agent/src/execution/runtime.rs` ~L168
- **Change**: Define the cleanup-report grace in one execution-owned module and use it for both host cancellation timing and runtime deadline expansion.
- **Test plan**: Retain `host_deadline_aligns_host_cancel_with_backend_timeout` and add one assertion that both calculations derive from the shared value.

#### Fix 3.6: Preserve runtime failures on text and TUI surfaces

- **Finding source**: `docs/snapshots/phase16/audit.codex.md` (`audit`, model `codex`, `P16-SPEC-002`)
- **Cluster**: C9
- **Decision**: D6
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/runner.rs` ~L438-L465, ~L680-L740; `crates/opi-coding-agent/src/interactive.rs` ~L980-L1074; `crates/opi-coding-agent/src/diagnostic_bridge.rs`
- **Change**: Add one shared redacted formatter for tool diagnostics. Text runners write stable code and remediation to stderr; the TUI stores/renders the same information instead of literal `failed`. Keep recovery text on stdout and preserve the current process exit result when the provider recovers.
- **Test plan**: Drive a routed bash `execution_failed` followed by provider recovery through the real text runner and headless TUI. Assert code/remediation visibility and redaction. Retain runner/server-level NDJSON and RPC assertions.

#### Fix 3.7: Contain Unix descendants that leave the process group

- **Finding source**: `docs/snapshots/phase16/audit.codex.md` (`audit`, model `codex`, `P16-SPEC-001`); `docs/snapshots/phase16/audit.glm5.2.md` (`audit`, model `glm5.2`, degraded narrative `D2` and contradictory C7 PASS narrative)
- **Cluster**: C8, C23
- **Decision**: D5
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/tool/process_tree.rs` ~L43-L51, ~L257-L382; `crates/opi-coding-agent/src/tool/supervision.rs`; `crates/opi-coding-agent/tests/sandbox_l0.rs` ~L432; `crates/opi-coding-agent/tests/execution_protocol_host.rs` ~L1131
- **Change**: After the Unix containment precondition selects and proves supported Linux/macOS mechanisms, integrate them behind `ProcessTree`/`TreeGuard` while retaining the process-group fast path and Windows Job Object behavior. Track and terminate descendants that call `setsid`; confirm reap before reporting cleanup. Do not merge a best-effort implementation that cannot satisfy the native tests.
- **Test plan**: On Linux and macOS, cover timeout, cancellation, execute-future drop, and clean direct-child exit with a `setsid` descendant. Add the direct protocol-host drop-the-future test. Retain ordinary background-descendant and bounded-drain tests.

#### Fix 3.8: Use repository error style

- **Finding source**: `docs/snapshots/phase16/audit.codex.md` (`audit`, model `codex`, `P16-STD-003`); `docs/snapshots/phase16/audit.glm5.2.md` (`audit`, model `glm5.2`, `glm5.2-S1`)
- **Cluster**: C3
- **Decision**: D3
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/execution/protocol_host.rs` ~L160-L194; `crates/opi-coding-agent/src/tool/process_tree.rs` ~L141
- **Change**: Replace manual `Display`/`Error` implementations for `ExecutionProtocolFailure` and `AttachError` with `thiserror::Error` derives that preserve the current redacted text and source behavior.
- **Test plan**: Retain error formatting/source tests, protocol-host tests, `sandbox_l0`, and focused clippy.

#### Fix 3.9: Correct activated-contribution rustdoc

- **Finding source**: `docs/snapshots/phase16/audit.codex.md` (`audit`, model `codex`, `P16-STD-004`)
- **Cluster**: C4
- **Decision**: D3
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/package_activation.rs` ~L19, ~L121-L132, ~L603
- **Change**: State that activation performs no spawn but returns metadata plus immutable validated executable launch material. Remove the false claim that no validated-bytes handle is carried.
- **Test plan**: Retain contribution binding/TOCTOU tests and run warning-free rustdoc for `opi-coding-agent`.

#### Fix 3.10: Make artifact-audit test failures diagnosable before changing production

- **Finding source**: `docs/snapshots/phase16/audit.glm5.2.md` (`audit`, model `glm5.2`, `glm5.2-A2`)
- **Cluster**: C17
- **Decision**: D12
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-coding-agent/tests/artifact_audit_script.rs` ~L421-L470, ~L1021-L1081, ~L2157-L2200, ~L2541-L2567; `scripts/opi-artifact-audit.py` ~L1375-L1491
- **Change**: Make the test harness include subprocess status, stderr, stdout, and parsed issue codes in assertion failures. Do not change production reparse/identity logic until the exact four tests reproduce on current Ubuntu or macOS and identify a root cause.
- **Test plan**: Run the four exact tests sequentially and in parallel on Ubuntu current HEAD, then on macOS after Fix 3.1. If they pass, close as historical/non-reproducible; if they fail, add the smallest root-cause regression before production changes.

### Final layer: documentation and public contract synchronization

**Verification**:

    python scripts/opi-doc-check.py

#### Fix D.1: Move stable Phase 16 prose contracts into the documentation checker

- **Finding source**: `docs/snapshots/phase16/audit.codex.md` (`audit`, model `codex`, `P16-STD-002`)
- **Cluster**: C2
- **Decision**: D2
- **Verification status**: Confirmed
- **File(s)**: `scripts/opi-doc-check.py`; paired documentation files only where the source-derived contract requires synchronization
- **Change**: Add stable source-derived checks for current Phase 16 product claims that were removed from Rust tests: Minimal Runtime, five independent gates, external no-fallback, standalone CLI/SDK, current migration surface, Windows posture, and declared non-goals. Check semantic tokens/structured sources rather than exact narrative sentences. Update English and Chinese counterparts together if wording changes.
- **Test plan**: Run the documentation checker; add negative fixtures or direct checker tests that prove a missing contract fails while harmless rewording passes.

#### Fix D.2: Document the selected public event/context behavior

- **Finding source**: `docs/snapshots/phase16/audit.codex.md` (`audit`, model `codex`, `P16-STD-005`, `P16-STD-007`, `P16-INTEG-001`, `P16-INTEG-002`)
- **Cluster**: C5, C7, C13, C14
- **Decision**: D4, D9
- **Verification status**: Confirmed / Partially confirmed
- **File(s)**: affected crate rustdoc/README files; `CHANGELOG.md` under `Unreleased`; localized counterparts when present
- **Change**: Document `BashOperationContext`, recoverable preview/full-output semantics, and the fact that sandbox output/diagnostic events are now emitted with bounded backpressure. Record the 0.x public behavior change without modifying a released changelog section.
- **Test plan**: Run `python scripts/opi-doc-check.py` and warning-free affected-crate rustdoc.

## Final verification

After every unblocked layer passes its focused gate:

    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/opi-impl-smoke.ps1 full
    cargo test --workspace --doc
    python scripts/opi-doc-check.py

Required platform evidence:

1. Linux: native `opi-sandbox` policy tests, isolated Landlock TCP test,
   `setsid` L0 tests, affected clippy, and artifact-audit reparse tests.
2. macOS: both Apple target checks, native sandbox helper refusal tests,
   `setsid` L0 tests, and the extracted standalone smoke.
3. Windows: affected protocol-host tests including a constrained stress run of
   `cancellation_diagnostic_frame_count_is_bounded`.
4. CI: all ordinary jobs and all six strict target checks green at the exact
   remediation commit.
5. Evidence handoff: from a clean clone, retrieve the bound artifacts and run:

       python scripts/opi-artifact-audit.py <artifact-dir> --workspace-root . --phase-exit --workflow-run-id <run-id> --commit-sha <full-sha> --json

Do not claim Phase 16 fully remediated while the Unix containment precondition
or historical-evidence handoff remains unresolved.

## Scope exclusions

| Finding | Status | Reason |
|---|---|---|
| C7 `UnsupportedPlatform` variant | Info/No action | Reserved public state causes no current incorrect behavior; do not remove intentional API in remediation |
| C11, C12 | Deferred to source owner | Ledger/evidence mutation belongs to guarded `opi-implement`, not `opi-remediate` |
| C18 / `glm5.2-A3` | Cannot confirm | Current full target and eleven focused Windows runs pass; retain as a stress/reproduction gate |
| C19 / `glm5.2-A4` | Duplicate | Underlying actionable failures are C16, C17, C18, and C29; ahead-of-origin count is not a defect |
| C20 / GLM `S2` beyond Fix 3.4 | Info/No action | Do not perform an independent handle-bundle refactor |
| C21 / GLM `S3` | Info/No action | Parameter count is ergonomic and multiple designs are reasonable |
| C22 / GLM `S6` | Info/No action | Remaining exhaustive matches encode distinct code/remediation contracts |
| C24 / GLM `D3` | Refuted | Hostile backend-diagnostic canaries already exist and pass |
| C26 / GLM `D5` | Info/No action | Windows Phase 16 is explicitly L0-only and not an environment-confidentiality boundary |
| C27 / GLM `D6` | Info/No action | Windows ARM64 non-blocking release behavior is the explicit Tier 2 policy; PR checks remain strict |
| C28 / GLM `D7` | Info/No action | Absolute Windows paths are safely rejected; only the cosmetic variant differs |
| GLM `S5` | Source-refuted | `routed_store_factory_override` is `#[cfg(test)]` at definition and call site |
| GLM `Spec-1` | Source-refuted | `BashTool` correctly makes timed-out/cancelled completed outcomes errors |

## Test impact

Planned impact: `update` existing protocol, sandbox, runner/TUI, documentation,
and artifact-audit tests; `add` exact-cap CRLF, routed large-output,
standalone streamed-output, macOS helper refusal, Landlock TCP isolation,
Unix `setsid`, and direct dropped-future coverage. No tests are deleted unless
their source-derived documentation contract is first represented by
`opi-doc-check.py` or equivalent behavioral coverage.
