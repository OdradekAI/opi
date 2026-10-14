# Phase 19 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `23b39dcdc52f71dc1b17b4f87678f39aa3d86289c3753eb0b5f55373ad82873e`
**Remediation head**: `9ef3768968a559e0bc0366c1f3c66752d499595a`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: committed source `9ef3768968a559e0bc0366c1f3c66752d499595a`; staged [], unstaged [], untracked [] at planning admission. The replacement changes only this fixed plan pair.
**Unresolved decisions**: none

## Source and Admission

The source baseline is the user-authorized local commit above. Inspecting and
offline planning probes used a unique `git archive` export, not a Git worktree:
`C:/Users/Luiz/AppData/Local/Temp/opi-phase19-ready-8b4c434fcdb34b0191a95e9d6a47f7d4/source`. Archive SHA-256:
`69b8f67b55ce2dc67854f029894da6d2dedda3e42dfbb28c6d4d67846d1a910e`.
Admission evidence is `C:/Users/Luiz/AppData/Local/Temp/opi-phase19-ready-8b4c434fcdb34b0191a95e9d6a47f7d4/planning-admission.json`. The clean tracked fixed
plan group is replaced without consuming its earlier conclusions.

The validated live set is revision 25, aggregate FAIL. Its strict union has
eight distinct source keys; repeated textual IDs never merge.

| Alias | Exact audit run | Owning findings SHA-256 |
|---|---|---|
| C | phase19-codex-gpt61-2ec1ccb-20261004t023807z | bd454f15884f80e537772452aabc05593d73763e5f4500326edbf13e8d018bc4 |
| P | phase19-pi-glm53-9d805b4-20261004t094337z | 377d730be9a09201c6056b3513739fa2f524ec684019ae516ef04182da56eeaf |

Scope derives from `docs/opi-spec.md` (SHA-256
`1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db`)
and the currently registered Phase19 source pair:
`docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`
(`f507f29d91ddfc919ead6fa84df8d5f912b04af1d84ed5e68777e20c80c135c5`)
and its Chinese counterpart
(`7d97f00f3a06723765e8cb2c8ea7ed13933defc8d7c230bdc0d165ad1284e7dc`).
Sections 3/7/8/10 require TB2.1/TB3, four attributable real paired trials, and
current component conformance. AC005/D04 DeepSWE is deferred/reserved. Preserve
the original indexed findings/requirements and all scoped-out original evidence.

Prior remediation dispositions, history, or eval conclusions are not closure
authority. Current registered source, independently inspected current code,
fresh offline probes and individually qualified original raw proofs determine
the decisions below. The indexed advisory appendix supplies bounded leads only.

## Current Finding Verification

Source severity is retained for every key; Refuted does not rewrite its audit.

| Source run / Finding ID | Verification | Source/final severity and current rationale | Closure key / family | Batch | Decision |
|---|---|---|---|---|---|
| C / P19-AUD-001 | Refuted | Major/Major; parsed HTTP(S) host guard rejects both fragment bypasses and alternate remote routes; 32 actual CLI cases pass across both non-live modes. | evaluation.nonlive-parsed-loopback-host / evaluation.boundary | none | no action |
| C / P19-AUD-002 | Refuted | Major/Major; checked cardinality rejects before Vec allocation; equality/small-over/u32::MAX-over cases exit 0/2/2 under 256 MiB, with no over-cap plan. | evaluation.selection-cardinality-before-allocation / evaluation.boundary | none | no action |
| C / P19-AUD-003 | Partially confirmed | Major/Major; current TB2.1/TB3 native bodies remain unexecuted; DeepSWE is outside current scope and the exact selectors exist. Existing stand-in-client regrade does not alone prove saved actual-client artifacts. | evaluation.current-native-conformance-evidence / evaluation.native-conformance | B1 | complete private test fixture and native proof |
| C / P19-AUD-004 | Refuted | Major/Major; newly supplied original raw four-trial evidence, current-source public verifier/report and complete qualified workspace receipts establish the current operator predicate. | operator.current-attributable-raw-acceptance / operator.evidence | none | no action |
| P / P19-AUD-001 | Refuted | Major/Major; independent original seals, actual first connection/tool loops and four attributable native results are supplied and verified under current registered scope. | operator.current-attributable-raw-acceptance / operator.evidence | none | no action |
| P / P19-AUD-002 | Refuted | Minor/Minor; consumer reads matching flushed structured Opi/Pi stdout start records and empty stderr; removed stderr-token assertion is absent. | evaluation.deadline-structured-agent-start / evaluation.native-conformance | none | no action |
| P / P19-AUD-003 | Refuted | Minor/Minor; current exact native_provider_failure_report is one ignored Unix test, with a separately named ordinary fixture. Static selector proof is distinct from future native body passage. | evaluation.provider-report-exact-ignored-selector / evaluation.native-conformance | none | no action |
| P / P19-AUD-004 | Refuted | Info/Info; selected-family material/7 annotation matches the loader; material/6 survives only in intentional negative cases. | evaluation.cleanup-ignore-current-material-schema / evaluation.native-conformance | none | no action |

