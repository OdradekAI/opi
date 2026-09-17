# 离线诊断与新一轮验收预览

[English](20260910t154500z-opi082-head-pi0851-glm53flash-diagnosis-preview.md)

已离线复现发布版 v0.8.2 的签名事件故障。当前提交 `55fbc950` 已包含对应处理，独立构建的候选通过相同复现；本轮没有修复或修改源码。新的六次验收计划已经准备好，**六次均未执行，等待新的明确授权**。原验收仍为 blocked；离线检查不改变原生分数，也不构成 Phase 19 完成。

## 证据与范围

- 私有 Linux 诊断目录：`/home/luiz/opi-eval-runs/20260910t154500z-offline-diagnosis-opi082`；拟执行目录：`/home/luiz/opi-eval-runs/20260910t154500z-offline-diagnosis-opi082/next-acceptance`。
- 审阅副本：[E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082)，包含诊断记录和预览输入；可执行文件与 runner 缓存保留在原 Linux 路径。
- [操作范围](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/next-acceptance/operator-scope.json) SHA-256：`dbe94817a77673b2673ba59b3c643d71d946d068a881875bb5e2a816144c5a6e`。这是预览摘要，**不是验收 manifest 摘要**；本轮尚无真实试验或验收 manifest。
- 原[真实试验报告](20260910t104726z-opi082-pi0851-glm53flash-acceptance-live.zh.md)及其六个成功/失败槽位保持不变。之前的 `go` 只覆盖原范围；本预览将 Opi 换为另一源码提交，并新增六次真实试验，包括重新运行 pi 基线。
- 本轮没有读取真实凭据或调用真实模型。代理镜像准备访问了 Docker 仓库；Agent 复现只使用本机合成响应，容器检查均使用 `--network none`。

## 诊断与边界

`v0.8.2` 的 Anthropic `RawDelta` 未支持 `signature_delta`。[源码差异](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/checks/anthropic-source-diff.patch)表明当前 HEAD 已增加签名事件处理及 signed/redacted thinking 保留。合成响应依次返回 thinking 块、签名事件、bash 工具调用和最终响应。成功标准为两次模型请求、`proof.txt=offline-ok`，且工具续轮请求保留签名。

| 可执行文件与用例 | 子进程退出码 | 请求次数 | 工具效果 / 签名回传 | 结果 |
|---|---:|---:|---|---|
| 发布版 Opi v0.8.2，带签名，第 1 次 | 4 | 1 | 无 / 无 | `provider stream failed`；复现脚本退出 1 |
| 发布版 Opi v0.8.2，带签名，第 2 次 | 4 | 1 | 无 / 无 | 同样失败；脚本退出 1 |
| 发布版 Opi v0.8.2，不带签名对照 | 0 | 2 | 有 / 不适用 | 脚本退出 0 |
| 官方 pi 0.85.1，带签名 | 0 | 2 | 有 / 保留 | 脚本退出 0 |
| 当前 HEAD GNU 构建，带签名 | 0 | 2 | 有 / 保留 | 脚本退出 0 |
| 当前 HEAD musl release 构建，带签名 | 0 | 2 | 有 / 保留 | 脚本退出 0 |

[复现记录](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/repro)保留精确参数、二进制摘要、时间、合成请求/响应、stdout、stderr 和工具效果；[复现脚本](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/stream-repro.py)不含真实凭据。

```sh
# Synthetic loopback only. --out must be a fresh directory.
python3 /home/luiz/opi-eval-runs/20260910t154500z-offline-diagnosis-opi082/stream-repro.py --binary /home/luiz/opi-eval-runs/20260910t154500z-offline-diagnosis-opi082/next-acceptance/cache/agents/opi --product opi --case signed --out <fresh-private-directory>
```

原 pi 真实日志中，TB 2.1 观察到的 thinking 块有 7/7 携带签名，TB 3.0 为 8/8。[元数据记录](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/checks/live-signature-metadata.json)只包含计数与来源身份，不导出签名值。这支持原 Opi 故障的签名假设，但原 Opi 试验没有保留模型原始线缆事件，因此不能确认每次真实失败恰好由哪个事件触发。原生零分和未知遥测保持原样。原 pi TB 3.0 失败到达 8192 输出上限，未完成预期任务效果，仍是另一项独立失败。

原 DeepSWE 两个槽位在生成 egress proxy 镜像时因 Docker Hub metadata/auth 访问超时而失败，Agent 尚未启动。本轮逐字复制保留的 Dockerfile 和启动脚本，未作修改。`docker build --progress=plain --tag opi-eval-pier-egress-proxy:20260910t154500z /home/luiz/opi-eval-runs/20260910t154500z-offline-diagnosis-opi082/proxy-context` 退出 0；使用合成 token、禁用外部网络的健康检查最终在 localhost:8080 就绪。[构建与检查记录](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/checks/proxy-smoke.json)对应镜像 `sha256:95c565d09bbf73b2990489346866f719d90566d775524b73c040a11ddfda22ea`。

