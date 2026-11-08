# 第四阶段：pricing、cache 与 provider cost 语义

日期：2026-11-08。实施票：[Align pricing tiers, cache pricing, and provider-reported cost](issues/25-cost-pricing-semantics.md)。

本切片把 Pi 1.5.0 的成本模型固定到 Rust 公共值：catalog 分量与 provider
上报总额分离，tier 的严格边界、cache-write 1h 子集和 OpenRouter BYOK 规则均
有独立回归。OpenAI/Anthropic 的通用解析器不会因为输入中出现同名扩展字段就
错误覆盖成本；只有 OpenRouter route 应用 provider accounting。

验证结果：cost/pricing 专项 47 项、OpenRouter accounting 与 route gate 6 项、
AI telemetry 20 项通过；其中 2 项直接覆盖 `ModelInfo::apply_response_cost` 的
终态 catalog 计费与 provider total 保留。OpenAI Chat fixture 114 项、Anthropic
fixture 153 项、`model_wire_metadata` 24 项通过。后者的 Anthropic fixture 已补齐
Pi 要求的 `message_delta.delta.stop_reason=end_turn`，生产端仍保持 pending-stop
严格失败语义。

`cargo check -p opi-ai --all-targets`、Clippy（warnings denied）、rustdoc、fmt 和
`git diff --check` 均通过。全量 `cargo test -p opi-ai --offline -j4` 已通过；
本轮同时收敛了 Radius 固定回调端口的并行测试竞争、过时的 feature 契约，以及
schema-engine 的 sibling `$id` resource identity 回归。后两项是独立基线修复，
不改变本切片的计费语义。

尚未完成：真实 OpenRouter/Anthropic 账单服务、完整 TS/Rust differential、
feature/platform/MSRV、性能和六门发布验收。外部 `.crate` package proof 已在
提交产物上执行并通过。
