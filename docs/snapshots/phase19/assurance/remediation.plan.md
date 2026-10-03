# Phase 19 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `fe9e6af9cd26f69556c0224f219500626181ae2829d7099af99500e73faff5db`
**Remediation head**: `f039d682aec11365e381685186faad114f0b909f`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: original plan admission at `3e1301178c471499f0fc36d4a8b1cd1f467cef7d` had staged=[], unstaged=[], untracked=[]; commit continuation at remediation head has staged=[], task-owned unstaged=[remediation.plan.md, remediation.plan.dispositions.jsonl], carried-in=[], untracked=[].
**Unresolved decisions**: none

## Admission and source identity

audit-set validation passed before semantic inputs. Index revision 21 aggregates FAIL. Two equal peers contribute 12 strict-union source keys (Codex 11, pi 1). Same textual IDs stay separate. Prior fixed plan outputs were tracked/clean and replaced without reading earlier content. Existing results stay untouched and are not current closure evidence.

| Alias | Full source run | Audit head | Findings SHA-256 | Verdict | Requirements |
|---|---|---|---|---|---|
| C | `phase19-codex-gpt61-f60e458-20261003t041015z` | `f60e458f9d1bceffcba5caaef11e8c0d7312ff59` | `37f779a15dad2250e807239cc4b4718256a117402fdba496fc3da8452e47c28a` | FAIL | 56 |
| P | `phase19-pi-glm53-2e82c99-20261002t204027z` | `2e82c99aa32ca3a56acef3e2b969223161d370c7` | `d8dd6dd9833a5575975171b815f790d5c1c26ae3df4d5b94b21a84663cb8c7d1` | PASS | 13 |

Aliases resolve only to these exact run/digest identities. Original source evidence/severities stay unchanged. Current run/digest controls admission; title similarity cannot replace identity, and an older source cannot enter this union.

Committed source was exported using git archive into unique external temporary directories, without a Git worktree. WSL/Linux locked/offline Cargo build passed using the existing stable external cache and incremental compilation. B6 adds only its regression test in a separate archive copy; production bytes remain baseline. Broad searches and a final Git diff accidentally exposed history rows and removed prior-plan text; none supplied a finding decision, authorization or closure evidence. The affected worker was replaced, and current-file reads name active files exactly.

The user subsequently authorized committing all project changes. Research commit `f039d682aec11365e381685186faad114f0b909f` adds only the 16 non-normative docs/research/pi-1.0.0/ files, so the fixed plan is renewed at that committed source baseline. A new unique git archive export was inspected: 922 existing non-plan file/symlink identities (exact byte digests, modes and link targets) match the original `3e1301178c471499f0fc36d4a8b1cd1f467cef7d` evidence export; the current index and all registered source digests also match. The new export at /tmp/opi-commit-baseline-SzKBh64x passed its own python3 scripts/opi-doc-check.py. All recorded original source-dependent red checks and two executed native witnesses therefore remain applicable at remediation_head; their build/execution provenance stays at the original head. No invalidating source change or new runtime execution occurred. Pending green/native/final checks remain pending. The exact comparison inventory and receipts appear below; research prose supplies no finding or closure authority.

Only registered source fields were projected from the committed ledger: docs/opi-spec.md (1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db), Phase19 English source (5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a) and Chinese counterpart (d41c826412b0d71802327d60f953708f6cb2083436a2a2b0d99a2e52d851f194). Scope derives from parent CTRL-002/003/005/006 and registered Phase19 sections4-9/AC001,006,008,009,010,011,012,013. No spec, ledger, dependency graph, Opi runtime or authority change is proposed.

## Criterion cross-reference

Exact criterion identity remains each sidecar `(path, sha256, citation)`. Both peers cite the same registered source bytes with different citation granularity. This table cross-references coverage; it does not merge original identities/states or resolve disagreement by voting. All 69 original states remain represented.

| Registered AC family | C original requirement/state | P original requirement/state |
|---|---|---|
| P19-AC-001 | P19-AC-001-diagnose: partially-met; P19-AC-001-prepare: partially-met; P19-AC-001-plan: partially-met; P19-AC-001-run: partially-met; P19-AC-001-live: not-assessable | P19-R-001: met |
| P19-AC-002 | P19-AC-002-binding: met; P19-AC-002-reject: met; P19-AC-002-settlement: met; P19-AC-002-capture: partially-met | P19-R-002: met |
| P19-AC-003 | P19-AC-003-lifecycle: met; P19-AC-003-witness: not-assessable; P19-AC-003-live: not-assessable | P19-R-003: met |
| P19-AC-004 | P19-AC-004-handoff: met; P19-AC-004-witness: not-assessable; P19-AC-004-live: not-assessable | P19-R-004: met |
| P19-AC-005 | P19-AC-005-patch: met; P19-AC-005-witness: not-assessable; P19-AC-005-live: not-assessable | P19-R-005: met |
| P19-AC-006 | P19-AC-006-projection: met; P19-AC-006-fixture: not-assessable; P19-AC-006-controls: met; P19-AC-006-terminal: partially-met; P19-AC-006-fallback: met; P19-AC-006-live: not-assessable | P19-R-006: met |
| P19-AC-007 | P19-AC-007-inventory: met; P19-AC-007-preflight: partially-met; P19-AC-007-freeze: partially-met | P19-R-007: met |
| P19-AC-008 | P19-AC-008-metrics: partially-met; P19-AC-008-coverage: partially-met; P19-AC-008-classification: met; P19-AC-008-pairs: partially-met; P19-AC-008-telemetry: met | P19-R-008: met |
| P19-AC-009 | P19-AC-009-verify: met; P19-AC-009-recompute: met; P19-AC-009-regrade: partially-met; P19-AC-009-regrade-reject: met; P19-AC-009-witness: not-assessable | P19-R-009: met |
| P19-AC-010 | P19-AC-010-credentials: met; P19-AC-010-bounds: not-met; P19-AC-010-cancel: partially-met; P19-AC-010-recovery: met; P19-AC-010-replacement: met; P19-AC-010-native-cleanup: not-assessable | P19-R-010: met |
| P19-AC-011 | P19-AC-011-default: met; P19-AC-011-ci: met; P19-AC-011-verifier: partially-met; P19-AC-011-provenance: not-assessable | P19-R-011: met |
| P19-AC-012 | P19-AC-012-six: not-assessable; P19-AC-012-freeze: not-assessable; P19-AC-012-raw: not-assessable; P19-AC-012-gates: not-assessable | P19-R-012: met |
| P19-AC-013 | P19-AC-013-placement: met; P19-AC-013-package: met; P19-AC-013-contracts: met; P19-AC-013-legacy: partially-met; P19-AC-013-rollback: met | P19-R-013: met |

## Current Finding Verification

Current evidence supports ten bounded behavioral closures. C008 remains partially confirmed: the actual-client matrix and real Docker cancellation witness execute/pass, while the four grader/regrade filters are blocked by a reproduced current Desktop storage-admission defect. Exact required inputs are materialized; the current namespace observer proves engine/root/process/mount identity and capacity. B8 restores the existing Desktop observation path for its second exact OS identity and requires preparation plus executed native proof after repair. C009 is partially confirmed: its raw six-trial absence is countered by current offline verification, while final-state/current-build applicability remains a Final Verification obligation. P001 is an Info navigation limitation with documented historical resolution. P012 met does not vote down C evidence gaps.

