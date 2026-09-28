# Phase 19 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `f4fffdfe386eb7a796e1cc0a13d41e40c976b2e2f37dd1c58d58c614ff41d3c7`
**Remediation head**: `aec029b2226f23dd76d33cc29eeacda2ac78bc77`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: initial admission staged=[]; unstaged=[]; untracked=[]; committed=none-for-this-task
**Continuation carried-in paths**: only this task-owned uncommitted fixed plan pair; no unrelated overlap; source HEAD unchanged
**Unresolved decisions**: none

## Admission and scope

The revision-10 active index validates; aggregate verdict remains `FAIL`.
This plan covers exactly 15 source keys from both peers. Textual IDs are
qualified by the full run below. No vote, historical closure label, or alternate
plan supplies membership or closure. Every source severity is retained.

| Alias | Full audit run | Findings SHA-256 | Audit head |
|---|---|---|---|
| C | `phase19-codex-gpt6-3e8fee6-20260928t050232z` | `ccf574271b308b2bb72e13df1721824a9bda153e08b813e0679320b65ceb4ca1` | `3e8fee6075c719ca0c3192790b800c13421e4fc4` |
| G | `phase19-pi-glm53-ac80281-20260928t070222z` | `8e47b6f1d2a5d74d6514568731c03e7a4ba99f31c638de369c6125a91f7b8435` | `ac802811f2f0216ceddc9d802906570d9f4d67e8` |

Each alias resolves only to that exact current run/digest. Title similarity is
not identity; an older source cannot enter this finding union. The original
metadata, requirement, finding and report bytes remain bound by the index.
There is no severity downgrade based on a peer's contrary requirement state.

The source evidence baseline is the committed head above. Inspection and
plan-stage behavior probes used separate unique `git archive` exports, never
a Git worktree. Only test code was changed in those exports. The workspace
plan pair replaces its clean tracked predecessors without reading them; no
production changes, ledger changes, commits or materialized results are part
of this planning invocation. Fixed assurance plans have no localized
counterpart in the owning contract; proposed maintained product documents
below update English/Chinese together. The local walkthrough has no Chinese
counterpart.

The main specification and registered Phase19 English/Chinese sources still
match both indexed baseline hashes. Product sources under `crates/` and
Cargo manifests/lock are unchanged since either audit head; current assurance
and documentation checker changes are verified at this remediation head.

Observable completion: validate the live set -> verify every current finding
at this head -> define one predicate per independent repair -> preserve and
hash observed red evidence -> validate the fixed disposition coverage and
plan -> run document and whitespace checks. This ready plan contains bounded
repairs and explicit evidence exclusions. Readiness admits those repairs only;
C/011, C/013 and G/001 remain unclosed and retain their Phase-exit effects.

## Current Finding Verification

| Source / finding | Verification | Source -> final severity and rationale | Closure key / family | Batch | Decision |
|---|---|---|---|---|---|
| C / P19-AUD-001 | Confirmed | Major -> Major; Independently reproduced the current mandatory invariant failure; no severity change. | native.selected-credential-preseal-guard / native.privacy | B1 | fix:automatically-guard-selected-credential-before-seal |
| C / P19-AUD-002 | Confirmed | Major -> Major; Independently reproduced the current mandatory invariant failure; no severity change. | replacement.every-executed-trial-fresh / run.replacement | B2 | fix:fresh-trial-id-for-every-replacement-execution |
| C / P19-AUD-003 | Confirmed | Major -> Major; Independently reproduced the current mandatory invariant failure; no severity change. | material.installed-runner-byte-pins / native.material-identity | B3 | fix:bind-and-reverify-installed-runner-entrypoints |
| C / P19-AUD-004 | Confirmed | Major -> Major; Independently reproduced the current mandatory invariant failure; no severity change. | native.observed-task-reference-binding / native.task-identity | B4 | fix:compare-observed-and-dispatched-task-package-reference |
| C / P19-AUD-005 | Confirmed | Major -> Major; Current evidence confirms the source impact; source severity retained. | acceptance.admitted-material-provenance / native.evidence-admission | B5 | fix:bind-acceptance-provenance-to-admitted-material |
| C / P19-AUD-006 | Confirmed | Major -> Major; Current evidence confirms the source impact; source severity retained. | controls.required-reasoning-before-effects / native.control-admission | B6 | fix:reject-required-native-reasoning-before-effects |
| C / P19-AUD-007 | Confirmed | Major -> Major; Current evidence confirms the source impact; source severity retained. | plan.frozen-relative-material-base / native.frozen-inputs | B7 | fix:freeze-canonical-native-material-path |
| C / P19-AUD-008 | Confirmed | Minor -> Minor; Current evidence confirms the source impact; source severity retained. | ci.shared-preparation-admission / native.material-preparation | B8 | fix:delegate-ci-materialization-to-local-prepare |
| C / P19-AUD-009 | Confirmed | Minor -> Minor; Current evidence confirms the source impact; source severity retained. | docs.local-prepare-walkthrough / native.user-contract | B9 | fix:correct-local-plan-command-and-preparation-ownership |
| C / P19-AUD-010 | Partially confirmed | Major -> Major; Mandatory evidence priority retained. Current exact-client and actual cleanup proofs fill the audited evidence limitation at the baseline; no production defect was reproduced. Planned source/material changes require final-source evidence refresh. | native.client-and-cleanup-current-evidence / native.real-evidence | B11 | fix:retain-actual-grader-cleanup-observation-and-refresh-current-client-proof |
| C / P19-AUD-011 | Confirmed | Major -> Major; Mandatory original pinned benchmark-native offline regrade execution has no admitted qualifying witness; current test provides diagnostic mechanism coverage only. | native.regrade-pinned-saved-artifacts / native.real-evidence | explicit exclusion | no-action:Cannot-confirm:original-pinned-regrade-inputs-unavailable |
| C / P19-AUD-012 | Partially confirmed | Major -> Major; Retain mandatory source evidence priority. The audit's missing-proof limitation is now independently supplied for current HEAD; the finding never alleged a reproduced production failure. Planned runtime/material changes will invalidate current proof unless relevant dependencies are shown unchanged. | native.three-family-original-lifecycle-witness / native.real-evidence | B12 | fix:refresh-final-source-native-lifecycle-evidence |
| C / P19-AUD-013 | Partially confirmed | Major -> Major; Formal six-trial acceptance and final relevant source gate union still cannot be established from exact admitted candidates; records do not prove no trials occurred. | acceptance.six-trials-and-final-gates / acceptance.real-evidence | explicit exclusion | no-action:Cannot-confirm:six-trial-acceptance-inputs-unavailable |
| G / P19-AUD-001 | Confirmed | Minor -> Minor; Reproducibility/retention gap is confirmed within bounded /home/luiz location; no native outcome corruption established. | acceptance.staged-evidence-retention / acceptance.real-evidence | explicit exclusion | no-action:Cannot-confirm:staged-root-inputs-unavailable |
| G / P19-AUD-002 | Confirmed | Minor -> Minor; Current fail-closed report contract is intentional and correct; documented retained-attempt layout lacks a reproducible per-execution staging instruction. | report.retained-attempt-staging-documentation / report.reconstruction | B10 | fix:document-per-execution-read-only-staging |

## Unresolved Decisions

None. The user confirmed there is no additional six-trial evidence root.
The bounded original-material regrade attempts and direct dependency inspection
also establish an input gap, not a production defect. C/011, C/013 and G/001
are explicit `Cannot confirm` exclusions, not open design choices, registered
deferrals or acceptance waivers. Their original severity and blocking effects
remain. Restoring/replacing their evidence requires separate input admission;
any new real-provider execution requires explicit scope/budget authorization.

Current exact-client/cleanup and original-material lifecycle evidence is
recorded in B11/B12. Those batches refresh the affected evidence after source
repairs; no earlier PASS is treated as future green-after.

## Closure Batches

Each batch has one root correction and one falsifiable closure predicate.
Shared files or family names do not merge batches. Execute sequentially in
the listed order so shared runner/tests/docs edits remain attributable.
All Major source repairs B1-B7 and evidence batches B11/B12 precede Minor
repairs B8-B10; numeric identifiers are stable references, not priority.
Execution order is B1-B7, B11, B12, B8-B10. B8 uses B3's final material
contract. Recheck B11/B12 only if later edits invalidate their dependencies. Every permanent
new or modified test/filter must actually execute, with counts recorded.

### Batch B1: Selected credentials echoed into captures bypass the default seal guard

**Closure predicate**: Without --canaries, an exact selected credential echoed in any staged native export prevents sealing and stops later dispatch; diagnostics remain private and the secret is never serialized into a guard list.
**Dependencies**: none
**Verification union**: matching discriminating check below, affected test/script families, and the deduplicated final union.

#### Fix B1.1: automatically-guard-selected-credential-before-seal

