# Phase 14 Provider and Auth -- Independent Code Audit

**Auditor**: gpt5 (independent, no prior audit reports consulted)  
**Date**: 2026-07-22  
**Scope**: Tasks 14.1--14.21, task commits `d9f21a97d0d93a57c1a84e248b9254ece2ea2bb8` through `8364e74a9077a194cb4a7fd68db2e3c4b420111a`  
**Implementation audited**: `3ef05d16afb17b86dd536ad1fb00bfb45b9fef32`  
**Method**: Independent full-file review of the Phase 14 designs, task ledger, affected provider/auth/session/accounting implementations, tests, fixtures, public documentation, and localized counterparts. The audit covered correctness, security/redaction, test quality, spec compliance, explicit invariants, cross-task integration, and residual concurrency/API concerns. Targeted suites and all current workspace release gates were run. Existing `audit.*.md`, evaluator reports, and detailed Phase F/acceptance artifacts were not consulted.

---

## 1. Executive Summary

**Verdict: PASS-WITH-FINDINGS**

| Severity | Count |
|----------|------:|
| Blocker | 0 |
| Major | 3 |
| Minor | 3 |
| Info | 0 |

Phase 14 is broad, substantially implemented, and passes the current workspace verification gates. The principal risks are three non-atomic or insufficiently scoped boundaries: direct Anthropic/Codex OAuth credentials can inherit a credential-supplied URL intended only for Copilot routing, the public keychain read bypasses the marker half of the fail-closed protocol, and a rejected mapped-catalog replacement can leave the live provider partially mutated. These are significant but localized rather than systemic, so the phase is a qualified pass and the Major findings should be resolved before the next phase.

### Per-task summary

| Task | Title | Verified task commit | Verdict |
|------|-------|----------------------|---------|
| 14.1 | Credential store substrate | `d9f21a97` | Finding: Major, Minor |
| 14.2 | OAuth architecture and per-request auth | `e058eeb9` | Finding: Major |
| 14.3 | Request scalars and session affinity | `43bf0e4b` | Pass |
| 14.4 | Model capabilities and cache markers | `3e1663b8` | Pass |
| 14.5 | Usage and cost accounting | `d0caa629` | Finding: Minor |
| 14.6 | Dynamic model refresh | `b66c3c7f` | Pass |
| 14.7 | Documentation and structural guards | `2bdfd30f` | Finding: Minor |
| 14.8 | Native keyring and probes | `07c14548` | Pass |
| 14.9 | Auth dispatcher and persistence | `c68f9201` | Pass |
| 14.10 | Live auth/session interaction | `e83b76d4` | Pass |
| 14.11 | Factory-built Anthropic cache behavior | `76ee74ea` | Pass |
| 14.12 | Usage/cost contract repair | `2f855627` | Pass |
| 14.13 | Documentation, verification, and residual closure | `6c6ceb41` | Finding: Minor |
| 14.14 | Native keyring host selection | `62b12dde` | Pass |
| 14.15 | Wire API and model metadata | `0589a18b` | Finding: Major |
| 14.16 | API-mapped providers and TOML routing | `f34949c8` | Finding: Major |
| 14.17 | GitHub Copilot three-wire provider | `708f0260` | Pass |
| 14.18 | Dedicated OpenAI Codex provider | `1833876d` | Finding: Major |
| 14.19 | Concrete OAuth dispatcher paths | `502ea52d` | Pass |
| 14.20 | Outer TUI auth retry | `aa3bee7e` | Pass |
| 14.21 | Documentation and final acceptance closure | `8364e74a` | Finding: Minor |

---

## 2. Security and Credential Findings

### 2.1 MAJOR: Direct OAuth providers trust a Copilot-only credential URL

**Files:** `crates/opi-ai/src/openai_codex_responses.rs`, `crates/opi-ai/src/anthropic.rs`, `crates/opi-ai/tests/openai_codex_responses.rs`  
**Lines:** `openai_codex_responses.rs:349--353`; `anthropic.rs:1352--1356`; `openai_codex_responses.rs:121--155`  
**Affected tasks:** 14.2, 14.18  
**Spec ref:** The Phase 14 exit-remediation design scopes credential-derived `base_url` to GitHub Copilot enterprise routing. The dedicated Codex route is specified as the exact ChatGPT backend API endpoint.

