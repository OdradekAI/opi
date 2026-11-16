# #139 Freeze the current worktree and audit static contract rows

Status: current worktree frozen; static-row audit complete. The historical
contract index is intentionally not rewritten in this slice.

## Freeze

The current candidate was captured from the shared worktree before this issue
and its receipt were added. The receipt records exact current-byte SHA-256
hashes for every tracked modified/deleted and untracked path, the `HEAD` and
`origin/main` identities, status codes, file classes, and whether a tracked
change is line-ending-only after CRLF/CR normalization.

- Receipt: [current worktree freeze](../evidence/phase8-worktree-freeze-20261116-v1.json)
- Manifest preimage SHA-256: `9f773dc1268633ab7c225507ff052672feadaae7bb274fe4cd993aafcffff9c7`
- Receipt file SHA-256: `dba9424192863d362b8ebf1447b77e0fc3148eab4d4f6d714ca758df27413eac`
- Captured status: 118 tracked modifications, 2 tracked deletions, 616 untracked files; no staged entries.
- Current classification: 59 Rust source files, 123 test/example files, 345 evidence files, 143 iteration documents, 45 tooling files, 4 manifests and 16 other files.
- Tracked changes are semantic or non-EOL (`eol_only=0`); the strict diff still has the repository's CRLF noise and is checked separately with `core.whitespace=cr-at-eol`.

The final rerun excludes only the receipt and this issue path because those
files contain the resulting hashes. Both paths are otherwise bound by the
receipt's explicit exclusion list and the exact hashes recorded above.

## `static-not-run` audit

The audit covers the four contract-index sections that use a `static-*` status:

| Section | Rows | Current interpretation |
| --- | ---: | --- |
| `consumers` | 161 | 52 production callers, 4 package consumers, 3 examples and 102 test callers. Production/package/example rows are milestone-relevant caller evidence; the status itself is only a mapping. |
| `test_files` | 266 | 183 frozen Pi TypeScript inventory rows and 83 Rust integration targets/helpers. Pi rows are source inventory; only frozen, applicable differential cases need dynamic execution. Rust rows require selected profile execution; `cargo check --all-targets` proves compilation only. |
| `reviewed_cases` | 38 | Static observations retained as design evidence. Promote only in-scope lifecycle, telemetry, resource or behavior cases to dynamic receipts. |
| `models_methods` | 27 | Candidate public-method mappings, not equivalence proof. Complete public parity remains open until caller and contract evidence is bound. |

All recorded paths still exist. Forty-three recorded file hashes drift from
the current worktree: 9 consumer rows and 34 Rust integration-target rows.
Those old hashes remain in the freeze receipt; `contract-index.json` is not
rewritten here because it is the historical phase-one mapping and its old
verifier must remain reproducible. The current-byte hashes are authoritative
in the new freeze receipt until a dedicated current-index refresh is approved.

The binding scan found path/name mentions in existing evidence and issues for
all 161 consumer rows, all 83 Rust target/helper rows, all 27 method names and
26 of 38 reviewed cases. The remaining 183 Pi TypeScript inventory rows have
no current evidence mention, and 12 reviewed cases remain unmentioned: the
four event-stream cases, four telemetry cases, two models-runtime cases and
two Faux-provider cases. A mention is only a routing signal, not dynamic proof;
these rows stay candidates for the applicability audit.

## Disposition and next step

This audit does not turn static rows into failures and does not claim that all
266 upstream test files must run. The next bounded slice is to bind the 52
production callers, 4 package consumers, 3 examples and the 83 Rust target/
helper rows to existing current receipts, then refresh only the rows whose
contract scope is actually part of the complete milestone. The remaining Pi
TS inventory and static reviewed cases stay as retained mapping material unless
their applicability audit promotes them.

Verification for this receipt: `python scripts/opi-doc-check.py` passed,
`cargo fmt --all -- --check` passed, the CRLF-tolerant `git diff --check`
passed, the manifest preimage hash recomputed exactly, and all 733 current
file hashes in the receipt matched the worktree. Strict `git diff --check`
still reports 70,824 CRLF-related lines in the carried diff; that noise is
preserved in `C:/Temp/opi-review-strict-diff-check-20261116-v3.log` rather than
normalizing unrelated files.

Test impact: `add` the freeze/audit receipt and issue; `update` none in
production code or historical contract-index; `delete` none; `retain` old
baseline hashes, Pi differential records, package receipts and prior evidence.
