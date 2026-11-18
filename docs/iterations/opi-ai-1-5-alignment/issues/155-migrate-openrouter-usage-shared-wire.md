# #155 Move OpenRouter usage accounting into the canonical API shared namespace

Status: implemented; canonical API/shared namespace enforced  
Owner: current implementation slice  
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)

## Goal

Remove the root-level `src/openrouter_usage.rs` implementation module and put
the shared OpenRouter usage/accounting wire seam under the responsibility
oriented API namespace:

- `opi_ai::api::shared::openrouter_usage` owns the OpenRouter billing fields,
  deserialization shapes and the pure projection into the common `Usage` cost
  breakdown used by API adapters;
- `opi_ai::providers::openrouter` owns provider identity, endpoint, headers and
  catalog, and intentionally re-exports the accounting types as the existing
  provider-facing public contract;
- no provider profile imports or owns the shared implementation, and no root
  `opi_ai::openrouter_usage` compatibility path remains.

## Ownership decision

`openrouter_usage.rs` is a wire-level accounting mapper, not a Provider
profile. Its inputs are OpenRouter extension fields attached to otherwise
generic Anthropic/OpenAI usage objects; its output is the common `Usage` cost
state. Anthropic and OpenAI Chat are the production API callers, while the
OpenRouter profile only exposes the established public types/functions. The
canonical owner is therefore `crate::api::shared::openrouter_usage`, alongside
the shared Responses wire mapper. `providers::openrouter` remains the public
provider-facing re-export so external consumers do not need a crate-private
implementation path.

## Acceptance

1. The implementation lives at `src/api/shared/openrouter_usage.rs`, is
   declared by `api::shared`, and is selected by
   `provider-openrouter-usage`.
2. Anthropic and OpenAI Chat wire adapters use the canonical
   `crate::api::shared::openrouter_usage` path.
3. `src/openrouter_usage.rs` and its root `lib.rs` declaration are deleted;
   no compatibility shim or active old-root reference remains.
4. The public `providers::openrouter` accounting re-export continues to work,
   while provider construction remains in the Provider profile module.
5. Existing OpenRouter cost/fixture tests, Anthropic and OpenAI Chat usage
   tests, conformance scans and active documentation identify the canonical
   namespace.
6. Provider-specific external consumers built from the actual crate archive
   validate both the OpenRouter public accounting surface and the affected
   provider feature closures.

Historical snapshots and prior evidence may retain the old path as audit data;
active source and this migration record must not depend on it.

## Implementation and verification

The accounting mapper now lives at
`crates/opi-ai/src/api/shared/openrouter_usage.rs`. `api::shared` is compiled
when either shared Responses mapping or OpenRouter usage accounting is
selected. Anthropic and OpenAI Chat import the canonical path, while
`providers::openrouter` keeps the deliberate public re-export. The obsolete
root module and declaration are gone; no shim remains.

- PASS: the exact `provider-openrouter` owner compile and the narrow
  `provider-anthropic` / `provider-openai-chat` feature checks.
- PASS: OpenRouter cost and fixture tests plus affected Anthropic/OpenAI Chat
  usage fixtures: 186 tests passed, 0 failed; the shared mapper behavior
  remains unchanged. The full built-in `opi-ai` test target also compiles.
- PASS: warnings-denied `opi-ai` Clippy with the repository's existing narrow
  `too_many_arguments` exception.
- PASS: provider protocol Python tests (15 passed), canonical caller scan
  (1,272 files scanned; OpenRouter owner 23 files/40 matches; four canonical
  shared-accounting entries), Python syntax, documentation, formatting and
  whitespace checks.
- PASS: external package consumers built from the actual crate archive for the
  OpenRouter owner ([receipt](../evidence/issue-155-openrouter-usage-openrouter-consumer-20261117-r1.json)),
  Anthropic ([receipt](../evidence/issue-155-openrouter-usage-anthropic-consumer-20261117-r1.json))
  and OpenAI Chat ([receipt](../evidence/issue-155-openrouter-usage-openai-chat-consumer-20261117-r1.json))
  owner closures; all report `PASS`, and the OpenRouter consumer references
  the canonical public provider re-export.

Baseline caveat: selecting the implementation-only `provider-openrouter-usage`
feature by itself still reaches the repository's pre-existing
`provider-protocol-common` feature graph, which currently compiles unrelated
HTTP-backed modules without `http-transport`. The three owner-specific feature
closures above are the supported consumer checks and all pass; this migration
does not widen or otherwise change that existing graph.

## Test impact

`add`: canonical root-module deletion/caller assertions and external consumer
receipts; `update`: API shared module declaration, wire adapter imports,
provider re-export, feature/docs/conformance metadata and focused tests;
`delete`: obsolete root accounting module and declaration; `retain`: public
`providers::openrouter` accounting contract and all existing cost semantics.
