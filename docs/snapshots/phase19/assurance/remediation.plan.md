# Phase 19 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `7550892cafbe6db2df1b56cd3f2eadb2104303428019940d4dc341e621f48f9d`
**Remediation head**: `131f003d4ae883e30df3ec85eae7469b0d8c60cc`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: worktree-only continuation; staged=[]; unstaged=["CHANGELOG.md","crates/opi-eval/README.md","crates/opi-eval/README.zh.md","crates/opi-eval/src/agent/opi.rs","crates/opi-eval/src/agent/pi.rs","crates/opi-eval/src/benchmark/deepswe.rs","crates/opi-eval/src/benchmark/process.rs","crates/opi-eval/src/cli/run.rs","crates/opi-eval/src/main.rs","crates/opi-eval/src/regrade.rs","crates/opi-eval/src/runner/experiment.rs","crates/opi-eval/tests/assembled_run.rs","crates/opi-eval/tests/fail_closed_cli.rs","crates/opi-eval/tests/native_driver.rs"]; untracked=["docs/snapshots/phase19/assurance/remediation.plan.dispositions.jsonl","docs/snapshots/phase19/assurance/remediation.plan.md"]; committed=none-for-this-task
**Unresolved decisions**: none

This plan covers all 13 source keys in revision 2 of the active set. Eleven
confirmed behavioral findings have eleven separate repair batches. The external
evidence finding and provenance Info finding are partially confirmed and have
two explicit exclusions. The user accepted D1, D2 and D3's recommended choices
with “同意”; the resulting contract, path and verification scope is fixed below.
No source key is refuted or silently dropped. Planning approval is not apply
authorization and this plan does not declare Phase conformance.

## Admission and committed baseline

`validate_assurance_artifact.py audit-set` passed before semantic consumption.
The active aggregate remains `FAIL`; the pi member's `PASS` does not override
any Codex finding. Full run identity, rather than textual finding ID, controls
coverage. In particular the two `P19-AUD-001` records are different source keys.

The committed baseline was inspected using `git archive` exports, not a Git
worktree. Windows export:
`C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-47ih7fw5/source`.
WSL/Linux test export: `/tmp/opi-remediate-phase19.EzEQLM6C`.
Only planning probes were appended to the exported `native_driver.rs`;
production source remained byte-identical to the recorded head. Rust 1.97.1,
Ubuntu WSL, incremental compilation, and the existing repository cache
`/mnt/e/opi/cargo-targets/opi-8abcf38f9871653e` were used. The cache was resolved
with `opi-cargo-cache.py resolve --workspace /mnt/d/Luiz/Odradek/opi --root
/mnt/e/opi/cargo-targets` and leased during Cargo runs; no per-session target
directory was created.

At the original planning admission all four fixed remediation output paths were absent; there was no
carried-in work or output overlap. `git diff --name-status` from the Codex audit
head to the remediation head for `crates/opi-eval`, Cargo manifests/lock, and the
English normative sources was empty. Source hashes were also rechecked; the
registered baseline files match both indexed metadata records. No prior audit
history, earlier remediation, eval artifacts, or task conclusions were used.
The pi finding's Git commit path inventory is direct source evidence, not an
earlier remediation conclusion.

The normative parent is `docs/opi-spec.md` (PRIN-004/005, CTRL-002 through
CTRL-007, PHASE-006). The registered Phase source is
`docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`
(sections 4-9, P19-AC-001 through P19-AC-013). Its English source owns meaning;
the Chinese registration is byte-bound too. These fixed assurance artifact names
have no maintained Chinese counterpart; no parallel translated plan is created.

### User-authorized continuation baseline

The current task began apply with approved digest
`56f1e066147768e77a609466bb415f60c7cca70ea029f5458d9c331b979c4005`.
B1-B4 changes and their checks below were observed in this same worktree and
remain uncommitted. Before B5 production edits, the user explicitly selected
“修订验证顺序并生成新计划摘要”: retain those changes and revise the scheduling
of B5's material/5 native witness. This authorizes this fixed-plan amendment;
it does not authorize applying its new digest. No result pair is fabricated
for unfinished B5-B11. The committed head, active index, member sidecars,
D1-D3 product contracts and eleven closure predicates are unchanged.

The dirty paths are owned effects of this same task, not unrelated overlap.
On the next apply, verify HEAD/index/plan and these byte hashes, preserve the
changes, and resume B5 after checking that B1-B4's evidence is still applicable.
Unexpected overlapping changes require reconciliation before further edits.
The two fixed untracked plan files are bound by the validator-emitted digest.

| Carried-in task-owned path | SHA-256 at this amendment |
|---|---|
| `CHANGELOG.md` | `edf902411030ac58cbc5568d06f97c15b5c4d439626ef63cde31663d50c45d36` |
| `crates/opi-eval/README.md` | `60c28f2612bf73e3998e8faf916388d46a0ea9eba365aec4c5d6f5e5b095078a` |
| `crates/opi-eval/README.zh.md` | `662ce0a2fbab1d5b51b933d153b6eb86b47fb2917c201afc1ca8c5fb8eb6976e` |
| `crates/opi-eval/src/agent/opi.rs` | `af99db58343677231c94821d86f9967d66948a5f41c50007714582e25dca1421` |
| `crates/opi-eval/src/agent/pi.rs` | `9f96167b60e6c98f9e1c94d799d655dbfa16fc7707a43b77b7d5192d26d09ca4` |
| `crates/opi-eval/src/benchmark/deepswe.rs` | `4aabe51ea5e6269b00ecef93160a75b10f3e9053bc02b5a27163a2665128fbc0` |
| `crates/opi-eval/src/benchmark/process.rs` | `056252168c645c0ba451ccdfa0b1adbcea45b71001f1ebc62b30ee1866ea6128` |
| `crates/opi-eval/src/cli/run.rs` | `e696aac34f301e08dcdef3ab5f31ef3d868d31f755c19a6293bdf65003573348` |
| `crates/opi-eval/src/main.rs` | `ac0290f28346917120f4bed50e701bdc34c0a87a098db1501eaf29563eff25cc` |
| `crates/opi-eval/src/regrade.rs` | `bef82f27219b221ffa022541d26c859d5b0122f2fd9ca380b05e2875e7adaaf1` |
| `crates/opi-eval/src/runner/experiment.rs` | `cffbb4e7d11434e042141bf355190f7152b0499af35e1d56ec7e6bbbb1a6d8b9` |
| `crates/opi-eval/tests/assembled_run.rs` | `47247c957e6d7224d11e0e342f0b0c2dc5f697f000557689d5161c0cb4ab65c3` |
| `crates/opi-eval/tests/fail_closed_cli.rs` | `553875e37d4ebc02cb340f9356fc169b68b7e933b5c10170b110c4663a1d3370` |
| `crates/opi-eval/tests/native_driver.rs` | `4e1773c777d719e16319d5811772f669a4e68f63f4a1eeb162324f16ad95bb70` |

B1's expanded actual-child probe observed ambient credentials before the fix
and now passes for arbitrary selected credential names, synthetic API_KEY,
TOKEN/SECRET/PASSWORD/CREDENTIAL variables, admitted non-secret configuration,
and both successful and failing verifiers. B2's missing-plan probe observed
panic 101 before typed rejection; the expanded fixture-free frozen-plan tests
pass for implicit/explicit material and independent-directory execution, with
pre-effect rejection of wrong digests, product drift, conflicting inputs and
missing hermetic fixtures. B3 observed twelve admitted product/build/model
substitutions or omissions across three families before the repair; matching
positives, every negative, pinned slash splitting and diagnostic isolation now
pass. B4's expanded pre-fix matrix observed all thirty provider-error scenarios
scored as valid=2, invalid=0; its sixty final scenarios now exclude terminal
provider errors while retaining retry success, native zero, task failure and
timeout, with sealed causes under both capture policies. These are observed
continuation evidence, not Phase acceptance or a new result disposition.

| Exact command (Cargo under WSL with the same leased external cache) | Observed at amendment |
|---|---|
| `cargo test -p opi-eval --locked --test native_driver remediation_001_regrade_child_has_no_ambient_secret -- --exact --nocapture --test-threads=1` | PASS: 1 test after observed credential-leak FAIL. |
| `cargo test -p opi-eval --locked --test native_driver remediation_006_plan_material_rejects_without_panic -- --exact --nocapture --test-threads=1` | PASS: 1 test after observed panic FAIL. |
| `cargo test -p opi-eval --locked --test native_driver remediation_003_substituted_terminal_identity_rejects -- --exact --nocapture --test-threads=1` | PASS: 1 expanded three-family matrix after observed substitution FAIL. |
| `cargo test -p opi-eval --locked --test native_driver remediation_004_native_provider_error_is_not_scored -- --exact --nocapture --test-threads=1` | PASS: 1 sixty-scenario matrix after observed scoring FAIL; later sealed-evidence assertions also passed in the full binary below. |
| `cargo test -p opi-eval --locked --test native_driver native_offline_regrade` | PASS: 2 tests after B3 caller migration. |
| `cargo test -p opi-eval --locked --lib benchmark::` | PASS: 54 tests, including the incidental DeepSWE constructor and diagnostic oracle isolation. |
| `cargo test -p opi-eval --locked --test native_driver --test assembled_run --test fail_closed_cli --test benchmark_integration_conformance --test agent_integration_conformance --test report_contract --test end_to_end_report --test bundle_recompute` | PASS: 90 passed, 7 material-gated tests ignored; those ignored tests are not native witness evidence. |
| `cargo test -p opi-eval --locked --lib` | PASS: 138 tests, including bounded client-capture and link rejection. |
| `cargo clippy -p opi-eval --all-targets --locked -- -D warnings` | PASS. |
| `cargo fmt --check --all` | PASS. |
| `python3 crates/opi-eval/scripts/test_native_integrations.py` | PASS: 21 tests, 1 skipped; mocked integration coverage only. |
| `python scripts/opi-doc-check.py` | PASS before amendment; rerun for publication. |
| `git diff --check` | PASS before amendment; rerun for publication. |

Test impact: add credential-isolation, frozen-plan, terminal identity, provider
classification, model-spelling and bounded-capture regressions; update existing
positive native fixtures and CLI requirement checks; retain existing conformance,
report, oracle, lifecycle and substitution coverage; delete none. No paid
provider call or native grader witness was executed. Docker reports a reachable
29.7.2 engine, but `OPI_EVAL_NATIVE_WITNESS_MATERIAL` is unset. Preparing or
supplying verified no-model material/5 remains a prerequisite for the deferred
witness; available images alone do not prove the required runner/task identity.

One bounded incidental, I1, must be carried into the eventual result machine
records: B3's private `SelectionKind::Integration` payload change caused
`cargo clippy -p opi-eval --all-targets --locked -- -D warnings` to FAIL with
E0533 at the existing DeepSWE test constructor in
`crates/opi-eval/src/benchmark/deepswe.rs`. Adding its already-known expected
product/digest is test-only, directly necessary and inside the causal surface;
it changes no public API, durable format, dependency, specification, authority,
manifest, lockfile or ledger. The identical clippy command subsequently PASSed,
and `benchmark::deepswe::adapter_tests::settle_matrix_maps_unpinned_and_pier_report_outputs`
passed in the exact benchmark library filter above. Preserve I1 as a separate
`incidental-repair` record with trigger B3, all required bounded guardrails,
that sole changed path, the observed FAIL/PASS pair, and status Closed; do not
silently absorb it into a source finding or discard its evidence.

### Current member identities

| Member | Full audit run ID | Audit head | Findings SHA-256 | Requirements | Findings |
|---|---|---|---|---:|---:|
| `audit.codex.gpt6` | `phase19-codex-gpt6-c18efa8-20260917t133157z` | `c18efa83191a577f4f115b25a439febfc8ad68ad` | `67c0ab6ca8b90178e3caf2d7cdd6cb4a6c9a6b2fe342c85d8cbd3ef105c95b36` | 71 | 12 |
| `audit.pi.glm53` | `phase19-pi-glm53-d61590b-20260917t152753z` | `d61590b9aeae1685d605a4b86dd7a24f1151de1c` | `b85fa7a405aee5d3da9ce410bcd9619cf55af85fb3bce75a4c8b1f9aae96bc0b` | 14 | 1 |

### Criterion cross-reference

Both members bind the same registered source path and digest, but their citation
strings and granularity differ. The following is an explicit parent-criterion
crosswalk, not a claim of identical `(path, sha256, citation)` records. The
original citations, states, checks, and reciprocal finding links remain in each
unchanged indexed requirements sidecar. Codex subrequirements are counted by
state; pi's coarser record retains its own `met` state. No majority rule applies.

