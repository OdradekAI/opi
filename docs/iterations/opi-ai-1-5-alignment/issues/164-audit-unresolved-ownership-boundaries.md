# #164 Audit unresolved ownership boundaries after the root-module pass

Status: audit complete; migrations open  
Owner: current implementation slice  
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)  
Related: [#163 root-module ownership](163-audit-root-module-ownership.md)

## Why this follow-up exists

[#163](163-audit-root-module-ownership.md) answered a narrow question: which
modules declared from `src/lib.rs` still have a legitimate crate-wide owner. It
did not prove that the retained owners are separated from one another. The
original module decision requires both properties. A directory move is not a
structure proof when an API adapter still imports a concrete provider profile,
or when `Models` still owns protocol contracts and wire-specific rules.

This brief records the remaining evidence before another migration is started.
It deliberately separates observed violations from design choices that still
need a dedicated issue. No compatibility shim is introduced.

## Current evidence

| Boundary | Evidence in the current tree | Required rule | Result |
| --- | --- | --- | --- |
| Chat operation contract | `api::contract.rs` defines `ChatApi`, neutral options and typed handles; adapters implement `crate::api::contract::ChatApi` | `api` owns operation contracts; `models` owns the facade and dispatch | **Conforming after #165** |
| Deferred operation contract | `api::contract.rs` defines `DeferredApi`, fetch/cancel calls and typed registration handles; `types::deferred` keeps value/error types | Deferred submit/fetch/cancel contract belongs with the API operation surface; `types::deferred` keeps value/handle types | **Conforming after #166** |
| API facade base types | `api.rs` re-exports the canonical contracts and imports `crate::types::ApiId`; adapters use `crate::types::ApiId` directly | `api` must expose the operation contract and use `types::ApiId` directly | **Conforming after #165/#166** |
| API → provider dependency | `api/anthropic.rs`, `azure_openai.rs`, `bedrock.rs`, `gemini.rs`, `mistral.rs`, `openai.rs`, `openai_responses.rs`, `openai_codex_responses.rs`, `pi_messages.rs` and `vertex.rs` import `crate::providers::*` | API adapters must consume provider-neutral route/profile inputs; `providers` composition may instantiate adapters | **Violation** |
| Provider profile → wire option dependency | `providers/openai.rs` is clean after #167, `providers/openai_responses.rs` is clean after #168, `providers/openai_codex.rs` is clean after #169, `providers/anthropic.rs` is clean after #170, and `providers/azure_openai.rs` is clean after #171; `bedrock.rs`, `gemini.rs`, `mistral.rs`, `pi_messages.rs` and `vertex.rs` still import/store native option state | Profile owns identity/endpoint/catalog/auth policy; protocol-native options and wire state belong to the adapter/binding | **Partially conforming; remaining profiles are violations; #172 is next** |
| Models facade size/ownership | `models.rs` is about 230 KB and owns `Models`, registry lookup, auth/store/OAuth coordination, catalog refresh, four registration families, `ChatApi`, erased dispatch, `PreparedProviderCall`, `ChatCall`, and call execution | `models` keeps the stable facade; internal owners are split into registration, auth/catalog orchestration and execution modules | **Violation** |
| Models → protocol rules | `models.rs:1871` special-cases Pi Messages; `models.rs:4242–4283` computes Anthropic and Bedrock thinking budgets; `models.rs:4094–4117` selects wire defaults | wire-specific validation/projection policy belongs to provider/API capability owners | **Violation** |
| Provider composition | `providers/collection.rs` selects concrete wire routes, registers typed image/classifier adapters and builds mapped routes | composition root may depend on API adapters and `Models`; it must not contain parsers or stream state machines | **Conforming with a boundary to preserve** |
| Neutral shared wire helpers | `api::shared::{headers,env,text_decode,endpoint,openai_responses,openrouter_usage}` have no provider profile ownership | shared wire seams stay below provider composition and remain canonical-only | **Conforming** |
| Root module ownership | The #163 table remains valid for public domains, transport, auth/catalog, lifecycle and private substrates | Root retention is allowed only for those owners; this does not close cross-layer coupling | **Conforming, insufficient by itself** |

The most consequential cycle is currently structural rather than a Rust module
cycle: adapter implementations need concrete profile structs for endpoint,
client and policy state, while profile structs need adapter-native option types
for direct registration. This prevents the intended “one protocol, many
providers” seam even though the files now live under separate directories.

## Canonical ownership target

The next target keeps the current one-crate packaging decision but makes the
seams explicit:

```text
types / errors / pure semantics
              ↑
provider contract + transport/auth/catalog contracts
              ↑
api::contract (Chat/Image/Classifier/Deferred) + api wire adapters
              ↑
providers profiles and composition
              ↑
Models facade and external consumers
```

`Models` remains the public facade, but its implementation is split behind the
same entry point. A provider profile supplies a provider-neutral route context
(identity, complete model record, endpoint, headers, auth policy and injected
client); an API adapter owns native options, wire encoding/decoding, protocol
state and stream mapping. The composition root binds the two. This is the seam
that allows a new protocol to be used by two providers without importing either
provider from the adapter.

## Evidence-backed migration order

1. **Contract extraction.** Create the canonical `api::contract` surface for
   `ChatApi`, `DeferredApi`, their call/handle types and the existing image and
   classifier traits. Move `ApiId` call sites to `types::ApiId`. Keep
   `SimpleOptions` as a neutral value while removing its private request
   mutation from the public contract.
2. **Adapter/profile decoupling.** Introduce the smallest provider-neutral route
   context needed by one protocol reused by two providers. Migrate one pair
   first (OpenAI Chat plus OpenRouter/Mistral profile) and prove that the API
   module no longer imports `providers::*` and the profile no longer stores
   adapter-native options. The same evidence-backed rule has now been applied
   to Anthropic in [#170](170-decouple-anthropic-profile-options.md), including
   moving pure configured header values below the API layer. Azure OpenAI now
   follows the same rule in [#171](171-decouple-azure-openai-profile-options.md):
   its per-call options and dual-wire dispatch state live in the canonical API
   adapter while the profile keeps route/catalog state. The next evidence-backed
   profile slice is Bedrock; create its brief only after the remaining ownership
   evidence is refreshed.
3. **Models internal owners.** Split registration/erasure, auth/catalog
   orchestration, and prepared-call/execution state into private submodules
   while keeping `opi_ai::Models` and existing accepted public methods stable.
   Move wire-specific thinking/default policy out of the facade as part of the
   owning adapter/provider capability contract, with focused behavior tests.
4. **Contract and consumer proof.** Add structural checks for forbidden imports,
   rerun affected unit/integration tests, then run the actual `.crate`
   external-consumer probe. The proof must include one new protocol and two
   providers using the same contract; directory existence alone is not enough.

## Acceptance for this brief

- A checked-in ownership table covers every active root module and every
  unresolved cross-layer edge above.
- Each migration issue names its moved symbols, real callers, behavior tests,
  documentation updates and archive-consumer command.
- No adapter imports a concrete `providers::*` profile after the decoupling
  wave, and no provider profile stores an adapter-native options value.
- `models.rs` retains only the facade and orchestration interface after its
  private owner split; protocol-specific branches are removed from it.
- The old root provider modules remain deleted according to the
  canonical-only policy. No compatibility module is added merely to make the
  migration easier.

## Test impact

`add`: this audit and its structural acceptance criteria.  
`update`: #163, the module-structure decision and the iteration record now
distinguish root ownership from cross-layer decoupling.  
`delete`: none.  
`retain`: all existing provider behavior, workspace consumer and archive
consumer checks until a specific migration replaces them.

## Follow-up boundary review

The 2026-11-18 review found two lower-layer edges that this audit did not
close. `provider.rs` imports `api::contract` for `DeferredApi` and
`SimpleOptions`, while `api::contract` imports provider request/stream/error
values. `stream.rs` also re-exports the execution owner while `execution.rs`
consumes stream-private types. These are graph violations even when Rust can
compile the module cycle. They are tracked with measurable gates in
[#172](172-freeze-layer-graph-and-lower-provider-contract.md), which now
controls the order of the remaining profile and Models migrations.
