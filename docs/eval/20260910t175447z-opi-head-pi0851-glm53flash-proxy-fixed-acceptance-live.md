# Acceptance rerun: Opi HEAD and pi 0.85.1 after proxy repair

[简体中文](20260910t175447z-opi-head-pi0851-glm53flash-proxy-fixed-acceptance-live.zh.md)

**Execution: completed, 6/6 valid native trials. Evidence checks: verified. Formal acceptance handoff: blocked pending a redacted evidence handoff.** Both subjects earned reward 1 on TB 2.1 and reward 0 on TB 3.0 and DeepSWE 1.1. The four zero scores are valid task failures; there are no invalid or unexecuted slots. The earlier DeepSWE registry startup failure did not recur. Offline structural acceptance returned `accepted: true`, including after cleanup; the operator manifest still truthfully declares private, unredacted evidence. This report does not close a Phase or implementation task.

## Run and evidence identity

- Authoritative private Linux run: `/home/luiz/opi-eval-runs/20260910t175447z-proxy-fixed-acceptance`. This path and its pinned cache dependencies remain available for offline native inspection.
- [Staged evidence](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live) and [final manifest](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/manifest.json): 570 preserved files, excluding the manifest itself; SHA-256 **`e1604c1cfad0833f55a21e35df6fda2588969205236c39fcde892dc0603cef12`**.
- [Post-cleanup acceptance](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live-checks/acceptance-post-cleanup.json), [native trial summary](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/reports/trial-summary.json), [usage/timing/test detail](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/reports/metrics-detail.json), [external checks](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live-checks).
- [Scope](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/operator-scope.json) SHA-256 `47158dc87139b2631bb9714335efd201825046068ef7cae1ac4fa5599a9e8044`; [authorization](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/authorization.json) SHA-256 `7d7f3db46f14b429ea6c31567b1eec2da657652a90b460e6c643aa4edd56de4d`. The user's “复跑” authorized these six fresh slots under the already reviewed controls. Preview counters remain frozen; completion is recorded separately.

Execution began `2026-09-10T17:58:32.789263+00:00` and ended `2026-09-10T18:45:59.825502+00:00`: 2847.04 seconds (47 minutes 27 seconds), including between-family checks. Native trial durations sum to 2730.61 seconds. Execution order was DeepSWE, TB 2.1, TB 3.0, with pi then Opi in each pair. [Prior attempts](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/attempts/prior-attempts.json) retain the original release run, HEAD run and [proxy repair](20260910t170439z-deepswe-network-repair.md); this rerun replaces no earlier bytes or results.


| Component / 组件 | Source commit / 源码提交 | Executable SHA-256 / 可执行文件摘要 |
|---|---|---|
| Opi HEAD, package 0.8.2, musl | `55fbc95009e6f47145196dbc539454ebcb1cf692` | `d7e5ab2d2636b04bfefafdeb2285bf50933247e7fc06836b6778d91ff4056e60` |
| pi 0.85.1, official Linux x64 | `d981de1229ef899957bbe968bc8dcda02a21f477` | `443bd83f30e4dbc7bac2eed9c6aa2461b9a15016fd555f48c92a0591d028c403` |
| opi-eval Companion, package 0.8.2 | `55fbc95009e6f47145196dbc539454ebcb1cf692` | `80da1b3b3fc83802035d77cf3df6e9e4e220f36e0f86ef3eacc1733c12c7c560` |


The approved Opi subject is an **unreleased HEAD candidate with package version 0.8.2, not tag v0.8.2**. The clients and Companion were reused byte-for-byte; no rebuild or runtime source change occurred. The retained build observation records a clean tracked source at build time. The present worktree contains carried-in report/README changes and this report pair. pi package manifest SHA-256 is `0701993b1f4ddd9f7b4a6b2e9ecd963b3ac9457aca0807b71d4eb98339343aae`; Cargo.lock SHA-256 is `261a3ec501be2e045dc6c9b2b9a940adc1c56305f477008eea940f26545925f3`. The build record identifies rustc and Cargo 1.97.1, Linux x86_64.

