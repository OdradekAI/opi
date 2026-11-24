# opi-ai test organization

[targets.tsv](targets.tsv) records one owner, test layer, case family, cost class and any directly bound runners per integration target. Cargo owns the source path and required features. To see the joined inventory or check it before adding a target, run from the workspace root:

```sh
python scripts/pi-sdk-conformance/check_test_inventory.py --list
python scripts/pi-sdk-conformance/check_test_inventory.py
```

The layer identifies what the assertions exercise: a public contract, protocol wire, assembled services, or a feature profile. Cost describes the resources used. `in-process` includes frozen fixtures, `local-io` covers local peers and files, and `extended` covers subprocesses, TLS/WS lifecycle, performance measurements or larger scenarios. The runner column names entry points that execute cases or verify the same public boundary from a package. A dash means that ordinary Cargo selection applies with no named runner binding. The feature column in the joined view is Cargo's declared admission condition. `core_profile`, for example, runs four cases with no features and adds one with `telemetry-tracing`.

Keep shared setup in `tests/support/` (TLS peers, catalog store, pending auth). Keep changing protocol expectations and their data with the owning test family. For a new root target add an explicit Cargo declaration and one inventory row. Provider feature-profile targets each own a source file so the owner and feature gate are visible from the target path. A genuinely shared source needs an explicit reason and independent inventory rows. When renaming a target or case, inspect its bound runners before changing the name.

`tests/support/OWNERS.tsv` is the source of truth for shared-helper ownership. It
records each helper's role, owner, layers, cost, runners and resolved Cargo
consumers. The checker derives consumers from the actual Rust module graph, so a
manual consumer list cannot hide a stale or unused helper:

```sh
python scripts/pi-sdk-conformance/check_support_inventory.py --list
python scripts/pi-sdk-conformance/check_support_inventory.py
```

Use `stable-tool` for deterministic setup with no protocol policy, `protocol-fixture`
for request, auth or storage expectations tied to a contract family, and
`platform-fixture` for transport or operating-system behavior. A helper spanning
different layers lists all of them. Shared helpers remain test-only and must not
be copied into `src/`.

The combined daily gate checks architecture, Cargo target/source inventory,
shared-support ownership and selector coverage together:

```sh
python scripts/pi-sdk-conformance/check_structure_gate.py --json
```

Use `--strict-selection` with changed `crates/opi-ai` paths when an unmapped
path must fail immediately. Keep the full workspace regression explicit with
`cargo test --workspace --exclude opi-eval --all-targets --locked`.
For a root target with named topic modules, changing a topic source emits its qualified module filter. A shared fixture source with no tests keeps the full target selected.

### Choose tests for a change

Run the selector from the repository root with one or more changed paths. It prints the reason for each target, the Cargo command with its feature gates and relevant runner bindings. Use `--json` to read the commands as argument arrays.

```sh
python scripts/pi-sdk-conformance/select_tests.py crates/opi-ai/src/api/openai_decisions.rs
python scripts/pi-sdk-conformance/select_tests.py crates/opi-ai/tests/support/pending_auth.rs
python scripts/pi-sdk-conformance/select_tests.py --target openrouter_cost
python scripts/pi-sdk-conformance/select_tests.py --full
```

Reviewed classifier changes select the wire cases plus shared dispatch and cross-layer behavior. Shared test sources select consuming targets, including targets that compile one file with different features. Cargo feature changes, builtin composition and unmapped paths require a broader check. When moving a contract or changing a case name, update its reviewed selection and confirm the generated command executes the intended cases.

### Run one classifier protocol

The `classifier_contract` target groups wire tests by protocol and keeps Models lifecycle, cross-protocol behavior and declaration order in separate modules. Run the Decisions wire cases with this command.

```sh
cargo test --locked --offline -p opi-ai --test classifier_contract -- openai_decisions::
```

The other wire module prefixes are `typesafe_system_one::`, `cloudflare_workers_ai_system_one::` and `llama_cpp_classify::`. Use the selector for adapter source changes to include the cross-protocol and SDK cases that exercise that wire. A change to the shared System One implementation selects the entire classifier target. Keep wire-specific responses with their protocol. Local fixtures shared across these topics live under `tests/classifier_contract/`, and helpers shared across integration targets live under `tests/support/`.

[中文](README.zh.md)

Rust supports both inline test modules and separate test files. What matters is
the interface crossed, not the file size or number of modules exercised.
Descendant modules can access ancestor private items; integration targets are
separate crates using the public interface. See [Rust Book: test organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html).

| Purpose | Location | Evidence |
| --- | --- | --- |
| Private mechanisms | `src/<module>/tests.rs` or `src/<module>/tests/` | Algorithms, parsers, state transitions and resource ownership |
| Private combinations | The same crate-local test module, grouped by contract | Internal handoff, such as Upgrade-to-codec; no public provider caller is implied |
| Public module/combination acceptance | `tests/<contract>.rs`, optionally with `tests/<contract>/*.rs` submodules | Caller interfaces, actual adapters, request bytes, typed failures and lifecycle |
| Shared integration helpers | `tests/support/*.rs`, declared by consuming targets | Local peers and synthetic fixtures; no separate test executable |
| SDK consumers | `scripts/pi-sdk-conformance/` and external scratch | Actual package, independent dependency graph and public consumer behavior |

The focused layout is:

```text
src/crypto/mod.rs                  # production; cfg(test) mod tests
src/crypto/tests/mod.rs            # private test index/shared helpers
src/crypto/tests/admission.rs
src/crypto/tests/signing.rs
src/crypto/tests/worker.rs
src/crypto/tests/jwt.rs
src/ws/tests/                      # private codec/internal combinations
tests/transcript_contract.rs       # one integration target
tests/anthropic_fixtures.rs         # one Anthropic protocol target
tests/anthropic_fixtures/fixtures.rs
tests/anthropic_fixtures/sse.rs
tests/anthropic_fixtures/constrained_sampling.rs
tests/anthropic_fixtures/adapter.rs
tests/gemini_fixtures.rs          # one Google Gemini protocol target
tests/gemini_fixtures/fixtures.rs
tests/gemini_fixtures/wire.rs
tests/gemini_fixtures/lifecycle.rs
tests/gemini_fixtures/callbacks.rs
tests/gemini_fixtures/constrained_guard.rs
tests/gemini_fixtures/google_40.rs
tests/mistral_fixtures.rs        # one Mistral protocol target
tests/mistral_fixtures/fixtures.rs
tests/mistral_fixtures/chat_wire.rs
tests/mistral_fixtures/chat_lifecycle.rs
tests/mistral_fixtures/chat_callbacks.rs
tests/mistral_fixtures/native_conversations.rs
tests/mistral_fixtures/native_conversations/fixtures.rs
tests/mistral_fixtures/native_conversations/wire.rs
tests/mistral_fixtures/native_conversations/lifecycle.rs
tests/mistral_fixtures/native_conversations/replay.rs
tests/mistral_fixtures/native_conversations/callbacks.rs
tests/openrouter_fixtures.rs       # one OpenRouter protocol target
tests/openrouter_fixtures/fixtures.rs
tests/openrouter_fixtures/chat_wire.rs
tests/openrouter_fixtures/chat_lifecycle.rs
tests/openrouter_fixtures/images.rs
tests/openrouter_fixtures/images/fixtures.rs
tests/openrouter_fixtures/images/wire.rs
tests/openrouter_fixtures/images/lifecycle.rs
tests/openrouter_fixtures/images/callbacks.rs
tests/azure_openai_fixtures.rs # one Azure OpenAI protocol target
tests/azure_openai_fixtures/fixtures.rs
tests/azure_openai_fixtures/chat_wire.rs
tests/azure_openai_fixtures/chat_lifecycle.rs
tests/azure_openai_fixtures/chat_callbacks.rs
tests/azure_openai_fixtures/responses.rs
tests/azure_openai_fixtures/responses/fixtures.rs
tests/azure_openai_fixtures/responses/wire.rs
tests/azure_openai_fixtures/responses/lifecycle.rs
tests/azure_openai_fixtures/responses/options.rs
tests/chat_pi_messages.rs       # one Pi Messages protocol target
tests/chat_pi_messages/fixtures.rs
tests/chat_pi_messages/gateway.rs
tests/chat_pi_messages/wire.rs
tests/chat_pi_messages/stream.rs
tests/chat_pi_messages/lifecycle.rs
tests/chat_pi_messages/callbacks.rs
tests/chat_pi_messages/node5.rs
tests/chat_pi_messages/node5/fixtures.rs
tests/chat_pi_messages/node5/replay.rs
tests/auth_contracts.rs          # one authentication contract target
tests/auth_contracts/errors.rs
tests/auth_contracts/fixtures.rs
tests/auth_contracts/resolution.rs
tests/auth_contracts/mutations.rs
tests/auth_contracts/adc_native.rs
tests/auth_contracts/adc_external.rs
tests/auth_contracts/adc_impersonation.rs
tests/auth_contracts/scoped.rs
tests/bedrock_fixtures.rs         # one Bedrock protocol target
tests/bedrock_fixtures/fixtures.rs
tests/bedrock_fixtures/wire.rs
tests/bedrock_fixtures/lifecycle.rs
tests/bedrock_fixtures/aws_credentials.rs
tests/bedrock_fixtures/aws_credentials/support.rs
tests/bedrock_fixtures/aws_credentials/chain.rs
tests/bedrock_fixtures/aws_credentials/role_sources.rs
tests/bedrock_fixtures/aws_credentials/metadata.rs
tests/bedrock_fixtures/aws_credentials/lifecycle.rs
tests/bedrock_fixtures/aws_sso.rs
tests/bedrock_fixtures/aws_sso/support.rs
tests/bedrock_fixtures/aws_sso/discovery.rs
tests/bedrock_fixtures/aws_sso/expiry.rs
tests/bedrock_fixtures/aws_sso/login.rs
tests/bedrock_fixtures/aws_sso/chain.rs
tests/bedrock_fixtures/builtin_routes.rs
tests/bedrock_fixtures/constrained_guard.rs
tests/bedrock_fixtures/converse_adapter.rs
tests/bedrock_fixtures/converse_adapter/support.rs
tests/bedrock_fixtures/converse_adapter/projection.rs
tests/bedrock_fixtures/converse_adapter/stream.rs
tests/bedrock_fixtures/converse_adapter/replay.rs
tests/bedrock_fixtures/converse_adapter/lifecycle.rs
tests/openai_chat_fixtures.rs       # one OpenAI Chat protocol target
tests/openai_chat_fixtures/fixtures.rs
tests/openai_chat_fixtures/wire.rs
tests/openai_chat_fixtures/lifecycle.rs
tests/openai_chat_fixtures/constrained_guard.rs
tests/openai_responses_fixtures.rs # one OpenAI Responses protocol target
tests/openai_responses_fixtures/fixtures.rs
tests/openai_responses_fixtures/wire.rs
tests/openai_responses_fixtures/constrained_guard.rs
tests/openai_responses_fixtures/lifecycle.rs
tests/vertex_fixtures.rs       # one Google Vertex protocol target
tests/vertex_fixtures/fixtures.rs
tests/vertex_fixtures/builtin_routes.rs
tests/vertex_fixtures/wire.rs
tests/vertex_fixtures/lifecycle.rs
tests/vertex_fixtures/adc_vertex.rs
tests/vertex_fixtures/adc_external.rs
tests/vertex_fixtures/adc_impersonation.rs
tests/vertex_fixtures/callbacks.rs
tests/vertex_fixtures/constrained_guard.rs
tests/vertex_fixtures/google_41.rs
tests/transcript_contract/provider_projection.rs
tests/replay_contract.rs           # one integration target
tests/replay_contract/fixtures.rs
tests/replay_contract/association.rs
tests/replay_contract/execution_effects.rs
tests/replay_contract/malformed_arguments.rs
tests/replay_contract/wire_ids.rs
tests/replay_contract/content_projection.rs
tests/replay_contract/object_wire.rs
tests/replay_contract/responses_grammar.rs
tests/openai_codex_responses.rs    # one integration target
tests/openai_codex_responses/wire.rs
tests/openai_codex_responses/constrained_sampling.rs
tests/openai_codex_responses/ws_path.rs
tests/openai_codex_responses/ws_continuation.rs
tests/openai_codex_responses/ws_branches.rs
tests/openai_codex_responses/ws_cancellation.rs
tests/openai_codex_responses/ws_resources.rs
tests/openai_codex_responses/ws_protocol_regressions.rs
tests/openai_codex_responses/replay.rs
tests/openai_codex_responses/deflate_audit.rs  # shared helper, no own assertions
tests/catalog_operations.rs        # one builtins catalog assembly target
tests/catalog_operations/fixtures.rs
tests/catalog_operations/publication.rs
tests/catalog_operations/dispatch.rs
tests/catalog_operations/pricing.rs
tests/provider_collection.rs        # one builtins assembly target
tests/provider_collection/fixtures.rs
tests/provider_collection/auth_resolution.rs
tests/provider_collection/dispatch.rs
tests/provider_collection/builtin_registration.rs
tests/provider_collection/builtin_auth.rs
tests/provider_collection/builtin_chat.rs
tests/provider_collection/builtin_operations.rs
tests/provider_collection/azure.rs
tests/provider_collection/opencode.rs
tests/provider_collection/phase17.rs
tests/provider_collection/registry_preflight_and_refresh.rs
tests/provider_collection/catalog_store_refresh_20_33.rs
tests/provider_collection/helper_retry_hints.rs
tests/provider_collection/selected_owner_management.rs
tests/oauth_models_store.rs        # Models store lifecycle target
tests/oauth_regressions.rs         # shared cross-provider regression target
tests/tls_contract.rs              # public TLS target index
tests/ws_upgrade.rs
tests/support/tls_fixtures.rs
tests/support/ws_peer.rs
tests/support/OWNERS.tsv
tests/support/oauth_fixtures.rs
tests/support/oauth_assembly.rs
tests/support/oauth_anthropic.rs
tests/support/oauth_codex.rs
tests/support/oauth_chatgpt.rs
tests/support/oauth_copilot.rs
tests/support/oauth_kimi.rs
tests/support/oauth_meta.rs
tests/support/oauth_xai.rs
tests/support/oauth_loopback.rs
tests/support/oauth_radius.rs
tests/support/oauth_store.rs
```