| Source run / Finding ID | Verification | Source/final severity and rationale | Closure key / family | Batch | Decision |
|---|---|---|---|---|---|
| C/P19-AUD-001 | Confirmed | Major/Major, unchanged; current rationale in batch/disposition | experiment.execution.integer-bound-types / execution.bound-admission | B2 | fix:experiment.execution.integer-bound-types |
| C/P19-AUD-002 | Confirmed | Major/Major, unchanged; current rationale in batch/disposition | runner.explicit-trials.max-trials / execution.bound-admission | B3 | fix:runner.explicit-trials.max-trials |
| C/P19-AUD-003 | Confirmed | Major/Major, unchanged; current rationale in batch/disposition | prepare.external-probes.bounded-supervision / preparation.observation | B4 | fix:prepare.external-probes.bounded-supervision |
| C/P19-AUD-004 | Confirmed | Major/Major, unchanged; current rationale in batch/disposition | report.zero-seal.frozen-topology-and-metrics / report.frozen-coverage | B5 | fix:report.zero-seal.frozen-topology-and-metrics |
| C/P19-AUD-005 | Confirmed | Major/Major, unchanged; current rationale in batch/disposition | regrade.output.canonical-source-exclusion / evidence.immutability | B6 | fix:regrade.output.canonical-source-exclusion |
| C/P19-AUD-006 | Confirmed | Major/Major, unchanged; current rationale in batch/disposition | native-capture.stderr.closed-diagnostic-grammar / native-capture.privacy | B1 | fix:native-capture.stderr.closed-diagnostic-grammar |
| C/P19-AUD-007 | Confirmed | Major/Major, unchanged; current rationale in batch/disposition | acceptance.valid-native-task-failure / acceptance.native-eligibility | B7 | fix:acceptance.valid-native-task-failure |
| C/P19-AUD-008 | Partially confirmed | Major/Major, unchanged; current rationale in batch/disposition | native-witness.current-executed-proof / assurance.native-evidence | B8 | fix:native-witness.desktop-storage-admission |
| C/P19-AUD-009 | Partially confirmed | Major/Major, unchanged; current rationale in batch/disposition | phase-exit.current-acceptance-and-gates / assurance.acceptance-evidence | none | no-action:Cannot confirm final-state Phase exit |
| C/P19-AUD-010 | Confirmed | Minor/Minor, unchanged; current rationale in batch/disposition | doctor.selected-materials.complete-observations / preparation.observation | B9 | fix:doctor.selected-materials.complete-observations |
| C/P19-AUD-011 | Confirmed | Minor/Minor, unchanged; current rationale in batch/disposition | pi.native-terminal.protocol-truncation / native-provider.classification | B10 | fix:pi.native-terminal.protocol-truncation |
| P/P19-AUD-001 | Confirmed | Info/Info, unchanged; current rationale in batch/disposition | evidence-registry.report-navigation / evidence.navigation | none | no-action:Info/No action documented historical pointers |

## Unresolved Decisions

none. The former operator-input gap has become an observed, bounded implementation decision: exact corpus/runner/client/helper inputs and Docker mount/runtime access are available, and the unchanged storage observer independently proves the selected engine namespace. Only the current OS-name gate rejects preparation. No criterion, authority or model/provider decision is waived. Pending green execution belongs to B8 and the final union, as it does for the other behavioral fixes; it is not represented as completed proof.

## Current native inputs and executed evidence

The six exact native_driver filters are native_tb21_artifact_witness, native_tb3_artifact_witness, native_deepswe_patch_witness, native_protocol_client_matrix, native_verifier_only_regrade_witness and native_owned_resource_cancellation. Two execute/pass at the source baseline. Four grader/regrade filters are not run because current public prepare refuses before publishing material. After B8 and all final causal edits, rebuild the current driver and execute all six with admitted exact inputs under owned-process/resource supervision. B1/B5/B10 change capture/report/Pi classifications and invalidate relevant baseline native applicability, so the final six-filter pass is required. Local matrix servers are scripted, not paid providers; ignored/fixture/build-only cases cannot count.

### Current input contracts and preconditions

The unchanged archived test binary builds with locked/offline Cargo: native_driver SHA-256 6caf85a0c714c00095698bd2171d00184bbcb09496789ea6cdab8a1b08989d3e, linked Companion SHA-256 fca95253fd44015a21431cdc7bf0f5539a35c1c20127ad259d632c3c378cd3b1. Build-only execution has zero native witness tests; no runtime qualification follows from it. Use the stable external Cargo cache with incremental compilation and leases.

| Exact filter | Required inputs | Executed predicate / qualification boundary |
|---|---|---|
| native_tb21_artifact_witness | OPI_EVAL_NATIVE_WITNESS_MATERIAL; real pinned Harbor/task material and Docker | Oracle/nop native results, identity-bound rewards 1/0 |
| native_tb3_artifact_witness | Same variable; real pinned Harbor/task material and Docker | Rewards 1/0 plus distinct collected /app/evalbench identities |
| native_deepswe_patch_witness | Same variable; real pinned Pier/task material and Docker | Rewards 1/0 plus distinct model.patch collection identities |
| native_protocol_client_matrix | OPI_EVAL_NATIVE_CLIENT_MATERIAL; JSON file paths opi, node, pi_bundle | Three APIs, two actual clients, five cases: 30 launches; loopback only, synthetic credentials |
| native_verifier_only_regrade_witness | All three prepared family profiles, static lock and actual runner material; Linux git/tar/ldd; Docker | Seven paired source/regrade scenarios and refusal/cleanup checks. Controlled tasks and diagnostic Agents qualify regrade mechanics, not actual Opi/pi execution |
| native_owned_resource_cancellation | No material/client variable; real Docker and exact cached Python image | Real owned container/process cleanup, capture-helper removal, unchanged image and surviving neighbor; staged runner is a stand-in |

Current material requires /7 and the capture-helper pin; legacy /6 may support its explicitly permitted offline reader paths. Current supplied runners require their exact project/venv/interpreter/entrypoint paths. The artifact/regrade tests invoke real runners that may build images; admit cached exact images/build contexts or enforced network restrictions before calling execution offline. Cancellation alone explicitly uses --pull never and --network none. Test failure cleanup is not universal: an outer supervisor must settle its owned client/listener/CLI process tree and exact-root containers, preserving shared images and neighbors.

Exact dependency-path inventory SHA-256 ad1ef9d709c782a53d719054dfb1f325d010e64e8b91c6c803da9d009baad1f9 confirms the accepted root's Opi executable and output-control records still match; its old Pi package, uv, runners, task/profile/corpus and static/provider pin paths are absent. No alternate historical evidence was sought. Current Pi acquisition uses only the current static producer/source/Node/model-data pins in private storage; its result is separate from the old acceptance root.

Current Pi acquisition passed with the unchanged pinned producer and one agents/pi acquisition entry: exit0, 48.965s, pinned source d981de1229ef899957bbe968bc8dcda02a21f477, Node22.19.0, bundle0.85.1, package SHA-256 f3cb211841f234875479fb26484c3bae949d5f22a6c376ecd0a6622fe3a346c6. Runtime remains at /home/luiz/.local/state/opi-p19-current-native/4d56f0f8-e393-47cb-8c69-53d99d96ed2e/attempt03/cache/agents/pi. Windows-backed extraction was cancelled with settled owned processes; a private npm configuration wrapper error was repaired before the successful Linux-native attempt. No producer/pin/dependency change occurred. A nonfatal locked dependency Node-engine warning is retained; standalone runtime smoke and the actual matrix pass under the pinned Node version.

The unchanged current native_driver binary then ran `native_protocol_client_matrix --exact --ignored --nocapture --test-threads=1`, with OPI_EVAL_NATIVE_CLIENT_MATERIAL bound to native-pi/attempt-03/matrix/clients.json. PASS: 1 executed, 0 failed, 0 ignored, 123 filtered; exit0, 268.916s. Independently checked 30 case execution records, six loopback request logs/75 requests, 12 tool-effect hashes, expected 21 exit0/9 Opi exit4 outcomes, all output limits512, unchanged driver/Companion/Opi/Node/bundle, 1199 cache file identities and 230 evidence file identities. No owned-session processes remain. Raw root is native-pi/attempt-03/matrix/tmp/.tmpk1HJaY under the external evidence directory. This proves the host actual-client matrix only; it does not qualify a Docker task runtime, native grader, regrade, capture-helper task preflight or fresh material/7 task dispatch. The matrix qualifies the baseline host clients. The remaining four grader/regrade filters require B8; the final repaired-source six-filter pass is required after the planned causal edits.

