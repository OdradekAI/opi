# Phase 19 Audit

**Audit run ID**: `phase19-codex-gpt61-2ec1ccb-20261004t023807z`
**Audit head**: `2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618`
**Reviewer ID**: `codex`
**Model ID**: `gpt61`
**Reviewer identity**: Codex
**Reviewer model ID**: `gpt61`
**Model identity source**: operator-declared
**Independence**: fresh-context-same-family; fresh independent requirements/code context, with prior Phase audits, remediation conclusions, raw ledgers and sibling outputs excluded through installation.
**Baseline policy**: latest-committed-spec
**Verdict**: FAIL

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| `.opi-impl-state.json` | `c65c10635c4bb8b1754625b94e8a0a15459eef6d65634610ba9b65ea9ba5337e` | Committed baseline; allowlisted ledger projection, no stored-hash mismatch |
| `docs/snapshots/phase19/opi-impl-state.json` | `35b3378bd902cc976bb348d794dd1b5d3c5f160a58769c3aeb62fea175a7b7a9` | Committed baseline; allowlisted ledger projection, no stored-hash mismatch |
| `docs/opi-spec.md` | `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db` | Committed baseline; allowlisted ledger projection, no stored-hash mismatch |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Committed baseline; allowlisted ledger projection, no stored-hash mismatch |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.zh.md` | `d41c826412b0d71802327d60f953708f6cb2083436a2a2b0d99a2e52d851f194` | Committed baseline; allowlisted ledger projection, no stored-hash mismatch |

Scope comes from the registered Phase 19 tasks and both registered language counterparts; the parent spec retains authority. No prior acceptance prose or history was used as proof. Requirements were sealed before production inspection: 72 records, 71 mandatory, with initial identity-set SHA-256 `7e35f1fdab697e135bd01c921e889f17b5811f9feb38b944961763cfcb86df6e`. Identity, criterion and observable behavior fields remain unchanged.

Inspection and checks used a Git archive export at the head above. A Linux copy preserved source bytes; tar-archived committed aliases were restored for the documentation check. Cargo used the existing leased external cache `/mnt/e/opi/cargo-targets/opi-8abcf38f9871653e`; no cache cleanup or live-worktree product edit occurred.

## Requirement Conformance

States: 52 met, 7 partially-met, 13 not-assessable, 0 not-met. Fixture evidence establishes importer/authority predicates, not the explicit actual/native/live obligations. Every incomplete mandatory predicate has a reciprocal finding.

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| P19-R01 | Section 4;P19-AC-001 | Doctor resolves config, bounds engine probes and exposes engine/architecture observations. No Agent entry or credential resolution is reachable. Evidence: `crates/opi-eval/tests/native_driver.rs::stalled_engine_observer_bounds_plan_admission`; `crates/opi-eval/src/runner/prepare.rs::observations_use_selected_engine_and_runtime_registration_stays_unknown`. Full surfaces/checks in sidecar. | met | — |
| P19-R02 | Section 4;P19-AC-007 | CPU/memory/disk observations and engine-bound GPU facts retain typed unknowns and reasons; host/runtime registration does not invent device capacity. Evidence: `crates/opi-eval/src/runner/prepare.rs::observations_use_selected_engine_and_runtime_registration_stays_unknown`; `crates/opi-eval/src/runner/prepare.rs::independent_resource_deficits_and_unknowns_reject_each_role`. Full surfaces/checks in sidecar. | met | — |
| P19-R03 | Section 4;P19-AC-001 | Selected profile/corpus/runner/integration/client/capture inputs are inventoried before task effects; malformed or missing selected materials reject. Evidence: `crates/opi-eval/tests/native_driver.rs::witness_material_preflight_consumes_each_selected_profile_without_effects`; `crates/opi-eval/tests/native_driver.rs::witness_material_refuses_invalid_inputs_before_effects`. Full surfaces/checks in sidecar. | met | — |
| P19-R04 | Section 4;P19-AC-001 | Spawned public prepare from an independent directory freezes local-origin byte/pin identities; explicit cold-source commit and unpinned refusal are exercised. Engine/build tools are controlled fixtures. Evidence: `crates/opi-eval/tests/native_driver.rs::independent_directory_prepare_builds_a_local_origin_material`; `crates/opi-eval/tests/native_driver.rs::prepare_cold_selected_products_from_explicit_committed_source`. Full surfaces/checks in sidecar. | met | — |
| P19-R05 | Section 5;P19-AC-007 | Selected immutable image IDs and local-origin inventories are frozen; runtime, package and resource drift reject. Tests use controlled engine commands, not proof of an actual benchmark run. Evidence: `crates/opi-eval/tests/native_driver.rs::public_prepare_acquires_only_selected_immutable_images`; `crates/opi-eval/tests/native_driver.rs::prepare_binds_runtime_package_and_rejects_resource_drift`. Full surfaces/checks in sidecar. | met | — |
| P19-R06 | Section 4;P19-AC-013 | Only selected products are loaded; pi-only preparation and execution require neither Opi material nor a fabricated comparison edge. Evidence: `crates/opi-eval/tests/native_driver.rs::pi_only_preparation_requires_no_opi_material`; `crates/opi-eval/tests/assembled_run.rs::pi_only_selection_runs_without_an_artificial_edge`. Full surfaces/checks in sidecar. | met | — |
| P19-R07 | Section 4;P19-AC-001 | Preparation acquires materials through a filtered environment and has no call to resolve_connection_credential or Agent/provider dispatch. Controlled CLI refusal asserts no task effects. Evidence: `crates/opi-eval/tests/native_driver.rs::independent_directory_prepare_builds_a_local_origin_material`; `crates/opi-eval/tests/native_driver.rs::witness_material_refuses_invalid_inputs_before_effects`. Full surfaces/checks in sidecar. | met | — |
| P19-R08 | Section 5;P19-AC-007 | Task inventory distinguishes Agent/verifier roles, units, inheritance origins, collect/lifecycle/network and deadlines; full inventory retains infeasible entries. Evidence: `crates/opi-eval/src/runner/prepare.rs::pinned_requirement_defaults_and_role_inheritance_are_explicit`; `crates/opi-eval/src/runner/prepare.rs::effective_network_lifecycle_collect_and_deadlines_keep_origins`. Full surfaces/checks in sidecar. | met | — |
| P19-R09 | Section 5;P19-AC-007 | Full preflight lists all tasks and contradictions without running tasks; feasible explicit subsets freeze only requested identities. This does not prove full TB3 completion. Evidence: `crates/opi-eval/tests/native_driver.rs::full_inventory_preflight_lists_requirements_and_freezes_feasible_subsets`; `crates/opi-eval/tests/native_driver.rs::plan_rejections_name_every_contradiction`. Full surfaces/checks in sidecar. | met | — |
| P19-R10 | Section 5;P19-AC-007 | Unknown/unsupported resource semantics and deficits reject selected tasks; no automatic selection shrink or native-equivalent substitution is used. Evidence: `crates/opi-eval/tests/native_driver.rs::unmet_resource_requirement_rejects_without_silent_shrink`; `crates/opi-eval/src/runner/prepare.rs::invalid_units_and_unimplemented_requirements_never_disappear`. Full surfaces/checks in sidecar. | met | — |
| P19-R11 | Section 4;P19-AC-007 | Selection reason, exact IDs, repetitions and supplied seeds persist in the digest-bound selection/preview. Applicable seed data is retained; no claim of model determinism is made. Evidence: `crates/opi-eval/tests/experiment_contract.rs::resolve_selection_freezes_scope_tasks_repetitions_and_bounds`; `crates/opi-eval/tests/assembled_run.rs::frozen_subset_execution`. Full surfaces/checks in sidecar. | met | — |
| P19-R12 | Section 4;P19-AC-001 | Public preview contains selected subjects/connection/tasks/repetitions/count/concurrency/bounds and contradictions before execution; oversized expansion is separately failed under R34. Evidence: `crates/opi-eval/tests/native_driver.rs::full_inventory_preflight_lists_requirements_and_freezes_feasible_subsets`; `crates/opi-eval/tests/experiment_contract.rs::validate_cli_summarizes_selection_documents`. Full surfaces/checks in sidecar. | met | — |
| P19-R13 | Section 4;P19-AC-007 | Frozen plan embeds exact resolved contract and byte/material identities; public CLI rejects changed material/plan before dispatch and executes frozen slots. Native result producer is controlled in deterministic cases. Evidence: `crates/opi-eval/tests/assembled_run.rs::frozen_subset_execution`; `crates/opi-eval/tests/assembled_run.rs::tampered_plan_rejects_before_any_trial`. Full surfaces/checks in sidecar. | met | — |
| P19-R14 | Section 4;P19-AC-001 | Independent-directory and structured stage/settlement output are exercised with controlled producers. No supplied actual six-trial local workflow evidence establishes the native assembled path. Evidence: `crates/opi-eval/tests/native_driver.rs::native_frozen_plan_runs_without_fixtures_and_rejects_material_drift`; `crates/opi-eval/tests/assembled_run.rs::hermetic_paired_group_runs_end_to_end`. Full surfaces/checks in sidecar. | partially-met | P19-AUD-004 |
| P19-R15 | Section 5;P19-AC-002 | Production importer rejects wrong/absent/oracle Agent, terminal identity and preflight substitution despite a successful subprocess. Synthetic result records exercise the exact consumer. Evidence: `crates/opi-eval/tests/native_driver.rs::native_result_binding_rejects_every_substitution`; `crates/opi-eval/tests/native_driver.rs::remediation_003_substituted_terminal_identity_rejects`. Full surfaces/checks in sidecar. | met | — |
| P19-R16 | Section 5;P19-AC-002 | Exact task package/reference, fresh job/trial, subject and result bindings reject stale/swapped/substituted observations before a score seal. Evidence: `crates/opi-eval/tests/native_driver.rs::native_result_binding_rejects_every_substitution`; `crates/opi-eval/tests/native_driver.rs::material_five_rejects_swapped_omitted_and_stale_task_bindings`. Full surfaces/checks in sidecar. | met | — |
| P19-R17 | Section 5;P19-AC-002 | Delivered artifact digest and verifier-input identity are compared to expected handoff; missing/mismatched evidence cannot obtain native Agent attribution. Evidence: `crates/opi-eval/tests/native_driver.rs::native_result_binding_rejects_every_substitution`; `crates/opi-eval/tests/native_driver.rs::material_five_rejects_swapped_omitted_and_stale_task_bindings`. Full surfaces/checks in sidecar. | met | — |
| P19-R18 | Section 5;P19-AC-002 | Selected client upload/launch, terminal and grading share the handoff/job in current source. Controlled integration environments prove dispatch ordering, but actual Opi/pi same-scored-trial evidence was not supplied. Evidence: `crates/opi-eval/tests/native_driver.rs::native_result_binding_rejects_every_substitution`; `crates/opi-eval/scripts/test_native_integrations.py`. Full surfaces/checks in sidecar. | partially-met | P19-AUD-004 |
| P19-R19 | Section 5;P19-AC-002 | Oracle/nop are explicitly selected diagnostics; normal Agent attribution rejects oracle and run_native_trial does not silently run oracle preflight. Evidence: `crates/opi-eval/tests/native_driver.rs::native_conformance_reruns_the_admitted_cases_through_the_material`; `crates/opi-eval/tests/native_driver.rs::native_result_binding_rejects_every_substitution`. Full surfaces/checks in sidecar. | met | — |
| P19-R20 | Section 5;P19-AC-003 | Shared verifier lifecycle is parsed and credential cleanup precedes return to the upstream verifier; deterministic integration tests observe cleanup/failure boundaries. Actual task witness is separately open in R21. Evidence: `crates/opi-eval/tests/native_driver.rs::native_trial_outcome_classifies_every_terminal_shape`; `crates/opi-eval/scripts/test_native_integrations.py`. Full surfaces/checks in sidecar. | met | — |
| P19-R21 | Section 5;P19-AC-003 | NOT RUN: pinned native witness material and raw retained execution proof were not supplied. Existing oracle/nop witness requires real upstream runner, task, Docker and reward discrimination. Evidence: `crates/opi-eval/tests/native_driver.rs::native_tb21_artifact_witness (ignored)`. Full surfaces/checks in sidecar. | not-assessable | P19-AUD-003 |
| P19-R22 | Section 7;P19-AC-003 | No supplied TB2.1 Opi/pi live captures, tool effects, original seals and native score bundle are available to verify. Synthetic acceptance tests are not real-client evidence. Evidence: `crates/opi-eval/scripts/test_verify_local_acceptance.py`. Full surfaces/checks in sidecar. | not-assessable | P19-AUD-004 |
| P19-R23 | Section 5;P19-AC-004 | TB3 task-declared collect and separate verifier lifecycle are consumed, declared artifacts bound and unsupported shapes rejected. Python controlled environments and Rust consumer negatives validate the boundary. Evidence: `crates/opi-eval/scripts/test_native_integrations.py`; `crates/opi-eval/tests/native_driver.rs::material_five_dispatches_two_exact_tasks_per_family_from_both_entries`. Full surfaces/checks in sidecar. | met | — |
| P19-R24 | Section 5;P19-AC-004 | NOT RUN: actual reward response to changed declared artifacts and exclusion of the unrelated marker needs supplied pinned native witness material/proof. Evidence: `crates/opi-eval/tests/native_driver.rs::native_tb3_artifact_witness (ignored)`. Full surfaces/checks in sidecar. | not-assessable | P19-AUD-003 |
| P19-R25 | Section 7;P19-AC-004 | No supplied TB3 real Opi/pi collected-artifact verifier trial captures/bundles are available. Fixture-generated native results do not establish actual-client execution. Evidence: `crates/opi-eval/scripts/test_verify_local_acceptance.py`. Full surfaces/checks in sidecar. | not-assessable | P19-AUD-004 |
| P19-R26 | Section 5;P19-AC-005 | DeepSWE handoff admits one committed-patch collect hook, exact base and declared pristine verifier/network semantics; wrong base/unsupported lifecycle reject. Native patch sensitivity witness remains R27. Evidence: `crates/opi-eval/scripts/test_native_integrations.py`; `crates/opi-eval/tests/native_driver.rs::material_five_rejects_swapped_omitted_and_stale_task_bindings`. Full surfaces/checks in sidecar. | met | — |
| P19-R27 | Section 5;P19-AC-005 | NOT RUN: no supplied actual pinned Pier correct/incorrect patch execution proof; current ignored witness checks differing collected digests and native rewards. Evidence: `crates/opi-eval/tests/native_driver.rs::native_deepswe_patch_witness (ignored)`. Full surfaces/checks in sidecar. | not-assessable | P19-AUD-003 |
| P19-R28 | Section 7;P19-AC-005 | No supplied real Opi/pi DeepSWE original patch captures/native trials are available to validate the actual committed patch reaching the pristine verifier. Evidence: `crates/opi-eval/scripts/test_verify_local_acceptance.py`. Full surfaces/checks in sidecar. | not-assessable | P19-AUD-004 |
| P19-R29 | Section 5;P19-AC-007 | Task Agent/verifier resources and declared network remain separate; unsupported fields reject. Pier limits provider egress through a parsed selected-endpoint allowlist. R60 records the separate shared loopback admission defect. Evidence: `crates/opi-eval/src/runner/prepare.rs::effective_network_lifecycle_collect_and_deadlines_keep_origins`; `crates/opi-eval/src/runner/prepare.rs::invalid_units_and_unimplemented_requirements_never_disappear`. Full surfaces/checks in sidecar. | met | — |
| P19-R30 | Section 4;P19-AC-006 | Selected connection freezes provider/API/endpoint/model/credential reference and generated client config; no implicit protocol/model/provider billing fallback is visible. Endpoint safety is separately failed in R60. Evidence: `crates/opi-eval/tests/experiment_contract.rs::connection_doc_resolves_and_freezes_the_selected_connection`; `crates/opi-eval/tests/experiment_contract.rs::resolve_rejects_unknown_or_duplicate_connection_selection`. Full surfaces/checks in sidecar. | met | — |
| P19-R31 | Section 4;P19-AC-006 | NOT RUN: actual pinned Opi/pi client paths/material not supplied. Projection/unit and scripted-server tests do not prove that both exact native clients complete authenticated tool loops on every API. Evidence: `crates/opi-eval/tests/native_driver.rs::native_protocol_client_matrix (ignored)`; `crates/opi-eval/tests/native_driver.rs::native_output_control_client_matrix (ignored)`. Full surfaces/checks in sidecar. | not-assessable | P19-AUD-003 |
| P19-R32 | Section 4;P19-AC-006 | Requested/effective/unsupported/unknown controls and origins remain explicit. Required unsupported reasoning/sampling and unproven exact-client output controls reject before dispatch. Evidence: `crates/opi-eval/src/experiment.rs::projection_tests`; `crates/opi-eval/tests/native_driver.rs::p19_plan_required_sampling_rejects_before_dispatch`. Full surfaces/checks in sidecar. | met | — |
| P19-R33 | Section 4;P19-AC-006 | Configured model remains a request identity; absent observed service model is typed unknown with reason, not filled from config. Evidence: `crates/opi-eval/tests/report_contract.rs::asymmetric_native_facts`; `crates/opi-eval/src/report.rs::tests::native_facts_are_measured_or_typed_unknown_never_fabricated`. Full surfaces/checks in sidecar. | met | — |
| P19-R34 | Section 4;P19-AC-010 | Small over-cap expansions reject, but repetitions=4294967295/max_trials=1 allocates the full trial vector first. The 256 MiB reproduction aborts SIGABRT before the declared bound can reject. Evidence: `crates/opi-eval/tests/experiment_contract.rs::resolve_rejects_unenforceable_execution_bounds`; `crates/opi-eval/tests/native_driver.rs::max_trials_caps_every_resolved_trial_list_before_effects`. Full surfaces/checks in sidecar. | partially-met | P19-AUD-002 |
| P19-R35 | Section 7;P19-AC-006 | No supplied first-connection Zhipu glm-5.3-flash Anthropic real Opi/pi request/auth/tool-loop/effective-control proof is available. Verifier fixture tests establish rejection predicates only. Evidence: `crates/opi-eval/scripts/test_verify_local_acceptance.py::test_first_connection_and_live_mode_are_required`; `crates/opi-eval/scripts/test_verify_local_acceptance.py::test_live_agent_observations_cannot_be_substituted`. Full surfaces/checks in sidecar. | not-assessable | P19-AUD-004 |
| P19-R36 | Section 4;P19-AC-010 | Selected launch resolves explicit env/private-file reference only, private stage permissions precede credential writes, client configs/environment are isolated, and capture/export omits secret material. Evidence: `crates/opi-eval/tests/native_driver.rs::native_missing_credential_rejects_before_any_effect`; `crates/opi-eval/tests/native_driver.rs::native_host_credential_stage_is_private_on_searchable_root`. Full surfaces/checks in sidecar. | met | — |
| P19-R37 | Section 4;P19-AC-010 | Selected temporary credentials are removed on settlement; verifier-only environment excludes ambient secrets, and cleanup failure stops grade attribution. Evidence: `crates/opi-eval/tests/native_driver.rs::native_settlement_removes_actual_credentials`; `crates/opi-eval/tests/native_driver.rs::remediation_001_regrade_child_has_no_ambient_secret`. Full surfaces/checks in sidecar. | met | — |
| P19-R38 | Section 4;P19-AC-010 | Closed native capture grammar strips private thinking before first retained sink, selected credential canary rejects sealing and halts new dispatch, and rejected capture remains unscored. Evidence: `crates/opi-eval/tests/native_driver.rs::native_capture_helper_sanitizes_private_thinking`; `crates/opi-eval/tests/native_driver.rs::native_capture_admits_only_closed_stderr_diagnostics`. Full surfaces/checks in sidecar. | met | — |
| P19-R39 | Section 4;P19-AC-010 | Agent receives selected runtime/config/private log staging rather than broad host trees or Docker socket; owned Docker runner/capture handoff validates narrow mounts. Controlled integration tests are not actual-Docker proof. Evidence: `crates/opi-eval/scripts/test_native_integrations.py`; `crates/opi-eval/tests/native_driver.rs::native_host_credential_stage_is_private_on_searchable_root`. Full surfaces/checks in sidecar. | met | — |
| P19-R40 | Section 6;P19-AC-008 | One sealed final classifier joins native terminal, native grade admission and lifecycle; zero exit alone cannot turn provider/identity/grade failure into score eligibility. Evidence: `crates/opi-eval/tests/native_driver.rs::native_trial_outcome_classifies_every_terminal_shape`; `crates/opi-eval/tests/authority_boundaries.rs::fal002_boundary_failures_stop_grade_dispatch`. Full surfaces/checks in sidecar. | met | — |
| P19-R41 | Section 6;P19-AC-008 | Valid native zero, graded Agent failure and benchmark-valid timeout stay in the native metric denominator and eligible grading path. Evidence: `crates/opi-eval/tests/native_driver.rs::native_trial_outcome_classifies_every_terminal_shape`; `crates/opi-eval/tests/report_contract.rs::native_outcome_counts_keep_denominators_and_failure_classes`. Full surfaces/checks in sidecar. | met | — |
| P19-R42 | Section 6;P19-AC-008 | Provider/auth/adapter/infra/invalid-grader failures remain attempted-invalid without Agent reward, including bound unsealed failure diagnostics. Evidence: `crates/opi-eval/tests/native_driver.rs::native_provider_failure_report`; `crates/opi-eval/tests/native_driver.rs::remediation_004_native_provider_error_is_not_scored`. Full surfaces/checks in sidecar. | met | — |
| P19-R43 | Section 6;P19-AC-008 | Native stream import distinguishes JSON terminal auth/rate/protocol errors at exit zero from a transient retry followed by completion. Assertions discriminate stream histories; actual-client matrix is separately open in R31. Evidence: `crates/opi-eval/tests/agent_integration_conformance.rs::pi_protocol_truncations_classify_typed_across_all_three_wires`; `crates/opi-eval/tests/agent_integration_conformance.rs::agent_conformance_matrix_settles_every_pinned_case`. Full surfaces/checks in sidecar. | met | — |
| P19-R44 | Section 6;P19-AC-008 | Durable intent/planned slots keep unstarted/cancelled/effect-unknown coverage; no resume path silently reruns or invents a native grade. Evidence: `crates/opi-eval/tests/assembled_run.rs::effect_unknown_reopen`; `crates/opi-eval/tests/assembled_run.rs::recovery_preserves_every_original_file_and_planned_trial`. Full surfaces/checks in sidecar. | met | — |
| P19-R45 | Section 6;P19-AC-008 | Sealed closure binds subject/client build, selected connection/control, task/runner/job/trial, artifact/verifier/native outputs and observed environment proof; substitution fails at the production importer/seal. Evidence: `crates/opi-eval/tests/native_driver.rs::native_result_binding_rejects_every_substitution`; `crates/opi-eval/tests/bundle_recompute.rs::sealed_manifest_attributes_artifacts_to_their_producers`. Full surfaces/checks in sidecar. | met | — |
| P19-R46 | Section 6;P19-AC-008 | Report derives metric definitions and corpus/selected/planned/attempted/valid/failure/invalid/unexecuted counts from frozen control and verified settlement rather than surviving bundles alone. Evidence: `crates/opi-eval/tests/report_contract.rs::native_outcome_counts_keep_denominators_and_failure_classes`; `crates/opi-eval/tests/report_output_containment.rs::all_unsealed_attempts_keep_coverage_and_reject_conflicting_contracts`. Full surfaces/checks in sidecar. | met | — |
| P19-R47 | Section 6;P19-AC-008 | Binary native task metric is separate from per-trial passed/total; subset/full scope survives missing bundles and no cross-benchmark average is generated. Evidence: `crates/opi-eval/tests/native_driver.rs::native_test_counts_match_producer_and_offline_projection`; `crates/opi-eval/tests/native_driver.rs::full_selection_report_counts_frozen_tasks_after_bundle_loss`. Full surfaces/checks in sidecar. | met | — |
| P19-R48 | Section 6;P19-AC-008 | Pair universe binds task/repetition/group and control fingerprints; protocol/same-product mismatches and missing sides stay explicit, while single-subject requires no absent peer. Evidence: `crates/opi-eval/tests/report_contract.rs::protocol_safe_pairing_and_single_subject_coverage`; `crates/opi-eval/tests/pairing_and_integrity.rs::pair_and_control_fail_closed`. Full surfaces/checks in sidecar. | met | — |
| P19-R49 | Section 6;P19-AC-008 | Telemetry missingness has state/reason distinct from numeric zero; exclusions and per-family native metrics survive offline report reconstruction. Evidence: `crates/opi-eval/tests/report_contract.rs::asymmetric_native_facts`; `crates/opi-eval/src/report.rs::tests::native_facts_are_measured_or_typed_unknown_never_fabricated`. Full surfaces/checks in sidecar. | met | — |
| P19-R50 | Section 6;P19-AC-009 | Byte-only verification consumes original seals offline, rejects drift/linked entries/unsealed evidence, and has no dispatch route. Evidence: `crates/opi-eval/tests/bundle_recompute.rs::covered_byte_mutation_is_rejected`; `crates/opi-eval/tests/bundle_recompute.rs::bnd001_retained_byte_closure_rejects_drift`. Full surfaces/checks in sidecar. | met | — |
| P19-R51 | Section 6;P19-AC-009 | Repeated report/verify is byte-stable; canonical output/source containment rejects overwrite/alias/symlink targets and preserves original bytes. Evidence: `crates/opi-eval/tests/bundle_recompute.rs::repeated_offline_operations_are_stable`; `crates/opi-eval/tests/report_output_containment.rs::output_rejects_ancestor_alias_into_run_root`. Full surfaces/checks in sidecar. | met | — |
| P19-R52 | Section 6;P19-AC-009 | Current native entry uses copied saved artifacts, filtered environment, pinned offline cached grader and fresh verifier/job. Controlled public CLI tests prove handoff and no ambient secret; actual native verifier response to preserved real input was not executed. Evidence: `crates/opi-eval/tests/native_driver.rs::native_offline_regrade`; `crates/opi-eval/tests/native_driver.rs::remediation_001_regrade_child_has_no_ambient_secret`. Full surfaces/checks in sidecar. | partially-met | P19-AUD-003 |
| P19-R53 | Section 6;P19-AC-009 | Missing cache/artifacts, task/base drift, unsupported network/collect and tampered source reject before verifier effects; no fallback live acquisition or provider route exists. Evidence: `crates/opi-eval/tests/native_driver.rs::native_offline_regrade_rejects_every_fail_closed_case`; `crates/opi-eval/tests/native_driver.rs::native_offline_regrade_rejects_all_tampered_sources_before_effects`. Full surfaces/checks in sidecar. | met | — |
| P19-R54 | Section 6;P19-AC-009 | Regrade writes fresh derived source/artifact/verifier identities in a new contained target; byte snapshots prove original scores/source bundle unchanged in controlled cases. Evidence: `crates/opi-eval/tests/native_driver.rs::native_offline_regrade_accepts_relative_source`; `crates/opi-eval/tests/native_driver.rs::native_offline_regrade`. Full surfaces/checks in sidecar. | met | — |
| P19-R55 | Section 6;P19-AC-009 | NOT RUN: real three-family verifier-only witness requires pinned material. Declared native_offline_regrade --ignored --exact runs zero tests; the real ignored witness has another name. Evidence: `crates/opi-eval/tests/native_driver.rs::native_verifier_only_regrade_witness (ignored)`; `crates/opi-eval/tests/native_driver.rs::native_offline_regrade (non-ignored fixture case)`. Full surfaces/checks in sidecar. | not-assessable | P19-AUD-003 |
| P19-R56 | Section 9;P19-AC-010 | Shared cancellation stops new trial dispatch and bounds task/process settlement; cleanup/identity/privacy failures stop further affected dispatch. Pre-expansion max_trials failure remains separate R34. Evidence: `crates/opi-eval/tests/native_driver.rs::native_signal_cancellation_bounds_dispatch`; `crates/opi-eval/tests/native_driver.rs::plan_p19_cleanup_stops_dispatch`. Full surfaces/checks in sidecar. | met | — |
| P19-R57 | Section 9;P19-AC-010 | Real Unix process-tree and controlled Docker ownership cases pass, requiring daemon IDs/exact root and preserving unrelated resources. Actual Docker owned-resource cancellation witness/raw orphan check was not supplied. Evidence: `crates/opi-eval/tests/native_driver.rs::native_settlement_cleans_root_bound_containers`; `crates/opi-eval/tests/native_driver.rs::native_cancel_preserves_bounded_partial_capture`. Full surfaces/checks in sidecar. | partially-met | P19-AUD-003 |
| P19-R58 | Section 4;P19-AC-010 | Recovery only reads durable control/intent/settlement/seal state; byte snapshots and missing-intent cases retain original files, every planned slot and uncertainty without invoking an Agent. Evidence: `crates/opi-eval/tests/assembled_run.rs::recovery_preserves_every_original_file_and_planned_trial`; `crates/opi-eval/tests/assembled_run.rs::recovery_rejects_frozen_inventory_identity_drift`. Full surfaces/checks in sidecar. | met | — |
| P19-R59 | Section 4;P19-AC-010 | Replacement expands the same frozen group into a fresh run root and new IDs for all re-executed trials; original bytes and uncertain attempts are preserved. Evidence: `crates/opi-eval/tests/assembled_run.rs::replacement_fresh_resolved_pairs_preserve_original_scope`; `crates/opi-eval/tests/native_driver.rs::remediation_009_rejected_rerun_preserves_control`. Full surfaces/checks in sidecar. | met | — |
| P19-R60 | Section 8;P19-AC-011 | Plain remote host rejects, but example.invalid#@localhost and #@127.0.0.1 pass non-live resolution. Native-fixture reproduction also accepts the remote fragment URL; locked URL parser confirms example.invalid remains the host. Evidence: `crates/opi-eval/tests/experiment_contract.rs::resolve_rejects_remote_routes_without_explicit_live_mode`; `private probe-loopback.py`. Full surfaces/checks in sidecar. | partially-met | P19-AUD-001 |
| P19-R61 | Section 8;P19-AC-011 | CI/manual wrappers call the public local deterministic material/conformance path without required GitHub dispatch or paid inference. No supplied current executed local native conformance proof; default suite ignores 11 native/material cases. Evidence: `crates/opi-eval/scripts/test_verify_native_smoke_ci.py`; `crates/opi-eval/scripts/test_verify_external_lock_ci.py`. Full surfaces/checks in sidecar. | partially-met | P19-AUD-003 |
| P19-R62 | Section 8;P19-AC-011 | Offline supplied-evidence validator checks exact first connection, controls, command/exit/build/material/original seals/provider/preflight/tool proof and rejects oracle/fixture/missing substitutions. Positive test evidence uses constructed records/stub companion and proves validator predicates only. Evidence: `crates/opi-eval/scripts/test_verify_local_acceptance.py`; `crates/opi-eval/tests/native_driver.rs::p19_plan_acceptance_rejects_fixture_evidence`. Full surfaces/checks in sidecar. | met | — |
| P19-R63 | Section 8;P19-AC-012 | NOT ASSESSABLE: no actual six-trial evidence root was supplied or committed in the Phase snapshot. Constructed fixture records cannot establish attributable real Opi/pi trials across the three families. Evidence: `crates/opi-eval/scripts/test_verify_local_acceptance.py::test_valid_native_task_failures_count_as_graded_trials`. Full surfaces/checks in sidecar. | not-assessable | P19-AUD-004 |
| P19-R64 | Section 8;P19-AC-012 | NOT ASSESSABLE: absent supplied frozen operator scope/commands/exits/redacted logs/material/build/control seals and offline outputs prevent independent validation of pre-effect live acceptance. Evidence: `crates/opi-eval/scripts/test_verify_local_acceptance.py`. Full surfaces/checks in sidecar. | not-assessable | P19-AUD-004 |
| P19-R65 | Section 8;P19-AC-012 | WSL/Linux deterministic CLI/checks execute here, but no supplied actual doctor/prepare/plan/run acceptance path and source-bound final workspace gate union/raw proof are available. Scoped checks cannot replace the registered workspace union. Evidence: `crates/opi-eval/scripts/test_verify_local_acceptance.py`; `private WSL scoped checks`. Full surfaces/checks in sidecar. | not-assessable | P19-AUD-004 |
| P19-R66 | Section 8;P19-AC-012 | NOT ASSESSABLE: validator implements exhaustive byte-bound three-Opi view and repository-qualified upstream locks, but no actual originals/scoped view/pi/Harbor/Pier raw provenance were supplied for checking. Evidence: `crates/opi-eval/scripts/test_verify_local_acceptance.py`. Full surfaces/checks in sidecar. | not-assessable | P19-AUD-004 |
| P19-R67 | Section 3;P19-AC-013 | Publish=false Independent Companion has no normal/dev/build/optional/target Opi dependencies and no reverse runtime activation. Native runners/data/images are externally pinned; public modules remain provisional. Evidence: `crates/opi-eval/tests/experiment_contract.rs::independent_companion_dependency_boundary`. Full surfaces/checks in sidecar. | met | — |
| P19-R68 | Section 9;P19-AC-013 | Current versioned contracts, generated CLI help, callers/fixtures, bilingual guide and Unreleased document the cutovers. Linux export documentation checker passes with committed symlink aliases restored; legacy fixture reads remain classified, not relabelled real native scores. Evidence: `crates/opi-eval/tests/experiment_contract.rs`; `crates/opi-eval/tests/report_contract.rs::verify_regrade_split_names_each_operation`. Full surfaces/checks in sidecar. | met | — |
| P19-R69 | Section 9;P19-AC-010 | Owning identity/oracle/artifact/privacy/integrity/cleanup gates reject score/seal/report and stop affected dispatch. Fixture diagnostics remain explicit, normal native scoring has one attribution path. Default remote route breach is tracked in R60. Evidence: `crates/opi-eval/tests/authority_boundaries.rs::fal002_boundary_failures_stop_grade_dispatch`; `crates/opi-eval/tests/authority_boundaries.rs::fal002_seal_failure_stops_report`. Full surfaces/checks in sidecar. | met | — |
| P19-R70 | PRIN-002; PLACE-001; CTRL-007 | Public CLI/resolution seams own frozen contract/state transitions; private Agent and benchmark traits have two actual product adapters and three family adapters with shared conformance. Dependencies stay within the companion rather than exposing an Agent Core seam. Evidence: `crates/opi-eval/tests/experiment_contract.rs::independent_companion_dependency_boundary`; `crates/opi-eval/tests/agent_integration_conformance.rs::agent_conformance_matrix_settles_every_pinned_case`. Full surfaces/checks in sidecar. | met | — |
| P19-R71 | PRIN-001; PRIN-003; GOAL-003 | All 11 admitted tasks were traced to existing runner/supervisor/seal/report consumers. New config/persistent seams encode required frozen inputs/provenance; no new crate, generic plugin SDK or alternate score owner found. Behavioral ceiling triggers and native-proof limits are recorded task by task. Evidence: `crates/opi-eval/tests/assembled_run.rs`; `crates/opi-eval/tests/native_driver.rs`. Full surfaces/checks in sidecar. | met | — |
| P19-R72 | AUTH-003; PHASE-001 to PHASE-006 | Registered finite three-family/two-Agent scope remains in the Independent Companion. No Learning/Promotion/Agent Core activation, extra provider SDK or hosted worker path is introduced; this audit does not authorize Phase exit. Evidence: `crates/opi-eval/tests/experiment_contract.rs::independent_companion_dependency_boundary`. Full surfaces/checks in sidecar. | met | — |

## Standards Review

The Independent Companion boundary is enforced by manifest inspection and `independent_companion_dependency_boundary`: unpublished `opi-eval` has no Opi crate dependency in any dependency class, and no runtime registration/reverse activation. Workspace-owned version/MSRV metadata remains unchanged. `lib.rs` exposes provisional CLI/resolution modules; durable state, adapters, supervision, capture, bundle, report and regrade remain private.

Rust ownership uses typed outcomes/errors and a shared process supervisor. OS process-tree/capture FFI stays isolated and justified. `cargo fmt --check --all`, scoped Clippy with warnings denied and scoped rustdoc with warnings denied pass. The bounds failure in P19-AUD-002 is an exception to the required bounded failure behavior. CLI/experiment/material/report 0.x identities, bilingual guides, callers/fixtures and Unreleased reflect current contracts; the documentation checker passes in the faithful Linux export. No dependency or production change was made by this audit.

## Spec Review

Current production paths implement explicit selected native handoff, result/Agent/task/artifact/runner binding, versioned immutable plans, resource preflight, one final settlement classifier, offline seal verification/report reconstruction and a separate verifier-only regrade entry. Tests discriminate ordinary identity substitution, stale job/task, oracle attribution, control mismatch, seal mutation, containment, provider failures, coverage loss and recovery/replacement preservation.

The shared non-live URL guard violates P19-AC-011. The post-allocation trial cap violates P19-AC-010. Actual three-family witnesses, pinned native client matrices, native verifier response and Docker cleanup evidence remain unavailable within the independent input boundary. The six-trial real operator gate is also unavailable, and §8 does not allow fixture success or prior conclusion prose to close it. No Phase-exit approval follows from this audit.

## Security, Invariants, Integration, Test Quality, and Residuals

- Security/authority: P19-AUD-001 reproduces remote URL admission in both non-live modes. Pier has a stricter fragment-rejecting endpoint allowlist; no actual remote call or universal egress exploit is claimed. Selected credential staging/removal, private first-sink capture, closed diagnostics, exact runtime preflight and canary failure stop/seal refusal have current discriminating tests.
- Invariants: P19-AUD-002 demonstrates the declared cap cannot bound plan allocation. Intent-before-effect, one sealed native classifier and frozen planned coverage survive failed/unsealed/missing bundles; invalid provider/infra observations do not become Agent scores. Recovery is read-only; replacement preserves the old group and obtains fresh IDs.
- Integration: shared `AgentAdapter` has Opi/pi consumers; `BenchmarkAdapter` has TB2.1/TB3/DeepSWE consumers. Exact current-import negative tests exercise their production consumers. Harbor/Pier integration tests use controlled environments, not a real upstream runner/Docker proof. Such limits are attached to the actual native requirements rather than hidden by fixture passes.
- Test quality: 403 Rust tests pass across 14 test groups; 11 native/material tests are ignored. Python discovery runs 273 tests, OK with one skip. Two declared `--ignored --exact` filters run zero tests because the matching fixture tests are non-ignored; the actual native tests have different names. Rust doctests run zero examples and only establish successful doc compilation. No zero/ignored filter is counted as native acceptance.
- Residuals: normal native scoring uses the selected Agent path, while oracle/nop remain explicit diagnostics and legacy fixture reads retain historical classification. No alternate default/oracle score owner, newly exposed plugin SDK or in-scope TODO/unimplemented placeholder was found in the inspected production surfaces. Future-facing comments in existing private modules do not establish runtime acceptance. External witness/live proof and full workspace gates remain explicit limitations.

## Minimum-change Conformance

The allowlisted planning notes were checked against current consumers, config/state ownership and ceilings. “Net deletion” below records absence of the superseded mechanism in current source; historical line-count/deletion claims are not assessed. Behavioral failures are normal findings, not a separate historical axis.

| Task | Status | Reuse, seam consumers and evidence | Net deletion / residual glue / ceiling |
|---|---|---|---|
| 19.1 | conforming | Reuses run_trial, benchmark result importer, Agent settlement, lifecycle, bundle and report. Production: three benchmark adapters, both Agent adapters, native run/report. Nonproduction: substitution/outcome/conformance tests. | No host-Agent then independent default/oracle score route in current normal native branch; private Harbor Agent entry and typed binding remain. Actual witness/live proof stays open. |
| 19.2 | triggered | Reuses resolved experiment and selected connection projection. Production: Opi/pi integrations on all fixed APIs. Nonproduction: projection tests and ignored exact-client matrices. | One explicit connection/config owner replaces hardcoded scripted route; client-specific staging remains. Loopback fail-closed ceiling is breached by P19-AUD-001. |
| 19.3 | conforming | Reuses task lifecycle/parser and Harbor result admission. Production: TB3 adapter/native importer/regrade. Nonproduction: controlled collection tests and ignored TB3 witness. | Declared collection/separate verifier glue stays family-specific and private; real artifact sensitivity proof is unavailable. |
| 19.4 | conforming | Reuses task lifecycle/parser and result import for DeepSWE. Production: Pier integration/DeepSWE adapter/regrade. Nonproduction: base/patch rejection and ignored witness. | One committed-patch/pristine-verifier boundary, no embedded dataset/grader or broad generalized lifecycle. Real patch sensitivity proof is unavailable. |
| 19.5 | conforming | Reuses native materials, pin/inventory validation, supervisor and external-lock helpers. Production: independent prepare and thin CI material caller. Nonproduction: cold build/pi-only/drift/acquisition cases. | Local origin does not require upload receipt/repo fixtures; provenance and CI transport wrapper remain. No source fetch/config mutation of Agent Core. |
| 19.6 | triggered | Reuses resolved trials, corpus inventory/resource observations and run admission. Production: public plan/config run/frozen run. Nonproduction: subset/full/resource/tamper tests. | One frozen plan with required task/resource state; no speculative scheduler. Enforceable max_trials ceiling fails on large expansion (P19-AUD-002). |
| 19.7 | conforming | Reuses bundle verify, sealed interpretation and one ReportBuilder. Production: verify/report/native settlement/regrade. Nonproduction: denominator/metric/containment/bundle tests. | Byte-only operation is explicit verify; one score classifier/report owner remains; legacy reads are classified rather than relabelled native. |
| 19.8 | not-assessable | Reuses seals/material pins/task lifecycle/supervisor and private integration entry. Production: native regrade for both runner families. Nonproduction: controlled fail-closed cases and ignored real witness. | Fresh verifier-only derived record adds required state, not a second Agent/LLM route. Actual input-sensitive native response/no-provider witness is unavailable. |
| 19.9 | not-assessable | Reuses CancellationToken, ProcessSupervisor, process trees, owned cleanup, durable recovery/replace. Production: run/prepare/regrade/witness cleanup. Nonproduction: Unix cancellation/ownership/recovery and ignored actual Docker witness. | Private exact-root cleanup remains; no global prune/cargo cleanup/new execution owner. Actual Docker orphan proof is unavailable; planning cap failure is recorded under 19.6. |
| 19.10 | not-assessable | Reuses public preparation/conformance, package boundary and offline acceptance validator. Production: local CLI/manual thin CI callers. Nonproduction: Python CI/material/acceptance negatives. | No CI-owned competing scoring policy found; transport/pin wrappers remain. Two stale zero-test scope commands do not close local native proof. |
| 19.11 | not-assessable | Reuses offline verifier/report and generic artifact auditor. Production consumer: supplied operator evidence; nonproduction: constructed six-record validator fixture. | No new runtime seam or extra 18-live-trial requirement; six minimum actual trials and final gate proof were not supplied. Prior audit/acceptance prose is excluded. |

Public surfaces: `cli::{run,prepare,plan,verify,report,regrade,conformance}` are binary entry facades for durable effect/state boundaries; `experiment::{ResolvedExperiment,project_connection,...}` owns parsed/frozen identities and explicit native-client projections. Production consumers are the same-package binary, native runner and two integrations; nonproduction consumers are contract/CLI tests and docs. Agent/benchmark adapter traits and effect/persistence internals are private with shared conformance. No new Agent Core seam/crate/dependency or compatibility layer is justified or introduced here.

## Findings

### P19-AUD-001: URL fragments bypass non-live loopback admission

- Axis: security
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-R60
- Claim: Non-live resolution accepts http://example.invalid#@localhost (and #@127.0.0.1) as loopback although the actual HTTP URL host is example.invalid. The shared guard therefore does not enforce the default remote-route rejection boundary.
- Evidence:
  - `crates/opi-eval/src/experiment.rs:1438`: Authority is split only on / or ?, then rsplit @; a fragment becomes a false host. The same gate at line 738 guards hermetic and native-fixture modes.
  - `crates/opi-eval/src/runner/experiment.rs:1650`: Native handoff retains the selected projected endpoint; prepare_native does not add endpoint validation.
  - `crates/opi-eval/integrations/harbor/opi_eval_harbor.py:426`: Harbor setup copies/verifies projected config but provides no independent URL-fragment/remote-host rejection before selected client dispatch.
  - `Cargo.lock:3674`: Exact locked url 2.5.8 probe returns host=example.invalid, fragment=@localhost; opi-ai Anthropic HTTP request consumes the URL through the HTTP parser.
  - `crates/opi-eval/tests/experiment_contract.rs:700`: Strongest counter-evidence: ordinary remote endpoints are rejected. Private native-fixture CLI probe confirms control exit 1 and fragment exit 0; the test does not cover fragment authorities.
  - `crates/opi-eval/integrations/pier/opi_eval_pier.py:434`: Pier network_allowlist rejects fragments, so this finding does not claim DeepSWE egress bypass or an observed remote request.
  - `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-audit-8mm5zoko\probe-native-loopback.json`: Built binary SHA-256 a84d8017cdb48f6ad26755c9658b2ced24d5dd0deaf4562d07189cd3000a8d55. Hermetic and native-fixture validation accept the fragment; no Agent/provider/network effect was launched.
- Refutation attempted: Ordinary remote rejection, no-live defaults, and Pier fragment validation were checked. They do not defeat the shared guard failure: both hermetic/native-fixture CLI controls reject the plain remote host but accept its fragment variant; Harbor consumes the projection. Locked url 2.5.8 retains the actual remote host.
- Reproduction: Replace base_url in local-paired.toml with http://example.invalid#@localhost; opi-eval validate --config C accepts with exit 0 in hermetic and native-fixture modes.; python3 C:\Users\Luiz\AppData\Local\Temp\opi-phase19-audit-8mm5zoko\probe-loopback.py; python3 C:\Users\Luiz\AppData\Local\Temp\opi-phase19-audit-8mm5zoko\probe-native-loopback.py; Locked-url probe: http://example.invalid#@localhost => host_str Some(example.invalid), fragment Some(@localhost).
- Suggested closure: Parse the full URL with a validated host/authority and fail closed on malformed/credential/fragment ambiguity. Exercise the public non-live CLI with fragment/userinfo/malformed forms and valid IPv4/IPv6; the current parser also rejects valid [::1].
- Confidence: high. Publication status: unverified; remediation owns subsequent confirmation/closure.

### P19-AUD-002: Trial expansion allocates before enforcing max_trials

- Axis: invariants
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-R34
- Claim: A valid positive repetition count can force enormous plan allocation despite max_trials=1 because expand_selection checks the cap only after constructing every trial. The public plan CLI aborts under a bounded memory limit instead of returning the required bound rejection.
- Evidence:
  - `crates/opi-eval/src/runner/plan.rs:437`: Nested selected task/repetition/subject loops push into Vec before the max_trials comparison at line 449.
  - `crates/opi-eval/src/experiment.rs:876`: Positive repetitions up to u32::MAX are admitted.
  - `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-audit-8mm5zoko\probe-trial-bound.json`: One task, two subjects, repetitions 4294967295, max_trials 1, RLIMIT_AS 256 MiB and no core dump: returncode -6 in 0.82s, memory allocation failed, no plan file. Repetitions 2 returns exit 2 with planned count 4 exceeds max_trials 1.
  - `crates/opi-eval/tests/native_driver.rs:8648`: Strongest counter-evidence: cap tests reject small explicit/selected lists before native dispatch; they do not bound cardinality calculation/allocation. The shared run_experiment check is also downstream of expansion.
- Refutation attempted: Small-list max_trials tests and run-level checks are current counter-evidence. The small reproduction gets the correct typed error, while the identical cap with u32::MAX repetitions aborts. All cap checks are downstream of vector expansion and cannot bound it.
- Reproduction: python3 C:\Users\Luiz\AppData\Local\Temp\opi-phase19-audit-8mm5zoko\probe-trial-bound.py under WSL/Linux against the sealed export; Change the fixture from explicit trials to full selection, repetitions=4294967295; execution.max_trials=1; run opi-eval plan under RLIMIT_AS=268435456.
- Suggested closure: Compute checked selected-task × repetition × subject cardinality and enforce the bound before constructing per-trial strings/vector. Add a discriminating large-count CLI refusal and overflow case; no paid/native provider is required.
- Confidence: high. Publication status: unverified; remediation owns subsequent confirmation/closure.

### P19-AUD-003: Mandatory actual native conformance proof is unavailable

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-R21, P19-R24, P19-R27, P19-R31, P19-R52, P19-R55, P19-R57, P19-R61
- Claim: Current deterministic passes do not establish mandatory real TB2.1/TB3/DeepSWE witnesses, actual Opi/pi three-protocol tool loops, real verifier-only regrade, or actual Docker cleanup. Required native filters are ignored and no source-bound raw proof/material was supplied. Two declared ignored filters execute zero tests.
- Evidence:
  - `crates/opi-eval/tests/native_driver.rs:3614`: Three real family artifact/patch witness tests exist at 3614, 3636, 3680 but are ignored pending OPI_EVAL_NATIVE_WITNESS_MATERIAL. They were not executed by the 403-pass suite.
  - `crates/opi-eval/tests/native_driver.rs:3999`: Actual pinned protocol client matrix is ignored pending explicit native client material; projection/server fixture passes are weaker than real-client authenticated tool-loop proof.
  - `crates/opi-eval/tests/native_driver.rs:11251`: Real native_verifier_only_regrade_witness exists and is ignored; native_offline_regrade at 7902 is a non-ignored controlled producer case.
  - `crates/opi-eval/tests/native_driver.rs:10046`: Actual owned resource cancellation test is ignored pending pinned local Docker material; real Unix process tests and controlled Docker ID tests are strongest partial counter-evidence.
  - `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-audit-8mm5zoko\stale-provider-filter.log`: Declared native_provider_failure_report --ignored --exact runs 0 tests, 128 filtered out; real opt-in native_client_provider_failure_report is named differently.
  - `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-audit-8mm5zoko\stale-regrade-filter.log`: Declared native_offline_regrade --ignored --exact runs 0 tests, 128 filtered out.
  - `read_audit_scope.py projection: task 19.1/19.2/19.3/19.4/19.8/19.9/19.10`: These are current mandatory local native obligations, not registered deferrals; no raw witness root was supplied. Existence of opt-in tests does not establish executed acceptance.
- Refutation attempted: Current opt-in witnesses are substantive and inspect native rewards, artifact changes, authenticated tool loops, verifier-only entry and owned resource cleanup. The passing controlled consumer cases demonstrate implementation support, not their executed actual-native proof. Default test ignore counts and the two zero-test declared commands do not satisfy these mandatory predicates; no registered deferral permits treating them as met.
- Reproduction: Run current named native filters with exact pinned local client/witness/regrade material and Docker engine; preserve counts, command/exits, native raw results, artifacts and identities.; cargo test --locked -p opi-eval --test native_driver native_offline_regrade -- --ignored --exact (observed 0 tests); cargo test --locked -p opi-eval --test native_driver native_provider_failure_report -- --ignored --exact (observed 0 tests)
- Suggested closure: Provide or execute the exact current native filters with explicit pinned local material and preserve source/build/client/runtime/task/image identities, nonzero executed counts, command/exit/raw logs, input-sensitive results, no Agent/provider regrade calls and Docker orphan/ownership observations. Correct stale scope filters only through the owning workflow.
- Confidence: high. Publication status: unverified; remediation owns subsequent confirmation/closure.

### P19-AUD-004: Six-trial actual local acceptance cannot be independently verified

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-R14, P19-R18, P19-R22, P19-R25, P19-R28, P19-R35, P19-R63, P19-R64, P19-R65, P19-R66
- Claim: No supplied raw six-trial operator evidence root establishes the registered actual independent-directory WSL/Linux path, both native Agents across all three families, prescribed first connection, frozen pre-effect scope, original seals/provenance, exhaustive Opi audit view and final workspace gates. These mandatory live acceptance predicates remain unassessed.
- Evidence:
  - `crates/opi-eval/scripts/verify-local-acceptance.py`: Production verifier requires live first-connection traces, build/runtime/material/native identity, commands/results, frozen inputs, seals and byte-bound Opi captures. Its implementation is present.
  - `crates/opi-eval/scripts/test_verify_local_acceptance.py:117`: Strongest counter-evidence: positive constructed evidence root plus stub companion exercises validator acceptance/negative mutations; it does not attest six actual benchmark/client executions.
  - `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md: Section 8`: Missing actual evidence explicitly leaves P19-AC-012 unmet; deterministic fixtures and score=0 acceptance are different from actual operator evidence.
  - `read_audit_scope.py projection: task 19.11`: Current scope requires separately authorized operator-supplied evidence and full local workspace gates. No such raw root is named in this invocation/projection.
  - `docs/snapshots/phase19 (path inventory only)`: Committed snapshot inventory provides the state pointer and assurance records, not six original trial bundles. Prior Phase audit/acceptance prose remains excluded; no global claim that evidence never existed is made.
  - `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-audit-8mm5zoko\rust-tests.log`: 403 deterministic tests executed on WSL, 11 ignored; scoped Clippy/rustdoc/format pass. These are current isolated checks rather than preserved actual live product-path/final workspace-union proof.
- Refutation attempted: The offline validator and extensive constructed positive/negative evidence tests are present. Source-bound scoped WSL checks also pass. Neither contains actual original six-trial captures and the final workspace gate union. §8/task 19.11 explicitly require separately authorized operator-supplied real evidence and do not allow prior conclusion prose or fixture scores to substitute. This is availability within the independent scope, not a claim that no past execution existed.
- Reproduction: Supply frozen redacted actual six-trial evidence, then run python3 crates/opi-eval/scripts/verify-local-acceptance.py --root E --out O using fresh output outside E.; Run python3 scripts/opi-artifact-audit.py E/opi-artifact-audit --workspace-root . after byte-bound completeness/cross-repository provenance validation.; Verify actual WSL operator command logs/exits, exact commits/builds/material, failed attempts and all applicable final workspace gates.
- Suggested closure: Supply the frozen redacted actual six-trial root, all failed attempts, original seals/traces/tool effects/native results and repository-qualified provenance; run the real offline validator and exhaustive Opi generic audit view from fresh outputs. Independently establish the actual WSL product path and full final workspace verification union. Any new paid/live run remains an operator-authorized action outside this audit.
- Confidence: high. Publication status: unverified; remediation owns subsequent confirmation/closure.

## Verification Commands

| Command | Result | Requirement/finding |
|---|---|---|
| `python scripts/opi-cargo-cache.py status` and repository cache resolve/lease | PASS; persistent external cache retained | Cache contract |
| `cargo test --locked -p opi-eval --all-targets` | PASS; 403 passed, 0 failed, 11 ignored; 14 test result groups | Requirement sidecar lists exact discriminating cases |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p "test_*.py"` | PASS; Ran 273 tests, OK (skipped=1), 83.170s. Initial shell wrapper returned 1 after unittest success because its post-command exit variable was mangled; the unittest log is the result evidence. | Python integrations/provider/acquisition/CI/acceptance validators |
| `cargo fmt --check --all` | PASS | Standards |
| `cargo clippy --locked -p opi-eval --all-targets -- -D warnings` | PASS | Standards |
| `RUSTDOCFLAGS="-D warnings" cargo doc --locked -p opi-eval --no-deps` | PASS | Standards/API docs |
| `cargo test --locked -p opi-eval --doc` | PASS compilation; 0 doctests executed, no behavioral proof | Documentation limit |
| `python3 scripts/opi-doc-check.py` in faithful Linux archive | PASS. Windows flattened-alias attempt failed; committed tar symlinks restored before passing Linux check. | Bilingual/docs/metadata contracts |
| `python3 probe-loopback.py` / `probe-native-loopback.py` | FAIL boundary: plain remote exit 1, fragment remote exit 0 in both non-live modes | P19-AUD-001 |
| Locked `url` 2.5.8 standalone parser probe | `host_str=example.invalid`; fragment has `@localhost` or `@127.0.0.1` | P19-AUD-001 refutation |
| `python3 probe-trial-bound.py` with RLIMIT_AS=256 MiB | FAIL bound: large expansion SIGABRT/-6; small control correctly returns exit 2 | P19-AUD-002 |
| `cargo test --locked -p opi-eval --test native_driver native_provider_failure_report -- --ignored --exact` | 0 tests executed, 128 filtered; not native acceptance | P19-AUD-003 |
| `cargo test --locked -p opi-eval --test native_driver native_offline_regrade -- --ignored --exact` | 0 tests executed, 128 filtered; not native acceptance | P19-AUD-003 |
| Exact real native family/client/regrade/Docker filters | NOT RUN; explicit pinned material/raw proof not supplied | P19-AUD-003 |
| Actual six-trial verifier/artifact audit and full workspace smoke union | NOT RUN; redacted original evidence root not supplied; this audit uses narrow scoped checks | P19-AUD-004 |

