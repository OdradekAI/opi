# Phase 16 Independent Code Audit

- Audit head: `5c8d2ba561392bc054625a50c1ac8d72e020e8d9`
- Audit date: 2026-08-10
- Scope: Phase 16 tasks `16.1` through `16.16.3`, criteria `C1` through `C16`
- Authoritative sources: `docs/opi-spec.md`, `docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md`, `docs/snapshots/phase16/opi-impl-state.json`, `AGENTS.md`
- Method: independent Standards and Spec reviews, followed by correctness, security, test-quality, invariant, integration, and residual-risk review
- Checkout discipline: all reads and commands ran in a clean isolated checkout at the pinned head; existing Phase 16 audit reports were not read or searched

## Verdict

**FAIL**. No Blocker was found, but seven Major findings affect protocol interoperability, process-tree cleanup, diagnostic observability, output integrity, and phase-exit reproducibility. The implementation has strong lifecycle, routing, redaction, packaging, and platform-posture foundations, and the current-host test suite is green, but the Major findings are systemic enough that Phase 16 should not retain an unqualified passing verdict.

Finding count: **7 Major, 8 Minor**.

## Standards review

### P16-STD-001 — Major — The host's duplicate line reader rejects a valid exact-cap CRLF frame

The shared synchronous codec treats the CR in CRLF as delimiter material and accepts `max_line_size` data bytes followed by CRLF. The host's private async implementation first stores the CR as data, observes that the cap is already reached, and rejects before reading LF. This is both a protocol defect and concrete semantic drift caused by duplicated framing logic.

```yaml
id: P16-STD-001
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: standards
severity: Major
title: Host async line reader rejects exact-cap CRLF frames
claim: The Opi protocol host rejects a valid line with exactly max_line_size data bytes followed by CRLF, while the canonical opi-protocol codec accepts it.
evidence:
  - location: crates/opi-protocol/src/execution/v1/codec.rs:64
    detail: LineReader keeps a pending CR outside the data-size cap and treats CRLF as one delimiter.
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:1366
    detail: CappedReader appends CR to the capped data buffer and checks line.len() >= cap before it can observe LF.
  - location: crates/opi-protocol/tests/execution_v1_contract.rs:445
    detail: The canonical contract explicitly accepts both LF and CRLF at the same exact data cap.
criterion_source: docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md:464-467,842-856; Fowler Duplicated Code
reproduction:
  - Feed [b'x'; cap] followed by b"\\r\\n" to protocol_host::CappedReader with cap bytes.
  - Observe ReadErr::Oversized; the canonical LineReader returns the cap-byte line.
confidence: high
status: unverified
```

### P16-STD-002 — Minor — Rust tests pin narrative and phase wording

`cli_native_and_docs.rs` asserts exact help prose, source comments, phase placeholders, and the absence of historical phrases. This conflicts with the repository rule that narrative wording, phase status, and roadmap placeholders belong in `scripts/opi-doc-check.py`, not Rust tests.

```yaml
id: P16-STD-002
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: standards
severity: Minor
title: Phase prose is pinned in Rust tests
claim: Phase 16 Rust tests fail on harmless narrative rewording instead of checking source-derived contracts through the documentation checker.
evidence:
  - location: crates/opi-sandbox/tests/cli_native_and_docs.rs:230
    detail: The help test requires one exact full descriptive sentence.
  - location: crates/opi-sandbox/tests/cli_native_and_docs.rs:247
    detail: Source-scanning tests assert exact comments and obsolete phase phrases such as lands in 16.13 and empty in 16.11.2.
criterion_source: AGENTS.md:406-409
reproduction:
  - Reword the pinned help or source comment without changing behavior.
  - Run cargo test -p opi-sandbox --test cli_native_and_docs and observe the narrative guard fail.
confidence: high
status: unverified
```

### P16-STD-003 — Minor — Phase 16 library errors hand-roll standard traits

Two Phase 16 error types manually implement `Display` and `Error` even though neighboring errors use the workspace `thiserror` dependency.

