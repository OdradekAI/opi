# Acceptance execution: Opi HEAD vs pi 0.85.1

[中文](20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live.zh.md)

**Acceptance handoff: blocked.** All six approved slots settled: four real Terminal-Bench trials produced attributable native grades; both DeepSWE slots failed during environment setup before either Agent started. Opi's TB 2.1 reward is 1; the other three graded trials have reward 0. No paid retry, source repair, commit, regrade, ledger update or Phase exit occurred.

## Evidence and authorization

- Private execution root: `/home/luiz/opi-eval-runs/20260910t154500z-offline-diagnosis-opi082/next-acceptance`.
- [Private evidence root](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live), 586 inventoried files. Inventory SHA-256: `988ebb7448b4affe05431df7e007693c6d124cbe89ea89cad570b791113ba66a`. This is a diagnostic inventory, **not an acceptance manifest**; no acceptance `manifest.json` was fabricated.
- [External checks](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live-checks/final-acceptance/verification-summary.json), [cleanup](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live-checks/cleanup-live.json), [machine trial summary](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live/reports/trial-summary.json), and [attempt history](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live/reports/attempt-history.json).
- Frozen [scope](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live/operator-scope.json) SHA-256: `dbe94817a77673b2673ba59b3c643d71d946d068a881875bb5e2a816144c5a6e`. [Authorization](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live/authorization.json) SHA-256: `e39590678f537f6c99fa6cb420c375af504b7e0ffa31011e5152cbce54ceadca` records the user's explicit approval of the new candidate and six trials.
- First launch `2026-09-10T16:26:53.972154+00:00`, last settlement `2026-09-10T16:51:00.716292+00:00`; observed span 1446.74 seconds, including between-family operator checks. All three families used independent native plans and serial execution.
- The original release evaluation and the [offline diagnosis/preview](20260910t154500z-opi082-head-pi0851-glm53flash-diagnosis-preview.md) remain unchanged. The new Opi executable is an **unreleased HEAD candidate, not tag v0.8.2**, despite its package version.

## Identities and frozen limits


| Component / 组件 | Source / 源码 | Executable SHA-256 / 可执行文件摘要 |
|---|---|---|
| Opi candidate, package 0.8.2, musl | `55fbc95009e6f47145196dbc539454ebcb1cf692` | `d7e5ab2d2636b04bfefafdeb2285bf50933247e7fc06836b6778d91ff4056e60` |
| pi 0.85.1, official Linux x64 | `d981de1229ef899957bbe968bc8dcda02a21f477` | `443bd83f30e4dbc7bac2eed9c6aa2461b9a15016fd555f48c92a0591d028c403` |
| opi-eval Companion, package 0.8.2 | `55fbc95009e6f47145196dbc539454ebcb1cf692` | `80da1b3b3fc83802035d77cf3df6e9e4e220f36e0f86ef3eacc1733c12c7c560` |

Tracked Opi source remained clean. pi runtime-package manifest SHA-256 is `0701993b1f4ddd9f7b4a6b2e9ecd963b3ac9457aca0807b71d4eb98339343aae`; Cargo.lock SHA-256 is `261a3ec501be2e045dc6c9b2b9a940adc1c56305f477008eea940f26545925f3`. Builds used Rust/Cargo 1.97.1. Harbor source is `4407eb5227a2ff4f0d3f16b2eb48849382fdf276`, Pier is `c1ebc6d145b40fae8425215e3fca528945065124`; repository-qualified commit/tree observations and provenance are preserved in [source identities](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live/builds/source-identities.json) and [upstream cache](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live/upstream-identity-cache.json). Exact binaries, full runner checkouts, runtime packages and shared caches remain at their original Linux paths.

The frozen connection is `zhipu:glm-5.3-flash`, `anthropic-messages`, `https://open.bigmodel.cn/api/anthropic`. The launch helper parsed only the selected `.env` variable `OPI_BIGMODEL_API_KEY` without interpolation or shell execution. pi completed-response metadata reports `glm-5.3-flash`; Opi records requested/resolved `zhipu:glm-5.3-flash`, but its trace labels actual wire identity `wire_unknown/not_reported`. The latter is retained as unknown, not inferred from the selector. Opi's recorded auth provenance resolves to the selected environment reference.

