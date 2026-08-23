# Phase 17 Audit

**Audit head**: `528989279e9be308abc963ec22f377ee47bbde47`  
**Reviewer/model**: Codex  
**Independence**: `fresh-context-same-family` — the review used a fresh audit context, but the reviewer is conservatively classified in the same model family as possible implementation or prior assurance work.  
**Run ID**: `20260823t055516z`  
**Contamination**: none. The live worktree was clean at endpoint capture; all inspection and commands ran in detached worktree `C:\Users\Luiz\AppData\Local\Temp\opi-phase17-audit-6df76cf863e74710a6f282672bbcf3e8` at the exact audit head.  
**Verdict**: **FAIL**

The finding sidecar `audit.codex.5289892.20260823t055516z.findings.jsonl` is the normalized source of truth. The current findings and requirement conclusions were sealed before prior Phase 17 audit conclusions were opened.

## Requirement Conformance

Each mandatory Phase 17 row is preserved separately. “Evidence” names the production surface, discriminating test/fixture, and check used; passing tests are not treated as sufficient where the fixture does not exercise the claimed state.

| Requirement | Criterion source | Observable, production surface, test/check evidence | Requirement state | Finding IDs |
|---|---|---|---|---|
| P17-AUTH-001 | phase17 design:21 | Parent gates are largely implemented, but current legacy-session and runtime-binding behavior lowers `INV-007`/`INV-008`. | `partially-met` | P17-AUD-001, P17-AUD-002 |
| P17-AUTH-002 | phase17 design:22 | Status remains in `.opi-impl-state.json` and `docs/snapshots`; `python scripts/opi-doc-check.py` passed. | `met` | — |
| P17-AUTH-003 | phase17 design:23 | The v2 binding remediation made supported v1 product resume infeasible without revising the registered migration requirement. | `not-met` | P17-AUD-001 |
| P17-OUT-001 | phase17 design:116 | `ProviderCollection`, `Agent::replace_state`; cross-provider tests in `agent_wrapper` and `phase17_provider_runtime` passed. | `met` | — |
| P17-OUT-002 | phase17 design:117 | Complete `NextTurnState` validation/apply and rollback; `agent_wrapper`/`hooks_queues` state snapshots passed. | `met` | — |
| P17-OUT-003 | phase17 design:118 | Immutable registry, schema, authorizer, freshness, execution counter; authority suites passed. | `met` | — |
| P17-OUT-004 | phase17 design:119 | Correlation/config/policy/artifact fields exist, but session-backed DirectRuntimeInput does not address current material run inputs. | `partially-met` | P17-AUD-002 |
| P17-PRV-001 | phase17 design:302 | Sole `Provider::stream_prepared`, collection lookup/auth/dispatch; two-provider production tests passed. | `met` | — |
| P17-PRV-002 | phase17 design:303 | Canonical `provider:model`, ambiguous bare rejection, no alias registry; route tests passed. | `met` | — |
| P17-PRV-003 | phase17 design:304 | Opaque `PreparedProviderCall` freezes route/auth across attempts; resolver-once retry tests passed. | `met` | — |
| P17-PRV-004 | phase17 design:305 | Typed route/auth failures and typed allowed fallback; zero-dispatch negative tests passed. | `met` | — |
| P17-PRV-005 | phase17 design:306 | Requested/resolved/actual/auth/fallback typed facts; product evidence mismatch tests passed. | `met` | — |
| P17-PRV-006 | phase17 design:307 | Provider-specific encoders/decoders remain behind provider-neutral request/stream interfaces; Cargo/source review passed. | `met` | — |
| P17-NXT-001 | phase17 design:380 | One complete state replacement surface persists through public Agent operations; tests passed. | `met` | — |
| P17-NXT-002 | phase17 design:381 | Candidate validation precedes atomic apply; invalid/cancel snapshots remain unchanged. | `met` | — |
| P17-NXT-003 | phase17 design:382 | Stop observes the applied complete state; hook-order test passed. | `met` | — |
| P17-NXT-004 | phase17 design:383 | Stop returns before queue polling; zero-poll probes passed. | `met` | — |
| P17-NXT-005 | phase17 design:384 | Product compaction uses complete replacement and removes superseded provider context; session runtime test passed. | `met` | — |
| P17-NXT-006 | phase17 design:385 | Next call resolves from applied model state; cross-provider Agent test passed. | `met` | — |
| P17-AUT-001 | phase17 design:537 | Registry-owned registration/origin/capability resolves before later boundaries; unknown/duplicate/forgery tests passed. | `met` | — |
| P17-AUT-002 | phase17 design:538 | Final arguments are schema-validated and forwarded unchanged; validation/identity tests passed. | `met` | — |
| P17-AUT-003 | phase17 design:539 | Product authorizer consumes immutable policy, scoped state, and current evidence generation; reauthorization tests passed. | `met` | — |
| P17-AUT-004 | phase17 design:540 | Model/hooks/extensions/results cannot mutate trusted grants/capability/scope; malicious-source matrix passed. | `met` | — |
| P17-AUT-005 | phase17 design:541 | Missing/error/deny/expired/stale/schema-invalid cases reach zero executions; counter tests passed. | `met` | — |
| P17-AUT-006 | phase17 design:542 | `BeforeToolCallResult` is Continue/Deny only; hook tests passed. | `met` | — |
| P17-AUT-007 | phase17 design:543 | After-call replacement cannot rewrite authorization evidence or later authority; consecutive-call test passed. | `met` | — |
| P17-AUT-008 | phase17 design:544 | Tool projection is rebuilt from trusted registrations per request; consecutive projection tests passed. | `met` | — |
| P17-EVD-001 | phase17 design:654 | UUIDv7 run identity and monotonic turn/call/sequence allocated before emit; contract/runtime tests passed. | `met` | — |
| P17-EVD-002 | phase17 design:655 | Provider/retry/tool/compaction/terminal records carry typed correlation and kind; graph tests passed. | `met` | — |
| P17-EVD-003 | phase17 design:656 | Manifest schema and direct-vs-snapshot validation exist, but product session binding digest omits current material inputs. | `partially-met` | P17-AUD-002 |
| P17-EVD-004 | phase17 design:657 | Route and measurement categories remain closed/distinct; unknown is not zero; serialization tests passed. | `met` | — |
| P17-EVD-005 | phase17 design:658 | Redacted typed payloads cross the sink boundary; canary sink/file tests passed. | `met` | — |
| P17-EVD-006 | phase17 design:659 | Capture absent by default and no Eval-triggered activation; default harness test passed. | `met` | — |
| P17-EVD-007 | phase17 design:660 | Product setup occurs before provider/tool effects; setup-failure zero-call test passed. | `met` | — |
| P17-EVD-008 | phase17 design:661 | Emission/finalization failure advances health and withholds manifest while retaining outcome; tests passed. | `met` | — |
| P17-EVD-009 | phase17 design:662 | Incomplete health reauthorizes stale allows and blocks unlaunched effects; in-flight result tests passed. | `met` | — |
| P17-EVD-010 | phase17 design:663 | Core contains only no-op/in-memory adapters; file sink stays in product; API audit passed. | `met` | — |
| P17-EVD-011 | phase17 design:664 | Shared lifecycle/failure/redaction behavior across adapters; conformance suites passed. | `met` | — |
| P17-FAL-001 | phase17 design:693 | Closed typed boundary errors and exhaustive mapping tests passed. | `met` | — |
| P17-FAL-002 | phase17 design:694 | Boundary-precedence call-count tests passed; detected failures stop later boundaries. | `met` | — |
| P17-FAL-003 | phase17 design:695 | Most cancellation/timeout/queue/evidence/partial cases are preserved, but Bedrock can ignore a truncated trailer after terminal success. | `partially-met` | P17-AUD-005 |
| P17-FAL-004 | phase17 design:696 | Text/JSON/RPC/evidence canary matrix passed; errors use safe typed summaries. | `met` | — |
| P17-MIG-001 | phase17 design:740 | Core can read v1 for historical export, but product resume/fork reject it; claimed legacy fixtures are v2. | `not-met` | P17-AUD-001 |
| P17-MIG-002 | phase17 design:741 | Route normalization works for v2 entries with legacy model shape, but actual v1 sessions are rejected before normalization. | `not-met` | P17-AUD-001 |
| P17-MIG-003 | phase17 design:742 | Explicit CLI/RPC/file capture remains available through product `FileEvidenceSink`; tests passed. | `met` | — |
| P17-MIG-004 | phase17 design:743 | Opaque legacy trace files are not read or overwritten; coexistence/byte tests passed. | `met` | — |
| P17-MIG-005 | phase17 design:744 | Shared cross-mode fixture covers route/authority/cancellation/evidence; seven tests passed. | `met` | — |
| P17-MIG-006 | phase17 design:745 | Removed interfaces absent without aliases/flags; source/API audit passed. | `met` | — |
| P17-PLT-001 | phase17 design:761 | Windows gates were exercised, but GitHub Actions has no run for audit head; parent CI cannot prove current Linux/macOS behavior. | `not-assessable` | P17-AUD-003 |
| P17-PLT-002 | phase17 design:762 | Hermetic source guard and mock/loopback fixtures passed; no live/paid provider use observed. | `met` | — |
| P17-PLT-003 | phase17 design:763 | Docs/diagnostics state authorization is not an OS sandbox; source audit passed. | `met` | — |
| P17-A01 | phase17 design:769 | One harness switches alpha to beta with matching route/auth/fallback evidence; test passed. | `met` | — |
| P17-A02 | phase17 design:770 | Unknown/ambiguous/auth/refresh/wire failures produce typed zero-dispatch results; tests passed. | `met` | — |
| P17-A03 | phase17 design:771 | Retry attempts share route, prepared auth, parent call, and terminal evidence; test passed. | `met` | — |
| P17-A04 | phase17 design:772 | Stop observes complete context/model/thinking/token/temperature replacement; tests passed. | `met` | — |
| P17-A05 | phase17 design:773 | Invalid/cancelled prepare preserves all prior fields and skips stop/queues; tests passed. | `met` | — |
| P17-A06 | phase17 design:774 | Model request to expand permission leaves policy unchanged and execution zero; product test passed. | `met` | — |
| P17-A07 | phase17 design:775 | Forged content sources cannot enter registry/grant fields; authority matrix passed. | `met` | — |
| P17-A08 | phase17 design:776 | Expired/failed authority denies with visible safe source/code and zero execution; tests passed. | `met` | — |
| P17-A09 | phase17 design:777 | One run covers provider/retry/tool/compaction graph, but its product runtime-input digest is not exact for current material inputs. | `partially-met` | P17-AUD-002 |
| P17-A10 | phase17 design:778 | Prompt/args/environment/provider-error canaries absent from sink/files/diagnostics/artifact metadata; tests passed. | `met` | — |
| P17-A11 | phase17 design:779 | Emission/finalization failures retain outcome, mark incomplete, and withhold manifest; tests passed. | `met` | — |
| P17-A12 | phase17 design:780 | Stale allow reauthorization and prelaunch/in-flight distinctions are exercised; tests passed. | `met` | — |
| P17-A13 | phase17 design:781 | Trace coexistence is covered, but the session fixture is v2 and actual v1 product load/resume fails. | `not-met` | P17-AUD-001 |
| P17-A14 | phase17 design:782 | Interactive assembly, harness, print, JSON/NDJSON, and RPC fixtures share semantics; tests passed. | `met` | — |
| P17-A15 | phase17 design:783 | No Linux/macOS/Windows CI run exists for the audited commit. | `not-assessable` | P17-AUD-003 |
| P17-RBK-001 | phase17 design:837 | This audit converts mandatory gaps into `FAIL`, so the registered blocking mechanism is applied. | `met` | P17-AUD-001, P17-AUD-002, P17-AUD-003, P17-AUD-005 |
| P17-RBK-002 | phase17 design:838 | Source/API audit shows one dispatch/state/authority/evidence path and no hidden compatibility runtime. | `met` | — |
| P17-RBK-003 | phase17 design:839 | Phase 17 session/evidence byte-preservation rollback fixture passed. | `met` | — |
| P17-RBK-004 | phase17 design:840 | Before/after policy digest and denied capability remain unchanged; rollback test passed. | `met` | — |