[Qualified source identities](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/builds/source-identities.json) bind Opi to `https://github.com/OdradekAI/opi`, pi to `https://github.com/earendil-works/pi`, Harbor to `https://github.com/harbor-framework/harbor` at `4407eb5227a2ff4f0d3f16b2eb48849382fdf276`, and Pier to `https://github.com/datacurve-ai/pier` at `c1ebc6d145b40fae8425215e3fca528945065124`. Original observation dates were retained, not presented as new online observations. [Reuse checks](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/checks/reused-build-evidence.json) and preparation revalidated the exact retained bytes and runner pins.

WSL Ubuntu used Docker Desktop's Linux/amd64 engine. Doctor observed 32 CPUs, 16611237888 bytes of memory and approximately 950.7 GB available disk; GPU capability was present but not required. The three tasks require 1 CPU/2 GiB, 1 CPU/4 GiB and 2 CPU/8 GiB respectively. Frozen materials, native task locks, image/resource inventories and the before/after Docker image identities are retained with the [plans](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/plans), [materials](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/materials), [baseline](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/baseline.json) and [cleanup image inventory](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live-checks/cleanup-images.json).

## Frozen controls and scope

Both subjects requested `zhipu:glm-5.3-flash`, `anthropic-messages`, endpoint `https://open.bigmodel.cn/api/anthropic`. The authorized launcher selected `OPI_BIGMODEL_API_KEY` from the user's `.env`; neither the file nor the key value is included in this report. pi completed-response metadata reports `glm-5.3-flash`. Opi requested/resolved route matches, while actual wire model identity remains `wire_unknown/not_reported`; a selector is not proof of the server's model identity.

Required controls were `max_output_tokens=8192` and client `context_window=131072`, with retained exact-client output-control proofs. The context declaration is not a server capacity measurement. Temperature, top_p and reasoning were omitted. Each family selected one admitted local task, each subject repeated once, no seeds, concurrency 1 and `max_trials=2`: six total. The full upstream corpus was not enumerated. Full private captures were retained.

Native job caps were 900/900/1800 seconds per subject for TB 2.1/TB 3.0/DeepSWE, totaling 7200 seconds. They cover setup, Agent execution and grading; native stdout/stderr caps are each 1 MiB, while separate Agent captures may exceed that size. There is no guaranteed total token, billed-cost or overall wall-clock cap. No additional paid trial retry, control override, model substitution or native regrade occurred. Transport/SDK retries and compaction were not fully measured. Opi's existing 50-provider-turn limit was observed during DeepSWE; it was not newly imposed or edited during execution.

The preceding repair added only `HTTPS_PROXY=http://127.0.0.1:19828` to DeepSWE's explicit verifier/oracle environment maps. It carries no credential and preserves the Companion's deterministic environment isolation. Exact-environment public-registry prelaunch inspection passed. Initial material validation lacked output-control proof attachment and exited 1; byte-verified existing exact-client evidence was reattached, after which all three final validations and plans exited 0. The initial refusal remains in `commands/validate-derive-deepswe-v1.*`. Shared prepared-cache inventory was unchanged at SHA-256 `cbba9c15a338c696f9b816390cd66d9b72de43ecb391e0736b9cfe1e74819f2c`.


| Family / 家族 | Task / 任务 | Plan digest / 计划摘要 | Material SHA-256 / 材料摘要 |
|---|---|---|---|
| terminal-bench-2.1 | `openssl-selfsigned-cert` | `98edc79c1d9dc70f2b7d8d539e2fb59cf35a48077c1b319f5c1ecb4d97cd78e0` | `0255bc9c9b6efa299b735343e37fcdfb9d8ca33bd3681c9cf5db43e1c9a705ba` |
| terminal-bench-3.0 | `batched-eval-parity` | `d753efbe02f64e77b35789dc1222251b7891e9b66a686639949fd609f7e1b16c` | `a68fc184b63a377a4abac7957a8532f97c3f33ace6eb8fec1d96cc70cdc2cbac` |
| deepswe-v1.1 | `abs-module-cache-flags` | `4cba865a6d7dd37cf888cb1150526f4cd470dd09d6c9c4a642064b0d1028bb74` | `0e173791ad908eca5fdedbff22e96367621441217582d1499b216a831c3daf59` |