The initial Rust wrapper was interrupted before completion to correct its cache lease quoting; only the completed leased rerun is counted. No tests were added, modified or deleted: **test impact: none** (existing tests retained; private audit probes added outside the repository). Raw logs/probe scripts remain in `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-audit-8mm5zoko`; they are local private evidence, not a fifth assurance member file or guaranteed long-term shared artifact. Exact reproduction cases and results are embedded above. Binary SHA-256 for CLI probes: `a84d8017cdb48f6ad26755c9658b2ced24d5dd0deaf4562d07189cd3000a8d55`.

## Verdict Rationale

FAIL is mechanical: 20 of 71 mandatory predicates are incomplete. Two Major findings reproduce current boundary failures; two Major findings record mandatory unavailable execution/evidence predicates. Passing isolated deterministic checks does not lower these findings or close Phase 19. No product repair, ledger progress, Phase exit, commit or publication is authorized by this audit. The four-file independent member is ready for locked installation; the post-install historical supplement must remain advisory and cannot modify this core.

<!-- opi-audit-history/1
{
  "audit\u005fhead": "2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618",
  "audit\u005frun\u005fid": "phase19-codex-gpt61-2ec1ccb-20261004t023807z",
  "comparison": {
    "comparisons": [
      {
        "cause": "The current string-based authority splitter excludes / and ? but not #, then trusts the suffix after @. An unexamined fragment branch is the supported cause; regression timing is not established.",
        "current\u005fevidence": "Independent current core at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618 reproduces hermetic/native-fixture fragment acceptance while plain remote rejects; locked url 2.5.8 preserves host example.invalid. Harbor copies the projected endpoint; Pier rejects fragments.",
        "current\u005ffinding\u005fid": "P19-AUD-001",
        "prior\u005fclaim": "The immediately superseded run contains no finding about fragment-based loopback URL admission; this is only an omission within that one report, not evidence of when the bug was introduced.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [
          {
            "checked": "Checked remote/fragment/IPv4/IPv6 CLI outputs and binary SHA.",
            "limitations": "Validation only; no actual remote dispatch.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-8mm5zoko\\probe-loopback.json",
            "sha256": "8976aa5a762ae326919081acf93371babb24917f4162b274e4b21f3b92da1e08"
          },
          {
            "checked": "Checked native-fixture remote exit=1 and fragment exit=0, same built binary.",
            "limitations": "No provider, credentials or benchmark execution; distinct Pier guard limits egress claim.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-8mm5zoko\\probe-native-loopback.json",
            "sha256": "f14ab8dee67880c634bf287499a070d96a63bc997e377b590ebead8d72f759d0"
          }
        ],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use parsed URL authority at the owning resolve seam and run non-live CLI fragment/userinfo/malformed/IPv4/IPv6 cases before any Agent/provider effect."
      },
      {
        "cause": "The repaired post-expansion count boundary protects dispatch and small lists; it does not bound selected cardinality/string allocation. This is another branch of the same bound invariant, not recurrence of the old explicit-list behavior.",
        "current\u005fevidence": "Current shared run-level count check and max\u005ftrials\u005fcaps\u005fevery\u005fresolved\u005ftrial\u005flist\u005fbefore\u005feffects reject the old explicit-list case. Current independent large-selection probe still aborts on allocation because expand\u005fselection pushes all trials before its cap at line449.",
        "current\u005ffinding\u005fid": "P19-AUD-002",
        "prior\u005fclaim": "Prior P19-AUD-002 found explicit trial lists bypassing max\u005ftrials; its concrete case is two explicit trials under max\u005ftrials=1. The claim did not test huge selected expansion allocation.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-f60e458-20261003t041015z",
            "finding\u005fid": "P19-AUD-002",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Parsed 10 preserved case records with argv/exits/outputs; seven quoted-bound admissions, explicit max\u005ftrials case and outer-timed doctor are present.",
            "limitations": "Historical reproductions only; original build/source/material identity is attributed to the superseded report and not adopted as current conformance. Current conclusions use independently executed tests/probes.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\prepare-probes\\results.json",
            "sha256": "d9cd0d5548b55cd02207d14519052725a321a50ab0fb5549dc1f1260927cc9e3"
          },
          {
            "checked": "Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar.",
            "limitations": "Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-8mm5zoko\\rust-tests.log",
            "sha256": "e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa"
          },
          {
            "checked": "Large case RLIMIT\u005fAS 256MiB aborts -6; small repetitions2 returns typed exit2; exact binary digest recorded.",
            "limitations": "Private no-provider hermetic CLI probe; not actual native execution.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-8mm5zoko\\probe-trial-bound.json",
            "sha256": "26c4f6ce29bfac0b15e7f1ff425f7f3e9de003fd10ebbfbd62c04b8185351666"
          }
        ],
        "relationship": "missed-branch",
        "repair\u005fcommits": [
          "0fa995de504bcb9854e8ee841d54d99579da1bd4"
        ],
        "verification": "Check cardinality/overflow and max\u005ftrials before vector allocation; retain the explicit-list refusal test."
      },
      {
        "cause": "This is continued evidence availability and stale verification-command coverage, not a reproduced failure of the actual native graders/clients. Prior default-suite proof cannot substitute for the new source/client identities.",
        "current\u005fevidence": "Current default suite executes 403 with 11 ignored. Actual native harnesses remain present; declared native\u005fprovider\u005ffailure\u005freport and native\u005foffline\u005fregrade --ignored --exact execute0 while real ignored witnesses have other names. No permitted raw actual-native root was available to the independent current core.",
        "current\u005ffinding\u005fid": "P19-AUD-003",
        "prior\u005fclaim": "Prior P19-AUD-008 likewise left actual native graders/client protocol loops/regrade/Docker cleanup unassessed because its default suite ignored them and no raw witness root was supplied.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-f60e458-20261003t041015z",
            "finding\u005fid": "P19-AUD-008",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Checked exact old default-suite result groups; prior report count 390 and 11 ignored is supported.",
            "limitations": "Earlier sealed f60e458 head, not current actual-native proof; current pi/source and remediation changes invalidate assuming old client applicability.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\checks\\rust.log",
            "sha256": "424c0fc0e82b4c77b698a2d38dd7d88acd08126e08d2fefad7cc030f634f3901"
          },
          {
            "checked": "Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar.",
            "limitations": "Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-8mm5zoko\\rust-tests.log",
            "sha256": "e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa"
          },
          {
            "checked": "0 tests executed,128 filtered.",
            "limitations": "Only demonstrates the stale exact ignored filter; no actual client provider-failure predicate executed.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-8mm5zoko\\stale-provider-filter.log",
            "sha256": "795f03a6e6c8c2b4c1242bbcd8bc82211935e2dacb3c11cf09621bd538adb326"
          },
          {
            "checked": "0 tests executed,128 filtered.",
            "limitations": "Only demonstrates stale exact ignored filter; no actual native regrade predicate executed.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-8mm5zoko\\stale-regrade-filter.log",
            "sha256": "289e501508e775e3c4c5d2a1eb12bc19b80db3519e58a4fbbd7f346def3b79f4"
          }
        ],
        "relationship": "evidence-gap",
        "repair\u005fcommits": [],
        "verification": "Supply exact current pinned material and retained native witness/client/regrade/owned-cleanup proofs; rerun invalidated exact native filters with nonzero execution counts."
      },
      {
        "cause": "The required operator-supplied raw root is absent from both bounded audit inputs. No claim of global absence or failed real-provider trials follows, and historical prose is not proof.",
        "current\u005fevidence": "Current core has no supplied original six-trial root and leaves 20 mandatory predicates incomplete overall. Current verifier implements eligible graded outcomes and exhaustive provenance/view checks, but current positive Python records are constructed. WSL isolated checks do not prove the actual product path/final union.",
        "current\u005ffinding\u005fid": "P19-AUD-004",
        "prior\u005fclaim": "Prior P19-AUD-009 states six-trial real operator evidence, pre-effect freeze, raw provenance and final gate union were unavailable within its supplied independent inputs; it does not establish a past real acceptance result.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-f60e458-20261003t041015z",
            "finding\u005fid": "P19-AUD-009",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Unittest reports Ran 273 tests and OK (skipped=1), with controlled integration and acceptance assertions.",
            "limitations": "Constructed acceptance/native-environment tests do not establish actual real-client/native benchmark execution; wrapper post-command exit failed after unittest success.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-8mm5zoko\\python-tests.log",
            "sha256": "bfe4f638e90cd44e1aa000dd3f130433927a19cffe724c8e6d69c56e7f94045c"
          },
          {
            "checked": "Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar.",
            "limitations": "Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-8mm5zoko\\rust-tests.log",
            "sha256": "e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa"
          }
        ],
        "relationship": "evidence-gap",
        "repair\u005fcommits": [],
        "verification": "Supply source-bound redacted originals, frozen scope, all attempts, exact six Opi/pi native trials, prescribed connection/tool loops, exhaustive byte-bound Opi view/upstream provenance and final workspace/WSL receipts; validate offline."
      },
      {
        "cause": "R34 still has a distinct large-allocation failure; this row preserves only strict type admission, not universal bound correctness.",
        "current\u005fevidence": "Quoted integer bounds: Current optional\u005fu64 rejects every present non-positive/non-integer shape rather than turning String into None. resolve\u005frejects\u005fwrongly\u005ftyped\u005fexecution\u005fbounds and nonpositive/unenforceable filters pass in the independent 403 suite. Current evidence was established before historical exposure at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Prior Nonempty TOML strings at integer execution-limit fields validate as absence. A requested usage/output/time/trial bound can therefore be erased instead of rejected or enforced before effects.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-f60e458-20261003t041015z",
            "finding\u005fid": "P19-AUD-001",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Parsed 10 preserved case records with argv/exits/outputs; seven quoted-bound admissions, explicit max\u005ftrials case and outer-timed doctor are present.",
            "limitations": "Historical reproductions only; original build/source/material identity is attributed to the superseded report and not adopted as current conformance. Current conclusions use independently executed tests/probes.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\prepare-probes\\results.json",
            "sha256": "d9cd0d5548b55cd02207d14519052725a321a50ab0fb5549dc1f1260927cc9e3"
          },
          {
            "checked": "Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar.",
            "limitations": "Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-8mm5zoko\\rust-tests.log",
            "sha256": "e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "0fa995de504bcb9854e8ee841d54d99579da1bd4"
        ],
        "verification": "Retain the current discriminating case and rerun it only if its owning contract/dependencies change. This advisory row does not close any current finding."
      },
      {
        "cause": "The old explicit-list bypass is repaired. New P19-AUD-002 tracks the separate pre-allocation selected-expansion branch.",
        "current\u005fevidence": "Explicit max\u005ftrials enforcement: Current run\u005fexperiment checks every resolved trial list before acquisition/root reservation/dispatch, and explicit/selection small-list cap tests pass. Current evidence was established before historical exposure at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Prior The max\u005ftrials admission check applies only to selection expansion. A public explicit-trial config with max\u005ftrials=1 executes and seals both declared trials.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-f60e458-20261003t041015z",
            "finding\u005fid": "P19-AUD-002",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Parsed 10 preserved case records with argv/exits/outputs; seven quoted-bound admissions, explicit max\u005ftrials case and outer-timed doctor are present.",
            "limitations": "Historical reproductions only; original build/source/material identity is attributed to the superseded report and not adopted as current conformance. Current conclusions use independently executed tests/probes.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\prepare-probes\\results.json",
            "sha256": "d9cd0d5548b55cd02207d14519052725a321a50ab0fb5549dc1f1260927cc9e3"
          },
          {
            "checked": "Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar.",
            "limitations": "Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-8mm5zoko\\rust-tests.log",
            "sha256": "e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "0fa995de504bcb9854e8ee841d54d99579da1bd4"
        ],
        "verification": "Retain the current discriminating case and rerun it only if its owning contract/dependencies change. This advisory row does not close any current finding."
      },
      {
        "cause": "The original raw unbounded observer branch is absent from inspected current callers; controlled observer tests do not establish all actual engine/platform semantics.",
        "current\u005fevidence": "Bounded doctor/prepare observers: Current doctor uses one 12s operation deadline and engine/version/df/context/GPU observers route through supervised bounded helpers; prepare propagates cancellation. Current stalled observer and cleanup tests pass. Current evidence was established before historical exposure at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Prior Raw synchronous engine/version/df/context/GPU probes have no deadline, capture cap or cancellation. A stalled selected observer blocks the public operation and on the current-thread runtime prevents preparation cancellation from being polled.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-f60e458-20261003t041015z",
            "finding\u005fid": "P19-AUD-003",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Parsed 10 preserved case records with argv/exits/outputs; seven quoted-bound admissions, explicit max\u005ftrials case and outer-timed doctor are present.",
            "limitations": "Historical reproductions only; original build/source/material identity is attributed to the superseded report and not adopted as current conformance. Current conclusions use independently executed tests/probes.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\prepare-probes\\results.json",
            "sha256": "d9cd0d5548b55cd02207d14519052725a321a50ab0fb5549dc1f1260927cc9e3"
          },
          {
            "checked": "Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar.",
            "limitations": "Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-8mm5zoko\\rust-tests.log",
            "sha256": "e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "0fa995de504bcb9854e8ee841d54d99579da1bd4"
        ],
        "verification": "Retain the current discriminating case and rerun it only if its owning contract/dependencies change. This advisory row does not close any current finding."
      },
      {
        "cause": "Pre-effect frozen contract replaces reconstructing report identity only from surviving seals; missing evidence remains explicit.",
        "current\u005fevidence": "Frozen zero-seal report contract: Current run-control/2 freezes resolved experiment/integrity before intent; report restores topology/native identity without a seal. zero\u005fseal\u005frun\u005fcontrol\u005frestores\u005fnative\u005freport\u005fidentity and planned\u005fcoverage\u005fsurvives\u005ftotal\u005fbundle\u005floss pass. Current evidence was established before historical exposure at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Prior A genuine interruption before any seal publishes counts but drops all declared pair coverage and renders known native benchmark identity as unknown/unknown. Missing pairs cannot be reconstructed from the incomplete pre-effect run-control.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-f60e458-20261003t041015z",
            "finding\u005fid": "P19-AUD-004",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar.",
            "limitations": "Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-8mm5zoko\\rust-tests.log",
            "sha256": "e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "0fa995de504bcb9854e8ee841d54d99579da1bd4"
        ],
        "verification": "Retain the current discriminating case and rerun it only if its owning contract/dependencies change. This advisory row does not close any current finding."
      },
      {
        "cause": "Only current containment invariants are preserved here; actual native input-sensitive regrade remains an evidence gap in current003.",
        "current\u005fevidence": "Regrade source containment: Current resolve\u005ffresh\u005foutput canonicalizes the nearest existing ancestor and refuses source-contained output before writes; regrade\u005foutput\u005fcontainment\u005frejects\u005fcanonical\u005fsource\u005faliases observes preserved source bytes. Current evidence was established before historical exposure at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Prior A fresh --out inside the canonical source's bundle/artifacts is accepted. Regrade then copies task files there before later lifecycle/base/image checks, adding unmanifested source files and invalidating subsequent verify/report, even when grading later rejects.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-f60e458-20261003t041015z",
            "finding\u005fid": "P19-AUD-005",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar.",
            "limitations": "Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-8mm5zoko\\rust-tests.log",
            "sha256": "e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "0fa995de504bcb9854e8ee841d54d99579da1bd4"
        ],
        "verification": "Retain the current discriminating case and rerun it only if its owning contract/dependencies change. This advisory row does not close any current finding."
      },
      {
        "cause": "The substring admission path from old006 is replaced; no old sanitized/Closed claim is adopted and actual live captures remain unassessed.",
        "current\u005fevidence": "Closed stderr diagnostic grammar: Current native capture admits whole records with safe structured cause vocabulary. native\u005fcapture\u005fadmits\u005fonly\u005fclosed\u005fstderr\u005fdiagnostics and private-thinking/credential canary tests pass under both policies. Current evidence was established before historical exposure at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Prior Stderr is classified validated-diagnostic when each line merely contains one allowed token, then the entire original line is written. Arbitrary private/provider payload surrounding a token crosses the first capture sink under a sanitized receipt in both capture policies.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-f60e458-20261003t041015z",
            "finding\u005fid": "P19-AUD-006",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar.",
            "limitations": "Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-8mm5zoko\\rust-tests.log",
            "sha256": "e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "0fa995de504bcb9854e8ee841d54d99579da1bd4"
        ],
        "verification": "Retain the current discriminating case and rerun it only if its owning contract/dependencies change. This advisory row does not close any current finding."
      },
      {
        "cause": "Validator predicate repair is retained, independently of absent actual six-trial operator evidence in current004.",
        "current\u005fevidence": "Eligible graded failures in acceptance: Current acceptance validation follows verified graded settlement and tests valid native nonzero/timeout outcomes while excluding invalid provider/infra substitutions. test\u005fvalid\u005fnative\u005ftask\u005ffailures\u005fcount\u005fas\u005fgraded\u005ftrials passes. Current evidence was established before historical exposure at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Prior The local acceptance verifier rejects every nonzero/timed-out Agent terminal and any native exception before consuming benchmark-valid sealed classification, although eligible real task failures/timeouts may count among the six native trials.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-f60e458-20261003t041015z",
            "finding\u005fid": "P19-AUD-007",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Unittest reports Ran 273 tests and OK (skipped=1), with controlled integration and acceptance assertions.",
            "limitations": "Constructed acceptance/native-environment tests do not establish actual real-client/native benchmark execution; wrapper post-command exit failed after unittest success.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-8mm5zoko\\python-tests.log",
            "sha256": "bfe4f638e90cd44e1aa000dd3f130433927a19cffe724c8e6d69c56e7f94045c"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "0fa995de504bcb9854e8ee841d54d99579da1bd4"
        ],
        "verification": "Retain the current discriminating case and rerun it only if its owning contract/dependencies change. This advisory row does not close any current finding."
      },
      {
        "cause": "The missing-diagnosis branch is removed without a new inventory ledger. This row is source/control-test evidence, not actual native material preparation acceptance.",
        "current\u005fevidence": "Complete selected doctor inventory: Current doctor inventories selected corpus/profile/uv/runner/runtime/integration/scripted provider/capture helper paths from selected preparation inputs; current preflight/refusal tests and doc check pass. Current evidence was established before historical exposure at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Prior Doctor observes only the static lock and selected Agent paths, omitting selected corpus/profile, uv, runner project/lock/interpreter/entrypoint, integrations/provider and capture helper from missing-input diagnosis.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-f60e458-20261003t041015z",
            "finding\u005fid": "P19-AUD-010",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar.",
            "limitations": "Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-8mm5zoko\\rust-tests.log",
            "sha256": "e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "0fa995de504bcb9854e8ee841d54d99579da1bd4"
        ],
        "verification": "Retain the current discriminating case and rerun it only if its owning contract/dependencies change. This advisory row does not close any current finding."
      },
      {
        "cause": "Typed diagnostic repair is retained at the importer seam; actual pinned clients exercising all wires remains open in current003.",
        "current\u005fevidence": "All three typed Pi truncation causes: Current Pi importer recognizes the exact Anthropic, Responses and Completions truncation vocabulary; pi\u005fprotocol\u005ftruncations\u005fclassify\u005ftyped\u005facross\u005fall\u005fthree\u005fwires passes. Current evidence was established before historical exposure at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Prior Pi's native importer recognizes only the Completions truncation string as provider-protocol-error; the exact Anthropic and Responses native protocol strings acknowledged by the client matrix fall through to generic provider-failure.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-f60e458-20261003t041015z",
            "finding\u005fid": "P19-AUD-011",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar.",
            "limitations": "Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-8mm5zoko\\rust-tests.log",
            "sha256": "e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "0fa995de504bcb9854e8ee841d54d99579da1bd4"
        ],
        "verification": "Retain the current discriminating case and rerun it only if its owning contract/dependencies change. This advisory row does not close any current finding."
      }
    ],
    "limitations": "Earlier actual native/operator roots are not established by the permitted superseded record; its named default/probe logs cannot prove current native/live conformance. Current pi/runtime/source changed, so old client applicability cannot be assumed. Repair commit 0fa995de504bcb9854e8ee841d54d99579da1bd4 is identified from scoped code history and changed surfaces; commit-level success assertions are not accepted as conformance evidence. No later remediation ledger/report or other historical run is used. No regression chronology is proved for the fragment or allocation branches. No additional defect is introduced into the frozen core after history exposure.",
    "schema": "opi-audit-history/1",
    "scope": "Only the immediately superseded same-pair run phase19-codex-gpt61-f60e458-20261003t041015z moved by this complete was compared. Its exact report/findings bytes match committed active paths at 3e1301178c471499f0fc36d4a8b1cd1f467cef7d. No older run, sibling member, raw ledger or other remediation report was ingested; current core remains frozen at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618.",
    "sources": [
      {
        "audit\u005frun\u005fid": "phase19-codex-gpt61-f60e458-20261003t041015z",
        "commit": "3e1301178c471499f0fc36d4a8b1cd1f467cef7d",
        "id": "prior-report",
        "path": "docs/snapshots/phase19/assurance/audit.codex.gpt61.md",
        "sha256": "4acf8018dbab993cdd94367e3241fdc88783487ff8e09a694b011e69701ed608"
      },
      {
        "audit\u005frun\u005fid": "phase19-codex-gpt61-f60e458-20261003t041015z",
        "commit": "3e1301178c471499f0fc36d4a8b1cd1f467cef7d",
        "id": "prior-findings",
        "path": "docs/snapshots/phase19/assurance/audit.codex.gpt61.findings.jsonl",
        "sha256": "37f779a15dad2250e807239cc4b4718256a117402fdba496fc3da8452e47c28a"
      }
    ],
    "status": "compared"
  },
  "core": {
    "findings\u005fsha256": "bd454f15884f80e537772452aabc05593d73763e5f4500326edbf13e8d018bc4",
    "meta\u005fsha256": "a556820d5b47f9ea82c3c6a15cde7d143f383e28cb96b58568cea14269f72c28",
    "report\u005fbytes": 60923,
    "report\u005fsha256": "cd62651e2c9bb1a3c5b2e4e9446a583b22314ebfb8d87a3be51b8ac43d2c7972",
    "requirements\u005fsha256": "aadfa9bb5e6d796876619a80cf60cbe40e983ccb66b051b5c8ca7c33f1f8b0d5"
  },
  "schema": "opi-audit-history/1"
}
-->

