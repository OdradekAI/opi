# Phase 14 pi-0.80.6 Alignment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close Phase 14 findings F14-01 through F14-04 and align opi's GitHub Copilot/OpenAI Codex provider identity, wire routing, catalog metadata, OAuth flows, and interactive retry behavior with the approved pi-0.80.6 design.

**Architecture:** Introduce a typed `ModelInfo -> WireApi -> Provider` routing layer in `opi-ai`, then lower built-in and TOML-defined providers through it. Keep authentication lazy at the concrete wire provider, preserve the native-keychain and typed-error hardening, give OpenAI Codex its own Responses implementation, and make the production dispatcher and outer TUI state machine the acceptance boundaries.

**Tech Stack:** Rust 2024, Tokio, reqwest, serde/TOML, wiremock, ratatui/crossterm, keyring-core, existing `opi-ai`/`opi-agent`/`opi-coding-agent` test support.

---

## Binding Sources and Non-Negotiable Invariants

Implement against:

- `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md`, especially the **pi-0.80.6 Alignment Revision (2026-07-17)**.
- `target/opi-artifacts/phase14-phase-exit/PHASE_EXIT_REPORT.md`.
- `target/opi-artifacts/phase14-phase-exit/PHASE_EXIT_SCENARIO_AUDIT.md`.
- `.repo/pi-0.80.6/packages/ai/src/providers/github-copilot.models.ts`.
- `.repo/pi-0.80.6/packages/ai/src/providers/openai-codex.models.ts`.
- `.repo/pi-0.80.6/packages/ai/src/api/openai-codex-responses.ts`.
- `.repo/pi-0.80.6/packages/ai/src/utils/oauth/openai-codex.ts`.

Preserve these approved opi behaviors:

- Native OS keychain; never add an opi-managed plaintext credential file.
- Typed `CredentialNeeded` and `CredentialRevoked`.
- Same-turn retry only after explicit successful login for the same provider.
- Strict `Usage` validation and `None` versus `Some(0)` preservation.
- Provider-managed auth headers remain reserved.
- Model refresh remains deterministic and atomic.
- Existing Anthropic cache-marker placement remains unchanged.
- Static model listing never reads OAuth credentials or contacts Copilot entitlement endpoints.
- No provider-id aliases and no keychain migration from `copilot`/`codex`.
- No session schema version change.
- No real keychain, browser, terminal, OAuth endpoint, provider endpoint, user config, or session directory in tests.

The pre-existing dirty workflow relocation is outside every task in this plan:

```text
M  .claude/skills/opi-implement/references/ledger-schema.md
M  .claude/skills/opi-implement/skill.md
M  .gitignore
D  scripts/exec.workflow.js
D  scripts/phase-exit.workflow.js
D  scripts/plan.workflow.js
?? .claude/skills/opi-implement/scripts/
```

Do not stage, revert, reformat, or otherwise absorb those paths into tasks 14.14-14.21. Do not hand-edit `.opi-impl-state.json`; invoke the `opi-implement` skill to query, reconcile, advance, and verify its ledger.

This plan file is also an untracked planning artifact until the user separately requests a documentation commit:

```text
?? docs/superpowers/plans/2026-07-17-phase14-pi-0806-alignment.md
```

Do not absorb it into an implementation task commit.

## Fixed Public Contracts

Create `crates/opi-ai/src/model_info.rs` as the canonical home of model routing metadata. Re-export existing public names from their old modules where that avoids needless import churn, but migrate all workspace construction through the new public constructor/builder surface.

```rust
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WireApi {
    AnthropicMessages,
    OpenAiCompletions,
    OpenAiResponses,
    OpenAiCodexResponses,
    GoogleGenerativeAi,
    GoogleVertex,
    BedrockConverseStream,
    AzureOpenAiCompletions,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingLevel {
    None,
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
    Max,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ThinkingLevelMapping {
    Identity,       // TOML true; use the level's canonical wire spelling
    Mapped(String), // TOML string; for example minimal -> low
    Unsupported,    // TOML false or pi null
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelPricing {
    pub base: Pricing,
    pub tiers: Vec<PricingTier>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PricingTier {
    pub input_tokens_above: u64,
    pub pricing: Pricing,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum WireCompat {
    AnthropicMessages(AnthropicMessagesCompat),
    OpenAiCompletions(OpenAiCompletionsCompat),
    OpenAiResponses(OpenAiResponsesCompat),
    OpenAiCodexResponses(OpenAiCodexResponsesCompat),
    GoogleGenerativeAi,
    GoogleVertex,
    BedrockConverseStream,
    AzureOpenAiCompletions,
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub wire_api: WireApi,
    pub capabilities: ModelCapabilities,
    pub base_url: Option<String>,
    pub thinking_level_map: ThinkingLevelMap,
    pub compat: WireCompat,
    pub pricing: Option<ModelPricing>,
}
```

Required constructors/builders:

```rust
impl ModelInfo {
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        wire_api: WireApi,
        capabilities: ModelCapabilities,
    ) -> Self;

    pub fn with_base_url(self, base_url: impl Into<String>) -> Self;
    pub fn with_thinking_level_map(self, map: ThinkingLevelMap) -> Self;
    pub fn with_compat(self, compat: WireCompat) -> Result<Self, ModelInfoError>;
    pub fn with_pricing(self, pricing: ModelPricing) -> Result<Self, ModelInfoError>;
    pub fn validate(&self) -> Result<(), ModelInfoError>;
}

impl ThinkingLevelMap {
    pub fn disabled() -> Self;
    pub fn reasoning_default() -> Self;
    pub fn with_mapping(
        self,
        level: ThinkingLevel,
        mapping: ThinkingLevelMapping,
    ) -> Self;
    pub fn resolve(
        &self,
        level: ThinkingLevel,
    ) -> Result<Option<String>, UnsupportedThinkingLevel>;
}

impl ModelPricing {
    pub fn try_new(
        base: Pricing,
        tiers: Vec<PricingTier>,
    ) -> Result<Self, ModelInfoError>;
    pub fn effective(&self, input_tokens: u64) -> Pricing;
}
```

`ThinkingLevelMap` is complete rather than sparse:

- `disabled()` supports only `off`/`ThinkingLevel::None`.
- `reasoning_default()` supports `off`, `minimal`, `low`, `medium`, and `high`; `xhigh` and `max` are explicitly unsupported.
- `ModelInfo::new` selects `disabled()` or `reasoning_default()` from the supplied capabilities and creates the matching default `WireCompat` variant for its `WireApi`.
- Model overrides replace individual entries.
- `off` resolves to omission/disabled behavior, not the literal string `"off"` unless a concrete wire explicitly requires it.
- `false`/pi `null` means unsupported; `true` means identity; a string is an explicit wire alias.
- Unsupported levels fail before request-body construction or network I/O.

`ThinkingConfig` becomes:

```rust
pub struct ThinkingConfig {
    pub enabled: bool,
    pub budget_tokens: Option<u64>,
    pub level: ThinkingLevel,
}
```

The old serialized session value `none` remains `ThinkingLevel::None`; CLI/TUI configuration continues to spell that choice `off`. Additive values are `minimal`, `xhigh`, and `max`.

`ModelPricing::effective(input_tokens)` selects the highest tier for which:

```rust
input_tokens > tier.input_tokens_above
```

Equality stays on the lower price. Reject non-finite/negative rates, zero thresholds, duplicate thresholds, descending thresholds, and any invalid base/tier before provider construction.

`ResolvedAuth` is binding:

```rust
pub struct ResolvedAuth {
    pub scheme: AuthScheme,
    pub secret: SecretString,
    pub base_url: Option<String>,
    pub account_id: Option<String>,
}
```

Static resolvers return `None` for both new fields. Stored GitHub Copilot OAuth returns its enterprise `base_url`; stored OpenAI Codex OAuth returns `account_id`.

## Task Dependency and Commit Rules

Execute in this order:

```text
14.14
14.15 -> 14.16 -> 14.17 --+
               \-> 14.18 --+-> 14.19 -> 14.20
14.14 -------------------------------> 14.21
14.17 + 14.18 + 14.19 + 14.20 ------> 14.21
```

Every task has `evaluator_required = true` and exactly one commit. Before each task:

- Run `git status --short` and confirm only the baseline paths, this untracked plan, and the current/prior task changes are present.
- Invoke `opi-implement <task-id>` so the skill owns ledger advancement and evaluator evidence.
- Follow RED -> GREEN -> REFACTOR.
- Run the task's focused tests and `cargo clippy --workspace --all-targets -- -D warnings`.
- Stage only the exact paths owned by that task.
- Confirm `git diff --cached --name-only` contains no baseline workflow-relocation path.
- Commit with the exact message listed below, plus the `opi-implement` evidence footers required by the current skill.

Do not archive automatically after 14.21. Archive remains a separate user-approved gate.

