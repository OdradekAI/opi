# Phase 17 Audit

**Audit head**: `96f7d161045c94113ec9f02f5ad3ff4c8121cea5`
**Reviewer/model**: GLM-5.3 (glm-5.3[1M] via Claude Code harness; static verification fan-out ran 14 GLM-5.3 subagents)
**Independence**: fresh-context-same-family - this reviewer authored none of the implementation commits (all under one git identity, agent-driven per the repo workflow), is a fresh context with no remediation-session memory, but the same model family produced the prior glm5.3 audits at a680c5d and 136c380, so same-family contribution to implementation cannot be excluded
**Run ID**: 20260822t182354z
**Contamination**: endpoint sealed clean (no staged/unstaged/untracked paths at start). Three events recorded:
1. A concurrent Codex audit session (run-id 20260823t180722z at the same head per its untracked artifacts `docs/snapshots/phase17/audit.codex.96f7d16.20260822t180722z.*`, plus a stray root `audit.codex.md`) ran in parallel; its files are not this audit's and were not touched.
2. The isolated checkout `D:/Luiz/Odradek/opi-audit-96f7d16` was deleted by an external actor after ALL gate and verification evidence had been collected (subagent transcripts contain no deletion commands); later read-only confirmations used the live worktree re-verified clean at the identical SHA.
3. `origin/main` sits at 136c380; audit_head is 2 unpushed commits ahead, differing only in `.agents/skills/**`, `docs/snapshots/phase17/**`, and `scripts/*.py` (verified by `git diff --name-status 136c380..96f7d16`; no `crates/`, `Cargo.*`, or `.github/` files), so the fully-green three-OS CI run 32484643147 at 136c380 is transferable evidence for the Rust surface.
HEAD remained `{HEAD}` throughout (re-verified after each event).
**Verdict**: PASS-WITH-FINDINGS

## Requirement Conformance

All 70 registered criteria (55 P17-* + A01-A15 from `docs/snapshots/phase17/opi-impl-state.json` `criteria_trace`, normative source `docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md`) were independently re-verified against the committed tree at the audit head. Ledger citations had drifted; every surface and test below was relocated by symbol/content and re-read. Evidence column gives the primary current-SHA surface and test.

