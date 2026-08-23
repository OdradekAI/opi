# Phase 17 Remediation Plan

**Status**: READY-FOR-APPLY
**Verification target**: committed `96f7d161045c94113ec9f02f5ad3ff4c8121cea5`
**Round**: r1
**Finding sources**: `docs/snapshots/phase17/audit.codex.96f7d16.20260822t180722z.md`; `docs/snapshots/phase17/audit.glm53.96f7d16.20260822t182354z.md`
**Disposition artifact**: `remediation.96f7d16.r1.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged none; unstaged none; untracked `audit.codex.md`, `docs/snapshots/phase17/audit.codex.96f7d16.20260822t180722z.findings.jsonl`, `docs/snapshots/phase17/audit.codex.96f7d16.20260822t180722z.md`, `docs/snapshots/phase17/audit.glm53.96f7d16.20260822t182354z.findings.jsonl`, and `docs/snapshots/phase17/audit.glm53.96f7d16.20260822t182354z.md`
**Unresolved decisions**: none

The endpoint was verified from a detached isolated checkout. Both normalized source sidecars validate, contain 37 unique findings (5 Codex and 32 GLM), and bind to the full committed target. No prior immutable remediation result dispositions exist, so the lineage comparator classifies every record as `new`; legacy narrative remediation prose is degraded context only and is not recurrence evidence.

## Lineage and verification summary

| Source finding | Verification | Final severity + rationale | Lineage | Closure batch | Decision |
|---|---|---|---|---|---|
| P17-CODEX-001 | Confirmed | Major — v1 skips all unknown entries and has no durable binding | new | B1 | D1 |
| P17-CODEX-002 | Confirmed | Major — CRC-invalid complete frames are silently consumed | new | B2 | D2 |
| P17-CODEX-003 | Confirmed | Minor — post-Allow health change is not reauthorized | new | B3 | D3 |
| P17-CODEX-004 | Confirmed | Minor — exact external-cache test fails to find the binary | new | B4 | D4 |
| P17-CODEX-005 | Confirmed | Minor — exit evidence omitted the profile; plan-stage reverse/tree-equality preflight passed | new | B5 | D5 |
| F-001 | Confirmed | Minor — false Unreleased wording; documentation-only downgrade | new | B6 | D6 |
| F-002 | Partially confirmed | Minor — representative citation drift confirmed | new | none | no-action:D7 |
| F-003 | Confirmed | Minor — cited CI run failed overall; later run passed | new | none | no-action:D8 |
| F-004 | Confirmed | Minor — frozen threshold enumeration omits one item | new | none | no-action:D9 |
| F-005 | Confirmed | Minor — live ledger design hash is stale | new | none | no-action:D10 |
| F-006 | Partially confirmed | Info — public seam exists, but no live bypass; downgraded | new | none | no-action:D11 |
| F-007 | Confirmed | Minor — consecutive projection test is absent | new | B7 | D12 |
| F-008 | Confirmed | Minor — actual-source forgery matrix is absent | new | B8 | D13 |
| F-009 | Confirmed | Minor — empty manifest records panic | new | B9 | D14 |
| F-010 | Confirmed | Minor — contract test builds an invalid Retry record | new | B10 | D15 |
| F-011 | Confirmed | Minor — two early precedence edges lack counters | new | B11 | D16 |
| F-012 | Confirmed | Minor — anyhow is declared and unused | new | B12 | D17 |
| F-013 | Cannot confirm | Minor — exact test passed three consecutive runs | new | none | no-action:D18 |
| F-014 | Confirmed | Minor — three current module docs retain sprint history | new | B13 | D19 |
| F-015 | Confirmed | Info — weak isolated test has stronger boundary coverage | new | none | no-action:D20 |
| F-016 | Confirmed | Info — populated-queue probes cover the meaningful case | new | none | no-action:D20 |
| F-017 | Confirmed | Info — public health/finalization contract is sufficient | new | none | no-action:D20 |
| F-018 | Confirmed | Info — product compaction covers context replacement | new | none | no-action:D20 |
| F-019 | Confirmed | Info — prepared-call and evidence tests jointly prove route | new | none | no-action:D20 |
| F-020 | Confirmed | Info — no evidence reader is a registered design choice | new | none | no-action:D20 |
| F-021 | Confirmed | Info — no violating helper was found | new | none | no-action:D20 |
| F-022 | Confirmed | Info — overflow is warned and no success conversion exists | new | none | no-action:D20 |
| F-023 | Refuted | Info — current ActiveSnapshot rejection is required | new | none | no-action:D23 |
| F-024 | Confirmed | Info — zero artifacts is deliberate and valid | new | none | no-action:D20 |
| F-025 | Refuted | Info — RPC recorder behavior matches registered mapping | new | none | no-action:D23 |
| F-026 | Confirmed | Info — stale registered-design name is shaping-owned | new | none | no-action:D21 |
| F-027 | Partially confirmed | Info — polling exists; count and locations do not match | new | none | no-action:D20 |
| F-028 | Confirmed | Info — no clap wiring defect was found | new | none | no-action:D20 |
| F-029 | Confirmed | Info — torn-tail recovery is deliberate | new | none | no-action:D20 |
| F-030 | Confirmed | Info — harmless adjacent rustdoc duplication | new | none | no-action:D20 |
| F-031 | Refuted | Info — Option::None is the intended disabled capture | new | none | no-action:D23 |
| F-032 | Confirmed | Info — pre-existing Phase 14 substrate is outside scope | new | none | no-action:D20 |

Totals: 30 Confirmed, 3 Partially confirmed, 1 Cannot confirm, 3 Refuted; 13 actionable closure batches and 24 verified exclusions. Final severity is 2 Major, 16 Minor, and 19 Info.

## Decision record

| ID | Decision | Governing evidence |
|---|---|---|
| D1 | Write a version-2 durable session header with immutable `RuntimeInputBinding` and typed entry envelopes; keep v1 as legacy read-only input, reject unsupported versions and unknown required v2 entries, and skip only explicitly ignorable observations. Return the validated committed prefix and binding together to resume, fork, export, and evidence callers. | INV-007 and P17-MIG-002 define the exact fail-closed meaning; a format-version boundary avoids guessing v1 intent. |
| D2 | Distinguish incomplete input from malformed complete Bedrock frames. A CRC/length-invalid complete frame emits one `ProviderError::StreamError` and terminates both fixture and HTTP decode loops before later Done. | P17-FAL-001/002 and provider error semantics. |
| D3 | If evidence generation changes while recording an Allow, rebuild the identical authorization request from live health, authorize once again, record that decision, and launch only if the new decision and generation validate. | P17-EVD-009. |
| D4 | Resolve the integration-test executable only through Cargo's `CARGO_BIN_EXE_opi` contract; remove the workspace-target fallback. | Repository external-cache workflow and Cargo integration-test contract. |
| D5 | Drill the complete registered range `4c4a8404b8a05e09f3479bc320c6f361ed2c9437..40f2e6ee4866f1cd44eefb952b8f40afcbb029ac` from a detached Phase-exit checkout, prove the resulting tree equals `4c4a8404b8a05e09f3479bc320c6f361ed2c9437`, run the pre-Phase regression profile, and record exact outcomes in the immutable result. | P17-RBK-002. |
| D6 | Remove only the false Unreleased clause saying `FileEvidenceSink::dir` was removed; preserve the intentional public accessor. | Current source and no registered authorization for API removal. |
| D7 | Do not rewrite frozen citation history; use these fresh dispositions and a fresh audit as renewed evidence. | Snapshot immutability. |
| D8 | Do not rewrite the frozen CI claim; retain the independent run comparison in remediation evidence. | Snapshot immutability; later fully green run. |
| D9 | Do not rewrite the frozen RBK enumeration; renewed audit owns complete threshold review. | Snapshot immutability. |
| D10 | Return the stale live ledger hash to an explicitly invoked `opi-implement` workflow; `opi-remediate` does not edit `.opi-impl-state.json`. | Workflow ownership rule. |
| D11 | Make no API/architecture change for the name-based builtin helper without a demonstrated untrusted caller or registered removal decision. | Minimum-change and public 0.x boundary rules. |
| D12 | Add the named two-turn product projection test without production abstraction. | AUT-008 mechanical verification. |
| D13 | Add one malicious-content matrix through actual model-visible convergence points; do not invent retrieval or child-agent runtime seams. | A07 and current type boundaries. |
| D14 | Validate non-empty records before correlation and return `EvidenceError::Finalization`; never unwind. | Public typed-error contract. |
| D15 | Make the Retry contract test use structured valid Retry facts and pass sink validation before parent-link assertions. | Evidence sink kind/payload contract. |
| D16 | Add observable hook/authorizer counters for both early precedence edges. | P17-FAL-002. |
| D17 | Remove the unused direct dependency from crate/root workspace manifests, regenerate `Cargo.lock` with Cargo, and review the diff. | Dependency and supply-chain rules. |
| D18 | No change after three consecutive passes; retain the source severity and require deterministic reproduction before altering timeout policy. | Verification evidence. |
| D19 | Remove only `S8.1`, `S8.2`, and `S10` from the three cited module-opening comments while retaining their current contract text. | AGENTS.md comment rule and minimum scope. |
| D20 | Take no action on independently covered or informational observations that do not expose incorrect runtime behavior. | Minimum-change rule and cited stronger coverage. |
| D21 | Return the stale `TraceConfig` name to human-led shaping; do not mutate the registered supplemental design in remediation. | Registered-source authority. |
| D23 | Take no action on refuted findings whose proposed change would contradict current registered semantics. | Current normative/design contracts. |

## Unresolved decisions

none

## Closure batches

### Batch B1: Durable session meaning and binding

**Closure predicate**: New sessions retain one immutable runtime-input binding with their validated committed prefix; unknown required v2 entries and unsupported versions fail typed, while only explicitly ignorable observations can be skipped.
**Dependencies**: none
**Verification union**: focused `opi-agent` session tests, product resume/fork/export/evidence tests, then workspace gates.

#### Fix B1.1: Introduce the versioned durable session envelope

- **Finding source**: `docs/snapshots/phase17/audit.codex.96f7d16.20260822t180722z.md` — `P17-CODEX-001`
- **Lineage**: new; no prior immutable result disposition
- **Decision**: D1
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/session.rs`, `crates/opi-agent/tests/session_storage.rs`, `crates/opi-agent/tests/session_branching.rs`, `crates/opi-coding-agent/src/harness.rs`, `crates/opi-coding-agent/src/session_cli.rs`, and their focused session tests
- **Change kind**: behavioral
- **Change**: Add v2 header/binding and required-versus-ignorable entry classification, retain v1 as byte-preserving legacy input, and make every product caller consume the returned prefix/binding pair.
- **Closure predicate**: Resume, fork, export, and evidence reconstruct from the same validated prefix/binding pair and cannot silently accept lost required semantics.
- **Red-before**: `cargo test -p opi-agent --test remediation_phase17_session_red` — observed FAIL: `unsupported session version 2, expected 1`.
- **Green-after**: `cargo test -p opi-agent --test session_storage v2_retains_runtime_binding_and_rejects_unknown_required_entries -- --exact` — expected PASS, followed by all focused session binaries.