## Post-audit historical reference — advisory

This section was appended after installation of the independent core. It cannot change findings, requirements, independence or verdict. Historical closure claims require current independent verification.

- Owning audit run: phase19&#45;codex&#45;gpt61&#45;2ec1ccb&#45;20261004t023807z
- Audit head: 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618
- Core report bytes: 60923
- Core report SHA-256: cd62651e2c9bb1a3c5b2e4e9446a583b22314ebfb8d87a3be51b8ac43d2c7972
- Current findings SHA-256: bd454f15884f80e537772452aabc05593d73763e5f4500326edbf13e8d018bc4
- Comparison status: compared
- Inspected history: Only the immediately superseded same&#45;pair run phase19&#45;codex&#45;gpt61&#45;f60e458&#45;20261003t041015z moved by this complete was compared&#46; Its exact report/findings bytes match committed active paths at 3e1301178c471499f0fc36d4a8b1cd1f467cef7d&#46; No older run, sibling member, raw ledger or other remediation report was ingested; current core remains frozen at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618&#46;
- Limitations: Earlier actual native/operator roots are not established by the permitted superseded record; its named default/probe logs cannot prove current native/live conformance&#46; Current pi/runtime/source changed, so old client applicability cannot be assumed&#46; Repair commit 0fa995de504bcb9854e8ee841d54d99579da1bd4 is identified from scoped code history and changed surfaces; commit&#45;level success assertions are not accepted as conformance evidence&#46; No later remediation ledger/report or other historical run is used&#46; No regression chronology is proved for the fragment or allocation branches&#46; No additional defect is introduced into the frozen core after history exposure&#46;

