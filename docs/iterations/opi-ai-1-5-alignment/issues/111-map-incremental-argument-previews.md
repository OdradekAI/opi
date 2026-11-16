# #111 Map independent incremental argument previews

Status: bounded incremental mapping and caller/package verification complete;
representation and performance limits remain explicitly recorded.

Follow-up: [#112](112-retain-complete-preview-at-representation-boundaries.md)
settles the non-finite/depth preview policy, repairs whole-snapshot holding and
assesses actual caller cadence. The observations below remain the #111 record;
its receipts and sealed executables are retained unchanged.

Continuation of [#110](110-verify-chat-argument-previews-and-retain-literals.md).
Worktree candidate HEAD remains `ee18dd1323a089d536bdc7e293d0b4869777c344`.
Frozen Pi 1.5.0 is `C:/Temp/opi-ai-frozen-lock-20261111`, revision
`f9a26365568127461c47a07a37dcb42c140a3ac5`, with its original lock and OpenAI SDK
7.19.0. Comparisons use `packages/ai/src/utils/streaming-json.ts`, its frozen dist,
the public Models Chat route and the existing Durable/Chord component probe.

## Contract and implementation

The new [public owner](../../../../crates/opi-ai/src/streaming_json.rs) is
`streaming_json::ToolCallArgumentsStream`. One owner belongs to one tool within
one call. `append` keeps the exact raw delta and parses only new UTF-16 units;
`text` returns the accumulated raw input. `replace` extends on a prefix match,
returns the suffix (including empty), and otherwise resets state. `snapshot`
returns a caller-owned JSON object independent of earlier/later snapshots and
caller edits. `finish` delegates directly to the existing strict finalizer,
returns raw/normalized final text plus an optional error, and is nonconsuming.
Final message validation retains authority over execution.

An explicit parser stack and flat arena retain incremental strings, containers
and pending scalars. Ordered object entries retain duplicate-key position;
canonical JavaScript array-index keys enumerate numerically before other keys.
Missing values, split escapes and partial numbers follow the frozen preview
rules. A pending replacement skips the superseded child during materialization,
so an overwritten invalid Unicode value cannot block a later valid preview.

The old `message::parse_streaming_json`, raw ToolCall arguments, public raw delta
events, provider adapters and Agent execution are unchanged. The additive owner
is used by the actual Chat consumer probe and documented for opt-in rendering.
Pi Messages and native Mistral have not been silently migrated to a new helper.

## Differences and caller responsibility

- **Approved Unicode timing difference:** the user selected standard Rust JSON
  on 2026-11-14. An unpaired UTF-16 surrogate in a reachable key/value holds the
  previous representable snapshot; a later pair resumes publication. Pi emits
  the lone code unit immediately. Raw text is preserved. Strict final rejection
  remains the existing native JSON contract. See the
  [decision](../decisions/04-public-contract.md#增量参数预览中的-unicode-映射).
- **Recorded finite-number boundary:** Pi can expose `Infinity`/`-Infinity` for
  JSON-number overflow and accept that final; Rust ignores the unrepresentable
  new value and keeps its strict final rejection. A duplicate key may retain
  its previous value; an array omits that item. The probe records non-finite
  types before serialization, plus negative-zero paths, so JSON `null` or zero
  cannot hide these facts. This is outside the approved Unicode exception.
- **Recorded depth boundary:** a snapshot exceeding 128 reachable containers
  retains the prior value, even after closure. The cap is checked before deep
  `Value` construction because its clone/drop is recursive. An isolated child
  process covers 127/128 layers, open/closed 129 and 10,000 layers, repeat reads,
  replacement, duplicate-key recovery and drop. Final parsing keeps its existing
  recursion boundary. This safety boundary is not a strict Pi equivalence claim.
- **Existing derived numeric precision:** the long-scalar control has equal
  public `finish` raw text/validity, while independently re-decoding that text
  yields Pi `1e64` versus serde_json `9.999999999999999e63`. The new public API
  returns no final JSON Value and does not change the finalizer or dependency.
  Record this separately rather than claiming decoded-value equality.

## Cost and ownership

Open-path copy credit matches frozen Pi: every append earns 256 plus eight
units per UTF-16 code unit; refresh spends open container members/items plus
one per container and one pending scalar. Empty append adds credit; identical
replace and repeat snapshot do not. The 1,200-item array records every published
length and the first credit-lag control. Superseded unreachable nodes do not
inflate snapshot traversal.

Owned JSON materialization copies the visible value. The 25,000/250,000-character
controls separate append-only parsing from requesting a snapshot every 32 bytes,
retain an early snapshot and count returned string characters. These counts
are output volume, not cumulative allocator measurements. Host debug timings
are observations, not a frozen performance budget or an equal-work Pi benchmark;
Pi structurally shares data. Rendering every tiny delta can cost quadratic total
copying, so the caller controls display cadence independently.

## Verification

The [paired receipt](../evidence/phase5-incremental-preview-differential.json)
binds the [frozen materials](../evidence/phase5-incremental-preview-materials.json),
Cargo-reported sealed test executables, input fixtures and probes before and
after three executions. Each frozen Pi Chat run consumes its same-run Rust
trace, checked again by path/hash after execution. Frozen source bindings and
Durable/Chord trees remain pinned. Only the SDK production tree changes, by
adding this module and exporting it from `lib.rs`.

- All **18 remaining #110 differences close** through the new owner across the
  original ten Chat inputs / 65 checkpoints. Raw prefixes, requests, final
  arguments and actual Durable `PartialChanges`/Chord commits agree. Legacy
  helper values equal the sealed #110 executable at all 65 points; both its
  old receipt and executable remain intact.
- Twenty public-interface cases / **105 operations** cover append, replace,
  final validation, independent snapshots, interleaved owners, key order and
  Unicode/numeric/depth controls. All 1,200 published array lengths and credit
  controls match. Three traces are stable except recorded debug elapsed times.
  Representation exceptions require their actual input/value condition; a
  fixture flag alone cannot exempt a mismatch.
- **73 top-level Rust tests plus one depth subprocess** pass: owner 10,
  interface/cost 2, Chat preview 1, unchanged whole-prefix utilities 54,
  shared Chat terminal target 5 (75 scenarios), and Agent caller 1 (three
  scenarios). Four SDK doctests pass, including the new public example.
  Affected-target Clippy and rustdoc with warnings denied, formatting, Node
  syntax, documentation and whitespace checks pass. The
  [check receipt](../evidence/phase5-incremental-preview-checks.json) records
  commands, logs, source applicability and worktree inventory.
- The [comparator](../../../../scripts/pi-sdk-conformance/differential/compare_incremental_argument_preview.py)
  requires exactly the original 18 closures and independently checks key order,
  negative zero, retained snapshots, raw suffixes and copy credit. Twelve tests
  include 38 mutation subcases, Unicode reset/isolation, duplicate-key Unicode,
  malformed trace values and CLI input drift/overwrite controls. A standalone
  CLI positive/negative record retains input/output hashes.
- The [README package proof](../evidence/phase5-incremental-preview-readme-package.json)
  and [independent consumer](../evidence/phase5-incremental-preview-package-consumer.json)
  both pass defaults off/on. The consumer copies the archive's three exact test
  targets and two fixtures, runs all 13 tests per profile plus the depth child,
  and rejects workspace-source or product crate dependencies.
  [Archive bindings](../evidence/phase5-incremental-preview-package-bindings.json)
  match all **317 SDK + 17 telemetry** original payloads to current workspace
  bytes. SDK archive SHA-256 is
  `f836805d2034314f07b40790fe0657638824b53c61054eeb1dbf607a9b86b07b`;
  telemetry remains
  `54ffe66285c677661c4f64991b53fa580f69fe8f0a9e370588bc6af285d2fd7a`.
  All **153** carried-in JSON evidence files retain their original bytes.

| String characters | Chunk characters | Snapshot calls | Returned string characters |
| --- | --- | --- | --- |
| 25,000 | 32 | 782 | 9,796,872 |
| 250,000 | 32 | 7,813 | 976,812,496 |

Append-only controls make no intermediate snapshot calls. Output counts agree
with Pi; full owned copying and structurally shared values are different work.
Elapsed samples are retained in each replay and actual consumer logs; they are
not a performance threshold or allocation measurement.

New raw logs, source backups, sealed executables and one-shot runners are under
`C:/Temp/opi-incremental-preview-20261114/`; package suppliers and consumers are
under `C:/Temp/opi-incremental-preview-package-20261114/`. Initial numeric
representation assertion failure and rejected unsupported cache-wrapper fmt/doc
commands are retained; later successful focused tests and the explicitly leased
rustdoc command supersede them. Old #110 materials and conclusions are preserved.

## Next slice and handoff

The original 18-item display gap is closed for the new opt-in mapping. Next,
settle the recorded non-finite-number and snapshot-depth boundaries, and
calibrate owned-materialization cost against an actual display cadence before
requesting a performance budget. The existing final JSON large-number decoding
gap needs its own contract/caller assessment; this change does not hide it or
broaden final validation.

No full-workspace rerun, other-platform/MSRV, minimal-network-free feature,
real Deferred (#94), performance budget or six-gate conclusion follows from
this slice. The #109 full-workspace run remains historical baseline evidence.

Test impact: `add` independent owner, ordered-key, Unicode, overflow, depth,
copy-credit, interface and comparator controls; `update` actual Chat preview
probe to record incremental and legacy values together; `retain` old helper,
terminal and Agent caller controls and all historical evidence; `delete=none`.
English/Chinese SDK README and utility guides change together; this iteration
issue has no separate language mirror. No commit, staging, push or release.

Worktree handoff: `committed=none-for-this-task`, `staged=none`, HEAD unchanged.
Nine carried-in files are deliberately touched; `src/lib.rs` was initially
clean and gains only the public module export. New implementation/tests,
fixtures, comparator/probe, this issue and evidence are untracked. The checks
receipt preserves exact inventories and verifies all unrelated carried-in files.
