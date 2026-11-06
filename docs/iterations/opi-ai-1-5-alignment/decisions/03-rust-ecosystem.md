# 核实 Rust 依赖复用与成熟项目的测试策略
Type: research
Label: wayfinder:research
Status: resolved
Owner: research-3 (coordinated by Codex for user)
Parent: [opi-ai 1.5.0 独立库与能力对齐路线](../map.md)
Blocked by: none

## Question

以一手资料核查 Rust HTTP/WebSocket/云 SDK 的现状，尤其 tokio-tungstenite/tungstenite、reqwest、AWS SDK；区分通用协议能力与 pi/SDK 特有语义。参考成熟 Rust 项目和 Cargo 官方规范，哪些组织、测试和隔离方法适合本任务，哪些不应照搬？

## Answer

2026-11-06 研究已解决；这是事实调查完成，不是设计获批或行为验收通过。

已核实 Rust WebSocket/HTTP 与 AWS 官方 SDK 能力，以及 PMD、凭据优先序和异步 hook 等适配缺口；当前 Pi 走宿主 WebSocket。成熟项目采用私有机制、公共接口和专项并发/fuzz 互补验证。候选尚未编译或替换。

证据资产：[研究报告](../evidence/rust-ecosystem.md)。具体来源、暂定建议和未验证范围均保存在该报告。
