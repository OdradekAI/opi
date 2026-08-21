# Change scope and check selection

Use this contract to turn the actual outgoing change into the narrowest
sufficient verification set. Its authority is `check-selection-only`: it
selects checks after another source has established what work is in scope.

## Pin the comparison

For an outgoing committed delivery, record both inputs:

- `verified_base=<explicit live base ref>` — confirm the integration target
  from current repository or review context; never guess it from the branch
  name;
- `head=<ref>` — the outgoing committed tip, defaulting to `HEAD`.

Resolve and record the common ancestor with
`git merge-base <verified-base> <head>`. Then collect the four states without
collapsing them:

- `git diff --name-status --find-renames <merge-base>..<head>`;
- `git diff --cached --name-status`;
- `git diff --name-status`;
- `git ls-files --others --exclude-standard`.

The first command identifies the outgoing committed change. The remaining
commands identify index, working-tree, and new-file state. In the handoff,
record `committed`, `staged`, `unstaged`, and `untracked` separately so that a
dirty worktree is not confused with the branch delta. Recompute the merge base
and change inventory after retargeting, merging the base, or changing `head`.

### Worktree-only verification

When the task verifies uncommitted workspace state and does not hand off an
outgoing committed branch delta, use worktree-only mode. At task start, record
the `HEAD` and the three inventories below. At completion, rerun them, confirm
that `HEAD` is unchanged, record `committed=none-for-this-task`, and collect:

- `git diff --cached --name-status`;
- `git diff --name-status`;
- `git ls-files --others --exclude-standard`.

Do not require `verified_base` or run `git merge-base` in this mode. Classify
paths already dirty at task start as `carried-in`; if the task deliberately
touches one, report it as `carried-in-and-touched` rather than task-only.
Select verification for every deliberately touched surface, including that
class. Older branch commits are out of scope for the task and must not be
described as a verified outgoing delivery. Switch to the full comparison above
before a handoff, review, merge, or release that includes committed work.

## Map changed surfaces to checks

Choose the union of checks required by every changed surface:

| Changed surface | Minimum check family |
|---|---|
| Documentation, skills, or metadata only | `python scripts/opi-doc-check.py` and `git diff --check` |
| One crate's local behavior | focused crate test and affected-target clippy |
| CLI, TUI, or model-visible behavior | integration, snapshot, or subprocess test at the affected seam |
| Protocol, schema, fixture, or durable format | fixture and conformance tests |
| Cross-crate runtime behavior | applicable workspace gates |
| Publication or packaging | artifact/package smoke plus the explicitly invoked `opi-release` workflow |

Escalate when one change crosses rows; do not downgrade a protocol or
model-visible change because most edited files are prose.
Do not rerun unchanged evidence against the same relevant state. Record the
prior command, result, and state identity, then run only evidence invalidated
by subsequent changes.

## Authority boundary

This reference does not bound audit coverage, does not define task ownership,
does not admit or revise product scope, does not select remediation finding
sources, and does not define release manifest contents. The owning workflow,
registered specification, task graph, finding set, or release contract retains
those authorities. This reference creates no state file, cache, ledger, or
parallel source of progress truth.
