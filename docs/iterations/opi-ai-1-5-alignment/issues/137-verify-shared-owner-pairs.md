# #137 Verify shared-owner capability V pairs

Status: completed for the two remaining shared-owner pair rows; native macOS
remains blocked by the user-confirmed lack of a runner or SSH host.

## Scope

This slice closes the two pair rows that were still open after the provider
owner matrix:

- cloud-auth + catalog-bundled;
- websocket-compression + oauth-store.

Each row records the transitive feature closure, native check, the two
owner-specific profile test targets, and an external consumer built from the
actual crate. The package graph checks required transport/TLS and owner
packages while rejecting the unrelated cloud or compression owner packages.

## Compatibility and history

The default and builtins compatibility surfaces remain unchanged. Existing
cloud-auth, catalog, WebSocket/compression and OAuth/store caller inventories
remain the source mapping for these additive feature unions. Frozen Pi 1.5.0
paired differential receipts remain immutable; this slice validates feature
composition and public package consumption without changing protocol source
behavior. The user-confirmed native macOS gap stays BLOCKED.

## Evidence

- Native pair receipt: [phase8-shared-owner-pairs-native-20261116-v1](../evidence/phase8-shared-owner-pairs-native-20261116-v1.json)
- Package pair receipt: [phase8-shared-owner-pairs-package-20261116-v1](../evidence/phase8-shared-owner-pairs-package-20261116-v1.json)
- Combined receipt: [phase8-shared-owner-pairs-20261116](../evidence/phase8-shared-owner-pairs-20261116.json)
- Updated capability V matrix: [phase8-capability-v-20261116-v11](../evidence/phase8-capability-v-20261116-v11.json)
- Prior owner caller and package receipts remain linked from the v10 matrix.

Test impact: add the shared-owner pair runner and unit tests; update capability
V, testing documentation, changelog and spec history; delete none; retain all
prior Pi 1.5.0, provider, adapter/observer, package and cache receipts.