构建日志将 Ubuntu 解析为 `sha256:224a1869083a311ef3f13648a154ba79832fbef6364d31493642ca03082da254`。保留 BuildKit 缓存；本地没有 `ubuntu:24.04` 镜像标签。固定版本的原生 runner 仍会生成并构建自身未修改的代理；缓存预热不保证后续仓库访问成功，也不修复原来的两个无效试验。

## 拟测软件与控制项

| 组件 | 源码 / 版本 | 可执行文件 SHA-256 |
|---|---|---|
| 原 Opi，仅保留 | v0.8.2；`765608a55a78db47e8b41d2639c29ef8c90e0f31` | `f1bd9bbf82d03120427e4de42318f31e939c0a94dde99705f4fdde9eabf063ee` |
| 新 Opi 候选 | 未发布提交 `55fbc95009e6f47145196dbc539454ebcb1cf692`；包版本 0.8.2；x86_64 musl | `d7e5ab2d2636b04bfefafdeb2285bf50933247e7fc06836b6778d91ff4056e60` |
| 新 pi 基线 | 官方 Linux x64 0.85.1；`d981de1229ef899957bbe968bc8dcda02a21f477` | `443bd83f30e4dbc7bac2eed9c6aa2461b9a15016fd555f48c92a0591d028c403` |
| Companion，与被测 Opi 分开记录 | opi-eval 包版本 0.8.2；`55fbc95009e6f47145196dbc539454ebcb1cf692` | `80da1b3b3fc83802035d77cf3df6e9e4e220f36e0f86ef3eacc1733c12c7c560` |

虽然包版本输出相同，新候选**不是发布标签 v0.8.2**。已跟踪源码干净，本轮未编辑源码。pi 完整运行包 manifest SHA-256 为 `0701993b1f4ddd9f7b4a6b2e9ecd963b3ac9457aca0807b71d4eb98339343aae`；Cargo.lock SHA-256 为 `261a3ec501be2e045dc6c9b2b9a940adc1c56305f477008eea940f26545925f3`。Rust/Cargo 为 1.97.1。Harbor 固定为 `4407eb5227a2ff4f0d3f16b2eb48849382fdf276`，Pier 固定为 `c1ebc6d145b40fae8425215e3fca528945065124`；使用独立复制的干净源码与离线重新绑定的虚拟环境。完整身份见[构建记录](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/next-acceptance/builds)。

连接沿用 `zhipu:glm-5.3-flash`、`anthropic-messages`、`https://open.bigmodel.cn/api/anthropic`。只有在获准启动 Agent 时才从仓库 `.env` 选取 `OPI_BIGMODEL_API_KEY`。共同控制项为每次响应输出 8192、上下文窗口 131072；temperature、top_p、reasoning 均省略。上下文值是客户端声明，不是服务端容量证明。实际候选与 pi 的本机输出控制探针已通过，覆盖初次请求及工具续轮；证明在冻结计划之前绑定。

三个家族各选择一个本地已准入的固定任务包，两个软件，各重复一次，并发 1，共六次新试验。这是与原计划相同的验收子集，不是完整上游题库，也不足以推断统计优劣。三个任务依次需要 1 CPU/2 GiB、1 CPU/4 GiB、2 CPU/8 GiB；无需 GPU。


| Family / 家族 | Task / 任务 | Trials / 次数 | Native job cap (s) / 原生作业上限 | Plan digest / 计划摘要 |
|---|---|---:|---:|---|
| terminal-bench-2.1 | `openssl-selfsigned-cert` | 2 | 900 | `25edf422fd80ada6843311f0df0d7c530cb27f783677df34b9b5d0e20a993509` |
| terminal-bench-3.0 | `batched-eval-parity` | 2 | 900 | `e47cbc4bd1c8d6d0b0641f14744ebb4c2f3c3252d0d9e13199095c97d95535d1` |
| deepswe-v1.1 | `abs-module-cache-flags` | 2 | 1800 | `a616c7ed6222bb247a259145f932bda7e26524dcacc5c81f0773a6889913dec0` |

| Family / 家族 | Material SHA-256 / 材料摘要 |
|---|---|
| terminal-bench-2.1 | `865548a1fc487b61ecaa923651190db247566b9e877fc32edd52c891c900bc93` |
| terminal-bench-3.0 | `8798fc9e817c36a933de361472746b074b87a89e5ca7005fe2140ac6a4ad69bc` |
| deepswe-v1.1 | `a2cf5cd83a577b76f1d070785e74cd9b47170ad1a0dbf8f7128d02faf42f169f` |

| Trial / 试验 | Family / 家族 | Subject / 被测对象 | State / 状态 |
|---|---|---|---|
| `trial-baseline-pi-next-openssl-selfsigned-cert-r1` | terminal-bench-2.1 | baseline-pi-next | unexecuted / 未执行 |
| `trial-candidate-opi-head-openssl-selfsigned-cert-r1` | terminal-bench-2.1 | candidate-opi-head | unexecuted / 未执行 |
| `trial-baseline-pi-next-batched-eval-parity-r1` | terminal-bench-3.0 | baseline-pi-next | unexecuted / 未执行 |
| `trial-candidate-opi-head-batched-eval-parity-r1` | terminal-bench-3.0 | candidate-opi-head | unexecuted / 未执行 |
| `trial-baseline-pi-next-abs-module-cache-flags-r1` | deepswe-v1.1 | baseline-pi-next | unexecuted / 未执行 |
| `trial-candidate-opi-head-abs-module-cache-flags-r1` | deepswe-v1.1 | candidate-opi-head | unexecuted / 未执行 |