```yaml
id: P16-STD-003
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: standards
severity: Minor
title: Library error types diverge from repository thiserror style
claim: ExecutionProtocolFailure and AttachError manually implement Display and Error despite the module and repository convention to derive thiserror::Error.
evidence:
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:157
    detail: ExecutionProtocolFailure has manual Display and std::error::Error implementations.
  - location: crates/opi-coding-agent/src/tool/process_tree.rs:111
    detail: AttachError likewise hand-rolls both traits.
  - location: crates/opi-coding-agent/src/execution/failure.rs:40
    detail: Neighboring execution errors use thiserror::Error.
criterion_source: AGENTS.md:121-125
reproduction:
  - Run rg -n 'impl std::error::Error for|derive.*thiserror::Error' crates/opi-coding-agent/src/execution crates/opi-coding-agent/src/tool/process_tree.rs.
confidence: high
status: unverified
```

### P16-STD-004 — Minor — Activation documentation contradicts its type

The activation type is documented as metadata-only and as carrying no validated-bytes handle, but it owns validated contributions containing the executable `Arc<File>` later used at launch.

```yaml
id: P16-STD-004
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: standards
severity: Minor
title: ActivatedContribution documentation denies its launch handle
claim: ActivatedContribution is documented as carrying no validated-bytes handle even though its validated field owns immutable executable launch material.
evidence:
  - location: crates/opi-coding-agent/src/package_activation.rs:121
    detail: Documentation says metadata only and no validated-bytes handle is carried.
  - location: crates/opi-coding-agent/src/execution/contribution.rs:113
    detail: ValidatedExecutableContribution contains executable Arc<File>.
  - location: crates/opi-coding-agent/src/execution/runtime.rs:651
    detail: Runtime launches through that bound executable handle.
criterion_source: AGENTS.md Code quality and accurate technical documentation
reproduction:
  - Follow ActivatedContribution.validated to ValidatedExecutableContribution.executable and then to the runtime launch call.
confidence: high
status: unverified
```

### P16-STD-005 — Minor — Typed execution state is smuggled through magic JSON diagnostics

Local and routed backends independently construct the same operation-context JSON marker. `bash.rs` locates that marker twice, reparses string keys, and silently defaults missing values. Contract changes require coordinated edits across three modules.

```yaml
id: P16-STD-005
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: standards
severity: Minor
title: Bash execution context relies on a magic diagnostic schema
claim: Typed execution outcome and contract fields are duplicated into JSON diagnostics and reparsed by string key, creating primitive obsession and shotgun surgery.
evidence:
  - location: crates/opi-coding-agent/src/tool/operations.rs:826
    detail: Local operations define and construct the magic operation-context diagnostic.
  - location: crates/opi-coding-agent/src/execution/runtime.rs:687
    detail: Routed operations independently recreate the marker and JSON shape.
  - location: crates/opi-coding-agent/src/tool/bash.rs:291
    detail: BashTool searches by marker and reads string keys with false or None defaults.
criterion_source: Fowler Primitive Obsession, Duplicated Code, and Shotgun Surgery
reproduction:
  - Rename or add one operation-context field and enumerate the coordinated producer and consumer edits required across operations.rs, runtime.rs, and bash.rs.
confidence: high
status: unverified
```

### P16-STD-006 — Minor — One cleanup contract has two private constants

The host subtracts one private 1500 ms constant while the runtime adds another. They currently agree, but nothing prevents one side from drifting and changing the effective cleanup window.

```yaml
id: P16-STD-006
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: standards
severity: Minor
title: Cleanup report grace is duplicated across runtime and host
claim: The same cleanup timing contract is encoded as two independent private CLEANUP_REPORT_GRACE constants.
evidence:
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:53
    detail: The host defines the grace used to derive cancel_at.
  - location: crates/opi-coding-agent/src/execution/runtime.rs:161
    detail: Runtime defines another grace used to expand the host deadline.
criterion_source: Fowler Duplicated Code and Shotgun Surgery
reproduction:
  - Change only one CLEANUP_REPORT_GRACE constant and observe that runtime and host compute different cleanup intervals.
confidence: high
status: unverified
```

