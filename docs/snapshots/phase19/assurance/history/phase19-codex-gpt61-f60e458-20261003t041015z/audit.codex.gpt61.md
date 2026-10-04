# Phase 19 Audit

**Audit run ID**: `phase19-codex-gpt61-f60e458-20261003t041015z`
**Audit head**: `f60e458f9d1bceffcba5caaef11e8c0d7312ff59`
**Reviewer ID**: `codex`
**Model ID**: `gpt61`
**Reviewer identity**: Codex
**Reviewer model ID**: `gpt61`
**Model identity source**: operator-declared
**Independence**: fresh-context-same-family. Fresh independent context and bounded read-only subreviews excluded previous conclusions and sibling peer reports; model-family independence is not asserted.
**Baseline policy**: latest-committed-spec
**Verdict**: FAIL

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| `.opi-impl-state.json` | `c65c10635c4bb8b1754625b94e8a0a15459eef6d65634610ba9b65ea9ba5337e` | Committed raw bytes; ledgers exposed only through allowlisted scope reader. |
| `docs/snapshots/phase19/opi-impl-state.json` | `35b3378bd902cc976bb348d794dd1b5d3c5f160a58769c3aeb62fea175a7b7a9` | Committed raw bytes; ledgers exposed only through allowlisted scope reader. |
| `docs/opi-spec.md` | `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db` | Committed raw bytes; ledgers exposed only through allowlisted scope reader. |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Committed raw bytes; ledgers exposed only through allowlisted scope reader. |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.zh.md` | `d41c826412b0d71802327d60f953708f6cb2083436a2a2b0d99a2e52d851f194` | Committed raw bytes; ledgers exposed only through allowlisted scope reader. |

No stored-hash mismatch was projected. All implementation reads and checks used the Git-archive export of the sealed head. The requirements were sealed before production inspection; their identities/mandatory flags/criteria/behaviors remain unchanged. Criterion citations use the registered AC identifiers; the pre-seal section-symbol text has encoding noise and is preserved verbatim rather than rewritten after inspection.

## Requirement Conformance

Every record has production consumers, discriminating tests, check observations or an explicit limitation. Tests marked ignored or synthetic are not substituted for actual-native/live predicates. The exact path/hash/citation and full evidence fields live in the requirements sidecar.

| Requirement ID | Criterion / observable behavior | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| `P19-AC-001-diagnose` | Doctor reports Linux engine, architecture, resources and missing inputs without models. | crates/opi-eval/src/runner/prepare.rs:143-337 doctor observations and partial material inventory; crates/opi-eval/src/cli/doctor.rs; tests: crates/opi-eval/tests/fail_closed_cli.rs:179-233 engine failure/missing materials and no mutation. See sidecar check observations. | partially-met | P19-AUD-003, P19-AUD-010 |
| `P19-AC-001-prepare` | Prepare produces verified local task, runner, executable and immutable image inventories from an independent directory without GitHub receipt requirements. | crates/opi-eval/src/runner/prepare.rs:1177-1471 selected acquisitions, identities and raw version/context probes; crates/opi-eval/src/runner/material.rs:358-471,668-918 selected products and full corpus byte closure; tests: crates/opi-eval/scripts/test_prepare_acquisition.py supervised acquisition/refusal tests; crates/opi-eval/tests/native_driver.rs:5180,5826,6011,6473 local independent preparation, runner/tool/image drift and pi-only material. See sidecar check observations. | partially-met | P19-AUD-003 |
| `P19-AC-001-plan` | Plan previews exact subjects, connection, selection, repetitions, trial count, concurrency and bounds before effects. | crates/opi-eval/src/runner/plan.rs:355-453 inventory, expansion and bound admission; crates/opi-eval/src/experiment.rs:690-723,1423-1425 execution integer parsing; tests: crates/opi-eval/tests/assembled_run.rs:1428-1542 exact selected tasks/repetitions; crates/opi-eval/tests/native_driver.rs:6847-6990 full inventory and selected limits. See sidecar check observations. | partially-met | P19-AUD-001 |
| `P19-AC-001-run` | Public CLI executes exactly its frozen selected task plan from an independent directory and saves stage/settlement results. | crates/opi-eval/src/runner/experiment.rs:538-588 frozen/config resolution and explicit-trial branch; crates/opi-eval/src/cli/run.rs public plan/config dispatch; tests: crates/opi-eval/tests/assembled_run.rs frozen selection and whole-group replacement; crates/opi-eval/tests/native_driver.rs:8558-8674 immutable plan/material drift and two-subject dispatch. See sidecar check observations. | partially-met | P19-AUD-001, P19-AUD-002 |
| `P19-AC-001-live` | Supplied redacted operator evidence proves the independent prepare/run/report assembled real workflow. | crates/opi-eval/scripts/verify-local-acceptance.py redacted raw operator-root verifier; crates/opi-eval/src/cli/verify.rs and src/cli/report.rs sealed offline reconstruction; tests: crates/opi-eval/scripts/test_verify_local_acceptance.py synthetic positive/negative root fixtures; crates/opi-eval/tests/native_driver.rs:10068 fixture evidence refuses actual acceptance. See sidecar check observations. | not-assessable | P19-AUD-009 |
| `P19-AC-002-binding` | Result admission binds expected/observed selected Agent build, task, job, trial, environment, delivered artifact and verifier input. | crates/opi-eval/src/benchmark/process.rs:476-697 fresh unique job/Agent/task/runtime binding; crates/opi-eval/src/benchmark/process.rs:718-965 native reward/lifecycle/collection/delivered admission; tests: crates/opi-eval/tests/native_driver.rs:2244-2304 wrong Agent/task/artifact, stale job and missing terminal rejected; crates/opi-eval/src/benchmark/process.rs:3130-3373 aggregate/collection/substitution negatives. See sidecar check observations. | met | - |
| `P19-AC-002-reject` | Missing/wrong/oracle Agent, stale job and substituted artifact cannot become scores even at process exit zero. | crates/opi-eval/src/benchmark/process.rs:476-697 fail-closed selected job reader; crates/opi-eval/src/runner/experiment.rs:1760-1894 terminal/capture/identity admission; tests: crates/opi-eval/tests/native_driver.rs:2244-2304 substitutions never seal; crates/opi-eval/tests/native_driver.rs:12723 substituted observed task reference rejects. See sidecar check observations. | met | - |
| `P19-AC-002-settlement` | One runner settlement owner combines typed terminal observations, admitted native grade and lifecycle into an evidence-bound sealed classification. | crates/opi-eval/src/runner/experiment.rs:1408-1506 native_trial_class and2680-2715 settlement sealing; crates/opi-eval/src/agent/process.rs AgentFailureClass; tests: crates/opi-eval/tests/native_driver.rs:2108-2240 scored zero/task timeout/nonzero versus authority failures; crates/opi-eval/tests/report_contract.rs:208-327 denominators and failure classes. See sidecar check observations. | met | - |
| `P19-AC-002-capture` | Failed attempts preserve capturable redacted native evidence independently of success; capture/seal rejection cannot manufacture scored success. | crates/opi-eval/src/native_capture.rs typed privacy projection; crates/opi-eval/src/native_capture_runtime.rs:130-158,506-529 diagnostic admission and persistence; tests: crates/opi-eval/tests/native_driver.rs:11724,11794,11876 private thinking/overflow/provider diagnostic cases; crates/opi-eval/tests/native_driver.rs:9646 interrupted bounded partial capture. See sidecar check observations. | partially-met | P19-AUD-006 |
| `P19-AC-003-lifecycle` | TB2.1 grades the same Agent trial environment after credential cleanup under selected native lifecycle. | crates/opi-eval/integrations/harbor/opi_eval_harbor.py:426-487,666-799 preflight, native launch and credential cleanup; crates/opi-eval/src/benchmark/terminal_bench_21.rs:742-903 single selected upstream job; tests: crates/opi-eval/tests/native_driver.rs:1457-1523 actual temp credential file cleanup in production integration seam; crates/opi-eval/tests/native_driver.rs:2015-2105 same-job binding and no independent workspace. See sidecar check observations. | met | - |
| `P19-AC-003-witness` | TB2.1 correct/incorrect native artifact witnesses traverse production handoff and change real grader outcomes. | crates/opi-eval/src/main.rs explicit native conformance/regrade/run entry; crates/opi-eval/src/runner/material.rs verified actual client/runner/material identities; tests: crates/opi-eval/tests/native_driver.rs ignored native_tb21_artifact_witness/native_tb3_artifact_witness/native_deepswe_patch_witness; crates/opi-eval/tests/native_driver.rs ignored native_protocol_client_matrix/native_verifier_only_regrade_witness/native_owned_resource_cancellation. See sidecar check observations. | not-assessable | P19-AUD-008 |
| `P19-AC-003-live` | Both actual Agents supply real provider tool effects, native handoff and attributable TB2.1 scores. | crates/opi-eval/scripts/verify-local-acceptance.py redacted raw operator-root verifier; crates/opi-eval/src/cli/verify.rs and src/cli/report.rs sealed offline reconstruction; tests: crates/opi-eval/scripts/test_verify_local_acceptance.py synthetic positive/negative root fixtures; crates/opi-eval/tests/native_driver.rs:10068 fixture evidence refuses actual acceptance. See sidecar check observations. | not-assessable | P19-AUD-009 |
| `P19-AC-004-handoff` | TB3 honors selected task collection and separate verifier declarations, transferring only declared artifacts without widening network policy. | crates/opi-eval/src/benchmark/task_lifecycle.rs:201-322 declared separate-verifier lifecycle; crates/opi-eval/src/benchmark/process.rs:830-945 exact selected collection and digests; tests: crates/opi-eval/tests/native_driver.rs:2423-2512 separate handoff, declared artifacts only and substitution refusal. See sidecar check observations. | met | - |
| `P19-AC-004-witness` | TB3 real native positive/negative witnesses discriminate declared artifacts and exclude unrelated canary files. | crates/opi-eval/src/main.rs explicit native conformance/regrade/run entry; crates/opi-eval/src/runner/material.rs verified actual client/runner/material identities; tests: crates/opi-eval/tests/native_driver.rs ignored native_tb21_artifact_witness/native_tb3_artifact_witness/native_deepswe_patch_witness; crates/opi-eval/tests/native_driver.rs ignored native_protocol_client_matrix/native_verifier_only_regrade_witness/native_owned_resource_cancellation. See sidecar check observations. | not-assessable | P19-AUD-008 |
| `P19-AC-004-live` | Both real Agents traverse TB3 collected-artifact verifier boundary with attributable results. | crates/opi-eval/scripts/verify-local-acceptance.py redacted raw operator-root verifier; crates/opi-eval/src/cli/verify.rs and src/cli/report.rs sealed offline reconstruction; tests: crates/opi-eval/scripts/test_verify_local_acceptance.py synthetic positive/negative root fixtures; crates/opi-eval/tests/native_driver.rs:10068 fixture evidence refuses actual acceptance. See sidecar check observations. | not-assessable | P19-AUD-009 |
| `P19-AC-005-patch` | DeepSWE collects committed patch against declared base, applies exactly that patch in pristine verifier and rejects wrong base/substitution/uncommitted replacement. | crates/opi-eval/src/benchmark/deepswe.rs:1005-1022 separate pristine/no-network/collect lifecycle; crates/opi-eval/src/benchmark/process.rs:1081-1154 base/image/lifecycle and collected byte admission; tests: crates/opi-eval/tests/native_driver.rs:2569-2686 committed patch/base/image binding, drift rejection and honest empty-patch zero; crates/opi-eval/tests/support/native_witness_material.rs:183-185,255-257 committed-only collect and pristine patch apply harness. See sidecar check observations. | met | - |
| `P19-AC-005-witness` | DeepSWE native correct/incorrect committed-patch witnesses reach the real production grader. | crates/opi-eval/src/main.rs explicit native conformance/regrade/run entry; crates/opi-eval/src/runner/material.rs verified actual client/runner/material identities; tests: crates/opi-eval/tests/native_driver.rs ignored native_tb21_artifact_witness/native_tb3_artifact_witness/native_deepswe_patch_witness; crates/opi-eval/tests/native_driver.rs ignored native_protocol_client_matrix/native_verifier_only_regrade_witness/native_owned_resource_cancellation. See sidecar check observations. | not-assessable | P19-AUD-008 |
| `P19-AC-005-live` | Both actual Agents supply patch-to-native-result traces under declared verifier policy. | crates/opi-eval/scripts/verify-local-acceptance.py redacted raw operator-root verifier; crates/opi-eval/src/cli/verify.rs and src/cli/report.rs sealed offline reconstruction; tests: crates/opi-eval/scripts/test_verify_local_acceptance.py synthetic positive/negative root fixtures; crates/opi-eval/tests/native_driver.rs:10068 fixture evidence refuses actual acceptance. See sidecar check observations. | not-assessable | P19-AUD-009 |
| `P19-AC-006-projection` | Both native Agent configurations consume explicitly selected provider/protocol/endpoint/exact model and isolated credential reference. | crates/opi-eval/src/experiment.rs:1151-1378 exact connection projection for NativeClient::Opi/Pi; crates/opi-eval/integrations/harbor/opi_eval_harbor.py isolated Opi/Pi config; tests: crates/opi-eval/src/experiment.rs projection tests for all three protocols and selected model; crates/opi-eval/tests/native_driver.rs:1524 file-reference credential projection. See sidecar check observations. | met | - |
| `P19-AC-006-fixture` | Actual Opi and pi clients complete local fake-server streamed tool loops for all three named protocols with authentication and request-path proof. | crates/opi-eval/src/main.rs explicit native conformance/regrade/run entry; crates/opi-eval/src/runner/material.rs verified actual client/runner/material identities; tests: crates/opi-eval/tests/native_driver.rs ignored native_tb21_artifact_witness/native_tb3_artifact_witness/native_deepswe_patch_witness; crates/opi-eval/tests/native_driver.rs ignored native_protocol_client_matrix/native_verifier_only_regrade_witness/native_owned_resource_cancellation. See sidecar check observations. | not-assessable | P19-AUD-008 |
| `P19-AC-006-controls` | Effective sampling/reasoning/context/output controls and origins are retained; required unsupported or mismatched controls fail before effects. | crates/opi-eval/src/experiment.rs:1032-1067 required unsupported control rejection; crates/opi-eval/src/runner/output_control.rs:35-53 exact-client wire-bound proof; tests: crates/opi-eval/tests/native_driver.rs:1821 required sampling fails before dispatch; crates/opi-eval/tests/native_driver.rs:1921 optional requested unsupported values retained. See sidecar check observations. | met | - |
| `P19-AC-006-terminal` | Native terminal auth/rate-limit/protocol errors are distinguished from valid zero and transient 429 then success, including error JSON at exit zero. | crates/opi-eval/src/agent/pi.rs:295-348 retry-aware typed native provider terminal importer; crates/opi-eval/src/agent/opi.rs typed native terminal importer; tests: crates/opi-eval/tests/agent_integration_conformance.rs helper-produced saved streams discriminate auth/rate/protocol/retry; crates/opi-eval/tests/native_driver.rs:2108-2172 invalid provider versus graded Agent outcome. See sidecar check observations. | partially-met | P19-AUD-011 |
| `P19-AC-006-fallback` | No implicit provider/model/endpoint/protocol/billing fallback; missing actual service model/prices remain unknown with reasons. | crates/opi-eval/src/experiment.rs explicit ConnectionApi/provider/base URL/model selection; crates/opi-eval/src/runner/experiment.rs selected connection facts; tests: crates/opi-eval/tests/experiment_contract.rs missing/incompatible selection rejects; crates/opi-eval/tests/report_contract.rs:125-207 asymmetric native facts and unknown telemetry. See sidecar check observations. | met | - |
| `P19-AC-006-live` | Both real native clients prove first Anthropic Zhipu glm-5.3-flash connection, effective controls and preserved real tool-loop evidence. | crates/opi-eval/scripts/verify-local-acceptance.py redacted raw operator-root verifier; crates/opi-eval/src/cli/verify.rs and src/cli/report.rs sealed offline reconstruction; tests: crates/opi-eval/scripts/test_verify_local_acceptance.py synthetic positive/negative root fixtures; crates/opi-eval/tests/native_driver.rs:10068 fixture evidence refuses actual acceptance. See sidecar check observations. | not-assessable | P19-AUD-009 |
| `P19-AC-007-inventory` | Full pinned corpus inventory preserves instructions/lifecycle/network/CPU/memory/GPU/disk/time requirements without starting every task image. | crates/opi-eval/src/runner/prepare.rs:1670-1716,2823-3165 complete pinned corpus inventory and requirement origins; crates/opi-eval/src/runner/plan.rs:355-416 full inventory plus explicit selection; tests: crates/opi-eval/tests/native_driver.rs:6847 full three-task inventory retains infeasible unselected GPU task; crates/opi-eval/tests/native_driver.rs:5370,5462 corpus metadata links pinned or refused. See sidecar check observations. | met | - |
| `P19-AC-007-preflight` | WSL/Docker resource insufficiency, unknown observations and unsupported selected semantics are visible and prevent execution without implicit subset filtering. | crates/opi-eval/src/runner/prepare.rs:459-484,802-897 selected resource observations and unknown refusal; crates/opi-eval/src/runner/prepare.rs:281-337,2637-2640 unbounded raw observers; tests: crates/opi-eval/tests/native_driver.rs:6520 insufficient/unknown selected CPU/memory/GPU fails; crates/opi-eval/tests/native_driver.rs:6656 unknown engine storage prevents run. See sidecar check observations. | partially-met | P19-AUD-003 |
| `P19-AC-007-freeze` | Explicit full/subset IDs/reasons/repetitions/seeds, material/image/executable identities and narrowed task/control bounds freeze into digest-bound plan; drift fails closed. | crates/opi-eval/src/runner/plan.rs digest-bound exact selection/material/task/control plan; crates/opi-eval/src/runner/experiment.rs:538-588 config/frozen plan admission; tests: crates/opi-eval/tests/native_driver.rs:8558-8674 frozen plan runs and material drift rejects; crates/opi-eval/tests/assembled_run.rs:1428-1542 repetitions/seeds and trial identities. See sidecar check observations. | partially-met | P19-AUD-001, P19-AUD-002 |
| `P19-AC-008-metrics` | Reports retain each native metric meaning and denominator, label subset/incomplete full scope, and never average benchmarks into a headline. | crates/opi-eval/src/report.rs:634-866 native metric meanings/test counts/denominators; crates/opi-eval/src/report.rs:786-815 unknown benchmark fallback when zero seals; tests: crates/opi-eval/tests/report_contract.rs:208-327 failure classes/native denominators; crates/opi-eval/tests/report_contract.rs:125-207 asymmetric native observations. See sidecar check observations. | partially-met | P19-AUD-004 |
| `P19-AC-008-coverage` | Corpus/selected/planned/attempted/valid/Agent-failure/invalid/unexecuted counts and all exclusions are preserved, including zero sealed trials. | crates/opi-eval/src/runner/experiment.rs:703-749 durable planned inventory before effects; crates/opi-eval/src/report.rs:340-440 sealed-only authoritative experiment; tests: crates/opi-eval/tests/bundle_recompute.rs:152-207 planned/unexecuted counts survive total bundle loss; crates/opi-eval/tests/native_driver.rs:11566 full-selection frozen task counts after bundle loss. See sidecar check observations. | partially-met | P19-AUD-004 |
| `P19-AC-008-classification` | Report consumes verified sealed interpretation: native zero and valid task timeout are scored; terminal provider/adapter/identity/infra/invalid-grade failures are attempted but unscored. | crates/opi-eval/src/runner/experiment.rs:1408-1506,2680-2715 sealed native classification; crates/opi-eval/src/report.rs:880-1066 reward eligibility from verified settlement; tests: crates/opi-eval/tests/native_driver.rs:2108-2240 task timeout/nonzero kept graded; provider errors invalid; crates/opi-eval/tests/native_driver.rs:7003 provider failures versus native-zero report coverage. See sidecar check observations. | met | - |
| `P19-AC-008-pairs` | Only same-task/repetition/frozen-control/protocol groups pair; missing pairs remain visible and single subjects need no artificial edge. | crates/opi-eval/src/comparison.rs task/repetition/control/protocol grouping; crates/opi-eval/src/report.rs:1395-1609 coverage and consistency with sealed contract; tests: crates/opi-eval/tests/report_contract.rs:410-491 protocol-safe pairs and single-subject behavior; crates/opi-eval/tests/pairing_and_integrity.rs frozen identity/invalid exclusions. See sidecar check observations. | partially-met | P19-AUD-004 |
| `P19-AC-008-telemetry` | Unknown telemetry keeps reasons and distinct estimated/provider/billed origins rather than zero fabrication. | crates/opi-eval/src/report.rs:1213-1307 reasoned measured/native/estimated unknown facts; crates/opi-eval/src/trajectory.rs Fact provenance and unknown reasons; tests: crates/opi-eval/tests/report_contract.rs:125-207 asymmetric usage/cost/native facts; crates/opi-eval/tests/report_contract.rs:208-327 failure categories retain correct denominator. See sidecar check observations. | met | - |
| `P19-AC-009-verify` | Verify only checks original byte/seal integrity and rejects tampered/unsealed or unsupported legacy evidence. | crates/opi-eval/src/cli/verify.rs byte/seal-only OfflineRegrader; crates/opi-eval/src/regrade.rs:81-194 read-only source verification; tests: crates/opi-eval/tests/bundle_recompute.rs:15-87 linked/junction source refusal without mutation; crates/opi-eval/tests/bundle_recompute.rs:259-457 covered byte and retained closure mutation rejects. See sidecar check observations. | met | - |
| `P19-AC-009-recompute` | Report reconstruction is deterministic offline with no Agent/provider/network and cannot overwrite source or earlier outputs. | crates/opi-eval/src/report.rs sealed/control byte-only deterministic reconstruction; crates/opi-eval/src/cli/report.rs:64-107 canonical containment and create-new output; tests: crates/opi-eval/tests/end_to_end_report.rs independent runs/repeated reports are byte identical; tamper blocks publication; crates/opi-eval/tests/bundle_recompute.rs:498 repeated offline operations preserve source. See sidecar check observations. | met | - |
| `P19-AC-009-regrade` | Native regrade consumes preserved actual artifacts with pinned cached native verifier in a fresh owned environment and writes a source-referencing derived record without altering originals. | crates/opi-eval/src/regrade.rs:314-387 source verification and missing output containment; crates/opi-eval/src/regrade.rs:537-655 source reverify then task copy into output before later rejection; tests: crates/opi-eval/tests/native_driver.rs:7812-7901 external-output offline regrade source preserved; crates/opi-eval/tests/native_driver.rs:9272 fail-closed artifact/base/network/cache cases. See sidecar check observations. | partially-met | P19-AUD-005 |
| `P19-AC-009-regrade-reject` | Missing artifacts/cache/service offline equivalent, wrong base or tamper rejects regrade; no Agent launch or credential resolution occurs. | crates/opi-eval/src/regrade.rs:525-1088 missing/altered artifacts/material/base/image/network/cache admission; crates/opi-eval/src/cli/regrade.rs no Agent/credential resolution; tests: crates/opi-eval/tests/native_driver.rs:9272 every fail-closed case; crates/opi-eval/tests/native_driver.rs:8678 no ambient secret in verifier child. See sidecar check observations. | met | - |
| `P19-AC-009-witness` | Real native witnesses for all three families prove offline regrade responds to changed saved artifact inputs and preserves source bytes. | crates/opi-eval/src/main.rs explicit native conformance/regrade/run entry; crates/opi-eval/src/runner/material.rs verified actual client/runner/material identities; tests: crates/opi-eval/tests/native_driver.rs ignored native_tb21_artifact_witness/native_tb3_artifact_witness/native_deepswe_patch_witness; crates/opi-eval/tests/native_driver.rs ignored native_protocol_client_matrix/native_verifier_only_regrade_witness/native_owned_resource_cancellation. See sidecar check observations. | not-assessable | P19-AUD-008 |
| `P19-AC-010-credentials` | Only selected supervised Agent resolves private credentials; global config untouched; canary absent from grader, handoff and export, temporary secrets removed before shared verifier. | crates/opi-eval/src/runner/experiment.rs selected connection credential projection, private stage and canary scan; crates/opi-eval/integrations/harbor/opi_eval_harbor.py credential-stage cleanup before shared verifier; tests: crates/opi-eval/tests/native_driver.rs:1457 removes actual temporary credential files; crates/opi-eval/tests/native_driver.rs:1727 private0700/0600 stage before first byte. See sidecar check observations. | met | - |
| `P19-AC-010-bounds` | Positive bounded concurrency and enforceable time/output/usage bounds apply by narrowing; required unenforceable caps reject before effects. | crates/opi-eval/src/experiment.rs:690-723,1423-1425 required bounds parsing; crates/opi-eval/src/runner/plan.rs:427-453 selected count limit only; tests: crates/opi-eval/tests/experiment_contract.rs:936-999 valid numeric bound negatives; crates/opi-eval/tests/native_driver.rs:6925-6990 selection cap rejection. See sidecar check observations. | not-met | P19-AUD-001, P19-AUD-002, P19-AUD-003 |
| `P19-AC-010-cancel` | One shared supervisor stops affected dispatch on cancellation/breach, settles observations and cleans only owned process trees/containers and temporary credentials, with orphan detection. | crates/opi-eval/src/process.rs and src/process/tree.rs shared bounded process-tree owner; crates/opi-eval/src/runner/native_cleanup.rs exact owned Compose/container cleanup; tests: crates/opi-eval/tests/native_driver.rs:9441 native signal stops affected dispatch; crates/opi-eval/tests/native_driver.rs:9646 truthful bounded partial capture. See sidecar check observations. | partially-met | P19-AUD-003 |
| `P19-AC-010-recovery` | Recovery is offline and preserves completed/cancelled/failed/effect-unknown/unstarted lifecycle plus sealed outcome classifications without inventing scores. | crates/opi-eval/src/lifecycle.rs monotone intent/effect/settlement/seal states; crates/opi-eval/src/runner/experiment.rs:4182-4401 offline recovery without redispatch; tests: crates/opi-eval/tests/assembled_run.rs:686-974 unchanged originals/effect-unknown recovery; crates/opi-eval/tests/native_driver.rs:10894 rejected rerun preserves prior control. See sidecar check observations. | met | - |
| `P19-AC-010-replacement` | Fresh root and identities replace whole paired group under same frozen scope while preserving previous immutable attempt classification/evidence. | crates/opi-eval/src/runner/experiment.rs:4121-4180 fresh whole target paired group; crates/opi-eval/src/cli/replace_group.rs existing replacement facade; tests: crates/opi-eval/tests/assembled_run.rs:44-215 same frozen scope, fresh experiment/trial identities and original byte preservation; crates/opi-eval/tests/native_driver.rs:10942 one owner of simultaneous root claim. See sidecar check observations. | met | - |
| `P19-AC-010-native-cleanup` | Actual local no-model Docker cancellation witness proves owned resources stop and unrelated files/cache remain intact. | crates/opi-eval/src/main.rs explicit native conformance/regrade/run entry; crates/opi-eval/src/runner/material.rs verified actual client/runner/material identities; tests: crates/opi-eval/tests/native_driver.rs ignored native_tb21_artifact_witness/native_tb3_artifact_witness/native_deepswe_patch_witness; crates/opi-eval/tests/native_driver.rs ignored native_protocol_client_matrix/native_verifier_only_regrade_witness/native_owned_resource_cancellation. See sidecar check observations. | not-assessable | P19-AUD-008 |
| `P19-AC-011-default` | Default/CI tests are deterministic, credential-free and do not activate live dispatch when dummy credentials exist. | crates/opi-eval/src/cli/run.rs explicit fixture/native mode selection; crates/opi-eval/scripts/native-smoke.sh explicit local native invocation; tests: crates/opi-eval/tests/fail_closed_cli.rs populated dummy credentials cannot alter fixture evidence or enable live mode; crates/opi-eval/tests/authority_boundaries.rs no activation or hidden runtime dependency. See sidecar check observations. | met | - |
| `P19-AC-011-ci` | CI is a thin caller of shared local material/conformance/run/verify/report policy; live acceptance remains explicit and independently identified. | crates/opi-eval/scripts/materialize-external-locks.sh:603-775 shared prepare/conformance policy plus transport receipt; crates/opi-eval/scripts/native-smoke.sh local prepare/conformance/run/verify/report delegation; tests: crates/opi-eval/scripts/test_verify_native_smoke_ci.py workflow/transport boundaries; crates/opi-eval/scripts/test_verify_external_lock_ci.py cached offline transport identity. See sidecar check observations. | met | - |
| `P19-AC-011-verifier` | Offline local acceptance verifier rejects oracle/substitution/missing/fixture-only evidence and validates six trials, controls, command results, identities, native artifacts and report reconstruction without Agent/auth/provider access. | crates/opi-eval/scripts/verify-local-acceptance.py:640-682 runtime identity and unconditional task-success requirement; crates/opi-eval/scripts/verify-local-acceptance.py sealed artifacts/provider/tool/provenance and actual verify/report command checks; tests: crates/opi-eval/scripts/test_verify_local_acceptance.py substituted/missing/oracle/fixture/wrong repository and command-exit negatives; crates/opi-eval/tests/native_driver.rs:10068 fixture-only acceptance refusal. See sidecar check observations. | partially-met | P19-AUD-007 |
| `P19-AC-011-provenance` | Opi-only generic-audit capture view is nonempty, exhaustive and byte-bound to three original Opi captures; pi/Harbor/Pier provenance stays repository-qualified and validates offline in correct upstream caches. | crates/opi-eval/scripts/verify-local-acceptance.py redacted raw operator-root verifier; crates/opi-eval/src/cli/verify.rs and src/cli/report.rs sealed offline reconstruction; tests: crates/opi-eval/scripts/test_verify_local_acceptance.py synthetic positive/negative root fixtures; crates/opi-eval/tests/native_driver.rs:10068 fixture evidence refuses actual acceptance. See sidecar check observations. | not-assessable | P19-AUD-009 |
| `P19-AC-012-six` | At least six valid real-provider native-graded trials exist, one feasible task per family by each Opi/pi Agent on selected first connection; auth/infra/missing loop/invalid handoff cannot count. | crates/opi-eval/scripts/verify-local-acceptance.py redacted raw operator-root verifier; crates/opi-eval/src/cli/verify.rs and src/cli/report.rs sealed offline reconstruction; tests: crates/opi-eval/scripts/test_verify_local_acceptance.py synthetic positive/negative root fixtures; crates/opi-eval/tests/native_driver.rs:10068 fixture evidence refuses actual acceptance. See sidecar check observations. | not-assessable | P19-AUD-009 |
| `P19-AC-012-freeze` | Operator evidence freezes task/build/runner/material/model/control/concurrency/usage/time scope before effects and preserves failed attempts/replacement identities. | crates/opi-eval/scripts/verify-local-acceptance.py redacted raw operator-root verifier; crates/opi-eval/src/cli/verify.rs and src/cli/report.rs sealed offline reconstruction; tests: crates/opi-eval/scripts/test_verify_local_acceptance.py synthetic positive/negative root fixtures; crates/opi-eval/tests/native_driver.rs:10068 fixture evidence refuses actual acceptance. See sidecar check observations. | not-assessable | P19-AUD-009 |
| `P19-AC-012-raw` | Dynamic acceptance is supported by accessible preserved redacted raw commands/exits/logs/bundles/provider/tool/native proofs and reproducible reports. | crates/opi-eval/scripts/verify-local-acceptance.py redacted raw operator-root verifier; crates/opi-eval/src/cli/verify.rs and src/cli/report.rs sealed offline reconstruction; tests: crates/opi-eval/scripts/test_verify_local_acceptance.py synthetic positive/negative root fixtures; crates/opi-eval/tests/native_driver.rs:10068 fixture evidence refuses actual acceptance. See sidecar check observations. | not-assessable | P19-AUD-009 |
| `P19-AC-012-gates` | Final relevant state has actual WSL/Linux acceptance, workspace verification union and artifact-audit evidence; missing proof blocks Phase exit. | crates/opi-eval/scripts/verify-local-acceptance.py redacted raw operator-root verifier; crates/opi-eval/src/cli/verify.rs and src/cli/report.rs sealed offline reconstruction; tests: crates/opi-eval/scripts/test_verify_local_acceptance.py synthetic positive/negative root fixtures; crates/opi-eval/tests/native_driver.rs:10068 fixture evidence refuses actual acceptance. See sidecar check observations. | not-assessable | P19-AUD-009 |
| `P19-AC-013-placement` | Companion has no Opi crate dependency, pi-only runtime has value without Opi, and ordinary Minimal Runtime remains unaffected. | crates/opi-eval/Cargo.toml publish=false and no Opi crate dependency in any table; crates/opi-eval/src/runner/material.rs:353-414 exactly selected products; tests: crates/opi-eval/tests/native_driver.rs:10253 pi-only runtime/packaging without Opi; crates/opi-eval/tests/native_driver.rs:6473 pi-only preparation needs no Opi source. See sidecar check observations. | met | - |
| `P19-AC-013-package` | Task corpora/native grader implementations/images/sandbox/hosted policy are external; no speculative stable SDK/new Core seam/dependency. | crates/opi-eval/Cargo.toml include/package and dependency contracts; crates/opi-eval/src/runner/material.rs external pinned corpora/runners/images; tests: crates/opi-eval/tests/native_driver.rs:10253 public CLI/package independence; crates/opi-eval/tests/authority_boundaries.rs no Core/runtime activation. See sidecar check observations. | met | - |
| `P19-AC-013-contracts` | Changed 0.x CLI/config/material/bundle/report contracts synchronize help/schemas/fixtures/callers/bilingual docs/examples and deliberate Unreleased records. | crates/opi-eval/src/main.rs generated help for doctor/prepare/plan/run/verify/report/regrade/recover/replace; crates/opi-eval/src/experiment.rs versioned config; src/bundle/mod.rs seals; src/report.rs report/2; tests: crates/opi-eval/tests/experiment_contract.rs schema/CLI invalid inputs; crates/opi-eval/tests/report_contract.rs operation semantics. See sidecar check observations. | met | - |
| `P19-AC-013-legacy` | Original bundles/snapshots remain immutable; legacy conformance is labelled or unsupported and never relabelled native scores. | crates/opi-eval/src/runner/material.rs legacy material readable for sealed offline use, fresh dispatch requires helper; crates/opi-eval/src/bundle/mod.rs immutable original interpretation and byte closure; tests: crates/opi-eval/tests/native_driver.rs:12579 installed bytes and legacy fresh-dispatch refusal with offline load allowed; crates/opi-eval/tests/bundle_recompute.rs repeated read-only operations unchanged. See sidecar check observations. | partially-met | P19-AUD-005 |
| `P19-AC-013-rollback` | Documented rollback disables affected optional path or restores verified build without reviving known invalid attribution, broadening authority or auto-activation. | crates/opi-eval/README.md local operator rollback instructions; crates/opi-eval/README.zh.md:589-594 disable affected path/restore verified Companion without obsolete scoring; tests: crates/opi-eval/tests/authority_boundaries.rs no ordinary Opi activation; crates/opi-eval/tests/fail_closed_cli.rs explicit operation admission. See sidecar check observations. | met | - |

## Standards Review

The Companion remains publish=false with no internal Opi dependency in any manifest table, and pi-only preparation/execution has a production CLI consumer without Opi runtime material. cargo metadata/package-list and authority-boundary tests support inward dependency placement and absence of bundled full corpora/native graders/images. The integrations are external-runner adapter glue. No new stable SDK/Core seam was justified or found in the scoped implementation. Current schema/help/fixtures/bilingual README/examples and deliberate Unreleased contract entries are synchronized structurally; exported doc check, fmt, clippy and warning-denied rustdoc pass.

Fail-closed validation is incomplete: malformed execution bounds are silently erased001, explicit trial count skips the admitted cap002, and selected diagnosis/preparation observers bypass the shared bounded supervisor003. Original-source immutability is not enforced for nested native regrade output005. Comments/Unreleased claim a validated privacy capture, but the stderr implementation admits arbitrary token-bearing payload006. These are current behavior findings, independent of prior delivery assertions.

## Spec Review

Selected native job/Agent/task/runtime/artifact binding, evidence-bound settlement, same/shared versus declared separate lifecycle, exact connection projection, explicit unsupported control rejection, sealed reward denominators and offline operation distinction are traced to production and negative tests. Zero/task nonzero/benchmark-valid timeout versus provider/infra-invalid classification is exercised by the production settlement seam. Known planning/report information is lost when zero trials seal004, and the operator acceptance verifier nevertheless insists on successful per-Agent terminals007.

Required real artifact sensitivity, actual-client three-protocol tool loops, offline native regrade and actual Docker cleanup witnesses were not executed by the default suite008. No redacted raw acceptance root was supplied, so six actual real-provider trials, pre-effect freeze, cross-repository/scoped capture evidence and final workspace/WSL/artifact-audit predicates are not assessable009. Earlier reports/ledger delivery conclusions were deliberately excluded. These are evidence limitations, not claims that prior runs never existed or authorization to spend/access credentials.

## Security, Invariants, Integration, Test Quality, and Residuals

Security: private native credential staging0700/0600, selected reference resolution, known canary/credential scan and cleanup before shared verifier have discriminating production-seam tests. Regrade clears environment and does not resolve Agent credentials. These protections do not validate unknown token-bearing stderr006. No actual secret was used in the marker probe.

Invariants/integration: unique fresh result selection rejects oracle/wrong/stale/substituted results before score; durable intent/settlement/seal and offline replacement preserve originals under normal external roots. Existing report containment is sound but the native regrade facade omits it005. Pi Anthropic/Responses errors remain invalid/unscored, yet lose promised typed protocol precision011. Doctor missing-input observations are incomplete010.

Test quality: passing fixtures establish owning mechanism behavior, not actual external grader/client execution. Count-only zero-seal assertions do not establish missing-pair/native benchmark reconstruction004. Existing numeric bound/selection tests miss quoted and explicit-list branches001/002. The accepted structural acceptance fixture is not live evidence; its targeted terminal mutation proves the unconditional verifier gate007. Static ordered source mutation proof005 states its legitimate source/material preconditions and does not claim an actual native reproduction.

Residuals: explicit legacy/fixture paths remain labelled and do not become fresh native dispatch authority. No actionable claim is made from speculative helper/receipt binding or delivered-artifact completeness leads; those lacked sufficient falsifiable proof in this run. No prior Git deletion count is inferred. Near-identical Harbor/Pier adapter modules serve different upstream APIs and do not by themselves justify an added public abstraction.

## Minimum-change Conformance

All eleven admitted tasks have reuse_search/surface_necessity/simplification_ceiling declarations in the safe projection. The following current-code assessments identify consumers, tests, net deletion limits and remaining glue; actionable failures use normal finding axes.

| Task | Status | Current evidence and seam assessment |
|---|---|---|
| 19.1 | conforming | Existing BenchmarkExecution, private NativeTrialBinding, per-family adapters, AgentFailure and one runner settlement owner reused. Durable binding/seal value is intrinsic; no new public generic Rust seam. Consumers: run_trial, all three native adapters, conformance and verified report. Tests: native_driver and importer/bundle unit tests. Current legacy newest-job/oracle-score helper routes absent; no historical numeric deletion asserted. Residual explicit hermetic fixture branch is labelled; capture defect006 remains a runtime finding. |
| 19.2 | drifted | ModelControls/NativeClient/finite ConnectionApi projection reused; two real adapter consumers plus run/output-control/conformance justify projection seam. Tests: projection/importer matrices and native_driver. Fixed native config helpers absent; selected private Pi credential config and fixture script remain necessary glue. Exact typed protocol causes drift011; real protocol proof gap008. No Cargo edge or Eval LLM client. |
| 19.3 | conforming | TB3 adapter reuses shared lifecycle/binding reader; private declared collection has intrinsic state. Consumers: native run, conformance, cached regrade lifecycle. Tests: declared/substituted artifacts and witness harness. No numeric deletion claim; no separate result importer introduced. External Harbor collection authority remains residual glue; actual proof gap008. |
| 19.4 | conforming | DeepSWE adapter reuses selected Pier lifecycle/shared binding/material; no patch runner/Core dependency. Consumers: native run/conformance/regrade; tests: base/image/collection/empty-patch and actual witness harness. No numeric deletion claim. Different upstream Harbor/Pier APIs require module glue; synthetic helper path remains explicitly fixture-only. Actual witness proof gap008. |
| 19.5 | triggered | NativeMaterial, pinned corpus/runner/runtime identities and materializer/process owner reused. CLI doctor/prepare, plan/run/regrade and shared CI are production consumers; acquisition/native_driver/fail_closed tests are nonproduction consumers. Local origin removes upload-receipt prerequisite while byte provenance remains; no numeric history claim. Duplicate raw probes violate bounded-observation ceiling003; incomplete input inventory010 should reuse the existing selected material owner. |
| 19.6 | triggered | Existing resolver, frozen selection and runner provide intrinsic persisted plan identity; consumers: plan plus run --config/--plan and offline consumers. Tests: selected count/drift/repetition/concurrency. No new scheduler or dependency needed. No promised numeric deletion; explicit hermetic/native preflight difference remains labelled. Bound parsing/count ceiling fails001/002; fix final resolved list before effects. |
| 19.7 | triggered | Existing verify/report facade and single ReportBuilder consume verified settlement; public CLI/report record has durable reconstruction value. Tests: byte closure, denominators, pairing and deterministic output. Native exit/stderr score inference replaced by sealed interpretation; explicit legacy compatibility read remains. Zero-seal contract loss004 violates frozen report ceiling; use existing run-control rather than a second authority/ledger. |
| 19.8 | drifted | Recorded reuse names BenchmarkExecution/family adapters/ReportBuilder/NativeTrialBinding; current regrade dispatches ProcessSupervisor directly and uses a private FreshVerifierObservation/request-bound reader. This factual drift alone is not an overengineering Major: verifier-only execution must avoid Agent job dispatch. Consumers: NativeRegrader CLI and both cached native regrade integrations; tests: fail-closed/external-source preservation and ignored real witness. No net deletion count; private verifier-only glue is intrinsically stateful. Source containment ceiling fails005. |
| 19.9 | conforming | Existing ProcessSupervisor/tree, cancellation token, TrialLifecycle/AuthorityLedger and run/recover/replace reused; exact-owned resource cleanup is private stateful mechanism. Consumers: Agent/benchmark processes, native/regrade cleanup and offline recovery/replacement; tests: signal/process/fixture-container cleanup and immutable replacement. No extra knobs/dependencies or numeric deletion claim. Docker ownership/capture glue remains; actual no-model witness is unassessed008, and prepare raw probes outside this owner are003. |
| 19.10 | drifted | Existing workflows/scripts and public local CLI reused for policy; CI retains transport verification. Acceptance verifier is a distinct offline raw-evidence operation with intrinsic provenance state; production consumers: operator command and explicit CI transport/native-smoke callers; tests:272 Python cases plus spawned CLI independence/dummy credentials. Shared prepare/conformance replaces duplicated CI material admission; no historical numeric delta claimed. Residual receipt/hash/cross-repository verification is necessary glue. Success-only acceptance gate007 contradicts eligible native outcome semantics. |
| 19.11 | not-assessable | Safe scope registers a separate raw-operator verification/exit task, not another runtime seam or ledger. Consumers: supplied operator-root offline verifier, verify/report and generic scoped artifact auditor. No deletion requirement; raw records and bilingual acceptance handoff are delivery artifacts. Earlier handoff conclusions are excluded; no raw root supplied. Live/final exit states remain open009; source capability and scoped tests cannot certify six actual trials. |

## Findings

### P19-AUD-001: Quoted execution bounds silently disappear before the plan freezes

- Axis: security
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-010-bounds, P19-AC-007-freeze, P19-AC-001-plan, P19-AC-001-run
- Claim: Nonempty TOML strings at integer execution-limit fields validate as absence. A requested usage/output/time/trial bound can therefore be erased instead of rejected or enforced before effects.
- Evidence: `crates/opi-eval/src/experiment.rs:1423` — optional_u64 maps None and every Some(String(_)) to None; callers690-723 own seven integer execution limits.
- Evidence: `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-codex-gpt61-o95pfn2b\prepare-probes\results.json` — Seven quoted1 inputs exit0 with identical resolved digest8d85d2ffac59bbb30041b5b9dba7d74410821c44d43fd45180cef564cdd95a83; quoted max_total_tokens avoids explicit numeric unenforceable-cap rejection.
- Refutation attempted: experiment_contract.rs:936-999 rejects numeric unenforceable/contradictory/nonpositive bounds and output_control.rs:35-53 requires numeric wire proof. Those checks occur after optional parsing and cannot recover a string converted to None; no current invalid-type assertion covers these strings.
- Reproduction: Copy local-paired.toml to an isolated document directory with valid fixture paths; add [execution] mode="hermetic" and each of max_total_tokens,time_limit_seconds,max_output_bytes,max_trials,agent_timeout_seconds,verifier_timeout_seconds,max_output_tokens="1" in turn; run opi-eval validate --config C --fixtures F.
- Reproduction: Exact seven input files, argv and outputs: C:\Users\Luiz\AppData\Local\Temp\opi-phase19-codex-gpt61-o95pfn2b\prepare-probes\results.json; executable probe: C:\Users\Luiz\AppData\Local\Temp\opi-phase19-codex-gpt61-o95pfn2b\prepare-probes.py.
- Suggested closure: Reject non-integer supplied execution limits at the existing parser, and assert no plan/run effects on malformed required bounds. Preserve explicitly documented omission spelling only if the owning contract requires it.
- Confidence: high; normalized status at publication: unverified (remediation must independently confirm).

### P19-AUD-002: Explicit trial lists bypass max_trials

- Axis: invariants
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-010-bounds, P19-AC-001-run, P19-AC-007-freeze
- Claim: The max_trials admission check applies only to selection expansion. A public explicit-trial config with max_trials=1 executes and seals both declared trials.
- Evidence: `crates/opi-eval/src/runner/plan.rs:445` — The only cap check follows selection expansion.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:538` — None selection branch accepts the explicit experiment without resolved-count enforcement.
- Evidence: `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-codex-gpt61-o95pfn2b\prepare-probes\results.json` — Hermetic paired fixture with max_trials1 exits0/completed, saves two sealed trials, and reports execution:2 trials,width1.
- Refutation attempted: native_driver.rs:6925-6990 correctly rejects a two-subject selection at cap1; assembled_run.rs1428-1542 checks frozen selected repetitions. Both reach selection, not the unchecked explicit-trial branch. Material/image/identity admission does not impose the count cap.
- Reproduction: Run a fresh copy of tests/fixtures/experiment/local-paired.toml with [execution] mode="hermetic", max_trials=1 using opi-eval run --config C --root NEW --fixtures F --behavior happy.
- Reproduction: Exact config/output retained under C:\Users\Luiz\AppData\Local\Temp\opi-phase19-codex-gpt61-o95pfn2b\prepare-probes; probe C:\Users\Luiz\AppData\Local\Temp\opi-phase19-codex-gpt61-o95pfn2b\prepare-probes.py.
- Suggested closure: Check every final trial list before run-control/effects in the existing owner. Check selected count with bounded arithmetic before allocating the repetitions vector; retain current selection tests and add explicit-list rejection.
- Confidence: high; normalized status at publication: unverified (remediation must independently confirm).

