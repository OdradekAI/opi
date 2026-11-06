# AGENTS.md

`opi` is a Rust AI Agent toolkit with a terminal-first product. Learn from
[pi](https://github.com/earendil-works/pi) and other projects; choose designs
against current evidence and the user's goal, not a permanently pinned upstream.

## Context and decisions

Load only the sources needed for this task; follow dependencies when evidence
requires it. Reuse context already read unless it changed.

| When | Read |
|---|---|
| Writing or restructuring code | [Engineering](docs/engineering/README.md), relevant implementation and tests |
| Starting a feature, resuming work, or resolving conflicting meaning | [Iteration alignment](docs/agents/context.md); the selected task/iteration only |
| Naming or changing a domain concept | [Glossary](docs/GLOSSARY.md); [domain rules](docs/agents/domain.md) |
| Changing a consequential architectural decision | [ADR index](docs/adr/README.md), then the relevant decision |
| Finding current usage, topology, versions or dependencies | Owning README, source, schemas, fixtures, generated help and Cargo manifests |
| Navigating documentation | [Documentation map](docs/README.md) |

The user's agreed task and acceptance define desired change; source and tests
establish current behavior, not permission to preserve a bug or break a contract.
If a relevant decision, document and code disagree, identify the conflict and
ask when the choice changes scope, compatibility, authority or irreversible
outcomes. Continue independent work. Record the resolution in its owning doc.

Upstream comparisons record the actual revision/path in that comparison or
iteration. Global guidance defines engineering practice, not a version target.

## Working contract

- Carry action requests through edits, verification and a concrete handoff.
  For multi-step work use `step -> verify: <check>`; routine local choices and
  isolated offline checks need no repeated approval.
- Inspect the relevant contracts and callers before editing. Prefer deep
  modules, small interfaces, explicit ownership, and local reasoning. Search
  for reuse; avoid speculative traits, crates, flags and compatibility layers.
- Improve structure within the task's causal scope. Explain a consequential
  move or new dependency; propose unrelated improvements separately. Remove
  only code made unused by this change; ask before removing intentional behavior.
- Assume a shared worktree. Preserve unrelated staged, unstaged and untracked
  work. Delegate independent work with bounded ownership and an evidence handoff;
  keep dependent edits sequential and one writer per worktree.
- Update affected contracts with the code, not in a later documentation phase.
  Keep current facts in their owning source, resolved terms in the glossary,
  trade-offs in ADRs, temporary work in the iteration. Prune superseded claims.
  Update existing English/Chinese counterparts together or explain why unnecessary.
- Comments/rustdoc describe current contracts, timing, ownership and failures.
  Put implementation notes in the iteration, changelog or Git.

## Skills

All `opi-*` skills require explicit invocation; inspecting one is not invoking
it. For routing or skill maintenance, read [the manual](.agents/skills/README.md)
([中文](.agents/skills/README.zh.md)). Edit canonical `.agents/skills/` files;
`.claude/skills` aliases them. Each explicitly selected workflow owns its
artifacts and gates; consult its `SKILL.md` before modifying workflow state.

For requested Matt flows, read [tracker configuration](docs/agents/issue-tracker.md)
and [domain configuration](docs/agents/domain.md). Use `wayfinder` for large
unresolved decision spaces, not every feature. No skill is required just to
clarify a question with the user.

## Verification

For ordinary non-skill work, use
[change scope and check selection](.agents/skills/_shared/references/change-scope-and-check-selection.md)
(`check-selection-only`) and [engineering checks](docs/engineering/README.md#verification).
Run the union required by touched surfaces, including every created/modified
test. Fix change-caused failures and rerun affected checks; broaden only for
new changes, failures or unresolved evidence gaps.

Docs/skills/metadata: `python scripts/opi-doc-check.py` and `git diff --check`.
Checker/script changes also need their focused tests and a negative path.
Before Cargo builds or cache maintenance, read the shared
[Cargo cache lifecycle](.agents/skills/_shared/references/cargo-cache-lifecycle.md).
Keep incremental compilation and external shared caches; no `cargo clean`,
per-session targets or cache deletion as workflow cleanup.

## Git and release safety

Create/publish branches, commits, pushes, PRs or releases only on explicit user
request. Default branch prefix: `codex/`. Before staging/committing, check status
and stage task-owned paths explicitly. Use Conventional Commits and applicable
`fixes #N`/`closes #N`; exclude credentials, `.env` and Co-Authored-By trailers.
Never use `git add -A`, `git add .`, bypass hooks or rewrite shared history.
Forbidden: `git reset --hard`, `git checkout .`, `git clean -fd`, `git stash`,
`git commit --no-verify`, `git push --force`. Abort and ask on rebase conflicts
in files you did not modify. Review PRs with `gh pr view`, `gh pr diff`, `gh api`
or `git show`; do not pull or change branches/worktrees without permission.

Read the complete Unreleased section before editing it; add user-visible changes
there, reuse subsections and leave released sections immutable. Release requires
explicit `opi-release <version>`; public rollback uses a revert and tag deletion.
Edit only `AGENTS.md`; `CLAUDE.md` aliases it.

## Handoff

Lead with results, exact checks/outcomes and remaining gaps. Record test impact
as `add`, `update`, `delete`, `retain` or `none`. For review feedback, state
agreement/disagreement with evidence. Keep prose concise and technical.