Summary: 59 `met`, 5 `partially-met`, 4 `not-met`, and 2 `not-assessable`. Mandatory rows that are not `met` force `FAIL` independently of finding severity.

## Standards Review

Dependency direction, safe Rust posture, error typing, provider-neutral boundaries, formatting, clippy, rustdoc, and bilingual/document contracts are clean in the inspected surface. The required workspace test gate is not clean: `P17-AUD-004` identifies a hard-coded target-directory assumption in `shell_completions.rs` that conflicts with the repository's mandated external Cargo cache workflow.

## Spec Review

Provider dispatch, next-turn state, trusted tool authority, most evidence lifecycle semantics, failure precedence, cross-mode equivalence, removal audits, and rollback behavior conform. The material deviations are:

- `P17-AUD-001`: supported v1 product sessions no longer reach resume/fork or legacy route normalization;
- `P17-AUD-002`: DirectRuntimeInput binding contents do not cover current resolved material inputs;
- `P17-AUD-003`: current-commit three-platform acceptance is unavailable; and
- `P17-AUD-005`: a truncated Bedrock trailer can be converted into terminal success.

## Security, Invariants, Integration, Test Quality, and Residuals

Security and authority review found fail-closed registration/schema/authorization ordering, current evidence-health reauthorization, typed non-secret provider provenance, producer-side redaction, and no raw secret crossing the evidence sink in the inspected and executed canary cases.

