# #162 Move endpoint joining to the canonical API shared namespace

Status: implemented; canonical-only namespace enforced
Owner: current implementation slice
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)

## Goal

Remove the remaining crate-root `endpoint` helper and make URL joining an
explicit API-adapter shared seam:

- `opi_ai::api::shared::endpoint` owns joining an already-resolved base URL to
  an API path;
- provider profiles remain responsible for selecting and validating endpoints;
- API adapters share one neutral URL-joining implementation;
- the old root module is deleted with no compatibility shim.

## Ownership decision

`join_endpoint` only trims a trailing slash from a caller-selected base URL,
ensures the path has a leading slash, and concatenates the two strings. It
does not select a provider, resolve authentication, validate a provider
endpoint, access a catalog, or perform transport. Its production callers are
the OpenAI Chat, OpenAI Responses, Codex Responses and Mistral API adapters.
The root placement therefore obscured a shared API wire seam and placed a
protocol-neutral helper beside crate-wide lifecycle modules.

The canonical owner is the crate-private `api::shared::endpoint` module. The
helper is intentionally not a public consumer API: endpoint selection and
validation stay in provider profiles, while this seam only joins values after
resolution. No `providers::shared` namespace is introduced.

## Acceptance

1. `src/endpoint.rs` is deleted and the implementation lives only at
   `src/api/shared/endpoint.rs`.
2. `api::shared` declares the helper as a crate-private shared module; all
   existing API adapter callers use that canonical path.
3. Base/path joining behavior remains covered by the moved unit tests.
4. Canonical conformance asserts root deletion and absence of old callers.
5. A consumer built from the actual `.crate` archive confirms the canonical
   file is packaged and the protocol-adapters feature still compiles and runs.

## Implementation and verification

The former root implementation was moved to
[`src/api/shared/endpoint.rs`](../../../../crates/opi-ai/src/api/shared/endpoint.rs).
`api::shared` declares it as `pub(crate)` so endpoint selection remains owned
by provider profiles; all five adapter source files now reference the
canonical path and `lib.rs` no longer declares the root module. The old root
file is deleted and no compatibility re-export is retained.

The actual archive consumer and its layout checker passed. The receipt records
the deleted root and present canonical file, the exact protocol feature union,
and nine successful steps from packaging through consumer run. It is
[`issue-162-protocol-adapters-consumer-20261117.json`](../evidence/issue-162-protocol-adapters-consumer-20261117.json).

Focused verification passed:

- endpoint-filtered library tests: 7 passed;
- `external_sdk` and `tool_result_wire`;
- canonical provider conformance: 29 tests passed; protocol-adapters runner:
  5 tests passed;
- Python compilation for the changed conformance scripts;
- builtins and no-default `opi-ai` library checks;
- warnings-denied Clippy for `opi-ai` and `opi-coding-agent`;
- builtins and no-default warnings-denied rustdoc;
- formatting, documentation and whitespace checks.

## Test impact

`add`: canonical namespace/root-deletion assertions and archive-layout
consumer coverage.

`update`: API adapter imports, shared module declaration, protocol runner and
iteration documentation.

`delete`: `src/endpoint.rs` and its root module declaration.

`retain`: URL joining behavior tests and all existing endpoint-selection and
wire adapter tests.
