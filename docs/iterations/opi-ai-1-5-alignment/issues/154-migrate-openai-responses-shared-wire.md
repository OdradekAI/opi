# #154 Move the shared OpenAI Responses wire mapper into the canonical API namespace

Status: implemented; canonical API/shared namespace enforced  
Owner: current implementation slice  
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)

## Goal

Remove the remaining root-level `src/openai_responses_shared.rs` module and
place its provider-neutral wire machinery under the canonical API namespace:

- `opi_ai::api::shared::openai_responses` owns the shared Responses SSE frame
  decoder, event decoder, message/tool projection and `ResponsesMapper` used by
  the OpenAI Chat, OpenAI Responses, Codex and Azure wire adapters.
- `opi_ai::providers::*` continues to own provider identity and construction
  state; no provider profile imports this mapper as a profile concern.
- The shared module remains crate-private because it is an implementation seam,
  not a public provider API.

## Ownership decision

`openai_responses_shared.rs` is a wire mapper, not a provider. Its inputs and
outputs are protocol/message types and `WireApi` identities, while its callers
are API adapters for several providers. Moving it below `api::shared` makes the
dependency direction explicit: provider profiles do not depend on it, API
adapters may share it, and callers cannot observe a second root namespace.

## Acceptance

1. The mapper lives at `src/api/shared/openai_responses.rs`, is declared by the
   canonical `api::shared` namespace, and remains gated by
   `provider-openai-responses-shared`.
2. OpenAI Chat, OpenAI Responses, Codex and Azure adapters use the canonical
   `crate::api::shared::openai_responses` path.
3. `src/openai_responses_shared.rs` and its root module declaration are deleted;
   no compatibility shim or active old-path reference remains.
4. Existing mapper, Responses, Codex, Azure and OpenAI Chat tests continue to
   pass; conformance scans and active documentation identify the canonical path.
5. Provider-specific external consumers and the broader protocol-adapter
   consumer build from the actual crate archive after the namespace move.

Historical snapshots and prior evidence may retain the old path as audit data;
active source and this migration record must not depend on it.

## Implementation and verification

The mapper now lives at `src/api/shared/openai_responses.rs`, is declared by
the crate-private `api::shared` module and remains selected by
`provider-openai-responses-shared`. OpenAI Chat, OpenAI Responses, Codex and
Azure use the canonical path. The root file and `lib.rs` declaration are gone;
no shim remains.

- Minimal OpenAI Responses owner check with
  `provider-openai-responses`: PASS.
- Shared mapper unit tests: 3 passed, 0 failed.
- Responses/Codex/Azure/OpenAI Chat focused integration set: 386 passed,
  0 failed; full `opi-ai` built-in test target compilation: PASS.
- Provider protocol conformance: 13 passed, 0 failed; canonical caller scan
  reports 42 OpenAI Responses owner files and 117 matches, with six canonical
  shared-owner entries including `api/shared/openai_responses.rs`.
- `opi-ai` clippy with `-D warnings` (allowing the repository's existing
  `too_many_arguments` lint): PASS. Documentation checker, formatting,
  whitespace and Python compilation checks: PASS.
- External consumer receipts from the actual `opi-ai` archive: OpenAI
  Responses ([issue-154-openai-responses-shared-consumer-20261117-r1.json](../evidence/issue-154-openai-responses-shared-consumer-20261117-r1.json)),
  Codex ([issue-154-openai-responses-shared-codex-consumer-20261117-r1.json](../evidence/issue-154-openai-responses-shared-codex-consumer-20261117-r1.json)),
  Azure ([issue-154-openai-responses-shared-azure-consumer-20261117-r1.json](../evidence/issue-154-openai-responses-shared-azure-consumer-20261117-r1.json))
  and the full protocol-adapter group
  ([issue-154-protocol-adapters-consumer-20261117-r1.json](../evidence/issue-154-protocol-adapters-consumer-20261117-r1.json))
  all report PASS.

Historical snapshots and prior evidence retain the old path only as audit
records; active source and documentation use the canonical path. The negative
deletion assertion in the conformance test and this brief's acceptance text
intentionally name the removed path.

## Test impact

`add`: canonical shared-module deletion assertion and four external consumer
receipts; `update`: module declarations, API callers, conformance scanner,
tests and active documentation; `delete`: obsolete root shared mapper;
`retain`: mapper behavior and provider wire adapters.
