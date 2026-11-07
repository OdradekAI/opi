# 第四阶段第五个切片：公开 ChatStream telemetry 构造器

日期：2026-11-07。实施票：[Public ChatStream telemetry constructor](issues/12-public-chatstream-telemetry.md)。

本切片把已有的内部 AI telemetry 包装逻辑公开为
`ChatStream::new_with_telemetry(source, context, attributes)`。独立 provider
已经持有 `EventStream` 时，可以直接复用 `ChatStream` 的业务结果合同和 Pi
形状的 observer；无观测的 `ChatStream::new` 继续保留。

## 实现观察

- 只提升现有 observer seam 的可见性，普通 chat、deferred 和 prepared observed
  路径不各自复制 telemetry 实现；
- 观测仍由显式 `TelemetryContext` 控制，省略 context 不捕获 ambient tracing；
- observer 后端失败沿用现有隔离策略，通过宿主 tracing 告警，不替换业务结果；
- 终端吸收、source-tail 排空、取消和 `CallFailure` 仍由同一个 `ChatStream`
  实现负责。

## 验证

- `ai_telemetry`：新增 crate 外部调用公开构造器的测试，证明 `pi.ai.request`
  与最终 stop 属性均被记录；
- `cargo fmt --all --check`、受影响测试、`cargo check`、Clippy、rustdoc、文档
  检查和实际 `.crate` 外部消费验证沿用本票证据；
- 工作区先前记录的两项无关全量失败不因本切片改变。

## 未完成

raw `start_attempt` 的自动化或跨 attempt 聚合、图像、分类、deferred cancel、
完整 provider/feature 矩阵、真实服务、TS/Rust 差分、跨平台/MSRV、性能和六门
发布验收仍未执行。
