# #271 修复过期 ImageModelRecord fixture 字段

Status: done
Owner: opi-ai telemetry integration tests
Parent: [#265](265-split-provider-profile-test-sources.md)
Blocked by: none

## 问题

`tests/telemetry_operations.rs` 的 image fixture 仍初始化已经从
`ImageModelRecord` 删除的 `compat` 字段。单个 profile target 不会触发这条路径，
但 all-features 测试 Clippy 会在 test crate 编译阶段失败。Classifier record 仍然
拥有自己的 `compat` 字段，两者不能按相同的字段表维护。

## 实施与验收 · 2026-11-22

删除 image fixture 的过期字段，保留 classifier fixture 的有效 `compat` 元数据。
11 个 profile target 的定向 Clippy 继续通过，工作区 all-features test Clippy
不再因该字段报错。

Test impact: update one stale fixture initializer; retain image and classifier telemetry
behavior assertions.