### P16-STD-007 — Minor — Public SDK variants have no production path

The standalone SDK publicly exposes output and diagnostic events that `SandboxRun` never emits, plus a setup reason described as reserved for future compatibility. This adds matching and compatibility surface without present behavior.

```yaml
id: P16-STD-007
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: standards
severity: Minor
title: Standalone SDK exposes unproducible states
claim: SandboxEvent::Output, SandboxEvent::Diagnostic, and SetupFailureReason::UnsupportedPlatform expand the public API without a production construction path.
evidence:
  - location: crates/opi-sandbox/src/runner.rs:325
    detail: Public event variants are documented as not emitted by SandboxRun.
  - location: crates/opi-sandbox/src/runner.rs:419
    detail: UnsupportedPlatform is reserved for future compatibility rather than current runner behavior.
criterion_source: AGENTS.md minimum-change rule; Fowler Speculative Generality
reproduction:
  - Search production constructors for SandboxEvent::Output, SandboxEvent::Diagnostic, and SetupFailureReason::UnsupportedPlatform; only documentation, mapping, and tests reference them.
confidence: medium
status: unverified
```

## Spec review

### P16-SPEC-001 — Major — Unix descendants can escape L0 cleanup with `setsid`

The Phase 16 acceptance contract requires all descendants to be killed on timeout, cancellation, future drop, and clean direct-child exit. Unix containment is only an initial process group; a descendant that creates a new session leaves the group and survives a kill of the original negative PGID.

```yaml
id: P16-SPEC-001
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: spec
severity: Major
title: Unix L0 does not contain descendants that leave the process group
claim: A descendant that calls setsid can survive Opi's timeout, cancellation, drop, or clean-parent-exit cleanup, contrary to C7.
evidence:
  - location: crates/opi-coding-agent/src/tool/process_tree.rs:43
    detail: Unix containment creates only a new process group for the direct child.
  - location: crates/opi-coding-agent/src/tool/process_tree.rs:299
    detail: Unix attach performs no kernel tree tracking beyond retaining the original PGID.
  - location: crates/opi-coding-agent/src/tool/process_tree.rs:369
    detail: Termination sends SIGKILL only to the original negative PGID.
  - location: crates/opi-coding-agent/tests/sandbox_l0.rs:433
    detail: Existing acceptance coverage backgrounds sleep without moving it into a new session or process group.
criterion_source: docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md:863-871; ledger C7
reproduction:
  - On Linux, run a supervised shell that backgrounds `setsid sh -c 'sleep 60'` and reports its PID.
  - Let the direct shell exit, then observe that `kill -0 <pid>` still succeeds.
confidence: high
status: unverified
```

### P16-SPEC-002 — Major — Text and TUI drop runtime failure codes and remediation

`BashTool` attaches stable runtime failures to `ToolResult.diagnostics`, but text mode ignores those diagnostics and the TUI replaces an error with the literal `failed`. NDJSON and RPC preserve the event, so the required cross-surface consistency does not hold.

```yaml
id: P16-SPEC-002
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: spec
severity: Major
title: Runtime failure diagnostics disappear on text and TUI surfaces
claim: Text and TUI projections do not preserve the stable Phase 16 failure code and remediation fields carried by ToolExecutionEnd.
evidence:
  - location: crates/opi-coding-agent/src/runner.rs:453
    detail: run_with_content ignores ToolExecutionEnd diagnostics and collects only effective contract details.
  - location: crates/opi-coding-agent/src/runner.rs:695
    detail: The ordinary text runner has the same omission and can return success after a recovered tool failure.
  - location: crates/opi-coding-agent/src/interactive.rs:980
    detail: TUI handling ignores diagnostics and stores ToolCallStatus::Error("failed").
  - location: crates/opi-coding-agent/tests/execution_product.rs:755
    detail: Claimed cross-surface coverage serializes a prebuilt event to NDJSON/RPC and does not exercise text or TUI projection.
criterion_source: docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md:744-770; SC16-14
reproduction:
  - Make a mock external backend return execution_failed, then let the provider recover with a text response.
  - Observe that NonInteractiveRunner stderr lacks execution_failed and remediation and exits successfully.
  - Drive permission_denied through the TUI callback and observe only the word failed.
confidence: high
status: unverified
```