| Parent criterion | Codex source states | pi source record/state | Current disposition basis |
|---|---|---|---|
| P19-AC-001 | met=2, partially-met=3 | P19-AC-001: met | P19-AUD-005, P19-AUD-006, P19-AUD-007, P19-AUD-010 |
| P19-AC-002 | not-met=2, partially-met=3 | P19-AC-002: met | P19-AUD-003, P19-AUD-004, P19-AUD-010 |
| P19-AC-003 | not-assessable=2, partially-met=1 | P19-AC-003: met | P19-AUD-010 |
| P19-AC-004 | not-assessable=2, partially-met=2 | P19-AC-004: met | P19-AUD-003, P19-AUD-010 |
| P19-AC-005 | not-assessable=2, partially-met=2 | P19-AC-005: met | P19-AUD-003, P19-AUD-010 |
| P19-AC-006 | not-assessable=2, not-met=2, partially-met=3 | P19-AC-006: met | P19-AUD-004, P19-AUD-005, P19-AUD-010 |
| P19-AC-007 | met=2, not-met=3, partially-met=2 | P19-AC-007: met | P19-AUD-005, P19-AUD-007, P19-AUD-008, P19-AUD-010 |
| P19-AC-008 | met=4, not-met=2, partially-met=1 | P19-AC-008: met | P19-AUD-004, P19-AUD-011, P19-AUD-012 |
| P19-AC-009 | met=3, not-assessable=1, not-met=1, partially-met=2 | P19-AC-009: met | P19-AUD-002, P19-AUD-010 |
| P19-AC-010 | not-assessable=1, not-met=2, partially-met=5 | P19-AC-010: met | P19-AUD-001, P19-AUD-005, P19-AUD-009, P19-AUD-010 |
| P19-AC-011 | met=3, partially-met=1 | P19-AC-011: met | P19-AUD-010 |
| P19-AC-012 | not-assessable=4 | P19-AC-012: met | P19-AUD-010 |
| P19-AC-013 | met=1, partially-met=3 | P19-AC-013: met | P19-AUD-006, P19-AUD-009, P19-AUD-010 |

The pi `P19-SCOPE-001` record is separately `met`. Codex's two optional live
OpenAI-protocol records are separately `not-assessable` and link no finding;
they create no remediation work. This crosswalk preserves 71 + 14 requirement
records and the strict union of 12 + 1 findings.

## Current Finding Verification

`C` below expands to the full Codex run and findings digest in the identity
table; `P` expands to the full pi run and digest. Every machine disposition
contains those full values. Source severity is preserved without alteration.

| Source run / Finding ID | Verification | Source and final severity + rationale | Closure key/family | Batch | Decision |
|---|---|---|---|---|---|
| C / P19-AUD-001 | Confirmed | Blocker -> Blocker; Current code and independent FAIL reproduce the claim. | `regrade.verifier-credential-isolation` / `regrade.authority` | B1 | `fix:regrade.verifier-credential-isolation` |
| C / P19-AUD-002 | Confirmed | Major -> Major; Current code and independent FAIL reproduce the claim. | `regrade.real-native-grader-dispatch` / `regrade.execution` | B10 | `fix:regrade.real-native-grader-dispatch` |
| C / P19-AUD-003 | Confirmed | Major -> Major; Current code and independent FAIL reproduce the claim. | `native-result.selected-identity-binding` / `native-result.integrity` | B3 | `fix:native-result.selected-identity-binding` |
| C / P19-AUD-004 | Confirmed | Major -> Major; Current code and independent FAIL reproduce the claim. | `native-result.provider-failure-exclusion` / `native-result.settlement` | B4 | `fix:native-result.provider-failure-exclusion` |
| C / P19-AUD-005 | Confirmed | Major -> Major; Current code and independent FAIL reproduce the claim. | `native-execution.declared-deadlines` / `native-execution.supervision` | B5 | `fix:native-execution.declared-deadlines` |
| C / P19-AUD-006 | Confirmed | Blocker -> Blocker; Current code and independent FAIL reproduce the claim. | `cli.frozen-plan-native-material` / `local-workflow.plan-execution` | B2 | `fix:cli.frozen-plan-native-material` |
| C / P19-AUD-007 | Confirmed | Major -> Major; Current code and independent FAIL reproduce the claim. | `preflight.complete-resource-admission` / `materials.resource-admission` | B8 | `fix:preflight.complete-resource-admission` |
| C / P19-AUD-008 | Confirmed | Major -> Major; Current code and independent FAIL reproduce the claim. | `selection.multitask-native-execution` / `materials.task-selection` | B9 | `fix:selection.multitask-native-execution` |
| C / P19-AUD-009 | Confirmed | Major -> Major; Current code and independent FAIL reproduce the claim. | `run-root.rejected-reuse-preserves-evidence` / `evidence.immutability` | B6 | `fix:run-root.rejected-reuse-preserves-evidence` |
| C / P19-AUD-010 | Partially confirmed | Major -> Major; External evidence completeness cannot be inferred from this allowed context. | `acceptance.current-native-evidence-completeness` / `acceptance.evidence-admission` | exclusion | `no-action:cannot-confirm-external-acceptance-evidence` |
| C / P19-AUD-011 | Confirmed | Major -> Major; Current code and independent FAIL reproduce the claim. | `report.native-metric-semantics` / `evidence.metric-semantics` | B11 | `fix:report.native-metric-semantics` |
| C / P19-AUD-012 | Confirmed | Major -> Major; Current code and independent FAIL reproduce the claim. | `trajectory.native-agent-duration-honesty` / `evidence.telemetry` | B7 | `fix:trajectory.native-agent-duration-honesty` |
| P / P19-AUD-001 | Partially confirmed | Info -> Info; Mixed paths confirmed; exact opi-ai attribution overstated. | `provenance.mixed-acceptance-commit` / `provenance.delivery` | exclusion | `no-action:preserve-committed-provenance` |

### Independent evidence and counter-evidence

| Codex finding | Current committed source and observed result | Counter-evidence and limit |
|---|---|---|
| P19-AUD-001 | `src/regrade.rs:724-768`: validates the projected map, then inherits the parent environment. Actual-child canary probe sees `present`. | Existing env.json assertion checks only the recorded map; removing OPENAI_API_KEY does not remove ANTHROPIC_API_KEY. |
| P19-AUD-002 | `src/runner/prepare.rs:516,704-715` pins uv 0.9.7; `src/regrade.rs:757-768` supplies no argv. Probe observes argc=0. No production `OPI_EVAL_REGRADE` consumer exists in integrations/scripts. | `tests/native_driver.rs:192` implements the private regrade protocol only in a simulator. This is not proof of upstream dispatch or fresh native grading. |
| P19-AUD-003 | `src/benchmark/process.rs:475,552-563,897-935` copies model and parses terminal product/hash without equality to selection. All three independently altered fields still score. | Agent name/import, task, aggregate, lifecycle, and artifact comparisons exist and must remain; they do not validate the missing fields. |
| P19-AUD-004 | `integrations/harbor/opi_eval_harbor.py:309` and Pier sibling `:348` emit generic nonzero tags; `src/runner/experiment.rs:1313-1383,1498-1547` classifies tags and fabricates an empty-stream Agent record. Probe yields valid=2. | Dedicated Opi/pi parser coverage exists (`src/agent/opi.rs:261`, pi terminal stream parser), but is disconnected from native settlement. The probe proves the nonzero path; exit-zero pi coverage is required for closure. |
| P19-AUD-005 | `src/experiment.rs:379-397` defines per-Agent and per-verifier bounds; plan displays them; the native `BenchmarkRunRequest` at `src/runner/experiment.rs:1739` carries none. The refined actual-spawn probe observes neither Agent nor verifier caps for any of the three individually configured bounds. | Profile whole-job supervision and cancellation exist; they are not the accepted per-stage controls. The earlier whole-job sleep probe is not admitted as stage-enforcement evidence; the refined config probe still requires native delayed-stage green checks. |
| P19-AUD-006 | `src/cli/run.rs:94-100` panics at missing config; `src/main.rs:32-66` permits plan/material and requires fixtures. Both README examples omit fixtures. | The plan-only material branch checks digest, but the explicit-material arm bypasses that branch and panics first. Probe supplies fixtures to isolate the panic. |
| P19-AUD-007 | `src/runner/prepare.rs:912-1018` retains only CPU/memory/GPU count; `src/runner/plan.rs:170-222` treats runtime registration as capacity. Impossible storage is admitted and omitted. | Doctor does observe disk availability, but no task storage comparison uses it. The dynamic probe proves storage loss; GPU/type/network/stage claims have static evidence and require B8's matrix; reviewed task models declare no VRAM requirement. |
| P19-AUD-008 | `src/runner/plan.rs:338-349` explicitly rejects every non-profile task. Two feasible IDs reproduce the rejection. | Enumeration and hermetic expansion work; removing this guard alone would discard package admission. A per-task material contract is required. |
| P19-AUD-009 | `src/runner/experiment.rs:664-670` overwrites run-control before intent rejection at `:1567`; probe observes changed bytes. `src/report.rs:399-408` rejects mismatched sealed/control identity. | Existing reused-trial rejection protects the intent but not the run-level control. Dynamic red proves mutation; post-rejection offline report success and a complete tree-byte comparison are closure checks. |
| P19-AUD-010 | `tests/native_driver.rs:1-19` explicitly scopes default tests to stand-ins; material-gated real witnesses remain ignored. Thirty reciprocal requirements depend on original native/acceptance evidence. | pi reports offline verification of an external root, but that conclusion is not an independent current evidence substitute. This plan neither asserts the root is absent everywhere nor reads excluded eval artifacts. |
| P19-AUD-011 | `src/report.rs:821-829` uses one constant metric/definition for all revisions; public report probes reproduce identical definitions. | Exact native scalar and valid-trial denominator survive. `src/benchmark/process.rs:415-420,515-520` admits integral 0/1 rewards and separately models CTRF counters; inventing a fractional reward interpretation is not a valid prose fix. |
| P19-AUD-012 | `src/runner/experiment.rs:1544` constructs zero duration; `src/trajectory/mod.rs:624-630` labels it Known/supervisor-wall-clock. Corrected probe reads the actual externally tagged Fact and fails. | Whole-job/grader elapsed time and genuine host-Agent measurements exist, but cannot establish native Agent-only elapsed time. |

Paths in that table are relative to `crates/opi-eval/`. Source findings and their
evidence stay unchanged in the indexed sidecars. For P / P19-AUD-001,
`git show --stat 4bf5fc0` independently confirms acceptance artifacts alongside
opi-agent and opi-coding-agent changes; that exact commit lists no opi-ai paths.
The already committed provenance is retained with no repair or ledger edit.

## Unresolved Decisions

none.

### Accepted decisions

The owning user's “同意” accepts the three recommended choices presented in
this continuing plan task. It authorizes final plan refinement; a new
digest-bearing apply invocation remains required by the invoked skill.

| ID | Accepted contract | Owning batches |
|---|---|---|
| D1 | material/5 per-task map, full pinned inventory, effective Agent/verifier requirements with origins and required-observation rejection; run-plan/2 and plan-preview/2. Existing doctor/prepare schemas receive additive observations. | B8 resources and B9 task selection, separate predicates. |
| D2 | Private pinned Harbor/Pier verifier-only replay; sufficient separate-task evidence may regrade, incomplete shared state explicitly rejects under section 6. No snapshot engine. Changed outputs use regrade-record/2 and native-regrade/2. | B10; B1 separately owns credential isolation. |
| D3 | normalized-report/2 and reporter/2; integral reward and proven task success separated from test fractions with exact identities and honest denominators. | B11; B4 owns eligibility and B7 timing. |

D4 is resolved as a workflow disposition, not as acceptance: C/010 remains
Partially confirmed / Major with an explicit Cannot-confirm exclusion. The
current disposition contract permits that outcome; it does not require inventing
an evidence-input exception or reassessing all acceptance before code repairs.
The machine decision uses no-action:cannot-confirm-external-acceptance-evidence:
the READY validator requires no-action for an exclusion without a repair batch.
This describes no code action in this workflow, not Info severity or acceptance
closure. Verification remains Partially confirmed / Major; a later result must
retain Cannot confirm, not Closed. All 30 linked requirements remain unchanged. The operator/evidence owner may
later supply the originals to a separately invoked independent audit, after
fixes and the active set are committed and clean. This plan neither reads those
archives nor authorizes a live run, a new audit, or a specification change.

