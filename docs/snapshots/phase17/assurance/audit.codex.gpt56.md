# Phase 17 Audit

**Audit run ID**: phase17-codex-gpt56-23f5754-20260824t162222z
**Audit head**: 23f5754c6e9b1f46ea3151222fc1c1289ae5b64a
**Reviewer ID**: codex
**Model ID**: gpt56
**Reviewer identity**: Codex
**Reviewer model ID**: gpt56
**Model identity source**: operator-declared
**Independence**: fresh-context-same-family; the audit was sealed and performed without reading any prior audit, remediation conclusion, history run, or sibling member output.
**Baseline policy**: latest-committed-spec
**Verdict**: FAIL

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| .opi-impl-state.json | 9f2c0377fb5242d7a95a3abba799e5d9097960d137ec9b7464e79bfd21f52bb0 | Current committed phase pointer; its stored source hashes are historical admission metadata and differ from current committed bytes. |
| docs/snapshots/phase17/opi-impl-state.json | da4628f94e74c3d519aaaea3c3d4025d3f09a542a7d4dbec6e8021969d8824d0 | Pointed committed Phase 17 implementation and exit record. |
| docs/opi-spec.md | 0812c8c63b5f9dd1a884094d7c3713dd8e7b24efb59bf47f39edda8513b86330 | Latest committed normative parent source; it dominates stored historical hashes. |
| docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | 525d4339a78dc5e8b1fddc05f69affc6789f58d7954d6e18ccc1b11b7757635f | Current committed registered Phase 17 supplemental source. |

The audit export was sealed from committed HEAD with git archive. The four baseline digests above were recomputed from the export before production inspection. On Windows, the archive tool materialized .claude/skills as a non-directory symlink; documentation verification used a second hash-identical archive in which that committed link was reified as a directory symlink.

