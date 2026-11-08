# 第五阶段首个切片：验收材料冻结与生命周期差分

日期：2026-11-08。本切片冻结当前可取得的 Pi 1.5.0 验收材料，并建立第一条
TS/Rust 请求生命周期差分证据。它不修改生产行为，不调用真实 provider，也不代表
完整 Pi 或 pi-coding-agent 对齐通过。

> 历史初版记录：下述目录缺失描述是首次冻结时的状态。用户随后补充了材料；
> 加强差分屏障后又发现普通 provider source 尾部未执行，因此初版的有限字段
> 比较不能证明尾部对齐。当前身份、真实 FAIL 和剩余阻断以
> [补充材料与有屏障差分报告](phase5-supplemented-materials-and-gated-differential.md)为准。
> 本文件及原 JSON 保留初次运行事实，不作为当前通过结论。

## 材料冻结

冻结器和证据见：

- [冻结脚本](tools/freeze_acceptance_materials.py)
- [材料证据](evidence/phase5-acceptance-materials.json)
- [原始 pinned catalog](evidence/phase5-model-catalog.json)

当前可复核的身份如下：

- Pi 源快照的 `package.json`、`package-lock.json`、`packages/ai/package.json`、
  `event-stream.ts` 和 `lazy.ts` 均存在并记录 SHA-256；Node 为 24.13.0。
- Rust/Cargo 为 1.99.1，运行平台为 Windows 11 x86_64；当前 Git HEAD 为
  `3f2ff874af08a82108f2326281f579808b6dd25a`。
- pinned catalog revision 为
  `sha256-f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`；
  响应大小 940702 bytes，响应 SHA-256 与 revision 完全匹配，原始响应已保存在
  `evidence/phase5-model-catalog.json`。
- Pi `node_modules` 与 `packages/ai/dist` 不存在，因此“原版已安装 runtime /
  已打包 dist”仍是阻断项；不能把源码快照当作实际发布包或完整原版运行环境。

## 首条 TS/Rust 差分

驱动与证据见：

- [Rust driver](../../../crates/opi-ai/examples/lifecycle_differential.rs)
- [差分 runner](../../../scripts/pi-sdk-conformance/differential/run_lifecycle_differential.py)
- [差分证据](evidence/phase5-lifecycle-differential.json)

三个受控场景均执行成功，未出现意外 mismatch：

| 场景 | 共享观察 | 结论 |
| --- | --- | --- |
| `lazy-no-consumption` | 未轮询也启动 source，最终为 `stop` | `intentional-difference`：Pi 在构造时同步启动 setup，Rust 在 Tokio runtime 中调度后台执行 |
| `lazy-stop-consumption` | 消费一个事件后停止，source 仍完成，最终为 `stop` | `intentional-difference`：启动时点仍不同；停止消费不隐式取消这一共享行为成立 |
| `lazy-setup-failure` | setup failure 产生一个 `error` 终态 | `intentional-difference`：Pi 区分同步 throw 与 rejected setup，Rust 以 typed `CallFailure` 异步呈现 |

比较器只比较共享字段；driver、输入材料和每个原始 trace 的哈希都记录在证据中。
当前总判定为 `PASS_WITH_BLOCKED_GATES`：生命周期三例没有 unexpected failure，
但上游 runtime dependencies 和 packaged dist 仍未取得。

## 检查

- 材料冻结单元测试：2 项通过。
- 差分比较器与 Rust driver 合同测试：4 项通过。
- Node 原版三个场景：全部通过原版自断言。
- Rust driver 三个场景：全部输出可解析 JSON 并在 watchdog 内完成。

## 剩余范围与阻断

本切片没有执行完整 TS/Rust 能力差分、真实 provider、完整 provider 覆盖、feature/
platform/MSRV 矩阵、性能/耐久预算或六门完整验收。`node_modules`/`dist` 缺失只阻断
依赖实际上游安装或发布包的门，不阻断本切片的源码生命周期观察。下一步应在此证据
基础上扩大到 provider wire/stream 终态和 raw/prepared/non-chat 入口，并同时冻结
feature/platform/MSRV 集合。