D5 is the user's accepted verification-order amendment. B5 has two checkpoints:
its dispatch/configuration and offline regression gate before B6-B9, and its
unchanged real-stage closure gate immediately after B9 makes verified material/5
available. Passing the first gate permits B6-B9 implementation, but never marks
B5 Closed. This is the explicit exception to the normal prior-batch closure
sequence. B8/B9 retain their already coordinated schema/requirements checks.
After B9, run the combined B8/B9 gates and B5's exact real-stage witness; every
one must pass before B10 begins. Failure or missing material stops there and
leaves the respective source finding Not closed. Neither a simulator nor a
whole-job timeout substitutes for B5's actual delayed-stage checks. No closure
predicate, path ownership, finding disposition, or required check is removed.

D1-D3's accepted details and pinned evidence remain below. B5 uses those
locked APIs without introducing another public execution control.

## Closure Batches

Schedule Blocker B1/B2 before Major batches. A family key is descriptive only:
credential isolation and actual regrading are not one batch; resource admission
and multi-task selection are not one batch; score identity, provider failure,
metrics, and telemetry are distinct predicates. No two current findings share
one closure key, so none is clustered. Shared files/tests do not justify merging
their evidence. Each repair batch also owns its relevant Unreleased entry.

The commands below refer to the self-contained probe appendix installed only
in a fresh committed-source export. At apply time migrate the discriminating
cases into existing test surfaces, repair positive fixture identities under B3,
and execute the expanded closure matrices; a narrow probe alone does not close
a broader source claim. Plan-stage `green_after.observed` values remain
`not-run`; same-task B1-B4 observations are preserved in the continuation
baseline above and must not be confused with approval of this amended plan.

### Batch B1: Close the actual regrade child environment

**Closure predicate**: No ambient or explicitly rejected provider credential reaches the regrade child; admitted non-secret verifier configuration still works.
**Dependencies**: none; first priority (source Blocker)
**Verification union**: `cargo test -p opi-eval --locked --test native_driver remediation_001_regrade_child_has_no_ambient_secret -- --exact --nocapture --test-threads=1`; `cargo test -p opi-eval --locked --test native_driver native_offline_regrade`; affected-target clippy and the final union below.

#### Fix B1.1: Close the actual regrade child environment

- **Finding source(s)**: `phase19-codex-gpt6-c18efa8-20260917t133157z` + `67c0ab6ca8b90178e3caf2d7cdd6cb4a6c9a6b2fe342c85d8cbd3ef105c95b36` + `P19-AUD-001`.
- **Decision**: `fix:regrade.verifier-credential-isolation`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/regrade.rs`, `crates/opi-eval/tests/native_driver.rs`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Clear the child environment, then install only the already validated material verifier projection and explicit non-secret execution necessities. Preserve the credential-shaped projection rejection. Make the recorded credential_free claim describe the environment actually passed to the child. Extend the existing regrade test to observe the actual child environment, using synthetic credentials and isolated command environments.
- **Closure predicate**: No ambient or explicitly rejected provider credential reaches the regrade child; admitted non-secret verifier configuration still works.
- **Red-before**: `cargo test -p opi-eval --locked --test native_driver remediation_001_regrade_child_has_no_ambient_secret -- --exact --nocapture --test-threads=1` — FAIL: both verifier children were reached; the first actual-child observation was present, expected absent, for a synthetic ANTHROPIC_API_KEY. No real credential was used.
- **Green-after**: same discriminating check must PASS; The appended canary probe passes; extend it to an arbitrary selected credential name and TOKEN/SECRET/PASSWORD/CREDENTIAL names, while a required non-secret material variable reaches the child. Check a failing verifier too. These are environment tests, not proof of real native grading. Observed: `not-run`.

### Batch B2: Execute a frozen native plan without panic or fixture dependency

**Closure predicate**: A valid frozen native plan runs with either its recorded material or the same explicitly supplied material and without repository fixtures; missing, conflicting, tampered, or substituted inputs reject before effects with no panic.
**Dependencies**: none; source Blocker; retain existing digest validation
**Verification union**: `cargo test -p opi-eval --locked --test native_driver remediation_006_plan_material_rejects_without_panic -- --exact --nocapture --test-threads=1`; `cargo test -p opi-eval --locked --test native_driver`; `cargo test -p opi-eval --locked --test assembled_run`; `cargo test -p opi-eval --locked --test fail_closed_cli`; `python scripts/opi-doc-check.py`; affected-target clippy and the final union below.

#### Fix B2.1: Execute a frozen native plan without panic or fixture dependency

- **Finding source(s)**: `phase19-codex-gpt6-c18efa8-20260917t133157z` + `67c0ab6ca8b90178e3caf2d7cdd6cb4a6c9a6b2fe342c85d8cbd3ef105c95b36` + `P19-AUD-006`.
- **Decision**: `fix:cli.frozen-plan-native-material`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/cli/run.rs`, `crates/opi-eval/src/main.rs`, `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/assembled_run.rs`, `crates/opi-eval/tests/fail_closed_cli.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Resolve the verified embedded experiment before selecting material. An explicit material argument accompanying a plan must match that plan's material identity; it cannot bypass the recorded digest. Reuse the recorded material when no override is supplied. Make fixtures conditional on hermetic execution; the native route uses declared material only. Replace the config expect with typed rejection. Synchronize help and both README command examples, and record the deliberate 0.x CLI correction.
- **Closure predicate**: A valid frozen native plan runs with either its recorded material or the same explicitly supplied material and without repository fixtures; missing, conflicting, tampered, or substituted inputs reject before effects with no panic.
- **Red-before**: `cargo test -p opi-eval --locked --test native_driver remediation_006_plan_material_rejects_without_panic -- --exact --nocapture --test-threads=1` — FAIL: expected typed rejection exit 2 for missing input files; got panic exit 101 at src/cli/run.rs:98 before file validation.
- **Green-after**: same discriminating check must PASS; The missing-input probe exits 2. Add positive native plan execution from an independent directory, explicit/implicit same material, wrong digest, selected-products drift, both/neither config/plan, and retained hermetic fixture requirements. Assert no run root/trial effect on rejection. Observed: `not-run`.

### Batch B3: Bind terminal identity to the selected native execution

**Closure predicate**: A score seals only when the actual terminal product, executable digest, and model match the selected identity; each single-field substitution and omission rejects.
**Dependencies**: B1, B2; update positive stand-in identities before dependent batches
**Verification union**: `cargo test -p opi-eval --locked --test native_driver remediation_003_substituted_terminal_identity_rejects -- --exact --nocapture --test-threads=1`; `cargo test -p opi-eval --locked --test native_driver native_result_binding`; `cargo test -p opi-eval --locked --test benchmark_integration_conformance`; `cargo test -p opi-eval --locked --lib benchmark::process`; affected-target clippy and the final union below.

#### Fix B3.1: Bind terminal identity to the selected native execution

- **Finding source(s)**: `phase19-codex-gpt6-c18efa8-20260917t133157z` + `67c0ab6ca8b90178e3caf2d7cdd6cb4a6c9a6b2fe342c85d8cbd3ef105c95b36` + `P19-AUD-003`.
- **Decision**: `fix:native-result.selected-identity-binding`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/benchmark/process.rs`, `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/src/regrade.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/benchmark_integration_conformance.rs`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Carry expected product and executable digest through the existing private NativeSelection and compare them with the terminal observation before reward admission. Validate the reported model against the frozen native selector using only explicitly proven pinned runner spelling differences; missing/unknown/mismatched identity must not silently match. Update direct-run and regrade callers. Replace the stand-in's a*64 digest with the selected handoff digest and give positive fixtures authentic selected identity fields. Do not relax oracle, task, stale-job, lifecycle, or artifact checks.
- **Closure predicate**: A score seals only when the actual terminal product, executable digest, and model match the selected identity; each single-field substitution and omission rejects.
- **Red-before**: `cargo test -p opi-eval --locked --test native_driver remediation_003_substituted_terminal_identity_rejects -- --exact --nocapture --test-threads=1` — FAIL: independently substituted product, executable digest, and model were all admitted with exit 0. The corrected matrix completed all three cases before failing.
- **Green-after**: same discriminating check must PASS; Run separate product/build/model negatives for all three families plus matching positive cases and diagnostic oracle isolation. Assert invalid execution/no score, not merely a nonzero exit. The existing substitution suite retains its wrong-task/stale-job/artifact coverage. Observed: `not-run`.

### Batch B4: Use native client terminal causes in score eligibility

**Closure predicate**: Terminal authentication/rate-limit/protocol failures never enter valid Agent score denominators; valid zero, task failure, and recovered retry success retain their intended eligibility.
**Dependencies**: B3; keep one final settlement classifier
**Verification union**: `cargo test -p opi-eval --locked --test native_driver remediation_004_native_provider_error_is_not_scored -- --exact --nocapture --test-threads=1`; `cargo test -p opi-eval --locked --test native_driver native_provider`; `cargo test -p opi-eval --locked --test agent_integration_conformance`; `cargo test -p opi-eval --locked --test report_contract`; affected-target clippy and the final union below.

#### Fix B4.1: Use native client terminal causes in score eligibility

- **Finding source(s)**: `phase19-codex-gpt6-c18efa8-20260917t133157z` + `67c0ab6ca8b90178e3caf2d7cdd6cb4a6c9a6b2fe342c85d8cbd3ef105c95b36` + `P19-AUD-004`.
- **Decision**: `fix:native-result.provider-failure-exclusion`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/src/benchmark/process.rs`, `crates/opi-eval/src/agent/opi.rs`, `crates/opi-eval/src/agent/pi.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/agent_integration_conformance.rs`, `crates/opi-eval/tests/report_contract.rs`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Connect the existing Opi/pi terminal interpretation to the captured native Agent evidence before sealing settlement. Reuse the current native client parsers and typed causes instead of adding a second stderr-keyword policy. A generic runner nonzero tag cannot override an Opi authentication exit or a pi terminal JSON error (including exit zero). Retain valid native zero/task-timeout outcomes and retry-then-success eligibility. Preserve evidence sufficient to explain the classification at every capture policy.
- **Closure predicate**: Terminal authentication/rate-limit/protocol failures never enter valid Agent score denominators; valid zero, task failure, and recovered retry success retain their intended eligibility.
- **Red-before**: `cargo test -p opi-eval --locked --test native_driver remediation_004_native_provider_error_is_not_scored -- --exact --nocapture --test-threads=1` — FAIL: captured authentication error plus NonZeroAgentExitCodeError produced valid=2, agent_failure=2, invalid=0.
- **Green-after**: same discriminating check must PASS; Add actual Opi exit 3/4 diagnostics and the checked-in pi stream-auth-error/rate-limit/protocol fixtures to the native run-to-report seam, including exit-zero pi error and successful retry. Assert attempted/invalid/valid/Agent-failure counts and sealed cause for both clients and all families. Observed: `not-run`.

### Batch B5: Apply accepted Agent and verifier deadlines at their owning stages

**Closure predicate**: Each accepted deadline bounds its actual native stage; no accepted one-second stage runs for two seconds and then produces an unrestricted successful result.
**Dependencies**: B3, B4 for implementation and the initial offline gate; B9 for the deferred real-stage closure gate under D5. Run `remediation_native_stage_deadlines` immediately after B9 and before B10, then again only when later changes invalidate its evidence.
**Verification union**: `cargo test -p opi-eval --locked --test native_driver remediation_005_native_stage_deadlines_reach_runner -- --exact --nocapture --test-threads=1`; `cargo test -p opi-eval --locked --test native_driver native_job_and_verifier_timeouts`; `cargo test -p opi-eval --locked --test native_driver native_signal_cancellation`; `cargo test -p opi-eval --locked --test benchmark_integration_conformance`; affected-target clippy and the final union below.

#### Fix B5.1: Apply accepted Agent and verifier deadlines at their owning stages

- **Finding source(s)**: `phase19-codex-gpt6-c18efa8-20260917t133157z` + `67c0ab6ca8b90178e3caf2d7cdd6cb4a6c9a6b2fe342c85d8cbd3ef105c95b36` + `P19-AUD-005`.
- **Decision**: `fix:native-execution.declared-deadlines`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/src/runner/plan.rs`, `crates/opi-eval/src/benchmark/process.rs`, `crates/opi-eval/src/benchmark/terminal_bench_21.rs`, `crates/opi-eval/src/benchmark/terminal_bench_30.rs`, `crates/opi-eval/src/benchmark/deepswe.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/benchmark_integration_conformance.rs`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Thread accepted execution limits through the existing request/adapter path. The Agent cap is the minimum of supplied positive time_limit_seconds and agent_timeout_seconds; the verifier cap is verifier_timeout_seconds. Emit one complete native JobConfig JSON per selected trial and launch pinned uv run --locked harbor|pier run --config <absolute-json>. The single agents entry carries max_timeout_sec, the verifier entry carries its independent max_timeout_sec, and timeout multipliers stay 1. Preserve stricter task/step limits. If the task Agent timeout is unbounded, also set override_timeout_sec to the accepted cap: Harbor returns None before consulting max_timeout_sec when no base timeout exists. Include the selected integration, exact model, task, one attempt, zero retries and one-trial concurrency in the same config. Do not append --agent/--agent-import-path, because both pinned CLIs replace the configured agents list and would erase the caps. Persist/read back effective trial configuration. Keep the profile whole-job supervisor separate, retain correct timeout classification and owned-resource cleanup, and reject a stage control that cannot be enforced. This is private dispatch through existing upstream schemas; no new user-facing execution control is introduced.
- **Closure predicate**: Each accepted deadline bounds its actual native stage; no accepted one-second stage runs for two seconds and then produces an unrestricted successful result.
- **Red-before**: `cargo test -p opi-eval --locked --test native_driver remediation_005_native_stage_deadlines_reach_runner -- --exact --nocapture --test-threads=1` — FAIL: individually configured time_limit_seconds, agent_timeout_seconds and verifier_timeout_seconds were absent from both actual runner spawns (six missing cap observations).
- **Green-after**: same discriminating check must PASS; The actual-spawn configuration probe must pass. After B9 and before B10 (D5), add and run independent real Agent-stage and verifier-stage delays through the pinned runners, each bound tested alone and combined, with positive within-bound controls, an unbounded task default, and a stricter task limit. Assert bounded termination, no owned orphan, no silent scope relaxation, and distinct Agent-task-timeout versus invalid verifier timeout settlement. Merely shortening the whole-job timeout does not close all three fields. Observed: `not-run`.