各家族单次原生作业上限依次为 900/900/1800 秒，包含环境准备、Agent 和评分；六次串行合计 7200 秒。原生 stdout/stderr 各限 1 MiB。**不提供总费用、总 token 或总墙钟时间硬上限**。材料获取/清理开销不包含在上述累计值内；此构建没有原生配置超时覆盖的执行通路。不授权付费重试。采用完整私有捕获，分享前须检查隐私且不能编辑封存字节；无效、取消、未启动、效果未知试验均保留。

## 检查及准备历史

```sh
# cwd: /mnt/d/Luiz/Odradek/opi; retained external Cargo target; incremental enabled
cargo test --locked -p opi-ai --test anthropic_thinking_roundtrip
cargo build --locked -p opi-coding-agent --bin opi --message-format=json-render-diagnostics
CC_x86_64_unknown_linux_musl=gcc cargo build --locked --release --target x86_64-unknown-linux-musl -p opi-coding-agent --bin opi --message-format=json-render-diagnostics
cargo test --locked -p opi-eval --test native_driver native_output_control_client_matrix -- --ignored --exact
```

四条命令均退出 0：六个已有签名测试通过，两种构建成功，一个输出控制测试通过。[诊断命令](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/commands)和[新计划命令](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/next-acceptance/commands)记录精确 cwd、选定的缓存/探针环境、参数、时间与输出。测试影响为 **retain**，未新增、修改或删除仓库测试；独立复现脚本是仓库外诊断证据。本次仅新增文档，不声称执行了 workspace 门禁。

```sh
# cwd: /home/luiz/opi-eval-runs/20260910t154500z-offline-diagnosis-opi082/next-acceptance
# f = terminal-bench-2.1, terminal-bench-3.0, deepswe-v1.1
./builds/opi-eval doctor --config inputs/$f.toml
./builds/opi-eval prepare --config inputs/$f.toml --out materials/$f
./builds/opi-eval validate --config inputs/$f.toml --native-material materials/$f/material.json
./builds/opi-eval plan --config inputs/$f.toml --native-material materials/$f/material.json --out plans/$f.json
```

三个家族最后的 doctor/prepare/validate/plan 全部退出 0；精确绝对参数以及派生、固定摘要后的两次校验见[准备记录](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/next-acceptance/checks/preparation-commands.json)。在三个实际任务镜像内进行的无网络 `opi --version` 检查均退出 0，见[镜像身份与命令](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/next-acceptance/checks/task-image-executable-smoke.json)。

保留准备阶段失败：最初 runner 软链接被拒绝（退出 2），随后使用独立副本；最初对旧运行缓存执行离线 uv sync 缺少构建后端（两个 runner 各退出 1），使用现有全局 uv 缓存后均退出 0。`ubuntu:24.04` 标签检查失败，因为基础材料仅存在于 BuildKit 缓存。完整 `opi-cargo-cache.py status` 长时间扫描后被中止，退出 -15；窄范围缓存解析及本轮 lease 释放退出 0。上述操作都不计为基准试验。

新的原生 run、封存 verify、报告重建、验收验证、捕获审计和原生 regrade 均**未运行**，因为尚无新真实试验；用户也未要求重新评分。新候选真实任务覆盖率为 0/6，六次未执行，无原生分数/分母或真实 token、费用观测。P19-AC-001、P19-AC-003 至 P19-AC-006 的真实部分，以及 P19-AC-012 对新候选仍缺失。既有验收发现仍为 diagnostic/unverified，未进行独立审计准入或实现账本更新。

## 保留、清理与下一步

[清理记录](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/cleanup.json)已完成本轮临时容器范围：代理容器及三个版本检查容器均已不存在，四个可复用任务/代理镜像身份仍可读取。保留复现文件、候选、计划、缓存和原证据。已有 lease 133324 的归属未建立，保持原样；本轮 lease 214323 已释放且不存在。回收空间未知，没有清理镜像或共享缓存。

[保留检查](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/checks/preservation.json)确认原私有证据清单中的 694 个文件及原准备阶段 48 个记录摘要一致，新建 39 文件保护清单在清理后也一致。没有加载或暂存真实凭据；诊断副本含合成值与私有操作路径，仅供本地使用。最终 `python3 scripts/opi-doc-check.py` 和 `git diff --check` 的实际结果保存在[最终检查](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/checks/final-checks.json)。

下一步需批准这个具体候选和六次新增付费试验。批准后依次执行操作范围内的 `proposed_execution` 命令，保留所有结算，再验证封存、重建报告、检查隐私并尝试验收准入。本预览不包含额外源码修复、提交或发布。
