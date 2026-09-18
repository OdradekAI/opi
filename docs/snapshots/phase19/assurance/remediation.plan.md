# Phase 19 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `7550892cafbe6db2df1b56cd3f2eadb2104303428019940d4dc341e621f48f9d`
**Remediation head**: `e169ab41d91b25870312088616ab1cfe759ef793`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged `[]`; unstaged `[]`; untracked `[]` at plan admission. The four fixed remediation outputs were tracked and clean. Plan execution changes only the fixed plan pair.
**Unresolved decisions**: none

## Admission and evidence boundary

`audit-set` passed before semantic consumption. The live index is schema 2,
Phase 19, revision 2, aggregate `FAIL`. This plan covers its strict union of
13 source keys. It does not change that indexed verdict or either source sidecar.

| Alias used below | Exact source run | Owning findings SHA-256 |
|---|---|---|
| C | `phase19-codex-gpt6-c18efa8-20260917t133157z` | `67c0ab6ca8b90178e3caf2d7cdd6cb4a6c9a6b2fe342c85d8cbd3ef105c95b36` |
| P | `phase19-pi-glm53-d61590b-20260917t152753z` | `b85fa7a405aee5d3da9ce410bcd9619cf55af85fb3bce75a4c8b1f9aae96bc0b` |

Aliases are display abbreviations only. Every machine disposition contains the
full source key/digest and the unchanged source finding, including its claim,
severity, evidence and requirement IDs. Equal textual `P19-AUD-001` IDs in C
and P remain separate. Scheduling retains the highest source severity; there
is no vote-based closure and no source severity downgrade.

The latest committed ledger registers `docs/opi-spec.md` and the bilingual
`2026-09-05-phase19-local-native-agent-evaluation-design` pair; its Phase 19
snapshot pointer is `docs/snapshots/phase19/opi-impl-state.json`. Recomputed
digests are respectively `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db`,
`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`, and
`d41c826412b0d71802327d60f953708f6cb2083436a2a2b0d99a2e52d851f194`.
English owns the supplemental meaning. Parent authority is STRAT-002,
PLACE-001/002, CTRL-004..007, INV-006 and PHASE-006; supplemental sections
3, 5, 6, 8 and 9 assign supervision to the Companion and native environment,
collection and grading semantics to the external runner.

Committed source was inspected in the unique `git archive` export
`E:/opi/remediate-plan19-fb8c5146d1fc/source`, not a Git worktree. Only two
test probes were appended there. No production bytes were edited in that
export or the repository during planning. Checks used the existing external
Cargo target `/mnt/e/opi/cargo-targets/opi-8abcf38f9871653e`, incremental
compilation and explicit cache leases. No separate target directory, cache
cleanup, provider calls or user-global configuration changes were used.

The current member metadata, requirements and findings are semantic inputs.
Reports were used by the validator for integrity only. Prior remediation
conclusions, history runs, eval/acceptance archives, and another source/plan
path are not evidence for the new dispositions. Independently prepared pinned
runner source/runtime/image material is allowed; it is not an eval archive.

## Current Finding Verification

| Source / Finding ID | Verification at remediation head | Source / final severity | Closure key / family | Batch | Decision |
|---|---|---|---|---|---|
| C / P19-AUD-001 | Refuted | Blocker / Blocker (retained) | `regrade.closed-child-environment` / `regrade.authority` | - | `no-action:refuted-at-remediation-head` |
| C / P19-AUD-002 | Confirmed | Major / Major (retained) | `regrade.fresh-native-verifier` / `regrade.execution` | B2 | `fix:verifier-only-native-regrade` |
| C / P19-AUD-003 | Refuted | Major / Major (retained) | `native.selected-terminal-identity` / `native.admission` | - | `no-action:refuted-at-remediation-head` |
| C / P19-AUD-004 | Refuted | Major / Major (retained) | `native.provider-failure-exclusion` / `native.settlement` | - | `no-action:refuted-at-remediation-head` |
| C / P19-AUD-005 | Partially confirmed | Major / Major (retained) | `native.stage-deadline-stops-effects` / `native.supervision` | B1 | `fix:owned-native-stage-cancellation` |
| C / P19-AUD-006 | Refuted | Blocker / Blocker (retained) | `cli.frozen-plan-native-entry` / `cli.admission` | - | `no-action:refuted-at-remediation-head` |
| C / P19-AUD-007 | Refuted | Major / Major (retained) | `native.selected-resource-admission` / `native.material` | - | `no-action:refuted-at-remediation-head` |
| C / P19-AUD-008 | Refuted | Major / Major (retained) | `native.multi-task-selection` / `native.material` | - | `no-action:refuted-at-remediation-head` |
| C / P19-AUD-009 | Refuted | Major / Major (retained) | `run.exclusive-root-ownership` / `run.durability` | - | `no-action:refuted-at-remediation-head` |
| C / P19-AUD-010 | Cannot confirm | Major / Major (retained) | `acceptance.real-native-and-final-gates` / `acceptance.evidence` | - | `no-action:cannot-confirm-acceptance-within-input-boundary` |
| C / P19-AUD-011 | Confirmed | Major / Major (retained) | `report.native-metric-meaning` / `report.semantics` | B3 | `fix:benchmark-metric-semantics` |
| C / P19-AUD-012 | Refuted | Major / Major (retained) | `trajectory.unknown-native-agent-time` / `report.telemetry` | - | `no-action:refuted-at-remediation-head` |
| P / P19-AUD-001 | Partially confirmed | Info / Info (retained) | `provenance.consolidated-commit` / `provenance.history` | - | `no-action:info-consolidated-commit` |

