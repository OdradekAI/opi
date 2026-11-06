# opi-ai 1.5.0 独立库与能力对齐路线
Label: wayfinder:map
Status: resolved
Owner: user
Baseline: opi 90f953d5ee288c165dc295bfdfb7cb7a05d7cecd; initial worktree clean; upstream .pi/pi-1.5.0; SHA256 content manifest 959b3b486f111b53771d37da272c8951064952bd9b18596c5c244935cea45072 (algorithm in upstream evidence; not a Git commit)

## Destination

形成 opi-ai 对齐本地 pi-ai 1.5.0 的可实施重构方案：明确公共接入契约、telemetry 边界、工程结构、第三方依赖边界、测试策略、README 信息结构与验收等级。完成条件是实施前的重要决策均有用户确认和可追溯证据；本地图不实施代码或 README 重构。

## Notes

- 2026-11-06 用户确认本轮只讨论并形成方案。
- 用户确认“独立模块”：第三方 Rust 项目只依赖 opi-ai 及其声明的库依赖即可接入，无需 opi-agent、opi CLI、私有仓库路径或隐式全局初始化；不要求独立仓库。最终产品方向仍是对齐 pi-coding-agent。
- 每轮使用 wayfinder；人机决策使用 grilling 与 domain-modeling，外部证据使用 research。当前证据不替代用户选择，不默认旧调研仍成立。
- 本仓库 [tracker](../../agents/issue-tracker.md) 是位置和状态的依据：本地 Markdown，无远端 issue、研究分支、提交或 push。由一个写入者维护地图与决策票；研究代理只读仓库并回报证据。
- 同一问题只在其决策票保存答案；地图只记录已解决票的一行索引。研究报告是证据资产，不代表设计决策获批。
- 源码、文档、测试和实测冲突需显式记录。参考验收文档：C:/Users/Luiz/Documents/Codex/2026-11-03/r/outputs/pi-ai-rust-acceptance.md；它的 commit 和本文本地快照须核对，不直接等同。
- 研究结果区分事实、建议、未验证项；当前不开付费 provider 测试。
- 初次 chart map 只关闭研究票；后续每轮处理一个人机决策，用户可选择讨论票。公开产品文档保留现状，方案不冒充已实现能力。
- 2026-11-07：十一项研究/决策均已解决，规划交接至[重构方案与实施交接](spec.md)。地图 resolved 仅指本轮规划完成；实现、行为证据和数值预算冻结尚未完成。

## Decisions so far

- [核实 pi-ai 1.5.0、telemetry 与使用文档的真实边界](decisions/01-upstream-contracts.md)：已核实当前快照与旧验收 commit 的内容差异；AI 有 13 个变动文件、缺旧基线 2 个测试。遥测直接消费者为 ai/durable/chord，中立契约、AI 埋点、宿主 adapter 各自拥有不同职责。快照缺实际模型 JSON，未执行原版行为验证。
- [核实 opi-ai 工程、测试与独立接入的现状](decisions/02-current-structure-tests.md)：已核实最小依赖配置缺失、Models 职责集中、测试专用 public 探针和历史说明漂移；现有测试分层可保留，未证明任何大型测试可以整体删除。已存在实际打包消费 harness，应扩展复用。
- [核实 Rust 依赖复用与成熟项目的测试策略](decisions/03-rust-ecosystem.md)：已核实 Rust WebSocket/HTTP 与 AWS 官方 SDK 能力，以及 PMD、凭据优先序和异步 hook 等适配缺口；当前 Pi 走宿主 WebSocket。成熟项目采用私有机制、公共接口和专项并发/fuzz 互补验证。候选尚未编译或替换。

- [确定独立接入与行为对齐的公共契约](decisions/04-public-contract.md)：用户确认 Rust 原生接口、完整能力映射、逐项行为验收；有意差异单独决策和记录，不计严格等价通过。

- [确定 telemetry 的归属与依赖方向](decisions/05-telemetry-boundary.md)：用户确认独立中立 crate，覆盖 opi-ai 所需契约与记录/测试支持；tracing 可选，AI 埋点与宿主采集策略分别拥有。

- [确定遥测生命周期与失败语义](decisions/11-telemetry-lifecycle.md)：用户确认执行与消费分离，释放消费句柄不隐式取消；请求执行者负责资源与遥测结算，异常语义按入口保留并单列冲突。

- [确定第三方能力的复用与自研边界](decisions/07-dependency-boundary.md)：用户确认按能力封装、成熟依赖优先、自研须有具体反例、底层默认私有，并以同一契约验证候选替换。

- [确定 opi-ai 的模块结构与公开入口](decisions/06-module-structure.md)：用户确认按职责组织单一 AI crate、保留真实高级入口与最小依赖目标，允许有计划的 0.x 公共路径迁移并同步迁移消费者。

- [确定测试归属、差分证据与去重规则](decisions/08-test-boundaries.md)：用户确认按契约设置主要验证位置、保留互补贯通测试，允许精确联合验证替代简单私有单测，并以反例可检测性和可追溯替代证据去重。

- [确定面向第三方开发者的 README 信息结构](decisions/09-readme-contract.md)：用户确认 Pi 风格任务手册、专题与 rustdoc 分工，以单 provider 真实调用和离线 Faux 两条路径接入，并绑定中英文示例与打包消费验证。

- [确定验收标准的分层与发布判定](decisions/10-acceptance-policy.md)：用户确认默认完整内置能力、可选最小构建与显式注册；本次建立完整对齐基线，后续按影响补证，定量预算先校准再确认冻结。

## Not yet specified

本轮架构、接入、测试、文档和验收政策无待定分支。原有待展开项已明确归属：逐符号/消费者/案例映射、源数据补齐、配置矩阵和版本选择是[交接方案](spec.md)中的实施输入；工程预算的校准与后续确认是正式验收前置条件，并非已经批准全部数值。

Rust 与 JS 的共享、启动、异常等差异仍按公共契约逐项举证，不自动豁免。若探针揭示不能兼容的具体行为，另开具名决策后再改变合同。现有 agent/coding-agent 调用者随本次 API 迁移；完整 coding-agent 对齐继续属于后续独立工作。

## Out of scope

- 本轮生产代码、Cargo 依赖、现有测试和产品 README 的实际改写。
- 本轮实现整个 pi-coding-agent、要求 JS 调用方无修改运行或自动扩展到更新的 Pi 版本。
- 自动提交、创建远端 issue、推送分支、发布 crate、付费线上调用。
