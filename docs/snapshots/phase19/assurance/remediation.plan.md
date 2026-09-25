# Phase 19 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `702853fe37c1779c38612f7979a7e995395d1890954c4d3d8d6205592625e6d3`
**Remediation head**: `e834271d7ea6d114d67d9fc7e5298919fc7cb11e`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: committed=none-for-this-task; staged=none; unstaged=17 carried-in paths; untracked=none.
**Unresolved decisions**: none

## Admission and evidence boundary

The user's continuation accepts the immediately preceding proposal to replan Phase 19 and replace this ongoing task's fixed plan pair. It does not approve applying the new digest. The current 17 dirty paths were compared byte-for-byte with this task's retained opaque output copies; none had concurrent changes. The old plan and result pairs were copied opaquely before work. Only the plan pair is replaced; the result pair keeps its original approval identity and is not a result for this plan.

The revision-4 indexed set passed validation before semantic input. C is `phase19-codex-gpt6-d915e4a-20260918t084558z`, owning findings SHA-256 `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac`; P is `phase19-pi-glm53-ad67cb8-20260918t101920z`, owning findings SHA-256 `fa4d61c10efa6cb9d68146fab9f5f28eaf970393f941ba97119a1c68e5fd8301`. Each of the 11 indexed source keys is independently disposed; no peer is canonical. The aggregate remains FAIL. Current run/digest controls admission, title similarity cannot substitute for identity, and no older finding source is consulted.

All indexed metadata, findings and 85 requirements are copied under `$E/inputs/active`. `inputs/criterion-matrix.json` retains all original requirement records, grouped into 31 exact `(path, sha256, citation)` identities. No operator/eval artifact, prior remediation conclusion, history run or alternate plan is semantic input. Registered Phase meaning comes from the committed English Phase 19 source, with its registered Chinese counterpart preserved and parent constraints in `docs/opi-spec.md`; evidence does not revise that meaning.

`$E` is `/mnt/d/Luiz/Odradek/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967`; Windows: `D:\Luiz\Odradek\opi-remediation-evidence\phase19-replan-20260925T142217Z-7dced967`. `$S` is `/home/luiz/.local/state/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/source`, a unique `git archive` export of the exact remediation HEAD. Git modes are preserved on ext4, source mtimes were refreshed before compiling, and no worktree was created. Plan probes append only temporary test code inside the export and restore the original bytes. The real worktree's production files are not changed during planning.

Commands use a credential-free environment and the existing incremental cache `/mnt/e/opi/cargo-targets/opi-8abcf38f9871653e`. Exact argv, cwd, environment, source hashes, exit codes and log hashes are recorded in `$E/logs/*.json`. Pinned upstream Git sources, task packages, shrinkwrap/archive and image manifest bytes were independently verified as raw inputs. Both runner commits and the imported constructor code were checked against their Git blobs. Raw historical storage observations were separately rehashed, but the authoritative storage red below was newly reproduced against the current committed export and currently reachable selected engine.

## Dirty-worktree ownership

The baseline is worktree-only: committed=none-for-this-task; staged=none; untracked=none; unstaged=17 carried-in paths. Apply must compare the 15 non-plan hashes below before editing and preserve unrelated hunks; the two replaced plan files are bound by the new validator-emitted pair digest, not their superseded baseline hashes. The plan pair is carried-in-and-touched by this planning operation. Planning may not stage, commit, push, branch, edit the implementation ledger, change the audit set, or modify the opaque result pair. The fixed result pair remains an apply output, not a source repair.

| Path | Raw SHA-256 at admission | Planning ownership |
|---|---|---|
| `CHANGELOG.md` | `4a3693fba57189fb93b22c018186fd84eee601074a76af93ac12908684e02b60` | preserve carried-in source bytes |
| `crates/opi-eval/README.md` | `559dd7d1db4e22e9162690295e902558079fb3b1feb9c2cae3aee2efd52be8a3` | preserve carried-in source bytes |
| `crates/opi-eval/README.zh.md` | `f8c07282c659f864a4ac554e43ad9607949199ed6197ab71d49ec8a5f6173f35` | preserve carried-in source bytes |
| `crates/opi-eval/examples/local-evaluation/README.md` | `f1b20ef84313f6879e2ceb5f4134c6dc90d58f22e4befc6231803e9e7188974c` | preserve carried-in source bytes |
| `crates/opi-eval/external-locks/static/linux-x86_64.json` | `dd1b231306fd5ac4db97d3e9012bdcd0d1d241d98fb66c86c561b58dee1a5550` | preserve carried-in source bytes |
| `crates/opi-eval/scripts/build-agent-artifacts.sh` | `c9be8cb3e9afff89193253f68ce929226647d924a954ed889ed61283dce9f2ab` | preserve carried-in source bytes |
| `crates/opi-eval/scripts/materialize-external-locks.sh` | `f4ef0c23577223044f7860415e1966c23d2105a5f8b38590c013a593f861ce69` | preserve carried-in source bytes |
| `crates/opi-eval/scripts/native-smoke.sh` | `612c725dbd1336aaaf37c4ba34f83f1d046a3de8dec9a49b9f03d61bc9199f5a` | preserve carried-in source bytes |
| `crates/opi-eval/scripts/test_prepare_acquisition.py` | `b6d316ac9379bb258d55b60ed6956c4800449eed3aff637c02063789e94a28f8` | preserve carried-in source bytes |
| `crates/opi-eval/scripts/test_verify_native_smoke_ci.py` | `76656f4427646b74807959970d83f423a8851e5eea1ddb9b6c06f33f5ec4524a` | preserve carried-in source bytes |
| `crates/opi-eval/src/runner/material.rs` | `f48919c2dc1fa3e92213d7529021ac1e1957453c98903c0c68ad6529d7087379` | preserve carried-in source bytes |
| `crates/opi-eval/src/runner/prepare.rs` | `5c308a8f7367a08e71066c7590bddaa967dd01a45bad97fdaeb89b849cfe10f1` | preserve carried-in source bytes |
| `crates/opi-eval/tests/native_driver.rs` | `f95e21e1c06662e9aba788d29f8b1f4cc8f595c7c59fa9dbc837d4bd95b74727` | preserve carried-in source bytes |
| `docs/snapshots/phase19/assurance/remediation.plan.dispositions.jsonl` | `d2603a70af2c4ef3137b212672dd5267d16cf8c0e40613a14bc85aab6cdad4a6` | replace own fixed plan |
| `docs/snapshots/phase19/assurance/remediation.plan.md` | `dc5cf61ac9e692235b1f93e3441017d1f05cb9b2ea1910f1021f4d4947314555` | replace own fixed plan |
| `docs/snapshots/phase19/assurance/remediation.result.dispositions.jsonl` | `24d69e7720bff2ea0fc1e79df047ad1d5dad8ec1a744d43dcc6dcfd35f803068` | preserve opaque result |
| `docs/snapshots/phase19/assurance/remediation.result.md` | `8f10803a23825115451eace6e44bd0e11a922d4c1a4a75ff90d4fd4ae4e07189` | preserve opaque result |