Invariant and integration gaps are recorded as `P17-AUD-001`, `P17-AUD-002`, and `P17-AUD-005`. Test-quality gaps are recorded as `P17-AUD-003` and `P17-AUD-004`. The most important anti-vacuity observation is that all seven `phase17_legacy_migration` tests pass while their generated “legacy” artifact is schema v2; success therefore does not support the v1 compatibility claim.

Residual scans found no forbidden legacy provider/Agent/evidence interface, duplicate registry, core file exporter, product policy in Agent Core, live-provider acceptance dependency, or new OS-specific permission implementation.

## Minimum-change Conformance

| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |
|---|---|---|---|---|---|---|---|
| 17.1 | Dispatchable provider collection | Reuses registry, providers, auth resolver | `opi-ai` core | Collection + opaque prepared call | All Agent provider attempts | No router/retry-factory/second registry | `conforming` |
| 17.2 | Complete next-turn replacement | Reuses Agent loop/hooks/queues | `opi-agent` core | `NextTurnState` + one replacement seam | Prompt/continue/compaction | No patch protocol or mutable run binding | `conforming` |
| 17.3 | Evidence vocabulary/lifecycle | Reuses trace/event/redaction vocabulary | `opi-agent` core | Typed identities, health, sink, manifest | Core no-op/in-memory adapters | No file/exporter/Eval store | `conforming` |
| 17.4 | Trusted tool authorization | Reuses registry/schema/hooks/execution | Core mechanism + product policy | RegisteredTool/authorizer/policy | Every tool preflight | No hook grants/new permission language | `conforming` |
| 17.5 | Product provider assembly | Reuses provider factory/credentials/collection | Reference Product | Canonical route + resolver provenance | Interactive/headless/RPC harness assembly | No listing proxy or fallback dispatch | `conforming` |
| 17.6 | Core evidence runtime | Reuses Agent lifecycle/retry/tool/compaction | `opi-agent` core | Correlated evidence runtime | Every provider/tool/compaction boundary | No product file adapter in core | `conforming` |
| 17.7 | Product evidence cutover | Reuses capture inputs/runners/redaction | Reference Product | File sink + strict manifest + policy mapping | All public modes | No dual trace runtime/exporter | `drifted` — post-task session override weakens the recorded DirectRuntimeInput material binding (P17-AUD-002) |
| 17.8 | Legacy migration/preservation | Reuses session repository/route normalization | Reference Product | Read/normalize/fork + opaque trace coexistence | Product session CLI/harness | No rewrite/reader/shim | `drifted` — actual v1 resume/fork is rejected and acceptance fixture became v2 (P17-AUD-001) |
| 17.9 | Assurance/removal/rollback | Reuses hermetic modes, CI, source audits | Assurance only | Tests/docs/CI definition | No production source owned | No runtime repair/OS policy | `conforming` structurally; current verification gaps are P17-AUD-003/P17-AUD-004 |