- **Finding source(s)**: `phase19-codex-gpt6-3e8fee6-20260928t050232z` / `ccf574271b308b2bb72e13df1721824a9bda153e08b813e0679320b65ceb4ca1` / `P19-AUD-001`
- **Decision**: fix:automatically-guard-selected-credential-before-seal
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Use the existing fail-closed seal/cancel mechanism, adding the resolved credential value to a trial-local in-memory scan set before project_connection consumes it. Preserve declared canaries. Match the two enum variants inside runner; no public secret accessor is needed. Test both env/file references and both clients during apply. Original p19-001-003-red.log is INVALID for 001 because it checked path names; only final-red log is admissible.
- **Closure predicate**: Without --canaries, an exact selected credential echoed in any staged native export prevents sealing and stops later dispatch; diagnostics remain private and the secret is never serialized into a guard list.
- **Red-before**: `cargo test --offline --locked -p opi-eval --test native_driver remediation_p19_ -- --nocapture` — FAIL: Exit 101; native_driver ran 2 tests, 2 failed. This test independently exercised job-stdout and agent-stderr: each run code=0/status=sealed, manifest exists, exact selected dummy bytes observed in bundle. Desired absence of sealed secret failed. Log `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/bindings/p19-001-003-final-red.log`, SHA-256 `7f26b9f363ba2468c3d98b89f961cfdd73ea81de692536740f72be34120c693e`.
- **Green-after**: `cargo test --offline --locked -p opi-eval --test native_driver remediation_p19_selected_credential_echo_is_never_sealed -- --exact --nocapture` — PASS: Without --canaries, an exact selected credential echoed in any staged native export prevents sealing and stops later dispatch; diagnostics remain private and the secret is never serialized into a guard list. Observed: not-run.
- **Test impact**: {"add": ["remediation_p19_selected_credential_echo_is_never_sealed plus env/file credential and both-client coverage"], "retain": ["explicit-canary rejection/cancel test", "credential cleanup tests", "capture policy tests"], "delete": []}

Current evidence:

- crates/opi-eval/src/cli/run.rs:224-227: no canary argument returns an empty list.
- crates/opi-eval/src/runner/experiment.rs:1867-1875: selected credential resolves for projection only.
- crates/opi-eval/src/runner/experiment.rs:2271-2316: full Agent/native streams copied into staged artifacts.
- crates/opi-eval/src/runner/experiment.rs:2530-2551: only request.canaries guard sealing.
- crates/opi-eval/src/runner/experiment.rs:3481-3498: stage classifies passed bytes Exportable.
- docs/opi-spec.md:259 CTRL-003 and registered Phase19 sections 4,6 / AC-010 require secret exclusion.

Counterevidence boundary: The source finding counterevidence was rechecked against the current paths and desired-behavior probes; existing adjacent tests do not reject the demonstrated mutation.

### Batch B2: Replacement reruns other groups under their old trial identities

**Closure predicate**: Every trial actually re-executed by replacement has a fresh trial ID, including retained unrelated groups; frozen selection/controls and original source/run bytes remain unchanged.
**Dependencies**: none
**Verification union**: matching discriminating check below, affected test/script families, and the deduplicated final union.

#### Fix B2.1: fresh-trial-id-for-every-replacement-execution

- **Finding source(s)**: `phase19-codex-gpt6-3e8fee6-20260928t050232z` / `ccf574271b308b2bb72e13df1721824a9bda153e08b813e0679320b65ceb4ca1` / `P19-AUD-002`
- **Decision**: fix:fresh-trial-id-for-every-replacement-execution
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/tests/assembled_run.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Move the trial.id fresh-token assignment outside the target-group conditional; keep existing targeted-group suffix logic and whole-scope dispatch. Filtering unrelated groups would alter declared replacement scope and is unnecessary. Update paired READMEs to explicitly state retained groups execute with fresh trial IDs.
- **Closure predicate**: Every trial actually re-executed by replacement has a fresh trial ID, including retained unrelated groups; frozen selection/controls and original source/run bytes remain unchanged.
- **Red-before**: `cargo test --offline --locked -p opi-eval --test assembled_run replacement_fresh_resolved_pairs_preserve_original_scope -- --exact --nocapture` — FAIL: Exit 101; 1 executed, 1 failed. Replacement reused trial-opi-1-other in unrelated-group; assertion rejected reuse before remaining comparisons. Log `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/bindings/p19-002-final-red.log`, SHA-256 `b57d7396dbbbde663422d79a5eba73f5f5a131c7d2163bd5308147ce2eaf92c2`.
- **Green-after**: `cargo test --offline --locked -p opi-eval --test assembled_run replacement_fresh_resolved_pairs_preserve_original_scope -- --exact --nocapture` — PASS: Every trial actually re-executed by replacement has a fresh trial ID, including retained unrelated groups; frozen selection/controls and original source/run bytes remain unchanged. Observed: not-run.
- **Test impact**: {"update": ["replacement_fresh_resolved_pairs_preserve_original_scope: expect fresh IDs for all rows"], "retain": ["target-group identity, original bytes, repeated replacement, frozen selected-plan checks"], "delete": []}

Current evidence:

- crates/opi-eval/src/runner/experiment.rs:767-795 dispatches all retained experiment trials.
- crates/opi-eval/src/runner/experiment.rs:3950-3956 refreshes trial IDs only for the targeted group.
- crates/opi-eval/tests/assembled_run.rs:44-209 existing multi-group integration expects old unrelated IDs and reads newly sealed bundles.
- docs/opi-spec.md:306 CTRL-006 requires a new trial identity for new real execution.

Counterevidence boundary: The source finding counterevidence was rechecked against the current paths and desired-behavior probes; existing adjacent tests do not reject the demonstrated mutation.

### Batch B3: Frozen material omits the installed runner interpreter identity

**Closure predicate**: Prepared material carries exact selected interpreter and runner console-entrypoint byte identities; missing or changed bytes reject material loading before validate/plan/run effects and before accepting frozen plans.
**Dependencies**: none
**Verification union**: matching discriminating check below, affected test/script families, and the deduplicated final union.

#### Fix B3.1: bind-and-reverify-installed-runner-entrypoints

- **Finding source(s)**: `phase19-codex-gpt6-3e8fee6-20260928t050232z` / `ccf574271b308b2bb72e13df1721824a9bda153e08b813e0679320b65ceb4ca1` / `P19-AUD-003`
- **Decision**: fix:bind-and-reverify-installed-runner-entrypoints
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/runner/material.rs`, `crates/opi-eval/src/runner/prepare.rs`, `crates/opi-eval/src/runner/plan.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/scripts/native-smoke.sh`, `crates/opi-eval/scripts/verify-native-smoke-artifact.py`, `crates/opi-eval/scripts/test_verify_native_smoke_artifact.py`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `crates/opi-eval/examples/local-evaluation/README.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Concrete format decision: advance crate-private native material schema /5 -> /6, require RunnerProject.interpreter and RunnerProject.entrypoint PinnedFile records, emit from the already verified inventory entries, enforce their selected paths and verify_pinned hashes on load. Old material rejects with regenerate guidance; no compatibility fallback. Synchronize all current /5 producers/consumers listed, fixture strings, paired READMEs and Unreleased. The examples directory has no Chinese counterpart; explain that. This binds already inventoried executable bytes, not the whole installed Python dependency tree (outside this finding). No new spec/authority is needed for existing identity requirement.
- **Closure predicate**: Prepared material carries exact selected interpreter and runner console-entrypoint byte identities; missing or changed bytes reject material loading before validate/plan/run effects and before accepting frozen plans.
- **Red-before**: `cargo test --offline --locked -p opi-eval --test native_driver remediation_p19_ -- --nocapture` — FAIL: Exit 101; native_driver ran 2 tests, 2 failed. This test independently changed python and harbor console bytes from 306c6ca7407560340797866e077e053627ad409277d1b9da58106fce4cf717cb to 36718429c1772ec4af3554245a845958f3c50b23e656a4da16b1f9f795640963. Both baseline and changed validate exited 0 with equal native_integrity within each case. Desired rejection failed. Log `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/bindings/p19-001-003-final-red.log`, SHA-256 `7f26b9f363ba2468c3d98b89f961cfdd73ea81de692536740f72be34120c693e`.
- **Green-after**: `cargo test --offline --locked -p opi-eval --test native_driver remediation_p19_runner_installed_bytes_are_reverified -- --exact --nocapture` — PASS: Prepared material carries exact selected interpreter and runner console-entrypoint byte identities; missing or changed bytes reject material loading before validate/plan/run effects and before accepting frozen plans. Observed: not-run.
- **Test impact**: {"add": ["remediation_p19_runner_installed_bytes_are_reverified; extend to Harbor/Pier and stale-format/missing-pin cases"], "update": ["material format producer/consumer fixtures, prepare and frozen-plan conformance"], "retain": ["source/lock drift and UV path checks"], "delete": []}

Current evidence:

- crates/opi-eval/src/runner/prepare.rs:1199-1205 inventories interpreter and console entrypoint.
- crates/opi-eval/src/runner/prepare.rs:1773-1776 emits only runner project path and source digest.
- crates/opi-eval/src/runner/material.rs:128-187 RunnerProject verifies tracked source and UV path strings but no installed bytes.
- crates/opi-eval/src/runner/material.rs:351-355 load_for invokes that limited verification.
- crates/opi-eval/src/runner/material.rs:1293-1330 computes digest from tracked Git tree, excluding ignored venv.
- Registered Phase19 section 5 / P19-AC-007 requires verified runner/grader material identity and drift rejection.

Counterevidence boundary: The source finding counterevidence was rechecked against the current paths and desired-behavior probes; existing adjacent tests do not reject the demonstrated mutation.

### Batch B4: Native result admission does not bind the observed task package reference

**Closure predicate**: A substituted observed absolute task package reference cannot be admitted as the selected dispatch or sealed, even when result.task_name and lifecycle fields still match.
**Dependencies**: none
**Verification union**: matching discriminating check below, affected test/script families, and the deduplicated final union.

#### Fix B4.1: compare-observed-and-dispatched-task-package-reference

