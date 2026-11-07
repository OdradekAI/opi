# 第四阶段第十五个切片：保留可选 response id 的 telemetry 形状

日期：2026-11-08。实施票：[Preserve optional response id in AI telemetry](issues/23-optional-response-id-telemetry.md)。

Pi 的 `AssistantMessage.responseId` 是可选值。原 Rust telemetry 在值缺失时
写入空字符串，导致 `response.id` 这个高基数可选属性从“缺失”变成“存在但为空”。
本切片让 Rust 保持 Pi 的属性形状。

## 实现观察

- `AiRequestTelemetry::finish` 只在 `response_id` 为 `Some` 时写入
  `response.id`；
- 有值时保留原字符串，不改变 schema、usage、cost、stop 或结算时点；
- 外部可观察的无 id 结果不再产生空字符串属性。

## 验证

- 先运行新增回归，旧实现按预期失败（缺失 id 被观察为空字符串）；
- 修复后缺失 id 与存在 id 两条断言均通过；
- 受影响的 `ai_telemetry`：19 项通过；`cargo check`、Clippy、rustdoc、fmt、
  `git diff --check`、文档检查和外部 package proof 均通过；实际 `.crate`
  消费的两个 feature profile 都是 73/73。

## 尚未完成

完整 provider instrumentation、TS/Rust 全量差分、真实服务、feature/platform/MSRV、
性能和六门发布验收仍未执行。