## Requirement Conformance

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| PRIN-001 | 3.1 Minimal and deep / PRIN-001 | PASS: workspace and all targets completed without warnings. | met | — |
| PRIN-002 | 3.1 Minimal and deep / PRIN-002 | PASS: 22 passed; public seams are intrinsic state-machine boundaries or have real product/core consumers and conformance tests. | met | — |
| PRIN-003 | 3.1 Minimal and deep / PRIN-003 | PASS: product policy types are absent from opi-ai/opi-agent and core evidence adapters remain Noop/InMemory only. | met | — |
| PRIN-004 | 3.1 Minimal and deep / PRIN-004 | PASS reproducer: an earlier Trust vote shadows a later Deny and only the first resolver runs; this is the prohibited registration-order-dependent widening. | not-met | P17-AUD-001 |
| PRIN-005 | 3.1 Minimal and deep / PRIN-005 | PASS on a hash-identical sealed archive after reifying the committed .claude/skills directory symlink for Windows. | met | — |
| CTRL-001 | 6.1 Evidence and observability / CTRL-001 | PASS: 15 passed; trusted policy, capability/scope identity, denial, grant, and actual-effect paths are typed and observable. | met | — |
| CTRL-002 | 6.1 Evidence and observability / CTRL-002 | PASS: 15 passed; trusted policy, capability/scope identity, denial, grant, and actual-effect paths are typed and observable. | met | — |
| CTRL-003 | 6.1 Evidence and observability / CTRL-003 | PASS: 15 passed; trusted policy, capability/scope identity, denial, grant, and actual-effect paths are typed and observable. | met | — |
| INV-001 | 7.1 Provider runtime / INV-001 | PASS: 54 passed; one prepared provider route owns resolution, auth, attempts, retry identity, and terminal state. | met | — |
| INV-002 | 7.1 Provider runtime / INV-002 | PASS: auth resolution is per logical call, frozen across attempts, redacted, and fails without silent fallback. | met | — |
| INV-003 | 7.2 Agent turn transition / INV-003 | PASS: 24 passed; completion, prepare, validation/apply, stop, steering, and follow-up order is discriminated. | met | — |
| INV-004 | 7.2 Agent turn transition / INV-004 | PASS: 19 passed; complete next-turn state replacement is validated atomically and stale/foreign armed runs fail before mutation. | met | — |
| INV-005 | 7.3 Tools, cancellation, and backpressure / INV-005 | PASS reproducer: short-circuit behavior is intentional and prevents a later restrictive resolver from contributing. | not-met | P17-AUD-001 |
| INV-006 | 7.3 Tools, cancellation, and backpressure / INV-006 | PASS: 19 passed; cancellation, evidence failure, partial effects, cleanup uncertainty, and rollback remain typed and observable. | met | — |
| INV-007 | 7.4 Sessions and artifacts / INV-007 | PASS: 37 passed; v2 retains RuntimeInputBinding and rejects unknown required entries and unsupported versions fail closed. | met | — |
| INV-008 | 7.4 Sessions and artifacts / INV-008 | PASS: 60 passed; direct/session/snapshot binding variants remain distinct and Active Snapshot cannot be fabricated. | met | — |
| PHASE-001 | 10. Phase Derivation and Verification / PHASE-001 | PASS on hash-identical sealed archive; normative source and progress-ledger separation verified. | met | — |
| PHASE-002 | 10. Phase Derivation and Verification / PHASE-002 | PASS on hash-identical sealed archive; normative source and progress-ledger separation verified. | met | — |
| PHASE-003 | 10. Phase Derivation and Verification / PHASE-003 | PASS on hash-identical sealed archive; normative source and progress-ledger separation verified. | met | — |
| PHASE-004 | 10. Phase Derivation and Verification / PHASE-004 | PASS on hash-identical sealed archive; normative source and progress-ledger separation verified. | met | — |
| PHASE-005 | 10. Phase Derivation and Verification / PHASE-005 | PASS on hash-identical sealed archive; normative source and progress-ledger separation verified. | met | — |
| PHASE-006 | 10. Phase Derivation and Verification / PHASE-006 | PASS on hash-identical sealed archive; normative source and progress-ledger separation verified. | met | — |
| P17-AUTH-001 | Status and authority / P17-AUTH-001 | PASS on hash-identical sealed archive; authority and progress ownership remain separated. | met | — |
| P17-AUTH-002 | Status and authority / P17-AUTH-002 | PASS on hash-identical sealed archive; authority and progress ownership remain separated. | met | — |
| P17-AUTH-003 | Status and authority / P17-AUTH-003 | PASS on hash-identical sealed archive; authority and progress ownership remain separated. | met | — |
| P17-OUT-001 | Outcome / P17-OUT-001 | PASS: 22 passed; removed surfaces are absent and provider, transition, authority, and evidence seams are singular and product-neutral. | met | — |
| P17-OUT-002 | Outcome / P17-OUT-002 | PASS: 22 passed; removed surfaces are absent and provider, transition, authority, and evidence seams are singular and product-neutral. | met | — |
| P17-OUT-003 | Outcome / P17-OUT-003 | PASS: 22 passed; removed surfaces are absent and provider, transition, authority, and evidence seams are singular and product-neutral. | met | — |
| P17-OUT-004 | Outcome / P17-OUT-004 | PASS: 22 passed; removed surfaces are absent and provider, transition, authority, and evidence seams are singular and product-neutral. | met | — |
| P17-PRV-001 | Per-call route and auth preparation / P17-PRV-001 | PASS: 54 passed; route/auth preparation, cancellation, retry, capability validation, and no-fallback behavior are discriminated. | met | — |
| P17-PRV-002 | Per-call route and auth preparation / P17-PRV-002 | PASS: 54 passed; route/auth preparation, cancellation, retry, capability validation, and no-fallback behavior are discriminated. | met | — |
| P17-PRV-003 | Per-call route and auth preparation / P17-PRV-003 | PASS: 54 passed; route/auth preparation, cancellation, retry, capability validation, and no-fallback behavior are discriminated. | met | — |
| P17-PRV-004 | Per-call route and auth preparation / P17-PRV-004 | PASS: 54 passed; route/auth preparation, cancellation, retry, capability validation, and no-fallback behavior are discriminated. | met | — |
| P17-PRV-005 | Per-call route and auth preparation / P17-PRV-005 | PASS: 54 passed; route/auth preparation, cancellation, retry, capability validation, and no-fallback behavior are discriminated. | met | — |
| P17-PRV-006 | Per-call route and auth preparation / P17-PRV-006 | PASS: 54 passed; route/auth preparation, cancellation, retry, capability validation, and no-fallback behavior are discriminated. | met | — |
| P17-NXT-001 | Fixed ordering / P17-NXT-001 | PASS: 24 passed; complete candidate state, cancellation, validation, atomic replacement, stop visibility, and queue ordering are discriminated. | met | — |
| P17-NXT-002 | Fixed ordering / P17-NXT-002 | PASS: 24 passed; complete candidate state, cancellation, validation, atomic replacement, stop visibility, and queue ordering are discriminated. | met | — |
| P17-NXT-003 | Fixed ordering / P17-NXT-003 | PASS: 24 passed; complete candidate state, cancellation, validation, atomic replacement, stop visibility, and queue ordering are discriminated. | met | — |
| P17-NXT-004 | Fixed ordering / P17-NXT-004 | PASS: 24 passed; complete candidate state, cancellation, validation, atomic replacement, stop visibility, and queue ordering are discriminated. | met | — |
| P17-NXT-005 | Fixed ordering / P17-NXT-005 | PASS: 24 passed; complete candidate state, cancellation, validation, atomic replacement, stop visibility, and queue ordering are discriminated. | met | — |
| P17-NXT-006 | Fixed ordering / P17-NXT-006 | PASS: 24 passed; complete candidate state, cancellation, validation, atomic replacement, stop visibility, and queue ordering are discriminated. | met | — |
| P17-AUT-001 | Invocation order / P17-AUT-001 | PASS: 15 passed; untrusted forgery, expiry, failure, scope, broker, model-content, and actual-effect cases fail closed. | met | — |
| P17-AUT-002 | Invocation order / P17-AUT-002 | PASS: 15 passed; untrusted forgery, expiry, failure, scope, broker, model-content, and actual-effect cases fail closed. | met | — |
| P17-AUT-003 | Invocation order / P17-AUT-003 | PASS: 15 passed; untrusted forgery, expiry, failure, scope, broker, model-content, and actual-effect cases fail closed. | met | — |
| P17-AUT-004 | Invocation order / P17-AUT-004 | PASS: 15 passed; untrusted forgery, expiry, failure, scope, broker, model-content, and actual-effect cases fail closed. | met | — |
| P17-AUT-005 | Invocation order / P17-AUT-005 | PASS: 15 passed; untrusted forgery, expiry, failure, scope, broker, model-content, and actual-effect cases fail closed. | met | — |
| P17-AUT-006 | Invocation order / P17-AUT-006 | PASS: 15 passed; untrusted forgery, expiry, failure, scope, broker, model-content, and actual-effect cases fail closed. | met | — |
| P17-AUT-007 | Invocation order / P17-AUT-007 | PASS: 15 passed; untrusted forgery, expiry, failure, scope, broker, model-content, and actual-effect cases fail closed. | met | — |
| P17-AUT-008 | Invocation order / P17-AUT-008 | PASS: 15 passed; untrusted forgery, expiry, failure, scope, broker, model-content, and actual-effect cases fail closed. | met | — |
| P17-EVD-001 | Evidence failure / P17-EVD-001 | PASS: 28 passed; durable lifecycle, route truth, graph, manifest, redaction, failure, and policy-coupled evidence are discriminated. | met | — |
| P17-EVD-002 | Evidence failure / P17-EVD-002 | PASS: 28 passed; durable lifecycle, route truth, graph, manifest, redaction, failure, and policy-coupled evidence are discriminated. | met | — |
| P17-EVD-003 | Evidence failure / P17-EVD-003 | PASS: 28 passed; durable lifecycle, route truth, graph, manifest, redaction, failure, and policy-coupled evidence are discriminated. | met | — |
| P17-EVD-004 | Evidence failure / P17-EVD-004 | PASS: 28 passed; durable lifecycle, route truth, graph, manifest, redaction, failure, and policy-coupled evidence are discriminated. | met | — |
| P17-EVD-005 | Evidence failure / P17-EVD-005 | PASS: 28 passed; durable lifecycle, route truth, graph, manifest, redaction, failure, and policy-coupled evidence are discriminated. | met | — |
| P17-EVD-006 | Evidence failure / P17-EVD-006 | PASS: 28 passed; durable lifecycle, route truth, graph, manifest, redaction, failure, and policy-coupled evidence are discriminated. | met | — |
| P17-EVD-007 | Evidence failure / P17-EVD-007 | PASS: 28 passed; durable lifecycle, route truth, graph, manifest, redaction, failure, and policy-coupled evidence are discriminated. | met | — |
| P17-EVD-008 | Evidence failure / P17-EVD-008 | PASS: 28 passed; durable lifecycle, route truth, graph, manifest, redaction, failure, and policy-coupled evidence are discriminated. | met | — |
| P17-EVD-009 | Evidence failure / P17-EVD-009 | PASS: 28 passed; durable lifecycle, route truth, graph, manifest, redaction, failure, and policy-coupled evidence are discriminated. | met | — |
| P17-EVD-010 | Evidence failure / P17-EVD-010 | PASS: 28 passed; durable lifecycle, route truth, graph, manifest, redaction, failure, and policy-coupled evidence are discriminated. | met | — |
| P17-EVD-011 | Evidence failure / P17-EVD-011 | PASS: 28 passed; durable lifecycle, route truth, graph, manifest, redaction, failure, and policy-coupled evidence are discriminated. | met | — |
| P17-FAL-001 | Failure, cancellation, and partial-result semantics / P17-FAL-001 | PASS: 19 passed; owning failures stop later boundaries while actual effects, cancellation, cleanup, and recovery remain visible. | met | — |
| P17-FAL-002 | Failure, cancellation, and partial-result semantics / P17-FAL-002 | PASS: 19 passed; owning failures stop later boundaries while actual effects, cancellation, cleanup, and recovery remain visible. | met | — |
| P17-FAL-003 | Failure, cancellation, and partial-result semantics / P17-FAL-003 | PASS: 19 passed; owning failures stop later boundaries while actual effects, cancellation, cleanup, and recovery remain visible. | met | — |
| P17-FAL-004 | Failure, cancellation, and partial-result semantics / P17-FAL-004 | PASS: 19 passed; owning failures stop later boundaries while actual effects, cancellation, cleanup, and recovery remain visible. | met | — |
| P17-MIG-001 | Compatibility and migration / P17-MIG-001 | PASS: 7 passed; legacy sessions/traces remain byte-identical and route migration succeeds uniquely or fails deterministically. | met | — |
| P17-MIG-002 | Compatibility and migration / P17-MIG-002 | PASS: 7 passed; legacy sessions/traces remain byte-identical and route migration succeeds uniquely or fails deterministically. | met | — |
| P17-MIG-003 | Compatibility and migration / P17-MIG-003 | PASS: 7 passed; legacy sessions/traces remain byte-identical and route migration succeeds uniquely or fails deterministically. | met | — |
| P17-MIG-004 | Compatibility and migration / P17-MIG-004 | PASS: 7 passed; legacy sessions/traces remain byte-identical and route migration succeeds uniquely or fails deterministically. | met | — |
| P17-MIG-005 | Compatibility and migration / P17-MIG-005 | PASS: 7 passed; legacy sessions/traces remain byte-identical and route migration succeeds uniquely or fails deterministically. | met | — |
| P17-MIG-006 | Compatibility and migration / P17-MIG-006 | PASS: 7 passed; legacy sessions/traces remain byte-identical and route migration succeeds uniquely or fails deterministically. | met | — |
| P17-PLT-001 | Platform scope / P17-PLT-001 | LIMITATION: substantial provider, agent, product, test, manifest, and spec surfaces changed after the recorded three-platform run; current-head Linux/macOS pass status is unavailable in the sealed baseline. | not-assessable | P17-AUD-002 |
| P17-PLT-002 | Platform scope / P17-PLT-002 | PASS: platform-neutral acceptance manifest/matrix is present and no new OS-specific permission implementation is detected. | met | — |
| P17-PLT-003 | Platform scope / P17-PLT-003 | PASS on hash-identical sealed archive after Windows symlink reification. | met | — |
| P17-RBK-001 | Risk thresholds and rollback / P17-RBK-001 | PASS: 19 passed; rollback preserves bytes and policy, removed interfaces stay absent, and partial failures retain ownership. | met | — |
| P17-RBK-002 | Risk thresholds and rollback / P17-RBK-002 | PASS: 19 passed; rollback preserves bytes and policy, removed interfaces stay absent, and partial failures retain ownership. | met | — |
| P17-RBK-003 | Risk thresholds and rollback / P17-RBK-003 | PASS: 19 passed; rollback preserves bytes and policy, removed interfaces stay absent, and partial failures retain ownership. | met | — |
| P17-RBK-004 | Risk thresholds and rollback / P17-RBK-004 | PASS: 19 passed; rollback preserves bytes and policy, removed interfaces stay absent, and partial failures retain ownership. | met | — |
| P17-A01 | Acceptance scenarios and verification / P17-A01 | PASS: 9 passed; provider A/B dispatch and canonical route failure paths are discriminated. | met | — |
| P17-A02 | Acceptance scenarios and verification / P17-A02 | PASS: 9 passed; unknown, ambiguous, unauthenticated, lookup-only, and route failures stop before provider HTTP. | met | — |
| P17-A03 | Acceptance scenarios and verification / P17-A03 | PASS: 28 passed; retry graph and terminal evidence remain correlated. | met | — |
| P17-A04 | Acceptance scenarios and verification / P17-A04 | PASS: stop observes the complete five-field replacement and no mixed state is exposed. | met | — |
| P17-A05 | Acceptance scenarios and verification / P17-A05 | PASS: invalid/cancelled preparation restores prior state and skips stop and queues. | met | — |
| P17-A06 | Acceptance scenarios and verification / P17-A06 | PASS: model content cannot expand effective policy; denial yields zero tool executions. | met | — |
| P17-A07 | Acceptance scenarios and verification / P17-A07 | PASS: forged registration, capability, grant, skill/result, and child-output identities do not enter trusted authority. | met | — |
| P17-A08 | Acceptance scenarios and verification / P17-A08 | PASS: 9 passed; expired, failed, missing, denied, or stale authorization yields zero executions or one bounded reauthorization. | met | — |
| P17-A09 | Acceptance scenarios and verification / P17-A09 | PASS: one run reconstructs provider, retry, tool, compaction, sequence, parent, terminal, and exact binding graph. | met | — |
| P17-A10 | Acceptance scenarios and verification / P17-A10 | PASS: prompt/argument/provider/evidence canaries are absent at public and durable boundaries. | met | — |
| P17-A11 | Acceptance scenarios and verification / P17-A11 | PASS: emission/finalization failure retains execution outcome, marks evidence incomplete, and withholds the manifest. | met | — |
| P17-A12 | Acceptance scenarios and verification / P17-A12 | PASS: 34 passed; stale allow is reauthorized and every not-yet-launched effect fails closed while actual launched effects remain recorded. | met | — |
| P17-A13 | Acceptance scenarios and verification / P17-A13 | PASS: 7 passed; legacy session and trace bytes are preserved and route normalization is unique or deterministic. | met | — |
| P17-A14 | Acceptance scenarios and verification / P17-A14 | PASS: 7 passed; route, authority, cancellation, and evidence semantics agree across public product modes. | met | — |
| P17-A15 | Acceptance scenarios and verification / P17-A15 | LIMITATION: current runtime differs materially from the historical three-platform SHA, so current-head Linux/macOS/Windows acceptance cannot be established from the sealed evidence. | not-assessable | P17-AUD-002 |

