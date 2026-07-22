# Phase 14 Remediation Plan

**Date**: 2026-07-22
**Audit sources**: `docs/snapshots/phase14/audit.codex.md`, `docs/snapshots/phase14/audit.glm5.2.md`
**Source baseline**: `8d6e6cacbfd211a3d5db97e35f1b89210bdecaf0`
**Phase 14 task range**: `d9f21a9..8364e74`
**Later source remediations included in the baseline**: `9263114`, `b27905a`, `47400ee`
**Design specs**: `docs/opi-spec.md`, `docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md`, `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md`
**Execution status**: complete; remediation applied and verified on 2026-07-22

The previous plan targeted source commit `b27905a` and is superseded by this
plan. The refreshed audit files contain 22 enumerated findings. The GLM report's
executive count says ten Info findings, but only seven Info findings are
enumerated; every enumerated finding is included below.

---

## Audit cross-reference summary

With two auditors, a unique finding has a trust weight of 0.5 and requires
direct source verification. Several sections of the GLM report state that a
contract passes while the Codex report identifies a narrower counterexample;
those positions are recorded as disputed rather than majority-voted.

| Cluster | Theme | Auditors | Consensus | Unified severity | Verification |
|---------|-------|----------|-----------|------------------|--------------|
| C1 | Custom provider identity collisions | Codex 2.1 | Unique (1/2) | Major | Confirmed |
| C2 | Extension overrides are selectable but undispatchable by mapped providers | Codex 2.2 | Unique (1/2) | Major | Confirmed |
| C3 | Refresh stops at the first error instead of collecting the full batch | Codex 2.3; GLM SC7 says compliant | Disputed (1/2) | Major | Confirmed; GLM claim refuted |
| C4 | Copilot Anthropic compatibility metadata is not consumed | Codex 2.4; GLM cross-task section says no defect | Disputed (1/2) | Major | Confirmed |
| C5 | Initial OpenAI Chat tool-call arguments are dropped | Codex 2.5 | Unique (1/2) | Major | Confirmed |
| C6 | Public dispatch bypasses unsupported-thinking preflight and permits incoherent model metadata | Codex 2.6; GLM spec section says preflight passes | Disputed (1/2) | Major | Confirmed; GLM tests mask the bypass |
| C7 | OPI keyring leasing destroys a pre-existing process default store | Codex 2.7; GLM SC1 covers only empty-store lifecycle | Disputed scope (1/2) | Major | Confirmed |
| C8 | Credential envelopes accept mixed-kind and unknown fields | Codex 3.1 | Unique (1/2) | Minor | Confirmed |
| C9 | Cumulative session cost truncates at `u32::MAX` | Codex 4.1 | Unique (1/2) | Minor | Confirmed |
| C10 | Public docs omit `AccountIdMissing` | Codex 4.2 | Unique (1/2) | Minor | Confirmed |
| C11 | Custom-header negative tests fail at the earlier base-URL gate | Codex 5.1 | Unique (1/2) | Minor | Confirmed |
| C12 | Responses/Codex do not drain an unterminated trailing SSE frame | GLM 2.1 | Unique (1/2) | Info | Confirmed behavior; no required action |
| C13 | Compatible Chat reuses the session id as the request id | GLM 2.2 | Unique (1/2) | Info | Partially confirmed; intentional current contract |
| C14 | Usage subset validation lives in provider mappers, not the public struct | GLM 2.3 | Unique (1/2) | Info | Partially confirmed; matches binding design |
| C15 | Unknown Chat `finish_reason` maps to `StopReason::Error` | GLM 2.4 | Unique (1/2) | Info | Partially confirmed; impact overstated |
| C16 | Marker/envelope persistence is a two-entry non-atomic protocol | GLM 3.1 | Unique (1/2) | Minor | Partially confirmed; fail-closed behavior is intentional but under-documented |
| C17 | Persistence serialization is a second secret-exposure boundary | GLM 3.2 | Unique (1/2) | Info | Confirmed documentation error; implementation is bounded |
| C18 | Real crossterm `tui_event_loop` wiring lacks automated coverage | GLM 4.1; Codex says outer-TUI behavior is covered | Disputed scope (1/2) | Minor (from Major) | Confirmed coverage gap; no observed state-machine defect |
| C19 | Harness resume/fork session id is not traced into provider requests | GLM 4.2 | Unique (1/2) | Minor | Refuted; exact product test already exists |
| C20 | OAuth-shaped malformed-envelope redaction lacks a non-vacuous test | GLM 4.3 | Unique (1/2) | Info | Confirmed test gap |
| C21 | Refresh and public write lack a direct cross-process-lock race test | GLM 4.4 | Unique (1/2) | Info | Confirmed test gap |
| C22 | Shared resolver identity is caller-owned in `ApiMappedProvider` | GLM 7.1 | Unique (1/2) | Info | Confirmed limitation; no production defect |