## Findings

### P17-AUD-001: Product resume and fork reject supported v1 sessions

- Axis: `integration`
- Severity: Major
- Claim: See normalized sidecar; actual schema-v1 sessions are rejected before product resume/fork/route normalization, while the acceptance helper writes v2.
- Evidence: `session_cli.rs:175-218`, `session.rs:715-720`, `phase17_legacy_migration.rs:70-78`, and the generated v2 artifact.
- Criterion: `P17-MIG-001`, `P17-MIG-002`, `P17-A13`, parent `INV-007`.
- Refutation attempted: Core `read_with_recovery` and `SessionFacade` still read v1 for historical/export use. This does not refute the claim because the product resume and fork paths exclusively call the stricter v2-only reader.
- Suggested closure: Restore a truthful supported-v1 product resume/fork/normalization outcome with byte-preserving discriminating v1 fixtures, or explicitly revise the registered compatibility requirement before changing support.

### P17-AUD-002: DirectRuntimeInput digest omits material run inputs

- Axis: `invariants`
- Severity: Major
- Claim: See normalized sidecar; session-backed manifests use a fixed cwd/initial-model digest instead of a digest of current resolved material run inputs.
- Evidence: `session_coordinator.rs:990-997`, `harness.rs:196-318,2989-3004`, `session.rs:74-116`, and `phase17_product_evidence.rs:1978-1983`.
- Criterion: domain `Runtime Input Binding`, `P17-EVD-003`, `P17-OUT-004`, parent `CTRL-002`/`INV-008`.
- Refutation attempted: The manifest separately updates config, policy, input, and route identities, and `INV-007` requires the session prefix binding to remain immutable. Those facts preserve much of resolved-execution identity but do not make the DirectRuntimeInput digest cover the material inputs used by the current run; the provider-switch test proves the digest stays stale as a material input changes.
- Suggested closure: Make the direct binding truthfully address the resolved material inputs it denotes while retaining one coherent committed-prefix/session authority model.