### Batch B2: Bedrock frame integrity

**Closure predicate**: A complete malformed Bedrock event-stream frame produces exactly one typed stream error and no later event.
**Dependencies**: none
**Verification union**: Bedrock unit/fixture tests and workspace gates.

#### Fix B2.1: Terminate on CRC-invalid complete frames

- **Finding source**: `docs/snapshots/phase17/audit.codex.96f7d16.20260822t180722z.md` — `P17-CODEX-002`
- **Lineage**: new; no prior immutable result disposition
- **Decision**: D2
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/providers/bedrock/event_stream.rs` and `crates/opi-ai/tests/bedrock_fixtures.rs`
- **Change kind**: behavioral
- **Change**: Return distinct incomplete/malformed outcomes, map a complete malformed frame to one typed error, and terminate fixture and HTTP loops.
- **Closure predicate**: CRC/length corruption cannot be silently consumed or followed by Done.
- **Red-before**: `cargo test -p opi-ai --test remediation_phase17_bedrock_red` — observed FAIL: current output was `[Ok(Start), Ok(Done)]`.
- **Green-after**: `cargo test -p opi-ai --test bedrock_fixtures crc_invalid_complete_frame_emits_one_error_and_stops -- --exact` — expected PASS.

### Batch B3: Post-Allow evidence reauthorization

**Closure predicate**: No tool launches under an Allow bound to a stale evidence-health generation.
**Dependencies**: none
**Verification union**: evidence runtime and authority tests, then workspace gates.

#### Fix B3.1: Reauthorize after authorization evidence failure

- **Finding source**: `docs/snapshots/phase17/audit.codex.96f7d16.20260822t180722z.md` — `P17-CODEX-003`
- **Lineage**: new; no prior immutable result disposition
- **Decision**: D3
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/agent_loop.rs` and `crates/opi-agent/tests/evidence_runtime.rs`
- **Change kind**: behavioral
- **Change**: Rebuild and reauthorize the identical request once when decision evidence changes health; validate and record the second decision before launch.
- **Closure predicate**: Complete-evidence policy observes the new generation and all affected tool launches remain zero.
- **Red-before**: `cargo test -p opi-agent --test evidence_runtime parallel_authorization_record_failure_on_first_or_second_launches_zero_tools -- --exact` with a counting complete-evidence authorizer — observed FAIL, calls `2` rather than `3`.
- **Green-after**: the same exact test with the counting assertion — expected PASS with three calls and zero launches.

