# Phase 16 Remediation Plan

**Date**: 2026-08-10
**Finding sources**:

- `docs/snapshots/phase16/audit.codex.md` (`audit`, model `codex`, independence `unknown`)
- `docs/snapshots/phase16/audit.glm5.2.md` (`audit`, model `glm5.2`, independence `fresh-context-same-family`)

**Implementation commit range**: `1021842c937653de545cd335450df985f822bd06..f8aff0237221fbf7d56b58abb5dce02833344bfc`
**Verification head**: `c5de89216b316529d1c8c1c182fe496a3103f42f`
**Design specs**: `docs/opi-spec.md`; `docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md`
**Mode**: plan only (`execute=false`)
**Status**: plan confirmed with exact-byte manifest identity; execution still requires explicit opt-in

Neither report declares an `independent-family` reviewer. Cross-report agreement
is therefore correlated/degraded evidence, never independent consensus. The GLM
blocks `P16-correctness-02` and `P16-correctness-04` use the non-contract axis
`correctness`; this plan preserves that source field and ingests both blocks as
`degraded-legacy-input` rather than silently rewriting their provenance.

The previous checked-in remediation plan described superseded audit reports. It
has been replaced from the two current working-tree reports; no finding was
carried forward without current-code verification.

---

## Finding cross-reference summary

