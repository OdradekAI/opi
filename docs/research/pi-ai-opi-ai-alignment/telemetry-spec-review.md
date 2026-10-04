# Telemetry 候选 spec 专项复核

> **非规范性研究／DRAFT 候选复核记录，2026-10-04。** 本次只修改本研究目录，不启动产品实施。本页保存评审与实际文档检查；目标由 D-37 和双语 spec 表达，未来验收由 acceptance matrix 表达，不建立新的权威或进度账本。中文专项材料无既有英文 counterpart，候选契约同步更新英中版本。

## 覆盖面与遗漏修正

用户指出前稿遗漏 pi-telemetry 包的架构意义。前稿 P17/M-14 仅记录 context 透传，不能替代对包本身、作者意图、跨包消费者与 Rust 方案的调查。本轮补 [TC 契约](telemetry-pi-contracts.md)、[TE 演进](telemetry-evolution.md)、[TO 审计](telemetry-opi-audit.md)，由 [telemetry-design](telemetry-design.md) 综合；修订 README、P17、M-14、EVO-013、A-08、UX-17、D-37、双语 spec 和验收矩阵。

Q37 已实际确认 pi 的职责划分＋Rust tracing 模块。当前新增 AIR-054–056／V-46–48：选定 SDK 请求传播显式父 context；来源 Span＋Dispatch、异步 poll/drop、no-op 与宿主恢复规则；被动观测、保密和既有 evidence/diagnostic 所有权。当前整体为56条 AIR、48组未来验收设计。

独立 opi-telemetry crate、通用 schema/callback DSL、自动 SDK logical-call/attempt spans、完整 Agent span families、内建 exporter 和上传均未获本批范围选择。相关提案未合并不构成 roadmap；缺省不要求使用方启动 Opi 产品或配置 exporter。

## 独立评审与处理

三名研究 Agent 只拥有各自专项证据文件；主 Agent 顺序整合共享决策和双语 spec。评审均为源码／文档静态复核，未执行运行时测试。

| 评审者／范围 | 结论与反证检查 | 处理 |
|---|---|---|
| pi_contracts／公开包契约与 SDK options | 无 P1/P2；传播不冒充自动 spans，Rust native helpers 不冒充 pi callback/schema/status 等价，custom adapter 有公开可用路径 | 保留选定窄接口；将 Q37 前推荐明确标为历史访谈输入，不让实施者自行扩大 |
| pi_evolution／作者意图、发布、移除与官方提案 | 无阻断；中立包仍保留，旧 Agent schemas 确已移除，README 只部分过时，10009/10085 未被写成已接受计划 | 补 D-37 的 TE-09 引用，覆盖 PR/issue 状态证据 |
| opi_audit／Rust Dispatch、异步生命周期与真实消费者 | 无 P1/P2；Future combinator 不被误当 Stream/Drop 全覆盖；既有 sinks/ack 保留，任意 subscriber panic/block 不被虚假隔离 | 双语 annex 补明 no-op scoped dispatcher 不静默宿主独立日志或显式其他 context；不新增行为 |

三项复核覆盖相同的 AIR-054–056、D-37、V-46–48；细节结论保留在各证据文件末节。修正只有证据引用和已有 no-op 范围的澄清，未新增未确认目标。完整 spec 的此前 SDK/其他领域评审见 [sdk-spec-review](sdk-spec-review.md) 与其历史指针；本次专项复核不冒充重跑所有运行时路径。

## 共享基线与证据验证边界

本轮开始及整合后读取 HEAD `0fa995de504bcb9854e8ee841d54d99579da1bd4`，staged 为空。tracked modified 只有本任务未编辑的旧 `docs/research/pi-1.0.0/source-manifest.json`；本研究目录及无关 `docs/research/pi-evals-opi-eval-comparison.zh.md` 为 untracked。与初始 f721 基线的变化由共享工作区其他工作造成，本任务没有 commit/checkout/pull 等 Git 写入。

旧 manifest 输入 SHA-256 为 `7ba3764ae8921274ce78c6a8e20eeb5e75b949e8a57cc72438b2344345c0210f`；本次不修复、不覆盖其既有尾空白。未编辑原 pi 独立研究、正式规范、AGENTS、CLAUDE、实现账本、运行时、测试、依赖或配置。

