# Transport, Protocols and Resource Lifecycle

This reference covers transport and lifecycle behavior that affects an external
`opi-ai` caller. Provider registration and credential selection live in the
[Provider, Catalog, Auth and Refresh reference](provider-auth.md); provider
wire shapes live in the [Provider Wire and Operation Reference](providers.md).

## Proxy and Source-Scoped HTTP

`HttpClient` is the source-qualified HTTP owner. Build one client per source
policy with `HttpClientBuilder`; redirect limits, proxy routing, bypass rules,
header filtering, body limits, acquisition timeouts and typed terminal errors
are fixed before a provider starts its request. Provider code does not create a
second client or read ambient proxy policy behind the caller's back.

Proxy values can come from an explicit provider profile or the host's selected
proxy environment. Credentials embedded in a proxy URL are redacted before an
error or diagnostic is returned. A scoped bypass applies only to the source
that owns it, and loopback peers used by an external test can be reached
without changing the process-wide environment.

A response is admitted only once. Redirects, invalid status handling, body
limits, malformed headers, timeout and network failures become typed
`ProviderError` causes; a partial or bodyless failure is never converted into a
successful provider event. Retry ownership remains with the logical call or
provider adapter, so the transport does not silently resend a request.

## Source-Scoped TLS

`HttpClientBuilder::tls(TlsOptions)` is the public TLS boundary. Trust roots,
server-name policy, ALPN and the selected TLS backend are attached to the
source client, not to a global process client. The host owns native trust and
platform initialization. A provider may select a source profile, but it does
not mutate another provider's TLS policy.

The fixed rustls/AWS-LC/native-certs implementation is owned by the
`tls-backend` feature. A caller that only needs the typed source-scoped
configuration can enable `tls-backend` and use `opi_ai::tls::TlsOptions`;
`http-transport` selects the same owner transitively and keeps the historical
`opi_ai::http` re-exports for `HttpClient` callers.

TLS configuration failures are reported before a request is published. The
client preserves the same timeout, cancellation and redaction rules as HTTP;
closing a caller-owned stream releases the source resource through the normal
lifecycle rather than by dropping a global pool.

## WebSocket Upgrade

The WebSocket/compression capability is owned by the independently selectable
`websocket-compression` feature. It exposes the strict `opi_ai::ws` Upgrade
bridge and keeps RFC 6455 framing, permessage-deflate and async IO state private;
the feature transitively selects `http-transport` and `tls-backend`. Provider
adapters select the same owner transitively, while callers that only need the
public Upgrade boundary can avoid the provider modules.

`ws::upgrade` performs the strict dynamic Upgrade over the shared `HttpClient`.
The public transport preference can select SSE, WebSocket, or an automatic
route. Automatic selection preserves the provider's fallback policy and does
not turn an HTTP failure into an unbounded reconnect loop.

Codex reads WebSocket recovery codes from `error` and from
`response.failed.response.error.code`. The two event shapes share the
same recovery budgets. Codex reconnects once when a frame carries
`websocket_connection_limit_reached` before the first typed provider event.
A second such rejection in the same call falls back to SSE with the original
full input and selected route. This applies to `Auto`, `Websocket`
and `WebsocketCached`. Later calls in the same cache-enabled session use
SSE until resource cleanup clears the fallback marker. A successful upgrade
seals advanced acquisition even if the fallback POST fails. A connection
limit after a typed provider event ends the stream with its partial result.

A missing previous response admits one full-input attempt before any public
stream item. If that attempt fails, the terminal carries the last failure's
wire status and redacted diagnostic. After public output, a missing previous
response ends the call with its partial message.

Before sending a new `response.create` on a pooled socket, Codex checks
immediately readable input. It discards complete application messages
received while idle and answers readable Ping frames. A peer Close,
incomplete input, or exhaustion of the idle read budget retires the
socket and its continuation. A replacement socket starts with full input.

After `response.created` establishes a different ID on a reused socket,
Codex discards terminals whose nested `response.id` names the previous
successful response on that socket. Discarding happens before raw event
callbacks, recovery and message mapping.

On an active Codex WebSocket, a nonzero `Request::timeout` bounds the wait
for the next application message. Ping and Pong use the same deadline.
The next wait starts after the application event's callbacks and delivery.
On expiry Opi attempts a bounded Close with code `1000` and reason
`idle_timeout`. Before the first typed provider event the call falls back
to SSE with the full request. After that event the call returns its partial
message and a stream error. A cache-enabled session routes subsequent calls
to SSE until resource cleanup. Zero disables this WebSocket idle timer.

The handshake validates the HTTP status, version, `Connection` and `Upgrade`
headers, `Sec-WebSocket-Accept`, subprotocol selection, singleton extension
parameters and permessage-deflate ranges before returning the upgraded stream.
Invalid negotiation fails before provider events are emitted. The upgraded
stream retains its source proxy/TLS policy and caller cancellation token.

The frame, PMD and IO owners are implementation details. Consumers use the
provider or `ChatCall`/`ChatStream` contracts and do not depend on a codec type.
If a backend does not expose a public WebSocket route, use its documented SSE
or HTTP route instead.

