# Phase 19 Audit

**Audit run ID**: `phase19-codex-gpt6-c18efa8-20260917t133157z`
**Audit head**: `c18efa83191a577f4f115b25a439febfc8ad68ad`
**Reviewer ID**: `codex`
**Model ID**: `gpt6`
**Reviewer identity**: Codex
**Reviewer model ID**: `gpt6`
**Model identity source**: operator-declared
**Independence**: unknown — fresh audit context, no prior audit/remediation/peer conclusions consumed; the operator named gpt6, and exact runtime/implementer family separation was not attested.
**Baseline policy**: latest-committed-spec
**Verdict**: FAIL

本审计发现 2 个 Blocker、10 个 Major。现有默认检查通过，但隔离探针复现了权限、结果归属、分类、执行限制、CLI 和证据完整性问题；真实 native 与六次 trial 的原始材料不足，不能据此确认 Phase exit。仅安装审计产物，没有修改产品代码或提交。

## Baseline Sources

Requirements were sealed before implementation inspection: 71 records (69 mandatory, 2 optional). Initial sidecar SHA-256: `bb0ca992dafbe3ac0a8a9bf465580578ecf80ee92e85a5bb85e8a7aac1dca3de`. Its identity/criterion/behavior fields were compared against the final sidecar and remain unchanged. Latest committed source hashes match the ledger registrations. The source was exported with `git archive`, with no branch/worktree change; all checks below ran against that export under WSL/Linux. 195 relevant committed files were finally compared byte-for-byte against the archive, with no differences. Transient audit probes only add a test binary in that export and have been removed after execution.

