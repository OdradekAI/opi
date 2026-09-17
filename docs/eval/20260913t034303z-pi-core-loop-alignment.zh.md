# pi 核心循环对齐修复 — 2026-09-13

获准的源码修复已落地；完整工作区检查仍因规范修订后的实现账本哈希未同步而未通过。Agent Core 不再内置轮数上限，产品默认不设置有限轮数预算；实际 Opi 与 pi 客户端均完成相同的离线任务：51 轮工具调用，加一次最终回答。Opi 显式两轮预算仍会拒绝第三次请求。本报告记录源码修复验证，不是新的 TB3 得分或付费复跑结果。

## 授权与范围

用户“同意”批准了此前提出的修复及中英文规范第 7.2 节修订，按 opi-eval 的用户确认源码修复例外执行。[获准计划](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/plan.md)与[基线](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/baseline.json)保存授权和初始工作区清单。未提交、暂存、创建分支、修改实现账本、重新评分、解析真实凭据或执行新增付费试验。

`opi-ai` 继续负责 provider 无关的请求准备和流处理；`opi-agent` 负责通用的完成、停止、队列与下一轮状态转换；`opi-coding-agent` 负责可选执行预算策略；`opi-eval` 删除此前投影中的 `u32::MAX` 临时绕过。本次对齐的是受影响的循环语义，不代表 ai/agent 的所有功能已经完全等价。

中英文规范现要求：在已完成状态上先判断停止，再轮询队列，仅在确需续轮时准备下一次请求。历史 Phase 快照和此前评测报告仍作为历史证据保留。固定本地 pi ai/agent 包清单均为 0.85.1；六份参考文件及哈希保存在 [pi-reference.json](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/pi-reference.json)。该参考目录不是独立 Git checkout，因此没有把 Opi 的 HEAD 当作 pi revision。

## 最终行为

- 删除 `AgentLoopConfig.max_turns`。`PrepareNextTurnContext.turn` 改为每次调用内已完成轮数的 `u64` 计数器，不是预算。
- 完成一轮后：判断停止 → 选择 steering 和适用的 follow-up → 确需续轮才准备 → 校验并原子替换完整状态 → 下一次请求。准备钩子返回 `Some` 不会自行产生额外一轮；工具终止提示只停止自动工具续轮，队列中的工作仍可继续。
- 准备失败或取消时，保留完整状态的原子性及未送达输入。独立复核还发现首轮请求前失败可能丢失 steering，现已修复：上下文转换、消息转换、认证准备或取消失败均恢复未派发输入；已经派发的输入不会重复入队。
- `defaults.max_iterations: Option<u32>` 默认 `None`。显式整数按一次 prompt/continuation 调用生效，provider 重试仍属于同一逻辑轮。恰好在边界自然完成则成功；需要额外一轮时返回 `MaxTurnsExceeded`；零预算在 provider 请求前失败。下一次调用重新计数。
- 有显式预算时证据保留数值 quota；省略时记录 `Unknown(NotApplicable)`。诊断继续保留类型化原因，并提示调整调用方预算。
- 当前 Opi 评测投影省略 `max_iterations`。历史封存投影中的精确旧绕过值仍可验证。未指定 Opi 版本时使用当前源码的规则保持有效；单次输出上限、上下文容量与外部时限是独立控制。

产品现有持久化会话压缩仍由产品协调器负责；本修复没有把该协调器移入核心 hooks。

## 候选与实际客户端证据

候选目录：`/home/luiz/opi-eval-runs/20260913t034303z-pi-core-loop-alignment-verified`。源码 HEAD：`55fbc95009e6f47145196dbc539454ebcb1cf692`，包版本为 0.8.2 加本次记录的工作区修改，不能视为原版 0.8.2 发布包。两个二进制使用 release profile、工作区默认 features 和 `x86_64-unknown-linux-musl`。

源码清单 SHA-256：`93cb48d760f9f508d215b0d5843943a1fac798cac0027bbd7b44d60d0054e79c`。工作区补丁 SHA-256：`82d9ed30d5659fce361982a5d1de10529bcebce1e9245879df53afb38b906701`。[候选身份](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/evidence/candidate/candidate-identity.json)还记录 Cargo.lock、工具链、构建命令、源码清单和独立二进制路径。

| Executable | SHA-256 |
|---|---|
| opi | `c89ac4a103c6cf0f433469ae45dc6baea03328c3937b8c412f67e8f0457ff8de` |
| opi-eval | `bcfc147973de76579013b442860f7470c17b1d00b8233d1b5224975242f81812` |