## Verification evidence

- C1: `config.rs` validates only a non-empty custom id; runtime matching and
  listing registration use different paths, while model specs split on the
  first `:`.
- C2: the registry and harness consume extension overrides, but
  `ApiMappedProvider::stream` resolves only its construction-time private
  catalog.
- C3: `ProviderCollection::refresh` sorts ids, then returns immediately on a
  provider or candidate-validation error. Atomic rollback works, but later
  providers are not called.
- C4: Copilot models carry `force_adaptive_thinking` and
  `supports_temperature`; Anthropic request construction reads neither field.
- C5: an OpenAI Chat delta with an id emits `ToolCallStart`; initial arguments
  are processed only in the branch where the id is absent.
- C6: the standalone validator is used by `agent_loop`, but mapped, collection,
  and direct library dispatch can bypass it. Existing zero-HTTP tests call the
  validator manually before `stream`.
- C7: the first OPI lease overwrites the process default store and the last
  lease unconditionally unsets it. `KeyringCoreBackend::new` enters this path
  automatically.
- C8: one flattened envelope struct contains both credential variants and
  serde ignores unknown fields.
- C9: `CumulativeUsage` retains `u64`, but `cost_summary` converts through the
  saturating `Usage` projection before calculating cost.
- C10: the runtime variant is recorded in `CHANGELOG.md` but absent from all
  five English/Chinese public-document pairs.
- C11: the named header fixtures omit `base_url`, so validation exits before
  header parsing.
- C16: marker-first persistence can expose a transitional wrong-kind state and
  a second-step failure can leave a persistent fail-closed marker-only state.
  The existing test intentionally pins the latter behavior.
- C17: protected serialization necessarily calls `expose_secret`, but the
  returned JSON and intermediate fields are zeroized. The historical design
  and public rustdoc incorrectly say HTTP is the only exposure boundary.
- C18: tests enter `run_interactive_tui` through a debug driver and exercise the
  shared prompt/auth state machine, but return before the concrete crossterm
  event loop.
- C19 is refuted by
  `phase14_session_affinity_tracks_new_resume_and_fork`, which constructs a
  real `CodingHarness`, captures requests, and asserts new/resumed/forked
  coordinator ids.

## Decision record