| Path | SHA-256 | Registration note |
|---|---|---|
| `.opi-impl-state.json` | `d9b84dd2d85d64f0fa7b3462dbfd00f6f0a464b02393684e70840a9ecdc642a0` | Current committed authoritative/registered source; stored registration matched where applicable. |
| `docs/snapshots/phase19/opi-impl-state.json` | `35b3378bd902cc976bb348d794dd1b5d3c5f160a58769c3aeb62fea175a7b7a9` | Current committed authoritative/registered source; stored registration matched where applicable. |
| `docs/opi-spec.md` | `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db` | Current committed authoritative/registered source; stored registration matched where applicable. |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Current committed authoritative/registered source; stored registration matched where applicable. |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.zh.md` | `d41c826412b0d71802327d60f953708f6cb2083436a2a2b0d99a2e52d851f194` | Current committed authoritative/registered source; stored registration matched where applicable. |

## Requirement Conformance

The JSONL sidecar is the machine record. `met` refers to the stated source/fixture contract, never an unexecuted actual-client or real-provider claim. Dynamic requirements are kept distinct. Optional real-service Completions/Responses qualification was not performed and does not block the phase by itself.

| Requirement ID | Criterion / observable behavior | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| `P19-AC-001-diagnose` | Doctor and validate report engine, architecture, resources, unknown observations and missing materials without Agent/model invocation. | crates/opi-eval/tests/fail_closed_cli.rs::doctor_reports_observations_without_a_model / doctor_rejects_an_invalid_config_without_observing — Public doctor output has explicit observation/unknown states; invalid input rejects before observation. No Agent/model dispatch on this branch. | met | — |
| `P19-AC-001-prepare` | Prepare resolves and verifies pinned tasks, runners, selected Agent builds and immutable images in a reusable local cache without model calls. | crates/opi-eval/tests/native_driver.rs::prepare_binds_runner_project_and_rejects_source_drift / prepare_rejects_unpinned_runner_and_uv_identities — Seeded local cache is verified against pinned executable, source commit, uv lock and runtime package identity. Drift and unpinned tool versions reject. This establishes verification mechanics, not real dataset acquisition. | met | — |
| `P19-AC-001-independent` | Prepare/run/report work from an independent directory with declared external materials, no repository fixtures, skill runtime or GitHub execution metadata. | crates/opi-eval/tests/native_driver.rs::independent_directory_prepare_builds_a_local_origin_material — Temporary-directory fixture proves local-origin preparation without CI receipt; real independent native product path was not exercised with supplied external material. | partially-met | P19-AUD-010 |
| `P19-AC-001-preview` | Plan exposes subjects, selected connection, tasks, selection reasons, repetitions, trial count, concurrency, limits and resource failures before execution. | crates/opi-eval/tests/native_driver.rs::full_inventory_preflight_lists_requirements_and_freezes_feasible_subsets — Preview contains selection/trial count/concurrency/bounds, but drops disk and richer resource requirements. | partially-met | P19-AUD-007 |
| `P19-AC-001-progress` | Run executes the frozen plan and emits task/Agent/lifecycle/settlement/score or error progress plus a structured local result. | crates/opi-eval/tests/assembled_run.rs::frozen_subset_execution; audit_plan_material_panics — Hermetic frozen plan reaches stage/settlement output; the documented explicit native-material plan command panics and configured native limits are not applied. | partially-met | P19-AUD-005, P19-AUD-006 |
| `P19-AC-002-selection` | Upstream task jobs explicitly invoke the selected actual Opi/pi Agent inside the environment whose output is graded. | crates/opi-eval/tests/native_driver.rs::native_run_drives_paired_trials_through_one_selected_job_each — The public driver explicitly passes the integration into one task job. Stand-in assertions check argv and handoff; actual-client/environment execution requires missing native evidence. | partially-met | P19-AUD-010 |
| `P19-AC-002-identity` | Production result admission rejects missing, wrong, oracle and stale Agent/job/trial/task identities even after successful process exit. | audit_terminal_identity_mismatch_admitted — Both terminal product=oracle, hash=f*64 and unrelated model results sealed successfully through native CLI. | not-met | P19-AUD-003 |
| `P19-AC-002-artifacts` | Scored artifacts, verifier inputs, subject builds, environment, task/runner revisions and native output are bound and substituted artifacts are rejected. | crates/opi-eval/tests/native_driver.rs::native_result_binding_rejects_every_substitution; audit_terminal_identity_mismatch_admitted — Delivered/collected byte substitution is rejected, but build/model/terminal identity is not bound to those bytes. | partially-met | P19-AUD-003 |
| `P19-AC-002-oracle` | Oracle diagnostics are explicit and separately identified; they never supply Agent scores, pairs, denominators or implicit run prerequisites. | crates/opi-eval/tests/native_driver.rs::oracle_witnesses_are_native_only_conformance_diagnostics; audit_terminal_identity_mismatch_admitted — Explicit oracle diagnostics have a separate path; an oracle terminal product inside a nominal integration job still becomes an Agent score. | partially-met | P19-AUD-003 |
| `P19-AC-002-classification` | One evidence-bound settlement consumes typed terminal observations; valid zero/task timeout is distinguishable from provider/identity/adapter/infrastructure failure and effects unknown. | audit_nonzero_provider_error_enters_scores — A captured authentication error plus the real integration nonzero tag produces valid Agent failures and denominator=2. | not-met | P19-AUD-004 |
| `P19-AC-003-lifecycle` | TB2.1 grades the same Agent task environment under the selected native lifecycle after credential cleanup. | crates/opi-eval/scripts/test_native_integrations.py; tests/native_driver.rs::native_run_drives_paired_trials_through_one_selected_job_each — Shared-environment declarations and credential cleanup are wired and exercised with fake environments; no real pinned TB2.1 lifecycle witness was run. | partially-met | P19-AUD-010 |
| `P19-AC-003-witness` | Controlled correct and incorrect TB2.1 artifacts traverse production handoff and the real pinned native grader with discriminating outcomes. | crates/opi-eval/tests/native_driver.rs::native_tb21_artifact_witness — Ignored by default; requires supplied pinned material and Docker. No real positive/negative grader evidence supplied. | not-assessable | P19-AUD-010 |
| `P19-AC-003-real` | Both Opi and pi have preserved real TB2.1 tool traces, task effects and attributable native grading. | Original redacted TB2.1 Opi/pi trial roots — No original real-Agent roots supplied; summaries/fixture results are not admitted as proof. | not-assessable | P19-AUD-010 |
| `P19-AC-004-collection` | TB3 parses selected task collection/lifecycle rules; only declared artifacts reach the required fresh separate verifier. | crates/opi-eval/tests/native_driver.rs::native_tb30_run_grades_declared_artifacts_across_the_separate_verifier — Task lifecycle and declared artifact parsing reach native dispatch/import. Synthetic canary/substitution checks pass; actual separate verifier transfer remains unwitnessed here. | partially-met | P19-AUD-010 |
| `P19-AC-004-binding` | TB3 rejects missing/substituted/undeclared artifacts, unsupported lifecycle and unbound source-trial/verifier identities. | crates/opi-eval/tests/native_driver.rs::native_tb30_rejects_undeclared_and_substituted_collection — Missing/undeclared/changed collection rejects. Selected Agent build/product identity still has the shared importer gap. | partially-met | P19-AUD-003 |
| `P19-AC-004-witness` | Real native TB3 positive/negative witnesses change only declared input and prove an unrelated canary cannot reach the verifier. | crates/opi-eval/tests/native_driver.rs::native_tb3_artifact_witness — Ignored; no real TB3 positive/negative/canary witness supplied. | not-assessable | P19-AUD-010 |
| `P19-AC-004-real` | Both real Opi and pi TB3 trials traverse the declared collection and grading boundary. | Original redacted TB3 Opi/pi trial roots — No original real-Agent transfer/grading evidence supplied. | not-assessable | P19-AUD-010 |
| `P19-AC-005-patch` | DeepSWE invokes declared collect hooks, collects committed patches against the task base, and applies that exact patch in a pristine separate native verifier. | crates/opi-eval/tests/native_driver.rs::native_deepswe_run_grades_the_committed_patch_in_the_pristine_verifier — Production preserves declared collect hook/base/image/separate policy; stand-in writes fixture patch bytes, so real patch collection/application remains unproven here. | partially-met | P19-AUD-010 |
| `P19-AC-005-reject` | Wrong base, changed patch, wrong identity and substitution of uncommitted changes are rejected. | crates/opi-eval/tests/native_driver.rs::native_deepswe_rejects_undeclared_collection_and_base_drift — Base and undeclared collection negatives pass; actual committed/uncommitted patch discrimination and selected executable identity are not fully established. | partially-met | P19-AUD-003, P19-AUD-010 |
| `P19-AC-005-witness` | Correct/incorrect committed patch witnesses reach the pinned native grader and demonstrate changed outcomes. | crates/opi-eval/tests/native_driver.rs::native_deepswe_patch_witness — Ignored; requires exact external material. No real patch-sensitive grader result supplied. | not-assessable | P19-AUD-010 |
| `P19-AC-005-real` | Both real Opi and pi DeepSWE trials have preserved patch-to-result and base/verifier identity evidence. | Original redacted DeepSWE Opi/pi trial roots — No original real-Agent patch/base/verifier trace supplied. | not-assessable | P19-AUD-010 |
| `P19-AC-006-projection` | Each native Agent consumes named connection provider, protocol, exact endpoint/model and selected credential reference without implicit fallback. | crates/opi-eval/tests/native_driver.rs::native_missing_credential_rejects_before_any_effect; scripts/test_scripted_provider.py — Named connection and native config projections have deterministic fixture checks; exact actual-client request/auth/tool behavior was ignored without supplied binaries. | partially-met | P19-AUD-010 |
| `P19-AC-006-controls` | Requested/effective reasoning, sampling, context/output controls and origins are retained; unsupported required controls fail before inference. | crates/opi-eval/tests/native_driver.rs::native_output_control_binds_projection_wire_and_exact_client — Control snapshot pins executable/package/config/request bytes and rejects drift. Actual-client output-control matrix was ignored; generated stand-in wire records are not proof of effective client behavior. | partially-met | P19-AUD-010 |
| `P19-AC-006-fixtures` | Both actual pinned Agent clients exercise request/authentication/tool loops for all three APIs against local fake servers. | crates/opi-eval/tests/native_driver.rs::native_protocol_client_matrix — Ignored: no OPI_EVAL_NATIVE_CLIENT_MATERIAL. Python scripted-provider tests establish server behavior, not both actual clients. | not-assessable | P19-AUD-010 |
| `P19-AC-006-terminal` | Both native clients distinguish terminal authentication/rate-limit/protocol failures from retryable 429 followed by success, including JSON errors at exit zero. | audit_nonzero_provider_error_enters_scores — Native scoring never consumes the terminal parser used by the adapter client matrix. Provider authentication errors enter Agent scores. | not-met | P19-AUD-004 |
| `P19-AC-006-real` | Both actual Agents prove the selected Zhipu glm-5.3-flash Anthropic connection with preserved real tool-loop and effective-control evidence. | Original first-connection provider/tool captures — No supplied real Zhipu Anthropic glm-5.3-flash Opi/pi evidence; no live calls made by the audit. | not-assessable | P19-AUD-010 |
| `P19-AC-006-unknown` | Unknown service model and prices retain reasons; estimates, provider usage and billed cost remain distinct. | crates/opi-eval/tests/experiment_contract.rs::resolve_rejects_requested_caps_the_native_route_cannot_enforce — Unknown-price cost caps reject and report facts use unknown reasons; no original real service identity/usage/billing captures supplied to verify native distinctions. | partially-met | P19-AUD-010 |
| `P19-AC-006-bounds` | Requested unsupported usage/currency bounds are rejected before effects; enforceable time/output limits and their enforcement limitations are explicit. | audit_native_time_limit_not_applied — Unenforceable token/currency caps reject, but accepted configured native time bounds are ignored. | not-met | P19-AUD-005 |
| `P19-AC-007-inventory` | Full preflight enumerates the pinned corpus and task file/digest inventories, instructions, environment, collection and native grader identity without starting every image. | crates/opi-eval/tests/native_driver.rs::full_inventory_preflight_lists_requirements_and_freezes_feasible_subsets — Metadata can enumerate a supplied corpus tree. The built-in prepared package/profile path remains a single bound task; native full-corpus execution cannot follow that inventory. | partially-met | P19-AUD-008 |
| `P19-AC-007-resources` | Task network/CPU/memory/GPU/disk/Agent/verifier timeout requirements are preserved and compared with actual WSL/Docker resources; unknowns remain visible. | audit_unfulfillable_storage_is_ignored — Impossible storage declaration is omitted and admitted. Registered NVIDIA runtime is treated as sufficient for any positive GPU count. | not-met | P19-AUD-007 |
| `P19-AC-007-reject` | Insufficient/unknown selected resources or unsupported semantics reject freezing/running rather than filtering tasks or relaxing declarations. | audit_unfulfillable_storage_is_ignored — CPU/memory/unknown-runtime negatives reject, but a dropped storage requirement cannot reject freezing/preparation. | not-met | P19-AUD-007 |
| `P19-AC-007-scope` | Full/subset scope, exact IDs, selection reasons, repetitions/seeds and controls freeze into a digest-bound immutable plan. | crates/opi-eval/tests/assembled_run.rs::tampered_plan_rejects_before_any_trial; tests/experiment_contract.rs::resolve_selection_freezes_scope_tasks_repetitions_and_bounds — Schema validation rejects ambiguous/full-with-ids selections; canonical plan binds scope, ids, repetition and controls. Tampered plan is refused before trial effects. | met | — |
| `P19-AC-007-execution` | Multi-task execution follows the exact selected plan with positive bounded concurrency; material/config changes yield new identities. | crates/opi-eval/tests/native_driver.rs::plan_rejections_name_every_contradiction; audit_native_time_limit_not_applied — Hermetic waves are bounded, but native selected tasks outside one profile cannot run and configured native deadlines are ignored. | not-met | P19-AUD-005, P19-AUD-008 |
| `P19-AC-007-network` | Provider inference access does not widen task or verifier network authority. | crates/opi-eval/scripts/test_native_integrations.py — Declared verifier-network policy is preserved/rejected by source contracts. No actual Docker native boundary witness supplied for task/provider/verifier network separation. | partially-met | P19-AUD-010 |
| `P19-AC-007-materials` | Mutable image references become immutable before freeze; material drift/missing files are rejected and local origin never becomes fabricated CI provenance. | crates/opi-eval/tests/native_driver.rs::task_package_drift_after_materialization_is_rejected / agent_executable_digest_drift_is_rejected_at_load / prepare_binds_runtime_package_and_rejects_resource_drift — Byte/source/package drift rejects at the owning material boundary; local origin is explicit and does not require fabricated CI transport identity. | met | — |
| `P19-AC-008-metrics` | Reports preserve native metric definitions and denominators, distinguish task success/test fractions and do not average benchmark families. | crates/opi-eval/tests/report_contract.rs:206 — Exact native scalar and valid denominator survive, but generic metric wording does not preserve benchmark-specific success/fraction meaning. | partially-met | P19-AUD-011 |
| `P19-AC-008-coverage` | Reports retain corpus/selected/planned/attempted/valid/Agent-failed/invalid/unexecuted counts, missing coverage and full/subset/incomplete labels. | crates/opi-eval/tests/bundle_recompute.rs::planned_coverage_survives_total_bundle_loss; tests/report_output_containment.rs::unsealed_settled_failure_keeps_attempt_and_invalid_denominators — Pre-effect inventory plus verified/unsealed observations preserves planned, attempted, invalid and unexecuted counts; missing/corrupt controls block rather than fabricate coverage. | met | — |
| `P19-AC-008-failure` | Provider/access/adapter/identity/infra/invalid-grading failures are excluded from Agent success/failure scores; valid native zero remains an Agent outcome. | audit_nonzero_provider_error_enters_scores — Report consumes the sealed classifier faithfully, including its incorrect provider-failure eligibility. | not-met | P19-AUD-004 |
| `P19-AC-008-pairs` | Comparisons pair identical task/repetition/frozen controls, expose missing pairs/differences and reject protocol mismatch. | crates/opi-eval/tests/report_contract.rs::protocol_safe_pairing_and_single_subject_coverage; tests/pairing_and_integrity.rs::pair_and_control_fail_closed — Pairing fixtures vary group/protocol/control identity and expose missing/non-comparable pairs rather than matching incompatible observations. | met | — |
| `P19-AC-008-single` | Single-subject reports require no invented comparison edge or absent Agent. | crates/opi-eval/tests/assembled_run.rs::pi_only_selection_runs_without_an_artificial_edge; tests/experiment_contract.rs::single_subject_selection_needs_no_artificial_edge — Single-subject selection resolves and publishes without an invented comparison edge or second product. | met | — |
| `P19-AC-008-telemetry` | Missing timing/token/cost/call observations remain unknown with reasons, retaining origins, coverage and failed-call consumption. | audit_native_missing_time_becomes_measured_zero — Both native bundles seal unknown Agent duration as Known zero with supervisor-wall-clock origin. | not-met | P19-AUD-012 |
| `P19-AC-008-sealed` | Report generation consumes verified sealed interpretations and pre-effect planned inventory, not mutable logs or a second outcome classifier. | crates/opi-eval/tests/report_output_containment.rs::native_reward_without_matching_score_evidence_blocks_publication; tests/end_to_end_report.rs::outer_run_artifact_mutations_cannot_affect_output — Report re-verifies bundles, requires native reward evidence and consumes sealed settlement; mutable outer receipts do not override scored facts. | met | — |
| `P19-AC-009-verify` | Byte verification rejects tampering and unsealed evidence without invoking Agents/providers/network. | crates/opi-eval/tests/bundle_recompute.rs::covered_byte_mutation_is_rejected / bnd001_retained_byte_closure_rejects_drift — Manifest/sidecar/artifact closure and hashes are checked without spawning an Agent or provider; missing, mutated and unsealed inputs fail closed. | met | — |
| `P19-AC-009-recompute` | Report reconstruction from immutable verified native evidence is deterministic, offline and cannot overwrite source or prior output. | crates/opi-eval/tests/end_to_end_report.rs::normalized_report_is_byte_stable_across_independent_runs; tests/report_output_containment.rs — Offline reconstruction is deterministic. Output containment checks canonical ancestors, existing targets and create_new so report cannot overwrite source or prior output. | met | — |
| `P19-AC-009-regrade` | Actual regrade uses saved real artifacts with the pinned native grader in fresh verifier environments, never an Agent/provider. | audit_regrade_argv_and_ambient_credential — Zero argv is supplied to pinned uv; only test stand-in implements the requested regrade mode. | not-met | P19-AUD-002 |
| `P19-AC-009-cache` | Regrade rejects missing artifacts/cached immutable materials and unavailable permitted offline equivalents without relaxing verifier policy. | crates/opi-eval/tests/native_driver.rs::native_offline_regrade_rejects_every_fail_closed_case — Saved byte/missing-artifact checks exist, but there is no real fresh offline native environment/grader dispatch whose material policy is enforced. | partially-met | P19-AUD-002 |
| `P19-AC-009-derived` | Regrade emits a new derived identity bound to original bundle/artifacts and fresh verifier while preserving original bytes/scores. | crates/opi-eval/tests/native_driver.rs::native_offline_regrade — Derived record/source immutability assertions pass against a synthetic grader; the actual pinned native grading operation is absent. | partially-met | P19-AUD-002 |
| `P19-AC-009-witness` | All three families have real native regrade witnesses proving artifact-sensitive results, cache/tamper rejection and zero Agent/provider calls. | crates/opi-eval/tests/native_driver.rs::native_offline_regrade — The all-family test is a shell marker-byte simulator. No real all-family native regrade witness was supplied or executed. | not-assessable | P19-AUD-002, P19-AUD-010 |
| `P19-AC-009-legacy` | Legacy evidence stays legacy/conformance or unsupported and cannot be relabelled as a native Agent score. | crates/opi-eval/tests/report_contract.rs::verify_regrade_split_names_each_operation; tests/native_driver.rs::native_offline_regrade_rejects_every_fail_closed_case — Legacy fallback remains explicitly legacy-derived/conformance evidence and lacks the native artifact gate required for actual regrade. | met | — |
| `P19-AC-010-credentials` | Credentials resolve only for the selected Agent, with isolated temporary native config and no user-global configuration access or modification. | audit_regrade_argv_and_ambient_credential; tests/native_driver.rs::native_missing_credential_rejects_before_any_effect — Selected Agent credential reference and temporary config exist; the regrade child nevertheless inherits unrelated ambient provider secrets. | partially-met | P19-AUD-001 |
| `P19-AC-010-privacy` | Provider canaries do not reach graders/export; secret files are excluded from handoff and shared environments are cleaned before verification. | audit_regrade_argv_and_ambient_credential — Normal integration cleanup/canary gates do not prevent dummy ANTHROPIC_API_KEY crossing into the regrade verifier process. | not-met | P19-AUD-001 |
| `P19-AC-010-mounts` | Agent task execution does not mount host Docker socket or broad unrelated host directories. | crates/opi-eval/scripts/test_native_integrations.py — Integration stages a bounded Agent runtime/capture and no host Docker socket in inspected commands. Actual native task mount witness was not supplied. | partially-met | P19-AUD-010 |
| `P19-AC-010-supervision` | One bounded run supervisor propagates cancellation/timeouts/breach stops, stops affected dispatch and reports effects without silent success. | audit_native_time_limit_not_applied — Existing supervisor handles fixed profile limits and cancellation; accepted configured native time limits do not reach it, and regrade starts a raw Command. | not-met | P19-AUD-005 |
| `P19-AC-010-cleanup` | Cancellation settles only trial-owned processes/containers/temporary credentials, preserves user/shared cache/evidence, and cannot claim success with orphan execution. | crates/opi-eval/tests/native_driver.rs::native_settlement_cleans_root_bound_containers / native_settlement_cleanup_uncertainty_blocks_success / native_signal_cancellation_bounds_dispatch — Exact immutable IDs/root labels and cleanup rechecks discriminate owned versus neighboring fake resources. Real Docker cancellation witness is ignored. | partially-met | P19-AUD-010 |
| `P19-AC-010-recovery` | Offline recovery distinguishes completed/cancelled/failed/effect-unknown/unstarted states without Agents/providers or fabricated scores. | crates/opi-eval/tests/assembled_run.rs::effect_unknown_reopen — Durable fixture recovery retains effect-unknown and does not dispatch Agents. No original native interruption/recovery archive supplied to close the native effect-preservation claim. | partially-met | P19-AUD-010 |
| `P19-AC-010-replacement` | Replacement creates new roots and identities for the entire paired group under the same scope and preserves original attempts/unknown effects. | crates/opi-eval/tests/assembled_run.rs::effect_unknown_reopen; audit_rejected_rerun_changes_existing_control — Paired replacement identity rewriting exists; old-root admission is not transactional and a rejected changed run invalidates the original report. No native replacement archive supplied. | partially-met | P19-AUD-009, P19-AUD-010 |
| `P19-AC-010-native` | No-model native cancellation witness verifies local Docker cleanup and ownership boundaries. | crates/opi-eval/tests/native_driver.rs::native_owned_resource_cancellation — Ignored: requires local Docker/busybox; stand-in Docker checks are not real engine evidence. | not-assessable | P19-AUD-010 |
| `P19-AC-011-default` | Default tests/CI stay deterministic and credential-free even when dummy credential variables are populated; live mode requires explicit selection. | crates/opi-eval/tests/fail_closed_cli.rs::dummy_credentials_do_not_activate_live_behavior — Populated dummy credential environment does not select live execution; default suites use local fixtures, live mode/client materials are explicit. | met | — |
| `P19-AC-011-shared` | Local native conformance needs no workflow dispatch; CI is a thin caller of shared local materialization/scoring policy. | crates/opi-eval/scripts/test_native_smoke_output_control.py; test_verify_native_smoke_ci.py; test_verify_external_lock_ci.py — Shared script/module entrypoints own materialization and scoring policy; deterministic script/transport fixture checks pass and local commands do not require dispatch. | met | — |
| `P19-AC-011-verifier` | Offline acceptance verification consumes supplied frozen/redacted artifacts only, rejects oracle/fixture/substituted/missing evidence and invokes only verify/report. | crates/opi-eval/scripts/test_verify_local_acceptance.py; tests/native_driver.rs::local_acceptance_verifier_accepts_genuine_native_evidence — Structural/digest/output-control/source checks and verify/report-only invocation are tested. The test named genuine uses synthetic trials and identities; no original operator root supplied for actual offline verification. | partially-met | P19-AUD-010 |
| `P19-AC-011-provenance` | Acceptance verifies repository-qualified pi/Harbor/Pier source provenance and an exhaustive nonempty byte-bound three-Opi-capture audit view. | crates/opi-eval/scripts/test_verify_local_acceptance.py::test_mismatched_repository_for_commit_is_rejected / test_empty_scoped_view_is_rejected_even_with_generic_audit_success / test_substituted_capture_bytes_are_rejected — Negative fixtures distinguish repository-qualified cached commit identity and exact nonempty three-Opi capture equality. This proves validator behavior, not authenticity of an unavailable operator archive. | met | — |
| `P19-AC-012-six` | At least six attributable real local WSL/Linux trials cover one feasible task per family by both Agents through the selected first connection and native grading. | Required raw local acceptance and final workspace evidence — Six original attributable real trials were not supplied. | not-assessable | P19-AUD-010 |
| `P19-AC-012-freeze` | Acceptance preserves pre-run frozen task/build/runner/material/model/controls/concurrency/usage/time scope, credential references and separate live authorization. | Required raw local acceptance and final workspace evidence — Original pre-run frozen scope/control/build/material and separate live authorization evidence were not supplied. | not-assessable | P19-AUD-010 |
| `P19-AC-012-artifacts` | Acceptance preserves redacted provider/tool/native evidence, valid bundles, command exits and reproducible local reports, including failed/replacement attempts. | Required raw local acceptance and final workspace evidence — Original redacted provider/tool/bundle/exit artifacts, including failed/replaced attempts, were not supplied. | not-assessable | P19-AUD-010 |
| `P19-AC-012-gates` | Required workspace verification union and actual local WSL/Linux acceptance have final-relevant-state mechanical evidence; source inference alone cannot close dynamic claims. | Required raw local acceptance and final workspace evidence — This audit ran crate tests/clippy, fmt and doc check only. Final-relevant-state full workspace tests, doc tests, rustdoc and full clippy plus local real acceptance were not rerun or supplied. | not-assessable | P19-AUD-010 |
| `P19-AC-013-placement` | The Companion has no Opi product dependencies or Minimal Runtime activation; datasets/graders/images/sandbox engines stay external and public seams remain provisional. | crates/opi-eval/tests/experiment_contract.rs::independent_companion_dependency_boundary — Companion stays unpublished with no Opi product crate dependency; existing internal boundaries carry the work, and external tasks/graders/images remain external. | met | — |
| `P19-AC-013-pi` | Pi-only standalone preparation/execution/package use works without an Opi executable. | crates/opi-eval/tests/native_driver.rs::pi_only_preparation_requires_no_opi_material / pi_only_without_opi_runtime_and_packaging — Selection-aware material loading and hermetic pi-only runtime pass. The packaging-named test checks manifest strings/directories; no actual native pi-only supplied executable/package smoke was run. | partially-met | P19-AUD-010 |
| `P19-AC-013-contracts` | Actual changed 0.x CLI/config/material/bundle/report contracts synchronize schemas, fixtures, callers, help, bilingual READMEs and Unreleased history. | python3 scripts/opi-doc-check.py; audit_plan_material_panics — Documentation contract check passes, but both public plan examples disagree with actual required args and trigger a panic once parser requirements are met. | partially-met | P19-AUD-006 |
| `P19-AC-013-rollback` | Rollback disables the affected optional native path or restores a verified build without reviving competing legacy Agent-score paths; existing user config/session/bundles remain untouched. | crates/opi-eval/tests/fail_closed_cli.rs::reused_trial_identity_is_rejected_before_staging; audit_rejected_rerun_changes_existing_control — No implicit live activation/legacy score revival is observed, but existing run evidence is not preserved on rejected root reuse. | partially-met | P19-AUD-009 |
| `P19-OPTIONAL-openai-completions-live` | Optional independently selected real-service qualification for openai-completions; deterministic client coverage remains mandatory separately. | Optional real-service qualification was not requested or performed; not a phase blocker. | not-assessable | — |
| `P19-OPTIONAL-openai-responses-live` | Optional independently selected real-service qualification for openai-responses; deterministic client coverage remains mandatory separately. | Optional real-service qualification was not requested or performed; not a phase blocker. | not-assessable | — |

## Standards Review

The Companion remains in `crates/opi-eval`, unpublished and independent of Opi product crates. No new runtime dependency, feature, unsafe boundary or lockfile change was made by this audit. Current manifest checks pass. CLI/config/material/evidence/report contracts have production consumers, but a supported plan invocation panics and the bilingual command examples omit a required argument (P19-AUD-006). Passing the documentation checker does not prove executable command examples. Native identity admission and root reuse violate the repository's fail-closed and durable-evidence rules (P19-AUD-003/009).

## Spec Review

The native job architecture, lifecycle parsing, artifact hashing, offline bundle verification and sealed-settlement report consumer are present. The mandatory behavior is nevertheless incomplete: native regrading lacks its production native runner dispatch; general native multi-task selection is explicitly refused; resource admission drops declarations; configured native deadlines are not enforced; provider failures and substituted identities can score. Dynamic closure was assessed separately from static delivery registrations. The full pinned native corpus, exact-client wire matrices, actual native witnesses, and six real-provider trial archive are not established by synthetic results.

## Security, Invariants, Integration, Test Quality, and Residuals

Security review traced selected credentials through projection, normal integration cleanup, exported evidence and regrade. The verified dummy-secret exposure is at the regrade subprocess environment, not a hypothetical network attack. Container cleanup uses exact immutable IDs, root labels, dispatch context and post-removal confirmation; fake Docker negatives pass, while the real Docker witness was ignored.

Invariant review traced material identity → plan → trial intent → native job import → settlement → bundle → offline report. The importer verifies outer job/trial/task and artifact bytes but leaves terminal build/product/model disconnected. The report correctly trusts its sealed settlement, which preserves the upstream classifier error instead of repairing it. Original root admission occurs after a mutable run-control write.

Test review distinguishes actual production consumers from substitutes. The eight audit probes invoke the real built Companion against controlled materials and assert observable violations; they prove public-boundary behavior without claiming live-client or real Docker execution. Existing native tests exercise useful plumbing and negative byte checks, but their shell job generator supplies provider exception tags and regrade behavior that production lacks. Seven ignored Rust cases and one skipped Python case are not counted as dynamic acceptance. No provider calls or real credential reads were made.

Residual review found three consequential incomplete mechanisms: the test-only OPI_EVAL_REGRADE consumer, the explicit per-task-native-profile rejection, and native terminal parsing disconnected from final scoring. Generic metric text and synthetic zero timing also survive into durable output. They are recorded under their owning axes rather than treated as historical issues.

### Current durable/public seams

| Seam | Production consumers | Discriminating tests / limitation |
|---|---|---|
| experiment/2 and selected connection | resolver, prepare, plan, run, native config projection | Config rejection/control/credential fixtures; actual-client matrix ignored. |
| native-material/4 and pinned integrations | material loading, prepare, native run, regrade | Executable/package/module/source drift negatives; terminal identity comparison missing. |
| run-plan/1 | plan builder, run plan verification/expansion | Tamper rejection and hermetic subset execution; explicit material panic and native multi-task refusal. |
| native terminal and verifier-input records | benchmark importer, runner settlement, sealed bundle | Artifact substitutions reject; terminal build/product/model and provider cause not bound. |
| bundle manifest, intent, settlement | verify, report, regrade | Byte closure, sidecar/hash/symlink negatives and deterministic reconstruction; root-control overwrite is outside this protection. |
| normalized-report/1 | public report CLI and offline acceptance verifier | Coverage/pair/containment/canary tests; metric meaning and native timing gaps. |
| native-regrade/1 and derived record | regrade CLI → verifier executable → native importer | Shell stand-in only implements regrade; real runner/cache semantics not proven. |
| local-acceptance/1 | offline Python verifier → built Companion verify/report | Source-qualified identities and capture equality negatives; original operator archive unavailable. |

## Minimum-change Conformance

All 11 admitted tasks record reuse_search, surface_necessity and simplification_ceiling in the committed phase ledger. The statuses below compare those records to current production consumers; they do not infer intent from old reviews. There is no new stable SDK/Core seam. For every task, historical net deletion counts are **not-assessable** from current-state inspection and are not inferred from ledger summaries; current replacement/residual behavior is stated explicitly.

| Task | Status | Reuse and necessary surface | Production consumers / nonproduction consumers | Current replacement and residual glue |
|---|---|---|---|---|
| 19.1 | triggered | Existing adapters/bundles reused; native binding/settlement has intrinsic state value. | Native runner, three benchmark adapters, importer, reporter / native_driver and conformance fixtures. | One native task job is selected; detached terminal parsers and unchecked terminal identity prevent the intended classifier/identity replacement (003/004). Hermetic diagnostic route remains intentionally separate. |
| 19.2 | drifted | Existing native config/client projection and output-control snapshot reused. | Projection → integrations → selected client / scripted-provider and exact-client matrix. | No extra LLM client introduced; final native scoring does not consume the client terminal parser, and actual client witnesses were ignored (004/010). |
| 19.3 | not-assessable | Shared lifecycle/import seam serves TB2.1 and TB3; declared collection boundary is necessary. | Harbor integration, TB3 adapter, importer / canary/substitution fixtures. | Existing native declarations reused, no embedded grader; real separate-verifier witness unavailable (010). |
| 19.4 | not-assessable | Shared lifecycle/import seam extended for real Pier collect/base/image semantics. | Pier integration, DeepSWE adapter, importer / patch/base negatives. | Family-specific integration module is justified by upstream API difference; actual committed-patch native witness unavailable (010). |
| 19.5 | drifted | Existing pinned material/cache/tool identity verification reused. | doctor/prepare/validate/run / local-origin, runner-lock and resource fixtures. | No competing material ledger; resource normalization discards required fields (007). |
| 19.6 | triggered | Existing resolver and run assembly own selection and bounded execution. | plan → run → report / plan tamper, corpus and selection fixtures. | Frozen schema exists, but per-task native profiles remain unmaterialized and deadline fields have no native consumer (005/007/008); explicit material CLI branch panics (006). |
| 19.7 | drifted | Existing sealed bundle/trajectory/settlement consumers reused. | verify/report and acceptance verifier / report_contract, bundle_recompute, containment tests. | Single final classification is retained; generic metric semantics and fabricated duration remain in evidence (011/012). |
| 19.8 | triggered | Existing material/bundle/importer reused; derived identity is justified. | regrade CLI / native_offline_regrade stand-in. | New regrade protocol has no real native runner consumer and inherits credentials; test helper is the sole grading implementation (001/002). |
| 19.9 | drifted | ProcessSupervisor, CancellationToken, lifecycle and exact-root cleanup reused. | Native run/cancel/recover/replacement / fake Docker and lifecycle tests. | Cleanup ownership checks are substantive; configured deadlines are disconnected and rejected root reuse corrupts evidence (005/009); real native cancellation unavailable (010). |
| 19.10 | not-assessable | Shared scripts, package boundary and verify/report-only acceptance route retained. | Local scripts, CI callers, acceptance verifier / Python artifact/CI checks and hermetic pi-only fixture. | No CI receipt fabricated for local materials; actual standalone pi-only/native package acceptance unavailable (010). |
| 19.11 | not-assessable | Existing offline verifier and workspace gates are the proper consumers; no runtime seam justified. | Operator-supplied raw roots → offline verifier / synthetic acceptance shape tests. | No original raw roots or final full workspace gate evidence supplied; ledger status cannot replace those artifacts (010). |

## Findings

### P19-AUD-001: Offline regrade inherits ambient provider credentials

- Axis: security
- Severity: Blocker
- Conformance effect: blocks
- Requirement IDs: P19-AC-010-credentials, P19-AC-010-privacy
- Claim: The fresh verifier process inherits the caller environment, including ANTHROPIC_API_KEY, while env.json declares credential_free=true. The explicit projection denylist does not sanitize inherited variables.
- Evidence: `crates/opi-eval/src/regrade.rs:724` — Only material.verifier_env is checked for credential-shaped names.
- Evidence: `crates/opi-eval/src/regrade.rs:757` — Command inherits the environment; only three named variables are removed; no env_clear.
- Evidence: `crates/opi-eval/tests/native_driver.rs:4539` — The regression checks env.json projected keys, not the actual child environment.
- Reproduction: `cargo test -p opi-eval --locked --test audit_phase19_probes audit_regrade_argv_and_ambient_credential -- --nocapture`. Probe setup is in Appendix A.
- Refutation attempted: The normal Agent integration cleans its staged configuration and regrade removes OPENAI_API_KEY. Neither closes arbitrary inherited secrets. The public CLI probe supplied only a dummy ANTHROPIC_API_KEY and both verifier processes read that exact sentinel. No real credential was inspected.
- Suggested closure: Spawn regrade with an explicit minimal environment and prove actual child-environment canaries are absent.

### P19-AUD-002: Regrade has no production dispatch into Harbor or Pier

- Axis: integration
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-009-regrade, P19-AC-009-cache, P19-AC-009-derived, P19-AC-009-witness
- Claim: Preparation pins tools/uv as verifier_executable, but regrade starts it with zero arguments and only private OPI_EVAL_REGRADE variables. No production consumer launches the pinned native grader or restores artifacts into a verifier environment. The environment-variable grading implementation exists only in the test stand-in.
- Evidence: `crates/opi-eval/src/runner/prepare.rs:516` — The verifier is cache/tools/uv, validated as uv 0.9.7.
- Evidence: `crates/opi-eval/src/regrade.rs:733` — Four private regrade variables are emitted; lines 757-767 start the executable without argv.
- Evidence: `crates/opi-eval/tests/native_driver.rs:192` — The shell stand-in alone interprets OPI_EVAL_REGRADE and compares synthetic marker bytes to manufacture a fresh job.
- Reproduction: `cargo test -p opi-eval --locked --test audit_phase19_probes audit_regrade_argv_and_ambient_credential -- --nocapture`. Probe setup is in Appendix A.
- Refutation attempted: native_offline_regrade passes for all three families and rejects modified/missing artifacts, but its verifier is verifier_stand_in_script, not uv/Harbor/Pier. The probe confirms argc=0 at the real CLI spawn boundary. Production source search found no consumer of OPI_EVAL_REGRADE outside regrade.rs. No claim is made that a real native grader ran during this audit.
- Suggested closure: Connect saved artifacts and pinned cached environments to a real no-Agent native regrade dispatch, then run discriminating native witnesses for all three families.

### P19-AUD-003: Result admission accepts substituted terminal product, build and model

- Axis: invariants
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-002-identity, P19-AC-002-artifacts, P19-AC-002-oracle, P19-AC-004-binding, P19-AC-005-reject
- Claim: A native job with the expected outer integration name can seal a score even when its terminal record says product=oracle, executable hash differs from the selected build and its result model is unrelated. Those fields are parsed or copied but not checked against the selection.
- Evidence: `crates/opi-eval/src/benchmark/process.rs:475` — model_info.name is imported without equality to the frozen selected model.
- Evidence: `crates/opi-eval/src/benchmark/process.rs:556` — Terminal observation is parsed, then delivered artifact digests are checked; selected terminal product/build are not compared.
- Evidence: `crates/opi-eval/src/benchmark/process.rs:897` — Product and executable hash are accepted as arbitrary strings.
- Reproduction: `cargo test -p opi-eval --locked --test audit_phase19_probes audit_terminal_identity_mismatch_admitted -- --nocapture`. Probe setup is in Appendix A.
- Refutation attempted: Existing substitutions tests reject outer agent_name, task, stale job and delivered bytes; executable file drift is rejected at material load. Those checks do not connect the terminal observation to the admitted executable. The probe changes only reported identity fields and obtains two sealed graded-native reward=1 records and a comparable pair.
- Suggested closure: Bind terminal product/build and native request/result model to the selected material before accepting scores; test every field independently.

### P19-AUD-004: Native provider failures enter the Agent score denominator

- Axis: runtime-fidelity
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-002-classification, P19-AC-006-terminal, P19-AC-008-failure
- Claim: Harbor/Pier integrations map every nonzero Agent exit to NonZeroAgentExitCodeError and do not decode native JSON terminal errors. Native settlement treats that exception as a graded Agent failure and never invokes the existing native-client terminal parsers. Captured authentication errors therefore enter valid Agent scores; an error stream at exit zero has the same disconnected parser boundary.
- Evidence: `crates/opi-eval/integrations/harbor/opi_eval_harbor.py:309` — All nonzero exits raise NonZeroAgentExitCodeError; the Pier sibling does the same at line 348.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:1313` — The final classifier consumes upstream exception tags, not the actual client error stream.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:1498` — A synthetic AgentRecord is built from exit/timeout; stdout/stderr and usage default empty.
- Reproduction: `cargo test -p opi-eval --locked --test audit_phase19_probes audit_nonzero_provider_error_enters_scores -- --nocapture`. Probe setup is in Appendix A.
- Refutation attempted: Existing native_provider_failure_report passes by having the stand-in emit ApiError directly. Real integration code emits NonZeroAgentExitCodeError for the same terminal process shape. The synthetic native-job probe supplies an explicit authentication JSON error in stdout.log with the integration nonzero tag and reports valid=2, agent_failure=2, denominator=2. This is a public-driver reproduction, not a new live-provider/client trial.
- Suggested closure: Wire both actual clients terminal evidence into the single native settlement owner; prove auth/rate/protocol failures and exit-zero errors are invalid, while retry-then-success remains eligible.

### P19-AUD-005: Declared execution deadlines are not enforced by native dispatch

- Axis: invariants
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-001-progress, P19-AC-006-bounds, P19-AC-007-execution, P19-AC-010-supervision
- Claim: Resolved execution time_limit_seconds, agent_timeout_seconds and verifier_timeout_seconds are displayed/frozen but do not reach the native process dispatch; adapters retain profile limits. A one-second declared limit allows each synthetic job to sleep two seconds and still complete and score.
- Evidence: `crates/opi-eval/src/experiment.rs:705` — The three execution deadlines are parsed into ResolvedExecution.
- Evidence: `crates/opi-eval/src/runner/plan.rs:521` — Preview narrows and displays the configured deadline.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:1556` — Native trial dispatch does not consume the ResolvedExecution deadline fields.
- Reproduction: `cargo test -p opi-eval --locked --test audit_phase19_probes audit_native_time_limit_not_applied -- --nocapture`. Probe setup is in Appendix A.
- Refutation attempted: Profile-based timeout tests and cancellation tests pass, and max_trials/concurrency have consumers. They do not vary these configured deadlines. The probe requests all three limits=1 and observes completed scores after 4818 ms for two sequential delayed jobs.
- Suggested closure: Apply supported configured limits to the owning native supervisor/upstream stage, or reject unenforceable requests before effects; test a bound lower than the profile default.