### Batch B4: Cargo-resolved CLI test binary

**Closure predicate**: Session CLI integration tests use the exact binary Cargo built, independent of target-directory location.
**Dependencies**: none
**Verification union**: full `session_cli` binary and workspace tests under external cache.

#### Fix B4.1: Remove the workspace-target executable fallback

- **Finding source**: `docs/snapshots/phase17/audit.codex.96f7d16.20260822t180722z.md` — `P17-CODEX-004`
- **Lineage**: new; no prior immutable result disposition
- **Decision**: D4
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/tests/session_cli.rs`
- **Change kind**: test-only
- **Change**: Use Cargo's `CARGO_BIN_EXE_opi` integration-test executable and delete the checkout-relative fallback.
- **Closure predicate**: The exact test passes with the repository external `CARGO_TARGET_DIR`.
- **Red-before**: `cargo test -p opi-coding-agent --test session_cli e2e_list_sessions_empty_exits_zero -- --exact` — observed FAIL: `opi binary must be built`.
- **Green-after**: the same exact command — expected PASS; then `cargo test -p opi-coding-agent --test session_cli`.

### Batch B5: Registered rollback drill

**Closure predicate**: Reverting the complete Phase 17 range in isolation yields exactly the pre-Phase tree and its representative runtime regression profile passes.
**Dependencies**: none
**Verification union**: exact tree comparison, pre-Phase tests, and the already-registered RBK-003/RBK-004 checks at the Phase-exit endpoint.

#### Fix B5.1: Execute and record the pre-Phase runtime profile

- **Finding source**: `docs/snapshots/phase17/audit.codex.96f7d16.20260822t180722z.md` — `P17-CODEX-005`
- **Lineage**: new; no prior immutable result disposition
- **Decision**: D5
- **Verification status**: Confirmed
- **File(s)**: isolated temporary Git worktree and immutable remediation result only
- **Change kind**: metadata
- **Change**: Require `C:\Users\Luiz\AppData\Local\Temp\opi-remediate-phase17-rollback-96f7d16-r1` to be absent, add it as a detached worktree at `40f2e6ee4866f1cd44eefb952b8f40afcbb029ac`, run `git revert --no-commit 4c4a8404b8a05e09f3479bc320c6f361ed2c9437..40f2e6ee4866f1cd44eefb952b8f40afcbb029ac`, require the working tree to equal `4c4a8404b8a05e09f3479bc320c6f361ed2c9437`, run `provider_collection`, `session_storage`, `hooks_queues`, `session_runtime`, and `non_interactive_policy` through that worktree's manifest, record commit IDs, commands, exit codes, and the RBK-003/RBK-004 supporting test outcomes in the result artifact, then remove only this verified owned probe worktree.
- **Closure predicate**: The reverse change has one coherent pre-Phase runtime tree and its representative provider/session/hook/product-policy paths pass.
- **Red-before**: N/A — this is missing mechanical evidence, not a production behavior edit. Plan preflight observed `git revert --no-commit 4c4a8404b8a05e09f3479bc320c6f361ed2c9437..40f2e6ee4866f1cd44eefb952b8f40afcbb029ac` exit 0 and `git diff --exit-code 4c4a8404b8a05e09f3479bc320c6f361ed2c9437` exit 0 in an owned detached probe; the Cargo regression profile remains apply-stage work.
- **Green-after**: `git -C C:\Users\Luiz\AppData\Local\Temp\opi-remediate-phase17-rollback-96f7d16-r1 diff --exit-code 4c4a8404b8a05e09f3479bc320c6f361ed2c9437 && cargo test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-remediate-phase17-rollback-96f7d16-r1\Cargo.toml -p opi-ai --test provider_collection && cargo test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-remediate-phase17-rollback-96f7d16-r1\Cargo.toml -p opi-agent --test session_storage --test hooks_queues && cargo test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-remediate-phase17-rollback-96f7d16-r1\Cargo.toml -p opi-coding-agent --test session_runtime --test non_interactive_policy` — expected PASS with exact outcomes recorded.

### Batch B6: Unreleased changelog truth

**Closure predicate**: Unreleased history no longer claims that a still-public accessor was removed.
**Dependencies**: none
**Verification union**: documentation contract and diff check.

#### Fix B6.1: Remove the false accessor-removal clause

- **Finding source**: `docs/snapshots/phase17/audit.glm53.96f7d16.20260822t182354z.md` — `F-001`
- **Lineage**: new; no prior immutable result disposition
- **Decision**: D6
- **Verification status**: Confirmed
- **File(s)**: `CHANGELOG.md`
- **Change kind**: documentation
- **Change**: Remove only the false `FileEvidenceSink::dir` removal wording from the complete existing Unreleased subsection.
- **Closure predicate**: Changelog and public API agree.
- **Red-before**: N/A — documentation truth correction.
- **Green-after**: `python scripts/opi-doc-check.py` — expected PASS.

### Batch B7: Consecutive trusted-tool projection

**Closure predicate**: Two consecutive requests independently project the same allowed trusted definitions and exclude the same disallowed tool.
**Dependencies**: none
**Verification union**: focused product authority test and workspace tests.

#### Fix B7.1: Add the AUT-008 consecutive-request test

- **Finding source**: `docs/snapshots/phase17/audit.glm53.96f7d16.20260822t182354z.md` — `F-007`
- **Lineage**: new; no prior immutable result disposition
- **Decision**: D12
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/tests/phase17_tool_authority.rs`
- **Change kind**: test-only
- **Change**: Drive two provider requests through the public product seam and inspect both provider-visible tool lists.
- **Closure predicate**: Projection is recomputed per request without stale or widened visibility.
- **Red-before**: N/A — current production structure already recomputes; the missing item is registered test coverage.
- **Green-after**: `cargo test -p opi-coding-agent --test phase17_tool_authority phase17_tool_projection_is_recomputed_for_consecutive_requests -- --exact` — expected PASS.

