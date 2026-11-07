# 第四阶段第十四个切片：pending stop 与 telemetry 词汇对齐

日期：2026-11-08。实施票：[Align pending stop state and AI telemetry vocabulary](issues/22-pending-stop-and-telemetry-vocabulary.md)。

本切片补齐 Pi 1.5.0 的非终态 `pending`，并修正 AI telemetry 的 stop reason
词汇。Pi 的 telemetry 使用 `toolUse`，而 Rust canonical `StopReason::as_str()`
保留 `tool_use` 以维持 Rust/serde 既有接口；telemetry 单独使用 Pi 词汇映射。

## 实现观察

- `StopReason::Pending` 现在可公开构造、serde 编解码并返回 `"pending"`；
- Anthropic、Gemini、Bedrock、OpenAI Chat/Responses、Mistral 和 Pi Messages
  provider 的初始 partial 使用 `Pending`；
- `pending` 作为错误的 terminal `Done` 会被 typed stream failure 吸收，避免把
  非终态伪装成成功完成；
- pending/deferred poll 不计 token/cost，tool-use telemetry 的 span 和 duration
  metric 都记录 `toolUse`；schema 公开全部七个 Pi stop 值。

## 验证

- `ai_telemetry`：18 项通过；
- Anthropic fixtures：153 项通过；Gemini fixtures：71 项通过；OpenAI Chat
  fixtures：114 项通过；
- `cargo check`、Clippy、rustdoc、fmt 和文档检查通过；
- 精确 `.crate` 产物的仓库外双 profile proof 通过，两个 profile 均为 73/73。

## 尚未完成

本切片没有新增 pending terminal provider 协议；完整 provider instrumentation、
TS/Rust 全量差分、真实服务、feature/platform/MSRV、性能和六门发布验收仍未执行。
