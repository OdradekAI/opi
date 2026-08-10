# Phase 16 Pluggable Extensions and Command Execution — Independent Code Audit

## Audit metadata

- Auditor: `codex`
- Date: 2026-08-10
- Audit head: `c5de89216b316529d1c8c1c182fe496a3103f42f`
- Phase-exit implementation commit: `f8aff0237221fbf7d56b58abb5dce02833344bfc` (task 16.16.3's recorded verification commit)
- Scope: tasks 16.1 through 16.16.3, including their DoDs, evidence claims, acceptance scenarios, phase-exit criteria, registered Phase 16 design, and current implementation at the pinned head
- Normative sources: `docs/opi-spec.md`, `docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md`, `.opi-impl-state.json`, and `docs/snapshots/phase16/opi-impl-state.json`
- History use: commit history and the phase-exit commit were used only for provenance and discovery. History and diffs were not used as the coverage boundary.
- Prior-report isolation: the root auditor did not consult any prior `audit.*.md`. An initial Standards worker accidentally surfaced lines from a prohibited report through a broad grep; all of that worker's evidence was quarantined and discarded. A replacement clean-room Standards audit used explicit committed-object paths and attested that no prior report was read or surfaced.
- Post-lock cleanup event: after all findings and normalized blocks had been written, an untracked collaborator artifact named `.audit_survivors.json` appeared. It was opened only to identify cleanup ownership, was not an `audit_head` object, did not alter any finding, and was removed together with generated Python cache files.
- Independence: implementation-author model provenance is not recorded, so normalized findings use `independence: unknown`.

## Executive summary

**Verdict: FAIL — 5 Major findings and 7 Minor findings.**

The current implementation passes the complete Windows workspace test suite, focused feature-gated execution acceptance, scoped clippy, formatting, and documentation contracts. Protocol ordering, bounds, package lifecycle, routing, no-fallback behavior, archive structure, and the Windows unsupported posture are generally strong.

Phase 16 nevertheless cannot be accepted at the pinned head. Two runtime-liveness defects break mandatory cleanup guarantees: the outer Unix host process group does not contain the real `opi-sandbox` target process group, and synchronous backend stdout writes can block the async execution loop past every deadline. Public-surface redaction also trusts adapter-controlled free text, and normal in-band timeouts lose the promised stable `execution_timed_out` code. Finally, the committed phase-exit record contradicts its own open/null acceptance state and names ignored evidence that is absent and no longer replayable with the recorded command.

| Severity | Count |
|---|---:|
| Blocker | 0 |
| Major | 5 |
| Minor | 7 |
| Info | 0 |

## Per-task assessment

| Task | Assessment | Finding impact |
|---|---|---|
| 16.1 | PASS | Documentation contract is present and `opi-doc-check` passes. |
| 16.2 | FAIL | P16-CODEX-INT-001 violates L0 descendant cleanup. |
| 16.3 | PARTIAL | P16-CODEX-STD-001 leaves the wire framing state machine triplicated. |
| 16.4 | PARTIAL | P16-CODEX-STD-004 and P16-CODEX-SPEC-003 weaken identity exactness. |
| 16.5 | PARTIAL | P16-CODEX-STD-004 and P16-CODEX-SPEC-003 affect lifecycle identity. |
| 16.6 | FAIL | P16-CODEX-SPEC-001 loses the stable timeout code. |
| 16.7 | FAIL | P16-CODEX-INT-001 and P16-CODEX-SEC-001 affect teardown and redaction; two Standards findings add maintenance risk. |
| 16.8 | FAIL | P16-CODEX-INT-001, P16-CODEX-SPEC-001, and P16-CODEX-SEC-001 cross the runtime seam. |
| 16.9 | FAIL | Stable-code/redaction failures plus contradictory null evidence; P16-CODEX-STD-003 duplicates finalization. |
| 16.10 | PARTIAL | Behavior passes, but P16-CODEX-STD-004 leaves validated adapter identity convention-based. |
| 16.11.1 | FAIL | P16-CODEX-INT-001 and P16-CODEX-INV-001 break drop/deadline cleanup in composition. |
| 16.11.2 | FAIL | Backend liveness fails; P16-CODEX-TQ-001 leaves Windows standalone isolation unproved. |
| 16.12 | FAIL | P16-CODEX-INT-001 and P16-CODEX-INV-001 break the backend/host contract. |
| 16.13 | FAIL | Outer cleanup and phase evidence fail; native skip reporting is incomplete. |
| 16.14.1 | FAIL | Outer cleanup and phase evidence fail; native skip reporting is incomplete. |
| 16.14.2 | FAIL | Runtime posture passes, but phase evidence and Windows isolation acceptance do not. |
| 16.15.1 | PARTIAL | Packaging tests pass; manifest byte identity remains canonicalized rather than exact. |
| 16.15.2 | FAIL | P16-CODEX-SPEC-002 and P16-CODEX-TQ-002 invalidate the claimed evidence closure. |
| 16.16.1 | FAIL | Crate boundaries pass, but the task is marked passing with null evidence. |
| 16.16.2 | FAIL | Install-to-execute works, but cleanup, stable-code, and redaction findings reach public surfaces. |
| 16.16.3 | FAIL | Repository gates pass locally, but runtime invariants and the phase-exit evidence claim do not. |

## Standards axis

The clean-room Standards review found no hard repository-rule violation. It found four Fowler maintainability smells.

### P16-CODEX-STD-001 — Minor: Capped JSONL framing is implemented three times

**Cause:** `opi-protocol`, the Opi protocol host, and the sandbox backend independently implement the same CR/LF, EOF, exact-cap, and oversize state machine.

**Impact:** A boundary or framing fix must be reproduced in three implementations, increasing the probability of host/backend divergence.

**Fix:** Extract an I/O-neutral capped-line accumulator into `opi-protocol`; retain thin synchronous and Tokio I/O adapters in consumers.

```yaml
id: P16-CODEX-STD-001
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: standards
severity: Minor
title: Capped JSONL framing is implemented three times
claim: The same bounded JSONL byte-framing state machine has three independent production owners.
evidence:
  - location: crates/opi-protocol/src/execution/v1/codec.rs:50
    detail: The protocol crate implements capped CR/LF, EOF, and oversize handling through line 108.
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:1369
    detail: The host independently implements the same state transitions through line 1447.
  - location: crates/opi-sandbox/src/backend.rs:875
    detail: The backend carries a third implementation through line 934.
criterion_source: Fowler Duplicated Code; opi-protocol is the semantic owner of command-execution-jsonl-v1 framing.
reproduction:
  - "git show c5de89216b316529d1c8c1c182fe496a3103f42f:crates/opi-protocol/src/execution/v1/codec.rs"
  - "git show c5de89216b316529d1c8c1c182fe496a3103f42f:crates/opi-coding-agent/src/execution/protocol_host.rs"
  - "git show c5de89216b316529d1c8c1c182fe496a3103f42f:crates/opi-sandbox/src/backend.rs"
confidence: high
status: unverified
```

### P16-CODEX-STD-002 — Minor: Protocol teardown state is a positional data clump

**Cause:** Ownership-sensitive child, tree guard, stderr task, stdin, deadline, reports, output, and diagnostics are repeatedly threaded through large positional signatures.

**Impact:** Cleanup invariants are difficult to evolve safely, and the module suppresses `clippy::too_many_arguments` at its most sensitive lifecycle seam.

**Fix:** Introduce an owned active-backend lifecycle object and an execution accumulator with terminal, cancellation, and teardown methods.

```yaml
id: P16-CODEX-STD-002
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: standards
severity: Minor
title: Protocol teardown state is a positional data clump
claim: The host repeatedly passes the same ownership-sensitive lifecycle state through 12- and 16-parameter functions.
evidence:
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:983
    detail: finalize_terminal accepts 12 parameters through line 997.
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:1105
    detail: finish_with_cancel accepts 16 parameters through line 1123.
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:1185
    detail: The same process and accumulator group is threaded into teardown calls through line 1269.
criterion_source: Fowler Data Clumps; AGENTS.md requires the simplest design that preserves behavior.
reproduction:
  - "git show c5de89216b316529d1c8c1c182fe496a3103f42f:crates/opi-coding-agent/src/execution/protocol_host.rs"
confidence: high
status: unverified
```

### P16-CODEX-STD-003 — Minor: Headless and interactive config finalizers duplicate behavior

**Cause:** Both finalizers finalize project trust, apply the same execution overrides, validate, and return the result.

**Impact:** Future precedence or validation changes require synchronized edits in two startup paths.

**Fix:** Keep mode-specific orchestration, but delegate the common tail to one `finalize_trusted_config` helper.

```yaml
id: P16-CODEX-STD-003
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: standards
severity: Minor
title: Headless and interactive config finalizers duplicate behavior
claim: Two startup functions have the same trust finalization, execution override, and validation implementation.
evidence:
  - location: crates/opi-coding-agent/src/main.rs:356
    detail: resolve_headless_trust_config_finalization performs the common tail through line 370.
  - location: crates/opi-coding-agent/src/main.rs:479
    detail: resolve_interactive_trust_config_core repeats the same tail through line 491.
criterion_source: Fowler Duplicated Code.
reproduction:
  - "git show c5de89216b316529d1c8c1c182fe496a3103f42f:crates/opi-coding-agent/src/main.rs"
confidence: high
status: unverified
```

### P16-CODEX-STD-004 — Minor: Validated adapter identity remains primitive strings

**Cause:** Contribution validation, config, routing, permission policy, and runtime eligibility use `String`/`&str` for both validated adapter identities and raw selection text.

**Impact:** Reserved-ID semantics and the boundary between untrusted model text and validated identities depend on convention at every seam.

**Fix:** Introduce a validated `AdapterId` newtype and retain raw user/model input as a separate type until validation succeeds.

```yaml
id: P16-CODEX-STD-004
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: standards
severity: Minor
title: Validated adapter identity remains primitive strings
claim: Adapter identity validation and reserved-ID semantics are not preserved by the type system across execution boundaries.
evidence:
  - location: crates/opi-coding-agent/src/execution/contribution.rs:92
    detail: Validated contribution and lock structures retain adapter ids as String through line 128.
  - location: crates/opi-coding-agent/src/execution/router.rs:43
    detail: Eligibility and selection retain ids as String through line 79.
  - location: crates/opi-coding-agent/src/execution/permission.rs:38
    detail: Reserved local semantics are applied to raw &str values through line 75.
  - location: crates/opi-coding-agent/src/execution/runtime.rs:209
    detail: Enabled identities remain string-keyed through line 234.
criterion_source: Fowler Primitive Obsession; Phase 16 executable identity is a validated security/lifecycle boundary.
reproduction:
  - "git show c5de89216b316529d1c8c1c182fe496a3103f42f:crates/opi-coding-agent/src/execution/contribution.rs"
  - "git show c5de89216b316529d1c8c1c182fe496a3103f42f:crates/opi-coding-agent/src/execution/router.rs"
confidence: medium
status: unverified
```

## Spec axis

The Spec review found two material conformance failures and one minor exactness mismatch.

### P16-CODEX-SPEC-001 — Major: In-band timeout loses the stable `execution_timed_out` code

**Cause:** `Completed { timed_out: true }` is mapped into `BashOperationContext`; the generic bash failure builder then assigns `opi.tool.execution_failed` to every operation error.

**Impact:** Text/TUI/NDJSON/RPC do not preserve the Phase 16 stable failure code for the normal backend timeout path. Consumers cannot reliably match the documented timeout code.

**Fix:** Map the in-band timeout outcome to an `execution_timed_out` diagnostic before generic bash failure mapping, and assert that code on ToolResult, NDJSON, RPC, and trace surfaces.

```yaml
id: P16-CODEX-SPEC-001
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: spec
severity: Major
title: In-band timeout loses the stable execution_timed_out code
claim: A valid Completed frame with timed_out true surfaces opi.tool.execution_failed instead of execution_timed_out.
evidence:
  - location: crates/opi-coding-agent/src/execution/runtime.rs:681
    detail: completed_outcome_to_bash_result stores timed_out only in BashOperationContext.
  - location: crates/opi-coding-agent/src/tool/bash.rs:277
    detail: The result becomes an error based on the context flag.
  - location: crates/opi-coding-agent/src/tool/bash.rs:487
    detail: bash_operation_diagnostic unconditionally assigns CODE_TOOL_EXECUTION_FAILED.
  - location: crates/opi-coding-agent/tests/execution_product.rs:1145
    detail: The timeout acceptance test checks is_error and timed_out context but not the stable code.
criterion_source: docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md:740-768; task 16.6 DoD and SC16-14.
reproduction:
  - Add an assertion for diagnostic code execution_timed_out to timed_out_in_band_completed_is_not_a_success and run the exact test with execution-backend-test-fixture enabled.
confidence: high
status: unverified
```

### P16-CODEX-SPEC-002 — Major: Phase-exit acceptance is internally contradictory and not replayable

**Cause:** The committed snapshot declares Phase 16 complete while five passing tasks have `evidence: null`, seven task-owned scenarios remain `open`, and the claimed evidence bundle is under ignored `/target` and absent from the commit. The recorded audit command also omits identity arguments now required by the current audit script.

**Impact:** Current-head Linux/macOS native behavior, six-target checks, and per-category gates cannot be independently established from the authoritative Phase 16 record. The exact recorded acceptance command fails on a clean checkout.

**Fix:** Use the guarded `opi-implement` reconciliation flow to close or correct every task scenario and evidence field. Regenerate evidence against the current implementation, publish it durably with content hashes/run/commit identity, and record a replayable command including `--workflow-run-id` and `--commit-sha`.

```yaml
id: P16-CODEX-SPEC-002
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: spec
severity: Major
title: Phase-exit acceptance is internally contradictory and not replayable
claim: The committed Phase 16 state asserts complete evidence while its task state and clean-checkout audit command prove otherwise.
evidence:
  - location: docs/snapshots/phase16/opi-impl-state.json:3193
    detail: exit_criteria_met is true and the evaluator claims all 21 tasks clean and evidence preserved.
  - location: docs/snapshots/phase16/opi-impl-state.json:1469
    detail: Tasks 16.9, 16.14.1, 16.15.2, 16.16.1, and 16.16.3 have null evidence at lines 1469, 2323, 2646, 2809, and 3183.
  - location: docs/snapshots/phase16/opi-impl-state.json:2135
    detail: Seven acceptance scenarios remain open at lines 2135, 2151, 2272, 2288, 2409, 2618, and 2754.
  - location: .gitignore:1
    detail: /target is ignored; the named target/opi-artifacts/phase16-phase-exit bundle has no committed object.
  - location: docs/snapshots/phase16/opi-impl-state.json:3112
    detail: The recorded phase-exit command omits explicit workflow and commit identity arguments.
  - location: scripts/opi-artifact-audit.py:2016
    detail: Current phase-exit mode rejects invocations without --workflow-run-id and --commit-sha.
criterion_source: Phase 16 task DoDs/evidence/acceptance scenarios and .claude/skills/opi-implement/skill.md:200-207,236-242.
reproduction:
  - "git ls-tree -r c5de89216b316529d1c8c1c182fe496a3103f42f -- target/opi-artifacts/phase16-phase-exit"
  - "git check-ignore -v target/opi-artifacts/phase16-phase-exit"
  - "python scripts/opi-artifact-audit.py target/opi-artifacts/phase16-phase-exit --workspace-root . --phase-exit --json"
confidence: high
status: unverified
```

### P16-CODEX-SPEC-003 — Minor: Manifest trust hashes canonical line endings, not exact bytes

**Cause:** Manifest hash calculation converts CRLF to LF before hashing.

**Impact:** A line-ending-only byte change does not stale Package Trust, despite the design describing trust as matching the exact locked artifact. The normalization is intentional for checkout portability, so the code or the exact-artifact language must be made explicit.

**Fix:** Either hash exact distribution bytes and make packaging deterministic, or define the locked manifest artifact as a canonical LF-normalized representation in the spec and lock schema.

```yaml
id: P16-CODEX-SPEC-003
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: spec
severity: Minor
title: Manifest trust hashes canonical line endings rather than exact bytes
claim: Changing only a trusted manifest's LF and CRLF representation does not invalidate its stored manifest hash.
evidence:
  - location: crates/opi-coding-agent/src/execution/contribution.rs:277
    detail: manifest_hash is computed from lf_normalize(raw_manifest_bytes).
  - location: crates/opi-coding-agent/src/execution/contribution.rs:631
    detail: lf_normalize replaces every CRLF sequence with LF before hashing.
  - location: crates/opi-coding-agent/tests/execution_contribution_manifest.rs:186
    detail: The behavior is intentionally pinned by manifest_hash_is_crlf_stable.
criterion_source: docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md:53; task 16.4 exact lock-material DoD.
reproduction:
  - Enable a package, change only package.toml line endings, and rerun package activation diagnostics; trust remains current.
confidence: medium
status: unverified
```

## Integration, invariants, security, and test quality

### P16-CODEX-INT-001 — Major: Unix host teardown misses the real sandbox target process group

**Cause:** The protocol host creates a process group for the adapter backend, while `opi-sandbox` creates a second process group for the target. The host's Unix guard kills only the backend PGID. A SIGKILLed/crashed backend cannot run the sandbox guard's destructor.

**Impact:** Host-future drop, hard timeout, backend crash, or forced teardown can leave the command and its invocation temp root alive on Linux/macOS while the host reports the backend group reaped. The surviving command can continue consuming resources and mutating permitted user workspace data after cancellation.

**Fix:** Establish one compositional containment owner that includes backend and target descendants across group boundaries, or make adapter death fail-safe for the complete target tree. Add a real `opi-sandbox backend --stdio` integration test that creates the production nested PGID, drops/kills the host, and proves target and temp-root removal.

```yaml
id: P16-CODEX-INT-001
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: integration
severity: Major
title: Unix host teardown misses the real sandbox target process group
claim: Killing the backend process group does not kill the opi-sandbox target because the target is moved into a different Unix process group.
evidence:
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:223
    detail: The host applies configure_tree before spawning the backend and attaches a guard to its PID through line 235.
  - location: crates/opi-coding-agent/src/tool/process_tree.rs:43
    detail: Unix containment is process_group(0), and termination is kill(-pgid, SIGKILL) at lines 375-382.
  - location: crates/opi-sandbox/src/runner.rs:794
    detail: The sandbox independently applies configure_tree to the real target before spawning it at lines 847-872.
  - location: crates/opi-coding-agent/tests/fixtures/execution_backend_mock.rs:1336
    detail: The host L0 fixture spawns a grandchild without creating the production second process group, so the test cannot detect the escape.
  - location: WSL process-group probe
    detail: Backend PGID 456 and target PGID 490 were distinct; killpg(456) left target 490 alive, after which the probe explicitly killed PGID 490.
criterion_source: docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md:171-180,850-870; phase criterion C7.
reproduction:
  - On Unix, start a backend in a new PGID, start its target in another new PGID, kill the backend PGID, and verify kill(target_pid, 0) still succeeds.
confidence: high
status: unverified
```

### P16-CODEX-INV-001 — Major: Blocking backend stdout defeats all request deadlines

**Cause:** The async backend loop writes every frame through synchronous `std::io::Write::write_all` and `flush`. If the client stops reading, the task blocks inside the write and stops polling the execution stream, cancellation, and deadlines.

**Impact:** A chatty target can keep the backend, target tree, and temp root alive indefinitely after the execution and hard request deadlines. This also weakens the started-before-release gate if the `started` flush blocks.

**Fix:** Use bounded asynchronous stdout I/O or a separately supervised bounded writer whose backpressure is included in the absolute request deadline. On expiry, cancel and clean the target independently of writer progress. Add a non-reading-host pipe-capacity test.

```yaml
id: P16-CODEX-INV-001
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: invariants
severity: Major
title: Blocking backend stdout defeats all request deadlines
claim: A protocol client that stops reading stdout can block the backend event loop so execution timeout, cancellation, tree cleanup, and temp cleanup are never polled.
evidence:
  - location: crates/opi-sandbox/src/backend.rs:132
    detail: drive accepts a synchronous &mut dyn Write while the surrounding state machine is async.
  - location: crates/opi-sandbox/src/backend.rs:539
    detail: Output frames are emitted synchronously from inside the tokio select loop through line 553.
  - location: crates/opi-sandbox/src/backend.rs:861
    detail: write_all_nl_flush performs blocking write_all and flush with no deadline through line 869.
  - location: crates/opi-sandbox/tests/protocol_conformance.rs:204
    detail: Tests use an unbounded Vec sink or finite delayed writer, not a pipe-capacity/non-reading client.
criterion_source: docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md:516-529; SC16-06b.
reproduction:
  - Pipe backend --stdio stdout without reading it, execute a short-deadline target that emits more than pipe capacity, keep stdin open, and observe the backend alive after the deadline.
confidence: high
status: unverified
```

### P16-CODEX-SEC-001 — Major: Adapter-controlled protocol text bypasses public redaction

**Cause:** Diagnostic messages are redacted as unkeyed strings, so command/environment structural redaction cannot recognize plain command text. `started` placement/guarantee/policy/limitations are only checked for non-empty core fields and then copied verbatim into ToolResult details.

**Impact:** An adapter can place command text, environment values, credentials not recognized by generic patterns, PIDs, or paths into public ToolResult, NDJSON, RPC, and session events, contrary to SC16-14.

**Fix:** Treat adapter free text as untrusted at the host boundary. Replace public diagnostic text with closed codes/host-owned summaries, and make effective-contract fields closed typed tokens or apply an explicit allowlist before public serialization. Add canary tests across ToolResult, NDJSON, RPC, and trace.

```yaml
id: P16-CODEX-SEC-001
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: security
severity: Major
title: Adapter-controlled protocol text bypasses public redaction
claim: Free-form backend diagnostic and started-contract strings can reach public surfaces with command, environment, credential, path, or PID content intact.
evidence:
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:632
    detail: redact_backend_diagnostic passes the whole free-form message to generic redact_text.
  - location: crates/opi-agent/src/diagnostic.rs:533
    detail: redact_text wraps the text as an unkeyed JSON string, so key-based command/env redaction cannot apply.
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:823
    detail: valid_started_contract checks only non-empty placement, guarantee, and policy.
  - location: crates/opi-coding-agent/src/tool/bash.rs:299
    detail: copy_effective_contract writes adapter strings directly into public details through line 318.
  - location: crates/opi-coding-agent/tests/execution_product.rs:745
    detail: NDJSON/RPC propagation is asserted only with safe fixture literals through line 787.
criterion_source: docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md:763-771; SC16-14.
reproduction:
  - Emit a plain command canary in a backend Diagnostic and assert it is absent from ToolResult diagnostics; the assertion fails.
  - Emit a credential/path canary in started.limitations and inspect ToolResult, NDJSON, and RPC details; the canary is preserved.
confidence: high
status: unverified
```

### P16-CODEX-TQ-001 — Minor: Windows standalone smoke does not use an isolated copy and empty CWD

**Cause:** The PowerShell smoke invokes the workspace-built binary by absolute path but never copies it to isolation or changes the working directory. The Rust test inherits the repository CWD.

**Impact:** The smoke can pass while project-local Opi state is present, so it does not prove the required standalone empty-CWD condition. Source-level crate-boundary tests reduce the runtime risk but do not satisfy this acceptance case.

**Fix:** Copy the binary into a fresh isolated directory, create a separate empty working directory, invoke every smoke command with that CWD, and assert neither location acquires Opi state.

```yaml
id: P16-CODEX-TQ-001
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: test-quality
severity: Minor
title: Windows standalone smoke does not use an isolated copy and empty CWD
claim: The Windows smoke can pass while the tested binary runs from the workspace and inherits the repository working directory.
evidence:
  - location: scripts/opi-sandbox-smoke.ps1:16
    detail: The script validates the supplied binary and scrubs environment values but never copies it or changes CWD through line 68.
  - location: crates/opi-sandbox/tests/standalone_smoke.rs:153
    detail: The test passes CARGO_BIN_EXE_opi-sandbox without setting current_dir through line 172.
criterion_source: docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md:631-642.
reproduction:
  - Run standalone_smoke_script_windows from the repository root with a project-local sentinel; the smoke still reports success because only the environment sentinel is inspected.
confidence: high
status: unverified
```

### P16-CODEX-TQ-002 — Minor: Native early-return skips are recorded as passing evidence

**Cause:** Required outside-write tests print a skip message and return early instead of using an ignored/failed outcome. The artifact classifier recognizes Cargo ignored counts but not these textual skips.

**Impact:** A platform bundle can satisfy the pass-marker rule without executing the required outside-write assertion. Separate native smoke coverage lowers runtime risk, but the claimed skip-rejection evidence rule is false-green capable.

**Fix:** Make absence of a valid outside directory fail the required native job, or emit a machine-readable skip marker that the artifact auditor rejects. Add an artifact-audit fixture for the early-return marker.

```yaml
id: P16-CODEX-TQ-002
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: test-quality
severity: Minor
title: Native early-return skips are recorded as passing evidence
claim: Required Linux/macOS outside-write tests can skip their assertion while Cargo and the artifact auditor classify the run as passing.
evidence:
  - location: crates/opi-sandbox/tests/linux_policy.rs:396
    detail: outside_write_denied prints a skip message and returns when no candidate exists.
  - location: crates/opi-sandbox/tests/macos_policy.rs:202
    detail: The macOS equivalent uses the same early-return pattern through line 210.
  - location: scripts/opi-artifact-audit.py:531
    detail: Skip detection matches nonzero Cargo ignored counts, while pass detection accepts ordinary passed counts through line 684.
criterion_source: Phase criterion C16 and task 16.15.2 require skipped native evidence to be rejected.
reproduction:
  - Run either native policy test on a host with no accepted outside directory; Cargo reports the test passed with zero ignored and the artifact classifier accepts its pass marker.
confidence: high
status: unverified
```

## Invariant verification matrix

| Invariant | Code evidence | Test coverage / assessment |
|---|---|---|
| Minimal Runtime starts no extension machinery | `ExecutionRuntime::build` directly constructs local operations when no enabled identities exist | PASS: minimal-runtime tests and full workspace suite |
| Installed, Trusted, Enabled, Selected, Permitted remain independent | Package lock/activation/router/permission layers remain separate | PASS: lifecycle and selected-routing suites |
| Ready precedes command disclosure | Host emits `Execute` only after validating `Ready` identity | PASS: protocol host negative tests |
| Frame order, IDs, duplicates, and bounds are closed | `opi-protocol::Session`, bounded codec, host accumulator | PASS: protocol contract/schema suites |
| Started is flushed before target release | Backend emits/flushed `Started`, rechecks cutoff, then releases | CONDITIONAL: correct only while stdout makes progress; P16-CODEX-INV-001 fails bounded liveness |
| Timeout/cancel/drop kill child and descendants | Host and sandbox each own process-group guards | FAIL on Unix composition: P16-CODEX-INT-001 |
| Deadline covers startup through cleanup | One absolute request deadline and cleanup reserve exist | FAIL under blocked stdout: P16-CODEX-INV-001 |
| Selected external failure never falls back to local | Routed operations return the selected failure | PASS: execution product/routing suites |
| No degraded success state | timeout/cancel/signal/nonzero set `is_error` | PASS for boolean outcome; stable timeout code FAILS under P16-CODEX-SPEC-001 |
| Public surfaces omit command/env/credentials/paths/PIDs/raw stderr | Backend process stderr is hidden; generic redaction is applied to diagnostics | FAIL for adapter free text and raw contract fields: P16-CODEX-SEC-001 |
| Linux/macOS native restriction fails closed | Landlock/seccomp and Seatbelt setup precede release | Source and focused tests are strong; current-head native evidence is not replayable, and outer cleanup fails |
| Windows reports unsupported confinement honestly | Doctor unsupported, run refuses, local reports supervised | PASS on Windows; empty-CWD standalone acceptance is incomplete |
| Archive layout, hash, target, and evidence parser are bounded | Packaging and artifact-audit scripts validate exact members and limits | PASS structurally; phase-exit evidence closure FAILS |
| Phase exit is backed by closed task evidence | Archived ledger and named bundle | FAIL: P16-CODEX-SPEC-002 |

## Verification performed

- `cargo fmt --check --all` — PASS
- `cargo clippy -p opi-protocol -p opi-sandbox -p opi-coding-agent --all-targets --features execution-backend-test-fixture -- -D warnings` — PASS
- `cargo test --workspace --all-targets --quiet` — PASS on Windows
- `cargo test -p opi-protocol --all-targets` — PASS (35 unit, 32 contract, 14 schema tests)
- `cargo test -p opi-sandbox --all-targets` — PASS on Windows; native Linux/macOS policy tests are target-gated
- Feature-gated execution acceptance (`execution_product`, `execution_protocol_host`, `execution_runtime`) — PASS (22, 53, and 16 tests)
- Focused lifecycle/routing/release/audit/Windows-posture suites — PASS
- `python scripts/opi-doc-check.py` — PASS
- Recorded Phase 16 artifact command — FAIL as expected by P16-CODEX-SPEC-002 with missing identity, platform, six-target, and gate evidence
- Independent WSL nested-process-group probe — reproduced P16-CODEX-INT-001; the surviving target PGID was explicitly killed by the probe

The first full-workspace attempt hit the external 240-second command timeout after compilation and was interrupted, not failed by an assertion. A second warm-cache run with a longer bound completed successfully.

## Residuals and recommendations

1. Fix and test the Unix containment composition before relying on external adapter cancellation or cleanup claims.
2. Make backend output writing deadline-aware and independently supervise cleanup before expanding the protocol surface.
3. Close the adapter-text redaction boundary and stable timeout code across ToolResult, text/TUI, NDJSON, RPC, and trace.
4. Reconcile the canonical ledger only through the guarded workflow, then regenerate current-head Linux/macOS, Windows-posture, six-target, and gate evidence with durable identity.
5. Run native Linux and macOS policy and real backend-host teardown tests after remediation; this Windows audit could not execute those Rust target-gated suites.

Test impact: `none` — this audit changes only the report. No implementation, test, ledger, or documentation contract was modified.