Initial Docker attempts failed: the Ubuntu socket was absent and a private no-network/no-pull readonly mount refused with exit125. Those receipts remain unchanged. Current backend logs identified Resource Saver shutdown and disabled Ubuntu integration. The settings file was backed up and changed only by adding IntegratedWslDistros=["Ubuntu"] (original SHA-256 8154e247613551819eccf46cc2fc045ceb48bbc8251260fb53814307abb5808a; applied candidate b54de1091febd854c21eb3679e160d67f944d23aa67408265e80b252683976ae). After confirming the only active container was the exact task-owned wake container, it was removed by ID and Desktop was restarted through its CLI. Ordinary Ubuntu /var/run/docker.sock access now succeeds. Two actual --pull never --network none readonly mount probes pass for Linux-native and Windows-backed paths, with exact Python image272efc... and engine f7640520-9a9f-4acb-bd58-62c5b388982a. No shared image was removed. Backup/change/engine/mount receipts are retained in docker-runtime-current.

The unchanged driver executed native_owned_resource_cancellation: PASS, 1 executed/0 failed/0 ignored/123 filtered, exit0, 8.076s. The test removed one exact-root owned container and one helper, preserved its neighbor and shared image, retained incomplete/job-cancelled/no-seal evidence and restored three private files byte-identically. Raw trace permissions0700/file0600 and UID1000 match. Independent read-only verification passed28 checks and rehashed199/199 files; outer process/container cleanup had nothing to settle. This witness stages the GNU Companion helper, not the separate musl helper, and qualifies cleanup/private-file restoration only. The cached binary embeds a parallel committed export's CARGO_MANIFEST_DIR: three used fixtures and seven relevant production/support/manifest/lock dependencies independently match current HEAD; that export's driver has only an appended independent B6 regression beyond the unchanged complete baseline. No source-path metadata substitutes for these byte checks.

Current unchanged producers acquired uv0.9.7, three exact corpus commits7131e4375048a0e408a8fb404b5f499d726b695b / 2b0442c3c583b710ca8da14c8e601b99f2f1f244 / 435ee89ec2f2e2289f33b0da4f992f0b7b7266b9 and locked Harbor4407eb5227a2ff4f0d3f16b2eb48849382fdf276 / Pierc1ebc6d145b40fae8425215e3fca528945065124 runners, all exit0/no owned residuals. Main independently verified57 raw records,12 committed seed files and8421 corpus/runner source files, Git-clean identities and installed interpreter/entrypoint hashes. The current helper was rebuilt locked/offline for x86_64-unknown-linux-musl in the stable shared Cargo cache with a lease and incremental compilation: SHA-256 919d9a6795a16a10bf7255a1996d3336654832a084fd752959a1e05808a0d9bb, Cargo.lock261a3ec501be2e045dc6c9b2b9a940adc1c56305f477008eea940f26545925f3, static ELF/no PT_INTERP. Verified Opi/Pi/helper copies are warm in /home/luiz/.local/state/opi-p19-current-native/0cce3262-2e03-4111-8611-5353e1435da4/cache. Existing actual clients and baseline helper receipts remain preserved.

Current public prepare for openssl-selfsigned-cert rejects at agent/verifier engine_storage_bytes unknown: exit2, 9.169s, no timeout/process residual or material.json. TB3/DeepSWE configs exist but no dependent prepare or grader/regrade filter was invoked. Docker29.7.2 reports OperatingSystem="Docker Desktop (containerized)"; prepare.rs desktop_storage admits only the exact "Docker Desktop" name and returns before the existing WSL namespace observer. Running the unmodified DESKTOP_STORAGE_SCRIPT read-only against this actual selected engine succeeds: exact engine/root, PID187/start67050/mnt:[4026532311], stable before/after identity, available1016229654528 bytes. A separate committed archive with only the added regression reproduces the production gate: cargo test -p opi-eval --lib --locked --offline desktop_containerized_storage_observer_binds_current_identity -- --nocapture; exit101, 0 passed/1 failed/172 filtered, observed None vs expected Some(10485760). Production bytes remain unchanged. These are B8's discriminating reds, not a claimed native grader pass.

For green, prepare each family to a fresh output using the reviewed current configs and final Companion/helper. runner/material.rs:326-466 admits and verifies the nonempty benchmark map; public prepare emits one verified family branch. A derived three-family witness manifest may copy those exact branches while preserving identical schema/origin/static lock/helper/Agents/provider script+endpoint, choosing the recorded TB2.1 provider request-log path. Record source output paths/hashes and the derivation, then run current validate for all three configs against the merged material before regrade. Do not invent profiles, pins or a prepare-success receipt. This is a derived admitted manifest, not an unchanged public prepare output. Rebuild/repin the final musl helper in fresh proof storage and the task-owned warm cache; retain the baseline helper raw artifact. All final predicates must use matching final source/build/material identities.

Ten batches below repair existing obligations. B8 is required by the reproduced current preparation blocker on the C008 causal path; no additional audit source or general platform refactor is introduced. Major batches precede Minor, with final native evidence collected after the last causal edit. Shared files/family keys do not imply shared root closure.

## Closure Batches

### Batch B1: Admit only closed whole stderr diagnostics

**Closure predicate**: Both policies persist zero synthetic private-marker bytes from unrecognized plaintext/structured stderr; exact summaries, empty stderr and proven safe structured diagnostics retain correct receipt digests.
**Dependencies**: none
**Verification union**: matching external probe; cargo test -p opi-eval --test native_driver --locked --offline; final union below.

#### Fix B1.1: Admit only closed whole stderr diagnostics

- **Finding source(s)**: `phase19-codex-gpt61-f60e458-20261003t041015z` + `37f779a15dad2250e807239cc4b4718256a117402fdba496fc3da8452e47c28a` + `P19-AUD-006`
- **Decision**: fix:native-capture.stderr.closed-diagnostic-grammar
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/native_capture_runtime.rs`, `crates/opi-eval/tests/native_driver.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Replace substring admission with bounded whole-record validation: exact plain summaries and pinned structured Opi diagnostics through closed fields, duplicate-key rejection, static message identity and proven safe cause grammar. Unknown/prefixed/suffixed/malformed records follow the existing opaque branch without retaining source text. Preserve safe structured records byte-identically and receipt /1 retained-byte hashes. Preserve the intentional Opi emitter; no Opi runtime edit.
- **Closure predicate**: Both policies persist zero synthetic private-marker bytes from unrecognized plaintext/structured stderr; exact summaries, empty stderr and proven safe structured diagnostics retain correct receipt digests.
- **Red-before**: `wsl -d Ubuntu -- bash /mnt/c/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/capture-protocol-fresh/run-probe.sh privacy`; FAIL: exit 1; 32 cases executed, 12 controls pass and 20 privacy cases fail; arbitrary bytes retained under sanitized/validated-diagnostic.
- **Green-after**: same discriminating check against rebuilt candidate/source and fresh output directory; PASS: exit 0, 32/32 cases; unknown records opaque, marker absent. Observed: not-run.
- **Current evidence / counter-evidence**: native_capture_runtime.rs:130-158 uses line.contains(token); :511-517 writes original bytes. Opi runner.rs:889-904 intentionally emits a structured second diagnostic, so token equality on every line would remove existing behavior. Thinking/opaque/overflow controls do not cover this bypass.
- **Test impact**: add closed-grammar privacy negatives; retain safe structured/thinking/opaque/overflow and downstream receipt rejection tests.

### Batch B2: Reject wrongly typed integer execution bounds

