# #215 列清每个集成测试 target 的职责和最小 feature

Status: done
Owner: opi-ai Cargo manifest 与测试索引
Parent: [#209](209-maintain-module-test-and-catalog-boundaries.md)
Blocked by: none

上轮记录 102 个根测试源文件、112 个 Cargo test target；同一共享源可以生成多个 feature 专用 target。根文件数量不能用作测试计划。

## 验收

逐项记录 target、源路径、契约/用例族、层级、feature、成本、package runner 依赖，区分稳定测试工具与经常变化的 wire 场景。为当前 19 个自动发现的目标补显式声明，仅对实际需要 feature 的目标设置 `required-features`。核对映射中的每个用例及 minimum 下应运行的目标。

## 验证

以 `cargo metadata --no-deps --format-version 1` 核对 manifest 与实际 target 集合（名称映射路径和 required-features），无未归属或虚构 target。核对改前改后 `--list`，在 minimum/默认/相应 owner 下执行变更过的目标、报告真实用例数。

Test impact: add/更新 target 索引；retain 现有用例及 package runner 指向的 case。

## 实施与复核 · 2026-11-20

原先自动发现的 19 个目标在 minimum 下都能运行，现已逐项写入 Cargo.toml，均无 `required-features`。93 个原显式声明继续保留。修改前后 Cargo 的 112 个 target 名、源路径和 feature 准入条件逐项相等，共覆盖 102 个根源文件。11 个 provider 专用 target 仍以不同 feature 编译同一文件。

[测试职责清单](../../../../crates/opi-ai/tests/targets.tsv)保存 owner、层级、验证行为、定性成本和具名 runner。源与 feature 由 Cargo 提供，避免维护两份配置。`python scripts/pi-sdk-conformance/check_test_inventory.py --list` 输出合并视图，不触发构建。35 个目标有直接归档、矩阵、差分或 CI 绑定。其余目标通过通用 Cargo 入口选择。共享 helper 的位置和维护约定见[测试指南](../../../../crates/opi-ai/tests/README.zh.md)。

| 配置 | 改前/改后逐名比较 | 完整执行 |
| --- | --- | --- |
| minimum，全部 19 个目标 | 432 / 432，相同 | 432 通过 |
| 默认，全部 19 个目标 | 433 / 433，相同 | 433 通过 |
| 仅 `telemetry-tracing`，`core_profile` | 第四项为 tracing carrier | 4 通过 |

具体目标、两份用例清单的摘要和实际归档验证步骤在[运行回执](../evidence/issue-215-test-targets.json)。复核时对回执的 `per_target` 各键传入 `--test NAME`，分别运行 `cargo test --offline --locked -p opi-ai` 的 minimum（附 `--no-default-features`）和默认配置；加 `-- --list` 可重取用例清单。以 target 为键、排序后的用例名数组为值，将该映射按键排序紧凑序列化为 JSON 后取 SHA-256。

minimum 包验证器复制并运行四个原有目标。首次运行在 rustdoc 阶段发现 13 个失效的公共类型/方法链接，位于 API 合同、Provider operation 和 Models 拆出的模块。本批显式链接到 crate 的公开类型，保留了这些测试和严格 rustdoc 门槛。卫生清理的其余工作归 #219。

2026-11-22 的后续票 [#265](265-split-provider-profile-test-sources.md) 已把当时
记录的 11 个共享 provider profile 源拆成一 target 一源。本票中的历史计数和
“共享源可生成多个 feature target”只描述 #215 的基线，不再作为新增 profile 的
默认组织方式。

重新打包后，minimum 与 tracing 外部消费分别通过 20、21 项，两个消费者的严格 Clippy、minimum rustdoc 和缺少 builtin catalog 的编译负例均符合预期。#211 的 11 个 provider 消费者使用同一 SHA-256 的归档。

Test impact: add 清单检查器的四项回归测试（含 CLI 负例）；update 19 项 Cargo 声明；retain 全部 Rust 集成用例、测试源名和具名 runner。