## Task 14.14: Native Keyring Host Selection

**Files**

- Modify: `crates/opi-coding-agent/src/native_keyring.rs`

### RED

- [ ] Replace the existing acceptance test body with a test that calls the same cfg-gated selection path as `install_native_keyring()`.
- [ ] Have the test inject a mock constructor, count constructor calls, acquire two guards, and assert:
  - the constructor runs exactly once;
  - a default store exists after the first guard;
  - the second lease does not replace the first store;
  - dropping one guard retains the store;
  - dropping the last guard unsets the store.
- [ ] Do not create a `keyring_core::Entry`; do not call set/read/delete password.

Run:

```powershell
cargo test -p opi-coding-agent --lib native_keyring_host_selection_installs_a_default_store
```

Expected RED: compilation fails because the injected host-selection seam does not exist yet.

### GREEN

- [ ] Add one private constructor type:

```rust
type PlatformStoreConstructor =
    fn() -> Result<Arc<keyring_core::CredentialStore>, String>;
```

- [ ] Split native construction from cfg selection:

```rust
pub fn install_native_keyring() -> Result<NativeKeyringGuard, BackendError> {
    install_native_keyring_with(native_platform_store)
}

fn install_native_keyring_with(
    constructor: PlatformStoreConstructor,
) -> Result<NativeKeyringGuard, BackendError> {
    // Reuse an active lease; otherwise call cfg-gated platform_store_with.
}

#[cfg(target_os = "windows")]
fn platform_store_with(
    constructor: PlatformStoreConstructor,
) -> Result<Arc<keyring_core::CredentialStore>, BackendError>;
```

- [ ] Define equivalent macOS/Linux `platform_store_with` functions so each applies the current OS-specific `classify_platform_store_error` label.
- [ ] Make `native_platform_store` the only function that calls the real OS backend constructor.
- [ ] Preserve the existing lease mutex recovery, one-to-two reuse, and one-to-zero unset behavior.

Run:

```powershell
cargo test -p opi-coding-agent --lib native_keyring_host_selection_installs_a_default_store
cargo test -p opi-coding-agent --lib native_keyring::
cargo clippy --workspace --all-targets -- -D warnings
```

Expected GREEN: the filtered acceptance command selects one test and all commands exit 0.

### Commit

- [ ] Stage and commit only:

```powershell
git add crates/opi-coding-agent/src/native_keyring.rs
git diff --cached --name-only
git commit -m "fix(opi-coding-agent): test native keyring host selection"
```

## Task 14.15: WireApi, Model Metadata, Pricing, Thinking, and Canonical IDs

**Files**

- Create: `crates/opi-ai/src/model_info.rs`
- Create: `crates/opi-ai/tests/model_wire_metadata.rs`
- Create: `crates/opi-coding-agent/tests/provider_identity.rs`
- Modify: `crates/opi-ai/src/lib.rs`
- Modify: `crates/opi-ai/src/provider.rs`
- Modify: `crates/opi-ai/src/registry.rs`
- Modify: `crates/opi-ai/src/auth.rs`
- Modify: `crates/opi-ai/src/stream.rs`
- Modify: `crates/opi-ai/src/anthropic.rs`
- Modify: `crates/opi-ai/src/azure_openai.rs`
- Modify: `crates/opi-ai/src/bedrock/mod.rs`
- Modify: `crates/opi-ai/src/gemini.rs`
- Modify: `crates/opi-ai/src/mistral.rs`
- Modify: `crates/opi-ai/src/openai_chat.rs`
- Modify: `crates/opi-ai/src/openai_responses.rs`
- Modify: `crates/opi-ai/src/openrouter.rs`
- Modify: `crates/opi-ai/src/test_support.rs`
- Modify: `crates/opi-ai/src/vertex.rs`
- Modify: `crates/opi-ai/tests/auth_contracts.rs`
- Modify: `crates/opi-ai/tests/oauth_wire_shape.rs`
- Modify: `crates/opi-agent/src/session_event.rs`
- Modify: `crates/opi-agent/tests/image_capability.rs`
- Modify: `crates/opi-coding-agent/src/adapter_extension.rs`
- Modify: `crates/opi-coding-agent/src/harness.rs`
- Modify: `crates/opi-coding-agent/src/session_coordinator.rs`
- Modify: `crates/opi-coding-agent/src/pricing.rs`
- Modify: `crates/opi-coding-agent/src/provider_factory.rs`
- Modify: `crates/opi-coding-agent/src/oauth.rs`
- Modify: `crates/opi-coding-agent/tests/credential_store.rs`
- Modify: `crates/opi-coding-agent/tests/doctor_cli.rs`
- Modify: `crates/opi-coding-agent/tests/interactive_auth.rs`
- Modify every additional file listed in **ModelInfo construction migration** below.

### RED: public metadata and validation

- [ ] Add `model_wire_metadata.rs` tests:
  - `wire_api_serializes_exact_reviewed_names`
  - `model_info_public_constructor_sets_required_wire`
  - `model_info_rejects_wire_compat_mismatch`
  - `pricing_tier_uses_strict_greater_than_boundary`
  - `pricing_rejects_non_finite_negative_duplicate_and_unsorted_tiers`
  - `unsupported_thinking_level_is_rejected_before_request_build`
  - `every_builtin_model_declares_exact_wire`
- [ ] Make the external-consumer test use only public `opi_ai` imports and `ModelInfo::new`; no struct literal or crate-private helper.
- [ ] Add `provider_identity.rs` tests:
  - `canonical_oauth_provider_ids_are_exact`
  - `development_provider_ids_are_rejected_without_alias_or_migration`
  - `credential_needed_remediation_uses_canonical_provider_id`
- [ ] Extend session/runtime tests:
  - `embedded_model_pricing_overrides_legacy_fallback`;
  - `embedded_model_pricing_updates_on_model_switch_and_resume`;
  - `legacy_pricing_fallback_remains_for_unmigrated_model`;
  - `thinking_level_additions_round_trip_without_schema_bump`.

Run:

```powershell
cargo test -p opi-ai --test model_wire_metadata
cargo test -p opi-coding-agent --test provider_identity
cargo test -p opi-coding-agent --test session_runtime embedded_model_pricing_
```

Expected RED: missing `WireApi`, builders, selected thinking level, canonical ids, or embedded session pricing.

### GREEN: model and thinking foundation

- [ ] Implement the fixed contracts from **Fixed Public Contracts** in `model_info.rs`.
- [ ] Implement `Display` and `FromStr` for all eight exact `WireApi` values.
- [ ] Make `WireCompat::wire_api()` available for validation.
- [ ] Put all current OpenAI Chat compatibility switches in `OpenAiCompletionsCompat`:
  - `system_role_override`
  - `max_tokens_field`
  - `tool_result_name_field`
  - `usage_in_stream`
  - `strict_tool_schema`
  - `reasoning_effort`
  - `cache_key`
  - `send_session_affinity_headers`
  - `require_assistant_after_tool_result`
  - `chat_completions_path`
  - pi catalog flags `supports_store`, `supports_developer_role`, and `supports_reasoning_effort`.
- [ ] Put pi Anthropic flags in `AnthropicMessagesCompat`:
  - `supports_eager_tool_input_streaming`
  - `force_adaptive_thinking`
  - `supports_temperature`.
- [ ] Put standard Responses flags in `OpenAiResponsesCompat`; keep Codex-specific behavior out of it.
- [ ] Re-export `ModelInfo` through `opi_ai::provider` and `ModelCapabilities` through `opi_ai::registry` while making `model_info` their definition site.
- [ ] Move the canonical `ThinkingLevel` to `opi-ai`; re-export it from `opi_agent::session_event`.
- [ ] Preserve `ThinkingLevel::None` serialization as `none` and map CLI/TUI spelling `off` to it.
- [ ] Update `ThinkingConfig` constructors/defaults so disabled is `(enabled=false, budget=None, level=None)` and existing enabled defaults select `Medium`.
- [ ] Validate the selected level against `ModelInfo::thinking_level_map` in the existing capability-validation path before any provider builds a body.
- [ ] Preserve Anthropic's budget-based thinking behavior after level resolution.

### GREEN: pricing authority

- [ ] Implement `ModelPricing::validate` and `ModelPricing::effective`.
- [ ] Leave `calculate_cost` strict and unchanged except for accepting the already-selected `Pricing`.
- [ ] Add `Option<ModelPricing>` to `SessionCoordinator`.
- [ ] Add:

```rust
pub fn set_cost_model(
    &mut self,
    model_spec: impl Into<String>,
    pricing: Option<ModelPricing>,
);
```

- [ ] On new session, resume, fork, `set_model`, and `set_model_validated`, resolve the active `ModelInfo` and update the coordinator's model/pricing together.
- [ ] In `cost_summary`, use embedded pricing first, then `lookup_pricing(&self.model)` only when embedded pricing is absent.
- [ ] Never serialize pricing or computed costs into session JSONL.

