# #233 对齐 Bedrock 严格工具限额重发

Status: done

在 [#232](232-unify-amazon-bedrock-catalog-entrypoints.md) 通过验收后实施。行为以 Pi 1.5 的 `packages/ai/src/api/bedrock-converse-stream.ts`、`packages/ai/src/api/constrained-sampling.ts` 与 `packages/ai/test/bedrock-strict-tool-fallback.test.ts` 为依据。

## 实现边界

Converse 适配器记录普通 payload 是否投影出严格 `Prefer` 工具。限额分类只对交付流前的 HTTP 400 读取有界错误体，归一并按响应头、大小写不敏感的 `code`、`__type` 查找错误身份，再检查 `message` 或 `Message` 中的三个固定标记。正文不进入诊断。字段优先级核对了 Pi `package-lock.json` 锁定的 `@aws-sdk/core@3.977.9` 的 `dist-es/submodules/protocols/json/parseJsonBody.js`、`AwsRestJsonProtocol.js` 和 `@smithy/core@3.33.3` 的 `dist-es/submodules/client/smithy-client/exceptions.js`。

`AcquisitionCore` 授权一个 Bedrock payload 变体。两份 payload 各自消耗 Smithy 次数预算，共用冻结认证、逻辑调用 ID、配额和限速器。Opi 的 prepared 调用立即调度获准的变体，沿用现有普通重试和 adaptive 调度。Pi 锁定的 Smithy 版本在每个 SDK 命令生成 wire invocation ID，并在 adaptive 模式中为新命令获取发送令牌。底层 `BedrockWireAdapter::stream_simple` 发送单次尝试，`Models` 和 agent 使用调用级调度。

## 验收

使用真实 `Models` 和本地 Converse 服务。首次请求含实际启用严格模式的 `Prefer` 工具且服务在交付流前返回命中 Pi 固定标记的 `ValidationException` 时，允许重建一次。第二份 payload 将 `Prefer` 恢复为原始 schema，保留 `Require` 与普通工具，再次运行 payload 回调并使用冻结的认证重新签名。成功响应回调观察一次，结果携带一次 `bedrock_strict_tools_fallback` 诊断。

用仅 `Require`、普通校验错误、错误类型不匹配、第二次命中限额以及流已交付后的错误验证重发边界。沿用已准备调用的 acquisition 行为，覆盖 Smithy 普通传输重试与独立 payload 的计数。

Bedrock 专项回归可单独执行。

```sh
cargo test --locked --offline -p opi-ai --test bedrock_fixtures smithy_retry::strict_tool_limits
```

## 验收证据

- 初始专项复现限额拒绝仅发送一次。新增的大小写不敏感 `code` 和大写 `Message` 用例也先复现了正常 400 终止。修复后专项 4 项通过，覆盖相互冲突字段的优先级、取消、重复限额与交付后错误。
- 公共 simple 调用抓取 `504 → 400 ValidationException → 504 → 200`，两份 payload 的 Smithy 次数各为 `1, 2`。认证解析一次。四次请求的 SigV4 内容哈希均与请求体匹配，变体之间签名不同。两次 payload 回调及一次响应回调。成功和重发后的终止失败均携带一次降级诊断。
- `bedrock_fixtures` 186 项、`bedrock_pending_metadata` 1 项、`bedrock_provider_wiring` 8 项通过。共享回归包括 `anthropic_fixtures` 155 项、`whole_call_retry` 23 项、`node4_preparation` 26 项、`telemetry_operations` 12 项。Agent 的 `agent_loop_mock` 与 `provider_public_safety` 共 12 项通过。
- 独立 `provider-bedrock` 和无默认 feature 的 core profile 通过。最终 `.crate` 的独立 Bedrock 消费者验收通过，归档 SHA-256 为 `74026fa58ff43f3397f671332a0cb7d3727083829f4230218d88c49bb0a08064`。定向 Clippy 通过并报告 3 项既有库 lint，rustdoc 使用 `-D warnings` 构建通过。

Test impact — add Bedrock 限额重发的 4 个专项用例。retain Anthropic 降级、Smithy 普通传输重试、Agent 调度和 Converse 事件流回归。