## Native outcomes and traceability


| Family / 家族 | Subject / 软件 | Scope | Native reward | Denominator | Corpus/selected | Planned/attempted | Valid/Agent failure | Invalid/unexecuted |
|---|---|---|---|---|---|---|---|---|
| terminal-bench-2.1 | pi 0.85.1 | subset | 1 | 1 | 1/1 | 1/1 | 1/0 | 0/0 |
| terminal-bench-2.1 | Opi HEAD 0.8.2 | subset | 1 | 1 | 1/1 | 1/1 | 1/0 | 0/0 |
| terminal-bench-3.0 | pi 0.85.1 | subset | 0 | 1 | 1/1 | 1/1 | 1/1 | 0/0 |
| terminal-bench-3.0 | Opi HEAD 0.8.2 | subset | 0 | 1 | 1/1 | 1/1 | 1/1 | 0/0 |
| deepswe-v1.1 | pi 0.85.1 | subset | 0 | 1 | 1/1 | 1/1 | 1/1 | 0/0 |
| deepswe-v1.1 | Opi HEAD 0.8.2 | subset | 0 | 1 | 1/1 | 1/1 | 1/1 | 0/0 |


The metric is the native grader's reward over valid graded trials. Each family has denominator 2; each row has denominator 1. Corpus/selected is the admitted local task package, not the complete upstream benchmark. Native failures remain in the denominator. There are no excluded comparison sides or missing repetitions. This six-trial subset supports neither a full benchmark result nor statistical superiority; no cross-family average is reported. The normalized report's literal `classification: conformance-evidence` is preserved, with real execution established separately from native Agent and handoff records.

Each linked trial below is one of the six selected rows and points directly to its sealed native result. The [machine summary](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/reports/trial-summary.json) additionally binds Agent stdout/trace, tool counts, original result equality, collected input, timing and verifier identity. Every sealed native result is byte-identical to its originating runner result. All six settlements are `graded-native`; all three Companion `run` commands exited 0, including the legitimately graded Opi nonzero exit.


| Trial (repetition 1) / 试验 | Native job / 原生作业 | Agent exit / 退出码 | Bundle manifest SHA-256 / 封存摘要 |
|---|---|---|---|
| [trial-baseline-pi-proxy-fixed-openssl-selfsigned-cert-r1](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/runs/terminal-bench-2.1/trials/trial-baseline-pi-proxy-fixed-openssl-selfsigned-cert-r1/bundle/artifacts/native/trial-result.json) | `task-package__fry8gNy` | 0 | `3fbeba0483119c395a17abdca63a29e3202d3716804e18b0d1ca602c04a192fc` |
| [trial-candidate-opi-head-proxy-fixed-openssl-selfsigned-cert-r1](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/runs/terminal-bench-2.1/trials/trial-candidate-opi-head-proxy-fixed-openssl-selfsigned-cert-r1/bundle/artifacts/native/trial-result.json) | `task-package__oqvWJdN` | 0 | `cbf76c034eeed4a2f96b9ff1b22a5d0f0a55c7100637c0e22ebd730800334c9b` |
| [trial-baseline-pi-proxy-fixed-batched-eval-parity-r1](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/runs/terminal-bench-3.0/trials/trial-baseline-pi-proxy-fixed-batched-eval-parity-r1/bundle/artifacts/native/trial-result.json) | `task-package__eQkSCgA` | 0 | `62a87c881d37de6f4fb1bcc7e98db302e87d46d0a76bfe9a88e5b36ebe79b0f7` |
| [trial-candidate-opi-head-proxy-fixed-batched-eval-parity-r1](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/runs/terminal-bench-3.0/trials/trial-candidate-opi-head-proxy-fixed-batched-eval-parity-r1/bundle/artifacts/native/trial-result.json) | `task-package__DuPKUFb` | 0 | `78e50cd100df22e6678039755632a9a8338537af923f71aef815c86b0103e710` |
| [trial-baseline-pi-proxy-fixed-abs-module-cache-flags-r1](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/runs/deepswe-v1.1/trials/trial-baseline-pi-proxy-fixed-abs-module-cache-flags-r1/bundle/artifacts/native/trial-result.json) | `task-package__qqhcYZp` | 0 | `c94fea4a6d5d0ff0dd759cd0451f44b4f1dd35ebab22725f4bd180b613526388` |
| [trial-candidate-opi-head-proxy-fixed-abs-module-cache-flags-r1](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/runs/deepswe-v1.1/trials/trial-candidate-opi-head-proxy-fixed-abs-module-cache-flags-r1/bundle/artifacts/native/trial-result.json) | `task-package__WpfaU3B` | 1 | `c0156c3cf2608f60debd301bade0025e5364866d56fcc00619787b02118142d3` |