`Refuted` here means the current claim does not hold at the stated new head;
it does not assert that the older audit observation was wrong. The eight
counterexamples are independently described in `verification_evidence` and
were exercised by the current export's regression command. For C/005,
deadline projection now reaches the runner and the synthetic projection test
passes; real verifier process termination remains broken, so verification is
only partial. Three separate predicates require three separate batches;
shared Python/Rust files do not merge them.

### Requirement cross-reference

Both members cite the same supplemental path/digest, but C's split citations
and P's broad section citations are not identical criterion identities.
The rows below are a parent-criterion cross-reference, not merged records or
new requirement verdicts. Every detailed source state/evidence remains in its
own digest-bound requirements sidecar. P's broad `met` states cannot erase C's
reciprocal finding links or supply missing dynamic evidence.

| Parent criterion | C source subrequirement states (counts) | P source state |
|---|---|---|
| P19-AC-001 | met: 2, partially-met: 3 | met |
| P19-AC-002 | not-met: 2, partially-met: 3 | met |
| P19-AC-003 | not-assessable: 2, partially-met: 1 | met |
| P19-AC-004 | not-assessable: 2, partially-met: 2 | met |
| P19-AC-005 | not-assessable: 2, partially-met: 2 | met |
| P19-AC-006 | not-assessable: 2, not-met: 2, partially-met: 3 | met |
| P19-AC-007 | met: 2, not-met: 3, partially-met: 2 | met |
| P19-AC-008 | met: 4, not-met: 2, partially-met: 1 | met |
| P19-AC-009 | met: 3, not-assessable: 1, not-met: 1, partially-met: 2 | met |
| P19-AC-010 | not-assessable: 1, not-met: 2, partially-met: 5 | met |
| P19-AC-011 | met: 3, partially-met: 1 | met |
| P19-AC-012 | not-assessable: 4 | met |
| P19-AC-013 | met: 1, partially-met: 3 | met |

P's additional `P19-SCOPE-001` remains met in its source; C's two optional
OpenAI live-qualification records remain optional/not-assessable. No new
finding is created from those states. C/010's 30 linked obligations remain
outside claimed closure; the source's statement about seven ignored tests is
not copied as a current count (the committed native_driver now has eight).

## Unresolved Decisions

None.

## Resolved Decisions

| ID | Approved decision | Authority and evidence |
|---|---|---|
| D1 | Revise native-regrade and regrade-record to /2 for verifier-only execution, with synchronized callers, documentation and Unreleased entries, without migration or a compatibility facade. Preserve source identities without fabricating a fresh Agent execution or full native job. | The evaluation evidence/report owner (user) explicitly approved this decision with "同意" in the current conversation on 2026-09-18. |

All repair designs, decisions and red-before evidence are complete. This
approval resolves the output-contract design; apply still requires a new
explicit invocation containing the validator-emitted digest of this fixed plan.

## Repair Design

1. Preserve Harbor `4407eb5227a2ff4f0d3f16b2eb48849382fdf276` and Pier
   `c1ebc6d145b40fae8425215e3fca528945065124`. Use their existing
   `EnvironmentConfig.import_path` / `EnvironmentFactory` extension points
   for private subclasses of their Docker environments, in the already pinned
   integration module directories. Inherit upstream environment/resource/
   network/collection behavior; add owned-command supervision only. Do not
   patch cached upstream checkouts, task grader scripts, tasks' timeouts, or
   create a second container/task runner. The examined factory paths support
   both Agent and separate-verifier environment construction. Harbor's
   separate verifier discards extra compose files, so a compose-only patch is
   not the chosen mechanism. Pier has no custom verifier import field.
2. A deadline succeeds as an enforcement mechanism only after the owned
   execution has stopped. A host transport cancellation is insufficient.
   Preserve the runner's typed AgentTimeoutError versus VerifierTimeoutError,
   exact command-exit observation and Unknown reward where appropriate. If
   command termination cannot be proved, stop affected dispatch, perform only
   exact owned-resource escalation, and preserve invalid/effect-unknown state;
   never manufacture a successful cleanup or valid Agent score.
3. Regrade is a verifier-only operation. A private Python entry in each
   existing family integration module uses the pinned upstream task,
   environment, artifact-transfer and verifier APIs. It does not construct an
   Agent, run an oracle/nop Agent, manufacture an Agent terminal, or fabricate
   an upstream full job just to satisfy the initial-run importer. Use a
   separate private typed fresh-verifier observation with source-bundle,
   task/runner/module/image and input-byte bindings. Share low-level digest and
   native-result validation where their meaning is genuinely identical.
