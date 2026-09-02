# Phase 18 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `4a3b08e37620f26f5f2420dd35f027ec829c620d4cdd92206b1b96e561b3cab6`
**Remediation head**: `4f1f487e60349bd3baa8708ef606f6b860f01b0e`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged: none; unstaged: none; untracked: none
**Unresolved decisions**: none

## Current Finding Verification

| Source run / Finding ID | Verification | Source and final severity + rationale | Closure key/family | Batch | Decision |
|---|---|---|---|---|---|
| `phase18-codex-gpt56-0ea6188-20260901t111622z-048ef0c0` / `P18-AUD-001` | Partially confirmed | Source Minor; final Minor. Five listener tests reproducibly leave killed subprocesses and both captured pipes unreaped, while the sixth cited `kill()` is the readiness-failure helper rather than a sixth test. The resource-hygiene defect remains confirmed and advisory. | `phase18.scripted-provider.listener-cleanup` / `phase18.scripted-provider.test-hygiene` | B1 | `fix:reap-listener-test-processes` |

## Unresolved Decisions

none

## Closure Batches

### Batch B1: Reap scripted-provider listener test processes

**Closure predicate**: Every listener subprocess started by `scripts/test_phase18_scripted_provider.py` is reaped and its captured stdout/stderr pipes are closed on normal test cleanup and readiness failure, with no `ResourceWarning` emitted by the full test module.
**Dependencies**: none
**Verification union**: exact warning-enabled unittest module (including its listener negative paths), documentation contract check, and whitespace/error diff check.

#### Fix B1.1: Centralize listener subprocess cleanup

- **Finding source(s)**: `phase18-codex-gpt56-0ea6188-20260901t111622z-048ef0c0` + `d4782ba5b318e266cd16c063848dafdfd3e6c8aafd9383a260563418423e99f4` + `P18-AUD-001`
- **Decision**: `fix:reap-listener-test-processes`
- **Verification status**: Partially confirmed
- **File(s)**: `scripts/test_phase18_scripted_provider.py`
- **Change kind**: test-only
- **Change**: Add one listener-cleanup helper that kills a still-running child and always calls `communicate(timeout=10)` to wait for exit and close both captured pipes; use it in the readiness-failure path and all five listener-test `finally` blocks.
- **Closure predicate**: The full warning-enabled test module completes all 15 tests with no `subprocess ... is still running` or unclosed-file `ResourceWarning`.
- **Red-before**: `python -X tracemalloc=5 -W always::ResourceWarning -m unittest scripts/test_phase18_scripted_provider.py` -> FAIL (closure predicate) at `4f1f487e60349bd3baa8708ef606f6b860f01b0e`: 15 tests reported `OK`, but five listener subprocesses emitted `subprocess ... is still running`, each with unclosed stdout and stderr warnings; the same unreaped cleanup pattern is statically present in the readiness-failure path.
- **Green-after**: `python -X tracemalloc=5 -W always::ResourceWarning -m unittest scripts/test_phase18_scripted_provider.py` -> expected 15 tests `OK` with no `ResourceWarning` output.

## Final Verification

    python -X tracemalloc=5 -W always::ResourceWarning -m unittest scripts/test_phase18_scripted_provider.py
    python scripts/opi-doc-check.py
    git diff --check

## Exclusions

none
