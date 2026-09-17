# 验收执行：Opi HEAD 对照 pi 0.85.1

[English](20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live.md)

**验收交付状态：blocked。** 获批的六个槽位均已结算：四次 Terminal-Bench 真实试验获得可归属的原生分数；DeepSWE 两个槽位在 Agent 启动前即因环境准备失败而无效。Opi 的 TB 2.1 得分为 1，其余三次已评分试验得分为 0。本轮没有付费重试、源码修复、提交、重新评分、账本更新或 Phase 退出。

## 证据与授权

- 私有执行目录：`/home/luiz/opi-eval-runs/20260910t154500z-offline-diagnosis-opi082/next-acceptance`。
- [私有证据目录](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live)包含 586 个已登记文件。清单 SHA-256：`988ebb7448b4affe05431df7e007693c6d124cbe89ea89cad570b791113ba66a`。这是诊断清单，**不是验收 manifest**；未伪造验收 `manifest.json`。
- [外部检查](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live-checks/final-acceptance/verification-summary.json)、[清理记录](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live-checks/cleanup-live.json)、[机器试验汇总](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live/reports/trial-summary.json)、[历次尝试关系](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live/reports/attempt-history.json)。
- 冻结[范围](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live/operator-scope.json) SHA-256：`dbe94817a77673b2673ba59b3c643d71d946d068a881875bb5e2a816144c5a6e`；[授权记录](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live/authorization.json) SHA-256：`e39590678f537f6c99fa6cb420c375af504b7e0ffa31011e5152cbce54ceadca`，记录用户明确同意新候选和六次试验。
- 首次启动 `2026-09-10T16:26:53.972154+00:00`，最后结算 `2026-09-10T16:51:00.716292+00:00`，观察跨度 1446.74 秒，包含家族之间的操作检查。三个家族使用独立原生计划，串行运行。
- 原发布版评估和[离线诊断/预览](20260910t154500z-opi082-head-pi0851-glm53flash-diagnosis-preview.zh.md)保持不变。虽然包版本相同，本轮 Opi 是**未发布 HEAD 候选，不是 v0.8.2 标签**。

## 身份与冻结限制


| Component / 组件 | Source / 源码 | Executable SHA-256 / 可执行文件摘要 |
|---|---|---|
| Opi candidate, package 0.8.2, musl | `55fbc95009e6f47145196dbc539454ebcb1cf692` | `d7e5ab2d2636b04bfefafdeb2285bf50933247e7fc06836b6778d91ff4056e60` |
| pi 0.85.1, official Linux x64 | `d981de1229ef899957bbe968bc8dcda02a21f477` | `443bd83f30e4dbc7bac2eed9c6aa2461b9a15016fd555f48c92a0591d028c403` |
| opi-eval Companion, package 0.8.2 | `55fbc95009e6f47145196dbc539454ebcb1cf692` | `80da1b3b3fc83802035d77cf3df6e9e4e220f36e0f86ef3eacc1733c12c7c560` |

Opi 已跟踪源码保持干净。pi 运行包 manifest SHA-256 为 `0701993b1f4ddd9f7b4a6b2e9ecd963b3ac9457aca0807b71d4eb98339343aae`；Cargo.lock SHA-256 为 `261a3ec501be2e045dc6c9b2b9a940adc1c56305f477008eea940f26545925f3`。构建使用 Rust/Cargo 1.97.1。Harbor 提交为 `4407eb5227a2ff4f0d3f16b2eb48849382fdf276`，Pier 为 `c1ebc6d145b40fae8425215e3fca528945065124`；按仓库绑定的 commit/tree 观测和来源保存在[源码身份](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live/builds/source-identities.json)及[上游缓存](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live/upstream-identity-cache.json)。精确二进制、完整 runner 源码、运行包与共享缓存保留在原 Linux 路径。

冻结连接为 `zhipu:glm-5.3-flash`、`anthropic-messages`、`https://open.bigmodel.cn/api/anthropic`。启动助手只从 `.env` 选取 `OPI_BIGMODEL_API_KEY`，不做插值或 shell 执行。pi 已完成响应的元数据报告 `glm-5.3-flash`；Opi 记录 requested/resolved 为 `zhipu:glm-5.3-flash`，但实际 wire 身份标为 `wire_unknown/not_reported`，保留未知状态，不从选择器反推。Opi 的认证来源最终解析为选定的环境变量引用。

每次响应输出 8192、客户端上下文声明 131072，保持绑定准备阶段的本机探针证明；temperature、top_p、reasoning 均省略。上下文声明不是服务端容量实测。每个家族选择一个本地准入固定任务，两个软件各重复一次，共六个槽位，并发 1，无 seeds。CPU/内存依次为 1 CPU/2 GiB、1 CPU/4 GiB、2 CPU/8 GiB，无需 GPU。本验收子集未枚举或运行完整上游题库。

