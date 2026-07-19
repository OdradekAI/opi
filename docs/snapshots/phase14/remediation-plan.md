# Phase 14 Remediation Plan

**Date**: 2026-07-19
**Audit sources**:
- `docs/snapshots/phase14/audit.gpt-5-codex.md` (GPT-5 Codex)
- `docs/snapshots/phase14/audit.glm5.2.md` (GLM-5.2)
- `docs/snapshots/phase14/audit.opus4.6.md` (Opus 4.6)

**Commit range**: `d9f21a97d0d93a57c1a84e248b9254ece2ea2bb8..8364e74a9077a194cb4a7fd68db2e3c4b420111a`
**Verification HEAD**: `4758c090da55251f9ea74e2d7c90d9ee0d2b2c8c`
**Design specs**:
- `docs/opi-spec.md`
- `docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md`
- `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md`

---

## Working-tree state

The three audit files were untracked before this remediation-planning run. No
audited source file or `.opi-impl-state.json` was modified. This document is
plan-only: no Phase F execution, commit, or ledger checkpoint is authorized
without a separate user request.

## Audit cross-reference summary

Unified severity uses Blocker / Major / Minor / Info. Consensus is Full (3/3),
Majority (2/3), or Unique (1/3).

| Cluster | Theme | Auditors | Consensus | Unified severity | Verification |
|---|---|---|---|---|---|
| C1 | Anthropic `None` suppresses provider-default cache markers | Codex 1.1, Opus 7.3 | Majority | Major | Confirmed; Opus impact corrected |
| C2 | Anthropic marker scan stops at trailing role-only content | Codex 1.2 | Unique | Major | Confirmed |
| C3 | Chat/Responses validate but ignore thinking maps | Codex 1.3 | Unique | Major | Confirmed |
| C4 | Responses affinity defaults and field gating conflate direct and custom profiles | Codex 1.4, GLM 3.2 | Majority | Major | Confirmed with a normative conflict |
| C5 | Present custom-provider credentials are frozen at construction | Codex 1.5 | Unique | Major | Confirmed |
| C6 | Auth-invalid lifecycle is inferred from Bearer syntax and may expose echoed credentials | Codex 1.6, GLM 2.2, GLM 3.3, GLM 3.4 | Majority | Major | Confirmed; grouped root fix |
| C7 | Dedicated Codex ignores `CacheRetention::Disabled` affinity semantics | Codex 1.7 | Unique | Major | Confirmed |
| C8 | Manual PKCE URL parsing/state validation and percent decoding are incomplete | Codex 2.1, Opus 2.2 | Majority | Major | Confirmed |
| C9 | OAuth flow-wide deadlines and cancellation are incomplete | Codex 2.2, GLM 3.1, GLM 4.10, Opus 4.1 | Full | Major | Confirmed; duplicate test findings merged |
| C10 | Arbitrary OAuth `error` fields can echo secrets | Codex 2.3, GLM 2.3 | Majority | Major | Confirmed |
| C11 | Doctor/list availability does not mirror live credential precedence | Codex 2.4, GLM 3.6 | Majority | Major | Partially confirmed; unconditional subscription listing is correct |
| C12 | Invalid extra-header values become retryable network errors | Codex 3.1, GLM 3.5 | Majority | Minor | Confirmed |
| C13 | Cumulative `u64` usage wraps when converted to public `u32` fields | Codex 3.2, Opus 2.1, Opus 4.3 | Majority | Major | Confirmed; severity raised because constructor input is directly reachable |
| C14 | Provider replacement/refresh can retain stale dynamic-catalog state and has contradictory comments | Codex 3.3, GLM 3.12 | Majority | Minor | Confirmed |
| C15 | First native-keyring construction is not single-flight | Codex 3.4 | Unique | Minor | Confirmed |
| C16 | Credential lock file is truncated before ownership is acquired | Codex 3.5 | Unique | Minor | Confirmed |
| C17 | Corrupt marker is represented as `BackendUnavailable` | GLM 3.7 | Unique | Minor | Observation confirmed; defect refuted against the normative three-state contract |
| C18 | Validated model/catalog ingress accepts zero limits or skips `ModelInfo::validate` | GLM 3.8, GLM 3.13 | Unique | Minor | Partially confirmed; TOML already rejects zero |
| C19 | OAuth persistence failure omits the failure notification | GLM 3.9 | Unique | Minor | Confirmed |
| C20 | TUI auth errors omit the canonical provider ID | GLM 3.10 | Unique | Minor | Confirmed |
| C21 | Refresh timeout bypasses the specified post-failure re-read | GLM 3.11 | Unique | Minor | Confirmed as a spec-letter gap |
| C22 | Provider-bundle test is intentionally compile-only but named as behavioral | GLM 4.1 | Unique | Minor | Confirmed test-quality issue |
| C23 | `assert_optional_u64` is a type check presented as a behavioral assertion | GLM 4.2 | Unique | Info | Confirmed test-quality issue |
| C24 | Child-greater-than-parent rejection lacks coverage | GLM 4.3 | Unique | Minor | Refuted; provider fixture tests own the invariant |
| C25 | Reserved-header test covers 6 of 13 names | GLM 4.4 | Unique | Minor | Confirmed |
| C26 | `RouteCatalogMismatch` lacks a negative test | GLM 4.5 | Unique | Minor | Confirmed |
| C27 | Mapped-provider tests do not exercise real routes | GLM 4.6 | Unique | Minor | Refuted by production-factory three-route coverage |
| C28 | Terminal suspend-failure recovery lacks outer-TUI coverage | GLM 4.7 | Unique | Minor | Confirmed |
| C29 | Terminal-guard unit test claims flow-failure coverage it does not provide | GLM 4.8 | Unique | Minor | Confirmed |
| C30 | JSON/RPC credential-event tests do not pin exactly-once emission | GLM 4.9 | Unique | Minor | Confirmed |
| C31 | Credential-envelope serialization uses `expect` | Opus 2.3 | Unique | Minor | Syntax confirmed; operational defect refuted |
| C32 | Unknown credential expiry always takes the refresh slow path | Opus 2.4 | Unique | Info | Confirmed intentional behavior |
| C33 | Legacy `SecretKey<String>` and `SecretString` coexist | Opus 3.1, Opus 7.1 | Unique | Minor | Confirmed Phase 14 non-goal |
| C34 | Manual authorization code is temporarily held in `String` | Opus 3.2 | Unique | Minor | Confirmed intentional short-lived representation |
| C35 | Loopback callback does not validate Host/Origin | Opus 3.3 | Unique | Info | Observation confirmed; no effective boundary improvement |
| C36 | Secondary providers lack optional-child usage rejection tests | Opus 4.2 | Unique | Minor | Refuted for providers that never emit those fields |
| C37 | Provider-collection rustdoc says OAuth is unimplemented | Opus 5.1 | Unique | Minor | Confirmed; severity reduced from Major |
| C38 | `StoreCredential` dispatch without an injected probe is non-gating | Opus 5.2 | Unique | Info | Confirmed intentional contract |
| C39 | Two modules define types named `CredentialSource` | Opus 7.2 | Unique | Info | Refuted as a defect; types do not cross module boundaries |
| C40 | Redaction architecture and canaries are sound | GLM 2.1 | Unique | Info | Positive finding; no remediation |
| C41 | Subprocess-only RPC tests are ignored | GLM other test info | Unique | Info | Correct test pattern |
| C42 | Phase 14 documentation guard is a brittle source scan | GLM other test info | Unique | Info | Confirmed supplementary test style; no product gap |
| C43 | No structural allowlist pins every `expose_secret` call site | GLM other test info | Unique | Info | Behavioral canaries retained as the security contract |
| C44 | `Agent::retry_last_turn` lacks a direct unit test | GLM other test info | Unique | Info | Transitively covered; no Phase 14 defect |