实际研究检查包括本地源码／manifests／精确锁定 tracing 源码、官方版本文档、GitHub commit/tag/Release/issues/PR/comments/reviews 和12个 telemetry 官方 blobs 对比。TE-13列出官方只读命令和一次引号错误后的重读；TC/TO记录各自本地检查。root 的 `agent-reach doctor --json` 和 `firecrawl --status` 均 exit0；后者仅检查 CLI 状态，没有 scrape/search/extract 或付费抓取。官方 Rust 文档经 web 读取；首次错误路径改为正确 Instrument/WithSubscriber URL，无法读取的 dispatcher 页面由锁定源码补证。

不将 TypeScript 测试断言、文档声明、PR关闭或 readonly API 成功当作运行能力证明。没有执行 telemetry、Node/TS、Rust 测试／编译、exporter 或模型/live 请求，没有安装依赖。首轮70个既有测试不证明这次新增目标。

## 最终文档检查

| 实际命令／检查 | 结果与范围 |
|---|---|
| `python scripts/opi-doc-check.py` | exit0，`opi documentation contracts: PASS` |
| `git diff --check` | exit2；唯一诊断路径是上述未编辑的旧manifest，共11,720条既有尾空白，SHA-256不变；全仓库检查不记为通过 |
| 每个研究文件：`git diff --no-index --check -- /dev/null <path>` | 41文件无whitespace诊断；新文件存在差异的exit1是正常，不误当失败；普通diff未覆盖untracked不被当作证明 |
| Python标准库检查，经 `python -X utf8 -` 从stdin执行 | 41文件UTF-8/LF/final LF/无尾空白、705个本地链接目标存在、3个JSON解析，0问题 |
| 同一脚本的英中契约追踪核对 | AIR-001–056与V-01–48连续、唯一、索引完整；每条AIR有owner、MUST、D/V；双方最后证据列相同；388行逐行AIR/D/V、规范词、inline code及850个数字token对应，0问题 |
| `git rev-parse HEAD`、`git status --short`、`git diff --cached --name-only`及manifest hash读取 | 与本轮继续基线一致，staged仍为空；没有扩大写入范围 |

第一次组合文本检查命令因宿主替换字符串转义造成PowerShell ParserError，未执行检查；修正为literal replacement后重新执行exit0。没有把未执行的检查当成功，也没有修改runtime以解决文档检查。

最终受检主体SHA-256：

| 文件 | SHA-256 |
|---|---|
| spec-draft.md | `6e5fccfcccfbdad43f48fc72c34022343b33f0ddccee016e92e4b575cf25a01e` |
| spec-draft.zh.md | `9d606b901ceb035e1b19d0fb81ed342d25a4f76fe9b9001cfd8f1aaf09cde7af` |
| acceptance-matrix.md | `67f90af543209761ccef1cbf16487f84c8932fac704fe7fe2833d3cb92bfbc01` |

记录本节结果后再次执行doc checker和组合文本检查，以上结果保持不变。未运行的runtime验收仍按V-46–48保持待执行；这些文档检查不是新TelemetryContext已编译／已交付的证据。

## 剩余缺口与测试影响

Q37及此前目标问题全部已有实际回答，没有未回答的 telemetry 目标选择。DRAFT 保留的原因是正式文档协调、实施准入、运行时代码和验收均未发生。本轮只交付可评审的候选目标，不能登记 ready-for-agent。

未核 telemetry npm 发布 tarball、第三方生态 adapters、真实 exporter 吞吐／故障、任意宿主 feature unification 或 live 接受度；上游窗口固定于 TE 所记 commit，不宣称最新所有规划。旧实验 schema 和未接受提案仍可作设计输入，不能作为已承诺目标。拆独立 crate 或新增自动 spans 需按 D-37 的真实需求／契约触发重新选择。

实际测试影响：**none**，测试 add/update/delete 均0。未来 telemetry 验收建议 **add** 跨入口、Dispatch、poll/drop、秘密与被动性；**update** options/产品文档接线与外部 SDK fixture；**retain** DiagnosticSink、AgentEvents、EvidenceSink、ack、redaction和业务失败测试；telemetry 不提出 **delete** 既有 tests。其他候选淘汰项的测试处置仍以主验收矩阵为准。