### Batch B8: Actual-source authority-forgery matrix

**Closure predicate**: Content originating from hooks, tool output, retrieval-shaped content, skills, or child-shaped content cannot create trusted registration metadata or cause execution.
**Dependencies**: B7
**Verification union**: focused product authority tests and workspace tests.

#### Fix B8.1: Exercise A07 through real content convergence points

- **Finding source**: `docs/snapshots/phase17/audit.glm53.96f7d16.20260822t182354z.md` — `F-008`
- **Lineage**: new; no prior immutable result disposition
- **Decision**: D13
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/tests/phase17_tool_authority.rs`
- **Change kind**: test-only
- **Change**: Add one malicious-content matrix using actual message/tool/hook convergence; do not invent absent retrieval or child-agent abstractions.
- **Closure predicate**: Origin, capability, and policy facts remain trusted-only and execution count is zero for every vector.
- **Red-before**: N/A — type closure exists; registered end-to-end coverage is missing.
- **Green-after**: `cargo test -p opi-coding-agent --test phase17_tool_authority untrusted_content_sources_cannot_forge_tool_authority -- --exact` — expected PASS.

### Batch B9: Typed empty-manifest failure

**Closure predicate**: Empty evidence input returns a typed finalization error without unwinding.
**Dependencies**: none
**Verification union**: product evidence tests and workspace gates.

#### Fix B9.1: Validate records before terminal correlation

- **Finding source**: `docs/snapshots/phase17/audit.glm53.96f7d16.20260822t182354z.md` — `F-009`
- **Lineage**: new; no prior immutable result disposition
- **Decision**: D14
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/evidence.rs` and `crates/opi-coding-agent/tests/phase17_product_evidence.rs`
- **Change kind**: behavioral
- **Change**: Return `EvidenceError::Finalization` before correlation when records are empty.
- **Closure predicate**: The public builder never panics for caller-supplied empty input.
- **Red-before**: `cargo test -p opi-coding-agent --test phase17_product_evidence file_evidence_sink_writes_records_and_manifest -- --exact` with the empty-input assertion — observed FAIL: `manifest correlation requires at least one record`.
- **Green-after**: the same exact test — expected PASS.

