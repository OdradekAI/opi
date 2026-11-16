# #115 Preserve Anthropic usage updates through callers and observers

Status: complete for this entry-map and Anthropic usage slice; full Pi alignment remains open.

Continuation of #114, authorized by the user. Worktree-only baseline HEAD is
`ee18dd1323a089d536bdc7e293d0b4869777c344`; frozen Pi 1.5.0 is
`f9a26365568127461c47a07a37dcb42c140a3ac5`, with its original installed lock at
`C:/Temp/opi-ai-frozen-lock-20261111`. Initial worktree identities, 244 existing
JSON evidence hashes, failed builds and sealed pre-repair executable/source are
retained under `C:/Temp/opi-entry-contracts-20261114`.

The public-entry/caller audit found a concrete defect above the remaining static
mapping work: Anthropic `message_delta` converts absent and explicit-zero token
fields into the same value, then ignores zeros. It also overwrites stop reason
when omitted and delays catalog cost refresh until successful message_stop.
These facts reach canonical failure partials, ordinary Models observers and
actual Agent/session consumers; they are not only display differences.

Acceptance for this slice:

- Preserve missing/null fields, apply explicit zero, retain an omitted stop
  reason, and update costs before canonical snapshots/failures are published.
- Keep existing invalid cache-subset rejection, provider-reported cost authority,
  raw callback failure semantics and AI observer ownership/isolation.
- Pair frozen Pi success, wire error and raw callback failure across identical
  fixtures; exercise observed and raw SDK entries and actual package consumers.
- Verify actual Agent accounting and session persistence/recovery; record the
  refreshed entry/caller map in the existing contract index.
- Preserve all approved snapshot, Unicode, nonfinite/depth, numeric, cancellation
  and EOF differences and all prior evidence. No live call or publication.

Nested 1h/reasoning wire fields, other adapter mappers, full configuration set V,
MSRV/other native platforms, formal budgets and #94 real Deferred remain open.

## Current entry and caller map

The existing [contract index](../contract-index.json), under IMPL-115, now binds
all **64** current public methods declared on `Models` to a named family, with
current source hashes. The original phase-one 27-method inventory and historical
symbol/test maps remain unchanged. This updates operation and control-plane
entry coverage; it does not turn every public value/helper into a runtime pass
or establish a complete inventory of third-party callers.

| Entry | Actual consumer and observer ownership | Evidence scope |
| --- | --- | --- |
| Ordinary Models Chat | SDK ordinary callers; automatic `pi.ai.request` with context | New Anthropic simple entry; prior native/complete/direct wrapper evidence remains separately scoped |
| Prepared raw attempt | Agent main loop and preparation summary use `start_attempt` then `ChatStream::new(...).start()` | New raw attempt matrix and actual Agent tests; no automatic AI span is intentional |
| Prepared observed attempt | Public SDK helper; no product production caller found | One observer per attempt; caller owns cross-attempt aggregation |
| Direct provider/raw collector | Public low-level provider API; collector may explicitly install telemetry | New Anthropic raw result and callback-failure cases; raw context alone creates no span |
| Deferred fetch/cancel | Public SDK operations | Fetch uses poll observer; cancel forwards context only; #95–102 decisions retained and #94 real integration remains blocked |
| Images/classifier | Builtin SDK registration and public operations; no Coding Agent operation caller found | Context forwarding only under #19; separate fixtures, no new paired non-Chat proof |
| Registration/discovery/auth | Product provider factory, model listing, picker, dispatch validation and logout | Current static production callsites; product OAuth login calls `auth::oauth::login_to_store`, not `Models::login` |
| Catalog mutation/resource cleanup | Public SDK extension entry points | No direct product caller found in inspected production sources; this does not mean unsupported or unused externally |

Anthropic wire aliases share `AnthropicProvider::for_route` through the provider
collection. Coding Agent's native Anthropic, custom Anthropic wire and Copilot
Anthropic construction use that owner. Bedrock and Vertex have separate mappers
and do not inherit this repair or its paired evidence.

Canonical usage reaches Agent `MessageEnd` and state on success and failure.
`CodingHarness::aggregate_turn_usage` sums message buckets; the noninteractive
runner also records failed-run consumption. `SessionCoordinator::on_turn_end`
persists response accounting and `open_existing` reconstructs cumulative usage.
Persisted `Usage.cost` is the response snapshot; a host cost summary computed
from cumulative tokens and current pricing remains a distinct policy.

## Implementation and observations

`AnthropicUsageDelta` carries optional counters instead of discarding presence
in a complete `Usage` value. The mapper merges into a candidate, validates the
cache-write subset, and only then commits usage and any present stop reason.
Start and accepted delta frames refresh model-based costs before recording
producer state. Cost-only OpenRouter updates retain provider-gated authority,
including zero; ordinary Anthropic ignores that billing extension.

The public low-level `MessageDelta.usage` type changes in this 0.x SDK; the one
in-repository manual construction was migrated. Bilingual README migration
notes, the owning cost/pricing guide, rustdoc and Unreleased describe the
contract. Ordinary Models/Provider/Agent call signatures are unchanged.

The paired fixture matrix covers zero, empty, null, mixed and omitted usage,
usage-only after a stop reason, and first-chunk AI observer failure. Each uses
success, wire-error and functional raw-callback-error endings. Rust exercises
ordinary simple, prepared observed, prepared raw and direct provider routes;
Pi exercises public Models simple and direct native routes. A callback barrier
holds the second text chunk until the first event has been read.

