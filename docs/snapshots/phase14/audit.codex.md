# Phase 14 Provider & Auth -- Independent Code Audit

**Auditor**: codex (independent; no prior audit findings were consulted)
**Date**: 2026-07-22
**Scope**: Tasks 14.1--14.21; implementation commits `d9f21a9..8364e74`; current source at `8d6e6ca` (including later source remediations)
**Method**: Full specification and task-ledger read, grouped full-file source/test review, cross-crate tracing, invariant and negative-path analysis, documentation parity review, and fresh workspace verification.

**Independence note**: Audit context contaminated (isolated): one auxiliary provider-wire sub-pass reported accidental exposure to forbidden report snippets during a broad lookup. That sub-pass was discarded in full before returning findings. A fresh zero-context replacement re-read the source; only the clean pass and independently verified source evidence are used below. The primary auditor did not receive the snippets.

---

## 1. Executive Summary

**Verdict: FAIL**

| Severity | Count |
|----------|------:|
| Blocker | 0 |
| Major | 7 |
| Minor | 4 |
| Info | 0 |

Phase 14 has unusually strong automated coverage: all workspace targets, Clippy, doctests, and warning-denied rustdoc pass. Credential redaction, OAuth deadline/cancellation behavior, static catalog fixtures, per-stream auth resolution, and outer-TUI retry semantics are well covered.

The phase nevertheless does not meet its exit contract. The current implementation can advertise a custom provider but execute a built-in route, advertise extension model overrides that a mapped provider cannot dispatch, drop valid OpenAI tool-call argument bytes, ignore audited Copilot Anthropic compatibility metadata, and bypass the required unsupported-thinking preflight on public provider/collection paths. Together with the incomplete refresh batch contract and process-global keyring ownership bug, these are systemic cross-task failures rather than isolated polish issues.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 14.1 | Credential store model | Pass with Minor |
| 14.2 | OAuth architecture and per-request auth re-resolution | Pass |
| 14.3 | Request scalars and session-affinity production path | Pass |
| 14.4 | Model capabilities and Anthropic cache markers | Pass |
| 14.5 | Usage and cost cache/reasoning accounting | Pass with Minor |
| 14.6 | Dynamic provider model refresh | Fail -- Major |
| 14.7 | Provider/auth docs, non-goal guards, and final Phase 14 gates | Pass with Minor |
| 14.8 | Native keyring and production probes | Pass |
| 14.9 | Login/logout dispatcher and persistence | Pass |
| 14.10 | Live auth and session interaction | Pass |
| 14.11 | Factory-built Anthropic cache markers | Pass |
| 14.12 | Usage and cost contract | Pass with Minor |
| 14.13 | Documentation, verification, and residual closure | Pass with Minor |
| 14.14 | Native keyring host selection | Fail -- Major |
| 14.15 | WireApi, model metadata, pricing, thinking, and canonical IDs | Fail -- Major |
| 14.16 | ApiMappedProvider and TOML custom providers | Fail -- Major |
| 14.17 | GitHub Copilot three-wire catalog | Fail -- Major |
| 14.18 | OpenAI Codex dedicated wire, catalog, and dual login | Fail -- Major |
| 14.19 | Concrete OAuth dispatcher vertical path | Pass |
| 14.20 | Outer TUI credential retry | Pass |
| 14.21 | Documentation, acceptance artifacts, and Phase F | Pass with Minor |

---

## 2. Correctness and Cross-Task Integration Findings

### 2.1 MAJOR: Custom provider IDs can collide with built-in routing and send a request through the wrong provider

**Files:** `crates/opi-coding-agent/src/config.rs`, `crates/opi-coding-agent/src/provider_factory.rs`, `crates/opi-ai/src/provider_collection.rs`
**Lines:** `config.rs:910--927`; `provider_factory.rs:1773--1829, 2059--2079`; `provider_collection.rs:299--316`
**Cause:** Custom-provider validation rejects only an empty ID. Runtime construction matches built-in and deprecated IDs before consulting `providers.custom`, while model listing registers custom providers after built-ins and `ProviderCollection::register` replaces an equal ID. A configuration such as `[providers.custom.anthropic]` can therefore expose custom metadata in listing/pickers but construct the built-in Anthropic provider at execution time. Quoted IDs containing `:` are also accepted even though `provider:model` parsing makes them unroutable. A custom ID can additionally collide with an `openai_compatible` profile; listing and runtime availability then disagree depending on which credential is present.
**Impact:** A validly parsed configuration can list one endpoint/catalog and execute another. When a model ID overlaps, prompts may be sent to an unintended external service with a different credential; otherwise a listed model fails as unknown at runtime.
**Fix:** Validate provider IDs once across built-ins, deprecated IDs, custom providers, and `openai_compatible` profiles. Reject reserved IDs, cross-table duplicates, `:`, and IDs that cannot round-trip through `provider:model`. Add a product-path test that compares listing and captured runtime endpoint for collision cases.

