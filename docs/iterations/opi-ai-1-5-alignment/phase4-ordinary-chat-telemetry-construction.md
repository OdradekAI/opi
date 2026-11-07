# 第四阶段第十二个切片：普通 chat 构造时安装 AI telemetry

日期：2026-11-07。实施票：[Ordinary chat telemetry starts at construction](issues/20-ordinary-chat-telemetry-construction.md)。

本切片把普通 `Models` chat 的 observer 安装边界前移到 `ChatCall` 构造，
对应 Pi 1.5.0 在 `Models.stream`/`streamSimple` 构造 `lazyStream` 时传入
`observeModelRequest` 的行为。后台执行仍按既有 Rust 合同独立调度；本次只调整
观测开始时点和 setup 失败的结算归属。

## 实现观察

- 构造一个带显式 `TelemetryContext` 的普通 chat 后，recorder 立即拥有一个
  未结束的 `pi.ai.request` span，即使执行任务尚未完成认证或 provider setup；
- route、model、reasoning 和 pricing facts 在构造时冻结，执行 driver、
  `ChatStream` 与显式 abort 共享同一 observer；
- setup failure 使用 canonical `CallFailure.message()` 结束该 observer，保留
  `error` stop、诊断和已知 usage；
- 无 context、noop context、prepared raw attempt 以及 deferred/non-chat 边界
  沿用前序切片，不因此扩展 observer 范围。

## 验证

- `ai_telemetry` 16 项通过，新增构造后立即可见且随后 abort 的断言；
- `cargo check -p opi-ai --all-targets --offline -j4` 通过；
- `cargo clippy -p opi-ai --all-targets --offline -j4 -- -D warnings` 通过；
- `cargo fmt --all --check` 与 `git diff --check` 通过。

## 尚未完成

完整 provider instrumentation、prepared/raw 自动观测、TS/Rust 差分、真实
provider、feature/platform/MSRV、性能与六门发布验收仍未执行。