| ID | Finding cluster(s) | Decision | Rationale | Decided by |
|----|--------------------|----------|-----------|------------|
| D1 | C1 | Validate the merged provider-id namespace once and reject reserved ids, deprecated ids, `:`, and custom/profile duplicates. | Listing and runtime must not resolve the same valid config differently. Rejecting ambiguity is smaller and safer than precedence rules. | auto |
| D2 | C2 | Materialize extension overrides into the effective mapped catalog and route subsets before provider construction. | The normative extension contract advertises model overrides. Rejecting all mapped-provider overrides would silently remove that capability. | auto |
| D3 | C3 | Call and validate every provider in sorted order, retain the first sorted error, and mutate catalogs only after a fully successful batch. | This is the explicit T3e and task 14.6 contract. | auto |
| D4 | C4 | Consume `AnthropicMessagesCompat` during Anthropic request construction. | One compatibility interpretation is already encoded in the audited catalog; request assembly must honor it. | auto |
| D5 | C5 | Preserve non-empty arguments from the initial tool-call chunk and emit them in order after the start event. | Dropping valid stream bytes has one direct correction. | auto |
| D6 | C6 | Add one selected-model preflight used by every public dispatch path and enforce model-capability coherence at metadata validation. | The binding spec requires unsupported levels to fail before auth or network I/O on every wire. | auto |
| D7 | C7, C21 | Preserve and conditionally restore the pre-existing default keyring store; add a direct refresh-versus-write lock test. | OPI must not corrupt embedder process state. Restoration must not overwrite a newer external replacement. | auto |
| D8 | C8, C20 | Decode strict variant-specific v1 envelopes and add non-vacuous OAuth redaction cases. | Mixed-kind or unknown state is corruption and must fail closed without leaking canaries. | auto |
| D9 | C9 | Calculate cumulative cost from internal `u64` totals while retaining the public saturating `Usage` projection. | Public compatibility and exact aggregate cost can both be preserved. | auto |
| D10 | C10 | Document `AccountIdMissing` across every English/Chinese public pair and guard the taxonomy. | The public runtime error already exists; documentation must describe its distinct remediation and mode behavior. | auto |
| D11 | C11 | Repair header fixtures so they reach and specifically assert the header validation gate. | Test names and assertions must prove the invariant they claim. | auto |
| D12 | C16, C17 | Keep the fail-closed two-entry persistence protocol, document its transition/failure semantics, and correct the persistence exposure-boundary wording. | A generation-addressed commit protocol would be a new storage design. The current behavior is safe, retry-recoverable, and already partly pinned; its contract needs explicit evidence. | auto |
| D13 | C18 | Make the production event dispatcher injectable and drive it with a fake event source/terminal backend in debug and release tests. | This closes the real-loop wiring gap without redesigning the prompt/auth state machine. | auto |
| D14 | C12-C15, C22 | No behavioral change. | These are conformant-server robustness, intentional compatibility policy, mapper-level validation, fail-closed forward behavior, or a documented caller-owned invariant. | auto |
| D15 | C19 | Drop the finding. | The exact requested production test already exists and is cited by both normative specs. | auto |

## Remediation layers

The workspace dependency graph is:

```text
Layer 1: opi-ai, opi-tui
Layer 2: opi-agent -> opi-ai
Layer 3: opi-coding-agent -> opi-ai, opi-agent, opi-tui
Layer 4: documentation
```

No `opi-tui` or `opi-agent` source change is required. Their empty graph layers
are omitted, while the remaining layer numbers retain their dependency-graph
meaning.

### Layer 1: opi-ai (substrate)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-ai --all-targets -- -D warnings
    cargo test -p opi-ai --all-targets

#### Fix 1.1: Enforce capability preflight on every public dispatch path

- **Audit source**: Codex 2.6
- **Cluster**: C6
- **Decision**: D6
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/provider.rs` ~L226-275;
  `crates/opi-ai/src/provider_collection.rs` ~L389-411;
  `crates/opi-ai/src/api_mapped.rs` ~L175-196;
  `crates/opi-ai/src/model_info.rs` ~L538-564;
  `crates/opi-ai/src/anthropic.rs` ~L811-902;
  `crates/opi-ai/src/openai_chat.rs` ~L1096-1101;
  `crates/opi-ai/src/openai_responses.rs` ~L321-326;
  `crates/opi-ai/src/openai_codex_responses.rs` ~L110-123
- **Change**: Introduce a shared selected-model preflight and call it before
  auth resolution or request construction from mapped, collection, and direct
  built-in provider paths. Collection dispatch must use registry-resolved
  metadata so extension overrides are visible. Propagate thinking-map failures
  instead of omitting reasoning or falling back to raw level names. Extend
  `ModelInfo::validate` to reject thinking maps enabled on models without
  thinking support and long-cache support without cache-control support.
- **Test plan**: Counting auth resolver plus empty `wiremock` server for
  Anthropic, Chat, Responses, Codex, `ApiMappedProvider`, and
  `ProviderCollection`; unsupported levels must perform zero auth resolutions
  and zero HTTP calls. Add both incoherent-metadata cases.

#### Fix 1.2: Collect the complete dynamic-refresh batch

- **Audit source**: Codex 2.3; GLM SC7 contradictory pass claim
- **Cluster**: C3
- **Decision**: D3
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/provider_collection.rs` ~L442-491;
  `crates/opi-ai/tests/provider_collection.rs` ~L1064-1145