## Chat Completion Boundary

OpenAI Chat, OpenRouter, the legacy Mistral Chat profile and Azure Chat treat
`finish_reason` and usage frames as metadata. Text, thinking and tool blocks
remain open until `[DONE]` or natural HTTP EOF. At a valid end, blocks close
in content order, tool arguments are finalized, and one `Done` is emitted.
The adapter releases the response at the sentinel without waiting for HTTP
EOF; later frames do not reach raw callbacks or alter the cached result.

Provider, malformed-frame, usage, transport and callback failures before that
end return the accumulated partial instead of success. Explicit cancellation
preempts queued events and returns `Aborted` without synthesizing block ends.
This preserves Opi's cancellation contract; Pi 1.5.0 can close blocks during
its OpenAI SDK abort unwind. Only a successful terminal permits Agent tools.

A missing finish reason fails by default. A route with
`supports_finish_reason=false` infers `Stop` or `ToolUse` at the sentinel/EOF
without inventing `raw_stop_reason`. Azure's `with_compat` settings survive
simple dispatch. HTTP SSE fragments without a final blank line are not dispatched.

Custom parser/mapper callers handle `ParsedEvent::Done` and invoke
`OpenAiChatMapper::finish(supports_finish_reason)` at the protocol end. It
returns the complete end-event sequence, replacing `flush_pending_done()`;
stop reading after any terminal or failure.

## Codex Terminal Responses

On SSE and WebSocket, Codex accepts `response.done`, `response.completed`,
and `response.incomplete` as terminal frames. The adapter reads the response
status, ID, reported model and usage from that frame. A normal completion
with a tool call returns `ToolUse`. An incomplete response with
`max_output_tokens` returns `Length`. Other incomplete reasons, missing
incomplete reasons, and the provider statuses `failed` and `cancelled`
return typed stream failures with the partial message.

`AssistantMessage::end_turn()` exposes the terminal's boolean `end_turn`
as a diagnostic. Both boolean values survive serialization in
`AssistantMessage::diagnostics`. Use `stop_reason` to classify the result.
Cached continuation uses the successful terminal's response ID. The raw
observer sees the terminal's original wire frame.
Codex releases the SSE body after delivering the first terminal, even
when the peer keeps the body open.

## Resource Generation, Pool Cleanup and Awaited Release

A provider call owns a resource generation for the lifetime of its session.
Acquisition, use, abort and release are ordered operations. A captured call
keeps the adapter/resource revision it selected; deleting or replacing a
provider registration does not invalidate an already captured call's cleanup.

`Models` attempts every owner's release and returns an aggregate cleanup result.
A caller that drops a consumer without cancelling the request does not imply
that the provider generation ended. Explicit cancellation, terminal provider
status and resource cleanup are separate observations. Blocking cleanup is
awaited before the owner reports a completed release, and late results are
suppressed rather than published into a new generation.

For a raw prepared attempt, the caller owns the attempt boundary. The observed
`ChatStream` helpers add the standard result/observer settlement; the raw seam
remains available when a host needs to install its own lifecycle recorder.

## Deferred Transport Semantics

Deferred submission is an ordinary generation that may return an assistant
message with `StopReason::Deferred`, empty content and a typed handle. The
handle is caller-owned. A subsequent fetch creates a normal `ChatStream`, and
`Models::fetch_deferred` drives it to a message with `StopReason::Deferred`
and the handle if the response is still pending, a ready message, or a typed
failure. Pending polls skip token and cost accumulation. `poll_after_ms` is a
provider scheduling hint for the caller.

`Models::fetch_deferred` drives `ChatStream::result()`. On normal completion,
it caches the successful terminal, drains the source to EOF, releases it,
and finishes an enabled poll observer before returning when telemetry
recording succeeds. `wait_ms` sets the provider's long-poll budget. A
pending source tail can keep the fetch pending after the terminal,
including for a zero-wait status check.

For a Pi caller awaiting `Models.fetchDeferred` or
`Models.streamDeferred(...).result()` on Tokio, use
`models.fetch_deferred_ready(&handle, call).await?`. It dispatches one fetch
with the same routing, authentication and provider wait. The first normalized
terminal yields a pending message with its handle, a ready message, or a typed
failure. The task sends the outcome before draining the source and settling
the poll. A pending tail can continue after the future returns or a polled
waiter is dropped. An immediately available tail may finish before the caller
resumes. A caller can schedule its next fetch as soon as it gets a pending
message. Retain the cancellation token to release a pending local tail. To
await source release and poll settlement, use `fetch_deferred` or retain the
started stream and await its `result()`.

When porting application error handling, handle `Err(CallFailure)` where
the application classifies and records failed assistant messages. Use
`CallFailure::message()` or `into_parts()` to retain the canonical partial,
stop reason and available diagnostic. Pass the message to
`retry::classify_whole_call_retry` for the whole-call decision.
`ProviderError::is_retryable()` classifies the typed cause.
The application owns its retry budget, backoff and any compaction policy.