Apply may complete the 13 carried-in production/docs/test paths in B1 and B2, add the specifically owned `crates/opi-eval/src/benchmark/process.rs` change for diagnostic dispatch/binding, and create B3's private test helper. This is an explicit expansion of the production repair surface for the currently reproduced oracle defect. It does not admit arbitrary changes to integrations, public schemas, dependencies, task packages, runner sources, resource limits or scoring semantics.

## Criterion matrix

The full path and digest are part of each identity, including similar citations that do not align. Reviewer states are preserved without consensus rewriting. The complete records remain in `inputs/criterion-matrix.json`.

| Criterion path | Source SHA-256 | Exact citation | Unmerged reviewer records |
|---|---|---|---|
| `docs/opi-spec.md` | `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db` | AUTH-004 | C:P19-AUTH-004=met |
| `docs/opi-spec.md` | `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db` | PHASE-001 | C:P19-PHASE-001=met |
| `docs/opi-spec.md` | `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db` | PRIN-002 | C:P19-PRIN-002=met |
| `docs/opi-spec.md` | `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db` | PRIN-004 | C:P19-PRIN-004=partially-met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | P19-AC-001; sections 3–10 | C:P19-AC-001-01=met; C:P19-AC-001-02=met; C:P19-AC-001-03=partially-met; C:P19-AC-001-04=partially-met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | P19-AC-002; sections 3–10 | C:P19-AC-002-01=met; C:P19-AC-002-02=met; C:P19-AC-002-03=met; C:P19-AC-002-04=met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | P19-AC-003; sections 3–10 | C:P19-AC-003-01=met; C:P19-AC-003-02=not-assessable; C:P19-AC-003-03=met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | P19-AC-004; sections 3–10 | C:P19-AC-004-01=met; C:P19-AC-004-02=met; C:P19-AC-004-03=not-assessable; C:P19-AC-004-04=met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | P19-AC-005; sections 3–10 | C:P19-AC-005-01=met; C:P19-AC-005-02=met; C:P19-AC-005-03=not-assessable; C:P19-AC-005-04=met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | P19-AC-006; sections 3–10 | C:P19-AC-006-01=met; C:P19-AC-006-02=partially-met; C:P19-AC-006-03=not-assessable; C:P19-AC-006-04=partially-met; C:P19-AC-006-05=met; C:P19-AC-006-06=met; C:P19-AC-006-07=met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | P19-AC-007; sections 3–10 | C:P19-AC-007-01=met; C:P19-AC-007-02=met; C:P19-AC-007-03=met; C:P19-AC-007-04=met; C:P19-AC-007-05=met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | P19-AC-008; sections 3–10 | C:P19-AC-008-01=met; C:P19-AC-008-02=met; C:P19-AC-008-03=met; C:P19-AC-008-04=met; C:P19-AC-008-05=met; C:P19-AC-008-06=partially-met; C:P19-AC-008-07=met; C:P19-AC-008-08=partially-met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | P19-AC-009; sections 3–10 | C:P19-AC-009-01=met; C:P19-AC-009-02=met; C:P19-AC-009-03=met; C:P19-AC-009-04=met; C:P19-AC-009-05=met; C:P19-AC-009-06=not-assessable |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | P19-AC-010; sections 3–10 | C:P19-AC-010-01=met; C:P19-AC-010-02=met; C:P19-AC-010-03=partially-met; C:P19-AC-010-04=partially-met; C:P19-AC-010-05=met; C:P19-AC-010-06=not-met; C:P19-AC-010-07=partially-met; C:P19-AC-010-08=not-assessable |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | P19-AC-011; sections 3–10 | C:P19-AC-011-01=met; C:P19-AC-011-02=partially-met; C:P19-AC-011-03=partially-met; C:P19-AC-011-04=met; C:P19-AC-011-05=partially-met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | P19-AC-012; sections 3–10 | C:P19-AC-012-01=met; C:P19-AC-012-02=met; C:P19-AC-012-03=partially-met; C:P19-AC-012-04=not-assessable |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | P19-AC-013; sections 3–10 | C:P19-AC-013-01=met; C:P19-AC-013-02=met; C:P19-AC-013-03=met; C:P19-AC-013-04=met; C:P19-AC-013-05=met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7, table row P19-AC-001 | P:P19-AC-001=met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7, table row P19-AC-002 | P:P19-AC-002=met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7, table row P19-AC-003 | P:P19-AC-003=partially-met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7, table row P19-AC-004 | P:P19-AC-004=partially-met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7, table row P19-AC-005 | P:P19-AC-005=partially-met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7, table row P19-AC-006 | P:P19-AC-006=met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7, table row P19-AC-007 | P:P19-AC-007=met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7, table row P19-AC-008 | P:P19-AC-008=met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7, table row P19-AC-009 | P:P19-AC-009=met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7, table row P19-AC-010 | P:P19-AC-010=met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7, table row P19-AC-011 | P:P19-AC-011=met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7, table row P19-AC-012 | P:P19-AC-012=met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7, table row P19-AC-013 | P:P19-AC-013=met |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 8, verification union | P:P19-EXIT-GATES=not-met |

## Current Finding Verification