4. Regrade restores only preserved, digest-bound declared inputs under the
   selected task's native rules. A shared-environment source with insufficient
   preserved state must reject `agent-artifacts-missing` (or a more precise
   typed missing-state reason). Do not pretend a pristine base is that Agent's
   final filesystem, create a snapshot retroactively, add new collection
   declarations to a task, or relax the verifier network policy. Demonstrate
   the shared case with a controlled task whose native declared inputs are
   sufficient. Any need for a new snapshot/state authority returns to shaping.
5. Regrade uses explicit pinned offline argv and the existing Rust process
   supervisor, a closed credential-free environment, finite stage/job/output
   bounds, one cancellation token, and a fresh output identity. Preserve the
   selected local Docker context. Check cached runtime/image/material before
   dispatch; no implicit downloads, Agent/provider calls or fallback.
6. The regrade output/derived-record contracts become
   `opi-eval-native-regrade/2` and `opi-eval-regrade-record/2` so their fresh
   verifier evidence is not confused with a fabricated fresh Agent trial.
   Old source bundles are never rewritten or relabelled. Invalid/incomplete
   old artifacts fail closed. Module directory digest changes flow through
   existing material/5 integrity; this plan does not bump material schema,
   upstream pins, Cargo dependencies, the ledger or a normative source.
7. Reporting exposes separate native task outcomes and test-case counts.
   The currently admitted primary reward domain remains binary 0/1; this plan
   does not broaden it to arbitrary fractional rewards or infer one from
   counters. Definitions identify TB2.1 native task success, TB3 native task
   success after declared collection, and DeepSWE resolved native task outcome
   after committed-patch verification, each with the admitted revision and
   raw grader source. A measured test-case pass fraction uses native passed /
   native total tests, with numerator, denominator, artifact digest and
   observation coverage. Missing counters or zero total produce an explicit
   unknown reason. A partially passing test set can coexist with task reward
   zero. Extra opaque upstream reward fields keep their original names and
   provenance; do not guess their meanings from abbreviations.
8. The already accepted report/reporter /2 direction is retained. It records the two metric
   kinds with distinct denominators, preserves per-trial source citations,
   excludes invalid/provider/verifier failures, labels subsets and missing
   observations, and never averages the three benchmark families. Report
   reconstruction remains offline and deterministic; unchanged original
   bundles remain immutable. There is no compatibility facade. Update the
   actual callers/fixtures, bilingual README and Unreleased together.

## Closure Batches

Execute B1, then B2, then B3. B2 and B3 remain undispatched if B1's real
stage witness fails. Sharing integration files is an edit dependency, not a
reason to combine closure predicates. Apply is not authorized by this plan
invocation: it requires a new explicit invocation with the emitted digest.

### Batch B1: Stop owned native stage execution at the effective deadline

**Closure predicate**: PASS for all 3 families x 9 cases x 2 subjects: effective stage limits preserved, delayed completion absent after timeout, observed command exit, exact owned Agent/verifier resources removed, neighboring resources and shared images unchanged. The Python cancellation/normal-completion/race/cleanup-failure cases also pass.
**Dependencies**: none
**Verification union**: OPI_EVAL_NATIVE_WITNESS_MATERIAL=/home/luiz/.cache/opi-remediate-plan19-fb8c5146d1fc/stage/material.json cargo test -p opi-eval --locked --test native_driver remediation_native_stage_deadlines -- --exact --ignored --nocapture --test-threads=1 ; python3 -m unittest discover -s crates/opi-eval/scripts -p test_native_integrations.py; final verification union below.

#### Fix B1.1: Stop owned native stage execution at the effective deadline

- **Finding source(s)**: `phase19-codex-gpt6-c18efa8-20260917t133157z` + `67c0ab6ca8b90178e3caf2d7cdd6cb4a6c9a6b2fe342c85d8cbd3ef105c95b36` + `P19-AUD-005`
- **Decision**: `fix:owned-native-stage-cancellation`; the design above is part of this scope; the verifier-only regrade /2 revisions were approved in D1.
- **Verification status**: Partially confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `crates/opi-eval/integrations/harbor/opi_eval_harbor.py`, `crates/opi-eval/integrations/pier/opi_eval_pier.py`, `crates/opi-eval/scripts/test_native_integrations.py`, `crates/opi-eval/src/benchmark/process.rs`, `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/tests/native_driver.rs`
- **Change kind**: behavioral
- **Change**: Implement only the bounded behavior detailed below.
- **Closure predicate**: PASS for all 3 families x 9 cases x 2 subjects: effective stage limits preserved, delayed completion absent after timeout, observed command exit, exact owned Agent/verifier resources removed, neighboring resources and shared images unchanged. The Python cancellation/normal-completion/race/cleanup-failure cases also pass.
- **Red-before**: `OPI_EVAL_NATIVE_WITNESS_MATERIAL=/home/luiz/.cache/opi-native-19-mzcSc2/witness-material-cancel/material.json cargo test -p opi-eval --locked --test native_driver remediation_native_stage_deadlines -- --exact --ignored --nocapture --test-threads=1` — FAIL: exit 101 after 194.47 s; TB2.1 positive/time-cap/agent-cap completed, but verifier-cap left verifier/stage-end present for both subjects after native VerifierTimeoutError (~1.08 s). Remaining cases/families were not reached. Fresh root /tmp/.tmpM95EMZ; no claim about unexecuted cases.
- **Green-after**: `OPI_EVAL_NATIVE_WITNESS_MATERIAL=/home/luiz/.cache/opi-remediate-plan19-fb8c5146d1fc/stage/material.json cargo test -p opi-eval --locked --test native_driver remediation_native_stage_deadlines -- --exact --ignored --nocapture --test-threads=1 ; python3 -m unittest discover -s crates/opi-eval/scripts -p test_native_integrations.py` — expected PASS for all 3 families x 9 cases x 2 subjects: effective stage limits preserved, delayed completion absent after timeout, observed command exit, exact owned Agent/verifier resources removed, neighboring resources and shared images unchanged. The Python cancellation/normal-completion/race/cleanup-failure cases also pass. Observed: not-run.

