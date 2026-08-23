# Phase 17 Audit

**Audit head**: `528989279e9be308abc963ec22f377ee47bbde47`
**Reviewer/model**: GLM-5.3 (1M context), driven by the Claude Code harness
**Independence**: fresh-context-same-family — a fresh audit context with no
involvement in implementing or remediating Phase 17, but the same model family
that produced the prior audit round and the remediation; every finding below was
re-derived from current evidence at `audit_head`, not copied from any prior
artifact.
**Run ID**: `20260823t052233z`
**Contamination**: none — staged, unstaged, and untracked inventories were all
empty at seal time; the live worktree stayed clean for the whole run and the
endpoint was re-verified before publication. All builds, tests, and
reproductions ran in an isolated checkout
(`D:\Luiz\Odradek\opi-audit-5289892-20260823`, detached at `audit_head`) with
the repository's external Cargo cache; only these two audit artifacts were
written in the original worktree.
**Verdict**: PASS-WITH-FINDINGS

## Requirement Conformance

Mandatory set: the 55 `P17-*` requirement rows plus the 15 `P17-A01..A15`
acceptance scenarios registered in
`docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md`
(the registered supplemental source; the recorded phase exit claims 70/70 over
this set). Every row is decidable from cited current evidence.

| Requirement | Criterion source | Evidence (current head) | Requirement state | Finding IDs |
|---|---|---|---|---|
| P17-AUTH-001 | design §Status and authority | Parent clauses (INV-001..008, PRIN, CTRL) traced into collection dispatch, atomic next-turn state, trusted authorization, and the evidence seam; no lowered or bypassed gate found across the inspected surfaces | `met` | — |
| P17-AUTH-002 | design §Status and authority | `scripts/opi-doc-check.py` PASS in the isolated checkout; no implementation status in the design doc or `docs/opi-spec.md` (spec contains no Phase-17 mentions; live ledger spec-hash matches the live file) | `met` | — |
| P17-AUTH-003 | design §Status and authority | No silent exception found; the strengthened INV-007 envelope semantics were implemented, and the current live spec/ledger pair is hash-synced | `met` | 01, 02 (recording gaps around that change) |
| P17-OUT-001 | design §Outcome 1 | `phase17_two_providers_dispatch_through_one_collection_without_rebuild`, `phase17_coding_harness_cross_provider_switch_dispatches_both_providers`, `phase17_harness_switches_providers_with_matching_route_evidence` | `met` | — |
| P17-OUT-002 | design §Outcome 2 | Loop: candidate built from a snapshot, `validate_next_turn_candidate`, single `mem::replace` (agent_loop.rs:1028-1067); A04/A05 tests; `Agent::replace_state` + `self.state = run.state` persistence | `met` | — |
| P17-OUT-003 | design §Outcome 3 | `preflight_tool` resolve→hook→schema→authorize→verify-stale→launch (agent_loop.rs:1703-1924) with zero-execution counters across AUT tests | `met` | — |
| P17-OUT-004 | design §Outcome 4 | `build_finalized_manifest` binds correlation/session/binding/config/route/policy/inputs/environment/usage/completeness; `phase17_complete_run_reconstructs_graph_and_rejects_missing_bindings` | `met` | — |
| P17-PRV-001 | design §Dispatchable provider collection | `ProviderCollection::prepare_call` is the sole dispatch entry (provider_collection.rs:410-479); `PreparedProviderCall::start_attempt`; source-structure + two-provider tests | `met` | — |
| P17-PRV-002 | design §Per-call route and auth preparation | Canonical `provider:model` resolution with `RequestRouteMismatch` guard; `phase17_canonical_selection_resolves_and_bare_is_ambiguous`, `phase17_bare_selection_ambiguous_across_dispatchable_routes_fails_typed`; no alias registry (api-audit token scan) | `met` | — |
| P17-PRV-003 | design §Per-call route and auth preparation | `prepare_call_resolves_auth_once_across_retries` (counting resolver, one preparation, per-attempt streams); frozen route/auth/request with at-most-one-active and credential-terminal guards | `met` | — |
| P17-PRV-004 | design §Per-call route and auth preparation | No fallback path in `prepare_call`; `phase17_auth_failure_does_not_silently_fallback`, `phase17_route_and_auth_failures_do_not_dispatch_model_http` | `met` | — |
| P17-PRV-005 | design §Per-call route and auth preparation | `PreparedRoute` + `auth_provenance()` as the only public facts; `phase17_harness_switches_providers_with_matching_route_evidence` | `met` | — |
| P17-PRV-006 | design §Dispatchable provider collection | Wire code behind the neutral `Provider`/stream/usage interfaces (`stream_prepared`); provider fixture suites; api-audit crate-boundary scan | `met` | — |
| P17-NXT-001 | design §State boundary | `NextTurnState` complete replacement; `Agent` is the durable owner (`replace_state`, post-run store at agent.rs:962) | `met` | — |
| P17-NXT-002 | design §State boundary | `validate_next_turn_candidate` before apply; error/cancel paths restore or retain prior state (agent_loop.rs:1045-1067, 1080-1096); A05 tests | `met` | — |
| P17-NXT-003 | design §Fixed ordering | `should_stop_after_turn` observes the post-`mem::replace` state (agent_loop.rs:1068-1092) | `met` | — |
| P17-NXT-004 | design §Fixed ordering | Stop returns via `finish_success!` before steering/follow-up polling (agent_loop.rs:1104-1115); queue probe tests | `met` | — |
| P17-NXT-005 | design §Fixed ordering | Product compaction through the complete-state transition (`PendingCompaction`/`begin_compaction` lifecycle); compaction evidence tests | `met` | — |
| P17-NXT-006 | design §Fixed ordering | Candidate validation resolves the route against the collection before apply; cross-provider next-turn test (`phase17_agent_...`/A01 family) | `met` | — |
| P17-AUT-001 | design §Trusted boundary | `RegisteredTool`/`ToolRegistry` (authority.rs:83-185): immutable, duplicate-name rejection, registration-owned origin/capability; unknown tool never reaches hook | `met` | — |
| P17-AUT-002 | design §Invocation order | Final-args schema validation before authorization; exact validated value is executed (agent_loop.rs:1768-1783, `PreparedToolExecution.args`) | `met` | — |
| P17-AUT-003 | design §Effective product policy | Decision derives from immutable `EffectiveUserPolicy` digest + capability + current `EvidenceHealth` copy (tool_authority.rs:418-507); stale-allow reauthorization (agent_loop.rs:1843-1870, `authorize_and_verify`) | `met` | — |
| P17-AUT-004 | design §Effective product policy | `phase17_model_content_cannot_expand_effective_policy` (adversarial arguments, digest immutable); `phase17_untrusted_sources_cannot_forge_registration_or_grants` | `met` | — |
| P17-AUT-005 | design §Invocation order | Missing authorizer / authorizer error / denial / stale generation / invalid schema → zero `Tool::execute` (preflight paths + execution-counter tests incl. `phase17_expired_or_failed_authority_is_fail_closed`) | `met` | — |
| P17-AUT-006 | design §Invocation order | `BeforeToolCallResult::{Continue,Deny}` (rename landed); hook can deny only | `met` | — |
| P17-AUT-007 | design §Invocation order | `phase17_after_call_replace_keeps_later_authorization_unchanged` (Replace keeps later decision + policy digest identical) | `met` | — |
| P17-AUT-008 | design §Invocation order | `registry.definitions()` recomputed per provider request (agent_loop.rs:276-281); `phase17_tool_projection_is_recomputed_for_consecutive_requests` | `met` | — |
| P17-EVD-001 | design §Core vocabulary | `IdentityAllocator` mints run/turn/call identities + monotonic sequence before emission (evidence.rs); identity uniqueness/ordering tests | `met` | — |
| P17-EVD-002 | design §Core vocabulary | Provider/tool/retry/compaction records carry run/turn/call/parent correlation; `phase17_one_run_graph_includes_tool_execution_record`, `compaction_emits_correlated_evidence_record` | `met` | — |
| P17-EVD-003 | design §Resolved-execution manifest | Manifest binds session branch, exact binding variant, policy digest, artifacts; direct runs cannot claim ActiveSnapshot (`RuntimeInputBinding` constructors); harness pairs the session binding into capture (harness.rs:2996-3000); `..._rejects_missing_bindings` | `met` | — |
| P17-EVD-004 | design §Redaction boundary | `Measurement::Known/Unknown` with reason; `usage_facts` keeps unknown ≠ zero; serialization fixtures | `met` | — |
| P17-EVD-005 | design §Redaction boundary | Values classified/redacted before sink entry; `phase17_canaries_stop_before_sink_file_and_manifest` | `met` | — |
| P17-EVD-006 | design §Evidence failure | `phase17_default_harness_emits_no_evidence`; no-op sink default | `met` | — |
| P17-EVD-007 | design §Evidence failure | `setup_failure_aborts_before_provider_call`, `file_evidence_setup_failure_poisons_recorder_health` | `met` | — |
| P17-EVD-008 | design §Evidence failure | `finalization_failure_advances_health_and_preserves_execution_outcome`; emission/finalization failure → no finalized manifest (`phase17_in_flight_...` asserts `completed_manifest().is_none()`) | `met` | — |
| P17-EVD-009 | design §Evidence failure | Stale allow reauthorized once then denied (`parallel_authorization_record_failure_on_first_or_second_launches_zero_tools`: 3 records, 3 authorizer calls, 0 launches); in-flight effect retains actual outcome under the real authorizer with complete-evidence required | `met` | — |
| P17-EVD-010 | design §Core vocabulary | Production `EvidenceSink` impls in `opi-agent` are only Noop + InMemory; `FileEvidenceSink` lives in `opi-coding-agent`; `phase17_core_evidence_adapters_are_limited_to_noop_and_in_memory` | `met` | — |
| P17-EVD-011 | design §Core vocabulary | Shared lifecycle conformance; in-memory/file redaction-before-sink tests; no-op capture-disabled test | `met` | — |
| P17-FAL-001 | design §Failure/cancellation | `AgentError` variants cover route/auth/next-turn/tool-authority/evidence/execution classes (loop_types.rs:120-208); `phase17_failure_boundaries_expose_distinguishable_typed_classes` | `met` | — |
| P17-FAL-002 | design §Failure/cancellation | `phase17_failure_precedence_stops_before_later_boundaries`; unknown tool pre-hook, denied hook pre-schema, invalid schema pre-authorizer, denied authorizer pre-tool, invalid candidate pre-stop/queues (loop order verified in source) | `met` | — |
| P17-FAL-003 | design §Failure/cancellation | `phase17_cancellation_and_evidence_failure_are_not_converted_to_success`; PartialSideEffect/CleanupUnknown remain distinct (`retain_strongest_terminal_error`) | `met` | — |
| P17-FAL-004 | design §Failure/cancellation | `ProviderErrorSummary` closed constructors; secret-canary matrix across text/JSON/RPC/trace/manifest (`phase17_canaries_stop_before_sink_file_and_manifest`, `exception_frame_does_not_expose_upstream_message`) | `met` | — |
| P17-MIG-001 | design §Compatibility | v1 fixtures remain readable (`make_legacy_header` read tests; `list_sessions`) and nothing rewrites them (v1 is read-only; writers refuse it); byte-identity assertions pass — against v2 fixtures, see finding 03 | `met` | 01, 03 |
| P17-MIG-002 | design §Compatibility | Bare-model normalization unique/ambiguous/missing tests through the real harness resume; typed remediation diagnostics; no dispatch on failure | `met` | 03 |
| P17-MIG-003 | design §Compatibility | `--trace` capture in interactive/non-interactive/JSON/RPC (cross-mode suite with durable evidence) | `met` | — |
| P17-MIG-004 | design §Compatibility | `phase17_legacy_trace_file_blocks_evidence_setup_and_stays_byte_identical`; `phase17_legacy_sessions_and_opaque_traces_are_byte_preserved`; no trace reader (api-audit token scan) | `met` | — |
| P17-MIG-005 | design §Compatibility | `phase17_cross_mode` drives the same fixture through TUI-loop, harness, print, JSON, and RPC with equivalent route/authority/cancellation/evidence semantics | `met` | — |
| P17-MIG-006 | design §Compatibility | `phase17_removed_interfaces_are_absent_from_production_source` (TraceSink, AgentHarness, SharedProvider, `Allow` hook naming, metadata-only providers) | `met` | — |
| P17-PLT-001 | design §Platform scope | Same state/failure contracts per platform; 3-OS CI matrix selects identical acceptance (`phase17_ci_matrix_selects_same_acceptance_on_three_platforms`); local full gates green at this head; remediation delta contains no cfg-gated code — but no CI run exists for the unpushed head itself | `met` (limitation recorded) | 05 |
| P17-PLT-002 | design §Platform scope | `phase17_tests_are_hermetic_no_network_no_paid_providers` (+ guard self-tests) | `met` | — |
| P17-PLT-003 | design §Platform scope | `phase17_documentation_claims_no_os_sandbox` | `met` | — |
| P17-RBK-001 | design §Risk thresholds | Exit-audit pass over every listed threshold at current head: no misroute, silent fallback, unauthorized execution, secret crossing, fabricated snapshot, rewritten legacy session, or duplicated mechanism found | `met` | — |
| P17-RBK-002 | design §Risk thresholds | Single coherent runtime path (removed-interface scan); rollback = revert of the change set | `met` | — |
| P17-RBK-003 | design §Risk thresholds | `phase17_rollback_preserves_session_and_evidence_bytes` | `met` | — |
| P17-RBK-004 | design §Risk thresholds | `phase17_rollback_does_not_widen_user_policy` | `met` | — |
| P17-A01 | design §Acceptance scenarios | `phase17_harness_switches_providers_with_matching_route_evidence` (+ two-provider dispatch tests) | `met` | — |
| P17-A02 | design §Acceptance scenarios | `phase17_route_and_auth_failures_do_not_dispatch_model_http` | `met` | — |
| P17-A03 | design §Acceptance scenarios | `phase17_retry_keeps_route_parent_and_terminal_evidence`, `prepare_call_resolves_auth_once_across_retries` | `met` | — |
| P17-A04 | design §Acceptance scenarios | Complete-replacement + consecutive-prompt persistence tests (A04 family; loop source verified) | `met` | — |
| P17-A05 | design §Acceptance scenarios | Failure/cancel retains all mutable fields; no stop/queue run (A05 family) | `met` | — |
| P17-A06 | design §Acceptance scenarios | `phase17_model_content_cannot_expand_effective_policy` | `met` | — |
| P17-A07 | design §Acceptance scenarios | `phase17_extension_builtin_names_cannot_acquire_product_registrations` + `phase17_untrusted_sources_cannot_forge_registration_or_grants` | `met` | — |
| P17-A08 | design §Acceptance scenarios | `phase17_expired_or_failed_authority_is_fail_closed` (visible denial/authority source, no secrets) | `met` | — |
| P17-A09 | design §Acceptance scenarios | `phase17_complete_run_reconstructs_graph_and_rejects_missing_bindings` (provider+retry+tool+compaction graph, exact binding variant) | `met` | — |
| P17-A10 | design §Acceptance scenarios | `phase17_canaries_stop_before_sink_file_and_manifest` | `met` | — |
| P17-A11 | design §Acceptance scenarios | `phase17_cancellation_and_evidence_failure_are_not_converted_to_success` + core finalization-failure tests | `met` | — |
| P17-A12 | design §Acceptance scenarios | `parallel_authorization_record_failure_on_first_or_second_launches_zero_tools`, `phase17_in_flight_effect_retains_actual_outcome_under_evidence_failure` | `met` | — |
| P17-A13 | design §Acceptance scenarios | `phase17_legacy_sessions_and_opaque_traces_are_byte_preserved` (trace leg genuine; session leg is v2 — see finding 03) | `met` | 03 |
| P17-A14 | design §Acceptance scenarios | `phase17_all_public_product_modes_share_runtime_semantics` (cross-mode suite) | `met` | — |
| P17-A15 | design §Acceptance scenarios | CI matrix selects the same hermetic acceptance on 3 platforms; last actual 3-platform run is at pushed ancestor 136c380, not at this unpushed head | `met` (limitation recorded) | 05 |

