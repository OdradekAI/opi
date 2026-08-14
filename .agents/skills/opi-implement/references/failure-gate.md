# Failure Decision Gate Reference

When `iteration_count` reaches `max_iterations` (default 5), STOP and hand
the decision to the user through the harness's available user-input mechanism.
No self-deliberation past this.

## Gate Payload

Print this information:

```text
Task: <id> <title>
DoD: <definition_of_done>
Tier: <tier>
Acceptance scenarios: <ids + status + verification command>
Production call sites: <tasks[].production_call_sites>
Iterations: <iteration_count> / <max_iterations>
Last gate output (truncated to 50 lines): <…>
Tests added but failing: <list>
Files modified: <list>
Smallest failing assertion: <quote from test output>
Start commit: <tasks[].start_commit>
Baseline dirty files at Phase B: <tasks[].baseline_dirty_files>
Dirty status: <git status --short>
Task-owned dirty files: <files matched by tasks[].task_owned_paths and changed since start_commit>
Reproduction commands: <exact commands>
```

## Options

| Option | Effect |
|---|---|
| (a) Retry with extended cap | +5 attempts (total 10). Status stays `in_progress`. |
| (b) Return to shaping | Apply the `skill.md` Source-return rule: route missing facts to `opi-research` or `opi-realign`; for unresolved product meaning, recommend the exact explicit user invocation of Matt `wayfinder` or `grill-with-docs`. After the reviewed source changes, the user re-runs `plan`. |
| (c) Mark blocked | Record blocker text. Leave failing tests. Stage nothing. Status → `blocked`. Skipped on auto until `--clear-blocker`. |
| (d) Drop to manual | Print reproduction commands, touched files, suggested cleanup. Do NOT run cleanup. User finishes manually, then `--resume-from-manual`. |

**No "auto-revert" option.** MUST NOT run `git restore`, `git clean`,
`git reset`, or equivalent. If cleanup is needed, print candidate commands
scoped only to task-owned files changed since `start_commit`. Never include
files that were already dirty in `baseline_dirty_files` unless the task also
modified them and the user explicitly confirms they are task-owned.

## Meta-Warning

If **three consecutive** task invocations hit the failure gate, print:

> "Harness components may be misaligned with the current spec or model.
> Re-read the registered source and return missing facts to research/realignment
> or recommend explicit wayfinding/grilling for unresolved decisions before
> continuing."
