# Phase 19 Audit

**Audit run ID**: `phase19-codex-gpt6-3e8fee6-20260928t050232z`
**Audit head**: `3e8fee6075c719ca0c3192790b800c13421e4fc4`
**Reviewer ID**: `codex`
**Model ID**: `gpt6`
**Reviewer identity**: `codex`
**Reviewer model ID**: `gpt6`
**Model identity source**: `operator-declared`
**Independence**: `unknown`
**Baseline policy**: `latest-committed-spec`
**Verdict**: `FAIL`

The user declared codex / gpt6. No more specific runtime identity was inferred. This fresh context excluded earlier Phase audit/remediation conclusions and other assurance peers. Three bounded evidence subtasks contributed to this one report; they are not independent assurance members. Model-family independence from implementation authors is unknown.

The requirement set was sealed before production inspection: 55 mandatory and 1 optional record. Mandatory states: {'met': 25, 'not-met': 4, 'not-assessable': 19, 'partially-met': 7}. Findings: {'Major': 11, 'Minor': 2}. Findings explicitly labelled evidence limitations do not claim a reproduced runtime defect.

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| `.opi-impl-state.json` | `c65c10635c4bb8b1754625b94e8a0a15459eef6d65634610ba9b65ea9ba5337e` | current committed source |
| `docs/snapshots/phase19/opi-impl-state.json` | `35b3378bd902cc976bb348d794dd1b5d3c5f160a58769c3aeb62fea175a7b7a9` | current committed source |
| `docs/opi-spec.md` | `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db` | current committed source |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | current committed source |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.zh.md` | `d41c826412b0d71802327d60f953708f6cb2083436a2a2b0d99a2e52d851f194` | current committed source |

Stored source-hash mismatches: none. The root and Phase ledgers entered context only through read_audit_scope.py. Checks ran in a unique git-archive export; no Git worktree or production fix was created. The temporary negative-test append was restored, and every regular archived file compared byte-for-byte equal afterward.

## Requirement Conformance

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|
| `P19-002-agent` | Sections 5; P19-AC-002 | crates/opi-eval/src/benchmark/process.rs:439-659: import requires a unique fresh job, exactly one direct trial result, selected Agent/task/model labels, explicit integration import, and same logical trial/product/executable runtime proof.; crates/opi-eval/src/benchmark/process.rs:3046-3284: production reader tests wrong Agent, oracle aggregate, changed task label, missing selection/terminal, artifact replacement, stale and ambiguous jobs; Unix-gated, not run by this subtask. | met | — |
| `P19-002-artifact` | Sections 5–6; P19-AC-002 | crates/opi-eval/src/benchmark/process.rs:790-907: lifecycle, manifest and delivered digest comparisons feed the binding; declared input digests are persisted.; crates/opi-eval/tests/native_driver.rs:2116-2174 and 2352-2382: substituted delivered bytes reject; cases retain a nonempty delivered manifest and do not replace observed task package path. | not-met | P19-AUD-004 |
| `P19-003-lifecycle` | Sections 5; P19-AC-003 | crates/opi-eval/src/runner/experiment.rs:1830-2033: one task package, one selected integration and one native job request own Agent and grader execution.; crates/opi-eval/tests/native_driver.rs:3485-3494: ignored pinned TB2.1 oracle/nop witness asserts native raw rewards 1/0 via retained_official_witness:3380-3475. | not-assessable | P19-AUD-012 |
| `P19-003-witness` | Sections 5,8; P19-AC-003 | crates/opi-eval/src/cli/conformance.rs:1086-1165: explicit oracle/nop diagnostic selects shared native BenchmarkExecution and bound result reader.; crates/opi-eval/tests/native_driver.rs:3380-3475,3485-3494: retains commands/results and asserts selected diagnostic identity plus raw pinned native reward 1/0. Ignored; requires OPI_EVAL_NATIVE_WITNESS_MATERIAL and Docker, not executed here. | not-assessable | P19-AUD-012 |
| `P19-004-collection` | Sections 5; P19-AC-004 | crates/opi-eval/src/benchmark/task_lifecycle.rs:188-322: parses declared artifacts and shared/separate mode; unsupported lifecycle shapes reject.; crates/opi-eval/tests/native_driver.rs:2352-2382: CLI negative cases discriminate undeclared and substituted declared input and reject sealing. | not-assessable | P19-AUD-012 |
| `P19-004-witness` | Sections 5,8; P19-AC-004 | crates/opi-eval/src/cli/conformance.rs:1086-1165,1206-1212: witness dispatch uses the production native binding and exposes collected source/digest notes.; crates/opi-eval/tests/native_driver.rs:3507-3537: real pinned-material oracle/nop witness is ignored and unexecuted. Different collected digest and grade are asserted. The canary assertion only checks a note excludes the word canary; no unrelated file is injected/queried at this native witness seam. | not-assessable | P19-AUD-012 |
| `P19-005-patch` | Sections 5; P19-AC-005 | crates/opi-eval/src/benchmark/deepswe.rs:973-1022: exact task package admitted; separate/no-network/nonempty collect hook+artifacts required.; crates/opi-eval/tests/native_driver.rs:2441-2500: stand-in run asserts patch digest and pristine-mode evidence; it does not execute real committed-patch collection. | not-assessable | P19-AUD-012 |
| `P19-005-witness` | Sections 5,8; P19-AC-005 | crates/opi-eval/src/cli/conformance.rs:1086-1165: selected Pier oracle/nop entry owns the diagnostic native job, not an Agent score.; crates/opi-eval/tests/native_driver.rs:3551-3580: positive/negative pinned patch witness asserts one model.patch collection, distinct digest, and raw reward 1/0 through helper 3380-3475; ignored/unexecuted. | not-assessable | P19-AUD-012 |
| `P19-002-oracle` | Sections 5; P19-AC-002 | crates/opi-eval/src/runner/experiment.rs:1333-1365,1971-1999: ordinary native run always constructs SelectionKind::Integration; no oracle preflight call remains in this run path.; crates/opi-eval/tests/native_driver.rs:3354-3377: oracle-witness without native material is rejected. | met | — |
| `P19-009-regrade` | Sections 6; P19-AC-009 | crates/opi-eval/src/cli/regrade.rs:1-78: distinct public regrade facade dispatches NativeRegrader and writes separate output.; crates/opi-eval/tests/native_driver.rs:7622-7709: public CLI stand-in test spans all three families, verifies source reports/seals and no Agent handoff in derived output. It does not run real benchmark graders. | not-assessable | P19-AUD-011 |
| `P19-009-regrade-boundary` | Sections 6; P19-AC-009 | crates/opi-eval/src/regrade.rs:335-376,535-552,623-711,724-795: source tampering, missing artifacts, changed base/task identity and unsafe/tampered restored bytes reject.; crates/opi-eval/tests/native_driver.rs:9056-9211: CLI stand-in matrix covers tampered seal/artifact, missing source, wrong base, network policy and existing output; tests immutable outputs. | met | — |
| `P19-009-regrade-witness` | Sections 6,8; P19-AC-009 | crates/opi-eval/src/regrade.rs:524-1007 and both integration verifier-only entrypoints: actual regrade implementation exists but needs pinned-native dynamic evidence.; crates/opi-eval/tests/native_driver.rs:7621-7623: native_offline_regrade is an ordinary stand-in test; admitted scope command ending -- --ignored --exact would execute zero tests with this name. | not-assessable | P19-AUD-011 |
| `P19-006-projection` | Sections 4; P19-AC-006 | crates/opi-eval/src/experiment.rs:590-647,1002-1008,1121-1151,1241-1341: selected named identity projects exact provider/API/base URL/model, credential-reference match, Opi TOML or pi JSON.; crates/opi-eval/src/experiment.rs:1860-1903: exact config identity/auth fields and Opi env-only secret vs pi private inline config; unit evidence inspected, execution delegated to parent. | met | — |
| `P19-006-fixture` | Sections 4,7; P19-AC-006 | crates/opi-eval/tests/native_driver.rs:3870-4082: explicitly ignored native_protocol_client_matrix invokes both supplied clients for three APIs, auth/rate-limit/retry/protocol cases, observed wire output limits and final retry status.; Current test body distinguishes successful retry from exhausted failure and observes headers/model/request paths at the loopback server; skipped by default and not run by this subtask. | not-assessable | P19-AUD-010 |
| `P19-006-controls` | Sections 4; P19-AC-006 | crates/opi-eval/src/experiment.rs:760-784 accepts any supplied required reasoning value; 1017-1057 checks required sampling only; 1173-1176 labels supplied reasoning Effective.; crates/opi-eval/tests/experiment_contract.rs:757-779 rejects reasoning only while fixture value remains omitted; does not exercise supplied required high. | not-met | P19-AUD-006 |
| `P19-006-terminal` | Sections 6; P19-AC-006; task 19.2 | crates/opi-eval/src/agent/pi.rs:286-348: native parser determines terminal error even at exit zero; known status prefixes distinguish auth/rate-limit, unknown cause remains generic.; crates/opi-eval/src/agent/pi.rs:1012-1181 and 1270-1345: final correlated retry success, drift/incomplete retry chain and error-at-exit-zero fixtures are discriminating; parent handles Cargo execution. | met | — |
| `P19-006-model-unknown` | Sections 4,6; P19-AC-006 | crates/opi-eval/src/report.rs:1215-1245 retains typed unknown reasons for unobserved native fact families; src/experiment.rs:689-700 rejects total-token/currency caps.; crates/opi-eval/src/report.rs:1734-1751 checks asymmetric usage/cost unknown reasons without manufactured parity. | not-assessable | P19-AUD-010 |
| `P19-010-secret-input` | Sections 4; P19-AC-010 | crates/opi-eval/src/runner/experiment.rs:1231-1269 resolves only selected Env/File reference, and call is at native launch 1863-1874; recovery returns before launch at 500-501.; crates/opi-eval/tests/native_driver.rs:3795-3830 asserts missing selected credential prevents job effect; 1474-1550 covers file credential projection; 3832-3868 mode mismatch. | met | — |
| `P19-010-secret-grader` | Sections 4; P19-AC-010 | crates/opi-eval/integrations/harbor/opi_eval_harbor.py:638-679 and integrations/pier/opi_eval_pier.py:677-718 remove stage before terminal publication, propagate cleanup failure.; crates/opi-eval/scripts/test_native_integrations.py:687-729,1022-1049 asserts config stage cleanup and failure prevents terminal success; executed against fake transfer/environment. | not-assessable | P19-AUD-010 |
| `P19-010-redaction` | Sections 4,6; P19-AC-010 | crates/opi-eval/src/runner/experiment.rs:1867-1874 consumes actual selected credential without adding it to privacy guard.; crates/opi-eval/tests/native_driver.rs:1821-1883 verifies explicit --canaries blocks seal and further dispatch, but every leak case opts into the declaration. | not-met | P19-AUD-001 |
| `P19-010-supervision` | Sections 4,9; P19-AC-010 | crates/opi-eval/src/experiment.rs:680-687 positive bounded concurrency; runner/experiment.rs:758-796 narrows native width to one and shares run cancellation token.; crates/opi-eval/tests/native_driver.rs:1624-1657 dispatch stops after cleanup/identity failure; 1821-1883 privacy stop; 9225-9429 signal cancellation and 9430-9539 bounded partial capture; inspected, parent executes Unix tests. | met | — |
| `P19-010-cleanup` | Sections 9; P19-AC-010 | crates/opi-eval/src/runner/native_cleanup.rs:137-177 and 320-400 queries complete container IDs by exact Compose directory, inspects identity, removes only verified IDs and rechecks absence; structured error retains uncertainty.; crates/opi-eval/tests/native_driver.rs:1552-1684 distinguishes root-bound IDs, unrelated containers, unconfirmed removal and selected Docker context using stand-in daemon; inspected. | not-assessable | P19-AUD-010 |
| `P19-010-recovery` | Sections 4,6,9; P19-AC-010 | crates/opi-eval/src/runner/experiment.rs:500-501 branches to offline recovery before native preparation/launch; 3982-4198 consumes durable control/inventory, verifies identity and seals, lists every planned row and retains settlement facts.; crates/opi-eval/tests/assembled_run.rs:682-787 snapshots every original file and planned trial across offline recovery and rejects frozen inventory drift; 788-968 checks effect-unknown reopen; inspected, parent executes Unix test binary. | met | — |
| `P19-010-replacement` | Sections 4,6,9; P19-AC-010 | crates/opi-eval/src/runner/experiment.rs:3932-3976 generates fresh experiment/token but changes trial IDs only within target group, retaining other groups unchanged.; crates/opi-eval/tests/assembled_run.rs:44-209 includes unrelated groups and two replacements. Lines 149-190 explicitly expect unchanged old IDs outside target group and load their newly sealed outputs, proving current behavior rather than refuting the normative violation. | not-met | P19-AUD-002 |
| `P19-010-budgets` | Sections 4,9; P19-AC-010 | crates/opi-eval/src/experiment.rs:649-714 rejects caps, total tokens and max_cost; 939-955 requires execution output tokens equal numeric model control.; crates/opi-eval/tests/experiment_contract.rs:730-753 rejects unsupported caps; native_driver.rs:3584-3793 checks required exact-binary output proof and tampered projection/wire/client; 10109-10196 stage timeout consumer checks. | met | — |
| `P19-001-doctor` | Sections 4; P19-AC-001 | crates/opi-eval/src/main.rs:241 dispatches doctor.; crates/opi-eval/tests/fail_closed_cli.rs:179-247 spawns public doctor with closed loopback engine; asserts unreachable/unknown reasons, cache misses, resource fields, and malformed config exit 2; Unix-only file. | met | — |
| `P19-001-prepare` | Sections 4–5; P19-AC-001 | crates/opi-eval/src/cli/prepare.rs:60-80 delegates public prepare under cancellation.; crates/opi-eval/tests/native_driver.rs:5023-5123 public independent-directory prepare then validate, asserts local origin, no GitHub metadata, no repository paths. | met | — |
| `P19-007-material` | Sections 5; P19-AC-007 | crates/opi-eval/src/runner/material.rs:390-499 verifies per-task profile, complete package inventory, pinned corpus identities and materialized bytes.; crates/opi-eval/tests/native_driver.rs:6063 local_material_integrity exercises static lock/profile/corpus drift; 5213/5305 corpus metadata links and unsafe targets. | partially-met | P19-AUD-003 |
| `P19-007-origin` | Sections 5; P19-AC-007 | crates/opi-eval/src/runner/prepare.rs:1755-1791 explicitly emits origin=local, no GitHub transport receipt.; crates/opi-eval/tests/native_driver.rs:5060-5099 explicitly checks truthful local origin and no GitHub/receipt/repository path. | partially-met | P19-AUD-008 |
| `P19-007-inventory` | Sections 5; P19-AC-007 | crates/opi-eval/src/runner/plan.rs:64-112 enumerates all task directories from verified corpus; 275-460 keeps full per-task preflight and exact selected set.; crates/opi-eval/tests/native_driver.rs:6686-6761 public plan enumerates all three fixture corpus tasks while selecting one, retains unknown GPU task and explicit subset. | met | — |
| `P19-007-resources` | Sections 5; P19-AC-007 | crates/opi-eval/src/runner/prepare.rs:2315-2525 selected-engine observations carry unknown/source reasons; check_requirements compares both Agent and verifier CPU/memory/storage/GPU/type requirements.; crates/opi-eval/tests/native_driver.rs:6359-6491 proves unmet/unknown GPU/memory reject entire prepare, no material file, exact selected endpoint, no pull/start. | met | — |
| `P19-007-selection` | Sections 4–5; P19-AC-007 | crates/opi-eval/src/experiment.rs:815-918 validates selection/full/subset/repetitions/seeds.; crates/opi-eval/tests/assembled_run.rs:1424-1540 asserts exact frozen repeated subset, preview and saved report; 1543-1588 tamper rejects before trial effects. | met | — |
| `P19-001-plan-run` | Sections 4; P19-AC-001 | crates/opi-eval/src/main.rs:118-133 public plan; cli/plan.rs:39-47 delegates.; crates/opi-eval/tests/native_driver.rs:8342-8458 independent native frozen-plan run without original config/fixtures and substituted material refusal, but uses absolute staged.material path. | partially-met | P19-AUD-007 |
| `P19-013-placement` | Sections 3; P19-AC-013 | crates/opi-eval/Cargo.toml:1-32 publish=false, workspace-owned metadata/dependencies, no internal Opi crate or LLM client dependency in any table.; crates/opi-eval/tests/native_driver.rs:6312 pi-only material preparation with Opi executable absent. | met | — |
| `P19-013-packaging` | Sections 3,5; P19-AC-013 | crates/opi-eval/Cargo.toml unpublished Independent Companion with no build.rs/features or new crate.; crates/opi-eval/tests/native_driver.rs:10089-10107 package-directory/internal-dependency assertions are narrow structural checks, supplemented by current 189-file tree inspection. | met | — |
| `P19-013-contracts` | Sections 9; P19-AC-013 | crates/opi-eval/src/main.rs current Doctor/Prepare/Plan/Run/Verify/Regrade/Report commands; material/5 and plan/2 headers reject legacy formats.; crates/opi-eval/tests/native_driver.rs:8876 legacy material/4 and plan/1 reject before effects, preserving source artifacts. | partially-met | P19-AUD-009 |
| `P19-011-default` | Sections 8; P19-AC-011 | crates/opi-eval/src/experiment.rs execution mode explicitly resolves hermetic/native-fixture/live; runner/experiment.rs:512-531 rejects mode/material mismatch.; crates/opi-eval/tests/fail_closed_cli.rs:254-312 populates dummy OPENAI/ANTHROPIC/fixture credentials, runs public hermetic command and asserts fixture outcomes plus no credential bytes in durable artifacts. | met | — |
| `P19-011-ci` | Sections 8; P19-AC-011 | .github/workflows/opi-eval-native-smoke.yml:172-183 invokes materialize-configs stage; scripts/native-smoke.sh:893-975 and1029-1187 own a parallel task/image/profile/wrapper/material producer.; crates/opi-eval/scripts/test_verify_native_smoke_ci.py proves existing workflow/producer wiring and safety fields, not equivalence with local prepare. | partially-met | P19-AUD-008 |
| `P19-008-classification` | Sections 6; P19-AC-008; task 19.1 | crates/opi-eval/src/runner/experiment.rs:1398 classify_native_trial combines admitted binding, native terminal and benchmark result; :2399 applies one classification and :2511 seals it; crates/opi-eval/tests/native_driver.rs:1980 native_trial_outcome_classifies_every_terminal_shape drives public run with differing terminal observations | met | — |
| `P19-008-invalid` | Sections 6; P19-AC-008 | crates/opi-eval/src/runner/experiment.rs:1460 native Agent authority failure and :1480 runner provider/verifier failure taxonomy exclude scores; crates/opi-eval/tests/native_driver.rs:6842 native_provider_failure_report and :7836 native_test_counts_exclude_provider_and_verifier_failures; deterministic production path, no real-client qualification | met | — |
| `P19-008-missing` | Sections 6; P19-AC-008 | crates/opi-eval/src/report.rs:255 reads pre-effect run-control; :581 checks binding; :717 planned and unexecuted counts; crates/opi-eval/tests/bundle_recompute.rs:152 planned_coverage_survives_total_bundle_loss and :209 run_control_binds_before_any_effect_and_rejects_drift; observe all planned IDs and zero invented grades | met | — |
| `P19-008-denominators` | Sections 6; P19-AC-008 | crates/opi-eval/src/report.rs:722 valid/invalid/attempted counts and :786 native binary metric definitions; crates/opi-eval/tests/report_contract.rs:208 asserts exact coverage counts, metric meaning, no cross-benchmark average | met | — |
| `P19-008-pairs` | Sections 6; P19-AC-008 | crates/opi-eval/src/report.rs:1323 repetition control pairs and :1395 edge/task/group coverage with protocol/unsupported-control exclusions; crates/opi-eval/tests/report_contract.rs:410 protocol_safe_pairing_and_single_subject_coverage verifies protocol refusal, no artificial solo edge and missing repetition groups | met | — |
| `P19-008-scope` | Sections 6; P19-AC-008 | crates/opi-eval/src/report.rs:780 selection_scope and :828 incomplete run outcome; crates/opi-eval/tests/report_contract.rs:125 asymmetric_native_facts checks own artifacts/digests and unknowns; :208 coverage | met | — |
| `P19-009-verify` | Sections 6; P19-AC-009 | crates/opi-eval/src/cli/verify.rs:43 delegates to byte-only OfflineRegrader; crates/opi-eval/tests/bundle_recompute.rs:15 rejects linked trials preserving source; :259 byte mutation rejection; :331 retained byte closure negative cases | met | — |
| `P19-009-report` | Sections 6; P19-AC-009 | crates/opi-eval/src/report.rs:247 recompute verifies seals and parses sealed controls; :635 build refuses failed verification before render; crates/opi-eval/tests/end_to_end_report.rs:77 normalized_report_is_byte_stable_across_independent_runs; :170 mutation blocks publication; :204 outer run report cannot alter output | met | — |
| `P19-012-verifier` | Sections 7–8; P19-AC-012; tasks 19.10–19.11 | crates/opi-eval/scripts/verify-local-acceptance.py:785 Acceptance.run assembles checks and :758 invokes only verify/report; crates/opi-eval/scripts/test_verify_local_acceptance.py 45 tests passed with recording Companion stub; structural tests are not native grading acceptance | partially-met | P19-AUD-005 |
| `P19-012-provenance` | Sections 8; task 19.10 acceptance declarations | crates/opi-eval/scripts/verify-local-acceptance.py:657 check_scoped_view requires three captures and exact native directory digests; 45 existing local acceptance tests reject mismatched pairs but do not reject coordinated source/cache substitution | partially-met | P19-AUD-005 |
| `P19-001-live` | Sections 7–8; P19-AC-001 | crates/opi-eval/scripts/verify-local-acceptance.py:785 offline verification consumer; crates/opi-eval/tests/native_driver.rs ignored real-material witnesses; No raw six-trial operator evidence root supplied in this audit. Eligible committed export contains deterministic fixtures and source; prior Phase audit conclusions are deliberately excluded. This is an audit evidence limitation, not a claim that no external runs exist. | not-assessable | P19-AUD-013 |
| `P19-003-live` | Sections 7–8; P19-AC-003 | crates/opi-eval/scripts/verify-local-acceptance.py:785 offline verification consumer; crates/opi-eval/tests/native_driver.rs ignored real-material witnesses; No raw six-trial operator evidence root supplied in this audit. Eligible committed export contains deterministic fixtures and source; prior Phase audit conclusions are deliberately excluded. This is an audit evidence limitation, not a claim that no external runs exist. | not-assessable | P19-AUD-013 |
| `P19-004-live` | Sections 7–8; P19-AC-004 | crates/opi-eval/scripts/verify-local-acceptance.py:785 offline verification consumer; crates/opi-eval/tests/native_driver.rs ignored real-material witnesses; No raw six-trial operator evidence root supplied in this audit. Eligible committed export contains deterministic fixtures and source; prior Phase audit conclusions are deliberately excluded. This is an audit evidence limitation, not a claim that no external runs exist. | not-assessable | P19-AUD-013 |
| `P19-005-live` | Sections 7–8; P19-AC-005 | crates/opi-eval/scripts/verify-local-acceptance.py:785 offline verification consumer; crates/opi-eval/tests/native_driver.rs ignored real-material witnesses; No raw six-trial operator evidence root supplied in this audit. Eligible committed export contains deterministic fixtures and source; prior Phase audit conclusions are deliberately excluded. This is an audit evidence limitation, not a claim that no external runs exist. | not-assessable | P19-AUD-013 |
| `P19-006-live` | Sections 7–8; P19-AC-006 | crates/opi-eval/scripts/verify-local-acceptance.py:785 offline verification consumer; crates/opi-eval/tests/native_driver.rs ignored real-material witnesses; No raw six-trial operator evidence root supplied in this audit. Eligible committed export contains deterministic fixtures and source; prior Phase audit conclusions are deliberately excluded. This is an audit evidence limitation, not a claim that no external runs exist. | not-assessable | P19-AUD-013 |
| `P19-012-six-trials` | Sections 7–8; P19-AC-012 | crates/opi-eval/scripts/verify-local-acceptance.py:785 offline verification consumer; crates/opi-eval/tests/native_driver.rs ignored real-material witnesses; No raw six-trial operator evidence root supplied in this audit. Eligible committed export contains deterministic fixtures and source; prior Phase audit conclusions are deliberately excluded. This is an audit evidence limitation, not a claim that no external runs exist. | not-assessable | P19-AUD-013 |
| `P19-012-gates` | Sections 8; P19-AC-012 | crates/opi-eval/scripts/verify-local-acceptance.py:785 offline verification consumer; crates/opi-eval/tests/native_driver.rs ignored real-material witnesses; No raw six-trial operator evidence root supplied in this audit. Eligible committed export contains deterministic fixtures and source; prior Phase audit conclusions are deliberately excluded. This is an audit evidence limitation, not a claim that no external runs exist. | not-assessable | P19-AUD-013 |
| `P19-006-optional-live` | Section 4 optional Chat Completions and Responses connections | crates/opi-eval/src/experiment.rs named connections; README first-connection/local acceptance instructions; Optional real-service qualification was not selected or run; deterministic protocol coverage is distinct. | not-assessable | — |

The requirements JSONL contains the complete surface, test and command evidence. A met record is limited to the behavior it names; adjacent ignored/native/live requirements remain separately visible.

## Standards Review

The Companion remains an unpublished crate with inward dependency direction and no Opi product dependency. Existing private adapters, typed errors, bounded supervision, immutable bundle checks and separate CLI operations remain the main mechanisms. Bilingual product documentation and package/CI surfaces were inspected against actual commands. Contract drift and duplicated material policy are identified in the findings rather than treated as delivery history.

## Spec Review

The production run joins native Agent observations, native grading, lifecycle and artifact admission before sealing a single classification. Offline reports consume that classification and preserve invalid-attempt coverage. Deterministic public CLI tests discriminate identity, outcome, tamper, denominator, pairing and output-containment boundaries. Current negative probes expose unbound runtime bytes, contradictory task-package evidence, required-control admission and replacement-identity failures. Real native and six-trial acceptance obligations are not closed by fixture success.

## Security, Invariants, Integration, Test Quality, and Residuals

Security review traced selected credentials through isolated configuration, native capture, cleanup and export classification. Cleanup and optional canary tests do not close the actual-key seal-guard gap. Invariant review followed task/result/material identity and fresh replacement identity. Integration review covered Harbor/Pier, both Agent clients and the independent prepare/plan/run/verify/report/regrade entry points. Eleven ignored Rust witnesses were not counted as passes. Native verifier diagnostic tasks are useful runner-mechanism coverage, but do not prove pinned benchmark-native regrading. Production test assertions were read for branch reachability and discriminating inputs, not just command exit status.

## Minimum-change Conformance

### Task 19.1: triggered

- reuse_search: Existing private family adapters, BenchmarkExecution/importer, material identity and RunBundle remain the implementation homes. 19.8 reuses source seal/material/digest contracts, while verifier-only execution has its own narrow observation reader.
- surface_necessity: No new crate or public Agent/benchmark trait. NativeSelection/NativeTrialBinding remain pub(crate). CLI regrade is a necessary distinct operation; Harbor/Pier integration modules are materialized process adapters.
- simplification_ceiling: Task identity evidence does not meet selected package binding: P19-AUD-004.
- production_consumers: runner::experiment::run_native_trial; TerminalBench21Adapter::settle; TerminalBench30Adapter::settle; DeepSweAdapter::settle; cli::conformance
- nonproduction_consumers: tests/native_driver.rs; benchmark/process.rs unit tests; scripts/test_native_integrations.py
- net_deletion: Current source has no ordinary run_oracle_preflight or host-Agent-then-native-job score path. Fixture mode stays explicit. Historical quantitative deletion not inferred. Regrade retains newest_job as artifact lookup only, protected by sealed artifact digests; it is not current initial result authority.
- residual_glue: Harbor/Pier modules duplicate environment supervision, runtime staging and much of verifier-only orchestration, with real family-specific API differences; no unused public layer identified. Native grader logic stays external except explicitly test-only private diagnostics.

### Task 19.2: triggered

- reuse_search: Current code reuses ResolvedExperiment/ModelControls, Agent adapters, native terminal importer and shared runner settlement; private duplicated fixed-provider helpers are absent from current source.
- surface_necessity: Named connection/execution/capture controls have production consumers in plan, run, conformance, projection and native integrations; no new Agent Core/provider client/crate dependency is introduced. Public ConnectionProjection is consumed by actual runner and exact-client probes.
- simplification_ceiling: Three fixed APIs and two native clients remain bounded. The recorded revisit trigger is met: required reasoning cannot be expressed by the current native launch; finding P19-AUD-006. Credential projection has a downstream privacy gap P19-AUD-001.
- production_consumers: experiment::ResolvedExperiment::resolve/project_connection; runner::experiment::prepare_native/run_native_trial; runner::plan::build_plan; runner::output_control::snapshot; cli::conformance; Harbor/Pier integration configuration uploads and native commands
- nonproduction_consumers: experiment.rs inline projection tests; experiment_contract.rs; native_driver.rs matrices; test_native_integrations.py; scripted-provider.py and README examples
- net_deletion: Current production uses selected projection; no competing fixed scripted/OpenAI-only native projection found. No historical deletion count inferred from this baseline.
- residual_glue: Family-local Python runtime upload/command/capture/cleanup adapters remain at native runner boundary; terminal parsing stays in existing Rust adapters and final scoring in runner settlement. No new independent classifier found.

### Task 19.3: conforming

- reuse_search: Existing private family adapters, BenchmarkExecution/importer, material identity and RunBundle remain the implementation homes. 19.8 reuses source seal/material/digest contracts, while verifier-only execution has its own narrow observation reader.
- surface_necessity: No new crate or public Agent/benchmark trait. NativeSelection/NativeTrialBinding remain pub(crate). CLI regrade is a necessary distinct operation; Harbor/Pier integration modules are materialized process adapters.
- simplification_ceiling: Native task declarations and family-specific upstream lifecycle are retained; unsupported shapes reject. External pinned execution still needs P19-AUD-012 evidence.
- production_consumers: runner::experiment::run_native_trial; TerminalBench21Adapter::settle; TerminalBench30Adapter::settle; DeepSweAdapter::settle; cli::conformance
- nonproduction_consumers: tests/native_driver.rs; benchmark/process.rs unit tests; scripts/test_native_integrations.py
- net_deletion: Current source has no ordinary run_oracle_preflight or host-Agent-then-native-job score path. Fixture mode stays explicit. Historical quantitative deletion not inferred. Regrade retains newest_job as artifact lookup only, protected by sealed artifact digests; it is not current initial result authority.
- residual_glue: Harbor/Pier modules duplicate environment supervision, runtime staging and much of verifier-only orchestration, with real family-specific API differences; no unused public layer identified. Native grader logic stays external except explicitly test-only private diagnostics.

### Task 19.4: conforming

- reuse_search: Existing private family adapters, BenchmarkExecution/importer, material identity and RunBundle remain the implementation homes. 19.8 reuses source seal/material/digest contracts, while verifier-only execution has its own narrow observation reader.
- surface_necessity: No new crate or public Agent/benchmark trait. NativeSelection/NativeTrialBinding remain pub(crate). CLI regrade is a necessary distinct operation; Harbor/Pier integration modules are materialized process adapters.
- simplification_ceiling: Native task declarations and family-specific upstream lifecycle are retained; unsupported shapes reject. External pinned execution still needs P19-AUD-012 evidence.
- production_consumers: runner::experiment::run_native_trial; TerminalBench21Adapter::settle; TerminalBench30Adapter::settle; DeepSweAdapter::settle; cli::conformance
- nonproduction_consumers: tests/native_driver.rs; benchmark/process.rs unit tests; scripts/test_native_integrations.py
- net_deletion: Current source has no ordinary run_oracle_preflight or host-Agent-then-native-job score path. Fixture mode stays explicit. Historical quantitative deletion not inferred. Regrade retains newest_job as artifact lookup only, protected by sealed artifact digests; it is not current initial result authority.
- residual_glue: Harbor/Pier modules duplicate environment supervision, runtime staging and much of verifier-only orchestration, with real family-specific API differences; no unused public layer identified. Native grader logic stays external except explicitly test-only private diagnostics.

### Task 19.5: drifted

- reuse_search: Confirmed NativeMaterial/load_for, verify_pinned, corpus/task manifests, ProcessSupervisor and build-agent script reuse; no Cargo/core seam.
- surface_necessity: Doctor/prepare, local origin/inventory necessary for independent entry; implementation stays private.
- simplification_ceiling: CI/local material admission divergence remains (P19-AUD-008); cache inventory interpreter identity has no load consumer (P19-AUD-003).
- production_consumers: main doctor/prepare; validate_native; plan build_plan; run prepare_native; regrade material
- nonproduction_consumers: native_driver independent/cold/warm/drift tests; test_prepare_acquisition.py; README pair/local walkthrough
- net_deletion: Current local path removes mandatory GitHub upload receipt/repository fixtures and extra local verifier wrapper; no historical line-count claim.
- residual_glue: CI no-origin runner_project exception and duplicate native-smoke materializer/wrappers remain.

### Task 19.6: conforming

- reuse_search: ResolvedExperiment/DeclaredTrial/NativeMaterial/prepare_native retained; shared expand_selection consumed by plan and run.
- surface_necessity: Plan/run --plan exposes necessary intrinsic freeze/verify state; no new trait/crate/dependency.
- simplification_ceiling: Exact admitted family selection, unsupported/unknown resource refusal; no full TB3 completion promise. Relative path functional defect is P19-AUD-007.
- production_consumers: main/cli plan; cli run; run_experiment; recovery plan verification
- nonproduction_consumers: assembled_run frozen subset/tamper/pi-only; native_driver inventory/multi-task/material drift
- net_deletion: Current config/plan converge on shared expansion; no parallel native selection mechanism.
- residual_glue: Hermetic resource classify remains synthetic path; native uses common effective requirements/selected engine observations.

### Task 19.7: conforming

- reuse_search: ReportBuilder, RunBundle::verify, original byte verifier and existing trajectory/settlement are reused. The CLI verify name separates byte checks from regrade.
- surface_necessity: verify/report have production CLI consumers; planned coverage and settlement are durable inputs, not a new trace protocol.
- simplification_ceiling: One final native classification is consumed for report denominators and invalid coverage. No synthetic cross-benchmark average.
- production_consumers: main::Command::{Verify,Report}; cli::{verify,report}; ReportBuilder; OfflineRegrader; RunBundle::verify
- nonproduction_consumers: bundle_recompute; report_contract; report_output_containment; end_to_end_report; native_driver
- net_deletion: Byte-only regrade survives only as a private historical type name; the public regrade path performs a distinct native operation. No historical deletion count inferred.
- residual_glue: Private legacy-derived handling and deterministic fixture branches stay labelled conformance-only; no public compatibility alias.

### Task 19.8: triggered

- reuse_search: Existing private family adapters, BenchmarkExecution/importer, material identity and RunBundle remain the implementation homes. 19.8 reuses source seal/material/digest contracts, while verifier-only execution has its own narrow observation reader.
- surface_necessity: No new crate or public Agent/benchmark trait. NativeSelection/NativeTrialBinding remain pub(crate). CLI regrade is a necessary distinct operation; Harbor/Pier integration modules are materialized process adapters.
- simplification_ceiling: Pinned native regrade evidence has not been established; private diagnostic graders cannot close the native scope: P19-AUD-011.
- production_consumers: cli::regrade::regrade; NativeRegrader::regrade; Harbor _regrade; Pier _regrade
- nonproduction_consumers: tests/native_driver.rs; benchmark/process.rs unit tests; scripts/test_native_integrations.py
- net_deletion: Current source has no ordinary run_oracle_preflight or host-Agent-then-native-job score path. Fixture mode stays explicit. Historical quantitative deletion not inferred. Regrade retains newest_job as artifact lookup only, protected by sealed artifact digests; it is not current initial result authority.
- residual_glue: Harbor/Pier modules duplicate environment supervision, runtime staging and much of verifier-only orchestration, with real family-specific API differences; no unused public layer identified. Native grader logic stays external except explicitly test-only private diagnostics.

### Task 19.9: drifted

- reuse_search: ProcessSupervisor, process::tree, TrialLifecycle, AuthorityLedger and run_experiment remain current owners; native_cleanup owns exact-root daemon cleanup and recovery reads existing durable bundle facts.
- surface_necessity: Existing run recovery/replacement flags, signal cancellation and private owned-resource receipt have actual production consumers; no extra scheduler or public OS-handle seam.
- simplification_ceiling: Bounded local process/Docker execution is retained; actual Docker cleanup still needs fresh witness evidence. Replacement violates fresh identity outside target group (P19-AUD-002).
- production_consumers: cli::run::run/install_signal_cancellation; runner::experiment::run_experiment/run_native_trial/recovery_report/replace_group; runner::native_cleanup::cleanup_owned_resources; ProcessSupervisor; cli::conformance and regrade cleanup consumers
- nonproduction_consumers: assembled_run.rs recovery/replacement; native_driver.rs cancellation/cleanup/context/partial-capture tests; test_native_integrations.py native supervision tests
- net_deletion: Current single cancellation token and existing settlement/lifecycle govern native execution; no historical deletion count claimed.
- residual_glue: Owned native environment execution wraps runner command lifecycle; exact daemon identity checks and bounded capture-ownership helper are private cleanup mechanisms, not a second scheduler. Existing CancellationSource::User is unused in production, but no separate normative finding raised here.

### Task 19.10: triggered

- reuse_search: Workflow producers and validators retained; shared validate/run downstream. Parent owns acceptance-verifier portion.
- surface_necessity: Transport and explicit fixture/provider/witness orchestration necessary; independent native material policy is not transport-only.
- simplification_ceiling: Recorded replace trigger is met: CI materialize_task/image rewrite/profile generation/wrappers differ from local path (P19-AUD-008).
- production_consumers: native-smoke workflow -> materialize-configs/run-trials; external-lock materialization workflow -> legacy transport
- nonproduction_consumers: native-smoke/external CI verifier tests; external artifact tests; bilingual READMEs
- net_deletion: Required removal of duplicate CI producer absent from current code; no historical diff inferred.
- residual_glue: CI verifier/oracle shell wrappers, generated Agent-wrapper receipt evidence, stale docs ownership; local uses direct uv project.

### Task 19.11: not-assessable

- reuse_search: Consumes existing offline acceptance verifier and Companion verify/report; no new runtime seam.
- surface_necessity: Evidence verification only. No provider authority or Phase ledger writes in this audit.
- simplification_ceiling: The six real trial threshold cannot be lowered to passing component tests. Missing admitted raw evidence prevents closure.
- production_consumers: verify-local-acceptance.py; cli::verify; cli::report
- nonproduction_consumers: test_verify_local_acceptance.py
- net_deletion: Not applicable to assurance evidence; no historical implementation delta inferred.
- residual_glue: Repository-qualified provenance remains separate from Opi-only capture view; current validation gap recorded.

## Findings

### P19-AUD-001: Selected credentials echoed into captures bypass the default seal guard

- Axis: security
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-010-redaction
- Claim: For an explicitly selected full capture run without --canaries, a selected credential echoed in Agent/native output is copied byte-for-byte into an Exportable artifact and can be successfully sealed. Temporary credential-file cleanup does not prevent that exposure.
- Evidence: `crates/opi-eval/src/cli/run.rs:224` — No canary file produces an empty list.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:1867` — Resolved selected credential flows to projection, not automatic privacy scan list.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:2281` — Complete raw Agent stdout/stderr captured without redaction.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:3496` — Generic stage assigns Sensitivity::Exportable to these bytes.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:2530` — Only request.canaries determine privacy seal refusal.
- Refutation attempted: searched: Selected-credential resolver/projection, both integrations, generic bundle staging, CLI/report canary paths, current native privacy/credential tests. strongest_counterevidence: Explicit --canaries rejects before seal and stops dispatch (native_driver.rs:1821-1883); actual config/handoff files are removed (1407-1470); raw failed diagnostics intentionally remain private. why_not_refuted: Neither cleanup nor opt-in canary tests automatically enrolls the selected key; a normal full capture without --canaries still promotes echoed key bytes to sealed export. The finding does not object to unsealed private diagnostic retention.
- Reproduction / closure evidence: Inline Python imported test_native_integrations.py with bytecode disabled; set synthetic STDOUT and agent_env TEST_CREDENTIAL to the same dummy string, then setup/run each Harbor/Pier Opi/pi integration with existing FakeEnvironment
- Reproduction / closure evidence: All four combinations retained the exact dummy key in stdout.log and confirmed temporary stage cleanup. No shell, container, provider, network or real secret involved. This proves capture preservation; the subsequent successful seal/export exposure is the static source-to-sink invariant.
- Reproduction / closure evidence: Static continuation: raw captures at runner/experiment.rs:2271-2287 -> stage Exportable at 3481-3498 -> empty-list scan at 2530-2548 -> bundle.seal at 2551. Use only a dummy selected key and a full-capture tool/native stream that echoes it; omit --canaries.

### P19-AUD-002: Replacement reruns other groups under their old trial identities

- Axis: invariants
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-010-replacement
- Claim: Replacing one group in a multi-group experiment dispatches the complete retained trial list, while only target-group trial IDs are refreshed. Every unrelated group is therefore executed and sealed again with its old trial ID, violating the fresh-identity rule for new real execution. Same-scope replacement is allowed; every rerun must receive a fresh trial identity.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:3952` — Refreshes IDs only where trial.group equals target group.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:767` — All experiment trials are iterated and executed into the new root.
- Evidence: `crates/opi-eval/tests/assembled_run.rs:153` — Current test expects unrelated new_id = old_id and later reads the new sealed bundle for that ID.
- Evidence: `crates/opi-eval/src/bundle/mod.rs:738` — TrialIdentity is the unqualified String; IntentRecord stores it separately from experiment contract.
- Refutation attempted: searched: replace_group, subsequent dispatch loop, TrialIdentity/IntentRecord, recovery and current replacement integration test. strongest_counterevidence: A fresh experiment ID and target paired-group suffix are created; original root bytes remain unchanged. Current test deliberately expects unrelated IDs unchanged. why_not_refuted: Fresh experiment identity and immutable prior bytes do not give each new execution a fresh TrialIdentity. Test expectations document current behavior but cannot override normative CTRL-006.
- Reproduction / closure evidence: cargo test -p opi-eval --test assembled_run replacement_fresh_resolved_pairs_preserve_original_scope -- --exact (parent handles execution).
- Reproduction / closure evidence: Read the selected=false branch: it creates unrelated-group, invokes replacement in fresh roots, preserves unchanged old IDs at 149-166 and reads newly sealed output for them at 168-190. Static run loop establishes each is a new execution.

### P19-AUD-003: Frozen material omits the installed runner interpreter identity

- Axis: invariants
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-007-material
- Claim: prepare records runner interpreter/console entrypoint hashes in inventory.json but material/5 carries only runner source digest and path. validate/plan/run do not read inventory. Replacing the selected interpreter bytes after preparation is accepted with unchanged native_integrity and can enter a new frozen plan, so verified grader execution identity does not bind the bytes selected by UV_PYTHON.
- Evidence: `crates/opi-eval/src/runner/prepare.rs:1199` — Interpreter/entrypoint inventory entries1200-1205; inventory written1430, absent from emitted runner_project1776-1779.
- Evidence: `crates/opi-eval/src/runner/material.rs:165` — Rechecks source digest and UV_* path strings170-174, no interpreter bytes.
- Evidence: `crates/opi-eval/src/runner/material.rs:1293` — Source digest enumerates tracked HEAD; ignored .venv excluded.
- Evidence: `crates/opi-eval/examples/local-evaluation/README.md:74` — Acknowledges interpreter/console-script only checked during prepare and installed runtime is not fully bound.
- Refutation attempted: strongest: native_driver.rs5665-5785 rejects changed tracked runner source/lock and altered UV_PYTHON string; prepare again rejects missing entrypoint. UV_OFFLINE/--locked constrain resolution. README admits incomplete installed closure. why_not_defeated: Loader never consumes interpreter hash in inventory and checks path equality, not bytes. Reproduction replaces already-inventoried selected interpreter, not merely unclaimed transitive dependency. Source/lock and fixed path cannot identify those bytes; current doc caveat is not a registered deferral.
- Reproduction / closure evidence: Existing offline acquisition Fixture with local source repos and fake builders; fresh export CLI prepare=0; validate=0.
- Reproduction / closure evidence: Replace only .venv/bin/python bytes: SHA256306c6ca7407560340797866e077e053627ad409277d1b9da58106fce4cf717cb -> e70dda590096e5760229717e7b454f4ac3e1338c134f2953513f515246a1dd71. validate again=0 with same native_integrity7b96bc3e44af4a34feafee73ad8af047efe0d6000ed0ce096bf28f18ab1958a2; subsequent native plan=0.
- Reproduction / closure evidence: Full MATERIAL-BYTE-PROBE and subsequent plan code preserved in checks. No real model/auth/network/Docker trial/build.

### P19-AUD-004: Native result admission does not bind the observed task package reference

- Axis: invariants
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-002-artifact
- Claim: A completed native result can change config.task.path to another absolute package with the same lifecycle (even a different declared task name or grader bytes), while retaining the selected result.task_name, and still pass the native result importer and post-settlement configuration comparison. The path is not compared with the frozen dispatched package and its contents are not admitted against that package. The runner then seals the contradictory config as selected-trial evidence. This claim concerns rejection of substituted observed evidence, not a claim that the pinned upstream runner normally dispatches the wrong task.
- Evidence: `crates/opi-eval/src/benchmark/process.rs:1049` — Reads any absolute observed config.task.path and parses it; result is only mode/declarations/base/image (1111-1116), omitting task name, path and package hash.
- Evidence: `crates/opi-eval/src/benchmark/process.rs:794` — Equality checks only TaskLifecycleSelection fields against selected facts.
- Evidence: `crates/opi-eval/src/benchmark/process.rs:1636` — Only effective environment import/caps/multipliers compared; expected tasks[0].path is unused.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:2319` — Admitted native config/result are copied into sealed evidence; classification/sealing has no later task path/package check.
- Refutation attempted: searched: All three family admission/settle methods, BenchmarkExecution::run, run_native_trial classification/sealing, current native_driver and process.rs substitution tests. strongest_counter_evidence: Before spawn the selected task bytes are admitted (terminal_bench_21.rs:742-762; terminal_bench_30.rs:942-967; deepswe.rs:973-1022), native_job_config:1589-1629 sends the exact canonical task path, and cleanup ownership is bounded to expected task roots. Tests reject pre-dispatch package drift and changed result task label. why_not_refuted: Those facts prove intended dispatch and cleanup authority, not equality of the observed result package reference with the dispatched package. The importer explicitly accepts observed task identity from another path. No registered deferral permits contradictory native task evidence.
- Reproduction / closure evidence: Static invariant: NativeSelection contains no admitted task path/package digest; parse_recorded_lifecycle consumes arbitrary absolute observed task path but projects away task name and file identity; native_stage_config_matches never compares task path.
- Reproduction / closure evidence: Executed the included audit_probe appended only to the disposable exported process.rs: cargo test --offline -p opi-eval --lib audit_native_result_accepts_substituted_task_reference. PASS: 1 executed, importer accepted the substituted task reference and native_stage_config_matches returned true. Original source bytes were restored and the complete archive byte comparison passed.