## Standards Review

The current workspace topology and dependency direction remain consistent with the documented core/product split. cargo fmt --check --all, workspace clippy with warnings denied, documentation contracts, doctests, and warning-denied rustdoc all passed on the sealed committed source. The Phase 17 API audit also passed all 22 source-manifest, removed-interface, hermeticity, and platform-matrix shape checks.

The literal cargo test --workspace --all-targets gate did not complete cleanly in the mandated archive: artifact_audit_accepts_real_declared_commit_objects was the sole failure in its 81-test binary because git archive contains no .git object database, so its declared commit could not resolve locally. A diagnostic rerun with a global --skip reached later binaries but then caused the adapter_host_mock harness-free helper to exit on the injected test argument. Focused Phase 17 suites and the remaining required documentation/build gates were run separately; this archive/tooling limitation is not used as evidence that a product requirement is met.

## Spec Review

Provider-call preparation, complete next-turn replacement, product-neutral tool authorization, typed evidence, durable session migration, failure precedence, rollback, and cross-mode behavior conform to the current Phase 17 criteria under discriminating focused tests. Removed public interfaces remain absent, and current sessions preserve the durable runtime-input binding while unknown required entries fail closed.

The current parent authority contract is not satisfied. ProjectTrustResolverRegistry is a public embedder/startup seam whose independent votes are evaluated in registration order; the first decided vote returns immediately. The committed test suite explicitly asserts that Trust followed by Deny yields Trusted and that the Deny resolver is never invoked. This violates PRIN-004 and INV-005 even though standard CLI startup constructs an empty registry.