**Cause:** Direct Anthropic and OpenAI Codex stream construction gives `ResolvedAuth.base_url` precedence over the model/provider base URL. The credential field is generic at the type boundary, but its designed routing purpose is provider-specific. The Codex test currently makes the unsafe precedence an asserted behavior by directing the bearer token and account ID to the resolver-supplied URL.

**Impact:** A malformed, stale, migrated, or tampered OAuth envelope can redirect a direct Anthropic or Codex bearer credential to another endpoint. Codex can also violate its exact-host contract. Normal login currently writes no base URL for these providers, so the issue requires invalid stored metadata and is Major rather than Blocker, but the credential boundary fails open.

**Fix:** Ignore or reject credential-derived base URLs for direct Anthropic and Codex providers, and honor them only on GitHub Copilot mapped routes. Preserve testability through an explicit constructor or transport seam rather than credential metadata. Add negative tests proving a resolver-supplied URL receives neither direct-provider request nor authorization headers.

### 2.2 MAJOR: Public keychain reads bypass the two-entry fail-closed protocol

**Files:** `crates/opi-coding-agent/src/credential_store.rs`, `crates/opi-coding-agent/tests/credential_store.rs`  
**Lines:** source `credential_store.rs:835--840`, compared with marker-first resolver paths at `995--1030` and `1141--1175`; transition test at `crates/opi-coding-agent/tests/credential_store.rs:923--969`  
**Affected task:** 14.1

**Cause:** `CredentialStore::read` decodes only the protected envelope. It does not first read the non-secret presence/kind marker or reconcile the marker kind with the envelope kind. The production resolver paths do perform that reconciliation, but the public trait method exposes the lower-level protected-entry behavior directly.

**Impact:** During an API-key/OAuth kind transition, a public reader can return the old protected credential after the marker has changed instead of returning the required typed wrong-kind or corrupt-store error. It can also return a protected entry whose marker is absent. In-tree live authentication uses the safer resolver, but embedders and other public callers can bypass the intended fail-closed protocol.

**Fix:** Make public `read` marker-first: marker absence returns `None`, and a present marker must agree with a valid protected envelope before any credential is returned. Add public-method tests for marker absence, marker-only state, corrupt marker, mismatched kinds, and a paused kind transition.

The remaining two-entry mutation ordering is sound: writes publish the marker before the protected envelope; deletes remove the protected envelope before the marker; resolver paths block environment fallback for corrupt, partial, and wrong-kind states.

---

## 3. Correctness and Cross-task Integration Findings

### 3.1 MAJOR: Rejected mapped-catalog replacement can partially mutate the live provider

**Files:** `crates/opi-ai/src/api_mapped.rs`, `crates/opi-coding-agent/src/provider_factory.rs`, `crates/opi-coding-agent/src/harness.rs`  
**Lines:** `api_mapped.rs:204--243`; `provider_factory.rs:2136--2152`; `harness.rs:754--758`, `837--844`  
**Affected tasks:** 14.15, 14.16

**Cause:** `ApiMappedProvider::replace_model_catalog` validates model identities and route existence, then mutates route providers sequentially. It checks for an empty route subset only while iterating and delegates route replacement immediately. If a later subset is empty or a later route rejects its catalog, earlier route catalogs remain changed while aggregate `self.models` is not updated. Construction prevents empty routes initially, but replacement does not re-establish the invariant before mutation.

**Production reachability:** `assemble_harness_collection` materializes active extension model overrides. An override can move the sole model on an existing mapped wire, leaving that route empty. The factory records the replacement error only as a diagnostic and continues; the harness then moves the same partially mutated provider into the runtime agent.

**Impact:** The aggregate catalog used for listing, resolution, and outer capability validation can retain old metadata while concrete route catalogs contain replacement metadata. Valid requests can be rejected or processed using capabilities different from those advertised. A custom route that rejects a late replacement produces the same split state.

**Fix:** Precompute and validate every per-route subset, including non-empty coverage, before mutating any route. Then make replacement transactional by staging and swapping all route catalogs together, or by snapshotting and restoring every route on any error. Add rollback tests for an override that empties a later route and for a late custom-route rejection; assert that both aggregate and concrete catalogs remain unchanged.

### 3.2 MINOR: Credential mutations acquire the global lock but do not re-read

**File:** `crates/opi-coding-agent/src/credential_store.rs`  
**Lines:** `843--864`  
**Affected task:** 14.1  
**Spec ref:** The provider-auth design requires every mutation to acquire the store lock and then re-read current state.

