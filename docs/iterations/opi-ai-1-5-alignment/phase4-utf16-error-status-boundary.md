# 第四阶段第十三个切片：AI telemetry 错误状态的 UTF-16 截断边界

日期：2026-11-08。实施票：[Bound AI telemetry error status messages in UTF-16 units](issues/21-utf16-error-status-boundary.md)。

本切片修正 `provider_error` 状态消息的长度单位。Pi 使用 JavaScript
`ownedSlice(..., 0, 500)`，上限是 UTF-16 code unit；Rust 旧实现按 Unicode
标量计数，非 BMP 文本会超出上限。

## 实现观察

- `owned_slice` 现在按 `char::len_utf16()` 累计，保证结果不超过 500 个
  UTF-16 单元；
- 若上限落在代理对中间，Rust 在前一个完整 scalar 处停止，避免构造无效
  `String`；这一点作为明确的 Rust-native 边界记录；
- telemetry 的状态名称、错误类型和其他结算字段保持不变。

## 验证

- 先运行新增回归，旧实现按预期失败；
- 修复后 `error_status_message_cap_counts_utf16_units` 通过；
- `ai_telemetry` 全部 17 项、`cargo check`、Clippy、rustdoc、fmt、文档检查
  均通过；
- 精确 `.crate` 产物的仓库外双 profile proof 通过，两个 profile 均为 73/73；
- 证据命令和剩余限制见[检查证据](evidence/phase4-utf16-error-status-boundary-checks.json)。

## 尚未完成

完整 provider instrumentation、TS/Rust 全量差分、真实服务、feature/platform/MSRV、
性能和六门发布验收仍未执行；孤立代理对切片差异已单独记录，不能据此宣称严格
字节等价。