- **Change**: Invoke every provider in sorted id order, store all refresh
  results, validate every successful candidate without touching the live
  registry, retain the first error in sorted order, and replace dynamic
  catalogs only when no provider or validation error occurred.
- **Test plan**: Shared invocation log with an early provider error and a
  separate early invalid catalog. Assert all providers ran in sorted order,
  the first sorted error was returned, and the prior registry stayed byte-for-
  byte equivalent.

#### Fix 1.3: Apply Copilot Anthropic compatibility metadata

- **Audit source**: Codex 2.4
- **Cluster**: C4
- **Decision**: D4
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/anthropic.rs` ~L811-902;
  `crates/opi-ai/tests/anthropic_fixtures.rs`
- **Change**: Resolve the selected model's
  `WireCompat::AnthropicMessages`. Omit `temperature` when
  `supports_temperature` is false and emit the adaptive-thinking wire shape
  when `force_adaptive_thinking` is true. Normalize defaults so direct
  Anthropic and custom-model behavior does not regress.
- **Test plan**: Direct request captures for forced-adaptive/no-temperature
  and fixed-budget/temperature-supporting controls. Layer 3 adds the required
  factory-built Copilot captures.

#### Fix 1.4: Preserve initial OpenAI Chat tool arguments

- **Audit source**: Codex 2.5
- **Cluster**: C5
- **Decision**: D5
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/openai_chat.rs` ~L256-359;
  `crates/opi-ai/tests/openai_chat_fixtures.rs` ~L237-246, L1420-1428
- **Change**: When an initial tool-call delta contains id, name, and non-empty
  arguments, emit `ToolCallStart` followed by the corresponding argument delta
  and seed the accumulator exactly once. Preserve source ordering for multiple
  calls.
- **Test plan**: Single- and multi-tool fixtures with non-empty initial
  argument prefixes and later deltas; assert byte-exact final JSON arguments
  and event order.

#### Fix 1.5: Add exact `u64` cumulative cost calculation

- **Audit source**: Codex 4.1
- **Cluster**: C9
- **Decision**: D9
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/stream.rs` ~L127-233, L270+;
  `crates/opi-ai/tests/usage_cost.rs` ~L243+
- **Change**: Add an internal/publicly suitable cost calculation path that
  consumes `CumulativeUsage`'s `u64` parent and optional child totals directly.
  Keep `as_usage()` saturating at `u32::MAX` for the existing public shape.
- **Test plan**: Exact expected cost above `u32::MAX`, including base and
  tiered pricing plus partial/all one-hour cache-write subsets. Assert the
  public `Usage` remains saturated while cost continues increasing.

### Layer 3: opi-coding-agent (product and integration)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-coding-agent --all-targets -- -D warnings
    cargo test -p opi-coding-agent --all-targets

#### Fix 3.1: Reject ambiguous provider identities after config merge

- **Audit source**: Codex 2.1
- **Cluster**: C1
- **Decision**: D1
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/config.rs` ~L921-1059;
  `crates/opi-coding-agent/src/provider_factory.rs` ~L292, L1430, L1778;
  `crates/opi-coding-agent/tests/custom_provider_map.rs`;
  `crates/opi-coding-agent/tests/provider_identity.rs`
- **Change**: Validate the final layered namespace across canonical built-ins,
  deprecated ids, `[providers.custom]`, and
  `[providers.openai_compatible]`. Reject empty ids, `:`, reserved names, and
  cross-table duplicates before listing or provider construction. Both paths
  must consume the same validated configuration.
- **Test plan**: Table-driven invalid cases for built-in/deprecated collisions,
  colon ids, and custom/profile duplicates. Add a two-endpoint product test
  proving no accepted config can list one provider and execute another.

#### Fix 3.2: Build mapped providers from the effective override catalog