### GREEN: exact wire migration

Assign these wires to every existing built-in catalog:

| Provider implementation | `WireApi` |
|---|---|
| Anthropic | `AnthropicMessages` |
| OpenAI Chat | `OpenAiCompletions` |
| OpenAI Responses | `OpenAiResponses` |
| OpenRouter | `OpenAiCompletions` |
| Mistral | `OpenAiCompletions` |
| Direct Gemini | `GoogleGenerativeAi` |
| Vertex | `GoogleVertex` |
| Bedrock | `BedrockConverseStream` |
| Azure Chat implementation | `AzureOpenAiCompletions` |

- [ ] Migrate every `ModelInfo { ... }` in source and tests to `ModelInfo::new(...).with_*`.
- [ ] Remove direct `ModelInfo` struct literals so future required metadata cannot be skipped.
- [ ] Keep `ApiKind` unchanged; it still describes normalized assistant-message source rather than transport selection.

**ModelInfo construction migration**

```text
crates/opi-agent/tests/image_capability.rs
crates/opi-ai/src/anthropic.rs
crates/opi-ai/src/azure_openai.rs
crates/opi-ai/src/bedrock/mod.rs
crates/opi-ai/src/gemini.rs
crates/opi-ai/src/mistral.rs
crates/opi-ai/src/openai_chat.rs
crates/opi-ai/src/openai_responses.rs
crates/opi-ai/src/openrouter.rs
crates/opi-ai/src/provider.rs
crates/opi-ai/src/test_support.rs
crates/opi-ai/src/vertex.rs
crates/opi-ai/tests/custom_provider_registration.rs
crates/opi-ai/tests/model_capabilities_migration.rs
crates/opi-ai/tests/openai_chat_fixtures.rs
crates/opi-ai/tests/provider_collection.rs
crates/opi-ai/tests/provider_diagnostics.rs
crates/opi-ai/tests/provider_trait.rs
crates/opi-ai/tests/registry.rs
crates/opi-ai/tests/request_enrichment.rs
crates/opi-coding-agent/src/adapter_extension.rs
crates/opi-coding-agent/src/provider_factory.rs
crates/opi-coding-agent/tests/anthropic_cache_markers.rs
crates/opi-coding-agent/tests/custom_provider_registration.rs
crates/opi-coding-agent/tests/extensions.rs
crates/opi-coding-agent/tests/list_models.rs
crates/opi-coding-agent/tests/oauth_auth.rs
crates/opi-coding-agent/tests/picker_integration.rs
crates/opi-coding-agent/tests/rpc_jsonl.rs
crates/opi-coding-agent/tests/session_runtime.rs
```

After migration:

```powershell
rg -n "ModelInfo\s*\{" crates
```

Expected: only the type definition, its own implementation/destructuring, and deliberately documented compile-fail text; no construction literal.

### GREEN: provider-id migration

- [ ] Change built-in provider id, OAuth registry id, keychain key, model spec, diagnostic, and remediation text:
  - `copilot` -> `github-copilot`
  - `codex` -> `openai-codex`
- [ ] Do not accept old ids in parsing, registry lookup, listing, login, logout, or stored-credential lookup.
- [ ] Do not read, copy, delete, or migrate an old keychain account.
- [ ] Update tests and test fixtures using development ids; defer public documentation prose to 14.21.
- [ ] Expand `ResolvedAuth`; static resolvers return both new fields as `None`, existing stored OAuth exposes `base_url`, and `account_id` remains `None` until 14.18.
- [ ] Add typed, non-retryable:

```rust
ProviderError::UnknownModel { provider_id: String, model_id: String }
ProviderError::MissingWireRoute { provider_id: String, wire_api: WireApi }
ProviderError::WireCompatMismatch {
    model_id: String,
    wire_api: WireApi,
    compat_wire: WireApi,
}
```

Classify unknown model as request error; missing route and mismatch as configuration errors.

### Verify

```powershell
cargo test -p opi-ai --test model_wire_metadata
cargo test -p opi-ai --test model_capabilities_migration
cargo test -p opi-ai --test request_enrichment
cargo test -p opi-ai --test provider_collection
cargo test -p opi-agent --test image_capability
cargo test -p opi-coding-agent --test provider_identity
cargo test -p opi-coding-agent --test session_runtime embedded_model_pricing_
cargo test -p opi-coding-agent --test rpc_jsonl rpc_set_thinking_
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: every filter selects at least one test; all commands exit 0.

### Commit

- [ ] Stage only the files listed for 14.15, inspect the staged list, then:

```powershell
git commit -m "feat(opi-ai): add wire-aware model metadata"
```

## Task 14.16: ApiMappedProvider and TOML Custom Providers

**Files**

- Create: `crates/opi-ai/src/api_mapped.rs`
- Create: `crates/opi-ai/src/provider_headers.rs`
- Create: `crates/opi-ai/tests/api_mapped_provider.rs`
- Create: `crates/opi-coding-agent/tests/custom_provider_map.rs`
- Modify: `crates/opi-ai/src/lib.rs`
- Modify: `crates/opi-ai/src/provider.rs`
- Modify: `crates/opi-ai/src/anthropic.rs`
- Modify: `crates/opi-ai/src/openai_chat.rs`
- Modify: `crates/opi-ai/src/openai_responses.rs`
- Modify: `crates/opi-coding-agent/src/config.rs`
- Modify: `crates/opi-coding-agent/src/provider_factory.rs`
- Modify: `crates/opi-coding-agent/tests/config_tests.rs`
- Modify: `crates/opi-coding-agent/tests/provider_factory.rs`
- Modify: `crates/opi-coding-agent/tests/list_models.rs`

### RED: mapped routing

- [ ] Add `api_mapped_provider.rs` tests:
  - `mapped_provider_dispatches_one_catalog_across_three_wires`
  - `mapped_routes_share_one_lazy_auth_resolver`
  - `mapped_provider_re_resolves_auth_for_every_stream`
  - `unknown_model_fails_before_route_or_network`
  - `missing_route_fails_at_construction`
  - `wire_compat_mismatch_fails_at_construction`
  - `mapped_provider_refresh_is_static_none`
  - `mapped_provider_rejects_duplicate_models_routes_and_route_id_mismatch`
- [ ] Use recording route providers and an atomic-counting resolver. Do not weaken the production trait to expose test counters.

Run:

```powershell
cargo test -p opi-ai --test api_mapped_provider
```

Expected RED: `ApiMappedProvider` and route validation do not exist.

### GREEN: ApiMappedProvider

Implement:

```rust
pub struct ApiMappedProvider {
    id: String,
    models: Vec<ModelInfo>,
    routes: BTreeMap<WireApi, Box<dyn Provider>>,
}

impl ApiMappedProvider {
    pub fn try_new(
        id: impl Into<String>,
        models: Vec<ModelInfo>,
        routes: BTreeMap<WireApi, Box<dyn Provider>>,
    ) -> Result<Self, ApiMapError>;
}
```

- [ ] Reject empty id/model ids, duplicate model ids, duplicate routes, route provider-id mismatch, model validation errors, and any catalog wire without exactly one route.
- [ ] Resolve the unqualified model id from `provider:model`; reject a different provider prefix.
- [ ] On `stream`, locate the model, select `model.wire_api`, and delegate the unchanged request to the route.
- [ ] Each route receives only its catalog subset, so its model-derived base URL, thinking map, compatibility, and pricing lookup remain deterministic.
- [ ] The mapped layer never resolves auth. Every route receives the same `Arc<dyn AuthResolver>` and resolves once immediately before its HTTP request.
- [ ] `refresh_models()` returns `Ok(None)`.

### GREEN: route construction and header separation

- [ ] Add a `ProviderHeaders` value that keeps provider-configured/static/dynamic headers separate from `Request::extra_headers`.
- [ ] Validate `Request::extra_headers` before merging; a request cannot override auth or provider-managed header names.
- [ ] Validate configured provider headers with reqwest/http header parsing and reject:
  - empty or invalid names;
  - invalid values;
  - `authorization`, `x-api-key`, `api-key`, `anthropic-version`, `content-type`, `chatgpt-account-id`, and other concrete route-managed names.
- [ ] Generalize Anthropic, OpenAI Chat, and standard Responses constructors to accept:
  - mapped provider id;
  - route catalog subset;
  - shared `Arc<dyn AuthResolver>`;
  - provider default base URL;
  - `ProviderHeaders`;
  - shared HTTP client.
- [ ] Resolve base URL at stream time with exact precedence:

```text
ResolvedAuth.base_url > ModelInfo.base_url > provider default base_url
```

- [ ] Make Anthropic Bearer behavior configurable by route identity: direct Anthropic keeps its reviewed OAuth beta header; GitHub Copilot/custom Bearer does not inherit Anthropic-account OAuth headers.
- [ ] Make all three routes map Bearer 401/403 to `CredentialRevoked`; static API-key failures retain existing direct-provider classification.

### RED: TOML schema and final-merge validation

- [ ] Add `custom_provider_map.rs` and config tests for:
  - `custom_provider_api_and_base_url_precedence` covers provider API inheritance plus model API/base-URL overrides;
  - three routes under one provider identity;
  - one shared env credential source;
  - thinking-map identity/alias/unsupported values;
  - wire-specific compat decoding;
  - pricing and strict threshold tiers;
  - `custom_mapped_provider_lists_one_identity` in `list_models.rs` proves list-models/picker shows one provider, never hidden route ids;
  - `[providers.openai_compatible]` lowers through `ApiMappedProvider`.
- [ ] Add `invalid_custom_provider_contracts_fail_at_load` covering unknown/disabled wires, missing API, duplicate models, missing routes, mismatched compat, invalid token limits, invalid tiers, invalid headers, reserved headers, and invalid auth/wire combinations.
- [ ] Add `custom_provider_final_merge_validates_once` where a lower layer is incomplete but the final merged user -> project -> CLI config is valid.

Run:

```powershell
cargo test -p opi-coding-agent --test custom_provider_map
cargo test -p opi-coding-agent --test config_tests custom_provider_
```

Expected RED: no `[providers.custom]` schema or mapped factory path exists.

### GREEN: TOML contract

Implement `[providers.custom.<id>]`:

```toml
[providers.custom.acme]
name = "Acme"
base_url = "https://api.acme.example"
api_key_env = "ACME_API_KEY"
auth_scheme = "bearer"
api = "openai-completions"
headers = { "X-Acme" = "opi" }