```text
default: requests=52, exit=Some(0)
explicit-two: requests=2, exit=Some(1)
projected: requests=52, exit=Some(0)
pi-projected: requests=52, exit=Some(0)
omitted-output: requests=0, exit=Some(2)
```

实际客户端使用相同回环 provider 和合成凭据。Opi 默认配置、投影配置以及固定 pi 均成功完成 52 次请求；Opi 显式两轮预算停在两次请求。自定义 Opi 模型省略输出能力时，在请求前被拒绝。所有已准入请求均保留单次回答输出上限 131072，声明的上下文容量为 1000000。另一组输出控制矩阵也在两个客户端上通过，将精确 executable 和 pi runtime package 与实际请求绑定。

## 验证与复核

工作区全目标测试 **4452 项通过**、**1 项失败**（规范账本哈希检查）、**12 项忽略**；文档测试 **12 项通过**。离线 Python acceptance verifier 的 37 项测试通过。两个显式选择的实际客户端测试过滤器均通过；被忽略的 live/client-material 测试没有因环境中存在凭据而自动启用。

修复前的有效失败复现了默认 50 轮停止、旧准备/队列顺序和首轮输入丢失。初始测试编译错误及认证错误类型预期写错的尝试均保留记录，但不算语义复现证据。除明确列出的规范账本哈希检查外，下列最终检查均通过。[核心报告](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/core-report.md)与[独立复核](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/independent-review.md)保留聚焦命令和问题关闭记录；没有未解决的可操作复核发现。

所有命令从 `/mnt/d/Luiz/Odradek/opi` 执行，通过已记录的净化环境及带 lease 的命令包装器使用外部 Cargo 缓存，并启用增量编译。精确 argv、cwd、起止时间、退出码和日志均已保留：

首次工作区检查在一项测试启动前中止：对应缓存 ELF 含 8189 条非法相对重定位记录，逐字节复制到 Linux 文件系统后仍同样失败。单独构建的该测试三项均通过。仅刷新该测试源码时间戳以触发 Cargo 重新链接，源码字节和依赖设置未变，没有删除缓存；最终工作区复跑已能正常加载该测试。[加载器诊断](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/loader-diagnosis.json)和[重新链接记录](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/startup-test-relink.json)保留了这次构建产物故障，其底层成因尚未确定。

下一次并行工作区运行通过了重新链接的测试，随后两个未修改的 OAuth 测试在发送合成授权码时失败；它们使用 100 毫秒实际时钟期限。同一测试程序串行执行时 122 项全部通过，断言未变。最终完整工作区验证使用 `RUST_TEST_THREADS=1`，未修改 OAuth 测试或产品行为。此前未通过的工作区尝试均保留在验证清单中。

后续 RPC fixture 还假定接受 prompt 就代表请求已经派发。新的首轮 steering 规则下，立即发送的消息可能正确地进入首轮。现复用已有 mock stream 控制器，等待 `MessageStart` 后发送 steering，再释放流；测试断言恰好两次请求且消息出现在第二次。RPC 80 项测试全部通过，该修订也经过独立复核。最终候选记录完整的已测源码，其运行时二进制与此前离线候选逐字节一致，前一候选另行保留。