## Decision record

| ID | Finding cluster(s) | Decision | Rationale | Decided by |
|---|---|---|---|---|
| D1 | C3 | Make `request.thinking` plus the selected `ModelInfo::thinking_level_map` authoritative for Chat and Responses. Keep static compatibility data from driving wire output. | The alternatives presented were static fallback and static override. Both can silently contradict the caller's selected thinking level; the model map is already the validated per-model contract. | user |
| D2 | C4 | Use profile-scoped Responses affinity: built-in direct emits prompt-cache/request IDs for an effective session and independently gates `session_id`; custom/proxy defaults off and explicit opt-in enables the reviewed mapping. | The alternatives were universal upstream behavior and the current all-or-nothing flag. The selected policy satisfies both direct-profile parity and the design's custom-profile opt-in requirement. | user |
| D3 | C6 | Add an explicit route/profile auth-invalid policy. Canonical credential-managed routes may return `CredentialRevoked`; static custom/OpenRouter/Mistral routes return bodyless `AuthFailed`. Never include 401/403 bodies in either result. | Alternatives were a provider-ID allowlist or adding credential lifecycle to every `ResolvedAuth`, plus secret-aware body scrubbing. Explicit construction policy is harder to misclassify than syntax checks and body omission closes the echo channel completely. | user |
| D4 | C9 | Make cancellation responsive while awaiting callback/manual input/device polling, but stop accepting cancellation after an authorization code is acquired; the code exchange remains bounded by the single flow deadline. | The alternative was dropping an in-flight exchange, which can burn a single-use code. The selected policy preserves consume-once safety while bounding the whole flow. | user |
| D5 | C13 | Saturate public usage fields at `u32::MAX`, document the ceiling, and preserve child-within-parent invariants. | The alternative was widening the public and serialized `Usage` contract to `u64`, a breaking change disproportionate to Phase 14 remediation. | user |
| D6 | C1, C2, C7 | Apply the documented cache/affinity semantics directly: `None` preserves provider defaults, `Disabled` suppresses reusable affinity, and marker scans stop only after marking text. | These are unambiguous corrections to existing contracts and do not require a new configuration surface. | auto |
| D7 | C5 | Resolve custom credentials lazily on each stream through store-then-env precedence, even when a credential exists at construction. | The current absent-at-construction test covers only one branch; a source-backed resolver is the minimum fix that supports rotation and revocation uniformly. | auto |
| D8 | C8, C10 | Reuse one fallible manual-input/redirect parser and one closed OAuth server-error classifier across flows. | Shared parsing prevents callback/manual divergence; allowlisted error classes prevent untrusted server strings from reaching Display, Debug, notifications, or diagnostics. | auto |
| D9 | C11 | Centralize a secret-free availability calculation that mirrors live precedence, while preserving static no-secret listing for Copilot and Codex. | Doctor needs stored/OAuth-env/custom awareness, but list-models must retain the reviewed static subscription catalog behavior without retrieving credentials. | auto |
| D10 | C12, C18 | Validate at shared ingress: parse header names and values before HTTP, validate positive limits in `ModelInfo::validate`, and require registry overrides to call it. | TOML already rejects zero. Keeping zero only in detached/default data avoids changing `ModelCapabilities::new` while closing production mapped/extension ingress. | auto |
| D11 | C14 | Treat dynamic catalogs as replace-all snapshots and invalidate a provider's prior snapshot when the provider is replaced or refresh returns no catalog. | Existing tests and implementation establish snapshot replacement; changing to merge would preserve stale models and conflict with the reviewed semantics. | auto |
| D12 | C15, C16, C21 | Make credential initialization/locking atomic and route all refresh failures through the post-failure re-read. | These are local concurrency/order corrections. Initial construction must be single-flight, and a losing lock contender must not mutate the lock file. | auto |
| D13 | C19, C20 | Preserve typed auth failures through notification and TUI presentation, including fixed failure text and canonical provider IDs. | The provider ID is diagnostic context, not a secret, and every failed login path must deliver one failure notification before returning. | auto |
| D14 | C22, C23, C25, C26, C28, C29, C30 | Close or accurately name the verified test gaps without adding new product behavior. | Each item is additive coverage or misleading-test cleanup. | auto |
| D15 | C37 | Correct stale provider-collection comments without changing the descriptor or resolver contract. | Runtime OAuth is already implemented and tested; the defect is documentation-only. | auto |
| D16 | C17, C24, C27, C31-C36, C38-C44 | Do not modify production behavior for refuted, normative, duplicate, positive, or explicitly deferred/non-goal observations. | The exclusions table records primary evidence for each no-action finding. | auto |

