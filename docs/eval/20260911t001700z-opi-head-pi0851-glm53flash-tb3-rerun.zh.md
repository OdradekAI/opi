# 更新模型参数后的 TB3 复跑

[English](20260911t001700z-opi-head-pi0851-glm53flash-tb3-rerun.md)

**已执行并结算两次试验，但未形成完整有效对照。** pi 被外层原生作业时间预算截停，没有评分；Opi 以 `provider stream failed` 退出，独立验证器对已收集环境给出原生奖励 0，5 项测试全部失败。Companion 将 Opi 归为已评分的 Agent 失败，将 pi 归为无效执行。不能据此比较两端正常解题能力。

私有证据位于 [E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence)，[证据索引](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/evidence-index.json) SHA-256：`e83315c24911f8d3ac9325c5fcd29424a1789fc32d134a89db75463f9cb329a8`。[规范化报告](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/reports/terminal-bench-3.0-a.json)、[语义核对](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/checks/semantic-review.json)与[用量记录](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/reports/telemetry.json)保留了这些区别。本轮只复跑原 TB3 对照任务，共两次，按 benchmark 子集记录；不替换此前六次验收清单，也不关闭 Phase 或实施任务。

## 范围与身份

用户在更新 opi-eval 后要求“复跑tb3”。本轮沿用已有明确选择：Opi／pi、pi 0.85.1、GLM-5.3-Flash、原 TB3 任务与 `.env` 凭据引用。未指定 Opi 版本，因此从当前本地工作树进行锁定构建。[授权与冻结范围](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/operator-scope.json)明确限定两次试验；没有追加付费重试或修复源代码。构建前后源码输入清单一致，已有技能／文档改动保留；[构建目录](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/builds)保存源码快照、差异及二进制身份。

| Item | Identity |
|---|---|
| Opi | Local working-tree candidate, package 0.8.2; HEAD `55fbc95009e6f47145196dbc539454ebcb1cf692`; existing documentation/skill changes retained; no runtime change in this rerun |
| Opi executable SHA-256 | `d7e5ab2d2636b04bfefafdeb2285bf50933247e7fc06836b6778d91ff4056e60`; Linux musl; locked build returned the current compiler artifact; same bytes as prior candidate |
| pi | 0.85.1; source `d981de1229ef899957bbe968bc8dcda02a21f477`; previously acquired standalone package revalidated |
| pi executable / package SHA-256 | `443bd83f30e4dbc7bac2eed9c6aa2461b9a15016fd555f48c92a0591d028c403` / `0701993b1f4ddd9f7b4a6b2e9ecd963b3ac9457aca0807b71d4eb98339343aae` |
| Companion | Separately rebuilt opi-eval 0.8.2, Linux GNU; SHA-256 `c730f1fe85a5eed9cb8c2207572ac551dca7e69d8d1a0feb6537e87465d377ee` |
| TB3 / Harbor | TB3 commit `2b0442c3c583b710ca8da14c8e601b99f2f1f244`; Harbor `4407eb5227a2ff4f0d3f16b2eb48849382fdf276` |
| Experiment digest | `451736416157c883e7de81e2060aee01f975b655e6fa5ceede3de9fa5d581623` |
| Plan digest | `ebec713871be25f4de4c9945e531e5de14cef8012673bddf81b58721ee649de4` |
| Material SHA-256 | `85c306b3cbd987fce1010f0a621dd158a75d897888eb98c8036f19c15852a962` |
| Config SHA-256 | `b7c3df1ae3a8cd1c15a94ec0744a064d9a92b4f4f480584618993409affaa9e6` |
| Evidence index SHA-256 | `e83315c24911f8d3ac9325c5fcd29424a1789fc32d134a89db75463f9cb329a8` |

环境为 WSL Ubuntu/Linux x86_64 与 Docker Linux 引擎。doctor 观测到 32 个 CPU、16,611,237,888 字节内存、可用 NVIDIA GPU 和足够磁盘；任务需要 1 CPU、4 GiB 内存，不需要 GPU。材料与基线记录保留任务、runner、镜像身份。本地已准入库存只有 `batched-eval-parity` 一项，不能视为完整上游 TB3 测试集。