### P19-AUD-006: Documented run --plan plus --native-material panics

- Axis: standards
- Severity: Blocker
- Conformance effect: blocks
- Requirement IDs: P19-AC-001-progress, P19-AC-013-contracts
- Claim: The CLI permits --plan with --native-material while --plan excludes --config. The native material branch unconditionally expects config, so the documented combination panics with exit 101 before file validation. Both README examples also omit the still-required --fixtures argument.
- Evidence: `crates/opi-eval/src/cli/run.rs:98` — The Some(native_material) arm calls config.as_ref().expect("checked above: --native-material pairs with --config").
- Evidence: `crates/opi-eval/src/main.rs:32` — Run exposes independently optional config/plan/native_material and a required fixtures argument.
- Evidence: `crates/opi-eval/README.md:114` — The plan workflow documents run --plan with an explicit --native-material; Chinese counterpart has it at line 107.
- Reproduction: `cargo test -p opi-eval --locked --test audit_phase19_probes audit_plan_material_panics -- --nocapture`. Probe setup is in Appendix A.
- Refutation attempted: run --plan without an explicit material argument can use embedded material and tests cover that arm. The public help/parser and both READMEs permit/document the explicit combination. The probe includes --fixtures to pass the earlier clap requirement and deterministically gets the config expectation panic, independent of file contents.
- Suggested closure: Resolve material for the plan path without expecting config, and make actual CLI/help/bilingual standalone examples agree.