### Pinned historical sources

- prior&#45;report: docs/snapshots/phase19/assurance/audit&#46;codex&#46;gpt61&#46;md; commit 3e1301178c471499f0fc36d4a8b1cd1f467cef7d; SHA-256 4acf8018dbab993cdd94367e3241fdc88783487ff8e09a694b011e69701ed608; audit run phase19&#45;codex&#45;gpt61&#45;f60e458&#45;20261003t041015z
- prior&#45;findings: docs/snapshots/phase19/assurance/audit&#46;codex&#46;gpt61&#46;findings&#46;jsonl; commit 3e1301178c471499f0fc36d4a8b1cd1f467cef7d; SHA-256 37f779a15dad2250e807239cc4b4718256a117402fdba496fc3da8452e47c28a; audit run phase19&#45;codex&#45;gpt61&#45;f60e458&#45;20261003t041015z

### Comparison 1: P19&#45;AUD&#45;001

- Relationship: new&#45;omission
- Prior claim (attributed): The immediately superseded run contains no finding about fragment&#45;based loopback URL admission; this is only an omission within that one report, not evidence of when the bug was introduced&#46;
- Current evidence: Independent current core at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618 reproduces hermetic/native&#45;fixture fragment acceptance while plain remote rejects; locked url 2&#46;5&#46;8 preserves host example&#46;invalid&#46; Harbor copies the projected endpoint; Pier rejects fragments&#46;
- Cause / missed boundary: The current string&#45;based authority splitter excludes / and ? but not &#35;, then trusts the suffix after @&#46; An unexamined fragment branch is the supported cause; regression timing is not established&#46;
- Suggested current verification: Use parsed URL authority at the owning resolve seam and run non&#45;live CLI fragment/userinfo/malformed/IPv4/IPv6 cases before any Agent/provider effect&#46;
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;8mm5zoko&#92;probe&#45;loopback&#46;json; SHA-256 8976aa5a762ae326919081acf93371babb24917f4162b274e4b21f3b92da1e08; checked: Checked remote/fragment/IPv4/IPv6 CLI outputs and binary SHA&#46;; limits: Validation only; no actual remote dispatch&#46;
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;8mm5zoko&#92;probe&#45;native&#45;loopback&#46;json; SHA-256 f14ab8dee67880c634bf287499a070d96a63bc997e377b590ebead8d72f759d0; checked: Checked native&#45;fixture remote exit=1 and fragment exit=0, same built binary&#46;; limits: No provider, credentials or benchmark execution; distinct Pier guard limits egress claim&#46;