- **Finding source(s)**: `phase19-codex-gpt6-3e8fee6-20260928t050232z` / `ccf574271b308b2bb72e13df1721824a9bda153e08b813e0679320b65ceb4ca1` / `P19-AUD-004`
- **Decision**: fix:compare-observed-and-dispatched-task-package-reference
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/benchmark/process.rs`, `crates/opi-eval/tests/native_driver.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Minimum root fix: native_stage_config_matches compares observed task.path to the exact absolute dispatched expected.tasks[0].path (reject missing/nonstring/mismatch). Existing completion/runner failure mapping closes sealing. No NativeSelection public/format expansion is required. Apply adds end-to-end mutation coverage to verify both ordinary completed and setup-refusal paths; do not broaden into whole post-execution package rehashing or unrelated task lifecycle refactoring.
- **Closure predicate**: A substituted observed absolute task package reference cannot be admitted as the selected dispatch or sealed, even when result.task_name and lifecycle fields still match.
- **Red-before**: `cargo test --offline --locked -p opi-eval --lib remediation_p19_observed_task_substitution_rejected -- --nocapture` — FAIL: Exit 101; 1 executed, 1 failed. Other absolute package contains task name different-task and new test.sh bytes, retains same lifecycle; importer_accepted=true and post_settlement_config_matches=true. Combined desired rejection assertion failed. Log `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/bindings/p19-004-final-red.log`, SHA-256 `cd6276132ccfe46ff86e09899d35921689c238853ef611a8773c6b7d5ba0a918`.
- **Green-after**: `cargo test --offline --locked -p opi-eval --lib remediation_p19_observed_task_substitution_rejected -- --nocapture` — PASS: A substituted observed absolute task package reference cannot be admitted as the selected dispatch or sealed, even when result.task_name and lifecycle fields still match. Observed: not-run.
- **Test impact**: {"add": ["remediation_p19_observed_task_substitution_rejected in process.rs", "CLI native_driver substituted-task-reference no-seal test including setup-refusal branch"], "retain": ["pre-dispatch package drift and result task label rejection tests"], "delete": []}

Current evidence:

- crates/opi-eval/src/benchmark/process.rs:1043-1116 parses an arbitrary absolute observed task path and returns only lifecycle fields.
- crates/opi-eval/src/benchmark/process.rs:1636-1665 compares stage environment/timeouts but omits task path.
- crates/opi-eval/src/benchmark/process.rs:1773-1783 already maps configuration mismatch to integrity failure.
- crates/opi-eval/src/runner/experiment.rs:2122-2140 excludes rejected completion from binding/setup-refusal exemption; 2541-2548 refuses seal without binding.
- Registered Phase19 section 5 / P19-AC-002 requires binding the same selected Agent/task/trial evidence.

Counterevidence boundary: The source finding counterevidence was rechecked against the current paths and desired-behavior probes; existing adjacent tests do not reject the demonstrated mutation.

### Batch B5: Acceptance provenance trusts an unbound operator cache and optional inventories

**Closure predicate**: Complete existing admitted evidence passes offline; source/cache substitutions or missing/duplicate/mismatched per-family material, preserved lock, or sealed integrity association fail before accepted=true or Companion invocation. Sources are repository-qualified identities derived from the admitted lock bytes, not mutual agreement of supplied maps.
**Dependencies**: none
**Verification union**: matching discriminating check below, affected test/script families, and the deduplicated final union.

#### Fix B5.1: bind-acceptance-provenance-to-admitted-material