## Remediation layers

### Layer 1: opi-ai (provider substrate)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-ai --all-targets -- -D warnings
    cargo test -p opi-ai --all-targets

#### Fix 1.1: Introduce explicit auth-invalid route policy

- **Audit source**: Codex 1.6; GLM 2.2, 3.3, 3.4.
- **Cluster**: C6.
- **Decision**: D3.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-ai/src/auth.rs` ~L28-52; `crates/opi-ai/src/anthropic.rs` ~L1045-1071; `crates/opi-ai/src/openai_chat.rs` ~L1263-1292; `crates/opi-ai/src/openai_responses.rs` ~L416-442; `crates/opi-ai/src/api_mapped.rs`; `crates/opi-ai/tests/api_mapped_provider.rs`; `crates/opi-ai/tests/provider_diagnostics.rs`.
- **Change**: add a typed auth-invalid policy to reusable route construction and consume it in Anthropic, Chat, and Responses 401/403 mapping. Remove Bearer-scheme inference. Produce either `CredentialRevoked` or a fixed, bodyless `AuthFailed` according to the constructed profile, and never pass the 401/403 body through `safe_excerpt`.
- **Test plan**: add a table-driven matrix covering canonical Anthropic/Copilot/Codex policy, Anthropic API-key behavior required by the canonical profile, custom Bearer across all three mapped wires, and OpenRouter/Mistral. Every case must assert the typed error, non-retryability, and absence of a non-standard echoed-key canary from Display and Debug.

#### Fix 1.2: Enforce validated model capabilities at mapped and registry ingress

- **Audit source**: GLM 3.8, 3.13.
- **Cluster**: C18.
- **Decision**: D10.
- **Verification status**: Partially confirmed.
- **File(s)**: `crates/opi-ai/src/model_info.rs` ~L16-40 and ~L538-550; `crates/opi-ai/src/registry.rs` ~L170-194; `crates/opi-ai/src/api_mapped.rs` ~L112-134; `crates/opi-ai/tests/api_mapped_provider.rs`; `crates/opi-ai/tests/custom_provider_registration.rs`.
- **Change**: make `ModelInfo::validate` reject zero `context_window` or `max_output_tokens` with a typed validation error, while leaving the zero-valued detached `Default` representation constructible. Require `ProviderRegistry::register_model` and mapped route/catalog construction to call the validator and return their existing non-exhaustive typed error surfaces.
- **Test plan**: add zero-limit and incompatible wire/compat cases through `ApiMappedProvider::try_new` and `register_model`; retain the existing TOML invalid-limits test to prove the earlier product ingress still rejects zero.

#### Fix 1.3: Drive Chat and Responses reasoning from the selected thinking map

- **Audit source**: Codex 1.3.
- **Cluster**: C3.
- **Decision**: D1.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-ai/src/provider.rs` ~L252-268; `crates/opi-ai/src/openai_chat.rs` ~L1013-1078; `crates/opi-ai/src/openai_responses.rs` ~L239-291; `crates/opi-ai/tests/openai_chat_fixtures.rs`; `crates/opi-ai/tests/openai_responses_fixtures.rs`.
- **Change**: carry the selected model's resolved thinking wire value through request validation into Chat and Responses serialization. Emit it only when thinking is enabled; an unsupported or disabled level emits no reasoning field. Static `compat.reasoning_effort` remains descriptive compatibility data and no longer overrides the request/model mapping.
- **Test plan**: add identity-map, remapped-level, disabled/off, and unsupported-level captures for both wires; replace tests that currently pin unconditional static emission.