Output limit 8192 per response and client context declaration 131072 remain bound to the prepared loopback proofs. Temperature, top_p and reasoning are omitted. Context declaration is not a measured server-capacity guarantee. Scope is one pinned, locally admitted task per family, one repetition per subject, six slots, concurrency 1, no seeds. CPU/memory requirements are 1 CPU/2 GiB, 1 CPU/4 GiB and 2 CPU/8 GiB; no GPU. The upstream full corpora were not enumerated or evaluated by this acceptance subset.

Native job limits are 900/900/1800 seconds per family, including setup, Agent and grading; their six-slot sum is 7200 seconds. Native job stdout/stderr each cap at 1 MiB; detailed retained Agent logs are separate and may be larger. There is no total-token, monetary or total-wall-clock hard cap; acquisition/cleanup overhead is outside the native timeout sum. No timeout override, changed task image, model substitution or retry was introduced.


| Family / 家族 | Task / 任务 | Plan digest / 计划摘要 | Material SHA-256 / 材料摘要 |
|---|---|---|---|
| terminal-bench-2.1 | `openssl-selfsigned-cert` | `25edf422fd80ada6843311f0df0d7c530cb27f783677df34b9b5d0e20a993509` | `865548a1fc487b61ecaa923651190db247566b9e877fc32edd52c891c900bc93` |
| terminal-bench-3.0 | `batched-eval-parity` | `e47cbc4bd1c8d6d0b0641f14744ebb4c2f3c3252d0d9e13199095c97d95535d1` | `8798fc9e817c36a933de361472746b074b87a89e5ca7005fe2140ac6a4ad69bc` |
| deepswe-v1.1 | `abs-module-cache-flags` | `a616c7ed6222bb247a259145f932bda7e26524dcacc5c81f0773a6889913dec0` | `a2cf5cd83a577b76f1d070785e74cd9b47170ad1a0dbf8f7128d02faf42f169f` |

| Trial / 试验 | Actual native job / 实际原生作业 | Reward / 奖励 | Bundle manifest SHA-256 / 封存摘要 |
|---|---|---|---|
| `trial-baseline-pi-next-openssl-selfsigned-cert-r1` | `task-package__wB5a2Zr` | 0 | `8981cdeb9cf7959e2f7d208a4c4753985c2a4fc464892bad1dfe78e0904b9a80` |
| `trial-candidate-opi-head-openssl-selfsigned-cert-r1` | `task-package__hoHRdH8` | 1 | `ceff5c0729a8017f93dc83365f8c8c4ff65631386dff43f788c755d1a4323286` |
| `trial-baseline-pi-next-batched-eval-parity-r1` | `task-package__KqCHM39` | 0 | `56b8fd7ce70caea13cc5640bfb7ee773d3a8bbf1346b7579d88d83e616f3d729` |
| `trial-candidate-opi-head-batched-eval-parity-r1` | `task-package__qRK9VgR` | 0 | `3f17e5713befe9b52ab914c1425a98ab17cb0ed9dc23c929428951aefe74de1e` |
| `trial-baseline-pi-next-abs-module-cache-flags-r1` | `task-package__ockJrEn` | invalid | `unsealed` |
| `trial-candidate-opi-head-abs-module-cache-flags-r1` | `task-package__JSDwcs8` | invalid | `unsealed` |


## Native outcomes and task evidence

| Family | Agent | Native reward | Graded denominator | Planned/attempted | Valid/Agent failure | Invalid/unexecuted |
|---|---|---:|---:|---|---|---|
| TB 2.1 | Opi | 1 | 1 | 1/1 | 1/0 | 0/0 |
| TB 2.1 | pi | 0 | 1 | 1/1 | 1/1 | 0/0 |
| TB 3.0 | Opi | 0 | 1 | 1/1 | 1/1 | 0/0 |
| TB 3.0 | pi | 0 | 1 | 1/1 | 1/1 | 0/0 |
| DeepSWE 1.1 | Opi | not graded | 0 | 1/1 | 0/0 | 1/0 |
| DeepSWE 1.1 | pi | not graded | 0 | 1/1 | 0/0 | 1/0 |

