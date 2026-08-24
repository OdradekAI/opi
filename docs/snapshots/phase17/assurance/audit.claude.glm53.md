# Phase 17 Audit

**Audit run ID**: `phase17-claude-glm53-87377fc-20260824t135741z`
**Audit head**: `87377fcf750a5d0a38919bf82e740b7baefe8a8b`
**Reviewer ID**: `claude`
**Model ID**: `glm53`
**Reviewer identity**: Claude
**Reviewer model ID**: `glm-5.3[1M]`
**Model identity source**: runtime-attested
**Independence**: fresh-context-same-family (re-run of the claude peer at the new committed head; no sibling output read)
**Baseline policy**: latest-committed-spec
**Verdict**: PASS

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| .opi-impl-state.json | de5420ab9fc38407704fb2cc1a0fc999061d1aa817706198db6910941b974133 | current committed source |
| docs/snapshots/phase17/opi-impl-state.json | 801ac6d69b32acaa0f6301419c397c94450fc131e765ad9895c80c7cc33dd879 | current committed source |
| docs/opi-spec.md | cc7f8898f60c0d8abaa667f4b49b7affc721412e75dd3a67dcde37a783e1bc4c | current committed source |
| docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | 72f7e2996b0fbab7fd0c56d349b9dc0d5764c8a54e4ffd6cfc48245ac2bd4917 | current committed source; stored-hash mismatch vs ledger 9709183f (adjudicated clarification fe58c38) |

