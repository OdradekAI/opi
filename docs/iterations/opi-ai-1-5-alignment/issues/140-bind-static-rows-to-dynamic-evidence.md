# #140 Bind milestone static rows to dynamic evidence

Status: current binding audit complete. The historical contract index and #139
freeze receipt remain unchanged.

## Scope and result

This receipt binds the milestone-relevant rows from the four `static-*` sections
without treating source mentions as execution proof:

- 52 production callers: all have a current-byte hash in the passing
  workspace dynamic receipt; the 20 `opi-agent` callers also appear in the
  shared Chat consumer receipt. This is aggregate source/input binding, not a
  per-caller behavioral claim.
- 4 package consumers: `consumer/src/main.rs` has an exact current hash in a
  passing packaged Schema run. `consumer-crypto`, `consumer-http` and
  `consumer-ws` are bound to passing TLS/HTTP/WebSocket capability-family
  package receipts whose runners generated temporary consumers; their template
  source hashes are retained but are not claimed as directly executed.
- 3 examples: all three exact current hashes are recorded by the passing
  external README package consumer.
- 83 Rust target/helper rows: 73 have current-byte hashes in the final-numbers
  workspace dynamic receipt. Ten fixture/platform rows use newer
  contract-specific dynamic/material receipts because the aggregate receipt
  carries an older historical hash; both old and current hashes are retained.

The machine-readable receipt is
[phase8-contract-bindings-20261116-v1.json](../evidence/phase8-contract-bindings-20261116-v1.json).
Its manifest preimage SHA-256 is `ec6d47f0393cd8d251dae630c804504612add72f285db6e715aa350abde3dd6f` and its file SHA-256 is
`a204b9f0f94c9660f53cfe283f26b21a5e71584cac0c8a0956123789ffcf598a`.

## The 12 previously unmentioned reviewed cases

All 12 are now dispositioned individually in the same receipt. The four
`event-stream` cases, four telemetry cases and two Faux-provider cases have
related Rust/differential/package evidence, but remain `related-dynamic` because
the frozen Pi TypeScript assertions were not run. The two `models-runtime`
cases have only adjacent lifecycle/catalog evidence and remain `related-only`;
there is no exact dynamic receipt for OAuth-refresh persistence across
cancellation or late publication from a superseded non-cooperative provider.
No reviewed case is promoted to exact upstream equivalence.

## Historical integrity

- #139 freeze receipt: `9f773dc1268633ab7c225507ff052672feadaae7bb274fe4cd993aafcffff9c7` manifest,
  `dba9424192863d362b8ebf1447b77e0fc3148eab4d4f6d714ca758df27413eac` file.
- Contract-index hash at #139 freeze: `c863b3632c2286c9f3bb423c5cb4e906dcc9329ddb58d8cb3df98631546a3aa6`.
- Current contract-index hash is recorded separately as
  `c863b3632c2286c9f3bb423c5cb4e906dcc9329ddb58d8cb3df98631546a3aa6`; the historical index is not rewritten by this slice.
- Existing Pi 1.5.0 differential, package-consumer and capability receipts are
  retained and referenced by hash in the new binding receipt.

## Verification and next action

This is a documentation/evidence-only change. The documentation contract check,
Cargo format check and CRLF-tolerant diff check all pass; the binding self-check
recomputed the manifest and verified all 142 row hashes plus 12 reviewed-case
rows and 195 evidence references with zero errors. Strict `git diff --check` still exits 2 on the carried
CRLF noise (70,828 reported lines); the log is retained at
`C:/Temp/opi-contract-bindings-strict-diff-check-20261116-v1.log`. The next
bounded work item is to close the three package-template direct-run gaps and
the two exact models-runtime reviewed cases, or record an explicit
non-applicability decision; do not bulk-run the 183 frozen Pi TypeScript
inventory.

Test impact: `add` the binding receipt and issue; `update` spec history only;
`delete` none; `retain` all old hashes and historical receipts.
