# 验收执行：Opi 0.8.2 与 pi 0.85.1

[English](20260910t104726z-opi082-pi0851-glm53flash-acceptance-live.md)

**验收交接：blocked；执行：incomplete，六个计划槽位均已结算。** 四次实际 Agent 运行获得原生评分并封存；两次 DeepSWE 在环境准备阶段失败，Agent 未启动。TB2.1 得分为 pi 1 / Opi 0，TB3 为 pi 0 / Opi 0，DeepSWE 无有效分数。本次每个基准仅选一项任务，不能作为完整基准或统计优劣结论。

本次人工语义复核中，两条 pi 试验具有合格的真实循环与原生结算；两条 Opi 因 provider 流失败排除，两条 DeepSWE 因基础设施失败排除。Companion 仍保留四条有效原生评分；acceptance 的语义准入另行判断。没有追加试验、源码修复、重新评分、提交、实现台账更新或 Phase 完成声明。

## 证据与精确身份

- WSL 私有运行目录 `R`：`/home/luiz/opi-eval-runs/20260910t100917z-acceptance-opi082-pi0851`。
- 本地诊断副本 `E`：`E:\opi\evaluations\20260910t104726z-opi082-pi0851-glm53flash-acceptance-live`；[inventory.json](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/inventory.json) SHA-256 为 `dfde6308817c65cfc95ce672fd331a1a294d72b0fbcd30daa34e062bf3d4e87a`，共 694 个逐字节登记的文件。
- 独立检查目录 `C`：`E:\opi\evaluations\20260910t104726z-opi082-pi0851-glm53flash-acceptance-live-checks`；[命令记录](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live-checks/commands.json)、[报告一致性](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live-checks/report-comparisons.json)、[清理结果](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live-checks/cleanup/cleanup.json)。
- 未生成 acceptance `manifest.json`：六条合格试验并不存在。`inventory.json` 是私有诊断文件清单，不是验收清单。
- [冻结范围](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/operator-scope.json) SHA-256：`16f9a80798479e574f5f4e9b4c2278940b3dcd2ecd3e18811c0f2fd82559d553`。旧预览保持原样；[授权记录](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/authorization.json) 将 2026-09-10T10:46:31.965304Z 收到的 `go` 绑定至该范围。

源码身份如下，四份检出均为 clean：

- opi: `https://github.com/OdradekAI/opi`, commit `765608a55a78db47e8b41d2639c29ef8c90e0f31`, tree `a12e0b6877d2217f7c21a3b1102c8309d90dae7d`, clean.
- pi: `https://github.com/earendil-works/pi`, commit `d981de1229ef899957bbe968bc8dcda02a21f477`, tree `346294a615d2d0ad4f6e5fbccb4cee4ccd7b2d6c`, clean.
- harbor: `https://github.com/harbor-framework/harbor`, commit `4407eb5227a2ff4f0d3f16b2eb48849382fdf276`, tree `cc4e705e7d439332ba8562f8f9c9707630bb73f7`, clean.
- pier: `https://github.com/datacurve-ai/pier`, commit `c1ebc6d145b40fae8425215e3fca528945065124`, tree `b8cae7a6624893d5757f6c0d0e302ef982a390ca`, clean.
- opi executable SHA-256: `f1bd9bbf82d03120427e4de42318f31e939c0a94dde99705f4fdde9eabf063ee`.
- pi executable SHA-256: `443bd83f30e4dbc7bac2eed9c6aa2461b9a15016fd555f48c92a0591d028c403`.
- pi 为官方独立 Linux x64 0.85.1 包，运行时包 manifest SHA-256：`0701993b1f4ddd9f7b4a6b2e9ecd963b3ac9457aca0807b71d4eb98339343aae`。
- Opi 使用保留的 v0.8.2 musl 静态构建，未以当前源码替换。原始构建记录位于 `builds/source-records/`，源码、客户端和依赖材料保留于原 WSL 缓存。
- Companion 单独记录：包版本 0.8.2，clean 源码 `55fbc95009e6f47145196dbc539454ebcb1cf692`，可执行文件 SHA-256 `80da1b3b3fc83802035d77cf3df6e9e4e220f36e0f86ef3eacc1733c12c7c560`，Cargo.lock SHA-256 `261a3ec501be2e045dc6c9b2b9a940adc1c56305f477008eea940f26545925f3`；Rust/Cargo 1.97.1。详见[构建身份](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/builds/companion-identity.json)与[独立上游身份观察](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/builds/source-identities.json)。

