# 迁移 agent 请求消费与显式中止
Status: done
Owner: Codex for user
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)
Depends on: [显式启动 chat 请求并分离执行与消费](04-started-chat-execution.md)

## Scope

迁移 agent 主循环和 preparation summary 两处高级请求消费到独立执行者。
普通消费句柄或外层 future Drop 不中止已启动的源；显式 caller cancellation
保留 agent 的取消优先策略，并等待执行者释放其持有的源，包含不响应 token 的
自定义 provider。新增显式中止操作保留已完成 outcome、未完成 partial 和 panic。

本票不改变旧 ChatCall/ChatStream 手动 poll、raw EventStream、provider 内部任务
清理合同或 first-terminal absorption；不宣称 Pi source.result/遥测结算等价。

## Acceptance

- agent 主调用和 summary 调用被丢弃后，受控 provider 仍能推进到终态。
- caller token 取消时，不合作源被释放，返回 Cancelled，不提交 partial 或执行工具。
- 显式中止等待 owner 退出；保留 canonical partial；已完成结果和 panic 不被覆盖。
- 重试准入、strict variant、provider 证据顺序和资源清理的相关回归保持通过。
- 同步双语合同、能力/消费者/测试映射及实际检查记录。

## Delivery

已迁移两个生产消费者，并实现借用式显式 abort。准备流程持有执行句柄，
在外层 hook 取消后仍可等待源释放；普通消费者 Drop 保持已启动模型执行。
报告见[本切片交付](../phase3-consumers.md)，命令及结果见
[检查证据](../evidence/phase3-consumers-checks.json)。定向回归、最终 Clippy、
文档示例及 rustdoc 通过；全量回归的两项既有失败仍开放，本票不代表工作区全绿。
旧默认入口和 AI instrumentation 留待后续切片。
