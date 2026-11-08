# 测试与包消费

本文区分下游用户可以运行的测试和维护者用来证明发布包的检查，不是 runtime API 参考。

## Faux Provider 测试

消费者需要在无凭据、无真实服务的情况下得到确定响应时，可使用
`test_support::MockProvider` 和公开 Faux 注册 helper。显式脚本化最终消息、stream event、
tool call、延迟或 pending result 以及 Provider failure。测试拥有 response script 和
cancellation token；SDK 仍拥有请求构造、event 顺序和类型化终态结算。

Faux 适合 unit 和 integration 测试调用方行为，但不能证明真实 Provider 的认证、wire 序列化、
rate limit 或服务可用性。需要验证这些边界时，再加入受控 peer 或 Provider 专属 fixture。

## 受控 Provider 与协议 Fixture

仓库的受控 peer 记录 request、header、source read 和 response 顺序，不访问真实 Provider。
Fixture 应断言自己拥有的可观察边界：request body、认证来源、重试次数、stream 终态 event、
取消阶段或 cleanup acknowledgement。

协议 fixture 与模块测试互补。纯 parser 或 state machine 使用聚焦 unit test；unit 无法观察的
接线、顺序和副作用使用受控 peer test。

## 测试分层与替代规则

- 模块测试覆盖 private parser、state machine 和外部调用者无法观察的不变量。
- 公共契约测试覆盖 public value、option、error、feature 和 rustdoc example。
- 协议/consumer 测试覆盖 request 构造、stream event、认证读取、取消、Store 发布和跨模块所有权。
- 包证明在 workspace 外消费实际 `.crate`，并在两种 feature profile 下运行三个 README onboarding 程序。

删除测试前，必须确认其输入类别、时序、错误阶段、副作用和资源风险仍由具名替代证据检测。
完整 integration test 可以替代简单委派 unit test，但不能替代独有的 public value 或 parser 边界断言。

## 实际包消费者证明

在仓库根目录运行离线证明：

```text
python -B scripts/pi-sdk-conformance/run_readme_examples.py --offline --scratch C:/Temp/opi-readme-proof-20261108-restructure --target-dir E:/opi/cargo-targets/opi-72fa37a0fa044958
```

证明会打包 `opi-telemetry` 与 `opi-ai`，创建普通外部 Cargo consumer，验证默认和
`default-features = false` 两种 profile，确认 setup 不做 source/store/request I/O，并执行
README 中的逐字示例。JSON verdict 写入
`scripts/pi-sdk-conformance/evidence/readme-examples-proof.json`。

## 开发检查

文档修改应运行 conformance suite、文档契约检查、Rust formatter check、diff 空白检查、
package file listing 和实际包消费者证明。真实 Provider 调用、fuzz 和性能检查需要单独显式
启用验收范围。