### P19-AUD-003: Doctor and preparation probes escape bounded supervision

- Axis: runtime-fidelity
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-001-diagnose, P19-AC-001-prepare, P19-AC-007-preflight, P19-AC-010-bounds, P19-AC-010-cancel
- Claim: Raw synchronous engine/version/df/context/GPU probes have no deadline, capture cap or cancellation. A stalled selected observer blocks the public operation and on the current-thread runtime prevents preparation cancellation from being polled.
- Evidence: `crates/opi-eval/src/runner/prepare.rs:281` — Docker version uses blocking Command::output; equivalent raw calls at333-337,1369-1372,1463-1471,2637-2640 and298-306.
- Evidence: `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-codex-gpt61-o95pfn2b\prepare-probes\results.json` — Isolated fake docker exec sleep60 left doctor blocked after15.026s with no output; harness killed only its own process group.
- Evidence: `crates/opi-eval/src/runner/prepare.rs:2299` — Existing observation_command already provides16KiB, explicit deadline and verified cleanup, but raw callers bypass it.
- Refutation attempted: prepare.rs2299-2341,655-677 and2116-2150 supervise image observation/acquisition with timeout/cancel/cleanup. fail_closed_cli.rs179-233 uses a quickly refusing engine. These correct paths do not wrap the remaining synchronous probes or exercise a stalled observer.
- Reproduction: In an isolated PATH place an executable docker that records its own PID and execs sleep60; invoke public doctor --config C in a fresh process group, with a15-second outer watchdog; capture output and terminate only the probe group.
- Reproduction: Exact safe harness C:\Users\Luiz\AppData\Local\Temp\opi-phase19-codex-gpt61-o95pfn2b\prepare-probes.py and results C:\Users\Luiz\AppData\Local\Temp\opi-phase19-codex-gpt61-o95pfn2b\prepare-probes\results.json; static raw calls establish absent supervisory bound rather than inferring infinity from15 seconds.
- Suggested closure: Use the existing bounded observation/ProcessSupervisor owner for all local probes, sharing preparation deadline/cancellation and explicit unknown reasons; add public stalled-observer and cancellation coverage.
- Confidence: high; normalized status at publication: unverified (remediation must independently confirm).

