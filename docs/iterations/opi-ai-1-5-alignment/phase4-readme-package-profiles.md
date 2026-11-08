# 第四阶段：独立消费者与 README feature profile 验收

日期：2026-11-08。实施票：[Exercise README onboarding in both public feature profiles](issues/26-readme-package-profiles.md)。

本切片把“独立模块”收敛成可以重复执行的打包消费者验收：从实际
`.crate` 解压 `opi-ai`，在 workspace 外部消费者中运行 README 的三个示例，
并分别验证 Cargo 默认依赖 feature 与 `default-features = false` 两档。

两个 profile 的 metadata、编译、controlled-peer 回归和实际执行均通过；
`defaults-off` 的 `opi-ai` feature 集为空，`defaults-on` 解析出默认的
`telemetry-tracing`。三条流程继续验证构造阶段不读取 source/store、调用在
消费者 poll 前已经启动、最终消息/流终态以及 tool-result continuation。
外部依赖图没有 `opi-agent`、`opi-coding-agent`、`opi-protocol` 或 `opi-tui`。
示例中的编译和运行命令也已改为普通消费者项目的
`cargo check --example ...` / `cargo run --example ...`，不再依赖
`-p opi-ai` workspace 语境。
两份 README 还提供了三个 onboarding 示例所需的完整外部消费者
`Cargo.toml`，并明确 `default-features = false` 替代写法，避免接入者猜测
runtime 依赖。

随后增加了双语任务导航，将安装、普通调用、流式、Context/工具、Models/认证、
Provider、Telemetry 和测试包证明作为新接入的明确入口；长篇 provider/协议参考
保留在导航之后。导航契约由 `test_readmes_expose_task_oriented_integration_guide`
固定。

在导航与 Quick Start 之间又加入双语公共 API 总览，明确 `Models`、`Request`、
`ChatCall`、Context/工具与 `TelemetryContext` 的责任边界；契约由
`test_readmes_publish_public_api_at_a_glance` 固定。

随后在 API 总览之后加入双语高级能力参考索引，把 Provider、工厂/认证、凭据
刷新、Telemetry、cost/pricing、WebSocket、Deferred 与测试包证明集中成可跳转
入口；契约由 `test_readmes_index_advanced_capability_references` 固定。该索引
只改善导航，不改变 feature profile 或运行时语义。

随后补上双语 feature profile 矩阵，明确当前只有 `telemetry-tracing` 一个公开
feature；`default-features = false` 只关闭 `opi-telemetry` tracing carrier，
不会移除内置 Provider 或网络依赖。契约由
`test_readmes_describe_feature_profiles_without_overclaiming_minimal_builds` 固定。

最后同步独立 `opi-telemetry` 的双语 README，修正过时的 AI instrumentation 描述，
明确普通 chat 的 `pi.ai.request`、deferred fetch 的 `pi.ai.poll` 以及 raw/non-chat
手动边界；契约由 `test_telemetry_readmes_describe_current_ai_integration_boundary`
固定。两个 crate 的实际打包消费者 smoke proof 见
[`phase4-telemetry-readme-ai-boundary.json`](evidence/phase4-telemetry-readme-ai-boundary.json)。

随后把 cost/pricing 专题从接入正文拆出：主 README 保留未知用量、缺失定价和成功流
不被费用阻塞的短语义，并保留原估算器标题作为稳定跳转目标；完整的 estimator、
sampling、pricing tier、cache pricing、service tier 与兼容规则进入 crate 内的
双语 `docs/cost-pricing*.md`。`test_readmes_split_cost_pricing_into_package_local_topic_references`
固定链接、专题文件和代表性规则，避免文档重排时丢失精确计价入口。

随后把 Provider/Auth 专题从接入正文拆出：双语 `docs/provider-auth*.md` 集中覆盖
Provider 工厂与能力矩阵、凭据/OAuth/Request 扩充/Refresh 生命周期、OpenAI-compatible
profile、离线与动态 Catalog，以及 canonical built-in factory。主 README 保留原
标题锚点和短链接，Core API 与具体协议任务仍留在正文。契约由
`test_readmes_split_provider_catalog_auth_into_package_local_topic_references` 固定。

随后清理公开手册中的历史对齐标签：README 与四份包内专题只保留当前能力、行为、
公开入口和接入限制，不再把内部任务号、阶段号、验收快照号或仓库私有快照路径暴露给
下游开发者。原始追溯继续保留在 `docs/iterations/opi-ai-1-5-alignment/` 的 issue、
report、evidence 和 Git 历史中；契约由
`test_public_opi_ai_manuals_omit_internal_alignment_labels` 固定。

随后按 Pi 风格任务顺序重构双语主 README：安装、feature profile、Quick Start、
stream/result、Context/工具、Provider、图像/分类、reasoning、错误、telemetry、
自定义 Provider、Faux、deferred、迁移和开发成为公共入口。Proxy/TLS/WebSocket/
资源生命周期迁入 `docs/transport*.md`，schema/replay/transcript/utility 迁入
`docs/utilities*.md`，Provider wire/operation 迁入 `docs/providers*.md`，Faux、测试
分层和包证明迁入 `docs/testing*.md`。契约由
`test_readmes_follow_pi_style_public_task_order`、
`test_public_readmes_do_not_expose_private_implementation_sections` 和
`test_public_topic_references_expose_current_contracts` 固定；实施记录见
[README 公共手册重构票](issues/35-readme-public-manual-restructure.md)。

证据见 [`readme-examples-proof.json`](../../../scripts/pi-sdk-conformance/evidence/readme-examples-proof.json)。
入口文档见 [`opi-ai README`](../../../crates/opi-ai/README.md) 和
[`pi-sdk-conformance README`](../../../scripts/pi-sdk-conformance/README.md)。

这项验收证明的是独立打包接入和两个公开 feature profile，不等同于所有
provider、完整 pi-coding-agent 或发布前六门验收已经完成。Live provider、
MSRV、性能和正式发布仍保持独立门槛。