### Qualified current proof

- Five-key independent proof: `C:/Users/Luiz/AppData/Local/Temp/opi-phase19-ready-8b4c434fcdb34b0191a95e9d6a47f7d4/refutations/verification.json`,
  SHA-256 `00d65ecb2bddd104cefae78410e5490b83cc3073edd985d38a01d9204bdf6b34`.
  Exact current build applicability covers 716 build input files; 35 offline CLI
  cases pass. Native test bodies/Docker/live calls executed by these probes: zero.
- Operator proof: `C:/Users/Luiz/AppData/Local/Temp/opi-phase19-ready-8b4c434fcdb34b0191a95e9d6a47f7d4/operator-qualification/current-status.json`,
  SHA-256 `0bba904035b7bb30bd09d558c5a11bc7e2478dc923e34ed4a204124b34b0f6d6`.
  Fresh public acceptance, two verify, two report and exhaustive two-Opi artifact
  audit commands exit 0. 117 preserved hashes, original material bytes, both
  frozen plans and six command-log/exit files match. Four eligible actual trials:
  TB2.1 pi/Opi rewards 1/1, completed tool pairs 13/7; TB3 pi/Opi 1/0, tool pairs
  61/75. Both reports are comparable; unknown telemetry remains unknown.
- Original operator root: `/home/luiz/phase19-acceptance-r2`. Primary derived view:
  `/mnt/c/Users/Luiz/AppData/Local/Temp/opi-phase19-evidence-continuation-lcprjvy7/operator-f2-evidence`,
  manifest SHA-256 `936e722ae9d2c54a5940a9d83c48a5db10eeceab7bccf1a913d4f01ace787e14`.
  Its authority is limited by the original-reference qualification above.
  Retain all 28 original sealed trials, including 24 nonprimary and six
  scoped-out DeepSWE trials, separately from the primary four-trial headline.
- Qualified current Companion SHA-256:
  `cdce44783b820bb34f4ef93209cb2a04a2a027181aa31e2bc987d6e66d39cd37`.
  Successful raw locked/offline build receipt:
  `8a5f34959873f6c9b737767a251aa1c3ce5d0de3cda077754056e0a57940f6e9`;
  source patch `de4fc115c0ff0d64263b024f73554276b2e0210eaa6ffb057b330401cbd0cdf9`
  equals the exact committed ten-source-file diff. Manifest/lock and remaining
  build inputs are independently unchanged. Original outer CLI argv/build
  timestamps remain unknown; they are not reconstructed from filesystem mtimes.
- Raw prior final gate command/result/log hashes were individually requalified
  against identical current runtime/test/build inputs: 4679 tests passed,
  zero failed, 18 ignored; 12 doc tests passed; fmt/Clippy/rustdoc passed.
  This is current applicability evidence, not a claim that ignored native
  bodies ran. New plan documentation/validator checks are recorded separately.

## Unresolved Decisions

none. Required future material preparation and native executions are explicit
steps in B1. Their unexecuted green checks remain `not-run`; missing material or
a failing predicate yields nonclosure, never an invented PASS.

## Closure Batches

### Batch B1: Complete current native conformance evidence

