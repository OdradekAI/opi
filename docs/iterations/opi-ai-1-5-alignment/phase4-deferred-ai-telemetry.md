# 第四阶段第三个切片：deferred fetch 自动 AI poll telemetry

日期：2026-11-07。实施票：[deferred fetch 自动安装 AI poll observer](issues/10-deferred-ai-telemetry.md)。

本切片让 `Models::stream_deferred` 返回标准 `ChatStream`，并在调用携带显式
`TelemetryContext` 时自动安装 `pi.ai.poll` observer。fetch 仍只派发一次
后端检查；pending、ready、预校验失败和取消继续由既有 `ChatStream` 合同
处理。`fetch_deferred` 与直接消费 stream 共用同一 observer 和结算路径。

## 检查

| 检查 | 结果 |
| --- | --- |
| `cargo fmt --all --check` | PASS |
| `cargo test -p opi-ai --test ai_telemetry --offline -j4` | 6 passed，包括 ready `pi.ai.poll` |
| `cargo test -p opi-ai --test deferred_contract --offline -j4` | 16 passed |
| `cargo check -p opi-ai --all-targets --offline -j4` / Clippy | PASS |
| 实际 `.crate` 外部消费者双 feature profile | PASS；两档 73/73 |

检查命令、范围和限制见[证据](evidence/phase4-deferred-ai-telemetry-checks.json)。

## 尚未完成

raw ChatStream、prepared attempt、图像、分类和 deferred cancel 仍没有统一的
自动 observer。完整 provider/feature 矩阵、真实服务、TS/Rust 差分、跨平台、
MSRV、性能与六门发布验收仍未执行。工作区全量回归的两项既有失败仍适用：
`schema_engine::encoded_pointer_targets_preserve_selected_resource_identity` 和
`spec_ledger::live_ledger_spec_hash_matches_current_spec`。
