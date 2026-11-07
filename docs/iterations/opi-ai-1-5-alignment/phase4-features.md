# 第四阶段首个切片：可选 telemetry tracing feature

日期：2026-11-07。实施票：[可选 telemetry tracing feature](issues/08-optional-telemetry-feature.md)。

本切片完成依赖闭包的第一步：默认使用路径不变，第三方项目可以通过
`default-features = false` 关闭 `opi-telemetry` 的 tracing 适配器。中立 crate
仍不依赖 AI、agent 或 CLI；`opi-ai` 自身的 `tracing` 诊断依赖保留。

随后使用实际 `cargo package` 产物做了 workspace 外消费者重放：
`defaults-off` 关闭 `opi-ai` 默认 feature，`caller-defaults-on` 显式启用
`opi-ai/telemetry-tracing`；两档都通过既有 73 项公共 Schema 检查。依赖解析只
对精确打包的 `opi-telemetry` 候选做受控路径补丁，没有使用 workspace 源码替代。

`crates/opi-ai/Cargo.toml` 现在声明 `telemetry-tracing` feature，并把它映射到
`opi-telemetry/tracing`。这使 `TelemetryContext::from_span` 的可用条件成为
显式 manifest 合同，而不是隐藏在依赖行中的强制 feature。

## 检查结果

| 检查 | 结果 |
| --- | --- |
| 默认 `cargo check -p opi-ai --offline -j4` | PASS |
| `cargo check -p opi-ai --no-default-features --offline -j4` | PASS |
| `cargo tree -p opi-ai --no-default-features -e features` | PASS；`opi-telemetry` 未带 `tracing` feature |
| `cargo tree -p opi-ai -e features` | PASS；默认路径带 `opi-telemetry/tracing` |
| `cargo clippy -p opi-ai --lib --no-default-features -- -D warnings` | PASS |
| `cargo clippy -p opi-ai --lib -- -D warnings` | PASS |
| `test_package*.py` harness | PASS；74 项 |
| 实际 `.crate` 外部消费者：defaults-off / caller-defaults-on | PASS；两档各 73/73 |
| 双语 README、文档契约与 diff 检查 | PASS |

## 尚未完成

这是 feature 声明和最小构建切片，不是完整 feature 矩阵。网络 backend、云认证、
内嵌目录、平台、MSRV、真实包的两两组合和完整六门验收仍待后续。普通 `Models`
chat 已自动安装 observer；`AiRequestTelemetry` 在 raw/prepared/non-chat 入口
仍是手动 seam，没有在所有 provider execution path 自动安装。