TB 2.1: both Agents exited 0 and passed all 6 native tests in their actual shared task environments. pi executed 9 tools (7 bash, 1 write, 1 edit); Opi executed 5 bash tools. Certificate and verification artifacts were graded natively. The changed pi result versus the prior run is a new observation, not evidence that the DeepSWE proxy repair caused a TB improvement.

TB 3.0: both Agents exited 0 and passed 0/5 tests. Their declared `/app/evalbench/` directory was collected into `app/evalbench` for the separate verifier: pi SHA-256 `07f9891814dcfbd40a178e04084d6f6fbbb5f6bb2fcb849aaac4cfae612d24fd`, Opi `9fafdfb3805cde053c0b8794a9d63998aa8dc6f9656e65a14d42913b271935a7`. pi executed 24 tools (6 bash, 18 read), ending with a thinking-only response at `stopReason=length`, output 8192. Opi executed 16 bash tools; its session records one error diagnostic despite process exit 0. The specific Opi diagnostic cause was not investigated in this rerun. Directory digests alone do not establish which source edits occurred. [pi semantic inspection](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/checks/tb3-pi-semantic-review.json) and native test records preserve the observed boundary.

DeepSWE: both Agents ran and received attributable native reward 0, with 3/23 tests passed (`f2p=0/20`, `p2p=3/3`, native partial 0.13043478260869565). pi executed 33 tools (25 bash, 8 read), exited 0 and ended with thinking-only output at the 8192 response limit. Opi executed 71 tools (45 bash, 8 read, 2 write, 16 edit) across 50 provider turns, then exited 1 with `max turns exceeded (50)` / `NonZeroAgentExitCodeError`. This is a valid task failure, not an infrastructure failure or timeout. The literal Opi session `turns=0` coexists with `provider_turns=50`; it does not mean no model calls.