单次原生作业上限依次为 900/900/1800 秒，包含环境准备、Agent 与评分；六次累计上限为 7200 秒。原生作业 stdout/stderr 各限 1 MiB；另存的详细 Agent 日志可能更大。不提供总 token、总费用或总墙钟时间硬上限，材料获取和清理开销在原生时限累计值之外。本轮未覆盖超时配置、修改任务镜像、替换模型或重试。


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


## 原生结果与任务证据

| 家族 | Agent | 原生奖励 | 评分分母 | 计划/尝试 | 有效/Agent 失败 | 无效/未执行 |
|---|---|---:|---:|---|---|---|
| TB 2.1 | Opi | 1 | 1 | 1/1 | 1/0 | 0/0 |
| TB 2.1 | pi | 0 | 1 | 1/1 | 1/1 | 0/0 |
| TB 3.0 | Opi | 0 | 1 | 1/1 | 1/1 | 0/0 |
| TB 3.0 | pi | 0 | 1 | 1/1 | 1/1 | 0/0 |
| DeepSWE 1.1 | Opi | 未评分 | 0 | 1/1 | 0/0 | 1/0 |
| DeepSWE 1.1 | pi | 未评分 | 0 | 1/1 | 0/0 | 1/0 |

所有行均为 subset；各家族已准入语料包数/选中数为 1/1。原生指标定义是有效已评分试验上的 grader reward，有效零分与基准定义的有效 Agent 超时保留在分母中。三个家族分母分别为 2、2、0，不跨基准取总平均，也不声称统计优劣。Companion 规范化报告中的 `classification: conformance-evidence` 字符串按原样保留；真实执行含义另由原始 Agent/native 证据核对。

TB 2.1 在实际共享任务环境中评分。两个 Agent 都执行了 5 次 bash 和 1 次 write，交付 `/app/ssl/server.key`、`.pem`、`.crt`、`verification.txt` 和 `/app/check_cert.py`，终端回执保留各文件摘要。Opi 六项原生检查全过；pi 通过 5/6，失败原因是交付的验证脚本需要 `cryptography`，原生测试调用时该依赖不可用。两个 Agent 均退出 0。本次 Opi 真实试验未再出现原发布版的流中断。

TB 3.0 将 `/app/evalbench/` 收集后交给独立验证环境。Opi 收集摘要为 `f551524326e8b279a490f1b7d02b1c3fcaa212f1948b63bfa66bd4fa3d1ebbf9`，pi 为 `23a7c76b14fd8e3dc7c5a83f01eb3dd67672df278541eb264268e4602777146b`。Opi 执行 6 次 bash/18 次 read，pi 执行 8 次 bash/17 次 read；均退出 0，五项原生测试均失败。pi 最后一轮只包含 thinking，`stopReason=length`，输出 8192。Opi trace 记录 success 且没有 error diagnostic；这是执行结果，不是任务成功。

DeepSWE 自动生成的代理 Dockerfile 仍从 `FROM ubuntu:24.04` 开始。两个原生 `result.json` 均记录 `RuntimeError`：从 `auth.docker.io` 获取匿名令牌时连接 `69.63.176.15:443` 超时，环境准备分别耗时 31.22、31.93 秒。Agent execution 和 verifier result 为 null，没有提交补丁、收集后的验证器输入或封存 bundle。两个结算均为 `invalid-execution`、边界 `integrity`、原因 `native-binding-aggregate-invalid`。先前代理缓存预热没有消除该联网依赖。[EVAL-NEXT-001](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live/reports/findings.jsonl)记录此操作基础设施问题，状态为 unverified，评估模型身份/独立性未知；未将其纳入正式审计，也未据此断言 Opi 源码有缺陷。

## 用量与时间


| Family / 家族 | Agent | Trial seconds / 试验秒数 | Completed provider turns / 已完成模型轮次 | Tool calls / 工具调用 | Input | Output | Cache read | Cache write |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| terminal-bench-2.1 | pi | 177.85 | 7 | 6 | 32964 | 5966 | 9088 | 0 |
| terminal-bench-2.1 | opi | 108.98 | 6 | 6 | 4570 | 2749 | 15296 | 0 |
| terminal-bench-3.0 | pi | 459.16 | 14 | 25 | 36784 | 12887 | 160896 | 0 |
| terminal-bench-3.0 | opi | 524.01 | 12 | 24 | 38949 | 13051 | 173312 | 0 |
| deepswe-v1.1 | pi | 31.63 | not started | 0 | not observed | not observed | not observed | not observed |
| deepswe-v1.1 | opi | 32.33 | not started | 0 | not observed | not observed | not observed | not observed |

用量来自 pi 已完成 assistant 响应和 Opi session/trace 汇总，是客户端/服务端报告的观测值，不是账单。DeepSWE 没有启动 Agent，因此没有用量观测。pi 配置中的费用零值是占位值，不代表免费；实际账单费用未知，传输层重试/压缩次数也未完整测量。逐阶段时间戳与用量来源保存在机器汇总中。四个封存的 native trial-result 文件均与原始 runner 结果字节一致。