Add a private bounded Docker environment class to each existing integration
module and select it in the native job's environment config. Bind the selected
import path and effective timeout configuration in result admission; a
substituted or missing supervision environment must not silently seal a score.
Supervise each exec invocation by an owned command identity/process group,
shield the observation needed to obtain its real exit, propagate asynchronous
cancellation and stop only that invocation's execution. Normal completion,
nonzero exit, timeout/exit races and repeated cancellation must settle once.
If exact command termination is uncertain, escalate only against verified
trial-owned container identity, retain diagnostics, and classify uncertainty
honestly. Agent timeouts with successful bounded cleanup may still proceed to
native grading; verifier timeouts remain invalid with unknown reward.

Keep resource cleanup bounded for shared and separate environments. Preserve
shared images and caches: Pier stop(delete=True) uses compose down --rmi all,
so it cannot be used indiscriminately. Retain upstream lifecycle behavior
while selecting/overriding the narrow teardown path that removes only owned
containers/volumes and never shared images. No global prune or neighboring
container kill is allowed. Material modules for green checks must be freshly
byte-pinned; do not mutate an existing immutable material manifest.

Update the native witness to assert command-start evidence and no delayed
command completion, count both environment and tests compose roots, and
separate actual command lifetime from environment startup/download/teardown.
The current blanket verifier-phase <4 s includes lifecycle overhead and is not
a valid independent limit for all separate environments. Prove the stricter
task deadline against a longer configured cap with distinguishable command
timing; a loose <4 s assertion alone does not prove a 0.5 s task cap. Keep all
three families, two subjects, positive, time-only, agent-only, verifier-only,
both caps (each stage), minimum cap, stricter-task and unbounded-task cases.
Use only local controlled no-model clients and real pinned runners/engine.

### Batch B2: Dispatch real verifier-only regrading with preserved artifacts

**Closure predicate**: PASS: nonempty explicit offline runner argv, real pinned verifier-only dispatch in all three families, preserved-input-sensitive correct/incorrect native grades, source bytes unchanged, missing/tampered cache rejected, zero Agent/provider invocations and credentials, bounded cancellation with owned-resource cleanup.
**Dependencies**: B1, including its complete real native stage witness
**Verification union**: cargo test -p opi-eval --locked --test native_driver plan_002_regrade_dispatches_a_native_entrypoint -- --exact --nocapture ; cargo test -p opi-eval --locked --test native_driver native_offline_regrade ; OPI_EVAL_NATIVE_WITNESS_MATERIAL=/home/luiz/.cache/opi-remediate-plan19-fb8c5146d1fc/regrade/material.json cargo test -p opi-eval --locked --test native_driver native_verifier_only_regrade_witness -- --exact --ignored --nocapture --test-threads=1; final verification union below.

#### Fix B2.1: Dispatch real verifier-only regrading with preserved artifacts

