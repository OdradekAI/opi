# AGENTS.md

`opi` is a Rust AI agent toolkit with a terminal-first product. Learn from
[pi](https://github.com/earendil-works/pi) and other projects; choose designs
using current evidence and the user's goal. Record source revisions/paths in comparisons.

## Working principles

### Think Before Coding

The agreed task defines desired behavior; source and tests establish current
behavior. Inspect affected contracts, callers and tests in enough context to
support the change; broad conclusions need more than search excerpts. Separate
evidence from assumptions, surface materially different interpretations, and
challenge flawed premises with evidence and a recommended alternative. Resolve
facts independently. Ask only about unresolved choices that materially change
scope, compatibility, safety, authority or costly/irreversible outcomes;
continue independent work while a decision is pending.

### Simplicity First

Build the smallest complete solution that meets acceptance. Reuse existing
owners and patterns; prefer deep modules with small interfaces and explicit
ownership. Added features, abstractions, options and compatibility paths need
a demonstrated requirement or contract. Simplify indirection that adds no
explanatory or behavioral value.

### Surgical Changes

Make each change serve the requested outcome or its necessary verification.
Follow local style and improve structure within the task's causal scope.
Preserve intentional behavior unless its change is authorized; remove code made
unused by this change. Review the diff for unrelated edits and propose unrelated
cleanup separately.

### Goal-Driven Execution

Define observable acceptance before implementation; pair multi-step work with
checks. Reproduce reported bugs and verify preserved behavior for refactors.
Carry authorized work through edits, affected docs, verification, repair and
handoff. Make routine reversible decisions and reuse authorization already given.
Demonstrate acceptance with evidence; report any unmet criteria and concrete blockers.

## Design defaults

Prefer safe Rust, explicit state/resource ownership and typed failures. Make
invariants, cancellation and cleanup responsibilities clear at interfaces.
Use the engineering guidance below for Rust-specific trade-offs and standards.

## Communication

- Answer the user's questions before making implementation changes. For action
  requests, continue through delivery within the agreed scope.
- Explain consequential decisions through the problem, a concrete example or
  trace, and the proposed solution; explain why added complexity is necessary.
- State agreement or disagreement with evidence. Use concise technical prose
  and explain unfamiliar terms when needed.
- Report meaningful findings, changed assumptions and blockers during work.

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

Record resolved decisions in their owning docs.

## Shared work and documentation

- Assume a shared worktree. Preserve unrelated staged, unstaged and untracked
  work. Delegate independent investigation/review when it improves quality or
  saves time, with bounded ownership and evidence to verify before integration.
  Keep dependent edits sequential and one writer per worktree.
- Update affected contracts and owning docs in the same change, including existing
  English/Chinese counterparts, or explain why a mirror update is unnecessary.
  Comments/rustdoc describe current contracts, timing, ownership and failures;
  implementation history belongs in the iteration, changelog or Git.

## Skills

All `opi-*` skills require explicit invocation; inspection is not invocation.
For routing, requested Matt flows or maintenance, read [the manual](.agents/skills/README.md)
([中文](.agents/skills/README.zh.md)). Edit canonical `.agents/skills/` files;
`.claude/skills` aliases them. Read the selected `SKILL.md` for its artifacts and
gates before changing workflow state. Explicit user instructions take precedence
over skill guidance. If a rule blocks authorized work, cite its exact source and
requirement; distinguish an actual gate from an inferred need for approval.

## Verification

For ordinary non-skill work, use
[change scope and check selection](.agents/skills/_shared/references/change-scope-and-check-selection.md)
(`check-selection-only`) and [engineering checks](docs/engineering/README.md#verification).
Run the required check union, including every created/modified test. Fix
change-caused failures; repeat or broaden checks only for changed inputs,
failures or unresolved evidence gaps.

Docs/skills/metadata: `python scripts/opi-doc-check.py` and `git diff --check`.
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
For root guidance, edit `AGENTS.md`; `CLAUDE.md` aliases it.

## Handoff

Lead with results, exact checks/outcomes and remaining gaps. Record test impact
as `add`, `update`, `delete`, `retain` or `none`.
