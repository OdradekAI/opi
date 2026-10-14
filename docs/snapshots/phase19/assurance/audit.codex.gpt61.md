# Phase 19 Audit

**Audit run ID**: `phase19-codex-gpt61-ae6dff6-20261014t044949z`
**Audit head**: `ae6dff638db1247203a01483f26a0f81048cb52a`
**Reviewer ID**: `codex`
**Model ID**: `gpt61`
**Reviewer identity**: Codex
**Reviewer model ID**: `gpt61`
**Model identity source**: operator-declared
**Independence**: fresh-context-same-family; fresh audit context, no prior audit/remediation or sibling peer output consumed before independent installation. Model slug transcribes the request and does not infer an exact runtime model.
**Baseline policy**: latest-committed-spec
**Verdict**: FAIL

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| .opi-impl-state.json | `c65c10635c4bb8b1754625b94e8a0a15459eef6d65634610ba9b65ea9ba5337e` | complete committed bytes; current committed baseline |
| docs/snapshots/phase19/opi-impl-state.json | `35b3378bd902cc976bb348d794dd1b5d3c5f160a58769c3aeb62fea175a7b7a9` | complete committed bytes; current committed baseline |
| docs/opi-spec.md | `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db` | complete committed bytes; current committed baseline |
| docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md | `f507f29d91ddfc919ead6fa84df8d5f912b04af1d84ed5e68777e20c80c135c5` | complete committed bytes; stored hashes differ in both ledgers |
| docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.zh.md | `7d97f00f3a06723765e8cb2c8ea7ed13933defc8d7c230bdc0d165ad1284e7dc` | complete committed bytes; stored hashes differ in both ledgers |

The 52 immutable requirement identities/criteria/behaviors were sealed before production inspection (initial sidecar SHA-256 `85f1e92f96f5281e736ab572268988c68f0f037ea46c9239d18727b1f0f21ff7`). Evidence fields alone were subsequently populated. Proof classes: 34 fixture, 9 native, 7 live/operator, 2 platform. Latest registered Section 3/7/10 admits TB2.1/TB3 and four real trials; DeepSWE AC-005/D04 is reserved/deferred. Old projected tasks/hash registrations still describe six trials, but are scope/planning evidence and cannot override the current source.