- **Finding source(s)**: `phase19-codex-gpt6-c18efa8-20260917t133157z` + `67c0ab6ca8b90178e3caf2d7cdd6cb4a6c9a6b2fe342c85d8cbd3ef105c95b36` + `P19-AUD-002`
- **Decision**: `fix:verifier-only-native-regrade`; the design above is part of this scope; the verifier-only regrade /2 revisions were approved in D1.
- **Verification status**: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `crates/opi-eval/integrations/harbor/opi_eval_harbor.py`, `crates/opi-eval/integrations/pier/opi_eval_pier.py`, `crates/opi-eval/scripts/test_native_integrations.py`, `crates/opi-eval/src/cli/regrade.rs`, `crates/opi-eval/src/cli/run.rs`, `crates/opi-eval/src/regrade.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/report_contract.rs`
- **Change kind**: behavioral
- **Change**: Implement only the bounded behavior detailed below.
- **Closure predicate**: PASS: nonempty explicit offline runner argv, real pinned verifier-only dispatch in all three families, preserved-input-sensitive correct/incorrect native grades, source bytes unchanged, missing/tampered cache rejected, zero Agent/provider invocations and credentials, bounded cancellation with owned-resource cleanup.
- **Red-before**: `cargo test -p opi-eval --locked --test native_driver plan_002_regrade_dispatches_a_native_entrypoint -- --exact --nocapture` — FAIL: both fresh verifier children received argc=0; the recording executable exited 77 and regrade returned fresh-verifier-failed:77. Observed in the final plan_ filter run, exit 101; production regrade.rs supplies no argv.
- **Green-after**: `cargo test -p opi-eval --locked --test native_driver plan_002_regrade_dispatches_a_native_entrypoint -- --exact --nocapture ; cargo test -p opi-eval --locked --test native_driver native_offline_regrade ; OPI_EVAL_NATIVE_WITNESS_MATERIAL=/home/luiz/.cache/opi-remediate-plan19-fb8c5146d1fc/regrade/material.json cargo test -p opi-eval --locked --test native_driver native_verifier_only_regrade_witness -- --exact --ignored --nocapture --test-threads=1` — expected PASS: nonempty explicit offline runner argv, real pinned verifier-only dispatch in all three families, preserved-input-sensitive correct/incorrect native grades, source bytes unchanged, missing/tampered cache rejected, zero Agent/provider invocations and credentials, bounded cancellation with owned-resource cleanup. Observed: not-run.

Replace the zero-argv raw Command in regrade.rs with an explicit invocation
of the pinned runtime, for example uv run --offline --locked --no-sync
--project <verified-runner> python -m <pinned-family-module> regrade
--request <private-request>. Exact argv must follow the locked runtime's
observed interface. The private request binds the source bundle/trial,
task/lifecycle, artifact digest map, immutable environment, runner/module
identity and fresh output root; it carries no credentials or Agent config.

The family entry creates a fresh upstream verifier environment, uses upstream
artifact restoration rules (including the Pier committed patch/base and TB3
declared separate transfer), calls the actual native Verifier, preserves raw
results and emits a verifier-only observation. Validate the observation and
its input provenance before a derived /2 record; retain source Agent identity
only as source provenance, never as a newly executed Agent. Remove the
test-only environment-variable pseudo-grader contract from production.

Reuse ProcessSupervisor::run with a closed SpawnSpec and finite limits. Make
the existing CLI signal installation reusable within the private CLI modules
and pass one cancellation token into regrade. Cancellation/timeout must stop
new derived work, preserve attempted/unknown evidence, and clean exact owned
resources through B1's same supervised environment. Keep the ambient
credential negative probe and output-containment/tamper checks working.

Update the native_driver stand-in to observe the real explicit entry/request
shape; keep its argv assertions independent from the implementation. Add an
explicit ignored native_verifier_only_regrade_witness requiring fresh pinned
no-model material: all three task families, correct/incorrect preserved input,
source byte identity, missing cache/input and tamper negatives, zero
Agent/provider counters, credential canaries, no network fallback and owned
resource cleanup. Correct/incorrect cases use separately sealed source runs;
altering a sealed source artifact is only a rejection case.

### Batch B3: Publish native task and test-case metrics with distinct meaning

**Closure predicate**: PASS: family/revision-specific native task outcome definitions and denominators, separate measured test-case fraction (including a partial pass with task reward zero), explicit unknown fraction without native counters, invalid-trial exclusion, provenance/tamper checks, deterministic offline reconstruction and honest subset/missing coverage.
**Dependencies**: B1 and B2; preserve shared-file result bindings
**Verification union**: cargo test -p opi-eval --locked --test native_driver plan_011_native_metric_meanings_are_distinct -- --exact --nocapture ; cargo test -p opi-eval --locked --test report_contract --test end_to_end_report --test bundle_recompute --test report_output_containment; final verification union below.

#### Fix B3.1: Publish native task and test-case metrics with distinct meaning

- **Finding source(s)**: `phase19-codex-gpt6-c18efa8-20260917t133157z` + `67c0ab6ca8b90178e3caf2d7cdd6cb4a6c9a6b2fe342c85d8cbd3ef105c95b36` + `P19-AUD-011`
- **Decision**: `fix:benchmark-metric-semantics`; the design above is part of this scope; the verifier-only regrade /2 revisions were approved in D1.
- **Verification status**: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `crates/opi-eval/src/benchmark/process.rs`, `crates/opi-eval/src/report.rs`, `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/tests/bundle_recompute.rs`, `crates/opi-eval/tests/end_to_end_report.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/report_contract.rs`, `crates/opi-eval/tests/report_output_containment.rs`
- **Change kind**: behavioral
- **Change**: Implement only the bounded behavior detailed below.
- **Closure predicate**: PASS: family/revision-specific native task outcome definitions and denominators, separate measured test-case fraction (including a partial pass with task reward zero), explicit unknown fraction without native counters, invalid-trial exclusion, provenance/tamper checks, deterministic offline reconstruction and honest subset/missing coverage.
- **Red-before**: `cargo test -p opi-eval --locked --test native_driver plan_011_native_metric_meanings_are_distinct -- --exact --nocapture` — FAIL: three family reports all published metric=native-grader-reward and an identical generic definition, denominator=2. Observed in the final plan_ filter run, exit 101.
- **Green-after**: `cargo test -p opi-eval --locked --test native_driver plan_011_native_metric_meanings_are_distinct -- --exact --nocapture ; cargo test -p opi-eval --locked --test report_contract --test end_to_end_report --test bundle_recompute --test report_output_containment` — expected PASS: family/revision-specific native task outcome definitions and denominators, separate measured test-case fraction (including a partial pass with task reward zero), explicit unknown fraction without native counters, invalid-trial exclusion, provenance/tamper checks, deterministic offline reconstruction and honest subset/missing coverage. Observed: not-run.