| Source run / Finding ID | Verification | Source → final severity; current rationale | Closure key / family | Batch | Decision |
|---|---|---|---|---|---|
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-001 | Partially confirmed | Major → Major; Major retained: current production wires acquisition, refuting the blanket not-wired claim, but exact pinned pi archive, selected-engine storage, selected image binding/acquisition, relative package cache and pinned corpus metadata links each fail independent current probes. The default resource test also depends on ambient Docker. These block the declared preparation route. | `prepare.selected-pinned-materials` / `prepare.acquisition` | B1 | fix:selected-pinned-preparation-and-deterministic-checks |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-002 | Refuted | Major → Major; Source Major retained for scheduling; independent current evidence refutes the claim. Current committed acquisition uses the bounded supervisor; deadline/capture/failure settlement and cancellation cleanup tests passed in committed-tests. The original direct unbounded output claim is false at this HEAD. | `prepare.bounded-acquisition` / `prepare.acquisition` | exclusion | no-action:refuted-at-current-head |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-003 | Refuted | Major → Major; Source Major retained for scheduling; independent current evidence refutes the claim. Current required unsupported sampling rejects before dispatch; optional unsupported values remain recorded. Both exact tests passed in committed-tests. | `controls.required-sampling-admission` / `controls.capability` | exclusion | no-action:refuted-at-current-head |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-004 | Refuted | Major → Major; Source Major retained for scheduling; independent current evidence refutes the claim. Current offline reconstruction preserves unsupported effective output controls; the exact regression passed in committed-tests. | `controls.offline-unsupported-output` / `controls.capability` | exclusion | no-action:refuted-at-current-head |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-005 | Refuted | Major → Major; Source Major retained for scheduling; independent current evidence refutes the claim. Current recovery preserves every original file and planned trial, and rejects frozen-inventory identity drift; both regressions passed in committed-tests. | `recovery.immutable-original-inventory` / `run.durable-evidence` | exclusion | no-action:refuted-at-current-head |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-006 | Refuted | Major → Major; Source Major retained for scheduling; independent current evidence refutes the claim. Current fresh replacements preserve the original resolved scope and distinct identities; the regression passed in committed-tests. | `replacement.fresh-resolved-identities` / `run.durable-evidence` | exclusion | no-action:refuted-at-current-head |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-007 | Refuted | Major → Major; Source Major retained for scheduling; independent current evidence refutes the claim. Current privacy seal failure stops subsequent native dispatch; the exact two-subject regression passed in committed-tests. This does not replace B3 actual grader environment/canary inspection. | `privacy.stop-native-dispatch` / `run.authority` | exclusion | no-action:refuted-at-current-head |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-008 | Refuted | Major → Major; Source Major retained for scheduling; independent current evidence refutes the claim. Current acceptance rejects native-fixture/scripted substitutions; the exact Rust regression and all 42 acceptance-checker tests passed. No operator or eval artifacts were consulted. | `acceptance.reject-fixture-evidence` / `evidence.acceptance` | exclusion | no-action:refuted-at-current-head |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-009 | Partially confirmed | Major → Major; Major retained: current committed default union explicitly leaves nine native filters ignored and their implementation requires external materials; durable current protocol, lifecycle, deadline, privacy, regrade and assembled diagnostic observations remain to be constructed and retained. No conclusion is drawn about excluded historical live trials or Phase exit. | `native.current-durable-diagnostic-union` / `native.runtime-fidelity` | B3 | fix:retain-current-native-diagnostic-union |
| `phase19-pi-glm53-ad67cb8-20260918t101920z` / P19-AUD-001 | Refuted | Major → Major; Source Major retained for scheduling; independent current Git modes and focused execution refute the claim. At this HEAD both implicated scripts are committed mode 100755: scripted-provider.py blob cf8f7138bdfa8dd5ebdff82f7c63456c907896b4; fixture materialize-external-locks.sh blob 0c558b1b4a30adaf7666c67e3192e3581b876538. A fresh faithful ext4 archive passed agent_conformance_matrix_settles_every_pinned_case and supervised_acquisition_runs_the_pinned_materializer. The unrelated ambient-Docker test failure is B1, not a script-mode recurrence. | `checkout.executable-test-producers` / `checkout.portability` | exclusion | no-action:refuted-at-current-head |
| `phase19-pi-glm53-ad67cb8-20260918t101920z` / P19-AUD-002 | Partially confirmed | Major → Major; Major retained: the three official filters remain ignored in the current default union. Independent current Rust and exact pinned upstream constructor probes additionally show import-only oracle dispatch fails in both runner families, while explicit-name controls construct correctly; the current Opi reader rejects that valid name. Constructor observations do not prove native reward contrasts. | `native.explicit-diagnostic-reward-contrast` / `native.runtime-fidelity` | B2 | fix:explicit-diagnostic-dispatch-binding-and-official-contrasts |

The committed deterministic union produced 237 passing tests, one failure and nine ignored native tests: 147 library, 2 Agent conformance and 15 assembled tests passed; native-driver had 73 pass/1 fail/9 ignored. The failure is an ambient Docker dependency in the resource-preflight test's positive branch. Its targeted rerun passed after the engine became reachable; with an explicitly closed endpoint it failed again. That contrast is preserved, not relabelled a passing default gate. All eight refuted findings have passing focused observations in this current union, and the acceptance checker passed all 42 tests. Both scripts implicated by P/P19-AUD-001 have committed mode 100755 and their exact formerly failing tests passed on ext4.

## Unresolved Decisions

None. The diagnostic change follows the exact pinned runners' existing API and preserves registered native semantics. Runtime availability is an execution prerequisite, not a product-design choice. Docker was initially unreachable, then became reachable during this work; planning issued no engine-start, container-start or pull command against the real engine. The final storage probe bound engine `f7640520-9a9f-4acb-bd58-62c5b388982a` at `unix:///var/run/docker.sock`, `/var/lib/docker`. Apply must recheck the actual selected endpoint/resources and stop if unavailable or drifting; it may not substitute fabricated resource observations or start a stopped engine implicitly.

## Closure Batches

Execute B1 → B2 → B3. Sharing files does not merge the three distinct closure predicates. Each source retains Major severity and one batch/exclusion; no extra finding or incidental identity is manufactured.

### Batch B1: selected pinned preparation and its deterministic checks

**Finding source**: C / P19-AUD-001; full run/digest in the disposition.
**Verification status**: Partially confirmed.
**Closure predicate**: From declared pins and explicit committed Agent source, all three selected official tasks prepare, validate and plan with exact materials, bounded acquisition, truthful resources and stable warm-cache identity; malformed/untrusted material rejects before publication.
**Dependencies**: none.
**Change kind / paths**: behavioral; the 13 carried-in production/docs/test paths listed in this disposition.
**Verification union**: B1 script tests, material/prepare and native-driver tests, the exact probes below, and three official cold/warm CLI rows.
**Test impact**: retain carried-in coverage; update the existing resource-preflight test to make its positive and negative observations deterministic. Add no duplicate witness layer.

#### Fix B1.1: exact pi model archive

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z` / `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac` / P19-AUD-001
- **Decision**: `fix:selected-pinned-preparation-and-deterministic-checks`
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-eval/external-locks/static/linux-x86_64.json`, `crates/opi-eval/scripts/build-agent-artifacts.sh`, `crates/opi-eval/scripts/materialize-external-locks.sh`, `crates/opi-eval/scripts/test_prepare_acquisition.py`
- **Change kind**: behavioral
- **Closure predicate**: Exact pinned pi archive publishes; mismatches reject.
- **Red-before**: pi-archive-red exit 1; missing shrinkwrap integrity, no pi publication.
- **Green-after**: Same probe exit 0 against candidate; version/archive/hash negatives reject.