[[providers.custom.acme.models]]
id = "claude-model"
display_name = "Claude Model"
api = "anthropic-messages"
base_url = "https://api.acme.example/anthropic"
context_window = 200000
max_output_tokens = 32000
supports_images = true
supports_streaming = true
supports_thinking = true
thinking_level_map = { off = true, minimal = "low", xhigh = false, max = false }

[providers.custom.acme.models.compat]
api = "anthropic-messages"
supports_eager_tool_input_streaming = false

[providers.custom.acme.models.pricing]
input = 3.0
output = 15.0
cache_read = 0.3
cache_write = 3.75

[[providers.custom.acme.models.pricing.tiers]]
input_tokens_above = 272000
input = 6.0
output = 22.5
cache_read = 0.6
cache_write = 7.5
```

- [ ] Provider fields own shared name, credential env, auth scheme, default base URL, proxy, headers, and optional default API.
- [ ] Model fields may override API, base URL, capabilities, thinking map, matching compat, and pricing.
- [ ] Accept only `anthropic-messages`, `openai-completions`, and `openai-responses`.
- [ ] Reject `openai-codex-responses` and all other built-in-only wires from TOML.
- [ ] Bearer auth is legal on all three custom wires. API-key auth is legal only when the provider's route set is entirely Anthropic Messages; Chat/Responses require Bearer.
- [ ] Parse every layer to raw mergeable values, merge user -> project -> explicit config, then validate the final `OpiConfig` once.
- [ ] Keep `load_config_file(path)` strict by validating that standalone final document.
- [ ] Add typed `ConfigError` variants carrying provider/model/field without secret values.
- [ ] Lower existing `[providers.openai_compatible]` profiles to one-model-or-more `ApiMappedProvider` with `WireApi::OpenAiCompletions`, Bearer auth, and the existing compatibility settings.
- [ ] Remove the duplicate direct dispatcher for OpenAI-compatible profiles.

### Verify

```powershell
cargo test -p opi-ai --test api_mapped_provider
cargo test -p opi-coding-agent --test custom_provider_map
cargo test -p opi-coding-agent --test config_tests custom_provider_
cargo test -p opi-coding-agent --test provider_factory openai_compatible_
cargo test -p opi-coding-agent --test list_models custom_mapped_provider_
cargo clippy --workspace --all-targets -- -D warnings
```

### Commit

- [ ] Stage only 14.16 paths and commit:

```powershell
git commit -m "feat(opi-ai): add mapped provider routing"
```

## Task 14.17: GitHub Copilot Three-Wire Catalog

**Files**

- Create: `crates/opi-coding-agent/src/github_copilot.rs`
- Create: `crates/opi-coding-agent/tests/github_copilot_provider.rs`
- Create: `crates/opi-coding-agent/tests/fixtures/pi-0.80.6/github-copilot.models.json`
- Modify: `crates/opi-coding-agent/src/lib.rs`
- Modify: `crates/opi-coding-agent/src/provider_factory.rs`
- Modify: `crates/opi-coding-agent/src/oauth.rs`
- Modify: `crates/opi-ai/src/provider.rs`
- Modify the three route modules only as needed for provider-header policy:
  - `crates/opi-ai/src/anthropic.rs`
  - `crates/opi-ai/src/openai_chat.rs`
  - `crates/opi-ai/src/openai_responses.rs`
- Modify: `crates/opi-coding-agent/tests/oauth_auth.rs`
- Modify: `crates/opi-coding-agent/tests/list_models.rs`
- Modify: `crates/opi-coding-agent/tests/provider_factory.rs`

### RED: offline catalog provenance

- [ ] Check in a normalized JSON fixture with:

```json
{
  "pi_version": "0.80.6",
  "source_path": "packages/ai/src/providers/github-copilot.models.ts",
  "source_sha256": "6FE91A9895552B56F882428F124466DFBB08CE27F4D4CE0ED0C5F23168517EFA",
  "provider_id": "github-copilot",
  "default_base_url": "https://api.individual.githubcopilot.com",
  "models": []
}
```

Populate `models` with all 25 source records and all runtime-affecting metadata: id, display name, wire, base URL, capabilities/limits, input modes, thinking map, matching compat, headers, and pricing.

- [ ] Add tests:
  - `github_copilot_catalog_matches_pi_0806_fixture`
  - `github_copilot_catalog_has_25_models_and_three_wires`
  - `github_copilot_model_listing_is_static_and_secret_free`

The wire membership is exact:

| Wire | Model ids |
|---|---|
| `openai-completions` | `claude-fable-5`, `gemini-2.5-pro`, `gemini-3-flash-preview`, `gemini-3.1-pro-preview`, `gemini-3.5-flash`, `gpt-4.1`, `kimi-k2.7-code`, `mai-code-1-flash-picker` |
| `anthropic-messages` | `claude-haiku-4.5`, `claude-opus-4.5`, `claude-opus-4.6`, `claude-opus-4.7`, `claude-opus-4.8`, `claude-sonnet-4`, `claude-sonnet-4.5`, `claude-sonnet-4.6`, `claude-sonnet-5` |
| `openai-responses` | `gpt-5-mini`, `gpt-5.2`, `gpt-5.2-codex`, `gpt-5.3-codex`, `gpt-5.4`, `gpt-5.4-mini`, `gpt-5.4-nano`, `gpt-5.5` |

Run:

```powershell
cargo test -p opi-coding-agent --test github_copilot_provider github_copilot_catalog_
```

Expected RED: no Rust catalog/fixture parity implementation exists.

### RED: wire behavior

- [ ] Add wiremock tests:
  - `github_copilot_anthropic_model_posts_v1_messages_with_bearer`
  - `github_copilot_chat_model_posts_chat_completions`
  - `github_copilot_responses_model_posts_responses`
  - `github_copilot_headers_match_reviewed_static_contract`
  - `github_copilot_initiator_tracks_last_user_or_agent_message`
  - `github_copilot_vision_header_covers_user_and_tool_result_images`
  - `github_copilot_next_stream_observes_changed_token_and_enterprise_base_url`
  - `github_copilot_401_and_403_are_revoked_on_every_wire`
- [ ] Add `factory_routes_github_copilot_models_by_declared_wire` to `oauth_auth.rs`.
- [ ] Add `github_copilot_factory_builds_one_three_wire_provider` to `provider_factory.rs`.
- [ ] Add `github_copilot_static_catalog_lists_without_store_reads` to `list_models.rs`.

Expected static headers:

```text
User-Agent: GitHubCopilotChat/0.35.0
Editor-Version: vscode/1.107.0
Editor-Plugin-Version: copilot-chat/0.35.0
Copilot-Integration-Id: vscode-chat
```

Expected dynamic headers:

```text
X-Initiator: user|agent
Openai-Intent: conversation-edits
Copilot-Vision-Request: true   # only when user or tool-result content has an image
```

Run:

```powershell
cargo test -p opi-coding-agent --test github_copilot_provider github_copilot_
```

Expected RED: factory still exposes a single Chat route and incomplete headers.

### GREEN

- [ ] Build one `ApiMappedProvider` with id `github-copilot`, the exact 25-model catalog, and three route providers.
- [ ] Give all routes the same `Arc<CredentialResolver>` using `AuthSource::Store("github-copilot")`.
- [ ] Use `ResolvedAuth.base_url` for per-stream enterprise routing.
- [ ] Use Bearer on all routes, including Anthropic Messages; never emit `x-api-key`.
- [ ] Extend `Request::contains_image_input` or replace it with `contains_image_content` so it checks user and tool-result image content.
- [ ] Derive `X-Initiator` from the final message: user -> `user`, all other roles -> `agent`.
- [ ] Apply the exact static/dynamic headers through provider-owned header policy, after rejecting request attempts to override them.
- [ ] Map 401 and 403 on each route to non-retryable `CredentialRevoked { provider_id: "github-copilot" }`.
- [ ] Keep list-models and picker catalog construction independent of credential-store reads.
- [ ] Do not call Copilot live entitlement/model-enable endpoints.

### Verify

```powershell
cargo test -p opi-coding-agent --test github_copilot_provider
cargo test -p opi-coding-agent --test oauth_auth factory_routes_github_copilot_
cargo test -p opi-coding-agent --test list_models github_copilot_
cargo test -p opi-coding-agent --test provider_factory github_copilot_
cargo clippy --workspace --all-targets -- -D warnings
```

### Commit

- [ ] Stage only 14.17 paths and commit:

```powershell
git commit -m "feat(opi-coding-agent): add GitHub Copilot catalog"
```

## Task 14.18: OpenAI Codex Dedicated Wire, Catalog, and Dual Login

**Files**

- Create: `crates/opi-ai/src/openai_responses_shared.rs`
- Create: `crates/opi-ai/src/openai_codex_responses.rs`
- Create: `crates/opi-ai/tests/openai_codex_responses.rs`
- Create: `crates/opi-coding-agent/src/openai_codex.rs`
- Create: `crates/opi-coding-agent/tests/openai_codex_provider.rs`
- Create: `crates/opi-coding-agent/tests/fixtures/pi-0.80.6/openai-codex.models.json`
- Modify: `crates/opi-ai/src/lib.rs`
- Modify: `crates/opi-ai/src/openai_responses.rs`
- Modify: `crates/opi-ai/src/auth.rs`
- Modify: `crates/opi-ai/src/credential.rs`
- Modify: `crates/opi-coding-agent/src/credential_store.rs`
- Modify: `crates/opi-coding-agent/src/oauth.rs`
- Modify: `crates/opi-coding-agent/src/provider_factory.rs`
- Modify: `crates/opi-coding-agent/src/lib.rs`
- Modify: `crates/opi-coding-agent/tests/oauth_auth.rs`
- Modify: `crates/opi-coding-agent/tests/credential_store.rs`
- Modify: `crates/opi-coding-agent/tests/list_models.rs`
- Modify: `crates/opi-coding-agent/tests/provider_factory.rs`

### RED: catalog and dedicated wire

- [ ] Check in a normalized fixture with:

```json
{
  "pi_version": "0.80.6",
  "source_path": "packages/ai/src/providers/openai-codex.models.ts",
  "source_sha256": "5F4E155179DA36F67177C18181FB6E23AB884D75126A983310456EA60AFFDEED",
  "provider_id": "openai-codex",
  "default_base_url": "https://chatgpt.com/backend-api",
  "models": []
}
```

Populate the exact seven records:

```text
gpt-5.3-codex-spark
gpt-5.4
gpt-5.4-mini
gpt-5.5
gpt-5.6-luna
gpt-5.6-sol
gpt-5.6-terra
```

Copy exact display names, image capabilities, context/max-output limits, thinking maps, pricing, and tiers. `gpt-5.4`, `gpt-5.5`, and all three `gpt-5.6-*` models use the exact `inputTokensAbove = 272000` tier from pi.

- [ ] Add:
  - `openai_codex_catalog_matches_pi_0806_fixture`
  - `openai_codex_catalog_uses_only_dedicated_wire`
  - `openai_codex_pricing_tiers_keep_equality_on_base_rate`
- [ ] Add `dedicated_codex_request_uses_exact_base_path_body_and_headers` to the `opi-ai` wire test target.
- [ ] Add `openai_codex_factory_uses_dedicated_provider` to `provider_factory.rs`.
- [ ] Add `openai_codex_static_catalog_lists_without_store_reads` to `list_models.rs`.
- [ ] Add a source guard that fails if the Codex factory references `OpenAiResponsesProvider` or the standard module exposes Codex toggle fields.

Run:

```powershell
cargo test -p opi-coding-agent --test openai_codex_provider openai_codex_catalog_
cargo test -p opi-ai --test openai_codex_responses dedicated_codex_
```

Expected RED: dedicated module and exact catalog do not exist.

### GREEN: dedicated Codex Responses provider

- [ ] Extract only wire-neutral message/tool conversion and SSE event parsing from `openai_responses.rs` into `openai_responses_shared.rs`.
- [ ] Keep standard OpenAI Responses behavior in `openai_responses.rs`.
- [ ] Remove standard-provider Codex flags, Codex path selection, and JWT/account extraction.
- [ ] Implement `OpenAiCodexResponsesProvider` with:
  - `WireApi::OpenAiCodexResponses`
  - default base `https://chatgpt.com/backend-api`
  - endpoint `/codex/responses`
  - lazy `ResolvedAuth`
  - strict requirement for `account_id`
  - shared low-level SSE mapping only.