Disposable test used against unchanged production logic:

```rust
#[cfg(all(test, unix))]
#[test]
fn audit_native_result_accepts_substituted_task_reference() {
    let dir = tempfile::tempdir().unwrap();
    let selection = stage_bound_job(dir.path(), "j1", "opi-eval-opi");
    let config_path = dir.path().join("jobs/j1/fixture-task__bZZeEkw/config.json");
    let mut config: serde_json::Value = serde_json::from_slice(&std::fs::read(&config_path).unwrap()).unwrap();
    let original_path = config["task"]["path"].clone();
    let other = dir.path().join("substituted-task-package");
    std::fs::create_dir(&other).unwrap();
    // Distinct task label and verifier bytes, but identical lifecycle facts.
    std::fs::write(other.join("task.toml"), "schema_version = '1.0'\n[task]\nname = 'different-task'\n").unwrap();
    std::fs::write(other.join("test.sh"), "#!/bin/sh\necho 1\n").unwrap();
    config["task"]["path"] = serde_json::json!(other);
    config["agent_timeout_multiplier"] = serde_json::json!(1.0);
    config["verifier_timeout_multiplier"] = serde_json::json!(1.0);
    std::fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let expected = serde_json::json!({"tasks":[{"path":original_path}], "agents":[{}], "verifier":{}, "environment":{}});
    assert_ne!(config["task"]["path"], expected["tasks"][0]["path"]);
    let binding = import_bound_native_job(dir.path(), &selection).unwrap();
    assert_eq!(binding.task_name, "fixture-task");
    assert!(native_stage_config_matches(&config_path, &expected));
}
```

