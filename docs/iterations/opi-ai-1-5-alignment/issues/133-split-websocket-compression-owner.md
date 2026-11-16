# #133 Split the WebSocket/compression owner

Status: completed for the public WebSocket/compression owner; provider-specific
protocol/source and cloud-auth boundaries remain follow-up work.

## Decision

`opi-ai` now exposes `websocket-compression` independently. It owns the public
`opi_ai::ws` Upgrade bridge plus the private RFC 6455 frame/IO and
permessage-deflate implementation, and selects the shared `http-transport`
owner together with `libz-rs-sys` and `flate2`. It does not select
`protocol-adapters`, catalog data, provider collection or cloud-auth codecs.

`protocol-adapters` transitively selects `websocket-compression`, so Codex and
other provider WebSocket callers keep their existing feature path and wire
behavior. `builtins` and the default feature set retain their previous surface.
This is a packaging boundary change; the frozen Pi 1.5.0 WebSocket differential
receipts remain the runtime behavior control and are not rewritten.

## Evidence

- Windows native standalone checks pass: feature check, the public profile (2
  tests), Upgrade contract (16 tests), private frame/IO/PMD suite (96 tests),
  and warnings-denied Clippy. The first provider-only test and standalone
  dead-code failures are retained as historical repair receipts.
- The actual `opi-ai` archive is consumed outside the workspace with only
  `websocket-compression`; the graph contains HTTP/TLS and compression packages
  and excludes protocol, catalog and provider-auth packages. The external
  caller negotiates PMD and executes the public Upgrade contract.
- The caller inventory scans 665 Rust files and records 184 public WS-surface,
  189 provider-WS and 43 direct-compression matches with source hashes.
- HTTP and protocol compatibility was rerun after the split: native HTTP and
  protocol profiles, protocol Clippy, builtins HTTP acceptance, HTTP package
  consumption and protocol package consumption all pass. The protocol archive
  now records the exact feature union including `websocket-compression`.

Evidence: [combined WebSocket/compression receipt](../evidence/phase8-websocket-compression-20261115.json),
[native/package receipt](../evidence/phase8-websocket-compression-package-20261115-v3.json),
[caller inventory](../evidence/phase8-websocket-compression-callers-20261115-v1.json),
[current HTTP package receipt](../evidence/phase8-http-transport-package-20261115-v5.json),
[current protocol package receipt](../evidence/phase8-protocol-adapters-package-20261115-v7.json).

## Deliberate scope and remaining work

The owner preserves the current rustls/AWS-LC HTTP/TLS chain and PMD behavior;
it does not add a provider-selectable WebSocket backend or change the frozen Pi
wire contract. Per-provider protocol/source owners, cloud-auth families,
complete capability V, native macOS and #94 remain open.

Test impact: add the standalone `websocket-compression` profile and external
consumer runner/unit tests plus caller inventory; update protocol feature
closure, bilingual README/transport/testing docs, capability mapping, changelog
and spec; delete none; retain all prior Pi, HTTP, protocol, TLS, OAuth/store,
adapter/observer and failed-attempt receipts.