## Requirement Conformance

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| P19-AC-001-diagnose | P19-AC-001 | Doctor validates config before bounded selected-engine observation and reports unknown resources/missing materials. No credential resolution or model launch on this path. | met | - |
| P19-AC-001-prepare | P19-AC-001 | Independent-directory fixture proves local origin, verified task/runner/integration/capture material and selected immutable images; no mandatory GitHub receipt. Native material acquisition is external, not embedded. | met | - |
| P19-AC-001-preview | P19-AC-001 | Plan/direct run share corpus-aware Expansion and freeze exact config/material/trial identities; drift rejects before acquisition. Preview includes selection, repetitions, concurrency and preflight failures. | met | - |
| P19-AC-001-independent | P19-AC-001 | Current production trace/tests establish the implemented portion; full required proof remains limited as the linked finding explains. | not-assessable | P19-AUD-007 |
| P19-AC-002-identity | P19-AC-002 | Missing/wrong/oracle/stale job, task-reference and terminal identity negative cases reach public native result reader. Existing normal fixture assertions discriminate production rejection from subprocess exit success. | met | - |
| P19-AC-002-artifact | P19-AC-002 | Current production trace/tests establish the implemented portion; full required proof remains limited as the linked finding explains. | not-met | P19-AUD-001 |
| P19-AC-003-lifecycle | P19-AC-003 | Supplied raw root /home/luiz/phase19-acceptance-r2: chosen 104 raw proof paths SHA-256-inventoried before selected contents; inventory SHA-256 32d8b24fda8e7fc90bd9e648728e987c10bc0e18d9b735efa75e45bd292b9f5e. Current ... | met | - |
| P19-AC-003-witness | P19-AC-003 | Current production trace/tests establish the implemented portion; full required proof remains limited as the linked finding explains. | not-assessable | P19-AUD-006 |
| P19-AC-003-live | P19-AC-003 | Supplied raw root /home/luiz/phase19-acceptance-r2: chosen 104 raw proof paths SHA-256-inventoried before selected contents; inventory SHA-256 32d8b24fda8e7fc90bd9e648728e987c10bc0e18d9b735efa75e45bd292b9f5e. Current ... | met | - |
| P19-AC-004-collection | P19-AC-004 | Supplied raw root /home/luiz/phase19-acceptance-r2: chosen 104 raw proof paths SHA-256-inventoried before selected contents; inventory SHA-256 32d8b24fda8e7fc90bd9e648728e987c10bc0e18d9b735efa75e45bd292b9f5e. Current ... | met | - |
| P19-AC-004-witness | P19-AC-004 | Current production trace/tests establish the implemented portion; full required proof remains limited as the linked finding explains. | not-assessable | P19-AUD-006 |
| P19-AC-004-live | P19-AC-004 | Supplied raw root /home/luiz/phase19-acceptance-r2: chosen 104 raw proof paths SHA-256-inventoried before selected contents; inventory SHA-256 32d8b24fda8e7fc90bd9e648728e987c10bc0e18d9b735efa75e45bd292b9f5e. Current ... | met | - |
| P19-AC-005-reserved | P19-AC-005 | DeepSWE/Pier code/interfaces retained and current spec explicitly scoped out. Raw prior-attempt directories are present; byte-for-byte historical preservation was not independently established. Reserved criterion is n... | partially-met | - |
| P19-AC-006-projection | P19-AC-006 | Both projection branches are consumed by stage_native_handoff and selected native integrations. URL guard uses exact locked url 2.5.8; tests cover userinfo/fragments/backslashes/IPv6/ports and ordinary loopback. Nativ... | met | - |
| P19-AC-006-clients | P19-AC-006 | Current production trace/tests establish the implemented portion; full required proof remains limited as the linked finding explains. | not-assessable | P19-AUD-006 |
| P19-AC-006-credentials | P19-AC-006 | Selected reference resolves at native dispatch only. Exclusive 0700 directory/0600 files precede first secret write; default fixture credential echo blocks sealing. This proves the fixture boundary, not real grader ca... | met | - |
| P19-AC-006-controls | P19-AC-006 | Unsupported required reasoning/sampling and missing exact-client wire proofs reject before Agent dispatch. Effective controls and origins persist; real Anthropic output-control artifacts provide additional exact-clien... | met | - |
| P19-AC-006-route | P19-AC-006 | Selected connection/model/protocol have no fallback; unknown service identity remains unknown. Configuration changes alter manifest identity; protocol mismatch fixtures prevent pairing. | met | - |
| P19-AC-006-live | P19-AC-006 | Supplied raw root /home/luiz/phase19-acceptance-r2: chosen 104 raw proof paths SHA-256-inventoried before selected contents; inventory SHA-256 32d8b24fda8e7fc90bd9e648728e987c10bc0e18d9b735efa75e45bd292b9f5e. Current ... | met | - |
| P19-AC-007-inventory | P19-AC-007 | Corpus reader enumerates pinned complete task inventory rather than smoke profiles; fixture full/subset plans exercise unselected GPU tasks without pulling every image. Real supplied reports retain corpus counts 89 (T... | met | - |
| P19-AC-007-semantics | P19-AC-007 | Task parser preserves per-role resource/lifecycle/network/collection/time declarations and rejects invalid units, overflow and unsupported fields; native ingress consumes effective_task_requirements. | met | - |
| P19-AC-007-resources | P19-AC-007 | Selected-engine observations and unknown reasons reach check_requirements; unmet GPU/memory/storage/images cannot shrink the explicit selection. Resource observations are bounded read-only preflight, not Agent/image a... | met | - |
| P19-AC-007-freeze | P19-AC-007 | Unknown/duplicate selected IDs and material drift reject; config/trial/material/selection/seeds/repetition identities bind run-control and report. Frozen-plan tests exercise actual public CLI dispatch. | met | - |
| P19-AC-007-images | P19-AC-007 | Selected immutable image IDs bind frozen material; task Dockerfile/profile/corpus mismatches reject. Public prepare acquisition targets only selected immutable images. | met | - |
| P19-AC-007-subset-run | P19-AC-007 | Supplied raw root /home/luiz/phase19-acceptance-r2: chosen 104 raw proof paths SHA-256-inventoried before selected contents; inventory SHA-256 32d8b24fda8e7fc90bd9e648728e987c10bc0e18d9b735efa75e45bd292b9f5e. Current ... | met | - |
| P19-AC-008-metrics | P19-AC-008 | Current report reconstructs native TB2.1 1/1 and TB3 0/0 separately; denominators=2 each, test-case fraction stays separately unknown with reason. Fixture nonbinary/raw-counter cases preserve native semantics. | met | - |
| P19-AC-008-coverage | P19-AC-008 | Current production trace/tests establish the implemented portion; full required proof remains limited as the linked finding explains. | not-met | P19-AUD-002 |
| P19-AC-008-pairing | P19-AC-008 | Task/group/control/protocol matrix and missing pairs reach ComparisonSet/public report. Actual paired runs are same-task same-repetition with frozen controls; no cross-protocol match. | met | - |
| P19-AC-008-single | P19-AC-008 | No-edge single-subject configuration is accepted and reports without absent Agent. Its invalid completion defect is separately linked to coverage, not evidence that single-subject support is absent. | met | - |
| P19-AC-008-unknown | P19-AC-008 | Current production trace/tests establish the implemented portion; full required proof remains limited as the linked finding explains. | not-met | P19-AUD-004 |
| P19-AC-008-invalid | P19-AC-008 | Numerical outcome counts correctly exclude invalid/provider/infra/cancelled/unstarted attempts; valid native zero is retained. Successful completion of solo invalid trials is a separate defect. | met | - |
| P19-AC-009-verify | P19-AC-009 | Fresh current CLI verifies all four supplied bundles against original identities; all selected covered artifact hashes match. Tamper/sidecar/missing seal tests pass. Nested bundle-link disagreement is a boundary consi... | met | - |
| P19-AC-009-recompute | P19-AC-009 | Bundle recomputation/report tests and fresh supplied-run reports pass without network/models. Native metrics cite original sealed result digests; originals are unchanged. | met | - |
| P19-AC-009-regrade | P19-AC-009 | Current production trace/tests establish the implemented portion; full required proof remains limited as the linked finding explains. | not-assessable | P19-AUD-006 |
| P19-AC-009-missing | P19-AC-009 | All-source tamper, missing sibling input, missing cache/artifact/offline network equivalence and output containment tests reject before verifier effects. Verifier-only integration tests cover mocked cached-image no-bu... | met | - |
| P19-AC-010-privacy | P19-AC-010 | Current production trace/tests establish the implemented portion; full required proof remains limited as the linked finding explains. | not-assessable | P19-AUD-006 |
| P19-AC-010-mounts | P19-AC-010 | Private selected-Agent handoff and verifier environment projections keep provider secrets separate. Source integrates pinned external runner without host Docker socket/broad directory mounts into Agent tasks; supplied... | met | - |
| P19-AC-010-bounds | P19-AC-010 | Typed positive integers, checked trial multiplication/max_trials and addressability bound precede expansion/dispatch. Optional unenforceable token/currency/output caps reject. Bounded read-only resource/image observat... | met | - |
| P19-AC-010-cancel | P19-AC-010 | Actual Unix process-group fixture tests verify descendant cancellation/natural-exit cleanup and bounded pipes. Exact daemon-observed Compose roots/container IDs govern fake-Docker cleanup; uncertainty cancels further ... | met | - |
| P19-AC-010-recover | P19-AC-010 | Durable intent precedes process effects; recovered unsettled intent stays effect-unknown. Recovery/replacement public fixtures preserve originals, use fresh IDs and whole paired-group rules. | met | - |
| P19-AC-010-breach | P19-AC-010 | Authority ledger failure vetoes affected dispatch/grading/seal/report; private secret scan and unconfirmed cleanup stop new work, keep failed evidence and exclude shared caches from cleanup. | met | - |
| P19-AC-011-default | P19-AC-011 | Fresh fail_closed_cli dummy_credentials_do_not_activate_live_behavior executes successfully. Fixture modes require explicit fixtures; native real/client/witness cases remain ignored, and dummy ambient auth never selec... | met | - |
| P19-AC-011-entry | P19-AC-011 | 217 deterministic acceptance/CI verifier methods plus source callers show native-smoke materialize-configs/run-trials reuse public prepare/run; direct local entry does not require dispatch. Native smoke actual executi... | met | - |
| P19-AC-012-four | P19-AC-012 | Supplied raw root /home/luiz/phase19-acceptance-r2: chosen 104 raw proof paths SHA-256-inventoried before selected contents; inventory SHA-256 32d8b24fda8e7fc90bd9e648728e987c10bc0e18d9b735efa75e45bd292b9f5e. Current ... | met | - |
| P19-AC-012-freeze | P19-AC-012 | Supplied raw root /home/luiz/phase19-acceptance-r2: chosen 104 raw proof paths SHA-256-inventoried before selected contents; inventory SHA-256 32d8b24fda8e7fc90bd9e648728e987c10bc0e18d9b735efa75e45bd292b9f5e. Current ... | partially-met | P19-AUD-003 |
| P19-AC-012-attempts | P19-AC-012 | Current production trace/tests establish the implemented portion; full required proof remains limited as the linked finding explains. | not-assessable | P19-AUD-007 |
| P19-AC-012-platform | P19-AC-012 | Current production trace/tests establish the implemented portion; full required proof remains limited as the linked finding explains. | not-assessable | P19-AUD-007 |
| P19-AC-012-gates | P19-AC-012 | Current production trace/tests establish the implemented portion; full required proof remains limited as the linked finding explains. | not-assessable | P19-AUD-007 |
| P19-AC-013-placement | P19-AC-013 | Cargo: unpublished opi-eval, no internal Opi edges or new model client/core seam; datasets/native graders/images external. url 2.5.8 is workspace-owned and already locked transitively; only the direct Eval edge change... | met | - |
| P19-AC-013-pi-only | P19-AC-013 | Current production trace/tests establish the implemented portion; full required proof remains limited as the linked finding explains. | not-assessable | P19-AUD-006 |
| P19-AC-013-contracts | P19-AC-013 | Current production trace/tests establish the implemented portion; full required proof remains limited as the linked finding explains. | partially-met | P19-AUD-005 |
| P19-AC-013-minimal | P19-AC-013 | All eleven projected admitted tasks mapped to current consumers/reuse/ceilings below. Existing material, lifecycle, process, native importer, bundle and report boundaries are reused; no speculative new trait/crate/SDK... | met | - |

Machine sidecar carries exact criterion hash, full surfaces, actual test counts, check observations, limitations and reciprocal findings. Optional reserved AC-005 is not an exit obligation.

## Standards Review

Independent unpublished Companion topology conforms to root Cargo metadata, inward dependency direction, explicit typed error/state/ownership boundaries and scoped external process integration. Native data/graders/images stay external; no Agent Core provider/Eval policy seam was introduced. Exact locked URL parser is `url 2.5.8`; the new direct dependency reuses an already locked transitive package rather than adding unexpected package upgrades. Existing unsafe remains isolated in process-tree OS bindings. Primary bilingual docs and Unreleased synchronize actual 0.x changes; the independent walkthrough has the narrower drift in P19-AUD-005.

## Spec Review

Supplied raw root /home/luiz/phase19-acceptance-r2: chosen 104 raw proof paths SHA-256-inventoried before selected contents; inventory SHA-256 32d8b24fda8e7fc90bd9e648728e987c10bc0e18d9b735efa75e45bd292b9f5e. Current rebuilt CLI verify/report accepts original tb21-z2 and tb30-z3 bundles: two Opi/pi trials each, native rewards TB2.1=1/1, TB3=0/0, task-side tool activity and matching delivered/collected digests. Actual cached Opi executable ddb13617c26276d4c0d8b600f2172da91be62233bdfec0ffece1e03c3979007a and pi ec7efdc6655fe3100ff43e87d9c2c7ece4d6054f0c8e5bf7c56c60b28b7cfc3d hash to recorded identity; capture helper 774d795633149fa45cd49f780a397da23596e7ab53ecb51783125d1a24d32874. Harbor integration bytes 2054019e01003e337afe84aa93ec3d2c5e840a1e39912a41f4475997b545f0d8 equal audit_head export. Opi build identity says clean 3908fba723956a6af7daf298599cbe3d30bbbc09; Agent/AI/product production source and their locked dependency package bytes have no relevant delta (new url edge affects Eval only). These are older supplied execution observations rechecked by current readers, not fresh native/provider execution of every current path; broader operator/platform/workspace and native witness/regrade proof remains explicitly limited.

Four observed real trials satisfy the separate trial-count record. They do not close native witness/client/regrade/canary/pi-only predicates or current complete operator/platform/workspace proof. Actual TB3 zero scores are valid Agent outcomes; no requirement to beat pi or obtain positive reward is inferred. Current source defers DeepSWE, and no DeepSWE absence blocks this audit.

## Security, Invariants, Integration, Test Quality, and Residuals

Production flow is CLI resolve/plan/material admission -> selected native Agent projection/private handoff -> one explicit upstream job/trial -> actual result/collection/terminal reader -> typed settlement -> original bundle seal -> public offline report/verify/regrade. Guard interpretations were compared with their consumers. URL alternate forms and integer type/extreme/bound cases are in the executed current tests. Derived count bounds precede allocation/Agent acquisition; bounded read-only resource/image inspections happen earlier and are identified as such, not falsely described as model execution.

Credential reference resolution occurs only at native dispatch; exclusive private modes precede first secret byte, default ambient credentials never select live execution, and process environments are explicitly cleared. Actual Unix process fixtures exercise cancellation and natural-exit descendants; daemon-owned resource cleanup has conservative uncertainty. Real grader canary/native cancellation breadth is not inferred from fake-Docker passes.

The artifact-omission, solo-invalid and usage-state probes reach current production consumers and distinguish happy/rejected paths from the deficient cases. The acceptance source-removal argument keeps actual Companion inputs unchanged, so byte verification cannot repair missing outer freeze records. A nested bundle symlink passes byte verify but is rejected by report; linked bytes retain their valid seal and no score is published, so this remains a bounded consistency lead, not a score-tamper finding.

Test quality: every used package target actually executed; native_driver inventory/checker separates 124 passes from 14 opt-in ignored cases. Existing symmetric/asymmetric fixtures and snapshots were read for their claimed fields. Artifact-only measured-usage assertion and solo happy/paired-invalid cases miss their intersections. Internal `OfflineRegrader`/byte-report schema vocabulary is retained residual naming, not a competing native grading engine.

## Minimum-change Conformance

| Task | Status | Production/nonproduction consumers; reuse/ceiling; net deletion/residual glue |
|---|---|---|
| 19.1 | drifted | Native result importer -> runner/bundle/report; native_driver fixtures. Existing Agent/benchmark/process seams reused, but incomplete delivered observation violates artifact admission. Net deletion not historically quantified; residual external runner/terminal glue is necessary and completeness guard is deficient. |
| 19.2 | conforming | project_connection -> private handoff -> Opi/pi native clients; experiment/native-control tests/help. Existing TOML/config/material projection, no Eval LLM client/core seam. Net deletion not quantified; isolated client config/wire proof remains necessary glue, with native proof limitation. |
| 19.3 | drifted | TB3 importer/handoff -> native grade and regrade/report; declared-artifact tests. Reuses same runner/collection seam; P19-AUD-001 affects omission admission. Net deletion not quantified; no second container/grader engine found. |
| 19.4 | not-assessable | Current source explicitly defers DeepSWE D04/AC005; retained Pier/DeepSWE interfaces and code are inspected as residual shared consumers only. No current delivery/exit proof demanded. Historical net deletion/byte preservation not asserted. |
| 19.5 | conforming | Doctor/Prepare -> NativeMaterial/process; plan/run/regrade share selected inventory; tests/README. No mandatory upload receipt for local origin. Net deletion: obsolete receipt requirement removed semantically; external acquisition script/material glue retained. |
| 19.6 | conforming | Shared Expansion/effective requirements -> plan and run; run-control/report/recovery consumers; full/subset/limit fixtures. No duplicate trial generator. Net deletion not quantified; corpus/profile/resource glue demonstrates real variation. |
| 19.7 | drifted | ReportBuilder/Verify -> original bundle and native settlement; public report/containment/recompute tests. P19-AUD-002/004 leave status/usage heuristics wrong. Byte verification and native grading are distinct; residual internal byte-regrader name remains. |
| 19.8 | not-assessable | NativeRegrader -> family verifier-only integration/new derived records; selected-family/tamper/missing-cache tests. Existing upstream verifier lifecycle reused, no Agent/provider. Native actual fresh regrade proof missing; net deletion not quantified, verifier-only restore glue retained. |
| 19.9 | conforming | ProcessSupervisor/private handoff/owned cleanup/lifecycle -> public run/recover/replacement; Unix/fake-Docker/authority tests. Shared cache/user resources excluded from cleanup. No additional public runtime seam; net deletion not quantified, bounded platform glue justified. Native canary proof remains limited. |
| 19.10 | drifted | Local/native CI scripts call public prepare/run; offline acceptance calls built verify/report; Python regressions/docs. No separate scoring policy, but P19-AUD-003/005 omit provenance and stale walkthrough synchronization. Net deletion not quantified; transport/assurance glue remains bounded. |
| 19.11 | not-assessable | Supplied operator handoff -> current offline verifier/report; Python negative cases. Four raw real outcomes are retained, but current complete handoff/platform/workspace/native predicates are not qualified. Evidence review creates no runtime seam or implementation ledger here. |

Reuse-search, surface-necessity and simplification-ceiling projections were checked against the current consumers above. Every durable/public seam has current consumers/tests or an explicit native/operator proof limitation. Historical intent or numerical net deletion was not invented from prior audit conclusions. Current code defects are routed through normal finding axes.

## Findings

### P19-AUD-001: Incomplete delivered observations admit substituted native artifacts

- Axis: security
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-002-artifact
- Claim: The result reader admits a graded-native score for substituted declared bytes when the terminal delivered list is empty/incomplete; bounded observation omissions therefore fail open.
- Evidence:
  - `crates/opi-eval/integrations/harbor/opi_eval_harbor.py:916`: Observation scans cwd only, depth<=3, files smaller than limit, first bounded file count; exceptions return an honest empty list without completeness proof.
  - `crates/opi-eval/src/benchmark/process.rs:976`: Cross-check iterates only observed delivered files; no reverse completeness check for collected declared artifacts. Terminal validation accepts [] after passed dispatch.
  - `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-audit-afd5e0a19d6d4e6784ecd3ffaa90ac89\native-artifact-probe-assets-v2\observed-substitution.stdout.json`: Bounded production CLI stand-in: observed substituted schema file rejects with native-binding-delivered-artifact-substituted, status failed, no seal.
  - `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-audit-afd5e0a19d6d4e6784ecd3ffaa90ac89\native-artifact-probe-assets-v2\empty-observation-substitution.stdout.json`: Same substituted declared input, changing delivered observation to []: status sealed, graded-native reward 1, verified-native binding, completed. Fake runner/Docker/Agent only; original process exit was not separately preserved in this probe, so stdout/seal and static trace are the evidence, not an invented exit.
- Refutation attempted: Existing substituted-artifact tests and collected directory digest checks succeed when the file is observed. The differential probe exercises that exact rejection branch first. Rehashing collected bytes proves only post-collection self-consistency, not equality to omitted Agent-side bytes. Valid four supplied shallow-artifact trials do not cover omission limits. No arbitrary path or external attack was executed.
- Suggested closure: Bind a complete inventory of the declared artifact roots before handoff, or reject score admission when required observation is unavailable/truncated; retain legitimate empty task outcomes without inventing artifact proof.

### P19-AUD-002: Single-subject invalid trials return completed and exit zero

- Axis: invariants
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-008-coverage
- Claim: A no-edge single-subject run containing only invalid adapter/grader outcomes returns successful completion; repeated invalid missing headlines are labelled consistent.
- Evidence:
  - `crates/opi-eval/src/runner/experiment.rs:914`: all() over an empty comparison set is true; completion uses only pair comparability and sealing.
  - `crates/opi-eval/src/report.rs:876`: Offline run_outcome repeats the pair-only predicate independently of valid/invalid counts; control_pairs compares null headlines at 1417-1434.
  - `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-audit-afd5e0a19d6d4e6784ecd3ffaa90ac89\selection-probe.json`: Fresh CLI happy control: 2 valid/0 invalid, completed. agent-malformed-stream and verifier-failure: run exit0/completed; report published/completed, planned=attempted=invalid=2, valid=0, repetition state consistent.
- Refutation attempted: README exit0 wording mentions every declared pair, and numerical invalid counts remain honest. However the admitted useful single-subject route has no pair; README exit1 means settled non-success. Shape alone must not turn the same adapter/grader failure into successful completion. Valid native zero remains a separately passing case.
- Suggested closure: Derive completion and repetition comparability from each planned trial valid interpretation as well as edge pairing; keep typed invalid attempts and valid zero outcomes.

### P19-AUD-003: Acceptance verifier omits required execution provenance and freeze records

- Axis: runtime-fidelity
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-012-freeze
- Claim: The offline acceptance verifier can return accepted when Opi source/build linkage, original command records and required operator usage/time/platform freeze records are missing or malformed.
- Evidence:
  - `crates/opi-eval/scripts/verify-local-acceptance.py:256`: Build checks compare supplied executable digest strings with attribution; no source-to-build proof is required.
  - `crates/opi-eval/scripts/verify-local-acceptance.py:397`: Command evidence consists of asserted family/command/exit tuples, without original argv/log/exit binding.
  - `crates/opi-eval/scripts/verify-local-acceptance.py:1158`: Acceptance.run has no required Opi source/build/original command/operator budget/platform admission. check_companion at 1133 receives only unchanged family roots.
  - `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-audit-afd5e0a19d6d4e6784ecd3ffaa90ac89\acceptance-probes.py`: Three structural fixture probes accepted: remove source/operator outer metadata; supply malformed Opi URL/commit; coordinated supplied-lock/source/cache replacement. Third is fixture/stub-only trust-boundary evidence, not an actual native authentication claim.
- Refutation attempted: 74 existing acceptance tests and real Companion bundle verification establish substantial control/material/seal/identity checks. Removing only outer source/operator records leaves every Companion argument and family byte unchanged, so the real byte-only verify/report consumer cannot restore the missing freeze facts. Supplied Opi build-identity metadata helps this audit but is not required or consumed by this verifier. Independently admitted lock authenticity must be established externally; mutual lock/cache agreement alone is not that admission.
- Suggested closure: Require and byte-bind original execution/build/source/frozen scope evidence, or expose incomplete predicates and require an independently identified verifier for them before acceptance.

### P19-AUD-004: Unknown Opi usage is advertised as measured

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-008-unknown
- Claim: The normalized native usage projection reports measured solely because a records artifact exists, even when both actual token measurements are unknown and the cited records contain no usage.
- Evidence:
  - `crates/opi-eval/src/report.rs:1294`: native_facts checks entry presence, not measurement availability or origin, and cites native/evidence/records.
  - `crates/opi-eval/tests/report_contract.rs:181`: Asymmetric native-fact assertion validates only measured marker/artifact digest rather than content containing usage.
  - `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-audit-afd5e0a19d6d4e6784ecd3ffaa90ac89\selection-probe.json`: Privately cloned Opi fixture manifest has both input/output Unknown(not_reported); public run/report exit0. diagnostics retain both unknown reasons, while native_facts.usage says measured and points to Provider/Diagnostic-only records.
- Refutation attempted: Per-token diagnostics correctly retain origins/reasons and no numerical token value is fabricated. This narrows impact, but does not refute the false measured coverage/provenance statement. Current mandatory unknown-telemetry contract makes this a blocking factual report defect.
- Suggested closure: Derive coverage/state from sealed native measurements and cite the containing source; retain partial/unknown reasons and measurement origins.

### P19-AUD-005: Independent walkthrough retains six-trial scope and missing installation paths

- Axis: standards
- Severity: Minor
- Conformance effect: blocks
- Requirement IDs: P19-AC-013-contracts
- Claim: The independent-directory example still requires six attributable trials and invokes script/binary locations its walkthrough never installs, despite current four-trial TB2.1/TB3 acceptance.
- Evidence:
  - `crates/opi-eval/examples/local-evaluation/README.md:211`: Handoff says six attributable trials; DeepSWE is deferred and verifier/primary docs use schema /2 and at least four admitted-family trials.
  - `crates/opi-eval/examples/local-evaluation/README.md:217`: Independent-directory command assumes scripts/verify-local-acceptance.py and ./target/release/opi-eval; earlier cache seeding does not install those paths.
- Refutation attempted: Primary English/Chinese README, Unreleased and verifier/tests are synchronized to four trials. The narrower remaining consumer is the independent walkthrough.
- Suggested closure: Synchronize the example to four admitted-family trials and explicitly installed or supplied absolute verifier/Companion locations.

### P19-AUD-006: Required native conformance predicates lack applicable raw execution proof

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-003-witness, P19-AC-004-witness, P19-AC-006-clients, P19-AC-009-regrade, P19-AC-010-privacy, P19-AC-013-pi-only
- Claim: Supplied first-connection real bundles and passing stand-in tests do not establish correct/incorrect native witnesses, all three protocol actual-client loops, fresh native offline regrade, real grader/export credential canaries or pi-only native execution without Opi material.
- Evidence:
  - `crates/opi-eval/tests/native_driver.rs:11862`: Real native regrade/client/witness cases are opt-in ignored. Current complete normal run executes 124 cases and ignores 14; fixture successes cannot qualify their predicates.
  - `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-audit-afd5e0a19d6d4e6784ecd3ffaa90ac89\raw-inventory.json`: Selected supplied records are final real trials and Anthropic output-control proof; no applicable raw witness/regrade/all-six-wire/canary/pi-only execution records were supplied or found in the selected proof surface.
  - `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md#5. Native execution and material contract`: Correct/incorrect real native grader witnesses are distinct from real-provider trial minimum; actual offline regrading is distinct from byte verification.
- Refutation attempted: Four real native bundles pass current verify/report, Harbor integration bytes match current export, exact cached Agent binaries match their pins, and 437 Rust + 217 acceptance/CI Python + 4 verifier-only Python tests pass. None changes only submitted native artifact at real grader, executes all required native client protocol paths, or performs fresh verifier-only native regrading. Anthropic proof is useful partial evidence, not qualification of optional wires. DeepSWE witness/regrade/real trial is explicitly deferred and excluded from this finding.
- Suggested closure: Supply applicable original commands/exits, actual executed identities/counts, source/build/client/material pins and original seals for each missing admitted-family predicate; rerun invalidated native checks only within explicit existing execution authorization.

### P19-AUD-007: Current assembled acceptance and exit evidence remains incomplete

- Axis: runtime-fidelity
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-001-independent, P19-AC-012-attempts, P19-AC-012-platform, P19-AC-012-gates
- Claim: The supplied root does not provide a current complete independent-directory handoff, byte-bound retained-attempt coverage, operator platform record and final workspace verification union; its staged acceptance manifest is legacy /1 and rejected by the current /2 verifier.
- Evidence:
  - `/home/luiz/phase19-acceptance-r2/evidence-root/manifest.json`: SHA-256 31d290e2cefb4c617ea6801f678e7a15772f1e16cbb531975f0ac17aae8a4535; schema /1, pins older tb21-r5/tb30-r1 and scoped-out DeepSWE. Current verifier explicitly rejects it before acceptance.
  - `/home/luiz/phase19-acceptance-r2/operator-record.json`: SHA-256 95afee6d45b3088f99ed0835188107ab0c835855bcfc29d80dea0491e0477ea3; projected verification field has earlier-session package-level PASS strings, not original full-workspace logs/source seals/executed counts.
  - `C:\Users\Luiz\AppData\Local\Temp\opi-phase19-audit-afd5e0a19d6d4e6784ecd3ffaa90ac89\rust-execution.json`: Fresh package-local all-target test run is 437 passed/14 ignored; full workspace fmt/clippy/all-targets/doc-test/rustdoc union not rerun or evidenced for final relevant state.
- Refutation attempted: Top-level newer tb21-z2/tb30-z3 raw runs prove four real outcomes and current offline readers verify/reconstruct them. WSL is available and used for this audit. Those observations do not prove the complete independently prepared product handoff, all original failed-attempt retention, frozen local Docker/WSL operator platform conditions or final workspace gates. This is an evidence gap, not a claim that four trials are absent or that native score zero is invalid. Source reduction itself does not repair/requalify a legacy evidence manifest.
- Suggested closure: Assemble current /2 raw handoff for admitted families with original source/build/prepare/plan/run/report/attempt/platform records and byte-bound Opi-only capture view, and provide original final-state workspace gate logs with execution counts; retain scoped-out DeepSWE originals separately.

## Verification Commands

| Command | Result | Requirement/finding |
|---|---|---|
| `cargo test --locked -p opi-eval --all-targets` | PASS: 437 passed, 0 failed, 14 ignored; original exit 0 | Current package behavior; ignored native cases excluded |
| `check_test_execution.py` native_driver full list/ignored list/run | PASS: 138 inventory, 124 executed, 14 ignored, 0 filtered | Native anti-vacuity |
| WSL six acceptance/CI Python suites | PASS: 217 actual methods | AC-011/012/013 deterministic paths |
| `python -B -m unittest test_native_integrations.VerifierOnlyEnvironmentTest -v` | PASS: 4 actual methods | Mocked verifier-only protocol; not real regrade |
| Ten bounded isolated selection public CLI probes | Expected discriminating observations recorded; original exits preserved | P19-AUD-002/004; nested-link lead |
| Two bounded fake native artifact scenarios | Differential rejected/accepted seals preserved; original process exits not separately retained | P19-AUD-001; no live/native trial |
| Three isolated acceptance metadata probes | All fixture structures accepted | P19-AUD-003; actual-family inputs unchanged in source-removal case |
| Current `opi-eval verify/report` on tb21-z2/tb30-z3 | Verified four original bundles; reports valid=2 each, TB2.1=1/1 TB3=0/0 | Separate observed real-trial count and offline reconstruction |
| Current `verify-local-acceptance.py` on supplied staged evidence-root | REJECTED: legacy /1 instead of required /2 | P19-AUD-007 |
| Real native witness/all-wire clients/regrade/canary/pi-only checks | NOT RUN by this audit; no applicable supplied full proof | P19-AUD-006; no credentials or paid/native trials implicitly authorized |
| Full final-state workspace verification union | NOT EVIDENCED; package tests alone are narrower | P19-AUD-007 |

## Verdict Rationale

Mandatory state counts: {'met': 36, 'not-assessable': 10, 'not-met': 3, 'partially-met': 2}. Seven reciprocal findings: six Major and one Minor, all blocking their linked mandatory records. FAIL follows mechanically from mandatory non-met/partially-met/not-assessable records. Source hash mismatches and deferred scope are recorded independently of verdict; valid zero scores and four observed real trials do not compensate for other missing/incorrect mandatory predicates.

Test impact: **retain** existing repository tests; **none** production/test changes. Private audit probes and captured execution evidence live outside the source export and assurance set. Audit output follows the required four-file contract, so no extra Chinese counterpart is introduced; registered bilingual sources/primary docs were reviewed together. No production remediation, ledger write, branch, commit, push or live provider call was performed.

<!-- opi-audit-history/1
{
  "audit\u005fhead": "ae6dff638db1247203a01483f26a0f81048cb52a",
  "audit\u005frun\u005fid": "phase19-codex-gpt61-ae6dff6-20261014t044949z",
  "comparison": {
    "comparisons": [
      {
        "cause": "Introduction time is not established. For the walkthrough, the latest source reduction to two families/four trials makes its retained six-trial instruction newly inconsistent; no behavioral regression is inferred from matching IDs.",
        "current\u005fevidence": "Current independently installed P19-AUD-001 contains current-head source/consumer observations and discriminating proof; its own core remains unchanged.",
        "current\u005ffinding\u005fid": "P19-AUD-001",
        "prior\u005fclaim": "The immediately superseded four-finding sidecar did not record incomplete delivered-artifact observation admits substituted bytes. This absence claim is limited to that exact sidecar, not older/sibling audits or the time the code defect originated.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use the current finding reproduction/criterion at ae6dff638db1247203a01483f26a0f81048cb52a; do not use historical absence as refutation or closure."
      },
      {
        "cause": "Introduction time is not established. For the walkthrough, the latest source reduction to two families/four trials makes its retained six-trial instruction newly inconsistent; no behavioral regression is inferred from matching IDs.",
        "current\u005fevidence": "Current independently installed P19-AUD-002 contains current-head source/consumer observations and discriminating proof; its own core remains unchanged.",
        "current\u005ffinding\u005fid": "P19-AUD-002",
        "prior\u005fclaim": "The immediately superseded four-finding sidecar did not record solo invalid trials return completed/exit0 and consistent null repetitions. This absence claim is limited to that exact sidecar, not older/sibling audits or the time the code defect originated.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use the current finding reproduction/criterion at ae6dff638db1247203a01483f26a0f81048cb52a; do not use historical absence as refutation or closure."
      },
      {
        "cause": "Introduction time is not established. For the walkthrough, the latest source reduction to two families/four trials makes its retained six-trial instruction newly inconsistent; no behavioral regression is inferred from matching IDs.",
        "current\u005fevidence": "Current independently installed P19-AUD-004 contains current-head source/consumer observations and discriminating proof; its own core remains unchanged.",
        "current\u005ffinding\u005fid": "P19-AUD-004",
        "prior\u005fclaim": "The immediately superseded four-finding sidecar did not record all-unknown opi usage is projected as measured. This absence claim is limited to that exact sidecar, not older/sibling audits or the time the code defect originated.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use the current finding reproduction/criterion at ae6dff638db1247203a01483f26a0f81048cb52a; do not use historical absence as refutation or closure."
      },
      {
        "cause": "Introduction time is not established. For the walkthrough, the latest source reduction to two families/four trials makes its retained six-trial instruction newly inconsistent; no behavioral regression is inferred from matching IDs.",
        "current\u005fevidence": "Current independently installed P19-AUD-005 contains current-head source/consumer observations and discriminating proof; its own core remains unchanged.",
        "current\u005ffinding\u005fid": "P19-AUD-005",
        "prior\u005fclaim": "The immediately superseded four-finding sidecar did not record independent walkthrough remains six-trial and assumes missing local install paths. This absence claim is limited to that exact sidecar, not older/sibling audits or the time the code defect originated.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use the current finding reproduction/criterion at ae6dff638db1247203a01483f26a0f81048cb52a; do not use historical absence as refutation or closure."
      },
      {
        "cause": "A different branch of the same immutable execution/freeze invariant is missing: actual external evidence availability and enforcement by the acceptance consumer are separate predicates. No intermediate working enforcement state or later regression was proven.",
        "current\u005fevidence": "Current core P19-AUD-003 traces Acceptance.run/check\u005fbuilds/check\u005fcommand\u005fexits/check\u005fcompanion and three private metadata probes. Deleting outer source/operator metadata leaves every real Companion argument and family byte unchanged.",
        "current\u005ffinding\u005fid": "P19-AUD-003",
        "prior\u005fclaim": "Superseded P19-AUD-004 described the offline verifier implementation as present and requiring identity/commands/frozen inputs, while finding actual operator proof unavailable. It did not independently establish that the tool enforces every required outer source/build/command/budget/platform field; this is not an old closure claim.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-2ec1ccb-20261004t023807z",
            "finding\u005fid": "P19-AUD-004",
            "source\u005fid": "superseded-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Inspected the exact private fixture mutations, source-removal argument and bounded local execution handoff.",
            "limitations": "Fixture/stub checks cannot alone authenticate native runs; the source-removal predicate keeps actual Companion inputs unchanged.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-afd5e0a19d6d4e6784ecd3ffaa90ac89\\acceptance-probes.py",
            "sha256": "3ee38b18cff28f36d53facc018fd721b0d21dbb56740d7035fd36ce526a01919"
          }
        ],
        "relationship": "missed-branch",
        "repair\u005fcommits": [],
        "verification": "Add original-proof source/build/argv/exit/frozen scope admission or explicit incomplete predicates at the existing verifier; test absent/malformed fields against an actual already-valid family root."
      },
      {
        "cause": "Continuing lack of applicable proof for distinct predicates, with changed scope and additional supplied raw observations; no runtime failure or recurrence is inferred from the old title. Old zero-filter claims were not adopted as a fresh current finding.",
        "current\u005fevidence": "Current normal native\u005fdriver execution is 124 passes/14 ignored, with a PASS execution checker. New supplied Anthropic/real trial raw proof is useful, but native witness/all-wire/regrade/canary/pi-only predicates remain unqualified in the current core. DeepSWE is now explicitly deferred and cannot block current acceptance.",
        "current\u005ffinding\u005fid": "P19-AUD-006",
        "prior\u005fclaim": "Superseded P19-AUD-003 lacked actual family witnesses, all-wire clients, verifier-only regrade and actual Docker cleanup; it also recorded two stale ignored filters and included then-mandatory DeepSWE.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-2ec1ccb-20261004t023807z",
            "finding\u005fid": "P19-AUD-003",
            "source\u005fid": "superseded-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Current complete Cargo log parsed into actual identities/counts; package exit0, 437 passes/14 ignored.",
            "limitations": "Package fixture success is not actual native execution.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-afd5e0a19d6d4e6784ecd3ffaa90ac89\\rust-execution.json",
            "sha256": "326795b089d55fed1e2e9a45919ec5b87814d59895b6fa211c3d39ebf5561ee4"
          },
          {
            "checked": "104 selected supplied raw paths/digests inventoried; four latest bundles and selected Anthropic proof checked.",
            "limitations": "Selection excludes private auth and audit/remediation/sibling/history conclusions; not a global claim no other proof exists.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-afd5e0a19d6d4e6784ecd3ffaa90ac89\\raw-inventory.json",
            "sha256": "32d8b24fda8e7fc90bd9e648728e987c10bc0e18d9b735efa75e45bd292b9f5e"
          }
        ],
        "relationship": "evidence-gap",
        "repair\u005fcommits": [],
        "verification": "Use exact current opt-in case identities, original source/build/client/material conditions and original executed counts. Supply missing admitted-family raw proofs without implicitly authorizing live/native trials."
      },
      {
        "cause": "Evidence availability improved and normative scope changed; this is narrower current proof incompleteness, not persistent missing six-trial behavior, and scope reduction is not behavioral repair.",
        "current\u005fevidence": "Current core recognizes four real TB2.1/TB3 native outcomes and fresh verify/report results. Provided staged manifest is /1 and rejected by current /2 verifier; complete independent prepare/run handoff, original attempts/platform proof and final workspace union remain explicitly limited.",
        "current\u005ffinding\u005fid": "P19-AUD-007",
        "prior\u005fclaim": "Superseded P19-AUD-004 had no supplied six-trial root and could not qualify assembled local/platform/workspace proof under its then-registered three-family scope.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-2ec1ccb-20261004t023807z",
            "finding\u005fid": "P19-AUD-004",
            "source\u005fid": "superseded-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Current verifier rejection record read; accepted=false, expected /2 versus supplied /1.",
            "limitations": "This rejects the staged evidence view, not the existence of the four newer top-level real trials.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-afd5e0a19d6d4e6784ecd3ffaa90ac89\\provided-root-verification.json",
            "sha256": "44ae5a7b61e7ba601c8aa660d0937c5669e6aeb3e4d04f4353c52b92f262e18c"
          },
          {
            "checked": "Current reporter reconstructs two valid TB2.1 results, rewards1/1, subset/corpus89.",
            "limitations": "Not complete native witness/current operator handoff/workspace qualification.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-afd5e0a19d6d4e6784ecd3ffaa90ac89\\raw-tb21-report.json",
            "sha256": "1fa715c4684b65e10ac67b7f632b39a48698418840ec4ace51382ddd16f4318c"
          },
          {
            "checked": "Current reporter reconstructs two valid TB3 results, rewards0/0, subset/corpus74.",
            "limitations": "Valid zeros count; not complete native witness/current operator handoff/workspace qualification.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase19-audit-afd5e0a19d6d4e6784ecd3ffaa90ac89\\raw-tb30-report.json",
            "sha256": "ef96acfef560cb259ef7246b1458e0449adc4e6b70679ea01ec9ba9fa41d4a29"
          }
        ],
        "relationship": "evidence-gap",
        "repair\u005fcommits": [],
        "verification": "Build a byte-bound current admitted-family /2 handoff with original commands/builds/attempts/platform/gate records; retain scoped-out DeepSWE evidence separately."
      },
      {
        "cause": "Current parser/consumer interpretation and discriminating tests show the previously reported guard defect is repaired at audit\u005fhead. Specific repair commit and intermediate state were not reconstructed; repair\u005fcommits stays empty.",
        "current\u005fevidence": "Current core independently found met P19-AC-006-projection: full URL parsing uses exact locked url2.5.8 with scheme/host matching. Current experiment\u005fcontract tests reject fragment/backslash/userinfo/port variants and accept actual loopback IPv4/IPv6; 35 actual test cases passed.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded P19-AUD-001 reproduced non-live fragment/userinfo authority splitting that accepted a remote host as loopback. It made no current closure claim.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-2ec1ccb-20261004t023807z",
            "finding\u005fid": "P19-AUD-001",
            "source\u005fid": "superseded-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain public non-live actual-consumer route matrix and locked dependency agreement; verify exact current tests if relevant source/dependency bytes change."
      },
      {
        "cause": "Current guard ordering and discriminating large-count test retain the repaired allocation invariant. No broad claim that every earlier observational subprocess is skipped, no extreme reviewer allocation and no unproven regression claim.",
        "current\u005fevidence": "Current core independently found met P19-AC-010-bounds: plan.rs checks cardinality multiplication/max\u005ftrials/addressability before generated per-trial loops; max\u005ftrials\u005frejects\u005flarge\u005fselection\u005fbefore\u005fallocation actually passed in native\u005fdriver. Bounded read-only resource/image observations occur earlier and were separately disclosed.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded P19-AUD-002 reproduced per-trial vector allocation before max\u005ftrials for u32::MAX repetitions, with bounded-memory abort.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt61-2ec1ccb-20261004t023807z",
            "finding\u005fid": "P19-AUD-002",
            "source\u005fid": "superseded-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain checked multiplication/addressability and large-selection public CLI refusal; recompute derived cardinality before any additional costly acquisition/expansion branches."
      }
    ],
    "limitations": "Current independent core, sidecars and verdict are frozen and unchanged. No repair commits or intermediate repaired states were established, so no regression relationship is asserted. Older required six-trial/DeepSWE scope is superseded by current four-trial TB2.1/TB3 scope. Private raw proofs may not be durable shared artifacts; no real credentials or paid/native trials were run. Historical comparison highlights actual Docker cancellation fidelity as a bounded lead for fresh independent proof review beyond the current fixture-qualified process/ownership evidence; no new current finding or old-only defect row is created. History cannot close current findings or qualify missing native/operator/workspace predicates.",
    "schema": "opi-audit-history/1",
    "scope": "Only the immediately superseded same (codex,gpt61) run phase19-codex-gpt61-2ec1ccb-20261004t023807z, moved by this complete to assurance/history/phase19-codex-gpt61-2ec1ccb-20261004t023807z, cited at the commit last carrying its bytes. Only its four finding records and independent report prefix were inspected; its prior advisory appendix/older sources, every other pair/history/remediation record and all sibling output were excluded. Absence claims cover only its four-record finding sidecar.",
    "sources": [
      {
        "audit\u005frun\u005fid": "phase19-codex-gpt61-2ec1ccb-20261004t023807z",
        "commit": "9d805b498a56a276b141cc0c0728ce8201dbe6f3",
        "id": "superseded-findings",
        "path": "docs/snapshots/phase19/assurance/audit.codex.gpt61.findings.jsonl",
        "sha256": "bd454f15884f80e537772452aabc05593d73763e5f4500326edbf13e8d018bc4"
      }
    ],
    "status": "compared"
  },
  "core": {
    "findings\u005fsha256": "85c2bbf73b8b452603fea1969b777219b6f84e05af161230ab8f23c5905f5ad2",
    "meta\u005fsha256": "62ea4686a104f9d7dc232da6350d75c487ca6892413ffc62c560ae8a8c87436b",
    "report\u005fbytes": 39384,
    "report\u005fsha256": "4e43beb801dcfc8ae1d7459916a93483d8eaa23eec37527585ce5767335e7699",
    "requirements\u005fsha256": "7552524d838c8443f4219931507108125b4bc346501217b4f035ed0c77834cda"
  },
  "schema": "opi-audit-history/1"
}
-->