Retain the explicit `@earendil-works/pi-ai` version `0.85.1` archive URL `https://registry.npmjs.org/@earendil-works/pi-ai/-/pi-ai-0.85.1.tgz`, SHA-256 `af7d11986179445ce6fe88b37d57de22f823c0ffd3a65cae31c555b7f5e99253`, in the pi source pin. The pinned shrinkwrap SHA-256 is `5ff461d87e0a7ccac07e6a9eb3edcd18841d3373d375bed043606b51082f1a26`; its workspace dependency has no required SRI. The producer must consume the declared archive/version/hash without inventing integrity or changing the pinned source/dependency graph. Propagate only this admitted pin through the existing materializer/build path and its tests.

Red: `pi-archive-red` exits 1, pi is not published, and production reports missing shrinkwrap integrity. Green: the same exact-shrinkwrap fixture publishes verified pi; mismatched package/version/archive/hash rejects, and normal cold/warm producer checks pass. This fixture tests acquisition wiring; real client build identities are established by the official CLI rows.

#### Fix B1.2: selected Docker Desktop storage

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z` / `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac` / P19-AUD-001
- **Decision**: `fix:selected-pinned-preparation-and-deterministic-checks`
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-eval/src/runner/prepare.rs`, `crates/opi-eval/tests/native_driver.rs`
- **Change kind**: behavioral
- **Closure predicate**: Selected local engine storage is identity-bound and bounded; uncertain samples stay unknown.
- **Red-before**: storage-current-red exit 1 with current reference/doctor contrast.
- **Green-after**: Same probe exit 0 with wrong-ID/traversal rejection and unchanged inventories.

Retain the bounded read-only Desktop namespace probe only for the selected local Unix endpoint. Bind engine ID, Docker root, one dockerd PID/start time and mount namespace before/after the sample; reject malformed paths, ambiguous process matches, identity/endpoint drift, overflow and time/capture overruns. Do not start a distribution/engine or create a container for diagnosis. Unknown remains unknown when a required observation cannot be safely obtained; host/cache free space cannot replace engine storage.

Red: fresh `storage-current-red` exits 1: identity-bound reference capacity is 1016512094208 bytes, while the committed public doctor reports selected-engine storage unknown. Wrong-ID and traversal controls exit 17 and 16; image/container inventories are unchanged. Green: rerun `probe-storage.py` against the candidate on the rechecked selected engine; report a bounded known sample, preserve the negative cases/inventories, and retain unknown/unmet rejection coverage.

#### Fix B1.3: selected TB2.1 build image

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z` / `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac` / P19-AUD-001
- **Decision**: `fix:selected-pinned-preparation-and-deterministic-checks`
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-eval/external-locks/static/linux-x86_64.json`, `crates/opi-eval/scripts/test_prepare_acquisition.py`
- **Change kind**: behavioral
- **Closure predicate**: Official selected TB2.1 Python 3.13 build reference resolves to its exact immutable image pin.
- **Red-before**: image-bindings-red exit 101 for the missing official task image.
- **Green-after**: Same official binding probe exit 0; missing/mutable/mismatched pins reject.

Retain only the missing `tb21-python-base` build-input pin: `python:3.13-slim-bookworm@sha256:3e2de9c40ca4e3d73240059f9d48baff27908f10293e985a2f382a0378e6df4a`. Fresh hashing of the raw 1752-byte manifest binds config `sha256:915835de46a63f9598442159437904042f1c69a9b41c4965fba1db9ceab66578`. Other source, runner, package and image revisions remain pinned. This does not add a Cargo/native dependency.

Red: `image-bindings-red` exits 101 on the selected official Python 3.13 image reference. Green: the same official task-byte binding test passes with this exact pin and rejects missing/mutable/mismatched identities.

#### Fix B1.4: supervised selected-image acquisition

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z` / `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac` / P19-AUD-001
- **Decision**: `fix:selected-pinned-preparation-and-deterministic-checks`
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-eval/scripts/materialize-external-locks.sh`, `crates/opi-eval/scripts/native-smoke.sh`, `crates/opi-eval/scripts/test_prepare_acquisition.py`, `crates/opi-eval/scripts/test_verify_native_smoke_ci.py`
- **Change kind**: behavioral
- **Closure predicate**: Only selected missing immutable images are acquired and verified within supervision; warm is no-op.
- **Red-before**: image-acquisition-red exit 1; no Docker call, no publication.
- **Green-after**: Same recording probe exit 0 plus B1 official cold/warm material rows.

Retain acquisition of only missing images selected by the frozen task set, using immutable references and the existing supervised, bounded materializer. Share the acquisition deadline, keep output bounded, verify the resulting digest on the selected engine, and avoid pulling warm verified images. Preserve endpoint selection; acquisition must not use a different daemon or hide unknown completion. No unselected image prefetch or shared-cache deletion.

Red: `image-acquisition-red` exits 1, publishes no image and makes no Docker call through the recording stand-in. Green: the same missing-image case records the exact immutable pull/inspect sequence; mismatch/failure rejects, warm is no-op, and the official cold/warm rows verify real material acquisition.

#### Fix B1.5: lexical config/cache identities

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z` / `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac` / P19-AUD-001
- **Decision**: `fix:selected-pinned-preparation-and-deterministic-checks`
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-eval/src/runner/prepare.rs`, `crates/opi-eval/tests/native_driver.rs`
- **Change kind**: behavioral
- **Closure predicate**: Relative config/cache/package paths produce verified stable material identities.
- **Red-before**: replan_public_prepare_relative_package_cache exit 101, ENOENT.
- **Green-after**: Same public prepare test exit 0; existing drift and traversal controls remain passing.

Retain lexical absolute normalization of config/cache/package paths before source acquisition and material publication. Relative config/cache entries must produce the same verified package identities; do not relax package traversal, source-pin or runtime-entrypoint validation.

Red: `replan_public_prepare_relative_package_cache` exits 101 with ENOENT. Green: that exact test succeeds and validates the packaged pi material; existing invalid-entrypoint/package-drift checks remain passing.

#### Fix B1.6: inert pinned corpus metadata links

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z` / `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac` / P19-AUD-001
- **Decision**: `fix:selected-pinned-preparation-and-deterministic-checks`
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-eval/src/runner/material.rs`, `crates/opi-eval/src/runner/prepare.rs`, `crates/opi-eval/tests/native_driver.rs`, `CHANGELOG.md`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `crates/opi-eval/examples/local-evaluation/README.md`
- **Change kind**: behavioral
- **Closure predicate**: Only immediate pinned inert corpus metadata links are verified before hashing/inventory; unsafe links reject.
- **Red-before**: Official corpus and public pinned-metadata-link probes each exit 101.
- **Green-after**: Same two probes exit 0 plus carried-in positive/warm and full rejection matrix.