### P17-AUD-003: Audited remediation commit has no three-platform CI evidence

- Axis: `test-quality`
- Severity: Major
- Claim: See normalized sidecar; no Actions run exists for the audited SHA.
- Evidence: `gh run list` returned `[]`; HEAD changes Phase 17 runtime and tests.
- Criterion: `P17-PLT-001`, `P17-A15`.
- Refutation attempted: CI run `32484643147` is green for parent `136c380`, but it predates the audited production/session/evidence/provider changes and therefore cannot verify this head.
- Suggested closure: Obtain a green repository three-platform acceptance run bound to the exact implementation commit under audit.

### P17-AUD-004: Shell-completion tests break the required external Cargo-cache gate

- Axis: `test-quality`
- Severity: Major
- Claim: See normalized sidecar; all eight completion tests fail under the canonical external target directory.
- Evidence: `shell_completions.rs:9-18`; Cargo metadata target `E:/opi/cargo-targets/windows-direct`; focused test 0/8.
- Criterion: repository verification and external-cache rules in `AGENTS.md`.
- Refutation attempted: CI environments using the default workspace-local target directory may pass. That does not refute the repository's explicitly required external-cache workflow, and other integration tests already use Cargo's exact `CARGO_BIN_EXE_opi` path.
- Suggested closure: Resolve the tested executable from Cargo's integration-test binary contract and make the exact workspace gate pass under the repository cache workflow.

### P17-AUD-005: Bedrock accepts a terminal stream followed by a truncated frame

- Axis: `integration`
- Severity: Major
- Claim: See normalized sidecar; transport EOF does not reject bytes left by the incremental frame parser after a valid terminal application event.
- Evidence: `event_stream.rs:27-40`, `bedrock/mod.rs:389-452`, and current complete-frame-only regression coverage.
- Criterion: `P17-FAL-003`, parent `INV-006`.
- Refutation attempted: Malformed complete frames now produce a typed error, and a stream without `messageStop` fails the `saw_done` check. Neither covers a valid `messageStop` followed by an incomplete declared frame: the buffer remains non-empty while `saw_done` is already true.
- Suggested closure: Treat non-empty decoder residue at HTTP EOF as a typed stream failure and add a production-path terminal-plus-truncated-trailer fixture.