#### Fix 1.4: Separate direct and opt-in Responses affinity policies

- **Audit source**: Codex 1.4; GLM 3.2.
- **Cluster**: C4.
- **Decision**: D2.
- **Verification status**: Confirmed with a normative conflict.
- **File(s)**: `crates/opi-ai/src/model_info.rs` ~L326-343; `crates/opi-ai/src/openai_responses.rs` ~L344-350 and ~L490-501; `crates/opi-ai/tests/openai_responses_fixtures.rs`; `crates/opi-ai/tests/openai_responses_lifecycle.rs`.
- **Change**: represent direct versus custom opt-in affinity explicitly instead of overloading `send_session_id_header`. For built-in direct Responses, emit `prompt_cache_key` and a fresh `x-client-request-id` whenever an effective session exists; gate only `session_id` on the header flag. For custom/proxy mode, suppress all three by default and emit the reviewed mapping only after explicit opt-in.
- **Test plan**: capture built-in direct with the session header disabled and prove prompt key/request ID remain; capture custom default-off with all affinity absent; capture custom opt-in with all agreed fields present and correctly sourced.

#### Fix 1.5: Restore Anthropic provider-default cache markers

- **Audit source**: Codex 1.1; Opus 7.3.
- **Cluster**: C1.
- **Decision**: D6.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-ai/src/provider.rs` ~L48-64; `crates/opi-ai/src/anthropic.rs` ~L768-794 and inline tests ~L1565-1573; `crates/opi-coding-agent/tests/anthropic_cache_markers.rs`.
- **Change**: treat `CacheRetention::None` as provider-default short retention for cache-capable Anthropic models. Emit ordinary ephemeral markers for `None` and `Short`; suppress markers only for `Disabled`.
- **Test plan**: replace the inline test that expects unmarked `None` and add a factory-built `None` wire capture alongside existing Short/Long/Disabled coverage.

#### Fix 1.6: Continue Anthropic marker search past non-text role content

- **Audit source**: Codex 1.2.
- **Cluster**: C2.
- **Decision**: D6.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-ai/src/anthropic.rs` ~L1203-1229; `crates/opi-coding-agent/tests/anthropic_cache_markers.rs`.
- **Change**: break each reverse role scan only after a text block is actually marked. A matching role that contains only tool results, images, tool calls, or thinking must not terminate the search.
- **Test plan**: add trailing tool-result/image-only user messages and tool-call/thinking-only assistant messages, asserting the last eligible text block for each role receives the marker.

#### Fix 1.7: Honor disabled affinity in the dedicated Codex route

- **Audit source**: Codex 1.7.
- **Cluster**: C7.
- **Decision**: D6.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-ai/src/openai_codex_responses.rs` ~L67-89, ~L156-168, and ~L249-270; `crates/opi-ai/tests/openai_codex_responses.rs`.
- **Change**: derive reusable affinity only when retention is not `Disabled`. Omit the user session from `prompt_cache_key`; use a fresh request-local UUID for the mandatory transport `session-id` fallback, and retain the already-fresh `x-client-request-id`.
- **Test plan**: add a Disabled capture proving the supplied user session appears in neither body nor headers while request-local transport IDs remain valid and distinct.

#### Fix 1.8: Reject invalid extra-header names and values before transport

- **Audit source**: Codex 3.1; GLM 3.5.
- **Cluster**: C12.
- **Decision**: D10.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-ai/src/provider.rs` ~L155-193; `crates/opi-ai/src/provider_headers.rs` ~L94-105; `crates/opi-ai/src/openai_codex_responses.rs` ~L172-185 and ~L316-327; `crates/opi-ai/tests/openai_codex_responses.rs`.
- **Change**: have shared request validation parse every extra-header name and value with the HTTP header types before building a request. Return non-retryable `RequestFailed` on syntax errors rather than allowing reqwest construction to become retryable `Network`.
- **Test plan**: add invalid-name and CR/LF/NUL value cases that assert `RequestFailed`, zero HTTP requests, and no retry classification.

#### Fix 1.9: Saturate cumulative usage at the public ceiling

