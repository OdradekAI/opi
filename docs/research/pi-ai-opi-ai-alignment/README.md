# pi-ai / opi-ai 对齐研究

> 当前交付来源：**Phase 20，本地 Pi 1.3.0**。目标仅限 opi-ai 的公开能力和可观察语义，接口采用 Rust 惯用设计，逐项声明语言／平台／安全差异。[技能注册表](../../../.agents/skills/opi-implement/SKILL.md#reviewed-supplemental-sources)固定精确来源集；[上位规范](../../opi-spec.md)拥有持久方向。注册不等于来源准入、任务图确认或产品实施；七组产品覆盖仍 OPEN。

## 当前阅读入口

| 目的 | 入口与当前状态 |
|---|---|
| 设计就绪判定 | [设计决定与证据](pi-1.3.0-design-readiness.md)、[独立复审](pi-1.3.0-design-readiness-review.md)：READY仅指设计，七组产品实施／平台验收仍OPEN |
| 双语 Phase 来源／责任／完成条件 | [English spec](spec-draft.md)、[中文spec](spec-draft.zh.md)：Phase 20 条款映射、分解、风险／回滚／平台及精确章节采纳；67 AIR身份＝57 active SDK＋9 host-impact＋1 retired；没有实施证明 |
| 本次逐项升级 | [1.2.0→1.3.0复核](pi-1.3.0-upgrade-review.md)、[固定来源与全路径摘要](pi-1.3.0-source-evidence.json)：AI47与telemetry2变化路径全部处置；本地身份不等于已验证上游tag |
| 当前全部公开能力 | [Pi1.3.0 SDK合同](pi-1.3.0-sdk-contracts.md)：10 chat、1 image、4 classifier、42 factories、9 OAuth；profile复用wire仍须独立证明 |
| Rust调用体验与差异 | [RDF-01–20](pi-1.3.0-rust-differences.md)：owned/lazy/Result/knownness/tracing/platform与真实能力缺口分别说明 |
| 本轮确定后端与维护成本 | [Schema](pi-1.3.0-readiness-schema.md)、[WS](pi-1.3.0-readiness-ws.md)、[HTTP/TLS/auth](pi-1.3.0-readiness-transport-auth.md)、[telemetry/交付输入](pi-1.3.0-readiness-runtime.md)；source/data/namespace/native/unsafe边界明示，不靠consumer rootpatch |
| Rust机制与OPEN后端缺口 | [后端／五项合同及缺口表](pi-1.3.0-rust-backends.md)、[专项源码核查](pi-1.3.0-backend-gap-audit.md)、[Schema／WS离线实测](pi-1.3.0-schema-ws-offline-validation.md)：实测反例、候选判定与未证项；七组均OPEN |
| 唯一验收与测试迁移 | [验收矩阵](acceptance-matrix.md)：59 V身份＝54 active SDK＋4 host-impact＋1 retired；Pi174 test files不是执行通过，Rust41 integration＋7 source-test处置保留 |
| 参数、预算与新增拒绝依据 | [1.2.0来源审计](pi-1.2.0-parameter-audit.md)按本次同摘要继承，变化项以1.3.0复核为准；无来源固定cap／总deadline／blanket拒绝仍撤回 |
| 复审与检查 | [离线差分复审](pi-1.3.0-schema-ws-offline-review.md)、[离线证据](pi-1.3.0-schema-ws-offline-evidence.json)；[前置源码专项复审](pi-1.3.0-backend-gap-review.md)及[升级轮复审](pi-1.3.0-revision-review.md)保留各自原输入 |
| 版本、规划与决策 | [tracking与后续模板](upstream-tracking.md)、[decisions历史与本次替代关系](decisions.md)；不建立第二实施账本或自动监控 |

当前行为修订集中在：退出compact frames；跨模型thinking全部drop；最终工具参数保留arguments_error而非执行可解析前缀；Responses/Anthropic effort与off；error-tool images、Chat同chunk次序、Kimi/OpenCode路由和模型metadata。telemetry实现／测试／README未变，内部依赖精确pin不代表新增instrumentation。所有source默认／backend默认／Rust差异按具体消费路径区分，不因库升级增加无依据限制。

两快照仍缺hydrated provider JSON。前置源码轮未执行运行实验；后续[最小离线差分](pi-1.3.0-schema-ws-offline-validation.md)实际执行了固定Node／Bun、Pi公开validation utility、原生Schema引擎和WS loopback／确定性reader／codec实验。实验在TEMP执行，可重跑脚本与原始结果留存为研究资产；没有改产品代码、根Cargo、产品测试或CI，没有hydrate/live/OAuth。受控实验离线或仅127.0.0.1；准备阶段补取两个官方归档。另一次未关闭Bun自动解析的导入诊断生成了缓存，已记录／隔离并废弃该诊断，其联网与解析范围未知，不计作离线证据；正式WS已用--no-install重跑。Schema／WS等七组仍OPEN，不能借typed Unsupported、可覆盖cap或SSE fallback关闭；当前opi-ai调用体验仍未被证明与Pi等价。

## 维护与历史证据

`固定输入 → verify: manifest/hash与Git归属`；`逐项更新 → verify: source事实／Rust差异／AIR/V一致`；`测试处置 → verify: add/update/delete/retain与实际运行分开`；`复审 → verify: 独立发现闭环、文档检查、原始证据完整`。来源注册与上位路线修订限文档／技能维护，不授予后续实施、安装、live、提交或推送权限。后续 plan 重新审查精确来源集并经整图确认安装台账；本次不刷新旧账本 hash 以掩盖规范漂移。

旧文档只作为其固定日期／输入的证据。三份1.2.0现行附件已清成恢复入口：[SDK](pi-1.2.0-sdk-contracts.md)、[RDF](pi-1.2.0-rust-differences.md)、[backend](pi-1.2.0-rust-backends.md)；原正文和本README旧历史长正文可用`git show c5ab42a:docs/research/pi-ai-opi-ai-alignment/<file>`恢复。该提交对象已核存在，旧源快照1.2.0本轮仍可读；不把历史正文重贴成当前要求。

| 历史窗口 | 保全入口（不作为当前通过证明） |
|---|---|
| 1.0.0研究、人类D/Q、SDK与telemetry分析 | [baseline](baseline.json)、[decisions](decisions.md)、[SDK proposal](sdk-target-proposal.md)、[telemetry design](telemetry-design.md)、[spec reviews](spec-review.md) |
| 1.2.0两轮来源与候选修订 | [版本审查](pi-1.2.0-review.md)、[五问补核](pi-1.2.0-followup.md)、[候选复审](pi-1.2.0-revision-review.md)、[独立Pi全包报告](../pi-1.2.0/README.md) |
| Rust架构与参数复核 | [架构审查](pi-1.2.0-rust-architecture-review.md)、[后端修订复审](pi-1.2.0-backend-revision-review.md)、[参数修订复审](pi-1.2.0-parameter-revision-review.md)、[冻结验证](pi-1.2.0-parameter-revision-verification.json) |

只移除被替代的现行要求／叙述，不改历史hash、旧测试执行结果或人类回答。只有已验证替代与保全关系后才可进一步删除历史证据；产品交付进度由既有流程拥有。产品测试影响 **none**；本轮新增离线研究harness与证据，所有V的完整实施／运行验收仍未交付。

## 设计收敛研究窗口（注册前）

用户已明确选定设计READY口径，未授予产品实施。Schema改选私有方言IR／Unicode16 regex与URL数据profile；HTTP受控Hyper动态存储、WS受控Ratchet组合、private OpenSSL EVP配rustls公共扩展、full-source auth、safe tracing载体及版本化目录交付合同均在新报告确定。维护代价包括10个URL/IDNA/ICU后端namespace包、有限其他source/data补丁、zlib/OpenSSL native与unsafe审查；它们不新增Opi领域API或rootworkspace拓扑，也未获得依赖准入。

独立复审复现后，WS长度/溢出/声明长度提前分配需要最新纯内存补救；旧80观察保留原补丁身份，不能替代新patch验证。后续遵守用户限制不建立socket或发网络请求，不能执行的范围继续OPEN。设计状态以最新独立复审和主spec标记为准，七组产品覆盖全部OPEN。源码/文档/探针更新不代表发布依赖、真实服务成功、正式规范准入或V通过。