### P19-AUD-005: Acceptance provenance trusts an unbound operator cache and optional inventories

- Axis: invariants
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-012-verifier, P19-012-provenance
- Claim: The offline acceptance verifier admits arbitrary upstream repository/commit pairs when the same pairs are added to the supplied identity-cache JSON. It does not bind those records to admitted external inventories. Empty material and preserved-artifact lists also pass their checks. Thus acceptance can affirm provenance without proving the required pinned runner/source/material identities.
- Evidence: `crates/opi-eval/scripts/verify-local-acceptance.py:220` — check_material loops over an optional list without required-family cardinality; check_preserved at 282 similarly accepts an empty list.
- Evidence: `crates/opi-eval/scripts/verify-local-acceptance.py:711` — check_upstream_identities compares manifest.sources only to the supplied upstream-identity-cache.json. It verifies no cache identity, admitted lock entry or repository contents.
- Evidence: `crates/opi-eval/scripts/verify-local-acceptance.py:758` — Downstream Companion calls are only verify/report, whose inputs are bundle seals and report content, not manifest.sources or the upstream cache.
- Refutation attempted: searched: Complete Acceptance.run, check_controls/check_agent_observations/check_scoped_view/check_companion, the actual Rust verify/report consumers and all 45 acceptance verifier tests. strongest_counterevidence: Tests reject a source pair absent from the supplied cache. Controls and runtime preflight are byte-bound to bundle entries, and real Companion verification rejects malformed or mutated bundles. why_not_refuted: Coordinated source/cache substitution is outside those checks. Companion byte verification never consumes the upstream cache and cannot establish its link to admitted material; selected runtime hashes do not validate Harbor/Pier source claims.
- Reproduction / closure evidence: Using existing test_verify_local_acceptance.EvidenceRoot, set each source URL to https://example.invalid/unadmitted/<name> and commit to forty zeroes, add those pairs to upstream-identity-cache.json, and empty manifest.material and manifest.preserved. check_upstream_identities/check_material/check_preserved all return successfully.
- Reproduction / closure evidence: Run current public verification CLI with this existing recording Companion stub: exit 0, accepted=true, six trials. Audit script ../checks/probe_acceptance.py reproduced this. The stub isolates verifier validation; it is not real native-trial evidence.