The crypto owner keeps only this test declaration:

```rust,ignore
#[cfg(test)]
mod tests;
```

Small private tests may remain inline. Crypto's admission, signing, worker and
JWT contracts justify separate topics. Its fixture constants remain in the
existing private, test-only `src/crypto/fixtures.rs`.

The [Rust Reference](https://doc.rust-lang.org/reference/items/modules.html#outlined-modules)
supports both `tests.rs` and `tests/mod.rs` and encourages named module files to
avoid many identically named `mod.rs` files. Here the existing WS/crypto
directory convention is retained; the production module tree is not renamed.

Cargo gives each discovered integration target a separate executable. Group
related cases as submodules of an existing contract target before adding another
executable; [Cargo documents the compilation/scheduling tradeoff](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#tests).
Shared helpers go under `tests/support`, rather than becoming root `tests/*.rs`
targets that run zero tests. Do not widen production visibility or copy
production source into integration tests to reach private mechanisms.

A root integration file explicitly declares its topic submodule, for example:

```rust,ignore
#[path = "transcript_contract/provider_projection.rs"]
mod provider_projection;
```

A single public Schema function can be tested by an integration target.
Conversely, several private modules working together remain crate-local
evidence. Directory organization communicates that access boundary; it does not
replace assertions about the actual behavior.

The Codex Responses target keeps one executable while exposing focused selectors
for request wire, constrained sampling, WebSocket acquisition and continuation,
recovery, cancellation, resource cleanup, and protocol regressions. Its
`deflate_audit.rs` module is a shared decoder helper, so changing it correctly
selects the whole target.

The provider collection target also keeps one executable because its assertions
cross the builtin composition root and several providers. Its topic modules own
the phase-17 execution state machine, registry preflight, catalog-store refresh,
retry hints, and selected-owner management. Provider-specific request checks
remain in the neighboring provider modules. The five topic files expose
qualified selectors, while changing a shared root helper or the builtin index
selects the complete target.

The Bedrock fixture target keeps one executable for AWS credential-source and
Converse protocol coverage. `wire.rs` owns the 28 parser, event-stream, model,
error and projection cases. `lifecycle.rs` retains 25 HTTP, callback, request-header
and prepared-call cases. `aws_credentials.rs` is the credential-source topic index.
Its `support.rs` owns shared source fixtures and helper contracts, while `chain.rs`
owns 15 environment/profile/process precedence cases, `role_sources.rs` owns 11
assume-role/MFA/web-identity cases, `metadata.rs` owns nine ECS/IMDS cases, and
the nested `lifecycle.rs` owns 11 Models, HTTP, cancellation and frozen-process
cases. `aws_sso.rs` is the SSO topic index. Its `support.rs` owns shared SSO,
login, refresh and DPoP fixtures, while `discovery.rs` owns nine source discovery
and federation cases, `expiry.rs` owns 11 expiry and passive-refresh cases,
`login.rs` owns eight login and DPoP cases, and `chain.rs` owns three precedence
and throttling cases. The SSO topics reuse the credential helpers through the
topic index. `fixtures.rs` owns shared requests,
event-stream bytes, auth and response helpers consumed by the root target and
nested topics. `builtin_routes.rs` owns eight builtin registration and local
endpoint cases, `constrained_guard.rs` owns two strict sampling cases, and
`converse_adapter.rs` is the adapter topic index. Its `projection.rs` owns 13
request and message projection cases, `stream.rs` owns six stream, stop-reason,
usage and strict-tool cases, `replay.rs` owns five persisted-history cases, and
`lifecycle.rs` owns four Models lifecycle and route-contract cases plus the
optional telemetry case. `support.rs` owns adapter-local helpers without its
own assertions. `converse_adapter` therefore owns 28 default cases and one
telemetry-tracing case. `aws_source_transport.rs` remains a shared local
transport helper. The nested selectors are `lifecycle::aws_credentials::chain::`,
`lifecycle::aws_credentials::role_sources::`,
`lifecycle::aws_credentials::metadata::`,
`lifecycle::aws_credentials::lifecycle::`,
`lifecycle::aws_sso::discovery::`, `lifecycle::aws_sso::expiry::`,
`lifecycle::aws_sso::login::`, `lifecycle::aws_sso::chain::`,
`converse_adapter::projection::`,
`converse_adapter::stream::`, `converse_adapter::replay::` and
`converse_adapter::lifecycle::`. Changing an adapter-local helper, the shared
fixture or root index selects the complete target.

The OpenAI Chat target keeps one executable with `wire.rs` owning parser,
request-body, projection and transport assertions, `lifecycle.rs` owning Models,
timing, callback and source-compatibility assertions, and `fixtures.rs` owning
shared response, request and local-server helpers. `constrained_guard.rs` keeps
the strict sampling admission cases as a focused wire submodule. Changing the
shared fixture source selects the complete target.

The OpenAI Responses fixture target also keeps one executable. `wire.rs` owns
payload and SSE mapping, terminal and usage projection, request routing,
acquisition sealing and transport cases. `constrained_guard.rs` owns grammar and
strict constrained-sampling admission and stream projection. `lifecycle.rs`
owns Models registration, callbacks, native options, reasoning replay, timing
and source-compatibility cases. `fixtures.rs` owns shared request, model,
response and reasoning builders. The existing `openai_responses_lifecycle`
target continues to own the separate HTTP and deferred submit/fetch/cancel
lifecycle contract. Changes to `fixtures.rs` or the root index select the
complete `openai_responses_fixtures` target.

The Vertex fixture target keeps one executable. `fixtures.rs` owns shared request,
stream, provider and local response helpers without assertions. `builtin_routes.rs` owns
builtin registration and cross-provider route cases. `wire.rs` owns endpoint, auth, catalog,
model resolution, request bodies, SSE, tool, error and usage projection. `lifecycle.rs` owns
local HTTP, timeout, cancellation, body limits and header routing. `adc_vertex.rs`,
`adc_external.rs` and `adc_impersonation.rs` own the three ADC credential-source contracts.
`callbacks.rs`, `constrained_guard.rs` and `google_41.rs` own callback transforms, strict
sampling admission and the Google 4.1 adapter contract. The package consumer remains
`provider-vertex`. Changing the shared fixture or root index selects the complete
`vertex_fixtures` target.

The Gemini fixture target keeps one executable. `fixtures.rs` owns shared provider, request,
stream and local stalled-server helpers without assertions. `wire.rs` owns provider/model
resolution, GenerateContent request bodies, SSE mapping, usage, errors and stop reasons.
`lifecycle.rs` owns HTTP auth classification, request enrichment, timeout, cancellation and
chunk-splitting behavior. `callbacks.rs` owns payload/raw/async observer contracts.
`constrained_guard.rs` owns strict sampling admission, and `google_40.rs` owns the Google 4.0
chat adapter, native thinking/tool choice, signed replay and telemetry cases. The package
consumer remains `provider-gemini`. Changing the shared fixture or root index selects the
complete `gemini_fixtures` target.

The Mistral fixture target keeps one executable. `fixtures.rs` owns shared provider, request,
stream and stalled-server helpers without assertions. `chat_wire.rs`, `chat_lifecycle.rs` and
`chat_callbacks.rs` own the shared OpenAI-compatible Chat request/SSE, cancellation and callback
contracts. `native_conversations.rs` is the native adapter topic index. Its `fixtures.rs` owns native
builders and mounts, while `wire.rs`, `lifecycle.rs`, `replay.rs` and `callbacks.rs` own native
Mistral options, SSE, timeout/conversion, replay/cache and callback behavior. The package
consumer remains `provider-mistral`. Changing shared fixtures or the root index selects the
complete `mistral_fixtures` target.

The OpenRouter fixture target keeps one executable. `fixtures.rs` owns shared Chat provider and
local peer helpers without assertions. `chat_wire.rs` owns OpenRouter-compatible model, request,
SSE, usage and endpoint cases, while `chat_lifecycle.rs` owns cancellation, compatibility and
custom registration lifecycle. `images.rs` is the Images topic index. Its `images/fixtures.rs`
owns image records, request builders and retry/timeout peers, `images/wire.rs` owns image request,
response, modality, usage and header projection, `images/lifecycle.rs` owns registration,
retry, timeout, admission and auth cancellation, and `images/callbacks.rs` owns callback,
observer and telemetry behavior. The package consumer remains `provider-openrouter`. Changing
shared fixtures or the root index selects the complete `openrouter_fixtures` target.

The Azure OpenAI fixture target keeps one executable. `fixtures.rs` owns shared Chat
provider, request, SSE and local-peer helpers without assertions. `chat_wire.rs`,
`chat_lifecycle.rs` and `chat_callbacks.rs` own Chat request/SSE, transport cancellation and
callback contracts. `responses.rs` is the native Responses topic index. Its `responses/fixtures.rs`
owns model, request, SSE and local-peer builders, while `wire.rs`, `lifecycle.rs` and `options.rs`
own endpoint and stream mapping, execution timing and Azure Responses options, projection and
replay behavior. The package consumer remains `provider-azure-openai`. Changing shared fixtures or
either topic index selects the complete `azure_openai_fixtures` target.

The Pi Messages fixture target keeps one executable. `fixtures.rs` owns shared model, request,
SSE, local-peer and projection helpers without assertions. `gateway.rs` owns Radius gateway and
catalog behavior. `wire.rs` owns endpoint, option whitelist, cache retention and context projection.
`stream.rs` owns SSE blocks, sparse indices, terminal markers and typed stream failures.
`lifecycle.rs` owns HTTP failures, cancellation, auth and model admission, simple bypass, wire
mismatch and Models completion. `callbacks.rs` owns callback ordering. `node5.rs` is the replay
topic index, with `node5/fixtures.rs` owning replay builders and `node5/replay.rs` owning producer
and reload regressions. The package consumer remains `provider-pi-messages`. Changing a shared
fixture or topic index selects the complete `chat_pi_messages` target.

The authentication contract target keeps one executable. `errors.rs` owns typed credential
failures, object-safe auth contracts and redaction assertions. `fixtures.rs` owns the shared
resolver/store contexts, OAuth doubles, stored API-key constructor and synthetic service-account
key. The key is shared by native and impersonated ADC exchanges. `resolution.rs` owns lazy source
selection, explicit overrides, store precedence, logout and descriptor resolution. `mutations.rs`
owns serialized credential mutations, barriers, receipts and passive refresh handoff.
`adc_native.rs`, `adc_external.rs` and `adc_impersonation.rs` each own one ADC source family,
including their local transport fixtures and lifecycle assertions. `scoped.rs` owns credential
snapshot and redaction behavior. The external `oauth-store` and `cloud-auth` package consumers
exercise the public auth entry points. Changing a shared fixture or the root index selects the
complete `auth_contracts` target.

The `provider_collection` index assembles one `builtins` executable. `auth_resolution.rs`
owns descriptor and probe behavior, `dispatch.rs` owns admission and stream outcomes, and
`registry_preflight_and_refresh.rs` owns registry lookup, preflight and in-memory refresh.
The built-in assembly topics own registration and catalog projection, credential binding,
chat routes and non-chat operations. `azure.rs` and `opencode.rs` own the corresponding
provider-specific assembly paths. `fixtures.rs` owns shared requests, scripted responses, auth,
refresh and projection helpers. A topic change selects its module. A shared fixture or index
change selects the complete target. The archived README examples exercise `register_builtin`
from an external consumer in two feature profiles. [README example runner](../../../scripts/pi-sdk-conformance/run_readme_examples.py)

The `catalog_operations` target exercises operation-qualified catalogs through the public SDK
with `builtins`. `publication.rs` owns decode, refresh, atomic publication and route metadata
restored from the store. `dispatch.rs` owns operation selection, availability checks and actual
Chat, Images and Classifier calls. `pricing.rs` owns missing prices, exact tier thresholds and
fallback prices through decode, store reopen and cost calculation. `fixtures.rs` keeps the
shared mixed-operation records, Anthropic records, auth double and request builders. Changing a
topic selects its module. Changing the index or shared fixtures selects the full target.
The external `catalog-bundled` consumer checks the embedded catalog and feature graph from a
Cargo archive. The `catalog_operations` integration target owns dynamic publication, dispatch
and persistence and can also run from the extracted archive. [Catalog package runner](../../../scripts/pi-sdk-conformance/run_catalog_bundled_package.py)

The core `execution_lifecycle` target exercises the public execution contract with
`--no-default-features`. Its topic files under `tests/execution_lifecycle/` select the
following filters.

| Topic | Filter | Assertions owned here |
| --- | --- | --- |
| `ordinary.rs` | `ordinary::` | Ordinary and raw started-stream progress, call setup, event order, and typed outcomes |
| `deferred.rs` | `deferred::` | Deferred fetch readiness, result timing, and fetch-owned source and poll settlement |
| `terminal_release.rs` | `terminal_release::` | EOF and source release for started, ordinary and prepared calls, including abort joins and attempt seals |

`fixtures.rs` owns the controlled channels, held tails and chat setup shared within this
target. Changing it or the root index selects all three topics. Use the selector on a topic
path, or pass its filter to `cargo test --locked --offline -p opi-ai --no-default-features --test execution_lifecycle`.

The external minimal consumer runs `core_profile` from actual archives for core ordinary
calls and Faux Deferred fetch. The extracted `execution_lifecycle` target provides focused
proof of abort, tail release and cached-outcome timing. [Minimal package runner](../../../scripts/pi-sdk-conformance/run_minimal_package.py)

The `telemetry_operations` target owns frozen context across public SDK operations.
It requires `builtins,telemetry-tracing` and uses controlled local peers for wire checks.
The topic files under `tests/telemetry_operations/` each have a selector.

| Topic | Filter | Assertions owned here |
| --- | --- | --- |
| `chat.rs` | `chat::` | Native, simple and direct Chat, task progress, poll, Drop and no-op context |
| `prepared.rs` | `prepared::` | Prepared Anthropic calls, strict-tool admission, retry and callback parentage |
| `images.rs` | `images::` | OpenRouter Images request and callbacks, result equivalence and wire isolation |
| `classifier.rs` | `classifier::` | System One classify and classify-native callbacks, result equivalence and wire isolation |
| `deferred.rs` | `deferred::` | Submit, fetch, cancel, abandoned fetch and serialized handles |

`fixtures.rs` owns the recording subscriber, ambient polling, redaction canaries,
and the stream used by Chat and Deferred. The shared fixture and root index select the
complete target. Operation-specific replies and input builders stay in each topic.
Changes to `src/api/system_one.rs` select `classifier_calls_freeze_context` in this target.

The external README consumer verifies packaged builtins with and without tracing.
The extracted `telemetry_operations` target verifies the frozen dispatch and parent,
callback scope and wire isolation. [README package runner](../../../scripts/pi-sdk-conformance/run_readme_examples.py)

The `request_enrichment` target owns request options and their effect on provider wire
behavior. It uses `builtins` and local HTTP peers. Each topic under `tests/request_enrichment/`
can be selected by its module filter.

| Topic | Filter | Assertions owned here |
| --- | --- | --- |
| `scalars_and_timeout.rs` | `scalars_and_timeout::` | Request defaults, explicit fields, sampling, and terminal timeouts |
| `protected_headers.rs` | `protected_headers::` | Header admission, transformation, and protection of auth and route identity |
| `session_affinity.rs` | `session_affinity::` | Session headers, prompt cache keys, retention, and caller overrides |
| `callbacks.rs` | `callbacks::` | Payload shape and budget checks, observer order, and frozen retry bytes |

`fixtures.rs` holds shared requests, captured HTTP readers, and the projection assertion
for accepted failures. Changing it or the root index selects all four topics. The
Responses affinity matrix stays nested under `session_affinity`.

The external `protocol-adapters` consumer verifies public exports and the feature graph
of the actual Cargo archive. The extracted `request_enrichment` target verifies the four
topics against local peers using `builtins`. [Protocol package runner](../../../scripts/pi-sdk-conformance/run_protocol_adapters_package.py)

The `node4_preparation` target covers Models preparation using `builtins` and
local fixtures. Each topic under `tests/node4_preparation/` has a module filter.

| Topic | Filter | Assertions owned here |
| --- | --- | --- |
| `call_limits.rs` | `call_limits::` | Native and simple limits on Chat and Responses requests, including defaults and context budget |
| `catalog_restore.rs` | `catalog_restore::` | Models store reopen, publication ownership, refresh failures and authoritative removal |
| `projection.rs` | `projection::` | Projection across call entries, retry ownership and failure timing around auth, dispatch, EOF and cancellation |

`fixtures.rs` holds the test target's shared requests, Chat SSE and captured body reader.
Changing it or the root index selects the full target. `catalog_restore.rs` imports the
cross-target `tests/support/catalog_store.rs` helper directly. Its path selects all full
consumer targets recorded in [the support inventory](support/OWNERS.tsv).

The external `catalog-bundled` consumer verifies embedded catalog data and the feature
graph of the actual archive. The extracted `node4_preparation` target exercises the three
behavioral topics with `builtins`. [Catalog package runner](../../../scripts/pi-sdk-conformance/run_catalog_bundled_package.py)

`http_acceptance` is the HTTP transport public-contract target. It uses `builtins`
and drives real loopback TCP or TLS peers through `HttpClient` and Models Chat.
Pass a topic path under `tests/http_acceptance/` to the selector or use its libtest filter.

| Topic | Filter | Assertions owned here |
| --- | --- | --- |
| `response_parsing.rs` | `response_parsing::` | HTTP/1 response-head sizes and counts, chunk framing, trailers and EOF acceptance |
| `redirects_and_disconnects.rs` | `redirects_and_disconnects::` | Same-origin and cross-origin redirects, protected headers, send budgets and failures around response acquisition |
| `https_proxy.rs` | `https_proxy::` | Outer TLS, CONNECT, inner TLS and scoped `NO_PROXY` bypass |

`fixtures.rs` holds the shared request reader, scripted peer, request capture and
Models Chat driver. The root index and this fixture select the full target.
Response boundary bytes, redirect scripts and TLS/CONNECT setup stay with the
topic that uses them. Certificates and keys come from the shared
`tests/support/tls_fixtures.rs`, whose path selects all six consumer targets.
[Support inventory](support/OWNERS.tsv)

The external `consumer-http` exercises public Models and HTTP transport calls,
including HTTP proxy routing and bypass, in ordinary and caller-union graphs.
Run the extracted `http_acceptance` target to verify the detailed parser
boundaries and HTTPS tunnels. [HTTP package runner](../../../scripts/pi-sdk-conformance/package_slice.py)

`tls_contract` is the TLS transport public-contract target. It uses `builtins` and
real loopback TLS peers. Select a topic path under `tests/tls_contract/` or pass its
filter to `cargo test --locked --offline -p opi-ai --no-default-features --features builtins --test tls_contract`.

| Topic | Filter | Assertions owned here |
| --- | --- | --- |
| `negotiation_and_trust.rs` | `negotiation_and_trust::` | TLS versions, H1/H2 ALPN, key exchange, trust roots, hostname and expiry |
| `mtls.rs` | `mtls::` | Client identity admission, verification and per-connection isolation |
| `cancellation.rs` | `cancellation::` | Handshake and response cancellation, EOF, transport release and the send budget after H2 stream refusal |
| `provider_isolation.rs` | `provider_isolation::` | Process-global rustls state and instance TLS under host providers in fresh subprocesses |

The target-local `fixtures.rs` owns the loopback peer and HttpClient-to-Models Chat
driver. It and the root index select the full target. The shared `tests/support/tls_fixtures.rs`
owns certificate and key material and selects all six consumer targets, including
`ws_upgrade`. [Support inventory](support/OWNERS.tsv)

The external `crypto` package consumer uses its own test peer and trust material to
verify public TLS/Models requests in ordinary and caller-union graphs. The extracted
`tls_contract` target verifies all four topics, including cancellation and cleanup.
The `ws` package family covers the shared
certificates at the Upgrade/WSS boundary. [Package runner](../../../scripts/pi-sdk-conformance/package_slice.py)

The `models` owner runs `external_sdk` with `builtins` and controlled peers. Pass
a topic path under `tests/external_sdk/` to the selector or use its libtest filter.

| Topic | Filter | Assertions owned here |
| --- | --- | --- |
| `chat_matrix.rs` | `chat_matrix::` | Ten Chat routes on one Models instance with builtin registration, injected auth and real adapters to local peers |
| `classifier_and_images.rs` | `classifier_and_images::` | Typed dispatch to System One, Workers AI, OpenAI Decisions and llama.cpp classifiers, plus OpenRouter image generation |
| `models_contract.rs` | `models_contract::` | Option admission, callbacks, timing, tool continuation, instance isolation, constructors, zero-I/O reads, lawful large collections and explicit Deferred fetch |

`fixtures.rs` owns the recording auth context, request builders and SSE replies
shared by Chat and Models. Topic-specific peers, replies and assertions live with
their topic. The root index and shared fixture paths select the full target. The
production selectors can still run `assembled_matrix_drives` and
`classifier_and_image_apis_answer_typed_through_one_instance` individually.

The external `consumer-readme` executes the three examples shipped in the Cargo
archive in both caller feature profiles. The extracted `external_sdk` target
verifies the complete assembled matrix and public contracts above. [README package runner](../../../scripts/pi-sdk-conformance/run_readme_examples.py)

`chat_capture_replay` is the `replay` assembly target for real producer
output and continuation requests. It uses `builtins` and controlled local
peers. Pass a topic path under `tests/chat_capture_replay/` to the selector
or use its libtest filter.

| Topic | Filter | Assertions owned here |
| --- | --- | --- |
| `producer_capture.rs` | `producer_capture::` | Provider and wire identity, thinking, usage and terminal facts from five builtin producer cases |
| `persistence_restore.rs` | `persistence_restore::` | Exact AssistantMessage and canonical JSON round trip for those five producer cases |
| `target_projection.rs` | `target_projection::` | Thirteen complete capture, restore and continuation cases, including Chat grammar, Azure, Responses, Codex and Anthropic |

The capture and restore matrices each finish after one source request per
case. `fixtures.rs` owns their shared builtin case list, route setup, SSE
peers, request builders and JSON reload helper. That file and the root index
select the full target. Projection-specific replies and assertions stay
with their scenarios. The two nested projection files select
`target_projection::replay_boundaries::` and
`target_projection::persisted_producers::` independently.

The external `consumer-readme` owns the packaged examples, including
capturing a public tool call and sending its result in a continuation. The
extracted `chat_capture_replay` target owns JSON restoration and detailed
wire projection across the builtin, Codex and Anthropic paths.
[README package runner](../../../scripts/pi-sdk-conformance/run_readme_examples.py)

`replay_contract` is the `replay` wire target and runs with `builtins`. Pass a
topic path under `tests/replay_contract/` to the selector or use the filter.

| Topic | Filter | Assertions owned here |
| --- | --- | --- |
| `association.rs` | `association::` | Tool-call and result pairing, validation before dispatch, and terminal failure |
| `execution_effects.rs` | `execution_effects::` | Missing results, failed turns, unknown effects and persisted non-execution facts |
| `malformed_arguments.rs` | `malformed_arguments::` | String and object argument projection and sanitized failure context |
| `wire_ids.rs` | `wire_ids::` | Responses item IDs and deterministic ID pairing and collision handling |
| `content_projection.rs` | `content_projection::` | Thinking identity, visible Chat text and tool-result images |
| `object_wire.rs` | `object_wire::` | Gemini and Bedrock object payloads with trusted facts and preserved pairs |
| `responses_grammar.rs` | `grammar_replay_tests::` | Responses custom call and output items and the `ctc_` item ID |

The root and `fixtures.rs` select the whole target. The shared fixtures
provide histories and requests for these tests. `chat_capture_replay` owns
producer capture and restoration. The external `consumer-readme` uses the
public API to replay a recorded call and its linked result on a second
request. The extracted SDK runs the full `replay_contract` target.
[README package runner](../../../scripts/pi-sdk-conformance/run_readme_examples.py)

The `retry` owner runs the `retry_backoff` public-contract target with `builtins`.
Pass a topic path under `tests/retry_backoff/` to the selector or use its libtest
filter. The root index selects the full target.

| Topic | Filter | Assertions owned here |
| --- | --- | --- |
| `response_headers.rs` | `response_headers::` | Retry-After seconds and HTTP dates, rate-limit reset, precedence and malformed inputs |
| `backoff_policy.rs` | `backoff_policy::` | Exponential delays, caps, attempt limits and retryability of provider errors |
| `acquisition.rs` | `acquisition::` | Typed acquisition errors, real OpenAI Chat send budgets, frozen payloads, cancellation and sealing at response acceptance or unknown dispatch |
| `smithy.rs` | `smithy::` | AWS retry modes and configuration precedence, classification, jitter, hint bounds, quotas and adaptive limiting |

The header topic owns its date formatter. Acquisition owns the loopback SSE
scripts and scripted provider setup and uses the existing `opi_ai::test_support`
mock provider. Smithy owns its deterministic clock, inputs and error fixtures.

The external `consumer-http` verifies typed default acquisition failure and
single-send EOF after response acceptance in two feature graphs. The extracted
`retry_backoff` target verifies all four topics, including admitted retries and
Smithy policy. `bedrock_retry` owns Bedrock Converse-specific retry and rebuild
behavior. `whole_call_retry` owns explicit retries of completed calls.
[HTTP package runner](../../../scripts/pi-sdk-conformance/package_slice.py)

The `retry` owner runs `whole_call_retry` with `builtins` at the public-contract
boundary. Select a topic by its path under `tests/whole_call_retry/` or by its
libtest filter.

| Topic | Filter | Assertions owned here |
| --- | --- | --- |
| `classification.rs` | `classification::` | Retry decisions on canonical assistant messages, quota and context vetoes, and transient diagnostic matching |
| `call_and_callbacks.rs` | `call_and_callbacks::` | New logical calls, retry budget, callbacks, terminal producer results and wire sends across Anthropic and OpenAI adapters |
| `backoff_and_cancellation.rs` | `backoff_and_cancellation::` | Delay caps, caller cancellation during backoff, and connection release when an OpenAI-family 429 body is pending |

The root and `fixtures.rs` select the full target. The fixture holds canonical
messages, the request builder, retry policy and finished-callback sink shared
within this target. Anthropic SSE and OpenAI quota response setup stay in their
own wire modules. Those files select `call_and_callbacks::frozen_target_wire::`
and `call_and_callbacks::quota_transport::` independently.

The OpenAI Responses failure fixture `tests/data/responses-failed.json` is
shared by `call_and_callbacks::responses_failed_sse_reaches_the_explicit_whole_call_policy`,
`openai_responses_fixtures`'s `wire::responses_sse_error_preserves_the_callers_recovery_decision`,
and the Responses differential probes. Its data path asks the selector for broader checks.

The external `consumer-http` owns the public OpenAI Chat acquisition and
single-send EOF boundary. The extracted `whole_call_retry` target verifies
explicit whole-call retry, including callback timing, across the three topics.
[HTTP package runner](../../../scripts/pi-sdk-conformance/package_slice.py)

The `provider` owner runs the `provider_lifecycle` wire target with `builtins`.
Select a topic by its path under `tests/provider_lifecycle/` or by its libtest
filter.

| Topic | Filter | Assertions owned here |
| --- | --- | --- |
| `request_and_errors.rs` | `request_and_errors::` | An actual Anthropic POST with auth headers and JSON fields, typed 401/429/500 mapping, retry hints and terminal failures |
| `sse_decoding.rs` | `sse_decoding::` | Anthropic text and tool events, missing terminal, byte fragmentation, UTF-8 replacement and final tool arguments |
| `cancellation_and_release.rs` | `cancellation_and_release::` | Started-call completion after observer drop, cancellation at wait points and Gemini/Bedrock connection release before and during a response |

The request builder and successful text SSE fixture are shared within this
target in `fixtures.rs`. The root and fixture select the full target. The typed
failure assertion helper, byte-chunked SSE peer and staged connection peers stay
with the topics that use them.

The external `consumer-readme` executes Anthropic-backed public examples,
including POST/auth checks, starting the call before consumption, and tool
continuation. The extracted `provider_lifecycle` target owns typed failures,
decoding edges and socket release. [README package runner](../../../scripts/pi-sdk-conformance/run_readme_examples.py)

The `provider/operation` owner runs `deferred_contract` at the public-contract
boundary with no default features. Select a file under `tests/deferred_contract/`
or use its libtest filter.

| Topic | Filter | Assertions owned here |
| --- | --- | --- |
| `handle_validation.rs` | `handle_validation::` | Exact registration, handle JSON, identity and expiry, and fetch/cancel rejection before auth or dispatch |
| `submission_and_fetch.rs` | `submission_and_fetch::` | Ordinary generation, single-check `wait0` fetch, unknown jobs, and typed auth failures with canonical retry inputs |
| `cancellation.rs` | `cancellation::` | Local abort, best-effort remote cancel and refetch, and cancellation before or during auth |
| `reopen.rs` | `reopen::` | Caller-owned checkpoint and clock, fetch after reopening, no resubmission, and missing-model or cancel failure reporting |
| `telemetry.rs` | `telemetry::` | Explicit carrier forwarding and absence of implicit capture in the minimum SDK |

The root and `fixtures.rs` select the whole target. The target shares the
scripted route and request setup internally. The checkpoint and scheduler stay
in `reopen.rs`. The separately owned `tests/support/pending_auth.rs` selects
`classifier_contract`, `deferred_contract` and `openrouter_fixtures` in full.

The external `opi-minimum-consumer` verifies public Faux submission and
explicit pending-to-ready fetch in the core and core-tracing profiles. Run
all five topics from the extracted SDK archive for the rest of the contract.
To replay the route-cancel observations, select
`handle_validation::cancel_checks_all_deferred_route_fields_before_auth_or_dispatch`
with `--exact --nocapture`. The lifecycle differential uses its own executable
and upstream captures. [Minimal package runner](../../../scripts/pi-sdk-conformance/run_minimal_package.py)

The `utilities` owner runs `public_utilities` with `builtins`. Each topic under
`tests/public_utilities/` has a file-path selector and a libtest filter.

| Topic | Filter | Assertions owned here |
| --- | --- | --- |
| `rendering.rs` | `rendering::` | Visible text extraction, system section chronology, and read-only projections |
| `tool_arguments.rs` | `tool_arguments::` | Streamed JSON preview, authoritative final repair, and portable string enum schemas |
| `context_overflow.rs` | `context_overflow::` | Overflow and recoverable length based on diagnostics, stop reason, usage and context window |
| `provider_diagnostics.rs` | `provider_diagnostics::` | Typed provider codes and redacted error summaries |
| `identity.rs` | `identity::` | Short hash fixtures and UUIDv7 format, range, sequence and clock rollback |
| `provider_configuration.rs` | `provider_configuration::` | Header precedence and deletion, and scoped environment fallback |
| `cancellation.rs` | `cancellation::` | Cancellable timers, dropped futures and one-way token propagation |
| `grammar_property_names.rs` | `grammar_property_names::` | Literal keys in grammar resolution and the Responses request builder |

Each topic owns its fixtures. The environment lock stays with the provider
configuration cases. The root selects the whole target, and the existing
`grammar_property_names::*` names support exact selection after the move.

The external `opi-protocol-adapters-consumer` exercises canonical header and
environment exports in an adapter-only feature graph. Run `public_utilities`
from the extracted SDK archive with `builtins` for the complete assertions.
The loopback Chat argument-preview differential uses `chat_argument_preview`
and its own fixture. [Protocol adapters package runner](../../../scripts/pi-sdk-conformance/run_protocol_adapters_package.py)

The `schema` owner runs `schema_engine` with no default features. Select files
under `tests/schema_engine/` or pass the corresponding libtest filter.

| Topic | Filter | Assertions owned here |
| --- | --- | --- |
| `corpus.rs` | `corpus::` | All 64 frozen differential inputs and approved outcomes |
| `value_keywords.rs` | `value_keywords::` | Value guards, scalar constraints, and JavaScript equality and number semantics |
| `container_keywords.rs` | `container_keywords::` | Array and object cardinality, items, properties and dependencies |
| `applicators.rs` | `applicators::` | Branch selection and propagation of evaluated properties and items |
| `reference_resolution.rs` | `reference_resolution::` | Pointers, anchors, resource identity, enumeration and evaluation recursion |
| `admission.rs` | `admission::` | Compile-time diagnostics and resource traversal, including recursive frames |
| `stack_safety.rs` | `stack_safety::` | Deep compile, check, reference search and release on a 2 MiB child worker stack |

The root and `fixtures.rs` select the whole target. The shared helpers call the
public compile/check API. `corpus.rs` consumes `tests/data/schema-cases.jsonl`
and `tests/data/schema-expected.json`. For data edits, use `corpus::` alongside
the broader-check guidance from the selector. The depth tests require the
module-qualified child case to execute and report completion.

The external `opi-sdk-conformance-consumer` runs 73 named public Schema checks
in each of its two feature profiles. The extracted SDK archive can also run
the complete `schema_engine` integration target. `schema_public` owns
tool-argument normalization and public error paths.
[Schema package runner](../../../scripts/pi-sdk-conformance/run_early_package_proof.py)

The `models` owner runs `custom_provider_registration` with no default
features. Select files under `tests/custom_provider_registration/` or use
their module names as libtest filters.

| Topic | Filter | Assertions owned here |
| --- | --- | --- |
| `registry_providers.rs` | `registry_providers::` | Provider registration, replacement, lookup and mock streaming |
| `registry_models.rs` | `registry_models::` | Override validation, precedence, capabilities and deterministic listing |
| `chat_registration.rs` | `chat_registration::` | Exact API IDs, catalog visibility and owned model lookup |
| `chat_dispatch.rs` | `chat_dispatch::` | Explicit records, native and simple options, and direct auth |
| `chat_lifecycle.rs` | `chat_lifecycle::` | Frozen bindings, execution before consumption and owned calls |
| `chat_admission.rs` | `chat_admission::` | Capability, API ID, options type, provider ownership and cancellation checks before auth |

The root and its `fixtures.rs` select the whole target. The fixtures provide
registry mocks, typed Chat adapters and an observable auth resolver for this
target. The external `opi-minimum-consumer` copies this test tree from the
actual SDK archive and runs it in the core and tracing profiles. The extracted
SDK can also run the integration target directly.
[Minimal package runner](../../../scripts/pi-sdk-conformance/run_minimal_package.py)

The `types/model` owner runs `model_wire_metadata` with `builtins`. Select a
file under `tests/model_wire_metadata/` or use its libtest filter.

| Topic | Filter | Assertions owned here |
| --- | --- | --- |
| `model_contract.rs` | `model_contract::` | Wire names, model validation, routing JSON, eight builtin catalogs and configured Azure |
| `thinking_admission.rs` | `thinking_admission::` | Provider entry points reject unsupported thinking before HTTP and preserve stored metadata |
| `sampling_parameters.rs` | `sampling_parameters::` | Presence, shallow merge and model, level and request precedence |
| `sampling_wire.rs` | `sampling_wire::` | Chat, Responses, Azure and Anthropic body projection and payload callbacks |
| `pricing_usage.rs` | `pricing_usage::` | Valid rates, strict tier thresholds and service-tier usage roundtrip |
| `anthropic_complete_metadata.rs` | `anthropic_complete_metadata::` | Defaults on a complete Anthropic catalog record |
| `anthropic_response_cost.rs` | `anthropic_response_cost::` | Fallback declaration and attribution, cache tiers and unknown versus zero cost |

The root and `fixtures.rs` select the whole target. Request and sampling-data
builders are local to this target. The external catalog-bundled consumer
reparses the archive's embedded resources with `catalog-bundled` alone.
Run the extracted SDK's `model_wire_metadata` target with `builtins` for the
complete metadata, loopback request and pricing tests.
[Catalog package runner](../../../scripts/pi-sdk-conformance/run_catalog_bundled_package.py)

Use the existing [shared Cargo cache](../../../scripts/opi-cargo-cache.py), with
incremental compilation enabled, local fixtures/peers and no live credentials:

```powershell
cargo test --locked --offline -p opi-ai --lib crypto::tests::
cargo test --locked --offline -p opi-ai --lib crypto::tests::worker::
cargo test --locked --offline -p opi-ai --test transcript_contract --test replay_contract --test openai_codex_responses
cargo test --locked --offline -p opi-ai --test http_acceptance --test tls_contract --test ws_upgrade
```

Use the WS three-graph package check to exercise private protocol tests, public
Upgrade tests and the external Models → Codex → Upgrade → codec call. The crypto
family tracks production JWT/ADC caller coverage separately.
[SDK package checks](../../../scripts/pi-sdk-conformance/README.md)
