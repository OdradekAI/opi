# 第四阶段第六个切片：独立执行 ChatStream 的 telemetry 结算

日期：2026-11-07。实施票：[Started ChatStream telemetry settlement](issues/13-started-chatstream-telemetry-settlement.md)。

公开 `ChatStream::new_with_telemetry` 已允许独立 provider 选择观测路径；本切片
修复其后续调用 `ChatStream::start()` 时 observer 不会在正常后台终态结算的问题。

## 实现观察

- `ChatStream::start()` 仍把请求执行与消费分离，后台句柄仍不因消费句柄释放而隐式取消；
- `StartedChatStream` 继续只负责业务事件与最终结果，AI observer 的所有权仍在被调度的
  `ChatStream` 内部；
- 后台正常终态由 `result_without_tail` 复用现有 `finish_ai_telemetry()`；已交付
  terminal 后再调用 `start()` 的快速路径也先完成同一 observer，不复制 span、
  metric 或 source 处理；
- raw `ChatStream::new` 没有 observer，因此行为不变。

## 验证

- `ai_telemetry`：10 passed，新增独立执行句柄及 terminal 后转换的
  `pi.ai.request` 终态断言；
- `cargo fmt --all --check`、`cargo check`、Clippy、rustdoc、文档检查和包消费验证
  继续按受影响范围执行；
- 工作区先前记录的两项无关全量失败不因本切片改变。

## 未完成

显式 abort 无业务终态时的 observer 终态由后续 issue 15 补齐；跨 retry attempt 聚合、
raw attempt 自动化、图像、分类、deferred cancel、完整 provider/feature 矩阵、真实
服务、TS/Rust 差分、跨平台/MSRV、性能和六门发布验收仍未执行。