## Security, Invariants, Integration, Test Quality, and Residuals

Security and invariants: P17-AUD-001 is a Major blocking authority-composition defect. It permits an earlier permissive resolver to shadow a later restrictive contribution, making effective project authority registration-order-dependent.

Integration: Phase 17 provider, next-turn, authority, evidence, migration, failure, rollback, and public-mode integration suites all pass on Windows from the sealed audit head. Product authority still fails closed in the Phase 17 tool path; the finding is specifically the separate public project-trust resolver boundary.

Test quality: focused suites use local mocks and fixtures and contain strong negative-path assertions. The current source also statically defines a Linux/macOS/Windows matrix. However, the only committed three-platform result is run 31798070731 at Phase-exit SHA 40f2e6ee4866f1cd44eefb952b8f40afcbb029ac; substantial runtime and test surfaces changed before audit head 23f5754c6e9b1f46ea3151222fc1c1289ae5b64a. Therefore P17-PLT-001 and P17-A15 are not-assessable, captured as advisory evidence limitation P17-AUD-002.

Residuals: stored implementation-ledger source hashes do not equal the current committed baseline bytes. Per the latest-committed-spec policy they are treated as historical mismatch evidence, not as an audit-completeness failure; this report binds the exact current baseline hashes independently.

## Minimum-change Conformance