| Cluster | Theme | Source findings | Independence / coverage | Source severity range | Final severity + rationale | Verification |
|---|---|---|---|---|---|---|
| C1 | Triplicated capped JSONL framing | Codex `P16-CODEX-STD-001`; GLM `P16-integration-02` | unknown + same-family; correlated/degraded overlap | Minor / Info | Minor; three current owners can drift although no current semantic divergence was found | Confirmed |
| C2 | Protocol teardown state data clump | Codex `P16-CODEX-STD-002` | single unknown source | Minor | Minor; 12- and 16-parameter lifecycle functions weaken cleanup ownership | Confirmed |
| C3 | Duplicated config-finalization tail | Codex `P16-CODEX-STD-003` | single unknown source | Minor | Info; maintainability-only duplication with separate coverage | Confirmed |
| C4 | Adapter identities remain primitive strings | Codex `P16-CODEX-STD-004` | single unknown source | Minor | Info; validation and indexed candidates prevent a demonstrated identity mix-up | Confirmed as smell |
| C5 | In-band timeout loses `execution_timed_out` | Codex `P16-CODEX-SPEC-001` | single unknown source; GLM narrative disagrees | Major | Major; a valid timed-out `Completed` outcome emits the generic tool-failure code | Confirmed |
| C6 | Phase-exit acceptance is not replayable | Codex `P16-CODEX-SPEC-002` | single unknown source | Major | Minor; real historical provenance/closure defect, not a current runtime defect | Confirmed |
| C7 | Manifest identity uses incompatible raw/canonical bases | Codex `P16-CODEX-SPEC-003`; GLM `P16-spec-01` | unknown + same-family; correlated/degraded overlap | Minor / Minor | Minor; both the exact-byte mismatch and internal two-hash inconsistency are real; semantics require user decision | Confirmed / Partially confirmed |
| C8 | Host/backend/target cleanup composition is unproved | Codex `P16-CODEX-INT-001`; GLM `P16-correctness-03` | unknown + same-family; correlated/degraded overlap | Major / Major | Minor; separate process groups exist, but the audits omitted the Unix parent-death watchdog; target and temp-root cleanup still lack a real composition test | Partially confirmed |
| C9 | Backend protocol stdout blocks its async driver | Codex `P16-CODEX-INV-001`; GLM `P16-correctness-04` | unknown + same-family; correlated/degraded overlap; GLM axis degraded | Major / Minor | Minor; backend-side cancellation/deadline polling can stall, while the outer host still enforces its deadline | Partially confirmed / Confirmed |
| C10 | Adapter-controlled text crosses public redaction boundaries | Codex `P16-CODEX-SEC-001`; GLM `P16-security-03` | unknown + same-family; correlated/degraded overlap | Major / Info | Major; generic pattern redaction cannot remove arbitrary command/env text, and event diagnostics copy messages raw | Confirmed |
| C11 | Windows standalone smoke is not isolated | Codex `P16-CODEX-TQ-001` | single unknown source | Minor | Minor; the binary and cwd do not satisfy the mandatory isolated-executable acceptance path | Confirmed |
| C12 | Native early-return skips look like passing evidence | Codex `P16-CODEX-TQ-002` | single unknown source | Minor | Minor; required outside-write assertions can return successfully and evade artifact skip detection | Confirmed |
| C13 | Windows Job-Object FFI is duplicated | GLM `P16-standards-01` | single same-family source | Info | Info; intentional crate-boundary trade-off | Confirmed |
| C14 | Standalone crates lack paired READMEs | GLM `P16-standards-02` | single same-family source | Info | Info/no defect; lockstep applies when a localized counterpart exists, not to every crate | Refuted as a standards violation |
| C15 | Startup and doctor use different top-level diagnostic envelopes | GLM `P16-spec-02` | single same-family source | Minor | Info; stable execution code/remediation is preserved in startup `details.code` and the convention is tested | Partially confirmed |
| C16 | Inactive rule tables are validated eagerly | GLM `P16-spec-03` | single same-family source | Info | Info; deliberate fail-fast validation permitted by the spec | Confirmed / no defect |
| C17 | Store-level reinstall resets trust and enablement | GLM `P16-spec-04` | single same-family source | Info | Info; CLI idempotence preserves exact unchanged state while store install enforces fresh gates | Confirmed / no defect |
| C18 | `Bounds::validate` omits diagnostic-line realizability | GLM `P16-correctness-01` | single same-family source | Minor | Minor; custom bounds may validate but be unable to encode their declared diagnostic size | Confirmed |
| C19 | `CompletedPayload` permits ambiguous exit/signal state | GLM `P16-correctness-02` | single same-family degraded source | Minor | Minor; both/neither is accepted for normal completion, while neither is valid for timeout/cancel | Partially confirmed |
| C20 | Store activation failure omits adapter identity | GLM `P16-security-01` | single same-family source | Info | Info; intentional redaction because the error carries no validated safe identity | Confirmed / no defect |
| C21 | Windows target sees bootstrap metadata variables | GLM `P16-security-02` | single same-family source | Info | Info; environment confidentiality is an explicit non-goal on the unsupported Windows restriction path | Confirmed / no defect |
| C22 | External execution always inherits the environment | GLM `P16-invariants-01` | single same-family source | Info | Info; deliberate current-local-behavior policy, not a missing Phase 16 option | Confirmed / no defect |
| C23 | Local/protocol shell mapping is duplicated | GLM `P16-integration-01` | single same-family source | Minor | Info; no current divergence and a shared helper would be optional refactoring | Confirmed |
| C24 | Host accepts out-of-range wire exit values | GLM `P16-integration-03` | single same-family source | Info | Info; conforming backend masks correctly, but hostile values should fail protocol validation | Confirmed |
| C25 | Some tests are source-text tripwires | GLM `P16-testquality-01` | single same-family source | Info | Info; structural guards are intentional where no stable behavioral seam exists | Confirmed / no blanket action |
| C26 | Three contribution tests are Unix-only | GLM `P16-testquality-02` | single same-family source | Info | Info; executable-bit coverage is intrinsically Unix and the remaining Windows cases need platform-specific setup | Partially confirmed |
| C27 | Protocol-host subprocess suite is feature-gated | GLM `P16-testquality-03` | single same-family source | Info | Info; CI explicitly runs the documented heavy feature-gated suite | Confirmed / no defect |
| C28 | Drain-grace integration threshold is loose | GLM `P16-testquality-04` | single same-family source | Info | Info; integration deliberately allows scheduling jitter and focused unit coverage pins expiry behavior | Partially confirmed / no action |
| C29 | Model strategy lacks duplicate mismatch cases | GLM `P16-testquality-05` | single same-family source | Info | Info; activation validation is strategy-independent and already covered through production fixed routing | Confirmed / no action |
| C30 | Invocation temp variables override SDK additions | GLM `P16-residuals-01` | single same-family source | Info | Info; correct restriction invariant but underdocumented reserved-key behavior | Confirmed |
| C31 | Executable hashing materializes the whole file | GLM `P16-residuals-02` | single same-family source | Info | Info; bounded only by artifact size and not release-blocking | Confirmed |
| C32 | Local HEAD is ahead of `origin/main` | GLM `P16-residuals-03` | single same-family source | Info | Info; release hygiene based on an unfetched local tracking ref, not a code defect | Confirmed locally only |

