# opi-ai testing and acceptance plan

[中文](opi-ai-test-acceptance-plan.zh.md)

Execution evidence: [2026-10-21 results](opi-ai-test-acceptance-results.md).

Follow-up: [evidence closure and test layout](opi-ai-test-closure-results.md).
The gap table records the initial inspection; the follow-up records subsequent
closure without admitting new production callers. Current directory conventions
are documented in [test organization](../../crates/opi-ai/tests/README.md).

## Scope and decisions

Keep private mechanism tests beside their implementation, and accept caller
behavior through existing public interfaces. An acceptance unit can span several
Rust modules; it does not require a new crate, trait, feature or test binary.

This is a non-normative organization proposal for `opi-ai`, dated 2026-10-21.
The inspected baseline is HEAD `7cb5f0c7b3f3db277bc4e4672b08b324dc26dfbf`
plus the current working tree. In particular, `tests/ws_upgrade.rs` and
`tests/support/ws_peer.rs` are existing untracked inputs, while the WS tests,
crate READMEs and package harness already have local edits. Coverage below means
test code exists, not that it passed a fresh run. Recheck these inputs before
execution.

The governing contracts remain [PRIN-002 and PRIN-005](../opi-spec.md#31-minimal-and-deep),
[SDK direction](../opi-spec.md#32-alignment-with-pi), and
[assembled-product acceptance and extraction](../opi-spec.md#42-reference-product-ownership).
This proposal does not admit implementation, change registered delivery gates,
write the implementation ledger, or authorize package extraction or publication.

## Test layers and completion criteria

| Layer | Test surface | Completion evidence |
| --- | --- | --- |
| Private mechanism | Crate-local state, parsers, algorithms, scheduling and resource ownership | Deterministic success, invalid-input and state-transition assertions; retained fixtures/oracles; actual executed tests |
| Public module | Supported functions and values such as Schema, transcript and replay | Observable results, typed failures, input immutability and declared semantic differences |
| Public combination | `Models`/prepared calls, real adapters, `HttpClient`, local peers and concrete stores | Correct wire bytes and send counts, source selection, failure stage, terminal outcome, cancellation and owned cleanup |
| SDK package | Actual `opi-ai.crate`, extracted tests and separate external public consumers | Package/lock/toolchain identities, dependency features, native provenance and scenario-specific execution records |
| Workspace and product | Cross-crate callers and the assembled Reference Product | Applicable workspace gates plus the owning registered end-to-end acceptance cases |

Crate-local tests and extracted private tests cannot substitute for public
consumers. Public Upgrade success cannot substitute for a production codec
caller. A Cargo filter still builds the containing crate and its dependencies;
it is not an independent build or release boundary.

For each selected acceptance scope, record the exact contract, command, source
state, selected test names/counts, results and remaining exclusions. Exit zero
with zero selected tests is insufficient. Failed/ignored cases and missing
platform evidence remain explicit. Reuse unchanged evidence only when its
source, fixture, feature, toolchain and platform applicability is established.

## Module ownership and existing coverage

Command group names refer to the commands below. Keep the existing test layout;
logical grouping does not require moving or duplicating tests.

| Acceptance unit | Keep private | Public acceptance and existing coverage |
| --- | --- | --- |
| WS | Frame/PMD/Close/IO state, partial progress, cancellation, backpressure, compression oracles and internal Upgrade-to-Driver combinations in [src/ws/tests](../../crates/opi-ai/src/ws/tests/mod.rs) | **Transport**: [ws_upgrade](../../crates/opi-ai/tests/ws_upgrade.rs) exercises strict 101, negotiation, WSS/H1, trust, mTLS, CONNECT and detached raw bytes. The public bridge does not start the private codec. Keep the three declared feature graphs. |
| Crypto | Key admission, signing, independent verification, JWT vectors, worker cancellation and key lifetime in [crypto tests](../../crates/opi-ai/src/crypto/tests/mod.rs) | **Transport**: TLS identity admission and mTLS consume the key-admission owner. TLS uses rustls for handshake signing; it does not exercise `WorkerSigning`. A real signing caller must later supply its own public acceptance. |
| TLS | Frozen rustls configuration, explicit provider and root bundle stay private; no test-only verifier or config export | **Transport**: [tls_contract](../../crates/opi-ai/tests/tls_contract.rs) uses public TLS options through `HttpClient` and Models/Chat. Existing cases cover TLS 1.2/1.3, H1/H2, supported identities, isolation, typed pre-I/O rejection, trust/hostname/expiry/EKU/KX negatives, cancellation, partial usage/EOF and global-provider independence. No `--lib tls::` battery is established. |
| HTTP and retry | Acquisition leases/readiness/seals in [retry](../../crates/opi-ai/src/retry.rs), endpoint construction and internal transport details | **Transport**: [http_acceptance](../../crates/opi-ai/tests/http_acceptance.rs), [retry_backoff](../../crates/opi-ai/tests/retry_backoff.rs), [proxy_support](../../crates/opi-ai/tests/proxy_support.rs) and [connection_pooling](../../crates/opi-ai/tests/connection_pooling.rs). Existing combinations assert redirects, protected headers, proxy layers, precise sends, frozen auth/payload, body failures and no retry after handoff. Pool configuration/Arc identity is separate from socket reuse. |
| Schema | IR, evaluation stack, recursion proofs, formats, Number/URL/IDNA/hash helpers | **Schema**: [schema_engine](../../crates/opi-ai/tests/schema_engine.rs) and [schema_public](../../crates/opi-ai/tests/schema_public.rs) exercise `compile/check/validate_tool_arguments/validate_tool_call`, frozen corpora, no-progress failures, normalization, final validation against the original schema, immutable inputs and redacted reports. Tool declaration/provider projection remains part of **Canonical** and **Adapters**. |
| Credential store and mutation | Queue scheduling and bookkeeping remain implementation details; do not expose them to create a test seam | **Credentials**: [auth_contracts](../../crates/opi-ai/tests/auth_contracts.rs) exercises public store/queue/receipt/barrier contracts, per-provider FIFO, runtime handoff, synchronous admission, waiter cancellation, shared-store cutoff and unknown/late outcomes. These are integration tests, not an established `--lib mutation` group. Concrete persistent-store ownership requires the cross-crate checks below. |
| Models and auth lifecycle | Auth-source parsing, secret formatting, binding and lease implementation | **Credentials**: [provider_collection](../../crates/opi-ai/tests/provider_collection.rs), [custom_provider_registration](../../crates/opi-ai/tests/custom_provider_registration.rs), [per_request_auth](../../crates/opi-ai/tests/per_request_auth.rs), `auth_contracts` and `oauth_wire_shape`. Existing cases cover precedence/no fallback, resolve once, frozen retries, lazy dispatch, one active attempt, cancellation, typed call entries and old bindings surviving provider replacement. |
| Transcript | Ordered section patching, tool deltas, collapse/rendering helpers | **Canonical**: [transcript_contract](../../crates/opi-ai/tests/transcript_contract.rs) covers normalized chronology, serde admission, declarations, projection and canonical immutability. `adapters_consume_only_the_normalized_transcript` specifically exercises Anthropic; its plural name does not establish all-adapter conformance. |
| Replay | Batch association, loss facts, identity/thinking rules and ID legalization | **Canonical**: [replay_contract](../../crates/opi-ai/tests/replay_contract.rs) checks real request bodies, missing results with unknown effects, strict failures, malformed arguments and bijective IDs. `association_matrix_through_real_adapters` exercises Anthropic and OpenAI Chat. Tool-result wire/leak tests add coverage, but do not by themselves establish every wire/capability combination. |
| Context, usage and cost | Approximation/arithmetic, saturation and pricing calculations need no new public seam | **Accounting**: [context_budget](../../crates/opi-ai/tests/context_budget.rs), [usage_cost](../../crates/opi-ai/tests/usage_cost.rs) and [stream_events](../../crates/opi-ai/tests/stream_events.rs). Combine with `provider_collection` to prove current-context output/thinking clamping; arithmetic alone does not prove preparation consumes it. |
| Registry, metadata and catalog | Model-spec parsing and metadata validation can be checked locally | **Catalog**: [registry](../../crates/opi-ai/tests/registry.rs), [model_wire_metadata](../../crates/opi-ai/tests/model_wire_metadata.rs), migration tests and [api_mapped_provider](../../crates/opi-ai/tests/api_mapped_provider.rs). Together with `provider_collection`, existing tests cover replacement, wire/compat validation, atomic refresh/rollback and dispatch across three real wire adapters. Refresh is currently a caller-owned substrate, not a coding-agent production refresh feature. |
| Bedrock event stream | CRC, frame bounds, retained incomplete data and every-byte split tests in [event_stream](../../crates/opi-ai/src/bedrock/event_stream.rs) | **Bedrock**: [bedrock_fixtures](../../crates/opi-ai/tests/bedrock_fixtures.rs) also exercises HTTP draining, terminal mapping, truncated trailers and typed cancellation. Filter the whole `bedrock::event_stream::` prefix to include both `tests` and `split_frame_tests`. Fixture-only cancellation is deliberately a no-op; the signed HTTP path has cancellation tests. |
| Bedrock SigV4 | Fixed signing/key-derivation vectors, canonical headers, whitespace and session-token rules in [sigv4](../../crates/opi-ai/src/bedrock/sigv4.rs) | **Bedrock**: `stream_prepared` tests assert protected signature/session-token headers and resolve-once frozen auth across retry. Pure signing success does not prove actual path/header/body construction. This module is distinct from the private `crypto` owner. |
| Provider adapters and shared stream types | SSE/event mappers, UTF-8 decoder and producer/queue cancellation remain local | **Adapters**, **Fixtures**, **Wire**, **Accounting**: fixtures prove mapping; local HTTP lifecycle cases prove dispatch, wire shape, typed errors, terminal/EOF and cancellation. `ChatStream` tests bind iteration and `result` to one outcome. Retain provider-trait/error/diagnostic and test-support tests. |

## Gaps and bounded follow-up

The following distinguishes missing production paths, test discriminating power,
and evidence still to be mapped. None is a newly observed runtime test failure.

| Gap or limitation | Required acceptance evidence and next action |
| --- | --- |
| WS production path is absent | [ws.rs](../../crates/opi-ai/src/ws.rs) declares no production codec caller; [Codex](../../crates/opi-ai/src/openai_codex_responses.rs) currently streams HTTP/SSE. Until the admitted caller exists, leave Models-to-Codex-to-Upgrade-to-codec, continuation/pool and full cleanup acceptance open. More private tests cannot close it. |
| Crypto worker has no production caller | `sign_on_worker` currently has only private tests. Keep those tests; when an admitted JWT/ADC caller exists, prove its public request, cancellation, ownership and late-result suppression. TLS handshake cancellation is different evidence. |
| Socket reuse is not proved by the pooling battery | `connection_pooling` checks settings and Arc identity; primary HTTP/TLS helpers disable idle pooling. Add only a distinct public H1 keep-alive case: one shared client, two fully consumed calls, two requests and exactly one accepted socket. Keep existing settings tests. |
| Public Upgrade rejection branches lack named cases in the inspected batteries | Add table-driven public cases for missing/wrong/duplicate `Upgrade` and duplicate selected subprotocol, asserting the specific typed terminal rejection and exact sends. Existing unsolicited Version, Accept, 307, trust, mTLS/CONNECT and PMD cases should be retained, not duplicated. |
| Worker cancellation timing is not discriminated | Existing cancellation tests poll then drop, or preset cancellation. Strengthen private tests with a controlled signing key and entry/release barriers: prove cancellation after synchronous-call entry, key retention until completion, suppressed publication and a call counter for cancellation before entry. Do not claim a native call is preemptible. |
| Adapter coverage needs an explicit consumer map | Map transcript/replay/schema/auth/usage cases to actual consumers and supported capabilities. Reuse existing fixtures, wire and lifecycle cases; add only a missing contract/adapter cell with a distinguishing counterexample. Test names alone do not establish a complete matrix. |
| Current artifact/platform execution is not established | Existing package scripts cover bounded WS/HTTP/TLS and early Schema slices. Bind new runs to actual package/consumer locks and features; record native host versus WSL, target and toolchain. Historical Schema evidence is not a current-tree or all-platform pass; a missing run is an evidence gap, not automatically a code defect. |

Persistent credentials already have a concrete
[KeychainCredentialStore](../../crates/opi-coding-agent/src/credential_store.rs)
and [credential_store tests](../../crates/opi-coding-agent/tests/credential_store.rs),
including lock, rotation cancellation, logout ordering and barriers. Do not
label that coverage absent. Cross-process/concrete-store acceptance belongs to
that adapter and the assembled caller; `InMemoryCredentialStore` success cannot
replace it. Full platform execution remains to be checked.

## Acceptance commands

Run from the repository root. These are planned commands, not execution results.
Choose the union for changed surfaces; after it passes, do not rerun unchanged
groups merely because they appear in this catalogue. The final crate gate can
replace the default-graph focused runs when the scope requires the whole crate.

### Environment and existing shared cache

Use local fixtures/scripted loopback peers, isolated temporary data and
`MockProvider`; no paid provider, ambient credential activation or live endpoint
is part of this plan. Cargo offline mode controls dependency resolution, not a
test's network behavior. Preserve environment-mutation serialization.

```powershell
python scripts/opi-cargo-cache.py status
$sharedTarget = (python scripts/opi-cargo-cache.py resolve).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cargo cache resolution failed' }
$env:CARGO_TARGET_DIR = $sharedTarget
$env:CARGO_INCREMENTAL = '1'
```

`status` is read-only; `resolve` may initialize the stable workspace/toolchain
cache marker. Do not create a per-session target or clean/delete shared caches.
Offline execution needs the dependency/registry archives and native toolchain
already available. The manifest declares edition 2024 and MSRV 1.97; successful
execution with a newer compiler does not establish MSRV support.

### Private mechanisms

Select the relevant filters below. Cargo uses substring matching; these scoped
strings avoid selecting another module merely because its name contains the
same word. Each has existing tests; confirm actual selected names/counts. TLS
and mutation have no established private test filter to substitute for their
integration targets.

```powershell
cargo test --locked --offline -p opi-ai --lib ws::
cargo test --locked --offline -p opi-ai --lib crypto::
cargo test --locked --offline -p opi-ai --lib retry::
cargo test --locked --offline -p opi-ai --lib schema::
cargo test --locked --offline -p opi-ai --lib transcript::
cargo test --locked --offline -p opi-ai --lib replay::
cargo test --locked --offline -p opi-ai --lib bedrock::event_stream::
cargo test --locked --offline -p opi-ai --lib bedrock::sigv4::
cargo test --locked --offline -p opi-ai --lib stream::cancellation_tests::
cargo test --locked --offline -p opi-ai --lib text_decode::
cargo test --locked --offline -p opi-ai --lib endpoint::
```

### Public module and combination groups

Use whole integration targets to include distributed cases and nested modules.
`tests/support/tls_fixtures.rs` and the other `tests/support` modules are helpers,
not acceptance batteries.

```powershell
# Transport
cargo test --locked --offline -p opi-ai --test ws_upgrade --test tls_contract --test http_acceptance --test proxy_support --test connection_pooling --test retry_backoff
# Schema
cargo test --locked --offline -p opi-ai --test schema_engine --test schema_public
# Credentials and Models lifecycle
cargo test --locked --offline -p opi-ai --test auth_contracts --test per_request_auth --test oauth_wire_shape --test provider_collection --test custom_provider_registration
# Canonical transcript and replay
cargo test --locked --offline -p opi-ai --test transcript_contract --test replay_contract --test tool_result_wire --test tool_result_no_leak
# Accounting and stream outcomes
cargo test --locked --offline -p opi-ai --test context_budget --test usage_cost --test stream_events
# Catalog
cargo test --locked --offline -p opi-ai --test registry --test model_wire_metadata --test model_capabilities_migration --test api_mapped_provider
# Bedrock
cargo test --locked --offline -p opi-ai --test bedrock_fixtures
# Adapter dispatch and lifecycle
cargo test --locked --offline -p opi-ai --test provider_trait --test provider_error_classes --test provider_diagnostics --test provider_lifecycle --test openai_compat_lifecycle --test openai_responses_lifecycle --test gemini_lifecycle --test openai_codex_responses
# Provider fixtures
cargo test --locked --offline -p opi-ai --test anthropic_fixtures --test anthropic_thinking_roundtrip --test openai_chat_fixtures --test openai_responses_fixtures --test gemini_fixtures --test azure_openai_fixtures --test vertex_fixtures --test mistral_fixtures --test openrouter_fixtures
# Additional wire contracts and test infrastructure
cargo test --locked --offline -p opi-ai --test request_enrichment --test image_input --test image_provider_serialization --test output_content_image --test test_support_tests
```

### WS feature graphs

Keep ordinary, caller-default and C-zlib union graphs distinct. `--all-features`
is not a replacement. The C-zlib union requires its native build tools.

```powershell
cargo test --locked --offline -p opi-ai --lib ws:: --features flate2/default
cargo test --locked --offline -p opi-ai --test ws_upgrade --features flate2/default
cargo test --locked --offline -p opi-ai --lib ws:: --features flate2/zlib
cargo test --locked --offline -p opi-ai --test ws_upgrade --features flate2/zlib
```

### SDK artifact and external consumers

Use a new external scratch for every runner. The existing marked target must
belong to this workspace. Independent consumers resolve their own locks offline;
the workspace lock alone does not establish their dependency availability.

```powershell
$acceptanceRoot = Join-Path $env:TEMP ('opi-ai-acceptance-' + [guid]::NewGuid().ToString('N'))
python -B scripts/pi-sdk-conformance/package_slice.py --family ws --offline --scratch (Join-Path $acceptanceRoot 'ws') --target-dir $sharedTarget
python -B scripts/pi-sdk-conformance/package_slice.py --family crypto --offline --scratch (Join-Path $acceptanceRoot 'crypto') --target-dir $sharedTarget
python -B scripts/pi-sdk-conformance/package_slice.py --family http --offline --scratch (Join-Path $acceptanceRoot 'http') --target-dir $sharedTarget
python -B scripts/pi-sdk-conformance/run_early_package_proof.py --scratch (Join-Path $acceptanceRoot 'schema') --target-dir $sharedTarget
```

The [family runner](../../scripts/pi-sdk-conformance/package_slice.py) supports
only `ws`, `crypto` and `http`. Its WS runs distinguish extracted private tests,
packaged public Upgrade tests and external public consumers, across three
graphs. Crypto/HTTP have ordinary and caller-union public consumers; the crypto
family exercises TLS, not the complete private signing/worker contract.
The [early runner](../../scripts/pi-sdk-conformance/run_early_package_proof.py)
internally fixes offline operation and accepts no `--offline` option; it checks
early public Schema behavior in two graphs. Its `ICU4X_DATA_DIR` precondition
requires that variable to be absent. Neither runner accepts `--output`/`--json`;
logs and `evidence.json` are written inside the scratch. Follow the
[harness prerequisites](../../scripts/pi-sdk-conformance/README.md), including
native provenance and rejected source/config overrides. These slices do not
establish package-level behavior for every module or final SDK distribution.

When changing the harness itself, also run its tests; they do not replace the
actual package/consumer runs:

```powershell
python -B -m unittest discover -s scripts/pi-sdk-conformance -p 'test_package*.py' -v
```

### Crate and cross-crate gates

For a local runtime change, use focused tests plus clippy for affected targets,
for example the command below; never pass a test-name filter to clippy. A full
crate scope uses the subsequent crate commands.

```powershell
cargo clippy --locked --offline -p opi-ai --lib --test ws_upgrade -- -D warnings
cargo test --locked --offline -p opi-ai --all-targets
cargo test --locked --offline -p opi-ai --doc
cargo clippy --locked --offline -p opi-ai --all-targets -- -D warnings
```

When the concrete credential adapter is in scope, include
`cargo test --locked --offline -p opi-coding-agent --test credential_store`
and its affected-target clippy, then the applicable workspace gates. Select
cross-crate checks by the existing
[change-scope contract](../../.agents/skills/_shared/references/change-scope-and-check-selection.md).
For cross-crate runtime, Phase exit, CI or release scope, retain the workspace
gates below and the owning product cases; module results do not replace them.

```powershell
python scripts/opi-doc-check.py
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
$savedRustdocFlags = $env:RUSTDOCFLAGS
try {
    $env:RUSTDOCFLAGS = '-D warnings'
    cargo doc --workspace --no-deps
    if ($LASTEXITCODE -ne 0) { throw 'cargo doc failed' }
} finally {
    if ($null -eq $savedRustdocFlags) {
        Remove-Item Env:RUSTDOCFLAGS -ErrorAction SilentlyContinue
    } else {
        $env:RUSTDOCFLAGS = $savedRustdocFlags
    }
}
```

## Execution order and handoff

1. Bind the current state and relevant contract to existing tests -> verify:
   HEAD, staged/unstaged/untracked inventories, exact consumer/capability map,
   test discovery and the verification union. Recheck the carried-in WS inputs.
2. Run the selected private/public groups and dependency profiles -> verify:
   actual nonzero execution, observable positive/negative outcomes and affected
   clippy targets. Add only distinguishing cases for confirmed gaps after
   implementation admission; preserve existing test/fixture ownership.
3. Run applicable artifact/platform and cross-crate acceptance -> verify:
   actual package identities, external-consumer results, host/target/MSRV,
   feature/native provenance and registered product cases.
4. Hand off the bounded result -> verify: exact commands/exits/counts, material
   identities, accepted differences, unresolved production/evidence gaps and
   test impact (`add`, `update`, `delete`, `retain` or `none`). A failed step is
   not closed by a passing narrower step.

For this documentation-only preparation, test impact is `retain`: no runtime,
test, manifest, fixture, harness or ledger change is proposed for immediate
application. Verify the new document pair with
`python scripts/opi-doc-check.py` and `git diff --check`.