### P19-AUD-004: Zero-seal reports lose frozen pair and benchmark identity

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-008-metrics, P19-AC-008-coverage, P19-AC-008-pairs
- Claim: A genuine interruption before any seal publishes counts but drops all declared pair coverage and renders known native benchmark identity as unknown/unknown. Missing pairs cannot be reconstructed from the incomplete pre-effect run-control.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:703` — Durable planned rows omit benchmark/subjects/edges/integrity exclusions and frozen report controls.
- Evidence: `crates/opi-eval/src/report.rs:340` — Authoritative experiment restored only from sealed trials; diagnostic unsealed contract does not populate it.
- Evidence: `crates/opi-eval/src/report.rs:1395` — Without experiment.edges/trials coverage returns empty;786-815 metric benchmark falls back unknown/unknown.
- Evidence: `/tmp/opi-evidence-zero-seal-qsubxxpp/report.stdout.json` — Untouched public crash-after-intent root has1 intent/0 seals; run70, report0, planned2/unexecuted2, coverage=[],control_pairs=[],benchmark unknown/unknown.
- Refutation attempted: bundle_recompute.rs152-207 asserts planned2/attempted0/unexecuted2 after total bundle loss and output-containment tests check counts/conflicts. Neither asserts declared pair coverage, benchmark identity or exclusions. Scores remain absent correctly; this is lost known planning/report information, not fabricated scores.
- Reproduction: In a fresh isolated root run opi-eval run --config tests/fixtures/experiment/local-paired.toml --root NEW --fixtures tests/fixtures --behavior crash-after-intent; expected exit70. On the same untouched root run opi-eval report --root NEW.
- Reproduction: Exact env-cleared argv/results are in C:\Users\Luiz\AppData\Local\Temp\opi-phase19-codex-gpt61-o95pfn2b\evidence-review.json; original raw report /tmp/opi-evidence-zero-seal-qsubxxpp/report.stdout.json.
- Suggested closure: Preserve sufficient frozen report contract in existing pre-effect run-control and reconstruct planned/missing coverage and native metric identity from it; only verified settlements may supply scored denominators.
- Confidence: high; normalized status at publication: unverified (remediation must independently confirm).

### P19-AUD-005: Nested regrade output can invalidate its original source seal

- Axis: invariants
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-009-regrade, P19-AC-013-legacy
- Claim: A fresh --out inside the canonical source's bundle/artifacts is accepted. Regrade then copies task files there before later lifecycle/base/image checks, adding unmanifested source files and invalidating subsequent verify/report, even when grading later rejects.
- Evidence: `crates/opi-eval/src/regrade.rs:314` — Canonical source exists but only out.exists() is checked at328; mkdir at378/380 precedes output canonicalization387.
- Evidence: `crates/opi-eval/src/regrade.rs:537` — Initial empty nested directories pass file-only source reverify; task copy at623 follows create_dir_all620-621, before later checks625-655.
- Evidence: `crates/opi-eval/src/bundle/mod.rs:1409` — All new files inside retained artifact tree reject as UnmanifestedFile;1617-1641 ignores only empty directories.
- Evidence: `crates/opi-eval/src/cli/report.rs:81` — Canonical-parent source containment exists for report output but is absent from regrade.
- Refutation attempted: native_driver.rs7812-7901 normal external outputs preserve source hashes;9272+ tests missing/tampered/base/network/cache/existing-output refusal;10978+ ignored real witness also asserts preservation. Every tested output is external. Existing-output rejection does not exclude a new nested path, and report's separate facade guard does not protect regrade.
- Reproduction: Static ordered proof: with a legitimate sealed native source R, admitted cached material M and a trial T, invoke opi-eval regrade --root R --native-material M --out R/trials/T/bundle/artifacts/derived. Before native verifier effects, task copy adds source files; opi-eval verify --root R then rejects unmanifested files.
- Reproduction: Actual native reproduction was not run; the explicit source/material preconditions and earliest mkdir/copy/verification ordering are the invariant proof.
- Suggested closure: Before any mkdir canonicalize the prospective output through its nearest existing ancestor and reject any path/alias contained in the canonical source root; retain create-new external-output behavior and test source tree byte preservation.
- Confidence: high; normalized status at publication: unverified (remediation must independently confirm).

### P19-AUD-006: A diagnostic substring allows arbitrary stderr bytes into sanitized capture

- Axis: security
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-002-capture
- Claim: Stderr is classified validated-diagnostic when each line merely contains one allowed token, then the entire original line is written. Arbitrary private/provider payload surrounding a token crosses the first capture sink under a sanitized receipt in both capture policies.
- Evidence: `crates/opi-eval/src/native_capture_runtime.rs:130` — Allowed static tokens are tested with line.contains(token) at153 rather than a validated whole diagnostic grammar.
- Evidence: `crates/opi-eval/src/native_capture_runtime.rs:511` — Entire stderr_bytes written at517 whenever validated-diagnostic; receipt reports sanitized529.
- Evidence: `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-codex-gpt61-o95pfn2b\checks\stderr-probe.json` — Valid Pi stdout plus provider stream failed: PRIVATE_PAYLOAD_PROBE retained arbitrary suffix under both full and terminal-only; exit0/sanitized/validated-diagnostic.
- Refutation attempted: Strict typed stdout projection and native_driver.rs11724 private-thinking assertions work.11876 tests an exact provider diagnostic and oversized padding. Host1796-1807 checks receipt/digests, and selected credential/canary scans block known secret bytes before export. None validates/redacts arbitrary unlisted suffix text; canaries do not prove all unknown stderr private content safe. No actual credential exposure is claimed by this marker probe.
- Reproduction: Run offline probe C:\Users\Luiz\AppData\Local\Temp\opi-phase19-codex-gpt61-o95pfn2b\privacy-probe.py using the exported binary's __native-capture and current valid Pi fixture; its synthetic stderr contains only an innocuous private-payload marker.
- Reproduction: Inspect resulting stderr sink and receipt for both policies; no real credential or private provider data is used.
- Suggested closure: Persist only canonical typed diagnostic metadata or apply a fully validated privacy-safe grammar before the first sink; opaque bytes must retain only count/digest metadata. Add arbitrary prefix/suffix and private-marker stderr cases under both policies.
- Confidence: high; normalized status at publication: unverified (remediation must independently confirm).

### P19-AUD-007: Acceptance verifier demands Agent success for valid native task failures

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-011-verifier
- Claim: The local acceptance verifier rejects every nonzero/timed-out Agent terminal and any native exception before consuming benchmark-valid sealed classification, although eligible real task failures/timeouts may count among the six native trials.
- Evidence: `crates/opi-eval/scripts/verify-local-acceptance.py:665` — Task exit must equal0, timed_out=false and failure_kind=None; exception_info must be absent672.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:1832` — Production settlement admits Agent task timeout and scored nonzero at1839-1847; provider/infra/preflight failures remain invalid.
- Evidence: `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-codex-gpt61-o95pfn2b\checks\acceptance-probe.log` — Current structural EvidenceRoot baseline accepted0; changing only task terminal to exit1 or timeout124 rejects2 with terminal/stream/model differs. This is not actual live acceptance proof.
- Refutation attempted: Current acceptance tests reject arbitrary task exit3 to prevent unqualified failure evidence, and check_run_exit permits a paired command exit1. Neither distinguishes benchmark-valid Agent outcome from auth/infra or relaxes the per-task terminal success condition. Native settlement tests prove this distinction already exists. Completed tool-loop proof remains independently required; a timeout before any loop must not count.
- Reproduction: Run C:\Users\Luiz\AppData\Local\Temp\opi-phase19-codex-gpt61-o95pfn2b\acceptance-timeout-probe.py offline against current test_verify_local_acceptance.EvidenceRoot. Baseline succeeds; native-task nonzero/timeout variants trigger the unconditional gate.
- Reproduction: Inspect scored task failure/timeout production counterexamples in native_driver.rs2108-2240; valid zero remains eligible and provider errors remain excluded.
- Suggested closure: Validate eligibility using verified evidence-bound native settlement and actual benchmark terminal semantics, retaining runtime preflight/identity/tool/handoff proof. Add eligible graded nonzero/timeout cases and invalid provider/infra countercases.
- Confidence: high; normalized status at publication: unverified (remediation must independently confirm).

