# #128 Split the bundled catalog capability

Status: complete for the catalog slice; the remaining capability matrix is
still open.

The current `builtins` group combined provider adapters, authentication,
transport, compression and the embedded Pi model catalog. This slice gives the
embedded artifact its own `catalog-bundled` feature while keeping
`builtins = ["catalog-bundled", ...]`. Existing `features = ["builtins"]`
callers therefore retain the same registration surface and default behavior.
`Catalog::from_bytes`, custom sources and store contracts remain core APIs;
`Catalog::builtin()` and `Catalog::builtin_bundle_bytes()` now belong to the
independently selectable bundled-data capability.

The pure artifact test target `catalog_artifact` now requires only
`catalog-bundled`. The lifecycle target remains on `builtins` because it also
exercises HTTP catalog and OpenAI provider paths. A native Windows run of the
catalog-only profile passed all 11 artifact tests, the builtins compatibility
profile passed all 9 lifecycle tests, and catalog-only Clippy passed with
warnings denied.

The actual `opi-ai-0.8.3.crate` was consumed by a fresh workspace-external
caller with `default-features = false, features = ["catalog-bundled"]`. Its
metadata contained exactly the catalog feature on `opi-ai` and no reqwest,
TLS, cloud, crypto or compression backend; the caller built and executed
`Catalog::builtin()` from the archive and observed the pinned 1608 chat-model
count. The package receipt and the combined native/package receipt retain the
archive, source, command and log hashes.

This does not claim that protocol, auth, TLS, WebSocket, OAuth/store or other
`builtins` capabilities are independently selectable. Those rows remain open
in the capability-granular V work and must be split and revalidated one owner
at a time. macOS remains blocked by the previously recorded lack of an
environment, and #94 real Deferred integration remains blocked.

Evidence:

- [updated capability V matrix](../evidence/phase8-capability-v-20261115-v2.json)
- [combined native/package receipt](../evidence/phase8-catalog-bundled-20261115.json)
- [external package receipt](../evidence/phase8-catalog-bundled-package-20261115.json)

Test impact: `add` the `catalog-bundled` feature, package consumer runner and
artifact-only profile; `update` the catalog artifact target and bilingual
feature guidance; `retain` prior builtins/default receipts and all historical
Pi/package evidence; `delete=none`.