### P19-AUD-007: Resource admission drops disk requirements and mistakes GPU runtime for capacity

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-001-preview, P19-AC-007-resources, P19-AC-007-reject
- Claim: TaskRequirements retains only CPU/memory/GPU count. Disk, network and stage timeout declarations are absent from the resource comparison; any positive GPU count is accepted if a runtime name contains nvidia. The public prepare path accepts storage_mb=9000000000000 as status ok and omits it from requirements.
- Evidence: `crates/opi-eval/src/runner/prepare.rs:918` — Parser retains task_id, cpus, memory_bytes and gpus only.
- Evidence: `crates/opi-eval/src/runner/prepare.rs:289` — gpu_runtime inspects registered Docker runtimes, not actual count/model/VRAM.
- Evidence: `crates/opi-eval/src/runner/plan.rs:209` — A positive GPU requirement is checked only against runtime presence; no capacity comparison.
- Reproduction: `cargo test -p opi-eval --locked --test audit_phase19_probes audit_unfulfillable_storage_is_ignored -- --nocapture`. Probe setup is in Appendix A.
- Refutation attempted: Existing unmet_resource_requirement test closes the Docker endpoint and verifies unknown GPU rejection; CPU and memory comparisons are real. Those cases do not distinguish a registered runtime from sufficient hardware or test storage. The added public prepare probe supplies an impossible declared disk size and receives an admitted manifest with status ok.
- Suggested closure: Preserve all declared resource/authority/time requirements and reject selected unmet or unobservable requirements using actual relevant WSL/engine capacity.

