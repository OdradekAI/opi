# Test an SDK integration offline

[简体中文](testing.zh.md) · [SDK quick start](../README.md#quick-start)

## Script a response

Use `test_support::MockProvider` to run your application's real request and
response code with scripted events. `single_route_collection` sets up a
dispatchable `Models` collection with synthetic credentials and a
`mock-model` chat record.

Add this test to the quick start's `src/main.rs` and run `cargo test`.

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

Give the provider one response batch per expected attempt. To exercise
failures or cancellation, use `MockProvider::new_with_errors` and
`MockResponse::EventsThenError` or `HangingUntilCancelled`.
For a deferred flow, `ScriptedDeferredBackend` lets you script pending
and ready responses.

Use a controlled HTTP peer to verify wire details such as endpoint paths,
headers and payloads. The crate's test guide describes how to run those
fixtures and the external package checks. [Contributor test guide](../tests/README.md)