- **Finding source(s)**: `phase19-codex-gpt6-3e8fee6-20260928t050232z` / `ccf574271b308b2bb72e13df1721824a9bda153e08b813e0679320b65ceb4ca1` / `P19-AUD-005`
- **Decision**: fix:bind-acceptance-provenance-to-admitted-material
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/scripts/verify-local-acceptance.py`, `crates/opi-eval/scripts/test_verify_local_acceptance.py`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: At the existing acceptance admission root, require exactly one material entry for each trial family and exhaustive required preserved material/lock/evidence references with unique paths. Recompute material and preserved file hashes. Locate preserved admitted static-lock bytes by the existing material.static_lock pin, require its digest to agree with each family sealed control/integrity upstream_identity, and derive pi source identity from lock subjects plus Harbor/Pier identities from existing locked tools/canonical acquisition routes. Treat upstream identity cache as evidence only, never authority. Reject missing, duplicate, substituted or cross-family rows before acceptance. No new external trust input or artifact authority.
- **Closure predicate**: Complete existing admitted evidence passes offline; source/cache substitutions or missing/duplicate/mismatched per-family material, preserved lock, or sealed integrity association fail before accepted=true or Companion invocation. Sources are repository-qualified identities derived from the admitted lock bytes, not mutual agreement of supplied maps.
- **Red-before**: `python3 /mnt/c/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/controls/probe_acceptance.py` — FAIL: WSL/Linux: Ran 5 tests; baseline PASS; coordinated source/cache substitution, empty material, empty preserved, combined all FAIL because public verifier exited 0 accepted=true. Recording Companion stub only; not native acceptance. Log `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/controls/acceptance-red.log`, SHA-256 `34dabb61dd60eea30771d3f6f977a987f9641bedde8f8fa408946bb05c7efac3`.
- **Green-after**: `python3 crates/opi-eval/scripts/test_verify_local_acceptance.py; python3 /mnt/c/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/green/controls/probe_acceptance.py` — Existing fully bound positive passes; each of four current negative cases rejects; add lock-digest/cross-family/duplicate/tamper negatives using the same fixture with admitted lock/sealed integrity rows. Observed: not-run.
- **Test impact**: {"add": "coordinated source/cache substitution, missing/duplicate inventories and material-lock-sealed-integrity identity rejection cases", "update": "EvidenceRoot positives to include preserved admitted lock and sealed integrity associations", "retain": "existing control, runtime, tool-loop, scoped-view and Companion invocation isolation checks"}

Current evidence:

- crates/opi-eval/scripts/verify-local-acceptance.py:216: material list has no mandatory per-family cardinality
- crates/opi-eval/scripts/verify-local-acceptance.py:282: preserved list is optional; empty loops succeed
- crates/opi-eval/scripts/verify-local-acceptance.py:700: upstream source records compared only to operator cache
- crates/opi-eval/src/runner/experiment.rs:1191: current material static_lock.sha256 becomes integrity.upstream_identity
- crates/opi-eval/src/runner/experiment.rs:1942: control/integrity.json is sealed
- crates/opi-eval/src/runner/prepare.rs:1067: existing pinned static lock owns local authority
- crates/opi-eval/src/runner/material.rs:135: RunnerProject verifies source commit and uv.lock identities against that static lock

Counterevidence boundary: Current mismatch/unknown-cache tests reject only one-sided edits. Byte seals and exact runtime hashes protect other surfaces but never consume the supplied identity cache; fresh coordinated edit still passes.

### Batch B6: Required reasoning can run with unsupported effective control

**Closure predicate**: For Opi-only, pi-only and paired native selections across all three protocols, required reasoning=high rejects at validate, plan and run before Agent/verifier dispatch or run/plan artifact publication; optional unsupported reasoning remains explicitly non-comparable.
**Dependencies**: none
**Verification union**: matching discriminating check below, affected test/script families, and the deduplicated final union.

#### Fix B6.1: reject-required-native-reasoning-before-effects

- **Finding source(s)**: `phase19-codex-gpt6-3e8fee6-20260928t050232z` / `ccf574271b308b2bb72e13df1721824a9bda153e08b813e0679320b65ceb4ca1` / `P19-AUD-006`
- **Decision**: fix:reject-required-native-reasoning-before-effects
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/experiment.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Extend the existing pre-effect native-control admission to reject required reasoning values while current native execution cannot prove that control effective. Apply through existing validate/plan/run callers. Preserve optional unsupported reasoning reporting and existing configured client launches; do not implement a new reasoning capability or proof protocol.
- **Closure predicate**: For Opi-only, pi-only and paired native selections across all three protocols, required reasoning=high rejects at validate, plan and run before Agent/verifier dispatch or run/plan artifact publication; optional unsupported reasoning remains explicitly non-comparable.
- **Red-before**: `cargo test --offline -p opi-eval --lib current_required_reasoning_rejects_before_native_admission -- --nocapture` — FAIL: Windows x86_64: 1 test executed; 0 passed, 1 failed, 0 ignored, 113 filtered; all 9 protocol x paired/Opi/Pi configurations admitted required reasoning=high. Log `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/controls/reasoning-red.log`, SHA-256 `a21c8ef7a0bc090dff463a4201416318e2e98fafef97109e957cbbf5b8f259d3`.
- **Green-after**: `cargo test --offline --locked -p opi-eval --lib current_required_reasoning_rejects_before_native_admission -- --nocapture; cargo test --offline --locked -p opi-eval --test native_driver required_reasoning -- --nocapture` — Same 9-case gate probe passes, plus public validate/plan/run dispatch-marker assertions; no production provider or real credential use. Observed: not-run.
- **Test impact**: {"add": "supplied required reasoning gate regression and public-entry no-effects matrix", "retain": "existing required omitted reasoning, supported/unsupported sampling and output-control proof cases"}

Current evidence:

- crates/opi-eval/src/experiment.rs:760: required reasoning supplied as any Value passes resolve
- crates/opi-eval/src/experiment.rs:1017: native admission consults only unsupported sampling
- crates/opi-eval/src/runner/output_control.rs:34: additional admission covers only output limits
- crates/opi-eval/integrations/harbor/opi_eval_harbor.py:810: Pi command disables thinking
- crates/opi-eval/integrations/pier/opi_eval_pier.py:855: Pi command disables thinking
- crates/opi-eval/src/runner/experiment.rs:2568: all supplied native reasoning marked unsupported only after execution/sealing

Counterevidence boundary: Required omitted reasoning, sampling and missing output proof already reject; those tests do not exercise a supplied high reasoning value. Post-settlement non-comparability occurs too late.

### Batch B7: Frozen relative material paths change base at execution

**Closure predicate**: A public plan created with relative native material and nested output loads that same unchanged material by default and completes its deterministic native fixture run from both the creation directory and unrelated cwd; existing missing/tampered/substituted material failures remain.
**Dependencies**: none
**Verification union**: matching discriminating check below, affected test/script families, and the deduplicated final union.

#### Fix B7.1: freeze-canonical-native-material-path

- **Finding source(s)**: `phase19-codex-gpt6-3e8fee6-20260928t050232z` / `ccf574271b308b2bb72e13df1721824a9bda153e08b813e0679320b65ceb4ca1` / `P19-AUD-007`
- **Decision**: fix:freeze-canonical-native-material-path
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/runner/plan.rs`, `crates/opi-eval/tests/native_driver.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Canonicalize the accepted material path at plan creation before loading/hashing/recording it; preserve exact material bytes and existing drift checks. No new config flag/schema or fallback path guessing.
- **Closure predicate**: A public plan created with relative native material and nested output loads that same unchanged material by default and completes its deterministic native fixture run from both the creation directory and unrelated cwd; existing missing/tampered/substituted material failures remain.
- **Red-before**: `cargo test --offline --locked -p opi-eval --test native_driver current_ -- --nocapture` — FAIL: WSL/Linux: exit 101; 2 tests executed; 0 passed, 2 failed, 0 ignored, 116 filtered. Relative material froze successfully then default frozen run failed exit2 ENOENT from both same and unrelated cwd. Walkthrough command failed exit2 requires --native-material. Log `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/controls/plan-red.log`, SHA-256 `e4cb79937ba7848a9753f926c65e832f53ac6c6772f54b77a04b36b9dd3a6648`.
- **Green-after**: `cargo test --offline --locked -p opi-eval --test native_driver current_relative_material_survives_nested_frozen_plan -- --exact --nocapture` — 1 executed test passes; nested plan and both same/unrelated cwd deterministic fixture runs succeed. Observed: not-run.
- **Test impact**: {"add": "nested relative native-material plan/run from same and unrelated cwd", "retain": "native_frozen_plan_runs_without_fixtures_and_rejects_material_drift"}

Current evidence:

- crates/opi-eval/src/runner/plan.rs:540: producer preserves caller relative path
- crates/opi-eval/src/runner/plan.rs:603: stored material path and digest emitted verbatim
- crates/opi-eval/src/runner/plan.rs:734: consumer interprets relative path from plan parent
- crates/opi-eval/src/cli/run.rs:96: default frozen run uses that consumer resolution

Counterevidence boundary: Existing unrelated-cwd frozen-plan regression stages an absolute material path; explicit override succeeds but does not repair frozen default lookup.

### Batch B11: Native client and real cleanup evidence remains unassessed

**Closure predicate**: All four commands actually execute one test each and pass; exact two-client/three-protocol/five-scenario matrix has 30 attributable cases; ten shared-grader native client trials preserve success/negative failure distinctions without scoring provider failures, and two separately attributed TB3 diagnostic trials exercise the separate grader; actual shared and separate graders observe no selected dummy credential or Agent staging, exported bundles contain no selected credential, and native cancellation removes only owned resources while neighboring resources survive. Preserve source/build/client/material identities and counts; remote service-model/price origin remains explicitly unknown where not observed.
**Dependencies**: B1, B3, B4 and B6, after B1-B7 final runtime bytes; rerun affected cases if later edits invalidate dependencies
**Verification union**: matching discriminating check below, affected test/script families, and the deduplicated final union.

#### Fix B11.1: retain-actual-grader-cleanup-observation-and-refresh-current-client-proof

- **Finding source(s)**: `phase19-codex-gpt6-3e8fee6-20260928t050232z` / `ccf574271b308b2bb72e13df1721824a9bda153e08b813e0679320b65ceb4ca1` / `P19-AUD-010`
- **Decision**: fix:retain-actual-grader-cleanup-observation-and-refresh-current-client-proof
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-eval/tests/native_driver.rs`
- **Change kind**: test-only
- **Change**: Promote the retained test-only actual grader observation and export scan into native_client_provider_failure_report, and refresh its exact source-identity precondition from independently verified client provenance. Keep all existing behavioral assertions. The observer uses the dummy credential hash rather than putting secret bytes in grader source; it checks environment value absence, removed client staging, removed host handoff and bundle bytes. Bind the final actual source tree (including applied uncommitted changes) in a complete source manifest, with separate Companion/test/Opi binary hashes and selected pi launcher/node/package hashes. A source_commit records the committed base; editing that field alone is never evidence of a build. Reuse baseline Opi only if its complete relevant source/dependency closure is proven unchanged, otherwise rebuild; no new runtime configuration or weaker provenance check. After B3 regenerate fresh final-format material through the final producer without rewriting old manifests, then execute all four exact tests and retain raw roots, counts and cleanup evidence. This is test/evidence repair only; diagnostic grader machinery is explicitly limited to client/cleanup observation and cannot qualify B12 or original benchmark regrade. For separate-grader replay, copy continuation-d1/run_separate_cleanup.py into a new evidence directory; rebind only its root, final binary and final generated diagnostic-source/material paths, preserving every observation/assertion. Derive its retry-success diagnostic task from the final shared test output, preserve the separate environment marker assertion, and record the adapted script digest. Never rerun the hardcoded baseline script over frozen evidence. Expected diagnostic aggregate run exit is 1 for the unsupported optional control, report exit 0, with both native trial observations graded and clean; an incomplete pair cannot be claimed as formal acceptance.
- **Closure predicate**: All four commands actually execute one test each and pass; exact two-client/three-protocol/five-scenario matrix has 30 attributable cases; ten shared-grader native client trials preserve success/negative failure distinctions without scoring provider failures, and two separately attributed TB3 diagnostic trials exercise the separate grader; actual shared and separate graders observe no selected dummy credential or Agent staging, exported bundles contain no selected credential, and native cancellation removes only owned resources while neighboring resources survive. Preserve source/build/client/material identities and counts; remote service-model/price origin remains explicitly unknown where not observed.
- **Red-before**: N/A — Test/evidence-only correction: prior source-identity fixture is stale and actual grader absence observation was missing; baseline executed proofs now pass with test-only instrumentation. No production failure is asserted and no behavioral edit is proposed by this source key.
- **Green-after**: `With verified final-source Opi/pi client manifests and prepared native material, execute each exact filter with cargo test --offline --locked -p opi-eval --test native_driver <filter> -- --ignored --exact --nocapture --test-threads=1: native_protocol_client_matrix; native_client_provider_failure_report; native_conformance_owned_cleanup_witness; native_owned_resource_cancellation. Set OPI_EVAL_NATIVE_CLIENT_MATERIAL and OPI_EVAL_NATIVE_WITNESS_MATERIAL to separately preserved, verified current inputs; use only dummy credentials and local scripted servers. Also replay the retained two-client TB3 separate-grader diagnostic CLI probe with freshly regenerated final-format material and the same explicit environment/staging/export absence assertions.` — All four commands actually execute one test each and pass; exact two-client/three-protocol/five-scenario matrix has 30 attributable cases; ten shared-grader native client trials preserve success/negative failure distinctions without scoring provider failures, and two separately attributed TB3 diagnostic trials exercise the separate grader; actual shared and separate graders observe no selected dummy credential or Agent staging, exported bundles contain no selected credential, and native cancellation removes only owned resources while neighboring resources survive. Preserve source/build/client/material identities and counts; remote service-model/price origin remains explicitly unknown where not observed. Observed: not-run.
- **Test impact**: update native_client_provider_failure_report test assertions and exact client provenance precondition; retain native_protocol_client_matrix, native_conformance_owned_cleanup_witness and native_owned_resource_cancellation; add/delete test binaries none

Current evidence:

- Current Opi 0.8.2 source-bound static binary b0e6929bec763b222e5625e92f2e504d785d962217b6e1b487f8cc4eee9c3262 and unchanged pinned pi 0.85.1 completed 30 loopback cases (2 clients x 3 protocols x 5 scenarios). All four ignored-test commands executed exactly one test and passed, zero ignored; 953-file source manifest has only the retained native_driver.rs test instrumentation delta.
- Ten actual native shared-grader trial observations and two separately dispatched TB3 diagnostic trials attest selected dummy credential and client staging absence. Separate grader also rejects an Agent-environment-only marker; both raw trial results report separate mode and graded-native reward 1. Host handoffs removed; independent parent scan finds no dummy bytes in exported separate bundles.
- Separate diagnostic run exits 1 for its explicitly unsupported optional control/incomplete pair; report exits 0 and both attributable native trials grade 1. The helper initially expected exit 2, corrected only that orchestration assertion and resumed read-only/report checks without rerunning or changing trial results. No production red is inferred.
- Native real-runner timeout and actual-resource SIGTERM tests pass with trial-owned cleanup and neighboring-resource survival. Cancellation uses a stand-in dispatcher around real resources; it is not relabelled as exact-client or original-benchmark execution.
- Parent independently verified command counts/source hashes and raw observations in continuation-main/parent-d1-base-verification.json and parent-d1-separate-verification.json; local fixture observations do not attest a real service model, pricing or remote account.
- Final D1 handoff: C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/continuation-d1/handoff.json SHA-256 7367d22618f840917b31e52624cf0fdeb42c99c3af4fbf27e8a845302cc0bd74

Counterevidence boundary: The audit correctly limited its claim to missing current execution evidence. These actual client/Docker checks supply that baseline evidence without inferring service-model identity from configured model. No diagnostic benchmark reward is accepted as original benchmark scoring or real-provider acceptance. Current PASS is not future green-after.