Retain the narrow immediate-`tasks/` metadata-link allowance. The raw relative link text must equal its pinned Git blob; the same-commit target must be a tracked regular file; every intermediate directory must be real, including components subsequently removed by `..`. Require the lexical path to stay inside the checkout. Reject absolute/ambiguous/control/backslash targets, escapes, cycles/chains, untracked or changed targets, symlink ancestors/root, nested selected or unselected task links, directory links, special files and submodules. This allowance never applies to executable task packages or runtime packages.

Hash the link text, separately verify target blob/mode, preserve the link-free corpus digest, and verify the complete pinned tree before generic hashing, root canonicalization or inventory publication. Enumerate the tree from the repository root so Git's implicit subdirectory prefix cannot erase expected entries. The pinned DeepSWE root metadata link is `tasks/README.md` → `../README.md`; this file is not a runnable task.

Red: official corpus verification and `replan_public_prepare_pinned_metadata_link` both exit 101 with link rejection. Green: those checks and the carried-in positive/warm-identity and rejection-matrix tests pass, including the linked-directory/`..` case. Preserve no-publication and outside-canary-not-read observations. Do not generalize to arbitrary task symlinks.

#### Fix B1.7: isolate the resource-preflight test

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z` / `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac` / P19-AUD-001
- **Decision**: `fix:selected-pinned-preparation-and-deterministic-checks`
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-eval/tests/native_driver.rs`
- **Change kind**: test-only
- **Closure predicate**: The default resource-preflight test verifies truthful sufficient/unknown/unmet observations independent of ambient Docker.
- **Red-before**: Exact test passes with real engine and fails under closed endpoint (exit 101).
- **Green-after**: Exact test passes under available and closed ambient endpoints using its isolated recording stand-in.

The existing `unmet_resource_requirement_rejects_without_silent_shrink` positive branch calls ambient Docker. Keep its CPU/memory/GPU requirements and assertions, but supply an isolated recording Docker stand-in with explicit sufficient resource observations through that child command's environment. Unknown/unmet controls remain explicit and do not dispatch or publish material. No host-global environment mutation, new production fallback, real image/container action, skipped assertion or weaker resource requirement is allowed.

Red: current committed default union fails with CPU/memory unknown; the exact test passes when real Docker returns, then fails again under `DOCKER_HOST=tcp://127.0.0.1:1` and empty `DOCKER_CONTEXT` (`resource-closed-endpoint-red`, exit 101). Green: the exact test passes with both unavailable and available ambient Docker while the test's own stand-in records all relevant observations. This test-only repair is directly required for B1's preparation verification union.

#### B1 official CLI rows and matching commands

Use fresh independent ext4 pin-only caches, both Opi and pi selected, explicit Opi source commit `e834271d7ea6d114d67d9fc7e5298919fc7cb11e`, and no model execution. Do not borrow completed material directories to call a row cold. Existing immutable engine images may be reused truthfully without deleting shared images. Capture doctor → prepare → validate → plan, then repeat prepare/validate/plan warm. Warm acquisition is empty and material identities stay equal except declared observation/output locations. Bound resources remain unchanged.

| Family | Official task | Corpus commit | Native runner | Agent/verifier storage |
|---|---|---|---|---|
| TB2.1 | openssl-selfsigned-cert | `7131e4375048a0e408a8fb404b5f499d726b695b` | Harbor `4407eb5227a2ff4f0d3f16b2eb48849382fdf276` | 10240/10240 MiB |
| TB3 | batched-eval-parity | `2b0442c3c583b710ca8da14c8e601b99f2f1f244` | same Harbor | 10240/10240 MiB |
| DeepSWE | abs-module-cache-flags | `435ee89ec2f2e2289f33b0da4f992f0b7b7266b9` | Pier `c1ebc6d145b40fae8425215e3fca528945065124` | 20480/20480 MiB |

The red-only orchestration wrappers intentionally assert failures. For green, replay the individual probes unchanged against the candidate in new output paths, expecting success, not those wrappers:

```sh
python3 $E/inputs/probes/probe-pi-pin.py <candidate-export> $E/inputs/pi-upstream <fresh-ext4-output>
python3 $E/inputs/probes/probe-storage.py <candidate-binary> <candidate-export> <fresh-output>
python3 $E/inputs/probes/probe-image-bindings.py <candidate-export> <fresh-copy-of-raw-inputs>
python3 $E/inputs/probes/probe-image-acquisition.py <candidate-export> <fresh-output>
python3 $E/inputs/probes/probe-corpus-prepare.py <candidate-export> <fresh-evidence-root-with-inputs-and-logs>
python3 crates/opi-eval/scripts/test_prepare_acquisition.py
python3 crates/opi-eval/scripts/test_verify_native_smoke_ci.py
cargo test -p opi-eval --lib --test native_driver -- --test-threads=4
DOCKER_HOST=tcp://127.0.0.1:1 DOCKER_CONTEXT= cargo test -p opi-eval --test native_driver unmet_resource_requirement_rejects_without_silent_shrink -- --exact --nocapture
```

### Batch B2: valid explicit diagnostics and official reward contrasts

**Finding source**: P / P19-AUD-002; full run/digest in the disposition.
**Verification status**: Partially confirmed; the constructor defect is now directly reproduced in both pinned runner families.
**Closure predicate**: Explicit oracle/nop diagnostics survive pinned runner construction and production identity binding, then all three official task pairs retain distinct artifacts/patches and native rewards 1/0 without becoming Agent scores.
**Dependencies**: B1.
**Change kind / paths**: behavioral; `crates/opi-eval/src/benchmark/process.rs`, `crates/opi-eval/tests/native_driver.rs`, `CHANGELOG.md`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`.
**Verification union**: exact diagnostic dispatch and reader checks, pinned upstream constructor check, the existing result-binding rejection matrix, and the three exact ignored official filters.
**Test impact**: add the missing dispatch/name-binding cases in existing owning test modules; update existing native binding/witness fixtures; retain unrelated tests.

#### Fix B2.1: named upstream diagnostic selection

- **Finding source(s)**: `phase19-pi-glm53-ad67cb8-20260918t101920z` / `fa4d61c10efa6cb9d68146fab9f5f28eaf970393f941ba97119a1c68e5fd8301` / P19-AUD-002
- **Decision**: `fix:explicit-diagnostic-dispatch-binding-and-official-contrasts`
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-eval/src/benchmark/process.rs`
- **Change kind**: behavioral
- **Closure predicate**: Explicit registered oracle/nop selection constructs both exact pinned native Agent classes.
- **Red-before**: Diagnostic dispatch Rust filter exit 101 and both real upstream oracle constructor probes exit 1.
- **Green-after**: Same generated-config assertions and pinned constructor probes exit 0; integration dispatch unchanged.