### P19-AUD-008: Native selection still admits only the single profile-bound task

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-007-inventory, P19-AC-007-execution
- Claim: A feasible task outside the one task bound by the benchmark profile is rejected as per-task native profiles are not materialized yet. Thus a general feasible multi-task native subset/full corpus cannot execute, although that capability is mandatory in the registered phase.
- Evidence: `crates/opi-eval/src/runner/plan.rs:338` — Every selected id except the single profile-bound id is unconditionally rejected.
- Evidence: `crates/opi-eval/src/runner/plan.rs:408` — All three families resolve one task from one profile.
- Evidence: `crates/opi-eval/tests/native_driver.rs:4305` — The test explicitly expects the otherwise feasible alpha-notes task to be rejected for native profile binding.
- Reproduction: `Inspect the cited production path at c18efa83191a577f4f115b25a439febfc8ad68ad`. Probe setup is in Appendix A.
- Refutation attempted: full_inventory_preflight enumerates a synthetic three-task tree and frozen_subset_execution proves hermetic multiple-task dispatch. Neither executes multiple native tasks. The native plan regression intentionally asserts the missing profile rejection. The spec excludes completing all resource-heavy tasks, not general native multi-task execution; no registered deferral was found.
- Suggested closure: Materialize and bind profiles/requirements for each selected pinned native task and execute a feasible multi-task plan without changing its declared scope.