| Task | Status | Current evidence |
|---|---|---|
| 17.1 | conforming | ProviderCollection::prepare_call is the single route/capability/auth preparation seam and has multiple real provider/product consumers. |
| 17.2 | conforming | NextTurnState owns the complete atomic candidate and validation/rollback tests discriminate mixed-state failures. |
| 17.3 | conforming | Evidence primitives are typed, provider-neutral, graph-correlated, and conformance-tested. |
| 17.4 | conforming | Phase 17 tool authority remains product-neutral in core and product policy remains in opi-coding-agent; zero-execution negative tests pass. |
| 17.5 | conforming | Reference Product provider assembly uses the collection-owned prepared route without a parallel dispatch seam. |
| 17.6 | conforming | Core evidence lifecycle has only Noop/InMemory adapters and explicit failure health. |
| 17.7 | conforming | Product durable evidence owns file publication, redaction, manifest, and failure recovery. |
| 17.8 | conforming | Legacy sessions and opaque traces are byte-preserved while canonical route migration is deterministic. |
| 17.9 | drifted | Latest parent authority composition is violated by the public project-trust registry, and the recorded three-platform run no longer covers the current audit head. |

No production code, test, manifest, or normative document was changed by this audit.

## Findings

### P17-AUD-001: Trust resolver conflicts widen authority by registration order