## Post-audit historical reference — advisory

This section was appended after installation of the independent core. It cannot change findings, requirements, independence or verdict. Historical closure claims require current independent verification.

- Owning audit run: phase19&#45;codex&#45;gpt61&#45;ae6dff6&#45;20261014t044949z
- Audit head: ae6dff638db1247203a01483f26a0f81048cb52a
- Core report bytes: 39384
- Core report SHA-256: 4e43beb801dcfc8ae1d7459916a93483d8eaa23eec37527585ce5767335e7699
- Current findings SHA-256: 85c2bbf73b8b452603fea1969b777219b6f84e05af161230ab8f23c5905f5ad2
- Comparison status: compared
- Inspected history: Only the immediately superseded same &#40;codex,gpt61&#41; run phase19&#45;codex&#45;gpt61&#45;2ec1ccb&#45;20261004t023807z, moved by this complete to assurance/history/phase19&#45;codex&#45;gpt61&#45;2ec1ccb&#45;20261004t023807z, cited at the commit last carrying its bytes&#46; Only its four finding records and independent report prefix were inspected; its prior advisory appendix/older sources, every other pair/history/remediation record and all sibling output were excluded&#46; Absence claims cover only its four&#45;record finding sidecar&#46;
- Limitations: Current independent core, sidecars and verdict are frozen and unchanged&#46; No repair commits or intermediate repaired states were established, so no regression relationship is asserted&#46; Older required six&#45;trial/DeepSWE scope is superseded by current four&#45;trial TB2&#46;1/TB3 scope&#46; Private raw proofs may not be durable shared artifacts; no real credentials or paid/native trials were run&#46; Historical comparison highlights actual Docker cancellation fidelity as a bounded lead for fresh independent proof review beyond the current fixture&#45;qualified process/ownership evidence; no new current finding or old&#45;only defect row is created&#46; History cannot close current findings or qualify missing native/operator/workspace predicates&#46;

