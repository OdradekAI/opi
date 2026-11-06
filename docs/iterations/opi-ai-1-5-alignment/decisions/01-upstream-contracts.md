# 核实 pi-ai 1.5.0、telemetry 与使用文档的真实边界
Type: research
Label: wayfinder:research
Status: resolved
Owner: research-1 (coordinated by Codex for user)
Parent: [opi-ai 1.5.0 独立库与能力对齐路线](../map.md)
Blocked by: none

## Question

本地 .pi/pi-1.5.0 的来源身份是什么？pi-ai 的 README/公开入口如何组织，telemetry 的实际职责与三个下游依赖关系是什么？区分通用遥测契约、AI 埋点和产品 exporter；指出与用户提供验收文档基准的一致性或差异。

## Answer

2026-11-06 研究已解决；这是事实调查完成，不是设计获批或行为验收通过。

已核实当前快照与旧验收 commit 的内容差异；AI 有 13 个变动文件、缺旧基线 2 个测试。遥测直接消费者为 ai/durable/chord，中立契约、AI 埋点、宿主 adapter 各自拥有不同职责。快照缺实际模型 JSON，未执行原版行为验证。

证据资产：[研究报告](../evidence/upstream-contracts.md)。具体来源、暂定建议和未验证范围均保存在该报告。