### Comparison 2: P19&#45;AUD&#45;002

- Relationship: missed&#45;branch
- Prior claim (attributed): Prior P19&#45;AUD&#45;002 found explicit trial lists bypassing max&#95;trials; its concrete case is two explicit trials under max&#95;trials=1&#46; The claim did not test huge selected expansion allocation&#46;
- Current evidence: Current shared run&#45;level count check and max&#95;trials&#95;caps&#95;every&#95;resolved&#95;trial&#95;list&#95;before&#95;effects reject the old explicit&#45;list case&#46; Current independent large&#45;selection probe still aborts on allocation because expand&#95;selection pushes all trials before its cap at line449&#46;
- Cause / missed boundary: The repaired post&#45;expansion count boundary protects dispatch and small lists; it does not bound selected cardinality/string allocation&#46; This is another branch of the same bound invariant, not recurrence of the old explicit&#45;list behavior&#46;
- Suggested current verification: Check cardinality/overflow and max&#95;trials before vector allocation; retain the explicit&#45;list refusal test&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;f60e458&#45;20261003t041015z / P19&#45;AUD&#45;002; source prior&#45;findings
- Repair commits: 0fa995de504bcb9854e8ee841d54d99579da1bd4
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;prepare&#45;probes&#92;results&#46;json; SHA-256 d9cd0d5548b55cd02207d14519052725a321a50ab0fb5549dc1f1260927cc9e3; checked: Parsed 10 preserved case records with argv/exits/outputs; seven quoted&#45;bound admissions, explicit max&#95;trials case and outer&#45;timed doctor are present&#46;; limits: Historical reproductions only; original build/source/material identity is attributed to the superseded report and not adopted as current conformance&#46; Current conclusions use independently executed tests/probes&#46;
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;8mm5zoko&#92;rust&#45;tests&#46;log; SHA-256 e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa; checked: Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar&#46;; limits: Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed&#46;
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;8mm5zoko&#92;probe&#45;trial&#45;bound&#46;json; SHA-256 26c4f6ce29bfac0b15e7f1ff425f7f3e9de003fd10ebbfbd62c04b8185351666; checked: Large case RLIMIT&#95;AS 256MiB aborts &#45;6; small repetitions2 returns typed exit2; exact binary digest recorded&#46;; limits: Private no&#45;provider hermetic CLI probe; not actual native execution&#46;