### Pinned historical sources

- superseded&#45;findings: docs/snapshots/phase19/assurance/audit&#46;codex&#46;gpt61&#46;findings&#46;jsonl; commit 9d805b498a56a276b141cc0c0728ce8201dbe6f3; SHA-256 bd454f15884f80e537772452aabc05593d73763e5f4500326edbf13e8d018bc4; audit run phase19&#45;codex&#45;gpt61&#45;2ec1ccb&#45;20261004t023807z

### Comparison 1: P19&#45;AUD&#45;001

- Relationship: new&#45;omission
- Prior claim (attributed): The immediately superseded four&#45;finding sidecar did not record incomplete delivered&#45;artifact observation admits substituted bytes&#46; This absence claim is limited to that exact sidecar, not older/sibling audits or the time the code defect originated&#46;
- Current evidence: Current independently installed P19&#45;AUD&#45;001 contains current&#45;head source/consumer observations and discriminating proof; its own core remains unchanged&#46;
- Cause / missed boundary: Introduction time is not established&#46; For the walkthrough, the latest source reduction to two families/four trials makes its retained six&#45;trial instruction newly inconsistent; no behavioral regression is inferred from matching IDs&#46;
- Suggested current verification: Use the current finding reproduction/criterion at ae6dff638db1247203a01483f26a0f81048cb52a; do not use historical absence as refutation or closure&#46;

