# 第四阶段第十六个切片：收窄 request duration metric 属性

日期：2026-11-08。实施票：[Keep request duration metric attributes bounded](issues/24-request-duration-metric-attributes.md)。

Pi 的 `pi.ai.request.duration` 只记录 `provider`、`model` 和 `stop`。原 Rust
实现复用了 request span 的完整结束属性，把 usage、cost 及高基数
`response.id` 一并传入 histogram。本切片恢复低基数 metric 合同。

## 实现观察

- request duration 使用独立属性 map，只包含 provider、model、Pi stop；
- request span 继续保留完整 usage、cost 和可选 response id；
- first-chunk、tokens、cost metric 的属性没有改变。

## 验证

- 先运行属性集合回归，旧实现按预期失败并显示额外 usage/cost/response.id；
- 修复后 `ai_telemetry`：19 项通过；Anthropic 153、Gemini 71、OpenAI Chat 114、
  Bedrock 178、Mistral 54 均通过；
- `cargo check`、Clippy、rustdoc、fmt、文档检查和外部 package proof 均通过，
  两个 feature profile 都是 73/73。

## 尚未完成

完整 provider instrumentation、TS/Rust 全量差分、真实服务、feature/platform/MSRV、
性能和六门发布验收仍未执行。
