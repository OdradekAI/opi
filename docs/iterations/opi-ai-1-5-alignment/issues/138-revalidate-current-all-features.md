# #138 Revalidate the current all-features candidate

Status: complete for the current Windows manifest-wide compilation row; native
macOS, quantitative budgets and the real Deferred route remain open.

## Scope

The shared-owner pair slice did not change production feature ownership, but it
added new verification tooling and evidence. This issue reruns the current
candidate's manifest-wide `opi-ai` union so the declared all-features row is
bound to the same worktree candidate as #137 rather than relying only on the
older #126 receipt.

## Result

`cargo check -p opi-ai --locked --offline --all-features --all-targets` passes
on the native Windows MSVC host with Cargo 1.99.1 and rustc 1.99.1. The
resolved feature set contains the default/builtins compatibility group,
catalog, cloud-auth, HTTP/TLS, OAuth/store, WebSocket/compression, all eleven
provider owners, shared protocol owners and telemetry tracing. Cargo emits the
existing warning that the shared `provider_protocol_profile.rs` file appears
in multiple integration-test targets; it is a target-layout warning, not a
compile failure.

The run uses the persistent marked target
`E:/opi/cargo-targets/opi-72fa37a0fa044958`; no cache deletion or per-session
target was used. The prior #126 native Linux/Windows and all-features receipt,
the #137 shared-owner pair receipts and all frozen Pi 1.5.0 differential
receipts remain immutable.

## Evidence

- [current all-features receipt](../evidence/phase8-all-features-current-20261116-v1.json)
- [#137 shared-owner pair receipt](../evidence/phase8-shared-owner-pairs-20261116.json)
- [updated capability V matrix](../evidence/phase8-capability-v-20261116-v12.json)
- [previous capability V matrix](../evidence/phase8-capability-v-20261116-v11.json)
- [#126 platform/all-features baseline](../evidence/phase8-native-v.json)

## Remaining scope

Native macOS remains `BLOCKED` because the user confirmed that no macOS host,
SSH entry point or self-hosted runner is available. Real Deferred integration
remains blocked by [#94](94-record-real-deferred-integration-gap.md), and the
quantitative resource/performance budgets are not yet frozen acceptance.

Test impact: add the current all-features receipt; update the capability V
current-validation pointer; delete none; retain prior Pi, provider, adapter,
observer, package-consumer, platform and cache evidence.
