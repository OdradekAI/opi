# Phase 17 Remediation Plan

**Status**: READY-FOR-APPLY
**Verification target**: committed `528989279e9be308abc963ec22f377ee47bbde47`
**Round**: r2
**Finding sources**: docs/snapshots/phase17/audit.codex.5289892.20260823t055516z.md; docs/snapshots/phase17/audit.glm-5-3.5289892.20260823t052233z.md
**Disposition artifact**: `remediation.5289892.r2.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged none; unstaged none; untracked six carried-in immutable audit/r1-plan artifacts listed under Endpoint freeze
**Unresolved decisions**: none

## Endpoint freeze

- Remediation head: `528989279e9be308abc963ec22f377ee47bbde47`.
- Verified base: `origin/main`.
- Merge base: `136c380f0c5eea541190cc1a0f5c1d62f983b4e8`.
- Committed inventory: 81 paths from `git diff --name-status --find-renames 136c380f0c5eea541190cc1a0f5c1d62f983b4e8..528989279e9be308abc963ec22f377ee47bbde47`; exact paths are in Appendix A.
- Staged inventory: none.
- Unstaged inventory: none.
- Untracked carried-in inventory:
  - `docs/snapshots/phase17/audit.codex.5289892.20260823t055516z.findings.jsonl`
  - `docs/snapshots/phase17/audit.codex.5289892.20260823t055516z.md`
  - `docs/snapshots/phase17/audit.glm-5-3.5289892.20260823t052233z.findings.jsonl`
  - `docs/snapshots/phase17/audit.glm-5-3.5289892.20260823t052233z.md`
  - `docs/snapshots/phase17/remediation.5289892.r1.plan.dispositions.jsonl`
  - `docs/snapshots/phase17/remediation.5289892.r1.plan.md`
- Verification checkout: detached at the exact head in `D:/Luiz/Odradek/opi-remediate-plan-5289892-r2`. Temporary discriminating test edits were restored after observing both new red-before failures; only this plan and its disposition sibling are written to the original worktree.

## Lineage and verification summary

Both finding sources report `fresh-context-same-family` independence. Overlap is correlated/degraded rather than an independent vote. The CI claim has full overlap; the schema-v1 behavior and fixture claims partially overlap; the remaining claims have one source. Source validation and unchanged r1 verification evidence were reused only at the same committed head; D1/D2 received new discriminating red-before observations in this round.

| Source finding | Verification | Final severity + rationale | Lineage | Closure batch | Decision |
|---|---|---|---|---|---|
| Codex P17-AUD-001 | Confirmed | Major — the approved compatibility contract keeps v1 resumable/forkable through a byte-preserving v2 child, while the current product rejects genuine v1 before route normalization | recurrent-adjacent-path | B2 | D1 |
| Codex P17-AUD-002 | Confirmed | Major — session-backed setup discards the exact per-run material binding and keeps one cwd/initial-model digest across an external between-run model change | recurrent-adjacent-path | B1 | D2 |
| Codex P17-AUD-003 | Confirmed | Major — the exact audited implementation head has no three-platform CI run | recurrent-adjacent-path | none | no-action:D3 |
| Codex P17-AUD-004 | Confirmed | Major — the focused test is 0/8 under the required external Cargo target and blocks the workspace all-targets gate | recurrent-adjacent-path | B5 | D4 |
| Codex P17-AUD-005 | Confirmed | Major — the production HTTP decoder emits Done when a valid terminal frame is followed by incomplete decoder residue | recurrent-adjacent-path | B6 | D5 |
| GLM phase17-5289892-01 | Confirmed | Minor — the complete Unreleased section omits the user-visible session-v2 and byte-preserving v1 transition contract | recurrent-adjacent-path | B3 | D1 |
| GLM phase17-5289892-02 | Confirmed | Minor — both crate README counterparts still describe the v1 additive/skip-unknown contract | recurrent-adjacent-path | B4 | D1 |
| GLM phase17-5289892-03 | Confirmed | Major — all seven advertised legacy migration tests pass over schema-v2 files and no product-seam test pins genuine v1 behavior | recurrent-adjacent-path | B2 | D1 |
| GLM phase17-5289892-04 | Partially confirmed | Info — the accessor has no workspace caller, but no incorrect behavior or authorized removal is established | new | none | no-action:D6 |
| GLM phase17-5289892-05 | Confirmed | Major — same exact-head CI gap as Codex P17-AUD-003; local Windows evidence cannot establish mandatory three-platform acceptance | recurrent-adjacent-path | none | no-action:D3 |

## Unresolved decisions

none.

## Recorded decisions

| ID | Decision | Governing evidence |
|---|---|---|
| D1 | Schema v1 remains read-compatible for Reference Product resume and fork. The v1 source stays byte-identical and is never reopened for writing. After deterministic route normalization and before new execution, the product creates and adopts one parented v2 child carrying the exact current Direct Runtime Input binding; only v2 is written. Unsupported/corrupt input and a v1 route that cannot be uniquely normalized remain fail-closed before provider/tool dispatch and do not create a guessed mutable child. | Human Authority approval in the conversation following r1; `P17-MIG-001`, `P17-MIG-002`, `P17-A13`, and parent `INV-007`. |
| D2 | One mutable v2 session file/branch has one immutable Runtime Input Binding. Before every run, trusted product assembly derives the exact binding from the same canonical external material inputs used by evidence. If those inputs changed between runs, the product creates and adopts a parented v2 child before evidence setup or provider/tool side effects. A provider/model change produced inside an already armed run by validated `NextTurnState` is runtime state and route evidence, not a binding mutation; a product/user model change between runs is a new material input and therefore a new bound child. | Human Authority approval in the conversation following r1; domain `Runtime Input Binding`, `INV-007`, `INV-008`, `P17-OUT-001`, and `P17-EVD-003`. |
| D3 | Do not push the soon-to-be-superseded remediation head merely to manufacture CI evidence. Defer the exact-head three-platform query to the user-materialized remediation commit and the required fresh audit. | The opi-remediate application protocol stops at the normal commit/materialization gate and requires a fresh audit at the new committed endpoint. |
| D4 | Resolve the shell-completion subprocess only through Cargo's `CARGO_BIN_EXE_opi` integration-test contract. | The external target is `E:/opi/cargo-targets/...`; other product subprocess suites already use the Cargo-provided binary. |
| D5 | At Bedrock HTTP EOF, treat any non-empty frame buffer as one typed `StreamError` and return before flushing pending Done. | `P17-FAL-003` and `INV-006` forbid converting partial transport state into silent success. |
| D6 | Retain `FileEvidenceSink::dir` in this remediation round. Zero internal callers alone do not prove incorrect behavior or authorize public API removal. | Minimum-change and ask-before-removing-intentional-behavior rules; prior immutable D6 also retained the accessor. |

## Closure batches

### Batch B1: Exact material-input binding per mutable session branch

**Closure predicate**: Before evidence setup or provider/tool side effects, the active mutable session is a v2 file whose header binding exactly equals the canonical Direct Runtime Input derived from the current externally assembled material inputs; a between-run material change creates one parented bound child, while an in-run next-turn route transition does not mutate that run's binding.
**Dependencies**: none
**Verification union**: exact provider-switch regression, complete product-evidence and session-runtime binaries, affected-target clippy, workspace all-targets and documentation gates

#### Fix B1.1: Share exact run-input derivation with session ownership

- **Finding source**: `docs/snapshots/phase17/audit.codex.5289892.20260823t055516z.md` — P17-AUD-002
- **Lineage**: recurrent-adjacent-path; prior P17-CODEX-001 closed the missing-binding family, but not exact material binding across later runs
- **Decision**: D2
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/session.rs`; `crates/opi-coding-agent/src/evidence.rs`; `crates/opi-coding-agent/src/harness.rs`; `crates/opi-coding-agent/src/session_cli.rs`; `crates/opi-coding-agent/src/session_coordinator.rs`; affected construction/tests including `crates/opi-coding-agent/tests/phase17_product_evidence.rs` and `crates/opi-coding-agent/tests/session_runtime.rs`
- **Change kind**: behavioral
- **Change**: Remove the fabricated header fallback by requiring callers to supply an exact binding. Extract one product-owned pure derivation of the canonical binding/config/input facts currently computed only for evidence, thread the existing direct assembly source and effective policy even when capture is disabled, and use that result for both evidence and session creation. Before each new run, compare it with the active branch binding; on mismatch, byte-preservingly copy the active chain into one parented v2 child with the new binding and adopt it before evidence setup. Do not branch for provider/model transitions that occur inside the already armed run.
- **Closure predicate**: Two prompts separated by a user/product model change finalize under different exact bindings and different parent-linked v2 session IDs, each manifest matching its header; the existing in-run cross-provider transition remains one run with route evidence and no binding mutation.
- **Red-before**: `cargo test -p opi-coding-agent --test phase17_product_evidence phase17_harness_switches_providers_with_matching_route_evidence -- --exact` — observed FAIL after the temporary discriminating assertion: alpha and beta manifests retained the identical `opi.session.header` digest, so no new bound child existed.
- **Green-after**: the same exact command — expected PASS with different bindings/session IDs for the between-run change, `beta.parent_session == alpha.id`, and the beta header binding equal to the beta manifest binding.