### Comparison 3: P19&#45;AUD&#45;003

- Relationship: evidence&#45;gap
- Prior claim (attributed): Prior P19&#45;AUD&#45;008 likewise left actual native graders/client protocol loops/regrade/Docker cleanup unassessed because its default suite ignored them and no raw witness root was supplied&#46;
- Current evidence: Current default suite executes 403 with 11 ignored&#46; Actual native harnesses remain present; declared native&#95;provider&#95;failure&#95;report and native&#95;offline&#95;regrade &#45;&#45;ignored &#45;&#45;exact execute0 while real ignored witnesses have other names&#46; No permitted raw actual&#45;native root was available to the independent current core&#46;
- Cause / missed boundary: This is continued evidence availability and stale verification&#45;command coverage, not a reproduced failure of the actual native graders/clients&#46; Prior default&#45;suite proof cannot substitute for the new source/client identities&#46;
- Suggested current verification: Supply exact current pinned material and retained native witness/client/regrade/owned&#45;cleanup proofs; rerun invalidated exact native filters with nonzero execution counts&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;f60e458&#45;20261003t041015z / P19&#45;AUD&#45;008; source prior&#45;findings
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;checks&#92;rust&#46;log; SHA-256 424c0fc0e82b4c77b698a2d38dd7d88acd08126e08d2fefad7cc030f634f3901; checked: Checked exact old default&#45;suite result groups; prior report count 390 and 11 ignored is supported&#46;; limits: Earlier sealed f60e458 head, not current actual&#45;native proof; current pi/source and remediation changes invalidate assuming old client applicability&#46;
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;8mm5zoko&#92;rust&#45;tests&#46;log; SHA-256 e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa; checked: Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar&#46;; limits: Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed&#46;
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;8mm5zoko&#92;stale&#45;provider&#45;filter&#46;log; SHA-256 795f03a6e6c8c2b4c1242bbcd8bc82211935e2dacb3c11cf09621bd538adb326; checked: 0 tests executed,128 filtered&#46;; limits: Only demonstrates the stale exact ignored filter; no actual client provider&#45;failure predicate executed&#46;
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;8mm5zoko&#92;stale&#45;regrade&#45;filter&#46;log; SHA-256 289e501508e775e3c4c5d2a1eb12bc19b80db3519e58a4fbbd7f346def3b79f4; checked: 0 tests executed,128 filtered&#46;; limits: Only demonstrates stale exact ignored filter; no actual native regrade predicate executed&#46;

