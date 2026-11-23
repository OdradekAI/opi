# #272 修复过期 OpenRouter image fixture 字段

Status: done
Owner: opi-ai OpenRouter image tests
Parent: [#271](271-repair-stale-image-fixture.md)
Blocked by: none

## 问题

`tests/openrouter_fixtures.rs` 的 image record 与 telemetry image fixture 一样，
仍初始化已经从 `ImageModelRecord` 删除的 `compat` 字段。这个错误只在包含该
fixture 的全量 test 编译中暴露。

## 实施与验收 · 2026-11-22

删除过期字段，保留 OpenRouter image request、Models registration、native options
和错误阶段断言。all-features test Clippy 不再因 image fixture 字段失败。

Test impact: update one stale OpenRouter image fixture initializer; retain all image
wire and Models assertions.