| Requirement | Criterion source | Evidence (current SHA) | Requirement state | Finding IDs |
|---|---|---|---|---|
| P17-AUT-001 | design doc `AUT` section | execute_authorized | `met` | F-006 |
| P17-AUT-002 | design doc `AUT` section | parse_tool_call_arguments | `met` | - |
| P17-AUT-003 | design doc `AUT` section | tool_authority | `met` | F-002 |
| P17-AUT-004 | design doc `AUT` section | tool_authority | `met` | F-008 |
| P17-AUT-005 | design doc `AUT` section | execute_authorized | `met` | F-002 |
| P17-AUT-006 | design doc `AUT` section | non_exhaustive | `met` | - |
| P17-AUT-007 | design doc `AUT` section | after_tool_call | `met` | F-010 |
| P17-AUT-008 | design doc `AUT` section | active_tool_names | `met` | F-007 |
| P17-AUTH-001 | design doc `AUTH` section | crates/opi-agent/src/agent_loop.rs:1025-1153; prepare_next_turn | `met` | F-002, F-005 |
| P17-AUTH-002 | design doc `AUTH` section | see axis notes | `met` | F-002 |
| P17-AUTH-003 | design doc `AUTH` section | crates/opi-agent/src/evidence.rs:483-542; evidence_contract | `met` | - |
| P17-EVD-001 | design doc `EVD` section | crates/opi-agent/src/evidence.rs:138-187; next_sequence | `met` | - |
| P17-EVD-002 | design doc `EVD` section | emit_tool_facts | `met` | - |
| P17-EVD-003 | design doc `EVD` section | rebind_evidence_capture | `met` | F-023, F-024 |
| P17-EVD-004 | design doc `EVD` section | from_prepared | `met` | - |
| P17-EVD-005 | design doc `EVD` section | evidence_contract | `met` | - |
| P17-EVD-006 | design doc `EVD` section | set_evidence_sink | `met` | F-025 |
| P17-EVD-007 | design doc `EVD` section | crates/opi-coding-agent/src/harness.rs:2989-3005; setup_evidence_run | `met` | - |
| P17-EVD-008 | design doc `EVD` section | finalize_evidence | `met` | F-017 |
| P17-EVD-009 | design doc `EVD` section | crates/opi-coding-agent/src/tool_authority.rs:428-438; evidence_incomplete | `met` | - |
| P17-EVD-010 | design doc `EVD` section | crates/opi-coding-agent/src/evidence.rs:65; phase17_api_audit | `met` | - |
| P17-EVD-011 | design doc `EVD` section | crates/opi-agent/src/evidence.rs:2254-2278; finalize_artifact | `met` | F-010 |
| P17-FAL-001 | design doc `FAL` section | provider_collection | `met` | F-009, F-002 |
| P17-FAL-002 | design doc `FAL` section | phase17_failure_precedence_stops_before_later_boundaries | `met` | F-011 |
| P17-FAL-003 | design doc `FAL` section | phase17_cancellation_and_evidence_failure_are_not_converted_to_success | `met` | F-022 |
| P17-FAL-004 | design doc `FAL` section | from_untrusted | `met` | - |
| P17-MIG-001 | design doc `MIG` section | crates/opi-coding-agent/src/harness.rs:2320-2394; apply_recorded_model | `met` | F-029 |
| P17-MIG-002 | design doc `MIG` section | normalize_recorded_route | `met` | - |
| P17-MIG-003 | design doc `MIG` section | crates/opi-coding-agent/src/evidence.rs:65-441; phase17_trace_cli_writes_evidence_files | `met` | F-028 |
| P17-MIG-004 | design doc `MIG` section | phase17_legacy_trace_file_blocks_evidence_setup_and_stays_byte_identical | `met` | - |
| P17-MIG-005 | design doc `MIG` section | phase17_all_public_product_modes_share_runtime_semantics | `met` | F-002, F-025 |
| P17-MIG-006 | design doc `MIG` section | crates/opi-agent/src/hooks.rs:26-32; phase17_removed_interfaces_are_absent_from_production_source | `met` | - |
| P17-NXT-001 | design doc `NXT` section | model_selection | `met` | F-018 |
| P17-NXT-002 | design doc `NXT` section | validate_next_turn_candidate | `met` | - |
| P17-NXT-003 | design doc `NXT` section | phase8_hook_contract_order | `met` | - |
| P17-NXT-004 | design doc `NXT` section | pop_follow_up | `met` | F-016 |
| P17-NXT-005 | design doc `NXT` section | execute_compaction | `met` | - |
| P17-NXT-006 | design doc `NXT` section | model_selection | `met` | - |
| P17-OUT-001 | design doc `OUT` section | crates/opi-agent/src/agent.rs:547; model_selection | `met` | - |
| P17-OUT-002 | design doc `OUT` section | crates/opi-agent/src/agent_loop.rs:1028-1033; validate_next_turn_candidate | `met` | - |
| P17-OUT-003 | design doc `OUT` section | crates/opi-agent/src/agent_loop.rs:1703-1895; before_tool_call | `met` | - |
| P17-OUT-004 | design doc `OUT` section | crates/opi-agent/src/evidence.rs:1806-1833; validate_observation | `met` | F-015, F-024 |
| P17-PLT-001 | design doc `PLT` section | crates/opi-agent/src/authority.rs:333; phase17_ci_matrix_selects_same_acceptance_on_three_platforms | `met` | F-003, F-013 |
| P17-PLT-002 | design doc `PLT` section | crates/opi-ai/src/test_support.rs:123-128; phase17_tests_are_hermetic_no_network_no_paid_providers | `met` | F-021 |
| P17-PLT-003 | design doc `PLT` section | crates/opi-coding-agent/tests/phase17_api_audit.rs:2641-2654; phase17_documentation_claims_no_os_sandbox | `met` | - |
| P17-PRV-001 | design doc `PRV` section | crates/opi-agent/src/agent.rs:547; model_selection | `met` | - |
| P17-PRV-002 | design doc `PRV` section | parse_model_spec | `met` | - |
| P17-PRV-003 | design doc `PRV` section | provider_collection | `met` | - |
| P17-PRV-004 | design doc `PRV` section | provider_collection | `met` | - |
| P17-PRV-005 | design doc `PRV` section | from_reported_provider_model | `met` | F-002 |
| P17-PRV-006 | design doc `PRV` section | crates/opi-ai/src/provider.rs:22-75; stream_prepared | `met` | F-002 |
| P17-RBK-001 | design doc `RBK` section | crates/opi-coding-agent/tests/phase17_failure_rollback.rs:561-601; phase17_failure_precedence_stops_before_later_boundaries | `met` | F-004 |
| P17-RBK-002 | design doc `RBK` section | crates/opi-ai/src/provider.rs:22-75; stream_prepared | `met` | - |
| P17-RBK-003 | design doc `RBK` section | crates/opi-coding-agent/tests/phase17_failure_rollback.rs:1868-1952; phase17_rollback_preserves_session_and_evidence_bytes | `met` | F-020 |
| P17-RBK-004 | design doc `RBK` section | crates/opi-coding-agent/tests/phase17_failure_rollback.rs:1960-2045; phase17_rollback_does_not_widen_user_policy | `met` | - |
| P17-A01 | design doc `Acceptance` section | phase17_harness_switches_providers_with_matching_route_evidence | `met` | - |
| P17-A02 | design doc `Acceptance` section | phase17_route_and_auth_failures_do_not_dispatch_model_http | `met` | - |
| P17-A03 | design doc `Acceptance` section | prepare_call_resolves_auth_once_across_retries | `met` | F-019 |
| P17-A04 | design doc `Acceptance` section | phase17_stop_observes_complete_next_turn_state | `met` | - |
| P17-A05 | design doc `Acceptance` section | phase17_failed_prepare_preserves_state_and_skips_later_boundaries | `met` | F-002 |
| P17-A06 | design doc `Acceptance` section | phase17_model_content_cannot_expand_effective_policy | `met` | - |
| P17-A07 | design doc `Acceptance` section | phase17_untrusted_sources_cannot_forge_registration_or_grants | `met` | F-006, F-008 |
| P17-A08 | design doc `Acceptance` section | phase17_expired_or_failed_authority_is_fail_closed | `met` | - |
| P17-A09 | design doc `Acceptance` section | crates/opi-coding-agent/tests/phase17_product_evidence.rs:2179; phase17_product_evidence | `met` | - |
| P17-A10 | design doc `Acceptance` section | crates/opi-agent/src/diagnostic.rs:485-514; phase17_canaries_stop_before_sink_file_and_manifest | `met` | - |
| P17-A11 | design doc `Acceptance` section | crates/opi-agent/src/evidence.rs:2526-2531; phase17_product_evidence | `met` | - |
| P17-A12 | design doc `Acceptance` section | crates/opi-agent/tests/tool_authority.rs:471; tool_authority | `met` | - |
| P17-A13 | design doc `Acceptance` section | phase17_legacy_migration | `met` | - |
| P17-A14 | design doc `Acceptance` section | phase17_cross_mode | `met` | - |
| P17-A15 | design doc `Acceptance` section | phase17_api_audit | `met` | F-003 |