Use the existing `SelectionKind::DiagnosticWitness` branch to generate the exact upstream registered `name` (`oracle` or `nop`) with no custom import selector. Keep actual Opi/pi integration dispatch on its existing explicit import-path route. Preserve one-trial shape, model identifier, all Agent/verifier/whole-job caps, retry policy, lifecycle/environment choice and declared task identity. Reject an unsupported internal diagnostic selector rather than defaulting to an Agent. No new public enum, config field, CLI flag, adapter abstraction, schema or dependency is needed.

The pinned Harbor `Trial._init_agent` and Pier `TrialExecution._create_agent` inject `task_dir`, `trial_paths` and the task's Agent timeout only when the effective Agent name is `oracle`. The committed job config supplies only `import_path`. `inputs/diagnostic-job-configs.json` was generated by the actual committed Rust function; both pinned unmodified constructor routes fail with `OracleAgent.__init__() missing ... task_dir ... trial_paths`. Explicit-name oracle controls and both nop forms construct the correct class. These are constructor checks using isolated task/trace objects, not native grading or acceptance evidence.

Red: `benchmark::process::replan_diagnostic_probe::diagnostic_dispatch_uses_registered_native_names` exits 101 for all four generated family/witness selections; `upstream-harbor-diagnostic-red` and `upstream-pier-diagnostic-red` each exit 1 on their actual oracle invocation. Green: the same dispatch assertions pass, emitted configs instantiate both pinned oracle/nop classes, and no integration dispatch acquires a builtin diagnostic name.

#### Fix B2.2: kind-specific persisted selection binding

- **Finding source(s)**: `phase19-pi-glm53-ad67cb8-20260918t101920z` / `fa4d61c10efa6cb9d68146fab9f5f28eaf970393f941ba97119a1c68e5fd8301` / P19-AUD-002
- **Decision**: `fix:explicit-diagnostic-dispatch-binding-and-official-contrasts`
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-eval/src/benchmark/process.rs`, `CHANGELOG.md`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`
- **Change kind**: behavioral
- **Closure predicate**: Valid named diagnostics bind; defaults, dual selectors and integration substitutions reject.
- **Red-before**: Named diagnostic reader filter exit 101: AgentSelectionMismatch.
- **Green-after**: Same filter exit 0; existing substitution matrix and new wrong/missing/ambiguous selector controls reject.

In the production result reader, bind a diagnostic to its explicitly expected registered name (`oracle`/`nop`), requiring its custom import selector to be absent/null. Preserve the existing supported exact-import/name-as-import forms for real integrations. Do not accept a missing selector via upstream defaults, arbitrary builtin names, ambiguous dual selectors, wrong actual Agent, stale jobs, substituted tasks/artifacts, missing rewards or an oracle as a real Agent result. Preserve the diagnostic's lack of an integration terminal observation and its exclusion from Agent-score coverage.

Red: `benchmark::process::replan_diagnostic_probe::diagnostic_reader_accepts_explicit_native_name` exits 101 with `AgentSelectionMismatch { expected: "harbor.agents.oracle:OracleAgent", observed: "oracle" }`. Green: the same named diagnostic binds successfully while the existing `native_result_binding_rejects_every_substitution` matrix and new wrong/missing/ambiguous-name controls reject. Update only affected fixtures in the same owning modules. This changes diagnostic producer/reader behavior together, with Unreleased and bilingual documentation; no legacy Agent bundle migration or compatibility facade is introduced.

The probe test module is `$E/inputs/diagnostic-probe.rs`. Port its behavioral assertions into the existing owner or append it temporarily to a candidate export; run both exact Rust test filters expecting exit 0. Then run `$E/upstream-diagnostic-probe.py <harbor|pier> <verified-project> <verified-task> <new-generated-configs.json> <fresh-output>` with that family's pinned venv Python, expecting exit 0. The red-only `probe-diagnostic.py`/`verify-upstream.py` orchestration wrappers are not green gates. Never patch the pinned upstream runner or replace its constructor with a stand-in.

#### Fix B2.3: preserve actual reward evidence