### P16-SPEC-003 — Major — Common host teardown suppresses `cleanup_unconfirmed`

Several post-execute protocol failure paths ignore both `TreeGuard::terminate()` failure and child/stderr teardown confirmation. They return the original protocol code even when cleanup is not confirmed. A separate partial-transmission path already implements the required elevation, demonstrating the intended policy.

```yaml
id: P16-SPEC-003
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: spec
severity: Major
title: Protocol failures can mask unconfirmed cleanup
claim: After target disclosure or start, EOF, malformed frames, diagnostic overflow, and transition errors can return protocol_violation even when host-side tree termination or reap confirmation fails.
evidence:
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:511
    detail: Main post-execute EOF and codec failure branches call terminate_and_fail.
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:1247
    detail: terminate_and_fail discards TreeGuard termination outcome and finish_teardown confirmation.
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:1235
    detail: The cancel-path error branch has the same behavior.
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:1268
    detail: Partial-frame failure correctly returns cleanup_unconfirmed unless tree termination, reap, and stderr drain all confirm.
criterion_source: docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md:516-529,850-861; task 16.7
reproduction:
  - Inject a TreeGuard termination failure while a mock backend emits an out-of-order or malformed post-execute frame.
  - Observe protocol_violation instead of cleanup_unconfirmed.
confidence: high
status: unverified
```

### P16-SPEC-004 — Major — Phase-exit evidence is not reproducible at the audit head

The ledger claims preserved platform, six-target, and workspace-gate evidence under ignored `target/` state. A clean checkout contains none of it. Its recorded audit command also omits identity arguments now required by the auditor, and one recorded documentation test target no longer exists.

```yaml
id: P16-SPEC-004
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: spec
severity: Major
title: C16 preserved evidence cannot be reproduced from committed HEAD
claim: The committed Phase 16 ledger's final evidence commands do not pass in a clean checkout, so the 16/16 phase-exit claim is not independently verifiable at audit_head.
evidence:
  - location: docs/snapshots/phase16/opi-impl-state.json:3112
    detail: The recorded artifact-audit command omits mandatory workflow-run-id and commit-sha arguments.
  - location: scripts/opi-artifact-audit.py:2011
    detail: Current phase-exit mode requires both declared identity arguments.
  - location: .gitignore:1
    detail: target/ is ignored; the claimed target/opi-artifacts/phase16-phase-exit bundle is absent from a clean checkout.
  - location: docs/snapshots/phase16/opi-impl-state.json:3098
    detail: Verification names phase16_extension_docs, but that test target is absent at audit_head.
criterion_source: task 16.16.3 definition of done; ledger C16
reproduction:
  - Run `python scripts/opi-artifact-audit.py target/opi-artifacts/phase16-phase-exit --workspace-root . --phase-exit --json` in a clean checkout; it exits 1 with missing identity, platform, six-target, and gate evidence.
  - Run `cargo test --offline -p opi-coding-agent --test phase16_extension_docs --no-run`; Cargo reports no such test target.
confidence: high
status: unverified
```

### P16-SPEC-005 — Minor — Passing task records retain open scenarios and null evidence

The phase-exit summary says all 21 tasks pass, but seven passing tasks retain open acceptance scenarios and five passing tasks have null evidence. Later criteria traces may be intended to supersede these fields, but the ledger does not record that transition.