- [ ] Build the request body with current opi-supported fields:

```text
model
store = false
stream = true
instructions
input
tools with strict = null
tool_choice = "auto"
parallel_tool_calls = true
reasoning.effort
reasoning.summary
text.verbosity = "low"
include = ["reasoning.encrypted_content"]
prompt_cache_key
```

- [ ] Send exact headers:

```text
Authorization: Bearer <secret>
chatgpt-account-id: <account_id>
originator: opi
OpenAI-Beta: responses=experimental
accept: text/event-stream
session-id: <request session id or generated UUIDv7>
x-client-request-id: <fresh UUIDv7>
```

- [ ] Use the existing workspace `uuid` dependency already enabled with feature `v7`.
- [ ] Resolve base URL lazily; auth `base_url` still outranks model/default if a future stored credential supplies it.
- [ ] Map 401/403 to `CredentialRevoked { provider_id: "openai-codex" }`.
- [ ] Keep retry/timeout/provider-side classifications typed and redacted.

### RED: account-id persistence

- [ ] Extend credential tests:
  - `oauth_envelope_round_trips_optional_account_id`
  - `legacy_oauth_envelope_without_account_id_still_decodes`
  - `codex_login_rejects_token_without_chatgpt_account_id`
  - `codex_refresh_rejects_token_without_chatgpt_account_id`
  - `codex_resolver_propagates_account_id_without_secret_logging`
- [ ] Use synthetic JWT payloads only; never place real-format live tokens in fixtures or output assertions.

### GREEN: account-id persistence

- [ ] Add `account_id: Option<String>` to `OAuthCredential`.
- [ ] Add optional `account_id` to `Credential::OAuthToken` and the existing version-1 persisted envelope without bumping envelope/schema version.
- [ ] Preserve decode compatibility for envelopes where the field is absent.
- [ ] Parse JWT payload claim:

```text
https://api.openai.com/auth -> chatgpt_account_id
```

- [ ] Require a non-empty account id after Codex login and refresh. Return a dedicated typed, non-retryable, redacted error; do not defer failure to the HTTP provider.
- [ ] Anthropic and GitHub Copilot write `account_id = None`.
- [ ] Ensure all `Debug`, `Display`, diagnostics, and persisted-test captures redact access token, refresh token, authorization code, device verifier, JWT, and envelope payload.

### RED: login-method selector and Device Code

- [ ] Add `OAuthLoginMethod` and this presenter method:

```rust
fn select_login_method<'a>(
    &'a self,
    provider_id: &'a str,
    methods: &'a [OAuthLoginMethod],
    default: OAuthLoginMethod,
) -> BoxAuthFuture<'a, Result<OAuthLoginMethod, ProviderError>>;
```

- [ ] Add typed `ProviderError::LoginCancelled { provider_id: String }`.
- [ ] Extend all existing mock/presenter implementations with deterministic defaults. Anthropic and Copilot must never call the selector.
- [ ] Add focused tests:
  - `openai_codex_browser_is_default_and_preserves_pkce_manual_race`
  - `openai_codex_device_code_success_exchanges_authorization_code`
  - `openai_codex_device_code_pending_then_success`
  - `openai_codex_device_code_slow_down_increases_poll_delay`
  - `openai_codex_device_code_denial_is_typed_and_redacted`
  - `openai_codex_device_code_expiry_is_typed_and_redacted`
  - `openai_codex_device_code_timeout_is_15_minutes_under_paused_time`
  - `openai_codex_device_code_cancellation_writes_nothing`
  - `openai_codex_device_code_never_calls_await_manual_code`

