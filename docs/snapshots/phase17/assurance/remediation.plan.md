# Phase 17 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `bc8041d8e8aa26d9067b02f006263ecf922c23d8263e13e1ac60b8b434194ed1`
**Remediation head**: `68507a86b5e99a226bb65b219f274f4f729fd88c`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged=none; unstaged=[`.gitignore`] (carried-in, not touched); untracked=none before fixed plan output
**Unresolved decisions**: none

## Current Finding Verification

| Source run / Finding ID | Verification | Source and final severity + rationale | Closure key/family | Batch | Decision |
|---|---|---|---|---|---|
| `phase17-3a15ed4-20260824t024210z` / `P17-AUD-001` | Confirmed at remediation head. `execute_prepared_tool` passes the after-hook `final_result` to `ExecutedTool::ordinary`, which derives terminal evidence from the replacement `is_error`. A sealed-export discriminating test observed `[Failed, Failed]` where both lower-boundary executions succeeded. | Major -> Major. This rewrites the evidence fact for actual execution and partially violates P17-OUT-004. | `tool.execution-outcome.truthfulness` / `tool.evidence.truthfulness` | B1 | `fix:preserve-lower-boundary-tool-outcome` |
| `phase17-3a15ed4-20260824t024210z` / `P17-AUD-002` | Refuted as a current remediation defect. Independent GitHub query confirms run `32733627895` completed successfully at descendant head `87377fcf750a5d0a38919bf82e740b7baefe8a8b`, including all three literal Phase 17 acceptance jobs. The `crates` tree, CI workflow blob, and `Cargo.lock` blob at that head are byte-identical to the remediation head; the only later committed changes are assurance artifacts. The historical absence of checks at `3a15ed4fe3118536aca7457353e65782042465e5` remains true, but it no longer supports the claim that platform identity and passage cannot be established for the current committed implementation. | Major -> Major retained as source severity; the current defect is refuted by independently queried three-platform evidence and committed-object identity, not by reviewer vote. | `phase17.ci-three-platform.current-implementation` / `phase17.platform-acceptance` | none | `no-action:refuted-by-current-three-platform-evidence` |
| `phase17-claude-glm53-87377fc-20260824t135741z` / `P17-AUD-003` | Confirmed at remediation head. `RpcRunner::new_with_trace` and its `trace_sink` parameter remain, while the value is an `EvidenceRecorder`. | Info -> Info. P17-MIG-003 remains met, the RPC `trace` command is still supported, and the finding identifies an advisory naming residual rather than incorrect behavior. | `rpc.evidence-recorder.constructor-naming` / `rpc.evidence-compatibility` | none | `no-action:retain-rpc-trace-compatible-name` |

## Unresolved Decisions

none

## Closure Batches

### Batch B1: Preserve the lower-boundary tool execution outcome

**Closure predicate**: When `Tool::execute` returns an ordinary success or failure result and `after_tool_call` replaces only its presentation result, terminal tool evidence retains the outcome derived from the actual `Tool::execute` result while the replacement remains the user/model-visible result.
**Dependencies**: none
**Verification union**: focused `phase17_tool_authority` regression; documentation contract; format, clippy, workspace tests, doctests, rustdoc; `git diff --check`

#### Fix B1.1: Separate execution outcome from presentation replacement

- **Finding source(s)**: `phase17-3a15ed4-20260824t024210z` + `f0a75558fc5c5ad3f45d2f9a015d9d93ab0ffddd697ae8ca6e6d0b2735e7dea4` + `P17-AUD-001`
- **Decision**: `fix:preserve-lower-boundary-tool-outcome`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/agent_loop.rs`, `crates/opi-coding-agent/tests/phase17_tool_authority.rs`
- **Change kind**: behavioral
- **Change**: Derive `ToolExecutionOutcome` from the actual `Tool::execute` `ToolResult` before invoking `after_tool_call`; retain the hook replacement only as the presentation result used for diagnostics, events, messages, and context. Extend the existing after-call replacement acceptance test with an inverted `is_error` replacement and in-memory evidence assertion.
- **Closure predicate**: A successful lower-boundary result followed by an error-marked presentation replacement emits `ToolExecutionOutcome::Succeeded`; the replacement remains an error-marked tool result, and later authorization remains unchanged.
- **Red-before**: `cargo test -p opi-coding-agent --test phase17_tool_authority phase17_after_call_replace_keeps_later_authorization_unchanged -- --exact --nocapture` in unique `git archive` export -> observed `FAIL`: evidence outcomes were `[Failed, Failed]`, expected `[Succeeded, Succeeded]`.
- **Green-after**: Run the same focused command after the production change -> expected `PASS` with lower-boundary outcomes preserved and the existing later-authorization assertions green.

## Final Verification

    cargo test -p opi-coding-agent --test phase17_tool_authority phase17_after_call_replace_keeps_later_authorization_unchanged -- --exact
    python scripts/opi-doc-check.py
    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc
    $env:RUSTDOCFLAGS = "-D warnings"; cargo doc --workspace --no-deps
    git diff --quiet 87377fcf750a5d0a38919bf82e740b7baefe8a8b..68507a86b5e99a226bb65b219f274f4f729fd88c -- .github crates Cargo.toml Cargo.lock scripts
    gh run view 32733627895 --repo OdradekAI/opi --json headSha,status,conclusion,jobs --jq '[.headSha,.status,.conclusion,([.jobs[] | select(.name | startswith("Phase 17 acceptance")) | select(.status=="completed" and .conclusion=="success")] | length)] | @tsv'
    git diff --check

## Exclusions

| Finding ID | Disposition | Current evidence/authority |
|---|---|---|
| `P17-AUD-002` | Refuted | GitHub run `32733627895` independently reports `headSha=87377fcf750a5d0a38919bf82e740b7baefe8a8b`, `completed`, `success`, with successful `Phase 17 acceptance` jobs on `ubuntu-latest`, `macos-latest`, and `windows-latest` (and 24/24 total jobs successful). Git object identity independently proves the remediation head has the same `crates` tree (`7bd043b0e62ed15571da2c7307e8a7d5211e0d02`), CI workflow blob (`b4dae51ce69325dd83eb5460e152bd695e9dbd21`), and `Cargo.lock` blob (`7f185dcefb568d8df59351cec6f515d3301ecb5c`) as the tested head; `git diff 87377fc..68507a8` contains assurance artifacts only. The admitted source remains exactly run `phase17-3a15ed4-20260824t024210z` and findings digest `f0a75558fc5c5ad3f45d2f9a015d9d93ab0ffddd697ae8ca6e6d0b2735e7dea4`; title similarity cannot substitute for identity, and no older source was consulted. |
| `P17-AUD-003` | Info/No action | The trace-named Reference Product constructor remains a real compatibility entry point for the supported RPC `trace` command and accepts the evidence-recorder contract. P17-MIG-003 is met; no normative source requires a rename, so a public API/test churn change would exceed this advisory finding. Admission is bound only to run `phase17-claude-glm53-87377fc-20260824t135741z` and findings digest `5d2b87eff189de3513b4f5109fc074fa37137262f9842d427ede3495906edcd8`; title similarity cannot substitute for identity, and no older source was consulted. |