### Comparison 4: P19&#45;AUD&#45;004

- Relationship: evidence&#45;gap
- Prior claim (attributed): Prior P19&#45;AUD&#45;009 states six&#45;trial real operator evidence, pre&#45;effect freeze, raw provenance and final gate union were unavailable within its supplied independent inputs; it does not establish a past real acceptance result&#46;
- Current evidence: Current core has no supplied original six&#45;trial root and leaves 20 mandatory predicates incomplete overall&#46; Current verifier implements eligible graded outcomes and exhaustive provenance/view checks, but current positive Python records are constructed&#46; WSL isolated checks do not prove the actual product path/final union&#46;
- Cause / missed boundary: The required operator&#45;supplied raw root is absent from both bounded audit inputs&#46; No claim of global absence or failed real&#45;provider trials follows, and historical prose is not proof&#46;
- Suggested current verification: Supply source&#45;bound redacted originals, frozen scope, all attempts, exact six Opi/pi native trials, prescribed connection/tool loops, exhaustive byte&#45;bound Opi view/upstream provenance and final workspace/WSL receipts; validate offline&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;f60e458&#45;20261003t041015z / P19&#45;AUD&#45;009; source prior&#45;findings
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;8mm5zoko&#92;python&#45;tests&#46;log; SHA-256 bfe4f638e90cd44e1aa000dd3f130433927a19cffe724c8e6d69c56e7f94045c; checked: Unittest reports Ran 273 tests and OK &#40;skipped=1&#41;, with controlled integration and acceptance assertions&#46;; limits: Constructed acceptance/native&#45;environment tests do not establish actual real&#45;client/native benchmark execution; wrapper post&#45;command exit failed after unittest success&#46;
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;8mm5zoko&#92;rust&#45;tests&#46;log; SHA-256 e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa; checked: Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar&#46;; limits: Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed&#46;

