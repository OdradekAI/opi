# TB3 轮数预算修复

已完成授权范围内的修复与离线验证。原生评测生成的 Opi 配置现在显式设置 `[defaults].max_iterations = 4294967295`，即现有 `u32` 字段的最大有限值，避免继承产品默认的 50 轮截断。普通 Opi 默认值、Agent Core、原生超时、取消与历史评分均保持原样；该值是有限轮数上限，不是无限模式或 token 预算。

授权依据：用户在看到 TB3 的 `max turns exceeded (50)` 后要求“继续修复”，适用 `opi-eval` 的用户确认修复分支。本次只进行复现、源码修复、合成测试和候选构建，没有新增付费试验、读取真实凭据、重新评分、提交、发布或修改实现台账。

生产改动仅涉及 Companion 的 Opi 配置投影和配套 Python 离线验收比较。验收器识别新的精确上限，也保留对未写该字段的历史封存投影的验证；其他数值与类型继续拒绝。技能预览现在分别披露有限轮数与原生超时。后续使用新投影时须重新生成客户端证据、material 与 plan。

真实 Opi 程序使用旧投影时，在第 50 次模型请求后失败；使用修复后的投影时，完成 51 轮成功工具调用及最终回答，共 52 次模型请求，退出码 0。对照配置仍分别在默认 50 轮、显式 2 轮时停止，退出码均为 1。自定义模型省略输出能力时仍在请求前拒绝（0 次请求、退出码 2）；支持这种缺省模型能力不属于本次修复。另有九组单元用例覆盖三种 API 及输出控制的数值、omitted、unknown 状态。

验证结果：`opi-eval --all-targets` 共 273 个 Rust 测试通过；其中七个依赖额外物料的测试默认忽略，实际客户端的轮数与输出控制测试已单独执行并通过。Python 验收器 37 个测试全部通过，包括历史/新投影以及错误轮数值。原生超时、取消回归通过；格式检查、定向 clippy、Linux musl 构建通过，doc-test 命令通过但无 doctest。独立只读复核未发现可执行问题。

两个固定客户端再次通过回环证明：首个请求及工具续轮均发送 `max_tokens = 131072`，声明上下文为 1000000。新 Companion 对上次 TB3 原始 run 执行验证成功，两次重建报告互相字节一致，并与上次报告一致。四个受保护证据根共 2358 个索引文件全部保持原摘要。历史原生得分仍均为 0；离线验证不能证明任务成功率提高，也未修复 pi 之前的连接失败。

范围为 worktree-only：HEAD 未变，本次没有 committed/staged 改动。本次独立修改 `crates/opi-eval` 下的 `src/experiment.rs`、`scripts/verify-local-acceptance.py`、`scripts/test_verify_local_acceptance.py`；有既存修改并由本次继续触及的文件为 `tests/native_driver.rs`、中英文 crate README、`CHANGELOG.md`、技能入口与 native-run 参考。其他既存修改均保留。测试影响：**add** 两个 Rust 回归和一个 Python 回归；**update** 合成证据 fixture；**retain** 原有监督及验收测试；无删除。中英文 README 和本报告已同步；技能参考及 changelog 没有单独中文版。

检查证据保留了最初的编译错误、真实失败复现及完整 crate gate 捕获的验收器不匹配。通用技能校验器因不支持仓库既有 `disable-model-invocation` frontmatter 而退出 1；保留该显式用户调用规则，没有删除字段迎合校验器。仓库权威检查 `python3 scripts/opi-doc-check.py` 与 `git diff --check` 通过，最终记录随证据保存。

清理：测试生命周期已关闭隔离测试目录和监听器，本次 Cargo 租约已释放。共享缓存（观察值 149.85 GiB）、候选及历史证据保留。Docker 检查为 partial：引擎未启动且 WSL Docker executable 不可用，本次没有创建或删除真实 Docker 资源。详见清理记录。

尚未新增付费 TB3 复跑；后续仍需重新准备并授权新的冻结执行范围。保留的 probe 输入不等于 native plan。TB3 实际边界仍为原生 job 18000 秒、Agent 14400 秒、verifier 900 秒，不宣称总 token 或金额封顶。后续不指定 Opi 版本时，仍在当次准备时使用最新本地工作树源码。