### P19-AUD-008: Required actual-native witnesses have no executed proof in this run

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-003-witness, P19-AC-004-witness, P19-AC-005-witness, P19-AC-006-fixture, P19-AC-009-witness, P19-AC-010-native-cleanup
- Claim: This independent run cannot assess the required real grader artifact/regrade witnesses, actual Opi/pi three-protocol tool-loop matrix or real Docker cleanup witness. Their production harnesses are ignored in the passing default suite and no applicable raw witness root was supplied.
- Evidence: `crates/opi-eval/tests/native_driver.rs:3613` — TB2.1/TB3/DeepSWE positive/negative real witness filters are ignored and require pinned material plus Docker.
- Evidence: `crates/opi-eval/tests/native_driver.rs:3998` — Actual-client protocol matrix is ignored; saved-stream conformance is not execution by actual clients.
- Evidence: `crates/opi-eval/tests/native_driver.rs:9773` — Actual owned-resource cancellation and10978 verifier-only native regrade witnesses are ignored.
- Evidence: `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-codex-gpt61-o95pfn2b\checks\rust.log` — 390 pass/11 ignored; no claim that the ignored mandatory native predicates executed.
- Refutation attempted: Default native_driver tests traverse production dispatch/import/report using discriminating subprocess/material stand-ins and assert correct versus incorrect artifact semantics. Ignored harness source has meaningful assertions. Neither a stand-in nor harness presence proves the real grader/client/engine predicate; no registered deferral makes these optional. This is scoped unavailable evidence, not proof that no witness has ever run.
- Reproduction: Inspect cargo test -p opi-eval --all-targets --locked --offline executed and ignored counts.
- Reproduction: To assess the missing predicates, supply redacted current raw witness roots with pinned actual material/client/runner/build identities and production grading/tool/cleanup evidence, or separately authorize/run the explicit ignored local no-paid-provider filters with those prerequisites.
- Suggested closure: Supply and independently verify applicable raw actual-native proofs, including artifact sensitivity, unrelated-file exclusion, all three APIs by both clients, offline native regrade and exact-owned cleanup; keep these states unassessed until verified.
- Confidence: high; normalized status at publication: unverified (remediation must independently confirm).

### P19-AUD-009: Six-trial operator acceptance and final exit evidence are unavailable

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-001-live, P19-AC-003-live, P19-AC-004-live, P19-AC-005-live, P19-AC-006-live, P19-AC-011-provenance, P19-AC-012-six, P19-AC-012-freeze, P19-AC-012-raw, P19-AC-012-gates
- Claim: No accessible redacted raw acceptance root was supplied for this audit. Six real-provider native trials, pre-effect freeze, full raw trace/bundle/provenance closure and final Phase verification/artifact-audit predicates remain not assessable.
- Evidence: `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md:275` — AC012 requires six actual native-graded real trials, and sections6/8 distinguish valid native zero/task failure from invalid provider/infra/missing-loop evidence.
- Evidence: `crates/opi-eval/scripts/verify-local-acceptance.py` — Offline verifier expects a raw root with six trial records, sealed artifacts, command/build/material/client identities, three exhaustive Opi captures and repository-qualified upstream caches.
- Evidence: `Independent input boundary of this audit run` — No raw root path was supplied after the early request. Prior acceptance conclusions and ledger delivery assertions were excluded; no auth or live call was performed.
- Refutation attempted: 272 Python tests include valid/invalid structural acceptance fixtures and390 Rust tests include fixture rejection and truthful native outcome paths. They establish code behavior, not actual operator facts. A passing scoped export check also cannot substitute final workspace/native/raw acceptance gates. No absence outside supplied independent inputs is asserted.
- Reproduction: Supply the redacted Phase19 operator manifest.json root and its contained raw files/cached repository identities, then run python crates/opi-eval/scripts/verify-local-acceptance.py --root E --out NEW with current verify/report and inspect all six bindings.
- Reproduction: Independently verify pre-effect frozen scope, three original Opi captures/exhaustive scoped audit view, pi/Harbor/Pier repository-qualified provenance, actual provider/tool/task/native evidence and final workspace/WSL/artifact-audit command receipts.
- Suggested closure: Provide accessible redacted raw proofs with applicability to the sealed head or unchanged relevant dependencies; verify them offline. Keep live acceptance and Phase exit open until evidence is sufficient.
- Confidence: high; normalized status at publication: unverified (remediation must independently confirm).

### P19-AUD-010: Doctor omits selected required material and tool observations

- Axis: spec
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: P19-AC-001-diagnose
- Claim: Doctor observes only the static lock and selected Agent paths, omitting selected corpus/profile, uv, runner project/lock/interpreter/entrypoint, integrations/provider and capture helper from missing-input diagnosis.
- Evidence: `crates/opi-eval/src/runner/prepare.rs:213` — Complete doctor material list213-248 versus prepare required inputs1177-1221,1757-1765.
- Evidence: `crates/opi-eval/tests/fail_closed_cli.rs:227` — Empty-cache test asserts only the three current material IDs.
- Refutation attempted: Prepare still rejects or acquires the missing inputs1228-1335, limiting impact to incomplete diagnosis rather than scoring absent material. The supplied subreview label Medium normalizes to Minor.
- Reproduction: Compare the full doctor material branch213-248 with prepare's selected required list1177-1221/1757-1765. In a partially seeded cache retain only lock and Agent files; no observation names the absent selected runner/corpus/helper/tools.
- Suggested closure: Share the existing selected-input inventory between doctor and prepare and extend empty/partially seeded cache diagnostic assertions without credential reads or another ledger.
- Confidence: high; normalized status at publication: unverified (remediation must independently confirm).

### P19-AUD-011: Pi typed protocol failure classification covers only Completions

- Axis: integration
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: P19-AC-006-terminal
- Claim: Pi's native importer recognizes only the Completions truncation string as provider-protocol-error; the exact Anthropic and Responses native protocol strings acknowledged by the client matrix fall through to generic provider-failure.
- Evidence: `crates/opi-eval/src/agent/pi.rs:339` — Exact match only Stream ended without finish_reason; all other known stream truncation messages generic.
- Evidence: `crates/opi-eval/tests/native_driver.rs:4135` — Actual matrix recognizes Anthropic stream ended without a stop reason and Responses stream ended before a terminal response event.
- Evidence: `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-codex-gpt61-o95pfn2b\native-protocol-probe\result.json` — Public copied-fixture conformance gives completions0/met=true/protocol-error; Anthropic and Responses1/met=false/provider-failure.
- Refutation attempted: All variants remain authority-boundary failures, raw cause is retained, attempts stay unscored, and native_driver.rs7547-7550 permits generic failure. Severity is limited to promised typed diagnostic precision. The ignored matrix checks raw native shape rather than importer cause, so does not refute it.
- Reproduction: Copy the Pi provider-protocol fixture; change only API and native message to the exact Anthropic/Responses matrix strings. Run public conformance --suite agent --adapter pi --case provider-protocol-error with copied fixtures and declared scripted provider.
- Reproduction: Exact argv/raw results: C:\Users\Luiz\AppData\Local\Temp\opi-phase19-codex-gpt61-o95pfn2b\native-protocol-probe\result.json. No actual-client/live run is claimed.
- Suggested closure: Recognize each supported exact native protocol shape at the existing importer and add discriminating cases for all three APIs; preserve authority failure/no-score semantics.
- Confidence: high; normalized status at publication: unverified (remediation must independently confirm).

## Verification Commands

All implementation commands below ran in the sealed export, on WSL/Linux where applicable, with the existing external Cargo cache resolved for the canonical repository and a temporary lease. No cache cleanup, per-session target, auth read or paid/live invocation occurred. Private probes are retained outside the export; source and test files were unchanged.