All rows are subset scope; each family's admitted corpus/selected package count is 1/1. Native metric definition is grader reward over valid graded trials, retaining valid zeros and native-valid Agent timeouts in the denominator. Family denominators are 2, 2 and 0. No cross-benchmark average or statistical-superiority claim is made. The Companion's normalized `classification: conformance-evidence` string is retained verbatim; real execution semantics are reviewed from original Agent/native artifacts separately.

TB 2.1 used the actual shared task environment. Both Agents performed 5 bash calls and 1 write, and delivered `/app/ssl/server.key`, `.pem`, `.crt`, `verification.txt` and `/app/check_cert.py`; terminal receipts retain each digest. Opi passed all 6 native checks. pi passed 5/6: its verification script required `cryptography`, which was unavailable in the native test invocation. Both Agent exits were 0. Opi's previous release stream failure did not recur in this new trial.

TB 3.0 preserved `/app/evalbench/` and handed it to the separate verifier. Opi's collection digest is `f551524326e8b279a490f1b7d02b1c3fcaa212f1948b63bfa66bd4fa3d1ebbf9`; pi's is `23a7c76b14fd8e3dc7c5a83f01eb3dd67672df278541eb264268e4602777146b`. Opi performed 6 bash/18 read calls; pi performed 8 bash/17 read calls. Both exited 0 and failed all 5 native tests. pi's final response contains thinking only, with `stopReason=length` and output 8192. Opi's trace records success and no error diagnostic; that is an execution result, not task success.

DeepSWE's generated proxy Dockerfile still starts with `FROM ubuntu:24.04`. Both native `result.json` records report `RuntimeError`: anonymous-token retrieval from `auth.docker.io` timed out connecting to `69.63.176.15:443`. Setup lasted 31.22 and 31.93 seconds. Agent execution and verifier results are null; there is no committed patch, collected verifier input or sealed bundle. Both settlements are `invalid-execution`, boundary `integrity`, reason `native-binding-aggregate-invalid`. Earlier warm-cache proxy preparation did not remove this live registry dependency. See [EVAL-NEXT-001](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live/reports/findings.jsonl), an operator infrastructure finding with `status: unverified`, evaluator identity/independence unknown; it is not an admitted audit finding or an assertion of an Opi source defect.

## Usage and time


| Family / 家族 | Agent | Trial seconds / 试验秒数 | Completed provider turns / 已完成模型轮次 | Tool calls / 工具调用 | Input | Output | Cache read | Cache write |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| terminal-bench-2.1 | pi | 177.85 | 7 | 6 | 32964 | 5966 | 9088 | 0 |
| terminal-bench-2.1 | opi | 108.98 | 6 | 6 | 4570 | 2749 | 15296 | 0 |
| terminal-bench-3.0 | pi | 459.16 | 14 | 25 | 36784 | 12887 | 160896 | 0 |
| terminal-bench-3.0 | opi | 524.01 | 12 | 24 | 38949 | 13051 | 173312 | 0 |
| deepswe-v1.1 | pi | 31.63 | not started | 0 | not observed | not observed | not observed | not observed |
| deepswe-v1.1 | opi | 32.33 | not started | 0 | not observed | not observed | not observed | not observed |

Usage comes from pi completed assistant responses and Opi session/trace summaries; counts describe observed client/provider reports, not invoices. DeepSWE usage is not observed because Agent execution never began. pi's configured cost values are zero placeholders and do not mean free inference. Billed cost is unknown; transport retries/compaction are not exhaustively measured. Per-stage timestamps and usage origins remain in the machine summary. Four sealed native trial-result files match their original runner results byte-for-byte.

## Verification, privacy and cleanup

```sh
# Linux cwd: /home/luiz/opi-eval-runs/20260910t154500z-offline-diagnosis-opi082/next-acceptance
# f = terminal-bench-2.1, terminal-bench-3.0, deepswe-v1.1
./builds/opi-eval run --plan plans/$f.json --root runs/$f --fixtures unused-fixtures --canaries private/live-canaries.txt
./builds/opi-eval verify --root runs/$f
./builds/opi-eval report --root runs/$f --out reports/$f-a.json --canaries private/live-canaries.txt
./builds/opi-eval report --root runs/$f --out reports/$f-b.json --canaries private/live-canaries.txt
```