### Batch B2: Genuine v1 transition into a bound v2 child

**Closure predicate**: A genuine un-enveloped schema-v1 session with a uniquely provable route can be resumed or forked through the Reference Product into a parented, exactly bound v2 child without modifying the v1 source; ambiguous, missing, corrupt, or unsupported inputs fail closed before dispatch and do not create a guessed mutable child.
**Dependencies**: B1
**Verification union**: exact genuine-v1 regression, complete Phase 17 legacy-migration and session-CLI binaries, affected-target clippy, workspace all-targets and documentation gates

#### Fix B2.1: Defer mutable resume/fork until trusted binding assembly

- **Finding source**: `docs/snapshots/phase17/audit.codex.5289892.20260823t055516z.md` — P17-AUD-001; `docs/snapshots/phase17/audit.glm-5-3.5289892.20260823t052233z.md` — phase17-5289892-03
- **Lineage**: recurrent-adjacent-path; prior P17-CODEX-001/F-029 added the v2 envelope and binding but did not preserve genuine-v1 product migration
- **Decision**: D1
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/session.rs`; `crates/opi-coding-agent/src/session_cli.rs`; `crates/opi-coding-agent/src/harness.rs`; `crates/opi-coding-agent/src/main.rs`; `crates/opi-coding-agent/tests/phase17_legacy_migration.rs`; `crates/opi-coding-agent/tests/session_cli.rs`
- **Change kind**: behavioral
- **Change**: Keep the v1 reader read-only, but allow the loaded product value to represent a missing legacy binding. Separate immutable load from mutable fork/adoption so startup and in-harness resume/fork wait until the trusted product has normalized the recorded route and obtained B1's exact binding. Copy only the reconstructed active chain to a new v2 child whose `parent_session` names the v1 source; never append to or rewrite v1. Replace the misleading v2 `write_legacy_session` helper with genuine v1 header/unwrapped-entry bytes and assert the selected success and failure paths, source-byte identity, v2 parent/binding, zero dispatch on remediation, and opaque-trace coexistence.
- **Closure predicate**: Genuine v1 fixtures exercise unique/canonical resume and fork into v2, ambiguous/missing remediation with zero dispatch, byte-identical source preservation, and P17-A13 evidence/trace coexistence.
- **Red-before**: `cargo test -p opi-coding-agent --test phase17_legacy_migration phase17_legacy_session_fixture_byte_identical_after_resume_normalize_fork -- --exact` — observed FAIL, 0/1, after the helper temporarily wrote a genuine v1 file: `resume succeeds` received `legacy session is read-only because it has no runtime-input binding`.
- **Green-after**: the same exact command — expected PASS with the source v1 bytes unchanged and the adopted child at v2 with an exact binding and `parent_session` link.

### Batch B3: Unreleased session-format disclosure

**Closure predicate**: The complete Unreleased section records the v2 durable-format change, v1 byte-preserving resume/fork transition, v2-only writer, fail-closed cases, and affected constructor/session behavior without claiming that supported v1 input is merely refused.
**Dependencies**: B1, B2
**Verification union**: documentation contract and whitespace check

#### Fix B3.1: Record the selected user-visible transition

- **Finding source**: `docs/snapshots/phase17/audit.glm-5-3.5289892.20260823t052233z.md` — phase17-5289892-01
- **Lineage**: recurrent-adjacent-path; prior F-001 concerned another inaccurate Unreleased claim
- **Decision**: D1
- **Verification status**: Confirmed
- **File(s)**: `CHANGELOG.md`
- **Change kind**: documentation
- **Change**: Add one non-duplicated Unreleased entry under the existing appropriate subsection describing the exact D1/D2 session-v2 and v1-transition contract, including the explicit-binding constructor change if B1 removes the fabricated fallback.
- **Closure predicate**: A release reader can determine what v2 writes, how v1 resume/fork proceeds, what stays byte-identical, and which cases refuse execution.
- **Red-before**: not applicable to runtime behavior; the entire current Unreleased section was inspected and contains no session-v2 durable-format or v1 transition disclosure.
- **Green-after**: `python scripts/opi-doc-check.py` — expected PASS after exact Unreleased wording and section placement are reviewed.

### Batch B4: Bilingual session-contract synchronization

**Closure predicate**: Both `opi-agent` README counterparts describe v2 headers, required/ignorable envelopes, fail-closed unknown-required entries, read-only v1 source bytes, and the Reference Product's bound-v2 transition with equivalent meaning.
**Dependencies**: B1, B2
**Verification union**: documentation contract and whitespace check

#### Fix B4.1: Replace the superseded v1 README contract

- **Finding source**: `docs/snapshots/phase17/audit.glm-5-3.5289892.20260823t052233z.md` — phase17-5289892-02
- **Lineage**: recurrent-adjacent-path; prior documentation findings occurred in the same current-contract family
- **Decision**: D1
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/README.md`; `crates/opi-agent/README.zh.md`
- **Change kind**: documentation
- **Change**: Synchronize the English and Chinese overview/session sections to the exact current v2 envelope and selected v1 transition. Remove skip-all-unknown and v1-writer claims; distinguish the core read-only legacy reader from the Reference Product's new bound child.
- **Closure predicate**: Neither counterpart states `version = 1` as the current writer or permits an unknown required v2 entry to be skipped, and both express the same compatibility/failure contract.
- **Red-before**: not applicable to runtime behavior; both current counterparts were inspected and state the superseded v1 additive/skip-unknown contract.
- **Green-after**: `python scripts/opi-doc-check.py` — expected PASS with bilingual synchronization.