## Standards Review

Workspace dependency direction (`met`): all crate manifests re-read; `opi-agent`/`opi-ai` depend only on opi-ai and neutral libraries; internal deps via workspace declarations; no new crate. Error types (`met`): thiserror throughout library boundaries; anyhow unused everywhere - see F-012. Public contract consistency (`met` with F-030): rustdoc on the new seams states contracts/invariants. Documentation lockstep (`partially-met` -> F-001): README/README.zh and crate README pairs cover `provider:model` routing and `--trace` evidence capture symmetrically; CHANGELOG `[Unreleased]` covers all five breaking removals - but one bullet claims a removal that did not happen (F-001). No `unsafe` on phase surfaces (`met`). Sprint-tag comments remain on three module docs (F-014).

## Spec Review

Every registered requirement re-traced to current code and tests (table above). The two registered specification revisions inside the window are legitimate P17-AUTH-003 routes: fe58c38 widened P17-EVD-009/P17-AUT-008 wording in the design doc (explicit commit), and docs/opi-spec.md INV-008's DirectRuntimeInput-or-ActiveSnapshot binding is implemented fail-closed. Residual doc drift: the design doc still names the removed `SDK TraceConfig` type (F-026). No parent gate was lowered; opi-spec.md edits in-window strengthened INV-008.

## Security, Invariants, Integration, Test Quality, and Residuals

- **Security/authority**: the trusted chain (registry -> hook deny -> registered-schema -> authorize -> freshness verify -> execute) re-verified in code and counter-tests; fail-closed postures hold on every probed boundary. Latent hazard: name-based builtin capability assignment on a pub registration API with the origin-carrying alternative seam unused (F-006, no live path). Canary/redaction matrix verified across sink, file adapter, diagnostics, JSON/RPC surfaces.

- **Invariants**: atomic NextTurnState transition, stop-after-apply ordering, queue gating, INV-008 binding all behaviorally pinned. Noted: ActiveSnapshot unconditionally rejected today (F-023, correct fail-closed now); streaming-proxy overflow drop is pre-existing and outside the phase boundaries (F-022); session torn-tail edge (F-029).

- **Integration**: cross-mode semantics consistent with the two documented asymmetries (RPC always-on recorder F-025; interactive binary wiring per cross-mode test header). Legacy migration/byte-preservation verified with current-SHA line numbers.

- **Test quality**: anti-vacuity re-checked on every acceptance-cited test; weak or structural-only cases: F-007, F-008, F-009, F-010, F-011, F-015, F-016, F-018, F-019, F-020, F-021, F-027, F-028; one environmental flake recorded (F-013).

- **Residuals**: ledger evidence-integrity cluster (F-002, F-003, F-004, F-005); unused/dead surface cluster (F-006, F-012, F-031, F-032).

## Minimum-change Conformance

| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |
|---|---|---|---|---|---|---|---|
| 17.1 | design 'Dispatchable provider collection' | wraps existing ProviderRegistry | opi-ai | prepare_call/PreparedProviderCall/start_attempt | agent_loop.rs prepare once/turn, retries reuse | no router trait/second registry | conforming |
| 17.2 | design 'Atomic next-turn state' | deepens Agent state/cancellation/hooks | opi-agent | NextTurnState + one validated replacement | replace_state sole idle replacement; loop consumes | no patch protocol; run-bound facts unchanged | conforming |
| 17.3 | design 'Product-neutral evidence seam' | additive over existing vocabulary | opi-agent | EvidenceSink lifecycle + ids/health/binding | consumed downstream by 17.6/17.7 | no-op/in-memory only; no exporter/file in core | conforming |
| 17.4 | design 'Trusted tool authorization' | mechanism core / policy product | opi-agent + opi-coding-agent | RegisteredTool/ToolRegistry/ToolAuthorizer | harness assembly + loop chain | no policy engine/permission language | conforming |
| 17.5 | design provider assembly cutover | reuses provider/credential/OAuth construction | opi-coding-agent | build_harness_collection one dispatch collection | harness.rs + main.rs consumers | no aliases/eager auth/breadth | conforming |
| 17.6 | expand step over 17.3 contract | deepens Agent/loop | opi-agent | Agent::set_evidence_sink + run lifecycle | loop emit + AgentRunResult finalization | no product cutover in this task | conforming |
| 17.7 | product evidence cutover | reuses capture paths | opi-coding-agent | FileEvidenceSink implements both contracts | CLI --trace / runner / RPC / harness | no exporter/db/ActiveSnapshot fabrication | conforming |
| 17.8 | legacy session/trace migration | reuses session repo/branch logic | opi-coding-agent | read-side normalize_recorded_route + typed remediation | resume/fork/CLI seams | no rewrite; no reader/upgrades | conforming |
| 17.9 | local acceptance closure | reuses mode harnesses/CI/docs | tests/docs only | no new public API | acceptance binaries + CI matrix | no runtime source repair / OS-specific work | conforming |

Recorded `reuse_search`/`surface_necessity`/`simplification_ceiling` traces (ledger inference_notes) were each verified against current code; every new public seam has real production consumers. Residual glue: the unused origin-carrying collection seam (F-006) and dead `dir()` accessor (F-001).

## Findings

The JSONL sibling is the source of truth; entries below are navigation only.

### F-001: CHANGELOG [Unreleased] falsely claims FileEvidenceSink::dir() was removed; the dead public accessor is still present

- Axis: `standards` | Severity: **Major** | Confidence: high
- Claim: The [Unreleased] section added by commit fe58c38 states 'the fully dead `FileEvidenceSink::dir()` accessor was removed', but at audit_head 96f7d16 the pub fn dir(&self) -> &Path accessor still exists, has zero callers in the repository, and no commit after its introduction (32c79e7) ever removed it; fe58c38 itself does not touch crates/opi-coding-agent/src/evidence.rs. The changelog, the designate...
- Refutation attempted: full-history `git log -S`, repo-wide caller search, and the remediation commit's file list all fail to refute - the accessor exists, is callerless, and no removal commit exists; the only mitigations (no live exploit path; single-line fix) affect remediation urgency, not the false published claim.

### F-002: Phase-exit ledger evidence citations systematically drifted from audit_head and several claims are inaccurate in both directions

- Axis: `residuals` | Severity: **Minor** | Confidence: high
- Claim: The frozen phase-exit criteria_trace in docs/snapshots/phase17/opi-impl-state.json cites file:line locations (written at earlier SHAs) that no longer resolve to the claimed content at 96f7d16 across every criteria family, cites symbols that do not exist (AuthNotConfigured error variant; require_complete function - the actual gate is ManifestCandidate::validate at evidence.rs:2539-2565), cites an u...

### F-003: Phase-exit ledger overclaims 'three-platform CI green' at 40f2e6e; the cited run 31798070731 concluded failure overall

- Axis: `residuals` | Severity: **Minor** | Confidence: high
- Claim: The phase-exit record presents CI run 31798070731 at 40f2e6e as three-platform CI acceptance evidence, but that run's overall conclusion is failure: docs_contract failed and execution_acceptance failed on ubuntu, macos, and windows. The phase-scoped claims in the criterion rows (Phase 17 acceptance jobs green on all three OSes, workspace test green on all three, clippy green on all three) are indi...

### F-004: RBK-001 exit-audit enumeration omits one of the ten blocking thresholds

- Axis: `residuals` | Severity: **Minor** | Confidence: high
- Claim: The RBK-001 evidence claims an exit audit 'over every enumerated threshold' but enumerates only nine of the design doc's ten blocking observations, omitting 'one extension tool made visible or executable from project/package trust alone without an exact capability permission'. The omitted threshold is substantively covered at audit_head (register_builtin_tools drops non-builtin names; phase17_exte...