## Verification Commands

| Command | Result | Obligation/finding |
|---|---|---|
| `python scripts/opi-cargo-cache.py status` | PASS | External cache inspection before Cargo work |
| `cargo test -p opi-ai --test provider_collection --test per_request_auth` | PASS (66) | Provider route/auth/dispatch/retry |
| `cargo test -q -p opi-agent --test agent_wrapper --test hooks_queues --test phase17_prepare_call --test tool_authority --test tool_validation --test evidence_contract --test evidence_runtime --test session_storage --test session_facade --test session_context` | PASS (244) | State, authority, evidence, session contract |
| `cargo test -q -p opi-coding-agent --test phase17_provider_runtime --test phase17_tool_authority --test phase17_product_evidence --test phase17_legacy_migration --test phase17_cross_mode --test phase17_failure_rollback --test phase17_api_audit` | PASS (107) | Registered Phase 17 product acceptance; anti-vacuity inspection still found P17-AUD-001 |
| `cargo fmt --check --all` | PASS | Formatting gate |
| `python scripts/opi-doc-check.py` | PASS | Documentation/metadata contract |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | Workspace lint gate |
| `cargo test --workspace --all-targets` | FAIL | `shell_completions`: 0/8 under external target; P17-AUD-004 |
| `cargo test -q -p opi-coding-agent --test shell_completions` | FAIL (0/8) | Focused reproduction for P17-AUD-004 |
| `cargo test --workspace --doc` | PASS | Documentation tests |
| `$env:RUSTDOCFLAGS='-D warnings'; cargo doc --workspace --no-deps` | PASS | Rustdoc warnings gate |
| `gh run list --repo OdradekAI/opi --commit 528989279e9be308abc963ec22f377ee47bbde47 ...` | EMPTY | Current-head platform evidence; P17-AUD-003 |

## Prior-history Comparison

This comparison was performed only after the current sidecar, finding set, and requirement states were sealed. It does not alter those conclusions.

| Current finding | Prior lineage | Comparison |
|---|---|---|
| P17-AUD-001 | P17-CODEX-001 (`Closed`, `session.v2-required-entry-and-binding`) | New adjacent regression: the prior remediation correctly added v2 required/ignorable entries, but the product's strict v2 resume path now prevents the separately required v1 resume/fork migration behavior from running. |
| P17-AUD-002 | P17-CODEX-001 (`Closed`, same closure family) | New residual in the binding leg: the earlier audit found no durable binding; the remediation added one, but its product digest is not exact for the material run inputs it claims to bind. |
| P17-AUD-003 | GLM F-003 (`Info/No action`, `snapshot.phase17-ci-claim`) | New exact-head evidence gap. The prior finding concerned an inaccurate historical CI summary and identified a later green parent run; the audited remediation head has no run at all and contains material runtime changes. |
| P17-AUD-004 | P17-CODEX-004 (`Closed`, `session-cli.cargo-provided-binary`) | Recurring defect family in a distinct test binary: the session CLI path was repaired, but `shell_completions` independently retains the same checkout-local target assumption and now fails the workspace gate. |
| P17-AUD-005 | P17-CODEX-002 (`Closed`, `bedrock.crc-invalid-terminal-error`) | New adjacent stream-integrity edge: the prior remediation rejects malformed complete frames; it does not reject incomplete decoder residue after a valid terminal frame. |

The other prior Codex and GLM findings and their remediation dispositions do not duplicate the five sealed current claims. In particular, no closed prior finding is silently treated as open merely because it shares a subsystem; the table distinguishes a repeated claim from a new residual or adjacent regression.

## Verdict Rationale

`FAIL` follows mechanically from mandatory rows that are `partially-met`, `not-met`, or `not-assessable`. Green focused tests do not override the v2-only “legacy” fixture, stale/underinclusive runtime binding, missing exact-head platform evidence, workspace gate failure, or unhandled Bedrock EOF residue.

Test impact: `none` (audit artifacts only).
