# opi-ws — Agent-neutral owned WebSocket client substrate

`opi-ws` is the repository's Independent Companion for the neutral WebSocket
client protocol state (RDF-24, task 20.15). It owns strict RFC 6455 frame
decoding, message assembly, permessage-deflate (RFC 7692), the explicit
client Close exchange, and the async IO state machine. It deliberately has no
Opi crate dependency, is not a root workspace member, and knows nothing about
HTTP upgrades, credentials, proxies, TLS, providers, agents or the terminal:
the caller supplies an already-upgraded `AsyncRead + AsyncWrite` stream plus
validated negotiation parameters. The product-side strict Upgrade bridge
lives in `opi-ai/src/ws`; the real consumer is the Codex WebSocket provider
(20.16), not this crate.

- Status: companion substrate, `publish = false`; the registry identity and
  publication require their own authorization.
- Language: Rust, edition 2024, MSRV 1.97.
- License: MIT (see `LICENSE` and `NOTICE` for upstream compression
  identities and the supported feature-union set).

## Module map

| Module | Contract |
|---|---|
| `frame` | Strict client frame decoder (reserved opcodes/RSV, fragmentation, control-frame shape, shortest length encodings, 64-bit high bit) and message assembly with actual-received-byte, typed fallible storage growth. |
| `pmd` | Owned permessage-deflate: pre-tail `SyncPoint` proof, exactly-once `00 00 FF FF` tail injection, `Z_BLOCK` boundary/`BFINAL`/`ResetKeep` handling, context takeover, bounded per-poll work, explicit cancellation; the encoder is an ordinary safe `flate2` client. |
| `io` | The exclusive connection owner: owned pending read/write/decode jobs, explicit Close state machine, caller-supplied masks, backpressure, terminal failure absorption, cancellation that drops only the borrow. |

## Interface contract

- One `Driver` owns the whole connection. `next()` returns complete messages
  (`Ping`/`Pong`/`Close` included) and keeps partial frame, PMD and UTF-8
  state across dropped (cancelled) futures — a cancelled borrow resumes the
  same owned operation.
- `Driver::new(stream, Option<PmdParameters>)` receives both directional
  context-takeover flags and both negotiated windows by name. Server
  parameters configure reception; client parameters configure sending.
  `None` disables PMD; absent RFC window parameters map to 15. The pinned
  encoder backend normalizes requested window 8 to 9 internally; the wire
  window is checked independently by the Upgrade test described below.
- Outbound frames are queued with a caller-supplied fresh mask; there is no
  hidden mask, response, timer, reconnect or retry.
- `finish_close` completes the explicit Close: both frames, write shutdown
  and peer transport EOF are required, in-flight application events are
  retained (bounded), and the only deadline is the caller's absolute
  `tokio::time::Instant`.
- Abnormal EOF/protocol/Close timeout failures surface once as typed errors,
  discard retained receive events, then return permanent receive EOF.
  Successful Close completion still permits delivery of its retained event.
- Storage grows from actually received bytes with `try_reserve*` typed
  failures; declared payload lengths are never speculatively allocated, and
  there is no message-size or iteration cap (the internal 8 KiB/quantum
  scheduling values are cooperative bounds, not limits).

## Upstream compression and unsafe boundary

The pinned graph is unmodified `libz-rs-sys = 0.6.8` (default-features off,
`std` + `rust-allocator`), `zlib-rs 0.6.8`, and `flate2 = 1.1.10`
(default-features off, `zlib-rs`). The decoder calls the doc(hidden) public
`inflateSyncPoint` / `inflateResetKeep` exports (zlib-rs `__internal-api`).
The crate's single unsafe boundary is documented inline in `src/pmd.rs`:
a unique boxed `z_stream`, no clone/allocator/callback escape, borrowed
pointers rebound around each call, exactly-once `inflateEnd`, and a
deliberate `Send`-without-`Sync` implementation. See `NOTICE` for the
supported feature unions and the upstream audit obligations on upgrade.

## Feature graphs

| Graph | Encoder backend | Decoder backend |
|---|---|---|
| default (ordinary) | flate2 `zlib-rs` | direct `libz-rs-sys` Rust path |
| `union-default` | flate2 `zlib-rs` with default features unified | direct `libz-rs-sys` Rust path |
| `union-c-zlib` | flate2 `zlib` (native C `libz-sys` build) | direct `libz-rs-sys` Rust path |

flate2 1.1.10 gives C backends priority over `zlib-rs`, then uses miniz_oxide
only when neither is selected. The default union adds miniz_oxide to the
dependency graph while retaining the `zlib-rs` encoder.

## Tests

`cargo test` runs the ported research battery (76 distinct functions across
`frame`, `pmd`, `pmd_async`, `pmd_red`, `replay`, `io`, `close`, `gaps`) plus
the boundary replay in `tests/boundary.rs`, which regenerates the fixed
72,413-case deterministic PMD corpus from `fixtures/boundary-seeds.json` and
compares every case against the recorded classification in
`fixtures/boundary-expectations.json` (outputs longer than 64 bytes by
SHA-256). The fixtures were extracted from the sealed research oracle
(`extract-boundary-fixtures.py` records provenance). Run the same battery
under `--features union-default` and `--features union-c-zlib` to exercise
the caller feature unions.

The root `opi-ai` test
`upgrade_window8_wire_has_independent_decode_and_distance_proof` exercises
the public `opi_ai::ws::upgrade` / `HttpClient` path against a loopback peer
in ordinary/default/C-zlib graphs. It negotiates window 8 in both directions
and audits multiple messages with and without context takeover using an
independent RFC 1951 decoder. The audit reconstructs the original bytes,
checks actual wire distances against the 256-byte window, and includes a
distance-257 negative control that succeeds with window 9 and fails with
window 8.

An idle-read `cfg(test)` test registers a real pending read waker, selects
logical cancellation in the connection owner, then drops the `Driver`.
It observes transport and PMD release before acknowledgement and no late
output. Dropping only a borrowed receive future remains resumable; this
owner-cancellation test adds no production cancellation API.

These are local protocol and ownership checks. Full Codex WebSocket,
continuation, pool/cleanup integration and native platform/MSRV acceptance
remain with their later tasks.

## Maintenance boundaries

Reopen design review instead of silently adapting when: the hidden upstream
API disappears or changes semantics, any unsafe/feature/security change is
requested, the accepted-input set would shrink, or a second production
consumer needs a different interface. Upgrading libz-rs-sys/zlib-rs/flate2
requires recording the resolved source/checksum/features, re-running the
three-graph battery, the boundary corpus, Send/Close/cancellation/fairness
probes and the packaging proof.