### Comparison 2: P19&#45;AUD&#45;002

- Relationship: new&#45;omission
- Prior claim (attributed): The immediately superseded four&#45;finding sidecar did not record solo invalid trials return completed/exit0 and consistent null repetitions&#46; This absence claim is limited to that exact sidecar, not older/sibling audits or the time the code defect originated&#46;
- Current evidence: Current independently installed P19&#45;AUD&#45;002 contains current&#45;head source/consumer observations and discriminating proof; its own core remains unchanged&#46;
- Cause / missed boundary: Introduction time is not established&#46; For the walkthrough, the latest source reduction to two families/four trials makes its retained six&#45;trial instruction newly inconsistent; no behavioral regression is inferred from matching IDs&#46;
- Suggested current verification: Use the current finding reproduction/criterion at ae6dff638db1247203a01483f26a0f81048cb52a; do not use historical absence as refutation or closure&#46;

### Comparison 3: P19&#45;AUD&#45;004

- Relationship: new&#45;omission
- Prior claim (attributed): The immediately superseded four&#45;finding sidecar did not record all&#45;unknown opi usage is projected as measured&#46; This absence claim is limited to that exact sidecar, not older/sibling audits or the time the code defect originated&#46;
- Current evidence: Current independently installed P19&#45;AUD&#45;004 contains current&#45;head source/consumer observations and discriminating proof; its own core remains unchanged&#46;
- Cause / missed boundary: Introduction time is not established&#46; For the walkthrough, the latest source reduction to two families/four trials makes its retained six&#45;trial instruction newly inconsistent; no behavioral regression is inferred from matching IDs&#46;
- Suggested current verification: Use the current finding reproduction/criterion at ae6dff638db1247203a01483f26a0f81048cb52a; do not use historical absence as refutation or closure&#46;

