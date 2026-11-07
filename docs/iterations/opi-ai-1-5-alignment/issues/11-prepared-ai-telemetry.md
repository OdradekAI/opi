# Prepared attempt observed ChatStream seam

Status: done  
Owner: Codex for user  
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)  
Depends on: [Deferred fetch 自动安装 AI poll observer](10-deferred-ai-telemetry.md)

## Scope

为 `PreparedProviderCall` 增加一个兼容的 observed 入口，返回标准
`ChatStream` 并复用普通 Models chat 的 AI request observer。保留已有
`start_attempt()` 原始 `EventStream` 返回类型，避免改写高级 retry/admission
控制器和既有协议测试的低层消费合同。显式 `Request::telemetry_context` 才创建
observer；没有 context 时仍只提供业务 stream。

## Acceptance

- `start_attempt_with_telemetry()` 使用一次已有 prepared attempt，不重复 route、
  auth 或 provider preparation。
- 返回的 `ChatStream::result()` 复用终端吸收、source-tail 排空和 `CallFailure`
  语义，并在显式 context 下产生一个 `pi.ai.request`。
- observer 身份使用冻结的 provider、model 和 wire API；thinking/pricing 继续
  使用冻结 request/model 元数据。
- 原始 `start_attempt()`、重试 admission、取消、资源和 credential-terminal
  行为不改变。
- 双语 README 解释两个入口的边界；公共库测试覆盖 ready prepared attempt。

## Delivery

新增 `PreparedProviderCall::start_attempt_with_telemetry()` 和内部属性构造器，
仅在其外层安装 `ChatStream::new_with_ai_telemetry`。新增 `ai_telemetry` 集成
测试，证明 prepared stream 的 span 名称、身份和 stop 属性；文档与契约索引
记录 raw/observed 两条入口的职责。

## Limits

这是兼容增量，不把旧的 `start_attempt()` 直接改成 `ChatStream`，因此 raw
consumer 仍需自行包装。跨 attempt 的单逻辑调用聚合、重试期间的单 span、图像、
分类、raw ChatStream 和 deferred cancel 仍需单独讨论。未执行真实 provider、
TS/Rust 全量差分、跨平台/MSRV、性能或六门发布验收。