- **Finding source(s)**: `phase19-pi-glm53-ad67cb8-20260918t101920z` / `fa4d61c10efa6cb9d68146fab9f5f28eaf970393f941ba97119a1c68e5fd8301` / P19-AUD-002
- **Decision**: `fix:explicit-diagnostic-dispatch-binding-and-official-contrasts`
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-eval/tests/native_driver.rs`
- **Change kind**: test-only
- **Closure predicate**: Three official pairs retain real native reward 1/0, input/collection/patch identities and raw artifacts.
- **Red-before**: N/A for retention changes: three official filters remain ignored; B2.1/B2.2 own the independently reproduced production blocker.
- **Green-after**: All three exact ignored official filters pass with B1 materials and durable real-grader raw rewards 1/0.

Retain and finish the carried-in improvements to the three existing official tests: collision-safe durable roots; CLI stdout/stderr and exit; config/material bytes and hashes; job/trial identities; collection tables; verifier-input binding; raw result hashes; and direct assertions of `verifier_result.rewards.reward` as oracle 1 and nop 0. A normalized `met` result alone is insufficient. Preserve TB3 `/app/evalbench/` collection/canary exclusion and DeepSWE committed `/logs/artifacts/model.patch`, base and pristine-verifier bindings. Do not modify any official task bytes, native grading or reward interpretation.

```sh
OPI_EVAL_NATIVE_WITNESS_MATERIAL=<B1-tb21-material> cargo test -p opi-eval --test native_driver native_tb21_artifact_witness -- --ignored --exact --nocapture --test-threads=1
OPI_EVAL_NATIVE_WITNESS_MATERIAL=<B1-tb3-material> cargo test -p opi-eval --test native_driver native_tb3_artifact_witness -- --ignored --exact --nocapture --test-threads=1
OPI_EVAL_NATIVE_WITNESS_MATERIAL=<B1-deepswe-material> cargo test -p opi-eval --test native_driver native_deepswe_patch_witness -- --ignored --exact --nocapture --test-threads=1
```

These six diagnostics make no model calls, establish grader sensitivity and remain separate from six real-provider trials. Constructor success or a deterministic stand-in cannot replace the required official native executions. Any missing/failed contrast leaves B2 open and blocks B3.

### Batch B3: retained current native diagnostics and final verification

**Finding source**: C / P19-AUD-009; full run/digest in the disposition.
**Verification status**: Partially confirmed.
**Closure predicate**: Current source and exact client/native materials have retained no-model evidence for protocols/controls, real cleanup/privacy, stage limits, fresh regrading and assembled local operations; final verification union passes.
**Dependencies**: B1 and B2.
**Change kind / paths**: test-only; `crates/opi-eval/tests/native_driver.rs` and new private `crates/opi-eval/tests/support/native_witness_material.rs`.
**Verification union**: six exact ignored filters, the independent assembled matrix, and final gates below.
**Test impact**: update existing filters and add only their shared private material builder; no separate public product entry.

#### Fix B3.1: retain and execute the current diagnostic union

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z` / `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac` / P19-AUD-009
- **Decision**: `fix:retain-current-native-diagnostic-union`
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/support/native_witness_material.rs`
- **Change kind**: test-only
- **Closure predicate**: Retain current-source no-model protocols, lifecycle/privacy, unchanged deadlines, actual fresh regrade and independent CLI evidence; all final gates pass.
- **Red-before**: N/A: test-only retention/material construction; nine native filters are ignored in the current committed union and require external materials.
- **Green-after**: The six exact B3 ignored filters, independent three-family CLI matrix and Final Verification union pass with retained raw evidence.

1. Retain protocol/turn-budget roots, request/response/config bytes, exits, tool effects and exact executable/runtime hashes. Use B1's real Opi/pi clients with loopback fake servers across Anthropic Messages, OpenAI Chat Completions and OpenAI Responses. Isolate configuration and synthetic credentials; read no user-global Agent/auth files. Existing client JSON carries `opi`, `node`, `pi_bundle`, optional package roots and the experiment needed by output-control verification; its output directory must be fresh.
2. Replace the owned-resource test's mutable busybox acquisition with admitted `python@sha256:272efc3f94bec901e2bc7a5c9984b3d113c317839edca2714c6a388b9476f8fc`, using `--pull never`. Retain owned-process/container cleanup, neighboring-container survival and unchanged shared-image identity. A controlled resource fixture alone does not prove actual Harbor/Pier lifecycle behavior.
3. Build independently pinned controlled Git/Python tasks outside official corpora, with fixture commits only in isolated test repositories. Reuse exact B1 Harbor/Pier sources/venvs, integrations and admitted images. The helper emits fully byte-bound diagnostic material/5 and synthetic no-model clients for the existing tests. Combine verified per-family base materials privately as needed; no new public API/config/CLI/schema/dependency or alternate ledger. Never label controlled fixtures official or acceptance evidence.
4. Construct `positive`, `time-cap`, `agent-cap`, `verifier-cap`, `both-agent`, `both-verifier`, `minimum-cap`, `strict-task` and `unbounded-task` for all families. Preserve existing bounds, including the Pier verifier setup allowance: 30-second run cap, 60-second task verifier bound, delayed verifier and its native retry. Do not raise any bound to pass. Observe real Agent/separate-verifier stages, positive controls, deadline outcomes, partial evidence, command settlement and neighboring-resource survival.
5. Construct positive/negative fresh regrade cases for all families and the existing TB2.1 timeout case. Preserve original bundle/input hashes, produce new derived grading records, and prove zero Agent/provider calls during regrade. Keep missing/tampered input, collection/base/network/material/cache and cleanup-uncertainty rejection cases. Deterministic report reconstruction cannot replace actual fresh native regrading.
6. Inspect the actual grader environment, collected files and Docker mounts/network for synthetic canaries, temporary credential removal and absence of broad-host/socket mounts. Searching only reports is insufficient. Stop on a privacy or authority failure and retain diagnostics.
7. From an independent directory, run `doctor → validate → plan → run --plan → verify → report → regrade` once per family with no models. Supply exactly one config/plan input to run; consume declared external material without repository fixture paths or GitHub execution metadata. Preserve CLI identities, exits/classifications, native results, deterministic reconstruction and immutable original hashes. This is assembled diagnostic evidence, not real-provider acceptance.

Red-before is not applicable to these test-only retention/material-construction changes: the current default union reports nine ignored filters and the source requires external materials; no production failure is invented to justify evidence construction. B2 owns the official contrasts and the newly observed production dispatch/binding defect; B3 does not duplicate that closure.

```sh
OPI_EVAL_NATIVE_CLIENT_MATERIAL=<verified-clients.json> cargo test -p opi-eval --test native_driver native_protocol_client_matrix -- --ignored --exact --nocapture --test-threads=1
OPI_EVAL_NATIVE_CLIENT_MATERIAL=<verified-clients.json> cargo test -p opi-eval --test native_driver native_client_turn_budgets -- --ignored --exact --nocapture --test-threads=1
OPI_EVAL_NATIVE_CLIENT_MATERIAL=<verified-clients.json> OPI_EVAL_OUTPUT_CONTROL_OUT=<fresh-root> cargo test -p opi-eval --test native_driver native_output_control_client_matrix -- --ignored --exact --nocapture --test-threads=1
cargo test -p opi-eval --test native_driver native_owned_resource_cancellation -- --ignored --exact --nocapture --test-threads=1
OPI_EVAL_NATIVE_WITNESS_MATERIAL=<builder-base-material> cargo test -p opi-eval --test native_driver remediation_native_stage_deadlines -- --ignored --exact --nocapture --test-threads=1
OPI_EVAL_NATIVE_WITNESS_MATERIAL=<builder-base-material> cargo test -p opi-eval --test native_driver native_verifier_only_regrade_witness -- --ignored --exact --nocapture --test-threads=1
```

No live provider run, operator-evidence reassessment, Phase exit or additional explicit-only workflow is authorized.

## Final Verification

Run the union for final relevant candidate bytes using the existing external incremental Cargo cache. Preserve source/tool/static-pin hashes, argv/environment projections, exits, raw logs and native roots. Reuse an unchanged check only when its exact relevant state and raw evidence have been independently verified; prior conclusions never count. Repeat checks invalidated by relevant changes. A skipped native row, increased bound, synthetic substitute or stale binary cannot count as PASS.

```sh
python3 crates/opi-eval/scripts/test_prepare_acquisition.py
python3 crates/opi-eval/scripts/test_verify_native_smoke_ci.py
python3 crates/opi-eval/scripts/test_verify_local_acceptance.py
cargo test -p opi-eval --lib --test native_driver --test assembled_run --test agent_integration_conformance -- --test-threads=4
python3 scripts/opi-doc-check.py
git diff --check
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

B1/B2 matching probes, the official matrix and B3 native/assembled rows are additional required evidence. Before applicable edits and batches, recheck exact HEAD, active index/membership bytes, approved plan digest and carried-in ownership. Follow the execution protocol's stop and incidental-repair guardrails; this plan does not pre-authorize unidentified production defects.

## Exclusions