- **Audit source**: Codex 3.2; Opus 2.1, 4.3; GLM 4.2 (related test helper).
- **Cluster**: C13, C23.
- **Decision**: D5, D14.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-ai/src/stream.rs` ~L201-220; `crates/opi-ai/tests/usage_cost.rs` ~L11 and ~L136-258.
- **Change**: replace all unchecked `u64 as u32` conversions with one documented saturating helper. Apply the same ceiling to reported and unknown aggregates and preserve child-within-parent invariants. Rename `assert_optional_u64` to state that it is a compile-time type assertion, or replace it with explicit typed bindings.
- **Test plan**: add reported and unknown aggregates at `u32::MAX + 1`, child-boundary assertions, and a finite/nonnegative cost calculation at the saturated boundary.

#### Fix 1.10: Invalidate stale dynamic catalogs on replacement and empty refresh

- **Audit source**: Codex 3.3; GLM 3.12.
- **Cluster**: C14.
- **Decision**: D11.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-ai/src/registry.rs` ~L129-155 and ~L223-234; `crates/opi-ai/src/provider_collection.rs` ~L443-473; `crates/opi-ai/tests/provider_collection.rs`; `crates/opi-ai/tests/custom_provider_registration.rs`.
- **Change**: remove a provider ID's prior dynamic catalog in both provider-replacement APIs. Preserve replace-all refresh semantics: `Some(snapshot)` replaces, and `None` clears any prior snapshot for that provider. Correct comments that currently claim old entries are retained.
- **Test plan**: add dynamic-snapshot then provider-replacement coverage and a `Some` to `None` refresh transition; assert stale models no longer resolve while replacement built-ins do.

#### Fix 1.11: Cover the complete reserved-header set

- **Audit source**: GLM 4.4.
- **Cluster**: C25.
- **Decision**: D14.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-ai/src/provider_headers.rs` ~L10-24; `crates/opi-ai/tests/api_mapped_provider.rs` ~L324-349.
- **Change**: derive test cases from one complete canonical list of all 13 reserved names without exposing new public API solely for testing.
- **Test plan**: assert rejection for every reserved name in both configured and per-request headers.

#### Fix 1.12: Add negative route/catalog consistency coverage

- **Audit source**: GLM 4.5.
- **Cluster**: C26.
- **Decision**: D14.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-ai/src/api_mapped.rs` ~L112-134 and ~L243; `crates/opi-ai/tests/api_mapped_provider.rs` ~L244-322.
- **Change**: no production behavior change; exercise the existing `ApiMapError::RouteCatalogMismatch` branch.
- **Test plan**: add same-ID/different-capabilities and subset/superset catalog fixtures, asserting the typed mismatch error.

#### Fix 1.13: Correct provider-collection OAuth and refresh rustdoc

- **Audit source**: Opus 5.1; GLM 3.12.
- **Cluster**: C14, C37.
- **Decision**: D11, D15.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-ai/src/provider_collection.rs` ~L25-29, ~L91-97, and ~L443-464.
- **Change**: replace “Future OAuth not implemented” language with the current descriptor/resolver/store split and describe dynamic catalogs as replace-all snapshots. Do not imply that `Ok(None)` retains a prior snapshot.
- **Test plan**: run rustdoc with warnings denied; optionally extend the existing Phase 14 source guard to reject the stale phrase.

### Layer 2: opi-agent

No source change is planned. `agent_loop` intentionally produces
`CacheRetention::None`, which means provider default under the reviewed Phase
14 design. Fix 1.5 corrects the Anthropic consumer instead of adding an
unrequested harness/config producer.

### Layer 3: opi-coding-agent (product/auth integration)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-coding-agent --all-targets -- -D warnings
    cargo test -p opi-coding-agent --all-targets

#### Fix 3.1: Default custom Responses affinity off and wire explicit policy

- **Audit source**: Codex 1.4; GLM 3.2.
- **Cluster**: C4.
- **Decision**: D2.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-coding-agent/src/config.rs` ~L1213-1219; `crates/opi-coding-agent/src/provider_factory.rs`; `crates/opi-coding-agent/tests/custom_provider_map.rs`; `crates/opi-coding-agent/tests/request_enrichment.rs`.
- **Change**: lower an omitted custom `send_session_id_header` to false and construct the custom opt-in affinity policy from Fix 1.4. Keep built-in direct Responses on the direct policy with its existing default-true session header.
- **Test plan**: add config/factory/wire cases for omitted custom flag, explicit false, explicit true, and built-in direct false; assert the exact body/header matrix selected in D2.

#### Fix 3.2: Resolve custom credentials lazily on every stream

