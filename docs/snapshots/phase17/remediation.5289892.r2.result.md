# Phase 17 Remediation Result

**Status**: REMEDIATION-CHECKS-PASSED  
**Approved plan**: `docs/snapshots/phase17/remediation.5289892.r2.plan.md`  
**Starting endpoint**: committed `528989279e9be308abc963ec22f377ee47bbde47`  
**Resulting endpoint**: the same committed endpoint with the approved remediation changes uncommitted  
**Approval provenance**: User explicitly approved the exact plan in this task: `[$opi-remediate](D:\Luiz\Odradek\opi\\.agents\\skills\\opi-remediate\\SKILL.md) mode=apply plan=docs/snapshots/phase17/remediation.5289892.r2.plan.md`, received 2026-08-23 (Asia/Shanghai).  
**Dirty-worktree baseline**: staged none; unstaged none; six untracked audit/r1-plan artifacts carried unchanged.  
**Unresolved decisions**: none

## Closure batches

| Batch | Outcome | Evidence |
|---|---|---|
| B1 | Closed | Every mutable session branch now carries the exact product-derived Direct Runtime Input binding; an external between-run model change creates a parented child before execution. |
| B2 | Closed | Genuine v1 sessions remain byte-identical/read-only and are resumed or forked through a parented, exactly bound v2 child after trusted route normalization. |
| B3 | Closed | The Unreleased section now discloses the v2 writer, byte-preserving v1 transition, and fail-closed cases. |
| B4 | Closed | English and Chinese `opi-agent` README session contracts now agree on v2 envelopes and the Reference Product v1 transition. |
| B5 | Closed | Shell-completion tests invoke Cargo's integration-test `opi` binary. |
| B6 | Closed | Bedrock EOF with partial frame residue returns one typed stream error before terminal Done can be emitted. |

## Verification-discovered correction

The final all-targets gate exposed concurrent default session creation reusing a millisecond-only ID and therefore one JSONL path. `SessionCoordinator` now appends an in-process atomic sequence to each generated ID. The new concurrent 16-creation regression failed red (one distinct path) and passes green (16 distinct paths); the full RPC and workspace suites subsequently pass.

## Applied verification

- Plan union: exact provider-switch and genuine-v1 regressions, `phase17_product_evidence` (28), `session_runtime` (61), `phase17_legacy_migration` (7), `session_cli` (44), `shell_completions` (8), and Bedrock fixtures (47) — PASS.
- Additional recovery checks: `phase17_failure_rollback` (19), `harness_resource_integration` (24), `json_mode` (31), and `rpc_jsonl` (80) — PASS.
- Documentation and scope: `python scripts/opi-doc-check.py`, `git diff --check`, and `cargo fmt --check --all` — PASS.
- Workspace gates: `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --all-targets`, `cargo test --workspace --doc`, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` — PASS.
- Cargo commands used a temporary C-drive cache/temp override because the repository-configured E-drive cache was full; no repository Cargo configuration was changed.

## Test impact

| Batch | Impact |
|---|---|
| B1 | add, update |
| B2 | add, update |
| B3 | none |
| B4 | none |
| B5 | update |
| B6 | add, update |
| Verification-discovered session-ID correction | add, update |

## Remaining dispositions

Eight approved source records are Closed. `P17-AUD-003` and `phase17-5289892-05` remain Deferred by registered source (D3): exact three-platform CI evidence belongs to the user-materialized remediation commit and a fresh audit. `phase17-5289892-04` remains Info/No action (D6); the intentional public accessor was retained.

No commit was created. This completes local remediation verification, not a Phase PASS. After the user materializes a commit, run a fresh independent `$opi-audit` at that committed endpoint.