## Decision record

| ID | Finding cluster(s) | Decision | Rationale | Decided by |
|---|---|---|---|---|
| D1 | C1 | Put one I/O-neutral capped-line accumulator in `opi-protocol`; keep thin sync/Tokio readers in consumers | The wire owner can prevent future exact-cap/CRLF divergence without coupling I/O runtimes | auto |
| D2 | C2 | Introduce one owned active-protocol lifecycle/accumulator object while touching teardown paths | Preserves behavior and makes child/guard/stdin/stderr/deadline ownership explicit; avoid a larger protocol rewrite | auto |
| D3 | C5 | Map an in-band timed-out terminal outcome to the stable `execution_timed_out` diagnostic before generic bash failure mapping | The normative stable code determines one correction | auto |
| D4 | C6 | Do not edit the frozen snapshot or canonical ledger; disclose the historical gap and route fresh evidence to a new guarded `opi-implement` task | Remediation does not own ledger state and cannot manufacture past evidence | auto |
| D5 | C7 | Use exact raw `package.toml` bytes for resolver locks and Package Trust; CRLF-only byte changes invalidate trust | This follows the normative “exact locked artifact” contract and removes the incompatible raw/canonical hash bases | user (`recommended`, selected 2026-08-10) |
| D6 | C8 | Add a real host-to-`opi-sandbox` composition test before redesigning containment; change cleanup code only for a reproduced target/temp-root failure | The parent-death watchdog refutes the audits' unconditional orphan claim | auto |
| D7 | C9 | Replace synchronous protocol output with ordered cancellable async writes covered by the absolute request deadline | Restores backend-side cancellation without weakening framing or output order | auto |
| D8 | C10 | Redact exact request command, env values, cwd/workspace, and process identifiers at the host boundary; also redact diagnostic messages at the public event boundary | Meets the existing public-redaction criterion without redesigning the wire vocabulary in remediation | auto |
| D9 | C11 | Copy the tested binary into an isolated bin directory and run every smoke command from a separate empty cwd | Directly implements the mandatory standalone acceptance wording | auto |
| D10 | C12 | Required native jobs fail when no proven writable outside candidate exists; artifact audit also rejects the textual skip marker | Eliminates false-green evidence without changing restriction semantics | auto |
| D11 | C18, C19, C24 | Extend protocol semantic validation for diagnostic realizability, completion status shape, and exit range | These are fail-closed substrate checks with one conformant behavior | auto |
| D12 | C30 | Document `TMPDIR`/`TMP`/`TEMP` as invocation-owned reserved keys that override additions | Current behavior is correct; documentation is the minimum correction | auto |
| D13 | C3, C4, C13-C17, C20-C23, C25-C29, C31-C32 | No product change | Refuted, intentional, informational, release-hygiene, or low-value optional refactors do not justify remediation churn | auto |

### Selected manifest-identity decision

The user selected the recommended exact-byte behavior. Resolver locks and
Package Trust will hash raw `package.toml` bytes through one shared helper. A
CRLF-only byte change therefore invalidates trust, matching the current
normative phrase “exact locked artifact.” The prior canonical-LF option is not
part of this remediation plan and no normative spec change is required.

Fix 3.5 is design-unblocked. Phase F remains disabled until the user explicitly
requests execution.

## Remediation layers

### Layer 1: `opi-protocol` (substrate)