**Closure predicate**: Every exact selected native case executes against its
bound source/build/client/task/runner/image material and passes; official
TB2.1/TB3 graders distinguish submitted correct/incorrect artifacts, actual
Opi/pi clients prove authenticated protocol/tool/control flows, their saved
artifacts obtain fresh cache-only verifier observations with zero Agent/provider
regrade dispatch, and owned cleanup preserves original evidence and neighbors.
The complete predicate requires the twelve libtest cases and every named inner
proof/count below. This closes the component evidence gap, without claiming
new real-provider acceptance or Phase exit.

**Dependencies**: none. Current-source refutation and operator qualification
are already independently bound. Do not silently change their frozen inputs.

**Verification union**: exact native union and anti-vacuity checks; actual-client
saved-artifact proof; original evidence verification; paired handoff/doc checks;
final workspace union once after the actual edits.

#### Fix B1.1: Complete the private native fixture and retain executed evidence

- **Finding source(s)**: `phase19-codex-gpt61-2ec1ccb-20261004t023807z` + findings SHA-256 `bd454f15884f80e537772452aabc05593d73763e5f4500326edbf13e8d018bc4` + `P19-AUD-003`.
- **Decision**: `fix:complete-native-conformance-fixture-and-source-bound-proof`.
- **Verification status**: Partially confirmed.
- **File(s)**: `crates/opi-eval/tests/native_driver.rs`; `crates/opi-eval/tests/support/native_witness_material.rs`; `docs/audits/phase19-local-native-acceptance.md`; `docs/audits/phase19-local-native-acceptance.zh.md`.
- **Change kind**: test-only, plus synchronized documentation handoff.
- **Change**: One bounded private-fixture change aligns TB3's expected source with its admitted collection declaration and adds actual-client saved-artifact regrade coverage using current production operations. Run/retain the source-bound native union, then record exact executed proof/limits in both existing handoff documents. Production runtime/policy/schema/source/ledger/lock/client/task scripts stay byte-stable.
- **Closure predicate**: B1's complete native predicate, including the twelve exact filters and the actual-client/official-grader compositional proof; no zero-test, stand-in-only, unknown-effect or inferred native success substitutes.
- **Red-before**: N/A: test-only missing coverage and stale private path assertion; current source statically demonstrates both. No product behavior change is planned, and no unexecuted body is labelled an observed FAIL. Before changing the existing TB3 assertion, retain its actual baseline result if its pinned material is executable.
- **Green-after**: Each of the twelve exact filters executes one passed test; all material/inner-count/source-preservation/cleanup predicates and final gates pass. Plan-stage observation: `not-run`.

#### Fixture details

1. Replace the hardcoded `native-collected: /app/evalbench/ ` expectation at
   native_driver.rs:3879 with the exact sole declared source bound by the
   admitted selected task (`/app/results.txt`). Derive it from verified task
   metadata, or assert the exact canonical pin before the exact source.
   Retain distinct digests, declared-only collection, unrelated-file exclusion
   and native 1/0 result checks; do not broaden allowed paths.
2. Add ignored exact `native_real_client_offline_regrade` next to existing
   regrade tests. Reuse `admit_clients`, `retain`/`recheck`, `run_cli` and
   `controlled_material_selected(..., PHASE_FAMILIES)`. A private constructor
   restores the exact admitted actual `agents` entries after constructing
   explicitly labelled no-network diagnostic tasks; it preserves original
   packages, Node/pi bundle, build receipts, runner and capture-helper bytes.
   Reuse the existing real-client provider-report bootstrap and turn-budget
   import-wrapper seam; no transported client/provider code is patched.
3. Run four source native jobs: Opi and pi each execute one diagnostic TB2.1
   positive task and one diagnostic TB3 negative task. Prove actual shell/file
   tool effects, terminal preflight and exact artifact digests. Rewards are
   1,1,0,0. Each source grading job has one first request plus one tool
   continuation, eight grading-loop requests total. Count setup/preflight
   probes separately, keyed to their own jobs; never hide extra requests.
4. Through public `regrade`, preserve these source roots and produce four
   fresh native grading records with new verifier/job identities, source bundle
   references and matching rewards. Prove no Agent logs/trajectory/dispatch,
   unchanged provider logs, source directory digests/seals/scores, credential
   canaries absent from grader/export, and only declared TB3 artifacts crossing
   the separate-verifier boundary. Docker queries must show zero residual
   owned containers and preservation of the exact neighbor/shared images.