| Source run / Finding ID | Disposition | Current evidence |
|---|---|---|
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-002 | Refuted; no repair | No repair proposed: Current committed acquisition uses the bounded supervisor; deadline/capture/failure settlement and cancellation cleanup tests passed in committed-tests. The original direct unbounded output claim is false at this HEAD. |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-003 | Refuted; no repair | No repair proposed: Current required unsupported sampling rejects before dispatch; optional unsupported values remain recorded. Both exact tests passed in committed-tests. |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-004 | Refuted; no repair | No repair proposed: Current offline reconstruction preserves unsupported effective output controls; the exact regression passed in committed-tests. |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-005 | Refuted; no repair | No repair proposed: Current recovery preserves every original file and planned trial, and rejects frozen-inventory identity drift; both regressions passed in committed-tests. |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-006 | Refuted; no repair | No repair proposed: Current fresh replacements preserve the original resolved scope and distinct identities; the regression passed in committed-tests. |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-007 | Refuted; no repair | No repair proposed: Current privacy seal failure stops subsequent native dispatch; the exact two-subject regression passed in committed-tests. This does not replace B3 actual grader environment/canary inspection. |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-008 | Refuted; no repair | No repair proposed: Current acceptance rejects native-fixture/scripted substitutions; the exact Rust regression and all 42 acceptance-checker tests passed. No operator or eval artifacts were consulted. |
| `phase19-pi-glm53-ad67cb8-20260918t101920z` / P19-AUD-001 | Refuted; no repair | No repair proposed. At this HEAD both implicated scripts are committed mode 100755: scripted-provider.py blob cf8f7138bdfa8dd5ebdff82f7c63456c907896b4; fixture materialize-external-locks.sh blob 0c558b1b4a30adaf7666c67e3192e3581b876538. A fresh faithful ext4 archive passed agent_conformance_matrix_settles_every_pinned_case and supervised_acquisition_runs_the_pinned_materializer. The unrelated ambient-Docker test failure is B1, not a script-mode recurrence. |

## Observed plan-stage checks

These are observations at the committed remediation HEAD, not apply results. Full command records and output bytes are under `$E/logs`; probe modules and independently checked upstream/task inputs are under `$E/inputs`.

| Raw log label | Executed command (exact cwd/environment in JSON) | Exit |
|---|---|---|
| `acceptance-checker` | `python3 crates/opi-eval/scripts/test_verify_local_acceptance.py` | 0 |
| `committed-tests` | `cargo test -p opi-eval --lib --test native_driver --test assembled_run --test agent_integration_conformance -- --test-threads=4` | 101 |
| `corpus-relative-red` | `python3 /mnt/d/Luiz/Odradek/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/inputs/probes/probe-corpus-prepare.py /home/luiz/.local/state/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/source /mnt/d/Luiz/Odradek/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967` | 1 |
| `diagnostic_dispatch_uses_registered_native_names` | `cargo test -p opi-eval --lib benchmark::process::replan_diagnostic_probe::diagnostic_dispatch_uses_registered_native_names -- --exact --nocapture` | 101 |
| `diagnostic_reader_accepts_explicit_native_name` | `cargo test -p opi-eval --lib benchmark::process::replan_diagnostic_probe::diagnostic_reader_accepts_explicit_native_name -- --exact --nocapture` | 101 |
| `image-acquisition-red` | `python3 /mnt/d/Luiz/Odradek/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/inputs/probes/probe-image-acquisition.py /home/luiz/.local/state/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/source /home/luiz/.local/state/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/image-acquisition-red` | 1 |
| `image-bindings-red` | `python3 /mnt/d/Luiz/Odradek/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/inputs/probes/probe-image-bindings.py /home/luiz/.local/state/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/source /mnt/d/Luiz/Odradek/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/inputs` | 101 |
| `pi-archive-red` | `python3 /mnt/d/Luiz/Odradek/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/inputs/probes/probe-pi-pin.py /home/luiz/.local/state/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/source /mnt/d/Luiz/Odradek/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/inputs/pi-upstream /home/luiz/.local/state/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/pi-archive-red` | 1 |
| `resource-closed-endpoint-red` | `DOCKER_HOST=tcp://127.0.0.1:1 DOCKER_CONTEXT= cargo test -p opi-eval --test native_driver unmet_resource_requirement_rejects_without_silent_shrink -- --exact --nocapture` | 101 |
| `resource-test-red` | `cargo test -p opi-eval --test native_driver unmet_resource_requirement_rejects_without_silent_shrink -- --exact --nocapture` | 0 |
| `storage-current-red` | `python3 /mnt/d/Luiz/Odradek/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/inputs/probes/probe-storage.py /mnt/e/opi/cargo-targets/opi-8abcf38f9871653e/debug/opi-eval /home/luiz/.local/state/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/source /mnt/d/Luiz/Odradek/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/storage-current-red` | 1 |
| `upstream-harbor-diagnostic-red` | `/home/luiz/.local/state/opi-remediation-evidence/phase19-apply-20260921T183107Z-1c972793/cold/terminal-bench-2.1-eb5478ec/cache/runners/harbor/.venv/bin/python -B /mnt/d/Luiz/Odradek/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/upstream-diagnostic-probe.py harbor /home/luiz/.local/state/opi-remediation-evidence/phase19-apply-20260921T183107Z-1c972793/cold/terminal-bench-2.1-eb5478ec/cache/runners/harbor /home/luiz/.local/state/opi-remediation-evidence/phase19-apply-20260921T183107Z-1c972793/cold/terminal-bench-2.1-eb5478ec/cache/corpora/terminal-bench-2.1/tasks/openssl-selfsigned-cert /mnt/d/Luiz/Odradek/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/inputs/diagnostic-job-configs.json /mnt/d/Luiz/Odradek/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/upstream-probes/harbor` | 1 |
| `upstream-pier-diagnostic-red` | `/home/luiz/.local/state/opi-remediation-evidence/phase19-apply-20260921T183107Z-1c972793/cold/deepswe-v1.1-dcb0e237/cache/runners/pier/.venv/bin/python -B /mnt/d/Luiz/Odradek/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/upstream-diagnostic-probe.py pier /home/luiz/.local/state/opi-remediation-evidence/phase19-apply-20260921T183107Z-1c972793/cold/deepswe-v1.1-dcb0e237/cache/runners/pier /home/luiz/.local/state/opi-remediation-evidence/phase19-apply-20260921T183107Z-1c972793/cold/deepswe-v1.1-dcb0e237/cache/corpora/deepswe-v1.1/tasks/abs-module-cache-flags /mnt/d/Luiz/Odradek/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/inputs/diagnostic-job-configs.json /mnt/d/Luiz/Odradek/opi-remediation-evidence/phase19-replan-20260925T142217Z-7dced967/upstream-probes/pier` | 1 |

## Approval and materialization boundary

Only the fixed plan pair is changed by this turn. Product changes, result replacement, B1/B2/B3 execution and closure require a new explicit apply invocation carrying the validator-emitted digest for this plan. The plan remains bound to the exact current live index digest. No commit is authorized. A later independent audit/re-run requires fixes plus the current live assurance set committed, assurance clean, external evidence resolved and any owning-workflow return complete. Neither plan readiness nor a later remediation result grants a Phase PASS.
