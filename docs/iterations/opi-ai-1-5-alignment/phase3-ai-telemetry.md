# 第三阶段第四个切片：AI telemetry seam 与 source/result 尾部

日期：2026-11-07。实施票：[AI telemetry seam 与 source/result 尾部](issues/07-ai-telemetry-source-tail.md)。
本切片的限定范围已完成；完整 provider instrumentation、完整 Pi AI 差分和六门发布验收仍未完成。

## 本轮实现

`opi-ai::telemetry` 现在提供 AI 领域的公开请求观测入口。`AiRequestAttributes`
冻结 provider/model/api/reasoning、poll 和可选 pricing；`AiRequestTelemetry`
负责 `pi.ai.request` / `pi.ai.poll` span、首事件、block 事件、UTF-16 字符数、
stop/usage/response/cost 终态属性以及 request/first-chunk/token/cost metrics。
错误 stop reason 才标记 provider error；aborted 保持默认成功状态；deferred poll
只记录请求耗时，不累计 token/cost。中立后端、记录器和 schema 类型仍在
`opi-telemetry`，没有反向依赖 AI 或 agent。

流生命周期现在区分两个时点：业务消费者在第一个 terminal 后永久 EOF，
`ChatStream::result()` 继续排空保留的 source，确保 source-owned settlement
完成后再返回缓存结果。projection 和 prepared attempt 不再在业务 terminal
处过早丢掉底层源；排空期间不再次交付业务事件，也不修改 retry admission。
`StartedChatStream` 仍使用既有的后台资源合同，在 terminal 转发后释放源；这是
后台资源释放与显式 raw `ChatStream::result()` 的有意入口差异，尚未宣称 Pi
所有执行形态已经统一。

## 测试与证据

新增 `tests/ai_telemetry.rs` 覆盖四个公开契约：请求事件/结果结算、deferred
poll 计量排除、schema/status 策略和终态后 source EOF。`stream_events`、
`execution_lifecycle`、`provider_lifecycle`、`telemetry_operations` 迁移了
旧的终态 source 释放/轮询计数假设，并保留重复终态、跨线程、取消、prepared
attempt 和 ambient context 断言。

本轮定向检查均通过：格式检查、AI telemetry 4 项、stream 29 项、execution
lifecycle 24 项、provider lifecycle 22 项、telemetry operations 12 项，以及
`opi-ai` library unit tests 369 项。
命令与范围见[检查证据](evidence/phase3-ai-telemetry-checks.json)。

## 尚未完成

本票只建立可独立测试的 AI 观测 seam，没有把它自动安装到每个 provider 的
请求执行者，也没有完成 observer failure、真实服务、TS/Rust 全量差分、feature
矩阵、跨平台/MSRV、性能预算或六门验收。没有 pricing 时 cost.total 采用零值，
真实 catalog/pricing 对齐另需数据和入口级证据；这些限制不能被本票的内存 recorder
测试覆盖。
