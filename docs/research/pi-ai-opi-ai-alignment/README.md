# pi-ai / opi-ai 对齐研究

> 当前候选：本地 **Pi 1.3.0**，2026-10-16。目标仅限opi-ai的公开能力和可观察语义，接口采用Rust惯用设计，逐项声明语言／平台／安全差异。研究不改变Opi全局定位、正式规范或实施准入。

## 当前阅读入口

| 目的 | 入口与当前状态 |
|---|---|
| 双语目标／责任／完成条件 | [English spec](spec-draft.md)、[中文spec](spec-draft.zh.md)：67 AIR身份＝57 active SDK＋9 host-impact＋1 retired；没有实施证明 |
| 本次逐项升级 | [1.2.0→1.3.0复核](pi-1.3.0-upgrade-review.md)、[固定来源与全路径摘要](pi-1.3.0-source-evidence.json)：AI47与telemetry2变化路径全部处置；本地身份不等于已验证上游tag |
| 当前全部公开能力 | [Pi1.3.0 SDK合同](pi-1.3.0-sdk-contracts.md)：10 chat、1 image、4 classifier、42 factories、9 OAuth；profile复用wire仍须独立证明 |
| Rust调用体验与差异 | [RDF-01–20](pi-1.3.0-rust-differences.md)：owned/lazy/Result/knownness/tracing/platform与真实能力缺口分别说明 |
| Rust机制与OPEN后端缺口 | [后端／五项合同及缺口表](pi-1.3.0-rust-backends.md)：Schema、WS、HTTP/H2/proxy/redirect、trust/crypto、auth/source retry、native tracing、Cargo/platform/catalog均未关闭 |
| 唯一验收与测试迁移 | [验收矩阵](acceptance-matrix.md)：59 V身份＝54 active SDK＋4 host-impact＋1 retired；Pi174 test files不是执行通过，Rust41 integration＋7 source-test处置保留 |
| 参数、预算与新增拒绝依据 | [1.2.0来源审计](pi-1.2.0-parameter-audit.md)按本次同摘要继承，变化项以1.3.0复核为准；无来源固定cap／总deadline／blanket拒绝仍撤回 |
| 复审与检查 | [本轮独立复审](pi-1.3.0-revision-review.md)：记录实际发现、修订、检查与未证明部分；不以旧review追认新稿 |
| 版本、规划与决策 | [tracking与后续模板](upstream-tracking.md)、[decisions历史与本次替代关系](decisions.md)；不建立第二实施账本或自动监控 |

当前行为修订集中在：退出compact frames；跨模型thinking全部drop；最终工具参数保留arguments_error而非执行可解析前缀；Responses/Anthropic effort与off；error-tool images、Chat同chunk次序、Kimi/OpenCode路由和模型metadata。telemetry实现／测试／README未变，内部依赖精确pin不代表新增instrumentation。所有source默认／backend默认／Rust差异按具体消费路径区分，不因库升级增加无依据限制。

两快照缺hydrated provider JSON。本轮只读源码与测试预期，未hydrate/import/install/build，也未运行Pi/Rust产品测试、live／OAuth或差分runner。Schema／WS等当前机制仍有条件，不能借typed Unsupported、可覆盖cap、SSE fallback或“更Rust”关闭缺口。当前opi-ai调用体验尚未被证明与Pi等价。

## 维护与历史证据

`固定输入 → verify: manifest/hash与Git归属`；`逐项更新 → verify: source事实／Rust差异／AIR/V一致`；`测试处置 → verify: add/update/delete/retain与实际运行分开`；`复审 → verify: 独立发现闭环、文档检查、原始证据完整`。本轮只改研究文档，不授予后续实施、安装、live、提交或推送权限。

旧文档只作为其固定日期／输入的证据。三份1.2.0现行附件已清成恢复入口：[SDK](pi-1.2.0-sdk-contracts.md)、[RDF](pi-1.2.0-rust-differences.md)、[backend](pi-1.2.0-rust-backends.md)；原正文和本README旧历史长正文可用`git show c5ab42a:docs/research/pi-ai-opi-ai-alignment/<file>`恢复。该提交对象已核存在，旧源快照1.2.0本轮仍可读；不把历史正文重贴成当前要求。

| 历史窗口 | 保全入口（不作为当前通过证明） |
|---|---|
| 1.0.0研究、人类D/Q、SDK与telemetry分析 | [baseline](baseline.json)、[decisions](decisions.md)、[SDK proposal](sdk-target-proposal.md)、[telemetry design](telemetry-design.md)、[spec reviews](spec-review.md) |
| 1.2.0两轮来源与候选修订 | [版本审查](pi-1.2.0-review.md)、[五问补核](pi-1.2.0-followup.md)、[候选复审](pi-1.2.0-revision-review.md)、[独立Pi全包报告](../pi-1.2.0/README.md) |
| Rust架构与参数复核 | [架构审查](pi-1.2.0-rust-architecture-review.md)、[后端修订复审](pi-1.2.0-backend-revision-review.md)、[参数修订复审](pi-1.2.0-parameter-revision-review.md)、[冻结验证](pi-1.2.0-parameter-revision-verification.json) |

只移除被替代的现行要求／叙述，不改历史hash、旧测试执行结果或人类回答。只有已验证替代与保全关系后才可进一步删除历史证据；产品交付进度由既有流程拥有。本轮实际测试影响 **none**，所有未来V的实施／运行证据仍未交付。
