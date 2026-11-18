# #161 Move streaming text decoding to the canonical API shared namespace

Status: implemented; canonical-only namespace enforced
Owner: current implementation slice
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)

## Goal

Remove the remaining crate-root `text_decode` implementation and make the
shared streaming wire utility explicit:

- `opi_ai::api::shared::text_decode` owns incremental UTF-8 decoding used by
  multiple protocol adapters;
- provider profiles do not own byte framing or text decoding;
- API adapters share one decoder implementation and retain their existing
  streaming behavior;
- the old root module is deleted with no compatibility shim.

## Ownership decision

`StreamTextDecoder` is a protocol-neutral wire helper. It carries incomplete
UTF-8 sequences across network chunks and applies the pinned replacement
semantics; it does not select a provider, construct an endpoint, resolve
authentication, or own a transport. Its production callers are the
Anthropic, Azure OpenAI, Gemini, Mistral, OpenAI Chat, OpenAI Responses,
Codex Responses, Pi Messages and Vertex API adapters. The root placement
therefore hides a shared API-adapter seam and makes the ownership graph look
provider-specific when it is reused across wire implementations.

The canonical owner is the crate-private `api::shared::text_decode` module.
The implementation is intentionally not promoted to a public consumer API:
its contract is an internal adapter seam, while public stream/event contracts
remain under `stream`. No `providers::shared` namespace is introduced.

## Acceptance

1. `src/text_decode.rs` is deleted and the implementation lives only at
   `src/api/shared/text_decode.rs`.
2. `api::shared` declares the decoder as a crate-private shared module; all
   protocol adapter callers use that canonical path.
3. The decoder's split UTF-8, invalid-subsequence and EOF behavior remains
   covered by its moved unit tests.
4. Canonical conformance asserts root deletion and absence of old callers.
5. A consumer built from the actual `.crate` archive compiles and runs the
   protocol-adapters profile, proving the moved implementation is included
   in the published package without a root module.

## Implementation and verification

The former root implementation was moved to
[`src/api/shared/text_decode.rs`](../../../../crates/opi-ai/src/api/shared/text_decode.rs).
`api::shared` declares it as `pub(crate)` so the decoder remains an internal
adapter seam; all nine protocol adapters now reference the canonical path and
`lib.rs` no longer declares the root module. The old root file is deleted and
no compatibility re-export is retained.

The actual archive consumer passed with the protocol-adapters feature. The
runner also inspected the extracted package and recorded that the deleted root
file is absent while `src/api/shared/text_decode.rs` is present. The receipt is
[`issue-161-protocol-adapters-consumer-20261117.json`](../evidence/issue-161-protocol-adapters-consumer-20261117.json)
(`PASS`; rustc/cargo version, lease, package, lockfile, metadata, build, run
and lease-end all exited zero).

Focused verification passed:

- moved decoder unit tests: 9 passed;
- `external_sdk`: 11 passed;
- `tool_result_wire`: 2 passed;
- canonical provider conformance: 27 passed;
- protocol-adapters runner tests: 4 passed;
- Python compilation for the changed conformance scripts;
- builtins and no-default `opi-ai` library checks;
- warnings-denied Clippy for `opi-ai` and `opi-coding-agent`;
- builtins and no-default warnings-denied rustdoc;
- formatting, documentation and whitespace checks.

## Test impact

`add`: root-deletion/canonical-caller conformance assertions.

`update`: API adapter imports, shared module declaration and iteration
documentation; the existing decoder unit tests move with the implementation.

`delete`: `src/text_decode.rs` and its root module declaration.

`retain`: all streaming protocol fixtures and the decoder's behavioral cases.