### P19-AUD-006: Required reasoning can run with unsupported effective control

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-006-controls
- Claim: A native experiment with reasoning="high" and required=["reasoning"] is admitted before effects; Pi is launched with --thinking off, and reasoning is only declared unsupported during post-execution pairing. Required unsupported controls therefore do not fail before native execution.
- Evidence: `crates/opi-eval/src/experiment.rs:770` — Any supplied reasoning value satisfies required list.
- Evidence: `crates/opi-eval/src/experiment.rs:1042` — Native admission enumerates only temperature and top_p.
- Evidence: `crates/opi-eval/src/runner/output_control.rs:39` — Additional admission recognizes only max_output_tokens.
- Evidence: `crates/opi-eval/integrations/harbor/opi_eval_harbor.py:810` — Native pi launch explicitly disables thinking; Pier has same command at 855.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:2568` — Unsupported reasoning is added after job execution and sealing.
- Refutation attempted: searched: ResolvedExperiment required validation, both public native plan/run admission paths, output-control evidence, both integration commands, post-execution comparability and current required-control tests. strongest_counterevidence: Unsupported sampling and unproven required output limits reject before dispatch. Existing required reasoning test at experiment_contract.rs:771-778 rejects an omitted value; post-run pairing refuses reasoning comparability. why_not_refuted: The required reasoning test never supplies high; post-run non-comparability occurs after the forbidden effects and does not satisfy pre-execution rejection.
- Reproduction / closure evidence: Static invariant: configure native Pi with reasoning="high", required=["reasoning"], otherwise valid pinned material. Resolve sees a Value; admit_native_controls has no reasoning check; output_control::admit checks only max_output_tokens; native command still includes --thinking off.
- Reproduction / closure evidence: python crates/opi-eval/scripts/test_native_integrations.py -q: executed test_package_and_single_binary_launches_retain_evidence asserts --thinking off at line 715.

### P19-AUD-007: Frozen relative material paths change base at execution

- Axis: integration
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-001-plan-run
- Claim: plan saves relative --native-material as provided, but run --plan resolves it against the plan directory. Nested output paths produce accepted plans that fail to load unchanged material.
- Evidence: `crates/opi-eval/src/runner/plan.rs:540` — Copies supplied path verbatim;603-606 persists it.
- Evidence: `crates/opi-eval/src/cli/run.rs:96` — Default lookup uses resolve_plan_path; plan.rs734-743 joins relative path to plan parent.
- Refutation attempted: strongest: native_driver.rs8342 tests running frozen plan from unrelated cwd and material drift; explicit material override works. why_not_defeated: That test freezes absolute staged.material. An override workaround does not correct the default accepted relative-input path.
- Reproduction / closure evidence: Fresh export public prepare produced material; public plan --config selection.toml --native-material material/material.json --out plans/actual.json exits0 and persists material/material.json.
- Reproduction / closure evidence: From same directory, run --plan plans/actual.json --root run exits2: No such file or directory (os error2). Full offline reproduction code in checks.

### P19-AUD-008: CI retains a parallel native-material preparation policy

- Axis: spec
- Severity: Minor
- Conformance effect: blocks
- Requirement IDs: P19-007-origin, P19-011-ci
- Claim: CI native-smoke materialize-configs independently copies/rewrites tasks, resolves image references, generates profiles and verifier wrappers and emits material/5 without the local RunnerProject contract. This is the second materialization policy forbidden by Section 8 and task19.10's replacement ceiling; allowed fixture/provider/witness orchestration is not the finding.
- Evidence: `.github/workflows/opi-eval-native-smoke.yml:172` — Workflow invokes materialize-configs, not local prepare.
- Evidence: `crates/opi-eval/scripts/native-smoke.sh:893` — Independent materialize_task image/TOML/Dockerfile rewriting/profile inventory through975.
- Evidence: `crates/opi-eval/scripts/native-smoke.sh:1035` — Verifier/oracle wrappers, environment, material emission through1187 omit runner_project; local prepare.rs1435-1463,1776 instead binds direct pinned uv project.
- Evidence: `crates/opi-eval/src/runner/material.rs:351` — No-origin CI bypasses mandatory local runner_project, demonstrating policy difference beyond transport.
- Refutation attempted: strongest: native-smoke.sh1304-1316 invokes public validate and1487-1490 invokes shared run; scoring/admission readers are reused. Explicit bounded native witness/provider orchestration is authorized. why_not_defeated: Sharing downstream reader/scoring does not eliminate independent task/image/profile materialization and different runner source/environment admission. Task19.10 explicitly called for removing duplicate CI materialization after local replacement.
- Reproduction / closure evidence: Trace current workflow materialize-configs into embedded Python; no local prepare call in chain. Separate materialize-external-locks.sh prepare mode at40-425 is local acquisition only.
- Reproduction / closure evidence: WSL native-smoke CI verifier35PASS preserves current producer shape, not shared prepare equivalence.

### P19-AUD-009: Local walkthrough plan command omits required input

- Axis: standards
- Severity: Minor
- Conformance effect: blocks
- Requirement IDs: P19-013-contracts
- Claim: The packaged local walkthrough's plan command supplies neither native material nor fixtures and fails in every supported execution mode. Main bilingual README sections also retain CI-owned materialization/regeneration directions despite the independent prepare entry.
- Evidence: `crates/opi-eval/examples/local-evaluation/README.md:155` — plan invocation omits --native-material after local prepare.
- Evidence: `crates/opi-eval/src/runner/plan.rs:550` — Native without material rejects through568; hermetic without fixtures rejects at127.
- Evidence: `crates/opi-eval/README.md:530` — Workflow-owned materialization/fidelity; rejection guidance598, mirrored README.zh.md456-468,517.
- Refutation attempted: strongest: Earlier main README local plan examples include --native-material and parent doc checker passes. why_not_defeated: Correct earlier examples do not repair executable walkthrough or contradictory later ownership guidance; doc checker does not execute commands.
- Reproduction / closure evidence: Exact documented shape plan --config selection.toml --out plans/docs.json exits2: execution mode native-fixture/live requires --native-material. Full code in checks.

### P19-AUD-010: Native client and real cleanup evidence remains unassessed

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-006-fixture, P19-006-model-unknown, P19-010-secret-grader, P19-010-cleanup
- Claim: This fresh audit has no executed current exact-client matrix or actual Docker/shared-verifier credential cleanup witness. Fake-environment and stand-in tests cannot establish the mandatory real-client wire/tool loops, observed service-model identity or native grader/resource cleanup claims. This is an evidence limitation, not a reproduced defect in those native paths.
- Classification: explicit evidence limitation; no runtime failure is inferred from absence.
- Evidence: `crates/opi-eval/tests/native_driver.rs:3870` — Exact-client matrix is ignored and needs separately supplied local client material.
- Evidence: `crates/opi-eval/tests/native_driver.rs:9556` — Owned-resource cancellation is ignored and needs actual engine/image; conformance cleanup witness at 2894 is also ignored.
- Evidence: `crates/opi-eval/scripts/test_native_integrations.py:313` — Executed FakeEnvironment explicitly has no shell/container/provider; suite result 39 tests, 15 skipped.
- Evidence: `crates/opi-eval/src/runner/plan.rs:668` — Plan model is selected configuration; no independent actual service-model attestation is established from it.
- Refutation attempted: searched: Current matrix, output-control probe, fake integration tests, cleanup stand-ins and ignored native witness bodies; previous audit/operator records excluded by audit independence boundary. strongest_counterevidence: Current test bodies contain meaningful protocol/retry/auth and ownership assertions, and 24 Python tests passed. why_not_refuted: Default/Windows test passes cannot execute ignored or POSIX/Docker scenarios; configured model identity is not observed service-model identity.
- Reproduction / closure evidence: Run the ignored native_protocol_client_matrix, native_client_provider_failure_report and native_owned_resource_cancellation with admitted exact clients/materials/Docker in the local offline test environment; inspect actual captures and grader canary absence. Do not use live provider credentials.

### P19-AUD-011: Pinned benchmark-native offline regrade witness is not established

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-009-regrade, P19-009-regrade-witness
- Claim: Current native regrade evidence surfaces do not establish fresh offline grading of preserved pinned benchmark artifacts in all three families. The named native_offline_regrade test is a stand-in; the ignored Docker verifier witness replaces benchmark tasks and graders with private diagnostic answer.txt graders. No pinned benchmark-native regrade execution/artifacts were supplied to this audit subtask, so this mandatory dynamic requirement is not assessable.
- Classification: explicit evidence limitation; no runtime failure is inferred from absence.
- Evidence: `crates/opi-eval/tests/native_driver.rs:7621` — native_offline_regrade is a regular test using stage("ok") stand-ins, not an ignored real-native test.
- Evidence: `crates/opi-eval/tests/native_driver.rs:10756` — Ignored verifier-only regrade witness immediately calls controlled_material.
- Evidence: `crates/opi-eval/tests/support/native_witness_material.rs:45` — Helper explicitly builds private tasks and diagnostic clients around supplied runners.
- Evidence: `crates/opi-eval/tests/support/native_witness_material.rs:199` — Writes new diagnostic task identities, artifact declarations, policies and contexts.
- Evidence: `crates/opi-eval/tests/support/native_witness_material.rs:269` — Generates answer.txt reward/CTRF test script and replaces corpus lock subject at 284-299.
- Refutation attempted: searched: NativeRegrader, both Python _regrade entries, native_offline_regrade, native_verifier_only_regrade_witness, and controlled_material. strongest_counter_evidence: The ignored verifier-only witness runs actual Harbor/Pier machinery, tests positive/negative inputs, cleanup, tamper and immutable source bytes. Ordinary tests cover production CLI wiring for all families. why_not_refuted: Real runner execution is useful mechanism coverage, but it runs newly written diagnostic graders with replaced corpus identity. Specification and task 19.8 require pinned native grading of saved real artifacts; source inference or synthetic native mechanics do not supply that execution evidence.
- Reproduction / closure evidence: Inspect native_driver::native_offline_regrade and native_verifier_only_regrade_witness call chains; controlled_material writes the grader observed by the latter.
- Reproduction / closure evidence: A valid closure must preserve actual pinned task/grader material identity and artifacts for native regrade in each family, with no Agent/provider invocation. Running the existing synthetic test cannot by itself close that obligation.

### P19-AUD-012: Required pinned native lifecycle witnesses are not assessable in this run

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-003-lifecycle, P19-003-witness, P19-004-collection, P19-004-witness, P19-005-patch, P19-005-witness
- Claim: The code routes all three families through selected native integrations and has opt-in oracle/nop witness tests, but their pinned external material, local Docker execution and retained results were not supplied or executed within this audit. The audit therefore cannot mark actual shared-environment, separate artifact collection, or pristine committed-patch grading and their positive/negative witnesses met. This is an audit evidence limitation, not a claim that the implementations fail when correctly provisioned.
- Classification: explicit evidence limitation; no runtime failure is inferred from absence.
- Evidence: `crates/opi-eval/tests/native_driver.rs:3485` — TB2.1 witness explicitly ignored, requires pinned external material and local Docker.
- Evidence: `crates/opi-eval/tests/native_driver.rs:3507` — TB3 witness explicitly ignored; assertions compare collection notes/rewards; no supplied native result available.
- Evidence: `crates/opi-eval/tests/native_driver.rs:3551` — DeepSWE witness explicitly ignored; no supplied patch-to-native-grade execution available.
- Evidence: `crates/opi-eval/scripts/test_native_integrations.py:1` — Python tests explicitly fake imported runner and task environment; 8 narrow tests passed but cannot qualify actual upstream semantics.
- Refutation attempted: searched: All three production adapters/integrations, current witness helper/tests and deterministic native result/import rejection cases. strongest_counter_evidence: Real pinned witness entrypoints and discriminating rewards 1/0 exist; deterministic production CLI tests cover binding and lifecycle metadata. why_not_refuted: Ignored tests and fake upstream/environment stand-ins do not demonstrate actual native artifact handling or score responsiveness. Required external integration behavior cannot be established by source alone under the audit anti-vacuity rule.
- Reproduction / closure evidence: Execute native_tb21_artifact_witness, native_tb3_artifact_witness and native_deepswe_patch_witness explicitly on the local WSL/Linux/Docker platform with verified pinned OPI_EVAL_NATIVE_WITNESS_MATERIAL and retain artifacts. No paid provider required.
- Reproduction / closure evidence: The complete audit ran 362 deterministic Rust tests in WSL/Linux; all 11 material-dependent native witnesses remained ignored. No pinned external native material was supplied as audit input.

### P19-AUD-013: Six real local trials and full Phase-exit evidence are not assessable from admitted input

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-001-live, P19-003-live, P19-004-live, P19-005-live, P19-006-live, P19-012-six-trials, P19-012-gates
- Claim: No raw redacted six-trial operator evidence root was supplied to this fresh audit. Committed deterministic fixtures and an acceptance verifier do not establish the mandatory assembled local Zhipu/Opi/pi runs. The full workspace Phase-exit verification union was not rerun. These requirements remain not-assessable; this does not claim that external runs never occurred.
- Classification: explicit evidence limitation; no runtime failure is inferred from absence.
- Evidence: `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md:266` — P19-AC-012 requires six real attributable trials; authentication/infra/missing-tool-loop failures do not substitute.
- Evidence: `crates/opi-eval/scripts/test_verify_local_acceptance.py:115` — EvidenceRoot is synthetic and uses a recording Companion stub; its successful tests cannot supply the six real trials.
- Evidence: `crates/opi-eval/tests/native_driver.rs:3870` — Real-client and native/Docker witnesses are explicitly ignored; 11 tests were ignored in the audited all-targets run.
- Refutation attempted: searched: Current registered obligations, allowlisted scope declarations, committed fixture/test surfaces and verifier input contract. Previous Phase-level audit/remediation conclusions were not consulted. strongest_counterevidence: 362 Rust tests passed in WSL/Linux and the acceptance verifier has 45 passing deterministic tests. Current source includes real-client/native witness entrypoints. why_not_refuted: These establish deterministic implementation evidence but neither actual six-trial remote-provider evidence nor ignored native execution. Prior conclusions and scope declarations cannot replace preserved measurements.
- Reproduction / closure evidence: Supply preserved redacted operator material and validate it offline with verify-local-acceptance.py --root <evidence> --out <fresh-output> --companion <audited-binary>; inspect native handoff, per-trial provider/tool evidence and retained failed attempts.
- Reproduction / closure evidence: Retain the required workspace gate results for the final relevant source identity. This audit ran the narrow opi-eval all-targets suite, format and documentation checks, not the full workspace release/Phase-exit union.

## Verification Commands

| Command | Result | Scope / limitation |
|---|---|---|
| `cargo test --offline -p opi-eval --all-targets` (WSL/Linux) | PASS | 362 passed, 0 failed, 11 ignored; 14 suites; native_driver 105 passed / 11 ignored. |
| `python3 crates/opi-eval/scripts/test_verify_local_acceptance.py` | PASS | 45 tests; synthetic evidence and recording Companion stub. |
| `python3 scripts/opi-doc-check.py` (WSL/Linux export) | PASS | Native Windows extraction lookup did not resolve the POSIX alias; WSL resolved the committed alias and passed. |
| `cargo fmt --check --all` | PASS | Original exported source before temporary probe. |
| Temporary `audit_native_result_accepts_substituted_task_reference` | Defect reproduced | 1 test executed; substituted task reference accepted; original bytes restored. |
| Acceptance provenance substitution probe | Defect reproduced | Unadmitted source/cache pairs and empty inventories accepted by existing verifier. |
| Archive regular-file byte comparison | PASS | All committed file bytes unchanged after checks. |
| python crates/opi-eval/scripts/test_native_integrations.py VerifierOnlyEnvironmentTest NativeIntegrationTest.test_package_and_single_binary_launches_retain_evidence NativeIntegrationTest.test_old_handoff_schema_rejects NativeIntegrationTest.test_missing_or_unsafe_trial_identity_rejects_before_upload NativeIntegrationTest.test_rejects_runtime_drift_before_any_upload | recorded | PASS: 8 tests ran in 0.696 seconds, 0 skipped, exit 0. Both production integration modules are loaded; imported external runner types and task environment are fakes. Does not qualify native Docker/benchmark execution. |
| python crates/opi-eval/scripts/test_native_integrations.py -q | recorded | Exit 0; Ran 39 tests in 3.799s; OK (skipped=15). Windows Python 3.12.7 executed 24 offline fake-environment tests; POSIX shell/process-group and Docker tests skipped. |
| Inline Python imported test_native_integrations.py with bytecode disabled; set synthetic STDOUT and agent_env TEST_CREDENTIAL to the same dummy string, then setup/run each Harbor/Pier Opi/pi integration with existing FakeEnvironment | recorded | All four combinations retained the exact dummy key in stdout.log and confirmed temporary stage cleanup. No shell, container, provider, network or real secret involved. This proves capture preservation; the subsequent successful seal/export exposure is the static source-to-sink invariant. |
| python -m unittest discover -s crates/opi-eval/scripts -p test_verify_external_lock_artifact.py | recorded | 53PASS |
| python -m unittest discover -s crates/opi-eval/scripts -p test_verify_external_lock_ci.py | recorded | 15PASS |
| WSL python3 -m unittest discover -s crates/opi-eval/scripts -p test_verify_native_smoke_ci.py | recorded | 35PASS. Initial Windows one bash Windows-path failure; resolved on supported WSL. |
| WSL python3 -m unittest discover -s crates/opi-eval/scripts -p test_prepare_acquisition.py | recorded | 8PASS in47.524s. Initial Windows /usr/bin/git assumptions fail; WSL resolves. |
| MATERIAL-BYTE-PROBE: feed Python below to WSL python3 stdin at exported source cwd; uses parent-built binary | recorded | prepare0; validate before/after interpreter drift0 with identical integrity. Probe_root printed by script; local repo/build stand-ins only. |
| MATERIAL-RELATIVE-PLAN-PROBE: after byte probe feed Python below to WSL python3 stdin, replace root with printed probe_root | recorded | documented_plan2, real_plan0, real_run_relative2 ENOENT; plan freezes even after interpreter substitution. |
| Full workspace clippy/tests/doctests/rustdoc and real native/live witnesses | NOT RUN | Explicit gaps above; the narrow audit check union is not a Phase-exit claim. |

Installation additionally requires the live ancestry, staged-member, rotation and full audit-set validators. Their outcomes are reported in the handoff; no manual copying into the active set is permitted.

## Verdict Rationale

`FAIL` follows mechanically from the non-met mandatory records. Passing deterministic tests does not compensate for reproduced contract failures or missing mandatory native/live evidence. This audit authorizes no remediation, provider call, commit or publication.

Test impact: retain. No production/test changes are retained in the repository. The only outgoing files are this protocol-defined four-file report group, its live index change and the installer-managed immutable history of the replaced member. No bilingual counterpart was added because the assurance contract fixes exactly these four per-peer files; bilingual product sources were inspected, not changed.

## Supplemental Negative Probes

Run these only against an offline disposable export. The material probes use the existing fake acquisition fixtures and local repositories. Set the binary path to an opi-eval binary freshly built from the audited source; in the second material probe, replace root with the first probe printed probe_root. The acceptance probe uses the existing recording Companion stub and demonstrates validator admission only.

### MATERIAL-BYTE-PROBE

```python
from pathlib import Path
import importlib.util,tempfile,subprocess,json,os
source=Path.cwd()
spec=importlib.util.spec_from_file_location("acq",source/"crates/opi-eval/scripts/test_prepare_acquisition.py")
acq=importlib.util.module_from_spec(spec); spec.loader.exec_module(acq)
root=Path(tempfile.mkdtemp(prefix="opi-material-byte-probe-"))
f=acq.Fixture(root/"fixture")
binary="/mnt/opi-audit-e-20260926/opi/cargo-targets/opi-8abcf38f9871653e/debug/opi-eval"
text=(source/"crates/opi-eval/tests/fixtures/experiment/local-paired.toml").read_text()
text="\n".join("cache = "+json.dumps(str(f.cache)) if line.startswith("cache = ") else 'task = "openssl-selfsigned-cert"' if line.startswith("task = ") else line for line in text.splitlines())
config=root/"experiment.toml";config.write_text(text)
env={"PATH":str(f.bin)+":/usr/bin:/bin","HOME":str(root),"DOCKER_HOST":"tcp://127.0.0.1:1"}
args=[binary,"prepare","--config",str(config),"--out",str(root/"material"),"--opi-source",str(f.opi),"--opi-commit",f.opi_commit]
r=subprocess.run(args,capture_output=True,text=True,env=env)
print(json.dumps({"step":"prepare","code":r.returncode,"stdout":r.stdout[-1200:],"stderr":r.stderr[-3000:]}))
if r.returncode: raise SystemExit(1)
material=root/"material/material.json"
argv=[binary,"validate","--config",str(config),"--native-material",str(material)]
before=subprocess.run(argv,capture_output=True,text=True,env=env)
print(json.dumps({"step":"validate_before","code":before.returncode,"stdout":before.stdout,"stderr":before.stderr}))
target=f.cache/"runners/harbor/.venv/bin/python"
old=acq.sha(target);target.write_text("#!/bin/sh\nprintf substituted-runtime >&2\nexit 93\n");target.chmod(0o755)
after=subprocess.run(argv,capture_output=True,text=True,env=env)
print(json.dumps({"step":"validate_after_interpreter_drift","before_sha":old,"after_sha":acq.sha(target),"code":after.returncode,"stdout":after.stdout,"stderr":after.stderr}))
print("probe_root="+str(root))
```

### MATERIAL-RELATIVE-PLAN-PROBE

```python
from pathlib import Path
import subprocess,json,os
root=Path("/tmp/opi-material-byte-probe-927q309l")
binary="/mnt/opi-audit-e-20260926/opi/cargo-targets/opi-8abcf38f9871653e/debug/opi-eval"
config=root/"selection.toml"
config.write_text((root/"experiment.toml").read_text().split("[[trials]]")[0] + '\n[selection]\nscope="subset"\ntask_ids=["openssl-selfsigned-cert"]\nreason="audit relative path"\nrepetitions=1\n[execution]\nmode="native-fixture"\n')
(root/"plans").mkdir(exist_ok=True)
env={"PATH":str(root/"fixture/bin")+":/usr/bin:/bin","HOME":str(root),"DOCKER_HOST":"tcp://127.0.0.1:1"}
for label,args in [
("documented_plan",[binary,"plan","--config","selection.toml","--out","plans/docs.json"]),
("real_plan",[binary,"plan","--config","selection.toml","--native-material","material/material.json","--out","plans/actual.json"]),
("real_run_relative",[binary,"run","--plan","plans/actual.json","--root","run"])]:
    p=subprocess.run(args,cwd=root,capture_output=True,text=True,env=env)
    print(json.dumps({"label":label,"code":p.returncode,"stdout":p.stdout[:500],"stderr":p.stderr}))
if (root/"plans/actual.json").exists():
    p=json.loads((root/"plans/actual.json").read_text())
    print(json.dumps({"frozen_material":p["material"],"bound_interpreter":json.loads((root/"material/material.json").read_text())["benchmarks"]["terminal-bench-2.1"]["verifier_env"]["UV_PYTHON"]}))
```

### Acceptance provenance probe

Run from the exported repository root:

```python
import sys,importlib.util,json,tempfile
from pathlib import Path
source=Path.cwd()
sys.path.insert(0,str(source/"crates/opi-eval/scripts"))
import test_verify_local_acceptance as fixtures
spec=importlib.util.spec_from_file_location("acceptance",source/"crates/opi-eval/scripts/verify-local-acceptance.py")
module=importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
with tempfile.TemporaryDirectory(prefix="audit-provenance-") as tmp:
 e=fixtures.EvidenceRoot(Path(tmp))
 m=e.load_manifest()
 c=json.loads((e.root/"upstream-identity-cache.json").read_text())
 for name in ("pi","harbor","pier"):
  url="https://example.invalid/unadmitted/"+name
  commit="0"*40
  m["sources"][name]={"repository_url":url,"commit":commit}
  c["repositories"][url]={"commits":{commit:{"tree":"0"*40}}}
 fixtures.write_json(e.root/"upstream-identity-cache.json",c)
 m["material"]=[]; m["preserved"]=[]
 e.save_manifest(m)
 a=module.Acceptance(e.root,e.outside,e.companion)
 a.check_upstream_identities(m); a.check_material(m); a.check_preserved(m)
 print("PASS-THROUGH: all upstream sources replaced by unadmitted repository/zero commit; material/preserved inventories empty")
 result=e.verify()
 print("Full public verifier with existing recording stub:",result.returncode,result.stdout.strip())
 assert result.returncode == 0, result.stderr