- **Audit source**: Codex 2.2
- **Cluster**: C2
- **Decision**: D2
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/registry.rs` ~L184-251;
  `crates/opi-ai/src/api_mapped.rs` ~L175-196;
  `crates/opi-coding-agent/src/provider_factory.rs` ~L2121-2154;
  `crates/opi-coding-agent/src/harness.rs` ~L742-758, L948-1013, L1191-1196;
  extension/provider integration tests
- **Change**: Apply extension additions and same-id overrides before the mapped
  provider and its concrete route subsets are finalized. Validate that each
  effective model has a matching route and compatibility value. Make picker,
  registry validation, mapped-provider lookup, preflight, and route dispatch
  consume that same effective metadata.
- **Test plan**: A harness-level extension adds a model to a mapped provider,
  selects it, and captures the expected concrete route. Add a same-id
  wire/compatibility override and prove either valid dispatch or a pre-network
  validation error; no selectable model may later return `UnknownModel`.

#### Fix 3.3: Capture and restore foreign keyring ownership

- **Audit source**: Codex 2.7; GLM 4.4
- **Cluster**: C7, C21
- **Decision**: D7
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/native_keyring.rs` ~L10-64,
  ~L219-248; `crates/opi-coding-agent/src/credential_store.rs` ~L170-187,
  ~L809-820, ~L1043-1084; credential/OAuth tests
- **Change**: On the first OPI lease, retain the prior process default and the
  identity of the OPI-installed store. On the last drop, restore the prior
  store only if OPI still owns the current default; otherwise leave a newer
  external replacement untouched. Preserve nested-lease behavior and typed
  construction errors.
- **Test plan**: Serialized global-state lifecycle test: preinstall A, acquire
  two OPI leases installing B, drop both, and verify A is restored only after
  the last drop. Add an external B-to-C replacement case proving OPI does not
  clobber C. Add a blocking refresh versus public write test using two stores
  with the same lock path and assert strict serialized mutation.

#### Fix 3.4: Strictly decode credential envelope variants

- **Audit source**: Codex 3.1; GLM 4.3
- **Cluster**: C8, C20
- **Decision**: D8
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/credential_store.rs` ~L404-426,
  ~L480-592; `crates/opi-coding-agent/tests/credential_store.rs` ~L595-667,
  ~L786-846, ~L1446-1524
- **Change**: Replace the flattened all-fields decoder with strict
  variant-specific v1 payloads or explicit key validation. Reject fields from
  the other credential kind and all unknown fields as `MalformedEnvelope`;
  preserve version/kind-specific errors, no env fallback, and zeroized encode
  buffers.
- **Test plan**: API-key with OAuth fields, OAuth with `api_key`, and unknown
  fields for both variants. Seed real access/refresh canaries in structurally
  malformed OAuth JSON and assert store, resolver, and mapped-provider
  `Display`/`Debug` output omit them.

#### Fix 3.5: Use exact cumulative totals in session cost summaries

- **Audit source**: Codex 4.1
- **Cluster**: C9
- **Decision**: D9
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/session_coordinator.rs` ~L609-624;
  session usage/cost tests
- **Change**: Replace the `as_usage()` conversion in `cost_summary` with the
  Layer 1 `u64` cumulative-cost path. Preserve unknown-pricing behavior and
  existing model-pricing precedence.
- **Test plan**: Coordinator-level exact summary beyond `u32::MAX`, including
  a pricing tier boundary and optional subsets; assert cost before/after
  resume remains equal.

#### Fix 3.6: Make custom-header negative tests reach the intended gate

