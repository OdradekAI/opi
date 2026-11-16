# #129 Split the shared HTTP transport owner

Status: complete for the shared transport slice; provider protocol and auth
capabilities remain open.

The previous `builtins` group exposed the public `HttpClient` only as a side
effect of enabling every provider adapter, authentication codec, catalog and
WebSocket/compression backend. This slice gives the shared HTTP transport its
own `http-transport` feature. The owner includes the public `HttpClient` and
builder, proxy and redirect policy, bounded HTTP error classification, and the
source-scoped TLS/crypto wiring required to build the client. It does not
enable concrete provider protocol modules or built-in authentication.

`builtins` now selects `http-transport` transitively, so the default profile
and existing `features = ["builtins"]` callers retain the same public provider
surface and transport defaults. The provider protocol row remains coarse on
purpose: its modules still use the unresolved auth/provider ownership inside
`builtins`, and no protocol-only claim is made here.

The native Windows transport-only profile passed the three public contract
tests and warnings-denied Clippy. The same test passed with the default
profile and with explicit `builtins`, proving the compatibility path. The
actual `opi-ai-0.8.3.crate` was consumed by a fresh workspace-external caller
with `default-features = false, features = ["http-transport"]`; metadata showed
exactly that feature on `opi-ai`, included reqwest/rustls/AWS-LC transport
dependencies, and excluded cloud-auth codecs, catalog, and compression owners.
The caller built and executed `HttpClient::new`, proxy normalization and the
frozen redirect/pool contract from the extracted archive.

The caller audit found the public `HttpClient` construction sites in all
provider adapters and shared catalog/auth paths. They remain source-compatible
because those modules are still selected by `builtins`, which now transitively
selects `http-transport`; no no-default caller in the repository relied on the
HTTP module without an explicit provider feature. The independent package
consumer demonstrates the new supported caller shape.

This slice does not close the `protocol-adapters`, cloud-auth/OAuth-store,
TLS/crypto backend selection, WebSocket/compression, complete capability V,
native macOS, budget, or #94 real Deferred gaps. The old #128 matrix and
receipts remain immutable; the updated capability matrix and native/package
receipts are new evidence.

Evidence:

- [updated capability V matrix](../evidence/phase8-capability-v-20261115-v3.json)
- [combined native/package/caller receipt](../evidence/phase8-http-transport-20261115.json)
- [current external package receipt](../evidence/phase8-http-transport-package-20261115-v3.json)
- [superseded package receipt](../evidence/phase8-http-transport-package-20261115-v2.json)

Test impact: `add` the `http-transport` feature, transport-only contract
target, external package runner and caller inventory; `update` feature
profiles and the transport ownership mapping; `delete=none`; `retain` all
historical builtins/default, Pi pairing, adapter/observer, platform and
catalog receipts.