## Requirement Conformance

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| P17-AUTH-001 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_removed_interfaces_are_absent_from_production_source; phase17_next_call_routes_from_applied_state_nxt006 | met | - |
| P17-AUTH-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | 24/24 jobs success at head 87377fc: test (ubuntu/macos/windo | met | - |
| P17-AUTH-003 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | direct_run_never_fabricates_active_snapshot; binding_variants_are_distinguishable_and_not_normalizable | met | - |
| P17-OUT-001 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_next_call_routes_from_applied_state_nxt006; phase17_coding_harness_cross_provider_switch_dispatches_both_providers | met | - |
| P17-OUT-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_failed_prepare_preserves_state_and_skips_later_boundaries; phase17_invalid_prepare_candidate_preserves_state_with_typed_error | met | - |
| P17-OUT-003 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | missing_authorizer_yields_zero_executions; denying_authorizer_yields_zero_executions | met | - |
| P17-OUT-004 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | evidence_capture_finalizes_direct_runtime_input_manifest; phase17_complete_run_reconstructs_graph_and_rejects_missing_bindings | met | - |
| P17-PRV-001 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_two_providers_dispatch_through_one_collection_without_rebuild; phase17_removed_interfaces_are_absent_from_production_source | met | - |
| P17-PRV-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_canonical_selection_resolves_and_bare_is_ambiguous; phase17_legacy_bare_model_normalizes_to_unique_dispatchable_route | met | - |
| P17-PRV-003 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | prepare_call_resolves_auth_once_across_retries; prepare_call_resolves_route_and_auth_once_and_streams_via_prepared_seam | met | - |
| P17-PRV-004 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_route_and_auth_failures_do_not_dispatch_model_http; phase17_auth_failure_does_not_silently_fall_back_to_another_provider | met | - |
| P17-PRV-005 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | provider_turn_emits_correlated_provider_record; phase17_harness_switches_providers_with_matching_route_evidence | met | - |
| P17-PRV-006 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | 24/24 jobs success at head 87377fc: test (ubuntu/macos/windo | met | - |
| P17-NXT-001 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_agent_persists_complete_next_turn_state; phase17_removed_interfaces_are_absent_from_production_source | met | - |
| P17-NXT-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_invalid_prepare_candidate_preserves_state_with_typed_error; phase17_failed_prepare_preserves_state_and_skips_later_boundaries | met | - |
| P17-NXT-003 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_stop_observes_complete_next_turn_state | met | - |
| P17-NXT-004 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_failed_prepare_preserves_state_and_skips_later_boundaries; cancellation_while_stop_is_pending_restores_state_and_leaves_queues_untouched | met | - |
| P17-NXT-005 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_compaction_replaces_provider_visible_context; harness_compaction_emits_correlated_evidence_record | met | - |
| P17-NXT-006 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_next_call_routes_from_applied_state_nxt006 | met | - |
| P17-AUT-001 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | unknown_tool_yields_zero_executions; duplicate_provider_visible_name_is_rejected_at_construction | met | - |
| P17-AUT-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase8_tool_validation_failure_contract; phase8_hook_runs_before_schema_validation | met | - |
| P17-AUT-003 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_in_flight_effect_retains_actual_outcome_under_evidence_failure; stale_evidence_health_generation_yields_zero_executions | met | - |
| P17-AUT-004 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_model_content_cannot_expand_effective_policy; untrusted_content_sources_cannot_forge_tool_authority | met | - |
| P17-AUT-005 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | missing_authorizer_yields_zero_executions; denying_authorizer_yields_zero_executions | met | - |
| P17-AUT-006 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_removed_interfaces_are_absent_from_production_source; missing_authorizer_yields_zero_executions | met | - |
| P17-AUT-007 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_after_call_replace_keeps_later_authorization_unchanged | met | - |
| P17-AUT-008 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | policy_excluded tool visibility tests in tool_selection/json_mode suites (30+44 green); required_evidence_failure_denies_unlaunched_tool_side_effect | met | - |
| P17-EVD-001 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | run_ids_are_unique_across_allocators; call_ids_and_sequence_are_unique_and_monotonic_within_run | met | - |
| P17-EVD-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | tool_turn_emits_provider_then_tool_records_in_order; retry_emits_retry_record_parented_to_provider_call | met | - |
| P17-EVD-003 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | direct_run_never_fabricates_active_snapshot; binding_variants_are_distinguishable_and_not_normalizable | met | - |
| P17-EVD-004 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | measured_zero_is_not_unknown; measurement_origins_are_distinct | met | - |
| P17-EVD-005 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | structured_payload_is_redacted_at_the_producer_boundary; evidence_record_payload_channels_are_all_typed | met | - |
| P17-EVD-006 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_default_harness_emits_no_evidence; noop_sink_is_default_and_captures_nothing | met | - |
| P17-EVD-007 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | setup_failure_aborts_before_provider_call; phase17_failure_precedence_stops_before_later_boundaries | met | - |
| P17-EVD-008 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | evidence_emission_failure_withholds_manifest_and_preserves_outcome; finalization_failure_withholds_manifest_through_harness | met | - |
| P17-EVD-009 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | required_evidence_failure_denies_unlaunched_tool_side_effect; phase17_in_flight_effect_retains_actual_outcome_under_evidence_failure | met | - |
| P17-EVD-010 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_core_evidence_adapters_are_limited_to_noop_and_in_memory; noop_and_in_memory_satisfy_one_lifecycle_conformance_contract | met | - |
| P17-EVD-011 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | noop_and_in_memory_satisfy_one_lifecycle_conformance_contract; file_evidence_sink_writes_records_and_manifest | met | - |
| P17-FAL-001 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_failure_boundaries_expose_distinguishable_typed_classes; collection_error_match_is_exhaustive_for_current_public_surface | met | - |
| P17-FAL-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_failure_precedence_stops_before_later_boundaries; phase17_failed_prepare_preserves_state_and_skips_later_boundaries | met | - |
| P17-FAL-003 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_cancellation_and_evidence_failure_are_not_converted_to_success; phase17_real_cancellation_is_typed_and_complete_in_every_public_mode | met | - |
| P17-FAL-004 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_canaries_stop_before_sink_file_and_manifest; phase17_canary_is_absent_from_print_output | met | - |
| P17-MIG-001 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_legacy_session_fixture_byte_identical_after_resume_normalize_fork; phase17_legacy_sessions_and_opaque_traces_are_byte_preserved | met | - |
| P17-MIG-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_legacy_bare_model_normalizes_to_unique_dispatchable_route; phase17_legacy_ambiguous_route_keeps_cli_and_emits_typed_remediation | met | - |
| P17-MIG-003 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_trace_cli_writes_evidence_files; file_evidence_sink_writes_records_and_manifest | met | - |
| P17-MIG-004 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_legacy_trace_file_blocks_evidence_setup_and_stays_byte_identical; phase17_legacy_sessions_and_opaque_traces_are_byte_preserved | met | - |
| P17-MIG-005 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_all_public_product_modes_share_runtime_semantics; phase17_real_cancellation_is_typed_and_complete_in_every_public_mode | met | - |
| P17-MIG-006 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_removed_interfaces_are_absent_from_production_source | met | - |
| P17-PLT-001 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | 24/24 jobs success at head 87377fc: test (ubuntu/macos/windo | met | - |
| P17-PLT-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_tests_are_hermetic_no_network_no_paid_providers | met | - |
| P17-PLT-003 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_documentation_claims_no_os_sandbox | met | - |
| P17-RBK-001 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_failure_boundaries_expose_distinguishable_typed_classes; phase17_failure_precedence_stops_before_later_boundaries | met | - |
| P17-RBK-002 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_removed_interfaces_are_absent_from_production_source; phase17_rollback_preserves_session_and_evidence_bytes | met | - |
| P17-RBK-003 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_rollback_preserves_session_and_evidence_bytes; phase17_legacy_trace_file_blocks_evidence_setup_and_stays_byte_identical | met | - |
| P17-RBK-004 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_rollback_does_not_widen_user_policy | met | - |
| P17-A01 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_harness_switches_providers_with_matching_route_evidence; phase17_coding_harness_cross_provider_switch_dispatches_both_providers | met | - |
| P17-A02 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_route_and_auth_failures_do_not_dispatch_model_http; provider_refresh_error_survives_failed_post_failure_reread | met | - |
| P17-A03 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | prepare_call_resolves_auth_once_across_retries; start_attempt_allows_sequential_retry_after_terminal_and_resolves_auth_once | met | - |
| P17-A04 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_stop_observes_complete_next_turn_state | met | - |
| P17-A05 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_failed_prepare_preserves_state_and_skips_later_boundaries; phase17_invalid_prepare_candidate_preserves_state_with_typed_error | met | - |
| P17-A06 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_model_content_cannot_expand_effective_policy; phase17_command_execute_deny_is_fail_closed | met | - |
| P17-A07 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | untrusted_content_sources_cannot_forge_tool_authority; phase17_untrusted_sources_cannot_forge_registration_or_grants | met | - |
| P17-A08 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_expired_or_failed_authority_is_fail_closed; missing_authorizer_yields_zero_executions | met | - |
| P17-A09 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_one_run_graph_includes_tool_execution_record; phase17_complete_run_reconstructs_graph_and_rejects_missing_bindings | met | - |
| P17-A10 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_canaries_stop_before_sink_file_and_manifest; phase17_canary_is_absent_from_print_output | met | - |
| P17-A11 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | evidence_emission_failure_withholds_manifest_and_preserves_outcome; finalization_failure_withholds_manifest_through_harness | met | - |
| P17-A12 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | required_evidence_failure_denies_unlaunched_tool_side_effect; harness_complete_evidence_mapping_denies_unlaunched_tool | met | - |
| P17-A13 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_legacy_sessions_and_opaque_traces_are_byte_preserved; phase17_legacy_trace_file_blocks_evidence_setup_and_stays_byte_identical | met | - |
| P17-A14 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_all_public_product_modes_share_runtime_semantics; phase17_real_cancellation_is_typed_and_complete_in_every_public_mode | met | - |
| P17-A15 | docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md | phase17_ci_matrix_selects_same_acceptance_on_three_platforms; phase17_platform_contract_is_platform_neutral | met | - |

## Standards Review

Crate content at 87377fc is byte-identical to 890de6b (`git diff 890de6b 87377fc -- crates/ Cargo.toml Cargo.lock` is empty); the three commits between them touch only assurance skills, docs, and the assurance directory. The prior head's standards inspection therefore carries forward unchanged: single provider dispatch seam, no alias registry, opi-agent depends only inward, removed-interface scan green, and `python scripts/opi-doc-check.py` passes.

## Spec Review

All 70 registered criteria (55 `P17-*` + 15 `P17-A*`) are met. The two criteria previously marked not-assessable — P17-PLT-001 (three-platform CI matrix) and P17-A15 (platform-neutral acceptance) — are now met by actual three-platform evidence: CI run 32733627895 at 87377fc is fully green, including `test` and `Phase 17 acceptance` on ubuntu, macOS, and Windows, plus clippy/fmt/doctest/doc/docs_contract/execution_acceptance/Target check/opi-sandbox package. The prior Major finding (no current-head CI evidence) is refuted and removed.

## Security, Invariants, Integration, Test Quality, and Residuals

The prior head's focused local verification (165 opi-agent tests, 66 opi-ai tests, 107 opi-coding-agent phase17 tests, 190 mode/session tests, 80 rpc_jsonl, doc-check) applies to identical crate bytes and is superseded by the stronger current-head CI result: the full workspace test suite, clippy, doctest, and doc all pass on three OSes at 87377fc. One residual remains (P17-AUD-003, Info): the `RpcRunner::new_with_trace` naming. A prior parallel-load rpc timeout was not reproduced in isolation, serially, or in CI, and is not carried forward as a finding.

## Minimum-change Conformance

Status: conforming. The crate deltas between 890de6b and 87377fc are empty; all nine admitted implementation tasks remain conformant as previously inspected, and the assurance redesign commits change no crate source.

## Findings

### P17-AUD-003: Legacy 'trace' naming persists on the RPC evidence-recorder seam

- Axis: residuals
- Severity: Info
- Conformance effect: advisory
- Requirement IDs: P17-MIG-003
- Claim: RpcRunner::new_with_trace still names the evidence-recorder seam with the removed trace vocabulary.
- Evidence: crates/opi-coding-agent/src/rpc.rs retains `pub fn new_with_trace` at 87377fc.
- Refutation attempted: the naming is a documented 17.7 deferred residual with no behavioral effect; no refutation is required for an Info advisory.
- Suggested closure: rename the seam when that surface next changes.

## Verification Commands

| Command | Result | Requirement/finding |
|---|---|---|
| gh run view 32733627895 --repo OdradekAI/opi --json jobs | 24/24 success at 87377fc (test + Phase 17 acceptance on 3 OS) | P17-PLT-001, P17-A15 |
| git diff 890de6b 87377fc -- crates/ Cargo.toml Cargo.lock | empty | prior head's crate inspection carries forward |
| python scripts/opi-doc-check.py | PASS | P17-AUTH-002, P17-PLT-003 |
| git grep new_with_trace 87377fc -- crates/opi-coding-agent/src/rpc.rs | 1 | P17-AUD-003 |

## Verdict Rationale

All 70 mandatory requirements are met at 87377fc, with the three-platform CI run 32733627895 providing the current-head platform evidence that the prior head lacked. The only remaining finding is an Info advisory (naming residual), which does not affect the verdict. Member verdict: PASS.