### Comparison 4: P19&#45;AUD&#45;005

- Relationship: new&#45;omission
- Prior claim (attributed): The immediately superseded four&#45;finding sidecar did not record independent walkthrough remains six&#45;trial and assumes missing local install paths&#46; This absence claim is limited to that exact sidecar, not older/sibling audits or the time the code defect originated&#46;
- Current evidence: Current independently installed P19&#45;AUD&#45;005 contains current&#45;head source/consumer observations and discriminating proof; its own core remains unchanged&#46;
- Cause / missed boundary: Introduction time is not established&#46; For the walkthrough, the latest source reduction to two families/four trials makes its retained six&#45;trial instruction newly inconsistent; no behavioral regression is inferred from matching IDs&#46;
- Suggested current verification: Use the current finding reproduction/criterion at ae6dff638db1247203a01483f26a0f81048cb52a; do not use historical absence as refutation or closure&#46;

### Comparison 5: P19&#45;AUD&#45;003

- Relationship: missed&#45;branch
- Prior claim (attributed): Superseded P19&#45;AUD&#45;004 described the offline verifier implementation as present and requiring identity/commands/frozen inputs, while finding actual operator proof unavailable&#46; It did not independently establish that the tool enforces every required outer source/build/command/budget/platform field; this is not an old closure claim&#46;
- Current evidence: Current core P19&#45;AUD&#45;003 traces Acceptance&#46;run/check&#95;builds/check&#95;command&#95;exits/check&#95;companion and three private metadata probes&#46; Deleting outer source/operator metadata leaves every real Companion argument and family byte unchanged&#46;
- Cause / missed boundary: A different branch of the same immutable execution/freeze invariant is missing: actual external evidence availability and enforcement by the acceptance consumer are separate predicates&#46; No intermediate working enforcement state or later regression was proven&#46;
- Suggested current verification: Add original&#45;proof source/build/argv/exit/frozen scope admission or explicit incomplete predicates at the existing verifier; test absent/malformed fields against an actual already&#45;valid family root&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;2ec1ccb&#45;20261004t023807z / P19&#45;AUD&#45;004; source superseded&#45;findings
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;afd5e0a19d6d4e6784ecd3ffaa90ac89&#92;acceptance&#45;probes&#46;py; SHA-256 3ee38b18cff28f36d53facc018fd721b0d21dbb56740d7035fd36ce526a01919; checked: Inspected the exact private fixture mutations, source&#45;removal argument and bounded local execution handoff&#46;; limits: Fixture/stub checks cannot alone authenticate native runs; the source&#45;removal predicate keeps actual Companion inputs unchanged&#46;