5. Retain tamper, missing input/cache, existing/source-contained output,
   credential/control/identity substitution, timeout and cleanup-uncertainty
   negatives. Missing-cache probes use an isolated copied material/engine
   observation, never delete a shared image/cache. An original network-required
   task remains an honest offline refusal; no guest script/network flag/seal is
   rewritten to fabricate original-task success.

This is compositional conformance: the existing official task witnesses prove
the original graders' 1/0 response and attribution; the new real-client case
proves saved actual-client artifacts through production fresh verifier-only
execution in both lifecycle families. Private diagnostic metrics do not become
original benchmark Agent scores. `CTRL-006` does not ban ordinary initial native
task grading network; Section 6 requires cache-only/no-network regrade and
truthful rejection of missing offline material.

#### Pinned material and effect scope

The detailed independently reviewed design and input/source references are:
`C:/Users/Luiz/AppData/Local/Temp/opi-phase19-ready-8b4c434fcdb34b0191a95e9d6a47f7d4/ready-native-design.md`
(SHA-256 `6b9b8a8606873faf331629deb35a3926196b3be40fb7b70a811b1ece65a733da`),
`ready-native-design.json`
(`409727407c780f580de892bf007b5a7847a131eb17976cde905a359eaaa00907`)
and `ready-native-images.json`
(`437997abbbbf8d2aa3d37fa172b853ccc65a0bed91716f6e09176279b914b7fc`).
Recheck these exact raw bytes plus current input identities before reuse.

| Input | Frozen identity / rule |
|---|---|
| Current static lock | `crates/opi-eval/external-locks/static/linux-x86_64.json`, `e7e5b848f05a46a81ac236c216c3fe73820fd6547b2b87c40b5a1a9385a7c7f1` |
| TB2.1 task | openssl-selfsigned-cert; source `7131e4375048a0e408a8fb404b5f499d726b695b`; task tree `4c5b1214db4b807f2bc4c1bff8803402e36b648b`; package manifest `3e51600d207758c47863f77d116dc934852340a0f2aeb29f52c37f050e6b48e1` |
| TB3 task | foodstuff-beta-activity; source `2b0442c3c583b710ca8da14c8e601b99f2f1f244`; task tree `7ed5eb80cfd496444f1effd6c7050cae990a6c86`; 13-row package manifest `7285d3616fc527c3af1d5494e28abfa1fcd3ad9fbdbb5922a197b2c93fcca261` |
| TB2.1 environment | `alexgshaw/openssl-selfsigned-cert@sha256:4c948a4e630af2435ae0a19108fc0814a946ac2fa29a512469e0fc77b38c8c12` |
| TB3 complete selected FROM graph | Both environment and tests Dockerfiles use only `python:3.12-slim@sha256:2b4f19dae3a777dfc3b76730bda1e82e1f66ab2a2686fa93ca78edbfb4f04ffe`; raw/package/profile bytes independently matched |
| Private diagnostic base | `python@sha256:272efc3f94bec901e2bc7a5c9984b3d113c317839edca2714c6a388b9476f8fc` |
| Actual Opi client | 0.8.2; source `3908fba723956a6af7daf298599cbe3d30bbbc09`; Cargo.lock `261a3ec501be2e045dc6c9b2b9a940adc1c56305f477008eea940f26545925f3`; locked release/default-features musl package and clean exact build receipt |
| Actual pi client | 1.0.4; source `7c10bd4337495ee613f2224843ecdf349b80d1df`; Node 22.19.0; exact static-lock source/package/package-lock/model-data and clean build receipt |
| Runner | Harbor v0.22.0 `4407eb5227a2ff4f0d3f16b2eb48849382fdf276`; uv 0.9.7; exact source/runtime/dependency inventory |
| Helper/provider/integration | Current-source musl Companion capture helper, current scripted loopback provider and Harbor integration bytes; no credential/global configuration reads |