Both official committed patches were empty, SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`, bound to base `cb1b3b671d0ee9fa9da9f7b02f86967953ffd10a`. A pristine separate verifier with networking disabled consumed those actual patches using image `public.ecr.aws/d3j8x8q7/swe-bench-202605:kh75679ajj3b8dtd7se3h7z0a1833y6r-v1.1`. Uncommitted Opi task work was not reconstructed into a different patch or regraded. [pi](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/checks/deepswe-pi-semantic-review.json) and [Opi](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/checks/deepswe-opi-semantic-review.json) inspections preserve the source observations. Native scores remain authoritative; these inspections are operator diagnostics, not an independent model audit.

## Usage, time and cost


| Family / 家族 | Agent | Trial seconds | Agent seconds | Provider turns | Tool calls | Input | Output | Cache read | Cache write |
|---|---|---|---|---|---|---|---|---|---|
| terminal-bench-2.1 | pi | 127.40 | 97.70 | 8 | 9 | 16455 | 4205 | 17280 | 0 |
| terminal-bench-2.1 | opi | 265.91 | 238.01 | 6 | 5 | 33914 | 9797 | 19648 | 0 |
| terminal-bench-3.0 | pi | 301.89 | 262.98 | 10 | 24 | 29875 | 9275 | 86016 | 0 |
| terminal-bench-3.0 | opi | 381.52 | 343.26 | 10 | 16 | 30309 | 10445 | 97728 | 0 |
| deepswe-v1.1 | pi | 492.71 | 456.80 | 19 | 33 | 42100 | 19182 | 208192 | 0 |
| deepswe-v1.1 | opi | 1161.18 | 1126.08 | 50 | 71 | 62452 | 37716 | 1396160 | 0 |


Values come from pi completed assistant responses and Opi session/trace summaries; tool counts come from their actual event streams. [Metrics](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/reports/metrics-detail.json) preserve per-stage timing and native test paths/digests. These are reported token/cache observations, not billing or a cross-client efficiency conclusion. pi's configured zero cost values are placeholders; actual billed cost is unknown. Transport retries and compaction counts are unknown because this capture did not fully measure them.

## Verification and privacy

The commands below expand the exact recorded root, family and suffix values; [command records](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/commands) and [external checks](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live-checks) retain full absolute argv, cwd, times, exit codes and stdout/stderr. They are historical records: the canary file has been deleted and these used run roots must not be relaunched.

```sh
R=/home/luiz/opi-eval-runs/20260910t175447z-proxy-fixed-acceptance
E=/mnt/e/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live
C=/mnt/e/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live-checks
REPO=/mnt/d/Luiz/Odradek/opi
# cwd=$R; f=terminal-bench-2.1, terminal-bench-3.0, deepswe-v1.1
$R/builds/opi-eval doctor --config $R/inputs/$f.toml
$R/builds/opi-eval prepare --config $R/inputs/$f.toml --out $R/materials/$f
$R/builds/opi-eval validate --config $R/inputs/$f.toml --native-material $R/materials/$f/material.json
$R/builds/opi-eval plan --config $R/inputs/$f.toml --native-material $R/materials/$f/material.json --out $R/plans/$f.json
$R/builds/opi-eval run --plan $R/plans/$f.json --root $R/runs/$f --fixtures $R/unused-fixtures --canaries $R/private/live-canaries.txt
$R/builds/opi-eval verify --root $R/runs/$f
# s=a,b
$R/builds/opi-eval report --root $R/runs/$f --out $R/reports/$f-$s.json --canaries $R/private/live-canaries.txt
# cwd=$REPO
$R/builds/opi-eval verify --root $E/runs/$f
# s=c,d
$R/builds/opi-eval report --root $E/runs/$f --out $C/$f-$s.json
python3 $REPO/scripts/opi-artifact-audit.py $E/opi-artifact-audit --workspace-root $REPO --json
# o=acceptance.json, acceptance-rechecked.json, acceptance-post-cleanup.json
python3 $REPO/crates/opi-eval/scripts/verify-local-acceptance.py --root $E --out $C/$o --companion $R/builds/opi-eval
```

Doctor, prepare, final validate and plan each exited 0 for all three families. The first DeepSWE validation exit 1 is retained as described above. Original A/B reports and staged C/D reports rebuilt identically:


| Family / 家族 | run | verify original / staged | report A/B/C/D | A=B=C=D SHA-256 |
|---|---|---|---|---|
| terminal-bench-2.1 | 0 | 0 / 0 | all 0 | `3f6d87f2ab6a1bf8d3ea5a08362e19a361463d4bae8ef6311c4ddf4d3354ec40` |
| terminal-bench-3.0 | 0 | 0 / 0 | all 0 | `a7661edc32b3439709c7a70dc3a3365dc2563e3e6b8d46886b9177bf2d4befef` |
| deepswe-v1.1 | 0 | 0 / 0 | all 0 | `f53972e1e7a2fafc6dbe3b23c5ccb44e14b76b80227c49425566723e5d19f8f8` |


The first acceptance invocation exited 2 with `accepted:false`: the derived operator manifest had copied credential-reference metadata into the public connection object, so strict equality against the sealed experiment failed. Only that derived object's extra fields were removed after comparing all six sealed connections. The original rejected manifest and refusal remain in [correction evidence](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live-checks/manifest-connection-correction.json) and `manifest-before-connection-fix.json`; no native sealed bytes changed. The next invocation and the final post-cleanup invocation both exited 0 with `accepted:true`.

The Opi audit view includes exactly three complete native capture directories, seven files each (21 total), with directory membership and hashes checked against the originals. `opi-artifact-audit.py` exited 0, `ok:true`, issues empty. Its recognized `ndjson` and `sessions` arrays are empty: this checks the opaque native capture view, not full provider/tool/session semantics. Actual Agent event streams, task effects and native handoffs were inspected separately. It does not audit pi provenance or substitute for independent Phase verification.

Per-family canary review covered 110 DeepSWE, 91 TB 2.1 and 208 TB 3.0 files with zero selected-provider-credential hits; the staged selection was also scanned before cleanup. [Final privacy review](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/checks/privacy-final.json) records no private-key text markers and 137 files with operator path references. Task-generated material, author metadata and runtime proxy configuration remain unchanged. Thus `operator.redacted=false`, `public_shareable=false`; structural `accepted:true` does not mean the privacy-redacted handoff is ready. Offline acceptance checks did not reread the user's `.env`, and no regrade was requested or performed.

## Cleanup and repository verification

[Cleanup](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live-checks/cleanup-final.json) is completed: zero Docker containers and volumes, only default bridge/host/none networks, all 12 baseline image IDs retained, native processes exited. The temporary canary, its private directory, unused temporary directories, disposable operator HOME and three Docker Desktop client diagnostic logs were removed. Original task image tags were already present and needed no restoration. Frozen runner HOME dependencies, exact binaries, task materials, original evidence and shared Docker/Cargo caches remain. The user's `.env` remains unchanged. Reclaimed bytes are unknown because native cleanup and shared layers do not yield an attributable measurement.

All 418 protected current-run files and 570 staged preserved files matched after cleanup. [Prior-evidence checks](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live-checks/prior-evidence-unchanged.json) found zero mismatches in the earlier release, diagnosis, HEAD and repair evidence. The final offline acceptance still passes after cleanup. The original staging `inventory.json` is an earlier 568-file snapshot; the final 570-entry manifest is authoritative, and the earlier snapshot was not rewritten.

This request adds only this English/Chinese report pair. The preceding README edit and ten older untracked reports were preserved. No runtime/test source, ledger, audit archive, commit or staging changes were made. Test impact: **none**. Final `python3 scripts/opi-doc-check.py`, `git diff --check`, HEAD and worktree inventories are recorded in [final documentation checks](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live-checks/final-doc-checks.json). No Rust workspace gates were run for this documentation-only change.

## Acceptance trace and remaining work

| Criterion | Status | Evidence / remaining boundary |
|---|---|---|
| P19-AC-001, local execution/report path | evidenced | Three prepared family plans, six native runs and offline report reconstruction. |
| P19-AC-003, real portion | evidenced | TB 2.1 tool effects and actual shared-environment native grading. |
| P19-AC-004, real portion | evidenced | Two declared TB 3.0 directory collections and separate-verifier attribution. |
| P19-AC-005, real portion | evidenced | DeepSWE base-bound committed-patch collection and pristine grading; empty patch/valid zero preserved. |
| P19-AC-006, real portion | evidenced | Both selected clients used the frozen requested connection in all families; Opi actual wire identity remains explicitly unknown. |
| P19-AC-012, six native rows | evidenced | Six valid selected rows, original seals, command records and offline structural verification. |
| Formal redacted handoff / independent task 19.11 and Phase gates | missing | Private unredacted root; independent audit and workspace verification not performed. |

The [opi-eval acceptance handoff contract](../../.agents/skills/opi-eval/references/local-acceptance.md) states that the implementation harness “may only consume redacted evidence offline.” The next operator action is to prepare and validate that derivative handoff while retaining these original seals; then independently resume task 19.11 against the reviewed root and manifest digest. Broader/native witness and workspace gates remain unverified here. No further paid execution is authorized by this report.