### Batch B12: Required pinned native lifecycle witnesses are not assessable in this run

**Closure predicate**: Six CLI exit 0/met=true records; original pinned task/material identity matches; reward 1/0 in each family; shared TB2.1, exact declared TB3 separate collection, DeepSWE same base with nonempty/empty collected committed patch; owned resources settled; source/material original bytes unchanged.
**Dependencies**: B3 and B4, after B1-B7 final runtime bytes; rerun only cases affected by any later dependency change
**Verification union**: matching discriminating check below, affected test/script families, and the deduplicated final union.

#### Fix B12.1: refresh-final-source-native-lifecycle-evidence

- **Finding source(s)**: `phase19-codex-gpt6-3e8fee6-20260928t050232z` / `ccf574271b308b2bb72e13df1721824a9bda153e08b813e0679320b65ceb4ca1` / `P19-AUD-012`
- **Decision**: fix:refresh-final-source-native-lifecycle-evidence
- **Verification status**: Partially confirmed
- **File(s)**: none; retained external execution evidence only
- **Change kind**: metadata
- **Change**: Retain final-source six-case native lifecycle proof in a fresh external evidence directory and bind its manifest/hash in the fixed result. Current manifests are material/5 and remain immutable. If B3 emits material/6, regenerate through the final public prepare/material producer into fresh output/cache roots using independently verified original pinned runner/task/client inputs; do not relabel schema/5 files, patch hashes, or rewrite preserved proofs. Record final binary source head and hash plus regenerated manifest, static lock, runner source/lock/interpreter/entrypoint, selected integration and task-package/profile digests. B3 runtime-byte binding and B4 observed-package binding changes invalidate old admission proof even if benchmark names/rewards match. Keep each original task/grader/base/image identity. Do not use controlled_material, answer.txt diagnostics, changed task identities, or a new package merely to make a witness green. Replay each command with a fresh root and final native-material path. Scope environment to explicit PATH/HOME/UV_OFFLINE/UV_NO_ENV_FILE/UV_PYTHON_DOWNLOADS/HARBOR_TELEMETRY/DOCKER_HOST without provider credentials. Native task-declared public package acquisition may run within task containers; no Agent/provider/auth/paid calls. Verify resource receipts touch only owned objects; retain shared images and external cache. After final changes, rerun only affected proof cases when dependency equivalence is established; if B3/B4/common conformance or lifecycle paths change, the six-case family union is the appropriate revalidation batch.
- **Closure predicate**: Six CLI exit 0/met=true records; original pinned task/material identity matches; reward 1/0 in each family; shared TB2.1, exact declared TB3 separate collection, DeepSWE same base with nonempty/empty collected committed patch; owned resources settled; source/material original bytes unchanged.
- **Red-before**: N/A — Evidence-only verification batch: no behavioral production change is proposed for this source key, so no red-before production-defect reproduction exists or is required.
- **Green-after**: `For adapters terminal-bench-2.1, terminal-bench-3.0 and deepswe (the deepswe-v1.1 material family), execute each case oracle-witness and nop-witness with the final built opi-eval: opi-eval conformance --suite benchmark --adapter <adapter> --case <case> --root <fresh-unique-case-root> --fixtures crates/opi-eval/tests/fixtures --provider crates/opi-eval/scripts/scripted-provider.py --native-material <fresh-final-prepared-family-material>; independently verify all recorded original package/artifact/lifecycle bytes and cleanup receipts. Exact six-command replay is retained in continuation-d4/replay.sh.` — Six CLI exit 0/met=true records; original pinned task/material identity matches; reward 1/0 in each family; shared TB2.1, exact declared TB3 separate collection, DeepSWE same base with nonempty/empty collected committed patch; owned resources settled; source/material original bytes unchanged. Observed: not-run.
- **Test impact**: retain existing original-material native witness tests; repository tests add/update/delete none for this evidence-only batch

Current evidence:

- Current source binary e9ff92b9b51c205ad6c1c1285fe5d58f7649f84baaffe8a0ed5e9bfb16dcbcaf completed six native cases; all CLI exit 0, met=true, native exceptions null, three original task/material pairs rewards 1/0. Four cases newly executed; two current-source oracle cases reused by exact identity.
- Independent checks recorded 280 artifact hashes and 130 original package-file comparisons with no mismatch. TB2.1 both shared; TB3 both separate and only declared /app/evalbench/ collected with discriminating digests; DeepSWE preserves base cb1b3b671d0ee9fa9da9f7b02f86967953ffd10a, only model.patch collected, oracle nonempty/nop empty.
- Detailed commands/build/material/package/task/result identity: C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/continuation-d4/handoff.json SHA-256 e9d05c72eae2ea7e5b4d921307fac5f056df6d2cb29597705329da1e8076822d
- Verification: C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/continuation-d4/verification.json SHA-256 fc50a507fe5d7b1025ecf3d26b26eb5e7e2648a5528c6610946b8f79f2f4cc85
- All launched D4 jobs settled with no owned container remaining. Original task-declared public verifier acquisition occurred; no live model or provider invocation. These are lifecycle proofs, not saved-Agent offline regrade or six real-client trials.

Counterevidence boundary: The source finding explicitly described an audit evidence limitation, not a reproduced runtime defect. Current proof fills that baseline gap; B3/B4 will invalidate relevant binary/material observations, so no current PASS is future apply green. Diagnostic task replacement cannot satisfy this predicate.

### Batch B8: CI retains a parallel native-material preparation policy

**Closure predicate**: CI materialization delegates each selected family to the same local preparation/admission path; any shared prepare rejection prevents CI material publication, and accepted prepared per-family bytes retain identical task/profile/runner-project identities when consumed by CI callers. CI has no independent transform/admission rule.
**Dependencies**: B3 (one material format and runner identity contract)
**Verification union**: matching discriminating check below, affected test/script families, and the deduplicated final union.

#### Fix B8.1: delegate-ci-materialization-to-local-prepare

- **Finding source(s)**: `phase19-codex-gpt6-3e8fee6-20260928t050232z` / `ccf574271b308b2bb72e13df1721824a9bda153e08b813e0679320b65ceb4ca1` / `P19-AUD-008`
- **Decision**: fix:delegate-ci-materialization-to-local-prepare
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/scripts/native-smoke.sh`, `.github/workflows/opi-eval-native-smoke.yml`, `crates/opi-eval/scripts/verify-native-smoke-ci.py`, `crates/opi-eval/scripts/test_verify_native_smoke_ci.py`, `crates/opi-eval/external-locks/static/linux-x86_64.json`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`, `crates/opi-eval/scripts/fixtures/native-smoke-ci/workflow-floating-action.yml`, `crates/opi-eval/scripts/fixtures/native-smoke-ci/producer-second-listener.sh`, `crates/opi-eval/scripts/fixtures/native-smoke-ci/README.md`, `crates/opi-eval/scripts/verify-native-smoke-artifact.py`, `crates/opi-eval/scripts/test_verify_native_smoke_artifact.py`
- **Change kind**: behavioral
- **Change**: Replace only CI task/image/profile/runner-material production with the existing public local prepare path over its admitted pinned cache. Transport already pinned external/build inputs into that existing cache contract; keep CI provider/network/witness orchestration. Emit/use per-family prepared material directly, adapting conformance/oracle/trial callers to choose the relevant prepared manifest rather than synthesizing a second manifest. Remove now-unused duplicate materialize_task/profile/verifier-wrapper machinery. Update workflow/verifier fixtures and exact producer pins when their byte identities change. Update the native-smoke artifact reader and tests to validate the exact per-family prepared-manifest references rather than hardcoding 06-material/material.json. B3 owns their schema update; B8 separately owns their layout/routing update.
- **Closure predicate**: CI materialization delegates each selected family to the same local preparation/admission path; any shared prepare rejection prevents CI material publication, and accepted prepared per-family bytes retain identical task/profile/runner-project identities when consumed by CI callers. CI has no independent transform/admission rule.
- **Red-before**: `python3 /mnt/c/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/controls/probe_ci_preparation.py` — FAIL: WSL/Linux: exit1 assertion failure. Actual native-smoke.sh materialize-configs exited0 with synthetic task packages/Agent bytes and recording CLI/Cargo/uv/git stubs. CLI calls contained only validate, not prepare; material emitted all3 benchmark records with runner_project=null/origin=null. Shared prepare-refusal sentinel was bypassed. No real runner, Docker, provider, authentication or network used. Log `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/controls/ci-preparation-red.log`, SHA-256 `416ef4b1d39b467ea9e9e534803e728606f7fce465af7f665a6f767bee0d2a83`.
- **Green-after**: `python3 /mnt/c/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/green/controls/probe_ci_preparation.py; python3 -m unittest discover -s crates/opi-eval/scripts -p test_verify_native_smoke_ci.py` — Same public Bash stage with injected shared prepare refusal exits nonzero with that refusal and emits no material; add paired synthetic accepted/rejected local-cache equivalence cases to existing script tests. Observed: not-run.
- **Test impact**: {"add": "public Bash stage shared-prepare refusal prevents publication; prepared per-family accepted/rejected identity equivalence", "update": "existing native smoke structural verifier fixtures for shared preparation and per-family routing", "retain": "dispatch, transport, provider isolation, output-control probe, witness and seal checks"}

Current evidence:

- .github/workflows/opi-eval-native-smoke.yml:163: calls script materialize-configs stage
- crates/opi-eval/scripts/native-smoke.sh:893: owns task copying/image rewriting/profile inventory
- crates/opi-eval/scripts/native-smoke.sh:1028: emits all3 benchmark wrappers and verifier env independently
- crates/opi-eval/scripts/native-smoke.sh:1158: emits material/5 without runner_project
- crates/opi-eval/src/runner/material.rs:351: mandatory local runner_project differs from no-origin CI admission
- docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md:284: registered clause permits thin regression callers but forbids second materialization policy