Device Code contract:

```text
client id:         app_EMoamEEZ73f0CkXaXp7hrann
user-code POST:    https://auth.openai.com/api/accounts/deviceauth/usercode
poll POST:         https://auth.openai.com/api/accounts/deviceauth/token
verification URI:  https://auth.openai.com/codex/device
device redirect:   https://auth.openai.com/deviceauth/callback
token POST:        https://auth.openai.com/oauth/token
timeout:           15 minutes
```

Poll request JSON contains `device_auth_id` and `user_code`. A successful poll returns `authorization_code` and `code_verifier`; exchange them at the token endpoint with the device redirect URI. Treat HTTP 403/404 and `deviceauth_authorization_pending` as pending; honor `slow_down` without exceeding the total budget.

### Verify

```powershell
cargo test -p opi-ai --test openai_codex_responses
cargo test -p opi-coding-agent --test openai_codex_provider
cargo test -p opi-coding-agent --test oauth_auth openai_codex_
cargo test -p opi-coding-agent --test credential_store oauth_envelope_
cargo test -p opi-coding-agent --test list_models openai_codex_
cargo test -p opi-coding-agent --test provider_factory openai_codex_
cargo clippy --workspace --all-targets -- -D warnings
```

### Commit

- [ ] Stage only 14.18 paths and commit:

```powershell
git commit -m "feat(opi-coding-agent): align OpenAI Codex provider"
```

## Task 14.19: Concrete OAuth Dispatcher Vertical Path

**Files**

- Modify: `crates/opi-coding-agent/src/oauth.rs`
- Modify: `crates/opi-coding-agent/src/interactive_auth.rs`
- Modify: `crates/opi-coding-agent/src/harness.rs`
- Modify: `crates/opi-coding-agent/src/provider_factory.rs`
- Modify: `crates/opi-coding-agent/tests/interactive_auth.rs`
- Modify: `crates/opi-coding-agent/tests/oauth_auth.rs`

### RED

- [ ] Replace fake-registry acceptance coverage with tests that begin at:

```rust
dispatch_auth_command("/login <canonical-id>", ...)
dispatch_auth_command("/logout <canonical-id>", ...)
```

- [ ] The tests may replace only endpoints, HTTP client, locked credential store, presenter, terminal control, and clock/timing.
- [ ] Use the production registry builder and concrete `AnthropicOAuthProvider`, `CopilotOAuthProvider`, and `CodexOAuthProvider`.
- [ ] Cover:
  - `dispatcher_runs_concrete_anthropic_login_and_logout`
  - `dispatcher_runs_concrete_github_copilot_login_and_logout`
  - `dispatcher_runs_concrete_openai_codex_browser_login_and_logout`
  - `dispatcher_runs_concrete_openai_codex_device_login_and_logout`
  - `dispatcher_restores_terminal_once_on_every_concrete_exit`
  - `dispatcher_store_and_lock_failures_are_typed_redacted_and_unsuccessful`
  - `dispatcher_device_flows_never_call_manual_code`

Run:

```powershell
cargo test -p opi-coding-agent --test interactive_auth dispatcher_runs_concrete_
```

Expected RED: `AuthCommandServices` still accepts a prebuilt fake registry and concrete providers own hard-coded HTTP clients/endpoints.

### GREEN: injectable production services

- [ ] Add package-private `OAuthEndpointConfig` structs for Anthropic, Copilot, and Codex. `production()` returns the current reviewed constants.
- [ ] Include both Codex browser and device endpoints plus separate 15-minute Device Code budget.
- [ ] Make each concrete OAuth provider accept a shared `reqwest::Client` and its endpoint config; production construction remains network-equivalent.
- [ ] Add:

```rust
impl OAuthProviderRegistry {
    pub(crate) fn registry_with_services(
        endpoints: &OAuthEndpointConfig,
        client: reqwest::Client,
    ) -> Self;
}
```

- [ ] `registry_with_builtins()` delegates to `registry_with_services(production, production_client)`.
- [ ] Change `AuthCommandServices` to carry endpoint config, HTTP client, locked store, and presenter rather than a caller-supplied registry.
- [ ] Make `dispatch_auth_command` construct the real registry through `registry_with_services`.
- [ ] Keep logout validation against the production registry's exact ids.
- [ ] Preserve `LoginTerminalGuard` RAII. Suspension/restoration occurs exactly once on success, provider failure, selection cancellation, presenter failure, OAuth timeout, store/lock failure, and cancellation/drop.
- [ ] Mount wiremock endpoints for the complete concrete flows and assert exact URL, method, headers/body, stored profile, and deletion.
- [ ] Assert no success notification occurs before locked persistence completes.
- [ ] Delete or downgrade old fake-provider tests that duplicate the vertical scenario; retain focused parser/guard unit tests where they still add distinct value.

### Verify

```powershell
cargo test -p opi-coding-agent --test interactive_auth dispatcher_runs_concrete_
cargo test -p opi-coding-agent --test interactive_auth dispatcher_restores_terminal_once_
cargo test -p opi-coding-agent --test interactive_auth dispatcher_store_and_lock_failures_
cargo test -p opi-coding-agent --test interactive_auth dispatcher_device_flows_
cargo test -p opi-coding-agent --test oauth_auth registry_with_builtins_
cargo clippy --workspace --all-targets -- -D warnings
```

### Commit

- [ ] Stage only 14.19 paths and commit:

```powershell
git commit -m "test(opi-coding-agent): cover concrete OAuth dispatch"
```

## Task 14.20: Outer TUI Credential Retry

**Files**

- Modify: `crates/opi-coding-agent/src/interactive.rs`
- Modify: `crates/opi-coding-agent/src/interactive_auth.rs`
- Modify: `crates/opi-coding-agent/tests/interactive_auth.rs`
- Create: `crates/opi-coding-agent/tests/interactive_tui_auth.rs`

### RED

- [ ] Add tests that call the public `run_interactive_tui` entry point through the debug-only scripted adapter.
- [ ] Script a normal prompt first, then `/login anthropic`, then exit.
- [ ] Use `MockProvider` behavior: first provider call returns pre-output `CredentialNeeded("anthropic")`; retry returns success.
- [ ] Name the positive acceptance test `outer_tui_same_provider_login_retries_pending_turn_once`.
- [ ] Capture and assert:

```text
user_messages = 1
provider_calls = 2
retries = 1
```

- [ ] Add exact negative tests:
  - `outer_tui_different_provider_login_does_not_retry`
  - `outer_tui_login_selection_cancel_does_not_retry`
  - `outer_tui_presenter_failure_does_not_retry`
  - `outer_tui_oauth_failure_does_not_retry`
  - `outer_tui_store_failure_does_not_retry`
  - `outer_tui_terminal_restore_failure_does_not_retry`
  - `outer_tui_midstream_revocation_never_opens_login_or_retries`
  - `json_rpc_and_text_credential_needed_never_construct_presenter`
- [ ] Every negative path asserts one user message and zero retries.

Run:

```powershell
cargo test -p opi-coding-agent --test interactive_tui_auth outer_tui_
```

Expected RED: the current headless driver accepts only auth commands/exit and bypasses normal prompt completion.

### GREEN: one shared prompt/auth/pending-turn state machine

- [ ] Replace separate helper composition with a narrow production state machine used by both `tui_event_loop` and `run_headless_interactive_tui_driver`.
- [ ] The state machine owns:
  - normal prompt submission;
  - pending prompt completion;
  - capture of one pre-output `CredentialNeeded(provider_id)`;
  - auth command outcome routing;
  - one retry allowance;
  - retry consumption/clear rules.
- [ ] Keep the original user message in the harness/session exactly once; retry with `CodingHarness::retry_last_prompt`.
- [ ] Clear pending auth when:
  - a new normal prompt is submitted;
  - login succeeds for a different provider;
  - selection is cancelled;
  - presenter/OAuth/store/terminal handling fails;
  - the retry is consumed;
  - the failure is `CredentialRevoked` or occurred after output began.
- [ ] Extend `InteractiveTuiTestCapture` with:

```rust
pub user_messages: usize,
pub provider_calls: usize,
pub retries: usize,
pub presenter_constructions: usize,
pub system_messages: Vec<String>,
pub terminal_transitions: Vec<String>,
```

- [ ] Let the debug driver accept normal prompt strings and injected test auth services while still entering through `run_interactive_tui`.
- [ ] Await prompt completion deterministically in the headless path; do not use sleeps or real terminal polling.
- [ ] Keep JSON/RPC/text paths unchanged and assert they do not build `TuiLoginPresenter`, open a browser, or wait for input.

### Verify

