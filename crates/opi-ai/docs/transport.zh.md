# HTTP、流式调用与延迟响应

[English](transport.md) · [SDK 快速开始](../README.zh.md#快速开始)

## 配置代理或自定义 CA

`HttpClient::new()` 默认直连。要采用进程的代理环境变量，将
`proxy_from_env()` 传给 builder。固定代理和绕过列表可以通过 `ProxyConfig`
指定。构建一次客户端，注册时传入 `Arc<HttpClient>`，后续请求即可复用连接池。

以下示例使用进程的代理配置，并额外信任 PEM 字节指定的 CA。
需要使用操作系统的根证书时，可选 `TlsOptions::use_host_system_roots()`。

```rust
use opi_ai::http::{HttpClient, HttpClientBuilder, proxy_from_env};
use opi_ai::tls::TlsOptions;

pub fn client_for_proxy(ca_pem: &[u8]) -> Result<HttpClient, Box<dyn std::error::Error>> {
    let tls = TlsOptions::new().with_extra_roots_pem(ca_pem)?;
    Ok(HttpClientBuilder::new()
        .proxy(proxy_from_env())
        .tls(tls)
        .build()?)
}
```

代理、重定向和 TLS 配置作用于当前客户端。WebSocket 升级复用代理和 TLS 配置，
并拒绝重定向。配置错误会在 `HttpClientBuilder::build()` 时返回。要设置请求超时，可传
`Request::timeout`，具体边界由 adapter 的传输规则决定。

## 结束或取消聊天调用

- 在 Tokio 上调用 `Models::stream_simple` 会启动聊天并返回 `ChatCall`。
  可以一边读取增量一边显示，通过 `Done` 取最终消息，或用
  `call.result().await` 等待同一次调用的结果。
- Chat Completions 可能先发送 `finish_reason`，再发送最后的用量或错误。
  使用完整结果前，等 SDK 发出 `Done`。失败时，`CallFailure::cause()`
  返回具体错误，`message()` 保留已生成的部分。成功时也可同时查看
  `AssistantMessage::raw_stop_reason` 和规范化的 `stop_reason`。
- 在发起调用前克隆请求的 `cancel` 令牌，需要停止时将其取消。持有
  `ChatCall` 时，可用 `abort().await` 等待任务退出。放弃读取或等待结果后，
  任务仍可能继续。进行中的请求被取消时，SDK 返回 `Aborted` 和
  `Cancelled` 错误。
- 使用 `session_id` 复用 Codex 连接时，在关闭会话时等待
  `models.cleanup_session_resources(Some(session_id))`。结果涵盖仍注册和
  已移除的供应商连接资源。传 `None` 可清理所有会话。

重试与诊断见[观察请求与处理失败](utilities.zh.md#观察请求与处理失败)。

## WebSocket Upgrade

Codex 可通过 `Request::transport` 选择 `TransportPreference::Sse`，只走
SSE。`Auto`、`Websocket` 和 `WebsocketCached` 可用 WebSocket，并在首个
供应商事件之前按规则回退到 SSE。该事件之后发生失败时会返回部分结果。
Codex WebSocket 上的非零 `Request::timeout` 限制等待下一条应用消息的时间。

自行接入传输时，可启用 `websocket-compression`，使用公开的
`opi_ai::ws::upgrade` 完成升级。普通 `Models` 调用由 adapter 选择传输方式。

## Deferred Transport 语义

通过 `Models::register_deferred_api` 接入后端后，`submit_deferred` 可能返回
`StopReason::Deferred`。应用可保存 `message.deferred` 中的 `DeferredHandle`，
以便稍后恢复。handle 标识供应商、模型和 API。重启应用后要接回匹配的后端。

| 应用需要 | API | 返回时机 |
| --- | --- | --- |
| 查询一次并等待本地清理 | `fetch_deferred(&handle, call).await` | 响应源释放、轮询观测结束后返回。 |
| 查询一次并尽快拿到结果 | `fetch_deferred_ready(&handle, call).await` | 拿到首个规范化结果后即返回，本地清理由 Tokio 任务继续完成。 |
| 读取单次查询的流事件 | `stream_deferred(&handle, call)` | 可由调用方轮询驱动，也可在 Tokio 上调用 `.start()` 独立运行。`Done` 携带结果，EOF 和 `result()` 等待清理。 |
| 请求取消远端任务 | `cancel_deferred(&handle, cancel_call).await` | 单独向远端发起尽力取消。 |

`DeferredFetchCall` 默认 `wait_ms: 0`，只查一次状态。正值指定由后端解释的
长轮询等待预算。任务仍在等待时，查询结果也为 `Deferred`。使用返回的 handle
和 `poll_after_ms` 提示安排下次查询。需要在本地拒绝已过期的 handle 时，通过
`now_ms` 传入 Unix 毫秒时间。发起查询前，可用 `Models::validate_deferred_handle`
校验路由和有效期。未传入时间时，由后端处理过期。

`DeferredFetchCall::cancel` 中止本地查询或收尾。若结果已缓存，取消收尾会保留
结果。另用 `cancel_deferred` 停止远端任务。丢弃调用方驱动的流会释放响应源，
丢弃由 `.start()` 启动的流 handle 后，本地任务仍会继续读取。