Counterevidence boundary: CI already shares downstream validate/run and preserves bounded witness/provider/network orchestration. Sharing consumers cannot make the independent task/profile/runner producer equivalent.

### Batch B9: Local walkthrough plan command omits required input

**Closure predicate**: Copied walkthrough plan command, with only local fixture paths substituted, succeeds; both main READMEs describe the local prepare/validate/plan path and CI caller consistently.
**Dependencies**: B7 and B8 (final local workflow and CI ownership wording)
**Verification union**: matching discriminating check below, affected test/script families, and the deduplicated final union.

#### Fix B9.1: correct-local-plan-command-and-preparation-ownership

- **Finding source(s)**: `phase19-codex-gpt6-3e8fee6-20260928t050232z` / `ccf574271b308b2bb72e13df1721824a9bda153e08b813e0679320b65ceb4ca1` / `P19-AUD-009`
- **Decision**: fix:correct-local-plan-command-and-preparation-ownership
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/examples/local-evaluation/README.md`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`
- **Change kind**: documentation
- **Change**: Use the prepared native-material path in the walkthrough plan command and make the English/Chinese ownership/recovery passages refer to public local prepare with explicit output-control proof; describe CI as the regression caller after the shared preparation batch. Walkthrough has no Chinese counterpart; synchronize both main READMEs.
- **Closure predicate**: Copied walkthrough plan command, with only local fixture paths substituted, succeeds; both main READMEs describe the local prepare/validate/plan path and CI caller consistently.
- **Red-before**: `cargo test --offline --locked -p opi-eval --test native_driver current_ -- --nocapture` — FAIL: WSL/Linux: exit 101; 2 tests executed; 0 passed, 2 failed, 0 ignored, 116 filtered. Relative material froze successfully then default frozen run failed exit2 ENOENT from both same and unrelated cwd. Walkthrough command failed exit2 requires --native-material. Log `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/controls/plan-red.log`, SHA-256 `e4cb79937ba7848a9753f926c65e832f53ac6c6772f54b77a04b36b9dd3a6648`.
- **Green-after**: `opi-eval plan --config experiment.toml --native-material material/material.json --out plans/docs.json (using isolated fixture equivalents of the documented prepared files); python scripts/opi-doc-check.py; git diff --check` — documented plan exit0 and both documentation checks pass Observed: not-run.
- **Test impact**: {"none": "No new permanent prose-mirroring test; retain existing native plan/run behavioral tests and perform exact documented-command smoke."}

Current evidence:

- crates/opi-eval/examples/local-evaluation/README.md:155: plan example omits --native-material
- crates/opi-eval/README.md:530: native materialization ownership still assigned to workflow
- crates/opi-eval/README.md:598: regeneration advice routes to workflow
- crates/opi-eval/README.zh.md:456: Chinese ownership mirrors stale workflow claim
- crates/opi-eval/README.zh.md:517: Chinese regeneration mirrors stale workflow advice

Counterevidence boundary: Earlier main README native examples correctly include the material flag; doc checker passes structure but does not execute this walkthrough command.

### Batch B10: Durable roots retaining superseded attempts cannot publish report reconstruction

**Closure predicate**: Bilingual procedure tells operators to preserve original mixed root; create new staging copy for exactly one run-control-declared execution; copy byte-exact declared trial directories/control; verify and report outside staged/original roots; retain/identify all superseded attempts separately rather than drop them from evidence. Existing whole-root rejection remains.
**Dependencies**: B5 and B9 for synchronized documentation
**Verification union**: matching discriminating check below, affected test/script families, and the deduplicated final union.

#### Fix B10.1: document-per-execution-read-only-staging

- **Finding source(s)**: `phase19-pi-glm53-ac80281-20260928t070222z` / `8e47b6f1d2a5d74d6514568731c03e7a4ba99f31c638de369c6125a91f7b8435` / `P19-AUD-002`
- **Decision**: fix:document-per-execution-read-only-staging
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `docs/audits/phase19-local-native-acceptance.md`, `docs/audits/phase19-local-native-acceptance.zh.md`
- **Change kind**: documentation
- **Change**: Use the independently exercised staging procedure as reviewable docs instructions; validate selected root verify/report exit 0 and original mixed root remains fail-closed; compare original source hashes. Run doc checker and git diff --check after bilingual changes; no Rust runtime change or additional mirror test required.
- **Closure predicate**: Bilingual procedure tells operators to preserve original mixed root; create new staging copy for exactly one run-control-declared execution; copy byte-exact declared trial directories/control; verify and report outside staged/original roots; retain/identify all superseded attempts separately rather than drop them from evidence. Existing whole-root rejection remains.
- **Red-before**: N/A — Documentation repair preserves one-experiment-per-root behavior; do not weaken validation or silently filter originals.
- **Green-after**: `Execute the byte-preserving run-control-selected staging procedure below; run opi-eval verify and report on each selected copy; compare original hashes; python scripts/opi-doc-check.py; git diff --check` — All three staged roots verify and publish; whole mixed roots remain rejected; original bytes unchanged; paired documentation checks pass. Observed: not-run.
- **Test impact**: none; retain existing report control-drift tests

Current evidence:

- Fresh HEAD binary e9ff92b9... independently run on exact indexed original tb21/tb30/deepswe roots: report exits 1 on every whole root; verify exit 0/1/1 respectively. Copy exactly run-control planned_trials plus run-control.json to new per-family roots: all six verify/report commands exit 0.
- Original run-control.json and run-report.json before/after hashes unchanged. Detailed receipts/logs in /tmp/opi-p19-evidence-fc7d54373e7849f3ba2adea049b9b8cf/pi-root-checks.json.
- crates/opi-eval/src/report.rs:337-420 intentionally rejects control drift, undeclared trials and run-control drift. README.md:603-610 prescribes fresh replacement roots but omits legacy mixed-root report staging; acceptance handoff names roots 'retained failures included'.

Counterevidence boundary: Current byte verification and report reconstruction do not supply omitted acceptance inputs.

## Final Verification

Planning checks are distinct from future apply and Phase-exit evidence.
The following union applies after the planned changes. Missing excluded
acceptance inputs are not prerequisites for the bounded repairs below. Reuse a passing check for the same relevant bytes; rerun only
checks invalidated by subsequent changes. Do not count ignored native tests,
recording subprocess stubs, or zero matching tests as real/native acceptance.

```text
python scripts/opi-doc-check.py
git diff --check
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
python crates/opi-eval/scripts/test_prepare_acquisition.py
python crates/opi-eval/scripts/test_verify_native_smoke_artifact.py
python -m unittest discover -s crates/opi-eval/scripts -p test_verify_native_smoke_ci.py
python crates/opi-eval/scripts/test_verify_local_acceptance.py
python crates/opi-eval/scripts/test_native_integrations.py -q
bash -n crates/opi-eval/scripts/native-smoke.sh
```

Use the retained external Cargo caches and WSL/Linux for Unix/native seams;
use locked/offline Cargo resolution when dependencies are available. No
manifest/dependency/feature changes are planned. The /6 material change is a
deliberate 0.x data-contract change: synchronize the listed producers/readers,
fixtures, regeneration errors, paired docs and an Unreleased entry. B8 may
update only producer byte pins in the static external lock, not runner or
corpus revisions. Read the complete current Unreleased section before edits.

In addition, run each named red/green case above at its owning batch; extend
invalid-input coverage as stated there. B8 needs actual Bash subprocess
rejection propagation and positive per-family shared-prepare equivalence with
offline fixture cache, not only a source-text verifier. B9 executes the exact
corrected documented command with the existing isolated fixture. B10 stages
byte-exact copies of only `run-control.json` and its declared trial directories
under fresh per-execution roots, preserves all old attempts in their original
root, writes verify/report outputs outside both roots, and compares original
hashes. This report-only view must never masquerade as complete acceptance.

B11/B12 have additional native/material/evidence checks and are not satisfied
by ordinary workspace gates. Full workspace gates cover the changed public
material boundary and cross-crate use, and supply the final-source gate portion
of C/013 only. Its six-trial requirement remains excluded and unclosed.
Plan-only edits do not require those future runtime gates. Remediation does not itself declare Phase conformity.

## Exclusions and evidence admission

Every row below remains a separate source disposition. `Cannot confirm`
denotes an unestablished closure predicate, not a refuted finding or a
registered deferral. Source severity is unchanged.

### C/P19-AUD-011: Cannot confirm closure

Source: `phase19-codex-gpt6-3e8fee6-20260928t050232z` / `ccf574271b308b2bb72e13df1721824a9bda153e08b813e0679320b65ceb4ca1`.
Decision: `no-action:Cannot-confirm:original-pinned-regrade-inputs-unavailable`. Verification: Confirmed; Major retained.

**Closure predicate**: For each of TB2.1/TB3/DeepSWE, fresh verifier-only regrade consumes preserved real selected-Agent artifacts and original pinned grader/task/base material; produces new derived record referencing original immutable bundle and score; no Agent/provider invocation; missing-cache rejection and source-byte preservation proven.

Current independently checked evidence:

- crates/opi-eval/tests/native_driver.rs:10758 immediately derives controlled_material; tests/support/native_witness_material.rs:45 explicitly creates private tasks and diagnostic clients; helper rewrites grader and corpus lock identities.
- native_offline_regrade at native_driver.rs:7622 uses stage("ok") stand-ins.
- Both exact b9 candidate files exist and hash-match; command native_verifier_only_regrade_witness executes 1 passed, 0 failed, 0 ignored, 108 filtered. This proves the diagnostic path, not original benchmark graders.
- Three current public regrade invocations covered all six selected preserved trials: TB2.1/TB3 rejected task-package-drift; DeepSWE rejected environment-image-drift. All three commands exited 1 before native verifier dispatch; these are input rejections, not production red-before.
- All original input byte maps are unchanged. Direct sealed cache pointer inspection finds matching package/static-lock inventory but no matching original material manifest. TB2.1 original artifacts=[] and sealed collected=[] independently prevent preserved-artifact regrade; original grading script also requires public package acquisition without a proven offline equivalent.
- Current exact commands, isolated environment, results and input hashes: C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/continuation-d2/handoff.json (SHA-256 9efb412691cbb4fe0cf48cf23c919b79e9525546a59d0e458d9c6587e290b6a9)

Counterevidence/applicability: The current ignored witness meaningfully exercises actual runner verifier machinery, immutability, positive/negative inputs and no-provider mechanics, but its substituted answer.txt graders do not satisfy pinned original grader identity.

Required next verification: If actual matching preserved material/artifacts become available, admit their identities and run public regrade into fresh output roots with absent credentials, no Agent/provider dispatch and original input hash equality. A new collection or verifier-permitted offline equivalent requires separate scope/authority; it cannot retroactively supply the missing original artifact set.

Missing inputs / admission boundary: Originally admitted material and runtime/cache inputs matching each sealed task/image identity are unavailable in the bounded direct dependencies.; TB2.1 has no preserved declared/collected Agent artifact set; neither rewriting the grader nor resealing invented artifacts is admissible.; A qualifying original-material offline regrade needs separately admitted sufficient evidence; no production repair follows from the rejected mismatched inputs.

### C/P19-AUD-013: Cannot confirm closure

Source: `phase19-codex-gpt6-3e8fee6-20260928t050232z` / `ccf574271b308b2bb72e13df1721824a9bda153e08b813e0679320b65ceb4ca1`.
Decision: `no-action:Cannot-confirm:six-trial-acceptance-inputs-unavailable`. Verification: Partially confirmed; Major retained.

**Closure predicate**: Strict verify-local-acceptance accepted:true for preserved redacted root with six attributable first-connection trials across both clients/three families, original failures retained, material/build provenance and Opi-only scoped view; generic artifact audit passes; required final-source workspace gates pass without ignored required evidence standing in.

Current independently checked evidence:

- All ten exact appendix candidate files exist and match SHA256. Four gate receipts exit 0; all-targets has 229 result lines summing 4542 passes/16 ignores (includes nested subprocess lines); doc tests 8 result lines/12 passes; current source/build identity is not recorded in gate receipts.
- Exact acceptance-precheck.json has 6 actual trials but 4 terminal timeouts (exit 137), missing terminal events, and Opi TB3/DeepSWE missing settled tool loops; formal_acceptance.status=not-run and qualifying_six_established=false.
- Current verifier at crates/opi-eval/scripts/verify-local-acceptance.py:479-513 independently requires exit 0, no timeout/failure, settled Agent/provider events and nonempty tool execution. Summary/native-graded status cannot substitute.
- Independently compared primary reports: their selected IDs (openssl r1.r2; TB3/DeepSWE r1.r2.r2) and bundle hashes are distinct from portable-* r1 precheck. Their raw terminals are schema /1, exit 0/no timeout; all six sealed bundles lack native/agent-terminal.json required by current /2 preflight contract. Invoking current Acceptance.check_agent_observations against each actual selected bundle rejected with FileNotFoundError (selected-acceptance-observation-probe.json).
- Current reconstructed reports for the distinct primary selected roots classify as conformance-evidence, with outcome published. This does not establish current execution.mode=live acceptance.
- User explicitly answered “没有” when asked for any additional complete redacted six-trial evidence root: C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/continuation-main/scope-input.json (SHA-256 822de4c6330e7c57a4778beba50385fbdfd78070a97e399b8ab4b746a721d788)

Counterevidence/applicability: Primary selected roots all pass current byte verify/report reconstruction and are distinct from the codex precheck's portable run. They retain six real native-graded results with raw terminal exit 0/no timeout. However byte verification/report reconstruction does not satisfy current strict observation sealing/preflight requirements. Precheck failures are evidence about only its own distinct trials.

Required next verification: On later supplied qualifying inputs, validate exact seals/material/build provenance and the current strict acceptance predicates offline. Retain failed attempts and original bytes. Do not insert or backdate missing observations. Any new real-provider execution needs separate explicit scope/budget authorization. The planned final workspace gates supply only its final-source gate portion; their PASS cannot close the excluded six-trial requirement.

Missing inputs / admission boundary: No additional complete evidence root was supplied. Preserved primary bundles lack current sealed terminal/runtime-preflight observations and the admitted manifest/provenance/Opi-only view.; Source severity and original conformance effect remain; this is not a registered deferral or closure. New provider trials are outside this plan invocation.

### G/P19-AUD-001: Cannot confirm closure

Source: `phase19-pi-glm53-ac80281-20260928t070222z` / `8e47b6f1d2a5d74d6514568731c03e7a4ba99f31c638de369c6125a91f7b8435`.
Decision: `no-action:Cannot-confirm:staged-root-inputs-unavailable`. Verification: Confirmed; Minor retained.

**Closure predicate**: A durable named redacted evidence root is reconstructable from preserved raw evidence with exact manifest, scoped Opi view and independently admitted identity/material records; named offline verifier and artifact-audit commands run on it and fresh outputs retained; bilingual procedure records retention location/ownership without claiming unsupported acceptance.

Current independently checked evidence:

- Bounded current find /home/luiz -type d -path '*/phase19-operator/evidence-root' returned no match, exit 0; no conclusion from unrelated history.
- Current source verifier invoked against representative absent absolute root returns exit 2: evidence root does not exist (pi-001-absent-root.log).
- Exact appendix primary tb21 run-report.json is present/hash-matches; TB3/DeepSWE raw report candidates also hash-match. Primary results are not staged manifest/scoped view/identity inventory.
- Current README.md:631-655 requires redacted evidence root and verifies only supplied family roots; current docs name only relative phase19-operator/evidence-root and don't state durable retention/staging procedure.
- User explicitly answered “没有” when asked for any additional complete redacted six-trial evidence root: C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/continuation-main/scope-input.json (SHA-256 822de4c6330e7c57a4778beba50385fbdfd78070a97e399b8ab4b746a721d788)

Counterevidence/applicability: Current byte verification and report reconstruction do not supply omitted acceptance inputs.

Required next verification: On later supplied qualifying inputs, validate exact seals/material/build provenance and the current strict acceptance predicates offline. Retain failed attempts and original bytes. Do not insert or backdate missing observations. Any new real-provider execution needs separate explicit scope/budget authorization.

Missing inputs / admission boundary: No additional complete evidence root was supplied. Preserved primary bundles lack current sealed terminal/runtime-preflight observations and the admitted manifest/provenance/Opi-only view.; Source severity and original conformance effect remain; this is not a registered deferral or closure. New provider trials are outside this plan invocation.

## Plan-stage evidence and reproducibility

Current raw proof copies are retained outside the repository at
`C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1`. They contain test probes, exact logs, hashes and current
applicability checks, not a competing plan or progress ledger. The fixed pair
in this assurance directory remains the sole plan approval object.

Bindings proof: `bindings/final-probes.diff` applies to a fresh archive of this
head and only changes test-bearing files; commands and logs are enumerated in
`bindings/handoff.json`. Controls proof: `controls/handoff.json` embeds the
complete probe sources. For red replay only, export this exact head to
`source/` beneath the proof root, append the retained Rust test snippets only
to their stated test modules/files, and run the matching commands using the
existing external cache. Python commands using `/mnt/c/` execute inside WSL.
For B5/B8 green, create a separate `green/source/` from the complete intended
applied tree, including task-owned uncommitted changes; copy the byte-identical
original probes into `green/controls/` and run from `green/source/`. Verify a
source manifest against the applied workspace before that check. These probes
resolve their source as their parent directory's `source/`; never run green
against the unchanged baseline export or overwrite the preserved red copy.
Python producer probes use isolated subprocess doubles. Promote the behavior cases to existing test
owners during apply; no new permanent test binary or prose-mirroring test is
required. Initial invalid exploratory failures are excluded; only the listed
final discriminating logs support red-before.

The build used for report reconstruction, D2 regrade attempts and D4 lifecycle checks is
`e9ff92b9b51c205ad6c1c1285fe5d58f7649f84baaffe8a0ed5e9bfb16dcbcaf`,
built by `cargo build --offline --locked -p opi-eval` from a unique archive
with unchanged production bytes. D1 uses separately built current clients and
test/Companion binaries bound in its source/build manifest; do not substitute
one executable hash for the other. Later cache rebuilds are not proof identity.
Original primary evidence is read-only; current staging and outputs use new
temporary roots. Exact indexed appendix candidates and their named proof
dependencies were rehashed and checked; old `Closed` text was never closure
authority. Exact proof/hash inventory is in `evidence/handoff.json` and the
retained current receipts when present.

Current original-material lifecycle evidence now has all six oracle/nop
outcomes: each family yields rewards 1/0 with null native exceptions. Four
cases were added during continuation; two positive current-source cases were
reused by exact binary/material/source identity. TB2.1 used shared grading,
TB3 handed off exactly its declared collection, and DeepSWE preserved its
pinned base with discriminating nonempty/empty committed patch. Native
graders performed their task-declared public dependency acquisition. These
are no-model lifecycle checks, not offline regrade or real-Agent acceptance.
The current exact-client and cleanup receipts are separately bound by B11.

