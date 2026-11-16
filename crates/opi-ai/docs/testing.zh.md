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
显式 `builtins` 无 tracing 两种 profile，确认 setup 不做 source/store/request I/O，并执行
README 中的逐字示例。JSON verdict 写入
`scripts/pi-sdk-conformance/evidence/readme-examples-proof.json`。

最小配置使用 `scripts/pi-sdk-conformance/run_minimal_package.py`，指定新的
`--scratch`、共享 `--target-dir` 和新的 `--evidence-output`。它核对实际归档在
tracing 关闭/开启时的依赖图，执行核心 Faux 工具续轮、自定义登录 Store ACK 和
参数/数字契约，并拒绝意外的后端依赖。这不代表完整 Pi Faux 行为或所有
feature/platform 验收行已闭合。README runner 也应使用 `--evidence-output` 保留旧收据。

共享 HTTP transport profile 使用
`scripts/pi-sdk-conformance/run_http_transport_package.py`，指定新的
`--scratch`、带标记的 workspace `--target-dir` 和新的 `--evidence-output`。
它在 workspace 外以 `default-features = false, features = ["http-transport"]`
消费实际归档，核对 transport-only 依赖图，并执行公共 pool、proxy 和 redirect 合同。

TLS backend profile 使用
`scripts/pi-sdk-conformance/run_tls_backend_package.py`，指定新的
`--scratch`、带标记的 workspace `--target-dir` 和新的 `--evidence-output`。
它在 workspace 外以 `default-features = false, features = ["tls-backend"]`
消费实际归档，核对精确的 rustls/AWS-LC/native-certs 依赖闭包，确认 reqwest
和 provider owner 未泄漏，并执行公共 `opi_ai::tls` trust/ALPN 合同。
HTTP transport profile 另行证明 `http-transport` 仍传递选择该 backend。

WebSocket/compression profile 使用
`scripts/pi-sdk-conformance/run_websocket_compression_package.py`，指定新的
`--scratch`、带标记的 workspace `--target-dir` 和新的 `--evidence-output`。
它在 workspace 外以 `default-features = false, features = ["websocket-compression"]`
消费实际归档，核对精确的 HTTP/TLS/PMD 依赖闭包且不带 provider adapter，并执行
workspace 外的 HTTP/1.1 Upgrade 与 permessage-deflate 协商。原生 profile 还运行
公共 Upgrade/WSS 合同和私有 frame/PMD 回归套件。


云认证 profile 使用
`scripts/pi-sdk-conformance/run_cloud_auth_package.py`，指定新的
`--scratch`、带标记的 workspace `--target-dir` 和新的 `--evidence-output`。
它在 workspace 外以 `default-features = false, features = ["cloud-auth"]`
消费实际归档，核对 HTTP/TLS 与 AWS/GCP source/signing codec 的精确闭包，
确认 provider protocol/compression owner 未泄漏，并执行公共 AWS endpoint、
SigV4 和 GCP ADC 常量调用。

Provider protocol adapter profile 使用
`scripts/pi-sdk-conformance/run_protocol_adapters_package.py`，指定新的
`--scratch`、带标记的 workspace `--target-dir` 和新的 `--evidence-output`。
它在 workspace 外以 `default-features = false, features = ["protocol-adapters"]`
消费实际归档，核对包含传递选择 `cloud-auth`/`http-transport`/`tls-backend` 且不含内嵌
catalog 的 provider protocol 依赖闭包，并执行
直接 adapter、auth-store 和 WebSocket surface 调用。Provider collection 与
内置 OAuth/AWS/Vertex credential factory 仍通过 `builtins` profile 证明。

Provider OAuth/store profile 使用
`scripts/pi-sdk-conformance/run_oauth_store_package.py`，指定新的
`--scratch`、带标记的 workspace `--target-dir` 和新的 `--evidence-output`。
它在 workspace 外以 `default-features = false, features = ["oauth-store"]`
消费实际归档，核对精确的 `http-transport` feature union，拒绝内嵌 catalog
和 protocol/cloud-signing owner，并执行 provider OAuth 与
`StoreOAuthResolver` 的 workspace 外调用。中立 credential、mutation、Store
和 OAuth 合同由最小 profile 继续覆盖；provider collection 与平台
credential-source factory 仍通过 `builtins` profile 证明。

Provider protocol/source owner 使用
`scripts/pi-sdk-conformance/run_provider_protocol_package.py`，指定新的
`--scratch`、带标记的 workspace `--target-dir` 和新的 `--evidence-output`。
runner 在 workspace 外消费实际归档，分别核对以下十一项 owner：
`provider-anthropic`、`provider-azure-openai`、`provider-bedrock`、
`provider-gemini`、`provider-mistral`、`provider-openai-chat`、
`provider-openai-codex-responses`、`provider-openai-responses`、
`provider-openrouter`、`provider-pi-messages` 与 `provider-vertex`。
瞬时包构建失败可用 `--owner provider-vertex` 单独重跑，不覆盖旧回执。
原生 profile 测试和 warnings-denied Clippy 使用同样的 feature 写法。


Provider owner 的 pair 与高阶 capability V 使用
`scripts/pi-sdk-conformance/run_provider_protocol_matrix.py`，指定新的
`--scratch`、带标记的 workspace `--target-dir` 和新的 `--evidence-output`。
它在原生环境验证全部 55 个无序 provider-owner pair 与两个受支持的高阶
组合，再从实际归档由 workspace 外 Cargo consumer 重复验证同一批配置。
回执保留冻结 Pi 1.5.0 配对差分引用；没有 runner 时将原生 macOS 明确记录为
blocked，不用交叉编译代替。

剩余 shared-owner pair 使用
scripts/pi-sdk-conformance/run_shared_owner_pairs.py，指定新的
--scratch、带标记的 workspace --target-dir 和新的 --evidence-output。
它验证 cloud-auth+catalog-bundled 与 websocket-compression+oauth-store 的
owner profile test，再从实际归档由 workspace 外 consumer 验证精确 feature
closure 和禁止 owner 边界。可用 --only 单独重跑一个 pair，不覆盖另一个
pair 的历史回执。回执保留冻结 Pi 1.5.0 引用；没有 runner 时继续把原生
macOS 记录为 blocked。

## 开发检查

文档修改应运行 conformance suite、文档契约检查、Rust formatter check、diff 空白检查、
package file listing 和实际包消费者证明。真实 Provider 调用、fuzz 和性能检查需要单独显式
启用验收范围。