## 冻结范围与平台

| 基准 | 选定任务 | Plan digest | Material SHA-256 | 原生任务上限 |
|---|---|---|---|---|
| terminal-bench-2.1 | `openssl-selfsigned-cert` | `02215f12563dbebbec7ca91258383613f780f878a78d8d9e3edbceb69a51aa45` | `7a765b62a9637d082a8928fbd441e7d271b5b3e8679c60ac334479cfb9654e1e` | 900 s |
| terminal-bench-3.0 | `batched-eval-parity` | `62b5f4b11ddff301f8415f4c14b8e03e73e3ad156bc7f7789b8bbf81bfcfcbc6` | `b671b87603fdf25fc2a642e58b085d4ff882d068e0bd621bad9cae1adb6e0db3` | 900 s |
| deepswe-v1.1 | `abs-module-cache-flags` | `5a0c47671489748610b29f258c793700ad1dc9a13fc7e0d98d33372dfbf753fc` | `e87b0bc804053fe1fa519978cb95e7d312567d4dd4f0909fef88643442f7129b` | 1800 s |

每个基准一个已准入任务包，每款软件重复一次，无 seed 覆盖；每个 family 的 `max_trials=2`，并发 1，共六个槽位。报告中的 corpus=1 仅指已准入任务包，不是完整上游语料规模。选用已调研、适合本地验收的三个任务。平台为 WSL2 Ubuntu、Linux x86_64，Docker `default` / `unix:///var/run/docker.sock`；观察到 32 个逻辑 CPU、16611237888 字节内存，三个任务需求依次为 1/1/2 CPU 与 2/4/8 GiB。

两款软件均投影为 provider `zhipu`、模型选择器 `glm-5.3-flash`、API `anthropic-messages`、端点 `https://open.bigmodel.cn/api/anthropic`。仅在获准启动时从仓库 `.env` 读取 `OPI_BIGMODEL_API_KEY`。要求 context=131072、output=8192，temperature、top-p、reasoning 均省略。精确客户端的合成回环探针证明初始请求及工具续接请求中的 `max_tokens=8192`，仅证明这个选定值；没有另行抓取真实线上请求。选择器背后的实际服务模型身份仍未知。pi 获得真实响应并完成工具循环；Opi 收到响应，但以流失败结束。

每次原生任务上限包含准备、Agent 执行和评分，六次合计 7200 秒；无总耗时、总 token 或金额硬上限。当前 native 路径不执行可配置超时覆盖。没有授权或执行付费补跑。第一组开始于 2026-09-10T10:47:26.588650Z，最后一组结算于 2026-09-10T11:03:02.877674Z；两者之间包含操作与离线检查间隔。

完整镜像身份见[预构建记录](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/checks/prepared-image-builds.json)和 Docker 基线。TB2.1 基础镜像摘要 `4c948a4e630af2435ae0a19108fc0814a946ac2fa29a512469e0fc77b38c8c12`；Python 基础镜像 `272efc3f94bec901e2bc7a5c9984b3d113c317839edca2714c6a388b9476f8fc`；DeepSWE 基础镜像 `3a4d47f5281269305343c83729836ac2f3172811aee72681e472a4196178eda1`。此前预检覆盖任务和验证器镜像，但未成功预构建 Pier 在运行时生成的 Ubuntu/Squid 代理镜像；这个准备缺口在 DeepSWE 启动时暴露。

## 原生结果与覆盖

原生指标为基准定义的 grader reward，分母包括有效零分与基准认可的 Agent 失败。下表沿用 Companion 原生分类，与 acceptance 语义准入分开。