Replace the report's one generic metric row with the admitted revision's
explicit native task outcome definition and denominator. Keep raw native
reward attribution and the existing immutable outcome classification. Add a
separate test-case fraction view only from verified native summary counters,
with numerator, denominator, source digest and observation coverage. Prefer
already retained native result/CTRF artifacts; when a production native path
does not currently retain the needed grader counter artifact, bind and seal
that exact artifact before publishing a measured fraction. Do not read
mutable outer logs as report authority or infer test counts from reward.

Use family-qualified metric identities and report/reporter /2. Retain missing
counter facts as unknown, reject provenance/tamper mismatches, keep measured
zero distinct from unobserved, and avoid a full-score claim for incomplete or
subset coverage. Add a public report case with task reward zero and a partial
native passed/total ratio, plus missing/zero-total counters, provider failure,
valid Agent timeout, verifier timeout, missing pairs and protocol mismatch.
The source bytes and scores remain unchanged during offline reconstruction.
Synchronize the actual contract callers/tests and bilingual documentation.

## Plan-stage observations and reproduction

| Command in the archive export | Observed |
|---|---|
| `cargo test -p opi-eval --locked --test native_driver remediation_ -- --nocapture` | PASS, 9 passed / 1 ignored, 89.53 s; eight current counterexamples plus stage-config projection. This does not establish native termination. |
| `cargo test -p opi-eval --locked --test native_driver plan_ -- --nocapture` | FAIL, 2 probe failures / 3 passing existing plan cases; final rerun after making the argv recorder independent of OPI_EVAL_REGRADE. |
| Explicit ignored native deadline command in B1 | FAIL, exit 101, 194.47 s, at TB2.1 verifier-cap; later matrix entries not run. |
| Fresh-root exact Docker ownership check | PASS, 16 Agent/verifier working directories checked, zero matching containers after settlement; this is independent of the failing deadline predicate. |

The B1 material SHA-256 was
`b01c5956a37f364a4be05d3931c11a8f03d825579c0d2ab02831792cce2ee31c`.
It uses independent exact pinned Harbor/Pier sources, locked installed runtimes,
an immutable locally cached Python image and controlled delay tasks/clients.
It establishes no real Opi/pi, official corpus, six-trial or provider acceptance.
The new evidence root is `/tmp/.tmpM95EMZ`. Both verifier-cap trials recorded
VerifierTimeoutError at about 1.08 s and then produced `verifier/stage-end`.
The observed verifier finish and marker times were respectively
`03:56:43.597233Z` / `03:56:44.004448Z` for Opi and
`03:56:57.953243Z` / `03:57:01.089647Z` for pi on 2026-09-18.

Pinned source cause: Harbor trial.py `_run_shared_verifier` and
`_run_separate_verifier` await `asyncio.wait_for(verifier.verify(), ...)`;
Docker `_collect_buffered_output` handles its own TimeoutError but does not
settle the container command on outer CancelledError. Losing a Docker CLI
transport therefore does not prove remote command termination. The result
binding must retain the native exception while the adapter proves termination.
Pier's corresponding fixed API is independently checked through its factory,
Verifier and Docker implementations; passing Harbor alone cannot close B1.

Reproduce from a fresh archive of the exact remediation head, append only the
following two tests to native_driver.rs, and use the existing external target
with a cache lease. The helpers are already committed in native_driver.rs.
The argv check is an entry-dispatch negative, not a substitute for the required
real native green witness. The metric check is a current generic-definition
negative, not sufficient by itself to prove the new fraction semantics.

