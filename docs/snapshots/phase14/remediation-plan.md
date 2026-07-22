# Phase 14 Remediation Plan

**Date**: 2026-07-22
**Audit sources**: `audit.codex.md`, `audit.glm5.2.md`
**Commit range**: `d9f21a97d0d93a57c1a84e248b9254ece2ea2bb8..8364e74a9077a194cb4a7fd68db2e3c4b420111a`
**Audited HEAD**: `3ef05d16afb17b86dd536ad1fb00bfb45b9fef32`
**Design specs**: `docs/opi-spec.md`, `docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md`, `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md`

---

## Audit cross-reference summary

The two reports have no positive finding-level consensus: every active finding
was reported by one auditor only. Several Codex findings also conflict with a
broader PASS statement in the GLM report, so the code verification below is the
controlling evidence.

| Cluster | Theme | Auditors | Consensus | Unified severity | Verification |
|---------|-------|----------|-----------|------------------|-------------|
| C1 | Direct Anthropic/Codex routes consume credential-derived `base_url` | Codex 2.1 | Unique (1/2) | Major | Confirmed |
| C2 | Public `CredentialStore::read` bypasses the marker/envelope protocol | Codex 2.2 | Unique; GLM's store-wide PASS did not inspect this public boundary | Major | Confirmed |
| C3 | Mapped catalog replacement can leave earlier routes mutated after a later error | Codex 3.1 | Unique (1/2) | Major | Confirmed |
| C4 | Public credential mutations lock but do not re-read | Codex 3.2 | Unique; conflicts with GLM's store PASS | Minor | Partially confirmed |
| C5 | Current docs call browser usage a non-goal while documenting Browser PKCE | Codex 4.1 | Unique; conflicts with GLM's docs PASS | Minor | Confirmed |
| C6 | Task 14.5 ledger names a nonexistent target and a zero-selecting filter | Codex 4.2 | Unique; conflicts with GLM's acceptance PASS | Minor | Confirmed |
| C7 | Codex body helper strips any provider prefix while `stream` strips only `openai-codex:` | GLM 2.1 | Unique (1/2) | Minor | Confirmed |
| C8 | Codex body helper falls back around the model thinking map | GLM 2.2 | Unique (1/2) | Minor | Confirmed |
| C9 | OpenAI Chat/Responses discard safe usage-subset diagnostics | GLM 2.3 | Unique (1/2) | Minor | Confirmed |
| C10 | Credential envelope intermediates are manually zeroized rather than RAII-zeroized | GLM 3.1 | Unique (1/2) | Minor | Confirmed |
| C11 | Linux Secret Service absence classification uses error-text matching | GLM 3.2 | Unique (1/2) | Info (downgraded) | Partially confirmed |
| C12 | Interactive auth presentation distinguishes store errors by string prefix | GLM 3.3 | Unique (1/2) | Info | Confirmed |
| C13 | Standard Responses HTTP fixtures do not cover canonical data-only SSE | GLM 5.1 | Unique (1/2) | Minor | Confirmed |
| C14 | No focused test splits one SSE frame across byte chunks | GLM 5.2 | Unique (1/2) | Minor | Confirmed |
| C15 | Historical design used `Option<CacheRetention>` rather than the shipped enum sentinel | GLM 6.1 | Unique (1/2) | Info | Refuted as a current defect |
| C16 | Historical design used `HeaderMap` rather than the shipped validated vector | GLM 6.2 | Unique (1/2) | Info | Refuted as a current defect |
| C17 | Codex synthesizes affinity UUIDs when the request session id is empty | GLM 6.3 | Unique (1/2) | Info | Confirmed, intentional |
| C18 | `Usage::reported` does not itself validate child subsets | GLM 7.1 | Unique (1/2) | Info | Partially confirmed |
| C19 | `CumulativeUsage::as_usage` rustdoc omits its saturation behavior | GLM 7.2 | Unique (1/2) | Info | Confirmed |
| C20 | Non-interactive `AccountIdMissing` uses the `CredentialNeeded` event shape | GLM 7.3 | Unique (1/2) | Info | Confirmed, intentional |
| C21 | Credential lock path differs textually from the historical literal path | GLM 7.4 | Unique (1/2) | Info | Refuted |
| C22 | Fake keyring delay blocks a test runtime worker | GLM 7.5 | Unique (1/2) | Info | Confirmed, test-only |