```powershell
cargo test -p opi-coding-agent --test interactive_tui_auth
cargo test -p opi-coding-agent --test interactive_auth interactive_
cargo test -p opi-coding-agent --test oauth_auth anthropic_oauth_revoked_stops_turn_without_retry_or_relogin
cargo test -p opi-coding-agent --test non_interactive credential_needed_fails_without_prompt
cargo test -p opi-coding-agent --test oauth_auth rpc_credential_needed_fails_without_blocking
cargo clippy --workspace --all-targets -- -D warnings
```

### Commit

- [ ] Stage only 14.20 paths and commit:

```powershell
git commit -m "test(opi-coding-agent): cover outer TUI auth retry"
```

## Task 14.21: Documentation, Acceptance Artifacts, and Phase F

**Files**

- Modify: `README.md`
- Modify: `README.zh.md`
- Modify: `crates/opi-ai/README.md`
- Modify: `crates/opi-ai/README.zh.md`
- Modify: `crates/opi-agent/README.md`
- Modify: `crates/opi-agent/README.zh.md`
- Modify: `crates/opi-coding-agent/README.md`
- Modify: `crates/opi-coding-agent/README.zh.md`
- Modify: `docs/opi-spec.md`
- Modify: `docs/opi-spec.zh.md`
- Modify: `CHANGELOG.md`
- Modify public rustdoc in files changed by 14.15-14.20 where it still describes old behavior.
- Modify: `crates/opi-coding-agent/tests/phase14_provider_auth_docs.rs`

**Ignored runtime evidence — rebuild but never stage**

- `target/opi-artifacts/phase14-phase-exit/CRITERIA_TRACE.md`
- `target/opi-artifacts/phase14-phase-exit/criteria-trace.json`
- `target/opi-artifacts/phase14-phase-exit/PHASE_EXIT_SCENARIO_AUDIT.md`
- `target/opi-artifacts/phase14-phase-exit/PHASE_EXIT_REPORT.md`
- Command manifests/logs required by the current `opi-implement` Phase F protocol under `target/opi-artifacts/phase14-phase-exit/`.

### RED: public claim guards

- [ ] Update `phase14_provider_auth_docs.rs` so it fails on any current claim that:
  - Copilot is a Chat-only compatibility profile;
  - Codex is standard Responses with flags;
  - Codex supports Browser PKCE only;
  - current provider ids are `copilot` or `codex`;
  - `api-map` is deferred;
  - every OAuth flow supports manual code entry.
- [ ] Require positive paired EN/ZH claims:
  - one `github-copilot` identity with Anthropic/Chat/Responses routes;
  - one `openai-codex` identity with dedicated wire and Browser/Device Code login;
  - native-keychain persistence and explicit no-migration behavior;
  - custom Rust/TOML mapped-provider contract;
  - flow-specific manual semantics;
  - same-provider same-turn retry and non-interactive no-prompt behavior.

Run:

```powershell
cargo test -p opi-coding-agent --test phase14_provider_auth_docs
```

Expected RED: public docs still contain superseded Phase 14 claims.

### GREEN: documentation and help

- [ ] Update every English/Chinese pair in the file list in lockstep.
- [ ] Update public Rust docs for `WireApi`, `ModelInfo`, `ApiMappedProvider`, custom TOML, OAuth methods, dedicated Codex wire, and TUI retry.
- [ ] Update CLI/TUI help examples to canonical ids:

```text
/login anthropic
/login github-copilot
/login openai-codex
/logout <provider>
```

- [ ] Describe that only Browser PKCE flows have manual-code/callback behavior; Copilot and Codex Device Code use `present_device_code` and never `await_manual_code`.
- [ ] Document `[providers.custom.<id>]`, allowed TOML wires, one shared credential source, provider/model precedence, thinking map encoding, compat tagging, pricing tiers, and reserved headers.
- [ ] Record GitHub Copilot's static audited catalog/listing divergence from live entitlement filtering.
- [ ] Add entries only under `CHANGELOG.md` `## [Unreleased]`; do not modify released sections.
- [ ] Record `api-map` as `implemented` with task 14.16, both pi fixtures, and the mapped-provider acceptance tests.
- [ ] Remove all residual `deferred-by-updated-design` uses unless the current binding revision explicitly names them.
- [ ] When `docs/opi-spec.md` changes, let `opi-implement` reconcile registered ledger/spec hashes. Never hand-edit the ledger.

### Acceptance command manifest

Create the Phase F command manifest before running commands. For each row record:

```text
name
target
filter
selected_count
exit_code
result
started_at
finished_at
log_path
```

Treat exit 0 with `selected_count = 0` as failure.

#### New alignment acceptance commands

Run each separately:

```powershell
cargo test -p opi-coding-agent --lib native_keyring_host_selection_installs_a_default_store
cargo test -p opi-ai --test model_wire_metadata wire_api_serializes_exact_reviewed_names
cargo test -p opi-ai --test model_wire_metadata pricing_tier_uses_strict_greater_than_boundary
cargo test -p opi-ai --test model_wire_metadata unsupported_thinking_level_is_rejected_before_request_build
cargo test -p opi-coding-agent --test provider_identity canonical_oauth_provider_ids_are_exact
cargo test -p opi-coding-agent --test provider_identity development_provider_ids_are_rejected_without_alias_or_migration
cargo test -p opi-ai --test api_mapped_provider mapped_provider_dispatches_one_catalog_across_three_wires
cargo test -p opi-ai --test api_mapped_provider mapped_routes_share_one_lazy_auth_resolver
cargo test -p opi-coding-agent --test custom_provider_map custom_provider_api_and_base_url_precedence
cargo test -p opi-coding-agent --test custom_provider_map invalid_custom_provider_contracts_fail_at_load
cargo test -p opi-coding-agent --test github_copilot_provider github_copilot_catalog_matches_pi_0806_fixture
cargo test -p opi-coding-agent --test github_copilot_provider github_copilot_anthropic_model_posts_v1_messages_with_bearer
cargo test -p opi-coding-agent --test github_copilot_provider github_copilot_headers_match_reviewed_static_contract
cargo test -p opi-coding-agent --test github_copilot_provider github_copilot_next_stream_observes_changed_token_and_enterprise_base_url
cargo test -p opi-coding-agent --test github_copilot_provider github_copilot_401_and_403_are_revoked_on_every_wire
cargo test -p opi-coding-agent --test openai_codex_provider openai_codex_catalog_matches_pi_0806_fixture
cargo test -p opi-ai --test openai_codex_responses dedicated_codex_request_uses_exact_base_path_body_and_headers
cargo test -p opi-coding-agent --test oauth_auth openai_codex_browser_is_default_and_preserves_pkce_manual_race
cargo test -p opi-coding-agent --test oauth_auth openai_codex_device_code_success_exchanges_authorization_code
cargo test -p opi-coding-agent --test oauth_auth openai_codex_device_code_never_calls_await_manual_code
cargo test -p opi-coding-agent --test interactive_auth dispatcher_runs_concrete_anthropic_login_and_logout
cargo test -p opi-coding-agent --test interactive_auth dispatcher_runs_concrete_github_copilot_login_and_logout
cargo test -p opi-coding-agent --test interactive_auth dispatcher_runs_concrete_openai_codex_browser_login_and_logout
cargo test -p opi-coding-agent --test interactive_auth dispatcher_runs_concrete_openai_codex_device_login_and_logout
cargo test -p opi-coding-agent --test interactive_auth dispatcher_restores_terminal_once_on_every_concrete_exit
cargo test -p opi-coding-agent --test interactive_tui_auth outer_tui_same_provider_login_retries_pending_turn_once
cargo test -p opi-coding-agent --test interactive_tui_auth outer_tui_different_provider_login_does_not_retry
cargo test -p opi-coding-agent --test interactive_tui_auth outer_tui_terminal_restore_failure_does_not_retry
cargo test -p opi-coding-agent --test phase14_provider_auth_docs localized_docs_pin_exact_phase14_claims_and_acceptance_rows
```

#### Historical Phase 14 acceptance commands

Run all 29 exactly as recorded:

