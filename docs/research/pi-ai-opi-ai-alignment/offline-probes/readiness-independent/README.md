# Design-readiness independent evidence

Research evidence only. See [the independent review](../../pi-1.3.0-design-readiness-review.md) for scope, findings and remaining gates. All product acceptance groups remain OPEN; these files do not create another acceptance matrix.

`runtime/`, `schema-generator/` and `schema/` contain independent commands, raw outputs or semantic views. Schema raw stdout/stderr bytes are preserved as base64 rows in `schema/raw-streams.jsonl`; each row includes the exact byte count and SHA256. The additional fixed-executable replay in `schema-fixed-replay.json` records every input SHA and output after copying the built executable into TEMP, avoiding concurrent binary replacement in a shared Cargo target.

`ws-*.zlib-base64.json` preserve complete independent WS JSON bytes without repeating multi-megabyte text verbatim. Decode `payload` with base64, then zlib, and check `decoded_bytes` and `sha256` before parsing. No executable, third-party extracted tree or synthetic private key is stored here.

`ws-extra-results.json` is the historical independent malformed-frame counterexample against the earlier six-line PMD patch. Its script and loopback outputs predate the latest restriction; do not rerun it under the current pure-memory authorization. The published source paths are historical provenance, not portable executable discovery.

`ws-memory-final/` is the later independent verification under the explicit pure-memory restriction: no socket, connection, request or download. It records final source hashes, 14 passing tests including a public-codec loop over 12 frozen fixtures, local summarizer positive/negative checks and wrong-pin rejection before workspace creation. The earlier 80 network observations remain tied to the earlier patch. `ws-r02-local-summary.json` additionally verifies local-only path behavior from repo cwd without modifying historical files. Diagnostic Python sources are provenance; stage them in exclusive TEMP if their permitted scope is used again.

`final-document-review.json` binds the 13 reviewed document bodies before the explicitly approved mechanical status labels. It does not retroactively claim review of a later parent-generated evidence file or other unobserved bytes. Product/platform acceptance remains OPEN.

`negative-checks.json` records the actual four runtime negative CLI checks and WS wrong-source-pin rejection before workspace creation. Experiment timeouts, counts and sizes are containment or fixtures, not SDK budgets.

No public SDK, real hydrated catalog, platform delivery or formal admission is proven. The review explicitly distinguishes successful mechanism replays from the additional failures it discovered.