### Batch B10: Valid Retry evidence contract test

**Closure predicate**: The parent-link assertion is made on a Retry record accepted by the sink's kind/payload validation.
**Dependencies**: none
**Verification union**: evidence contract/runtime tests and workspace tests.

#### Fix B10.1: Build structured Retry facts in the direct contract test

- **Finding source**: `docs/snapshots/phase17/audit.glm53.96f7d16.20260822t182354z.md` — `F-010`
- **Lineage**: new; no prior immutable result disposition
- **Decision**: D15
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/tests/evidence_contract.rs`
- **Change kind**: test-only
- **Change**: Replace the invalid Digest payload with valid Retry facts and validate/emit it before asserting the parent.
- **Closure predicate**: The test cannot pass using evidence the production sink would reject.
- **Red-before**: N/A — test-quality correction; production retry coverage is already green.
- **Green-after**: `cargo test -p opi-agent --test evidence_contract parent_call_link_correlates_retry_to_origin -- --exact` — expected PASS.

### Batch B11: Pairwise early precedence proof

**Closure predicate**: An unknown tool invokes no hook, and invalid arguments invoke no authorizer.
**Dependencies**: none
**Verification union**: tool authority/validation tests and workspace tests.

#### Fix B11.1: Add observable early-edge counters

- **Finding source**: `docs/snapshots/phase17/audit.glm53.96f7d16.20260822t182354z.md` — `F-011`
- **Lineage**: new; no prior immutable result disposition
- **Decision**: D16
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/tests/tool_authority.rs` and `crates/opi-agent/tests/tool_validation.rs`
- **Change kind**: test-only
- **Change**: Add explicit hook/authorizer counters to the two registered invalid-input cases.
- **Closure predicate**: Both earlier failures prevent every later boundary from running.
- **Red-before**: N/A — production order is structural; observable coverage is missing.
- **Green-after**: `cargo test -p opi-agent --test tool_authority unknown_tool_skips_hook -- --exact && cargo test -p opi-agent --test tool_validation invalid_schema_skips_authorizer -- --exact` — expected PASS.