```

<!-- opi-audit-history/1
{
  "audit\u005fhead": "3e8fee6075c719ca0c3192790b800c13421e4fc4",
  "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
  "comparison": {
    "comparisons": [
      {
        "cause": "Capture/export does not automatically enroll selected credentials as canaries. Existing tests use explicit canaries or check private-file projection, so an echoed selected credential bypasses the default seal guard.",
        "current\u005fevidence": "For an explicitly selected full capture run without --canaries, a selected credential echoed in Agent/native output is copied byte-for-byte into an Exportable artifact and can be successfully sealed. Temporary credential-file cleanup does not prevent that exposure. Frozen evidence locations: crates/opi-eval/src/cli/run.rs:224; crates/opi-eval/src/runner/experiment.rs:1867; crates/opi-eval/src/runner/experiment.rs:2281; crates/opi-eval/src/runner/experiment.rs:3496; crates/opi-eval/src/runner/experiment.rs:2530",
        "current\u005ffinding\u005fid": "P19-AUD-001",
        "prior\u005fclaim": "Earlier detected-canary cancellation and credential-file projection were repaired; those repaired paths remain present.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-d915e4a-20260918t084558z",
            "finding\u005fid": "P19-AUD-007",
            "source\u005fid": "C18"
          },
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-e079780-20260926t093335z",
            "finding\u005fid": "P19-AUD-001",
            "source\u005fid": "C26"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "missed-branch",
        "repair\u005fcommits": [
          "2e2bd806d25e95a722b72ca7f4482753cb6d048b",
          "a7961baeea280a50df9c0ad86a8bed9d56078be0"
        ],
        "verification": "Echo a dummy selected credential through both agents' stdout/stderr; default seal/export must reject it without requiring an explicitly supplied canary. Retain cancellation and credential-file cleanup coverage."
      },
      {
        "cause": "Replacement retains all original trials and refreshes only the target group, while dispatch executes the retained groups again. assembled\u005frun.rs:149-190 explicitly expects unaffected IDs to remain unchanged, encoding an expectation narrower than fresh identity for each new execution.",
        "current\u005fevidence": "Replacing one group in a multi-group experiment dispatches the complete retained trial list, while only target-group trial IDs are refreshed. Every unrelated group is therefore executed and sealed again with its old trial ID, violating the fresh-identity rule for new real execution. Same-scope replacement is allowed; every rerun must receive a fresh trial identity. Frozen evidence locations: crates/opi-eval/src/runner/experiment.rs:3952; crates/opi-eval/src/runner/experiment.rs:767; crates/opi-eval/tests/assembled\u005frun.rs:153; crates/opi-eval/src/bundle/mod.rs:738",
        "current\u005ffinding\u005fid": "P19-AUD-002",
        "prior\u005fclaim": "Frozen selection and fixed .r2 replacement identity defects were repaired.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-d915e4a-20260918t084558z",
            "finding\u005fid": "P19-AUD-006",
            "source\u005fid": "C18"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "missed-branch",
        "repair\u005fcommits": [
          "2e2bd806d25e95a722b72ca7f4482753cb6d048b"
        ],
        "verification": "Cover a multi-group replacement: either do not reexecute untouched trials or assign new identities to every new execution. Verify old sealed attempts remain immutable and original selection remains frozen."
      },
      {
        "cause": "prepare.rs records interpreter/entrypoint hashes, but RunnerProject material consumers bind paths/source identity without rechecking interpreter bytes. This separate gap dates to 55fbc95; path-string, lockfile and missing-entrypoint tests do not cover byte replacement at the same path.",
        "current\u005fevidence": "prepare records runner interpreter/console entrypoint hashes in inventory.json but material/5 carries only runner source digest and path. validate/plan/run do not read inventory. Replacing the selected interpreter bytes after preparation is accepted with unchanged native\u005fintegrity and can enter a new frozen plan, so verified grader execution identity does not bind the bytes selected by UV\u005fPYTHON. Frozen evidence locations: crates/opi-eval/src/runner/prepare.rs:1199; crates/opi-eval/src/runner/material.rs:165; crates/opi-eval/src/runner/material.rs:1293; crates/opi-eval/examples/local-evaluation/README.md:74",
        "current\u005ffinding\u005fid": "P19-AUD-003",
        "prior\u005fclaim": "Earlier acquisition failures were repaired. No matching prior finding for post-prepare interpreter-byte substitution was found within the compared reports.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-d915e4a-20260918t084558z",
            "finding\u005fid": "P19-AUD-001",
            "source\u005fid": "C18"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [
          "2e2bd806d25e95a722b72ca7f4482753cb6d048b"
        ],
        "verification": "After prepare succeeds, change interpreter bytes in place while retaining path/source/lockfile claims. validate, plan and run must reject before effects; preserve successful real acquisition."
      },
      {
        "cause": "benchmark/process.rs:1049-1115 projects observed task paths into lifecycle facts; the later comparison around line 1636 does not restore package identity. A different package with the same lifecycle is accepted. This dates to 55fbc95; it does not prove upstream normally selects the wrong package.",
        "current\u005fevidence": "A completed native result can change config.task.path to another absolute package with the same lifecycle (even a different declared task name or grader bytes), while retaining the selected result.task\u005fname, and still pass the native result importer and post-settlement configuration comparison. The path is not compared with the frozen dispatched package and its contents are not admitted against that package. The runner then seals the contradictory config as selected-trial evidence. This claim concerns rejection of substituted observed evidence, not a claim that the pinned upstream runner normally dispatches the wrong task. Frozen evidence locations: crates/opi-eval/src/benchmark/process.rs:1049; crates/opi-eval/src/benchmark/process.rs:794; crates/opi-eval/src/benchmark/process.rs:1636; crates/opi-eval/src/runner/experiment.rs:2319",
        "current\u005ffinding\u005fid": "P19-AUD-004",
        "prior\u005fclaim": "No exact earlier finding for observed task-package substitution was identified in C18/P18/C26/P26.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Substitute an observed native task package while preserving lifecycle/configuration facts. Reject package mismatch and retain the correctly bound native result positive."
      },
      {
        "cause": "verify-local-acceptance.py compares provenance to a co-supplied cache and permits empty inventories. Tests mutate only one side; coordinated source/cache substitution escapes them. The cache/list checks date to 7647312 and are not validated by downstream bundle verify/report.",
        "current\u005fevidence": "The offline acceptance verifier admits arbitrary upstream repository/commit pairs when the same pairs are added to the supplied identity-cache JSON. It does not bind those records to admitted external inventories. Empty material and preserved-artifact lists also pass their checks. Thus acceptance can affirm provenance without proving the required pinned runner/source/material identities. Frozen evidence locations: crates/opi-eval/scripts/verify-local-acceptance.py:220; crates/opi-eval/scripts/verify-local-acceptance.py:711; crates/opi-eval/scripts/verify-local-acceptance.py:758",
        "current\u005ffinding\u005fid": "P19-AUD-005",
        "prior\u005fclaim": "The old fixture-substitution acceptance path was rejected after repair and remains rejected.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-d915e4a-20260918t084558z",
            "finding\u005fid": "P19-AUD-008",
            "source\u005fid": "C18"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "missed-branch",
        "repair\u005fcommits": [
          "2e2bd806d25e95a722b72ca7f4482753cb6d048b"
        ],
        "verification": "Reject coordinated changes to source claims and supplied cache; bind to an independently established provenance anchor and require material/preserved inventory contents. Retain fixture-substitution refusal."
      },
      {
        "cause": "experiment.rs:1017-1057 enumerates sampling controls but omits a supplied required reasoning value. Native Pi launch forces --thinking off while projection initially calls reasoning effective. Missing-value tests never exercise supplied-but-ineffective reasoning.",
        "current\u005fevidence": "A native experiment with reasoning=\"high\" and required=[\"reasoning\"] is admitted before effects; Pi is launched with --thinking off, and reasoning is only declared unsupported during post-execution pairing. Required unsupported controls therefore do not fail before native execution. Frozen evidence locations: crates/opi-eval/src/experiment.rs:770; crates/opi-eval/src/experiment.rs:1042; crates/opi-eval/src/runner/output\u005fcontrol.rs:39; crates/opi-eval/integrations/harbor/opi\u005feval\u005fharbor.py:810; crates/opi-eval/src/runner/experiment.rs:2568",
        "current\u005ffinding\u005fid": "P19-AUD-006",
        "prior\u005fclaim": "Required unsupported temperature/top\u005fp admission was repaired and still rejects correctly. The product default-thinking change in 99ca6e3 concerns a different layer.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-d915e4a-20260918t084558z",
            "finding\u005fid": "P19-AUD-003",
            "source\u005fid": "C18"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "missed-branch",
        "repair\u005fcommits": [
          "2e2bd806d25e95a722b72ca7f4482753cb6d048b"
        ],
        "verification": "Require reasoning=high with actual Pi --thinking off; reject before execution or implement effective support with independently observed proof. Preserve required sampling rejection and distinguish product defaults from companion capability admission."
      },
      {
        "cause": "Plan production resolves relative material from cwd; plan consumption resolves it from the plan directory. Both paths date to afa448d. The existing different-cwd test passes an absolute material path and therefore misses this combination.",
        "current\u005fevidence": "plan saves relative --native-material as provided, but run --plan resolves it against the plan directory. Nested output paths produce accepted plans that fail to load unchanged material. Frozen evidence locations: crates/opi-eval/src/runner/plan.rs:540; crates/opi-eval/src/cli/run.rs:96",
        "current\u005ffinding\u005fid": "P19-AUD-007",
        "prior\u005fclaim": "No exact earlier finding for relative material path plus nested plan output was identified.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Plan using a relative material path into a nested output directory, then run from the supported cwd combinations; the same material must resolve and bind. Keep the absolute-path case."
      },
      {
        "cause": "The producer-policy convergence requirement remains unclosed: CI rewrites tasks/images/profiles and emits material under a different runner-project admission shape. Shared consumers do not establish a shared material producer. This is a persistent requirement gap, not an exact earlier finding reappearing.",
        "current\u005fevidence": "CI native-smoke materialize-configs independently copies/rewrites tasks, resolves image references, generates profiles and verifier wrappers and emits material/5 without the local RunnerProject contract. This is the second materialization policy forbidden by Section 8 and task19.10's replacement ceiling; allowed fixture/provider/witness orchestration is not the finding. Frozen evidence locations: .github/workflows/opi-eval-native-smoke.yml:172; crates/opi-eval/scripts/native-smoke.sh:893; crates/opi-eval/scripts/native-smoke.sh:1035; crates/opi-eval/src/runner/material.rs:351",
        "current\u005ffinding\u005fid": "P19-AUD-008",
        "prior\u005fclaim": "C26 requirements P19-011-ci-policy and related rows were marked met using shared downstream validation/execution evidence.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "persistent",
        "repair\u005fcommits": [],
        "verification": "Trace local and CI material producers through the same admission contract and exercise equivalent invalid inputs. Verify producer equivalence, not merely downstream stage presence."
      },
      {
        "cause": "The local walkthrough omits required --native-material and README ownership passages remain stale. The stale commands/ownership statements date to September 8/5. General doc checks neither execute the complete walkthrough nor resolve conflicting ownership statements.",
        "current\u005fevidence": "The packaged local walkthrough's plan command supplies neither native material nor fixtures and fails in every supported execution mode. Main bilingual README sections also retain CI-owned materialization/regeneration directions despite the independent prepare entry. Frozen evidence locations: crates/opi-eval/examples/local-evaluation/README.md:155; crates/opi-eval/src/runner/plan.rs:550; crates/opi-eval/README.md:530",
        "current\u005ffinding\u005fid": "P19-AUD-009",
        "prior\u005fclaim": "C26 documentation/contract rows were marked met after general documentation checks. No exact earlier finding for this broken walkthrough was identified.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Update English/Chinese documentation together; exercise the documented local plan/run sequence with prepared native material, and check all ownership statements against the owning workflow."
      },
      {
        "cause": "No recurrence of those exact failures was reproduced by historical inspection. Current audit did not independently admit/apply these records. B6 used Opi source 5bee4f28738412c6ae3e326e5bb3cf60e57230c2; later product changes mean matching raw hashes do not establish the current exact client matrix. opi-eval production/integration/native-driver code is unchanged from a7961ba to audit\u005fhead.",
        "current\u005fevidence": "This fresh audit has no executed current exact-client matrix or actual Docker/shared-verifier credential cleanup witness. Fake-environment and stand-in tests cannot establish the mandatory real-client wire/tool loops, observed service-model identity or native grader/resource cleanup claims. This is an evidence limitation, not a reproduced defect in those native paths. Frozen evidence locations: crates/opi-eval/tests/native\u005fdriver.rs:3870; crates/opi-eval/tests/native\u005fdriver.rs:9556; crates/opi-eval/scripts/test\u005fnative\u005fintegrations.py:313; crates/opi-eval/src/runner/plan.rs:668",
        "current\u005ffinding\u005fid": "P19-AUD-010",
        "prior\u005fclaim": "R27 B6 actual-client public run/seal/report and B10 owned-container cleanup witnesses each executed one passing test; original logs survive and their recorded digests matched.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-e079780-20260926t093335z",
            "finding\u005fid": "P19-AUD-007",
            "source\u005fid": "C26"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Original log bytes match the digest recorded in historical remediation; earlier comparison inspected PASS and actual executed counts. Not rerun in this supplement.",
            "limitations": "Check applicability to remediation\u005fhead and exact build/client/material identities. Historical PASS is not current closure.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-apply-djm9yx8l/b6-actual-green.log",
            "sha256": "4dfb1bd0213189eba66567d08409e97eeb43bbda910e42aa9cddc09cae83f8a7"
          },
          {
            "checked": "Original command receipt is retained; preceding comparison checked exit 0. Packaged with its current exact byte digest.",
            "limitations": "Read alongside the named log; inspect command, environment/build identities and counts rather than treating exit 0 alone as behavioral proof.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-apply-djm9yx8l/b6-actual-green.json",
            "sha256": "6a2a11e5b71b1825fe0284aa09adc025cb1c8dc87f921dffb2d611a754977e56"
          },
          {
            "checked": "Original log bytes match the digest recorded in historical remediation; earlier comparison inspected PASS and actual executed counts. Not rerun in this supplement.",
            "limitations": "Check applicability to remediation\u005fhead and exact build/client/material identities. Historical PASS is not current closure.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-apply-djm9yx8l/b10-actual-green.log",
            "sha256": "a4253c00432d6d2de139f979995f12d22ea5efab603a87c6d7d6fd47bbdbd148"
          },
          {
            "checked": "Original command receipt is retained; preceding comparison checked exit 0. Packaged with its current exact byte digest.",
            "limitations": "Read alongside the named log; inspect command, environment/build identities and counts rather than treating exit 0 alone as behavioral proof.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-apply-djm9yx8l/b10-actual-green.json",
            "sha256": "d6b6929c0699965234f8f3e5e277cca0d2663a44a39a0e1738e9493fe0ef76e2"
          }
        ],
        "relationship": "evidence-gap",
        "repair\u005fcommits": [
          "a7961baeea280a50df9c0ad86a8bed9d56078be0"
        ],
        "verification": "Recheck raw log and command JSON identities/counts, exact client/build/material versions, public run-to-report cases and cleanup predicates. Prove unaffected dependency surfaces or rerun only invalidated checks within existing authorization; keep current finding pending until independently resolved."
      },
      {
        "cause": "The diagnostic helper creates task names, an answer.txt grader and a replacement corpus subject. This proves a mechanism but does not by itself prove regrading saved artifacts under the original pinned benchmark graders. Historical green is narrower than the current requirement.",
        "current\u005fevidence": "Current native regrade evidence surfaces do not establish fresh offline grading of preserved pinned benchmark artifacts in all three families. The named native\u005foffline\u005fregrade test is a stand-in; the ignored Docker verifier witness replaces benchmark tasks and graders with private diagnostic answer.txt graders. No pinned benchmark-native regrade execution/artifacts were supplied to this audit subtask, so this mandatory dynamic requirement is not assessable. Frozen evidence locations: crates/opi-eval/tests/native\u005fdriver.rs:7621; crates/opi-eval/tests/native\u005fdriver.rs:10756; crates/opi-eval/tests/support/native\u005fwitness\u005fmaterial.rs:45; crates/opi-eval/tests/support/native\u005fwitness\u005fmaterial.rs:199; crates/opi-eval/tests/support/native\u005fwitness\u005fmaterial.rs:269",
        "current\u005ffinding\u005fid": "P19-AUD-011",
        "prior\u005fclaim": "Pi-only regrade selection was repaired. R27 B9's native verifier-only diagnostic executed one passing test.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-e079780-20260926t093335z",
            "finding\u005fid": "P19-AUD-004",
            "source\u005fid": "C26"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Original log bytes match the digest recorded in historical remediation; earlier comparison inspected PASS and actual executed counts. Not rerun in this supplement.",
            "limitations": "Check applicability to remediation\u005fhead and exact build/client/material identities. Historical PASS is not current closure.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-apply-djm9yx8l/b9-regrade-witness-current.log",
            "sha256": "0d7bd7e0e752c3410adecf33e9e39eec11e5f26ecd36f6ac8d896d1438201fa2"
          },
          {
            "checked": "Original command receipt is retained; preceding comparison checked exit 0. Packaged with its current exact byte digest.",
            "limitations": "Read alongside the named log; inspect command, environment/build identities and counts rather than treating exit 0 alone as behavioral proof.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-apply-djm9yx8l/b9-regrade-witness-current.json",
            "sha256": "41e1e43b3a18c36af09ea9e2609ff6ff77d25dbd236ae9f27e0c70262cfb29ae"
          }
        ],
        "relationship": "evidence-gap",
        "repair\u005fcommits": [
          "a7961baeea280a50df9c0ad86a8bed9d56078be0"
        ],
        "verification": "Use saved original task artifacts and original pinned graders for independent verifier-only regrade evidence. Require no Agent/provider execution, unchanged source evidence and fresh derived records; retain the repaired Pi-only selection case."
      },
      {
        "cause": "Original official witnesses exist: TB2.1 openssl-selfsigned-cert, TB3 batched-eval-parity, DeepSWE abs-module-cache-flags. TB3 collected artifacts and DeepSWE patches differ as expected. Integration/import/cleanup changed afterward, so source/material applicability still needs independent admission; lack of current admission is not absence of historical execution.",
        "current\u005fevidence": "The code routes all three families through selected native integrations and has opt-in oracle/nop witness tests, but their pinned external material, local Docker execution and retained results were not supplied or executed within this audit. The audit therefore cannot mark actual shared-environment, separate artifact collection, or pristine committed-patch grading and their positive/negative witnesses met. This is an audit evidence limitation, not a claim that the implementations fail when correctly provisioned. Frozen evidence locations: crates/opi-eval/tests/native\u005fdriver.rs:3485; crates/opi-eval/tests/native\u005fdriver.rs:3507; crates/opi-eval/tests/native\u005fdriver.rs:3551; crates/opi-eval/scripts/test\u005fnative\u005fintegrations.py:1",
        "current\u005ffinding\u005fid": "P19-AUD-012",
        "prior\u005fclaim": "R26 B2 retained six official oracle/nop result files. All six hashes matched the receipt in the earlier September 28 comparison; each family rewards 1/0 with exception\u005finfo=null.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [
          {
            "checked": "Earlier comparison verified receipt hash and all six referenced native result hashes, original task identities, rewards and null exception\u005finfo.",
            "limitations": "Receipt includes a historical Closed assertion; only original evidence may be reverified for current applicability.",
            "path": "D:/Luiz/Odradek/opi-remediation-evidence/phase19-apply-20260926-bd37/b2-closure.json",
            "sha256": "f2867c01b63c31fa3589d256fee7a9fb0962e587e7f4ab4242311b2c0dc78230"
          },
          {
            "checked": "Earlier comparison in this conversation located and hashed this original terminal-bench-2.1 oracle result; reward=1, exception\u005finfo=null. Not re-executed.",
            "limitations": "Resolve this absolute path in the same WSL environment. Recheck source/material applicability and exact task identity; do not substitute a diagnostic task or infer current closure from reward alone.",
            "path": "WSL:/home/luiz/.local/state/opi-remediation-evidence/phase19-apply-20260926-bd37/retained-official-witnesses/opi-eval-terminal-bench-2.1-oracle-witness-MJdE5s/benchmark/trace/jobs/2026-09-26\u005f\u005f01-53-28/task-package\u005f\u005foLJyVvp/result.json",
            "sha256": "e68306e8b33ae31f6e70f3dfd17963c1477061238f09b5fd905d621b29ff3375"
          },
          {
            "checked": "Earlier comparison in this conversation located and hashed this original terminal-bench-2.1 nop result; reward=0, exception\u005finfo=null. Not re-executed.",
            "limitations": "Resolve this absolute path in the same WSL environment. Recheck source/material applicability and exact task identity; do not substitute a diagnostic task or infer current closure from reward alone.",
            "path": "WSL:/home/luiz/.local/state/opi-remediation-evidence/phase19-apply-20260926-bd37/retained-official-witnesses/opi-eval-terminal-bench-2.1-nop-witness-Dvv2Jg/benchmark/trace/jobs/2026-09-26\u005f\u005f01-54-29/task-package\u005f\u005fdsn9mZV/result.json",
            "sha256": "1335405a379766d93595332e442f8618dc4f42bfe7cafa363e7e8f626144b03d"
          },
          {
            "checked": "Earlier comparison in this conversation located and hashed this original terminal-bench-3.0 oracle result; reward=1, exception\u005finfo=null. Not re-executed.",
            "limitations": "Resolve this absolute path in the same WSL environment. Recheck source/material applicability and exact task identity; do not substitute a diagnostic task or infer current closure from reward alone.",
            "path": "WSL:/home/luiz/.local/state/opi-remediation-evidence/phase19-apply-20260926-bd37/retained-official-witnesses/opi-eval-terminal-bench-3.0-oracle-witness-YhVuml/benchmark/trace/jobs/2026-09-26\u005f\u005f01-55-43/task-package\u005f\u005f22b5rY5/result.json",
            "sha256": "f722d18ef9e8db1e64b41d84b1b2e9154a770ffbdb4e3e9cfec7cf67dcdf79d3"
          },
          {
            "checked": "Earlier comparison in this conversation located and hashed this original terminal-bench-3.0 nop result; reward=0, exception\u005finfo=null. Not re-executed.",
            "limitations": "Resolve this absolute path in the same WSL environment. Recheck source/material applicability and exact task identity; do not substitute a diagnostic task or infer current closure from reward alone.",
            "path": "WSL:/home/luiz/.local/state/opi-remediation-evidence/phase19-apply-20260926-bd37/retained-official-witnesses/opi-eval-terminal-bench-3.0-nop-witness-fZPdoB/benchmark/trace/jobs/2026-09-26\u005f\u005f01-57-43/task-package\u005f\u005fJL4TS8S/result.json",
            "sha256": "0e7c18038da13f17615383c6ee95ac2bb957a88d939539db25d62bb6cd159fda"
          },
          {
            "checked": "Earlier comparison in this conversation located and hashed this original deepswe-v1.1 oracle result; reward=1, exception\u005finfo=null. Not re-executed.",
            "limitations": "Resolve this absolute path in the same WSL environment. Recheck source/material applicability and exact task identity; do not substitute a diagnostic task or infer current closure from reward alone.",
            "path": "WSL:/home/luiz/.local/state/opi-remediation-evidence/phase19-apply-20260926-bd37/retained-official-witnesses/opi-eval-deepswe-oracle-witness-xv0sjJ/benchmark/trace/jobs/2026-09-26\u005f\u005f01-58-38/task-package\u005f\u005fS5kUP3V/result.json",
            "sha256": "0d1fe37f6fc4bcf1661367716d11bf9c00c22b21220da459fad3c082f23211d8"
          },
          {
            "checked": "Earlier comparison in this conversation located and hashed this original deepswe-v1.1 nop result; reward=0, exception\u005finfo=null. Not re-executed.",
            "limitations": "Resolve this absolute path in the same WSL environment. Recheck source/material applicability and exact task identity; do not substitute a diagnostic task or infer current closure from reward alone.",
            "path": "WSL:/home/luiz/.local/state/opi-remediation-evidence/phase19-apply-20260926-bd37/retained-official-witnesses/opi-eval-deepswe-nop-witness-xia1FI/benchmark/trace/jobs/2026-09-26\u005f\u005f01-59-19/task-package\u005f\u005fhrzzhfj/result.json",
            "sha256": "9ca090a0e3d9341f0ecbe1bb64be9ba235bddde0b75e2a46acafdf744bbc404f"
          }
        ],
        "relationship": "evidence-gap",
        "repair\u005fcommits": [
          "265078fd04442bc317a79175614064e3e243a7ae"
        ],
        "verification": "Read each exact native result and the receipt below; verify task/material/build/source identity and the positive/negative predicate. Compare changes after their bound state, then rerun only invalidated witnesses. Do not substitute generated diagnostic graders or close solely from recorded rewards."
      },
      {
        "cause": "The six trials were natively valid (TB2.1 1/1; TB3 and DeepSWE 0/0 after four Agent timeouts) but formal acceptance was blocked/not run. Reward zero alone is not the disqualifier: timed-out rows lack healthy terminal and complete assistant/tool/AgentEnd evidence. The old 1 MiB cap was fixed; later empty stdout came from JSON buffering, fixed by 3e8fee6. That cannot restore missing old events. The companion build-source identity remains unknown.",
        "current\u005fevidence": "No raw redacted six-trial operator evidence root was supplied to this fresh audit. Committed deterministic fixtures and an acceptance verifier do not establish the mandatory assembled local Zhipu/Opi/pi runs. The full workspace Phase-exit verification union was not rerun. These requirements remain not-assessable; this does not claim that external runs never occurred. Frozen evidence locations: docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md:266; crates/opi-eval/scripts/test\u005fverify\u005flocal\u005facceptance.py:115; crates/opi-eval/tests/native\u005fdriver.rs:3870",
        "current\u005ffinding\u005fid": "P19-AUD-013",
        "prior\u005fclaim": "R27 final workspace logs record 4542 passed, 0 failed, 16 ignored across 229 test binaries; clippy, doctests and rustdoc also passed. A later long run sealed six real Agent trials.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-e079780-20260926t093335z",
            "finding\u005fid": "P19-AUD-006",
            "source\u005fid": "C26"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Original log bytes match the digest recorded in historical remediation; earlier comparison inspected PASS and actual executed counts. Not rerun in this supplement.",
            "limitations": "Check applicability to remediation\u005fhead and exact build/client/material identities. Historical PASS is not current closure.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-apply-djm9yx8l/final-workspace-all-targets.log",
            "sha256": "b61468b48b4c26e226cd7c7f2e18203649c77fab806d2826e42d37260b4e441f"
          },
          {
            "checked": "Original command receipt is retained; preceding comparison checked exit 0. Packaged with its current exact byte digest.",
            "limitations": "Read alongside the named log; inspect command, environment/build identities and counts rather than treating exit 0 alone as behavioral proof.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-apply-djm9yx8l/final-workspace-all-targets.json",
            "sha256": "2e5a1a7d3074f70e32bfd324b4486357e610ba12cf7fa149d984d3645ca55f1e"
          },
          {
            "checked": "Original log bytes match the digest recorded in historical remediation; earlier comparison inspected PASS and actual executed counts. Not rerun in this supplement.",
            "limitations": "Check applicability to remediation\u005fhead and exact build/client/material identities. Historical PASS is not current closure.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-apply-djm9yx8l/final-workspace-clippy.log",
            "sha256": "43b2302ee9ce41cbbaf37d5c3a5e7bb9697021d69add334847789d6e23ea7dab"
          },
          {
            "checked": "Original command receipt is retained; preceding comparison checked exit 0. Packaged with its current exact byte digest.",
            "limitations": "Read alongside the named log; inspect command, environment/build identities and counts rather than treating exit 0 alone as behavioral proof.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-apply-djm9yx8l/final-workspace-clippy.json",
            "sha256": "b22734baf8306a30e4b58ceb9199c19c64c5e9748da2074b6c0a3bb416d252e6"
          },
          {
            "checked": "Original log bytes match the digest recorded in historical remediation; earlier comparison inspected PASS and actual executed counts. Not rerun in this supplement.",
            "limitations": "Check applicability to remediation\u005fhead and exact build/client/material identities. Historical PASS is not current closure.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-apply-djm9yx8l/final-workspace-doctests.log",
            "sha256": "8ccb9a9673fba1bfd7ad90b3cc7a6a0248ab9e82abee628cf09dac94f95263be"
          },
          {
            "checked": "Original command receipt is retained; preceding comparison checked exit 0. Packaged with its current exact byte digest.",
            "limitations": "Read alongside the named log; inspect command, environment/build identities and counts rather than treating exit 0 alone as behavioral proof.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-apply-djm9yx8l/final-workspace-doctests.json",
            "sha256": "6b3377579017504281a28f058f78e2ecf279a415716e3ba86749943f0db84208"
          },
          {
            "checked": "Original log bytes match the digest recorded in historical remediation; earlier comparison inspected PASS and actual executed counts. Not rerun in this supplement.",
            "limitations": "Check applicability to remediation\u005fhead and exact build/client/material identities. Historical PASS is not current closure.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-apply-djm9yx8l/final-workspace-rustdoc.log",
            "sha256": "f6352de3e77291ee9ff16d8bb639622165d93316658958d0b2e200f36be04e5e"
          },
          {
            "checked": "Original command receipt is retained; preceding comparison checked exit 0. Packaged with its current exact byte digest.",
            "limitations": "Read alongside the named log; inspect command, environment/build identities and counts rather than treating exit 0 alone as behavioral proof.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-apply-djm9yx8l/final-workspace-rustdoc.json",
            "sha256": "a4e05146bdfea016f7f4c91a514967ed7d55eba9619227808a85a920729bd1c7"
          },
          {
            "checked": "Earlier comparison inspected six sealed native-valid trials, four Agent timeouts, and formal acceptance blocked/not run.",
            "limitations": "No accepted manifest; original Companion build-source identity unknown.",
            "path": "E:/opi/evaluations/phase19-longrun-validation-20260927/live-final-summary.json",
            "sha256": "84ccf59a524b30eeb96abde0049765a491d069d8ebb38139b31cd7b3bc61d666"
          },
          {
            "checked": "Earlier comparison checked explicit strict-acceptance precheck blockers.",
            "limitations": "This is a precheck record, not a passed acceptance run; preserve required predicates.",
            "path": "E:/opi/evaluations/phase19-longrun-validation-20260927/live-reports/linux-v2/offline-final-audit/acceptance-precheck.json",
            "sha256": "17949233e8c059b71ebdbd56e8a5a36a44939435975d8e60c2de394b4702b1f0"
          }
        ],
        "relationship": "evidence-gap",
        "repair\u005fcommits": [
          "a7961baeea280a50df9c0ad86a8bed9d56078be0",
          "3e8fee6075c719ca0c3192790b800c13421e4fc4"
        ],
        "verification": "Establish final-source gate applicability and formal six-trial acceptance without weakening predicates. Preserve failed trials; obtain required event/terminal evidence under authorized execution, resolve companion source identity, and rerun only invalidated gates. Ledger selector corrections belong to their owner and do not add a current finding."
      }
    ],
    "limitations": "Compiled after independent installation on 2026-09-28; operator-declared codex/gpt6. No runtime/native/paid-provider jobs were rerun for this supplement. Historical proof checks recorded below came from the preceding history comparison; current raw-byte availability was additionally checked for the local paths while packaging. Old closure claims are attributed, not adopted. No identified prior repair was shown to have been reverted. Findings 001-009 are implementation/documentation issues; 010-013 are current evidence limitations. This appendix cannot change the sealed core, severity, requirement state, verdict or authorize repair/live execution. External temporary/WSL evidence may later be unavailable; such loss is a limitation. P26-003 ledger-selector corrections remain with the owning workflow and outside the 13 current finding keys.",
    "schema": "opi-audit-history/1",
    "scope": "Same Phase 19 C18/P18/C26/P26 reports, September 19/26/27 remediation results, and the September 28 long-run handoff, all pinned to commits available at audit\u005fhead 3e8fee6075c719ca0c3192790b800c13421e4fc4. Current behavior is the frozen C28 audit evidence. No concurrent sibling draft was used.",
    "sources": [
      {
        "audit\u005frun\u005fid": "phase19-codex-gpt6-d915e4a-20260918t084558z",
        "commit": "3e8fee6075c719ca0c3192790b800c13421e4fc4",
        "id": "C18",
        "path": "docs/snapshots/phase19/assurance/history/phase19-codex-gpt6-d915e4a-20260918t084558z/audit.codex.gpt6.md",
        "sha256": "621db86e28d9598adeb711805323faad105597c30cd10798fc31c2ac45f25117"
      },
      {
        "audit\u005frun\u005fid": "phase19-pi-glm53-ad67cb8-20260918t101920z",
        "commit": "3e8fee6075c719ca0c3192790b800c13421e4fc4",
        "id": "P18",
        "path": "docs/snapshots/phase19/assurance/history/phase19-pi-glm53-ad67cb8-20260918t101920z/audit.pi.glm53.md",
        "sha256": "41ef7c67007fe6c1a304aff1995f8e66936edd56fb27e49597cbd87830e7a003"
      },
      {
        "audit\u005frun\u005fid": "phase19-codex-gpt6-e079780-20260926t093335z",
        "commit": "3e8fee6075c719ca0c3192790b800c13421e4fc4",
        "id": "C26",
        "path": "docs/snapshots/phase19/assurance/audit.codex.gpt6.md",
        "sha256": "f2675a7e35b7a7c8950b02778f5a5bd8ef0fc7a72471ff61203bc68f42de8ec3"
      },
      {
        "audit\u005frun\u005fid": "phase19-codex-gpt6-e079780-20260926t093335z",
        "commit": "3e8fee6075c719ca0c3192790b800c13421e4fc4",
        "id": "C26-requirements",
        "path": "docs/snapshots/phase19/assurance/audit.codex.gpt6.requirements.jsonl",
        "sha256": "50f4438f43331ae2e7a5965094c0f97997ffa520d44d192b59ef8970247f50da"
      },
      {
        "audit\u005frun\u005fid": "phase19-pi-glm53-e079780-20260926t192024z",
        "commit": "3e8fee6075c719ca0c3192790b800c13421e4fc4",
        "id": "P26",
        "path": "docs/snapshots/phase19/assurance/audit.pi.glm53.md",
        "sha256": "da4843d72f239fe5e55f000f07075164c51ae91a5d3e4cb05184be2f0dd94673"
      },
      {
        "audit\u005frun\u005fid": null,
        "commit": "2e2bd806d25e95a722b72ca7f4482753cb6d048b",
        "id": "R19",
        "path": "docs/snapshots/phase19/assurance/remediation.result.md",
        "sha256": "5037a08fc7b59d663be49c04576ccc5c116fd1748ac011a6c1957b44db1ce6d5"
      },
      {
        "audit\u005frun\u005fid": null,
        "commit": "265078fd04442bc317a79175614064e3e243a7ae",
        "id": "R26",
        "path": "docs/snapshots/phase19/assurance/remediation.result.md",
        "sha256": "d1a0e9717af9786c8a22e543ee21ca4882b1057173a351c9178e54130ad668e8"
      },
      {
        "audit\u005frun\u005fid": null,
        "commit": "a7961baeea280a50df9c0ad86a8bed9d56078be0",
        "id": "R27",
        "path": "docs/snapshots/phase19/assurance/remediation.result.md",
        "sha256": "211216530de1b4617f025d9f2e0334948aa6cd9470793ee2ad7df5aa900671e2"
      },
      {
        "audit\u005frun\u005fid": null,
        "commit": "14d07894c9c8cd74c8b4df71e9fef2b50b78bdc6",
        "id": "longrun",
        "path": "docs/eval/20260927t195503z-opi082-pi0851-glm53flash-longrun.md",
        "sha256": "df59a3d983fe0c06b94014c24ce2edc106825916456974e330977825d8bd4541"
      }
    ],
    "status": "compared"
  },
  "core": {
    "findings\u005fsha256": "ccf574271b308b2bb72e13df1721824a9bda153e08b813e0679320b65ceb4ca1",
    "meta\u005fsha256": "0430d9f1b6617b40d347c61f4d41bfac7b738116ec5a4f864a71b35e8da3c03e",
    "report\u005fbytes": 83811,
    "report\u005fsha256": "510b04d7debb0f8fbd72fd249a3e4bd9a3f327598f50dd614c3c4b7b5bcd85fe",
    "requirements\u005fsha256": "b435819039d28b3e97efcacad0c3b7e2a21a12c0dbe69749fb56216fcec351e7"
  },
  "schema": "opi-audit-history/1"
}
-->

## Post-audit historical reference — advisory

This section was appended after installation of the independent core. It cannot change findings, requirements, independence or verdict. Historical closure claims require current independent verification.

- Owning audit run: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z
- Audit head: 3e8fee6075c719ca0c3192790b800c13421e4fc4
- Core report bytes: 83811
- Core report SHA-256: 510b04d7debb0f8fbd72fd249a3e4bd9a3f327598f50dd614c3c4b7b5bcd85fe
- Current findings SHA-256: ccf574271b308b2bb72e13df1721824a9bda153e08b813e0679320b65ceb4ca1
- Comparison status: compared
- Inspected history: Same Phase 19 C18/P18/C26/P26 reports, September 19/26/27 remediation results, and the September 28 long&#45;run handoff, all pinned to commits available at audit&#95;head 3e8fee6075c719ca0c3192790b800c13421e4fc4&#46; Current behavior is the frozen C28 audit evidence&#46; No concurrent sibling draft was used&#46;
- Limitations: Compiled after independent installation on 2026&#45;09&#45;28; operator&#45;declared codex/gpt6&#46; No runtime/native/paid&#45;provider jobs were rerun for this supplement&#46; Historical proof checks recorded below came from the preceding history comparison; current raw&#45;byte availability was additionally checked for the local paths while packaging&#46; Old closure claims are attributed, not adopted&#46; No identified prior repair was shown to have been reverted&#46; Findings 001&#45;009 are implementation/documentation issues; 010&#45;013 are current evidence limitations&#46; This appendix cannot change the sealed core, severity, requirement state, verdict or authorize repair/live execution&#46; External temporary/WSL evidence may later be unavailable; such loss is a limitation&#46; P26&#45;003 ledger&#45;selector corrections remain with the owning workflow and outside the 13 current finding keys&#46;

### Pinned historical sources

- C18: docs/snapshots/phase19/assurance/history/phase19&#45;codex&#45;gpt6&#45;d915e4a&#45;20260918t084558z/audit&#46;codex&#46;gpt6&#46;md; commit 3e8fee6075c719ca0c3192790b800c13421e4fc4; SHA-256 621db86e28d9598adeb711805323faad105597c30cd10798fc31c2ac45f25117; audit run phase19&#45;codex&#45;gpt6&#45;d915e4a&#45;20260918t084558z
- P18: docs/snapshots/phase19/assurance/history/phase19&#45;pi&#45;glm53&#45;ad67cb8&#45;20260918t101920z/audit&#46;pi&#46;glm53&#46;md; commit 3e8fee6075c719ca0c3192790b800c13421e4fc4; SHA-256 41ef7c67007fe6c1a304aff1995f8e66936edd56fb27e49597cbd87830e7a003; audit run phase19&#45;pi&#45;glm53&#45;ad67cb8&#45;20260918t101920z
- C26: docs/snapshots/phase19/assurance/audit&#46;codex&#46;gpt6&#46;md; commit 3e8fee6075c719ca0c3192790b800c13421e4fc4; SHA-256 f2675a7e35b7a7c8950b02778f5a5bd8ef0fc7a72471ff61203bc68f42de8ec3; audit run phase19&#45;codex&#45;gpt6&#45;e079780&#45;20260926t093335z
- C26&#45;requirements: docs/snapshots/phase19/assurance/audit&#46;codex&#46;gpt6&#46;requirements&#46;jsonl; commit 3e8fee6075c719ca0c3192790b800c13421e4fc4; SHA-256 50f4438f43331ae2e7a5965094c0f97997ffa520d44d192b59ef8970247f50da; audit run phase19&#45;codex&#45;gpt6&#45;e079780&#45;20260926t093335z
- P26: docs/snapshots/phase19/assurance/audit&#46;pi&#46;glm53&#46;md; commit 3e8fee6075c719ca0c3192790b800c13421e4fc4; SHA-256 da4843d72f239fe5e55f000f07075164c51ae91a5d3e4cb05184be2f0dd94673; audit run phase19&#45;pi&#45;glm53&#45;e079780&#45;20260926t192024z
- R19: docs/snapshots/phase19/assurance/remediation&#46;result&#46;md; commit 2e2bd806d25e95a722b72ca7f4482753cb6d048b; SHA-256 5037a08fc7b59d663be49c04576ccc5c116fd1748ac011a6c1957b44db1ce6d5; audit run not an audit report
- R26: docs/snapshots/phase19/assurance/remediation&#46;result&#46;md; commit 265078fd04442bc317a79175614064e3e243a7ae; SHA-256 d1a0e9717af9786c8a22e543ee21ca4882b1057173a351c9178e54130ad668e8; audit run not an audit report
- R27: docs/snapshots/phase19/assurance/remediation&#46;result&#46;md; commit a7961baeea280a50df9c0ad86a8bed9d56078be0; SHA-256 211216530de1b4617f025d9f2e0334948aa6cd9470793ee2ad7df5aa900671e2; audit run not an audit report
- longrun: docs/eval/20260927t195503z&#45;opi082&#45;pi0851&#45;glm53flash&#45;longrun&#46;md; commit 14d07894c9c8cd74c8b4df71e9fef2b50b78bdc6; SHA-256 df59a3d983fe0c06b94014c24ce2edc106825916456974e330977825d8bd4541; audit run not an audit report

### Comparison 1: P19&#45;AUD&#45;001

- Relationship: missed&#45;branch
- Prior claim (attributed): Earlier detected&#45;canary cancellation and credential&#45;file projection were repaired; those repaired paths remain present&#46;
- Current evidence: For an explicitly selected full capture run without &#45;&#45;canaries, a selected credential echoed in Agent/native output is copied byte&#45;for&#45;byte into an Exportable artifact and can be successfully sealed&#46; Temporary credential&#45;file cleanup does not prevent that exposure&#46; Frozen evidence locations: crates/opi&#45;eval/src/cli/run&#46;rs:224; crates/opi&#45;eval/src/runner/experiment&#46;rs:1867; crates/opi&#45;eval/src/runner/experiment&#46;rs:2281; crates/opi&#45;eval/src/runner/experiment&#46;rs:3496; crates/opi&#45;eval/src/runner/experiment&#46;rs:2530
- Cause / missed boundary: Capture/export does not automatically enroll selected credentials as canaries&#46; Existing tests use explicit canaries or check private&#45;file projection, so an echoed selected credential bypasses the default seal guard&#46;
- Suggested current verification: Echo a dummy selected credential through both agents&#39; stdout/stderr; default seal/export must reject it without requiring an explicitly supplied canary&#46; Retain cancellation and credential&#45;file cleanup coverage&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;d915e4a&#45;20260918t084558z / P19&#45;AUD&#45;007; source C18
- Prior finding: phase19&#45;codex&#45;gpt6&#45;e079780&#45;20260926t093335z / P19&#45;AUD&#45;001; source C26
- Repair commits: 2e2bd806d25e95a722b72ca7f4482753cb6d048b, a7961baeea280a50df9c0ad86a8bed9d56078be0

### Comparison 2: P19&#45;AUD&#45;002

- Relationship: missed&#45;branch
- Prior claim (attributed): Frozen selection and fixed &#46;r2 replacement identity defects were repaired&#46;
- Current evidence: Replacing one group in a multi&#45;group experiment dispatches the complete retained trial list, while only target&#45;group trial IDs are refreshed&#46; Every unrelated group is therefore executed and sealed again with its old trial ID, violating the fresh&#45;identity rule for new real execution&#46; Same&#45;scope replacement is allowed; every rerun must receive a fresh trial identity&#46; Frozen evidence locations: crates/opi&#45;eval/src/runner/experiment&#46;rs:3952; crates/opi&#45;eval/src/runner/experiment&#46;rs:767; crates/opi&#45;eval/tests/assembled&#95;run&#46;rs:153; crates/opi&#45;eval/src/bundle/mod&#46;rs:738
- Cause / missed boundary: Replacement retains all original trials and refreshes only the target group, while dispatch executes the retained groups again&#46; assembled&#95;run&#46;rs:149&#45;190 explicitly expects unaffected IDs to remain unchanged, encoding an expectation narrower than fresh identity for each new execution&#46;
- Suggested current verification: Cover a multi&#45;group replacement: either do not reexecute untouched trials or assign new identities to every new execution&#46; Verify old sealed attempts remain immutable and original selection remains frozen&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;d915e4a&#45;20260918t084558z / P19&#45;AUD&#45;006; source C18
- Repair commits: 2e2bd806d25e95a722b72ca7f4482753cb6d048b

### Comparison 3: P19&#45;AUD&#45;003

- Relationship: new&#45;omission
- Prior claim (attributed): Earlier acquisition failures were repaired&#46; No matching prior finding for post&#45;prepare interpreter&#45;byte substitution was found within the compared reports&#46;
- Current evidence: prepare records runner interpreter/console entrypoint hashes in inventory&#46;json but material/5 carries only runner source digest and path&#46; validate/plan/run do not read inventory&#46; Replacing the selected interpreter bytes after preparation is accepted with unchanged native&#95;integrity and can enter a new frozen plan, so verified grader execution identity does not bind the bytes selected by UV&#95;PYTHON&#46; Frozen evidence locations: crates/opi&#45;eval/src/runner/prepare&#46;rs:1199; crates/opi&#45;eval/src/runner/material&#46;rs:165; crates/opi&#45;eval/src/runner/material&#46;rs:1293; crates/opi&#45;eval/examples/local&#45;evaluation/README&#46;md:74
- Cause / missed boundary: prepare&#46;rs records interpreter/entrypoint hashes, but RunnerProject material consumers bind paths/source identity without rechecking interpreter bytes&#46; This separate gap dates to 55fbc95; path&#45;string, lockfile and missing&#45;entrypoint tests do not cover byte replacement at the same path&#46;
- Suggested current verification: After prepare succeeds, change interpreter bytes in place while retaining path/source/lockfile claims&#46; validate, plan and run must reject before effects; preserve successful real acquisition&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;d915e4a&#45;20260918t084558z / P19&#45;AUD&#45;001; source C18
- Repair commits: 2e2bd806d25e95a722b72ca7f4482753cb6d048b

### Comparison 4: P19&#45;AUD&#45;004

- Relationship: new&#45;omission
- Prior claim (attributed): No exact earlier finding for observed task&#45;package substitution was identified in C18/P18/C26/P26&#46;
- Current evidence: A completed native result can change config&#46;task&#46;path to another absolute package with the same lifecycle &#40;even a different declared task name or grader bytes&#41;, while retaining the selected result&#46;task&#95;name, and still pass the native result importer and post&#45;settlement configuration comparison&#46; The path is not compared with the frozen dispatched package and its contents are not admitted against that package&#46; The runner then seals the contradictory config as selected&#45;trial evidence&#46; This claim concerns rejection of substituted observed evidence, not a claim that the pinned upstream runner normally dispatches the wrong task&#46; Frozen evidence locations: crates/opi&#45;eval/src/benchmark/process&#46;rs:1049; crates/opi&#45;eval/src/benchmark/process&#46;rs:794; crates/opi&#45;eval/src/benchmark/process&#46;rs:1636; crates/opi&#45;eval/src/runner/experiment&#46;rs:2319
- Cause / missed boundary: benchmark/process&#46;rs:1049&#45;1115 projects observed task paths into lifecycle facts; the later comparison around line 1636 does not restore package identity&#46; A different package with the same lifecycle is accepted&#46; This dates to 55fbc95; it does not prove upstream normally selects the wrong package&#46;
- Suggested current verification: Substitute an observed native task package while preserving lifecycle/configuration facts&#46; Reject package mismatch and retain the correctly bound native result positive&#46;

### Comparison 5: P19&#45;AUD&#45;005

- Relationship: missed&#45;branch
- Prior claim (attributed): The old fixture&#45;substitution acceptance path was rejected after repair and remains rejected&#46;
- Current evidence: The offline acceptance verifier admits arbitrary upstream repository/commit pairs when the same pairs are added to the supplied identity&#45;cache JSON&#46; It does not bind those records to admitted external inventories&#46; Empty material and preserved&#45;artifact lists also pass their checks&#46; Thus acceptance can affirm provenance without proving the required pinned runner/source/material identities&#46; Frozen evidence locations: crates/opi&#45;eval/scripts/verify&#45;local&#45;acceptance&#46;py:220; crates/opi&#45;eval/scripts/verify&#45;local&#45;acceptance&#46;py:711; crates/opi&#45;eval/scripts/verify&#45;local&#45;acceptance&#46;py:758
- Cause / missed boundary: verify&#45;local&#45;acceptance&#46;py compares provenance to a co&#45;supplied cache and permits empty inventories&#46; Tests mutate only one side; coordinated source/cache substitution escapes them&#46; The cache/list checks date to 7647312 and are not validated by downstream bundle verify/report&#46;
- Suggested current verification: Reject coordinated changes to source claims and supplied cache; bind to an independently established provenance anchor and require material/preserved inventory contents&#46; Retain fixture&#45;substitution refusal&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;d915e4a&#45;20260918t084558z / P19&#45;AUD&#45;008; source C18
- Repair commits: 2e2bd806d25e95a722b72ca7f4482753cb6d048b

### Comparison 6: P19&#45;AUD&#45;006

- Relationship: missed&#45;branch
- Prior claim (attributed): Required unsupported temperature/top&#95;p admission was repaired and still rejects correctly&#46; The product default&#45;thinking change in 99ca6e3 concerns a different layer&#46;
- Current evidence: A native experiment with reasoning=&#34;high&#34; and required=&#91;&#34;reasoning&#34;&#93; is admitted before effects; Pi is launched with &#45;&#45;thinking off, and reasoning is only declared unsupported during post&#45;execution pairing&#46; Required unsupported controls therefore do not fail before native execution&#46; Frozen evidence locations: crates/opi&#45;eval/src/experiment&#46;rs:770; crates/opi&#45;eval/src/experiment&#46;rs:1042; crates/opi&#45;eval/src/runner/output&#95;control&#46;rs:39; crates/opi&#45;eval/integrations/harbor/opi&#95;eval&#95;harbor&#46;py:810; crates/opi&#45;eval/src/runner/experiment&#46;rs:2568
- Cause / missed boundary: experiment&#46;rs:1017&#45;1057 enumerates sampling controls but omits a supplied required reasoning value&#46; Native Pi launch forces &#45;&#45;thinking off while projection initially calls reasoning effective&#46; Missing&#45;value tests never exercise supplied&#45;but&#45;ineffective reasoning&#46;
- Suggested current verification: Require reasoning=high with actual Pi &#45;&#45;thinking off; reject before execution or implement effective support with independently observed proof&#46; Preserve required sampling rejection and distinguish product defaults from companion capability admission&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;d915e4a&#45;20260918t084558z / P19&#45;AUD&#45;003; source C18
- Repair commits: 2e2bd806d25e95a722b72ca7f4482753cb6d048b

### Comparison 7: P19&#45;AUD&#45;007

- Relationship: new&#45;omission
- Prior claim (attributed): No exact earlier finding for relative material path plus nested plan output was identified&#46;
- Current evidence: plan saves relative &#45;&#45;native&#45;material as provided, but run &#45;&#45;plan resolves it against the plan directory&#46; Nested output paths produce accepted plans that fail to load unchanged material&#46; Frozen evidence locations: crates/opi&#45;eval/src/runner/plan&#46;rs:540; crates/opi&#45;eval/src/cli/run&#46;rs:96
- Cause / missed boundary: Plan production resolves relative material from cwd; plan consumption resolves it from the plan directory&#46; Both paths date to afa448d&#46; The existing different&#45;cwd test passes an absolute material path and therefore misses this combination&#46;
- Suggested current verification: Plan using a relative material path into a nested output directory, then run from the supported cwd combinations; the same material must resolve and bind&#46; Keep the absolute&#45;path case&#46;

### Comparison 8: P19&#45;AUD&#45;008

- Relationship: persistent
- Prior claim (attributed): C26 requirements P19&#45;011&#45;ci&#45;policy and related rows were marked met using shared downstream validation/execution evidence&#46;
- Current evidence: CI native&#45;smoke materialize&#45;configs independently copies/rewrites tasks, resolves image references, generates profiles and verifier wrappers and emits material/5 without the local RunnerProject contract&#46; This is the second materialization policy forbidden by Section 8 and task19&#46;10&#39;s replacement ceiling; allowed fixture/provider/witness orchestration is not the finding&#46; Frozen evidence locations: &#46;github/workflows/opi&#45;eval&#45;native&#45;smoke&#46;yml:172; crates/opi&#45;eval/scripts/native&#45;smoke&#46;sh:893; crates/opi&#45;eval/scripts/native&#45;smoke&#46;sh:1035; crates/opi&#45;eval/src/runner/material&#46;rs:351
- Cause / missed boundary: The producer&#45;policy convergence requirement remains unclosed: CI rewrites tasks/images/profiles and emits material under a different runner&#45;project admission shape&#46; Shared consumers do not establish a shared material producer&#46; This is a persistent requirement gap, not an exact earlier finding reappearing&#46;
- Suggested current verification: Trace local and CI material producers through the same admission contract and exercise equivalent invalid inputs&#46; Verify producer equivalence, not merely downstream stage presence&#46;

### Comparison 9: P19&#45;AUD&#45;009

- Relationship: new&#45;omission
- Prior claim (attributed): C26 documentation/contract rows were marked met after general documentation checks&#46; No exact earlier finding for this broken walkthrough was identified&#46;
- Current evidence: The packaged local walkthrough&#39;s plan command supplies neither native material nor fixtures and fails in every supported execution mode&#46; Main bilingual README sections also retain CI&#45;owned materialization/regeneration directions despite the independent prepare entry&#46; Frozen evidence locations: crates/opi&#45;eval/examples/local&#45;evaluation/README&#46;md:155; crates/opi&#45;eval/src/runner/plan&#46;rs:550; crates/opi&#45;eval/README&#46;md:530
- Cause / missed boundary: The local walkthrough omits required &#45;&#45;native&#45;material and README ownership passages remain stale&#46; The stale commands/ownership statements date to September 8/5&#46; General doc checks neither execute the complete walkthrough nor resolve conflicting ownership statements&#46;
- Suggested current verification: Update English/Chinese documentation together; exercise the documented local plan/run sequence with prepared native material, and check all ownership statements against the owning workflow&#46;

### Comparison 10: P19&#45;AUD&#45;010

- Relationship: evidence&#45;gap
- Prior claim (attributed): R27 B6 actual&#45;client public run/seal/report and B10 owned&#45;container cleanup witnesses each executed one passing test; original logs survive and their recorded digests matched&#46;
- Current evidence: This fresh audit has no executed current exact&#45;client matrix or actual Docker/shared&#45;verifier credential cleanup witness&#46; Fake&#45;environment and stand&#45;in tests cannot establish the mandatory real&#45;client wire/tool loops, observed service&#45;model identity or native grader/resource cleanup claims&#46; This is an evidence limitation, not a reproduced defect in those native paths&#46; Frozen evidence locations: crates/opi&#45;eval/tests/native&#95;driver&#46;rs:3870; crates/opi&#45;eval/tests/native&#95;driver&#46;rs:9556; crates/opi&#45;eval/scripts/test&#95;native&#95;integrations&#46;py:313; crates/opi&#45;eval/src/runner/plan&#46;rs:668
- Cause / missed boundary: No recurrence of those exact failures was reproduced by historical inspection&#46; Current audit did not independently admit/apply these records&#46; B6 used Opi source 5bee4f28738412c6ae3e326e5bb3cf60e57230c2; later product changes mean matching raw hashes do not establish the current exact client matrix&#46; opi&#45;eval production/integration/native&#45;driver code is unchanged from a7961ba to audit&#95;head&#46;
- Suggested current verification: Recheck raw log and command JSON identities/counts, exact client/build/material versions, public run&#45;to&#45;report cases and cleanup predicates&#46; Prove unaffected dependency surfaces or rerun only invalidated checks within existing authorization; keep current finding pending until independently resolved&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;e079780&#45;20260926t093335z / P19&#45;AUD&#45;007; source C26
- Repair commits: a7961baeea280a50df9c0ad86a8bed9d56078be0
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;apply&#45;djm9yx8l/b6&#45;actual&#45;green&#46;log; SHA-256 4dfb1bd0213189eba66567d08409e97eeb43bbda910e42aa9cddc09cae83f8a7; checked: Original log bytes match the digest recorded in historical remediation; earlier comparison inspected PASS and actual executed counts&#46; Not rerun in this supplement&#46;; limits: Check applicability to remediation&#95;head and exact build/client/material identities&#46; Historical PASS is not current closure&#46;
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;apply&#45;djm9yx8l/b6&#45;actual&#45;green&#46;json; SHA-256 6a2a11e5b71b1825fe0284aa09adc025cb1c8dc87f921dffb2d611a754977e56; checked: Original command receipt is retained; preceding comparison checked exit 0&#46; Packaged with its current exact byte digest&#46;; limits: Read alongside the named log; inspect command, environment/build identities and counts rather than treating exit 0 alone as behavioral proof&#46;
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;apply&#45;djm9yx8l/b10&#45;actual&#45;green&#46;log; SHA-256 a4253c00432d6d2de139f979995f12d22ea5efab603a87c6d7d6fd47bbdbd148; checked: Original log bytes match the digest recorded in historical remediation; earlier comparison inspected PASS and actual executed counts&#46; Not rerun in this supplement&#46;; limits: Check applicability to remediation&#95;head and exact build/client/material identities&#46; Historical PASS is not current closure&#46;
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;apply&#45;djm9yx8l/b10&#45;actual&#45;green&#46;json; SHA-256 d6b6929c0699965234f8f3e5e277cca0d2663a44a39a0e1738e9493fe0ef76e2; checked: Original command receipt is retained; preceding comparison checked exit 0&#46; Packaged with its current exact byte digest&#46;; limits: Read alongside the named log; inspect command, environment/build identities and counts rather than treating exit 0 alone as behavioral proof&#46;

### Comparison 11: P19&#45;AUD&#45;011

- Relationship: evidence&#45;gap
- Prior claim (attributed): Pi&#45;only regrade selection was repaired&#46; R27 B9&#39;s native verifier&#45;only diagnostic executed one passing test&#46;
- Current evidence: Current native regrade evidence surfaces do not establish fresh offline grading of preserved pinned benchmark artifacts in all three families&#46; The named native&#95;offline&#95;regrade test is a stand&#45;in; the ignored Docker verifier witness replaces benchmark tasks and graders with private diagnostic answer&#46;txt graders&#46; No pinned benchmark&#45;native regrade execution/artifacts were supplied to this audit subtask, so this mandatory dynamic requirement is not assessable&#46; Frozen evidence locations: crates/opi&#45;eval/tests/native&#95;driver&#46;rs:7621; crates/opi&#45;eval/tests/native&#95;driver&#46;rs:10756; crates/opi&#45;eval/tests/support/native&#95;witness&#95;material&#46;rs:45; crates/opi&#45;eval/tests/support/native&#95;witness&#95;material&#46;rs:199; crates/opi&#45;eval/tests/support/native&#95;witness&#95;material&#46;rs:269
- Cause / missed boundary: The diagnostic helper creates task names, an answer&#46;txt grader and a replacement corpus subject&#46; This proves a mechanism but does not by itself prove regrading saved artifacts under the original pinned benchmark graders&#46; Historical green is narrower than the current requirement&#46;
- Suggested current verification: Use saved original task artifacts and original pinned graders for independent verifier&#45;only regrade evidence&#46; Require no Agent/provider execution, unchanged source evidence and fresh derived records; retain the repaired Pi&#45;only selection case&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;e079780&#45;20260926t093335z / P19&#45;AUD&#45;004; source C26
- Repair commits: a7961baeea280a50df9c0ad86a8bed9d56078be0
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;apply&#45;djm9yx8l/b9&#45;regrade&#45;witness&#45;current&#46;log; SHA-256 0d7bd7e0e752c3410adecf33e9e39eec11e5f26ecd36f6ac8d896d1438201fa2; checked: Original log bytes match the digest recorded in historical remediation; earlier comparison inspected PASS and actual executed counts&#46; Not rerun in this supplement&#46;; limits: Check applicability to remediation&#95;head and exact build/client/material identities&#46; Historical PASS is not current closure&#46;
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;apply&#45;djm9yx8l/b9&#45;regrade&#45;witness&#45;current&#46;json; SHA-256 41e1e43b3a18c36af09ea9e2609ff6ff77d25dbd236ae9f27e0c70262cfb29ae; checked: Original command receipt is retained; preceding comparison checked exit 0&#46; Packaged with its current exact byte digest&#46;; limits: Read alongside the named log; inspect command, environment/build identities and counts rather than treating exit 0 alone as behavioral proof&#46;

### Comparison 12: P19&#45;AUD&#45;012

- Relationship: evidence&#45;gap
- Prior claim (attributed): R26 B2 retained six official oracle/nop result files&#46; All six hashes matched the receipt in the earlier September 28 comparison; each family rewards 1/0 with exception&#95;info=null&#46;
- Current evidence: The code routes all three families through selected native integrations and has opt&#45;in oracle/nop witness tests, but their pinned external material, local Docker execution and retained results were not supplied or executed within this audit&#46; The audit therefore cannot mark actual shared&#45;environment, separate artifact collection, or pristine committed&#45;patch grading and their positive/negative witnesses met&#46; This is an audit evidence limitation, not a claim that the implementations fail when correctly provisioned&#46; Frozen evidence locations: crates/opi&#45;eval/tests/native&#95;driver&#46;rs:3485; crates/opi&#45;eval/tests/native&#95;driver&#46;rs:3507; crates/opi&#45;eval/tests/native&#95;driver&#46;rs:3551; crates/opi&#45;eval/scripts/test&#95;native&#95;integrations&#46;py:1
- Cause / missed boundary: Original official witnesses exist: TB2&#46;1 openssl&#45;selfsigned&#45;cert, TB3 batched&#45;eval&#45;parity, DeepSWE abs&#45;module&#45;cache&#45;flags&#46; TB3 collected artifacts and DeepSWE patches differ as expected&#46; Integration/import/cleanup changed afterward, so source/material applicability still needs independent admission; lack of current admission is not absence of historical execution&#46;
- Suggested current verification: Read each exact native result and the receipt below; verify task/material/build/source identity and the positive/negative predicate&#46; Compare changes after their bound state, then rerun only invalidated witnesses&#46; Do not substitute generated diagnostic graders or close solely from recorded rewards&#46;
- Repair commits: 265078fd04442bc317a79175614064e3e243a7ae
- Candidate raw proof: D:/Luiz/Odradek/opi&#45;remediation&#45;evidence/phase19&#45;apply&#45;20260926&#45;bd37/b2&#45;closure&#46;json; SHA-256 f2867c01b63c31fa3589d256fee7a9fb0962e587e7f4ab4242311b2c0dc78230; checked: Earlier comparison verified receipt hash and all six referenced native result hashes, original task identities, rewards and null exception&#95;info&#46;; limits: Receipt includes a historical Closed assertion; only original evidence may be reverified for current applicability&#46;
- Candidate raw proof: WSL:/home/luiz/&#46;local/state/opi&#45;remediation&#45;evidence/phase19&#45;apply&#45;20260926&#45;bd37/retained&#45;official&#45;witnesses/opi&#45;eval&#45;terminal&#45;bench&#45;2&#46;1&#45;oracle&#45;witness&#45;MJdE5s/benchmark/trace/jobs/2026&#45;09&#45;26&#95;&#95;01&#45;53&#45;28/task&#45;package&#95;&#95;oLJyVvp/result&#46;json; SHA-256 e68306e8b33ae31f6e70f3dfd17963c1477061238f09b5fd905d621b29ff3375; checked: Earlier comparison in this conversation located and hashed this original terminal&#45;bench&#45;2&#46;1 oracle result; reward=1, exception&#95;info=null&#46; Not re&#45;executed&#46;; limits: Resolve this absolute path in the same WSL environment&#46; Recheck source/material applicability and exact task identity; do not substitute a diagnostic task or infer current closure from reward alone&#46;
- Candidate raw proof: WSL:/home/luiz/&#46;local/state/opi&#45;remediation&#45;evidence/phase19&#45;apply&#45;20260926&#45;bd37/retained&#45;official&#45;witnesses/opi&#45;eval&#45;terminal&#45;bench&#45;2&#46;1&#45;nop&#45;witness&#45;Dvv2Jg/benchmark/trace/jobs/2026&#45;09&#45;26&#95;&#95;01&#45;54&#45;29/task&#45;package&#95;&#95;dsn9mZV/result&#46;json; SHA-256 1335405a379766d93595332e442f8618dc4f42bfe7cafa363e7e8f626144b03d; checked: Earlier comparison in this conversation located and hashed this original terminal&#45;bench&#45;2&#46;1 nop result; reward=0, exception&#95;info=null&#46; Not re&#45;executed&#46;; limits: Resolve this absolute path in the same WSL environment&#46; Recheck source/material applicability and exact task identity; do not substitute a diagnostic task or infer current closure from reward alone&#46;
- Candidate raw proof: WSL:/home/luiz/&#46;local/state/opi&#45;remediation&#45;evidence/phase19&#45;apply&#45;20260926&#45;bd37/retained&#45;official&#45;witnesses/opi&#45;eval&#45;terminal&#45;bench&#45;3&#46;0&#45;oracle&#45;witness&#45;YhVuml/benchmark/trace/jobs/2026&#45;09&#45;26&#95;&#95;01&#45;55&#45;43/task&#45;package&#95;&#95;22b5rY5/result&#46;json; SHA-256 f722d18ef9e8db1e64b41d84b1b2e9154a770ffbdb4e3e9cfec7cf67dcdf79d3; checked: Earlier comparison in this conversation located and hashed this original terminal&#45;bench&#45;3&#46;0 oracle result; reward=1, exception&#95;info=null&#46; Not re&#45;executed&#46;; limits: Resolve this absolute path in the same WSL environment&#46; Recheck source/material applicability and exact task identity; do not substitute a diagnostic task or infer current closure from reward alone&#46;
- Candidate raw proof: WSL:/home/luiz/&#46;local/state/opi&#45;remediation&#45;evidence/phase19&#45;apply&#45;20260926&#45;bd37/retained&#45;official&#45;witnesses/opi&#45;eval&#45;terminal&#45;bench&#45;3&#46;0&#45;nop&#45;witness&#45;fZPdoB/benchmark/trace/jobs/2026&#45;09&#45;26&#95;&#95;01&#45;57&#45;43/task&#45;package&#95;&#95;JL4TS8S/result&#46;json; SHA-256 0e7c18038da13f17615383c6ee95ac2bb957a88d939539db25d62bb6cd159fda; checked: Earlier comparison in this conversation located and hashed this original terminal&#45;bench&#45;3&#46;0 nop result; reward=0, exception&#95;info=null&#46; Not re&#45;executed&#46;; limits: Resolve this absolute path in the same WSL environment&#46; Recheck source/material applicability and exact task identity; do not substitute a diagnostic task or infer current closure from reward alone&#46;
- Candidate raw proof: WSL:/home/luiz/&#46;local/state/opi&#45;remediation&#45;evidence/phase19&#45;apply&#45;20260926&#45;bd37/retained&#45;official&#45;witnesses/opi&#45;eval&#45;deepswe&#45;oracle&#45;witness&#45;xv0sjJ/benchmark/trace/jobs/2026&#45;09&#45;26&#95;&#95;01&#45;58&#45;38/task&#45;package&#95;&#95;S5kUP3V/result&#46;json; SHA-256 0d1fe37f6fc4bcf1661367716d11bf9c00c22b21220da459fad3c082f23211d8; checked: Earlier comparison in this conversation located and hashed this original deepswe&#45;v1&#46;1 oracle result; reward=1, exception&#95;info=null&#46; Not re&#45;executed&#46;; limits: Resolve this absolute path in the same WSL environment&#46; Recheck source/material applicability and exact task identity; do not substitute a diagnostic task or infer current closure from reward alone&#46;
- Candidate raw proof: WSL:/home/luiz/&#46;local/state/opi&#45;remediation&#45;evidence/phase19&#45;apply&#45;20260926&#45;bd37/retained&#45;official&#45;witnesses/opi&#45;eval&#45;deepswe&#45;nop&#45;witness&#45;xia1FI/benchmark/trace/jobs/2026&#45;09&#45;26&#95;&#95;01&#45;59&#45;19/task&#45;package&#95;&#95;hrzzhfj/result&#46;json; SHA-256 9ca090a0e3d9341f0ecbe1bb64be9ba235bddde0b75e2a46acafdf744bbc404f; checked: Earlier comparison in this conversation located and hashed this original deepswe&#45;v1&#46;1 nop result; reward=0, exception&#95;info=null&#46; Not re&#45;executed&#46;; limits: Resolve this absolute path in the same WSL environment&#46; Recheck source/material applicability and exact task identity; do not substitute a diagnostic task or infer current closure from reward alone&#46;

### Comparison 13: P19&#45;AUD&#45;013

- Relationship: evidence&#45;gap
- Prior claim (attributed): R27 final workspace logs record 4542 passed, 0 failed, 16 ignored across 229 test binaries; clippy, doctests and rustdoc also passed&#46; A later long run sealed six real Agent trials&#46;
- Current evidence: No raw redacted six&#45;trial operator evidence root was supplied to this fresh audit&#46; Committed deterministic fixtures and an acceptance verifier do not establish the mandatory assembled local Zhipu/Opi/pi runs&#46; The full workspace Phase&#45;exit verification union was not rerun&#46; These requirements remain not&#45;assessable; this does not claim that external runs never occurred&#46; Frozen evidence locations: docs/superpowers/specs/2026&#45;09&#45;05&#45;phase19&#45;local&#45;native&#45;agent&#45;evaluation&#45;design&#46;md:266; crates/opi&#45;eval/scripts/test&#95;verify&#95;local&#95;acceptance&#46;py:115; crates/opi&#45;eval/tests/native&#95;driver&#46;rs:3870
- Cause / missed boundary: The six trials were natively valid &#40;TB2&#46;1 1/1; TB3 and DeepSWE 0/0 after four Agent timeouts&#41; but formal acceptance was blocked/not run&#46; Reward zero alone is not the disqualifier: timed&#45;out rows lack healthy terminal and complete assistant/tool/AgentEnd evidence&#46; The old 1 MiB cap was fixed; later empty stdout came from JSON buffering, fixed by 3e8fee6&#46; That cannot restore missing old events&#46; The companion build&#45;source identity remains unknown&#46;
- Suggested current verification: Establish final&#45;source gate applicability and formal six&#45;trial acceptance without weakening predicates&#46; Preserve failed trials; obtain required event/terminal evidence under authorized execution, resolve companion source identity, and rerun only invalidated gates&#46; Ledger selector corrections belong to their owner and do not add a current finding&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;e079780&#45;20260926t093335z / P19&#45;AUD&#45;006; source C26
- Repair commits: a7961baeea280a50df9c0ad86a8bed9d56078be0, 3e8fee6075c719ca0c3192790b800c13421e4fc4
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;apply&#45;djm9yx8l/final&#45;workspace&#45;all&#45;targets&#46;log; SHA-256 b61468b48b4c26e226cd7c7f2e18203649c77fab806d2826e42d37260b4e441f; checked: Original log bytes match the digest recorded in historical remediation; earlier comparison inspected PASS and actual executed counts&#46; Not rerun in this supplement&#46;; limits: Check applicability to remediation&#95;head and exact build/client/material identities&#46; Historical PASS is not current closure&#46;
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;apply&#45;djm9yx8l/final&#45;workspace&#45;all&#45;targets&#46;json; SHA-256 2e5a1a7d3074f70e32bfd324b4486357e610ba12cf7fa149d984d3645ca55f1e; checked: Original command receipt is retained; preceding comparison checked exit 0&#46; Packaged with its current exact byte digest&#46;; limits: Read alongside the named log; inspect command, environment/build identities and counts rather than treating exit 0 alone as behavioral proof&#46;
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;apply&#45;djm9yx8l/final&#45;workspace&#45;clippy&#46;log; SHA-256 43b2302ee9ce41cbbaf37d5c3a5e7bb9697021d69add334847789d6e23ea7dab; checked: Original log bytes match the digest recorded in historical remediation; earlier comparison inspected PASS and actual executed counts&#46; Not rerun in this supplement&#46;; limits: Check applicability to remediation&#95;head and exact build/client/material identities&#46; Historical PASS is not current closure&#46;
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;apply&#45;djm9yx8l/final&#45;workspace&#45;clippy&#46;json; SHA-256 b22734baf8306a30e4b58ceb9199c19c64c5e9748da2074b6c0a3bb416d252e6; checked: Original command receipt is retained; preceding comparison checked exit 0&#46; Packaged with its current exact byte digest&#46;; limits: Read alongside the named log; inspect command, environment/build identities and counts rather than treating exit 0 alone as behavioral proof&#46;
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;apply&#45;djm9yx8l/final&#45;workspace&#45;doctests&#46;log; SHA-256 8ccb9a9673fba1bfd7ad90b3cc7a6a0248ab9e82abee628cf09dac94f95263be; checked: Original log bytes match the digest recorded in historical remediation; earlier comparison inspected PASS and actual executed counts&#46; Not rerun in this supplement&#46;; limits: Check applicability to remediation&#95;head and exact build/client/material identities&#46; Historical PASS is not current closure&#46;
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;apply&#45;djm9yx8l/final&#45;workspace&#45;doctests&#46;json; SHA-256 6b3377579017504281a28f058f78e2ecf279a415716e3ba86749943f0db84208; checked: Original command receipt is retained; preceding comparison checked exit 0&#46; Packaged with its current exact byte digest&#46;; limits: Read alongside the named log; inspect command, environment/build identities and counts rather than treating exit 0 alone as behavioral proof&#46;
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;apply&#45;djm9yx8l/final&#45;workspace&#45;rustdoc&#46;log; SHA-256 f6352de3e77291ee9ff16d8bb639622165d93316658958d0b2e200f36be04e5e; checked: Original log bytes match the digest recorded in historical remediation; earlier comparison inspected PASS and actual executed counts&#46; Not rerun in this supplement&#46;; limits: Check applicability to remediation&#95;head and exact build/client/material identities&#46; Historical PASS is not current closure&#46;
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;apply&#45;djm9yx8l/final&#45;workspace&#45;rustdoc&#46;json; SHA-256 a4e05146bdfea016f7f4c91a514967ed7d55eba9619227808a85a920729bd1c7; checked: Original command receipt is retained; preceding comparison checked exit 0&#46; Packaged with its current exact byte digest&#46;; limits: Read alongside the named log; inspect command, environment/build identities and counts rather than treating exit 0 alone as behavioral proof&#46;
- Candidate raw proof: E:/opi/evaluations/phase19&#45;longrun&#45;validation&#45;20260927/live&#45;final&#45;summary&#46;json; SHA-256 84ccf59a524b30eeb96abde0049765a491d069d8ebb38139b31cd7b3bc61d666; checked: Earlier comparison inspected six sealed native&#45;valid trials, four Agent timeouts, and formal acceptance blocked/not run&#46;; limits: No accepted manifest; original Companion build&#45;source identity unknown&#46;
- Candidate raw proof: E:/opi/evaluations/phase19&#45;longrun&#45;validation&#45;20260927/live&#45;reports/linux&#45;v2/offline&#45;final&#45;audit/acceptance&#45;precheck&#46;json; SHA-256 17949233e8c059b71ebdbd56e8a5a36a44939435975d8e60c2de394b4702b1f0; checked: Earlier comparison checked explicit strict&#45;acceptance precheck blockers&#46;; limits: This is a precheck record, not a passed acceptance run; preserve required predicates&#46;