### Batch B6: Reject reused run roots before replacing control evidence

**Closure predicate**: A rejected same-root rerun leaves every pre-existing evidence byte unchanged and its prior verify/report result reproducible.
**Dependencies**: none for the root-admission change; run final integration after B3-B5
**Verification union**: `cargo test -p opi-eval --locked --test native_driver remediation_009_rejected_rerun_preserves_control -- --exact --nocapture --test-threads=1`; `cargo test -p opi-eval --locked --test native_driver`; `cargo test -p opi-eval --locked --test assembled_run`; `cargo test -p opi-eval --locked --test bundle_recompute`; affected-target clippy and the final union below.

#### Fix B6.1: Reject reused run roots before replacing control evidence

- **Finding source(s)**: `phase19-codex-gpt6-c18efa8-20260917t133157z` + `67c0ab6ca8b90178e3caf2d7cdd6cb4a6c9a6b2fe342c85d8cbd3ef105c95b36` + `P19-AUD-009`.
- **Decision**: `fix:run-root.rejected-reuse-preserves-evidence`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/assembled_run.rs`, `crates/opi-eval/tests/bundle_recompute.rs`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Admit the run-level control record without overwriting an existing run. Reserve a fresh control artifact with create-new semantics before publishing trial effects; reject incompatible/reused execution roots while retaining recovery and explicitly fresh replacement identities. A rejected request must not rewrite run-control, run-report, intents, bundles, or prior reports. Keep partial/crashed roots observable rather than deleting them.
- **Closure predicate**: A rejected same-root rerun leaves every pre-existing evidence byte unchanged and its prior verify/report result reproducible.
- **Red-before**: `cargo test -p opi-eval --locked --test native_driver remediation_009_rejected_rerun_preserves_control -- --exact --nocapture --test-threads=1` — FAIL: rejected rerun changed run-control SHA-256 from 951e197c169c07abf40fb10eb377982448128817175414e3b68c3287e8f07b62 to ec43d4045f549790541890e0910d8383428f27f459761adea6c09c5f4ede3680.
- **Green-after**: same discriminating check must PASS; Extend the probe to hash the complete evidence tree and compare offline verify/report before and after changed and identical rerun requests; exercise partial/crashed root, fresh replacement root, recovery, and a simultaneous claim of one fresh root. Retain planned coverage after incomplete execution. Observed: `not-run`.

### Batch B7: Keep unobserved native Agent duration unknown

**Closure predicate**: An unobserved native Agent duration seals as Unknown with a reason, while an actually measured duration (including genuine zero) keeps its measurement origin.
**Dependencies**: B3, B4; reuse existing Known/Unknown trajectory vocabulary
**Verification union**: `cargo test -p opi-eval --locked --test native_driver remediation_012_native_unobserved_time_is_unknown -- --exact --nocapture --test-threads=1`; `cargo test -p opi-eval --locked --lib trajectory::tests`; `cargo test -p opi-eval --locked --test native_driver`; `cargo test -p opi-eval --locked --test report_contract`; affected-target clippy and the final union below.

#### Fix B7.1: Keep unobserved native Agent duration unknown

- **Finding source(s)**: `phase19-codex-gpt6-c18efa8-20260917t133157z` + `67c0ab6ca8b90178e3caf2d7cdd6cb4a6c9a6b2fe342c85d8cbd3ef105c95b36` + `P19-AUD-012`.
- **Decision**: `fix:trajectory.native-agent-duration-honesty`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/agent/process.rs`, `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/src/trajectory/mod.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/report_contract.rs`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Represent availability of Agent wall time in the private execution record/projection instead of constructing Duration::from_secs(0) for native jobs. Propagate an explicit unknown reason for unavailable native Agent timing. Retain Known supervisor timing for the actual host-supervised Agent path and keep upstream whole-job/grader timing separate. Do not manufacture an Agent duration from job elapsed time or silently omit the fact.
- **Closure predicate**: An unobserved native Agent duration seals as Unknown with a reason, while an actually measured duration (including genuine zero) keeps its measurement origin.
- **Red-before**: `cargo test -p opi-eval --locked --test native_driver remediation_012_native_unobserved_time_is_unknown -- --exact --nocapture --test-threads=1` — FAIL: the sealed Agent execution node contained {"Known":{"origin":"supervisor-wall-clock","value":0}} without an observed native Agent duration.
- **Green-after**: same discriminating check must PASS; The corrected nested Known/Unknown probe passes for both subjects. Add a positive measured host-Agent case and check report diagnostics preserve unknown reasons. Require existing trajectory and bundle verification to pass unchanged; no on-disk enum migration is intended. Observed: `not-run`.

### Batch B8: Admit complete effective task requirements

**Closure predicate**: Prepare and plan retain effective Agent/verifier requirements with origins and reject unmet or required-but-unobserved resources before jobs or models start.
**Dependencies**: B5 initial offline gate under D5; final contract checks run after B9
**Verification union**: `cargo test -p opi-eval --locked --test native_driver remediation_007_prepare_rejects_impossible_storage -- --exact --nocapture --test-threads=1`; `cargo test -p opi-eval --locked --test native_driver`; `cargo test -p opi-eval --locked --lib runner::`; `cargo test -p opi-eval --locked --test benchmark_integration_conformance`; the final union below.

#### Fix B8.1: Admit complete effective task requirements

- **Finding source(s)**: `phase19-codex-gpt6-c18efa8-20260917t133157z` + `67c0ab6ca8b90178e3caf2d7cdd6cb4a6c9a6b2fe342c85d8cbd3ef105c95b36` + `P19-AUD-007`.
- **Decision**: `fix:preflight.complete-resource-admission`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/runner/prepare.rs`, `crates/opi-eval/src/runner/plan.rs`, `crates/opi-eval/src/benchmark/task_lifecycle.rs`, `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/benchmark_integration_conformance.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Implement D1 in existing requirement/lifecycle parsing and observation code. Resolve pinned Harbor/Pier defaults once with checked units and separate Agent/verifier CPU, memory, storage, GPU count/types, network, lifecycle/collect and stage limits. Reuse that value in prepare, full-corpus preview, freeze and dispatch. Record each declared/default origin and observation status/reason/source. Compare CPU/memory to the selected Docker engine and disk to actual cache/output and engine storage locations. Runtime registration alone leaves GPU capacity Unknown; require concrete engine-visible count/type observations. Unavailable required facts reject selected work with visible reasons. Do not start task images for preview, invent VRAM requirements, weaken network controls or silently shrink selection. B5 owns stage-deadline enforcement; B8 owns lossless requirements and admission. Existing doctor/prepare schemas receive additive observations; changed frozen structures use B9's run-plan/2 and plan-preview/2.
- **Closure predicate**: Prepare and plan retain effective Agent/verifier requirements with origins and reject unmet or required-but-unobserved resources before jobs or models start.
- **Red-before**: `cargo test -p opi-eval --locked --test native_driver remediation_007_prepare_rejects_impossible_storage -- --exact --nocapture --test-threads=1` — FAIL: prepare exited 0 and reported task status ok for storage_mb=9000000000000; the resource record omitted storage.
- **Green-after**: same discriminating check plus expanded matrix must PASS. PASS for impossible storage with a storage unmet/unknown reason and a valid satisfiable control. Exercise independent Agent/verifier CPU, memory, storage, GPU count/type, network, lifecycle and deadline cases. An nvidia runtime without observed devices cannot satisfy one requested GPU. Cover remote-engine unknown storage, host/engine differences, Harbor absent values versus Pier defaults, negative/overflowed units, stricter task limits, subsets and full inventory. Assert no job/model launch during preview or rejected freeze. Migrate fixtures to material/5 first so schema rejection cannot falsely satisfy a resource negative. Observed: `not-run`.

### Batch B9: Freeze and dispatch multiple byte-bound native tasks

**Closure predicate**: A feasible multi-task selection freezes each selected task's own canonical identity, verified profile/package and immutable images, dispatches exactly those tasks, and preserves the full pinned corpus inventory and rejection reasons.
**Dependencies**: B2, B3, B5 initial offline gate, B8; immediately after B9, pass B8/B9 final contract checks and B5 real-stage closure before B10 consumes the per-task map
**Verification union**: `cargo test -p opi-eval --locked --test native_driver remediation_008_feasible_second_task_can_freeze -- --exact --nocapture --test-threads=1`; `cargo test -p opi-eval --locked --test native_driver`; `cargo test -p opi-eval --locked --lib runner::`; `cargo test -p opi-eval --locked --test benchmark_integration_conformance`; `python crates/opi-eval/scripts/test_verify_native_smoke_artifact.py`; `python crates/opi-eval/scripts/test_native_smoke_output_control.py`; `python crates/opi-eval/scripts/test_verify_native_smoke_ci.py`; the final union below.

#### Fix B9.1: Freeze and dispatch multiple byte-bound native tasks