### Comparison 6: P19&#45;AUD&#45;006

- Relationship: evidence&#45;gap
- Prior claim (attributed): Superseded P19&#45;AUD&#45;003 lacked actual family witnesses, all&#45;wire clients, verifier&#45;only regrade and actual Docker cleanup; it also recorded two stale ignored filters and included then&#45;mandatory DeepSWE&#46;
- Current evidence: Current normal native&#95;driver execution is 124 passes/14 ignored, with a PASS execution checker&#46; New supplied Anthropic/real trial raw proof is useful, but native witness/all&#45;wire/regrade/canary/pi&#45;only predicates remain unqualified in the current core&#46; DeepSWE is now explicitly deferred and cannot block current acceptance&#46;
- Cause / missed boundary: Continuing lack of applicable proof for distinct predicates, with changed scope and additional supplied raw observations; no runtime failure or recurrence is inferred from the old title&#46; Old zero&#45;filter claims were not adopted as a fresh current finding&#46;
- Suggested current verification: Use exact current opt&#45;in case identities, original source/build/client/material conditions and original executed counts&#46; Supply missing admitted&#45;family raw proofs without implicitly authorizing live/native trials&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;2ec1ccb&#45;20261004t023807z / P19&#45;AUD&#45;003; source superseded&#45;findings
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;afd5e0a19d6d4e6784ecd3ffaa90ac89&#92;rust&#45;execution&#46;json; SHA-256 326795b089d55fed1e2e9a45919ec5b87814d59895b6fa211c3d39ebf5561ee4; checked: Current complete Cargo log parsed into actual identities/counts; package exit0, 437 passes/14 ignored&#46;; limits: Package fixture success is not actual native execution&#46;
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;afd5e0a19d6d4e6784ecd3ffaa90ac89&#92;raw&#45;inventory&#46;json; SHA-256 32d8b24fda8e7fc90bd9e648728e987c10bc0e18d9b735efa75e45bd292b9f5e; checked: 104 selected supplied raw paths/digests inventoried; four latest bundles and selected Anthropic proof checked&#46;; limits: Selection excludes private auth and audit/remediation/sibling/history conclusions; not a global claim no other proof exists&#46;