**Closure predicate**: All seven integer execution fields reject present non-integers before effects; supported integers and absent fields preserve their semantics.
**Dependencies**: none
**Verification union**: matching external probe; cargo test -p opi-eval --test experiment_contract --locked --offline; final union below.

#### Fix B2.1: Reject wrongly typed integer execution bounds

- **Finding source(s)**: `phase19-codex-gpt61-f60e458-20261003t041015z` + `37f779a15dad2250e807239cc4b4718256a117402fdba496fc3da8452e47c28a` + `P19-AUD-001`
- **Decision**: fix:experiment.execution.integer-bound-types
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/experiment.rs`, `crates/opi-eval/tests/experiment_contract.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Make optional_u64 return None only for absence and require a positive integer for each present execution limit. Reject all strings (including empty/omitted), booleans, floats, zero and negatives with the existing typed error. Retain numeric unenforceable-cap and output-control equality checks.
- **Closure predicate**: All seven integer execution fields reject present non-integers before effects; supported integers and absent fields preserve their semantics.
- **Red-before**: `python3 /mnt/c/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/boundary-probes.py --binary /mnt/c/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/opi-eval-baseline --source /mnt/c/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/source --case quoted-bounds --out /mnt/c/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/quoted-bounds-red`; FAIL: exit 1; 21 cases (seven fields times quoted 1/omitted/empty) all validate0 and erase the bound.
- **Green-after**: same discriminating check against rebuilt candidate/source and fresh output directory; PASS: all 21 wrong-type cases reject; numeric/omission controls retain their correct semantics. Observed: not-run.
- **Current evidence / counter-evidence**: experiment.rs:1419-1437 maps all String values to None; callers690-723 own seven limits. Existing numeric/nonpositive tests cannot discriminate this branch.
- **Test impact**: add wrong-type execution-field public validate cases; retain numeric/nonpositive/unenforceable-bound tests.

### Batch B3: Enforce caps on every resolved trial list

**Closure predicate**: A two-trial explicit/frozen request with max_trials=1 refuses before root/evidence creation or dispatch; equality to the cap executes exactly the plan.
**Dependencies**: B2; shared execution edits sequential
**Verification union**: matching external probe; cargo test -p opi-eval --test assembled_run --test native_driver --locked --offline; final union below.

#### Fix B3.1: Enforce caps on every resolved trial list

- **Finding source(s)**: `phase19-codex-gpt61-f60e458-20261003t041015z` + `37f779a15dad2250e807239cc4b4718256a117402fdba496fc3da8452e47c28a` + `P19-AUD-002`
- **Decision**: fix:runner.explicit-trials.max-trials
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/src/runner/plan.rs`, `crates/opi-eval/tests/assembled_run.rs`, `crates/opi-eval/tests/native_driver.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Check final resolved trial count on the shared plan/run admission path for explicit and expanded selections, before root reservation, native acquisition/preflight effects or dispatch. Include frozen-plan revalidation/replacement without widening limits; preserve selection resource refusal.
- **Closure predicate**: A two-trial explicit/frozen request with max_trials=1 refuses before root/evidence creation or dispatch; equality to the cap executes exactly the plan.
- **Red-before**: `python3 /mnt/c/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/boundary-probes.py --binary /mnt/c/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/opi-eval-baseline --source /mnt/c/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/source --case explicit-max-trials --out /mnt/c/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/explicit-max-trials-red-v2`; FAIL: exit 1; public hermetic run0/completed, two trials reported and two receipts sealed under max_trials=1.
- **Green-after**: same discriminating check against rebuilt candidate/source and fresh output directory; PASS: refusal nonzero, fresh root absent/no dispatch; equality/omission controls preserved. Observed: not-run.
- **Current evidence / counter-evidence**: plan.rs:445 caps selection expansion only; runner/experiment.rs:538-588 accepts None-selection without count admission. Current fresh two-seal reproduction confirms the public path.
- **Test impact**: add explicit/frozen/native no-effect cap cases; retain selection/equality/replacement coverage.

### Batch B4: Supervise every selected observer

**Closure predicate**: A stalled/overflowing observer cannot escape a finite operation bound; prepare polls cancellation and cleans only its owned observer tree, preserving caches/unrelated sentinel processes.
**Dependencies**: none
**Verification union**: matching external probe; cargo test -p opi-eval --test fail_closed_cli --test native_driver --locked --offline; cargo test -p opi-eval --lib --locked --offline; final union below.

#### Fix B4.1: Supervise every selected observer

- **Finding source(s)**: `phase19-codex-gpt61-f60e458-20261003t041015z` + `37f779a15dad2250e807239cc4b4718256a117402fdba496fc3da8452e47c28a` + `P19-AUD-003`
- **Decision**: fix:prepare.external-probes.bounded-supervision
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/runner/prepare.rs`, `crates/opi-eval/src/cli/prepare.rs`, `crates/opi-eval/tests/fail_closed_cli.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Route production engine/version/disk/context/GPU observations through existing ProcessSupervisor with finite deadlines, existing 16 KiB capture ceiling and verified owned-tree cleanup. Async prepare awaits supervised probes with its cancellation token instead of blocking the current-thread executor with raw output/thread join. Keep doctor facade synchronous but bounded through its private runtime/thread pattern; report truthful unknown/unreachable/refusal reasons. Respect remaining operation deadline with finite per-probe bounds; no new config/dependency/public seam.
- **Closure predicate**: A stalled/overflowing observer cannot escape a finite operation bound; prepare polls cancellation and cleans only its owned observer tree, preserving caches/unrelated sentinel processes.
- **Red-before**: `python3 /mnt/c/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/boundary-probes.py --binary /mnt/c/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/opi-eval-baseline --source /mnt/c/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/source --case doctor-stall --out /mnt/c/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/doctor-stall-red`; FAIL: exit 1; selected fake docker execs sleep60; no doctor output for 15.018s, then the outer watchdog kills only the isolated probe group.
- **Green-after**: same discriminating check against rebuilt candidate/source and fresh output directory; PASS: truthful bounded report inside the 15s watchdog; stalled version/context/disk/GPU, overflow and current-thread cancellation tests pass with no orphaned owned child. Observed: not-run.
- **Current evidence / counter-evidence**: prepare.rs:281,299,334,1369,1463,2637 use raw synchronous output; existing observation_command2299 has16 KiB/deadline/verified cleanup but these callers bypass it. The prepare facade uses a current-thread runtime.
- **Test impact**: add public stalled/overflow observers and async cancellation cases; retain selected-engine/resource/refusal/supervisor ownership tests.

### Batch B5: Freeze complete report identity before first intent

**Closure predicate**: Interruption before any seal retains declared edge/control pairs and terminal-bench/2.1 identity, planned 2/unexecuted 2, valid 0/denominator 0 and missing-side comparability; no invented grade/cross-protocol pair, and drifted frozen/sealed control refuses publication.
**Dependencies**: B3; freeze the same final admitted trial set
**Verification union**: matching external probe; cargo test -p opi-eval --test bundle_recompute --test report_contract --test report_output_containment --test native_driver --locked --offline; final union below.

#### Fix B5.1: Freeze complete report identity before first intent