- **Finding source(s)**: `phase19-codex-gpt6-c18efa8-20260917t133157z` + `67c0ab6ca8b90178e3caf2d7cdd6cb4a6c9a6b2fe342c85d8cbd3ef105c95b36` + `P19-AUD-008`.
- **Decision**: `fix:selection.multitask-native-execution`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/runner/material.rs`, `crates/opi-eval/src/runner/prepare.rs`, `crates/opi-eval/src/runner/plan.rs`, `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/src/cli/run.rs`, `crates/opi-eval/src/regrade.rs`, `crates/opi-eval/src/benchmark/terminal_bench_21.rs`, `crates/opi-eval/src/benchmark/terminal_bench_30.rs`, `crates/opi-eval/src/benchmark/deepswe.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/benchmark_integration_conformance.rs`, `crates/opi-eval/scripts/native-smoke.sh`, `crates/opi-eval/scripts/verify-native-smoke-artifact.py`, `crates/opi-eval/scripts/test_verify_native_smoke_artifact.py`, `crates/opi-eval/scripts/test_native_smoke_output_control.py`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Implement accepted material/5, run-plan/2 and plan-preview/2. Benchmark corpus root/commit/tasks-tree provides enumeration independently of the selected tasks map. Verify the complete corpus against the static-lock subject commit/tree; derive each selected native name, profile byte table and package digest through existing family-specific admission. Keep source-tree identity distinct from recorded image materialization; reject mutable or unbound task/verifier images. Cache profiles per adapter/canonical-task and bind their SHA-256. Tasks cannot borrow another task's name, profile, tree or image. Seed the complete pinned corpus through the existing external fetch/cache contract; unavailable corpus/package/image rejects with actionable identity. No new acquisition transport or scheduler. Update prepare, plan, both run entry paths, regrade task lookup, native-smoke producer and artifact consumer together. Preserve old sealed bundles; material/4 and plan/1 require regeneration without implicit conversion. Smoke keeps its explicit task selection while exposing the full corpus. Preserve pi-only and independent-directory operation.
- **Closure predicate**: A feasible multi-task selection freezes each selected task's own canonical identity, verified profile/package and immutable images, dispatches exactly those tasks, and preserves the full pinned corpus inventory and rejection reasons.
- **Red-before**: `cargo test -p opi-eval --locked --test native_driver remediation_008_feasible_second_task_can_freeze -- --exact --nocapture --test-threads=1` — FAIL: the two-task subset was rejected because alpha-notes was not bound by the single pinned profile (per-task native profiles are not materialized yet).
- **Green-after**: same discriminating check plus expanded matrix must PASS. PASS with two distinct feasible tasks per family in temporary pinned corpora: preview shows all entries, subset/full counts are exact, and dispatch logs plus sealed output prove per-task bindings. Test canonical/native names, unknown/duplicate IDs, swapped profile/package, source-tree drift, mutable/missing images, omitted selected material, corpus shrinkage, old schemas, unfeasible selection, pi-only and independent-directory execution. Retain one-task controls and oracle isolation. Synthetic corpora do not prove real grading; native checks require actual immutable images. Observed: `not-run`.

### Batch B10: Run native verification from preserved replay evidence

**Closure predicate**: Sufficient supported separate-task replay evidence launches the pinned native verifier in a fresh environment and yields a newly bound grade without Agent/provider/collect invocation or source mutation; insufficient state explicitly rejects without a score.
**Dependencies**: B1, B3, B5 fully closed by its post-B9 real-stage witness, B6, B9 with B8/B9 final checks green; original-run and verifier-only result admission stay distinct
**Verification union**: `cargo test -p opi-eval --locked --test native_driver remediation_002_regrade_dispatches_a_runner_command -- --exact --nocapture --test-threads=1`; `cargo test -p opi-eval --locked --test native_driver`; `cargo test -p opi-eval --locked --test benchmark_integration_conformance`; `python crates/opi-eval/scripts/test_native_regrade.py`; `python crates/opi-eval/scripts/test_native_integrations.py`; `python crates/opi-eval/scripts/test_verify_native_smoke_artifact.py`; `cargo test -p opi-eval --locked --test native_driver remediation_native_regrade_witnesses -- --exact --ignored --nocapture --test-threads=1`; the final union below.

#### Fix B10.1: Run native verification from preserved replay evidence

- **Finding source(s)**: `phase19-codex-gpt6-c18efa8-20260917t133157z` + `67c0ab6ca8b90178e3caf2d7cdd6cb4a6c9a6b2fe342c85d8cbd3ef105c95b36` + `P19-AUD-002`.
- **Decision**: `fix:regrade.real-native-grader-dispatch`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/regrade.rs`, `crates/opi-eval/src/benchmark/process.rs`, `crates/opi-eval/src/runner/material.rs`, `crates/opi-eval/src/runner/prepare.rs`, `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/integrations/harbor/opi_eval_regrade.py`, `crates/opi-eval/integrations/pier/opi_eval_regrade.py`, `crates/opi-eval/scripts/native-smoke.sh`, `crates/opi-eval/scripts/verify-native-smoke-artifact.py`, `crates/opi-eval/scripts/test_verify_native_smoke_artifact.py`, `crates/opi-eval/scripts/test_native_regrade.py`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/benchmark_integration_conformance.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Implement D2 with a private module in each existing integration directory. In material/5 pin benchmark verifier_integration as module_dir, module_manifest_sha256 and entrypoint, independently of the source Agent executable. Through the pinned uv project invoke uv run --locked python <verified-module-path> --request <absolute-request.json>; bind source bundle/config/result/artifact manifest, exact task/profile/package, output root and limits. Harbor uses pinned RegradeTrial for supported single-step separate replay. Pier uses its environment factory, artifact handler and Verifier with agent_install_spec=None, matching native preparation/upload/network/user behavior and finally cleanup; no normal Trial, Agent setup/run, collection or provider calls. Use the existing supervisor and B1 closed environment. Preserve artifact modes, links and native metadata, not just bytes. Existing sealed config/result/artifact-manifest records are replay authority; missing metadata rejects without retrofitting seals. Restore at a fresh staging path after identity validation. Import fresh output against verifier-only selection and source linkage; do not fabricate Agent observations or reuse old reward. Replace the test-only OPI_EVAL_REGRADE protocol. Changed outputs use opi-eval-regrade-record/2 and opi-eval-native-regrade/2, separating source Agent provenance from fresh verifier identity/result. Shared TB2.1 lacking complete state rejects as missing-replay-state under section 6; no snapshot engine or lifecycle conversion. Document the boundary in both READMEs and Unreleased.
- **Closure predicate**: Sufficient supported separate-task replay evidence launches the pinned native verifier in a fresh environment and yields a newly bound grade without Agent/provider/collect invocation or source mutation; insufficient state explicitly rejects without a score.
- **Red-before**: `cargo test -p opi-eval --locked --test native_driver remediation_002_regrade_dispatches_a_runner_command -- --exact --nocapture --test-threads=1` — FAIL: regrade dispatched the pinned entrypoint with argc=0; the test-only interpreter manufactured the grading result.
- **Green-after**: same discriminating check plus expanded matrix must PASS. PASS for explicit dispatch plus actual no-model native witnesses. Move the argc canary to a valid separate TB3 fixture and assert dispatch, not early shared-state rejection. Run TB3/Pier positive/negative saved-artifact cases through unchanged pinned graders in fresh environments with zero Agent/provider replay calls, distinct outputs and unchanged source hashes. Cover missing artifacts/metadata/cache, module/profile drift, shared state, unsupported lifecycle, network/credential rejection, failure, timeout/cancel cleanup, stale output and changed-artifact/old-reward substitution. Python tests exercise actual adapter functions; native tests prove exact-version API compatibility. Simulator or argv count alone cannot close this finding. Observed: `not-run`.

### Batch B11: Separate native task success from test fractions

**Closure predicate**: The report binds proven task/grader meanings and denominators, retains integral reward, separates subject/benchmark task success from per-trial test fractions, and never turns unknown meaning or empty denominators into measured success.
**Dependencies**: B3, B4, B7, B9; validate regraded output consistently after B10
**Verification union**: `cargo test -p opi-eval --locked --test native_driver remediation_011_reports_identify_native_metric_meaning -- --exact --nocapture --test-threads=1`; `cargo test -p opi-eval --locked --test native_driver`; `cargo test -p opi-eval --locked --test report_contract`; `cargo test -p opi-eval --locked --test end_to_end_report`; `cargo test -p opi-eval --locked --test benchmark_integration_conformance`; `cargo test -p opi-eval --locked --lib report::`; the final union below.

#### Fix B11.1: Separate native task success from test fractions

- **Finding source(s)**: `phase19-codex-gpt6-c18efa8-20260917t133157z` + `67c0ab6ca8b90178e3caf2d7cdd6cb4a6c9a6b2fe342c85d8cbd3ef105c95b36` + `P19-AUD-011`.
- **Decision**: `fix:report.native-metric-semantics`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/report.rs`, `crates/opi-eval/src/benchmark/process.rs`, `crates/opi-eval/src/benchmark/terminal_bench_21.rs`, `crates/opi-eval/src/benchmark/terminal_bench_30.rs`, `crates/opi-eval/src/benchmark/deepswe.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/report_contract.rs`, `crates/opi-eval/tests/end_to_end_report.rs`, `crates/opi-eval/tests/benchmark_integration_conformance.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Implement normalized-report/2 and reporter/2. Retain typed binary reward and sealed raw metrics. Bind meanings to exact task/grader identity, not a family name or reward==1 heuristic. Terminal-Bench fractions use validated CTRF passed/tests, separate from binary success. DeepSWE uses its pinned success rule and preserves raw f2p/p2p counts, fractions and partial. Report per-subject/per-benchmark success numerator and valid-graded denominator, plus per-trial named integer ratios and native artifact references. Preserve coverage/B4 exclusions, with no cross-family score or implicit micro/macro averaging. Missing/zero denominators and unknown meanings remain explicit. Unknown-meaning trials retain scalar and coverage; incomplete included task meanings make the labelled success aggregate Unknown, rather than silently reducing its denominator. Reconstruct from sealed artifacts without mutation or floats in integer Fact. Update contract tests, bilingual docs and Unreleased.
- **Closure predicate**: The report binds proven task/grader meanings and denominators, retains integral reward, separates subject/benchmark task success from per-trial test fractions, and never turns unknown meaning or empty denominators into measured success.
- **Red-before**: `cargo test -p opi-eval --locked --test native_driver remediation_011_reports_identify_native_metric_meaning -- --exact --nocapture --test-threads=1` — FAIL: TB2.1, TB3, and DeepSWE all emitted the identical generic metric definition. This probe proves missing differentiation; it does not establish upstream numeric reward semantics.
- **Green-after**: same discriminating check plus expanded matrix must PASS. PASS for the missing-meaning probe and public report cases: DeepSWE partial=1/reward=0, empty f2p, both sets empty, partial success, valid zero/task-timeout, provider exclusion, two subjects, different task identities, full/subset/incomplete coverage, unknown meaning, zero/missing counters and inconsistent input rejection. Assert numeric counts and artifact/definition identity rather than prose wording. Verify deterministic reconstruction, unchanged seals, retained raw values and no cross-family score. Existing source-aggregation probes guide expectations but are not native acceptance. Observed: `not-run`.

## Final Verification

### Planning checks actually executed

| Command / state | Observed result |
|---|---|
| `python .agents/skills/_shared/scripts/validate_assurance_artifact.py audit-set docs/snapshots/phase19/assurance` | PASS before semantic reads. |
| `cargo test -p opi-eval --locked --test native_driver native_run_drives_paired_trials_through_one_selected_job_each -- --exact` in the pristine export | PASS: 1 passed, 51 filtered. Establishes the positive fixture baseline. |
| `cargo test -p opi-eval --locked --test native_driver remediation_ -- --nocapture --test-threads=1` in the export with probes | Exit 101: 10 failed, 1 passed, 52 filtered. The early telemetry probe accessed the wrong JSON shape; its apparent pass is excluded as evidence. The original C/005 total-job sleep observation is superseded by the stage-config probe below and is not a stage-enforcement proof. |
| `cargo test -p opi-eval --locked --test native_driver remediation_003 -- --nocapture --test-threads=1` after making its matrix run all three substitutions | Exit 101: 1 failed, 62 filtered; product, executable and model all admitted. Other probe functions and production bytes were unchanged. |
| `cargo test -p opi-eval --locked --test native_driver remediation_012 -- --nocapture --test-threads=1` with the corrected Fact assertion | Exit 101: 1 failed, 62 filtered; Known zero with supervisor-wall-clock observed. This is the telemetry red-before. |
| `python3 /tmp/opi-remediate-phase19.EzEQLM6C/scripts/opi-doc-check.py` | PASS: committed-export documentation contracts. |

| `cargo test -p opi-eval --locked --test native_driver remediation_005_native_stage_deadlines_reach_runner -- --exact --nocapture --test-threads=1` after refining the stage oracle | Exit 101: 1 failed, 62 filtered; all three independently requested one-second caps absent from both actual runner configurations. |
| Repeated `python .agents/skills/_shared/scripts/validate_assurance_artifact.py audit-set docs/snapshots/phase19/assurance` during continuation | Exit 1: incomplete remediation file group, because a plan-only pair exists and no apply/result has run. No source/member/index defect was reported. |
| `python .agents/skills/_shared/scripts/validate_assurance_artifact.py plan docs/snapshots/phase19/assurance/remediation.plan.md` at continuation admission | PASS; this validator revalidates all indexed members through its explicit allow_partial_remediation=True context. The prior draft pair is owned by this same ongoing task; HEAD and index bytes are unchanged. |

Together the corrected observations establish 11 independent defect observations, not a passing
regression suite and not a claim that one final all-probe command ran. The
appendix contains the final probe source. No real model, provider credential,
native grader, or Docker workload was invoked by these Rust probes. The separate
metric-source check below executes only the pinned Python aggregation entry with
synthetic test reports, not a native environment or acceptance trial. The resource
probes queried an explicitly closed Docker endpoint; execution probes used the
existing private Docker stand-in. Two discarded shell-launch attempts produced
no source/test evidence; the successful export and commands above are the
admitted evidence.

Final local plan publication must pass the dispositions validator, the fixed
plan validator, `python scripts/opi-doc-check.py`, and `git diff --check`.
Their execution and emitted digest belong in the handoff, outside this plan's
own digest. No final-state runtime green or workspace gate is claimed here.

### Apply verification union (final remaining state requires the new digest)