### Verification notes

- C1: `AnthropicProvider::stream` and `OpenAiCodexResponsesProvider::stream`
  both give `ResolvedAuth.base_url` precedence. The binding corrective design
  assigns this field to GitHub Copilot enterprise routing; direct Codex has a
  fixed ChatGPT backend contract.
- C2: the trait implementation calls `read_unlocked` directly, while API-key
  and OAuth resolver paths read and reconcile the marker first. Existing public
  read tests also seed protected entries without markers, demonstrating the
  divergent contract.
- C3: route subsets are computed and applied in map order. An empty or rejected
  later route returns after earlier `replace_model_catalog` calls, while
  `self.models` remains unchanged. `assemble_harness_collection` retains this
  same provider after reporting only a diagnostic.
- C4: the missing re-read is real, but `write` and `delete` are unconditional,
  serialized last-writer-wins operations with no expected-version input. A
  discarded re-read would not add stale-state protection. This needs a
  semantics decision, not a ceremonial extra read.
- C6 was reproduced at audited HEAD: `usage_cost_wiring` is not a test target,
  and `phase14_usage_breakdowns_survive_resume` exits successfully after
  selecting zero tests. The real test is
  `phase14_usage_subsets_survive_session_resume`.
- C11 is a real implementation coupling, but it is dependency-drift risk rather
  than a current product defect. The keyring abstraction supplies opaque error
  text at this boundary; the classifier is deliberately narrow and covered by
  positive and negative tests.
- C15 and C16 are superseded historical-signature differences. The current
  normative spec records the shipped types, and both preserve the required
  behavior.
- C17 and C20 are explicitly specified Codex/non-interactive behavior. C21 is
  false because `user_config_dir()` is already the opi-specific directory.

## Decision record

| ID | Finding cluster(s) | Decision | Rationale | Decided by |
|----|-------------------|----------|-----------|------------|
| D1 | C1 | Only a route explicitly constructed for GitHub Copilot may consume `ResolvedAuth.base_url`; direct Anthropic and Codex use model/provider constructor URLs. | Enforces ownership of routing metadata without changing the credential envelope or Copilot enterprise routing. | auto |
| D2 | C2 | Introduce one marker-first consistent-read helper and make the public `read` plus resolver paths share it. | One protocol implementation avoids public/resolver drift and preserves typed corrupt/wrong-kind failures. | auto |
| D3 | C3 | Precompute and validate every route subset before mutation, snapshot every route catalog, and restore already-applied routes if a later route rejects replacement. | Closes the production empty-route case before mutation and gives custom-route failures rollback without a public trait change. | auto |
| D4 | C4 | **D4a selected:** narrow the normative acquire-then-re-read requirement to read-modify-write refresh and document public write/delete as serialized unconditional mutations. | The current public methods have no compare/version precondition, so a discarded re-read would add no observable concurrency semantics. | user |
| D5 | C5 | Scope the old browser statement explicitly to Phase 12 and make the current non-goal "browser automation outside approved OAuth login flows" in both languages. | Removes the contradiction while retaining the intended product boundary. | auto |
| D6 | C6 | Reconcile Task 14.5 only through the guarded `opi-implement` ledger flow; `opi-remediate` must not edit the ledger. | The finding is confirmed, but the canonical ledger has a separate atomic update protocol. | auto |
| D7 | C7, C8 | Make the Codex body helper use the same canonical model-id derivation and thinking-map result as the validated stream path, with no raw fallback. | Eliminates two public-helper/production-path divergences. | auto |
| D8 | C9 | Preserve locally generated subset-invariant messages through a typed usage-error path; continue replacing malformed upstream payload details with generic text. | Improves diagnostics without echoing provider-controlled data. | auto |
| D9 | C10 | Give secret-bearing encode/decode intermediates RAII zeroization through a small serde-compatible wrapper or explicit zeroizing `Drop` guard. | Makes cleanup panic-safe while keeping exposure confined to the serialization boundary. | auto |
| D10 | C13, C14 | Add one standard Responses HTTP test using data-only SSE and one focused split-frame buffering test. | Covers the canonical wire shape and the buffering invariant independently. | auto |
| D11 | C11, C12, C15-C22 | Take no remediation code change. Retain current tests/contracts and record the reasons in scope exclusions. | These items are dependency-watch notes, intentional behavior, superseded wording, or test-only implementation details. | auto |

