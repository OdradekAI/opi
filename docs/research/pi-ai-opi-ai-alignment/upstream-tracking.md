# Pi AI 上游版本与规划跟进记录

> 非规范性证据索引。目标：仅 `opi-ai` 对照选定 Pi 版本的公开能力和可观察语义，使用 Rust 惯用设计并明确差异。记录来源、变化、审查和证明边界；不承担实施准入、任务/阶段进度或测试完成账本。目标要求仍在双语候选，综合验收仍在 [acceptance matrix](acceptance-matrix.md)。

## 版本审查记录

| 研究日期 / 比较 | 固定输入与结论 | 可复用入口 |
|---|---|---|
| 2026-10-03–04；Pi 1.0.0 | 当时选定能力子集和 D/Q 确认；56 AIR／48 V，测试执行与未来设计分开。不因此覆盖新版本 | [历史基线](baseline.json)、[决策历史](decisions.md)、[原研究导航](README.md) |
| 2026-10-16；Pi 1.0.0 → 1.2.0 | 当前 Opi HEAD `e6c2eb80b99a69f80aefc7aafee3b61140a0a6c8`；Pi tree `a2c03e7c77aa5e6837ed69055a29d724eff9e65a18c1534221ef58e299cc0c10`。AI 378→394，新增18/变化79/删除2；406项 AI＋telemetry 摘要匹配。候选暂不核准，R1–R10 为修订依据 | [1.2.0 审查](pi-1.2.0-review.md)、[输入证据](pi-1.2.0-review-baseline.json)、[独立 Pi 报告](../pi-1.2.0/README.md) |
| 2026-10-16；同一 1.2.0 的五问补核 | 分清三类 Context；telemetry 主要设计仍有效、未实施。刷新官方规划状态；目录版本选择已有本地代码。当前调用体验未等价、没有差分 runner。补测试处置和持续记录；无 runtime tests/live | [五问补核](pi-1.2.0-followup.md)、[输入/清单/网页摘要](pi-1.2.0-followup-evidence.json)、[测试处置](acceptance-matrix.md#2026-10-16-测试迁移补核) |
| 2026-10-16；两轮核查后的候选修订 | 文档扩展为 67 AIR（58 SDK＋9 宿主）、59 V（55 SDK＋4 宿主）；完整公开能力及 RDF-01–20、测试迁移和版本跟进；不改变运行时。独立复审结论以链接报告为准 | [当前双语稿](spec-draft.zh.md)、[能力附件](pi-1.2.0-sdk-contracts.md)、[Rust 差异](pi-1.2.0-rust-differences.md)、[修订输入](pi-1.2.0-revision-baseline.json)、[复审](pi-1.2.0-revision-review.md) |

本表的“暂不核准”是研究候选结论，不是线上故障或产品实施进度。旧 Pi 1.0.0 源码当前不在工作区；[历史报告恢复关系](../pi-1.2.0/evidence/pi-1.0.0/README.md)不能替代源码恢复。上游网页按访问日记录；本地快照按 manifest/hash 固定，两种身份不混用。

## 规划观察表

每次版本跟进更新“观察证据/状态/触发”，不把提案直接加入实施范围。来源详述在 [本次补核](pi-1.2.0-followup.md#2-未来规划已有研究需刷新状态与实施证据)，这里仅作索引。

| 观察对象 | 2026-10-16 证据状态 | 重新评估触发 |
|---|---|---|
| Responses Lite / ordered Context | RFC0054 Discussion；chronology 已交付，完整 Lite 未证；DeveloperMessage 原型不等于当前 SystemMessage | 已接受的具体协议修订或实际发布的 Lite adapter/连接语义 |
| 动态目录与版本选择 | RFC0039 Discussion；本地 distribution v1 selector/publisher 与 data v6 已实现；pi.dev 部署未核 | schema/index/兼容版本选择变更；经验证的服务合同；不能用 Rust crate 版本冒充 Pi 兼容版本 |
| 凭据相关目录 | OpenRouter helper 与产品 wrapper 已交付；Radius/个性化/通用 overlay 的空目录规则不同 | key/账号/区域 endpoint 或 logout/login 变化；缓存与晚 publication 的新隔离要求 |
| 新操作/adapter | chat/image/classifier、Decisions 与 classifier images 是当前能力 | 新 public operation/model kind、API 或实际 consumer 的新增状态机需求 |
| Deferred remote | 公共生命周期、faux 和 durable consumer 已存在；内置真实远端 adapter 未证 | submit/fetch/cancel/expiry/account/idempotence 协议与 fixtures；不以假 consumer 证明远端支持 |
| pi-messages | 当前 Radius/custom HTTP/SSE wire；非未来统一 transport 承诺 | 已发布 wire versioning 或两个真实后端需要共同 conformance |
| Telemetry spans/exporter/carrier | context 传播已交付；所查 exporter/Agent spans PR closed/unmerged；analytics 另属产品 | 明确接受、合并和发布的 instrumentation，或用户单独选择的领域/数据政策 |
| Compat 退出 | deprecated 但仍存在消费者；未核准移除版本/日期 | 上游实际删除且调用方完成迁移，或本项目显式同步破坏性变更 |
| Virtual routing | 产品可选能力已有实现；历史引入时称 experimental，不能称未实现 | 稳定公开合同改变，或用户单独选择产品路由范围；分类不授予工具权限 |
| 通用 invocation Context | Chord 有自己的调用上下文；AI 未采用其载体作为公共合同 | AI 正式签名变化及真实 operation/adapter 的共同需要；不预建万能 Context |
| 产品 analytics / 实验入口 | RFC0038 Discussion；UI/settings 部分实现，完整出站未证；0043 Published | 实验毕业/删除、明确数据/关闭/发送合同；不据此给裸 SDK 默认上报或 feature-flag 框架 |

## 下一次版本跟进模板

追加一个记录并链接独立报告/差异；不要覆盖旧基线或把历史执行结果刷新日期后复用。

```text
审查日期：
Pi 声明版本 / source 路径 / manifest / tree hash / 已验证的上游身份：
比较前驱版本 / 旧报告或恢复关系：
Opi HEAD / 相关 dirty 文件及归属：
公开能力变化 / 实际 adapter 与消费者：
Context、auth、telemetry、序列化与资源所有权变化：
未来规划：已交付 / 明确接受但未实现 / 讨论实验 / 历史替代 / 未知：
调用方动作、默认值、失败、回调、retry/cancel/cleanup 变化：
Rust 有意差异及理由 / 尚缺能力：
测试处置：add / update / delete / retain，含替代证明与唯一矩阵定位：
实际检查：输入身份、精确命令、退出码、执行/skip/缺失；未运行项目：
候选/附录需修订处 / 人类确认来源 / 本轮核准结论：
报告与证据链接：
```

按 `固定输入 → public surface/实现/消费者/文档/测试 diff → 规划状态核对 → Rust 差异与唯一验收矩阵 → 核准与实际检查记录` 执行。报告可提示后续修订，但不自动启动实现、live、安装、提交或定时监控。产品交付进度只进入其拥有权限的既有工作流，本文件不记录百分比、任务状态或另建账本。
