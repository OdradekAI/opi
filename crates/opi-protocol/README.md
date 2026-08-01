# opi-protocol

Protocol types, bounded codecs, JSON schemas, and fixtures for opi command
execution. The current wire protocol is [`execution::v1`][v1], wire identity
`command-execution-jsonl-v1`.

[v1]: https://odradek.ai/opi-protocol/ex/v1/

## Dependency-neutral

This crate contains only protocol types, bounded codecs, schemas, and fixtures.
It has no dependency on `opi-agent` or `opi-coding-agent` and owns no process
launch, package policy, routing, permission, or sandbox behavior. Runtime
supervision (deadline, kill, cleanup), redaction, and the live handshake are
responsibilities of the execution host and backend, not this crate.

## Scope of `v1`

`execution::v1` exposes:

- a **closed** set of `command-execution-jsonl-v1` frames for both wire
  directions (host-to-backend and backend-to-host), each carrying one
  host-generated request id;
- **lossless native strings** (`NativeString`) for command program/args/cwd/env
  values that may be non-UTF-8 on the host;
- **base64** encoding for command stdout/stderr chunk payloads;
- **bounded JSONL codecs** (a capped line reader + encoder) and a stateful
  [`Session`][session] that enforces cumulative-output, cross-request-id, and
  duplicate-frame invariants for one execution;
- **deterministic JSON Schema generation** for the wire frames, with a reviewed
  snapshot;
- valid and invalid **fixtures** (plain JSON, language-neutral).

[session]: https://odradek.ai/opi-protocol/ex/v1/struct.Session.html

## Compatibility

`execution::v1`'s frame set is frozen at first release. Adding, removing, or
renaming a `v1` frame or field is a breaking change. Evolution is via a new
wire identity (for example `command-execution-jsonl-v2`) in a sibling module.
See the module documentation for the full compatibility and version-negotiation
rules.

The JSON Schema snapshot under `crates/opi-protocol/tests/snapshots/` is a
**reviewed artifact**, not a generated build product: it must not be updated via
`INSTA_UPDATE` without human review.