### 2.2 MAJOR: Extension model overrides are advertised but cannot be dispatched by `ApiMappedProvider`

**Files:** `crates/opi-ai/src/registry.rs`, `crates/opi-ai/src/api_mapped.rs`, `crates/opi-coding-agent/src/provider_factory.rs`, `crates/opi-coding-agent/src/harness.rs`
**Lines:** `registry.rs:224--241`; `api_mapped.rs:175--196`; `provider_factory.rs:2145--2152`; `harness.rs:951--957, 1191--1196`
**Cause:** `ProviderRegistry::resolve` gives extension overrides precedence and returns the original provider with override metadata. The harness uses that registry for picker entries and model validation. `ApiMappedProvider::stream`, however, looks up the model only in its private construction-time catalog. An extension-added model therefore resolves in the registry and is selectable, then fails with `UnknownModel`; a same-ID override is accepted as metadata but cannot change the route/compatibility actually used by the mapped provider.
**Impact:** The extension model-override contract is false for custom mapped providers, GitHub Copilot, and any other mapped provider. Users can select/persist a model that deterministically fails on the next provider call.
**Fix:** Apply overrides to the mapped catalog/route graph before provider construction, or reject overrides that the active provider cannot dispatch. Add an integration test that selects an extension-added mapped model and captures the concrete route.

### 2.3 MAJOR: Refresh returns on the first failure instead of collecting every provider result

**File:** `crates/opi-ai/src/provider_collection.rs`
**Lines:** `430--491`
**Spec ref:** Phase 14 design, T3e; task 14.6 DoD.
**Cause:** The method documentation and design require every provider to be called in sorted order, all results to be collected, and registry mutation to occur only if the entire batch succeeds. The loop returns immediately on a provider error or invalid candidate model, so later providers are never called.
**Impact:** The collection does preserve the prior registry atomically, but it does not implement the promised batch operation. Later providers cannot refresh or report their outcome after an earlier failure, and call order/coverage depends on the first error.
**Fix:** Collect every raw result first in sorted order, validate all successful candidates without mutating live catalogs, retain the first sorted error, and replace catalogs only when no result or validation error exists.
**Test gap:** `refresh_models_deterministic_ordering` checks sorted `provider_ids`, not recorded refresh invocation order; no test proves later providers are called after an earlier failure.

### 2.4 MAJOR: GitHub Copilot Anthropic compatibility metadata is stored but never applied to the wire

**Files:** `crates/opi-coding-agent/src/github_copilot.rs`, `crates/opi-coding-agent/src/provider_factory.rs`, `crates/opi-ai/src/anthropic.rs`
**Lines:** `github_copilot.rs:75--94, 146--166, 422--431`; `provider_factory.rs:1311--1339`; `anthropic.rs:820--898`
**Cause:** The audited Copilot catalog records `force_adaptive_thinking` and `supports_temperature`, including Opus 4.7/4.8 entries that require adaptive thinking and prohibit temperature. Factory construction passes those models to the concrete Anthropic route, but `AnthropicProvider::build_request_body` never reads `AnthropicMessagesCompat`: it always emits a requested `temperature` and always serializes enabled thinking as fixed budget-based `{type: "enabled", budget_tokens: ...}`.
**Impact:** Requests for audited Copilot Anthropic models can violate their declared wire contract and be rejected or behave differently from the selected model metadata. The full-catalog fixture proves metadata equality, not that the production route consumes it.
**Fix:** Resolve `WireCompat::AnthropicMessages` for the selected model in request construction. Suppress temperature when unsupported and emit the required adaptive-thinking shape when forced. Add captured factory-wire tests for Opus 4.7/4.8 and a non-adaptive control model.

### 2.5 MAJOR: OpenAI Chat drops arguments included in the initial tool-call chunk