### F-005: Live ledger design-doc spec hash is stale after the fe58c38 specification revision; the mechanical guard covers only docs/opi-spec.md

- Axis: `residuals` | Severity: **Minor** | Confidence: high
- Claim: The live root ledger .opi-impl-state.json records spec_files_sha256 for the Phase 17 design doc as 9709183f... (the version committed at 9d992ef), but fe58c38 revised the design doc (widening P17-EVD-009 and P17-AUT-008 wording) and the file at audit_head hashes to 72f7e299...; the hash was never re-synced. tests/spec_ledger.rs pins only docs/opi-spec.md (which is correctly synced), so the stale s...

### F-006: Builtin capability assignment is name-based on a pub registration API while the origin-carrying alternative seam sits unused

- Axis: `security` | Severity: **Minor** | Confidence: high
- Claim: register_builtin_tools/register_product_tools assign ToolOrigin::Builtin and product capabilities (including command.execute) purely by tool NAME via the pub builtin_capability(name) table, so any Box<dyn Tool> named 'bash' passed to the pub API would acquire the trusted builtin registration; the origin-carrying alternative (ExtensionRegistry::collect_tools_with_origin / CollectedExtensionTool) th...

### F-007: AUT-008 lacks the consecutive-request tool-projection test named by the design's mechanical verification

- Axis: `test-quality` | Severity: **Minor** | Confidence: high
- Claim: The design names 'Consecutive-request tool-projection snapshot tests' as P17-AUT-008's mechanical verification, but every provider-facing tool-projection assertion in the suite inspects exactly one captured request (calls[0] with calls.len()==1 asserted), so a regression caching or mutating the projection between turns would not be caught by any current test. The recompute-per-request property is ...

### F-008: A07 per-source forgery vectors rest on type closure; hook/skill/retrieved/child forgery has no end-to-end probe

- Axis: `test-quality` | Severity: **Minor** | Confidence: high
- Claim: The A07 evidence proves forged extension capability denial and untrusted-name exclusion, but the design's per-source list (hook, extension, skill, retrieval/tool result, child output) is covered for the extension vector only; the remaining vectors are excluded by type closure (BeforeToolCallResult is only Continue|Deny; tool output reaches the loop only as model-visible content or tool arguments) ...

### F-009: build_finalized_manifest panics on an empty record slice instead of failing typed

- Axis: `test-quality` | Severity: **Minor** | Confidence: high
- Claim: The public opi_coding_agent::evidence::build_finalized_manifest documents that 'Validation occurs before this function returns', but terminal_correlation() calls records.first().expect(...) / records.last().expect(...) unconditionally, so a zero-record input aborts the process instead of returning the typed EvidenceError the API contract promises. Both current production call sites guard the empty...

### F-010: Retry parent-linking contract test builds a record the sink contract would reject

- Axis: `test-quality` | Severity: **Minor** | Confidence: high
- Claim: parent_call_link_correlates_retry_to_origin constructs EvidenceRecord { kind: CallKind::Retry, payload: EvidencePayload::Digest(...) } and asserts hand-built struct fields without reaching any production emission path; the production kind/payload validation (validate_kind_payload) requires (Retry, Structured) and both sinks reject the mismatch, so the exact record this test builds could never cros...

### F-011: FAL-002 pairwise precedence edges (unknown-tool-before-hook, invalid-schema-before-authorizer) are verified structurally only

- Axis: `test-quality` | Severity: **Minor** | Confidence: high
- Claim: The fixed failure-precedence order is directly tested for the later boundaries (hook deny stops before schema/authorization; denial stops before execute; failed prepare stops before stop/queues), but the two earliest pairwise edges - an unknown tool never reaching the hook, and invalid schema never reaching the authorizer - are established by code reading of execute_tool's early returns rather tha...

### F-012: anyhow is declared as a dependency but used nowhere in the workspace

- Axis: `standards` | Severity: **Minor** | Confidence: high
- Claim: crates/opi-coding-agent/Cargo.toml declares anyhow = { workspace = true } (root Cargo.toml anyhow = "1"), but no .rs file in crates/, examples/, or scripts/ references anyhow at audit_head. The repository rule confines anyhow to binaries or tests; as declared it is a dead dependency edge on a publishable crate.

### F-013: OpenAI-Codex device-code timeout test is load-sensitive and flaked once under full-load audit conditions without a retry guard

- Axis: `test-quality` | Severity: **Minor** | Confidence: medium
- Claim: openai_codex_device_code_timeout_is_15_minutes_under_paused_time uses tokio start_paused time for the 15-minute budget but performs real localhost wiremock HTTP for start/poll; under heavy host load (this audit ran a 14-agent verification fan-out concurrently) the test panicked once with Config(ProviderErrorSummary(...)) instead of Timeout, then passed on two subsequent full re-runs in isolation. ...

