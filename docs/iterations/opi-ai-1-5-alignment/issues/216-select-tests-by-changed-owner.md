# #216 从改动路径选择有效的测试集合

Status: done
Owner: opi-ai 测试选择入口与 tests/README
Parent: [#209](209-maintain-module-test-and-catalog-boundaries.md)
Blocked by: [#215](215-own-integration-test-targets.md)

有了 target/契约索引以后，开发者需要可执行的日常入口。共享搭建、业务 wire、跨 provider 的公开合同具有不同影响范围。

## 验收

提供能说明选择理由的入口。单 wire 只选该 wire、直接契约和必要跨层用例。改变共享 helper 选所有实际使用方；改变 Cargo feature 或无法覆盖的路径报告需扩大检查。保留显式全量入口。

## 验证

用 #210 的三种 classifier 路径、一个共享 helper、一个 feature 修改、一个未映射路径作输入示例。验证输出目标存在、feature 满足、过滤非零；至少实际跑单 wire 和共享 helper 的代表用例。选择器的 CLI 拒绝无效 target。

## 实施与复核 · 2026-11-20

从仓库根目录运行 `python scripts/pi-sdk-conformance/select_tests.py <改动路径>…`。入口合并 #215 的目标职责、Cargo 的实际源路径与 required-features，逐目标给出理由和可执行命令。多个输入取并集，某个输入需要完整目标时覆盖同一目标上的窄过滤。文本和 `--json` 都显示需扩大的原因。[中英文用法与维护入口](../../../../crates/opi-ai/tests/README.zh.md#按改动选测)

| 改动 | 如何选择与验收 |
| --- | --- |
| 单 classifier wire | 从 3 个已审查协议的实际调用方取直接用例、共用分派和跨层消费者。只构造 TypeSafe 的同名风格用例留给 TypeSafe。 |
| `system_one.rs`、mapped、OpenAI Chat/Responses profile | 共享 classifier 实现运行整个 `classifier_contract`；3 个 profile 分别核对映射、装配、对应 wire 或外部 SDK。 |
| 集成目标、topic、共享 helper | 从 Cargo 声明的源文件跟踪外部模块、`#[path]` 与字面量 `include!`。一个源文件的 11 个 profile 目标保留各自 feature。WS peer 额外覆盖私有升级测试和 WS 归档入口。 |
| Cargo feature、collection、未映射路径 | 显示扩大原因，保留已知路径的精确命令，并列出默认及 minimum 回归入口。feature 改动还需检查受影响 owner 的单独配置及归档消费。 |

当前有 7 条已审查的生产路径规则。加入新规则时，以实际构造和调用链确认测试归属；迁移测试名时，用相同 feature 的 `cargo test … -- --list` 确认每个过滤条件命中，再执行受影响用例。相关 runner 单独列出，可按改动复核包消费、矩阵和架构检查。

12 个代表输入覆盖 35 个不同的 Cargo 目标与 feature 配置。用真实 libtest 清单核对每个过滤条件均有命中。Decisions 选中 5 个目标、27 项，全部实跑通过；Cloudflare 选中 3 个目标、18 项，llama.cpp 选中 2 个目标、20 项。改变 `pending_auth.rs` 选中 classifier、Deferred 和 OpenRouter 三个实际消费者，158 项全部实跑通过。feature、未映射路径、混合输入及显式全量均给出扩大计划，非法 target 的 CLI 退出码为 2。[逐路径命中数、实跑范围与负例](../evidence/issue-216-focused-selection.json)

Test impact: add 选择器正反例与按路径执行验证；retain 原有 Rust 业务断言及显式全量入口。
