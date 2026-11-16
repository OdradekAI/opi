# #125 Freeze the current feature matrix inventory

Status: inventory verified; complete configuration set V remains open.

The adapter and observer slices through #124 now have paired traces and actual
package consumers. The next acceptance gap is configuration scope. The current
`opi-ai` manifest declares only two independent switches: `builtins` and
`telemetry-tracing`, with both enabled by default. That yields four declared
profiles on the current workspace target.

## Result

The four metadata invocations all exit successfully and each profile maps to an
existing external archive consumer receipt:

| Profile | Requested features | External receipt |
| --- | --- | --- |
| `core` | none | `phase5-minimal-package-verified.json` / `core` |
| `core-tracing` | `telemetry-tracing` | `phase5-minimal-package-verified.json` / `core-tracing` |
| `builtins-no-tracing` | `builtins` | `phase5-minimal-readme-package.json` / `builtins-no-tracing` |
| `defaults-on` | `builtins`, `telemetry-tracing` | `phase5-minimal-readme-package.json` / `defaults-on` |

The raw command hashes, manifest identity and receipt mapping are recorded in
[the feature matrix inventory](../evidence/phase8-feature-matrix-inventory.json).
The package receipts remain the authority for external consumption; this issue
does not turn `cargo metadata` into a build or runtime proof.

## Disposition

This freezes the current declared matrix as a candidate V inventory, not as the
complete milestone V. `builtins` is still a coarse capability group, so the
remaining configuration work must decide which protocol, cloud-auth, catalog,
TLS/WS, OAuth/Store and telemetry combinations are independently supported and
which higher-order combinations are valid. MSRV 1.97, native Linux/macOS rows,
toolchain identities, all-features validity and quantitative budgets still need
separate evidence. Real Deferred integration remains blocked by #94.

No production source changed in this inventory slice. All previous Pi 1.5.0
pairing, adapter/observer, package and historical failure evidence is retained.

Test impact: `add` one manifest-level inventory and receipt mapping; `update`
none; `delete` none; `retain` all prior runtime and package evidence.