Use a unique caller-owned WSL/Linux cache/evidence directory, concurrency 1.
Seed the current exact pins, profiles and integrations, then public `doctor` and
`prepare --config C --out O` per family from that independent directory.
Cold Opi preparation uses the exact paired local source/commit above; missing
local source refuses without fetch. Preserve both prepared local-origin
material/7 manifests and receipts. A fresh test-only composite merges only
disjoint verified benchmark entries after exact common lock/client/helper/runner
equality; the existing material loader revalidates it. DeepSWE module shape may
remain compatible, but no DeepSWE corpus/image/native execution is selected.

Separate caller-owned preparation may acquire only these declared public pinned
sources/dependencies/images and build the exact selected task/verifier/sidecar
contexts and explicitly identified private diagnostic contexts. Cached names
or history alone never prove COPY payload identity. Require source/base/context/
tool/argv/package modes and resulting image IDs; if cache payload cannot attest
the exact context, build that context and record/freeze immutable outputs before
first task container/client dispatch. Optional builder uses the existing locked
BuildKit. No floating image/revision, shared retag or cache deletion is admitted.
Task CPU/memory/time/network declarations are preserved. Initial official
grading may use its declared public material network; provider traffic stays
literal loopback with synthetic keys. No real credentials, paid API calls,
new real-provider trials, uploads, pushes or releases are in scope.

Bound preparation to 3600 seconds per stage and the entire B1 to 48 hours.
Each test executes sequentially with finite source-declared task/settlement
budgets. Outer ceilings: TB2.1 witness 30120s; TB3 21720s; client-only and provider
filters 3600s each; owned/conformance cleanup 900s each; deadlines 10000s;
existing regrade 7200s; new real-client regrade 3600s. These ceilings never
weaken task declarations or convert timeout into PASS. Failure stops affected
dispatch and preserves partial proof; settle only observed owned processes/
containers/temp credentials. Original user evidence, packages and shared Cargo/
image caches are retention targets. No global Docker prune or Cargo clean.

Regrade itself remains cache-only, no-network, no build/pull, no Agent/provider.
The original openssl apt/uv transport cache alone does not establish offline
grader readiness. Unsupported original offline regrade must reject truthfully.

#### Execution and count contract

Run the exact same package/features/toolchain for complete all/ignored
inventories and executions. Current source has 13 ignored native definitions.
After this one addition it should have 14, with these twelve selected and the
two existing DeepSWE/all-family compatibility witnesses excluded:

- `native_tb21_artifact_witness`
- `native_tb3_artifact_witness`
- `native_protocol_client_matrix`
- `native_client_turn_budgets`
- `native_output_control_client_matrix`
- `native_operator_control_proof_validates_exact_clients`
- `native_provider_failure_report`
- `native_owned_resource_cancellation`
- `native_conformance_owned_cleanup_witness`
- `remediation_native_stage_deadlines`
- `native_offline_regrade`
- `native_real_client_offline_regrade`

Capture complete inventories to `native-all.log` and `native-ignored.log`:

```sh
cargo test --offline --locked -p opi-eval --test native_driver -- --list
cargo test --offline --locked -p opi-eval --test native_driver -- --ignored --list
```

For each exact filter use a complete separate transcript and original exit:

```sh
cargo test --offline --locked -p opi-eval --test native_driver FILTER -- --ignored --exact --nocapture --test-threads=1
python3 .agents/skills/opi-audit/scripts/check_test_execution.py --list-output "$APPLY_ROOT/native-all.log" --ignored-list-output "$APPLY_ROOT/native-ignored.log" --run-output "$APPLY_ROOT/FILTER.log" --filter FILTER --exact --selection ignored --expected-count 1
```

Required outer count: twelve tests actually executed, each 1 passed/0 failed.
Listing, normal ignored observations, source declaration counts or zero-test
exit 0 cannot replace it. Preserve every inner subcase command/exit, native
raw result, build/material/client identity and artifact/lifecycle proof.

Required inner proof union: official witnesses 2 native jobs per family
(correct/oracle=1, incorrect/nop=0); protocol 3 APIs x 2 actual clients x 5
scripts=30 executions; turn-budget request counts 52/2/52/52/0; output-control
and operator-control each two clients with first+tool continuation;
provider/report five cases x two clients; deadlines nine cases x two admitted
families x two subjects; existing regrade five selected cases x two subjects,
including two timeout refusals; new actual-client regrade four source jobs,
eight source grading-loop requests and four fresh grades. Current source
assertions own more detailed counts and negative predicates; preserve them.

