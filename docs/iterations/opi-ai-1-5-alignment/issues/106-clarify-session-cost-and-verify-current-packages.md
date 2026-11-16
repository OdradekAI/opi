# #106 Clarify session cost persistence and verify the current packages

Status: done

Package results below retain their original candidate identity.
[#109](109-repair-baseline-checks-and-refresh-packages.md) refreshes package
consumption after the subsequent Chat terminal repair, including Chat consumers.
Owner: Codex for user
Parent: [alignment spec](../spec.md)

The user requested resolution of the session cost persistence contract and
the failing resume regression, followed by actual package consumption for
the current candidate. Work starts from `ee18dd1323a089d536bdc7e293d0b4869777c344`
with the carried-in alignment worktree. No branch, commit, publication or
live provider call is part of this slice.

## Contract and evidence

Assistant messages persist their response-time `Usage.cost` unchanged:
catalog components, the independent total, and `total_from_provider`.
A provider-reported zero remains distinct from the absence of provider
accounting. Reading an older usage object without `cost` supplies
`CostBreakdown::default()` and makes no provider-total claim. This is an
additive message field under the existing session v2 format.

The session's `cost_summary()` is a separate estimate from cumulative token
usage and current session model pricing. It does not sum message costs,
persist the estimate, or rewrite message accounting when prices change.
Unknown usage or pricing continues to suppress the estimate. This slice
does not turn the summary into a historical provider-billing ledger.

The failure was an outdated assertion, not a failure to restore usage:

- Commit `2f85562` introduced the blanket prohibition on `"cost"` anywhere
  in JSONL. Its `crates/opi-ai/src/stream.rs::Usage` had no cost field.
- Commit `be73d1e` introduced serializable `Usage.cost` with a default for
  missing fields. [#25](25-cost-pricing-semantics.md) documents the separate
  catalog components and provider-reported total. Discarding it at session
  write would lose response accounting which tokens cannot reconstruct.
- `message_for_session` in `session_coordinator.rs` already retains the assistant
  message. `cost_for_usage` and the resume accumulator independently use
  tokens and the current pricing configuration. The runtime behavior is
  preserved; only its documentation and regression assertions change.
- Pi v1.5.0 `f9a26365568127461c47a07a37dcb42c140a3ac5`,
  `packages/coding-agent/src/core/session-manager.ts:1215` stores the supplied
  message, and `:1188` serializes the entry. This source observation supports
  retaining message accounting; it is not a new Pi execution comparison or
  a claim that the two products' session summaries are equivalent.

The owning English/Chinese coding-agent manuals and SDK cost references
now describe this boundary. No glossary term, format version or runtime
serialization change is needed. The changelog is unchanged because this
slice documents existing product behavior and repairs its test contract.

## Regression coverage

The original `phase14_usage_subsets_survive_session_resume` was rerun before
editing and failed at its JSONL cost prohibition. The revised test checks
the entire serialized and reconstructed usage, including nonzero catalog
components with an explicitly reported zero total. It still verifies token
subsets, actual harness resume, the session v2 format, and old messages with
missing optional fields; the legacy control now also removes `cost`.

`embedded_model_pricing_updates_on_model_switch_and_resume` now preserves a
provider total of `0.125` that differs from its catalog components. The
resumed session estimate is `18`; changing current pricing makes it `36`.
Reading the file and reconstructing context again retains the original
message usage. These assertions detect loss of provider accounting and
accidental use of persisted message costs as the session estimate.

The complete `session_runtime` target passes 64 tests. Affected-target
Clippy with `-D warnings`, workspace formatting, documentation and
whitespace checks pass. No executable product behavior changed, so these
checks do not claim a new full-workspace or platform acceptance run.

## Current package consumption

The existing README and Schema harnesses build real `opi-ai` and
`opi-telemetry` `.crate` archives, extract them outside the workspace, and
resolve the companion dependency only from that run's archive. They retain
the shared Cargo cache. Historical package reports and archives remain
unchanged; this slice records new evidence at separate paths.

Both package runs pass on Windows x64 with the toolchain and dependency
graphs recorded in the receipts:

- [README package proof](../evidence/phase5-current-candidate-readme-package.json):
  defaults-off and implicit defaults-on each execute the three exact
  onboarding programs and one controlled-peer regression. Setup side effects,
  execution before consumption, terminal results and tool continuation pass.
- [Schema package proof](../evidence/phase5-current-candidate-schema-package.json):
  defaults-off and the explicit caller feature union each pass 73 checks.
- Both independent package commands produced identical archives:
  `opi-ai` SHA-256
  `1385e87ecc491d1590002bb6294fdfc40393f079afba671ed14d68791ad8cd68`;
  `opi-telemetry` SHA-256
  `54ffe66285c677661c4f64991b53fa580f69fe8f0a9e370588bc6af285d2fd7a`.
  All 309 original AI payload files and 17 telemetry payload files match
  the current workspace. Cargo-generated manifests, lockfiles and VCS
  metadata are separately identified rather than compared as source files.
- [Checks and artifact bindings](../evidence/phase5-session-cost-and-package-checks.json)
  retain the red/green logs, current source hashes, each archive comparison,
  package command stdout/stderr hashes, and unchanged historical proof hashes.

Commands (each scratch and evidence output was new):

```text
python -B scripts/pi-sdk-conformance/run_readme_examples.py --offline --scratch C:/Temp/opi-readme-proof-20261114-cost-contract --target-dir E:/opi/cargo-targets/opi-72fa37a0fa044958 --evidence-output docs/iterations/opi-ai-1-5-alignment/evidence/phase5-current-candidate-readme-package.json
python -B scripts/pi-sdk-conformance/run_early_package_proof.py --scratch C:/Temp/opi-schema-proof-20261114-cost-contract --target-dir E:/opi/cargo-targets/opi-72fa37a0fa044958
```

The new proof covers current packaged onboarding and Schema consumption.
It does not claim every public API, Deferred lifecycle, a network-free
core-plus-Faux build, other operating systems, live services or the complete
six-gate milestone. `defaults-off` still only removes the optional tracing
carrier. The historical package receipts retain their original bytes.

Worktree handoff: HEAD is unchanged, `committed=none-for-this-task`,
`staged=none`. The spec and contract index are carried in and touched.
The two cost topics, coding-agent bilingual manual, coordinator rustdoc and
two session test cases are this slice's tracked edits. This issue and three
new evidence files are untracked. Other carried-in alignment work is preserved.

Test impact: `update` for the two existing session tests; `retain` for the
remaining session regressions, SDK contracts and package harnesses; `add`
and `delete`: none. The next protocol observation remains Chat block-end
timing between `finish_reason` and `[DONE]`. The real Deferred integration
gap in [#94](94-record-real-deferred-integration-gap.md), minimal feature
configuration and complete six-gate acceptance remain open.