### D4 decision record

**a. Narrow the specification (selected by the user).** Keep `write` and `delete` as
serialized, unconditional last-writer-wins mutations; correct the trait/module
comments and the Phase 14 design wording so acquire-then-re-read applies to the
OAuth refresh read-modify-write transaction. No public API change.

**b. Add post-lock mutation semantics (not selected).** Define observable behavior for state
changes found after lock acquisition (for example expected credential kind or
generation), extend the API as needed, and add write/write, write/delete, and
kind-transition race tests. This is a wider unstable-0.x API and concurrency
contract change.

Do not implement a re-read whose result is ignored.

## Remediation layers

### Layer 1: `opi-ai` (substrate)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-ai --all-targets -- -D warnings
    cargo test -p opi-ai --all-targets

#### Fix 1.1: Scope credential-derived routing to Copilot

- **Audit source**: Codex 2.1
- **Cluster**: C1
- **Decision**: D1
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/anthropic.rs` ~L1285-L1360; `crates/opi-ai/src/openai_codex_responses.rs` ~L260-L355; `crates/opi-ai/tests/oauth_wire_shape.rs`; `crates/opi-ai/tests/openai_codex_responses.rs`
- **Change**: Gate Anthropic credential URL consumption on the explicit Copilot-route setting. Remove credential URL precedence from the dedicated Codex provider. Use each provider's existing constructor/model URL seam for mock routing.
- **Test plan**: Add negative tests with a legitimate constructor URL and a second resolver-supplied URL; assert the direct request and authorization header reach only the constructor URL. Retain a positive mapped-Copilot enterprise URL test.

#### Fix 1.2: Make mapped catalog replacement transactional

- **Audit source**: Codex 3.1
- **Cluster**: C3
- **Decision**: D3
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/api_mapped.rs` ~L204-L243; `crates/opi-ai/tests/api_mapped_provider.rs`
- **Change**: Build all non-empty per-route subsets and validate route coverage before calling any route. Snapshot route catalogs, apply replacements, and restore prior catalogs in reverse order if a later route rejects its subset. Update the aggregate catalog only after every route succeeds.
- **Test plan**: Add `mapped_catalog_replacement_rolls_back_when_later_route_is_empty` and `mapped_catalog_replacement_rolls_back_when_late_route_rejects`; assert aggregate and every concrete route retain their original catalogs after failure.

#### Fix 1.3: Unify Codex body model/thinking derivation

- **Audit source**: GLM 2.1, GLM 2.2
- **Cluster**: C7, C8
- **Decision**: D7
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/openai_codex_responses.rs` ~L67-L123; `crates/opi-ai/tests/openai_codex_responses.rs`
- **Change**: Strip only `openai-codex:` in `build_request_body` and remove the raw thinking-level fallback after `thinking_level_map.resolve` rejects or suppresses a level.
- **Test plan**: Add body-helper tests for a foreign provider prefix and an unsupported mapped thinking level; neither may silently emit the canonical Codex model/reasoning value.

#### Fix 1.4: Preserve safe usage-subset diagnostics

- **Audit source**: GLM 2.3
- **Cluster**: C9
- **Decision**: D8
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/openai_chat.rs` ~L35-L60, ~L1130-L1150, ~L1240-L1260; `crates/opi-ai/src/openai_responses_shared.rs` ~L280-L370; `crates/opi-ai/src/openai_responses.rs` ~L330-L350, ~L440-L455; corresponding fixture tests
- **Change**: Distinguish locally generated usage-validation failures from malformed upstream frames. Surface the invariant message for the former and keep generic redacted text for provider-controlled parse failures.
- **Test plan**: Strengthen malformed-subset tests for Chat and Responses to assert the exact safe relationship error while existing malformed-JSON tests continue to assert generic errors without payload echo.

#### Fix 1.5: Cover canonical Responses SSE and split buffering