```powershell
cargo test -p opi-coding-agent --test doctor_cli stored_credential_probe_is_redacted
cargo test -p opi-coding-agent --test list_models stored_credential_metadata_is_redacted
cargo test -p opi-coding-agent --test credential_store headless_api_key_env_fallback
cargo test -p opi-coding-agent --test credential_store keychain_store_reaches_production_construction
cargo test -p opi-coding-agent --test oauth_auth all_builtin_flows_support_manual_fallback
cargo test -p opi-coding-agent --test oauth_auth resolve_oauth_near_expiry_refreshes_and_writes_new_token
cargo test -p opi-coding-agent --lib oauth_login_restores_terminal_after_flow_failure
cargo test -p opi-coding-agent --test oauth_auth rpc_credential_needed_fails_without_blocking
cargo test -p opi-coding-agent --test non_interactive credential_needed_fails_without_prompt
cargo test -p opi-coding-agent --test oauth_auth anthropic_oauth_revoked_stops_turn_without_retry_or_relogin
cargo test -p opi-coding-agent --test oauth_auth login_oauth_writes_oauth_credential_to_store
cargo test -p opi-coding-agent --test oauth_auth anthropic_env_oauth_token_precedence_stored_wins_env_fallback
cargo test -p opi-coding-agent --test credential_store mutation_lock_serializes_concurrent_writers
cargo test -p opi-coding-agent --test oauth_auth resolve_oauth_refresh_failure_rereads_fresh_token_without_partial_write
cargo test -p opi-coding-agent --test oauth_auth factory_routes_
cargo test -p opi-ai --test request_enrichment request_scalars_carry_explicit_values
cargo test -p opi-coding-agent --test provider_factory build_provider_wires_each_builtin_provider_family
cargo test -p opi-agent --test agent_loop_mock session_id_reaches_every_request
cargo test -p opi-coding-agent --test session_runtime phase14_session_affinity_tracks_new_resume_and_fork
cargo test -p opi-ai --test request_enrichment session_affinity_wire_mappings
cargo test -p opi-ai --lib anthropic::tests::cache_control_long_ttl_emits_all_markers
cargo test -p opi-ai --test model_capabilities_migration anthropic_builtin_models_have_cache_capabilities
cargo test -p opi-ai --test anthropic_fixtures cache_1h_tokens_parsed_as_subset_of_cache_write
cargo test -p opi-coding-agent --test session_runtime open_existing_replays_usage_from_assistant_messages
cargo test -p opi-ai --test usage_cost calculate_cost_all_cache_writes_are_1h
cargo test -p opi-ai --test provider_collection refresh_models_is_atomic_substrate
cargo test -p opi-coding-agent --test phase14_provider_auth_docs localized_docs_pin_exact_phase14_claims_and_acceptance_rows
cargo test -p opi-coding-agent --test oauth_auth login_logout_commands_are_discoverable
cargo test -p opi-coding-agent --test non_interactive credential_needed_fails_without_prompt
```

The historical manual-fallback test name must remain runnable, but its assertions must become flow-specific: Browser PKCE providers support manual fallback; Copilot and Codex Device Code explicitly do not. Rename only if the Phase F manifest and historical compatibility runner can preserve the exact recorded filter; otherwise keep the name and correct its semantics.

### Workspace and release gates

- [ ] Run formatting. If it changes files, inspect and stage only task-owned paths:

```powershell
cargo fmt --all
cargo fmt --check --all
```

- [ ] Run:

```powershell
cargo clippy --workspace --all-targets -- -D warnings
$env:RUSTDOCFLAGS="-D warnings"; cargo doc --workspace --no-deps
Remove-Item Env:RUSTDOCFLAGS
cargo test --workspace --doc
cargo test --workspace --all-targets
```

- [ ] Locate the reviewed smoke scripts at execution time:

```powershell
Get-ChildItem -Path .claude/skills/opi-implement/scripts,scripts -Filter "opi-impl-smoke.ps1" -Recurse
```

- [ ] Run the one location declared by the current `.claude/skills/opi-implement/skill.md`. The current skill text still names `scripts/opi-impl-smoke.ps1`, while the dirty relocation must remain outside this task; do not silently choose or stage a relocation.

### Commit task 14.21 before Phase F report reconstruction

- [ ] Run the current-claim and whitespace guards before staging:

```powershell
rg -n "copilot:|codex:|/login copilot|/login codex|Chat-only|Browser PKCE only|api-map.*deferred" README.md README.zh.md crates docs CHANGELOG.md
rg -n "deferred-by-updated-design" README.md README.zh.md crates docs CHANGELOG.md
rg -n "�|鈥|锟|Ã|Â" README.md README.zh.md crates docs CHANGELOG.md
git diff --check
```

Expected: no current old-id/old-architecture claim, no unauthorized residual deferral, no mojibake in changed public material, and no whitespace error.

- [ ] Stage only the task-owned public documentation, guard, and rustdoc/help files. Use explicit paths; the expected superset is:

```powershell
git add README.md README.zh.md CHANGELOG.md
git add crates/opi-ai/README.md crates/opi-ai/README.zh.md
git add crates/opi-agent/README.md crates/opi-agent/README.zh.md
git add crates/opi-coding-agent/README.md crates/opi-coding-agent/README.zh.md
git add docs/opi-spec.md docs/opi-spec.zh.md
git add crates/opi-coding-agent/tests/phase14_provider_auth_docs.rs
git add crates/opi-ai/src/model_info.rs crates/opi-ai/src/api_mapped.rs crates/opi-ai/src/openai_codex_responses.rs
git add crates/opi-ai/src/auth.rs crates/opi-ai/src/credential.rs
git add crates/opi-coding-agent/src/config.rs crates/opi-coding-agent/src/provider_factory.rs
git add crates/opi-coding-agent/src/oauth.rs crates/opi-coding-agent/src/interactive.rs crates/opi-coding-agent/src/interactive_auth.rs
git add crates/opi-coding-agent/src/github_copilot.rs crates/opi-coding-agent/src/openai_codex.rs
git diff --cached --name-only
```

If a listed rustdoc source has no 14.21 diff, `git add` is a no-op. If 14.21 changed a rustdoc/help source not listed, add that exact path separately; never use `git add .` or `git add -A`.

- [ ] Confirm the staged list excludes:
  - `.claude/skills/opi-implement/**`
  - `.gitignore`
  - `scripts/*.workflow.js`
  - `target/**`
  - `.opi-impl-state.json`
  - this plan file.
- [ ] Commit:

```powershell
git commit -m "docs(phase14): close provider auth alignment"
```

### Rebuild Phase F evidence

- [ ] Record the real 14.21 commit id in the trace/report; never use a planned or pre-commit hash.
- [ ] Rebuild `criteria-trace.json` and `CRITERIA_TRACE.md` from the binding revision.
- [ ] Mark:
  - F14-01 met only by the cfg-gated host-selection acceptance test.
  - F14-02 met only by flow-specific Browser versus Device Code semantics through concrete providers.
  - F14-03 met only by dispatcher-to-concrete-provider-to-locked-store tests.
  - F14-04 met only by `run_interactive_tui` outer-entry tests.
  - `api-map` implemented only by task 14.16 public Rust/TOML tests.
- [ ] Rebuild `PHASE_EXIT_SCENARIO_AUDIT.md` with target/filter/selected count/exit/result for every historical and new command.
- [ ] Rebuild `PHASE_EXIT_REPORT.md` with SC1-SC8, intentional divergences, Non-Goals, commit ids 14.14-14.21, fixture provenance, workspace gates, artifact audit, and evaluator verdict.
- [ ] Scan artifacts for secrets, absolute user paths, malformed UTF-8/mojibake, stale old ids, and stale deferral claims.
- [ ] Run the current Phase 14 artifact audit.
- [ ] Invoke the current `opi-implement` Phase F five-lens evaluator and its independent adversarial verification.
- [ ] If post-commit evaluation finds a real 14.21-owned defect, return to RED/GREEN, rerun the affected acceptance plus all workspace gates, and amend the still-local 14.21 commit so the task retains exactly one commit. Never hide an evaluator failure by editing only the ignored report.

### Final verification

```powershell
rg -n "copilot:|codex:|/login copilot|/login codex|Chat-only|Browser PKCE only|api-map.*deferred" README.md README.zh.md crates docs CHANGELOG.md
rg -n "deferred-by-updated-design" docs target/opi-artifacts/phase14-phase-exit
rg -n "�|鈥|锟|Ã|Â" README.md README.zh.md crates docs target/opi-artifacts/phase14-phase-exit
git status --short
git diff --check
```

Expected:

- no current public old-id/old-architecture claims;
- no unauthorized Phase 14 residual deferral;
- no replacement-character/mojibake hits in changed docs/artifacts;
- only task-owned changes plus the untouched baseline workflow relocation;
- `git diff --check` exits 0.

## Final Exit Checklist

- [ ] Commits 14.14-14.21 exist in order, exactly one per task.
- [ ] Every task evaluator is accepted.
- [ ] Every new alignment command selects at least one test and exits 0.
- [ ] All 29 historical commands select at least one test and exit 0.
- [ ] `cargo fmt --check --all` passes.
- [ ] Clippy passes with `-D warnings`.
- [ ] Rustdoc passes with `-D warnings`.
- [ ] Doctests pass.
- [ ] Workspace all-target tests pass.
- [ ] Reviewed smoke script passes.
- [ ] Artifact audit passes.
- [ ] Five-lens Phase F evaluator and independent adversarial verifier accept.
- [ ] SC1-SC8 are all `met`.
- [ ] `api-map` is `implemented`.
- [ ] Intentional divergences and Non-Goals remain intact.
- [ ] Baseline workflow relocation remains outside all eight commits.
- [ ] No archive commit is created until the user separately approves the archive gate.
