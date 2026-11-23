# #265 为每个 provider feature-profile 保留独立测试源

Status: done
Owner: opi-ai feature-profile tests
Parent: [#209](209-maintain-module-test-and-catalog-boundaries.md)
Blocked by: none
Related: [#215](215-own-integration-test-targets.md), [#216](216-select-tests-by-changed-owner.md)

## 问题

当前 113 个 Cargo integration target 仍有 11 个 provider profile target
复用 `tests/provider_protocol_profile.rs`。一个文件中的 `cfg` 分支同时代表
Anthropic、Azure、Bedrock、Gemini、Mistral、OpenAI 两种 Responses、OpenRouter、
Pi Messages 和 Vertex。Cargo 的 feature 隔离是正确的，但源文件、target 和
owner 之间的关系需要读者展开分支才能确认，选测器也只能把这个文件映射成
11 个目标。

## 目标和范围

- 每个 `provider-*` feature-profile target 拥有一个只承载该 owner 构造性检查的源文件。
- 保留现有 feature、测试语义、目标名称和 package runner 绑定。
- 共享搭建继续放在 `tests/support/`，不通过共享根 target 隐藏 owner 关系。
- 本票不删除 provider 行为断言，也不改变生产模块、目录和兼容行为。

## 验收

1. Cargo metadata 中 11 个 provider profile target 的 `src_path` 两两不同，
   `check_test_inventory.py` 输出 113 个 target、113 个源文件。
2. 每个独立源文件只选择一个对应 target，`select_tests.py` 对任一 profile
   源文件给出 focused 计划，不再展开其他 provider。
3. 11 个 profile target 在各自的 `--no-default-features --features provider-*`
   配置下通过，脚本测试覆盖目录变更和旧映射拒绝。

## 实施记录 · 2026-11-22

新增 `tests/provider_protocol_profile/` 下的 11 个 owner 文件，删除共享根源，
并为 Cargo 声明和 selector 回归测试更新路径。每个 target 保留一个明确命名的
构造性检查，Azure、Gemini、Vertex 的目录数量断言保持不变。

| 检查 | 结果 |
| --- | --- |
| `python scripts/pi-sdk-conformance/check_test_inventory.py` | 113 targets，113 sources |
| selector/inventory Python tests | 15 passed |
| 11 个 provider profile target | 11 passed，0 failed |
| 每个 profile 源的 selector 计划 | 11/11 focused，单目标命中 |

全量 test Clippy 期间发现的两个独立 image fixture 漂移已分别在
[#271](271-repair-stale-image-fixture.md) 和 [#272](272-repair-stale-openrouter-image-fixture.md)
修复。它们不是 profile 路径拆分造成的回归，但纳入本批验收以保证全量测试仍可编译。

Test impact: update Cargo source paths and selector coverage; retain all provider
profile assertions and external package runners.
