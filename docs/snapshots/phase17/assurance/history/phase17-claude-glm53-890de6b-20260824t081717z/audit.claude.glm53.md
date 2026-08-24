# Phase 17 Audit

**Audit run ID**: `phase17-claude-glm53-890de6b-20260824t081717z`
**Audit head**: `890de6b5316f151206f3d4680954d51fc39871b0`
**Reviewer ID**: `claude`
**Model ID**: `glm53`
**Reviewer identity**: Claude
**Reviewer model ID**: `glm-5.3[1M]`
**Model identity source**: runtime-attested
**Independence**: fresh-context-same-family (fresh task context; no prior audit, remediation, history-generation, or sibling-peer content read; baseline sealed from committed ledgers before implementation inspection)
**Baseline policy**: latest-committed-spec
**Verdict**: FAIL

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| .opi-impl-state.json | de5420ab9fc38407704fb2cc1a0fc999061d1aa817706198db6910941b974133 | current committed source; live root ledger |
| docs/snapshots/phase17/opi-impl-state.json | 801ac6d69b32acaa0f6301419c397c94450fc131e765ad9895c80c7cc33dd879 | current committed source; frozen Phase 17 snapshot; its stored spec hashes are historical |
| docs/opi-spec.md | cc7f8898f60c0d8abaa667f4b49b7affc721412e75dd3a67dcde37a783e1bc4c | current committed source; matches the live root ledger's stored hash |
| docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | 72f7e2996b0fbab7fd0c56d349b9dc0d5764c8a54e4ffd6cfc48245ac2bd4917 | current committed source; **stored-hash mismatch**: both ledgers record 9709183f265a5b215896cdb82c20530fdaeaf37220a08fba007295ea3ccc9f4c for this registered source; fe58c38 (post-exit adjudicated clarification of P17-AUT-008/P17-EVD-009) changed the bytes without re-syncing the live ledger entry; used current bytes per latest-committed-spec |