**Cause:** `write` and `delete` acquire the process-global store lock and immediately perform their mutation. The comment calls this acquire-then-re-read behavior, but no re-read occurs.

**Impact:** Writes and deletes remain serialized and untorn, and their current unconditional last-writer-wins semantics mean that merely discarding a re-read would not change observable behavior. Nevertheless, the explicit concurrency contract and its intended stale-state protection are unimplemented and untested.

**Fix:** Define the behavior when post-lock state differs from pre-lock expectations, then re-read and act on that state. Test competing writes, write/delete races, and kind transitions. Do not add a read whose result is ignored merely to satisfy the wording.

---

## 4. Specification, Documentation, and Verification Findings

### 4.1 MINOR: Current documentation calls browser usage a non-goal while shipping browser OAuth

**Files:** `docs/opi-spec.md`, `docs/opi-spec.zh.md`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `crates/opi-coding-agent/README.md`, `crates/opi-coding-agent/README.zh.md`  
**Lines:** `docs/opi-spec.md:1593--1596`, compared with `1748` and `1830`; `docs/opi-spec.zh.md:1348`, compared with `1453` and `1502`; `crates/opi-ai/README.md:260--274`; `crates/opi-ai/README.zh.md:245`; `crates/opi-coding-agent/README.md:489--495`; `crates/opi-coding-agent/README.zh.md:447--452`  
**Affected tasks:** 14.7, 14.13, 14.21

**Cause:** Phase 12 historical non-goal wording was retained as an unqualified statement of current behavior. The same documents now describe the implemented Browser PKCE flow, and the Phase 14 docs guard does not reject the contradictory phrase.

**Impact:** Normative and crate-level documentation gives conflicting answers about whether the product opens a browser, so the SC8 claim that public documentation describes final behavior exactly is incomplete.

**Fix:** Scope the old statement historically and replace the current non-goal with browser automation outside the approved OAuth login flows. Update English and Chinese counterparts together and add a negative guard for the unqualified wording.

### 4.2 MINOR: Task 14.5 retains invalid verification metadata

**Files:** `docs/snapshots/phase14/opi-impl-state.json`, `crates/opi-coding-agent/tests/session_runtime.rs`  
**Lines:** ledger `946`, `1000--1001`, `1010`, `1039`; actual test at `session_runtime.rs:2577`  
**Affected tasks:** 14.5, 14.13, 14.21

**Cause:** The Task 14.5 ledger still names nonexistent test target and file `usage_cost_wiring` and uses stale filter `phase14_usage_breakdowns_survive_resume`. The implemented test is `phase14_usage_subsets_survive_session_resume`. The task is marked passing while its acceptance scenario remains open.

**Impact:** Task 14.5 cannot be reproduced from its declared gate list. The nonexistent target fails, while the stale filter exits successfully after selecting zero tests, which defeats the ledger's verification purpose. Valid acceptance commands exist elsewhere, so this is a residual metadata defect rather than evidence that runtime accounting is broken.

**Fix:** Use the guarded ledger-update flow to remove the nonexistent target/file, name the real session test, reconcile the scenario status, and validate selected-test counts for task-level gates as well as acceptance commands.

---

## 5. Invariant and Success-Criterion Verification

