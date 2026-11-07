# 第四阶段：pricing、cache 与 provider cost 语义

日期：2026-11-08。实施票：[Align pricing tiers, cache pricing, and provider-reported cost](issues/25-cost-pricing-semantics.md)。

本切片把 Pi 1.5.0 的成本模型固定到 Rust 公共值：catalog 分量与 provider
上报总额分离，tier 的严格边界、cache-write 1h 子集和 OpenRouter BYOK 规则均
有独立回归。OpenAI/Anthropic 的通用解析器不会因为输入中出现同名扩展字段就
错误覆盖成本；只有 OpenRouter route 应用 provider accounting。

验证结果：cost/pricing 专项 45 项、OpenRouter accounting 与 route gate 6 项、
AI telemetry 20 项通过；OpenAI Chat fixture 114 项、Anthropic fixture 153 项
通过。`cargo check -p opi-ai --all-targets`、Clippy（warnings denied）、rustdoc、
fmt 和 `git diff --check` 均通过。

全量 `cargo test -p opi-ai` 已执行；当前工作区仍有一项既有的
`model_wire_metadata::sampling_presence_reaches_supported_wires_once_and_other_wires_keep_metadata_only`
失败，错误为 Anthropic pending stop 状态，与本切片无直接代码关联，单测重跑仍复现，
因此没有把它计入本切片通过证据。

尚未完成：真实 OpenRouter/Anthropic 账单服务、完整 TS/Rust differential、
feature/platform/MSRV、性能和六门发布验收。外部 `.crate` package proof 需在
提交后用新的产物重新执行。