#### Ordered work and evidence handoff

1. Admit exact plan/index/head/input bytes -> verify: locked `apply` validator
   with the emitted approved digest, current strict union and carried-in path
   inventory; refuse intervening non-plan source changes or overlap.
2. Complete the single private test fixture -> verify: exact declared TB3 source
   and actual-client package/preflight/2-request loop assertions; only four
   planned repo paths changed, no production/public/source/ledger/lock drift.
3. Prepare and freeze external materials -> verify: source/modes/client/package/
   runner/base/context/output image identities plus public doctor/prepare;
   no real auth/provider effect and no mutation of original supplied material.
4. Execute every exact native filter -> verify: complete inventory/checker
   counts and every inner predicate; new real-client regrade has zero Agent/
   provider requests and source/neighbor preservation.
5. Record paired current evidence handoff -> verify: both EN/ZH documents cite
   exact fresh command/exits/counts/raw hashes and explicit compositional limits,
   current four-trial criterion and retained scoped-out evidence.
6. Run the final deduplicated verification union -> verify: all applicable
   workspace gates once for final relevant inputs and source/evidence hash
   preservation; owner-bound result disposition/Markdown validation.
   The execution protocol owns any genuinely blocking bounded incidental repair;
   protected public/schema/authority changes require a new plan.

## Final Verification

The applicable Phase/final-source union is:

```sh
python3 scripts/opi-doc-check.py
git diff --check
cargo fmt --check --all
cargo clippy --offline --locked --workspace --all-targets -- -D warnings
cargo test --offline --locked --workspace --all-targets
cargo test --offline --locked --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --offline --locked --workspace --no-deps
```

Keep incremental compilation and lease the existing external Cargo target;
inspect with `python3 scripts/opi-cargo-cache.py status`. No per-session targets
or cache deletion. Default workspace tests cover ordinary conformance/negative
cases; the twelve explicitly selected ignored tests are additional actual native
checks. Run every added/modified test filter; do not repeat unchanged green
checks except for new source changes/failures/unresolved applicability.

Reverify the four operator bundles with a fresh source-bound Companion, render
both family reports deterministically, and use the public acceptance verifier
plus exhaustive two-Opi artifact audit. Reuse current qualified raw gate/probe
receipts only after proving their relevant inputs unchanged; new fixture/docs
inputs invalidate their affected checks. Original derived outputs stay intact;
all new outputs use distinct fresh paths outside originals.

## Exclusions

| Source key | Disposition | Current evidence/limit |
|---|---|---|
| C / P19-AUD-001 | Refuted | Current parsed-host guard plus 32 exact bounded CLI cases; no remote/network execution. |
| C / P19-AUD-002 | Refuted | Preallocation count and three actual bounded CLI cases; rejection before effects. |
| C / P19-AUD-004 | Refuted | Independently supplied/verified four-trial original proof and unchanged-source full gate receipts; overall C003/Phase conformance remains separate. |
| P / P19-AUD-001 | Refuted | Same falsifiable operator evidence predicate, independently source-bound; preserve this separate source identity. |
| P / P19-AUD-002 | Refuted | Structured producer/consumer start shapes; native timing/body passage is required in B1, not claimed by static proof. |
| P / P19-AUD-003 | Refuted | One current ignored exact declaration; actual execution count is separately required by B1. |
| P / P19-AUD-004 | Refuted | Current material/7 annotation; deliberate legacy-schema negatives retained. |

No source key is removed, silently merged, lowered in source severity or added
from eval/history. All plan `green_after.observed` values are `not-run`.
Ready status approves a concrete future apply scope; actual evidence determines
result closure. It does not change the indexed FAIL verdict, declare the Phase
conformant, update a ledger, invoke implementation/audit/archive or reuse an old
apply approval. The fixed contract has only this English-field Markdown plan and
its JSONL sidecar; no Chinese plan sibling exists. Planned user-facing handoff
documents are synchronized in both languages.

Test impact: **add** one ignored actual-client regrade case; **update** one TB3
witness assertion; **retain** current native/ordinary/compatibility tests;
**delete** none. Materialization of this plan replaces only its fixed pair.
