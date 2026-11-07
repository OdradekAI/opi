# Deferred fetch 自动安装 AI poll observer

Status: done  
Owner: Codex for user  
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)  
Depends on: [普通 Models chat 自动安装 AI observer](09-ordinary-chat-ai-telemetry.md)

## Scope

让 Pi `streamDeferred`/`fetchDeferred` 对应的标准 Rust 入口共享
`ChatStream` 结算和 `pi.ai.poll` 观测边界。handle 的
`provider`/`model_id`/`api` 是 poll 的冻结身份；`DeferredFetchCall` 的
`telemetry_context` 仍由调用方显式提供，缺省时不创建观测。一次 fetch
仍只检查一次，pending 仍返回 `StopReason::Deferred`，取消仍只影响本地
fetch，不触碰远端任务。

## Acceptance

- `Models::stream_deferred` 返回 `ChatStream`，复用普通 ChatStream 的终端、
  `CallFailure` 和 source 结算语义。
- 显式 telemetry context 下生成一个 `pi.ai.poll`，以 handle 身份写入
  start/end 属性，并从 `ChatStream::result()` 的业务终态结算。
- pending/deferred poll 不增加 token/cost 计数；ready fetch 使用现有 usage
  和 cost 规则。
- 无 context、预校验失败、后端失败和本地取消不改变原有 fetch 业务结果。
- 现有 deferred 合同、carrier 传递和序列化测试继续通过。

## Delivery

`Models::stream_deferred` 保留原有 source 生产函数为私有 helper，再用
`ChatStream::new_with_ai_telemetry` 包装；`fetch_deferred` 直接消费同一
`ChatStream`。新增 `ai_telemetry` 的 ready poll 端到端断言，README 双语说明
返回类型和 observer 边界。

## Limits

raw `ChatStream::new`、prepared attempt、图像、分类和 deferred cancel 仍不
自动安装 AI observer；这些入口继续依赖显式 seam 或后续切片。未执行真实
provider、TS/Rust 全量差分、跨平台/MSRV、性能或六门发布验收。