```rust
#[test]
fn plan_002_regrade_dispatches_a_native_entrypoint() {
    let staged = stage("ok");
    remediation_runner(&staged, |body| body.replacen(
        "set -eu",
        r#"set -eu
printf '%s\n' "$#" > observed-regrade-argc
if [ "$#" = 0 ]; then
  echo 'pinned uv received no executable/module/subcommand arguments' >&2
  exit 77
fi"#,
        1,
    ));
    let config = write_config(&staged);
    let (run_root, _) = regrade_family_root(&staged, &config, "initial");
    let out = staged.root.join("regrade");
    let (code, report, stderr) = regrade_run(&staged, &run_root, &out);
    let counts = walk_files_matching(&out, "observed-regrade-argc");
    assert_eq!(counts.len(), 2);
    for path in counts {
        let argc: usize = fs::read_to_string(path).unwrap().trim().parse().unwrap();
        assert!(argc > 0, "native regrade entrypoint absent: argc={argc}; exit={code}; {report}; {stderr}");
    }
    assert_eq!(code, 0, "{report}; {stderr}");
}

#[test]
fn plan_011_native_metric_meanings_are_distinct() {
    let staged = stage("ok");
    let mut definitions = std::collections::BTreeSet::new();
    for (family, config) in [
        ("terminal-bench-2.1", write_config(&staged)),
        ("terminal-bench-3.0", write_tb30_config(&staged)),
        ("deepswe-v1.1", write_deepswe_config(&staged)),
    ] {
        let (_, report) = regrade_family_root(&staged, &config, family);
        let report: serde_json::Value = serde_json::from_str(&report).unwrap();
        let metric = &report["metrics"][0];
        eprintln!("{family}: {metric}");
        definitions.insert((metric["metric"].to_string(), metric["definition"].to_string()));
    }
    assert!(definitions.len() > 1, "every family has identical generic metric semantics; task success and test fraction are not distinguished: {definitions:?}");
}
```

Local reproducibility files (outside the repository, not alternate plan paths):

- `E:/opi/remediate-plan19-fb8c5146d1fc/append_probes.py` — SHA-256 `a11863c657c0fdd8dba53b09007cb547d48a3f6387dddd75ff321bbec806c2fe`.
- `E:/opi/remediate-plan19-fb8c5146d1fc/checks.sh` — SHA-256 `8e1acf1bb92d3c50cd200cb1d54882af5849653c0ef2ec5171e1a184e5d090e3`.
- `E:/opi/remediate-plan19-fb8c5146d1fc/recheck_probe.sh` — SHA-256 `260606caccd02dc24abeabb5a4c38920bcfe6ff26f2956ca52fc7f041b64eda7`.
- `E:/opi/remediate-plan19-fb8c5146d1fc/inspect_witness.py` — SHA-256 `df8f9b62cfab35bdecf2b7dcdc9b721c039b781e1791aab58aab0f2ae467d39c`.
- `E:/opi/remediate-plan19-fb8c5146d1fc/regressions.log` — SHA-256 `c986bc5f4ebec4beb9ff8a3525d316a8b1b5acd3d3fd9923faa410b22736eca7`.
- `E:/opi/remediate-plan19-fb8c5146d1fc/red-probes-final.log` — SHA-256 `f6c89238f88535369bf7181f6ded0ae0cf55484004e8e53af4c680c7cc585117`.
- `E:/opi/remediate-plan19-fb8c5146d1fc/native-deadlines.log` — SHA-256 `60672a398dc6552fab2dee47ac6b002903f4a5a8d86c58ad69879390ed37ba69`.

## Apply material preparation for native green checks

B1 writes its new module-bound manifest to
`/home/luiz/.cache/opi-remediate-plan19-fb8c5146d1fc/stage/material.json`.
Reuse the independently prepared immutable runner/runtime/image and controlled
corpus/profile inputs from the red witness. Copy the changed integration
modules into a new owned directory and recompute their directory manifests
and material integrity using the existing material/5 contract. Preserve the
red material and its pinned modules byte-for-byte. Effective task declarations,
the nine cases and the delayed completion behavior stay unchanged. This is
the expected material identity change caused by the repair, not a new runner
pin or relaxed native task. The command-start/timing assertions are improved
as specified in B1; the delayed-completion rejection remains mandatory.

B2 prepares a separate controlled no-model material at
`/home/luiz/.cache/opi-remediate-plan19-fb8c5146d1fc/regrade/material.json`.
Its witness uses explicit sufficient preserved inputs in the shared task,
declared artifacts in the separate task, and a real committed patch/base in
the Pier task. It uses the same exact pinned runner/runtime and cached image,
the repaired module bytes and fresh source trial identities. The test records
the prepared manifest digest before dispatch and rejects drift. The apply
worker prepares these task-owned local inputs; no user-supplied eval archive
is requested or substituted. If the required native semantics cannot be
represented within these declared task/material contracts, the batch stays
Not closed and returns to planning instead of weakening the witness.

## Final Verification

Apply imports the two appendix probes as meaningful maintained regressions,
updates each affected exact test binary and executes the batch-specific native
witnesses. No standalone fixture score closes a real native predicate.
Deduplicate checks at the final relevant state; do not repeat unchanged evidence.

```sh
python scripts/opi-doc-check.py
git diff --check
cargo fmt --check --all
cargo clippy -p opi-eval --all-targets --locked -- -D warnings
cargo test -p opi-eval --all-targets --locked
python3 -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py'
```

Run the explicit B1 full matrix and B2 real regrade witness separately with
freshly pinned local no-model material, along with any affected opt-in Docker
integration cancellation witness. Default tests must remain credential-free
even with dummy ambient keys. Preserve raw outcomes and exact source/material
identity. Revalidate the current audit-set, plan/dispositions and apply result
with validate_assurance_artifact.py; any active-index change invalidates this
plan. Scope is one unpublished Companion's runtime and output contracts, so
crate all-targets tests/clippy plus script, fixture and native witnesses form
this repair gate. This is not Phase exit; workspace clippy/tests/doc tests/
rustdoc and independently authorized real acceptance remain their own
final-state requirements and are not claimed by these checks.