- **Audit source**: Codex 1.5; related Codex 1.6 and GLM 3.4 policy wiring.
- **Cluster**: C5, C6.
- **Decision**: D7, D3.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-coding-agent/src/provider_factory.rs` ~L655-699 and ~L784-822; `crates/opi-coding-agent/tests/custom_provider_map.rs`; `crates/opi-coding-agent/tests/provider_factory.rs`.
- **Change**: replace construction-time `StaticAuthResolver` selection for a present custom credential with a source-backed resolver that re-evaluates store-then-env precedence for each stream. Pass the explicit static/custom auth-invalid policy from Fix 1.1 when constructing reusable routes.
- **Test plan**: construct with a present credential, perform one stream, rotate the injected store/env source, perform a second stream, and assert two distinct Authorization values without rebuilding the provider. Also assert 401/403 remains bodyless `AuthFailed`.

#### Fix 3.3: Normalize raw manual codes and pasted redirect URLs safely

- **Audit source**: Codex 2.1; Opus 2.2.
- **Cluster**: C8.
- **Decision**: D8.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-coding-agent/src/oauth.rs` ~L166-178, ~L310-365, and ~L463-479; `crates/opi-coding-agent/tests/oauth_auth.rs`.
- **Change**: add one fallible normalizer used by both callback and manual paths. Continue accepting a raw authorization code. For pasted URLs, parse query pairs, strictly percent-decode UTF-8, require `code` and a matching `state`, and reject missing, mismatched, malformed, or invalid-UTF-8 input with fixed redacted errors before any token request.
- **Test plan**: for Anthropic and Codex Browser, cover encoded redirect URL, raw code, missing/mismatched state, malformed escape, invalid UTF-8, and malformed URL; rejected cases must produce zero token POSTs.

#### Fix 3.4: Apply one deadline and cancellation policy across every OAuth flow

- **Audit source**: Codex 2.2; GLM 3.1, 4.10; Opus 4.1.
- **Cluster**: C9.
- **Decision**: D4.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-coding-agent/src/oauth.rs` ~L166-294, ~L1017-1035, ~L1305-1425, and ~L1548-1563; `crates/opi-coding-agent/tests/oauth_auth.rs`; `crates/opi-coding-agent/tests/interactive_auth.rs`.
- **Change**: introduce a shared flow budget based on one start time/deadline and apply its remaining duration to every HTTP send, response-body decode, callback/manual wait, device poll, and final exchange. Add biased cancellation while awaiting callback/manual input/polling. Once a code is acquired, finish its exchange under the remaining deadline without a cancellation branch. Replace the per-call uncancellable `spawn_blocking(read_line)` pattern with a single owned cancellation-safe input coordination path so cancellation cannot leave a stray stdin consumer.
- **Test plan**: add cancellation tests for Anthropic Browser, Codex Browser, and Copilot, asserting typed `LoginCancelled`, canonical provider ID, one fixed failure notification, no exchange/persistence after pre-code cancellation, and terminal/listener restoration. Add delayed initial authorization, body decode, PKCE exchange, and final Copilot exchange tests proving the one flow deadline bounds every stage.

#### Fix 3.5: Classify OAuth server errors without echoing server text

- **Audit source**: Codex 2.3; GLM 2.3.
- **Cluster**: C10.
- **Decision**: D8.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-coding-agent/src/oauth.rs` ~L426-444 and ~L1161-1202; `crates/opi-coding-agent/tests/oauth_auth.rs`.
- **Change**: replace arbitrary `error` interpolation in PKCE and Copilot paths with a closed mapping of recognized protocol error codes and one fixed unknown-error class. Do not surface error descriptions or unknown values.
- **Test plan**: inject authorization-code, verifier, refresh-token, device-code, and GitHub-token canaries through the server `error` field; assert none appears in Display, Debug, notifications, or diagnostics.

#### Fix 3.6: Centralize secret-free provider availability

- **Audit source**: Codex 2.4; GLM 3.6.
- **Cluster**: C11.
- **Decision**: D9.
- **Verification status**: Partially confirmed.
- **File(s)**: `crates/opi-coding-agent/src/provider_factory.rs` ~L833-852, ~L1475-1510, and ~L1579-1653; `crates/opi-coding-agent/src/doctor.rs` ~L675-741; `crates/opi-coding-agent/tests/doctor_cli.rs`; `crates/opi-coding-agent/tests/list_models.rs`.
- **Change**: expose one secret-free availability/probe calculation that mirrors live precedence: stored Anthropic OAuth, `ANTHROPIC_OAUTH_TOKEN`, Anthropic API key; subscription identities; and configured custom store/env credentials. Use it in doctor and applicable listing gates. Preserve unconditional static Copilot/Codex catalogs and ensure listing does not retrieve their secrets.
- **Test plan**: cover doctor with stored OAuth, OAuth-env-only Anthropic, both subscription IDs, and configured custom providers; cover listing with OAuth-env-only Anthropic and prove subscription static listing performs no credential read.

#### Fix 3.7: Make initial native-keyring construction single-flight