| 命令 | 退出码 | 记录 |
|---|---|---|
| `cargo fmt --check --all` | 0 | [workspace-fmt-verified](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/workspace-fmt-verified.json) |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | 0 | [workspace-clippy-verified](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/workspace-clippy-verified.json) |
| `/usr/bin/env RUST_TEST_THREADS=1 cargo test --locked --workspace --all-targets --no-fail-fast` | 101 | [workspace-tests-all-recorded](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/workspace-tests-all-recorded.json) |
| `cargo test --locked --workspace --doc` | 0 | [workspace-doctests-final](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/workspace-doctests-final.json) |
| `/usr/bin/env RUSTDOCFLAGS=-D warnings cargo doc --locked --workspace --no-deps` | 0 | [workspace-doc-final](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/workspace-doc-final.json) |
| `cargo test --locked -p opi-eval --all-targets` | 0 | [eval-all-targets](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/eval-all-targets.json) |
| `python3 crates/opi-eval/scripts/test_verify_local_acceptance.py` | 0 | [python-acceptance](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/python-acceptance.json) |
| `/usr/bin/env OPI_EVAL_NATIVE_CLIENT_MATERIAL=/home/luiz/opi-eval-runs/20260913t034303z-pi-core-loop-alignment-verified/clients.json cargo test --locked -p opi-eval --test native_driver native_client_turn_budgets -- --ignored --exact --nocapture` | 0 | [candidate-turn-budget-verified](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/candidate-turn-budget-verified.json) |
| `/usr/bin/env OPI_EVAL_NATIVE_CLIENT_MATERIAL=/home/luiz/opi-eval-runs/20260913t034303z-pi-core-loop-alignment-verified/clients.json OPI_EVAL_OUTPUT_CONTROL_OUT=/home/luiz/opi-eval-runs/20260913t034303z-pi-core-loop-alignment-verified/output-control cargo test --locked -p opi-eval --test native_driver native_output_control_client_matrix -- --ignored --exact` | 0 | [candidate-output-control-verified](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/candidate-output-control-verified.json) |
| `/usr/bin/env CC_x86_64_unknown_linux_musl=gcc cargo build --locked --release --target x86_64-unknown-linux-musl -p opi-coding-agent --bin opi -p opi-eval --bin opi-eval --message-format=json-render-diagnostics` | 0 | [release-verified](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/release-verified.json) |
| `/home/luiz/opi-eval-runs/20260913t034303z-pi-core-loop-alignment-verified/builds/opi-eval verify --root /home/luiz/opi-eval-runs/20260911t141317z-tb3-repair/runs/terminal-bench-3.0` | 0 | [historical-verify-verified](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/historical-verify-verified.json) |
| `/home/luiz/opi-eval-runs/20260913t034303z-pi-core-loop-alignment-verified/builds/opi-eval report --root /home/luiz/opi-eval-runs/20260911t141317z-tb3-repair/runs/terminal-bench-3.0 --out /mnt/e/opi/evaluations/20260913t034303z-pi-core-loop-alignment/historical-report-a-verified.json` | 0 | [historical-report-a-verified](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/historical-report-a-verified.json) |
| `/home/luiz/opi-eval-runs/20260913t034303z-pi-core-loop-alignment-verified/builds/opi-eval report --root /home/luiz/opi-eval-runs/20260911t141317z-tb3-repair/runs/terminal-bench-3.0 --out /mnt/e/opi/evaluations/20260913t034303z-pi-core-loop-alignment/historical-report-b-verified.json` | 0 | [historical-report-b-verified](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/historical-report-b-verified.json) |

最终文档与 diff 检查分别保存在证据目录的 `checks/doc-check-final.json` 和 `checks/diff-check-final.json`；机器[验证汇总](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/verification-summary.json)列出包括已被修复的失败尝试在内的全部命令。

测试影响：**add** `continuation_contract.rs`、`turn_budget.rs`；**update** 受影响的核心/产品 hook、队列、配置、证据和调用方 fixture，以及原生客户端轮数 fixture；**retain** 权限、取消、重试、provider 失败、会话前缀、用量与历史证据保护。没有删除测试文件。本任务未修改 `opi-ai` 或 Cargo 依赖声明。

## 历史证据、清理与交付

剩余工作区失败为 `live_ledger_spec_hash_matches_current_spec`：账本记录 `cc7f8898f60c0d8abaa667f4b49b7affc721412e75dd3a67dcde37a783e1bc4c`，获准修改后的英文规范归一化哈希为 `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db`。[AGENTS.md](D:/Luiz/Odradek/opi/AGENTS.md:103) 明确规定 “`opi-implement` alone owns `.opi-impl-state.json`”，第 107 行禁止手工编辑该文件。[单字段修改提案](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/ledger-hash-proposal.patch)已准备但未应用，仅修改 `spec_files_sha256["docs/opi-spec.md"]`，不修改 Phase、任务或进度。需由所属流程同步并复验账本测试和最终检查，本修复未绕过或削弱该检查。

五个受保护历史证据索引及其 3365 个文件在修复前后均验证未变。新 Companion 验证了此前 TB3 root，并向两个独立路径重建报告，结果逐字节一致，SHA-256 仍为 `9c8e8eba8f0170d0f62452630115a32f108da14cf14731f50a17e876d4aa1bdf`。没有修改或重新评分此前试验。

私有修复证据：`E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/evidence`；其中 `evidence-index.json` 绑定候选、源码、上游参考、命令记录和双语报告。该目录含本地路径和合成 fixture 凭据，未声明为公开导出。索引身份另存于 `E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/final-index-identity.json`。

测试临时目录及回环子进程由 fixture 生命周期释放，命令 lease 已结束。新候选、共享 Cargo 缓存与全部历史材料保留。WSL 中 Docker 不可用，因此共享 image/container/volume/network 清理仍未验证；本操作没有修改 Docker 资源。[清理记录](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/cleanup.json)区分已完成的本次临时资源清理和该检查缺口。

候选已保留；需先由所属流程同步规范账本哈希，才能声明完整工作区检查通过。后续 live run 需生成新的 material、控制证据及 plan 身份，并取得对应精确范围的授权。离线通过不能作为新的 TB3 结果或性能对比结论。