| 基准 | 软件/版本 | Reward | 有效评分分母 | 任务包 corpus / selected | planned / attempted | 原生 valid / Agent failure | invalid / unexecuted |
|---|---|---|---|---|---|---|---|
| terminal-bench-2.1 | pi 0.85.1 | 1 | 1 | 1 / 1 | 1 / 1 | 1 / 0 | 0 / 0 |
| terminal-bench-2.1 | opi 0.8.2 | 0 | 1 | 1 / 1 | 1 / 1 | 1 / 1 | 0 / 0 |
| terminal-bench-3.0 | pi 0.85.1 | 0 | 1 | 1 / 1 | 1 / 1 | 1 / 1 | 0 / 0 |
| terminal-bench-3.0 | opi 0.8.2 | 0 | 1 | 1 / 1 | 1 / 1 | 1 / 1 | 0 / 0 |
| deepswe-v1.1 | pi 0.85.1 | unknown | 0 | 1 / 1 | 1 / 1 | 0 / 0 | 1 / 0 |
| deepswe-v1.1 | opi 0.8.2 | unknown | 0 | 1 / 1 | 1 / 1 | 0 / 0 | 1 / 0 |

TB2.1、TB3 各保留一个对照配对；DeepSWE 两侧都无有效结果，配对排除。其规范化报告指标标签为 `unknown/unknown`、分母 0，原因是没有准入的原生结果。人工语义复核可计入两条 pi 试验，Opi 为零条，无法选出六条必需验收记录。完整语料、广泛协议覆盖与其他 native witness 均未验证。

- TB2.1 pi 完成 9 次工具调用，并生成证书、密钥、PEM、验证信息及脚本，原生 reward=1。Opi 只有一次 provider turn、没有完成工具调用，以退出码 4 和 `provider stream failed` 结束；原生零分保持不变。
- TB3 pi 完成 24 次调用（6 bash、18 read）；最后响应只有 thinking 内容，`rawStopReason=max_tokens`、`stopReason=length`、输出用量 8192，没有修改进入收集目录。Opi 完成 3 次调用（2 bash、1 read）后以退出码 4 报 `provider stream failed`。两次独立收集的 `/app/evalbench/` 摘要均为 `9fafdfb3805cde053c0b8794a9d63998aa8dc6f9656e65a14d42913b271935a7`，均由声明的独立验证器评分，reward 均为 0。Opi 虽有真实工具调用，仍因 provider 失败排除于 acceptance。
- 两次 DeepSWE 原始异常均为 Pier 构建 egress proxy 时，获取 `ubuntu:24.04` 匿名 registry 元数据发生到 `auth.docker.io` 的 TCP 超时；`agent_execution` 和 `verifier_result` 均为 null。这是 Docker registry 基础设施问题，不能归为 Zhipu 认证失败。Companion 将其记为 `invalid-execution` / `native-binding-aggregate-invalid`，未封存文件与原始异常全部保留。没有观察到已提交补丁、收集交接或 pristine verifier 评分。

## 单次试验追溯

[机器可读试验摘要](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/reports/trial-summary.json)记录事件数、原生摘要、时间和来源路径。四份已封存 `native/trial-result.json` 与各自真实 job 结果逐字节一致。terminal 文件列表可能包含初始任务文件，不能仅凭该列表认定 Agent 修改。