- **Audit source**: Codex 3.4.
- **Cluster**: C15.
- **Decision**: D12.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-coding-agent/src/native_keyring.rs` ~L52-75; associated inline or `crates/opi-coding-agent/tests/credential_store.rs` tests.
- **Change**: serialize the initial check, construction, and installation under one initialization state. A failed construction must clear initialization so a later caller can retry; successful callers must share the installed store.
- **Test plan**: use a barrier/counting constructor with two concurrent first callers and assert constructor count is one, both guards reference the installed store, and a separate failed-first case remains retryable.

#### Fix 3.8: Acquire credential lock before any lock-file mutation

- **Audit source**: Codex 3.5.
- **Cluster**: C16.
- **Decision**: D12.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-coding-agent/src/credential_store.rs` ~L549-588; `crates/opi-coding-agent/tests/credential_store.rs`.
- **Change**: open the lock file without truncation, acquire `try_lock`, and perform any ownership-only file update only after success. Do not add unrelated link/reparse hardening in this remediation.
- **Test plan**: hold the lock with sentinel contents, run a losing contender, and assert the contender returns the expected contention error without changing file length or bytes.

#### Fix 3.9: Notify on OAuth credential persistence failure

- **Audit source**: GLM 3.9.
- **Cluster**: C19.
- **Decision**: D13.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-coding-agent/src/oauth.rs` ~L1774-1791; `crates/opi-coding-agent/tests/oauth_auth.rs`.
- **Change**: handle the store write explicitly, issue exactly one fixed `credential store write failed` notification, and return the mapped typed error without emitting success.
- **Test plan**: extend the existing failing-store case to assert exact notification count/text, no success notification, and no secret in Display or Debug.

#### Fix 3.10: Preserve provider IDs in TUI auth errors

- **Audit source**: GLM 3.10.
- **Cluster**: C20.
- **Decision**: D13.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-coding-agent/src/interactive_auth.rs` ~L265-290; `crates/opi-coding-agent/tests/interactive_auth.rs`.
- **Change**: include the canonical provider ID when rendering `CredentialNeeded`, `CredentialRevoked`, `AccountIdMissing`, and `LoginCancelled`; keep messages fixed and secret-free.
- **Test plan**: table-test all four variants for canonical ID presence and absence of env values or secret canaries.

#### Fix 3.11: Re-read credentials after every refresh failure class

- **Audit source**: GLM 3.11.
- **Cluster**: C21.
- **Decision**: D12.
- **Verification status**: Confirmed as a spec-letter gap.
- **File(s)**: `crates/opi-coding-agent/src/credential_store.rs` ~L961-997; `crates/opi-coding-agent/tests/credential_store.rs`.
- **Change**: route refresh timeout and provider refresh errors through the same post-failure credential re-read before returning. Preserve the current lock and single-flight ordering.
- **Test plan**: use a scripted backend to prove a timeout invokes the re-read and returns the newer credential if present; otherwise preserve the original typed timeout.

#### Fix 3.12: Name the provider-bundle compile test honestly

- **Audit source**: GLM 4.1.
- **Cluster**: C22.
- **Decision**: D14.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-coding-agent/tests/provider_factory.rs` ~L399-464.
- **Change**: rename the unpolled test/helper to state that it compile-checks the production return type and add a short comment. Keep the separate async bundle-lifetime test as the behavioral owner.
- **Test plan**: run both renamed compile-time coverage and the existing awaited bundle-lifetime test.

#### Fix 3.13: Close terminal suspend and guard-test gaps

- **Audit source**: GLM 4.7, 4.8.
- **Cluster**: C28, C29.
- **Decision**: D14.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-coding-agent/src/interactive_auth.rs` ~L122-150; `crates/opi-coding-agent/tests/interactive_auth.rs` ~L314-324; `crates/opi-coding-agent/tests/interactive_tui_auth.rs` ~L560-587; `crates/opi-coding-agent/src/interactive.rs` ~L1216-1234.
- **Change**: add the missing outer-TUI suspend-failure scenario. Rename the unit test that only proves guard ordering and remove its unrelated literal flow error so its claim matches its assertions.
- **Test plan**: inject `InteractiveTuiTestTerminalFailure::Suspend`; assert `[suspend, resume]`, fixed suspension failure, and zero OAuth/store work. Retain a narrowly named guard-order test for resume-on-scope-exit.

#### Fix 3.14: Pin exactly-once JSON and RPC credential events

