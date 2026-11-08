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

证据见 [`readme-examples-proof.json`](../../../scripts/pi-sdk-conformance/evidence/readme-examples-proof.json)。
入口文档见 [`opi-ai README`](../../../crates/opi-ai/README.md) 和
[`pi-sdk-conformance README`](../../../scripts/pi-sdk-conformance/README.md)。

这项验收证明的是独立打包接入和两个公开 feature profile，不等同于所有
provider、完整 pi-coding-agent 或发布前六门验收已经完成。Live provider、
MSRV、性能和正式发布仍保持独立门槛。
