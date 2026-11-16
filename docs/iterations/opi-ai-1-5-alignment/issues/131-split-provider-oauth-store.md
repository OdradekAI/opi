# #131 Split provider OAuth/store owner

Status: completed for the provider OAuth boundary; provider collection and
platform credential-source factories remain builtins-only.

## Decision

`opi-ai` now exposes `oauth-store` as an independently selectable Cargo
feature. It owns the concrete SDK provider OAuth flows under `auth::oauth` and
their store-backed login/refresh entry points. The feature transitively selects
`http-transport`, because the flows use the SDK's source-scoped HTTP client
policy, but it does not select the bundled catalog, provider protocol adapters,
AWS/ADC source chains or provider collection.

Neutral `OAuthProvider`, `OAuthCredential`, `LoginPresenter`,
`CredentialStore`, mutation and `auth::store_oauth` contracts remain in core.
This preserves custom OAuth/store callers in the minimum profile. The bundled
provider collection, `default_oauth_flow`, AWS/ADC/Vertex credential factories
and platform discovery remain behind `builtins`, where they register catalog
routes or credential-source policy.

## Evidence

- `--no-default-features --features oauth-store` builds the SDK, runs the
  provider OAuth/store profile (two tests) and passes warnings-denied Clippy.
- The minimum `core_profile` still passes, proving custom OAuth/store seams do
  not acquire the new provider owner.
- The external package runner consumes the actual `opi-ai` archive with only
  `oauth-store`, verifies the exact `http-transport` union, rejects catalog and
  protocol/cloud-signing owners, and executes an Anthropic OAuth flow plus
  `StoreOAuthResolver` outside the workspace.
- The caller inventory separates concrete flow/registry users, store bridge
  users, neutral core contracts and builtins-only credential factories. It
  records 154 matching Rust files and current source hashes.

Evidence: [current combined native/package receipt](../evidence/phase8-oauth-store-20261115.json),
[current oauth/store package receipt](../evidence/phase8-oauth-store-package-20261115-v2.json),
[historical package receipt](../evidence/phase8-oauth-store-package-20261115-v1.json),
and the [caller inventory](../evidence/phase8-oauth-store-callers-20261115-v1.json).

## Deliberate scope and remaining work

This slice does not split individual provider OAuth flows, TLS backend choice,
WebSocket/compression, AWS/ADC credential-source families or the provider
collection. Those remain the next capability boundaries. macOS native
verification and complete capability V remain open. No provider wire or login
behavior changed; this is a feature ownership and verification boundary.

Test impact: add the `oauth-store` profile, native contract test, package
runner/unit tests and caller inventory; update Cargo feature declarations,
bilingual README/testing docs, capability mapping, changelog and spec; delete
none; retain all prior Pi pairings, adapter/observer, catalog, HTTP and
protocol receipts.