```yaml
id: P16-SPEC-005
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: spec
severity: Minor
title: Phase ledger closure state is internally inconsistent
claim: The all-tasks-passing summary conflicts with open acceptance scenarios and null evidence on passing task records.
evidence:
  - location: docs/snapshots/phase16/opi-impl-state.json
    detail: Tasks 16.13, 16.14.1, 16.14.2, 16.15.2, and 16.16.1 retain open scenarios; 16.13 and 16.14.1 each retain two.
  - location: docs/snapshots/phase16/opi-impl-state.json
    detail: Tasks 16.9, 16.14.1, 16.15.2, 16.16.1, and 16.16.3 have evidence null while marked passing.
criterion_source: opi-audit requirement to verify every ledger evidence claim and task verdict
reproduction:
  - Parse the Phase 16 ledger and list passing tasks with acceptance_scenarios.status == open or evidence == null.
confidence: high
status: unverified
```

## Correctness, security, invariants, integration, and residuals

### P16-INTEG-001 — Major — Routed output is silently truncated at 64 KiB

External execution can return much more than 64 KiB within protocol bounds. `BashTool` takes only the first 64 KiB, while routed context hard-codes `truncated=false` and provides no `full_output`. A successful command therefore loses data irrecoverably and advertises complete success.

```yaml
id: P16-INTEG-001
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: integration
severity: Major
title: External bash output is silently discarded after 64 KiB
claim: A routed adapter result larger than MAX_BASH_OUTPUT_BYTES returns only the prefix with is_error=false, truncated=false, and no full_output recovery path.
evidence:
  - location: crates/opi-coding-agent/src/tool/bash.rs:242
    detail: Routed stdout and stderr are merged and sliced to the 64 KiB preview cap.
  - location: crates/opi-coding-agent/src/execution/runtime.rs:721
    detail: Routed operation context always records truncated=false.
  - location: crates/opi-coding-agent/src/tool/operations.rs:1003
    detail: The local path correctly detects overflow and spills complete output to full_output.
  - location: crates/opi-coding-agent/tests/execution_product.rs:721
    detail: End-to-end routed coverage asserts only a small hello payload.
criterion_source: Phase 16 no-degraded-success contract and transparent BashOperations integration
reproduction:
  - Use a valid external adapter to return 65,537 bytes and exit 0.
  - Invoke it through BashTool and observe 65,536 retained bytes, truncated=false, full_output absent, and is_error=false.
confidence: high
status: unverified
```

### P16-INTEG-002 — Major — Standalone CLI drops output after 1 MiB but returns success

The standalone runner caps each stream at 1 MiB. The CLI writes the retained prefix, emits a warning, and returns the target's unchanged exit code. This contradicts byte pass-through and creates a degraded-success state.

```yaml
id: P16-INTEG-002
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: integration
severity: Major
title: opi-sandbox run truncates streams while preserving exit zero
claim: Direct opi-sandbox run drops bytes beyond 1 MiB per stream and still reports a successful target exit.
evidence:
  - location: crates/opi-sandbox/src/runner.rs:56
    detail: Runner explicitly drops output beyond a 1 MiB per-stream cap.
  - location: crates/opi-sandbox/src/cli.rs:258
    detail: CLI writes only retained bytes, adds a warning, then returns map_outcome unchanged.
  - location: crates/opi-sandbox/tests/sdk_contract.rs:464
    detail: SDK coverage confirms the cap but does not enforce direct CLI byte pass-through.
  - location: crates/opi-sandbox/src/backend.rs:763
    detail: Protocol mode also reports truncation only as a diagnostic while retaining successful completion.
criterion_source: docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md:580-595,773-774
reproduction:
  - On Linux or macOS, run opi-sandbox with a target that writes 1,048,577 stdout bytes and exits 0.
  - Observe 1,048,576 output bytes, a warning on stderr, and CLI exit 0.
confidence: high
status: unverified
```

### P16-TEST-001 — Minor — macOS missing-helper refusal is not tested end to end