The continuation evidence inventory below binds D1, D2, D4 and the user's
missing-input answer. Earlier proof copies and their original inventory are
unchanged. Distinct old precheck failures are never attributed to the later
primary selected six trials. Original seals are immutable; absent runtime
preflight/terminal observations may not be inserted or backdated.

Test impact for this planning delivery: **none** in the repository. Temporary
probes add/update behavior assertions only in exported copies. Future apply
test impact is recorded per batch: add/update/retain, delete none. Only the
fixed Markdown/JSONL plan pair is the outgoing worktree change.

Final artifact verification commands:

```text
python .agents/skills/_shared/scripts/validate_assurance_artifact.py dispositions docs/snapshots/phase19/assurance/remediation.plan.dispositions.jsonl
python .agents/skills/_shared/scripts/validate_assurance_artifact.py plan docs/snapshots/phase19/assurance/remediation.plan.md
python scripts/opi-doc-check.py
git diff --check
```

The baseline archive documentation check passed under WSL. Windows traversal
of the archive's extracted `.claude/skills` symlink raised PermissionError;
this was an export/platform check limitation, not a documentation failure.
Final validators use the real repository Git context; the new plan pair is
also copied into the source export for the final WSL documentation check.

The validator-emitted plan digest is reported in the handoff, not embedded
here. A ready replacement plan still needs a new explicit
`$opi-remediate mode=apply phase=19 plan_sha256=<exact emitted digest>`
invocation. This plan invocation authorizes no production edits. A plan-only commit, if
later explicitly requested, preserves this source baseline; any intervening
non-plan path or active-index change requires replanning.

Current raw proof inventory: `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/proof-inventory.json`, SHA-256 `b1d8636a7a579e9d421cd2b84e1d02c547e81b8631aa2be6f2ae8392ee320ff8`. This binds the final retained current logs, probes and applicability handoffs by exact bytes. Recheck those digests before reuse.

Continuation raw proof inventory: `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-plan-20260929-34c307b1/continuation-main/proof-inventory.json`, SHA-256 `c236141ec3511c938f1027002e5e03713f75f7e95cf508878b134315f6b4cd73`. Recheck both inventories before reuse.

## Criterion cross-reference matrix

The two members use different citation strings and granularity. Rows below group only the exact `(path, sha256, citation)` key; similar acceptance labels do not merge criteria or overwrite either source state. The immutable indexed sidecars retain each complete requirement record, observable, evidence and checks. There are 81 original records (80 mandatory), grouped into 64 exact criterion keys. An omitted member cell means no exact citation match, not agreement.

Source path aliases:

- `S`: `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`, SHA-256 `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`.
- `P`: `docs/opi-spec.md`, SHA-256 `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db`.

| Source / exact citation | C: original requirement IDs and states | G: original requirement IDs and states |
|---|---|---|
| S / Sections 5; P19-AC-002 | P19-002-agent = met; P19-002-oracle = met | — |
| S / Sections 5–6; P19-AC-002 | P19-002-artifact = not-met | — |
| S / Sections 5; P19-AC-003 | P19-003-lifecycle = not-assessable | — |
| S / Sections 5,8; P19-AC-003 | P19-003-witness = not-assessable | — |
| S / Sections 5; P19-AC-004 | P19-004-collection = not-assessable | — |
| S / Sections 5,8; P19-AC-004 | P19-004-witness = not-assessable | — |
| S / Sections 5; P19-AC-005 | P19-005-patch = not-assessable | — |
| S / Sections 5,8; P19-AC-005 | P19-005-witness = not-assessable | — |
| S / Sections 6; P19-AC-009 | P19-009-regrade = not-assessable; P19-009-regrade-boundary = met; P19-009-verify = met; P19-009-report = met | — |
| S / Sections 6,8; P19-AC-009 | P19-009-regrade-witness = not-assessable | — |
| S / Sections 4; P19-AC-006 | P19-006-projection = met; P19-006-controls = not-met | — |
| S / Sections 4,7; P19-AC-006 | P19-006-fixture = not-assessable | — |
| S / Sections 6; P19-AC-006; task 19.2 | P19-006-terminal = met | — |
| S / Sections 4,6; P19-AC-006 | P19-006-model-unknown = not-assessable | — |
| S / Sections 4; P19-AC-010 | P19-010-secret-input = met; P19-010-secret-grader = not-assessable | — |
| S / Sections 4,6; P19-AC-010 | P19-010-redaction = not-met | — |
| S / Sections 4,9; P19-AC-010 | P19-010-supervision = met; P19-010-budgets = met | — |
| S / Sections 9; P19-AC-010 | P19-010-cleanup = not-assessable | — |
| S / Sections 4,6,9; P19-AC-010 | P19-010-recovery = met; P19-010-replacement = not-met | — |
| S / Sections 4; P19-AC-001 | P19-001-doctor = met; P19-001-plan-run = partially-met | — |
| S / Sections 4–5; P19-AC-001 | P19-001-prepare = met | — |
| S / Sections 5; P19-AC-007 | P19-007-material = partially-met; P19-007-origin = partially-met; P19-007-inventory = met; P19-007-resources = met | — |
| S / Sections 4–5; P19-AC-007 | P19-007-selection = met | — |
| S / Sections 3; P19-AC-013 | P19-013-placement = met | — |
| S / Sections 3,5; P19-AC-013 | P19-013-packaging = met | — |
| S / Sections 9; P19-AC-013 | P19-013-contracts = partially-met | — |
| S / Sections 8; P19-AC-011 | P19-011-default = met; P19-011-ci = partially-met | — |
| S / Sections 6; P19-AC-008; task 19.1 | P19-008-classification = met | — |
| S / Sections 6; P19-AC-008 | P19-008-invalid = met; P19-008-missing = met; P19-008-denominators = met; P19-008-pairs = met; P19-008-scope = met | — |
| S / Sections 7–8; P19-AC-012; tasks 19.10–19.11 | P19-012-verifier = partially-met | — |
| S / Sections 8; task 19.10 acceptance declarations | P19-012-provenance = partially-met | — |
| S / Sections 7–8; P19-AC-001 | P19-001-live = not-assessable | — |
| S / Sections 7–8; P19-AC-003 | P19-003-live = not-assessable | — |
| S / Sections 7–8; P19-AC-004 | P19-004-live = not-assessable | — |
| S / Sections 7–8; P19-AC-005 | P19-005-live = not-assessable | — |
| S / Sections 7–8; P19-AC-006 | P19-006-live = not-assessable | — |
| S / Sections 7–8; P19-AC-012 | P19-012-six-trials = not-assessable | — |
| S / Sections 8; P19-AC-012 | P19-012-gates = not-assessable | — |
| S / Section 4 optional Chat Completions and Responses connections | P19-006-optional-live = not-assessable (optional) | — |
| S / #7 Requirements and acceptance scenarios, row P19-AC-002 | — | P19-AC-002 = met |
| S / #7 Requirements and acceptance scenarios, row P19-AC-003 (native witness scenario) | — | P19-AC-003-native = met |
| S / #7 row P19-AC-008 (settlement scenario, task 19.1) | — | P19-AC-008-outcome = met |
| S / #7 row P19-AC-006 (fixture scenario, task 19.2) | — | P19-AC-006-fixture = met |
| S / #7 row P19-AC-010 (credential scenario, task 19.2) | — | P19-AC-010-credentials = met |
| S / #7 row P19-AC-004 (native witness scenario, task 19.3) | — | P19-AC-004-native = met |
| S / #7 row P19-AC-005 (native witness scenario, task 19.4) | — | P19-AC-005-native = met |
| S / #7 row P19-AC-001 (prepare scenario, task 19.5) | — | P19-AC-001-prepare = met |
| S / #7 row P19-AC-007 (material scenario, task 19.5) | — | P19-AC-007-material = met |
| S / #7 row P19-AC-007 (selection scenario, task 19.6) | — | P19-AC-007-selection = met |
| S / #7 row P19-AC-001 (plan scenario, task 19.6) | — | P19-AC-001-plan = met |
| S / #7 row P19-AC-008 (report scenario, task 19.7) | — | P19-AC-008 = met |
| S / #7 row P19-AC-009 (verify scenario, task 19.7) | — | P19-AC-009-verify = met |
| S / #7 row P19-AC-009 (regrade scenario, task 19.8) | — | P19-AC-009-regrade = met |
| S / #7 row P19-AC-010 (lifecycle scenario, task 19.9) | — | P19-AC-010-lifecycle = met |
| S / #7 row P19-AC-011 (task 19.10) | — | P19-AC-011 = met |
| S / #7 row P19-AC-013 (task 19.10) | — | P19-AC-013 = met |
| S / #7 row P19-AC-001 (live scenario, task 19.11) | — | P19-AC-001-live = met |
| S / #7 row P19-AC-003 (live scenario, task 19.11) | — | P19-AC-003-live = met |
| S / #7 row P19-AC-004 (live scenario, task 19.11) | — | P19-AC-004-live = met |
| S / #7 row P19-AC-005 (live scenario, task 19.11) | — | P19-AC-005-live = met |
| S / #7 row P19-AC-006 (live scenario, task 19.11) | — | P19-AC-006-live = met |
| S / #7 row P19-AC-012 (task 19.11) | — | P19-AC-012 = met |
| S / #8 Verification and local acceptance handoff, verification union | — | P19-STD-001 = met |
| P / #2.1 GOAL-003 and repository AGENTS.md smallest-coherent-change rule, via .opi-impl-state.json phase 19 tasks 19.1-19.11 inference_notes | — | P19-MCC-001 = met |
