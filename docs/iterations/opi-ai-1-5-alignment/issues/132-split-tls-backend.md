# #132 Split the TLS backend owner

Status: completed for the fixed source-scoped TLS backend; backend selection
and WebSocket/compression remain separate follow-up boundaries.

## Decision

`opi-ai` now exposes `tls-backend` independently. It owns the fixed
rustls/AWS-LC/native-certs implementation and the public
`opi_ai::tls::{TlsOptions, TrustRoots, ClientIdentity, AlpnPreference}`
configuration surface. It does not select reqwest, provider protocol modules,
catalog data or provider authentication. `http-transport` transitively selects
`tls-backend` and keeps the historical `opi_ai::http` TLS re-exports, so HTTP
callers retain their existing API and dependency behavior.

## Evidence

- The native TLS-only profile and warnings-denied Clippy pass on Windows.
- The HTTP transport profile still passes with the expanded feature union
  (`http-transport`, `tls-backend`), and the explicit builtins HTTP profile
  remains green.
- The actual `opi-ai` archive is consumed outside the workspace with only
  `tls-backend`; metadata contains rustls/AWS-LC/native-certs, excludes reqwest
  and provider owners, and the external caller executes the public TLS
  trust/ALPN contract.
- The caller inventory records direct TLS API, HTTP bridge and backend fixture
  users, with source hashes preserved.

Evidence: [combined native/package/caller receipt](../evidence/phase8-tls-backend-20261115.json),
[current TLS backend package receipt](../evidence/phase8-tls-backend-package-20261115-v2.json),
[historical TLS backend package receipt](../evidence/phase8-tls-backend-package-20261115-v1.json),
[caller inventory](../evidence/phase8-tls-backend-callers-20261115-v1.json).

## Deliberate scope and remaining work

The backend is fixed to the current rustls/AWS-LC/native-certs profile; this
slice does not add a caller-selectable crypto implementation or native-tls
alternative. Provider protocol adapters, cloud-auth codecs,
WebSocket/compression, complete capability V, macOS and #94 remain open.

Test impact: add the `tls-backend` profile, public TLS module gate, native
contract test, package runner/unit tests and caller inventory; update feature,
transport/testing documentation, capability mapping, changelog and spec;
delete none; retain prior HTTP, protocol, OAuth/store, Pi and platform receipts.