- **Audit source**: Codex 5.1
- **Cluster**: C11
- **Decision**: D11
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/tests/custom_provider_map.rs` ~L186,
  ~L289-337; `crates/opi-coding-agent/src/config.rs` ~L921-1059
- **Change**: Supply a valid base URL in reserved-header and invalid-header
  fixtures. Assert the typed `InvalidCustomProvider` field and a safe specific
  reason class rather than only provider id/no-secret canaries.
- **Test plan**: Table-driven invalid header name, invalid value, and reserved
  auth-header cases, each proving the header gate was reached.

#### Fix 3.7: Exercise production TUI event dispatch

- **Audit source**: GLM 4.1
- **Cluster**: C18
- **Decision**: D13
- **Verification status**: Confirmed; severity reduced from Major to Minor
- **File(s)**: `crates/opi-coding-agent/src/interactive.rs` ~L742-781,
  ~L781-1116, ~L1242+;
  `crates/opi-coding-agent/tests/interactive_tui_auth.rs`
- **Change**: Extract the concrete event polling/dispatch boundary behind an
  internal injectable event source and terminal backend. Make the production
  crossterm loop and scripted tests enter the same dispatcher instead of
  returning to a separate headless loop before dispatch.
- **Test plan**: Drive prompt -> pre-output `CredentialNeeded` -> same-provider
  `/login` -> retry -> exit through the production dispatcher. Retain all
  negative paths and exact message/call counts. Run the owning target in both
  normal and `--release` test profiles to cover the former
  `debug_assertions` split.

#### Fix 3.8: Pin the fail-closed two-entry persistence protocol

- **Audit source**: GLM 3.1
- **Cluster**: C16
- **Decision**: D12
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-coding-agent/src/credential_store.rs` ~L756-770,
  ~L1110-1149; `crates/opi-coding-agent/tests/credential_store.rs` ~L808-846
- **Change**: Do not claim atomicity across the keychain's separate marker and
  protected entries. Keep the current fail-closed recovery behavior and make
  the transition/failure contract explicit in module documentation. Do not
  reverse write order as a false atomicity fix.
- **Test plan**: Pause deterministically between both backend writes during a
  kind change and assert the typed transitional result. Cover protected-write
  failure, blocked env fallback, and successful retry recovery.

#### Fix 3.9: Capture factory-built Copilot Anthropic wire behavior

