# #109 Repair baseline checks and refresh candidate package proof

Status: complete; current workspace checks and actual package consumption pass.

The user requested continuation after [#108](108-repair-chat-terminal-boundary.md):
repair its three independently diagnosed baseline failures, then refresh actual
package consumption for the resulting candidate. Work is worktree-only at
`ee18dd1323a089d536bdc7e293d0b4869777c344`; no publication or legacy Phase
operation is requested.

## Repairs and acceptance

- The unsafe inventory intentionally includes comment tokens. Preserve its
  frozen list and scanner; describe Codex's protocol condition as "unexpected
  idle input". This is a comment-only source change, with no runtime change.
- Compact NDJSON must omit cumulative payloads while retaining fixed message
  metadata, including response accounting. Replace the obsolete 1,000-byte
  threshold with equal event sizes across every delta and N/2N input sizes,
  explicit empty payload/signature checks and full usage equality. A nonzero
  cost fixture retains a provider total different from the catalog components.
  Complete terminal content, tool continuation, durable reconstruction and
  linear total output growth remain checked. The Agent loop does not forward
  tool-call deltas; its existing direct compact-serializer test remains their
  owner.
- Remove `spec_ledger.rs`, which requires the retired global `docs/opi-spec.md`.
  Commit `b161ba6` deliberately removed that source. The current documentation
  map and [legacy boundary](../../../../.agents/skills/_shared/references/legacy-phase-workflows.md)
  preserve old ledger hashes and limit their applicability to explicitly
  selected legacy work. Update the obsolete guard reference in the ledger
  schema. The root ledger and phase snapshots remain unchanged. The current
  `opi-doc-check.py` suite already checks operation without retired sources.

Acceptance requires the complete workspace tests and Clippy to pass, current
documentation checks, and real `.crate` consumption outside the workspace.
README and Schema proofs retain their existing feature profiles and assertions.
Because README uses Anthropic, supplement them with the archive's unchanged
Chat timing test and fixture (all five tests / 75 scenarios) in an independent
consumer, defaults off and on. A separate consumer test exercises the public
sentinel parser and mapper `finish(true)` migration, including one-shot terminal
delivery. This uses the packaged SDK and same-run companion archive only.

## Actual package consumption

- [README proof](../evidence/phase5-baseline-repair-readme-package.json):
  default features off and implicitly on, each executing the three exact
  packaged onboarding programs and the controlled-peer regression.
- [Schema proof](../evidence/phase5-baseline-repair-schema-package.json):
  defaults off and the caller feature union, each passing all 73 checks.
- [Chat proof](../evidence/phase5-baseline-repair-chat-package.json): defaults
  off and implicitly on, each passing the archive's five tests / 75 scenarios
  and one external public-mapper test. The latter observes the sentinel,
  verifies no early end after finish/usage, consumes `TextEnd` followed by
  `Done`, retains final usage and checks that a repeated finish is empty.
  The copied SDK test/fixture bytes are compared against the archive. Cargo
  metadata admits only the consumer and the exact SDK/telemetry artifacts as
  local packages; no Agent, coding-agent or workspace-source dependency enters
  the graph. All provider traffic is controlled loopback traffic.
- [Package/source bindings](../evidence/phase5-baseline-repair-package-bindings.json):
  both independently produced archives are identical, and all 311 original
  SDK payload files plus 17 telemetry payload files match the workspace.
  Generated manifests, lockfiles and VCS metadata are separately bound.
  `opi-ai` SHA-256 is
  `39adc6b7a1bc4f5d62ed1e9c79f656f9dfc71e2bab74ce0c9c1488d7fefb9455`;
  `opi-telemetry` remains
  `54ffe66285c677661c4f64991b53fa580f69fe8f0a9e370588bc6af285d2fd7a`.

The new Chat consumer is retained with its exact runner and per-profile locks
under `C:/Temp/opi-baseline-packages-20261114/chat-package-consumer-verified/`
and `run_chat_package_verified.py` in the parent directory. Its first attempt
passed the defaults-off behavior and mapper tests, then correctly rejected
defaults-on metadata under an incompatible lock. The final attempt generates
each profile's lock offline before all locked metadata/build/test commands.
The first runner, consumer, failure receipt and logs remain unchanged.

The targeted baseline tests pass (`platform_delivery`: nine; `json_mode`: 36;
documentation-checker tests: 32). Workspace Clippy with `-D warnings`, all 11
doctests and rustdoc with `-D warnings` pass. The full workspace test run
completed with **6,485 top-level tests passed, zero failed, five ignored**,
plus four passing isolated subprocess invocations. The command used no
top-level test filter. Formatting, documentation and whitespace
checks are recorded in the [check receipt](../evidence/phase5-baseline-repair-checks.json)
and its final post-write record. Compilation initially exhausted physical
memory through excessive concurrent Cargo jobs (99% usage); only the verified
task-owned workspace Cargo process tree was stopped. Clippy had already
completed successfully. The test command was restarted with four jobs, using
the same persistent cache; its interrupted attempt remains in the raw logs.
The two earlier JSON assertion-development failures are retained as well: the
extra tool-call-delta assertion was removed after confirming that the Agent
loop does not forward those events and its direct serializer seam already
owns their coverage. No production workaround or test filter was added.

## Evidence retention and scope

The task-start files are preserved under
`C:/Temp/opi-baseline-packages-20261114/before/`. Previous failed attempts and
all #106–108 receipts remain historical observations of their original bytes.
The sole SDK source change is the Codex comment; prior Chat behavioral proof
is reusable on that basis, while exact source hashes and package archives must
be renewed. The previous source version remains archived rather than rewriting
an old material seal.

This slice does not close the six-gate milestone, other-platform/MSRV proof,
the minimal dependency configuration, or real Deferred integration (#94).
No user-visible runtime behavior changes, so no new changelog entry is needed.
The ledger reference has one canonical English owner; no bilingual mirror
exists for this reference or issue.

Test impact: `update` the two JSON-mode cases sharing the stream fixture;
`delete` the obsolete legacy-spec guard; `retain` the unsafe inventory,
existing serializer regressions and historical receipts; `add` the external
package mapper control. Package Chat scenarios reuse exact archived test bytes.

Worktree handoff: HEAD is unchanged; `committed=none-for-this-task`,
`staged=none`. The SDK comment is carried-in and touched; JSON tests and the
ledger reference were initially clean, and the retired test is deleted.
Existing iteration documents are updated to point here. Other carried-in
work, legacy ledgers, snapshots and prior evidence bytes remain preserved.
