# #134 Split the cloud-auth owner

Status: completed for the protocol-independent cloud authentication owner;
per-provider protocol/source and platform credential boundaries remain open.

## Decision

`opi-ai` now exposes `cloud-auth` independently. It owns the AWS local source
chain and STS/SSO exchanges, GCP ADC families, shared AWS SigV4 signing, and
the pinned query/XML/DPoP codec closure. It transitively selects
`http-transport` and its fixed TLS backend. It does not select provider wire
adapters, the bundled catalog, provider collection or WebSocket compression.

`protocol-adapters` transitively selects `cloud-auth`, so Bedrock and Vertex
callers retain their existing feature path and wire behavior. The historical
`bedrock::sigv4` path re-exports the shared implementation; cloud-only callers
can use `auth::aws::sigv4` without compiling the Bedrock adapter.

## Evidence

- Windows native `cloud-auth` check passes, the standalone public profile passes,
  59 auth module tests pass, and warnings-denied Clippy passes.
- The actual archive is consumed outside the workspace with only
  `cloud-auth`; metadata contains HTTP/TLS plus HMAC, Smithy query/XML,
  xmlparser, P-256 and SHA-1, while provider protocol/compression/catalog
  owners are absent. The external caller executes endpoint, SigV4 and ADC
  constants.
- The caller inventory scans 667 Rust files and records AWS auth, ADC, cloud
  provider-wire, process-source and direct-codec usage with source hashes.
- Protocol adapter native profile, Clippy, builtins HTTP compatibility and
  builtins HTTP acceptance remain green after `protocol-adapters` began
  selecting `cloud-auth`; the current external protocol archive records the
  exact five-feature union.

Evidence: [combined cloud-auth receipt](../evidence/phase8-cloud-auth-20261116.json),
[cloud-auth package receipt](../evidence/phase8-cloud-auth-package-20261116-v3.json),
[caller inventory](../evidence/phase8-cloud-auth-callers-20261116-v1.json),
[current protocol package receipt](../evidence/phase8-protocol-adapters-package-20261116-v11.json).

## Deliberate scope and remaining work

This slice separates source resolution and signing ownership without changing
provider wire behavior or adding live cloud calls. Individual provider protocol
features, finer credential-source owners, complete capability V, native macOS
and #94 remain open.

Test impact: add the `cloud-auth` profile, native contract test, package
runner/unit tests and caller inventory; update the protocol feature union,
bilingual README/provider-auth/testing docs, capability mapping, changelog and
spec; delete none; retain all prior Pi, adapter/observer, provider, HTTP,
protocol, OAuth/store, TLS, WebSocket and failed-attempt receipts.
