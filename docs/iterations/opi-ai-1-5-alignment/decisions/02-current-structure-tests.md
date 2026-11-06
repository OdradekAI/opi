# 核实 opi-ai 工程、测试与独立接入的现状
Type: research
Label: wayfinder:research
Status: resolved
Owner: research-2 (coordinated by Codex for user)
Parent: [opi-ai 1.5.0 独立库与能力对齐路线](../map.md)
Blocked by: none

## Question

当前 opi-ai 的公开 API、目录、依赖、下游调用、README 与测试组织是什么？哪些混乱或重复有具体源码证据，哪些尚需执行验证？结合提供的验收标准指出现状与目标之间的关键缺口，避免把数量当作质量或把未执行当作通过。

## Answer

2026-11-06 研究已解决；这是事实调查完成，不是设计获批或行为验收通过。

已核实最小依赖配置缺失、Models 职责集中、测试专用 public 探针和历史说明漂移；现有测试分层可保留，未证明任何大型测试可以整体删除。已存在实际打包消费 harness，应扩展复用。

证据资产：[研究报告](../evidence/current-structure-tests.md)。具体来源、暂定建议和未验证范围均保存在该报告。