**File:** `crates/opi-ai/src/openai_chat.rs`
**Lines:** `332--361`
**Test:** `crates/opi-ai/tests/openai_chat_fixtures.rs:240--246`
**Cause:** When a tool-call delta includes `id`, `name`, and non-empty `function.arguments`, the mapper emits only `ToolCallStart`. It reads and emits `arguments` only in the `else` branch where `id` is absent. Existing fixtures make the initial arguments string empty and therefore mask the loss.
**Impact:** A legal OpenAI-compatible stream can lose the first argument fragment. The resulting tool JSON is truncated or invalid, causing the wrong tool input or a downstream validation failure. This affects direct OpenAI Chat and the Phase 14 custom/Copilot Chat routes.
**Fix:** Seed the accumulator from initial arguments or emit `ToolCallStart` followed by `ToolCallDelta` when both are present. Add single- and multi-tool fixtures whose initial chunk contains non-empty arguments.

### 2.6 MAJOR: Public provider and collection dispatch bypass the required unsupported-thinking preflight

**Files:** `crates/opi-ai/src/provider.rs`, `crates/opi-ai/src/provider_collection.rs`, `crates/opi-ai/src/api_mapped.rs`, `crates/opi-ai/src/openai_chat.rs`, `crates/opi-ai/src/openai_responses.rs`, `crates/opi-ai/src/openai_codex_responses.rs`
**Lines:** `provider.rs:217--275`; `provider_collection.rs:389--411`; `api_mapped.rs:175--196`; `openai_chat.rs:1096--1101`; `openai_responses.rs:321--326`; `openai_codex_responses.rs:111--123`
**Cause:** The public validator correctly maps an unresolvable thinking level to `UnsupportedCapability`, and the `opi-agent` loop calls it. `ProviderCollection::dispatch_stream` and `ApiMappedProvider::stream` do not. Chat and Responses silently ignore `ThinkingLevelMap::resolve` errors; Codex falls back to the raw level name. The public library paths therefore do not uphold the documented provider contract.
**Impact:** Direct Rust consumers and collection dispatch can perform network I/O with a reasoning value that model metadata explicitly marks unsupported. The same request is rejected in the agent loop, so behavior diverges by entry point.
**Fix:** Put capability preflight at the provider/collection dispatch boundary used by all public paths and propagate mapping errors instead of swallowing them. Add zero-auth/zero-HTTP negative tests for Anthropic, Chat, Responses, Codex, and `ApiMappedProvider`.
**Related gap:** `ModelInfo::validate` also accepts `supports_thinking = false` with an enabled thinking map, and `supports_long_cache_retention = true` with cache control disabled. Enforce coherence while validating model metadata.

### 2.7 MAJOR: Native keyring leasing destroys a pre-existing process default store

**Files:** `crates/opi-coding-agent/src/native_keyring.rs`, `crates/opi-coding-agent/src/credential_store.rs`
**Lines:** `native_keyring.rs:31--42, 52--64`; `credential_store.rs:168--187`
**Cause:** The first OPI lease unconditionally replaces `keyring-core`'s process-global default store. Dropping the last lease unconditionally calls `unset_default_store` instead of restoring the prior store. `KeyringCoreBackend::new` performs this installation automatically, including in an embedding process.
**Impact:** An application embedding `opi-coding-agent` can lose the credential store installed by another component while OPI is alive and end with no default store after OPI shuts down. This is cross-component global-state corruption on a valid library path.
**Fix:** Capture `keyring_core::get_default_store()` before the first OPI installation and restore it on the last guard drop. Alternatively, reject installation when a non-OPI store is already installed and expose an explicit injection path. Add a pre-installed-store lifecycle test.

---

## 3. Security, Redaction, and Storage Finding

### 3.1 MINOR: The versioned credential envelope accepts mixed-kind and unknown fields

**File:** `crates/opi-coding-agent/src/credential_store.rs`
**Lines:** `403--426, 523--592`
**Cause:** One flattened `EnvelopeFields` struct contains both API-key and OAuth fields. Deserialization ignores unknown fields, and decoding reads only the fields required by `kind`. A v1 `api_key` envelope containing `access`/`refresh`, or an OAuth envelope containing `api_key`, is silently accepted rather than classified as malformed.
**Impact:** The protected store remains secret, so this is not a direct disclosure. It weakens fail-closed corruption detection and can hide format drift or a buggy writer, making credential-kind invariants harder to trust.
**Fix:** Decode a strict tagged v1 enum with variant-specific fields and `deny_unknown_fields`, or explicitly reject fields that do not belong to the selected kind. Add mixed-kind and unknown-field tests.