### Batch B5: Cargo-provided shell-completion executable

**Closure predicate**: The shell-completion integration binary executes the exact `opi` binary Cargo built, and all eight tests pass under the repository's external Cargo target.
**Dependencies**: none
**Verification union**: focused shell-completion test, affected-target clippy, workspace all-targets test

#### Fix B5.1: Replace checkout-local binary discovery

- **Finding source**: `docs/snapshots/phase17/audit.codex.5289892.20260823t055516z.md` — P17-AUD-004
- **Lineage**: recurrent-adjacent-path; prior P17-CODEX-004 closed the same family in `session_cli`
- **Decision**: D4
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/tests/shell_completions.rs`
- **Change kind**: test-only
- **Change**: Return `env!("CARGO_BIN_EXE_opi")` from `opi_bin` and remove release/debug checkout-path probing.
- **Closure predicate**: Every completion case launches Cargo's current integration-test executable and the focused binary passes 8/8 with the external target directory.
- **Red-before**: `cargo test -p opi-coding-agent --test shell_completions` — observed FAIL at the same committed head in r1, 0 passed and 8 failed because the test tried the absent detached-checkout `target/debug/opi.exe` while Cargo built under the external target.
- **Green-after**: the same command — expected PASS, 8/8 under the external target.

### Batch B6: Bedrock truncated-EOF fail-closed handling

**Closure predicate**: A valid Bedrock terminal event followed by an incomplete binary frame produces a typed stream error and never emits terminal Done.
**Dependencies**: none
**Verification union**: exact regression test, complete Bedrock fixture binary, affected-target clippy, workspace all-targets test, and changelog/documentation checks

#### Fix B6.1: Reject decoder residue before terminal flush

- **Finding source**: `docs/snapshots/phase17/audit.codex.5289892.20260823t055516z.md` — P17-AUD-005
- **Lineage**: recurrent-adjacent-path; prior P17-CODEX-002 closed CRC-invalid complete frames in the same stream-integrity family
- **Decision**: D5
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/bedrock/mod.rs`; `crates/opi-ai/tests/bedrock_fixtures.rs`; `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Add a production-path HTTP fixture containing a valid `messageStop` plus a truncated trailer. At transport EOF, emit one sanitized `ProviderError::StreamError` and return before `flush_pending` when the frame buffer is non-empty. Record the user-visible fail-closed correction under Unreleased without adding a duplicate subsection.
- **Closure predicate**: The regression stream contains one typed StreamError, contains no Done, and existing valid/CRC-invalid Bedrock fixtures still pass.
- **Red-before**: `cargo test -p opi-ai --test bedrock_fixtures terminal_stream_with_truncated_trailer_fails_closed -- --exact` — observed FAIL at the same committed head in r1: the temporary production-path regression received `Ok(Done)` and no StreamError.
- **Green-after**: the same exact command — expected PASS.

## Final verification

Known deduplicated verification union:

    cargo test -p opi-coding-agent --test phase17_product_evidence phase17_harness_switches_providers_with_matching_route_evidence -- --exact
    cargo test -p opi-coding-agent --test phase17_product_evidence
    cargo test -p opi-coding-agent --test session_runtime
    cargo test -p opi-coding-agent --test phase17_legacy_migration phase17_legacy_session_fixture_byte_identical_after_resume_normalize_fork -- --exact
    cargo test -p opi-coding-agent --test phase17_legacy_migration
    cargo test -p opi-coding-agent --test session_cli
    cargo test -p opi-coding-agent --test shell_completions
    cargo test -p opi-ai --test bedrock_fixtures terminal_stream_with_truncated_trailer_fails_closed -- --exact
    cargo test -p opi-ai --test bedrock_fixtures
    python scripts/opi-doc-check.py
    git diff --check
    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc
    $env:RUSTDOCFLAGS='-D warnings'; try { cargo doc --workspace --no-deps } finally { Remove-Item Env:RUSTDOCFLAGS -ErrorAction SilentlyContinue }

## Exclusions and shaping returns

| Source finding | Disposition | Evidence/authority |
|---|---|---|
| Codex P17-AUD-003; GLM phase17-5289892-05 | Deferred by registered source through D3 | Exact post-remediation CI belongs to the user-materialized commit and fresh audit, not this worktree-only planning endpoint. |
| GLM phase17-5289892-04 | Info/No action through D6 | Zero internal callers are confirmed; no incorrect behavior or authorized removal is established. |

## Appendix A: committed endpoint inventory

The following 81 status/path records are the committed `origin/main`-to-remediation-head delta. They are endpoint evidence, not remediation scope:

    M .agents/skills/README.md
    M .agents/skills/README.zh.md
    M .agents/skills/_shared/references/finding-contract.md
    A .agents/skills/_shared/references/remediation-disposition-contract.md
    A .agents/skills/_shared/references/shared-decision-and-test-stewardship.md
    A .agents/skills/_shared/scripts/compare_finding_lineage.py
    A .agents/skills/_shared/scripts/validate_assurance_artifact.py
    M .agents/skills/opi-audit/SKILL.md
    M .agents/skills/opi-audit/agents/openai.yaml
    A .agents/skills/opi-audit/evals/evals.json
    A .agents/skills/opi-audit/references/audit-proof-obligations.md
    M .agents/skills/opi-audit/references/finding-template.md
    M .agents/skills/opi-implement/SKILL.md
    A .agents/skills/opi-implement/evals/evals.json
    M .agents/skills/opi-implement/references/anti-patterns.md
    M .agents/skills/opi-implement/references/initializer.md
    M .agents/skills/opi-implement/references/ledger-schema.md
    M .agents/skills/opi-implement/references/verification-tiers.md
    M .agents/skills/opi-implement/references/verify-engine.md
    M .agents/skills/opi-implement/scripts/exec.workflow.js
    M .agents/skills/opi-implement/scripts/phase-exit.workflow.js
    M .agents/skills/opi-implement/scripts/plan.workflow.js
    M .agents/skills/opi-implement/scripts/plan.workflow.tests.js
    A .agents/skills/opi-implement/scripts/skill-contract.tests.py
    M .agents/skills/opi-implement/scripts/validate-plan.py
    M .agents/skills/opi-implement/scripts/validate-plan.tests.py
    A .agents/skills/opi-implement/scripts/verify-workflows.tests.js
    M .agents/skills/opi-remediate/SKILL.md
    M .agents/skills/opi-remediate/agents/openai.yaml
    A .agents/skills/opi-remediate/evals/evals.json
    M .agents/skills/opi-remediate/references/cross-reference-matrix.md
    M .agents/skills/opi-remediate/references/execution-protocol.md
    M .agents/skills/opi-remediate/references/remediation-plan-template.md
    M .agents/skills/opi-slim-tests/SKILL.md
    M CHANGELOG.md
    M Cargo.lock
    M Cargo.toml
    M crates/opi-agent/Cargo.toml
    M crates/opi-agent/src/agent_loop.rs
    M crates/opi-agent/src/evidence.rs
    M crates/opi-agent/src/hooks.rs
    M crates/opi-agent/src/session.rs
    M crates/opi-agent/tests/evidence_contract.rs
    M crates/opi-agent/tests/evidence_runtime.rs
    M crates/opi-agent/tests/image_input_session.rs
    M crates/opi-agent/tests/session_context.rs
    M crates/opi-agent/tests/session_contract.rs
    M crates/opi-agent/tests/session_facade.rs
    M crates/opi-agent/tests/session_storage.rs
    M crates/opi-agent/tests/tool_authority.rs
    M crates/opi-agent/tests/tool_validation.rs
    M crates/opi-ai/src/bedrock/event_stream.rs
    M crates/opi-ai/src/bedrock/mod.rs
    M crates/opi-ai/src/provider.rs
    M crates/opi-ai/tests/bedrock_fixtures.rs
    M crates/opi-coding-agent/Cargo.toml
    M crates/opi-coding-agent/src/evidence.rs
    M crates/opi-coding-agent/src/harness.rs
    M crates/opi-coding-agent/src/runner.rs
    M crates/opi-coding-agent/src/session_cli.rs
    M crates/opi-coding-agent/src/session_coordinator.rs
    M crates/opi-coding-agent/tests/phase17_product_evidence.rs
    M crates/opi-coding-agent/tests/phase17_tool_authority.rs
    M crates/opi-coding-agent/tests/rpc_jsonl.rs
    M crates/opi-coding-agent/tests/session_cli.rs
    M crates/opi-coding-agent/tests/session_runtime.rs
    A docs/snapshots/phase17/audit.codex.96f7d16.20260822t180722z.findings.jsonl
    A docs/snapshots/phase17/audit.codex.96f7d16.20260822t180722z.md
    M docs/snapshots/phase17/audit.codex.md
    M docs/snapshots/phase17/audit.glm5.3.md
    A docs/snapshots/phase17/audit.glm53.96f7d16.20260822t182354z.findings.jsonl
    A docs/snapshots/phase17/audit.glm53.96f7d16.20260822t182354z.md
    D docs/snapshots/phase17/citation-addendum-2026-08-21.md
    M docs/snapshots/phase17/remediation-plan.md
    A docs/snapshots/phase17/remediation.96f7d16.r1.plan.dispositions.jsonl
    A docs/snapshots/phase17/remediation.96f7d16.r1.plan.md
    A docs/snapshots/phase17/remediation.96f7d16.r1.result.dispositions.jsonl
    A docs/snapshots/phase17/remediation.96f7d16.r1.result.md
    M scripts/opi-doc-check.py
    A scripts/test_opi_assurance_skills.py
    M scripts/test_opi_doc_check.py