Plan-only validation is the dispositions validator, plan validator, audit-set,
documentation check and diff check. The fixed plan has no Chinese counterpart;
its workflow contract defines only these two filenames. Apply's user-facing
README changes must update both languages. Read the full Unreleased section
before editing; do not touch released history.

Test impact: plan-only repository change `none`; export-only behavioral probes
`add`; proposed apply `update` existing native/report/Python regressions and
`add` the missing real verifier-only regrade witness; `delete` none. Retain
the eight passing counterexample regressions and all unaffected tests.

## Exclusions

| Source / Finding ID | Disposition | Current evidence / authority |
|---|---|---|
| C / P19-AUD-001 | Refuted | regrade.rs:758-764 clears the environment before applying a closed projection; the subprocess probe observes seven ambient credential-shaped names absent in both successful and failed regrades. Export check: cargo test -p opi-eval --locked --test native_driver remediation_ -- --nocapture: 9 passed, 1 ignored, 0 failed. |
| C / P19-AUD-003 | Refuted | benchmark/process.rs now compares selected terminal product, executable digest and model, in addition to integration/job/task/artifact identity. The public-run substitution matrix rejects each altered field independently. Export check: cargo test -p opi-eval --locked --test native_driver remediation_ -- --nocapture: 9 passed, 1 ignored, 0 failed. |
| C / P19-AUD-004 | Refuted | runner/experiment.rs::native_agent_record reads captured native stdout/stderr and invokes the Opi/pi terminal classifiers. Authentication, transport/protocol errors and exit-zero pi error streams are excluded, while valid zero/retry-success/Agent-timeout controls retain their proper outcome. Export check: cargo test -p opi-eval --locked --test native_driver remediation_ -- --nocapture: 9 passed, 1 ignored, 0 failed. |
| C / P19-AUD-006 | Refuted | cli/run.rs resolves --plan from its frozen source and byte-bound material without expecting --config; fixtures are optional for native execution. Malformed plan/material input exits with a typed rejection, not panic. The plan_ filter also passed native_frozen_plan_runs_without_fixtures_and_rejects_material_drift. Export check: cargo test -p opi-eval --locked --test native_driver remediation_ -- --nocapture: 9 passed, 1 ignored, 0 failed. |
| C / P19-AUD-007 | Refuted | prepare.rs::EffectiveTaskRequirements retains separate Agent/verifier storage, network, stage limits and origins; check_requirements compares engine CPU/memory/storage/GPU capacity and types, with unknown selected capacity rejecting admission. plan.rs::expand_selection uses this path whenever native material is present; its old lightweight classify branch serves the non-native path. The impossible-storage public prepare probe rejects. Export check: cargo test -p opi-eval --locked --test native_driver remediation_ -- --nocapture: 9 passed, 1 ignored, 0 failed. |
| C / P19-AUD-008 | Refuted | material.rs::BenchmarkMaterial.tasks verifies every selected task against the complete pinned corpus and its own profile/package digests. plan.rs expands selected canonical task IDs against that map rather than a single fixed profile; the feasible second-task probe passes. Export check: cargo test -p opi-eval --locked --test native_driver remediation_ -- --nocapture: 9 passed, 1 ignored, 0 failed. |
| C / P19-AUD-009 | Refuted | runner/experiment.rs reserves run-control.json with create_new and rejects execution evidence before dispatch; rerun rejection preserves control and reconstructed report bytes. Export check: cargo test -p opi-eval --locked --test native_driver remediation_ -- --nocapture: 9 passed, 1 ignored, 0 failed. |
| C / P19-AUD-010 | Cannot confirm | The active Codex requirement record set names 30 acceptance obligations. The active pi requirements assert met at a different audit head. Their original records remain distinct; peer PASS is not independent closure evidence. This workflow excludes eval archives and prior conclusions, and has no authorization for new provider calls. No archive was searched or consumed. |
| C / P19-AUD-012 | Refuted | runner/experiment.rs records Fact::Unknown(native-agent-duration-unobserved); trajectory projection preserves that fact. Both native execution nodes retain an explicit unknown reason instead of measured zero. Export check: cargo test -p opi-eval --locked --test native_driver remediation_ -- --nocapture: 9 passed, 1 ignored, 0 failed. |
| P / P19-AUD-001 | Info/no-action | Current Git commit object 4bf5fc0b64c3e93aee5726713670ffa34196038d; inspected path statistics only, without reading acceptance artifacts. |

Excluded findings still receive exactly one machine disposition. No source
outside the live run/digest set is admitted; title similarity never substitutes
for source identity; no older audit is consulted. No source is deferred by an
invented authority or closed by another member's verdict.

Only the plan pair is renewed in this mode. The tracked result pair is not a
result for this new plan; no apply-result claim is emitted. An approved apply
must replace it with exact current-plan coverage and observed evidence.