| Invariant / criterion | Code evidence | Test coverage / verdict |
|-----------------------|---------------|-------------------------|
| SC1: credential persistence is native, typed, and fail-closed | Native stores, marker/envelope ordering, probe classification, and marker-first resolver paths are implemented in `native_keyring.rs` and `credential_store.rs` | Store/keyring suites pass; partial because public `CredentialStore::read` bypasses the marker and mutations omit the specified re-read (2.2, 3.2) |
| SC2: approved OAuth flows dispatch concrete Browser/device flows without hidden relogin | Dispatcher and provider-specific OAuth handlers in `interactive_auth.rs` and `oauth.rs`; lazy resolver construction and typed failures | Auth/dispatcher suites pass; runtime met |
| SC3: credentials are re-resolved per stream and session retry is exact-once | Resolver use in provider stream paths; outer TUI retry state machine and pending-turn preservation | Auth contract, per-request auth, Codex, and TUI retry suites pass; partial because direct providers accept the Copilot-only URL field (2.1) |
| Secrets are neither listed nor emitted through normal diagnostics | Provider listings use model metadata; errors and trace surfaces are redacted; store probes return typed states | Listing, auth, provider, and redaction tests pass |
| Request timeout, additive safe headers, cancellation, and session affinity reach the intended production providers | `Request` fields at `provider.rs:68--108`; Anthropic/OpenAI Chat/OpenAI Responses consume them; harness forwards stable new/resume/fork session IDs | Request-enrichment and `session_id_reaches_every_request` tests pass; SC4 met |
| Model capabilities gate requests and Anthropic cache markers | Nested capability structures and validation in `model_info.rs`; marker gating/placement in `anthropic.rs` | Capability, remediation, factory-cache, and marker suites pass; SC5 met |
| Optional usage subsets preserve `None` versus zero and cannot exceed parents | `Option<u64>` usage subsets and provider-specific validation; malformed usage emits non-retryable stream error before completion | Usage-cost and provider fixture suites pass |
| Cost contains exactly four parent lines without child double-counting | `CostBreakdown` and weighted write folding in `stream.rs`; exact cumulative arithmetic uses `u64` while public parents saturate | Thirty usage/cost tests and provider fixtures pass; SC6 met |
| Session schema remains v1, persists no derived cost, and resume recomputes identical totals | Session replay accumulates persisted usage rather than storing cost | Both focused session-resume tests pass |
| Dynamic refresh publishes all-or-nothing deterministic snapshots | `ProviderCollection::refresh` collects before committing and atomically replaces the collection snapshot | Error, no-op, repeated replacement, and order tests pass; SC7 refresh substrate met |
| Mapped construction and replacement maintain route/catalog agreement | Constructor validates the initial route graph | Initial construction tests pass; replacement invariant fails on late error (3.1) |
| Public English/Chinese docs and structural guards describe final behavior | Positive Browser OAuth and provider behavior are documented and most obsolete claims are guarded | Docs guard passes, but the browser non-goal contradiction and stale verification metadata leave SC8 partial (4.1, 4.2) |

### Non-goals

The original eight Phase 14 non-goals remain structurally preserved. In particular, dynamic discovery remains substrate rather than an implicit network workflow, costs are derived rather than persisted, and provider/package examples have not become hidden core workflows. The documentation defect in 4.1 is contradictory wording, not accidental implementation of an additional browser-automation feature.

---

## 6. Test Quality and Verification

### Workspace gates

All current release gates passed at audited HEAD:

- `cargo fmt --check --all`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --all-targets`
- `cargo test --workspace --doc`
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`

### Targeted coverage

- Auth/security: 24 `opi-ai` auth/Codex tests; 9 native-keyring unit tests; 69 credential-store and interactive-auth tests with 2 intentional subprocess-only ignores; 122 OAuth dispatcher tests; focused factory/doctor/list/non-interactive tests all passed.
- Provider/model/request integration: 287 tests passed across `opi-ai`, `opi-agent`, and `opi-coding-agent`, covering request enrichment, capabilities, collection refresh, mapped routing, wire metadata, Anthropic cache markers, Copilot, Codex, and session affinity.
- Usage/cost/session: 30 accounting tests, 14 focused provider subset tests, two focused resume tests, and the documentation guard passed.
- Negative ledger verification reproduced 4.2: the nonexistent `usage_cost_wiring` target fails, and the stale session filter selects zero tests while exiting successfully.

The suites are generally strong and fixture-driven. The material gaps correspond to the findings: no negative direct-provider URL-scoping tests, no public marker-aware read transition tests, and no mapped-catalog rollback test that fails after an earlier route has mutated.

---

## 7. Residuals and Recommendations

### Priority recommendations

1. Enforce provider ownership of credential-derived routing metadata. Direct Anthropic and Codex requests must not accept the Copilot enterprise base URL field.
2. Make every public keychain read obey the marker/envelope protocol, then specify and test meaningful acquire-then-re-read mutation semantics.
3. Make mapped-catalog replacement transactional and retain the original live provider unchanged after any rejected extension override.
4. Reconcile the Browser OAuth statements in paired English/Chinese documentation and extend the structural guard.
5. Repair Task 14.5 verification metadata through the guarded implementation-ledger workflow and require nonzero selected-test counts.

No performance cliff, unsafe Rust, production-network test dependency, secret-bearing diagnostic, or additional release-blocking residual was found in the audited scope.
