# #136 Verify provider-owner pair and high-order capability V

Status: completed for the selected provider-owner pair/high-order scope; native
macOS remains blocked by the user-confirmed lack of a runner or SSH host.

## Scope

The eleven `provider-*` protocol/source owners are independently selectable,
but single-owner checks do not prove Cargo's additive feature behavior when
multiple provider routes are enabled together. This slice verifies all 55
unordered provider-owner pairs and two high-order unions:

- all provider owners;
- all provider owners plus `catalog-bundled`, `oauth-store` and
  `telemetry-tracing`.

Each row records its transitive feature closure, native `cargo check`, the
corresponding provider profile tests, and an external consumer built from the
actual `.crate`. The package consumer invokes every selected provider public
constructor/parser and, for the support union, the bundled catalog loader.

## Compatibility and history

`protocol-adapters`, default and `builtins` remain compatibility surfaces. The
frozen Pi 1.5.0 paired differential receipts and existing provider-specific
adapter/observer receipts remain immutable; this feature-union validation does
not change protocol source behavior. The user-confirmed native macOS gap stays
`BLOCKED` and is not inferred from cross compilation.

## Evidence

- Native matrix: [phase8-provider-protocol-matrix-native-20261116-v3](../evidence/phase8-provider-protocol-matrix-native-20261116-v3.json)
- Native high-order closure recheck: [phase8-provider-protocol-matrix-native-20261116-v4](../evidence/phase8-provider-protocol-matrix-native-20261116-v4.json)
- Package pair matrix: [phase8-provider-protocol-matrix-package-20261116-v2](../evidence/phase8-provider-protocol-matrix-package-20261116-v2.json)
- Package high-order support rerun: [phase8-provider-protocol-matrix-package-20261116-v3](../evidence/phase8-provider-protocol-matrix-package-20261116-v3.json)
- Combined receipt: [phase8-provider-protocol-matrix-20261116](../evidence/phase8-provider-protocol-matrix-20261116.json)
- Updated capability V matrix: [phase8-capability-v-20261116-v10](../evidence/phase8-capability-v-20261116-v10.json)
- Prior single-owner/native/package/caller receipts: [#135 evidence](135-split-provider-protocol-source-owners.md)
- Frozen Pi/provider paired receipts remain listed in the combined #135 receipt

Test impact: add the matrix runner and unit tests; update capability V,
testing documentation, changelog and spec history; delete none; retain all
prior Pi 1.5.0, adapter/observer, package and cache receipts.
