# Preserve the legacy provider abort terminal

A legacy provider `Error` event with `Aborted` in both stop fields now
delivers one `Err(CallFailure)` with `ProviderError::Cancelled` and the
original assistant message. Application cancellation is read from the
caller's own token or task state.
[Normalizer](../../../../crates/opi-ai/src/stream.rs#L229)

## Source and compatibility

Pi `v1.5.0` at `f9a26365568127461c47a07a37dcb42c140a3ac5` includes
`aborted` in `packages/ai/src/types.ts:839`. Faux uses the same stop value
for the event and message at `packages/ai/src/providers/faux.ts:449–451`.
The result stream returns the error message at
`packages/ai/src/utils/event-stream.ts:104–108`. Pi `v1.3.0` at
`71023aec1546d6d677a65669c5665c0927ce0dd0` already has the same error
event reason type.

Opi's `7b834481f336d47b4f4e5dabb8a35d65c04b4e8d` returned both fields
from `drain_to_completion` in `crates/opi-ai/src/providers/collection.rs:726–727`.
The unification in `b2ff2f398b8d2416f0284ac7a77731338cf0a436` rewrote
legacy terminals to `Error`. The repair recognizes the matching abort
pair. It keeps content, usage, provenance and the diagnostic's presence
or absence. A single `Aborted` field takes the existing generic failure
path, as does the historical `Error`/`Stop` shape from
`test_support::error_response`. The direct stream test checks both
one-sided pairs and a matching pair with and without a diagnostic.
[Direct stream tests](../../../../crates/opi-ai/tests/stream_events.rs#L573)

## Deferred consumers

The #78 frozen Durable probe records `model_error` for a provider
`aborted` response with a live task signal and the diagnostic `503 Service
Unavailable`. The task submits one job and issues zero remote cancels.
An application abort during a held fetch records `aborted` and explicitly
cancels the retained handle once. The source policy is in
`packages/durable/src/harness/generation.ts:758–824`.
[Bound Durable observations](../evidence/phase5-deferred-failure-consumer.json)

The Rust public `fetch_deferred_ready` probe now reads `Aborted` and
`Cancelled` for the same provider diagnostic, with a live caller token.
`classify_whole_call_retry` returns `Aborted`. The separate local
cancellation control observes a cancelled token and released source.
With the provider tail held, readiness delivers the abort while the poll
span is open. Releasing the tail settles the single poll with stop
`aborted` and `SpanStatus::Ok`. The controlled backend checks one fetch
and rejects an implicit remote cancel.
[Failure inputs](../../../../crates/opi-ai/tests/execution_lifecycle.rs#L1384) ·
[Held tail](../../../../crates/opi-ai/tests/execution_lifecycle.rs#L1196) ·
[Local cancellation](../../../../crates/opi-ai/tests/execution_lifecycle.rs#L1290)

## Verification

- Three Rust assertions reproduce the lost stop reason before the repair.
  After the repair, `ai_telemetry` (21), `api_mapped_provider` (17),
  `deferred_contract` (17), `execution_lifecycle` (40), `provider_collection`
  (111), `stream_events` (30) and `test_support_tests` (26) pass. Of the
  nine deferred observations, only the legacy provider abort's stop and
  cause differ from #79.
- Both modified test targets pass all 70 tests with `--no-default-features`.
  The existing `opi-agent` in-band failure test passes and keeps the
  partial with zero retries. Affected-target Clippy, rustdoc with warnings
  denied, Cargo format, the documentation checker and `git diff --check`
  pass.
- The 13 pinned #78 source and lock hashes and its three Durable receipts
  revalidate on the original-lock snapshot. The #79 manifest detects the
  Rust source drift and exits 2 before building.
- The new freeze uses the same original `package-lock.json` and has zero
  material blockers. All 24 shared comparisons and 96 trace receipts
  validate. The 24 case records match #79 exactly.
  [Materials](../evidence/phase5-deferred-legacy-abort-materials.json) ·
  [Replay](../evidence/phase5-deferred-legacy-abort-differential.json)
- The #79 driver copy is at
  `C:/Temp/opi-ai-lifecycle-executables/4fdc0c4ca40952601f79e6f0cf4cf0b15f69de2814f0527cff68936d3eb49cc7/lifecycle_differential.exe`.

Next, compare the existing message-level retry classifier with Durable's
pinned transient and quota rules using canonical messages from `CallFailure`.
The paired replay records ten deferred difference cases. Real-provider wire
behavior and six-gate acceptance remain open.

Test impact `add` for the stop-pair regression and `update` for the public
deferred failure, held-tail and caller token controls. `retain` for the
legacy generic failure controls and paired lifecycle cases. Worktree
handoff `committed=none-for-this-task` and `staged=none`.
The stream normalizer, lifecycle tests, bilingual transport guides,
changelog, iteration brief and index, and replay guide are carried-in and
touched. The error category rustdoc and `stream_events` test are task-owned
unstaged edits. The issue, freeze and replay are new untracked files.
Existing untracked reports have source anchors updated for the shifted lines.