- **Audit source**: GLM 5.1, GLM 5.2
- **Cluster**: C13, C14
- **Decision**: D10
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/tests/openai_responses_lifecycle.rs` ~L40-L66; `crates/opi-ai/tests/openai_responses_fixtures.rs` ~L404-L578; `crates/opi-ai/src/openai_responses_shared.rs` ~L795-L811
- **Change**: Add realistic data-only standard Responses coverage and a focused buffer test whose first chunk ends mid-`data:` line.
- **Test plan**: The HTTP-level data-only stream must complete with text and usage. The split-frame test must emit nothing for the first partial chunk, retain it, and produce the terminal event after the second chunk.

### Layer 2: `opi-coding-agent` (product and persistence)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-coding-agent --all-targets -- -D warnings
    cargo test -p opi-coding-agent --all-targets

#### Fix 2.1: Make public credential reads marker-first

- **Audit source**: Codex 2.2
- **Cluster**: C2
- **Decision**: D2
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/credential.rs` ~L220-L265; `crates/opi-coding-agent/src/credential_store.rs` ~L760-L840, ~L980-L1030, ~L1135-L1180; `crates/opi-coding-agent/tests/credential_store.rs`
- **Change**: Implement one internal consistent-read routine that reads the marker first, returns `None` on marker absence, requires a protected envelope when a marker is present, and verifies marker/envelope kind agreement. Route public `CredentialStore::read`, API-key resolution, and OAuth resolution through it without weakening env-fallback rules.
- **Test plan**: Add public-method cases for protected-only, marker-only, corrupt marker, both mismatch directions, and a paused kind transition. Update malformed/unknown-envelope tests to seed the matching marker so they continue testing envelope decoding rather than marker absence.

#### Fix 2.2: Make credential intermediate cleanup RAII-safe

- **Audit source**: GLM 3.1
- **Cluster**: C10
- **Decision**: D9
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/credential_store.rs` ~L410-L625; `crates/opi-coding-agent/tests/credential_store.rs`
- **Change**: Wrap encode and decode envelope secret fields in a serde-compatible zeroizing owner so every early return and panic path zeroizes intermediate strings. Keep `base_url`, `account_id`, timestamps, and discriminator fields as ordinary non-secret values.
- **Test plan**: Retain round-trip/redaction tests and add a drop-path unit test using an injectable zeroize-observation wrapper if it can be done without exposing real secret bytes; otherwise pin the wrapper types with a compile-time trait test and preserve canary scans.

#### Fix 2.3: Pin the direct-provider routing boundary through factory construction

- **Audit source**: Codex 2.1
- **Cluster**: C1
- **Decision**: D1
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/tests/oauth_auth.rs` ~L4300-L4500; provider factory construction near the direct Anthropic/Codex builders
- **Change**: Update tests that currently use stored Codex `base_url` as a mock seam to use provider configuration/construction injection. Add factory-built negative coverage for stale stored metadata on direct Anthropic and Codex.
- **Test plan**: A fake store may contain an alternate URL, but requests must reach the configured Anthropic endpoint and the injected Codex transport endpoint; Copilot tests must continue observing stored enterprise routing changes on the next stream.

#### Fix 2.4: Prove rejected extension overrides leave the live mapped provider unchanged

- **Audit source**: Codex 3.1
- **Cluster**: C3
- **Decision**: D3
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/provider_factory.rs` ~L2113-L2152; `crates/opi-coding-agent/src/harness.rs` ~L730-L845; `crates/opi-coding-agent/tests/extension_mapped_catalog.rs`
- **Change**: No product behavior change should be needed after Fix 1.2; add a production-reachability regression test through `assemble_harness_collection`.
- **Test plan**: Override the sole model on a later wire so that route becomes empty. Assert one diagnostic is emitted, collection resolution still reflects the old aggregate catalog, and streams on every route still use old route metadata.

#### Fix 2.5: Resolve public mutation semantics

- **Audit source**: Codex 3.2
- **Cluster**: C4
- **Decision**: D4a (selected by user)
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/src/credential.rs` ~L225-L265; `crates/opi-coding-agent/src/credential_store.rs` module docs and ~L840-L865; Phase 14 design wording
- **Change**: Correct the contract/comments and retain current locked last-writer-wins behavior. Do not add an unused read.
- **Test plan**: Keep the existing lock-serialization and kind-transition tests.