| Command (Linux cwd: `/mnt/d/Luiz/Odradek/opi`) | Exit | Record |
|---|---:|---|
| `cargo fmt --check --all` | 0 | [fmt](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/fmt.json) |
| `cargo clippy --locked -p opi-eval --all-targets -- -D warnings` | 0 | [clippy](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/clippy.json) |
| `cargo test --locked -p opi-eval --all-targets` | 0 | [crate-tests-final](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/crate-tests-final.json) |
| `cargo test --locked -p opi-eval --doc` | 0 | [crate-doctests](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/crate-doctests.json) |
| `python3 crates/opi-eval/scripts/test_verify_local_acceptance.py` | 0 | [green-acceptance](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/green-acceptance.json) |
| `/usr/bin/env CC_x86_64_unknown_linux_musl=gcc cargo build --locked --release --target x86_64-unknown-linux-musl -p opi-coding-agent --bin opi -p opi-eval --bin opi-eval --message-format=json-render-diagnostics` | 0 | [release-final](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/release-final.json) |
| `/usr/bin/env OPI_EVAL_OPI_BINARY=$CANDIDATE/builds/opi cargo test --locked -p opi-eval --test native_driver native_opi_turn_budget -- --ignored --exact --nocapture` | 0 | [candidate-turn-budget](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/candidate-turn-budget.json) |
| `/usr/bin/env OPI_EVAL_NATIVE_CLIENT_MATERIAL=$CANDIDATE/clients.json OPI_EVAL_OUTPUT_CONTROL_OUT=$CANDIDATE/output-control cargo test --locked -p opi-eval --test native_driver native_output_control_client_matrix -- --ignored --exact` | 0 | [candidate-output-control](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/candidate-output-control.json) |
| `$CANDIDATE/builds/opi-eval verify --root /home/luiz/opi-eval-runs/20260911t141317z-tb3-repair/runs/terminal-bench-3.0` | 0 | [historical-verify](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/historical-verify.json) |
| `$CANDIDATE/builds/opi-eval report --root /home/luiz/opi-eval-runs/20260911t141317z-tb3-repair/runs/terminal-bench-3.0 --out /mnt/e/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/historical-report-a.json` | 0 | [historical-report-a](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/historical-report-a.json) |
| `$CANDIDATE/builds/opi-eval report --root /home/luiz/opi-eval-runs/20260911t141317z-tb3-repair/runs/terminal-bench-3.0 --out /mnt/e/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/historical-report-b.json` | 0 | [historical-report-b](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/historical-report-b.json) |

- `$CANDIDATE = /home/luiz/opi-eval-runs/20260912t201159z-tb3-turn-budget-repair`
- Source HEAD: `55fbc95009e6f47145196dbc539454ebcb1cf692`; package `0.8.2`, current dirty working tree.
- Source inventory (873 files): `95b8f2a8988a7a5a9ce36f5a97707a4fb0fafe76cc48b1f9e9997c13e60fa647`.
- Working-tree patch: `5e722541c88c68ffa80f55a20637ad9ee145c04c4cbe8bf014fbf8278618b7d8`.
- Cargo.lock: `261a3ec501be2e045dc6c9b2b9a940adc1c56305f477008eea940f26545925f3`.
- Opi SHA-256: `62ea32cae8f95448cfb502dcd3ea0b96e793c14c366c8b046a4974617c94a955`.
- Companion SHA-256: `3d051747ff1c9c77ba6d872d2699bf4d4a1b2f49f1c410a5dfd4832b0cab06ea`.
- Opi output-control proof: `f3152169f9f3873a041185e1f0e2e84e488569c252f83880e63b6e5cbc902dde`.
- pi output-control proof: `310e6395b153a02b656af258536e598bea6188a03ec0fa2d3bd1aaf2806536a5`.
- Reconstructed historical report SHA-256: `9c8e8eba8f0170d0f62452630115a32f108da14cf14731f50a17e876d4aa1bdf`.

[Candidate identity](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/candidate/candidate-identity.json) · [Evidence index](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/evidence-index.json) · [Cleanup record](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/operator/cleanup.json) · [Worktree scope](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/operator/final-worktree.json)