| 试验 / 基准 / 任务 / 重复 | 软件 / 实际 job / 上游 trial | 结算 / 原生来源 | 工具循环 / 效果 | 官方交接 | Bundle manifest SHA-256 |
|---|---|---|---|---|---|
| `trial-baseline-pi-openssl-selfsigned-cert-r1`<br>terminal-bench-2.1<br>`openssl-selfsigned-cert`; r1 | pi<br>`2026-09-10__18-47-27` / `task-package__4g4NTjP` | sealed; reward 1; [native result](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-2.1/trials/trial-baseline-pi-openssl-selfsigned-cert-r1/bundle/artifacts/native/trial-result.json) | 9 calls: {'bash': 7, 'write': 1, 'edit': 1} | shared; [original trial files](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-2.1/trials/trial-baseline-pi-openssl-selfsigned-cert-r1/bundle/artifacts/control/settlement.json) | [bundle](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-2.1/trials/trial-baseline-pi-openssl-selfsigned-cert-r1/bundle/manifest.json)<br>`4a202ef0eb5892ce49cb9d2a2abe81489ecd656b13fb48617f22f4a12d020dc6` |
| `trial-candidate-opi-openssl-selfsigned-cert-r1`<br>terminal-bench-2.1<br>`openssl-selfsigned-cert`; r1 | opi<br>`2026-09-10__18-49-25` / `task-package__yiQiRX3` | sealed; reward 0; [native result](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-2.1/trials/trial-candidate-opi-openssl-selfsigned-cert-r1/bundle/artifacts/native/trial-result.json) | 0 calls: {} | shared; [original trial files](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-2.1/trials/trial-candidate-opi-openssl-selfsigned-cert-r1/bundle/artifacts/control/settlement.json) | [bundle](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-2.1/trials/trial-candidate-opi-openssl-selfsigned-cert-r1/bundle/manifest.json)<br>`0a249c347eef45ab0d98459fe5177003e03c9d4726d380e32b631e2529903c2e` |
| `trial-baseline-pi-batched-eval-parity-r1`<br>terminal-bench-3.0<br>`batched-eval-parity`; r1 | pi<br>`2026-09-10__18-53-32` / `task-package__vTmwXgc` | sealed; reward 0; [native result](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-3.0/trials/trial-baseline-pi-batched-eval-parity-r1/bundle/artifacts/native/trial-result.json) | 24 calls: {'bash': 6, 'read': 18} | separate; [original trial files](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-3.0/trials/trial-baseline-pi-batched-eval-parity-r1/bundle/artifacts/control/settlement.json) | [bundle](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-3.0/trials/trial-baseline-pi-batched-eval-parity-r1/bundle/manifest.json)<br>`6bbb3ae86ea56d13c6675eaab48a0718c68e3dc2faa652d6bd5f1f6566abdfc4` |
| `trial-candidate-opi-batched-eval-parity-r1`<br>terminal-bench-3.0<br>`batched-eval-parity`; r1 | opi<br>`2026-09-10__18-59-34` / `task-package__eXQcqMD` | sealed; reward 0; [native result](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-3.0/trials/trial-candidate-opi-batched-eval-parity-r1/bundle/artifacts/native/trial-result.json) | 3 calls: {'bash': 2, 'read': 1} | separate; [original trial files](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-3.0/trials/trial-candidate-opi-batched-eval-parity-r1/bundle/artifacts/control/settlement.json) | [bundle](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-3.0/trials/trial-candidate-opi-batched-eval-parity-r1/bundle/manifest.json)<br>`547838d8d9966358d50f765d15bdc35e1812b53b768a49768ffae8fe956fa739` |
| `trial-baseline-pi-abs-module-cache-flags-r1`<br>deepswe-v1.1<br>`abs-module-cache-flags`; r1 | pi<br>`2026-09-10__19-01-49` / `task-package__h6zmBKk` | failed; unsealed; no score; [native exception](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/deepswe-v1.1/trials/trial-baseline-pi-abs-module-cache-flags-r1/verifier-trace/jobs/2026-09-10__19-01-49/task-package__h6zmBKk/result.json) | Agent did not start | no patch or verifier handoff; [original trial files](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/deepswe-v1.1/trials/trial-baseline-pi-abs-module-cache-flags-r1/bundle/artifacts/control/settlement.json) | absent; original unsealed files retained |
| `trial-candidate-opi-abs-module-cache-flags-r1`<br>deepswe-v1.1<br>`abs-module-cache-flags`; r1 | opi<br>`2026-09-10__19-02-30` / `task-package__BHqYSsA` | failed; unsealed; no score; [native exception](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/deepswe-v1.1/trials/trial-candidate-opi-abs-module-cache-flags-r1/verifier-trace/jobs/2026-09-10__19-02-30/task-package__BHqYSsA/result.json) | Agent did not start | no patch or verifier handoff; [original trial files](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/deepswe-v1.1/trials/trial-candidate-opi-abs-module-cache-flags-r1/bundle/artifacts/control/settlement.json) | absent; original unsealed files retained |

## 用量、时间与费用

耗时来自原生时间戳。pi 用量是对已完成 assistant 响应按 response ID 去重后求和，属于诊断信息，不是账单；下表依次为 input/output/cache-read/cache-write。Opi 错误路径的零计数不代表没有消耗，其 trace manifest 将用量标为 unknown。DeepSWE 未启动 Agent。