## 验证、隐私与清理

```sh
# Linux cwd: /home/luiz/opi-eval-runs/20260910t154500z-offline-diagnosis-opi082/next-acceptance
# f = terminal-bench-2.1, terminal-bench-3.0, deepswe-v1.1
./builds/opi-eval run --plan plans/$f.json --root runs/$f --fixtures unused-fixtures --canaries private/live-canaries.txt
./builds/opi-eval verify --root runs/$f
./builds/opi-eval report --root runs/$f --out reports/$f-a.json --canaries private/live-canaries.txt
./builds/opi-eval report --root runs/$f --out reports/$f-b.json --canaries private/live-canaries.txt
```

[命令记录](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live/commands)保留精确绝对参数、cwd、起止时间、退出码和 stdout/stderr。canary 参数是历史私有引用，临时文件现已删除；不得重新启动这些已用运行目录。准备阶段 doctor/prepare/validate/plan 均在批准前退出 0。


| Family / 家族 | run | verify original/staged/post-cleanup | report A/B/C/D/post-cleanup | Normalized report byte equality / 报告字节一致 |
|---|---:|---|---|---|
| terminal-bench-2.1 | 0 | 0 / 0 / 0 | all 0 | yes |
| terminal-bench-3.0 | 0 | 0 / 0 / 0 | all 0 | yes |
| deepswe-v1.1 | 1 | 1 / 1 / 1 | all 0 | yes; incomplete/invalid results retained |

DeepSWE 的 verify 报告 `mutation-detected` 和两个 `io` 失败，原因是从未形成封存 bundle，不能据此断言发生了事后篡改。重建后的规范化报告保留 incomplete、invalid=2 和 denominator=0。字节比较对象是原始规范化报告；`run-report.json` 属于另一 schema，不要求与其字节相同。

实际[离线验收命令](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live-checks/final-acceptance/acceptance.json)退出 2、`accepted:false`，因为缺少满足六行条件的 manifest。通用 Opi 审计覆盖三个完整 native 捕获目录（7/7/3 文件），退出 0；第三个目录是未封存的失败片段。对两份真实 Opi stdout 的补充审计也退出 0。两项审计的已识别 NDJSON/session 列表均为空，因此不能据此证明完整 provider/tool 语义或验收通过；真实 trace、原生交付和结果归属另行核对。未请求或执行原生 regrade。

各家族和整个暂存选择的 provider canary 扫描均无命中。保留的 pi TB 2.1 stdout 含任务生成的 TLS 私钥内容，另有操作路径、任务作者元数据和运行时代理配置，因此证据**仅供本地私有使用，未批准公开分享**。未删改、重写或重新封存任何 sealed 字节。

[清理记录](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live-checks/cleanup-live.json)状态为 completed：容器和卷均为 0，临时 canary 与四个本轮空目录已删除。DeepSWE 原基础镜像引用从保留的本地镜像 `sha256:3a4d47f5281269305343c83729836ac2f3172811aee72681e472a4196178eda1` 恢复，无需联网下载。可复用镜像、保留标签、原证据、固定二进制及 Cargo/uv/BuildKit 共享缓存均保留。回收空间未知。549 个本轮保护文件、586 个暂存文件、39 个预览保护文件、410 个原运行文件，以及之前 694/276 文件的暂存清单，在清理后摘要均一致；清理后的验证与报告重建结果也保持一致。

仓库中只新增本中英报告对。测试影响为 **none**，未修改运行时或测试源码。最终 `python3 scripts/opi-doc-check.py` 和 `git diff --check` 的实际结果见[文档检查](E:/opi/evaluations/20260910t162653z-opi-head-pi0851-glm53flash-acceptance-live-checks/final-doc-checks.json)。本轮未运行 workspace 门禁或 Phase 审计。

## 验收映射与下一步

| 标准 | 本轮状态 | 证据或缺口 |
|---|---|---|
| P19-AC-001 | 部分具备，整体 blocked | 独立准备/执行/报告在 TB 完成，DeepSWE 准备失败。 |
| P19-AC-003 真实部分 | evidenced | 两次共享环境原生评分及真实工具/文件效果。 |
| P19-AC-004 真实部分 | evidenced | 两个收集目录、独立验证环境记录、有效原生零分。 |
| P19-AC-005 真实部分 | missing | DeepSWE 两个 Agent 未启动，无提交补丁或 pristine 评分。 |
| P19-AC-006 真实部分 | 部分具备 | TB 中两种客户端执行选定路由；DeepSWE 缺失；Opi 实际 wire 模型保持明确未知。 |
| P19-AC-012 | missing | 需要六次，已有四次合格真实试验，另两次因基础设施无效。 |

剩余执行阻塞是固定代理构建时 Docker/WSL 到 Docker Hub 认证服务的连接。应先离线解决并验证此依赖，再提出另行批准的新执行范围；两轮失败的 DeepSWE 尝试均须保留。本报告不授权进一步付费执行。