Use WSL/Linux, the existing external Cargo cache, deterministic fixtures, and no
paid-provider or credential-triggered tests. Apply B1 through B11 in dependency
order with D5's explicit deferred B5 closure checkpoint. Keep each batch's
red/green proof even where files overlap. B5's initial configuration/offline
gate permits B6-B9 implementation without declaring B5 Closed. After both B8/B9
changes, run their final contract checks and B5's real-stage witness; all must
pass before B10. Preserve and recheck the carried-in B1-B4 state as needed. Run each changed binary/filter and
the exact native witness names; ignored tests do not count as observed green.

```sh
cargo test -p opi-eval --locked --test native_driver --test assembled_run --test fail_closed_cli --test benchmark_integration_conformance --test agent_integration_conformance --test report_contract --test end_to_end_report --test bundle_recompute
cargo test -p opi-eval --locked --lib
cargo clippy -p opi-eval --all-targets --locked -- -D warnings
cargo fmt --check --all
python crates/opi-eval/scripts/test_native_regrade.py
python crates/opi-eval/scripts/test_native_integrations.py
python crates/opi-eval/scripts/test_verify_native_smoke_artifact.py
python crates/opi-eval/scripts/test_native_smoke_output_control.py
python crates/opi-eval/scripts/test_verify_native_smoke_ci.py
python scripts/opi-doc-check.py
git diff --check
```

Two new explicitly ignored native-driver tests are required behavioral closure
checks, distinct from the excluded six-trial live acceptance. The stage-deadline
witness runs after B9 and before B10; the regrade witness runs after B10
implementation. Both are required in the final verified state:

```sh
cargo test -p opi-eval --locked --test native_driver remediation_native_stage_deadlines -- --exact --ignored --nocapture --test-threads=1
cargo test -p opi-eval --locked --test native_driver remediation_native_regrade_witnesses -- --exact --ignored --nocapture --test-threads=1
```

They use `OPI_EVAL_NATIVE_WITNESS_MATERIAL` pointing to verified material/5
and isolated temporary outputs. The deadline test uses deterministic Agent and
verifier delays through pinned runners without an LLM. The regrade test creates
source records using deterministic no-model execution, then proves replay
invokes neither Agent nor provider. Actual TB3/Pier positive and negative
graders must run; shared TB2.1 missing-state rejection is a separate case.
Missing engine/image/cache/replay material means the relevant green is not
demonstrated: record Not closed, never skip-to-PASS or substitute a simulator.
Existing material-gated native witnesses remain retained, with no claim they
ran during planning. Python tests exercise actual private adapters; native
tests establish exact-version API compatibility and lifecycle.

Step -> verify: behavioral batch -> matching regression and expanded predicate
matrix; material producer/consumer migration -> exact script tests and schema/
identity negatives; report migration -> public reconstruction and numeric
semantics; final state -> the union above. Re-run checks affected by subsequent
changes or unresolved failures. Original planning test impact was `none` in production, with temporary exported
probes only. Carried-in B1-B4 test impact is recorded above. Remaining apply: `update` listed Rust/script tests;
`add` two native-driver cases and `scripts/test_native_regrade.py`; retain
existing discriminating coverage.

### Exact proposed production path union

The three new files are the two private `opi_eval_regrade.py` modules and their
Python test. Other fixtures are generated in isolated temporary test directories
or updated in the listed existing test sources. Shared paths remain attributed
per batch in JSONL. This amendment changes only the fixed plan pair; the
explicit continuation baseline retains same-task B1-B4 production/test edits.

- `CHANGELOG.md`
- `crates/opi-eval/README.md`
- `crates/opi-eval/README.zh.md`
- `crates/opi-eval/integrations/harbor/opi_eval_regrade.py`
- `crates/opi-eval/integrations/pier/opi_eval_regrade.py`
- `crates/opi-eval/scripts/native-smoke.sh`
- `crates/opi-eval/scripts/test_native_regrade.py`
- `crates/opi-eval/scripts/test_native_smoke_output_control.py`
- `crates/opi-eval/scripts/test_verify_native_smoke_artifact.py`
- `crates/opi-eval/scripts/verify-native-smoke-artifact.py`
- `crates/opi-eval/src/agent/opi.rs`
- `crates/opi-eval/src/agent/pi.rs`
- `crates/opi-eval/src/agent/process.rs`
- `crates/opi-eval/src/benchmark/deepswe.rs`
- `crates/opi-eval/src/benchmark/process.rs`
- `crates/opi-eval/src/benchmark/task_lifecycle.rs`
- `crates/opi-eval/src/benchmark/terminal_bench_21.rs`
- `crates/opi-eval/src/benchmark/terminal_bench_30.rs`
- `crates/opi-eval/src/cli/run.rs`
- `crates/opi-eval/src/main.rs`
- `crates/opi-eval/src/regrade.rs`
- `crates/opi-eval/src/report.rs`
- `crates/opi-eval/src/runner/experiment.rs`
- `crates/opi-eval/src/runner/material.rs`
- `crates/opi-eval/src/runner/plan.rs`
- `crates/opi-eval/src/runner/prepare.rs`
- `crates/opi-eval/src/trajectory/mod.rs`
- `crates/opi-eval/tests/agent_integration_conformance.rs`
- `crates/opi-eval/tests/assembled_run.rs`
- `crates/opi-eval/tests/benchmark_integration_conformance.rs`
- `crates/opi-eval/tests/bundle_recompute.rs`
- `crates/opi-eval/tests/end_to_end_report.rs`
- `crates/opi-eval/tests/fail_closed_cli.rs`
- `crates/opi-eval/tests/native_driver.rs`
- `crates/opi-eval/tests/report_contract.rs`

No Cargo, dependency, Agent Core, static external lock, registered specification,
implementation ledger, acquisition transport or workflow changes are planned.
The native-smoke producer is not in static-lock authority.producers; changed
producer bytes require fresh receipts, not edits to that lock or old receipts.
Existing sealed evidence is never rewritten to match new contracts. A further
durable contract or scope change returns to planning under the skill.

The registered Phase acceptance union remains a separate obligation under the resolved D4 exclusion:
full workspace fmt/clippy/all-target tests/doc tests/rustdoc plus the supplied
native acceptance and no-model witnesses. This plan does not equate one-crate
fix validation with Phase exit, does not waive those gates, and does not invoke
another workflow. Production changes invalidate affected real evidence; a
later evidence owner must identify what needs re-execution.

## Exclusions

| Exact source key | Disposition | Current evidence/authority |
|---|---|---|
| `phase19-codex-gpt6-c18efa8-20260917t133157z` / `P19-AUD-010` | Cannot confirm external completeness; explicit exclusion (verification Partially confirmed) | Stand-in/ignored witness limits confirmed; original archive existence and 30-criterion completion unproven in this permitted context. Retain Major and all reciprocal requirement links. |
| `phase19-pi-glm53-d61590b-20260917t152753z` / `P19-AUD-001` | Info-no-action (verification Partially confirmed) | Immutable commit path inventory confirms mixed provenance, with the exact opi-ai claim overstated. Preserve commits and ledger. |

## Accepted contract details and pinned evidence

The user asked to continue planning. This is not a digest-bearing apply
invocation. The same source head and active index remain bound. The two
untracked output files are the current task's own plan pair, not carried-in work.
No production edit or new normative source has been admitted.

### Locked upstream evidence

The following sources were fetched read-only through GitHub's Git-tree and
Git-blob APIs using `gh api`. Each tree was complete (`truncated=false`),
each fetched blob's Git object hash was recomputed, and the immutable commit
was taken from the committed external lock. No branch tip, newer release,
operator archive, or old evaluation conclusion was used. API/tree acquisition
and source inspection do not establish a runnable installed native environment.

