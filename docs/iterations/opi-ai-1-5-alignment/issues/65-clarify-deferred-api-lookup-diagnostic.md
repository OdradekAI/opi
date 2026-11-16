# Clarify the deferred provider and API lookup diagnostic

The diagnostic fix and paired replay are complete. The route mismatch
and unknown job ID differences remain open.
[Parent iteration](../spec.md) ·
[#63 route mismatch](63-compare-deferred-handle-route-mismatch.md) ·
[#64 unknown job ID](64-compare-deferred-unknown-job-id.md)

## Change

The local registry looks up the handle's exact API and requires the
provider to match. With the provider registered for `scripted-deferred`,
#63's request for `other-api` previously said the provider did not support
deferred responses. Fetch and explicit cancel now describe the missing
provider and API registration through the existing sanitized
`ProviderError::Config` cause. A provider mismatch names the same pair
of input fields. Direct validation preserves
`DeferredError::Unsupported { provider }` and describes the missing
registration for the requested API.
[Lookup](../../../../crates/opi-ai/src/models.rs#L1219) ·
[Cause conversion](../../../../crates/opi-ai/src/deferred.rs#L26) ·
[Transport guide](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics)

The route check comes from Opi's AIR-043 implementation at
`4b3600fcdeb3ade41bf4f66249337c382630af23`. Frozen Pi `v1.5.0` at
`f9a26365568127461c47a07a37dcb42c140a3ac5` selects its route from the
explicit model and lets Faux reject the mismatched handle after dispatch.
See #63 for the source paths and the matching Pi `v1.3.0` behavior.

## Replay

For #63 the complete Pi receipts match the baseline. The Rust rows for
provider and API mismatch each change two fields, the fetch failure
message and the poll span's error message. Model mismatch and original
handle refetch match exactly. The invalid routes still reach zero
auth resolutions and zero fetch dispatches on Rust. The original
handle still returns the ready result on both sides.

For #64 the complete case record matches its baseline. A handle with a
valid route and an unknown job ID still reaches each backend once after
one auth resolution. Rust preserves the scripted backend's cause and
diagnostic and the original handle remains fetchable.
[Route comparator](../../../../scripts/pi-sdk-conformance/differential/compare_lifecycle.py#L1074) ·
[Unknown ID comparator](../../../../scripts/pi-sdk-conformance/differential/compare_lifecycle.py#L1159)

## Verification

- [Material manifest](../evidence/phase5-deferred-lookup-diagnostic-materials.json)
  `edcb309d10a4998ebe6ac3354dd0b889e1bfbf3c68a90635dceda0bd992a3590`.
  [Paired replay](../evidence/phase5-deferred-lookup-diagnostic-differential.json)
  `4730cf6e6d9a315ae0934474a8d8791c170a68a9a9318c4a79f0a72c47eb039a`.
- The original-lock Pi snapshot, installed dependencies, toolchains and
  driver inputs match #64. The Rust source tree and comparator bindings
  changed. The #64 manifest returns `STALE_MATERIALS` with exit 2 and
  zero driver or build invocations against the updated tree.
- All 66 Pi and 22 Rust receipts match stdout and SHA-256. All 22 shared
  comparisons pass and 21 case records match #64 exactly. Three cases
  pass outright, ten have previously accepted scoped differences and
  nine deferred differences remain open. The raw verdict is
  `DIFFERENCES_REQUIRE_DECISION` with exit 0, zero material blockers and
  `strict_alignment: false`.
- The new Rust regression reproduced the provider-only message before the
  fix. It verifies the shared fetch and cancel diagnostic, the existing
  error types and zero auth or backend calls. The Python comparator also
  rejects the obsolete API-mismatch row from #63. The differential suite
  passed 71 Python tests. Cached `deferred_contract`, `ai_telemetry` and the
  lifecycle example passed 43 Rust tests. Focused Clippy and crate rustdoc
  with `-D warnings`, Cargo format, documentation and diff checks passed.

Test impact `add` for the fetch and cancel regression and obsolete
diagnostic rejection. `update` for the cancel assertion and route
mismatch replay fixture. `retain` for all other cases and tests.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
`HEAD` remains `ee18dd1323a089d536bdc7e293d0b4869777c344`. The edited
source, tests, guides, changelog and iteration records are carried in
and touched. This issue and its two evidence files are new, bringing the
untracked count from 83 to 86. The tracked modification count is 28.

Next, review the open deferred decisions together, starting with result
timing and ownership of source cleanup. Real-provider behavior and the
six acceptance gates still need evidence.
