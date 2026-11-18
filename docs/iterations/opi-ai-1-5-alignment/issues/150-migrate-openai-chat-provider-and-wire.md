# #150 Split the OpenAI Chat Provider profile from the wire adapter

Status: implemented; canonical namespaces enforced
Owner: current implementation slice
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)

## Goal

Move the legacy `src/api/openai.rs` owner into explicit canonical namespaces
without retaining a compatibility module:

- `opi_ai::providers::openai` owns the OpenAI Chat identity, endpoint, model
  catalog, compatibility profile, shared HTTP client and per-dispatch state.
- `opi_ai::api::openai` owns the Chat Completions SSE parser, event mapper,
  transcript/request projection, streaming transport, status/error handling and
  typed `OpenAiCompletionsChatApi` adapter.

OpenRouter, Mistral and Azure continue to reuse the shared OpenAI wire adapter,
but import the provider profile and wire symbols from their respective
canonical namespaces.

## Ownership boundary

The Provider profile contains identity and configuration state only. Wire
serialization, SSE/event mapping, request execution, terminal handling and the
`Provider`/`ChatApi` implementations live in the API module. No auth, catalog,
retry, stream terminal or telemetry behavior changes are intended.

## Acceptance

1. `OpenAiChatProvider`, `CompatConfig`, `ModelCompatOverride` and
   `model_catalog` live under `src/providers/openai.rs`.
2. Parser, mapper, options, request body, transport and typed Chat adapter live
   under `src/api/openai.rs`.
3. `src/openai_chat.rs` and the `opi_ai::openai_chat` module are deleted; no
   compatibility shim remains.
4. Shared provider callers, tests, scripts and current documentation use the
   canonical profile/API paths.
5. OpenAI Chat fixture/lifecycle/observer tests, provider protocol tests,
   coding-agent consumers and an external consumer built from the actual crate
   archive pass.

Historical snapshots and prior evidence may retain the old path as audit data;
active source and this migration record must not depend on it.

## Implementation and verification

The implementation creates `providers::openai` and `api::openai`, updates the
shared Azure/Mistral/OpenRouter callers and removes the root module.

- `cargo fmt --all -- --check`: PASS.
- Minimal owner check with `provider-openai-chat`: PASS.
- OpenAI Chat fixture/lifecycle/observer/metadata targets: 24 + 5 + 116 + 1 +
  1 + 1 + 3 + 15 tests passed.
- `provider_openai_chat_profile`: 1 passed; `protocol_adapter_profile`: 2
  passed.
- Shared regressions: `external_sdk`: 11 passed; `provider_collection`: 111
  passed.
- Coding-agent `provider_factory`: 58 passed.
- `cargo clippy -p opi-ai --all-targets -D warnings
  -A clippy::too_many_arguments`: PASS.
- Provider protocol Python harness: 8 passed; modified scripts compile;
  documentation checker and task-owned whitespace check: PASS.
- Canonical caller scan records the `openai` owner across API/profile modules;
  no active `opi_ai::openai_chat`, `crate::openai_chat` or root-file reference
  remains. Remaining matches are the negative deletion assertion and this
  migration record.
- External consumer receipt:
  `evidence/issue-150-openai-chat-consumer-20261117-r2.json` reports the
  actual packaged `opi-ai` archive passing metadata, build and run with the
  exact `provider-openai-chat` feature closure while compiling both
  `api::openai` and `providers::openai` symbols.

## Test impact

`add`: canonical profile/API assertions and external consumer receipt;
`update`: module declarations, shared callers, tests, scripts and docs;
`delete`: obsolete root module/file; `retain`: historical snapshots and prior
evidence as audit records. No commit or stage is performed.