### Batch B12: Remove unused anyhow dependency

**Closure predicate**: Cargo metadata and lockfile contain no direct workspace/opi-coding-agent anyhow dependency and affected targets remain warning-free.
**Dependencies**: none
**Verification union**: Cargo metadata, affected clippy/tests, lockfile review, workspace gates.

#### Fix B12.1: Remove unused manifest entries

- **Finding source**: `docs/snapshots/phase17/audit.glm53.96f7d16.20260822t182354z.md` — `F-012`
- **Lineage**: new; no prior immutable result disposition
- **Decision**: D17
- **Verification status**: Confirmed
- **File(s)**: `Cargo.toml`, `crates/opi-coding-agent/Cargo.toml`, and Cargo-generated `Cargo.lock`
- **Change kind**: metadata
- **Change**: Delete the two unused declarations, regenerate the lockfile with Cargo, and review unexpected transitive changes.
- **Closure predicate**: The dependency is absent and no affected target loses compilation coverage.
- **Red-before**: N/A — dependency metadata cleanup.
- **Green-after**: `cargo metadata --no-deps --format-version 1 && cargo clippy -p opi-coding-agent --all-targets -- -D warnings` — expected PASS.

### Batch B13: Remove delivery-history tags from current module docs

**Closure predicate**: The three cited module-opening comments describe only current contracts.
**Dependencies**: none
**Verification union**: documentation contract, rustdoc, and affected clippy.

#### Fix B13.1: Delete the three cited sprint labels