### Comparison 5: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Prior Nonempty TOML strings at integer execution&#45;limit fields validate as absence&#46; A requested usage/output/time/trial bound can therefore be erased instead of rejected or enforced before effects&#46;
- Current evidence: Quoted integer bounds: Current optional&#95;u64 rejects every present non&#45;positive/non&#45;integer shape rather than turning String into None&#46; resolve&#95;rejects&#95;wrongly&#95;typed&#95;execution&#95;bounds and nonpositive/unenforceable filters pass in the independent 403 suite&#46; Current evidence was established before historical exposure at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618&#46;
- Cause / missed boundary: R34 still has a distinct large&#45;allocation failure; this row preserves only strict type admission, not universal bound correctness&#46;
- Suggested current verification: Retain the current discriminating case and rerun it only if its owning contract/dependencies change&#46; This advisory row does not close any current finding&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;f60e458&#45;20261003t041015z / P19&#45;AUD&#45;001; source prior&#45;findings
- Repair commits: 0fa995de504bcb9854e8ee841d54d99579da1bd4
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;prepare&#45;probes&#92;results&#46;json; SHA-256 d9cd0d5548b55cd02207d14519052725a321a50ab0fb5549dc1f1260927cc9e3; checked: Parsed 10 preserved case records with argv/exits/outputs; seven quoted&#45;bound admissions, explicit max&#95;trials case and outer&#45;timed doctor are present&#46;; limits: Historical reproductions only; original build/source/material identity is attributed to the superseded report and not adopted as current conformance&#46; Current conclusions use independently executed tests/probes&#46;
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;8mm5zoko&#92;rust&#45;tests&#46;log; SHA-256 e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa; checked: Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar&#46;; limits: Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed&#46;

### Comparison 6: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Prior The max&#95;trials admission check applies only to selection expansion&#46; A public explicit&#45;trial config with max&#95;trials=1 executes and seals both declared trials&#46;
- Current evidence: Explicit max&#95;trials enforcement: Current run&#95;experiment checks every resolved trial list before acquisition/root reservation/dispatch, and explicit/selection small&#45;list cap tests pass&#46; Current evidence was established before historical exposure at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618&#46;
- Cause / missed boundary: The old explicit&#45;list bypass is repaired&#46; New P19&#45;AUD&#45;002 tracks the separate pre&#45;allocation selected&#45;expansion branch&#46;
- Suggested current verification: Retain the current discriminating case and rerun it only if its owning contract/dependencies change&#46; This advisory row does not close any current finding&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;f60e458&#45;20261003t041015z / P19&#45;AUD&#45;002; source prior&#45;findings
- Repair commits: 0fa995de504bcb9854e8ee841d54d99579da1bd4
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;prepare&#45;probes&#92;results&#46;json; SHA-256 d9cd0d5548b55cd02207d14519052725a321a50ab0fb5549dc1f1260927cc9e3; checked: Parsed 10 preserved case records with argv/exits/outputs; seven quoted&#45;bound admissions, explicit max&#95;trials case and outer&#45;timed doctor are present&#46;; limits: Historical reproductions only; original build/source/material identity is attributed to the superseded report and not adopted as current conformance&#46; Current conclusions use independently executed tests/probes&#46;
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;8mm5zoko&#92;rust&#45;tests&#46;log; SHA-256 e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa; checked: Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar&#46;; limits: Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed&#46;

### Comparison 7: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Prior Raw synchronous engine/version/df/context/GPU probes have no deadline, capture cap or cancellation&#46; A stalled selected observer blocks the public operation and on the current&#45;thread runtime prevents preparation cancellation from being polled&#46;
- Current evidence: Bounded doctor/prepare observers: Current doctor uses one 12s operation deadline and engine/version/df/context/GPU observers route through supervised bounded helpers; prepare propagates cancellation&#46; Current stalled observer and cleanup tests pass&#46; Current evidence was established before historical exposure at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618&#46;
- Cause / missed boundary: The original raw unbounded observer branch is absent from inspected current callers; controlled observer tests do not establish all actual engine/platform semantics&#46;
- Suggested current verification: Retain the current discriminating case and rerun it only if its owning contract/dependencies change&#46; This advisory row does not close any current finding&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;f60e458&#45;20261003t041015z / P19&#45;AUD&#45;003; source prior&#45;findings
- Repair commits: 0fa995de504bcb9854e8ee841d54d99579da1bd4
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;prepare&#45;probes&#92;results&#46;json; SHA-256 d9cd0d5548b55cd02207d14519052725a321a50ab0fb5549dc1f1260927cc9e3; checked: Parsed 10 preserved case records with argv/exits/outputs; seven quoted&#45;bound admissions, explicit max&#95;trials case and outer&#45;timed doctor are present&#46;; limits: Historical reproductions only; original build/source/material identity is attributed to the superseded report and not adopted as current conformance&#46; Current conclusions use independently executed tests/probes&#46;
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;8mm5zoko&#92;rust&#45;tests&#46;log; SHA-256 e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa; checked: Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar&#46;; limits: Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed&#46;

### Comparison 8: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Prior A genuine interruption before any seal publishes counts but drops all declared pair coverage and renders known native benchmark identity as unknown/unknown&#46; Missing pairs cannot be reconstructed from the incomplete pre&#45;effect run&#45;control&#46;
- Current evidence: Frozen zero&#45;seal report contract: Current run&#45;control/2 freezes resolved experiment/integrity before intent; report restores topology/native identity without a seal&#46; zero&#95;seal&#95;run&#95;control&#95;restores&#95;native&#95;report&#95;identity and planned&#95;coverage&#95;survives&#95;total&#95;bundle&#95;loss pass&#46; Current evidence was established before historical exposure at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618&#46;
- Cause / missed boundary: Pre&#45;effect frozen contract replaces reconstructing report identity only from surviving seals; missing evidence remains explicit&#46;
- Suggested current verification: Retain the current discriminating case and rerun it only if its owning contract/dependencies change&#46; This advisory row does not close any current finding&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;f60e458&#45;20261003t041015z / P19&#45;AUD&#45;004; source prior&#45;findings
- Repair commits: 0fa995de504bcb9854e8ee841d54d99579da1bd4
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;8mm5zoko&#92;rust&#45;tests&#46;log; SHA-256 e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa; checked: Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar&#46;; limits: Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed&#46;

### Comparison 9: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Prior A fresh &#45;&#45;out inside the canonical source&#39;s bundle/artifacts is accepted&#46; Regrade then copies task files there before later lifecycle/base/image checks, adding unmanifested source files and invalidating subsequent verify/report, even when grading later rejects&#46;
- Current evidence: Regrade source containment: Current resolve&#95;fresh&#95;output canonicalizes the nearest existing ancestor and refuses source&#45;contained output before writes; regrade&#95;output&#95;containment&#95;rejects&#95;canonical&#95;source&#95;aliases observes preserved source bytes&#46; Current evidence was established before historical exposure at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618&#46;
- Cause / missed boundary: Only current containment invariants are preserved here; actual native input&#45;sensitive regrade remains an evidence gap in current003&#46;
- Suggested current verification: Retain the current discriminating case and rerun it only if its owning contract/dependencies change&#46; This advisory row does not close any current finding&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;f60e458&#45;20261003t041015z / P19&#45;AUD&#45;005; source prior&#45;findings
- Repair commits: 0fa995de504bcb9854e8ee841d54d99579da1bd4
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;8mm5zoko&#92;rust&#45;tests&#46;log; SHA-256 e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa; checked: Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar&#46;; limits: Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed&#46;

### Comparison 10: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Prior Stderr is classified validated&#45;diagnostic when each line merely contains one allowed token, then the entire original line is written&#46; Arbitrary private/provider payload surrounding a token crosses the first capture sink under a sanitized receipt in both capture policies&#46;
- Current evidence: Closed stderr diagnostic grammar: Current native capture admits whole records with safe structured cause vocabulary&#46; native&#95;capture&#95;admits&#95;only&#95;closed&#95;stderr&#95;diagnostics and private&#45;thinking/credential canary tests pass under both policies&#46; Current evidence was established before historical exposure at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618&#46;
- Cause / missed boundary: The substring admission path from old006 is replaced; no old sanitized/Closed claim is adopted and actual live captures remain unassessed&#46;
- Suggested current verification: Retain the current discriminating case and rerun it only if its owning contract/dependencies change&#46; This advisory row does not close any current finding&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;f60e458&#45;20261003t041015z / P19&#45;AUD&#45;006; source prior&#45;findings
- Repair commits: 0fa995de504bcb9854e8ee841d54d99579da1bd4
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;8mm5zoko&#92;rust&#45;tests&#46;log; SHA-256 e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa; checked: Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar&#46;; limits: Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed&#46;

### Comparison 11: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Prior The local acceptance verifier rejects every nonzero/timed&#45;out Agent terminal and any native exception before consuming benchmark&#45;valid sealed classification, although eligible real task failures/timeouts may count among the six native trials&#46;
- Current evidence: Eligible graded failures in acceptance: Current acceptance validation follows verified graded settlement and tests valid native nonzero/timeout outcomes while excluding invalid provider/infra substitutions&#46; test&#95;valid&#95;native&#95;task&#95;failures&#95;count&#95;as&#95;graded&#95;trials passes&#46; Current evidence was established before historical exposure at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618&#46;
- Cause / missed boundary: Validator predicate repair is retained, independently of absent actual six&#45;trial operator evidence in current004&#46;
- Suggested current verification: Retain the current discriminating case and rerun it only if its owning contract/dependencies change&#46; This advisory row does not close any current finding&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;f60e458&#45;20261003t041015z / P19&#45;AUD&#45;007; source prior&#45;findings
- Repair commits: 0fa995de504bcb9854e8ee841d54d99579da1bd4
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;8mm5zoko&#92;python&#45;tests&#46;log; SHA-256 bfe4f638e90cd44e1aa000dd3f130433927a19cffe724c8e6d69c56e7f94045c; checked: Unittest reports Ran 273 tests and OK &#40;skipped=1&#41;, with controlled integration and acceptance assertions&#46;; limits: Constructed acceptance/native&#45;environment tests do not establish actual real&#45;client/native benchmark execution; wrapper post&#45;command exit failed after unittest success&#46;

### Comparison 12: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Prior Doctor observes only the static lock and selected Agent paths, omitting selected corpus/profile, uv, runner project/lock/interpreter/entrypoint, integrations/provider and capture helper from missing&#45;input diagnosis&#46;
- Current evidence: Complete selected doctor inventory: Current doctor inventories selected corpus/profile/uv/runner/runtime/integration/scripted provider/capture helper paths from selected preparation inputs; current preflight/refusal tests and doc check pass&#46; Current evidence was established before historical exposure at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618&#46;
- Cause / missed boundary: The missing&#45;diagnosis branch is removed without a new inventory ledger&#46; This row is source/control&#45;test evidence, not actual native material preparation acceptance&#46;
- Suggested current verification: Retain the current discriminating case and rerun it only if its owning contract/dependencies change&#46; This advisory row does not close any current finding&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;f60e458&#45;20261003t041015z / P19&#45;AUD&#45;010; source prior&#45;findings
- Repair commits: 0fa995de504bcb9854e8ee841d54d99579da1bd4
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;8mm5zoko&#92;rust&#45;tests&#46;log; SHA-256 e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa; checked: Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar&#46;; limits: Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed&#46;

### Comparison 13: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Prior Pi&#39;s native importer recognizes only the Completions truncation string as provider&#45;protocol&#45;error; the exact Anthropic and Responses native protocol strings acknowledged by the client matrix fall through to generic provider&#45;failure&#46;
- Current evidence: All three typed Pi truncation causes: Current Pi importer recognizes the exact Anthropic, Responses and Completions truncation vocabulary; pi&#95;protocol&#95;truncations&#95;classify&#95;typed&#95;across&#95;all&#95;three&#95;wires passes&#46; Current evidence was established before historical exposure at 2ec1ccb14a3d13af173fa0692dbf8f1cd8cb7618&#46;
- Cause / missed boundary: Typed diagnostic repair is retained at the importer seam; actual pinned clients exercising all wires remains open in current003&#46;
- Suggested current verification: Retain the current discriminating case and rerun it only if its owning contract/dependencies change&#46; This advisory row does not close any current finding&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;f60e458&#45;20261003t041015z / P19&#45;AUD&#45;011; source prior&#45;findings
- Repair commits: 0fa995de504bcb9854e8ee841d54d99579da1bd4
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;8mm5zoko&#92;rust&#45;tests&#46;log; SHA-256 e66529849d4b30bb57f4f50c9acc97a1d1c430e1821732c19a9ff8448eff2afa; checked: Read all 14 result groups: 403 passed, 0 failed, 11 ignored; discriminating current named tests in independent sidecar&#46;; limits: Scoped deterministic checks at the current head; ignored actual native material/client/Docker predicates were not executed&#46;