If the auth resolver returns `ProviderError::AuthFailed`, fetch ends before
provider dispatch. The failure retains that cause. Its synthesized message
contains the requested provider and model, `StopReason::Error` and the safe
resolver diagnostic. Use that message to record a failed poll. A resolver
diagnostic can describe a transient failure even when the typed cause is
`AuthFailed`.

A legacy provider `Error` terminal with `Aborted` in both stop fields keeps
its message and carries `ProviderError::Cancelled`. Check the application's
abort state before classifying a fetch outcome. With an active task, an
`Aborted` message ends whole-call retry.
An application that abandons the deferred job can call `cancel_deferred`
with the retained handle.

The collector returned by `Models::stream_deferred` is caller-driven. For a
valid handle, polling it drives auth and dispatch. Iteration can deliver
`Done` before source EOF. The next read drives the same tail drain,
releases the source and finishes
the enabled poll observer before returning public EOF. A pending tail keeps
that read pending. If a caller drops the pending `next()` future and keeps
the collector, `result()` or `.start()` can resume draining the same source
and reuse the cached outcome. Calling `result()` after public EOF also
reuses that outcome. Dropping the direct collector after a successful
`Done` releases the source at its current position. The collector then
settles the enabled poll observer once using the cached message. This also
works if the next read already reached a pending tail. Drop sends no remote
cancel.

To migrate Pi's independently driven `Models.streamDeferred`, use
`Models::stream_deferred(...).start()` within a Tokio runtime. Starting a fresh
collector schedules the fetch before the first event read. The task caches
the first successful `Done` before forwarding it and then drives the same
drain. Handle `Done` inside the loop to continue as soon as the message is
available. In Pi, the loop can end while the source tail is pending. In Rust,
read through public EOF or await `result()` to observe source release and
settlement of the enabled poll observer. After a direct read of `Done`,
`.start()` takes over the retained tail and reuses the same result.
Dropping the started handle ends consumption while the task continues
driving the source and settling the poll.

If `abort()` stops the task before a successful `Done` is cached, it waits
for source release and returns typed `Cancelled`. Once the result is cached,
`abort()` still joins source release and returns that result, including
when `Done` was buffered but unread. Joining a task stopped by runtime
shutdown after caching likewise returns the result and settles the poll
after release. Before caching, runtime shutdown surfaces as a typed stream
failure. These interruptions concern the local fetch. Remote cancellation
uses the separate `cancel_deferred` entry.

The SDK looks up the handle's exact API and provider and then checks its
model before resolving auth. `Models::validate_deferred_handle` returns
`DeferredError::Unsupported` for a missing provider/API registration and
`DeferredError::RouteMismatch` when the resolved model differs from the
handle. Fetch and explicit cancellation report either as
`ProviderError::Config`. The missing registration diagnostic identifies
the requested provider and API.

The handle selects the deferred registration. An application that saves
an expected route alongside the handle can compare the two before calling.
After dispatch, the backend uses the job ID to decide whether it can
return the result or cancel the job. The SDK preserves a typed backend
fetch error as the cause of `CallFailure`.

`DeferredFetchCall::wait_ms` sets the provider's long-poll budget in
milliseconds. Zero requests a single status check. Each call that reaches
the backend dispatches one fetch with the supplied value. The backend
determines when to return a pending or ready outcome. The caller starts
any subsequent fetch.

With `now_ms` omitted, the SDK forwards the entire handle, including
`expires_at_ms` when present. The backend decides whether the result is
still available. A caller can reuse a saved handle after reopening.

`DeferredFetchCall::now_ms` opts into a local expiry check. Use it when the
provider specifies a hard deadline for retrieving the result and the caller
has a comparable clock in Unix epoch milliseconds. At or after expiry the
SDK returns `Err(CallFailure)` with cause `ProviderError::Config` before
resolving auth or dispatching a fetch. Use `Models::validate_deferred_handle`
to obtain `DeferredError::Expired` directly.

A failure synthesized before a partial arrives carries the provider and
model IDs from the handle.

`DeferredFetchCall::cancel` controls the local fetch in all three entry points.
When cancellation ends the fetch before a terminal is cached, the result is
`Err(CallFailure)` with cause `ProviderError::Cancelled` and message stop
reason `StopReason::Aborted`. The SDK releases the pending source. Direct
`fetch_deferred` settles the enabled poll observer before returning when
recording succeeds. If the ready result is already cached from a successful
`Done`, cancellation can end a pending drain before natural EOF.
The SDK releases the local source and settles the enabled poll
observer from the cached ready result when recording succeeds. The direct
fetch returns that result. For a direct iterator waiting on its last read,
that cancellation lets the read return public EOF.
Use the separate `Models::cancel_deferred` operation for best-effort
remote cancellation.

Deferred routes register through the typed deferred API. There is no hidden
startup fetch or global deferred worker, and no real remote service is implied
by the deterministic Faux backend.

## Public Transport Boundaries

Use the public `Models`, `Request`, `ChatCall`, `ChatStream`, `HttpClient` and
provider registration contracts. The internal frame codec, compression state,
resource owner and protocol fixture layout can change without changing those
contracts. Exact trait signatures and option fields belong to rustdoc; this
page records only the behavior an external caller must rely on.