The remaining credential/OAuth review found no secret leakage or unsafe fallback: malformed/unknown persisted state blocks environment fallback, Linux daemon absence is narrowly classified, refresh is lock-coalesced, OAuth errors are redacted, one absolute deadline bounds all stages, and cancellation is honored only before one-use token acquisition.

---

## 4. Usage and Documentation Findings

### 4.1 MINOR: Session cost undercounts after a cumulative bucket exceeds `u32::MAX`

**Files:** `crates/opi-ai/src/stream.rs`, `crates/opi-coding-agent/src/session_coordinator.rs`
**Lines:** `stream.rs:157--179, 189--233`; `session_coordinator.rs:609--624`
**Cause:** `CumulativeUsage` correctly retains `u64` totals, but `SessionCoordinator::cost_summary` converts through `as_usage()`, which saturates parent buckets at `u32::MAX`, before calling `calculate_cost`.
**Impact:** Public cumulative `Usage` saturation matches the spec, but aggregate cost stops increasing after the cap, contradicting the separate requirement that cumulative cost remain correct. This requires an extremely large session, so practical risk is low.
**Fix:** Calculate cumulative cost directly from the internal `u64` totals (including optional child subsets) while keeping the public `Usage` projection saturated. Strengthen the saturation test to assert the exact expected cost, not only finite/non-negative values.

### 4.2 MINOR: Phase 14 documentation omits the typed `AccountIdMissing` outcome

**Files:** `README.md`, `README.zh.md`, `crates/opi-agent/README.md`, `crates/opi-agent/README.zh.md`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `crates/opi-coding-agent/README.md`, `crates/opi-coding-agent/README.zh.md`, `docs/opi-spec.md`, `docs/opi-spec.zh.md`
**Lines:** Representative omissions at `README.md:204--212`, `crates/opi-agent/README.md:45--55`, `crates/opi-ai/README.md:97--104`, `crates/opi-coding-agent/README.md:211--216, 371--381`, `docs/opi-spec.md:1717--1721, 1764--1770`, `docs/opi-spec.zh.md:1462--1466`
**Cause:** The changelog records `AgentError::AccountIdMissing { provider_id }` and its distinct `/login <provider>`/AuthFailure behavior, but the public auth taxonomy and non-interactive sections enumerate only `CredentialNeeded` and `CredentialRevoked`. English and Chinese are consistently incomplete rather than drifting from each other.
**Impact:** Embedders with exhaustive `AgentError` matches and users diagnosing an OpenAI Codex credential without `chatgpt_account_id` do not get the full documented contract.
**Fix:** Document `AccountIdMissing` across all EN/ZH pairs and the normative specs, including its distinction from revoked credentials and its non-interactive exit/remediation behavior.

---

## 5. Test Quality Finding

### 5.1 MINOR: Product custom-header negative tests fail before reaching header validation

**Files:** `crates/opi-coding-agent/tests/custom_provider_map.rs`, `crates/opi-coding-agent/src/config.rs`
**Lines:** `custom_provider_map.rs:289--312, 328--337`; `config.rs:1045--1060`
**Cause:** The `reserved header` and `invalid header name` cases omit every required `base_url`. Config validation therefore rejects them at the earlier base-URL gate, and the assertions check only that the provider ID appears and a canary does not leak. They never prove the intended header rejection.
**Impact:** Product TOML wiring for the reserved/invalid-header invariant can regress while these named tests remain green. Lower-level header tests do not close the config-order gap.
**Fix:** Supply a valid base URL, assert the specific header error class/message, and add an auth-header case with a syntactically valid non-secret value.

### Coverage assessment

| Area | Positive coverage | Negative/edge coverage | Assessment |
|------|-------------------|------------------------|------------|
| Credential store/probes | Round-trip, lock, native backend, listing/doctor | corruption, unavailable daemon, partial two-step mutation, env fallback | Strong; missing strict envelope and pre-existing global-store cases |
| OAuth | Three concrete providers, refresh, persistence, presenter/terminal | state mismatch, denial, timeout, cancellation cutover, secret canaries | Strong |
| Outer TUI / non-interactive | same-provider one-shot retry; text/JSON/RPC remediation | cancellation, different provider, store/presenter/terminal failure, mid-stream revocation | Strong |
| Model/wire metadata | exact catalog fixtures, route completeness, endpoint capture | unknown model, missing route, wire mismatch | Metadata strong; compatibility consumption and public preflight missing |
| OpenAI streaming | text, usage, tool calls, malformed arguments | multi-tool and chunked arguments | Initial coalesced tool arguments missing |
| Custom mapped providers | three wires, shared lazy auth, base precedence, invalid schemas | missing routes, reserved headers, invalid compat | Collision, extension-dispatch, and product header assertions missing |
| Dynamic refresh | replacement, clearing, rollback | invalid/duplicate catalogs and provider errors | Atomic state strong; all-provider invocation contract untested |
| Usage/cost | optional subsets, equality/absence, weighted cost, resume | malformed subsets and saturation | Exact cost above public saturation cap untested |
| Documentation | source guards and EN/ZH pairs | non-goal phrase guards | Auth taxonomy completeness missing |