**Verification**:

    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/opi-impl-smoke.ps1 scoped --crate opi-protocol --test execution_v1_contract --test execution_v1_schema

#### Fix 1.1: Own capped JSONL framing once

- **Finding source**: Codex audit `P16-CODEX-STD-001`; GLM audit `P16-integration-02`
- **Cluster**: C1
- **Decision**: D1
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-protocol/src/execution/v1/codec.rs` ~L50-L108; public execution-v1 module exports; `crates/opi-protocol/tests/execution_v1_contract.rs`
- **Change**: Extract the CR/LF, pending-CR, EOF, exact-cap, and oversize state machine into an I/O-neutral accumulator owned by `opi-protocol`. Keep existing synchronous codec behavior unchanged.
- **Test plan**: Add exact-cap LF/CRLF, cap-plus-one, lone-CR, EOF-after-CR, and chunk-boundary cases against the accumulator and synchronous codec.

#### Fix 1.2: Validate diagnostic bounds against the frame cap

- **Finding source**: GLM audit `P16-correctness-01`
- **Cluster**: C18
- **Decision**: D11
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-protocol/src/execution/v1/bounds.rs` ~L22-L83; contract/schema tests
- **Change**: Add framing-reserve-aware validation proving `max_diagnostics_size` can fit under `max_line_size`; return a typed bounds error when it cannot.
- **Test plan**: Cover exact fit, one byte over, default bounds, and interaction with the existing chunk/configuration checks.

#### Fix 1.3: Reject ambiguous terminal status and invalid exit range

- **Finding source**: GLM audit `P16-correctness-02` (degraded source axis `correctness`); GLM audit `P16-integration-03`
- **Cluster**: C19, C24
- **Decision**: D11
- **Verification status**: Partially confirmed / Confirmed
- **File(s)**: `crates/opi-protocol/src/execution/v1/frames.rs` ~L254-L270; codec/session validation; fixtures
- **Change**: Reject `exit` plus `signal` together; require one for ordinary completion but allow neither for timed-out/cancelled completion; reject wire exit values above 255 as a protocol violation.
- **Test plan**: Add valid normal/signal/timeout/cancel fixtures and invalid both/neither/out-of-range fixtures shared by host and backend consumers.

### Layer 2A: `opi-agent`

**Verification**:

    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/opi-impl-smoke.ps1 scoped --crate opi-agent --test tool_event_redaction

#### Fix 2A.1: Redact diagnostic messages at the public event boundary

- **Finding source**: Codex audit `P16-CODEX-SEC-001`; GLM audit `P16-security-03`
- **Cluster**: C10
- **Decision**: D8
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/event.rs` ~L152-L174; `crates/opi-agent/tests/tool_event_redaction.rs`
- **Change**: Apply summary redaction to `ToolExecutionEnd` diagnostic messages in addition to the existing context/details redaction. Keep host-boundary exact-value removal as the primary control.
- **Test plan**: Add raw message canaries for command text, env values, path, PID, and a non-pattern secret; assert all public event/session forms omit them.

### Layer 2B: `opi-sandbox`

**Verification**:

    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/opi-impl-smoke.ps1 scoped --crate opi-sandbox --test protocol_conformance --test backend_protocol_smoke --test sdk_contract --test standalone_smoke --test linux_policy --test macos_policy

#### Fix 2B.1: Consume canonical framing and make protocol output deadline-aware

- **Finding source**: Codex audit `P16-CODEX-STD-001`, `P16-CODEX-INV-001`; GLM audit `P16-integration-02`, `P16-correctness-04` (degraded source axis `correctness`)
- **Cluster**: C1, C9
- **Decision**: D1, D7
- **Verification status**: Confirmed / Partially confirmed
- **File(s)**: `crates/opi-sandbox/src/backend.rs` ~L132, ~L478-L573, ~L861-L934; protocol conformance tests
- **Change**: Replace the private capped reader with the shared accumulator. Replace blocking `std::io::Write` frame emission with ordered cancellable async writes whose backpressure is bounded by the absolute request deadline. Do not use an uninterruptible `spawn_blocking` writer.
- **Test plan**: Add a pipe-capacity/non-reading client case, cancellation during blocked output, `started` flush before release, ordered binary chunks, and deadline cleanup; retain all framing edge cases.

#### Fix 2B.2: Prove real nested cleanup composition

- **Finding source**: Codex audit `P16-CODEX-INT-001`; GLM audit `P16-correctness-03`
- **Cluster**: C8
- **Decision**: D6
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-sandbox/src/runner.rs` ~L739, ~L1402; `crates/opi-sandbox/tests/sdk_contract.rs` ~L790; protocol/standalone integration tests
- **Change**: Add a real backend-stdio composition test that hard-kills the owning backend after target start and proves target-group death plus invocation temp-root removal. Do not change process-group architecture unless the test reproduces a residual failure.
- **Test plan**: Run natively on Linux and macOS for hard owner death, host timeout, cancellation, and dropped host future. If temp cleanup fails, add the smallest OS-appropriate cleanup owner and first pin the failing case.

