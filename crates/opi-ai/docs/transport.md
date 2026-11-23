# HTTP, streaming, and deferred responses

[简体中文](transport.zh.md) · [SDK quick start](../README.md#quick-start)

## Configure a proxy or custom CA

`HttpClient::new()` connects directly. To use proxy variables from the
environment, pass `proxy_from_env()` to the builder. For a fixed proxy or
bypass list, pass an explicit `ProxyConfig`. Build the client once and pass
an `Arc<HttpClient>` to registration to share the pool between requests.

This example uses the process proxy settings and trusts an additional CA from
PEM bytes. To use OS trust roots for a client, select
`TlsOptions::use_host_system_roots()`.

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

Proxy, redirect and TLS settings apply to that client. WebSocket upgrades
share proxy and TLS settings and reject redirects. `HttpClientBuilder::build()`
reports invalid configuration at setup.
Use `Request::timeout` to set a request timeout within the adapter's transport
rules.

## Finish or cancel a chat call

- Call `Models::stream_simple` on Tokio to start chat and iterate its
  `ChatCall`. Render deltas as they arrive. Use `Done` for the finished
  message or await `call.result()` for the same execution's outcome.
- For Chat Completions, `finish_reason` can precede final usage or a late
  error. Wait for the SDK's `Done` before taking action on the final result.
  On errors, `CallFailure::cause()` provides the typed error and `message()`
  provides the partial response. A successful response can also have
  `AssistantMessage::raw_stop_reason` alongside its normalized `stop_reason`.
- Clone the request's `cancel` token before dispatch and cancel it to stop
  the call. If you own the `ChatCall`, use `abort().await` to stop and join it.
  Dropping the reader or result waiter can leave the task running. Local
  cancellation reports `Aborted` and a typed `Cancelled` cause when it
  interrupts an active request.
- If you use a `session_id` for pooled Codex calls, await
  `models.cleanup_session_resources(Some(session_id))` when you close that
  session. The result reports resource release for its active and retired
  registrations. Use `None` to clean up all sessions.

See [failure handling and callbacks](utilities.md#inspect-requests-and-handle-failures)
for retry and diagnostics.

## WebSocket Upgrade

For Codex, set `Request::transport` to `TransportPreference::Sse` to use
SSE only. `Auto`, `Websocket` and `WebsocketCached` allow WebSocket and
bounded fallback to SSE before the first provider event. After that event,
a failure returns the partial response. A nonzero `Request::timeout` on
Codex WebSocket bounds the wait for the next application message.

Custom transports can use `opi_ai::ws::upgrade` with
`websocket-compression` for the public Upgrade handshake. Normal
`Models` calls select the adapter's transport.

## Deferred transport semantics

A backend that you attach with `Models::register_deferred_api` can return
`StopReason::Deferred` from `submit_deferred`. Persist the message's
`DeferredHandle` from `message.deferred` if the app will resume the job later.
The handle identifies the provider, model and API. Reattach a compatible
backend when reopening the app.

| What the app needs | API | When it returns |
| --- | --- | --- |
| Fetch once and await local cleanup | `fetch_deferred(&handle, call).await` | Returns after release of the response source and settlement of the poll observer. |
| Fetch once and continue as soon as the outcome is known | `fetch_deferred_ready(&handle, call).await` | Returns the first normalized outcome. The Tokio task continues local cleanup. |
| Read events from one fetch | `stream_deferred(&handle, call)` | Poll the stream to drive it yourself or call `.start()` on Tokio to run it independently of reads. `Done` delivers the message. EOF and `result()` await cleanup. |
| Request cancellation of the remote job | `cancel_deferred(&handle, cancel_call).await` | Performs a separate best-effort remote cancel. |

A default `DeferredFetchCall` uses `wait_ms: 0` for a single status check.
A positive value is the backend's long-poll budget. A pending fetch returns
`Deferred` again. Use its handle and `poll_after_ms` hint to schedule another
fetch. To reject expired handles locally, pass the current Unix time in
milliseconds via `now_ms`. A caller can check routing and expiry before fetch
with `Models::validate_deferred_handle`. Without a clock value, the backend
handles expiry.

`DeferredFetchCall::cancel` stops the local fetch or drain. If the result is
already cached, cancellation of the drain preserves that result. Use
`cancel_deferred` if you also want to stop the remote job. Dropping a direct
stream releases its source. Dropping a started handle leaves its task driving
the source.