### Layer 3: Documentation (final layer)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-coding-agent --all-targets -- -D warnings
    cargo test -p opi-coding-agent --test phase14_provider_auth_docs

#### Fix 3.1: Reconcile Browser OAuth and non-goal wording

- **Audit source**: Codex 4.1
- **Cluster**: C5
- **Decision**: D5
- **Verification status**: Confirmed
- **File(s)**: `docs/opi-spec.md` ~L1590-L1600; `docs/opi-spec.zh.md` ~L1345-L1350; `crates/opi-ai/README.md` ~L260-L276; `crates/opi-ai/README.zh.md` ~L240-L247; `crates/opi-coding-agent/README.md` ~L488-L496; `crates/opi-coding-agent/README.zh.md` ~L445-L453; `crates/opi-coding-agent/tests/phase14_provider_auth_docs.rs`
- **Change**: Mark the Phase 12 statement as historical to that phase and replace current unqualified "browser usage" non-goals with "browser automation outside the approved Anthropic/OpenAI Codex OAuth login flows." Update English and Chinese counterparts together.
- **Test plan**: Extend the docs guard to reject the unqualified phrase on current-behavior surfaces and require the approved OAuth exception in every touched localized pair.

## Guarded ledger handoff (outside `opi-remediate` execution)

### Fix H1: Repair Task 14.5 acceptance metadata

- **Audit source**: Codex 4.2
- **Cluster**: C6
- **Decision**: D6
- **Verification status**: Confirmed
- **File(s)**: `docs/snapshots/phase14/opi-impl-state.json` ~L1000-L1010 and scenario status fields; actual test at `crates/opi-coding-agent/tests/session_runtime.rs` ~L2577
- **Change**: Invoke the guarded Phase 14 `opi-implement` ledger reconciliation flow. Remove the nonexistent `usage_cost_wiring` target/file, replace the stale filter with `phase14_usage_subsets_survive_session_resume`, and reconcile the affected scenario status/evidence. Do not hand-edit the JSON.
- **Test plan**: The guarded ledger validator must prove every repaired command selects at least one intended test. Run `cargo test -p opi-coding-agent --test session_runtime phase14_usage_subsets_survive_session_resume -- --exact` and record selected count `1`.

`opi-remediate` must not modify `.opi-impl-state.json`; this item is a required
handoff after the code/documentation remediation is accepted.

## Final verification

    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

Also rerun each new focused regression test by exact name and verify the
selected-test count is nonzero.

## Scope exclusions

| Finding | Status | Reason |
|---------|--------|--------|
| C11 | Info/No action | Current classification is intentionally narrow and tested; failure requires future dependency error-text drift. Re-evaluate whenever the Secret Service/keyring dependency changes. |
| C12 | Info/No action | Cosmetic label selection only. Adding a public provider-error variant is disproportionate to this Phase 14 remediation. |
| C15 | Refuted | The current normative spec and implementation use `CacheRetention::None` as the provider-default sentinel; behavior is equivalent to the superseded historical `Option` spelling. |
| C16 | Refuted | The current normative spec records `Vec<(String, String)>`, and the provider boundary validates both names and values before I/O. |
| C17 | Info/No action | The dedicated Codex contract requires affinity headers and explicitly synthesizes them unless caching is disabled. |
| C18 | Info/No action | Provider mappers enforce the invariant on untrusted wire data. A validated constructor cannot enforce it while `Usage` fields remain public; no Phase 14 API redesign is justified. |
| C19 | Info/No action | Saturation is already normative in `docs/opi-spec.md` and test-pinned; a local rustdoc enhancement may be made only if documentation scope is intentionally widened. |
| C20 | Info/No action | The normative spec intentionally maps `AccountIdMissing` to the `CredentialNeeded` remediation event while retaining its distinct diagnostic. |
| C21 | Refuted | `user_config_dir()` already returns the opi-specific directory, so appending another `opi` segment would be wrong. |
| C22 | Info/No action | Blocking sleep exists only in a multi-threaded fake backend used to prove lock serialization; no production behavior is affected. |
| GLM 4.1, 4.2 | Resolved/No action | The data-only Codex decoder and typed `AccountIdMissing` paths are present and separately covered; these were not active findings in the audited report. |