## Standards Review

Workspace gates at `audit_head` (isolated checkout, external Cargo cache):
`python scripts/opi-doc-check.py`, `cargo fmt --check --all`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace --all-targets`,
`cargo test --workspace --doc`,
`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` — all exit 0,
with one environmental exception described under Verification Commands.
Dependency direction and supply-chain posture conform: the remediation
removed `anyhow` from `opi-coding-agent` and the workspace, added `sha2` to
`opi-agent` as a `{ workspace = true }` reference, and introduced no new
crates, features, or build scripts. Comments and rustdoc describe current
contracts; the remediation also stripped two stale phase-tagged doc comments
(`(S8.1)`, `(S8.2)`, `(S10)`).

Standards findings: **01** (unrecorded on-disk-format break), **02**
(bilingual README documents the superseded session contract).

## Spec Review

Every registered P17 requirement row and acceptance scenario was traced to
current production surfaces and discriminating tests (matrix above). The
remediation under audit closed the prior cycle's provider-boundary defect
(Bedrock CRC-invalid complete frames now fail the stream closed with a typed
`StreamError`, with unit and fixture tests), added the stale-allow
reauthorization step at the tool launch boundary with exact-count tests,
tightened `build_finalized_manifest` to reject an empty evidence graph, and
implemented the INV-007 durable-entry envelope (session format v2) with a
runtime-input binding paired to the validated committed prefix. Spec
findings: the v1→v2 session semantics themselves are fail-closed and
consistent with the current normative INV-007; the defects are in recording
and test fidelity (findings 01-03), not in the mechanism.

## Security, Invariants, Integration, Test Quality, and Residuals

- **Security/authority**: authorization derives only from trusted
  registration + immutable policy + current health; forged
  capabilities/grants denied with zero execution; canaries never reach sink,
  manifest, or diagnostics; Bedrock CRC corruption now fails closed.
- **Invariants/integration**: fixed turn ordering, parallel launch boundary,
  evidence health generations, session binding pairing, and cross-mode
  equivalence all verified in source and tests at this head.
- **Test quality**: acceptance suites are discriminating (execution counters,
  byte digests, exact authorization counts, guard self-tests). One
  critical-path gap: finding **03** (legacy-fixture tests write v2).
- **Residuals**: finding **04** (dead `FileEvidenceSink::dir()` accessor).

## Minimum-change Conformance

| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |
|---|---|---|---|---|---|---|---|
| 17.1 | design §Dispatchable provider collection | registry/auth reuse | `opi-ai` | `ProviderCollection::prepare_call` | verified (`prepare_call` sole entry) | no new registry added | `conforming` |
| 17.2 | design §Atomic next-turn state | existing request types | `opi-agent` | `NextTurnState`, loop ordering | verified (agent_loop.rs:1028-1153) | AgentHarness removed, not adopted | `conforming` |
| 17.3 | design §Product-neutral evidence seam | — | `opi-agent` evidence module | lifecycle/identities/health | verified | no exporter surface | `conforming` |
| 17.4 | design §Trusted tool authorization | existing permission policy | `opi-agent` authority + product policy | `RegisteredTool`/`ToolAuthorizer` | verified (preflight + product authorizer) | policy stays product-side | `conforming` |
| 17.5 | design §Runtime ownership | provider factory | `opi-coding-agent` assembly | eager routes + `set_model_validated` | verified (provider_runtime suite) | bare-model unique-route proof | `conforming` |
| 17.6 | design §Core vocabulary | 17.3 contract | `opi-agent` loop integration | identity/health runtime | verified (evidence_runtime suite) | no-op default unchanged | `conforming` |
| 17.7 | design §Resolved-execution manifest | 17.3 + product facts | `opi-coding-agent` evidence | file adapter/finalization/redaction | verified (product_evidence suite) | capture remains opt-in | `conforming` |
| 17.8 | design §Compatibility | session_cli/coordinator | `opi-coding-agent` session seams | resume/fork/trace preservation | verified (legacy_migration suite — v2-fixture caveat, finding 03) | v1 read-only | `conforming` (drift risk recorded by 01/03) |
| 17.9 | design §Acceptance scenarios | assembled product | cross-mode/failure/rollback/api audits | CI workflow + acceptance suites | verified locally; CI at ancestor (finding 05) | platform matrix unchanged | `conforming` |

The post-exit remediation commit under audit is remediation-cycle work, not
an admitted ledger task; its surfaces were audited directly against the
registered requirements above rather than through a task trace.

## Findings

Machine records: `audit.glm-5-3.5289892.20260823t052233z.findings.jsonl`
(validated). Summary:

### phase17-5289892-01: Session format v1-to-v2 breaking change is unrecorded in CHANGELOG [Unreleased]

- Axis: `standards`; Severity: Major
- Claim/evidence/criterion: see sidecar record `phase17-5289892-01`.
- Refutation attempted: read the entire `[Unreleased]` section and grepped for
  any session/format entry (none); confirmed released 0.7.x builds shipped v1
  session resume/fork/export, so v1 files are existing user data; confirmed
  the only v2-related entry (`opi-implement` ledger schema) covers the
  implementation ledger, not user sessions. Not refuted.
- Suggested closure: record the session format v2 break under
  `[Unreleased]` Breaking Changes, including the v1 read-only consequence and
  the user remediation path, before any release is cut.

### phase17-5289892-02: opi-agent README (EN and ZH) still documents the superseded v1 additive session contract

- Axis: `standards`; Severity: Major
- Claim/evidence/criterion: see sidecar record `phase17-5289892-02`.
- Refutation attempted: checked whether the README claims remain true for the
  v1 read path (they partially do), but the section documents the storage this
  build writes, which is v2 with fail-closed unknown-required entries — the
  documented skip-and-count recovery contradicts the actual boundary. Not
  refuted.
- Suggested closure: update both README counterparts to the v2 contract
  (envelope classification, header binding, v1 read-only) in one change.

### phase17-5289892-03: Legacy-fixture acceptance tests write v2 sessions, leaving the v1 compat break untested at the product seam

- Axis: `test-quality`; Severity: Major
- Claim/evidence/criterion: see sidecar record `phase17-5289892-03`.
- Refutation attempted: searched all coding-agent tests for a v1 read-only
  expectation (none); confirmed core-layer tests do hand-write genuine v1
  headers, but only for reader semantics — the product resume/fork refusal
  path is unpinned everywhere. Not refuted.
- Suggested closure: add a product-seam test that writes a genuine v1 file
  (hand-written header, no envelope) and asserts the typed resume/fork
  refusal and byte-immutability; retarget or rename the current "legacy"
  fixtures to state they cover legacy model entries in v2 files.

### phase17-5289892-04: Dead public accessor FileEvidenceSink::dir() remains after its removal claim was retracted

- Axis: `residuals`; Severity: Minor
- Claim/evidence/criterion: see sidecar record `phase17-5289892-04`.
- Refutation attempted: workspace-wide caller search found zero uses
  (production, tests, examples). Not refuted.
- Suggested closure: remove the accessor (and record it) or give it a real
  consumer.

### phase17-5289892-05: Audit head is unpushed: the remediation delta has no three-platform CI evidence

- Axis: `integration`; Severity: Minor
- Claim/evidence/criterion: see sidecar record `phase17-5289892-05`.
- Refutation attempted: verified the unpushed runtime delta contains no
  platform-gated code and that all local gates plus feature-gated suites pass
  on Windows; the absence of a CI run for this exact tree remains. Not
  refuted.
- Suggested closure: push `main` and record the three-platform CI result for
  the remediation head, following the existing CI-closure practice.

## Verification Commands

All from the isolated checkout of `audit_head` with the external Cargo cache;
exit codes captured per step (no output-masking pipes on cargo commands).

| Command | Result | Obligation/finding |
|---|---|---|
| `python scripts/opi-doc-check.py` | PASS (rc 0) | AUTH-002, docs contract |
| `cargo fmt --check --all` | PASS (rc 0) | Standards |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (rc 0) | Standards |
| `cargo test --workspace --all-targets` | PASS with one environmental exception: `shell_completions` (8 tests) expects `<worktree>/target/debug/opi.exe`, absent under the external cache; after copying the built binary there, `cargo test -p opi-coding-agent --test shell_completions` = 8/8 PASS (rc 0). All other 176 test-result lines green; 0 code failures | All test-backed obligations |
| `cargo test --workspace --doc` | PASS (rc 0) | Docs as code |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | PASS (rc 0) | Standards |
| `cargo test -p opi-coding-agent --features execution-backend-test-fixture --test execution_backend_mock` | PASS (rc 0; fixture target) | Feature-gated suite |
| `... --test execution_product` | PASS (23) | Feature-gated suite |
| `... --test execution_protocol_host` | PASS (56, 1 ignored) | Feature-gated suite |
| `... --test execution_runtime` | PASS (16) | Feature-gated suite |
| `git rev-parse HEAD` (endpoint re-check before publication) | `528989279e9be308abc963ec22f377ee47bbde47` | Endpoint immutability |
| `python .agents/skills/_shared/scripts/validate_assurance_artifact.py findings <sidecar>` | PASS | Artifact contract |

## History Comparison (post-seal lineage annotation)

Written after the current matrix and findings above were sealed and validated;
it annotates recurrence and coverage only and does not alter any current
finding record.

Prior immutable findings at `96f7d16` (`audit.codex.96f7d16...`,
`audit.glm53.96f7d16...`) and the r1 remediation dispositions
(`remediation.96f7d16.r1.result.dispositions.jsonl`) were compared:

- **Verified closed at this head by this run's inspection**: P17-CODEX-002
  (Bedrock CRC fail-closed, D2), P17-CODEX-003 (post-Allow reauthorization,
  D3), F-009 (empty-records manifest guard, D14), F-012 (`anyhow` removed,
  D17), F-014 (sprint-tag doc comments, D19), F-007 (consecutive-request
  projection test, D12).
- **Recurrence**: finding 04 is the open residual of prior F-001/D6 — the
  false CHANGELOG removal claim was retracted, but the dead
  `FileEvidenceSink::dir()` accessor itself still exists.
- **New defects introduced by remediation**: findings 01, 02, and 03 arise
  from the P17-CODEX-001/D1 session-envelope fix (format v2): the mechanism
  is now conformant, but the break is unrecorded in the changelog, the
  bilingual README still documents the v1 contract, and the legacy-fixture
  acceptance tests no longer exercise the v1 format.
- **Environmental note**: P17-CODEX-004/D4 (workspace gate vs. resolved
  Cargo target directory) was closed for the verify-engine gate context, but
  the same class still manifests for tests that hardcode
  `<worktree>/target/debug/opi.exe` (`shell_completions`); this run hit it
  and neutralized it with the established binary-copy workaround. Not
  re-raised as a finding: pre-existing tooling/environment behavior, not a
  Phase 17 mandatory-requirement defect.
- Prior findings dispositioned no-action, returned-to-shaping, or refuted in
  r1 were not individually re-derived; none intersects a mandatory
  requirement state in the matrix above.

## Verdict Rationale

All 70 mandatory obligations are `met` at `audit_head`: 55 requirement rows
and 15 acceptance scenarios each have current, discriminating evidence
(matrix above), with two explicitly recorded verification limitations
(PLT-001/A15 CI-at-head, MIG-001 legacy-fixture fidelity) that are carried by
findings 05 and 03 rather than by unmet requirements — the observable
behaviors those requirements demand are present and verified. Five actionable
findings remain (three Major: unrecorded durable-format break, misdocumented
bilingual session contract, vacuous legacy-fixture coverage; two Minor: dead
public accessor, unpushed-head CI gap). Per the requirement-state rule this
is **PASS-WITH-FINDINGS**: no mandatory requirement is `not-met`,
`partially-met`, or `not-assessable`, and actionable non-conformance
findings remain open.
