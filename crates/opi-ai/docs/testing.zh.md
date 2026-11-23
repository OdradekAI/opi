# 离线测试 SDK 接入

[English](testing.md) · [SDK 快速开始](../README.zh.md#快速开始)

## 模拟一次响应

用 `test_support::MockProvider` 模拟模型事件，并运行应用原有的请求和响应代码。
`single_route_collection` 会准备一个可发起调用的 `Models`，附带测试用凭据
和一条 `mock-model` 聊天记录。

把下面的测试加到快速开始的 `src/main.rs`，再运行 `cargo test`。

```rust
#[cfg(test)]
mod tests {
    use opi_ai::message::assistant_content_text;
    use opi_ai::test_support::{MockProvider, single_route_collection, text_response};

    #[tokio::test]
    async fn reads_a_scripted_reply() {
        let provider = MockProvider::new("test", vec![text_response("ready")]);
        let models = single_route_collection(Box::new(provider));

        let reply = super::ask(&models, "test", "mock-model", "Is the build ready?")
            .await
            .expect("scripted reply");
        assert_eq!(assistant_content_text(&reply.content), "ready");
    }
}
```

每次预期的调用尝试对应一组响应事件。要测试错误或取消，用
`MockProvider::new_with_errors` 配合 `MockResponse::EventsThenError`
或 `HangingUntilCancelled`。延迟响应流程可用 `ScriptedDeferredBackend`
模拟等待中和已就绪的结果。

需要验证端点路径、请求头或请求体时，可以使用本地 HTTP 测试服务。
crate 的测试指南包含这些测试和外部包验证的运行方法。[贡献者测试指南](../tests/README.zh.md)