---

## 6. Invariant Verification

| Invariant | Code evidence | Test coverage | Result |
|-----------|---------------|---------------|--------|
| Probes do not read protected credential contents | Marker-only `probe_metadata`; async doctor/listing consume redacted probes | credential-store, doctor, listing secret-read counters | Pass |
| Corrupt/unknown persisted state never falls through to env | Resolver returns typed store error when marker/envelope is present but invalid | corrupt marker/envelope/version/kind tests | Pass |
| Every credential mutation and refresh uses one coordinator lock | public write/delete plus refresh unlocked operations under held coordinator | contention, concurrent refresh, timeout, retry tests | Pass |
| OAuth flows use one absolute deadline and bounded body reads | shared deadline propagated through method selection, send, body, poll, exchange | paused-time deadline/body tests for PKCE, Copilot, Codex device | Pass |
| Cancellation is accepted only before one-use code/token acquisition | biased pre-acquisition select; post-acquisition exchange ignores cancel under original deadline | all three flow families | Pass |
| Credentials are re-resolved inside every returned managed stream | concrete route auth resolution occurs in stream future | changed token/base URL and same-turn retry tests | Pass |
| Missing/revoked credentials never auto-login; only same-provider explicit success retries once before output | outer TUI pending-turn state machine | comprehensive `interactive_tui_auth` matrix | Pass |
| Unsupported thinking levels fail before network I/O on every public wire path | Validator exists, but collection/mapped/concrete public dispatch bypasses or swallows it | agent-loop tests only | **Fail (2.6)** |
| Mapped-provider listing/selection and dispatch use one coherent catalog | private mapped catalog differs from registry override layer | registry-only override tests | **Fail (2.2)** |
| Custom provider identity maps to the same listing and runtime route | listing can replace built-in metadata; runtime matches built-ins first | no collision test | **Fail (2.1)** |
| GitHub Copilot requests honor audited per-model compatibility metadata | metadata is constructed but Anthropic request builder ignores it | catalog equality only | **Fail (2.4)** |
| Refresh calls all providers, then atomically replaces only on full success | registry mutation is deferred, but loop returns on first failure | rollback covered; later-call behavior absent | **Partial (2.3)** |
| Usage child fields stay within parents and do not double-count | provider validation plus weighted four-line cost calculation | provider and persistence/resume tests | Pass for normal range; cumulative cost overflow fails 4.1 |
| GitHub Copilot and OpenAI Codex listing is static and secret-free | listing path excludes credential probes for both catalogs | zero-read listing tests | Pass |
| EN/ZH counterparts remain synchronized | full paired read | documentation guards | Pass with shared omission 4.2 |

---

## 7. Verification Results

| Command | Result |
|---------|--------|
| `cargo fmt --check --all` | Pass |
| `git diff --check` | Pass |
| `cargo test --workspace --all-targets` | Pass (alternate `CARGO_TARGET_DIR` on `E:`) |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo test --workspace --doc` | Pass |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | Pass |

The first all-targets invocation using the existing `D:` target failed during linking because the volume had zero free space; it did not reach a test assertion. The complete command was rerun from scratch with `CARGO_TARGET_DIR=E:\codex-opi-phase14-audit-full` and exited successfully. No live provider calls or user keychain access were used.

---

## 8. Priority Recommendations

1. Fix public provider preflight, Copilot Anthropic compatibility consumption, and the initial OpenAI tool-argument loss first; these are direct request/response correctness defects.
2. Make provider identity and model override layers coherent before listing or dispatch: reject all ID collisions and ensure every advertised override is executable.
3. Correct refresh all-results behavior and native keyring ownership, with tests that observe later-provider calls and restoration of a pre-installed store.
4. Close the smaller contract gaps: strict credential envelope decoding, exact cumulative cost beyond the public saturation cap, `AccountIdMissing` docs, and assertion-strong product header tests.