### Comparison 7: P19&#45;AUD&#45;007

- Relationship: evidence&#45;gap
- Prior claim (attributed): Superseded P19&#45;AUD&#45;004 had no supplied six&#45;trial root and could not qualify assembled local/platform/workspace proof under its then&#45;registered three&#45;family scope&#46;
- Current evidence: Current core recognizes four real TB2&#46;1/TB3 native outcomes and fresh verify/report results&#46; Provided staged manifest is /1 and rejected by current /2 verifier; complete independent prepare/run handoff, original attempts/platform proof and final workspace union remain explicitly limited&#46;
- Cause / missed boundary: Evidence availability improved and normative scope changed; this is narrower current proof incompleteness, not persistent missing six&#45;trial behavior, and scope reduction is not behavioral repair&#46;
- Suggested current verification: Build a byte&#45;bound current admitted&#45;family /2 handoff with original commands/builds/attempts/platform/gate records; retain scoped&#45;out DeepSWE evidence separately&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;2ec1ccb&#45;20261004t023807z / P19&#45;AUD&#45;004; source superseded&#45;findings
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;afd5e0a19d6d4e6784ecd3ffaa90ac89&#92;provided&#45;root&#45;verification&#46;json; SHA-256 44ae5a7b61e7ba601c8aa660d0937c5669e6aeb3e4d04f4353c52b92f262e18c; checked: Current verifier rejection record read; accepted=false, expected /2 versus supplied /1&#46;; limits: This rejects the staged evidence view, not the existence of the four newer top&#45;level real trials&#46;
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;afd5e0a19d6d4e6784ecd3ffaa90ac89&#92;raw&#45;tb21&#45;report&#46;json; SHA-256 1fa715c4684b65e10ac67b7f632b39a48698418840ec4ace51382ddd16f4318c; checked: Current reporter reconstructs two valid TB2&#46;1 results, rewards1/1, subset/corpus89&#46;; limits: Not complete native witness/current operator handoff/workspace qualification&#46;
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase19&#45;audit&#45;afd5e0a19d6d4e6784ecd3ffaa90ac89&#92;raw&#45;tb30&#45;report&#46;json; SHA-256 ef96acfef560cb259ef7246b1458e0449adc4e6b70679ea01ec9ba9fa41d4a29; checked: Current reporter reconstructs two valid TB3 results, rewards0/0, subset/corpus74&#46;; limits: Valid zeros count; not complete native witness/current operator handoff/workspace qualification&#46;

### Comparison 8: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded P19&#45;AUD&#45;001 reproduced non&#45;live fragment/userinfo authority splitting that accepted a remote host as loopback&#46; It made no current closure claim&#46;
- Current evidence: Current core independently found met P19&#45;AC&#45;006&#45;projection: full URL parsing uses exact locked url2&#46;5&#46;8 with scheme/host matching&#46; Current experiment&#95;contract tests reject fragment/backslash/userinfo/port variants and accept actual loopback IPv4/IPv6; 35 actual test cases passed&#46;
- Cause / missed boundary: Current parser/consumer interpretation and discriminating tests show the previously reported guard defect is repaired at audit&#95;head&#46; Specific repair commit and intermediate state were not reconstructed; repair&#95;commits stays empty&#46;
- Suggested current verification: Retain public non&#45;live actual&#45;consumer route matrix and locked dependency agreement; verify exact current tests if relevant source/dependency bytes change&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;2ec1ccb&#45;20261004t023807z / P19&#45;AUD&#45;001; source superseded&#45;findings

### Comparison 9: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded P19&#45;AUD&#45;002 reproduced per&#45;trial vector allocation before max&#95;trials for u32::MAX repetitions, with bounded&#45;memory abort&#46;
- Current evidence: Current core independently found met P19&#45;AC&#45;010&#45;bounds: plan&#46;rs checks cardinality multiplication/max&#95;trials/addressability before generated per&#45;trial loops; max&#95;trials&#95;rejects&#95;large&#95;selection&#95;before&#95;allocation actually passed in native&#95;driver&#46; Bounded read&#45;only resource/image observations occur earlier and were separately disclosed&#46;
- Cause / missed boundary: Current guard ordering and discriminating large&#45;count test retain the repaired allocation invariant&#46; No broad claim that every earlier observational subprocess is skipped, no extreme reviewer allocation and no unproven regression claim&#46;
- Suggested current verification: Retain checked multiplication/addressability and large&#45;selection public CLI refusal; recompute derived cardinality before any additional costly acquisition/expansion branches&#46;
- Prior finding: phase19&#45;codex&#45;gpt61&#45;2ec1ccb&#45;20261004t023807z / P19&#45;AUD&#45;002; source superseded&#45;findings