| 基准 / 软件 | 原生 trial 秒 | Agent 秒 | Provider turns / 已完成 pi 响应 | 工具调用 | Input / output / cache read / cache write tokens |
|---|---|---|---|---|---|
| terminal-bench-2.1 / pi | 116.595 | 89.378128 | 10 | 9 | 31340/4119/6272/0 |
| terminal-bench-2.1 / opi | 31.025 | 6.740122 | 1 | 0 | unknown; error-path zero counters are not usage evidence |
| terminal-bench-3.0 / pi | 361.119 | 311.540159 | 13 | 24 | 26822/9679/147072/0 |
| terminal-bench-3.0 / opi | 52.294 | 13.787313 | 3 | 3 | unknown; error-path zero counters are not usage evidence |
| deepswe-v1.1 / pi | 31.776 | not started | not started | not started | not called |
| deepswe-v1.1 / opi | 32.290 | not started | not started | not started | not called |

规范化原生报告将 usage、retry、compaction、cost 标为 unknown。pi 配置中的零费用字段是占位值，不是实际费用。账单金额、完整 Opi token 消耗未知。本次没有操作层补跑或重新评分；provider 内部重试与压缩次数未得到验证。

## 校验与隐私

以下命令表与英文版逐项对应：

| Check / command (absolute substitutions below) | Cwd | Exit / observed result |
|---|---|---|
| `$R/builds/opi-eval run --plan $R/plans/$F.json --root $R/runs/$F --fixtures $R/unused-fixtures --canaries $R/private/live-canaries.txt` | `$R` | TB2.1: 0; TB3: 0; DeepSWE: 1 |
| `$R/builds/opi-eval verify --root $R/runs/$F` | `$R` | TB2.1: 0; TB3: 0; DeepSWE: 1 (`mutation-detected`, two `io` failures because bundle manifests were never sealed; not evidence of a later edit) |
| `$R/builds/opi-eval report --root $R/runs/$F --out $R/reports/$F-a.json --canaries $R/private/live-canaries.txt` and fresh `-b.json` | `$R` | All six commands: 0; each A/B pair identical |
| `$R/builds/opi-eval verify --root $E/runs/$F` | `$R` | 0 / 0 / 1, same original outcomes |
| `$R/builds/opi-eval report --root $E/runs/$F --out $C/$F-a.json` and fresh `-b.json` | `$R` | All 0; A = B = original for each family |
| `python3 $REPO/crates/opi-eval/scripts/verify-local-acceptance.py --root $E --out $C/acceptance-result.json --companion $R/builds/opi-eval` | `$R` | 2; `accepted: false`; rejects absent acceptance manifest before deeper acceptance checks |
| `python3 $REPO/scripts/opi-artifact-audit.py $E/opi-artifact-audit --workspace-root $REPO --json` | `$R` | 0; 17 preserved files in three exact native directories; no detected issues. The third is an unsealed failure fragment; no DeepSWE Agent capture exists. |
| `python3 $R/verify-staged.py` supplemental `analyze_ndjson` from the same repository auditor | `$R` | 0; exactly two actual Opi stdout captures analyzed, zero reported issues; does not validate provider behavior or prove acceptance |
| Post-cleanup `verify` plus fresh `report --out $C/post-cleanup/$F-report.json` on original roots | `$R` | Verify 0 / 0 / 1; reports all 0 and equal to originals |

`F` is each of the three exact family names above. `R`, `E`, `C` denote their WSL paths; `REPO=/mnt/d/Luiz/Odradek/opi`. Exact expanded argv, timestamps and exits are in [original commands](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/commands/run-deepswe-v1.1.json), the other `commands/run-*.json` / `verify-*.json` / `report-*.json`, and the external check records. Credential variables were absent from offline check environments. The disposable canary path was removed after privacy checks.