Probe classification and production refusal are covered separately, but no macOS test drives an actually missing or rejected canonical `sandbox-exec` through the complete pre-start path.

```yaml
id: P16-TEST-001
source_kind: audit
source_path: docs/snapshots/phase16/audit.codex.md
source_model: codex
independence: unknown
axis: test-quality
severity: Minor
title: macOS rejected-helper plumbing lacks a chained test
claim: A regression between sandbox-exec probe classification and production pre-start refusal can pass the current split tests.
evidence:
  - location: crates/opi-sandbox/tests/macos_policy.rs:25
    detail: The test module explicitly documents split proof and the absence of a chained missing or unusable-helper test.
criterion_source: docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md:899-901
reproduction:
  - Break propagation from macos_posture_fields(Missing or Unusable) to production CLI/backend gating.
  - Run current macos_policy tests and observe that the independent probe and available-helper groups need not catch the plumbing break.
confidence: medium
status: unverified
```

### Invariant matrix

| Invariant | Verdict | Notes |
|---|---|---|
| Installed / Trusted / Enabled / Selected / Permitted are separate gates | PASS | Activation, routing, and permission paths remain independently gated. |
| Per-invocation named-package revalidation | PASS | Manifest, lock, executable, target, version, protocol, and trust are rechecked immediately before spawn. |
| User-only permission ownership | PASS | Project `[execution.permissions]` is rejected before merge. |
| Model cannot mutate trust, enablement, or permission | PASS | Model strategy supplies only a visible candidate backend. |
| Selected external failures never fall back to local | PASS | Activation and protocol errors propagate through one selected dispatch. |
| Stable 14-code redacted envelope | PARTIAL | Mapping and redaction pass, but text/TUI projections drop runtime codes and remediation (P16-SPEC-002). |
| Command/config absent from adapter argv | PASS | Manifest args only; command and config travel in protocol frames. |
| Protocol request id, state ordering, and bounds | PARTIAL | State and cumulative bounds pass; exact-cap CRLF interoperability fails (P16-STD-001). |
| Deadline, cancellation, and cleanup classification | FAIL | Common teardown masks unconfirmed cleanup (P16-SPEC-003). |
| L0 descendant cleanup | FAIL | Unix descendants can leave the tracked process group (P16-SPEC-001). |
| No degraded success | FAIL | Routed and standalone output truncation retain success (P16-INTEG-001, P16-INTEG-002). |
| Linux native restriction posture | PASS with residual | Static and test contracts are strong; this Windows audit did not rerun real-kernel Linux policy tests. |
| macOS native restriction posture | PASS with evidence gap | Available path is covered; missing/unusable helper chaining is P16-TEST-001. |
| Windows L0-only / no official sandbox artifact | PASS | Current-host posture tests and release topology pass. |
| Packaging and release topology | PASS | Four native sandbox archives and six ordinary Opi targets are encoded in workflows. |
| Crate boundaries | PASS | `opi-sandbox` remains standalone over `opi-protocol`; core does not link it. |

### Residual risks without a finding

- Linux kernels below the exercised Landlock ABI floor remain a platform-coverage limitation documented by the tests.
- Linux and macOS native enforcement was not dynamically rerun from this Windows host; the audit relied on source, platform-gated tests, workflow topology, and the explicit missing-evidence finding.
- Package trust records are fail-closed on corruption, but their plain-file writes remain a durability/concurrency area worth future focused review.

## Task and criterion matrix