#### Fix 2B.3: Run Windows standalone smoke from isolated locations

- **Finding source**: Codex audit `P16-CODEX-TQ-001`
- **Cluster**: C11
- **Decision**: D9
- **Verification status**: Confirmed
- **File(s)**: `scripts/opi-sandbox-smoke.ps1` ~L16-L68; `crates/opi-sandbox/tests/standalone_smoke.rs` ~L153-L172
- **Change**: Copy the supplied executable into a fresh isolated bin directory, create a distinct empty cwd, run every command from that cwd, retain invalid Opi sentinels, and assert neither location gains Opi or durable sandbox state.
- **Test plan**: Exercise help/version/doctor/run refusal against the isolated copy and assert binary path, cwd, sentinel, and no-state conditions.

#### Fix 2B.4: Make missing native outside-write coverage fail

- **Finding source**: Codex audit `P16-CODEX-TQ-002`
- **Cluster**: C12
- **Decision**: D10
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-sandbox/tests/linux_policy.rs` ~L396-L402; `crates/opi-sandbox/tests/macos_policy.rs` ~L202-L210
- **Change**: Select and prove a writable outside-workspace candidate before the restricted run; fail the required native job when no valid candidate exists instead of printing a skip and returning success.
- **Test plan**: Add candidate-unavailable failure coverage and native positive/negative controls; preserve outside-read and workspace-write assertions.

### Layer 3: `opi-coding-agent` and workspace integration

**Verification**:

    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/opi-impl-smoke.ps1 scoped --crate opi-coding-agent --test execution_contribution_manifest --test package_resolver --test artifact_audit_script --test bash_backend_diagnostics
    cargo test -p opi-coding-agent --features execution-backend-test-fixture --test execution_backend_mock --no-run
    cargo test -p opi-coding-agent --features execution-backend-test-fixture --test execution_protocol_host
    cargo test -p opi-coding-agent --features execution-backend-test-fixture --test execution_product

#### Fix 3.1: Consume the canonical capped-line accumulator

- **Finding source**: Codex audit `P16-CODEX-STD-001`; GLM audit `P16-integration-02`
- **Cluster**: C1
- **Decision**: D1
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/execution/protocol_host.rs` ~L1369-L1447; feature-gated protocol-host tests
- **Change**: Replace the host's private framing state machine with a thin Tokio reader feeding the `opi-protocol` accumulator.
- **Test plan**: Re-run host exact-cap LF/CRLF, cap-plus-one, lone-CR, EOF, malformed, and cumulative-bound cases against the production mock peer.

#### Fix 3.2: Give active protocol teardown one owner