### P19-AUD-009: A rejected rerun corrupts existing run-level evidence

- Axis: invariants
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-010-replacement, P19-AC-013-rollback
- Claim: Reusing a completed root with a changed experiment identity overwrites run-control.json before existing trial intents cause rejection. Subsequent offline report rejects the previously valid sealed root as run-control-drift.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:665` — run-control.json is overwritten unconditionally before dispatch.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:1567` — Only later does native dispatch reject the existing durable intent.
- Evidence: `crates/opi-eval/src/report.rs:399` — Report compares the overwritten control with sealed experiment identity and refuses publication.
- Reproduction: `cargo test -p opi-eval --locked --test audit_phase19_probes audit_rejected_rerun_changes_existing_control -- --nocapture`. Probe setup is in Appendix A.
- Refutation attempted: reused_trial_identity_is_rejected_before_staging and run_control drift tests pass but do not assert all run-level bytes remain unchanged on the rejected request. The probe first completes a valid root, changes only experiment_id, observes exit 2 plus changed control bytes, then confirms report outcome=verification-failed with run-control-drift.
- Suggested closure: Admit/reserve the run root before writing any control or report file; a rejected repeat must preserve every existing source byte.

### P19-AUD-010: Mandatory real native and final acceptance evidence is not assessable in this audit

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-001-independent, P19-AC-002-selection, P19-AC-003-lifecycle, P19-AC-003-witness, P19-AC-003-real, P19-AC-004-collection, P19-AC-004-witness, P19-AC-004-real, P19-AC-005-patch, P19-AC-005-reject, P19-AC-005-witness, P19-AC-005-real, P19-AC-006-projection, P19-AC-006-controls, P19-AC-006-fixtures, P19-AC-006-real, P19-AC-006-unknown, P19-AC-007-network, P19-AC-009-witness, P19-AC-010-mounts, P19-AC-010-cleanup, P19-AC-010-recovery, P19-AC-010-replacement, P19-AC-010-native, P19-AC-011-verifier, P19-AC-012-six, P19-AC-012-freeze, P19-AC-012-artifacts, P19-AC-012-gates, P19-AC-013-pi
- Claim: No supplied redacted six-trial acceptance root, exact-client material or native witness archive was available to this reviewer. The executed default suite explicitly ignores seven native client/grader/Docker checks. Those fixture results cannot close real client protocol, native lifecycle, real regrade, pi-only native packaging, six-trial or final workspace exit obligations.
- Evidence: `crates/opi-eval/tests/native_driver.rs:1` — The test module explicitly describes deterministic stand-ins and distinguishes ignored real native witnesses.
- Evidence: `crates/opi-eval/tests/native_driver.rs:4483` — Offline regrade witnesses use stage("ok"), not pinned upstream graders.
- Evidence: `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md:290` — Real acceptance requires separately supplied frozen operator evidence and cannot be waived by source inference.
- Reproduction: `Inspect the cited production path at c18efa83191a577f4f115b25a439febfc8ad68ad`. Probe setup is in Appendix A.
- Refutation attempted: All non-ignored crate tests pass and the implementation ledger registers delivery. That is not raw independently inspected dynamic evidence. The user was asked for raw evidence/material paths; none was supplied before report sealing, and no concrete raw root was recoverable from the registered ledger. Prior audits and remediation reports were deliberately not consulted. This finding does not assert the trials never happened.
- Suggested closure: Supply the original redacted frozen roots and pinned local material; independently run the offline acceptance verifier and exact native witnesses, and provide final-relevant-state workspace gate evidence.