两端连接均为 `zhipu:glm-5.3-flash`、`anthropic-messages`、`https://open.bigmodel.cn/api/anthropic`。仅在启动时读取用户此前指定的 `.env` 凭据，报告不包含密钥值。必需控制为 **1,000,000 上下文／131,072 单次输出 tokens**，temperature、top_p、reasoning 均 omitted。[两端本地请求证明](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/checks/output-control-r2)确认实际首轮和工具续轮均发送 `max_tokens=131072`，且已绑定冻结材料；官方[模型](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/checks/official-docs/glm-5.3-flash-recheck.md)、[API](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/checks/official-docs/chat-completion-recheck.md)、[端点](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/checks/official-docs/anthropic-endpoint-recheck.md)文档已复核。声明上下文容量不等于实际验证了服务端消费 1M 输入。

每端一次，串行，max_trials=2，先 pi 后 Opi。沿用的 profile 对**整项原生作业设置每端 900 秒上限**，覆盖准备、Agent 与评分；任务本身允许 **Agent 14,400 秒、验证器 900 秒**，与外层预算是不同限制。原生 stdout/stderr 各限 1 MiB，独立 Agent 捕获可以更大。Opi 仍为 max_iterations/max_turns=50，pi 0.85.1 没有同类固定轮数上限；不保证累计 token、账单或总墙钟时间上限。[执行记录](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/commands/run-terminal-bench-3.0.json)：`2026-09-11T00:23:36.037394+00:00` 至 `2026-09-11T00:43:57.897748+00:00`，共 1221.86 秒。

## 原生结果与覆盖

| TB3 软件／任务 | 计划／尝试 | 原生奖励 | Companion 有效／无效 | 实际停止原因 |
|---|---|---|---|---|
| pi 0.85.1／batched-eval-parity | 1／1 | 无评分，分母 0 | 0／1 | 外层作业超时；未封存 `grader/verifier-timeout`，graded=false |
| Opi 本地 HEAD 0.8.2／batched-eval-parity | 1／1 | 0，分母 1 | 1／0；Agent failure=1 | 退出 4，`provider stream failed`，原生 `NonZeroAgentExitCodeError` |

以上照录 Companion：corpus=1、selected=1、planned=2、attempted=2、valid=1、invalid=1、unexecuted=0。唯一对照组因 pi 无效被排除。Opi 的原生零分保留，但其流错误尚未定位，不能视为正常任务完成或完整能力比较；本轮不声明可供验收交接。

- [pi 结算](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/runs/terminal-bench-3.0/trials/trial-baseline-pi-tb3-rerun-batched-eval-parity-r1/bundle/artifacts/control/settlement.json)：未封存诊断日志中可见 16 次完整模型响应、32 次工具调用（11 bash、19 read、1 write、1 edit）。收集与评分未完成，没有从尝试中的编辑反推分数。
- [Opi 原生结果](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/runs/terminal-bench-3.0/trials/trial-candidate-opi-working-tree-tb3-rerun-batched-eval-parity-r1/bundle/artifacts/native/trial-result.json)：10 个 provider turns、23 次工具（6 bash、17 read），Agent 277.44 秒，原生 trial 总耗时 317.14 秒，验证器 30.69 秒。仅声明的 `/app/evalbench/` 进入独立验证器，收集摘要 `9fafdfb3805cde053c0b8794a9d63998aa8dc6f9656e65a14d42913b271935a7`。5 项原生测试全部失败，封存结果与 runner 原始结果逐字节一致。

**两端停止均不是触及 50 轮上限。** Opi 仅记录通用流错误，传输、服务端或解析器的具体责任尚未确定；本轮没有声称已经复现确定的源码缺陷，也没有实施修复。

## 用量与比较限制

| 软件 | 完整响应输入 tokens | 输出 tokens | 缓存读取 tokens | 观测范围 |
|---|---:|---:|---:|---|
| pi | 67,998 | 34,908 | 254,976 | 16 次完整响应；被中断响应的用量未知 |
| Opi | 23,291 | 4,297 | 81,600 | 终态 session summary；失败流可能存在未上报用量 |

pi 有一条 **20,719-token** 响应正常进入工具续轮，说明真实运行已突破原来的 8,192 上限。提高单次输出额度并未确保能在不变的外层时间预算内完成。传输重试、压缩、被中断响应的完整用量和实际账单均未知；模型配置中的零价格不代表零费用。

