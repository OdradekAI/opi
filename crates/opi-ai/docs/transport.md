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

TLS configuration failures are reported before a request is published. The
client preserves the same timeout, cancellation and redaction rules as HTTP;
closing a caller-owned stream releases the source resource through the normal
lifecycle rather than by dropping a global pool.

## WebSocket Upgrade

`ws::upgrade` performs the strict dynamic Upgrade over the shared `HttpClient`.
The public transport preference can select SSE, WebSocket, or an automatic
route. Automatic selection preserves the provider's fallback policy and does
not turn an HTTP failure into an unbounded reconnect loop.

The handshake validates the HTTP status, version, `Connection` and `Upgrade`
headers, `Sec-WebSocket-Accept`, subprotocol selection, singleton extension
parameters and permessage-deflate ranges before returning the upgraded stream.
Invalid negotiation fails before provider events are emitted. The upgraded
stream retains its source proxy/TLS policy and caller cancellation token.

The frame, PMD and IO owners are implementation details. Consumers use the
provider or `ChatCall`/`ChatStream` contracts and do not depend on a codec type.
If a backend does not expose a public WebSocket route, use its documented SSE
or HTTP route instead.

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
`Models::fetch_deferred` drives it to one ready assistant result or one typed
failure.

Pending results are non-terminal for token and cost accumulation. Ready results
carry the final message; `poll_after_ms` is a provider hint, not an implicit
sleep policy. `cancel_deferred` is best-effort remote cancellation and is
independent from cancelling a local fetch. A cancelled or superseded operation
cannot publish a late result.

Deferred routes register through the typed deferred API. There is no hidden
startup fetch or global deferred worker, and no real remote service is implied
by the deterministic Faux backend.

## Public Transport Boundaries

Use the public `Models`, `Request`, `ChatCall`, `ChatStream`, `HttpClient` and
provider registration contracts. The internal frame codec, compression state,
resource owner and protocol fixture layout can change without changing those
contracts. Exact trait signatures and option fields belong to rustdoc; this
page records only the behavior an external caller must rely on.