- **Finding source(s)**: `phase19-codex-gpt61-f60e458-20261003t041015z` + `37f779a15dad2250e807239cc4b4718256a117402fdba496fc3da8452e47c28a` + `P19-AUD-004`
- **Decision**: fix:report.zero-seal.frozen-topology-and-metrics
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/src/report.rs`, `crates/opi-eval/tests/bundle_recompute.rs`, `crates/opi-eval/tests/report_output_containment.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Persist the existing resolved experiment and admitted integrity/report controls under byte-bound run-control/2 before first intent. Retain experiment/manifest/planned-trial fields; validate the complete frozen snapshot and cross-check all sealed control bytes. Restore topology/native benchmark/exclusions/frozen report controls even without seals. Synchronize current report/recovery readers, fixtures, bilingual docs and deliberate 0.x Unreleased entry. Retain owning read-only meanings of sealed /1 evidence; /1 zero-seal roots without topology stay legacy/incomplete. Unsealed diagnostics never supply scores or authoritative control.
- **Closure predicate**: Interruption before any seal retains declared edge/control pairs and terminal-bench/2.1 identity, planned 2/unexecuted 2, valid 0/denominator 0 and missing-side comparability; no invented grade/cross-protocol pair, and drifted frozen/sealed control refuses publication.
- **Red-before**: `python3 /mnt/c/Users/Luiz/AppData/Local/Temp/opi-p19-report-regrade-acceptance-2407cbfcbd7842cba81f1bf7bd861a7f/zero_seal_probe.py --source /mnt/c/Users/Luiz/AppData/Local/Temp/opi-p19-report-regrade-acceptance-2407cbfcbd7842cba81f1bf7bd861a7f/source --binary /mnt/c/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/opi-eval-baseline --proof /mnt/c/Users/Luiz/AppData/Local/Temp/opi-p19-report-regrade-acceptance-2407cbfcbd7842cba81f1bf7bd861a7f/zero-seal-proof-v2`; FAIL: exit 1; two cases. crash-after-intent run 70/report 0 has planned 2/unexecuted 2 but coverage=[], control_pairs=[], unknown/unknown metric; sealed run 0/report 0 retains edge-1 and terminal-bench/2.1.
- **Green-after**: same discriminating check against rebuilt candidate/source and fresh output directory; PASS: interrupted/sealed cases retain frozen benchmark/pair topology; interrupted valid count remains0, drift/mutation tests refuse. Observed: not-run.
- **Current evidence / counter-evidence**: runner/experiment.rs:702-729 omits complete contract; crash3052 precedes staged experiment3073, so no staged bytes can be promoted. report.rs:340 restores authoritative control only from seals. /1 readers report.rs:587 and runner/experiment.rs:4191 require synchronization. This restores AC008/CTRL005 using existing resolved contracts, without new scoring meaning.
- **Test impact**: add zero-seal native/full/subset/pair/drift cases; update run-control version fixtures; retain sealed/legacy evidence checks.

### Batch B6: Reject source-contained regrade output before effects

**Closure predicate**: Nested/missing-parent/symlink-alias output rejects before creation/dispatch; source directory digest and verify remain unchanged, while fresh external output remains valid.
**Dependencies**: none
**Verification union**: matching external probe; cargo test -p opi-eval --test native_driver --locked --offline; final union below.

#### Fix B6.1: Reject source-contained regrade output before effects

- **Finding source(s)**: `phase19-codex-gpt61-f60e458-20261003t041015z` + `37f779a15dad2250e807239cc4b4718256a117402fdba496fc3da8452e47c28a` + `P19-AUD-005`
- **Decision**: fix:regrade.output.canonical-source-exclusion
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/regrade.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Before mkdir/copy/verifier effects, resolve requested fresh output through canonical nearest existing ancestor and validated missing components, then reject containment in canonical source, including symlink aliases/absent parent chains. Use the existing report containment rule where appropriate, without a speculative public abstraction. Preserve source directory inventory/bytes on refusal; keep external fresh-output success.
- **Closure predicate**: Nested/missing-parent/symlink-alias output rejects before creation/dispatch; source directory digest and verify remain unchanged, while fresh external output remains valid.
- **Red-before**: `CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/opi-8abcf38f9871653e CARGO_INCREMENTAL=1 P19_REMEDIATION_PROOF=/mnt/c/Users/Luiz/AppData/Local/Temp/opi-p19-report-regrade-acceptance-2407cbfcbd7842cba81f1bf7bd861a7f/regrade-proof cargo test -p opi-eval --test native_driver --locked --offline regrade_output_containment_rejects_canonical_source_aliases -- --exact --nocapture`; FAIL: exit 101; one test executed/failed,123 filtered. Three stand-in branches (nested-existing-parent, nested-missing-parents, symlink-alias) regrade 0/create output/change source digest; subsequent verify 1 reports unmanifested-file/mutation-detected.
- **Green-after**: same discriminating check against rebuilt candidate/source and fresh output directory; PASS: the executed regression refuses all three cases, leaves out absent/source digest stable/verify 0; existing native_offline_regrade external-output cases pass. Observed: not-run.
- **Current evidence / counter-evidence**: regrade.rs canonicalizes source314 but only checks out.exists328; mkdir378/380 and task copy620-623 precede later checks. Regression uses production regrade and existing stand-in helpers: containment proof, not actual-native grader qualification.
- **Test impact**: add stored standalone containment regression to native_driver; retain external-output, missing-cache, tamper and no-Agent cases.

### Batch B7: Use admitted native outcome for acceptance eligibility

**Closure predicate**: Valid sealed native task failure/timeout (including timeout tail after completed loop) counts among six execution trials; provider/preflight/infra or missing-loop evidence never counts.
**Dependencies**: B1/B5; verification edits sequential
**Verification union**: matching external probe; python3 -m unittest discover -s crates/opi-eval/scripts -p 'test_verify_local_acceptance.py'; cargo test -p opi-eval --test native_driver --locked --offline; final union below.

#### Fix B7.1: Use admitted native outcome for acceptance eligibility