### P19-AUD-011: Published metrics do not distinguish benchmark task success from test fraction

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-008-metrics
- Claim: Every benchmark receives the same metric id native-grader-reward and the same generic definition. Native scalar values and valid-trial denominator survive, but no benchmark-specific definition or distinction between task success and test-case pass fraction reaches the report.
- Evidence: `crates/opi-eval/src/report.rs:821` — The sole metrics row uses constant metric/definition strings for all benchmark revisions.
- Evidence: `crates/opi-eval/tests/report_contract.rs:206` — The assertion checks one benchmark label, denominator and the word zero; it never checks metric meaning or a partial pass fraction.
- Reproduction: `Inspect the cited production path at c18efa83191a577f4f115b25a439febfc8ad68ad`. Probe setup is in Appendix A.
- Refutation attempted: Raw grader artifacts and exact reward provenance are retained and different families are not averaged. This prevents fabrication but does not satisfy the explicit requirement that each report show its metric definition and distinguish task success from test-case pass fraction. A search of the complete report consumer finds no family-specific semantic branch.
- Suggested closure: Expose the actual selected native metric definition and its appropriate denominator without converting fractions into task success rates; add partial-result cases.

### P19-AUD-012: Native trajectories fabricate measured zero Agent duration

- Axis: runtime-fidelity
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-008-telemetry
- Claim: native_agent_record inserts Duration::from_secs(0) without observing Agent duration; projection serializes it as a Known wall_time_ms fact with origin supervisor-wall-clock. Missing native timing is therefore represented as measured zero rather than unknown with a reason.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:1544` — Native record hard-codes wall_time to zero.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:1978` — That record is passed to the production ProjectionPipeline and sealed.
- Evidence: `crates/opi-eval/src/trajectory/mod.rs:624` — Projection unconditionally records Known duration with supervisor-wall-clock provenance.
- Reproduction: `cargo test -p opi-eval --locked --test audit_phase19_probes audit_native_missing_time_becomes_measured_zero -- --nocapture`. Probe setup is in Appendix A.
- Refutation attempted: Unknown token/cost/retry fields are labelled in report native_facts, and the outer job has measured duration. Neither makes this synthesized Agent duration measured. Both public native trial bundles in the probe contain the false Known zero provenance.
- Suggested closure: Carry an observed Agent duration or a typed unknown reason through the native record and sealed projection; keep outer job timing separate.

## Verification Commands

All Cargo commands below used the existing repository external cache `/mnt/e/opi/cargo-targets/opi-8abcf38f9871653e`, `CARGO_INCREMENTAL=1`, and an active cache lease. No per-session Cargo target or cache deletion occurred. The Windows workspace itself was not used for runtime tests.

| Command | Result | Scope |
|---|---|---|
| `python .agents/skills/_shared/scripts/validate_assurance_artifact.py rotation docs/snapshots/phase19` | PASS, before requirements inspection | Rotation names/state only; no prior conclusions read. |
| `python scripts/opi-cargo-cache.py status` | PASS, existing external caches inspected | No cache mutation/cleanup. |
| `python3 scripts/opi-cargo-cache.py resolve --workspace /mnt/d/Luiz/Odradek/opi` | PASS, existing WSL target resolved | Cache selection. |
| `cargo test -p opi-eval --all-targets --locked` | PASS: 273 passed, 7 ignored, 0 failed | Current committed crate tests in WSL export. |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p "test_*.py"` | PASS: 205 run, OK (skipped=1) | Deterministic script/integration/artifact tests. |
| `cargo test -p opi-eval --locked --test audit_phase19_probes audit_ -- --nocapture` | PASS: initial 6 defect-characterization probes | The assertions establish defects, not conformance success. |
| `cargo test -p opi-eval --locked --test audit_phase19_probes audit_unfulfillable_storage_is_ignored -- --nocapture` | PASS: 1 probe | Impossible disk admission reproduced. |
| `cargo test -p opi-eval --locked --test audit_phase19_probes audit_rejected_rerun_changes_existing_control -- --nocapture` | PASS: 1 strengthened re-run of an existing probe | Subsequent report also refuses original root as run-control-drift. |
| `cargo test -p opi-eval --locked --test audit_phase19_probes audit_native_missing_time_becomes_measured_zero -- --nocapture` | PASS: 1 probe | Fabricated measured-zero timing reproduced. |
| `python3 scripts/opi-doc-check.py` | PASS | Export documentation contracts. |
| `cargo fmt --check --all` | PASS | Committed source formatting; transient harness removed. |
| `cargo clippy -p opi-eval --all-targets --locked -- -D warnings` | PASS | Current committed Companion, all targets. |
| Actual-client protocol/output-control/turn-budget matrices, three real native grader witnesses, real Docker cancellation | NOT RUN | Seven ignored cases require supplied exact materials/native environment. |
| `python3 crates/opi-eval/scripts/verify-local-acceptance.py --root <original-redacted-evidence-root> --out <fresh-output> --companion <built-opi-eval>` | NOT RUN | Original operator root not supplied. |
| Full workspace clippy/test/doc-test/rustdoc gate union and real first-connection acceptance | NOT RUN | Crate checks and ledger declarations do not stand in for these final-state obligations. |

Test impact: **retain** existing repository tests; **add** eight scratch-only characterization probes; outgoing repository test impact **none**. The exact added harness is reproduced below. No product code, spec, implementation state, dependency, fixture or test file is changed in the user worktree.

## Verdict Rationale

Mandatory states: **15 met**, **14 not-assessable**, **12 not-met**, **28 partially-met**. Two optional real-service qualifications remain not-assessable. Any mandatory non-met state mechanically requires FAIL. Twelve reciprocal findings contain current source evidence and attempted refutation: 2 Blocker and 10 Major. The missing raw dynamic evidence is an explicit limitation, not an inference that no real trial ever occurred. The report is complete as an audit; it does not claim phase completion.

## Appendix A — Reproducible isolated probes

Use a fresh export of the exact audit head. Copy `crates/opi-eval/tests/native_driver.rs` to `crates/opi-eval/tests/audit_phase19_probes.rs` **only in that export**, then append the Rust block below. It reuses the committed public-CLI fixture scaffolding, digest-refreshes only staged synthetic material, and never edits production code. Run the named exact test/filter with the shared Cargo cache leased. The first six ran together; resource, strengthened-root and timing checks ran as separate exact filters as recorded above. Agent/provider/native-engine behavior is simulated only where explicitly stated; defect assertions observe the actual Companion subprocess boundary, sealed artifacts or report output.

```rust

fn audit_replace_verifier(staged: &Staged, script: &str) {
    let path = staged.root.join("material/verifier-uv.sh");
    write_exec(&path, script);
    let mut material: serde_json::Value = serde_json::from_slice(&fs::read(&staged.material).unwrap()).unwrap();
    for (_, benchmark) in material["benchmarks"].as_object_mut().unwrap() {
        benchmark["verifier_executable"]["sha256"] = sha256_hex(script.as_bytes()).into();
        benchmark["oracle"]["sha256"] = sha256_hex(script.as_bytes()).into();
    }
    fs::write(&staged.material, serde_json::to_vec_pretty(&material).unwrap()).unwrap();
}

