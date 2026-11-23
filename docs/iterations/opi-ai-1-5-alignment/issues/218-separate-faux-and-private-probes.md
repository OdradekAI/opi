# #218 划定公开 Faux 与私有测试探针

Status: done
Owner: test_support、公用测试消费者、identity
Parent: [#209](209-maintain-module-test-and-catalog-boundaries.md)
Blocked by: none

doc-hidden 的 test_support 同时承载下游调用的 MockProvider/ScriptedDeferredBackend 和 force_sequence_exhausted 等私有状态控制。

## 验收

列出 workspace 与外部示例实际使用的公开能力，保留可脚本化服务与 deferred 行为。把只为验证私有状态的用例和探针移至所属模块；为公开路径变更更新消费者和双语文档。

## 验证

运行 `public_utilities`、`test_support_tests`、`deferred_contract` 和迁入的 lib 测试；从 minimum 实际归档构建使用正式 Faux 的程序。核查旧私有探针不再由生产 crate 导出，引用它的工作区测试已迁移。

## 归属与验证 · 2026-11-20

| 接口或测试 | 实际消费者与归属 |
| --- | --- |
| `MockProvider`、`MockResponse`、`single_route_collection`、事件构造器 | README 和包测试通过 Models 使用脚本化 chat。opi-agent、opi-coding-agent 和 differential 示例复用这些入口。保留 `opi_ai::test_support` 路径并将正式接口展示在 rustdoc。 |
| `ScriptedDeferredBackend`、`MockProvider::with_deferred` | `deferred_contract`、`lifecycle_differential`、下游 session 用例，以及归档的 `core_profile`。实现中的 `DeferredScripting` 归私有状态。 |
| `CountingAuthResolver`、预解析凭据与平台 helper | 现有 workspace 的 integration 示例和认证测试使用。保留 doc-hidden 的调用路径。 |
| `FakeSourceTransport`、`NeverTransport` | 实际消费方为 `bedrock_fixtures`，迁入其 `aws_source_transport.rs` 子模块。改 helper 文件可由选测器定位到同一个 Bedrock 目标。 |
| UUID sequence 探针 | 唯一活跃调用原在 `public_utilities`。两个 `SequenceExhausted` 断言迁进 `identity::tests`，直接用私有状态构造耗尽。可公开调用的生成和错误类型继续由 integration 目标验证。 |

改前 `public_utilities`、`test_support_tests`、`deferred_contract` 为 54、26、19 项。迁移后分别为 53、26、19 项，`identity` 下迁入的 1 项在 minimum 配置通过。Bedrock fixture 完整执行 179 项。程序代码中的 sequence setter 和 SDK 导出的耗尽入口已移除。Unreleased 记录原测试路径的调整。

实际 `opi-ai-0.8.3.crate` 在工作区外以关闭默认 feature 的方式消费。minimum 与仅加 `telemetry-tracing` 分别执行 21、22 项，均包含脚本化工具续轮和新增的 deferred 提交、pending fetch、ready fetch。两个依赖图均通过；旧 `force_sequence_exhausted` 导入单独编译得到预期 `E0432`。minimum rustdoc 包含 `MockProvider` 与 `ScriptedDeferredBackend` 的文档。受影响目标普通 Clippy 完成，仍报告已记录的 3 项库警告。[逐目标、归档与负例证据](../evidence/issue-218-faux-and-private-probes.json)

Test impact: update UUID 用例归属、Bedrock fixture 与 minimum runner。add 下游归档的 deferred 生命周期用例及私有探针编译负例。retain 正式 Faux 行为、两个 UUID 耗尽断言和 Bedrock 行为测试。