- Axis: security
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: PRIN-004, INV-005
- Claim: With conflicting registered project-trust resolvers, moving a Trust voter before a Deny voter produces Trusted and prevents the Deny contribution from running.
- Evidence: crates/opi-coding-agent/src/project_trust.rs:478-483 returns the first decided vote; crates/opi-coding-agent/tests/project_trust_store.rs:809-827 registers Trust then Deny, calls Deny “shadowed,” asserts Trusted, and asserts only the first resolver ran.
- Refutation attempted: Standard CLI construction uses an empty registry at crates/opi-coding-agent/src/main.rs:334-335, and the separate Phase 17 tool-authority suites pass monotonic fail-closed cases. This bounds normal CLI exposure but does not refute the public embedder/startup seam or its exact registration-order-dependent test contract.
- Suggested closure: Evaluate every applicable resolver contribution and combine decided votes monotonically, with Deny as the restrictive conflict result; add Trust/Deny order-permutation tests while retaining the empty standard-CLI registry and sealed-registration behavior.

### P17-AUD-002: Current audit head lacks three-platform execution evidence

- Axis: test-quality
- Severity: Info
- Conformance effect: advisory
- Requirement IDs: P17-PLT-001, P17-A15
- Claim: The sealed baseline contains no Linux/macOS/Windows acceptance result for the current audit head.
- Evidence: The committed Phase snapshot records run 31798070731 at older SHA 40f2e6ee4866f1cd44eefb952b8f40afcbb029ac; a name-only diff shows extensive runtime and test changes before the audit head.
- Refutation attempted: Current Windows Phase 17 focused tests and static CI-matrix checks pass, but they cannot establish current Linux/macOS behavior.
- Suggested closure: Run the repository acceptance matrix on audit head 23f5754c6e9b1f46ea3151222fc1c1289ae5b64a (or a descendant containing only the remediation) and retain immutable per-platform evidence.