- **Finding source(s)**: `phase19-codex-gpt61-f60e458-20261003t041015z` + `37f779a15dad2250e807239cc4b4718256a117402fdba496fc3da8452e47c28a` + `P19-AUD-007`
- **Decision**: fix:acceptance.valid-native-task-failure
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/scripts/verify-local-acceptance.py`, `crates/opi-eval/scripts/test_verify_local_acceptance.py`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Follow byte-bound sealed native graded classification. Permit benchmark-valid Agent nonzero/task timeouts and matching native task exceptions, including a bounded timeout tail without final AgentEnd after an already completed real provider/tool loop. Preserve preflight, actual Agent/task/artifact binding, selected first connection, actual model/response and completed tool-loop requirements. Provider/auth/infra/setup/undispatched/invalid grades remain ineligible. Structural fixtures never become real acceptance proof.
- **Closure predicate**: Valid sealed native task failure/timeout (including timeout tail after completed loop) counts among six execution trials; provider/preflight/infra or missing-loop evidence never counts.
- **Red-before**: `python3 /mnt/c/Users/Luiz/AppData/Local/Temp/opi-p19-report-regrade-acceptance-2407cbfcbd7842cba81f1bf7bd861a7f/acceptance_probe.py --source /mnt/c/Users/Luiz/AppData/Local/Temp/opi-p19-report-regrade-acceptance-2407cbfcbd7842cba81f1bf7bd861a7f/source --proof /mnt/c/Users/Luiz/AppData/Local/Temp/opi-p19-report-regrade-acceptance-2407cbfcbd7842cba81f1bf7bd861a7f/acceptance-proof-v4`; FAIL: exit 1; six structural cases. success0, invalid-provider2 and invalid-preflight2 controls pass; graded-nonzero2, graded-timeout2 and graded-partial-timeout2 fail despite consistent graded settlements.
- **Green-after**: same discriminating check against rebuilt candidate/source and fresh output directory; PASS: six cases pass; eligible nonzero/timeouts accepted and invalid controls rejected. Verify the exact actual six-trial root separately with rebuilt Companion. Observed: not-run.
- **Current evidence / counter-evidence**: verify-local-acceptance.py:665-672 unconditionally requires exit 0/no timeout/no exception; runner/experiment.rs:1839-1847 and terminal-outcome fixtures already admit benchmark-valid task failures. Structural probe uses a recording Companion stub, not live/native evidence.
- **Test impact**: add/update valid nonzero/timeout/partial-tail fixtures; retain provider/preflight/infra/missing-loop negatives and sealed-outcome consistency.

### Batch B8: Admit the current exact Docker Desktop identity

**Closure predicate**: Both known exact Desktop OS identities reach the existing bound storage observer; unknown/near-match/nonlocal/drifted identities stay unknown. Current prepare publishes verified material/7 and all six required native predicates execute/pass with final identities.
**Dependencies**: none; final native evidence follows all causal edits.
**Verification union**: matching regression; existing Desktop namespace/timeout/selection tests; three actual public prepares and material validation; six exact native filters; final union below.

#### Fix B8.1: Preserve Desktop namespace binding for its current OS name

- **Finding source(s)**: `phase19-codex-gpt61-f60e458-20261003t041015z` + `37f779a15dad2250e807239cc4b4718256a117402fdba496fc3da8452e47c28a` + `P19-AUD-008`
- **Decision**: fix:native-witness.desktop-storage-admission
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-eval/src/runner/prepare.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Admit exactly "Docker Desktop" and "Docker Desktop (containerized)" at the existing Linux Desktop OS gate. Retain normalized local Unix endpoint, selected engine/root, unique dockerd process/start/mount, bounded secret-free WSL observation and exact before/after endpoint/engine/root/OS rechecks. Add the stored positive regression and near-match/unknown-name negatives; retain current namespace/process/malformed/timeout/remote/context-precedence cases. No substring matching, fallback capacity, wrapper, config switch, dependency or Docker runtime modification belongs to this repair. Existing current prepare/validate and native harnesses collect the green proofs.
- **Closure predicate**: Both known exact Desktop OS identities reach the existing bound storage observer; unknown/near-match/nonlocal/drifted identities stay unknown. Current prepare publishes verified material/7 and all six required native predicates execute/pass with final identities.
- **Red-before**: In the isolated source export with only desktop-label-red/regression.rs inserted: `CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/opi-8abcf38f9871653e CARGO_INCREMENTAL=1 cargo test -p opi-eval --lib --locked --offline desktop_containerized_storage_observer_binds_current_identity -- --nocapture`; FAIL exit101, 0 passed/1 failed/172 filtered, storage None instead of10485760. Current public prepare independently exits2 with engine_storage_bytes unknown, while the exact unchanged real namespace script verifies1016229654528 bytes.
- **Green-after**: Same stored regression against repaired source: PASS/one executed. Retain existing Desktop binding/timeout/refusal tests. With rebuilt final Companion/helper, run public prepare using the three exact configs under native-material-current/attempt-01/public-prepare/{terminal-bench-2.1,terminal-bench-3.0,deepswe-v1.1}/experiment.toml to fresh b8-green family outputs; validate their /7 and derived map as described above. Run the six explicit native_driver ignored/exact/nocapture filters with admitted matching final inputs and retained raw logs. Expected: all prepares0, all validations0, six tests executed/pass and no owned residuals. Observed: not-run.
- **Current evidence / counter-evidence**: The original finding is an executed-proof gap, not an allegation of this OS gate. Two current predicates now pass. The independent current prepare refusal and test-only red identify the minimal production blocker for the remaining four; stock real engine-ID/root/process/mount checks pass, so no operator/spec decision or manufactured material is needed to design the repair. Source Major is retained until required native proof executes.
- **Test impact**: add one positive and closed-name negatives in existing prepare.rs tests; retain all namespace/selection/timeout guards and six native harnesses. Refresh the final musl helper and record every actual source/build/client/material/native identity. No paid provider or real credential access.

### Batch B9: Report every selected preparation input

**Closure predicate**: A cache with only static lock and selected Agent files explicitly reports all remaining selected requirements missing; pi-only/invalid-config no-effect behavior remains.
**Dependencies**: B4; tool observations inherit supervision
**Verification union**: matching external probe; cargo test -p opi-eval --test fail_closed_cli --test native_driver --locked --offline; final union below.

#### Fix B9.1: Report every selected preparation input

- **Finding source(s)**: `phase19-codex-gpt61-f60e458-20261003t041015z` + `37f779a15dad2250e807239cc4b4718256a117402fdba496fc3da8452e47c28a` + `P19-AUD-010`
- **Decision**: fix:doctor.selected-materials.complete-observations
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/runner/prepare.rs`, `crates/opi-eval/tests/fail_closed_cli.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Derive doctor observations from the same selected product/benchmark/runner requirements as prepare: corpus/profile, uv, runner project/lock/interpreter/entrypoint, applicable integration/provider and capture helper. Emit expected locations/present/missing/unknown reasons without download/model/credential/Agent effects. Preserve pi-only selection; unselected product inputs are not required.
- **Closure predicate**: A cache with only static lock and selected Agent files explicitly reports all remaining selected requirements missing; pi-only/invalid-config no-effect behavior remains.
- **Red-before**: `python3 /mnt/c/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/boundary-probes.py --binary /mnt/c/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/opi-eval-baseline --source /mnt/c/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/source --case doctor-materials --out /mnt/c/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/doctor-materials-red`; FAIL: exit 1; doctor 0 reports only static lock/two present Agents; all seven corpus/profile/uv/runner/integration/provider/capture categories absent.
- **Green-after**: same discriminating check against rebuilt candidate/source and fresh output directory; PASS: every required selected category has actionable observation; selected/unselected-product/no-mutation controls pass. Observed: not-run.
- **Current evidence / counter-evidence**: doctor213-248 versus prepare1177-1221 and capture helper1757-1765. Existing empty-cache test asserts only three material IDs.
- **Test impact**: add partial-cache/pi-only selected-input cases; update empty-cache observations; retain invalid-config/no-mutation tests.

### Batch B10: Classify all pinned Pi protocol truncations

**Closure predicate**: All three exact messages import provider-protocol-error with failed outcomes at client exit 0; unknown/near-match/auth/rate/success retain classifications.
**Dependencies**: none
**Verification union**: matching external probe; cargo test -p opi-eval --test agent_integration_conformance --locked --offline; cargo test -p opi-eval --lib --locked --offline; final union below.

#### Fix B10.1: Classify all pinned Pi protocol truncations

- **Finding source(s)**: `phase19-codex-gpt61-f60e458-20261003t041015z` + `37f779a15dad2250e807239cc4b4718256a117402fdba496fc3da8452e47c28a` + `P19-AUD-011`
- **Decision**: fix:pi.native-terminal.protocol-truncation
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/agent/pi.rs`, `crates/opi-eval/tests/agent_integration_conformance.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Exact-match three pinned native truncation messages for Completions/Anthropic/Responses as provider-protocol-error. Preserve unknown/near-miss fallback and auth/rate/success classes; no substring inference.
- **Closure predicate**: All three exact messages import provider-protocol-error with failed outcomes at client exit 0; unknown/near-match/auth/rate/success retain classifications.
- **Red-before**: `wsl -d Ubuntu -- bash /mnt/c/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/capture-protocol-fresh/run-probe.sh protocol`; FAIL: exit 1; eight cases executed, six controls pass; Anthropic/Responses generic provider-failure, Completions correctly typed.
- **Green-after**: same discriminating check against rebuilt candidate/source and fresh output directory; PASS: eight/eight conformance cases with known exact strings typed and negative controls unchanged. Observed: not-run.
- **Current evidence / counter-evidence**: agent/pi.rs:339-348 only recognizes Stream ended without finish_reason; native_driver.rs:4133-4140 acknowledges the other two messages. Generic fallback remains failed/unscored, supporting unchanged Minor severity.
- **Test impact**: add three-protocol copied-fixture cases; update Pi coverage; retain unknown/auth/rate/success controls.

## Raw current evidence and advisory applicability

Beyond validated indexed appendices, only their exact named candidate raw proofs were used: acceptance manifest/root (and its contained trial/control/material/artifact records), and the current registry path. No history report was reconstructed. Candidate acceptance manifest SHA-256 9897f08fed7876b757542235e77e54735d233b1d6b0d82ec5024eab26fc6a9ce matches. Current exported offline verifier returned accepted=true/six attributable trials/three families; current Companion verify/report accepted each family. No Agent/provider/credential/new native trial was invoked. Opi product paths/manifests/lockfile have no committed diff from manifest Opi commit daaf2cfb540cb9890d930973cff725802a742a93 to remediation_head. This supports retained client applicability, not current Eval fresh dispatch/capture or final/native witness predicates.

| Current raw artifact | SHA-256 | Predicate / limitation |
|---|---|---|
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/source.tar` | `15baf817301e0ef5415f2b567d97e174787493b74dbf67c625373aff93992096` | Original committed evidence archive at 3e1301178c471499f0fc36d4a8b1cd1f467cef7d; preserved |
| `C:/Users/Luiz/AppData/Local/Temp/opi-all-changes-commit-607dca8922694fa5bdbea364a194a5df/source-after-research.tar` | `902b32772611c03a65d2b6d999118f1cc0e6fb6b18696ae1ded36a39f315f323` | Current remediation-head archive; only 16 research additions |
| `C:/Users/Luiz/AppData/Local/Temp/opi-all-changes-commit-607dca8922694fa5bdbea364a194a5df/baseline-applicability.json` | `7b1b0f2333ac5acea6cc78fa374641b7fa6c2d3eca9715a84841bf8bf0a7890d` | Independent 922-entry equality and registered source/index bindings PASS |
| `C:/Users/Luiz/AppData/Local/Temp/opi-all-changes-commit-607dca8922694fa5bdbea364a194a5df/archive-doc-check.json` | `67cef76fb7f53aae1c220168b5f5d3027222a0e7f7913a09bff4d64efd5aadfb` | Current committed export documentation contracts PASS |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/opi-eval-baseline` | `fca95253fd44015a21431cdc7bf0f5539a35c1c20127ad259d632c3c378cd3b1` | Baseline WSL/Linux executable |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/boundary-probes.py` | `99df833a481fa01f8404794e44b81f1d1e9bb2fc830091f3ef6f305d95239906` | Four public boundary harnesses |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/quoted-bounds-red/result.json` | `06dc77d9c8c413df608559ace1c41683697cf8e36686cc374b97c8c48436a58f` | 21 cases; FAIL |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/explicit-max-trials-red-v2/result.json` | `7964e41af7932a30484a5704ef4b65b8f5ba62a024f8426420acfc6b5171c609` | two sealed trials under max 1; FAIL |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/doctor-stall-red/result.json` | `89db2febbe475a502f4691923d52743c764d530d4af0a73f3cb838e7a9184a65` | 15.018s owned-watchdog; FAIL |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/doctor-materials-red/result.json` | `dd195de9d5bf3470c660cd0c0f87b89200b7ffc4e31b51c0f73aa2bfb703a016` | seven categories missing; FAIL |
| `C:/Users/Luiz/AppData/Local/Temp/opi-p19-report-regrade-acceptance-2407cbfcbd7842cba81f1bf7bd861a7f/zero_seal_probe.py` | `115d4c18a9f75d346a6f895b86ad2a7e647889a286fdc494be308bb04a35775f` | Interrupted/sealed harness |
| `C:/Users/Luiz/AppData/Local/Temp/opi-p19-report-regrade-acceptance-2407cbfcbd7842cba81f1bf7bd861a7f/zero-seal-proof-v2/results.json` | `ea013445ff4c71c78cb3e252adabeb299e0a49b97239b7d7b157235336e96b7b` | two cases; FAIL |
| `C:/Users/Luiz/AppData/Local/Temp/opi-p19-report-regrade-acceptance-2407cbfcbd7842cba81f1bf7bd861a7f/regrade_regression.rs` | `fc42fd967d446cb169b8770cc6e6516a65002179171434146669af0ab365d227` | Only added test in independent archive |
| `C:/Users/Luiz/AppData/Local/Temp/opi-p19-report-regrade-acceptance-2407cbfcbd7842cba81f1bf7bd861a7f/regrade-proof/regrade-results.json` | `e4d00a3db688b7b0379c918ade2db474c5b4f81aaf13a4d45a0ffc9c6bbc74e0` | three stand-in branches; FAIL, no native grader qualification |
| `C:/Users/Luiz/AppData/Local/Temp/opi-p19-report-regrade-acceptance-2407cbfcbd7842cba81f1bf7bd861a7f/acceptance_probe.py` | `0708227318bfbc1c5b987b23e8bf0d3a7615dea34f8175bd8d74fa075ec871ef` | Structural acceptance harness |
| `C:/Users/Luiz/AppData/Local/Temp/opi-p19-report-regrade-acceptance-2407cbfcbd7842cba81f1bf7bd861a7f/acceptance-proof-v4/results.json` | `28ace421b9ba756a265a66f14f4b6e1d4927bb9ceb55ab75a89b69191487e7f1` | six cases; FAIL, recording Companion stub |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/capture-protocol-fresh/p19-capture-protocol-probe.py` | `0a86ab81fc52b3a028359be7b1fbaa26d1c6b4e3c221cd6b85a7fb849a7d72c8` | Synthetic capture/copied Pi conformance harness |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/capture-protocol-fresh/raw/privacy-v4tu7187/results.json` | `7aa4bafe2043b277d599c5c2dd705279e7c6e9a69fcce1101cc3958b7be25960` | 32 cases; FAIL |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/capture-protocol-fresh/raw/protocol-adyzauwx/results.json` | `d4879c92ad55e31c216cdd09ce488a746b798db169c44374ec3c622d61d64be0` | eight cases; FAIL |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/acceptance-current.json` | `83e921d9c83bf4737bed0b959438d4789512ce789727d9546f2bb4a728db6c06` | six retained actual trials accepted offline, not fresh execution |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/native-input-inventory.json` | `ad1ef9d709c782a53d719054dfb1f325d010e64e8b91c6c803da9d009baad1f9` | exact allowed dependency paths/hash checks; old native inputs missing |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/native-driver-build-identity.json` | `41d0903792d341656143ab778ca47752a115da74431119861b305b0ad860bc76` | unchanged current test binary built; zero native filters executed by build |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/native-docker/mount-probe.json` | `bc8276f8ed5acbc294a2c575c84ddcc0efd9d20350047c9e3b895aa9fb4a1f80` | exit125 daemon bind-source refusal; exact owned-name cleanup, not native witness |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/native-docker/preconditions.json` | `37e51676c6bf8e891d9e376b95d8e19fa5e875a9eec612cdd28ee6039b0dd41c` | later proxy absent; native Docker filter count zero |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/native-pi/attempt-03/result.json` | `1c981c158200b37e4306e7814982c23554da49fec102c56c70704dc1bd54eeb5` | current pinned Pi acquisition exit0 |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/native-pi/attempt-03/input-identities.json` | `53f6899784b4667787140fdadd3b634f98dd5adf7c5ba8377da68d9bcc060145` | current static lock and unchanged producer pins |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/native-pi/attempt-03/retained-cache-identities.json` | `cf8f6c94d50389d06ccd1b62af1bd3c76eb1813e559698cfecf7704d6d15fa6e` | 1199 independently rehashed cache files |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/native-pi/attempt-03/matrix/result.json` | `4d0c6574672f649b73dca85f41dcdead743c6ecc78750c6e092a5d711762de98` | actual matrix PASS/1 executed/30 clients/no owned residuals |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/native-pi/attempt-03/matrix/retained-evidence-identities.json` | `3165b90ad5cf4e702ac782fcc2f2ddadd32e41de8a8d217498f3fb9175a1c0c3` | 230 independently rehashed evidence files |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/current-native-matrix-verification.json` | `fd1d5c0addace98ae04f7a5ce9be33a19befd4e59fdac57911808a1387174d17` | independent identity/count/exit/tool/request-limit verification PASS |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/docker-runtime-current/settings-change.json` | `78798a29d49630f4a02a03d8a47d054e705e257c012e67d2b5b3fdf46bfcb137` | Ubuntu-only integration repair; original/candidate settings backups retained |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/docker-runtime-current/preflight.json` | `59d027b7a4d51bbc479b7c05eb9e173c124f06ffae3a24da9efb3d71938996ad` | two cached no-network readonly mounts PASS; actual engine/image binding |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/docker-runtime-current/engine-storage-observation.json` | `a070221720e755a0866ac94b38977dcbb17f67b1e30b1a70c31a96fac6ec3724` | unmodified real namespace script PASS; current OS-name gate bypassed only for read-only diagnosis |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/native-helper-current/identity.json` | `b06d4a20802f94bc628f9e37eff0dd4035691b0306cceff7c74cd7240b74ad22` | unchanged current musl helper build/static ELF identity |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/native-material-current/attempt-01/handoff.json` | `d4281035fbe5d2d584d28422d2e3f0cd83484cc532deea2c70415e26089edb38` | six pinned no-model material acquisition stages PASS |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/native-material-current/attempt-01/record-hashes.json` | `3c264f2922b18e428f5b4ea7e6a08985c92e71f87a1fd44a413e0b5c383a4874` | 57 independently rehashed raw acquisition records |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/current-native-material-verification.json` | `a081bfdf85f3ef57ea543fd8c6ee0874a56d687ae4ec834efcebdf735f0e4ba1` | 57 raw/12 committed seeds/8421 source files/runner/helper identities PASS |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/native-material-current/attempt-01/public-prepare/inputs.json` | `e370f49977e7c64761f5416175cc55cf51c437153554fca02014f0baffdd7d0f` | verified exact client/helper copies and reviewed three configs |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/native-material-current/attempt-01/public-prepare/terminal-bench-2.1/result.json` | `a36ea972c9dbbc29b0fef8d3407d2868474507d7ff8fadab577a14d7f6409946` | current public prepare exit2; no material published/no residuals |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/desktop-label-red/regression.rs` | `e1fd57cee5898e1a6c6427f225a034107fa6b5d93c662595f149a3a2632143e9` | only added test in separate committed archive |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/desktop-label-red/result.json` | `e98afad29b3288bec32ca2b77863446750e988b955f5ee61572d9c820201bc1a` | one executed regression FAIL, production gate unchanged |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/native-witness-current/native_owned_resource_cancellation/result.json` | `52bdf2abb21b938af1d6c0658c34c321cf54423789366cfd5497ea65d68baadc` | native cleanup PASS/1 executed/no outer rescue/no residuals |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/native-witness-current/native_owned_resource_cancellation/retained-evidence-identities.json` | `c2bd01f01b68c0c50bf82b35929c775ab4e76fcfe26a90014f16a83dc6473389` | 199 independently rehashed retained files |
| `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-phase19-ta59potl/native-witness-current/native_owned_resource_cancellation/verification.json` | `fc5f1cc95a2f146703096e5cc461ea6ca5c697e71b74e6ba5ddb4499de65c84b` | independent28 checks PASS; exact embedded fixture/source dependencies checked |

External proof paths remain available for review. Green uses the concrete commands in the disposition sidecar: replace only capture-protocol-fresh/opi-eval-under-test with the rebuilt candidate, preserve baseline/proofs, use their fresh green output directories, and record rebuilt digest. B7 loads the repaired workspace source; B6 executes in the repaired source after adding its stored regression. B6 first adds the exact stored test to the existing binary; its red evidence is baseline production plus test-only instrumentation. Other probes use unchanged baseline production. Raw JSON/log siblings retain argv/stdout/stderr/executed counts.

## Final Verification

Changed surfaces select local Rust behavior, public CLI/report/durable-control/privacy seams and Python checker behavior. This union includes every modified test binary, exact script tests and crate clippy. C009 retains the registered final Phase workspace union as a final-state obligation. Run once after the last invalidating edit; ignored/fixture/preliminary checks do not establish native or Phase exit.

```text
step -> verify: rerun ten matching discriminating probes with rebuilt candidates, final source and fresh proof directories.
cargo test -p opi-eval --all-targets --locked --offline
cargo clippy -p opi-eval --all-targets --locked --offline -- -D warnings
python3 -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py'
python scripts/opi-doc-check.py
git diff --check
cargo fmt --check --all
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --all-targets --locked --offline
cargo test --workspace --doc --locked --offline
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked --offline
step -> verify: exact six-trial raw root with final Companion verify/report; independently check selected route/control/native/artifact/provenance/current-build applicability.
step -> verify: B8 current Desktop admission regression and three fresh public prepares/derived-map validations, then six exact final-build native predicates with admitted inputs/raw executed proof; current matrix/cleanup passes are baseline evidence and causal edits invalidate their final applicability; no paid provider or credential access.
```

B5 deliberately extends durable run-control to /2 while restoring existing report meaning; synchronize readers/tests, bilingual README and Unreleased. Other batches retain current schemas/public meaning. Do not add manifests/lockfile/Opi source/ledger edits. Additional input/source/authority decisions or a new protected blocker require a new sealed plan; no incidental scope is inferred from readiness.

## Exclusions

| Exact source key | Disposition | Current evidence / authority |
|---|---|---|
| C/P19-AUD-009 | Cannot confirm final-state Phase exit; partially confirmed overall | Exact retained six-trial proof passes current exported verifier/Companion. Final repaired-state gates and current native/fresh-dispatch/capture applicability remain unproved; B8 and the final union explicitly retain them. Final Verification retains the obligations; no runtime fix fabricated for missing proof. |
| P/P19-AUD-001 | Info/No action | Registry hash matches; two of three report pointers absent, with documented Git-history convention. Severity remains Info; raw-root acceptance is independently checked. |

## Planning verification and handoff boundary

Observable completion: active set -> verify: audit-set validator; strict-union findings -> verify: current source/counter-evidence and recorded red checks; fixed pair -> verify: dispositions/plan validators, doc checker and git diff --check. Planning writes only this workspace pair: 12 dispositions/ten proposed batches; exact matrix/cleanup/current inputs/preflight/namespace/red proofs are retained externally. Test impact now: add two regressions in separate disposable source exports; retain two existing executed native tests; none of the workspace tests is added/updated/deleted. Planned add/update/retain impacts appear per batch. User-authorized commit sequence: research `f039d682aec11365e381685186faad114f0b909f`, then this fixed plan pair only. Inventory before the plan commit: staged=[], carried-in=[], task-owned unstaged=[remediation.plan.md, remediation.plan.dispositions.jsonl], untracked=[]. The 45-byte unattributed nul diagnostic is preserved outside the repository at C:/Users/Luiz/AppData/Local/Temp/opi-all-changes-commit-607dca8922694fa5bdbea364a194a5df/nul.diagnostic.txt and excluded from project commits. Research document integrity was checked separately (1848 source file hashes, 13 package identities and 2065 local links; no runtime or exhaustive narrative-review claim). Only source-index.md and source-manifest.json needed CRLF-to-LF normalization; original document bytes and the normalization receipt remain in the external commit evidence directory. Commit-continuation test impact: none; no workspace test changes.

The owning protocol defines exactly this English Markdown/JSONL pair; no Chinese sibling filename exists, so none is created. Planned product README changes later synchronize both languages. No production repair, ledger mutation or Phase-conformant assertion is made; commits are authorized separately by the user.

READY-FOR-APPLY admits only the ten bounded repairs and their recorded green/final checks through a new explicit apply invocation containing the validator-emitted digest. It does not assert native/Phase conformance or turn the four unexecuted baseline filters into proof. If a required final native predicate still cannot execute or exposes a protected new defect, retain the gap and follow the owning apply protocol instead of widening this repair. Digest binding covers complete index bytes and both fixed files. A member/index/plan byte change invalidates it. Committing only this Phase plan pair preserves remediation_head under the shared source-baseline rule.