| Source | Locked commit | Evidence relevant to these decisions |
|---|---|---|
| Harbor | `4407eb5227a2ff4f0d3f16b2eb48849382fdf276` | [Regrade admission and restoration](https://github.com/harbor-framework/harbor/blob/4407eb5227a2ff4f0d3f16b2eb48849382fdf276/src/harbor/trial/regrade.py): only single-step separate-verifier replay; it checks the original artifact manifest and skips Agent execution. [Job CLI](https://github.com/harbor-framework/harbor/blob/4407eb5227a2ff4f0d3f16b2eb48849382fdf276/src/harbor/cli/jobs.py) accepts a JobConfig but replaces its Agents when CLI Agent flags are present. |
| Harbor | same | [Trial config](https://github.com/harbor-framework/harbor/blob/4407eb5227a2ff4f0d3f16b2eb48849382fdf276/src/harbor/models/trial/config.py) has distinct Agent/verifier max and override timeouts. [Trial timeout resolution](https://github.com/harbor-framework/harbor/blob/4407eb5227a2ff4f0d3f16b2eb48849382fdf276/src/harbor/trial/trial.py#L1317) returns no Agent deadline when the base timeout is absent, before consulting max_timeout_sec. |
| Pier | `c1ebc6d145b40fae8425215e3fca528945065124` | [Trial config](https://github.com/datacurve-ai/pier/blob/c1ebc6d145b40fae8425215e3fca528945065124/src/pier/models/trial/config.py) has the same separate cap fields. The complete tree contains no dedicated regrade entry; [the separate-verifier path](https://github.com/datacurve-ai/pier/blob/c1ebc6d145b40fae8425215e3fca528945065124/src/pier/trial/trial.py#L387) uses the upstream environment factory, artifact handler, Verifier and finally cleanup. Reusing these APIs needs a concrete private integration and tests, not invented Pier CLI flags. |
| TB2.1 | `7131e4375048a0e408a8fb404b5f499d726b695b` | [Pinned task](https://github.com/harbor-framework/terminal-bench-2-1/blob/7131e4375048a0e408a8fb404b5f499d726b695b/tasks/openssl-selfsigned-cert/task.toml) has no declared artifacts and uses the shared verifier. [Grader entry](https://github.com/harbor-framework/terminal-bench-2-1/blob/7131e4375048a0e408a8fb404b5f499d726b695b/tasks/openssl-selfsigned-cert/tests/test.sh) makes pytest exit status a binary reward. Its [tests](https://github.com/harbor-framework/terminal-bench-2-1/blob/7131e4375048a0e408a8fb404b5f499d726b695b/tasks/openssl-selfsigned-cert/tests/test_outputs.py) inspect file modes and execute the submitted Python script, so copying only file bytes or guessing two output paths is not a complete state replay. |
| TB3 | `2b0442c3c583b710ca8da14c8e601b99f2f1f244` | [Pinned task](https://github.com/harbor-framework/terminal-bench/blob/2b0442c3c583b710ca8da14c8e601b99f2f1f244/tasks/batched-eval-parity/task.toml) declares `/app/evalbench/` and a separate verifier. Its [entry](https://github.com/harbor-framework/terminal-bench/blob/2b0442c3c583b710ca8da14c8e601b99f2f1f244/tasks/batched-eval-parity/tests/test.sh) also emits a binary reward from pytest; CTRF test counts are distinct evidence. |
| DeepSWE | `435ee89ec2f2e2289f33b0da4f992f0b7b7266b9` | The [grader](https://github.com/datacurve-ai/deep-swe/blob/435ee89ec2f2e2289f33b0da4f992f0b7b7266b9/tasks/abs-module-cache-flags/tests/grader.py) emits binary `reward`, raw f2p/p2p counts, separate fractions and `partial`. The [task](https://github.com/datacurve-ai/deep-swe/blob/435ee89ec2f2e2289f33b0da4f992f0b7b7266b9/tasks/abs-module-cache-flags/task.toml) supplies its committed-patch collection and separate no-network verifier contract. |

Resource parsing must follow the different pinned models rather than assume
identical defaults. Harbor's environment storage/GPU fields can be absent;
Pier's defaults include storage_mb=10240 and gpus=0. Pier rejects allowlist
network policy, while Harbor models it. Both expose GPU count and acceptable
GPU types; neither reviewed task model declares a GPU-VRAM field. Actual GPU
observations may include VRAM, but this repair must not invent a benchmark
requirement or claim an unsupported declaration is enforced. The source
`task.toml` and its digest remain authoritative for per-task and per-stage
requirements and unsupported declarations must stay explicit.

### D1 accepted choice: one versioned per-task material map

Select a deliberate 0.x change to `opi-eval-native-material/5`,
`opi-eval-run-plan/2` and `opi-eval-plan-preview/2`. Reuse the existing
byte-verified profile per task; do not create a public adapter SDK or general
resource scheduler. The shape below is the accepted contract, not a generated material file:

```text
benchmarks[adapter].corpus = { root, source_commit, tasks_tree }
benchmarks[adapter].tasks[canonical_task_id] = {
  native_task_name,
  profile: { path, sha256 },
  task_package,
  task_package_manifest_sha256
}
```

Existing runner/executable/environment/oracle fields remain benchmark-scoped.
The complete pinned corpus supplies the inventory and size; each selected
entry must bind its exact task tree/file table, canonical ID, native name and
immutable images before freezing. Map entries cannot be silently substituted
or omitted. Read per-task requirements from the pinned package instead of
adding a second editable declaration in the material manifest. Keep the full
inventory in the preview even when only an explicit feasible subset freezes.

Freeze separate Agent/verifier effective requirements, including network,
CPU, memory, storage, GPU count/types, lifecycle, collect and stage deadlines,
with their declared/default origins. Use checked native-unit conversion and
existing Known/Unknown evidence conventions. Observe the selected Linux engine
and the relevant storage locations: a host runtime name proves neither GPU
capacity nor container-visible resource availability. A selected unmet/unknown
requirement rejects; an explicit subset leaves every exclusion and reason
visible. Multiple independent task packages can then pass through the existing
profile admission and native execution without removing an identity guard.

This replaces one-task-per-family assumptions in material load, preparation,
plan freeze and dispatch together. The known producers/readers include
`src/runner/{material,prepare,plan,experiment}.rs`, `src/cli/run.rs`,
`scripts/native-smoke.sh`, `scripts/verify-native-smoke-artifact.py`, their
existing script/native-driver tests, and both Companion READMEs. Regrade also
selects material by the source trial's task under D2. B8/B9/B10 enumerate all
affected callers and fixtures, including an Unreleased breaking-contract entry. Existing sealed evidence is never
rewritten, and old preparation/plan files must be regenerated; no implicit
compatibility conversion is proposed.

Required apply verification: two distinct native tasks per family (with exact
profile/package bindings), full inventory preview without image startup,
selected disk/GPU/network/stage failures, unknown engine observations,
wrong-name/tree/profile negatives, independent-directory execution and
pi-only material. The existing C/007 and C/008 red observations remain separate
closure predicates. The alternative is separate per-task manifests plus a new
corpus index; that introduces more files and indirection without a demonstrated
consumer benefit, so the single map is preferred.

### D2 accepted choice: real replay where preserved evidence is sufficient

Select a private verifier-only integration over the pinned runner APIs and the
existing supervisor. Keep original grading, offline byte verification and
fresh regrading separate. Preserve lifecycle rather than translate a shared
task into a separate-verifier task just to fit a convenient command.

For a single-step separate Harbor task, reconstruct the verified native source
trial (config/result/artifacts/manifest) at a new staging path and use the
pinned RegradeTrial route. For the selected Pier separate task, implement the
corresponding small private path using its environment factory, artifact
handler and Verifier with `agent_install_spec=None`; enter no Agent setup/run,
collect hook or provider-client path. Do not instantiate an ordinary Trial and
allow its default Agent to run. The source committed patch is replayed, not
re-collected from an unrelated live checkout. Pin the integration module bytes
in the material inventory and select it explicitly through the pinned uv
project. Preserve exact artifact metadata/paths, task/config/base identity,
network, effective user and cleanup behavior. Bind the new native output to a
fresh derived record and to the source bundle; copied Agent metadata is source
provenance and must not pretend a new Agent execution occurred.

For shared TB2.1 records, absence of a complete restorable environment remains
an explicit per-trial missing-replay-state rejection. The pinned acceptance
task declares no artifacts; the existing saved byte observations do not prove
filesystem modes, dependencies or arbitrary script state. Do not manufacture
that proof, merely copy `/app`, or change task.toml to separate mode. This
accepted scope adds no generic container snapshot engine. It relies on the
registered section 6 rule that missing required artifacts/materials reject
regrading; it does not claim this currently recorded TB2.1 trial can regrade.

The user accepted this evidence-dependent rejection as the repair boundary.
Complete shared-environment capture/restore is outside this plan. The registered
missing-material rule remains authoritative; no unconditional all-family replay
guarantee or normative revision is inferred.

Required apply verification: actual fresh TB3/Pier native positive/negative
artifact witnesses with zero Agent/provider invocations; shared-state/missing
artifact/manifest/cache/network rejection; credential canaries at the real
verifier process; original evidence tree unchanged; changed artifacts cannot
borrow an old reward. A stand-in that interprets OPI_EVAL_REGRADE or a nonzero
argv count alone cannot satisfy closure. The chosen boundary and all remaining
shared-state limitations must be visible in both READMEs and the disposition.

### D3 accepted choice: preserve binary success and expose test fractions

Select `opi-eval-normalized-report/2` and `opi-eval-reporter/2` for the deliberate
report contract change. Keep native binary reward in its existing typed value;
there is no evidence requiring a floating-point replacement for that field.
Expose the additional metrics as distinct records with their native source
and denominator, retaining all raw native reward fields.

- For the pinned TB2.1 and TB3 task scripts, task success is the native 0/1
  reward produced from the full pytest invocation. Test-case pass fraction is
  a separate per-trial ratio from that trial's validated CTRF counts.
- For the pinned DeepSWE grader, success requires a nonempty f2p set, every f2p
  passing and no p2p failure. Preserve f2p/p2p raw counts and the native f2p,
  p2p and partial values. A 1.0 partial fraction does not imply reward=1.
- Trial-success rate uses valid graded trials for that subject and benchmark.
  Keep valid zero and benchmark-defined task timeout; exclude infrastructure,
  provider, identity and invalid grading. Preserve attempted/planned/missing
  coverage, and never aggregate different benchmark families into one score.
- Report test fractions per trial with integer numerator/denominator and a
  native artifact reference; do not silently choose a micro- or macro-average
  across different tasks. Missing/zero normalized denominators stay Unknown
  with a reason. Preserve an upstream empty-set convention as a labelled raw
  native value, not a fabricated measured ratio.

Definitions must bind the selected task/grader identity, not assume every
newly selected task in a corpus uses the same reward script. Existing canonical
profiles provide the initial definitions. A task with unknown metric meaning
keeps the native scalar/raw output plus an explicit unknown meaning; it cannot
be relabelled task success. Native input inconsistencies are validation errors,
not reasons to derive a score from incomplete counters.

The four no-model source-aggregation probes below demonstrate the distinction:

| Synthetic report case | Native reward | Native partial |
|---|---:|---:|
| one f2p and one p2p both pass | 1 | 1.0 |
| f2p fails, p2p passes | 0 | 0.5 |
| no f2p, one passing p2p | 0 | 1.0 |
| both sets empty | 0 | 0.0 |

These execute the unchanged pinned `grader.py grade` with isolated synthetic
config/CTRF inputs and closed environment, without its prepare entry, Git
operations, a task container, an Agent or a model. All four assertions passed.
They verify interpretation only; they are not a real native handoff witness
and do not close C/010. B11 requires public run-to-report tests for
partial versus binary success, unknown counters, zero denominators, different
subjects, subset/full coverage, deterministic reconstruction and unchanged
source seals. The report module, report/native-driver fixtures and both READMEs change with
Unreleased. The artifact reader changes under B9/B10 for material admission;
its current code does not read normalized report metrics.

### Review and evidence boundary

D1-D3 are accepted for this READY-FOR-APPLY plan. Eleven batches, two
exclusions, source-bound red evidence, exact paths and required checks are fixed
above. Every green-after remains not-run. Planning agreement does not authorize
production apply or Phase conformance. The emitted digest replaces all draft
digests; a new explicit apply invocation must contain that exact digest.

The fixed plan is the sole remediation plan. Read-only upstream source and
new probe evidence remain outside the repository at
`C:/Users/Luiz/AppData/Local/Temp/opi-phase19-upstream-li7fcj3v`.
The following identities bind the supplemental inspection and its metric
check. Source URLs above permit retrieval without relying on the temporary
cache; the archive is not an evaluation acceptance artifact.

| Supplemental artifact | SHA-256 |
|---|---|
| `C:/Users/Luiz/AppData/Local/Temp/opi-phase19-upstream-li7fcj3v/sources.json` | `9202c006ef61cfba994b43ff0ac73ea9ea8c8b54f9c0ab4368f45a20f5dfee17` |
| `C:/Users/Luiz/AppData/Local/Temp/opi-phase19-upstream-li7fcj3v/reviewed-files.json` | `3ac36b511118a6ba510dd8246994aabdc283cc2237480dff069f71f328b959b5` |
| `C:/Users/Luiz/AppData/Local/Temp/opi-phase19-upstream-li7fcj3v/check_pinned_sources.py` | `55ed374e3a6db38d015af857d0ba6783bdd0bcc0d96199378475fe93cc96405c` |
| `C:/Users/Luiz/AppData/Local/Temp/opi-phase19-upstream-li7fcj3v/metric-probes.log` | `0c975e9880ead67cb58183296c365d9230434887ed6e56a58bf9ad6152289aec` |

Executed: `python C:/Users/Luiz/AppData/Local/Temp/opi-phase19-upstream-li7fcj3v/check_pinned_sources.py`.
Result: source integrity PASS (36 files); aggregation PASS (4 cases).
The source-fetch commands were `gh api repos/<repository>/git/trees/<locked-commit>?recursive=1`
and `gh api repos/<repository>/git/blobs/<tree-recorded-blob>`; blob content was
base64-decoded and independently Git-hashed before inspection.

```python
"""Verify pinned source bytes and probe only DeepSWE's metric aggregation."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parent
files = json.loads((root / "reviewed-files.json").read_text())
for entry in files:
    raw = (root / entry["source"] / entry["path"]).read_bytes()
    assert hashlib.sha256(raw).hexdigest() == entry["sha256"]
    header = b"blob " + str(len(raw)).encode() + b"\0"
    assert hashlib.sha1(header + raw).hexdigest() == entry["git_blob"]
print(f"Pinned source integrity: PASS ({len(files)} files)")

grader = root / "deepswe/tasks/abs-module-cache-flags/tests/grader.py"
cases = [
    ("all-pass", ["new"], ["old"], [("new", "passed"), ("old", "passed")], (1, 1.0)),
    ("partial", ["new"], ["old"], [("new", "failed"), ("old", "passed")], (0, 0.5)),
    ("no-f2p", [], ["old"], [("old", "passed")], (0, 1.0)),
    ("empty", [], [], [], (0, 0.0)),
]
results = []
for name, f2p, p2p, rows, expected in cases:
    task = root / "metric-probes" / name
    task.mkdir(parents=True, exist_ok=True)
    report = task / "input.json"
    report.write_text(json.dumps({"results": {"tests": [
        {"name": n, "status": status} for n, status in rows
    ]}}), encoding="utf8")
    (task / "config.json").write_text(json.dumps({
        "p2p_node_ids": p2p, "f2p_node_ids": f2p,
        "grade": {"format": "ctrf", "node_id": "name", "reports": [str(report)]},
    }), encoding="utf8")
    env = {key: os.environ[key] for key in ("SystemRoot", "WINDIR", "PATH", "TEMP", "TMP")
           if key in os.environ}
    env.update(TESTS_DIR=str(task), VERIFIER_DIR=str(task), APP_DIR=str(task),
               ARTIFACTS_DIR=str(task), PYTHONIOENCODING="utf-8")
    result = subprocess.run([sys.executable, str(grader), "grade"], env=env,
                            cwd=task, capture_output=True)
    assert result.returncode == 0, result.stderr.decode()
    value = json.loads((task / "reward.json").read_text())
    assert (value["reward"], value["partial"]) == expected, value
    results.append({"case": name, **value})
(root / "metric-probes.log").write_text(json.dumps(results, indent=2), encoding="utf8", newline="\n")
print("Native metric aggregation: PASS (4 synthetic cases; no prepare/Agent/container/model)")
```

## Reproducible Planning Probes

These are diagnostic test additions, not production edits or a new maintained
test binary. They reuse the current native-driver helpers in a unique export.
The seeded-cache helper creates temporary fixture Git repositories solely for
source-identity tests; it does not commit, stage, or change this repository.
After B3 correct the positive helper identity fields and retain each negative
as an independent mutation; otherwise another fixed boundary could mask the
intended assertion. D2/D3 probes are necessary discriminators, not sufficient
proofs of real grader behavior or correct metric definitions.

To reproduce on the recorded head, export that commit, extract the single Rust
fence under `Probe source` below, append it to the exported
`crates/opi-eval/tests/native_driver.rs`, resolve/lease the existing Cargo cache,
and execute the exact filter recorded in the disposition. Never append these
planning probes directly to an unrelated dirty production checkout.

```sh
task_export=$(mktemp -d /tmp/opi-remediate-repro.XXXXXXXX)
git archive 131f003d4ae883e30df3ec85eae7469b0d8c60cc | tar -x -C "$task_export"
# Extract the Rust fence from this fixed plan to $task_export/probes.rs.
cat "$task_export/probes.rs" >> "$task_export/crates/opi-eval/tests/native_driver.rs"
# Resolve and lease the repository-owned cache with opi-cargo-cache.py;
# use CARGO_INCREMENTAL=1 and run Cargo from $task_export.
```

### Raw evidence identities

Raw command logs are retained in the task's external temporary directory. Their
hashes bind the observations below; the preserved source and commands allow a
new export to reproduce them without trusting test names or narrative claims.

| Artifact | SHA-256 |
|---|---|
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-47ih7fw5/source.tar` | `1b0e714af1ebf35df0b032fd2415c9357dcaadb1f7cec94fb7052a81da711e08` |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-47ih7fw5/baseline.log` | `c02aaf30cc2ab6a961569a61edf4d29648d157da14db6d8471089b4204291eba` |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-47ih7fw5/probes.log` | `718a1ff72351cf3f6cad1d32bd9c66a03f12c36101b7e04e5f95c5b72028526d` |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-47ih7fw5/probes-003.log` | `288604f80a35f839e9a12840aab6f3f4b534195561bd627f170f2c8d2e6805a4` |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-47ih7fw5/probes-012.log` | `b1d76f523366e837bdba96ad06227ef8bb9748aa7e6eff8d0af8e2abf51a7963` |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-47ih7fw5/probes.rs` | `dbe158f82428cc5ac6d17f77630c8a7c4875595fb351ede7e42e482493c78ea2` |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-47ih7fw5/probes-refined.rs` | `17d7aba63835cf5914fdfa5bc5a5fa6d4a46377c48b32b6cffea9e9cdcca6c8f` |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-47ih7fw5/probes-005-refined.log` | `53c63bae958750cd79a7501610de39af669931de2b76dd57237e8297674339cc` |

### Probe source

```rust

// Planning probes appended only to a sealed source export's native_driver test.
fn remediation_runner(staged: &Staged, edit: impl FnOnce(String) -> String) {
    let path = staged.root.join("material/verifier-uv.sh");
    let body = edit(fs::read_to_string(&path).unwrap());
    write_exec(&path, &body);
    let mut material: serde_json::Value = serde_json::from_slice(&fs::read(&staged.material).unwrap()).unwrap();
    for value in material["benchmarks"].as_object_mut().unwrap().values_mut() {
        value["verifier_executable"]["sha256"] = serde_json::json!(sha256_hex(body.as_bytes()));
        value["oracle"]["sha256"] = serde_json::json!(sha256_hex(body.as_bytes()));
    }
    fs::write(&staged.material, serde_json::to_vec(&material).unwrap()).unwrap();
}

fn remediation_report(root: &Path) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval")).args(["report", "--root"]).arg(root).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn remediation_001_regrade_child_has_no_ambient_secret() {
    let staged = stage("ok");
    remediation_runner(&staged, |body| body.replacen("set -eu", "set -eu\nif [ \"${OPI_EVAL_REGRADE:-0}\" = 1 ]; then\n  if [ -n \"${ANTHROPIC_API_KEY:-}\" ]; then printf present > ambient-credential-observation; else printf absent > ambient-credential-observation; fi\nfi", 1));
    let config = write_config(&staged);
    let (root, _) = regrade_family_root(&staged, &config, "run");
    let out = staged.root.join("regrade");
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .args(["regrade", "--root"]).arg(root).arg("--native-material").arg(&staged.material).arg("--out").arg(&out)
        .env_clear().env("PATH", std::env::var_os("PATH").unwrap()).env("ANTHROPIC_API_KEY", "synthetic-remediation-canary")
        .output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stdout));
    let observations = walk_files_matching(&out, "ambient-credential-observation");
    assert_eq!(observations.len(), 2);
    for path in observations { assert_eq!(fs::read_to_string(path).unwrap(), "absent", "actual verifier child inherited dummy credential"); }
}

#[test]
fn remediation_002_regrade_dispatches_a_runner_command() {
    let staged = stage("ok");
    remediation_runner(&staged, |body| body.replacen("set -eu", "set -eu\nif [ \"${OPI_EVAL_REGRADE:-0}\" = 1 ]; then printf '%s' \"$#\" > regrade-argc; fi", 1));
    let config = write_config(&staged);
    let (root, _) = regrade_family_root(&staged, &config, "run");
    let out = staged.root.join("regrade");
    let (code, _, stderr) = regrade_run(&staged, &root, &out);
    assert_eq!(code, 0, "{stderr}");
    let observations = walk_files_matching(&out, "regrade-argc");
    assert_eq!(observations.len(), 2);
    for path in observations { assert_ne!(fs::read_to_string(path).unwrap(), "0", "pinned uv receives no runner command"); }
}

#[test]
fn remediation_003_substituted_terminal_identity_rejects() {
    let mut admitted = Vec::new();
    for case in ["product", "executable", "model"] {
        let staged = stage("ok");
        remediation_runner(&staged, |body| match case {
            "product" => body.replace("\"product\": \"%s\"", "\"product\": \"oracle-%s\""),
            "executable" => body.replace("printf 'a%.0s'", "printf 'f%.0s'"),
            _ => body.replace("\"name\": \"scripted/eval\"", "\"name\": \"unrelated/model\""),
        });
        let config = write_config(&staged);
        let root = staged.root.join("run");
        let (code, stdout, stderr) = run_native(&staged, &config, &root);
        if code == 0 { admitted.push(case); }
        else { assert!(!stdout.is_empty() || !stderr.is_empty()); }
    }
    assert!(admitted.is_empty(), "substituted identity fields admitted: {admitted:?}");
}

#[test]
fn remediation_004_native_provider_error_is_not_scored() {
    let staged = stage("agent-nonzero");
    remediation_runner(&staged, |body| body.replace("write_job \"$job\"\nanswer=", "write_job \"$job\"\nprintf '%s\\n' '{\"type\":\"error\",\"error\":{\"type\":\"authentication_error\",\"message\":\"invalid API key\"}}' > \"$job/$trial/agent/stdout.log\"\nanswer="));
    let config = write_config(&staged);
    let root = staged.root.join("run");
    let _ = run_native(&staged, &config, &root);
    let report = remediation_report(&root);
    assert_eq!(report["coverage_counts"]["valid"], 0, "authentication failure entered denominator: {}", report["coverage_counts"]);
}

#[test]
fn remediation_005_native_stage_deadlines_reach_runner() {
    let mut missing = Vec::new();
    for field in ["time_limit_seconds", "agent_timeout_seconds", "verifier_timeout_seconds"] {
        let staged = stage("ok");
        remediation_runner(&staged, |body| body.replacen("set -eu", "set -eu\nprintf '%s\\n' \"$@\" > native-dispatch-argv.txt", 1));
        let config = write_config(&staged);
        let text = fs::read_to_string(&config).unwrap().replace("[execution]", &format!("[execution]\n{field} = 1"));
        fs::write(&config, text).unwrap();
        let root = staged.root.join("run");
        let (code, stdout, stderr) = run_native(&staged, &config, &root);
        assert_eq!(code, 0, "positive dispatch failed: {stdout} {stderr}");
        let invocations = walk_files_matching(&root, "native-dispatch-argv.txt");
        assert_eq!(invocations.len(), 2, "inspect both actual runner spawns");
        for path in invocations {
            let args = fs::read_to_string(path).unwrap();
            let args: Vec<_> = args.lines().collect();
            let job = args.windows(2).find(|pair| pair[0] == "-c" || pair[0] == "--config")
                .map(|pair| serde_json::from_slice::<serde_json::Value>(&fs::read(pair[1]).unwrap()).unwrap());
            let cap = job.as_ref().map(|job| if field == "verifier_timeout_seconds" {
                &job["verifier"]["max_timeout_sec"]
            } else { &job["agents"][0]["max_timeout_sec"] });
            if cap.and_then(|v| v.as_f64()) != Some(1.0) { missing.push(field); }
        }
    }
    assert!(missing.is_empty(), "accepted stage caps missing from actual runner configuration: {missing:?}");
}

#[test]
fn remediation_006_plan_material_rejects_without_panic() {
    let dir = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .args(["run", "--plan"]).arg(dir.path().join("missing-plan.json"))
        .arg("--native-material").arg(dir.path().join("missing-material.json"))
        .arg("--root").arg(dir.path().join("run")).arg("--fixtures").arg(fixtures()).output().unwrap();
    assert_eq!(output.status.code(), Some(2), "expected typed input rejection, got {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn remediation_007_prepare_rejects_impossible_storage() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache");
    seed_local_cache(&cache);
    let task = cache.join("packages/terminal-bench-2.1/task.toml");
    let mut text = fs::read_to_string(&task).unwrap();
    if text.contains("[environment]") { text = text.replace("[environment]", "[environment]\nstorage_mb = 9000000000000"); }
    else { text.push_str("\n[environment]\nstorage_mb = 9000000000000\n"); }
    fs::write(task, text).unwrap();
    let config = experiment_with_cache(dir.path(), &cache);
    let output = run_prepare_env(&config, &dir.path().join("prepared"), &[("DOCKER_HOST", "tcp://127.0.0.1:1")]);
    assert!(!output.status.success(), "impossible storage admitted: {}", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn remediation_008_feasible_second_task_can_freeze() {
    let staged = stage("ok");
    stage_corpus(&staged);
    let digest = derive_integrity(&staged);
    let config = staged.root.join("selection.toml");
    fs::write(&config, selection_experiment_text(&staged, &digest, "[\"fixture-task\", \"alpha-notes\"]")).unwrap();
    stage_output_control(&staged, &config);
    let (output, stdout) = run_plan_with_closed_engine(&staged, &config);
    assert!(output.status.success(), "feasible multi-task subset rejected: {stdout} {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn remediation_009_rejected_rerun_preserves_control() {
    let staged = stage("ok");
    let config = write_config(&staged);
    let (root, _) = regrade_family_root(&staged, &config, "run");
    let before = fs::read(root.join("run-control.json")).unwrap();
    let text = fs::read_to_string(&config).unwrap().replace("experiment_id = \"native-driver-test\"", "experiment_id = \"changed-experiment\"");
    fs::write(&config, text).unwrap();
    let (code, _, _) = run_native(&staged, &config, &root);
    assert_ne!(code, 0);
    let after = fs::read(root.join("run-control.json")).unwrap();
    assert_eq!(sha256_hex(&after), sha256_hex(&before), "rejected run changed existing run-control bytes");
    let _ = remediation_report(&root);
}

#[test]
fn remediation_011_reports_identify_native_metric_meaning() {
    let staged = stage("ok");
    let mut definitions = Vec::new();
    for (name, config) in [("tb21", write_config(&staged)), ("tb30", write_tb30_config(&staged)), ("deepswe", write_deepswe_config(&staged))] {
        let (root, _) = regrade_family_root(&staged, &config, name);
        let report = remediation_report(&root);
        definitions.push(report["metrics"][0]["definition"].clone());
    }
    assert!(definitions.iter().any(|d| d != &definitions[0]), "all families publish the same generic definition: {definitions:?}");
}

#[test]
fn remediation_012_native_unobserved_time_is_unknown() {
    let staged = stage("ok");
    let config = write_config(&staged);
    let (root, _) = regrade_family_root(&staged, &config, "run");
    let mut observed = 0;
    for path in walk_files(&root) {
        if path.file_name().and_then(|v| v.to_str()) != Some("trajectory.json") { continue; }
        let value: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        if let Some(nodes) = value["nodes"].as_array() {
            for node in nodes {
                if node["provenance"]["rule"] == "agent-execution/1" {
                    observed += 1;
                    assert!(node["facts"]["wall_time_ms"]["Unknown"]["reason"].as_str().is_some_and(|s| !s.is_empty()), "native unobserved time must carry an unknown reason: {}", node["facts"]["wall_time_ms"]);
                }
            }
        }
    }
    assert!(observed >= 2, "probe must inspect both native Agent execution nodes");
}
```

## Handoff boundary

Task output is the fixed plan/disposition pair only. Repository test impact:
`none`; planning probes: `add` in the temporary source export only. Proposed
apply test impact: update the listed Rust/script tests, add the two bounded
native-driver cases and private adapter Python test, and retain unrelated
coverage; no test deletion. Production, source records, ledgers, commits, and active index remain
unchanged. D1-D3 are resolved; C/010 remains explicitly excluded. Before apply,
validate this READY-FOR-APPLY plan's exact digest and source/index/head/path
bindings and obtain the new explicit digest-bearing invocation required by the
invoked skill.
