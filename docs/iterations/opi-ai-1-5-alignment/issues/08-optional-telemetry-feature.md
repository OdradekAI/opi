# 可选 telemetry tracing feature

Status: done  
Owner: Codex for user  
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)  
Depends on: [AI telemetry seam 与 source/result 尾部](07-ai-telemetry-source-tail.md)

## Scope

把 `opi-telemetry` 的 tracing 适配从 `opi-ai` 的强制依赖改成显式的
`telemetry-tracing` feature。默认构建继续开启该 feature，保持现有
`TelemetryContext::from_span` 接入；`default-features = false` 的第三方项目可以
只消费 backend-neutral context、recorder、schema 和 AI request 类型。

这个切片只改变依赖闭包和构建声明，不拆除 `opi-ai` 自身用于内部诊断的
`tracing` 依赖，也不声称已经完成完整的 provider feature 矩阵。

## Acceptance

- `opi-ai` 的默认 feature 集合包含 `telemetry-tracing`。
- `cargo check -p opi-ai --no-default-features --offline` 通过，并且不启用
  `opi-telemetry` 的 `tracing` feature。
- 默认构建仍通过；README 明确默认/最小配置和 `from_span` 的可用条件。
- feature 变化不会把 `opi-telemetry` 反向依赖 AI、agent 或 CLI。
- 实际 `.crate` 外部消费者分别以默认 feature 关闭和显式开启两档构建，且两档
  都通过既有 73 项公共 Schema 检查。

## Delivery

`crates/opi-ai/Cargo.toml` 新增：

```toml
[features]
default = ["telemetry-tracing"]
telemetry-tracing = ["opi-telemetry/tracing"]
```

`opi-ai` 对 `opi-telemetry` 使用 `default-features = false`；双语 README 增加
manifest 示例与边界说明。检查命令和 feature 树快照见
[`phase4-feature-checks.json`](../evidence/phase4-feature-checks.json)。实际打包
消费者的双 profile 证据见
[`phase4-package-feature-checks.json`](../evidence/phase4-package-feature-checks.json)。

## Limits

- `opi-ai` 仍直接依赖 `tracing`，所以 `default-features = false` 不是完全无
  tracing 的构建。
- 本票没有冻结所有 provider、平台、MSRV 或两两 feature 组合，也没有把
  `AiRequestTelemetry` 自动接入每条 provider 执行路径。
- 未进行真实 provider、live 服务或六门完整发布验收。