- **Finding source**: `docs/snapshots/phase17/audit.glm53.96f7d16.20260822t182354z.md` — `F-014`
- **Lineage**: new; no prior immutable result disposition
- **Decision**: D19
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/provider.rs`, `crates/opi-agent/src/hooks.rs`, and `crates/opi-coding-agent/src/runner.rs`
- **Change kind**: documentation
- **Change**: Remove only the `S8.1`, `S8.2`, and `S10` suffixes from the three cited opening comments.
- **Closure predicate**: Those current-contract module docs contain no delivery-history label.
- **Red-before**: N/A — documentation-only correction.
- **Green-after**: `python scripts/opi-doc-check.py`, then set `$env:RUSTDOCFLAGS='-D warnings'` and run `cargo doc --workspace --no-deps` — both expected PASS.

## Final verification

All Cargo commands use the repository external cache under a lease:

    python scripts/opi-cargo-cache.py lease start --target E:\opi\cargo-targets\opi-remediate-96f7d16-r1 --pid $PID
    $env:CARGO_TARGET_DIR='E:\opi\cargo-targets\opi-remediate-96f7d16-r1'
    cargo test -p opi-ai --lib bedrock::event_stream
    cargo test -p opi-ai --test bedrock_fixtures
    cargo test -p opi-agent --test session_storage --test session_branching --test session_facade
    cargo test -p opi-agent --test evidence_runtime --test evidence_contract --test tool_authority --test tool_validation
    cargo test -p opi-coding-agent --test session_cli --test phase17_legacy_migration --test phase17_tool_authority --test phase17_product_evidence
    python scripts/opi-doc-check.py
    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc
    $env:RUSTDOCFLAGS='-D warnings'
    cargo doc --workspace --no-deps
    git diff --check
    python scripts/opi-cargo-cache.py lease end --target E:\opi\cargo-targets\opi-remediate-96f7d16-r1 --pid $PID

The B5 rollback profile runs in its own detached temporary worktree and external cache lease. The immutable result must record every command, commit, exit status, and cleanup path. Test impact planned: add focused coverage for v2 sessions, Bedrock corruption, consecutive projections, content-origin forgery, and precedence counters; update evidence/session/Retry/CLI tests; delete no tests; retain all existing gates.

## Exclusions

| Source finding | Disposition | Evidence/authority |
|---|---|---|
| F-002 | Returned to shaping / frozen history | Representative drift confirmed; immutable snapshots are not rewritten. |
| F-003 | Info-no-action on frozen history | Run 31798070731 failed overall and later run 32484643147 passed; retain both facts. |
| F-004 | Info-no-action on frozen history | Omitted enumeration is historical; fresh audit owns current threshold coverage. |
| F-005 | Returned to `opi-implement` | Only that explicit workflow owns `.opi-impl-state.json`. |
| F-006 | Info-no-action | No untrusted production caller or authority bypass was found; public removal is not authorized. |
| F-013 | Cannot confirm | Exact timeout test passed three consecutive runs. |
| F-015 | Info-no-action | Stronger runtime boundary coverage exists. |
| F-016 | Info-no-action | Populated-queue probes cover the meaningful state. |
| F-017 | Info-no-action | Registered public contract exposes health/finalization, not stored detail. |
| F-018 | Info-no-action | Product compaction covers replacement. |
| F-019 | Info-no-action | Prepared-call and evidence coverage jointly prove the route. |
| F-020 | Info-no-action | Evidence-reader absence is deliberate; byte identity is applicable. |
| F-021 | Info-no-action | No network/paid-provider helper violation was found. |
| F-022 | Info-no-action | Overflow is warned and no success conversion was found. |
| F-023 | Refuted | ActiveSnapshot rejection is current fail-closed behavior. |
| F-024 | Info-no-action | Empty artifact reference set is valid and deliberate. |
| F-025 | Refuted | RPC recorder matches the registered cross-mode mapping. |
| F-026 | Returned to shaping | Registered supplemental design changes require human-led shaping. |
| F-027 | Info-no-action | Claim is only partially accurate and no flake was reproduced. |
| F-028 | Info-no-action | No clap parsing/wiring defect was found. |
| F-029 | Info-no-action | Torn-tail truncation is registered crash recovery. |
| F-030 | Info-no-action | Harmless adjacent prose is outside minimum remediation. |
| F-031 | Refuted | `Option::None` is the intended disabled-capture representation. |
| F-032 | Info-no-action | Pre-existing Phase 14 unused substrate is a separate scope decision. |