此前 TB3 两端均有原生零分，Agent 耗时为 pi 262.98 秒／Opi 343.26 秒。本轮改变了参数且停止条件不同，不足以支持速度或能力优劣结论。[此前报告](20260910t175447z-opi-head-pi0851-glm53flash-proxy-fixed-acceptance-live.zh.md)及其历史证据保留。

## 验证、隐私与清理

```sh
# cwd for native/check commands:
R=/home/luiz/opi-eval-runs/20260911t001700z-tb3-rerun
EVAL="$R/builds/opi-eval"
CONFIG="$R/inputs/terminal-bench-3.0.toml"
MATERIAL="$R/materials/terminal-bench-3.0/material.json"
PLAN="$R/plans/terminal-bench-3.0.json"
ROOT="$R/runs/terminal-bench-3.0"
"$EVAL" doctor --config "$CONFIG"
"$EVAL" prepare --config "$CONFIG" --out "$R/materials/terminal-bench-3.0"
"$EVAL" validate --config "$CONFIG" --native-material "$MATERIAL"
"$EVAL" plan --config "$CONFIG" --native-material "$MATERIAL" --out "$PLAN"
"$EVAL" run --plan "$PLAN" --root "$ROOT" --fixtures "$R/unused-fixtures" --canaries "$R/private/live-canaries.txt"
"$EVAL" verify --root "$ROOT"
"$EVAL" report --root "$ROOT" --out "$R/reports/terminal-bench-3.0-a.json" --canaries "$R/private/live-canaries.txt"
"$EVAL" report --root "$ROOT" --out "$R/reports/terminal-bench-3.0-b.json" --canaries "$R/private/live-canaries.txt"
# cwd=/mnt/d/Luiz/Odradek/opi; shared external Cargo cache, CARGO_INCREMENTAL=1
cargo build --locked --release --target x86_64-unknown-linux-musl -p opi-coding-agent --bin opi --message-format=json-render-diagnostics
cargo build --locked --release --target x86_64-unknown-linux-gnu -p opi-eval --bin opi-eval --message-format=json-render-diagnostics
```

两次构建、doctor、prepare、原生 validate、plan 均退出 **0**。首个本地探针退出 **4**，没有发送 HTTP 请求；原因是操作者使用的 D-Bus pathname 地址阻止凭据回退。仅改成仓库测试所用的隔离 abstract 地址后，两端探针均通过，无需修改 Opi。汇总打印助手也遇到 pi 没有原生结果路径的情况；[诊断记录](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/checks/operator-helper-diagnostic.json)说明只修正报告读取方式，未改变任何原生证据。

原生 `run` 退出 **1**，对应对照不完整。原目录和暂存副本 `verify` 都退出 **1**：Opi 封存通过，pi 缺少清单产生 `io`；通用 `mutation-detected` 标签不表示观测到了 Opi 字节被修改。四次 `report` 均退出 **0**，字节完全相同，SHA-256：`b8692d3e53fdf0dc76600027449565b8819839a97526850c50f552b48e9840ae`。[暂存复验](E:/opi/evaluations/20260911t001700z-tb3-rerun/checks/staging-verification.json)保存精确 argv、cwd、退出码与比较结果。没有执行原生 regrade 或再次调用模型。仓库文档检查另存于 [documentation-check](E:/opi/evaluations/20260911t001700z-tb3-rerun/checks/documentation-check.json)。

选定密钥 canary 扫描覆盖 177 个已结算运行／启动记录文件，命中为 0。完整捕获和源码快照仍含个人路径、私有信息，因此证据根**仅供私有使用，未批准公开分享**；封存字节没有脱敏改写。[最终清理](E:/opi/evaluations/20260911t001700z-tb3-rerun/checks/cleanup-final.json)已完成：临时 canary、可丢弃操作者目录已删除，运行拥有的容器剩余 0，构建租约已释放，本地监听器已停止。792 字节 Docker 客户端日志作为私有诊断保留；可复用镜像、共享缓存与原始／暂存证据保留，回收空间未知。复制后运行清单一致，此前验收清单摘要未变。

测试影响：**none**；没有修改 Rust 或测试文件，仓库只新增本组中英文评测报告。[Eval findings](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/reports/findings.jsonl)属于未验证的操作者证据，不进入活动审计／修复集合。后续需要定位 Opi 流错误；如再请求实跑，应先明确外层作业预算与任务较长预算的关系，重新冻结计划。本轮没有自动追加付费重试。