### F-014: Pre-existing sprint tags (S8.1, S8.2, S10) open module docs on Phase 17 surfaces

- Axis: `standards` | Severity: **Minor** | Confidence: high
- Claim: AGENTS.md forbids preserving Phase, task, PR, or review history in source comments, but three module docs on Phase 17 surfaces still open with sprint tags: opi-ai/src/provider.rs:1 '(S8.1)', opi-agent/src/hooks.rs:1 '(S8.2)', opi-coding-agent/src/runner.rs:1 '(S10)'. The tags predate Phase 17 (earlier delivery sprints); no new Phase/task tags were added by Phase 17 work (grep over phase surfaces f...

### F-015: direct_run_never_fabricates_active_snapshot is near-tautological in isolation

- Axis: `test-quality` | Severity: **Info** | Confidence: high
- Claim: The OUT-004-named test only constructs three direct bindings and asserts is_direct(); it cannot by itself detect a regression letting a direct run claim an ActiveSnapshot binding. The boundary is independently enforced behaviorally by manifest_validation_rejects_active_snapshot_binding and phase17_product_evidence.rs:2149-2169 plus the validate() binding gate, so the weak test is redundancy, not t...

### F-016: Terminal-stop event-order test wires no queues, so its no-polling assertion is trivially satisfied

- Axis: `test-quality` | Severity: **Info** | Confidence: high
- Claim: phase8_event_order_terminal_stop_runs_prepare_then_stops asserts absence of queue_update events with steering_queue: None and follow_up_queue: None, so the assertion cannot detect polling. The criterion is carried by populated-queue probes: phase8_queue_polling_order_compaction_stop_before_next_turn (queued 'must-not-deliver' never delivered, one provider call) and should_stop_prevents_queue_polli...

### F-017: Evidence recorder retains the first typed failure internally with no public accessor for its detail

- Axis: `test-quality` | Severity: **Info** | Confidence: high
- Claim: EvidenceRecorder keeps the first EvidenceError but exposes only boolean failure state; product callers cannot surface the typed failure detail without the sink's own completed_manifest being None. Observability gap only - the failure remains observable via has_failure and withheld manifest.

### F-018: No core-level in-loop test drives a prepare candidate that shrinks or rewrites conversation context

- Axis: `test-quality` | Severity: **Info** | Confidence: high
- Claim: NextTurnState replacement tests change model/inference fields and append context, but no core-level test drives a hook candidate that REMOVES or rewrites existing context messages through the in-loop transition (product compaction covers replacement at the harness seam). The complete-replacement semantics for the context field are exercised via growth and compaction paths, not shrinkage.

### F-019: A03 retry evidence asserts parent linkage but not route equality across attempts directly

- Axis: `test-quality` | Severity: **Info** | Confidence: high
- Claim: The retry scenario tests prove one resolver call across attempts, parent-call linkage, and one Provider record; the immutable-route-equality-across-attempts leg is established by the prepared-call structure (route frozen in PreparedProviderCall) and the resolved-route assertion on the single record rather than by comparing per-attempt route facts, which the record model intentionally does not dupl...

### F-020: RBK-003 byte-identity legs exercise no production reader of the evidence artifacts

- Axis: `test-quality` | Severity: **Info** | Confidence: high
- Claim: Rollback byte-preservation is proven by filesystem digest comparison; no production code path reads evidence.jsonl/manifest.json back (the design deliberately adds no reader), so 'a later runtime loading the same session' is exercised for session files but evidence-file non-mutation is write-side only. Consistent with the registered no-reader decision.

### F-021: Hermetic guard scan roots omit unnamed shared test-helper files

- Axis: `test-quality` | Severity: **Info** | Confidence: high
- Claim: phase17_tests_are_hermetic_no_network_no_paid_providers scans the phase17* test files by name; shared helper modules included via mod declarations (e.g. tests/common/mod.rs) are not named phase17* and are not scanned. Manual review of the helpers shows no network usage; the guard's coverage is narrower than its name implies.

### F-022: Streaming-proxy bounded event channel drops overflow events with only a server-side warn (pre-existing)

- Axis: `residuals` | Severity: **Info** | Confidence: high
- Claim: The only bounded queue in the core path (SDK streaming-proxy event channel) drops events on overflow with a tracing::warn and no consumer-visible overflow signal. Pre-existing behavior introduced with the proxy (82d9392, 2026-06-05), documented in module docs and pinned by bounded_event_channel_saturates_and_drops_overflow; outside Phase 17's five normative failure boundaries and not a side-effect...

### F-023: Manifest validation rejects ActiveSnapshot unconditionally, foreclosing the future Promotion Controller path until revised

- Axis: `invariants` | Severity: **Info** | Confidence: high
- Claim: validate() rejects any ActiveSnapshot binding because no Promotion Controller exists; per the current spec that is the correct fail-closed posture (direct runs must not claim Active Snapshot), but when a trusted Promotion Controller is introduced the validator will need a controlled acceptance path. Recorded so the future revision does not read today's rejection as a permanent invariant.

### F-024: Reference Product binds zero artifact references in every finalized manifest (documented vacuous satisfaction)

- Axis: `test-quality` | Severity: **Info** | Confidence: high
- Claim: Every production FinalizedManifest carries artifacts: Vec::new(); the source comment documents that no separate tool/provider artifact store exists, that the empty set satisfies the producer constraint vacuously, and that the adapter does not produce ArtifactRole/SensitivityClassification/FinalizationState facts. The artifact-reference machinery (finalize_artifact lifecycle, ArtifactReference vali...

### F-025: RPC mode always binds an in-memory evidence recorder, making complete-evidence policy unconditional there

- Axis: `integration` | Severity: **Info** | Confidence: high
- Claim: The RPC runner attaches an always-on in-memory recorder rather than forwarding --trace, so the complete-evidence policy fact is true for every RPC run regardless of explicit capture intent. The design names 'the RPC recording sink' as an explicit-capture source and the cross-mode test records the asymmetry honestly; embedders should know RPC differs from the Minimal Runtime default.

### F-026: Design doc names the removed 'SDK TraceConfig' type in the effective-policy mapping

- Axis: `spec` | Severity: **Info** | Confidence: high
- Claim: The design's complete-evidence mapping lists 'CLI --trace, SDK TraceConfig, or the RPC recording sink', but TraceConfig was removed by the 17.7 cutover (32c79e7); the current SDK embedder surface is CodingHarness::builder().evidence(EvidenceBuilderConfig). The closed semantic mapping is preserved (complete-evidence true exactly when evidence configuration is supplied); only the literal type name d...

### F-027: Three phase17 tests synchronize on bounded 10ms-poll/2s-deadline loops

- Axis: `test-quality` | Severity: **Info** | Confidence: medium
- Claim: A small number of phase17 tests synchronize async state via polling loops with a bounded deadline rather than deterministic notification; under extreme host load they can approach their deadline. None failed during this audit's full local runs; recorded as timing-sensitivity inventory.

### F-028: No spawned-binary test drives clap --trace parsing end-to-end; capture tests inject trace_path at the runner seam

- Axis: `test-quality` | Severity: **Info** | Confidence: high
- Claim: --trace activation is covered at the runner API seam (trace_path injection) and via json_mode's run_json path, but no test spawns the built binary with a literal '--trace DIR' argv through clap parsing. The flag-to-runner mapping is thin and covered structurally; noted as an end-to-end gap.

### F-029: Session byte-identity on load is conditional on a newline-terminated file (torn-tail truncation edge)

- Axis: `invariants` | Severity: **Info** | Confidence: high
- Claim: The session reader treats a final partial (non-newline-terminated) line as an interrupted tail and truncates it for reconstruction; byte-identity preservation tests write well-formed newline-terminated fixtures, so a torn-tail file loaded and re-observed would differ from its on-disk bytes at the tail only. This is the pre-existing INV-007 interrupted-tail semantics (crash recovery), not a Phase 1...

### F-030: Duplicated stray summary sentence in Provider::replace_model_catalog rustdoc

- Axis: `standards` | Severity: **Info** | Confidence: high
- Claim: The doc comment interleaves two summary sentences ('Replace this provider's effective model catalog before it is shared.' followed mid-paragraph by a stray 'Replace the effective model catalog.'), producing malformed rustdoc on a public seam.

### F-031: NoopEvidenceSink has no production construction site; the capture-disabled default is Option::None

- Axis: `residuals` | Severity: **Info** | Confidence: high
- Claim: The design says 'The no-op adapter is the default', but the runtime expresses the disabled default as evidence_sink: Option<EvidenceSink> = None; NoopEvidenceSink is constructed only by tests and name-pinned by phase17_api_audit. Semantically equivalent (no capture, no behavior change) and required by EVD-010/EVD-011 core-adapter limits; the design wording and the wiring disagree in mechanism.

### F-032: Phase-14 unstable substrate with zero production call sites (refresh_models, StaticApiKey/SecretKey)

- Axis: `residuals` | Severity: **Info** | Confidence: high
- Claim: ProviderCollection::refresh, Provider::refresh_models, and AuthDescriptor::StaticApiKey with its SecretKey have no production callers (opi-ai internals and tests only). Introduced in Phase 14 as documented 'unstable 0.x extension substrate' with the no-production-trigger admission in provider.rs docs; pre-existing, outside Phase 17 scope, recorded for a future slim pass.

## Verification Commands

| Command | Result | Obligation/finding |
|---|---|---|
| `python scripts/opi-doc-check.py` (isolated checkout) | PASS | AUTH-002, docs contract |
| `cargo fmt --all --check` | PASS | workspace gate |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | workspace gate |
| `cargo test --workspace --all-targets --no-fail-fast` | PASS (207 suites ok; session_cli 44/44 and shell_completions 8/8 after mirroring `opi.exe` into `target/debug` - documented external-target-dir workaround; 1 non-reproducing oauth_auth timing flake, F-013) | P17-PLT-001 local leg |
| `cargo test --workspace --doc` | PASS (exit 0) | doctest gate |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | PASS (exit 0) | doc gate |
| `cargo test -p opi-coding-agent --features execution-backend-test-fixture --test execution_product/protocol_host/runtime` (+ mock build) | PASS (23+56+16 passed, 1 ignored) | CI-only suite invisible to --all-targets |
| `cargo test -p opi-coding-agent --test phase17_cross_mode/failure_rollback/api_audit` | PASS (7+19+22) | P17-A14/A15, PLT |
| static fan-out: 14 read-only GLM-5.3 subagents over the isolated checkout (criteria families + axes), journal-preserved | PASS (70/70 met, 0 not-assessable) | all criteria |
| `gh run view 31798070731 / 32484643147` | run 40f2e6e overall FAILURE (phase-matrix jobs green); run 136c380 full SUCCESS | F-003, PLT-001 transferability |

## Verdict Rationale

Every one of the 70 mandatory registered requirements is `met` at audit_head 96f7d161045c94113ec9f02f5ad3ff4c8121cea5 (0 partially-met, 0 not-met, 0 not-assessable), with all six repository gates plus the feature-gated and phase-acceptance suites green locally on Windows and three-OS CI green at the Rust-identical pushed SHA 136c380. Actionable non-conformance findings remain - most materially F-001 (CHANGELOG asserts a public-API removal that did not happen) - so per the requirement-state rule the verdict is **PASS-WITH-FINDINGS**, not PASS. No risk-threshold observation from the design doc's blocking list exists at this head.

## Prior-Audit Lineage (post-seal annotation)

Composed only after the requirement matrix and findings above were sealed and validated;
it annotates lineage and coverage and does not add, remove, or rewrite any sealed record.

- **Naming-contract transition.** `docs/snapshots/phase17/audit.glm5.3.md` and
  `audit.codex.md` were overwritten in place by each successive run through 96f7d16
  (`git diff 136c380..96f7d16` rewrites the glm5.3 report wholesale, a680c5d edition ->
  136c380 edition). The finding contract introduced in those same commits now mandates
  immutable `audit.<model>.<head>.<run-id>` names because in-place reuse mutates the
  stable `(source_path, id)` key. This run is the first glm5.3 audit under that
  contract; prior editions remain recoverable from git history.
- **Open prior Majors at the same Rust surface.** No Rust, Cargo, or CI-workflow file
  changed between 136c380 and this audit head, so the prior glm5.3 audit's two Majors
  (edition dated 2026-08-22 at 136c380) are candidates for recurrence by construction:
  - *Bedrock event-stream parser silently discards CRC-invalid frames (prior 4.1,
    correctness/invariants, labeled pre-existing).* Not re-derived by this run: it sits
    in the provider wire adapter, outside the 70 registered criteria families this
    audit's fan-out covered. Recorded as a coverage annotation; it should ride the
    existing remediation queue unchanged.
  - *A07 registration-forgery scenario unexercised at a production call site; the
    name-based builtin filter unguarded (prior 6.1, test-quality).* This run re-derived
    the same underlying facts and sealed them as F-006 (latent name-based capability
    hazard, no live path, sole production caller verified) and F-008 (per-source forgery
    probes rest on type closure) at Minor severity, and judged P17-A07 `met` on the
    strength of the real-authorizer denial tests. The severity disagreement with the
    prior edition is recorded honestly rather than reconciled; the difference turns on
    whether an unexploitable latent API hazard plus missing defense-in-depth probes
    constitutes a critical-path test gap (prior: yes; this run: no).
- **Remediation integrity.** F-001 (false `FileEvidenceSink::dir()` removal claim in
  CHANGELOG `[Unreleased]`, authored by fe58c38) and the stale ledger-evidence cluster
  F-002..F-005 indicate the fe58c38 remediation round claimed at least one code change
  it did not make; consumers of that changelog should treat its removal bullet as
  unverified until the accessor is actually deleted.
- **Concurrent audit.** A Codex audit at the same head (untracked artifacts
  `audit.codex.96f7d16.20260822t180722z.*`, run 16 minutes before this one) ran in
  parallel. Its conclusions were not read and are not evidence here; cross-comparison
  belongs to the remediation workflow, which may cluster both sources' findings.