## Verification Commands

| Command | Result | Requirement/finding |
|---|---|---|
| python .agents/skills/_shared/scripts/validate_assurance_artifact.py rotation docs/snapshots/phase17 | PASS | Audit admission |
| cargo test -p opi-coding-agent --test project_trust_store first_resolver_decision_wins_and_short_circuits | PASS: 1/1 reproduces short-circuit | P17-AUD-001 |
| cargo test -p opi-coding-agent --test project_trust_store explicit_embedder_resolver_precedence_and_cli_empty_registry | PASS: 1/1 reproduces Trust shadowing Deny | P17-AUD-001 |
| cargo test -p opi-ai --test provider_collection | PASS: 54/54 | Provider requirements |
| cargo test -p opi-agent --test agent_wrapper | PASS: 19/19 | Next-turn state |
| cargo test -p opi-agent --test hooks_queues | PASS: 24/24 | Transition order/rollback |
| cargo test -p opi-agent --test tool_authority | PASS: 9/9 | Tool authority |
| cargo test -p opi-agent --test evidence_contract | PASS: 60/60 | Evidence contract |
| cargo test -p opi-agent --test evidence_runtime | PASS: 34/34 | Evidence runtime/failure |
| cargo test -p opi-agent --test session_storage | PASS: 37/37 | Durable session prefix |
| cargo test -p opi-agent --test streaming_proxy | PASS: 28/28 | Bounded overflow/cancellation |
| cargo test -p opi-coding-agent --test phase17_provider_runtime | PASS: 9/9 | Product provider behavior |
| cargo test -p opi-coding-agent --test phase17_tool_authority | PASS: 15/15 | Product authority behavior |
| cargo test -p opi-coding-agent --test phase17_product_evidence | PASS: 28/28 | Product evidence |
| cargo test -p opi-coding-agent --test phase17_legacy_migration | PASS: 7/7 | Migration |
| cargo test -p opi-coding-agent --test phase17_cross_mode | PASS: 7/7 | Cross-mode integration |
| cargo test -p opi-coding-agent --test phase17_failure_rollback | PASS: 19/19 | Failures/rollback |
| cargo test -p opi-coding-agent --test phase17_api_audit | PASS: 22/22 | Architecture/API/platform shape |
| cargo test -p opi-coding-agent --test phase17_artifact_truthfulness | PASS: 1/1 | Artifact truthfulness |
| python scripts/opi-doc-check.py | PASS on hash-identical sealed archive with committed symlink correctly reified | Documentation |
| cargo fmt --check --all | PASS | Standards |
| cargo clippy --workspace --all-targets -- -D warnings | PASS | Standards |
| cargo test --workspace --all-targets | FAIL: archive lacks .git; artifact_audit_accepts_real_declared_commit_objects was 1 failed of 81 in its binary | Verification limitation |
| cargo test --workspace --doc | PASS: 12 doctests | Documentation/API |
| RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps | PASS | Documentation/API |
| git diff --name-only 40f2e6ee4866f1cd44eefb952b8f40afcbb029ac 23f5754c6e9b1f46ea3151222fc1c1289ae5b64a -- Cargo.toml Cargo.lock crates .github docs/opi-spec.md | LIMITATION: extensive post-exit changes | P17-AUD-002 |

## Verdict Rationale

The member verdict is mechanically FAIL: PRIN-004 and INV-005 are mandatory and not-met; P17-PLT-001 and P17-A15 are mandatory and not-assessable at the current audit head. The remaining 88 mandatory obligations are met. The Major authority-composition finding blocks independently of the platform-evidence limitation.