| Task | Verdict | Primary evidence |
|---|---|---|
| 16.1 | PASS | Normative design and documentation contract present. |
| 16.2 | FAIL | P16-SPEC-001. |
| 16.3 | PASS | Shared v1 types, schemas, fixtures, state, and bounds are present. |
| 16.4 | PASS | Contribution gates and immutable launch binding are present. |
| 16.5 | PASS | Package trust and enable/disable lifecycle gates are present. |
| 16.6 | PASS | Config, routing, permission, failures, and project-layer rejection are present. |
| 16.7 | FAIL | P16-STD-001 and P16-SPEC-003. |
| 16.8 | FAIL | P16-INTEG-001 crosses the runtime-to-BashTool seam. |
| 16.9 | FAIL | P16-SPEC-002 and P16-INTEG-001. |
| 16.10 | FAIL | TUI loses permission/runtime failure diagnostics (P16-SPEC-002). |
| 16.11.1 | PASS | Standalone SDK state, runner, and cleanup structure are present. |
| 16.11.2 | FAIL | Direct CLI byte pass-through fails (P16-INTEG-002). |
| 16.12 | PASS | Atomic helper gate and backend state machine are present. |
| 16.13 | PASS | Linux restriction implementation and platform-gated contracts are present. |
| 16.14.1 | PARTIAL | Implementation present; P16-TEST-001 remains. |
| 16.14.2 | PASS | Windows unsupported posture and no-artifact topology are present. |
| 16.15.1 | PASS | Host-neutral packaging topology is present. |
| 16.15.2 | PARTIAL | Workflow topology passes; committed phase-exit evidence is not reproducible. |
| 16.16.1 | PASS | Core native sandbox surface and dependency are removed. |
| 16.16.2 | FAIL | Cross-surface diagnostics claim fails (P16-SPEC-002). |
| 16.16.3 | FAIL | P16-SPEC-004 and P16-SPEC-005. |

| Criterion | Verdict | Notes |
|---|---|---|
| C1 | PASS | Minimal Runtime remains direct local. |
| C2 | PASS | Five lifecycle gates and durable drift invalidation are present. |
| C3 | PASS | Routing/permission selection is deterministic and model non-authoritative. |
| C4 | PARTIAL | Codes/remediation exist but are not preserved by text/TUI. |
| C5 | FAIL | Silent/truncated success violates no-degraded-success semantics. |
| C6 | FAIL | Exact-cap CRLF host interoperability defect. |
| C7 | FAIL | Escaped Unix descendants and masked cleanup failure. |
| C8 | PASS | Standalone SDK/CLI architecture exists. |
| C9 | PASS | Linux policy implementation matches the registered design statically. |
| C10 | PARTIAL | macOS implementation present; missing-helper end-to-end evidence gap. |
| C11 | PASS | Windows L0-only posture is explicit. |
| C12 | PARTIAL | Workflow topology passes; preserved artifact evidence is absent. |
| C13 | PASS | Migration and rejected legacy surface are present. |
| C14 | PASS | Crate dependency boundaries are enforced. |
| C15 | PARTIAL | Paired docs pass current checks, but narrative tests violate policy and the ledger names a removed doc target. |
| C16 | FAIL | Final preserved evidence is not reproducible from committed HEAD. |

## Verification performed

| Command | Result |
|---|---|
| `cargo test --offline -p opi-protocol -p opi-sandbox -p opi-coding-agent --features execution-backend-test-fixture --all-targets` | PASS on Windows; exit 0. |
| `cargo clippy --offline -p opi-protocol -p opi-sandbox -p opi-coding-agent --features execution-backend-test-fixture --all-targets -- -D warnings` | PASS. |
| `cargo fmt --check --all` | PASS. |
| `python scripts/opi-doc-check.py` | PASS. |
| `cargo test --offline -p opi-protocol --test execution_v1_contract line_reader_allows_lf_and_crlf_at_the_same_data_cap -- --exact` | PASS; confirms the canonical codec contract. |
| `cargo test --offline -p opi-coding-agent --test phase16_extension_docs --no-run` | FAIL as evidence; no such test target exists. |
| `python scripts/opi-artifact-audit.py target/opi-artifacts/phase16-phase-exit --workspace-root . --phase-exit --json` | FAIL as evidence; declared identity and all preserved bundles are absent. |

No source, test, spec, or product documentation files were changed. Test impact: `none` (audit report only). The pinned root and isolated-checkout HEAD remained `5c8d2ba561392bc054625a50c1ac8d72e020e8d9` through report completion.