The exact absolute argv, cwd, start/end, exit and stdout/stderr are retained in [command records](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live/commands). The canary argument is a historical private reference; its temporary file has now been deleted. These roots must not be relaunched. Preparation doctor/prepare/validate/plan exits were all 0, as recorded before approval.


| Family / 家族 | run | verify original/staged/post-cleanup | report A/B/C/D/post-cleanup | Normalized report byte equality / 报告字节一致 |
|---|---:|---|---|---|
| terminal-bench-2.1 | 0 | 0 / 0 / 0 | all 0 | yes |
| terminal-bench-3.0 | 0 | 0 / 0 / 0 | all 0 | yes |
| deepswe-v1.1 | 1 | 1 / 1 / 1 | all 0 | yes; incomplete/invalid results retained |

DeepSWE `verify` reports `mutation-detected` with two `io` failures because the bundles were never sealed; this does not establish later tampering. Its reconstructed normalized report accurately preserves the incomplete run, invalid count 2 and denominator 0. Report comparisons use the original normalized reports; `run-report.json` has a different schema and is not expected to have identical bytes.

The exact [offline acceptance command](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live-checks/final-acceptance/acceptance.json) exited 2, `accepted:false`, because the qualifying six-row manifest is absent. Generic Opi audit exited 0 over the exact three native capture directories (7/7/3 files); the third is an unsealed failure fragment. A supplementary audit of the two actual Opi stdout files also exited 0. Both audits report empty recognized NDJSON/session lists, so these successes do not establish complete provider/tool semantics or acceptance. Actual traces, native handoff records and result bindings were reviewed separately. Native regrade was not requested or performed.

The selected-provider canary scan found no hits in any family or the complete staged selection. The retained pi TB 2.1 stdout contains task-generated TLS private-key material; operator paths, task-author metadata and runtime proxy configuration also remain. This evidence is **private/local-only and not approved for publication**. No sealed byte was redacted, rewritten or resealed.

[Cleanup](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live-checks/cleanup-live.json) completed: no containers or volumes remain; the temporary canary and four owned empty directories were removed. DeepSWE's original base-image reference was restored from the exact retained local image `sha256:3a4d47f5281269305343c83729836ac2f3172811aee72681e472a4196178eda1`, without a network fetch. Reusable images, aliases, original evidence, frozen binaries and shared Cargo/uv/BuildKit caches remain. Reclaimed bytes are unknown. The 549 protected live files, 586 staged files, 39 preview-protected files, 410 original-run files and previous 694/276-file staged inventories all matched after cleanup. Post-cleanup verification and report reconstruction retained the same outcomes.

Only this English/Chinese report pair was added to the repository. Test impact: **none**; no runtime/test source changed. `python3 scripts/opi-doc-check.py` and `git diff --check` final results are in [documentation checks](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live-checks/final-doc-checks.json). Workspace gates and Phase audit remain unverified here.

## Acceptance trace and next action

| Criterion | State for this run | Evidence / gap |
|---|---|---|
| P19-AC-001 | partial; blocked overall | Independent prepare/run/report works for TB; DeepSWE setup fails. |
| P19-AC-003 real portion | evidenced | Two shared-environment native grades and real tool/file effects. |
| P19-AC-004 real portion | evidenced | Two collected directories, separate-verifier records and native valid zeros. |
| P19-AC-005 real portion | missing | Neither DeepSWE Agent starts; no committed patch/pristine grading. |
| P19-AC-006 real portion | partial | Both clients execute the selected route in TB; DeepSWE absent; Opi actual wire model remains explicitly unknown. |
| P19-AC-012 | missing | Four qualifying real trials out of six required; two infrastructure-invalid slots. |

The remaining execution blocker is Docker/WSL access to Docker Hub authentication during the pinned proxy build. Resolve and verify that dependency offline before proposing a new, separately approved scope. Preserve both generations of failed DeepSWE attempts. No further paid execution is authorized by this report.