## Requirement Conformance

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| P17-AUTH-001 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-AUTH-001 | phase17_removed_interfaces_are_absent_from_production_source; phase17_next_call_routes_from_applied_state_nxt006 | met | - |
| P17-AUTH-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-AUTH-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md; docs/opi-spec.md | met | - |
| P17-AUTH-003 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-AUTH-003 | direct_run_never_fabricates_active_snapshot; binding_variants_are_distinguishable_and_not_normalizable | met | - |
| P17-OUT-001 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-OUT-001 | phase17_next_call_routes_from_applied_state_nxt006; phase17_coding_harness_cross_provider_switch_dispatches_both_providers | met | - |
| P17-OUT-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-OUT-002 | phase17_failed_prepare_preserves_state_and_skips_later_boundaries; phase17_invalid_prepare_candidate_preserves_state_with_typed_error | met | - |
| P17-OUT-003 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-OUT-003 | missing_authorizer_yields_zero_executions; denying_authorizer_yields_zero_executions | met | - |
| P17-OUT-004 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-OUT-004 | evidence_capture_finalizes_direct_runtime_input_manifest; phase17_complete_run_reconstructs_graph_and_rejects_missing_bindings | met | - |
| P17-PRV-001 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-PRV-001 | phase17_two_providers_dispatch_through_one_collection_without_rebuild; phase17_removed_interfaces_are_absent_from_production_source | met | - |
| P17-PRV-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-PRV-002 | phase17_canonical_selection_resolves_and_bare_is_ambiguous; phase17_legacy_bare_model_normalizes_to_unique_dispatchable_route | met | - |
| P17-PRV-003 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-PRV-003 | prepare_call_resolves_auth_once_across_retries; prepare_call_resolves_route_and_auth_once_and_streams_via_prepared_seam | met | - |
| P17-PRV-004 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-PRV-004 | phase17_route_and_auth_failures_do_not_dispatch_model_http; phase17_auth_failure_does_not_silently_fall_back_to_another_provider | met | - |
| P17-PRV-005 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-PRV-005 | provider_turn_emits_correlated_provider_record; phase17_harness_switches_providers_with_matching_route_evidence | met | - |
| P17-PRV-006 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-PRV-006 | crates/opi-ai/src/provider.rs; crates/opi-ai/src/provider_collection.rs | met | - |
| P17-NXT-001 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-NXT-001 | phase17_agent_persists_complete_next_turn_state; phase17_removed_interfaces_are_absent_from_production_source | met | - |
| P17-NXT-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-NXT-002 | phase17_invalid_prepare_candidate_preserves_state_with_typed_error; phase17_failed_prepare_preserves_state_and_skips_later_boundaries | met | - |
| P17-NXT-003 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-NXT-003 | phase17_stop_observes_complete_next_turn_state | met | - |
| P17-NXT-004 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-NXT-004 | phase17_failed_prepare_preserves_state_and_skips_later_boundaries; cancellation_while_stop_is_pending_restores_state_and_leaves_queues_untouched | met | - |
| P17-NXT-005 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-NXT-005 | phase17_compaction_replaces_provider_visible_context; harness_compaction_emits_correlated_evidence_record | met | - |
| P17-NXT-006 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-NXT-006 | phase17_next_call_routes_from_applied_state_nxt006 | met | - |
| P17-AUT-001 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-AUT-001 | unknown_tool_yields_zero_executions; duplicate_provider_visible_name_is_rejected_at_construction | met | - |
| P17-AUT-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-AUT-002 | phase8_tool_validation_failure_contract; phase8_hook_runs_before_schema_validation | met | - |
| P17-AUT-003 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-AUT-003 | phase17_in_flight_effect_retains_actual_outcome_under_evidence_failure; stale_evidence_health_generation_yields_zero_executions | met | - |
| P17-AUT-004 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-AUT-004 | phase17_model_content_cannot_expand_effective_policy; untrusted_content_sources_cannot_forge_tool_authority | met | - |
| P17-AUT-005 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-AUT-005 | missing_authorizer_yields_zero_executions; denying_authorizer_yields_zero_executions | met | - |
| P17-AUT-006 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-AUT-006 | phase17_removed_interfaces_are_absent_from_production_source; missing_authorizer_yields_zero_executions | met | - |
| P17-AUT-007 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-AUT-007 | phase17_after_call_replace_keeps_later_authorization_unchanged | met | - |
| P17-AUT-008 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-AUT-008 | policy_excluded tool visibility tests in tool_selection/json_mode suites (30+44 green); required_evidence_failure_denies_unlaunched_tool_side_effect | met | - |
| P17-EVD-001 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-EVD-001 | run_ids_are_unique_across_allocators; call_ids_and_sequence_are_unique_and_monotonic_within_run | met | - |
| P17-EVD-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-EVD-002 | tool_turn_emits_provider_then_tool_records_in_order; retry_emits_retry_record_parented_to_provider_call | met | - |
| P17-EVD-003 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-EVD-003 | direct_run_never_fabricates_active_snapshot; binding_variants_are_distinguishable_and_not_normalizable | met | - |
| P17-EVD-004 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-EVD-004 | measured_zero_is_not_unknown; measurement_origins_are_distinct | met | - |
| P17-EVD-005 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-EVD-005 | structured_payload_is_redacted_at_the_producer_boundary; evidence_record_payload_channels_are_all_typed | met | - |
| P17-EVD-006 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-EVD-006 | phase17_default_harness_emits_no_evidence; noop_sink_is_default_and_captures_nothing | met | - |
| P17-EVD-007 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-EVD-007 | setup_failure_aborts_before_provider_call; phase17_failure_precedence_stops_before_later_boundaries | met | - |
| P17-EVD-008 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-EVD-008 | evidence_emission_failure_withholds_manifest_and_preserves_outcome; finalization_failure_withholds_manifest_through_harness | met | - |
| P17-EVD-009 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-EVD-009 | required_evidence_failure_denies_unlaunched_tool_side_effect; phase17_in_flight_effect_retains_actual_outcome_under_evidence_failure | met | - |
| P17-EVD-010 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-EVD-010 | phase17_core_evidence_adapters_are_limited_to_noop_and_in_memory; noop_and_in_memory_satisfy_one_lifecycle_conformance_contract | met | - |
| P17-EVD-011 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-EVD-011 | noop_and_in_memory_satisfy_one_lifecycle_conformance_contract; file_evidence_sink_writes_records_and_manifest | met | - |
| P17-FAL-001 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-FAL-001 | phase17_failure_boundaries_expose_distinguishable_typed_classes; collection_error_match_is_exhaustive_for_current_public_surface | met | - |
| P17-FAL-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-FAL-002 | phase17_failure_precedence_stops_before_later_boundaries; phase17_failed_prepare_preserves_state_and_skips_later_boundaries | met | - |
| P17-FAL-003 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-FAL-003 | phase17_cancellation_and_evidence_failure_are_not_converted_to_success; phase17_real_cancellation_is_typed_and_complete_in_every_public_mode | met | - |
| P17-FAL-004 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-FAL-004 | phase17_canaries_stop_before_sink_file_and_manifest; phase17_canary_is_absent_from_print_output | met | - |
| P17-MIG-001 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-MIG-001 | phase17_legacy_session_fixture_byte_identical_after_resume_normalize_fork; phase17_legacy_sessions_and_opaque_traces_are_byte_preserved | met | - |
| P17-MIG-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-MIG-002 | phase17_legacy_bare_model_normalizes_to_unique_dispatchable_route; phase17_legacy_ambiguous_route_keeps_cli_and_emits_typed_remediation | met | - |
| P17-MIG-003 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-MIG-003 | phase17_trace_cli_writes_evidence_files; file_evidence_sink_writes_records_and_manifest | met | P17-AUD-003 |
| P17-MIG-004 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-MIG-004 | phase17_legacy_trace_file_blocks_evidence_setup_and_stays_byte_identical; phase17_legacy_sessions_and_opaque_traces_are_byte_preserved | met | - |
| P17-MIG-005 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-MIG-005 | phase17_all_public_product_modes_share_runtime_semantics; phase17_real_cancellation_is_typed_and_complete_in_every_public_mode | met | P17-AUD-002 |
| P17-MIG-006 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-MIG-006 | phase17_removed_interfaces_are_absent_from_production_source | met | - |
| P17-PLT-001 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-PLT-001 | .github/workflows/ci.yml; crates (unpushed deltas 5289892/21eaacf | not-assessable | P17-AUD-001 |
| P17-PLT-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-PLT-002 | phase17_tests_are_hermetic_no_network_no_paid_providers | met | - |
| P17-PLT-003 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-PLT-003 | phase17_documentation_claims_no_os_sandbox | met | - |
| P17-RBK-001 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-RBK-001 | phase17_failure_boundaries_expose_distinguishable_typed_classes; phase17_failure_precedence_stops_before_later_boundaries | met | - |
| P17-RBK-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-RBK-002 | phase17_removed_interfaces_are_absent_from_production_source; phase17_rollback_preserves_session_and_evidence_bytes | met | - |
| P17-RBK-003 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-RBK-003 | phase17_rollback_preserves_session_and_evidence_bytes; phase17_legacy_trace_file_blocks_evidence_setup_and_stays_byte_identical | met | - |
| P17-RBK-004 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-RBK-004 | phase17_rollback_does_not_widen_user_policy | met | - |
| P17-A01 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-A01 | phase17_harness_switches_providers_with_matching_route_evidence; phase17_coding_harness_cross_provider_switch_dispatches_both_providers | met | - |
| P17-A02 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-A02 | phase17_route_and_auth_failures_do_not_dispatch_model_http; provider_refresh_error_survives_failed_post_failure_reread | met | - |
| P17-A03 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-A03 | prepare_call_resolves_auth_once_across_retries; start_attempt_allows_sequential_retry_after_terminal_and_resolves_auth_once | met | - |
| P17-A04 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-A04 | phase17_stop_observes_complete_next_turn_state | met | - |
| P17-A05 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-A05 | phase17_failed_prepare_preserves_state_and_skips_later_boundaries; phase17_invalid_prepare_candidate_preserves_state_with_typed_error | met | - |
| P17-A06 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-A06 | phase17_model_content_cannot_expand_effective_policy; phase17_command_execute_deny_is_fail_closed | met | - |
| P17-A07 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-A07 | untrusted_content_sources_cannot_forge_tool_authority; phase17_untrusted_sources_cannot_forge_registration_or_grants | met | - |
| P17-A08 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-A08 | phase17_expired_or_failed_authority_is_fail_closed; missing_authorizer_yields_zero_executions | met | - |
| P17-A09 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-A09 | phase17_one_run_graph_includes_tool_execution_record; phase17_complete_run_reconstructs_graph_and_rejects_missing_bindings | met | - |
| P17-A10 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-A10 | phase17_canaries_stop_before_sink_file_and_manifest; phase17_canary_is_absent_from_print_output | met | - |
| P17-A11 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-A11 | evidence_emission_failure_withholds_manifest_and_preserves_outcome; finalization_failure_withholds_manifest_through_harness | met | - |
| P17-A12 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-A12 | required_evidence_failure_denies_unlaunched_tool_side_effect; harness_complete_evidence_mapping_denies_unlaunched_tool | met | - |
| P17-A13 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-A13 | phase17_legacy_sessions_and_opaque_traces_are_byte_preserved; phase17_legacy_trace_file_blocks_evidence_setup_and_stays_byte_identical | met | - |
| P17-A14 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-A14 | phase17_all_public_product_modes_share_runtime_semantics; phase17_real_cancellation_is_typed_and_complete_in_every_public_mode | met | - |
| P17-A15 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md# P17-A15 | phase17_ci_matrix_selects_same_acceptance_on_three_platforms; phase17_platform_contract_is_platform_neutral | not-assessable | P17-AUD-001 |

## Standards Review

Verified at audit_head 890de6b in a sealed `git archive` export: single provider dispatch seam (`stream_prepared` is the only stream method; no `fn stream(` in opi-ai/src); no alias registry (grep); `opi-agent` depends only on `opi-ai` plus generic libraries; workspace members unchanged (six crates); CHANGELOG `[Unreleased]` documents the post-exit breaking remediations (session contracts, UUIDv7 RunId, `abandon_run` split, closed `ProviderErrorSummary` construction); `python scripts/opi-doc-check.py` passes in the export. The removed-interface scan (phase17_api_audit, 15/15 green) proves SharedProvider, AgentLoopTurnUpdate, AgentHarness, HarnessRuntimeConfig, BeforeToolCallResult::Allow, MetadataProvider, TraceSink, and TraceReader are absent from all production source and product policy symbols stay out of Agent Core. Bookkeeping defect (report-level, no owning P17 requirement): the live root ledger's `spec_files_sha256` entry for the registered Phase 17 design spec is stale relative to its committed bytes (see Baseline Sources); only `docs/opi-spec.md`'s live-ledger entry is pinned by `tests/spec_ledger.rs`, so this drift is unpinned and defeats drift detection for that source.

## Spec Review

All 70 registered criteria (55 `P17-*` + 15 `P17-A*`) were sealed before implementation inspection and individually re-derived at audit_head. Mechanism requirements verified against current source: turn-end ordering (agent_loop.rs:1028-1152: prepare -> validate -> atomic apply -> stop -> queues; cancellation restores prior state), tool authority chain (agent_loop.rs:1723-1923: resolve -> hook deny-or-Continue -> registered-schema validation -> authorize_and_verify with registration/capability/generation freshness and one stale reauthorization -> only a current AllowFresh launches), collection-owned preparation (provider_collection.rs:410-479 frozen route/auth once per logical call; start_attempt one-active-attempt and credential-terminal guards), evidence contract (identities, EvidenceHealth, DirectRuntimeInput-or-ActiveSnapshot binding, strict FinalizedManifest, producer-boundary RedactedValue), product policy binding (EffectiveUserPolicy digest folds complete_evidence_required; ProductToolAuthorizer denies unhealthy when required; FileEvidenceSink lifecycle fail-closed), and legacy session normalization (unique-route-only with typed ambiguous/missing remediation). Two mandatory criteria are not-assessable at this head: P17-PLT-001 and P17-A15 (P17-AUD-001).

## Security, Invariants, Integration, Test Quality, and Residuals

Security/authority: zero-execution matrix green (missing/denying/stale authorizer, unknown tool, invalid schema, expired/denied policy, evidence-incomplete); forged registration/capability/grant tests green; canary redaction verified across sink, file adapter, diagnostics, print/JSON/NDJSON/RPC outputs. Invariants: cancellation, timeout, queue, partial-side-effect, and cleanup-unknown outcomes stay typed and non-success (failure_rollback 9/9, evidence_runtime 34/34 incl. in-band stream-Error terminal). Integration: cross-mode equivalence green (28 tests) with recorded honest asymmetries; session resume/fork/branch and legacy byte-preservation green (7+31+44 tests). Test quality: anti-vacuity spot checks passed (A09 one-run four-kind graph over a real harness prompt with a real `read` execution; api_audit scans enumerate >100 real source files; removed-symbol scan is comment-aware). One parallel-load timing flake recorded as P17-AUD-002. Residuals: trace-named RPC seam (P17-AUD-003); manifest.artifacts remains a digest-only constraint with no emitter (documented 17.7 residual, unchanged). Audit-infrastructure observation (report-level): assurance artifacts are committed as LF blobs under `core.autocrlf=true` with only `*.sh` pinned in `.gitattributes`; git itself warned at commit 890de6b that checkout will materialize CRLF for all five assurance files, and digest validation is over exact on-disk bytes, so a fresh Windows checkout/clone of this repository would fail `audit-set` validation until the paths are pinned (e.g. `docs/snapshots/**/assurance/** text eol=lf` or `-text`).

## Minimum-change Conformance

Status: conforming. All nine admitted tasks were inspected against their recorded `reuse_search`/`surface_necessity`/`simplification_ceiling` at current code: the removed surfaces stay removed with no alias, feature flag, or shim (api_audit token scan); `ProviderCollection`/`PreparedProviderCall` remain the single dispatch surface with no router/retry-factory trait or second registry; NextTurnState keeps one complete value with one validated replacement (`Agent::replace_state`); no new crate or workspace edge was introduced; product policy remains outside Agent Core. Post-exit remediation commits (fe58c38, a680c5d, 5289892, 21eaacf) stayed inside the recorded task-owned path families (session contracts, provider auth, bedrock event stream, harness) and are changelogged.

## Findings

### P17-AUD-001: Current-head three-platform CI evidence is absent

- Axis: invariants
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P17-PLT-001, P17-A15
- Claim: At audit_head 890de6b no CI run covers the current crate content, so the three-platform acceptance matrix that mechanically verifies P17-PLT-001 and P17-A15 has not executed on the audited implementation.
- Evidence: GitHub Actions (OdradekAI/opi) - gh run list: last completed run is success at head 136c380 (run 32484643147, 2026-08-21); no runs exist for the six later commits.; git log origin/main..HEAD - 6 unpushed commits; 5289892 and 21eaacf change crate source: opi-agent/src/session.rs (+285/+71), opi-agent/src/agent_loop.rs (+45), opi-ai/src/bedrock/event_stream.rs (+88), opi-coding-agent/src/harness.rs (+394), session_coordinator.rs, session_cli.rs, rpc.rs.; docs/snapshots/phase17/assurance/audit.claude.glm53.requirements.jsonl - P17-PLT-001 and P17-A15 are not-assessable: current evidence for platform parity of the unpushed deltas does not exist.
- Refutation attempted: searched all GitHub Actions runs for any run covering content after 136c380 (none exist); confirmed the phase17_acceptance job definition is present in ci.yml:92-108 at audit_head so the gap is execution, not definition; local Windows batteries are green in the sealed export (19+60+34+24+2+9+17 / 12+54 / 22+7+19+7+28+9+15 / 9+31+15+44+61+30 / 80 tests, 0 failures serial), which evidences Windows only and cannot substitute Linux/macOS parity for the unpushed bedrock event-stream and session file-IO deltas (applies to P17-AUD-001); the rpc timeout (P17-AUD-002) was re-run isolated and serially and passed, refuting a deterministic defect; the naming residual (P17-AUD-003) is a documented 17.7 deferral with no behavioral effect.
- Suggested closure: push the six unpushed commits and let the three-platform phase17_acceptance + CI matrix run green at the new head, then re-audit P17-PLT-001/P17-A15 against that run evidence (P17-AUD-001); serialize or widen the RPC wait budget for the thinking-level mode test (P17-AUD-002); rename the trace-named RPC seam when that surface next changes (P17-AUD-003).

### P17-AUD-002: RPC thinking-level test times out under concurrent test load

- Axis: test-quality
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: P17-MIG-005
- Claim: rpc_set_thinking_level_off_medium_high_change_runtime_config waits on RPC output with a budget that is exceeded when several test binaries run concurrently, so the RPC-mode leg of cross-mode verification is not deterministic under load.
- Evidence: crates/opi-coding-agent/tests/rpc_jsonl.rs:3050 - Panicked with 'timed out waiting for RPC output: Elapsed(())' while six other test binaries ran concurrently (79 passed, 1 failed).; sealed export @890de6b - Isolated rerun PASS in 0.20s; full rpc_jsonl serial rerun PASS 80/80 in 3.10s.
- Refutation attempted: searched all GitHub Actions runs for any run covering content after 136c380 (none exist); confirmed the phase17_acceptance job definition is present in ci.yml:92-108 at audit_head so the gap is execution, not definition; local Windows batteries are green in the sealed export (19+60+34+24+2+9+17 / 12+54 / 22+7+19+7+28+9+15 / 9+31+15+44+61+30 / 80 tests, 0 failures serial), which evidences Windows only and cannot substitute Linux/macOS parity for the unpushed bedrock event-stream and session file-IO deltas (applies to P17-AUD-001); the rpc timeout (P17-AUD-002) was re-run isolated and serially and passed, refuting a deterministic defect; the naming residual (P17-AUD-003) is a documented 17.7 deferral with no behavioral effect.
- Suggested closure: push the six unpushed commits and let the three-platform phase17_acceptance + CI matrix run green at the new head, then re-audit P17-PLT-001/P17-A15 against that run evidence (P17-AUD-001); serialize or widen the RPC wait budget for the thinking-level mode test (P17-AUD-002); rename the trace-named RPC seam when that surface next changes (P17-AUD-003).

### P17-AUD-003: Legacy 'trace' naming persists on the RPC evidence-recorder seam

- Axis: residuals
- Severity: Info
- Conformance effect: advisory
- Requirement IDs: P17-MIG-003
- Claim: RpcRunner::new_with_trace still names the evidence-recorder injection seam with the removed trace vocabulary, a documented 17.7 deferred residual that remains at audit_head.
- Evidence: crates/opi-coding-agent/src/rpc.rs:167 - pub fn new_with_trace constructs the RPC runner with an EvidenceRecorder despite the trace-named surface.
- Refutation attempted: searched all GitHub Actions runs for any run covering content after 136c380 (none exist); confirmed the phase17_acceptance job definition is present in ci.yml:92-108 at audit_head so the gap is execution, not definition; local Windows batteries are green in the sealed export (19+60+34+24+2+9+17 / 12+54 / 22+7+19+7+28+9+15 / 9+31+15+44+61+30 / 80 tests, 0 failures serial), which evidences Windows only and cannot substitute Linux/macOS parity for the unpushed bedrock event-stream and session file-IO deltas (applies to P17-AUD-001); the rpc timeout (P17-AUD-002) was re-run isolated and serially and passed, refuting a deterministic defect; the naming residual (P17-AUD-003) is a documented 17.7 deferral with no behavioral effect.
- Suggested closure: push the six unpushed commits and let the three-platform phase17_acceptance + CI matrix run green at the new head, then re-audit P17-PLT-001/P17-A15 against that run evidence (P17-AUD-001); serialize or widen the RPC wait budget for the thinking-level mode test (P17-AUD-002); rename the trace-named RPC seam when that surface next changes (P17-AUD-003).

## Verification Commands

| Command | Result | Requirement/finding |
|---|---|---|
| cargo test -p opi-agent --no-fail-fast --test tool_authority --test evidence_contract --test evidence_runtime --test phase17_prepare_call --test agent_wrapper --test hooks_queues --test tool_validation (sealed export) | PASS 19+60+34+24+2+9+17, 0 failed | core authority/state/evidence requirements |
| cargo test -p opi-ai --no-fail-fast --test provider_collection --test per_request_auth (sealed export) | PASS 12+54, 0 failed | P17-PRV-*, P17-FAL-001/002 |
| cargo test -p opi-coding-agent --no-fail-fast --test phase17_tool_authority --test phase17_provider_runtime --test phase17_product_evidence --test phase17_legacy_migration --test phase17_cross_mode --test phase17_failure_rollback --test phase17_api_audit (sealed export) | PASS 22+7+19+7+28+9+15, 0 failed | P17-AUT-*, P17-EVD-*, P17-MIG-*, P17-RBK-*, scenarios |
| cargo test -p opi-coding-agent --no-fail-fast --test session_runtime --test session_cli --test json_mode --test non_interactive --test interactive_mock --test tool_selection (sealed export) | PASS 9+31+15+44+61+30, 0 failed | P17-MIG-001/005, P17-A10/A13/A14 |
| cargo test -p opi-coding-agent --test rpc_jsonl (serial rerun, sealed export) | PASS 80/80 after 1 parallel-load timeout flake | P17-MIG-005, P17-AUD-002 |
| python scripts/opi-doc-check.py (sealed export) | PASS | P17-AUTH-002, P17-PLT-003 |
| gh run list --repo OdradekAI/opi --limit 8 | last success at 136c380; nothing at 890de6b | P17-PLT-001, P17-A15, P17-AUD-001 |
| git log origin/main..HEAD; git show --stat 5289892/21eaacf -- crates/ | 6 unpushed commits; ~700 lines of crate deltas | P17-AUD-001 |

## Verdict Rationale

68 of 70 mandatory requirements are met with current audit_head evidence (sealed-source verification plus green discriminating tests executed in the sealed export). P17-PLT-001 and P17-A15 are not-assessable because the three-platform acceptance matrix that mechanically verifies them has no run covering the current head: six commits are unpushed and two of them change platform-relevant crate source (P17-AUD-001, Major, blocks). Under the member verdict rule (FAIL if any mandatory state is not met), the member verdict is FAIL. Everything failing is closed by pushing the accumulated commits and letting CI execute; no source defect was observed in local verification.