#[test]
fn audit_regrade_argv_and_ambient_credential() {
    let staged = stage("ok");
    let script = verifier_stand_in_script().replace(
        "  identity=\"$OPI_EVAL_REGRADE_IDENTITY\"",
        "  printf '%s\\n' \"${ANTHROPIC_API_KEY-unset}\" > audit-ambient.txt\n  printf '%s\\n' \"$#\" > audit-argc.txt\n  identity=\"$OPI_EVAL_REGRADE_IDENTITY\"");
    audit_replace_verifier(&staged, &script);
    let config = write_config(&staged);
    let (root, _) = regrade_family_root(&staged, &config, "run");
    let out = staged.root.join("regrade");
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .args(["regrade", "--root"]).arg(&root)
        .arg("--native-material").arg(&staged.material).arg("--out").arg(&out)
        .env("ANTHROPIC_API_KEY", "opi-audit-dummy-credential").output().unwrap();
    assert!(output.status.success(), "{} {}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    let files = walk_files_matching(&out, "audit-ambient.txt");
    assert_eq!(files.len(), 2);
    for file in files { assert_eq!(fs::read_to_string(file).unwrap().trim(), "opi-audit-dummy-credential"); }
    for file in walk_files_matching(&out, "audit-argc.txt") { assert_eq!(fs::read_to_string(file).unwrap().trim(), "0"); }
    println!("AUDIT: both verifier processes received dummy ambient ANTHROPIC_API_KEY and zero command arguments");
}

#[test]
fn audit_terminal_identity_mismatch_admitted() {
    let staged = stage("ok");
    let script = verifier_stand_in_script().replace("\"$product\" \"$(printf 'a%.0s'", "\"oracle\" \"$(printf 'f%.0s'")
        .replace("\"model_info\": {\"name\": \"scripted/eval\"}", "\"model_info\": {\"name\": \"unrelated/model\"}");
    audit_replace_verifier(&staged, &script);
    let config = write_config(&staged);
    let root = staged.root.join("run");
    let (code, stdout, stderr) = run_native(&staged, &config, &root);
    assert_eq!(code, 0, "{stderr} {stdout}");
    let terminals = walk_files_matching(&root, "opi-eval-terminal.json");
    assert_eq!(terminals.len(), 2);
    for file in terminals {
        let value:serde_json::Value=serde_json::from_slice(&fs::read(file).unwrap()).unwrap();
        assert_eq!(value["product"], "oracle");
        assert_eq!(value["agent_executable_sha256"], "f".repeat(64));
    }
    println!("AUDIT: accepted two native scores with terminal product oracle, substituted executable hash and unrelated result model: {stdout}");
}

#[test]
fn audit_nonzero_provider_error_enters_scores() {
    let staged = stage("agent-nonzero");
    let script = verifier_stand_in_script().replace("answer=\"native stand-in answer\"", "printf '%s\\n' '{\"type\":\"message_end\",\"message\":{\"role\":\"assistant\",\"stopReason\":\"error\",\"errorMessage\":\"401 invalid API key\"}}' > \"$job/$trial/agent/stdout.log\"\nanswer=\"native stand-in answer\"");
    audit_replace_verifier(&staged, &script);
    let config=write_config(&staged);
    let root=staged.root.join("run");
    let (code,stdout,stderr)=run_native(&staged,&config,&root);
    assert_eq!(code,0,"{stderr} {stdout}");
    let report=Command::new(env!("CARGO_BIN_EXE_opi-eval")).arg("report").arg("--root").arg(&root).output().unwrap();
    assert!(report.status.success());
    let value:serde_json::Value=serde_json::from_slice(&report.stdout).unwrap();
    assert_eq!(value["coverage_counts"]["valid"],2);
    assert_eq!(value["coverage_counts"]["agent_failure"],2);
    println!("AUDIT: captured terminal authentication errors were scored as valid Agent failures; denominator={}", value["metrics"][0]["denominator"]);
}

#[test]
fn audit_native_time_limit_not_applied() {
    let staged=stage("ok");
    let script=verifier_stand_in_script().replace("set -eu\n", "set -eu\nsleep 2\n");
    audit_replace_verifier(&staged,&script);
    let config=write_config(&staged);
    let text=fs::read_to_string(&config).unwrap().replace("[execution]\n", "[execution]\ntime_limit_seconds = 1\nagent_timeout_seconds = 1\nverifier_timeout_seconds = 1\n");
    fs::write(&config,text).unwrap();
    let start=std::time::Instant::now();
    let (code,stdout,stderr)=run_native(&staged,&config,&staged.root.join("run"));
    assert_eq!(code,0,"{stderr} {stdout}");
    assert!(start.elapsed().as_secs()>=4);
    println!("AUDIT: declared 1-second time/agent/verifier bounds accepted completed scores after {}ms",start.elapsed().as_millis());
}

#[test]
fn audit_plan_material_panics() {
    let root=tempfile::tempdir().unwrap();
    let output=Command::new(env!("CARGO_BIN_EXE_opi-eval")).args(["run","--plan"]).arg(root.path().join("plan.json"))
        .arg("--native-material").arg(root.path().join("material.json"))
        .arg("--root").arg(root.path().join("run")).arg("--fixtures").arg(fixtures()).output().unwrap();
    let stderr=String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(),Some(101),"{stderr}");
    assert!(stderr.contains("checked above: --native-material pairs with --config"));
    println!("AUDIT: --plan with --native-material panics before opening the files: {stderr}");
}

#[test]
fn audit_rejected_rerun_changes_existing_control() {
    let staged=stage("ok");let config=write_config(&staged);let root=staged.root.join("run");
    let (code,stdout,stderr)=run_native(&staged,&config,&root);assert_eq!(code,0,"{stderr} {stdout}");
    let before=fs::read(root.join("run-control.json")).unwrap();
    let text=fs::read_to_string(&config).unwrap().replace("experiment_id = \"native-driver-test\"", "experiment_id = \"audit-unrelated-run\"");fs::write(&config,text).unwrap();
    let (code,stdout,stderr)=run_native(&staged,&config,&root);assert_ne!(code,0,"{stderr} {stdout}");
    let after=fs::read(root.join("run-control.json")).unwrap();
    assert_ne!(before,after);
    let report=Command::new(env!("CARGO_BIN_EXE_opi-eval")).arg("report").arg("--root").arg(&root).output().unwrap();
    let value:serde_json::Value=serde_json::from_slice(&report.stdout).unwrap();
    assert_eq!(value["outcome"], "verification-failed");
    assert!(String::from_utf8_lossy(&report.stdout).contains("run-control-drift"));
    println!("AUDIT: report now rejects original sealed root as run-control-drift");
    println!("AUDIT: rejected rerun changed existing run-control.json; exit={code}; {stderr}");
}

#[test]
fn audit_unfulfillable_storage_is_ignored() {
    let root=tempfile::tempdir().unwrap();let cache=root.path().join("cache");seed_local_cache(&cache);
    let config=experiment_with_cache(root.path(),&cache);let out=root.path().join("material");
    fs::write(cache.join("packages/terminal-bench-2.1/task.toml"), "artifacts = [\"/root/answer.txt\"]\n[task]\nid = \"openssl-selfsigned-cert\"\n[environment]\ncpus = 1\nmemory_mb = 256\ngpus = 0\nstorage_mb = 9000000000000\nos = \"linux\"\n").unwrap();
    let result=run_prepare(&config,&out);let stdout=String::from_utf8_lossy(&result.stdout);let stderr=String::from_utf8_lossy(&result.stderr);
    assert!(result.status.success(),"{stdout} {stderr}");
    let value:serde_json::Value=serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["preflight"]["tasks"][0]["status"],"ok");
    assert!(value["preflight"]["tasks"][0]["requirements"].get("storage_mb").is_none());
    println!("AUDIT: prepare admitted storage_mb=9000000000000, omitted storage requirement, status=ok");
}

#[test]
fn audit_native_missing_time_becomes_measured_zero() {
    fn count_zero(v: &serde_json::Value) -> usize {
        match v {
            serde_json::Value::Object(map) => map.iter().map(|(key,value)| {
                usize::from(key == "wall_time_ms" && value["Known"]["value"] == 0 && value["Known"]["origin"] == "supervisor-wall-clock") + count_zero(value)
            }).sum(),
            serde_json::Value::Array(items) => items.iter().map(count_zero).sum(),
            _ => 0,
        }
    }
    let staged=stage("ok");let config=write_config(&staged);let root=staged.root.join("run");
    let (code,stdout,stderr)=run_native(&staged,&config,&root);assert_eq!(code,0,"{stderr} {stdout}");
    let files=walk_files_matching(&root,"trajectory.json");assert_eq!(files.len(),2);
    for file in files {
        let value:serde_json::Value=serde_json::from_slice(&fs::read(file).unwrap()).unwrap();
        assert!(count_zero(&value)>=1);
    }
    println!("AUDIT: both native trajectories claim measured Agent wall_time_ms=0 with origin supervisor-wall-clock although the adapter records no measured Agent duration");
}
```

## Appendix B — Audit-local verification log identities

These hashes identify audit-local raw command captures, not native/live acceptance artifacts. Their concise results and reproducible commands are preserved above.

| Log | SHA-256 |
|---|---|
| `cargo-eval.log` | `76954fabd2e39e4bfe1622643cf06b2cd27b57f73576596850b02d79d9381e6f` |
| `python-eval.log` | `4f0a814a69a55e713ac9b64bbefc4293750984372a4c1d6a3cc952e0b9cf735e` |
| `audit-probes.log` | `e1c723d6129fba2faf86f0de883b5be89a8e448cdc092094c722b567d1395a18` |
| `audit-resource-probe.log` | `f28e73915920246c1c7b54f44dada91a87e2c2db4baab54d7b37d80364d49c3e` |
| `audit-rerun-probe.log` | `1489043b0e8e903a0d76543e6f27c2299b3323c6b0504f684eb7564fbd28d8d0` |
| `audit-time-probe.log` | `bcecba14ba789ebc6baac0415b03ea5a41ad53b7a201f588d034d73e1ce3ed13` |
| `doc-check.log` | `f6daa323bc29663fca184eebe13f625b82f750ee6dc2f9a70be199bc9397309c` |
| `fmt.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `clippy.log` | `943db3200daf8a92ee0442c6b775bb17b03a87840857bb5bb30760ed0c8752a2` |
| `source-integrity.txt` | `d3ed0703031f115af74b6f9c033315b0e59c6e200b3e385ee3a132d749f2511c` |