构建、合成探针以及三个 family 的 doctor/prepare/validate/plan 均退出 0，完整命令仍保留。历史[准备报告](20260910t100917z-opi082-pi0851-glm53flash-acceptance-preview.zh.md)未覆盖改写。探针命令为 `cargo test -p opi-eval --test native_driver native_output_control_client_matrix -- --ignored --exact`，1 passed / 0 failed。本次未修改 Rust、运行源码或测试文件，测试影响 **retain**；未执行 workspace runtime gates 或新的三协议 witness。

已对完整本地诊断副本的候选内容扫描本次选定凭据，零命中，未修改或重封存任何原生字节。[隐私记录](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/checks/privacy-final.json)包含扫描范围和限制。三个可用 Opi native 目录共 17 个文件，副本与原目录完全一致；通用审计在该目录形态下没有扫描 NDJSON/session，补充审计直接读取两份实际 Opi stdout。任何审计通过都不能替代缺失的 DeepSWE 轨迹或验收语义。

证据维持本地私有状态：包含操作者路径、任务公开作者元数据，以及 TB2.1 pi 日志中的任务生成 TLS 私钥标记。未复制 provider 密钥、dotenv、个人凭据存储或真实 canary 文件，未批准公开分享。acceptance verifier 因缺少 manifest 拒绝，仅证明此结构缺口，未执行更深层的六次验收检查。

文档校验：`python3 scripts/opi-doc-check.py` 通过，`git diff --check` 退出 0；[精确命令及结果](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live-checks/documentation-checks.json)已保留。Git 中仅两份历史预览和两份新执行报告为未跟踪文件，运行源码和测试未改变。

## 清理与验收交接

清理状态 **completed**。全部写入结束和隐私检查完成后，删除选定凭据的临时 canary 副本及四个空的自有目录。Docker 检查为零容器、零卷、仅默认网络。可复用任务/基础/验证器/runner 镜像、材料/源码/客户端/依赖缓存、冻结可执行文件以及全部成功和失败证据均保留。DeepSWE 基础镜像在原生清理后缺失，按原摘要重新拉取并恢复别名，两条操作退出 0，身份与原记录一致。没有全局 prune、操作层镜像删除或 Cargo 缓存删除。实际回收空间未知。

清理前后比较 404 个原始保护文件及全部 694 个副本文件，哈希无变化。清理后 verify 仍为 0 / 0 / 1，重建报告与原始报告完全一致。详见[清理记录](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live-checks/cleanup/cleanup.json)与[清理后命令](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live-checks/post-cleanup/commands.json)。

| 条款 | 操作者证据状态 | 已有证据 / 缺口 |
|---|---|---|
| P19-AC-001 | TB2.1/TB3 evidenced；DeepSWE 全流程 missing | 独立目录命令、四份封存和报告重建；DeepSWE 环境启动失败 |
| P19-AC-003 真实部分 | rejected/incomplete | pi 任务效果和同环境评分已保留；Opi 缺真实工具循环 |
| P19-AC-004 真实部分 | 机制 evidenced；完整验收配对 rejected | 两份声明目录收集和独立评分；Opi provider 失败使其验收行无效，广泛正反 witness 未在此运行 |
| P19-AC-005 真实部分 | missing | 两个 Agent 未启动，缺 base→committed patch→pristine verifier 证据 |
| P19-AC-006 真实部分 | 部分 evidenced；完整要求 rejected | 共同连接投影、真实 pi 循环和 Opi TB3 工具；Opi 流失败、实际后端模型身份缺失，未新增三协议测试 |
| P19-AC-012 | rejected | 六槽位已尝试、四次实际 Agent 运行、四份评分/封存、两条 pi 语义合格；缺六条合格记录 |

[诊断发现](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/reports/findings.jsonl)将 EVAL-001（Opi 流失败，根因未定）和 EVAL-002（DeepSWE 代理镜像启动失败）记录为 `source_kind=eval`、`axis=runtime-fidelity`、`status=unverified`，未加入活动审计集。评估者精确模型 ID 与独立性为 unknown，未使用独立模型裁判。

剩余动作是离线诊断已保留的 Opi 失败，并准备原版 Pier 代理镜像及依赖。源码修复需要先批准具体范围；替换真实试验需要新的冻结身份及明确执行授权。原证据继续保留于 R、E，不能凭这份 inventory 将 task 19.11 当作已通过验收来继续。
