# R01 pure-memory verification

Current selected patch identity is in `source-files.json`; `ratchet-core-memory.patch` shows changes against the pinned published Ratchet core 1.2.1 archive. It extends the earlier PMD-only six-line patch to three runtime files: RFC payload-length validation, checked storage arithmetic, and fallible growth based on bytes actually read. `src/ws.rs` adds only two comments needed to compile the upstream unit-test helpers under their existing missing-docs lint. The new unit-test module and its hook are test support, not another runtime parser.

This entry point has **no download path and executes no socket, connection, request, Node, Bun, `run.py`, or `drop.py`**. All 96 locked registry archives must already exist and match their lock checksums. It uses the existing external Cargo target configuration.

```powershell
python docs/research/pi-ai-opi-ai-alignment/offline-probes/readiness-ws/memory-only/verify.py --workspace C:/Users/Luiz/AppData/Local/Temp/opi-ws-memory-review-new
```

The absolute workspace must not exist. Before copying replacements it checks every original file SHA; every replacement also has an after SHA. It runs `cargo test --locked --offline --workspace --lib readiness_memory`, expecting 14 tests: 13 internal boundary tests and one public-codec test iterating 12 frozen memory fixtures. It then runs the directory-relative summarizer on copied historical 80-row JSON and on an intentionally incomplete JSON fixture, expecting success/failure respectively. These steps do not repeat any historical network observation.

```powershell
python docs/research/pi-ai-opi-ai-alignment/offline-probes/readiness-ws/memory-only/verify.py --workspace C:/Users/Luiz/AppData/Local/Temp/opi-ws-memory-review-negative --expected-before wrong
```

The negative command exits nonzero before creating a workspace. Build/test/summary failures stop the entry point and preserve their command/exit/log in TEMP.

`review-r01-historical-results.json` preserves the reviewer’s earlier two panics and nonminimal-length acceptance. Its recorded commands are historical data only. `memory-test.txt` and `command.json` are actual new pure-memory execution evidence. The earlier 80-row network results, old patch pins and old six-line preparation route remain historical; they are not results for this larger patch. Until separately authorized, use only this memory-only entry point.

Storage failure scope is explicit: `BytesMut` 1.11.1 has no public fallible reserve. The patch uses `Vec::try_reserve_exact`, then the public `Bytes::from(Vec)` / `try_into_mut` ownership path for bulk growth; preferred amortized capacity falls back to the exact required capacity. `Bytes::from(Vec)` may allocate small shared metadata, `BytesMut` splitting may allocate metadata, and the PMD/native allocator also has infallible allocations. This proves typed representation/bulk-reservation errors and removes declared-length speculative allocations; it does **not** prove global OOM recovery or immunity from process allocation failure. No fixed SDK byte/message budget is introduced.