- **Finding source**: Codex audit `P16-CODEX-STD-002`
- **Cluster**: C2
- **Decision**: D2
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/execution/protocol_host.rs` ~L983-L1269
- **Change**: Bundle child/tree guard/stdin/stderr task/deadline and output/diagnostic accumulation into owned lifecycle objects with terminal, cancel, and teardown methods. Preserve current failure precedence and redaction.
- **Test plan**: Retain every terminal/cancel/EOF/overflow/drop teardown case and add ownership-focused tests for exactly-once termination and reap.

#### Fix 3.3: Preserve the stable in-band timeout code

- **Finding source**: Codex audit `P16-CODEX-SPEC-001`
- **Cluster**: C5
- **Decision**: D3
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/execution/runtime.rs` ~L667-L703; `crates/opi-coding-agent/src/tool/bash.rs` ~L458-L487; `crates/opi-coding-agent/tests/execution_product.rs` ~L1145
- **Change**: Carry the stable execution failure code/remediation in `BashOperationContext` for a valid `Completed { timed_out: true }` and use it before generic bash error construction. Do not change the deliberate host-deadline `cleanup_unconfirmed` path.
- **Test plan**: Assert `execution_timed_out` on ToolResult, ToolExecutionEnd, text/TUI, NDJSON, RPC, session, and trace surfaces; retain typed backend-timeout and host-cleanup-unconfirmed cases.

#### Fix 3.4: Remove adapter-controlled secrets at the host boundary

