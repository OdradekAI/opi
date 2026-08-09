# Finding Template and Severity Guide

## Severity definitions

Canonical four-tier definitions and auditor-scale unification live in
`../../_shared/references/finding-contract.md`. The per-tier guidance below is
audit-specific.

### Blocker

The implementation cannot ship safely. Examples:
- Data loss on a normal (non-error) code path
- Security vulnerability exposing credentials or user data
- Crash or panic on expected inputs
- Infinite loop or deadlock on common paths

Blocker findings should be rare. If you find more than 2-3 in a well-tested
phase, reconsider whether some are actually Major.

### Major

Incorrect behavior or significant gap that needs fixing before the next phase.
Examples:
- Function produces wrong output for valid inputs
- Edge case causes silent data corruption
- Significant spec deviation without documented justification
- Missing error handling that could cause cascading failures
- Algorithm divergence between components that should agree
- Test gap leaving a critical path unverified

### Minor

Code quality or completeness gap that does not cause incorrect behavior in
practice. Examples:
- Missing test for a non-critical edge case
- Documentation out of sync with code
- Redundant code or duplicated logic
- Inconsistent naming or style within the phase's changes
- Localized doc (.zh.md) not updated alongside English counterpart

### Info

Improvement opportunity or future consideration. Not a defect. Examples:
- Potential performance improvement
- API ergonomics suggestion
- Pattern that might become a problem at scale
- Observation about design trade-offs
- Carry-forward item for next phase

## Finding format

Each finding should include enough context that a developer can locate and
understand the issue without re-reading the full source. Narrative fields may be
adapted for clarity, but every actionable finding also includes the normalized
block from `../../_shared/references/finding-contract.md`.

### Recommended fields

```markdown
### <section>.<number> <SEVERITY>: <Short descriptive title>

**File:** `<relative/path/to/file.rs>`
**Lines:** <start>--<end>
**Cause:** <What is wrong and why. Be specific about the mechanism.>
**Impact:** <What happens if this is not fixed. Who or what is affected.>
**Fix:** <Concrete suggested fix. Reference specific functions or patterns.>
```

### Optional fields

- **Spec ref:** when the finding relates to a specific spec section or criterion
- **Test gap:** when the finding includes a missing test observation
- **Related:** when multiple findings are connected

### Normalized block

Append a YAML block with the exact fields from the shared finding contract.
For audit output, `source_kind` is `audit`; `axis` preserves `standards` and
`spec` separately or names the applicable opi audit dimension. Set
`status: unverified` even when the auditor is confident: `opi-remediate` owns
independent verification.

## Complete example

This example is drawn from a real Phase 12 audit finding:

```markdown
### 2.1 MAJOR: Bedrock stream does not flush pending Done on metadata absence

**File:** `crates/opi-ai/src/bedrock/mod.rs`
**Lines:** 364--370 vs 164--167
**Cause:** The `stream_from_fixture` path (L164--167) calls
`mapper.flush_pending()` after the stream loop to emit any deferred `Done`
event. The production `stream_http` path (L364--370) only checks
`!mapper.saw_done` and emits a `StreamError` if the stream ended without a
terminal event, but does NOT call `flush_pending()`. When the Bedrock stream
delivers `messageStop` (setting `saw_done = true` and storing a
`pending_done`) but the subsequent `metadata` event never arrives,
`pending_done` is never flushed.
**Impact:** Callers waiting for `Done` (the agent loop's
`process_stream_event` returning `Some(msg)`) may never see the complete
assistant message. The stream terminates silently without error or completion
signal.
**Fix:** Add `mapper.flush_pending(&tx).await;` after the stream loop in
`stream_http`, before the `!mapper.saw_done` check.
```

## Minor / Info example

```markdown
### 6.3 Minor: session_context test does not verify empty-session edge case

**File:** `crates/opi-agent/tests/session_context.rs`
**Lines:** (not present)
**Cause:** No test calls `reconstruct_context` with an empty entry list.
The function handles this case (returns empty vec), but the behavior is
unverified.
**Impact:** Low -- the code path is trivial and unlikely to regress. But
adding a one-line test would close the coverage gap.
**Fix:** Add a test case passing an empty `Vec<SessionEntry>` and asserting
the result is empty.
```

## Invariant verification matrix format

When the spec defines explicit invariants (guarantees that must hold across all
code paths), verify each with a two-column assessment:

```markdown
| Invariant | Code evidence | Test coverage |
|-----------|--------------|---------------|
| Metadata entries do not advance content tip | All `append_*` methods in session_coordinator.rs pass `advances_tip: false` | session_facade::test_metadata_does_not_advance_tip |
| Context reconstruction is deterministic | reconstruct_context uses stable sort by entry index | phase13_context_builder_deterministic |
| Export does not modify source session | export_session reads via SessionReader, writes to separate path | session_export::test_export_preserves_source |
```

Invariants without test coverage should be flagged as Minor or Major depending
on the invariant's criticality.
