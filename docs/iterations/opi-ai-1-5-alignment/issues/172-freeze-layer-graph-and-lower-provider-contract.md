# #172 Freeze the layer graph and extract the lower provider contract

Status: open
Owner: current implementation slice
Parent: [#164 unresolved ownership audit](164-audit-unresolved-ownership-boundaries.md)
Related: [module structure decision](../decisions/06-module-structure.md)

## Why the direction changes here

The directory migration has already established `api/`, `providers/`,
`transport/`, `auth/` and `catalog/` owners, and #157–#162 removed the old
root provider/helper paths. The remaining problem is dependency ownership. A
file-count comparison with `pi-ai` would now send the work in the wrong
direction.

The current graph still contains these structural violations:

| Boundary | Current evidence | Required direction |
| --- | --- | --- |
| Provider contract ↔ API contract | `src/provider.rs:10,71,84,152` imports `api::contract`; `src/api/contract.rs:15` imports provider request/stream/error types | The lower provider contract may be consumed by API adapters, but it must not import API contracts |
| API adapter → provider profile | Gemini, Vertex, Bedrock, OpenAI Chat, Mistral Conversations and Pi Messages now use `provider::routes` contexts; 6 concrete `crate::providers::` edges remain under `src/api` | Adapters consume a provider-neutral route context and protocol inputs; composition binds concrete profiles to adapters |
| Provider profile → provider profile | Mistral and OpenRouter now construct `OpenAiRouteContext`; Azure is the remaining concrete OpenAI profile edge | Shared protocol configuration and route bindings are reused; one provider profile does not construct another provider profile |
| Profile → native wire state | No remaining `providers` native-option state after #177 | Profiles own identity, route, auth policy and catalog; adapters own native options and wire state |
| Stream ↔ execution | `stream.rs` re-exports `StartedChatStream` from `execution.rs`; `execution.rs` consumes private stream types | A stream contract owns event/result consumption; an execution driver owns producer/task state and depends on the lower contract |
| Models ownership | `models.rs` is 5,335 lines and combines facade, registry, auth/catalog orchestration, four registration families, erased dispatch, prepared calls and execution; it still has Pi/Anthropic/Bedrock wire branches | Keep `Models` as the stable facade and move implementation into private owners. Protocol policy belongs to the adapter/provider capability that owns it |

The `types` and `errors` entries in the bottom row are currently a shared
contract bundle, not a claim that every value module already forms a strict
acyclic graph. `errors.rs` still carries some stream/transcript-facing values
and several `types` result contracts carry `ProviderError`. A later purity
slice may reduce that coupling, but it must not be hidden inside a provider
directory move or counted as complete merely because the crate compiles.

These are dependency-graph failures, not missing files. The work therefore
pauses provider-by-provider directory moves until the lower contract and the
structural gates below are explicit.

## Target graph

```text
types / errors / pure request + stream values
                    ↑
provider route/dispatch + transport/auth/catalog contracts
                    ↑
api operation contracts and protocol adapters
                    ↑
provider profiles and factories
                    ↑
builtin composition root  ──→  Models facade  ──→  external consumers

execution driver / stream consumer
  → lower provider dispatch contract + types + telemetry
  → never concrete provider profile or wire adapter
```

`api::contract` remains the public operation surface for chat, image,
classifier and deferred adapters. Its shared request, event, error and
authentication inputs must come from a lower, dependency-neutral contract.
The exact module name may be `provider::contract`, `provider::operation` or a
new `types` submodule. The deciding property is the graph, not the filename.

The lower provider surface should be composed from small contracts rather than
one new universal trait. Route identity/catalog, prepared dispatch, catalog
lifecycle, resource ownership and deferred registration are separate concerns.
`Provider` may remain a compatibility facade while these owners are extracted,
but its implementation must not mention `api::*` or protocol-specific policy.
Neutral `SimpleOptions` and request values belong below API adapters. Deferred
backend registration is a composition concern and must not force the lower
provider trait to import `api::contract::DeferredApi`.

## Measurable convergence gates

These gates are the progress ledger. Counts are observations from the
2026-11-18 worktree, not acceptance by themselves.

| Gate | Current | Close condition |
| --- | ---: | --- |
| G0 canonical root paths | #157–#162 complete | No deleted root provider/helper path is recreated; canonical archive consumer stays green |
| G1 profile native state | 0 tracked references after #177 | `rg` finds no `native_options`, `with_native_options` or API-native option imports under `src/providers` |
| G2 adapter/profile inversion | 6 concrete API/profile edges remain; `api_route_context_edges=6`; 1 concrete profile-to-profile edge remains | `src/api` has zero concrete `crate::providers` imports and profiles have zero concrete provider-to-provider imports; one protocol is exercised by two profiles through the same adapter |
| G3 lower contract direction | The first slice moved `SimpleOptions` and Deferred backend contracts to `provider::operation`; neutral `Request` and stream/error values still live in the broader provider layer | `provider.rs` has no `crate::api` dependency, `api::contract` consumes the lower operation seam through re-exports, and the remaining `types`/`errors` bundle coupling is recorded explicitly rather than silently treated as a DAG |
| G4 execution/stream ownership | mutual implementation references remain | consumer/event contract and execution driver have one-way ownership; cancellation, stop-consumption and source cleanup tests remain green |
| G5 Models owners | `RegistryOwner`, `AuthOwner`, `CatalogOwner` and `OperationOwner` extracted, 5,424-line facade implementation remains; `PreparedProviderCall` is the execution owner | private owners cover registry, auth/catalog, operation registration and prepared execution; facade methods stay source-compatible and contain no protocol-specific branches |
| G6 composition proof | feature/package slices exist | core/Faux, one-provider and full builtins profiles pass structural checks, focused behavior tests and an external archive consumer |

Line count is diagnostic only. G5 closes on ownership and dependency checks,
not on an arbitrary file-size threshold.

## Ordered work

1. Freeze a dependency matrix and add a failing structural check for G1–G4.
   The check must inspect source imports and forbidden profile-owned symbols,
   not only directory existence.
2. Extract the lower neutral request/dispatch seam. Remove the
   `provider.rs → api::contract` imports first. Preserve public re-exports only
   where a real 0.x consumer contract requires them, and record each bridge.
3. Finish profile option removal for Bedrock, Gemini, Mistral, Pi Messages and
   Vertex. Then extract route context so no API adapter needs a concrete
   profile type.
4. Split `Models` behind its existing public facade. Migrate one owner at a
   time and move Pi/Anthropic/Bedrock policy into the owning adapter or
   provider capability with focused regression tests.
5. Split stream contract from the execution driver only after the lower
   dispatch seam is stable. Re-run lifecycle, telemetry and resource tests,
   followed by one new protocol plus two provider archive consumption.

No provider feature, transport backend or Pi behavior is removed by this
issue. It changes the order and admission criteria so that each subsequent
provider migration reduces a measured edge instead of adding another partial
composition.

## Latest receipt

2026-11-18 的 [#182 Gemini route context](182-invert-gemini-route-context.md)
与 [#183 Vertex route context](183-invert-vertex-route-context.md) 将 Gemini
与 Vertex route state 抽取到 lower `provider::routes` 上下文。API adapter
直接依赖中性上下文，两个 `providers::*Provider` 保留为 canonical alias。
Gemini 71 项与 Vertex 73 项 fixture 通过，`api_provider_edges` 从 14 降至
12，`api_route_context_edges=2`，`provider_to_provider_edges` 从 4 降至 3。
随后 [#184 Bedrock route context](184-invert-bedrock-route-context.md) 又将
Bedrock adapter 接到 lower context，Bedrock fixture 179 项通过，当前
`api_provider_edges=9`、`api_route_context_edges=3`。其余 adapter/profile
edges 继续按本票推进。

随后 [#185 OpenAI Chat route context](185-invert-openai-chat-route-context.md)
将 OpenAI-compatible route state 抽取到 `OpenAiRouteContext`，OpenAI Chat
fixture 116 项通过，当前 `api_provider_edges=8`、
`api_route_context_edges=4`。

随后 [#187 Mistral Conversations route context](187-invert-mistral-conversations-route-context.md)
将 native Mistral route state 抽取到 `MistralRouteContext`，Mistral fixture
55 项通过，当前 `api_provider_edges=7`、`api_route_context_edges=5`。

随后 [#186 OpenAI route context sharing](186-share-openai-route-context-across-profiles.md)
将 Mistral 与 OpenRouter 直接接到 `OpenAiRouteContext`，profile-to-profile
edges 从 3 降至 1。Azure 的 dual-wire route context 继续按本票推进。

随后 [#188 Pi Messages route context](188-invert-pi-messages-route-context.md)
将 Radius route state 抽取到 `PiMessagesRouteContext`，Pi Messages chat
回归 42 项通过，当前 `api_provider_edges=6`、`api_route_context_edges=6`。

## Acceptance

- A checked-in dependency matrix records the allowed edges and current counts.
- `python scripts/pi-sdk-conformance/check_architecture_boundaries.py --json`
  emits the current edge counts, and `--enforce` fails while any forbidden
  edge remains.
- A structural check fails on any API adapter concrete-profile import, any
  profile-owned native option, and any lower provider contract import of
  `api::*`.
- The lower contract and execution/stream ownership changes preserve the
  existing public facade, lifecycle semantics and external consumer proofs.
- G1–G6 each have a focused receipt. No issue is marked complete because a
  directory exists or a package compiles under `--all-features` alone.

## Test impact

`add`: this boundary issue and its structural-check receipt.
`update`: #164, the module-structure decision and the implementation handoff.
`delete`: none.
`retain`: all provider fixtures, lifecycle/telemetry/resource tests, feature
profiles and archive consumers until a specific migration replaces them.