Retained Pi partials share the final response, while Rust's first text event
keeps `A`, the original counters and its original cost. This continues the
approved event-time snapshot ownership contract. On first-chunk telemetry
failure, Pi publishes a synthetic failure; Rust disables observation and keeps
the provider outcome, as decided in #48. The Pi probe separately releases its
text barrier and awaits the real source result so its producer is not stranded
by the public observer failure. That test-host drain is not a new SDK cleanup
guarantee. Raw callback errors remain business failures, not telemetry faults.

The sealed pre-repair driver has 84 route cases. Its accounting/outcome checks
expose 76 mismatches; its raw stdout and failed Rust assertion remain intact.
The current driver adds the observer fault control and has 96 Rust cases,
paired to 48 Pi cases. Three repetitions per runtime/version retain raw timing
samples; deterministic comparisons omit only measured duration values, never
their presence or multiplicity. The comparator derives expected accounting
from fixture constants, so synchronized incorrect Pi/Rust values cannot pass.

Actual product tests dispatch through the built-in Anthropic provider and the
real Agent loop. They check successful and failed `MessageEnd` and state, then
write a successful session to disk, read/reconstruct it and reopen it without
rewriting bytes. Failed messages are not artificially saved as committed turns.
The frozen Pi Agent/Durable callsites are source-bound controls; this slice does
not claim a whole Pi Agent or session-manager runtime differential.

## Remaining priority

The nested `cache_creation.ephemeral_1h_input_tokens` and
`output_tokens_details.thinking_tokens` mapping, accounting and caller recovery
are now covered by [#116](116-preserve-anthropic-nested-usage.md). Continue with
the other adapter partial/observer contracts and unresolved fault sites. Static
mappings and one adapter's passing matrix do not close all capabilities or
six-gate acceptance.

## Delivery and verification

The [final paired receipt](../evidence/phase5-anthropic-usage-final-differential.json)
binds the frozen source, installed lock, fixture, final formatted Rust sources,
sealed executables and all three raw repetitions. The pre-format pass and
pre-repair failures remain separate historical observations. The
[stricter regrade](../evidence/phase5-anthropic-usage-regrade.json) checks the same
raw runs for ordered business events, fixture-derived shared/owned snapshot
stages, matching terminal/result, exact text-block attributes, metric counts
and companion tracing features. Independent review verified these inputs and
recomputed all three results. Pi's first gated text event has initial accounting;
later shared snapshots can expose only actual fixture stages. Its intermediate
text_end need not already contain a later wire error. Duration values remain in
raw traces and are excluded only from repeat-stability comparisons.

The [actual package receipt](../evidence/phase5-anthropic-usage-package.json)
executes all three SDK test functions in both default and builtins/no-tracing
consumers. SDK and telemetry come from freshly created `.crate` archives;
ordinary third-party dependencies resolve from the registry. No workspace
product source replaces either archive. The strengthened feature guard is
reapplied to both retained metadata graphs in the regrade, while the original
executed runner/checker sources are separately preserved. The
[package bindings](../evidence/phase5-anthropic-usage-package-bindings.json)
match all **321 SDK and 17 telemetry non-generated files** to current source.
Generated packaging manifests/lock/VCS metadata are identified separately.

The [check receipt](../evidence/phase5-anthropic-usage-checks.json) binds exact
commands, raw logs, source identities, platform scope and preserved history:

- Windows host workspace excluding `opi-eval`: **6517 passed,
  0 failed, 5 ignored**, plus 5 passing isolated
  child invocations. Three harness-free helper targets are recorded separately.
- Workspace Clippy with warnings denied, **12 doctests**, rustdoc with
  warnings denied, and formatting pass. Actual Agent/session success and failure
  regressions pass in the targeted and full workspace runs.
- Python suites pass **114 package/conformance and 150 differential tests**;
  after checker hardening, all **3 package admission and 11 usage-comparator**
  tests pass again, including newly added adversarial controls. These are
  overlapping executions, not summed as unique tests.
- Both new CLIs reject existing output before material loading/building and
  leave the unused scratch path absent. Documentation/link and whitespace
  checks pass.

All **244 pre-existing JSON receipts** remain byte-identical. The six #111–113
archives and five #113 sealed executables recorded by #114 are rehashed and
unchanged. Earlier compiler/assertion/probe/orchestration and Clippy failures
remain in scratch with their original logs. A first event regrade assumed Pi
text_end already held the later error; its corrected fixture-stage rule is
explicitly recorded, without rewriting raw observations or original receipts.

Test impact: `add` Anthropic SDK route/partial/observer and actual Agent/session
regressions, frozen Pi probe, paired runner, actual archive runner and adversarial
checker tests; `update` the existing cancellation test's manual MessageDelta
construction and builtins test-target declaration; `retain` all prior numeric,
preview, terminal, cancellation and EOF tests/contracts; `delete=none`.
Published README and cost/pricing guides were updated in English and Chinese;
this iteration issue has no language mirror.

Worktree-only delivery: no commit, staging, branch change, publication, remote
CI, live-service call or cache deletion. This is a bounded acceptance result
with the existing approved differences, not strict equivalence or full six-gate
acceptance. The remaining priority above is unchanged.