- **Finding source**: Codex audit `P16-CODEX-SEC-001`; GLM audit `P16-security-03`
- **Cluster**: C10
- **Decision**: D8
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/execution/protocol_host.rs` ~L632, ~L823; `crates/opi-coding-agent/src/execution/runtime.rs` ~L703; `crates/opi-coding-agent/src/tool/bash.rs` ~L299-L318; protocol/product tests
- **Change**: Treat backend diagnostic and effective-contract text as untrusted. Before public serialization, remove exact request command, env values, workspace/cwd, backend/target PIDs, and known path values, then apply generic summary redaction. Keep closed/typed effective-contract vocabulary as a future API design rather than changing the wire in remediation.
- **Test plan**: Use arbitrary non-pattern canaries embedded in diagnostics, policy, placement, guarantee, and limitations; assert absence from ToolResult, text/TUI, NDJSON, RPC, session, and trace while safe contract values remain visible.

#### Fix 3.5: Unify manifest identity on exact raw bytes

- **Finding source**: Codex audit `P16-CODEX-SPEC-003`; GLM audit `P16-spec-01`
- **Cluster**: C7
- **Decision**: D5
- **Verification status**: Confirmed / Partially confirmed
- **File(s)**: `crates/opi-coding-agent/src/execution/contribution.rs` ~L93-L94, ~L277, ~L545-L551, ~L629-L643; `crates/opi-coding-agent/src/package_resolver.rs` ~L210-L214, ~L365-L399; manifest/resolver lifecycle tests
- **Change**: Hash raw `package.toml` bytes through one shared helper in every resolver, lock, and Package Trust path. Remove LF normalization from manifest identity and make CRLF-only drift invalidate trust. Optionally stream the executable SHA-256 while touching the hash helper, without changing executable-hash semantics.
- **Test plan**: Pin identical-byte stability, LF/CRLF inequality, durable trust invalidation, resolver/contribution hash equality, re-enable flow, and pre-spawn revalidation.

#### Fix 3.6: Reject textual native skips in artifact evidence

- **Finding source**: Codex audit `P16-CODEX-TQ-002`
- **Cluster**: C12
- **Decision**: D10
- **Verification status**: Confirmed
- **File(s)**: `scripts/opi-artifact-audit.py` ~L530-L632; `crates/opi-coding-agent/tests/artifact_audit_script.rs`
- **Change**: Recognize the legacy textual skip marker as failed required evidence in addition to Cargo ignored/zero-test/failure markers.
- **Test plan**: Add a fixture whose Cargo output otherwise passes but contains the early-return marker; assert a stable failed-evidence issue code.

### Final layer: documentation and evidence handoff

**Verification**:

    python scripts/opi-doc-check.py

#### Fix D.1: Document reserved invocation temp variables and shipped fixes

- **Finding source**: GLM audit `P16-residuals-01`; all implemented Major/Minor clusters above
- **Cluster**: C30 plus implemented remediation clusters
- **Decision**: D12
- **Verification status**: Confirmed
- **File(s)**: affected public rustdoc; `CHANGELOG.md` under `Unreleased`; paired EN/ZH docs only if product wording changes
- **Change**: State that `TMPDIR`, `TMP`, and `TEMP` are invocation-owned reserved keys and override caller additions. Record user-visible timeout/redaction/trust changes under `Unreleased`; synchronize English/Chinese counterparts if D5 changes normative wording.
- **Test plan**: Run warning-free affected-crate rustdoc and `python scripts/opi-doc-check.py`.

## Historical evidence handoff

Cluster C6 is real but cannot be fixed by editing history. The frozen snapshot
has five null task-evidence fields, seven open acceptance scenarios, an
identity-less artifact-audit command, and a phase-exit summary claiming ignored
`target/` evidence that is absent from the commit and current worktree.

The owning workflow must create a new guarded `opi-implement` task that:

1. records current acceptance criteria rather than rewriting the Phase 16
   snapshot;
2. runs current Linux/macOS/Windows, six-target, and gate evidence at one exact
   commit with workflow/run identity;
3. stores a durable digest-bound retrieval record outside ignored `target/`;
4. explicitly cites the historical Phase 16 gap instead of presenting new
   evidence as retroactive proof.

This handoff does not authorize ledger mutation, commit, push, or publication.

## Final verification

After each layer's scoped gate passes, the cross-crate change requires the
workspace tier:

    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/opi-impl-smoke.ps1 full
    cargo test --workspace --doc
    python scripts/opi-doc-check.py

Required native evidence:

1. Linux: real host/backend/target cleanup composition, blocked-output deadline,
   outside-write required assertion, and extracted standalone smoke.
2. macOS: the same cleanup composition, blocked-output deadline, outside-write
   required assertion, and extracted standalone smoke.
3. Windows: isolated-copy/empty-cwd standalone smoke and all affected protocol
   and public-surface tests.
4. CI: ordinary jobs and all six strict target checks green at the exact
   remediation commit before release.

## Scope exclusions

| Cluster / finding | Status | Reason |
|---|---|---|
| C3 | Info/No action | Optional helper extraction is not needed for correctness |
| C4 | Info/No action | Validated candidates prevent a demonstrated identity mix-up; a newtype is broad refactoring |
| C6 | Handoff | Frozen snapshot/ledger cannot be edited by remediation |
| C13 | Info/No action | Duplication preserves intentional crate isolation |
| C14 | Refuted | Repository rules do not require a README pair for every crate |
| C15 | Info/No action | Startup envelope retains stable execution code/remediation in structured details |
| C16 | Info/No action | Eager validation is deliberate fail-fast behavior |
| C17 | Info/No action | Store install resets gates; CLI transaction preserves exact unchanged state |
| C20 | Info/No action | Store detail lacks a safe validated identity and is intentionally redacted |
| C21 | Info/No action | Environment confidentiality is an explicit Windows/non-goal boundary |
| C22 | Info/No action | Environment inheritance is the specified current-behavior policy |
| C23 | Info/No action | Shell mapping agrees; shared helper is optional refactoring |
| C25 | Info/No blanket action | Structural guards remain appropriate where no stable behavioral seam exists |
| C26 | Info/No action | Unix-only mechanics need platform-specific tests, not copied assertions |
| C27 | Info/No action | CI explicitly runs the feature-gated protocol-host suite |
| C28 | Info/No action | Unit coverage pins behavior; integration threshold permits scheduler jitter |
| C29 | Info/No action | Activation validation is strategy-independent |
| C31 | Info/No action | Streaming SHA-256 is optional unless combined with D5 implementation |
| C32 | Release handoff | Fetch/push/CI require explicit user authorization and are not code remediation |

## Test impact

Current plan-only change: `none` for Rust tests and runtime behavior.

If executed: `add` protocol bound/terminal fixtures, blocked-output and nested
cleanup integration tests, public redaction canaries, isolated Windows smoke,
and artifact-skip fixtures; `update` existing timeout, manifest identity, and
native policy tests. No test deletion is planned.
