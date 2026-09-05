# opi-protocol

[![Crates.io](https://img.shields.io/crates/v/opi-protocol.svg)](https://crates.io/crates/opi-protocol)
[![Docs.rs](https://docs.rs/opi-protocol/badge.svg)](https://docs.rs/opi-protocol)

> Product-neutral command-execution protocol types, bounded codecs, schemas,
> and fixtures.

[Simplified Chinese](README.zh.md) | [opi workspace](../../README.md)

`opi-protocol` is a reusable Rust library for hosts and execution backends that
need the versioned `command-execution-jsonl-v1` wire contract. It contains no
process launcher, sandbox, package manager, routing policy, or permission
system.

```sh
cargo add opi-protocol
```

Requires Rust 1.97+ (workspace MSRV; edition 2024).

## Status

Current crate version: `0.8.2`, inherited from the workspace package version.

The Cargo crate version and wire version are separate. The current and only
wire identity is `command-execution-jsonl-v1`, defined by
[`execution::v1`][v1] and exposed by `execution::v1::WIRE_IDENTITY`.

[v1]: https://odradek.ai/opi-protocol/ex/v1/

## Package Boundary

This crate owns only protocol data and validation:

- closed host-to-backend and backend-to-host frame types;
- native-string and byte-payload representations;
- bounded JSONL encoding and decoding;
- per-execution request-id, duplicate-frame, and cumulative-output checks;
- deterministic JSON Schema generation and reviewed schemas/fixtures.

It does not depend on `opi-agent` or `opi-coding-agent`. Process launch,
deadline enforcement, process-tree termination, cleanup, redaction, live
handshake ordering, routing, permissions, and sandbox guarantees belong to the
host and backend. [`opi-sandbox`](../opi-sandbox/README.md) is one independent
consumer of this protocol; `opi-protocol` does not depend on it.

## `command-execution-jsonl-v1`

The host starts a one-execution backend over stdio. Host-to-backend frames use
stdin, backend-to-host frames use stdout, and backend stderr is out-of-band
crash evidence rather than a protocol channel.

```text
host starts backend
  -> initialize
  <- ready
  -> execute
  <- accepted
  <- started
  <- stdout | stderr | diagnostic   (zero or more)
  <- completed | failed
  -> host closes stdin
  -> backend exits
```

Every frame carries the same non-empty, host-generated `RequestId`. Program,
arguments, working directory, and environment values use `NativeString` so
native non-UTF-8 values can round-trip. Command stdout/stderr chunks use
base64-backed `Base64Bytes`; these representations are intentionally distinct.

`initialize` carries an ordered protocol preference list. `select` chooses the
first identity supported by both peers; it does not choose a numeric maximum.
The command is sent only after the runtime validates `ready`.

## Core API

| Item | Purpose |
|------|---------|
| `HostToBackend` / `BackendToHost` | Closed frame enums for the two wire directions. |
| `RequestId`, `ProtocolId`, `ImplementationId` | Validated non-empty wire identities. |
| `select` | First-match protocol negotiation using host preference order. |
| `NativeString` / `Base64Bytes` | Lossless native command values and binary output payloads. |
| `Bounds` / `LineReader` / `encode_line` | Per-frame size limits and bounded JSONL codecs. |
| `Session` | Stateful request-id, duplicate-frame, and cumulative-output validation for one execution. |
| `schema` / `schema_with_bounds` | Deterministic JSON Schema generation. |
| `FailureCode` / `FailurePhase` | Closed wire-level failure taxonomy; product-level policy failures stay outside this crate. |

The codec enforces line, configuration, diagnostic, and decoded output-chunk
limits. `Session` adds the cumulative decoded stdout-plus-stderr limit. Frame
rate/count limits, process deadlines, and full state-machine transition order
remain runtime responsibilities.

## Compatibility

The `execution::v1` frame and field set is frozen at first release. Adding,
removing, or renaming a `v1` frame or field is a breaking wire change. Protocol
evolution uses a new identity, such as `command-execution-jsonl-v2`, in a
sibling module so versions can coexist.

Unknown frame tags and unknown fields in known frames are protocol violations.
The reviewed JSON Schema snapshot under `tests/snapshots/` must not be updated
through `INSTA_UPDATE` without human review. Language-neutral valid and invalid
fixtures live under `tests/fixtures/`.

## License

MIT. See the workspace [LICENSE](../../LICENSE).
