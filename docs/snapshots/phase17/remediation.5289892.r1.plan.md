# Phase 17 Remediation Plan

**Status**: DRAFT-UNRESOLVED
**Verification target**: committed `528989279e9be308abc963ec22f377ee47bbde47`
**Round**: r1
**Finding sources**: docs/snapshots/phase17/audit.codex.5289892.20260823t055516z.md; docs/snapshots/phase17/audit.glm-5-3.5289892.20260823t052233z.md
**Disposition artifact**: `remediation.5289892.r1.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged none; unstaged none; untracked four carried-in immutable audit artifacts listed under Endpoint freeze
**Unresolved decisions**: D1, D2

## Endpoint freeze

- Remediation head: 528989279e9be308abc963ec22f377ee47bbde47.
- Verified base: origin/main.
- Merge base: 136c380f0c5eea541190cc1a0f5c1d62f983b4e8.
- Committed inventory: 81 paths from git diff --name-status --find-renames 136c380f0c5eea541190cc1a0f5c1d62f983b4e8..528989279e9be308abc963ec22f377ee47bbde47; exact paths are in Appendix A.
- Staged inventory: none.
- Unstaged inventory: none.
- Untracked carried-in inventory:
  - docs/snapshots/phase17/audit.codex.5289892.20260823t055516z.findings.jsonl
  - docs/snapshots/phase17/audit.codex.5289892.20260823t055516z.md
  - docs/snapshots/phase17/audit.glm-5-3.5289892.20260823t052233z.findings.jsonl
  - docs/snapshots/phase17/audit.glm-5-3.5289892.20260823t052233z.md
- Verification checkout: detached at the exact head; all plan-stage checks ran there. Only this plan and its disposition sibling are written to the original worktree.

## Lineage and verification summary

Both sources report fresh-context-same-family independence. Overlap is therefore correlated/degraded rather than an independent vote. The CI claim has full overlap; the schema-v1 behavior and fixture claims partially overlap; the remaining claims have one source.

| Source finding | Verification | Final severity + rationale | Lineage | Closure batch | Decision |
|---|---|---|---|---|---|
| Codex P17-AUD-001 | Partially confirmed | Major — product resume/fork reject genuine v1 and the registered fixture is v2, but whether v1 remains in the supported set is not decided by current sources | recurrent-adjacent-path | none | pending:D1 |
| Codex P17-AUD-002 | Confirmed | Major — a session-backed run replaces the freshly computed material-input digest with a cwd/initial-model binding that remains unchanged across provider switch | recurrent-adjacent-path | none | pending:D2 |
| Codex P17-AUD-003 | Confirmed | Major — the exact audited implementation head has no three-platform CI run | recurrent-adjacent-path | none | no-action:D3 |
| Codex P17-AUD-004 | Confirmed | Major — the focused test is 0/8 under the required external Cargo target and blocks the workspace all-targets gate | recurrent-adjacent-path | B1 | D4 |
| Codex P17-AUD-005 | Confirmed | Major — the production HTTP decoder emits Done when a valid terminal frame is followed by incomplete decoder residue | recurrent-adjacent-path | B2 | D5 |
| GLM phase17-5289892-01 | Confirmed | Minor — the complete Unreleased section omits the user-visible session-v2/read-only change; the defect is release documentation rather than runtime behavior | recurrent-adjacent-path | none | pending:D1 |
| GLM phase17-5289892-02 | Confirmed | Minor — both crate README counterparts still describe the v1 additive/skip-unknown contract | recurrent-adjacent-path | none | pending:D1 |
| GLM phase17-5289892-03 | Confirmed | Major — all seven advertised legacy migration tests pass over schema-v2 files and no product-seam test pins genuine v1 behavior | recurrent-adjacent-path | none | pending:D1 |
| GLM phase17-5289892-04 | Partially confirmed | Info — the accessor has no workspace caller, but no incorrect behavior is shown and the prior immutable plan deliberately preserved it | new | none | no-action:D6 |
| GLM phase17-5289892-05 | Confirmed | Major — same exact-head CI gap as Codex P17-AUD-003; local Windows evidence cannot establish the mandatory three-platform result | recurrent-adjacent-path | none | no-action:D3 |

## Unresolved decisions

| ID | Required decision | Why evidence cannot decide | Alternatives | Authority needed |
|---|---|---|---|---|
| D1 | Decide whether schema-v1 sessions remain supported for Reference Product resume/fork after the v2 binding cutover. | INV-007 requires an immutable binding and fail-closed unsupported-version handling, while the registered Phase 17 migration source preserves supported legacy resume/fork and prior releases wrote v1. No current authority explicitly adds or removes v1 from the supported resume/fork set. | A: preserve source bytes and migrate/resume into a newly bound v2 branch with genuine-v1 route tests. B: keep v1 read-only, revise the registered P17-MIG-001/P17-MIG-002/P17-A13 meaning through shaping, document the breaking change, and test deterministic refusal. | Human Authority, Phase shaper, and specification maintainers |
| D2 | Decide how one immutable session-branch binding relates to material inputs that change between runs, including provider/model switching. | CONTEXT and P17-EVD require a Direct Runtime Input digest over the material inputs of one run; INV-007 requires the validated prefix and its immutable binding to remain one durable pair; Phase 17 also requires same-session cross-provider switching. The current sources do not define the transition between those obligations. | A: create a new immutable bound session branch whenever material inputs change. B: revise the parent/Phase contracts to distinguish durable-prefix binding from per-run Direct Runtime Input and persist their relation. C: forbid material changes inside a bound branch, which would require revising cross-provider acceptance. | Human Authority, Agent/evidence owners, and specification maintainers |

A later remediation round must resolve D1 and D2, define concrete discriminating red-before checks for their affected findings, observe those checks failing at that later round's exact head, and then reseal a new plan. This round cannot become READY-FOR-APPLY.

## Recorded decisions

| ID | Decision | Governing evidence |
|---|---|---|
| D3 | Do not push the soon-to-be-superseded remediation head merely to manufacture CI evidence. Defer the exact-head three-platform query to the user-materialized remediation commit and the required fresh audit. | The opi-remediate application protocol stops at the normal commit/materialization gate and requires a fresh audit at the new committed endpoint. |
| D4 | Resolve the shell-completion subprocess only through Cargo's CARGO_BIN_EXE_opi integration-test contract. | The external target is E:/opi/cargo-targets/windows-direct; other product subprocess suites already use the Cargo-provided binary. |
| D5 | At Bedrock HTTP EOF, treat any non-empty frame buffer as one typed StreamError and return before flushing pending Done. | P17-FAL-003 and INV-006 forbid converting partial transport state into silent success. |
| D6 | Retain FileEvidenceSink::dir in this remediation round. Its zero internal callers are informational, the prior immutable D6 explicitly preserved it, and removal would discard intentional public behavior without user approval. | Minimum-change rule, ask-before-removing-intentional-behavior rule, and remediation.96f7d16.r1.plan.md D6. |

## Closure batches

### Batch B1: Cargo-provided shell-completion executable

**Closure predicate**: The shell-completion integration binary executes the exact opi binary Cargo built, and all eight tests pass under the repository's external Cargo target.
**Dependencies**: none
**Verification union**: focused shell-completion test, affected-target clippy, and workspace all-targets test

#### Fix B1.1: Replace checkout-local binary discovery

- **Finding source**: docs/snapshots/phase17/audit.codex.5289892.20260823t055516z.md — P17-AUD-004
- **Lineage**: recurrent-adjacent-path; prior P17-CODEX-004 closed the same family in session_cli
- **Decision**: D4
- **Verification status**: Confirmed
- **File(s)**: crates/opi-coding-agent/tests/shell_completions.rs
- **Change kind**: test-only
- **Change**: Return env!(CARGO_BIN_EXE_opi) from opi_bin and remove release/debug checkout-path probing.
- **Closure predicate**: Every completion case launches Cargo's current integration-test executable and the focused binary passes 8/8 with the external target directory.
- **Red-before**: cargo test -p opi-coding-agent --test shell_completions — observed FAIL, 0 passed and 8 failed because the test tried the absent detached-checkout target/debug/opi.exe while Cargo built under E:/opi/cargo-targets/windows-direct.
- **Green-after**: cargo test -p opi-coding-agent --test shell_completions — expected PASS, 8/8 under the external target.

### Batch B2: Bedrock truncated-EOF fail-closed handling

**Closure predicate**: A valid Bedrock terminal event followed by an incomplete binary frame produces a typed stream error and never emits terminal Done.
**Dependencies**: none
**Verification union**: exact regression test, complete Bedrock fixture binary, affected-target clippy, workspace all-targets test, and documentation/changelog checks

#### Fix B2.1: Reject decoder residue before terminal flush

- **Finding source**: docs/snapshots/phase17/audit.codex.5289892.20260823t055516z.md — P17-AUD-005
- **Lineage**: recurrent-adjacent-path; prior P17-CODEX-002 closed CRC-invalid complete frames in the same stream-integrity family
- **Decision**: D5
- **Verification status**: Confirmed
- **File(s)**: crates/opi-ai/src/bedrock/mod.rs; crates/opi-ai/tests/bedrock_fixtures.rs; CHANGELOG.md
- **Change kind**: behavioral
- **Change**: Add a production-path HTTP fixture containing a valid messageStop plus a truncated trailer. At transport EOF, emit one sanitized ProviderError::StreamError and return before flush_pending when the frame buffer is non-empty. Record the user-visible fail-closed correction under Unreleased.
- **Closure predicate**: The regression stream contains one typed StreamError, contains no Done, and existing valid/CRC-invalid Bedrock fixtures still pass.
- **Red-before**: cargo test -p opi-ai --test bedrock_fixtures terminal_stream_with_truncated_trailer_fails_closed -- --exact — observed FAIL: current results ended in Ok(Done) and contained no StreamError.
- **Green-after**: cargo test -p opi-ai --test bedrock_fixtures terminal_stream_with_truncated_trailer_fails_closed -- --exact — expected PASS.

## Final verification

Known verification union for B1 and B2:

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

The D1/D2 verification additions are intentionally not guessed. The next round must add their exact focused and conformance commands before it can be ready.

## Exclusions and shaping returns

| Source finding | Disposition | Evidence/authority |
|---|---|---|
| Codex P17-AUD-001 | Returned to shaping pending D1 | Current behavior is factual; the supported-v1 product contract is not. |
| Codex P17-AUD-002 | Returned to shaping pending D2 | A run-material binding conflicts with the current immutable session-pair interpretation. |
| GLM phase17-5289892-01 | Returned to shaping pending D1 | The exact breaking-change wording and remediation path depend on the supported-v1 decision. |
| GLM phase17-5289892-02 | Returned to shaping pending D1 | The correct bilingual session contract depends on the supported-v1 decision. |
| GLM phase17-5289892-03 | Returned to shaping pending D1 | A genuine-v1 test must assert the behavior selected by D1. |
| Codex P17-AUD-003; GLM phase17-5289892-05 | Deferred by registered source through D3 | Exact post-remediation CI belongs to the user-materialized commit and fresh audit, not this worktree-only planning endpoint. |
| GLM phase17-5289892-04 | Info/No action through D6 | Zero internal callers are confirmed; no incorrect behavior or authorized removal is established. |

## Appendix A: committed endpoint inventory

The following 81 status/path records are the committed origin/main-to-remediation-head delta. They are endpoint evidence, not remediation scope:

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