| Command | Result | Requirement/finding |
|---|---|---|
| `python -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py'` | PASS:272 tests,1 skipped | Acquisition/CI/acceptance verifier; synthetic scope only |
| `cargo test -p opi-eval --all-targets --locked --offline` | PASS:390 passed,0 failed,11 ignored | All existing Eval test binaries; actual-native gaps008 |
| `cargo fmt --check --all` | PASS | Standards |
| `cargo clippy -p opi-eval --all-targets --locked --offline -- -D warnings` | PASS | Standards |
| `cargo test -p opi-eval --doc --locked --offline` | PASS:0 doctests | Not substituted for behavioral proof |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p opi-eval --no-deps --locked --offline` | PASS | Current crate documentation |
| `cargo metadata --no-deps --format-version1 --locked --offline` | PASS | Independent dependency graph |
| `cargo package -p opi-eval --list --allow-dirty --locked --offline` | PASS | Unpublished packaging boundary |
| `python scripts/opi-doc-check.py`; exported `opi-eval --help` | PASS | Structural bilingual/manifests/current help |
| Public validation/run/doctor isolated probes | Reproduced001/002/003 | Exact inputs/argv/results in private prepare-probes |
| Public crash-after-intent then report, same untouched root | Reproduced004 | One intent/no seals; lost known report contract |
| Source ordered invariant and report containment counterpath | Static violation005 | Native material preconditions explicit; no dynamic native claim |
| Production capture under both policies with benign arbitrary stderr suffix | Reproduced006 | Sanitized receipt persists suffix |
| Structural acceptance root baseline/nonzero/timeout variants | Reproduced007 | Baseline accepts; valid task terminal gates reject |
| Public Pi protocol-error conformance for3 API shapes | Reproduced011 | Generic cause on Anthropic/Responses |
| Actual-native ignored witness filters / raw operator six-trial verification / final workspace exit union | NOT RUN / raw inputs unavailable | Explicit not-assessable008/009; no Phase exit inferred |

Private raw logs/probe scripts: `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-codex-gpt61-o95pfn2b`. Export source corresponds exactly to `f60e458f9d1bceffcba5caaef11e8c0d7312ff59`. Ordinary audit artifact checks (member/install validation, doc check and git diff --check) are recorded by the installer/check handoff; they do not revise the independent runtime evidence.

Test impact: **none** added/updated/deleted by this audit; **retain** existing suites. Repair suggestions identify missing assertions without authorizing implementation. Audit artifacts are single-run English assurance records; structural validation governs them and there is no separately registered Chinese assurance counterpart. Existing English/Chinese product/spec counterparts were inspected together and not edited.

## Verdict Rationale

FAIL is mechanical: mandatory requirement states are {'partially-met': 15, 'not-assessable': 16, 'met': 24, 'not-met': 1}. Seven current Major behavior defects001–007, two Major evidence gaps008–009 and two advisory Minor completeness/diagnostic findings010–011 survive current refutation. Passing scoped checks do not close those defects or establish missing actual proof. The report audits committed behavior and available evidence only; it changes no production source, delivery ledger, branch, commit or release state.

<!-- opi-audit-history/1
{
  "audit\u005fhead": "f60e458f9d1bceffcba5caaef11e8c0d7312ff59",
  "audit\u005frun\u005fid": "phase19-codex-gpt61-f60e458-20261003t041015z",
  "comparison": {
    "comparisons": [
      {
        "cause": "The current numeric max\u005foutput\u005fbytes path rejects the unenforceable request and its tests pass; string parsing still maps every String to omission before that guard. This is a different parser branch violating the same bound-admission invariant. No repaired-then-regressed transition is inferred.",
        "current\u005fevidence": "At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: Nonempty TOML strings at integer execution-limit fields validate as absence. A requested usage/output/time/trial bound can therefore be erased instead of rejected or enforced before effects. Evidence: crates/opi-eval/src/experiment.rs:1423 — optional\u005fu64 maps None and every Some(String(\u005f)) to None; callers690-723 own seven integer execution limits.; C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\prepare-probes\\results.json — Seven quoted1 inputs exit0 with identical resolved digest8d85d2ffac59bbb30041b5b9dba7d74410821c44d43fd45180cef564cdd95a83; quoted max\u005ftotal\u005ftokens avoids explicit numeric unenforceable-cap rejection.",
        "current\u005ffinding\u005fid": "P19-AUD-001",
        "prior\u005fclaim": "The superseded AUD-001 claimed a positive numeric captured-output bound was accepted without an enforcement route. It did not address nonempty TOML strings at integer execution fields.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-2465aeb-20261001t055252z",
            "finding\u005fid": "P19-AUD-001",
            "source\u005fid": "superseded-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "missed-branch",
        "repair\u005fcommits": [],
        "verification": "Current independent reproduction and refutation remain unchanged. Suggested remediation check: Copy local-paired.toml to an isolated document directory with valid fixture paths; add [execution] mode=\"hermetic\" and each of max\u005ftotal\u005ftokens,time\u005flimit\u005fseconds,max\u005foutput\u005fbytes,max\u005ftrials,agent\u005ftimeout\u005fseconds,verifier\u005ftimeout\u005fseconds,max\u005foutput\u005ftokens=\"1\" in turn; run opi-eval validate --config C --fixtures F. Exact seven input files, argv and outputs: C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\prepare-probes\\results.json; executable probe: C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\prepare-probes.py."
      },
      {
        "cause": "A reproduced current defect or scoped completeness gap is established by the installed core. The single allowed previous report does not prove when it arose, whether it was previously working, or a repair transition; no regression/recurrence is inferred from related titles.",
        "current\u005fevidence": "At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: The max\u005ftrials admission check applies only to selection expansion. A public explicit-trial config with max\u005ftrials=1 executes and seals both declared trials. Evidence: crates/opi-eval/src/runner/plan.rs:445 — The only cap check follows selection expansion.; crates/opi-eval/src/runner/experiment.rs:538 — None selection branch accepts the explicit experiment without resolved-count enforcement.; C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\prepare-probes\\results.json — Hermetic paired fixture with max\u005ftrials1 exits0/completed, saves two sealed trials, and reports execution:2 trials,width1.",
        "current\u005ffinding\u005fid": "P19-AUD-002",
        "prior\u005fclaim": "The immediately superseded codex/gpt61 normalized finding sidecar contains no claim for this exact branch/predicate. This is an absence claim only over its eleven published records, not over all earlier implementations, reviews or tests.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "insufficient-information",
        "repair\u005fcommits": [],
        "verification": "Current independent reproduction and refutation remain unchanged. Suggested remediation check: Run a fresh copy of tests/fixtures/experiment/local-paired.toml with [execution] mode=\"hermetic\", max\u005ftrials=1 using opi-eval run --config C --root NEW --fixtures F --behavior happy. Exact config/output retained under C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\prepare-probes; probe C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\prepare-probes.py."
      },
      {
        "cause": "A reproduced current defect or scoped completeness gap is established by the installed core. The single allowed previous report does not prove when it arose, whether it was previously working, or a repair transition; no regression/recurrence is inferred from related titles.",
        "current\u005fevidence": "At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: Raw synchronous engine/version/df/context/GPU probes have no deadline, capture cap or cancellation. A stalled selected observer blocks the public operation and on the current-thread runtime prevents preparation cancellation from being polled. Evidence: crates/opi-eval/src/runner/prepare.rs:281 — Docker version uses blocking Command::output; equivalent raw calls at333-337,1369-1372,1463-1471,2637-2640 and298-306.; C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\prepare-probes\\results.json — Isolated fake docker exec sleep60 left doctor blocked after15.026s with no output; harness killed only its own process group.; crates/opi-eval/src/runner/prepare.rs:2299 — Existing observation\u005fcommand already provides16KiB, explicit deadline and verified cleanup, but raw callers bypass it.",
        "current\u005ffinding\u005fid": "P19-AUD-003",
        "prior\u005fclaim": "The immediately superseded codex/gpt61 normalized finding sidecar contains no claim for this exact branch/predicate. This is an absence claim only over its eleven published records, not over all earlier implementations, reviews or tests.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "insufficient-information",
        "repair\u005fcommits": [],
        "verification": "Current independent reproduction and refutation remain unchanged. Suggested remediation check: In an isolated PATH place an executable docker that records its own PID and execs sleep60; invoke public doctor --config C in a fresh process group, with a15-second outer watchdog; capture output and terminate only the probe group. Exact safe harness C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\prepare-probes.py and results C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\prepare-probes\\results.json; static raw calls establish absent supervisory bound rather than inferring infinity from15 seconds."
      },
      {
        "cause": "A reproduced current defect or scoped completeness gap is established by the installed core. The single allowed previous report does not prove when it arose, whether it was previously working, or a repair transition; no regression/recurrence is inferred from related titles.",
        "current\u005fevidence": "At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: A genuine interruption before any seal publishes counts but drops all declared pair coverage and renders known native benchmark identity as unknown/unknown. Missing pairs cannot be reconstructed from the incomplete pre-effect run-control. Evidence: crates/opi-eval/src/runner/experiment.rs:703 — Durable planned rows omit benchmark/subjects/edges/integrity exclusions and frozen report controls.; crates/opi-eval/src/report.rs:340 — Authoritative experiment restored only from sealed trials; diagnostic unsealed contract does not populate it.; crates/opi-eval/src/report.rs:1395 — Without experiment.edges/trials coverage returns empty;786-815 metric benchmark falls back unknown/unknown.; /tmp/opi-evidence-zero-seal-qsubxxpp/report.stdout.json — Untouched public crash-after-intent root has1 intent/0 seals; run70, report0, planned2/unexecuted2, coverage=[],control\u005fpairs=[],benchmark unknown/unknown.",
        "current\u005ffinding\u005fid": "P19-AUD-004",
        "prior\u005fclaim": "The immediately superseded codex/gpt61 normalized finding sidecar contains no claim for this exact branch/predicate. This is an absence claim only over its eleven published records, not over all earlier implementations, reviews or tests.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "insufficient-information",
        "repair\u005fcommits": [],
        "verification": "Current independent reproduction and refutation remain unchanged. Suggested remediation check: In a fresh isolated root run opi-eval run --config tests/fixtures/experiment/local-paired.toml --root NEW --fixtures tests/fixtures --behavior crash-after-intent; expected exit70. On the same untouched root run opi-eval report --root NEW. Exact env-cleared argv/results are in C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\evidence-review.json; original raw report /tmp/opi-evidence-zero-seal-qsubxxpp/report.stdout.json."
      },
      {
        "cause": "A reproduced current defect or scoped completeness gap is established by the installed core. The single allowed previous report does not prove when it arose, whether it was previously working, or a repair transition; no regression/recurrence is inferred from related titles.",
        "current\u005fevidence": "At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: A fresh --out inside the canonical source's bundle/artifacts is accepted. Regrade then copies task files there before later lifecycle/base/image checks, adding unmanifested source files and invalidating subsequent verify/report, even when grading later rejects. Evidence: crates/opi-eval/src/regrade.rs:314 — Canonical source exists but only out.exists() is checked at328; mkdir at378/380 precedes output canonicalization387.; crates/opi-eval/src/regrade.rs:537 — Initial empty nested directories pass file-only source reverify; task copy at623 follows create\u005fdir\u005fall620-621, before later checks625-655.; crates/opi-eval/src/bundle/mod.rs:1409 — All new files inside retained artifact tree reject as UnmanifestedFile;1617-1641 ignores only empty directories.; crates/opi-eval/src/cli/report.rs:81 — Canonical-parent source containment exists for report output but is absent from regrade.",
        "current\u005ffinding\u005fid": "P19-AUD-005",
        "prior\u005fclaim": "The immediately superseded codex/gpt61 normalized finding sidecar contains no claim for this exact branch/predicate. This is an absence claim only over its eleven published records, not over all earlier implementations, reviews or tests.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "insufficient-information",
        "repair\u005fcommits": [],
        "verification": "Current independent reproduction and refutation remain unchanged. Suggested remediation check: Static ordered proof: with a legitimate sealed native source R, admitted cached material M and a trial T, invoke opi-eval regrade --root R --native-material M --out R/trials/T/bundle/artifacts/derived. Before native verifier effects, task copy adds source files; opi-eval verify --root R then rejects unmanifested files. Actual native reproduction was not run; the explicit source/material preconditions and earliest mkdir/copy/verification ordering are the invariant proof."
      },
      {
        "cause": "Current first-sink typed stdout projection and private-thinking tests retain the narrow repair. The current diagnostic stderr branch instead admits arbitrary token-bearing line content. Shared privacy invariant, different capture stream/grammar; no claim that known stdout thinking removal regressed.",
        "current\u005fevidence": "At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: Stderr is classified validated-diagnostic when each line merely contains one allowed token, then the entire original line is written. Arbitrary private/provider payload surrounding a token crosses the first capture sink under a sanitized receipt in both capture policies. Evidence: crates/opi-eval/src/native\u005fcapture\u005fruntime.rs:130 — Allowed static tokens are tested with line.contains(token) at153 rather than a validated whole diagnostic grammar.; crates/opi-eval/src/native\u005fcapture\u005fruntime.rs:511 — Entire stderr\u005fbytes written at517 whenever validated-diagnostic; receipt reports sanitized529.; C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\checks\\stderr-probe.json — Valid Pi stdout plus provider stream failed: PRIVATE\u005fPAYLOAD\u005fPROBE retained arbitrary suffix under both full and terminal-only; exit0/sanitized/validated-diagnostic.",
        "current\u005ffinding\u005fid": "P19-AUD-006",
        "prior\u005fclaim": "The superseded AUD-003 claimed native stdout/terminal private thinking was marked Exportable. Its specific raw files and declared thinking shapes are not this current stderr-token case.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-2465aeb-20261001t055252z",
            "finding\u005fid": "P19-AUD-003",
            "source\u005fid": "superseded-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "missed-branch",
        "repair\u005fcommits": [],
        "verification": "Current independent reproduction and refutation remain unchanged. Suggested remediation check: Run offline probe C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\privacy-probe.py using the exported binary's \u005f\u005fnative-capture and current valid Pi fixture; its synthetic stderr contains only an innocuous private-payload marker. Inspect resulting stderr sink and receipt for both policies; no real credential or private provider data is used."
      },
      {
        "cause": "A reproduced current defect or scoped completeness gap is established by the installed core. The single allowed previous report does not prove when it arose, whether it was previously working, or a repair transition; no regression/recurrence is inferred from related titles.",
        "current\u005fevidence": "At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: The local acceptance verifier rejects every nonzero/timed-out Agent terminal and any native exception before consuming benchmark-valid sealed classification, although eligible real task failures/timeouts may count among the six native trials. Evidence: crates/opi-eval/scripts/verify-local-acceptance.py:665 — Task exit must equal0, timed\u005fout=false and failure\u005fkind=None; exception\u005finfo must be absent672.; crates/opi-eval/src/runner/experiment.rs:1832 — Production settlement admits Agent task timeout and scored nonzero at1839-1847; provider/infra/preflight failures remain invalid.; C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\checks\\acceptance-probe.log — Current structural EvidenceRoot baseline accepted0; changing only task terminal to exit1 or timeout124 rejects2 with terminal/stream/model differs. This is not actual live acceptance proof.",
        "current\u005ffinding\u005fid": "P19-AUD-007",
        "prior\u005fclaim": "The immediately superseded codex/gpt61 normalized finding sidecar contains no claim for this exact branch/predicate. This is an absence claim only over its eleven published records, not over all earlier implementations, reviews or tests.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "insufficient-information",
        "repair\u005fcommits": [],
        "verification": "Current independent reproduction and refutation remain unchanged. Suggested remediation check: Run C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\acceptance-timeout-probe.py offline against current test\u005fverify\u005flocal\u005facceptance.EvidenceRoot. Baseline succeeds; native-task nonzero/timeout variants trigger the unconditional gate. Inspect scored task failure/timeout production counterexamples in native\u005fdriver.rs2108-2240; valid zero remains eligible and provider errors remain excluded."
      },
      {
        "cause": "Controlled grader/regrade proof remains unassessed within this audit's independent inputs. Current lack of supplied protocol/cleanup proof is an additional input-availability limitation, not a reproduced regression or refutation of old log execution. Those old credited logs were not independently supplied or revalidated here.",
        "current\u005fevidence": "At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: This independent run cannot assess the required real grader artifact/regrade witnesses, actual Opi/pi three-protocol tool-loop matrix or real Docker cleanup witness. Their production harnesses are ignored in the passing default suite and no applicable raw witness root was supplied. Evidence: crates/opi-eval/tests/native\u005fdriver.rs:3613 — TB2.1/TB3/DeepSWE positive/negative real witness filters are ignored and require pinned material plus Docker.; crates/opi-eval/tests/native\u005fdriver.rs:3998 — Actual-client protocol matrix is ignored; saved-stream conformance is not execution by actual clients.; crates/opi-eval/tests/native\u005fdriver.rs:9773 — Actual owned-resource cancellation and10978 verifier-only native regrade witnesses are ignored.; C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\checks\\rust.log — 390 pass/11 ignored; no claim that the ignored mandatory native predicates executed.",
        "current\u005ffinding\u005fid": "P19-AUD-008",
        "prior\u005fclaim": "The superseded AUD-006 found no executed controlled positive/negative pinned-grader or successful fresh native-regrade proof in its inspected scope. It separately credited raw actual-client/Docker logs as source-applicable to that older audited state.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-2465aeb-20261001t055252z",
            "finding\u005fid": "P19-AUD-006",
            "source\u005fid": "superseded-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "evidence-gap",
        "repair\u005fcommits": [],
        "verification": "Current independent reproduction and refutation remain unchanged. Suggested remediation check: Inspect cargo test -p opi-eval --all-targets --locked --offline executed and ignored counts. To assess the missing predicates, supply redacted current raw witness roots with pinned actual material/client/runner/build identities and production grading/tool/cleanup evidence, or separately authorize/run the explicit ignored local no-paid-provider filters with those prerequisites."
      },
      {
        "cause": "The old manifest is now an exact post-install candidate with matching bytes, but current applicability and contained evidence remain unverified. Different available inputs explain broader current not-assessable states. A file hash or historical success cannot establish final-state acceptance and cannot revise the installed core.",
        "current\u005fevidence": "At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: No accessible redacted raw acceptance root was supplied for this audit. Six real-provider native trials, pre-effect freeze, full raw trace/bundle/provenance closure and final Phase verification/artifact-audit predicates remain not assessable. Evidence: docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md:275 — AC012 requires six actual native-graded real trials, and sections6/8 distinguish valid native zero/task failure from invalid provider/infra/missing-loop evidence.; crates/opi-eval/scripts/verify-local-acceptance.py — Offline verifier expects a raw root with six trial records, sealed artifacts, command/build/material/client identities, three exhaustive Opi captures and repository-qualified upstream caches.; Independent input boundary of this audit run — No raw root path was supplied after the early request. Prior acceptance conclusions and ledger delivery assertions were excluded; no auth or live call was performed.",
        "current\u005ffinding\u005fid": "P19-AUD-009",
        "prior\u005fclaim": "The superseded AUD-005 credited genuine source-applicable six-trial execution while finding missing explicit operator usage/time freeze. AUD-006 also retained grader/regrade proof gaps. Those historical credits are attributed, not adopted as current conformance.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-2465aeb-20261001t055252z",
            "finding\u005fid": "P19-AUD-005",
            "source\u005fid": "superseded-findings"
          },
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-2465aeb-20261001t055252z",
            "finding\u005fid": "P19-AUD-006",
            "source\u005fid": "superseded-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Only after independent installation, verified the exact raw manifest named by superseded AUD-005 exists and SHA-256 equals its cited 9897f08f... digest. No manifest content, credentials or provider were accessed by this hash check.",
            "limitations": "Candidate discovered from allowed historical record after core freeze, not supplied independent-core proof. Its six contained records, current source/build/capture applicability, operator freeze and final gates were not validated. The two exact native files named by prior AUD-003 were absent at their cited paths during metadata-only existence checks.",
            "path": "C:\\Users\\Luiz\\.local\\state\\opi-remediation-evidence\\phase19-acceptance-20260929\\evidence-root\\manifest.json",
            "sha256": "9897f08fed7876b757542235e77e54735d233b1d6b0d82ec5024eab26fc6a9ce"
          }
        ],
        "relationship": "evidence-gap",
        "repair\u005fcommits": [],
        "verification": "Current independent reproduction and refutation remain unchanged. Suggested remediation check: Supply the redacted Phase19 operator manifest.json root and its contained raw files/cached repository identities, then run python crates/opi-eval/scripts/verify-local-acceptance.py --root E --out NEW with current verify/report and inspect all six bindings. Independently verify pre-effect frozen scope, three original Opi captures/exhaustive scoped audit view, pi/Harbor/Pier repository-qualified provenance, actual provider/tool/task/native evidence and final workspace/WSL/artifact-audit command receipts. A fresh independent context can be supplied this exact candidate root, verify containment/raw files and current applicability offline, and evaluate the missing freeze/witness/gate predicates."
      },
      {
        "cause": "A reproduced current defect or scoped completeness gap is established by the installed core. The single allowed previous report does not prove when it arose, whether it was previously working, or a repair transition; no regression/recurrence is inferred from related titles.",
        "current\u005fevidence": "At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: Doctor observes only the static lock and selected Agent paths, omitting selected corpus/profile, uv, runner project/lock/interpreter/entrypoint, integrations/provider and capture helper from missing-input diagnosis. Evidence: crates/opi-eval/src/runner/prepare.rs:213 — Complete doctor material list213-248 versus prepare required inputs1177-1221,1757-1765.; crates/opi-eval/tests/fail\u005fclosed\u005fcli.rs:227 — Empty-cache test asserts only the three current material IDs.",
        "current\u005ffinding\u005fid": "P19-AUD-010",
        "prior\u005fclaim": "The immediately superseded codex/gpt61 normalized finding sidecar contains no claim for this exact branch/predicate. This is an absence claim only over its eleven published records, not over all earlier implementations, reviews or tests.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "insufficient-information",
        "repair\u005fcommits": [],
        "verification": "Current independent reproduction and refutation remain unchanged. Suggested remediation check: Compare the full doctor material branch213-248 with prepare's selected required list1177-1221/1757-1765. In a partially seeded cache retain only lock and Agent files; no observation names the absent selected runner/corpus/helper/tools."
      },
      {
        "cause": "A reproduced current defect or scoped completeness gap is established by the installed core. The single allowed previous report does not prove when it arose, whether it was previously working, or a repair transition; no regression/recurrence is inferred from related titles.",
        "current\u005fevidence": "At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: Pi's native importer recognizes only the Completions truncation string as provider-protocol-error; the exact Anthropic and Responses native protocol strings acknowledged by the client matrix fall through to generic provider-failure. Evidence: crates/opi-eval/src/agent/pi.rs:339 — Exact match only Stream ended without finish\u005freason; all other known stream truncation messages generic.; crates/opi-eval/tests/native\u005fdriver.rs:4135 — Actual matrix recognizes Anthropic stream ended without a stop reason and Responses stream ended before a terminal response event.; C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\native-protocol-probe\\result.json — Public copied-fixture conformance gives completions0/met=true/protocol-error; Anthropic and Responses1/met=false/provider-failure.",
        "current\u005ffinding\u005fid": "P19-AUD-011",
        "prior\u005fclaim": "The immediately superseded codex/gpt61 normalized finding sidecar contains no claim for this exact branch/predicate. This is an absence claim only over its eleven published records, not over all earlier implementations, reviews or tests.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "insufficient-information",
        "repair\u005fcommits": [],
        "verification": "Current independent reproduction and refutation remain unchanged. Suggested remediation check: Copy the Pi provider-protocol fixture; change only API and native message to the exact Anthropic/Responses matrix strings. Run public conformance --suite agent --adapter pi --case provider-protocol-error with copied fixtures and declared scripted provider. Exact argv/raw results: C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-codex-gpt61-o95pfn2b\\native-protocol-probe\\result.json. No actual-client/live run is claimed."
      },
      {
        "cause": "Preserved narrow numeric refusal, not closure of all supplied bound representations.",
        "current\u005fevidence": "At current sealed head f60e458f9d1bceffcba5caaef11e8c0d7312ff59: Current experiment.rs execution parser rejects positive numeric max\u005foutput\u005fbytes with the unenforceable-bound error; experiment\u005fcontract numeric-bound tests passed in the 390-test run. Current AUD-001 still covers the independent quoted-string branch.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "The prior numeric captured-output cap was accepted without an enforcement route.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-2465aeb-20261001t055252z",
            "finding\u005fid": "P19-AUD-001",
            "source\u005fid": "superseded-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain numeric invalid-bound/no-effect tests and add current quoted-field cases."
      },
      {
        "cause": "The exact host permission defect is not reproduced in the current scoped evidence; no Windows ACL or global privacy guarantee is inferred.",
        "current\u005fevidence": "At current sealed head f60e458f9d1bceffcba5caaef11e8c0d7312ff59: Current native\u005fhost\u005fcredential\u005fstage\u005fis\u005fprivate\u005fon\u005fsearchable\u005froot test at native\u005fdriver.rs1727 executed and passed, observing exclusive private0700/0600 host staging before credential bytes; actual temp cleanup tests also passed. Current core AC010-credentials is met at this code/test seam.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "The prior host handoff/config inherited searchable output-root permissions under Unix umask022.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-2465aeb-20261001t055252z",
            "finding\u005fid": "P19-AUD-002",
            "source\u005fid": "superseded-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain searchable-root mode/alias/exclusive-creation and cleanup negative cases."
      },
      {
        "cause": "Preserved repair for declared stdout/private-thinking grammar; no claim that all stderr/private payloads are safe or that prior raw captures were retroactively rewritten.",
        "current\u005fevidence": "At current sealed head f60e458f9d1bceffcba5caaef11e8c0d7312ff59: Current typed first-sink stdout filtering and native\u005fcapture\u005fhelper\u005fsanitizes\u005fprivate\u005fthinking at native\u005fdriver.rs11724 executed and passed, under both policies with retained non-private fields. Current AUD-006 is a distinct unvalidated stderr branch.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "The prior declared native stdout/terminal thinking shapes were persisted as Exportable.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-2465aeb-20261001t055252z",
            "finding\u005fid": "P19-AUD-003",
            "source\u005fid": "superseded-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain stdout/terminal thinking and duplicate/unknown/incomplete refusal assertions; add current stderr case separately."
      },
      {
        "cause": "One admitted root now serves all consumers; current AUD-007 is a different eligibility gate. No historical shadow-root success is asserted.",
        "current\u005fevidence": "At current sealed head f60e458f9d1bceffcba5caaef11e8c0d7312ff59: Current verify-local-acceptance.py has admit\u005ffamily\u005froots at198-210 reused by native/bundle/Companion readers and check\u005fcompanion at975-983. Current deterministic root/unsafe ID/substitution acceptance regression tests executed in the272-test suite.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "The prior acceptance verifier used manifest-selected roots for Python reads but hardcoded different roots for Companion semantics; trial IDs lacked early containment.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-2465aeb-20261001t055252z",
            "finding\u005fid": "P19-AUD-004",
            "source\u005fid": "superseded-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain noncanonical same-family roots, conflicting-root and unsafe-trial ID tests, plus exact Companion argv assertions."
      },
      {
        "cause": "Preserved shared policy delegation; manual transport wrappers and immutable byte receipt validation remain justified residual glue, not separate scoring authority.",
        "current\u005fevidence": "At current sealed head f60e458f9d1bceffcba5caaef11e8c0d7312ff59: Current materialize-external-locks.sh603-775 delegates local prepare/conformance and emits the transport receipt only after success; current CI/receipt/acquisition Python tests pass. The current core AC011-ci is met at this static/test seam.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "The prior external-lock default path owned duplicate materialization/oracle policy instead of shared local CLI.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-2465aeb-20261001t055252z",
            "finding\u005fid": "P19-AUD-007",
            "source\u005fid": "superseded-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain shared prepare/conformance refusal-before-receipt and workflow transport contract tests."
      },
      {
        "cause": "The specific formatting gate is passing now; no final full workspace/native acceptance claim follows.",
        "current\u005fevidence": "At current sealed head f60e458f9d1bceffcba5caaef11e8c0d7312ff59: Current sealed-export cargo fmt --check --all returned0 with no diff; raw current log checks/fmt.log is retained.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "The prior required rustfmt check failed on committed native\u005fdriver assertion wrapping.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-2465aeb-20261001t055252z",
            "finding\u005fid": "P19-AUD-008",
            "source\u005fid": "superseded-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain cargo fmt --check --all at required scope."
      },
      {
        "cause": "Fixture-owned executable removes the fixed machine-path dependency; passing current tests establish this narrow portability repair.",
        "current\u005fevidence": "At current sealed head f60e458f9d1bceffcba5caaef11e8c0d7312ff59: Current test\u005fnative\u005fsmoke\u005foutput\u005fcontrol.py66-69 creates a synthetic uv inside its own tools directory. Current WSL Python suite ran272 tests without errors, with one unrelated suite skip; both ordering/refusal fixture methods remain in the suite.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "The prior two output-control fixture tests depended on missing /usr/bin/uv.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-2465aeb-20261001t055252z",
            "finding\u005fid": "P19-AUD-009",
            "source\u005fid": "superseded-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain both output-control proof-before-freeze and refusal ordering fixture cases on clean local WSL/Linux PATH."
      },
      {
        "cause": "Specific automatic-oracle/fixture-only descriptions are corrected; this does not excuse current capture/security behavior defects.",
        "current\u005fevidence": "At current sealed head f60e458f9d1bceffcba5caaef11e8c0d7312ff59: Current runner/experiment.rs1-18 explicitly describes pinned real native path and oracle as selected diagnostic; cli/run.rs1-15 distinguishes hermetic and native material. Warning-denied scoped rustdoc and doc checker pass.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "The prior runner/run rustdoc described automatic oracle preflight and denied selected real native execution.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-2465aeb-20261001t055252z",
            "finding\u005fid": "P19-AUD-010",
            "source\u005fid": "superseded-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain current contract descriptions and warning-denied doc checks; inspect semantic wording with behavior consumers when changing native dispatch."
      },
      {
        "cause": "Projection and admitted native capability now agree for reasoning; no live effectiveness is inferred.",
        "current\u005fevidence": "At current sealed head f60e458f9d1bceffcba5caaef11e8c0d7312ff59: Current experiment.rs1202-1220 retains supplied reasoning as requested-but-Unsupported, required native admission rejects, and current projection/native control tests pass. AC006-controls is met at the current owner.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "The prior optional reasoning projection called unproven values Effective while native settlement/launch said Unsupported.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-2465aeb-20261001t055252z",
            "finding\u005fid": "P19-AUD-011",
            "source\u005fid": "superseded-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain all-protocol/two-client optional Unsupported and required pre-effect rejection tests."
      }
    ],
    "limitations": "The independent core is frozen byte-for-byte and cannot be revised after this historical exposure. No repair commit chain or repaired-then-failing transition was established, so no regression label is used. Old credits/closure claims are attributed only. The exact old manifest named by AUD-005 exists with matching hash, but contained/current-applicable six-trial proof was not inspected or replayed; two exact old native captures named by AUD-003 are absent at their cited paths. The current raw-root/native witness evidence gaps remain open. Historical raw candidates require a fresh independent context or authorized remediation applicability checks; no auth/provider/paid call or extra native execution occurred. Current checks referenced here are scoped export checks, not Phase-exit workspace proof.",
    "schema": "opi-audit-history/1",
    "scope": "Only the immediately superseded codex/gpt61 run phase19-codex-gpt61-2465aeb-20261001t055252z moved by this installation is compared. Its normalized sidecar/metadata and attributed core claims were inspected. Moved bytes equal their active paths at commit31b3ae11095a41f1cbec46c6886b8513de2a292c, verified by git show and SHA-256. Older runs, other pairs, sibling output, prior appendices and unrelated remediation conclusions were excluded. Published-claim absence is bounded to the eleven records of this sole prior sidecar.",
    "sources": [
      {
        "audit\u005frun\u005fid": "phase19-codex-gpt61-2465aeb-20261001t055252z",
        "commit": "31b3ae11095a41f1cbec46c6886b8513de2a292c",
        "id": "superseded-findings",
        "path": "docs/snapshots/phase19/assurance/history/phase19-codex-gpt61-2465aeb-20261001t055252z/audit.codex.gpt61.findings.jsonl",
        "sha256": "90d9a494d59824cb99dfcf749cf6b0c30e7fa6eb79959b7e406fa2eeea64b5a1"
      },
      {
        "audit\u005frun\u005fid": "phase19-codex-gpt61-2465aeb-20261001t055252z",
        "commit": "31b3ae11095a41f1cbec46c6886b8513de2a292c",
        "id": "superseded-meta",
        "path": "docs/snapshots/phase19/assurance/history/phase19-codex-gpt61-2465aeb-20261001t055252z/audit.codex.gpt61.meta.json",
        "sha256": "d67dc8f6ade4b8516624518e905e8b62d0788854eb1350a7b3373e46e8c3881d"
      }
    ],
    "status": "compared"
  },
  "core": {
    "findings\u005fsha256": "37f779a15dad2250e807239cc4b4718256a117402fdba496fc3da8452e47c28a",
    "meta\u005fsha256": "b0bd6f235db104bf1ca167ed02da994ebcf6ba605ca1838f7476417a2f994707",
    "report\u005fbytes": 74895,
    "report\u005fsha256": "c28f67fe6fbb4cd2544628a5ecb0f871142a7318d02e394d28789d50ca777757",
    "requirements\u005fsha256": "00ef0398b3b9b759369dd8ced604672793a8c9b1435af66ba223800f37be0083"
  },
  "schema": "opi-audit-history/1"
}
-->

## Post-audit historical reference — advisory

This section was appended after installation of the independent core. It cannot change findings, requirements, independence or verdict. Historical closure claims require current independent verification.

- Owning audit run: phase19&#45;codex&#45;gpt61&#45;f60e458&#45;20261003t041015z
- Audit head: f60e458f9d1bceffcba5caaef11e8c0d7312ff59
- Core report bytes: 74895
- Core report SHA-256: c28f67fe6fbb4cd2544628a5ecb0f871142a7318d02e394d28789d50ca777757
- Current findings SHA-256: 37f779a15dad2250e807239cc4b4718256a117402fdba496fc3da8452e47c28a
- Comparison status: compared
- Inspected history: Only the immediately superseded codex/gpt61 run phase19&#45;codex&#45;gpt61&#45;2465aeb&#45;20261001t055252z moved by this installation is compared&#46; Its normalized sidecar/metadata and attributed core claims were inspected&#46; Moved bytes equal their active paths at commit31b3ae11095a41f1cbec46c6886b8513de2a292c, verified by git show and SHA&#45;256&#46; Older runs, other pairs, sibling output, prior appendices and unrelated remediation conclusions were excluded&#46; Published&#45;claim absence is bounded to the eleven records of this sole prior sidecar&#46;
- Limitations: The independent core is frozen byte&#45;for&#45;byte and cannot be revised after this historical exposure&#46; No repair commit chain or repaired&#45;then&#45;failing transition was established, so no regression label is used&#46; Old credits/closure claims are attributed only&#46; The exact old manifest named by AUD&#45;005 exists with matching hash, but contained/current&#45;applicable six&#45;trial proof was not inspected or replayed; two exact old native captures named by AUD&#45;003 are absent at their cited paths&#46; The current raw&#45;root/native witness evidence gaps remain open&#46; Historical raw candidates require a fresh independent context or authorized remediation applicability checks; no auth/provider/paid call or extra native execution occurred&#46; Current checks referenced here are scoped export checks, not Phase&#45;exit workspace proof&#46;

### Pinned historical sources

- superseded&#45;findings: docs/snapshots/phase19/assurance/history/phase19&#45;codex&#45;gpt61&#45;2465aeb&#45;20261001t055252z/audit&#46;codex&#46;gpt61&#46;findings&#46;jsonl; commit 31b3ae11095a41f1cbec46c6886b8513de2a292c; SHA-256 90d9a494d59824cb99dfcf749cf6b0c30e7fa6eb79959b7e406fa2eeea64b5a1; audit run phase19&#45;codex&#45;gpt61&#45;2465aeb&#45;20261001t055252z
- superseded&#45;meta: docs/snapshots/phase19/assurance/history/phase19&#45;codex&#45;gpt61&#45;2465aeb&#45;20261001t055252z/audit&#46;codex&#46;gpt61&#46;meta&#46;json; commit 31b3ae11095a41f1cbec46c6886b8513de2a292c; SHA-256 d67dc8f6ade4b8516624518e905e8b62d0788854eb1350a7b3373e46e8c3881d; audit run phase19&#45;codex&#45;gpt61&#45;2465aeb&#45;20261001t055252z

### Comparison 1: P19&#45;AUD&#45;001

- Relationship: missed&#45;branch
- Prior claim (attributed): The superseded AUD&#45;001 claimed a positive numeric captured&#45;output bound was accepted without an enforcement route&#46; It did not address nonempty TOML strings at integer execution fields&#46;
- Current evidence: At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: Nonempty TOML strings at integer execution&#45;limit fields validate as absence&#46; A requested usage/output/time/trial bound can therefore be erased instead of rejected or enforced before effects&#46; Evidence: crates/opi&#45;eval/src/experiment&#46;rs:1423 — optional&#95;u64 maps None and every Some&#40;String&#40;&#95;&#41;&#41; to None; callers690&#45;723 own seven integer execution limits&#46;; C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;prepare&#45;probes&#92;results&#46;json — Seven quoted1 inputs exit0 with identical resolved digest8d85d2ffac59bbb30041b5b9dba7d74410821c44d43fd45180cef564cdd95a83; quoted max&#95;total&#95;tokens avoids explicit numeric unenforceable&#45;cap rejection&#46;
- Cause / missed boundary: The current numeric max&#95;output&#95;bytes path rejects the unenforceable request and its tests pass; string parsing still maps every String to omission before that guard&#46; This is a different parser branch violating the same bound&#45;admission invariant&#46; No repaired&#45;then&#45;regressed transition is inferred&#46;
- Suggested current verification: Current independent reproduction and refutation remain unchanged&#46; Suggested remediation check: Copy local&#45;paired&#46;toml to an isolated document directory with valid fixture paths; add &#91;execution&#93; mode=&#34;hermetic&#34; and each of max&#95;total&#95;tokens,time&#95;limit&#95;seconds,max&#95;output&#95;bytes,max&#95;trials,agent&#95;timeout&#95;seconds,verifier&#95;timeout&#95;seconds,max&#95;output&#95;tokens=&#34;1&#34; in turn; run opi&#45;eval validate &#45;&#45;config C &#45;&#45;fixtures F&#46; Exact seven input files, argv and outputs: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;prepare&#45;probes&#92;results&#46;json; executable probe: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;prepare&#45;probes&#46;py&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;2465aeb&#45;20261001t055252z / P19&#45;AUD&#45;001; source superseded&#45;findings

### Comparison 2: P19&#45;AUD&#45;002

- Relationship: insufficient&#45;information
- Prior claim (attributed): The immediately superseded codex/gpt61 normalized finding sidecar contains no claim for this exact branch/predicate&#46; This is an absence claim only over its eleven published records, not over all earlier implementations, reviews or tests&#46;
- Current evidence: At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: The max&#95;trials admission check applies only to selection expansion&#46; A public explicit&#45;trial config with max&#95;trials=1 executes and seals both declared trials&#46; Evidence: crates/opi&#45;eval/src/runner/plan&#46;rs:445 — The only cap check follows selection expansion&#46;; crates/opi&#45;eval/src/runner/experiment&#46;rs:538 — None selection branch accepts the explicit experiment without resolved&#45;count enforcement&#46;; C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;prepare&#45;probes&#92;results&#46;json — Hermetic paired fixture with max&#95;trials1 exits0/completed, saves two sealed trials, and reports execution:2 trials,width1&#46;
- Cause / missed boundary: A reproduced current defect or scoped completeness gap is established by the installed core&#46; The single allowed previous report does not prove when it arose, whether it was previously working, or a repair transition; no regression/recurrence is inferred from related titles&#46;
- Suggested current verification: Current independent reproduction and refutation remain unchanged&#46; Suggested remediation check: Run a fresh copy of tests/fixtures/experiment/local&#45;paired&#46;toml with &#91;execution&#93; mode=&#34;hermetic&#34;, max&#95;trials=1 using opi&#45;eval run &#45;&#45;config C &#45;&#45;root NEW &#45;&#45;fixtures F &#45;&#45;behavior happy&#46; Exact config/output retained under C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;prepare&#45;probes; probe C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;prepare&#45;probes&#46;py&#46;

### Comparison 3: P19&#45;AUD&#45;003

- Relationship: insufficient&#45;information
- Prior claim (attributed): The immediately superseded codex/gpt61 normalized finding sidecar contains no claim for this exact branch/predicate&#46; This is an absence claim only over its eleven published records, not over all earlier implementations, reviews or tests&#46;
- Current evidence: At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: Raw synchronous engine/version/df/context/GPU probes have no deadline, capture cap or cancellation&#46; A stalled selected observer blocks the public operation and on the current&#45;thread runtime prevents preparation cancellation from being polled&#46; Evidence: crates/opi&#45;eval/src/runner/prepare&#46;rs:281 — Docker version uses blocking Command::output; equivalent raw calls at333&#45;337,1369&#45;1372,1463&#45;1471,2637&#45;2640 and298&#45;306&#46;; C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;prepare&#45;probes&#92;results&#46;json — Isolated fake docker exec sleep60 left doctor blocked after15&#46;026s with no output; harness killed only its own process group&#46;; crates/opi&#45;eval/src/runner/prepare&#46;rs:2299 — Existing observation&#95;command already provides16KiB, explicit deadline and verified cleanup, but raw callers bypass it&#46;
- Cause / missed boundary: A reproduced current defect or scoped completeness gap is established by the installed core&#46; The single allowed previous report does not prove when it arose, whether it was previously working, or a repair transition; no regression/recurrence is inferred from related titles&#46;
- Suggested current verification: Current independent reproduction and refutation remain unchanged&#46; Suggested remediation check: In an isolated PATH place an executable docker that records its own PID and execs sleep60; invoke public doctor &#45;&#45;config C in a fresh process group, with a15&#45;second outer watchdog; capture output and terminate only the probe group&#46; Exact safe harness C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;prepare&#45;probes&#46;py and results C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;prepare&#45;probes&#92;results&#46;json; static raw calls establish absent supervisory bound rather than inferring infinity from15 seconds&#46;

### Comparison 4: P19&#45;AUD&#45;004

- Relationship: insufficient&#45;information
- Prior claim (attributed): The immediately superseded codex/gpt61 normalized finding sidecar contains no claim for this exact branch/predicate&#46; This is an absence claim only over its eleven published records, not over all earlier implementations, reviews or tests&#46;
- Current evidence: At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: A genuine interruption before any seal publishes counts but drops all declared pair coverage and renders known native benchmark identity as unknown/unknown&#46; Missing pairs cannot be reconstructed from the incomplete pre&#45;effect run&#45;control&#46; Evidence: crates/opi&#45;eval/src/runner/experiment&#46;rs:703 — Durable planned rows omit benchmark/subjects/edges/integrity exclusions and frozen report controls&#46;; crates/opi&#45;eval/src/report&#46;rs:340 — Authoritative experiment restored only from sealed trials; diagnostic unsealed contract does not populate it&#46;; crates/opi&#45;eval/src/report&#46;rs:1395 — Without experiment&#46;edges/trials coverage returns empty;786&#45;815 metric benchmark falls back unknown/unknown&#46;; /tmp/opi&#45;evidence&#45;zero&#45;seal&#45;qsubxxpp/report&#46;stdout&#46;json — Untouched public crash&#45;after&#45;intent root has1 intent/0 seals; run70, report0, planned2/unexecuted2, coverage=&#91;&#93;,control&#95;pairs=&#91;&#93;,benchmark unknown/unknown&#46;
- Cause / missed boundary: A reproduced current defect or scoped completeness gap is established by the installed core&#46; The single allowed previous report does not prove when it arose, whether it was previously working, or a repair transition; no regression/recurrence is inferred from related titles&#46;
- Suggested current verification: Current independent reproduction and refutation remain unchanged&#46; Suggested remediation check: In a fresh isolated root run opi&#45;eval run &#45;&#45;config tests/fixtures/experiment/local&#45;paired&#46;toml &#45;&#45;root NEW &#45;&#45;fixtures tests/fixtures &#45;&#45;behavior crash&#45;after&#45;intent; expected exit70&#46; On the same untouched root run opi&#45;eval report &#45;&#45;root NEW&#46; Exact env&#45;cleared argv/results are in C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;evidence&#45;review&#46;json; original raw report /tmp/opi&#45;evidence&#45;zero&#45;seal&#45;qsubxxpp/report&#46;stdout&#46;json&#46;

### Comparison 5: P19&#45;AUD&#45;005

- Relationship: insufficient&#45;information
- Prior claim (attributed): The immediately superseded codex/gpt61 normalized finding sidecar contains no claim for this exact branch/predicate&#46; This is an absence claim only over its eleven published records, not over all earlier implementations, reviews or tests&#46;
- Current evidence: At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: A fresh &#45;&#45;out inside the canonical source&#39;s bundle/artifacts is accepted&#46; Regrade then copies task files there before later lifecycle/base/image checks, adding unmanifested source files and invalidating subsequent verify/report, even when grading later rejects&#46; Evidence: crates/opi&#45;eval/src/regrade&#46;rs:314 — Canonical source exists but only out&#46;exists&#40;&#41; is checked at328; mkdir at378/380 precedes output canonicalization387&#46;; crates/opi&#45;eval/src/regrade&#46;rs:537 — Initial empty nested directories pass file&#45;only source reverify; task copy at623 follows create&#95;dir&#95;all620&#45;621, before later checks625&#45;655&#46;; crates/opi&#45;eval/src/bundle/mod&#46;rs:1409 — All new files inside retained artifact tree reject as UnmanifestedFile;1617&#45;1641 ignores only empty directories&#46;; crates/opi&#45;eval/src/cli/report&#46;rs:81 — Canonical&#45;parent source containment exists for report output but is absent from regrade&#46;
- Cause / missed boundary: A reproduced current defect or scoped completeness gap is established by the installed core&#46; The single allowed previous report does not prove when it arose, whether it was previously working, or a repair transition; no regression/recurrence is inferred from related titles&#46;
- Suggested current verification: Current independent reproduction and refutation remain unchanged&#46; Suggested remediation check: Static ordered proof: with a legitimate sealed native source R, admitted cached material M and a trial T, invoke opi&#45;eval regrade &#45;&#45;root R &#45;&#45;native&#45;material M &#45;&#45;out R/trials/T/bundle/artifacts/derived&#46; Before native verifier effects, task copy adds source files; opi&#45;eval verify &#45;&#45;root R then rejects unmanifested files&#46; Actual native reproduction was not run; the explicit source/material preconditions and earliest mkdir/copy/verification ordering are the invariant proof&#46;

### Comparison 6: P19&#45;AUD&#45;006

- Relationship: missed&#45;branch
- Prior claim (attributed): The superseded AUD&#45;003 claimed native stdout/terminal private thinking was marked Exportable&#46; Its specific raw files and declared thinking shapes are not this current stderr&#45;token case&#46;
- Current evidence: At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: Stderr is classified validated&#45;diagnostic when each line merely contains one allowed token, then the entire original line is written&#46; Arbitrary private/provider payload surrounding a token crosses the first capture sink under a sanitized receipt in both capture policies&#46; Evidence: crates/opi&#45;eval/src/native&#95;capture&#95;runtime&#46;rs:130 — Allowed static tokens are tested with line&#46;contains&#40;token&#41; at153 rather than a validated whole diagnostic grammar&#46;; crates/opi&#45;eval/src/native&#95;capture&#95;runtime&#46;rs:511 — Entire stderr&#95;bytes written at517 whenever validated&#45;diagnostic; receipt reports sanitized529&#46;; C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;checks&#92;stderr&#45;probe&#46;json — Valid Pi stdout plus provider stream failed: PRIVATE&#95;PAYLOAD&#95;PROBE retained arbitrary suffix under both full and terminal&#45;only; exit0/sanitized/validated&#45;diagnostic&#46;
- Cause / missed boundary: Current first&#45;sink typed stdout projection and private&#45;thinking tests retain the narrow repair&#46; The current diagnostic stderr branch instead admits arbitrary token&#45;bearing line content&#46; Shared privacy invariant, different capture stream/grammar; no claim that known stdout thinking removal regressed&#46;
- Suggested current verification: Current independent reproduction and refutation remain unchanged&#46; Suggested remediation check: Run offline probe C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;privacy&#45;probe&#46;py using the exported binary&#39;s &#95;&#95;native&#45;capture and current valid Pi fixture; its synthetic stderr contains only an innocuous private&#45;payload marker&#46; Inspect resulting stderr sink and receipt for both policies; no real credential or private provider data is used&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;2465aeb&#45;20261001t055252z / P19&#45;AUD&#45;003; source superseded&#45;findings

### Comparison 7: P19&#45;AUD&#45;007

- Relationship: insufficient&#45;information
- Prior claim (attributed): The immediately superseded codex/gpt61 normalized finding sidecar contains no claim for this exact branch/predicate&#46; This is an absence claim only over its eleven published records, not over all earlier implementations, reviews or tests&#46;
- Current evidence: At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: The local acceptance verifier rejects every nonzero/timed&#45;out Agent terminal and any native exception before consuming benchmark&#45;valid sealed classification, although eligible real task failures/timeouts may count among the six native trials&#46; Evidence: crates/opi&#45;eval/scripts/verify&#45;local&#45;acceptance&#46;py:665 — Task exit must equal0, timed&#95;out=false and failure&#95;kind=None; exception&#95;info must be absent672&#46;; crates/opi&#45;eval/src/runner/experiment&#46;rs:1832 — Production settlement admits Agent task timeout and scored nonzero at1839&#45;1847; provider/infra/preflight failures remain invalid&#46;; C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;checks&#92;acceptance&#45;probe&#46;log — Current structural EvidenceRoot baseline accepted0; changing only task terminal to exit1 or timeout124 rejects2 with terminal/stream/model differs&#46; This is not actual live acceptance proof&#46;
- Cause / missed boundary: A reproduced current defect or scoped completeness gap is established by the installed core&#46; The single allowed previous report does not prove when it arose, whether it was previously working, or a repair transition; no regression/recurrence is inferred from related titles&#46;
- Suggested current verification: Current independent reproduction and refutation remain unchanged&#46; Suggested remediation check: Run C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;acceptance&#45;timeout&#45;probe&#46;py offline against current test&#95;verify&#95;local&#95;acceptance&#46;EvidenceRoot&#46; Baseline succeeds; native&#45;task nonzero/timeout variants trigger the unconditional gate&#46; Inspect scored task failure/timeout production counterexamples in native&#95;driver&#46;rs2108&#45;2240; valid zero remains eligible and provider errors remain excluded&#46;

### Comparison 8: P19&#45;AUD&#45;008

- Relationship: evidence&#45;gap
- Prior claim (attributed): The superseded AUD&#45;006 found no executed controlled positive/negative pinned&#45;grader or successful fresh native&#45;regrade proof in its inspected scope&#46; It separately credited raw actual&#45;client/Docker logs as source&#45;applicable to that older audited state&#46;
- Current evidence: At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: This independent run cannot assess the required real grader artifact/regrade witnesses, actual Opi/pi three&#45;protocol tool&#45;loop matrix or real Docker cleanup witness&#46; Their production harnesses are ignored in the passing default suite and no applicable raw witness root was supplied&#46; Evidence: crates/opi&#45;eval/tests/native&#95;driver&#46;rs:3613 — TB2&#46;1/TB3/DeepSWE positive/negative real witness filters are ignored and require pinned material plus Docker&#46;; crates/opi&#45;eval/tests/native&#95;driver&#46;rs:3998 — Actual&#45;client protocol matrix is ignored; saved&#45;stream conformance is not execution by actual clients&#46;; crates/opi&#45;eval/tests/native&#95;driver&#46;rs:9773 — Actual owned&#45;resource cancellation and10978 verifier&#45;only native regrade witnesses are ignored&#46;; C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;checks&#92;rust&#46;log — 390 pass/11 ignored; no claim that the ignored mandatory native predicates executed&#46;
- Cause / missed boundary: Controlled grader/regrade proof remains unassessed within this audit&#39;s independent inputs&#46; Current lack of supplied protocol/cleanup proof is an additional input&#45;availability limitation, not a reproduced regression or refutation of old log execution&#46; Those old credited logs were not independently supplied or revalidated here&#46;
- Suggested current verification: Current independent reproduction and refutation remain unchanged&#46; Suggested remediation check: Inspect cargo test &#45;p opi&#45;eval &#45;&#45;all&#45;targets &#45;&#45;locked &#45;&#45;offline executed and ignored counts&#46; To assess the missing predicates, supply redacted current raw witness roots with pinned actual material/client/runner/build identities and production grading/tool/cleanup evidence, or separately authorize/run the explicit ignored local no&#45;paid&#45;provider filters with those prerequisites&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;2465aeb&#45;20261001t055252z / P19&#45;AUD&#45;006; source superseded&#45;findings

### Comparison 9: P19&#45;AUD&#45;009

- Relationship: evidence&#45;gap
- Prior claim (attributed): The superseded AUD&#45;005 credited genuine source&#45;applicable six&#45;trial execution while finding missing explicit operator usage/time freeze&#46; AUD&#45;006 also retained grader/regrade proof gaps&#46; Those historical credits are attributed, not adopted as current conformance&#46;
- Current evidence: At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: No accessible redacted raw acceptance root was supplied for this audit&#46; Six real&#45;provider native trials, pre&#45;effect freeze, full raw trace/bundle/provenance closure and final Phase verification/artifact&#45;audit predicates remain not assessable&#46; Evidence: docs/superpowers/specs/2026&#45;09&#45;05&#45;phase19&#45;local&#45;native&#45;agent&#45;evaluation&#45;design&#46;md:275 — AC012 requires six actual native&#45;graded real trials, and sections6/8 distinguish valid native zero/task failure from invalid provider/infra/missing&#45;loop evidence&#46;; crates/opi&#45;eval/scripts/verify&#45;local&#45;acceptance&#46;py — Offline verifier expects a raw root with six trial records, sealed artifacts, command/build/material/client identities, three exhaustive Opi captures and repository&#45;qualified upstream caches&#46;; Independent input boundary of this audit run — No raw root path was supplied after the early request&#46; Prior acceptance conclusions and ledger delivery assertions were excluded; no auth or live call was performed&#46;
- Cause / missed boundary: The old manifest is now an exact post&#45;install candidate with matching bytes, but current applicability and contained evidence remain unverified&#46; Different available inputs explain broader current not&#45;assessable states&#46; A file hash or historical success cannot establish final&#45;state acceptance and cannot revise the installed core&#46;
- Suggested current verification: Current independent reproduction and refutation remain unchanged&#46; Suggested remediation check: Supply the redacted Phase19 operator manifest&#46;json root and its contained raw files/cached repository identities, then run python crates/opi&#45;eval/scripts/verify&#45;local&#45;acceptance&#46;py &#45;&#45;root E &#45;&#45;out NEW with current verify/report and inspect all six bindings&#46; Independently verify pre&#45;effect frozen scope, three original Opi captures/exhaustive scoped audit view, pi/Harbor/Pier repository&#45;qualified provenance, actual provider/tool/task/native evidence and final workspace/WSL/artifact&#45;audit command receipts&#46; A fresh independent context can be supplied this exact candidate root, verify containment/raw files and current applicability offline, and evaluate the missing freeze/witness/gate predicates&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;2465aeb&#45;20261001t055252z / P19&#45;AUD&#45;005; source superseded&#45;findings
- Prior finding: phase19&#45;codex&#45;gpt61&#45;2465aeb&#45;20261001t055252z / P19&#45;AUD&#45;006; source superseded&#45;findings
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;&#46;local&#92;state&#92;opi&#45;remediation&#45;evidence&#92;phase19&#45;acceptance&#45;20260929&#92;evidence&#45;root&#92;manifest&#46;json; SHA-256 9897f08fed7876b757542235e77e54735d233b1d6b0d82ec5024eab26fc6a9ce; checked: Only after independent installation, verified the exact raw manifest named by superseded AUD&#45;005 exists and SHA&#45;256 equals its cited 9897f08f&#46;&#46;&#46; digest&#46; No manifest content, credentials or provider were accessed by this hash check&#46;; limits: Candidate discovered from allowed historical record after core freeze, not supplied independent&#45;core proof&#46; Its six contained records, current source/build/capture applicability, operator freeze and final gates were not validated&#46; The two exact native files named by prior AUD&#45;003 were absent at their cited paths during metadata&#45;only existence checks&#46;

### Comparison 10: P19&#45;AUD&#45;010

- Relationship: insufficient&#45;information
- Prior claim (attributed): The immediately superseded codex/gpt61 normalized finding sidecar contains no claim for this exact branch/predicate&#46; This is an absence claim only over its eleven published records, not over all earlier implementations, reviews or tests&#46;
- Current evidence: At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: Doctor observes only the static lock and selected Agent paths, omitting selected corpus/profile, uv, runner project/lock/interpreter/entrypoint, integrations/provider and capture helper from missing&#45;input diagnosis&#46; Evidence: crates/opi&#45;eval/src/runner/prepare&#46;rs:213 — Complete doctor material list213&#45;248 versus prepare required inputs1177&#45;1221,1757&#45;1765&#46;; crates/opi&#45;eval/tests/fail&#95;closed&#95;cli&#46;rs:227 — Empty&#45;cache test asserts only the three current material IDs&#46;
- Cause / missed boundary: A reproduced current defect or scoped completeness gap is established by the installed core&#46; The single allowed previous report does not prove when it arose, whether it was previously working, or a repair transition; no regression/recurrence is inferred from related titles&#46;
- Suggested current verification: Current independent reproduction and refutation remain unchanged&#46; Suggested remediation check: Compare the full doctor material branch213&#45;248 with prepare&#39;s selected required list1177&#45;1221/1757&#45;1765&#46; In a partially seeded cache retain only lock and Agent files; no observation names the absent selected runner/corpus/helper/tools&#46;

### Comparison 11: P19&#45;AUD&#45;011

- Relationship: insufficient&#45;information
- Prior claim (attributed): The immediately superseded codex/gpt61 normalized finding sidecar contains no claim for this exact branch/predicate&#46; This is an absence claim only over its eleven published records, not over all earlier implementations, reviews or tests&#46;
- Current evidence: At f60e458f9d1bceffcba5caaef11e8c0d7312ff59, the installed independent finding states: Pi&#39;s native importer recognizes only the Completions truncation string as provider&#45;protocol&#45;error; the exact Anthropic and Responses native protocol strings acknowledged by the client matrix fall through to generic provider&#45;failure&#46; Evidence: crates/opi&#45;eval/src/agent/pi&#46;rs:339 — Exact match only Stream ended without finish&#95;reason; all other known stream truncation messages generic&#46;; crates/opi&#45;eval/tests/native&#95;driver&#46;rs:4135 — Actual matrix recognizes Anthropic stream ended without a stop reason and Responses stream ended before a terminal response event&#46;; C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;native&#45;protocol&#45;probe&#92;result&#46;json — Public copied&#45;fixture conformance gives completions0/met=true/protocol&#45;error; Anthropic and Responses1/met=false/provider&#45;failure&#46;
- Cause / missed boundary: A reproduced current defect or scoped completeness gap is established by the installed core&#46; The single allowed previous report does not prove when it arose, whether it was previously working, or a repair transition; no regression/recurrence is inferred from related titles&#46;
- Suggested current verification: Current independent reproduction and refutation remain unchanged&#46; Suggested remediation check: Copy the Pi provider&#45;protocol fixture; change only API and native message to the exact Anthropic/Responses matrix strings&#46; Run public conformance &#45;&#45;suite agent &#45;&#45;adapter pi &#45;&#45;case provider&#45;protocol&#45;error with copied fixtures and declared scripted provider&#46; Exact argv/raw results: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;codex&#45;gpt61&#45;o95pfn2b&#92;native&#45;protocol&#45;probe&#92;result&#46;json&#46; No actual&#45;client/live run is claimed&#46;

### Comparison 12: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): The prior numeric captured&#45;output cap was accepted without an enforcement route&#46;
- Current evidence: At current sealed head f60e458f9d1bceffcba5caaef11e8c0d7312ff59: Current experiment&#46;rs execution parser rejects positive numeric max&#95;output&#95;bytes with the unenforceable&#45;bound error; experiment&#95;contract numeric&#45;bound tests passed in the 390&#45;test run&#46; Current AUD&#45;001 still covers the independent quoted&#45;string branch&#46;
- Cause / missed boundary: Preserved narrow numeric refusal, not closure of all supplied bound representations&#46;
- Suggested current verification: Retain numeric invalid&#45;bound/no&#45;effect tests and add current quoted&#45;field cases&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;2465aeb&#45;20261001t055252z / P19&#45;AUD&#45;001; source superseded&#45;findings

### Comparison 13: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): The prior host handoff/config inherited searchable output&#45;root permissions under Unix umask022&#46;
- Current evidence: At current sealed head f60e458f9d1bceffcba5caaef11e8c0d7312ff59: Current native&#95;host&#95;credential&#95;stage&#95;is&#95;private&#95;on&#95;searchable&#95;root test at native&#95;driver&#46;rs1727 executed and passed, observing exclusive private0700/0600 host staging before credential bytes; actual temp cleanup tests also passed&#46; Current core AC010&#45;credentials is met at this code/test seam&#46;
- Cause / missed boundary: The exact host permission defect is not reproduced in the current scoped evidence; no Windows ACL or global privacy guarantee is inferred&#46;
- Suggested current verification: Retain searchable&#45;root mode/alias/exclusive&#45;creation and cleanup negative cases&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;2465aeb&#45;20261001t055252z / P19&#45;AUD&#45;002; source superseded&#45;findings

### Comparison 14: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): The prior declared native stdout/terminal thinking shapes were persisted as Exportable&#46;
- Current evidence: At current sealed head f60e458f9d1bceffcba5caaef11e8c0d7312ff59: Current typed first&#45;sink stdout filtering and native&#95;capture&#95;helper&#95;sanitizes&#95;private&#95;thinking at native&#95;driver&#46;rs11724 executed and passed, under both policies with retained non&#45;private fields&#46; Current AUD&#45;006 is a distinct unvalidated stderr branch&#46;
- Cause / missed boundary: Preserved repair for declared stdout/private&#45;thinking grammar; no claim that all stderr/private payloads are safe or that prior raw captures were retroactively rewritten&#46;
- Suggested current verification: Retain stdout/terminal thinking and duplicate/unknown/incomplete refusal assertions; add current stderr case separately&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;2465aeb&#45;20261001t055252z / P19&#45;AUD&#45;003; source superseded&#45;findings

### Comparison 15: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): The prior acceptance verifier used manifest&#45;selected roots for Python reads but hardcoded different roots for Companion semantics; trial IDs lacked early containment&#46;
- Current evidence: At current sealed head f60e458f9d1bceffcba5caaef11e8c0d7312ff59: Current verify&#45;local&#45;acceptance&#46;py has admit&#95;family&#95;roots at198&#45;210 reused by native/bundle/Companion readers and check&#95;companion at975&#45;983&#46; Current deterministic root/unsafe ID/substitution acceptance regression tests executed in the272&#45;test suite&#46;
- Cause / missed boundary: One admitted root now serves all consumers; current AUD&#45;007 is a different eligibility gate&#46; No historical shadow&#45;root success is asserted&#46;
- Suggested current verification: Retain noncanonical same&#45;family roots, conflicting&#45;root and unsafe&#45;trial ID tests, plus exact Companion argv assertions&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;2465aeb&#45;20261001t055252z / P19&#45;AUD&#45;004; source superseded&#45;findings

### Comparison 16: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): The prior external&#45;lock default path owned duplicate materialization/oracle policy instead of shared local CLI&#46;
- Current evidence: At current sealed head f60e458f9d1bceffcba5caaef11e8c0d7312ff59: Current materialize&#45;external&#45;locks&#46;sh603&#45;775 delegates local prepare/conformance and emits the transport receipt only after success; current CI/receipt/acquisition Python tests pass&#46; The current core AC011&#45;ci is met at this static/test seam&#46;
- Cause / missed boundary: Preserved shared policy delegation; manual transport wrappers and immutable byte receipt validation remain justified residual glue, not separate scoring authority&#46;
- Suggested current verification: Retain shared prepare/conformance refusal&#45;before&#45;receipt and workflow transport contract tests&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;2465aeb&#45;20261001t055252z / P19&#45;AUD&#45;007; source superseded&#45;findings

### Comparison 17: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): The prior required rustfmt check failed on committed native&#95;driver assertion wrapping&#46;
- Current evidence: At current sealed head f60e458f9d1bceffcba5caaef11e8c0d7312ff59: Current sealed&#45;export cargo fmt &#45;&#45;check &#45;&#45;all returned0 with no diff; raw current log checks/fmt&#46;log is retained&#46;
- Cause / missed boundary: The specific formatting gate is passing now; no final full workspace/native acceptance claim follows&#46;
- Suggested current verification: Retain cargo fmt &#45;&#45;check &#45;&#45;all at required scope&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;2465aeb&#45;20261001t055252z / P19&#45;AUD&#45;008; source superseded&#45;findings

### Comparison 18: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): The prior two output&#45;control fixture tests depended on missing /usr/bin/uv&#46;
- Current evidence: At current sealed head f60e458f9d1bceffcba5caaef11e8c0d7312ff59: Current test&#95;native&#95;smoke&#95;output&#95;control&#46;py66&#45;69 creates a synthetic uv inside its own tools directory&#46; Current WSL Python suite ran272 tests without errors, with one unrelated suite skip; both ordering/refusal fixture methods remain in the suite&#46;
- Cause / missed boundary: Fixture&#45;owned executable removes the fixed machine&#45;path dependency; passing current tests establish this narrow portability repair&#46;
- Suggested current verification: Retain both output&#45;control proof&#45;before&#45;freeze and refusal ordering fixture cases on clean local WSL/Linux PATH&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;2465aeb&#45;20261001t055252z / P19&#45;AUD&#45;009; source superseded&#45;findings

### Comparison 19: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): The prior runner/run rustdoc described automatic oracle preflight and denied selected real native execution&#46;
- Current evidence: At current sealed head f60e458f9d1bceffcba5caaef11e8c0d7312ff59: Current runner/experiment&#46;rs1&#45;18 explicitly describes pinned real native path and oracle as selected diagnostic; cli/run&#46;rs1&#45;15 distinguishes hermetic and native material&#46; Warning&#45;denied scoped rustdoc and doc checker pass&#46;
- Cause / missed boundary: Specific automatic&#45;oracle/fixture&#45;only descriptions are corrected; this does not excuse current capture/security behavior defects&#46;
- Suggested current verification: Retain current contract descriptions and warning&#45;denied doc checks; inspect semantic wording with behavior consumers when changing native dispatch&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;2465aeb&#45;20261001t055252z / P19&#45;AUD&#45;010; source superseded&#45;findings

### Comparison 20: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): The prior optional reasoning projection called unproven values Effective while native settlement/launch said Unsupported&#46;
- Current evidence: At current sealed head f60e458f9d1bceffcba5caaef11e8c0d7312ff59: Current experiment&#46;rs1202&#45;1220 retains supplied reasoning as requested&#45;but&#45;Unsupported, required native admission rejects, and current projection/native control tests pass&#46; AC006&#45;controls is met at the current owner&#46;
- Cause / missed boundary: Projection and admitted native capability now agree for reasoning; no live effectiveness is inferred&#46;
- Suggested current verification: Retain all&#45;protocol/two&#45;client optional Unsupported and required pre&#45;effect rejection tests&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;2465aeb&#45;20261001t055252z / P19&#45;AUD&#45;011; source superseded&#45;findings