- **Audit source**: GLM 4.9.
- **Cluster**: C30.
- **Decision**: D14.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-coding-agent/src/main.rs` test modules ~L1840-1847 and ~L1941-1946; `crates/opi-coding-agent/tests/json_mode.rs`; `crates/opi-coding-agent/tests/rpc_jsonl.rs`.
- **Change**: replace first-match-only assertions with collection/count assertions before validating the payload.
- **Test plan**: assert exactly one `CredentialNeeded` event in both NDJSON and RPC streams and retain the existing provider/error payload checks.

### Layer 4: Documentation

**Verification**: update English and Chinese normative documentation together;
keep historical design records intact; run source/doc guards and rustdoc.

#### Fix 4.1: Record the accepted Phase 14 wire and auth semantics

- **Audit source**: Codex 1.3, 1.4, 1.6, 2.2, 3.2; GLM 3.1-3.4; Opus 2.1, 4.1.
- **Cluster**: C3, C4, C6, C9, C13.
- **Decision**: D1-D5.
- **Verification status**: Confirmed.
- **File(s)**: `docs/opi-spec.md`; `docs/opi-spec.zh.md`; `crates/opi-coding-agent/tests/phase14_provider_auth_docs.rs`.
- **Change**: document that model thinking maps drive Chat/Responses reasoning; distinguish built-in direct from custom opt-in Responses affinity; define auth-invalid classification by constructed credential policy with bodyless failures; state pre-code cancellation/post-code bounded exchange behavior; and document the saturated `u32` usage ceiling. Synchronize the Chinese counterpart in the same change.
- **Test plan**: extend the Phase 14 documentation guard for the stable terms and run `cargo test -p opi-coding-agent --test phase14_provider_auth_docs`.

## Final verification

Run focused tests while implementing each fix, then:

    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc

With `RUSTDOCFLAGS=-D warnings`:

    cargo doc --workspace --no-deps

No test may use a real provider endpoint or credential. OAuth/provider wire
tests must use existing injected stores, fixtures, `MockProvider`, or
`wiremock`.

## Scope exclusions

Every audited item that does not produce a fix is recorded below.

| Finding | Status | Reason |
|---|---|---|
| C17 / GLM 3.7 | Refuted as a defect | The public and normative credential probe is deliberately three-state: Present / Absent / BackendUnavailable. Corrupt readable markers fail closed with an explicit corruption reason and never fall through to env. A fourth exhaustive enum variant would be a separate source-breaking design change. |
| C24 / GLM 4.3 | Duplicate | Child-greater-than-parent rejection is already pinned in `anthropic_fixtures.rs`, `openai_chat_fixtures.rs`, and `openai_responses_fixtures.rs`; `usage_cost.rs` correctly treats provider mapping as the invariant owner. |
| C27 / GLM 4.6 | Refuted | `custom_provider_map.rs` already builds the production factory and drives real Anthropic, Chat, and Responses routes over exact HTTP paths with shared lazy auth and header assertions. Repeating the same integration in the substrate test file would add locality, not coverage. |
| C31 / Opus 2.3 | Info/No action | The closed credential envelope contains only serializable strings, options, and integers, so serialization is structurally infallible today. Converting the internal helper to `Result` would cascade signatures without changing reachable behavior. |
| C32 / Opus 2.4 | Info/No action | `expires_at = None` intentionally means expiry cannot be trusted, so refresh-on-use is the fail-safe behavior. |
| C33 / Opus 3.1, 7.1 | Info/No action | End-to-end migration of legacy `SecretKey<String>` is an explicit Phase 14 non-goal. Active credential/auth paths use `SecretString`; no new dual-path code is planned. |
| C34 / Opus 3.2 | Info/No action | A manually entered code is a short-lived, single-use string needed for parsing and immediate exchange. It is neither persisted nor formatted. |
| C35 / Opus 3.3 | Info/No action | The callback binds loopback and validates high-entropy state plus PKCE. Host/Origin is attacker-controlled by a local client and does not strengthen that boundary; Fix 3.3 closes the actual state/parser gap. |
| C36 / Opus 4.2 | Refuted | Only Anthropic and OpenAI Chat/Responses emit optional child usage fields, and all three validate them. Tests for providers that always emit `None` would be speculative. |
| C38 / Opus 5.2 | Info/No action | No-probe `StoreCredential` dispatch is intentional and tested; the live resolver remains the authoritative enforcement point. |
| C39 / Opus 7.2 | Refuted | Bedrock and provider-auth `CredentialSource` types live in separate modules and are never cross-imported; the shared name causes no ambiguity at use sites. |
| C40 / GLM 2.1 | Info/No action | Positive finding: secret wrappers, fixed diagnostics, trace redaction, and behavioral canaries are working as designed. |
| C41 / GLM other info | Info/No action | Subprocess-only RPC stdio child tests are correctly `#[ignore]` and are launched by owning parent tests. |
| C42 / GLM other info | Info/No action | Source-scan documentation guards are supplementary. Behavioral provider tests and explicit doc review remain the authoritative checks; no vacuous pass was demonstrated. |
| C43 / GLM other info | Info/No action | Security is pinned by end-to-end canaries across Display, Debug, diagnostics, traces, and wire failures. A source-level `expose_secret` allowlist would be brittle and would not prove runtime redaction. |
| C44 / GLM other info | Info/No action | `retry_last_turn` is covered through harness/TUI behavior. The audit identified no divergent direct-agent behavior requiring another test. |

## Notes for execution (Phase F)

- Execute in dependency order: Layer 1, Layer 3, then documentation. Layer 2
  has verification-only coverage.
- Fix 1.1 must land before product construction supplies its route policy in
  Fix 3.2. Fix 1.4 must land before Fix 3.1 lowers custom affinity.
- Fix 3.4 is the highest-risk item. Its cancellation-safe manual input
  ownership and flow deadline should be reviewed before proceeding to later
  product fixes.
- Do not modify `.opi-impl-state.json` by hand. If execution is later
  authorized through the implementation workflow, use its guarded checkpoint
  protocol.
- Do not commit unless the user explicitly asks. If a commit is authorized,
  stage only files changed during Phase F; never stage the pre-existing audit
  artifacts implicitly.