- **Audit source**: Codex 2.4
- **Cluster**: C4
- **Decision**: D4
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/github_copilot.rs` ~L75-94,
  ~L146-181, ~L414-438;
  `crates/opi-coding-agent/src/provider_factory.rs` ~L1295-1372;
  `crates/opi-coding-agent/tests/github_copilot_provider.rs`
- **Change**: No additional production logic beyond Layer 1 unless factory
  normalization is required. Add product-path evidence that final catalog
  metadata reaches the concrete Anthropic request.
- **Test plan**: Factory-built captures for Copilot Opus 4.7 and 4.8 with a
  requested temperature and enabled thinking; assert temperature is absent and
  adaptive thinking is emitted. Add a non-adaptive control model.

### Layer 4: Documentation

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-coding-agent --all-targets -- -D warnings
    cargo test -p opi-coding-agent --test phase14_provider_auth_docs
    $env:RUSTDOCFLAGS="-D warnings"; cargo doc --workspace --no-deps

#### Fix 4.1: Document the complete typed authentication taxonomy

- **Audit source**: Codex 4.2
- **Cluster**: C10
- **Decision**: D10
- **Verification status**: Confirmed
- **File(s)**: `README.md`, `README.zh.md`;
  `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`;
  `crates/opi-agent/README.md`, `crates/opi-agent/README.zh.md`;
  `crates/opi-coding-agent/README.md`,
  `crates/opi-coding-agent/README.zh.md`;
  `docs/opi-spec.md`, `docs/opi-spec.zh.md`;
  `crates/opi-coding-agent/tests/phase14_provider_auth_docs.rs`
- **Change**: Add `AccountIdMissing { provider_id }` beside
  `CredentialNeeded` and `CredentialRevoked`. Describe its non-retryability,
  distinction from revocation, canonical `/login <provider>` remediation,
  pre-output interactive handling, non-interactive AuthFailure exit, and
  JSON/RPC event behavior. Update every localized counterpart in the same
  change.
- **Test plan**: Extend the EN/ZH documentation guard to require the variant
  and its distinct behavior on every relevant surface.

#### Fix 4.2: Correct credential persistence boundary documentation

- **Audit source**: GLM 3.1, GLM 3.2
- **Cluster**: C16, C17
- **Decision**: D12
- **Verification status**: Confirmed documentation error; bounded
  implementation
- **File(s)**: `crates/opi-ai/src/credential.rs` ~L39-43;
  `crates/opi-coding-agent/src/credential_store.rs` module documentation;
  `docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md`
  ~L272-274; relevant documentation guards
- **Change**: State that secret exposure is restricted to concrete HTTP and
  protected keychain serialization boundaries, with zeroized serialized and
  intermediate buffers. Document that marker/envelope writes form a
  fail-closed, retry-recoverable two-entry protocol rather than an atomic
  transaction.
- **Test plan**: Rustdoc/documentation guard plus the Layer 3 transition,
  recovery, and redaction tests.

## Final verification

Run after every layer passes:

    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc
    $env:RUSTDOCFLAGS="-D warnings"; cargo doc --workspace --no-deps
    git diff --check

All new provider, OAuth, keyring, session, and TUI tests remain offline and use
`wiremock`, fake stores, fake terminals/event sources, `tempfile`, or
`MockProvider`. Tests that mutate the process-global keyring store or process
environment must be serialized.

## Scope exclusions

| Finding | Status | Reason |
|---------|--------|--------|
| C12 trailing unterminated SSE frame | Info / No action | SSE requires event termination by a blank line. Supporting a complete residual frame is optional non-conformant-server robustness, not a Phase 14 contract defect. |
| C13 compatible Chat request-id reuse | Info / No action | Current wire tests and the reviewed Phase 14 mapping intentionally set all three compatible-affinity headers from the clamped session id. A fresh UUID would be a policy change. |
| C14 Usage struct subset validation | Info / No action | Binding design requires provider mappers to reject malformed upstream usage, and they do. Public fields prevent the struct from enforcing the invariant universally. |
| C15 unknown Chat finish reason | Info / No action | The mapper returns `Done` with `StopReason::Error`; it does not emit a provider `Err` or stream `Error`. This is a defensible fail-closed forward-compatibility policy. |
| C19 harness session-id trace | Refuted | `phase14_session_affinity_tracks_new_resume_and_fork` already performs the requested real-harness request capture for new, resumed, and forked sessions. |
| C22 shared-resolver identity | Info / No action | `ApiMappedProvider` documents this as a caller-owned invariant, and all production construction paths share one resolver. Enforcing identity would require a new route-construction API without a current defect. |

## Execution result

The user authorized execution after reviewing the plan. All confirmed action
clusters were remediated:

| Clusters | Result |
|----------|--------|
| C1, C11 | Final merged provider namespaces are validated once; reserved, deprecated, colon-containing, and duplicate ids are rejected, and header-negative fixtures now reach the header gate. |
| C2 | Extension model additions and overrides are materialized into the mapped provider's effective catalog and route subsets before the provider is shared; failed materialization cannot advertise an undispatchable model. |
| C3 | Dynamic refresh invokes every provider in sorted order, retains the first sorted error, and atomically preserves the previous registry on any error. |
| C4, C5, C6 | Anthropic compatibility metadata is consumed, initial Chat tool arguments are preserved, and all public dispatch paths preflight coherent selected-model capabilities before auth or network work. |
| C7, C21 | Native keyring leases preserve and conditionally restore a foreign process default; refresh and public writes are proven to serialize on the same lock path. |
| C8, C20 | Credential envelopes decode through strict variant-specific schemas; malformed OAuth-shaped values fail closed without exposing access or refresh canaries. |
| C9 | Session cost calculation consumes exact cumulative `u64` totals while the compatibility `Usage` projection remains saturating. |
| C10, C16, C17 | All five English/Chinese public-document pairs cover `AccountIdMissing`; persistence docs now describe the non-atomic, fail-closed two-entry protocol and both bounded secret-exposure points. |
| C18 | Scripted debug and release tests now enter the same injectable production TUI event dispatcher used by crossterm. |
| C12-C15, C19, C22 | No production change, as recorded in the decision table and scope exclusions. |

Editing the normative English spec changed its CRLF-normalized digest. The
Phase 4 and Phase 6 archived snapshot hash fields were re-synchronized through
the repository's established maintenance convention so their executable drift
guards remain truthful. The canonical root `.opi-impl-state.json` and both
audit reports were not modified by remediation.

Fresh final verification:

    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc
    $env:RUSTDOCFLAGS="-D warnings"; cargo doc --workspace --no-deps
    git diff --check

The product TUI auth target was additionally exercised in both debug and
release profiles. No commit or push was requested or performed.
