# AGENTS.md

`opi` is a Rust AI Agent toolkit with a terminal-first Reference Product,
informed by [pi](https://github.com/earendil-works/pi), not a port or compatibility
layer.

## Authority and context

Load the source needed for the current decision; expand when evidence reveals
a dependency. Reuse context already read unless it changed.

| Decision | Source of truth |
|---|---|
| Scope, architecture, admission, priority | `docs/opi-spec.md` |
| Domain language and authority boundaries | `docs/CONTEXT.md` |
| Current product/protocol behavior | Source, schemas, fixtures, generated help, crate docs, READMEs |
| Topology, versions, edition, MSRV, dependencies | Cargo manifests |
| Release history and unreleased changes | `CHANGELOG.md` |
| Progress and delivery history | `.opi-impl-state.json`, `docs/snapshots/` |
| Non-normative evidence | `docs/realign/`, `.repo/pi-0.84.1`, `docs/research/` |

Consult the spec for scope or architecture decisions. If work contradicts a
normative clause, cite it and ask whether the user intends to revise the spec.
Evidence and arbitrary `docs/superpowers/specs/` files do not authorize delivery.

Update English/Chinese counterparts together or explain why unnecessary.
Edit only `AGENTS.md`; `CLAUDE.md` aliases it. The doc checker warns on a
symlinkless clone's text pointer and rejects real content drift.

## Working contract

- Answer questions directly. Carry action requests through authorized edits,
  verification and a concrete handoff. Resolve routine choices; ask when missing
  information materially changes scope, correctness, authority or irreversible
  outcomes, while continuing independent work.
- Define observable completion. For multi-step work, use
  `step -> verify: <check>`. Scoped local edits and isolated offline checks
  need no repeated approval. Fix change-caused failures and rerun affected
  checks until complete or a concrete blocker remains.
- Make the smallest coherent change; preserve local style and unrelated work.
  Remove only code made unused by this change. Ask before removing intentional
  behavior. Prefer simpler solutions over speculative abstraction, configuration,
  compatibility or adjacent refactoring.
- Inspect relevant contracts and implementation before editing; read full files
  when interactions require it, not before every edit. Batch independent reads.
  Delegate independent work with bounded scope, file ownership and an evidence
  handoff; keep dependent edits sequential.
- Comments/rustdoc describe current contracts, timing, ownership and failure
  behavior. Put task/Phase/review history in snapshots, ADRs, changelog or Git.

## Design and Rust

- Keep Agent Core small and deep, with mechanism below policy. Optional Opi
  workflows belong in the Extension Ecosystem; Agent-neutral capabilities begin
  as Independent Companions with Agent-neutral contracts.
- Dependencies point inward. A public seam needs intrinsic state-machine value
  or two real adapters/consumers with shared conformance. No speculative traits,
  crates, flags or config keys.
- Prefer safe Rust, explicit ownership, enums, typed errors, bounded concurrency
  and fail-closed boundary validation. Justify and isolate unavoidable `unsafe`.
  Use `thiserror` in libraries, `anyhow` in binaries/tests, trait objects at crate
  boundaries and generics internally when concrete types are known.
- Internal dependencies use root `[workspace.dependencies]` and
  `{ workspace = true }`; publishable paths also need versions. Keep lockstep
  versioning and workspace-owned metadata in root manifests.
- Review manifests, features, lockfile, `build.rs`, native/proc-macro dependencies
  as code. Verify APIs against the exact locked version. Generate `Cargo.lock`
  with Cargo and review unexpected transitive changes. Explain portability,
  authority and supply-chain impact before broadening default features, build
  scripts, native code or proc macros.
- Deliberately scoped 0.x breaking changes require an Unreleased entry and
  synchronized callers, schemas, fixtures, tests and docs. Add compatibility only
  when the owning contract requires it; revisit this policy before 1.0.

## Workspace layout

The doc checker validates this graph against Cargo manifests:

```text
opi-ai       (no internal deps)       - provider-neutral LLM API
opi-tui      (no internal deps)       - terminal UI components
opi-eval     (no internal deps)       - unpublished Independent Companion eval toolkit
opi-agent    -> opi-ai                - product-neutral Agent runtime
opi-protocol (no internal deps)       - command-execution protocol
opi-sandbox  -> opi-protocol          - standalone restriction SDK/CLI
opi-coding-agent -> opi-ai, opi-agent, opi-protocol, opi-tui - opi binary and coding harness
```

## Skills and workflow

For workflow selection/maintenance, use [the skill manual](.agents/skills/README.md)
([中文](.agents/skills/README.zh.md)). Edit canonical `.agents/skills/` files;
`.claude/skills` aliases them. All `opi-*` skills require explicit invocation;
use `opi-workflow` only when asked for routing. Load the selected `SKILL.md` and
references for its active mode. Inspecting a skill is not invoking it.

`opi-implement plan` owns implementation admission; user-confirmed eval repairs
use [the scoped exception](.agents/skills/opi-eval/SKILL.md#user-confirmed-source-repairs).
Only `opi-implement` writes `.opi-impl-state.json`. Preserve each workflow's
gates and artifact ownership; no competing ledger or progress in the spec.
Documentation/skill maintenance alone does not start product implementation.

## Verification

For ordinary non-skill work, use
[change scope and check selection](.agents/skills/_shared/references/change-scope-and-check-selection.md)
to inventory changes and select the verification union. Its authority is
`check-selection-only`; active workflows retain admission and tier gates.
After required checks pass, broaden/repeat only for new changes, failures or
unresolved evidence gaps.

- Docs/skills/metadata only: `python scripts/opi-doc-check.py` and
  `git diff --check`. Run the doc check after changing this guidance.
- Runtime: reproduce bugs, exercise invalid inputs for validation, use focused
  behavior tests and integration/snapshot/conformance checks at changed public
  seams. Run every created/modified test binary or filter. Avoid tests that
  restate implementation/prose; doc contracts belong in the doc checker.
- Cross-crate runtime, Phase exit, CI/release: applicable workspace gates are
  `cargo fmt --check --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace --all-targets`, `cargo test --workspace --doc`, and
  `cargo doc --workspace --no-deps` with `RUSTDOCFLAGS="-D warnings"`.
- Use `opi_ai::test_support::MockProvider`, local fixtures and isolated temp
  directories; serialize environment mutations. Tests must not require paid
  providers/live credentials or activate live calls because credentials exist.
- Keep incremental compilation and the external Cargo cache workflow. Inspect
  with `python scripts/opi-cargo-cache.py status`; no `cargo clean`, per-session
  targets or cache deletion as cleanup.

## Git and release safety

Assume a shared worktree; preserve unrelated staged, unstaged and untracked
changes. Create/publish branches, commits, pushes, PRs or releases only on an
explicit user request. Default branch prefix: `codex/`.

For authorized commits: check `git status` before staging and committing; stage
task-owned paths explicitly; use Conventional Commits and applicable
`fixes #N`/`closes #N`; exclude credentials, `.env` and Co-Authored-By trailers.
Never use `git add -A`, `git add .`, bypass hooks or rewrite shared history.
Forbidden: `git reset --hard`, `git checkout .`, `git clean -fd`, `git stash`,
`git commit --no-verify`, `git push --force`. If a rebase conflicts in a file
you did not modify, abort and ask.

Review PRs with `gh pr view`, `gh pr diff`, `gh api` or `git show`; do not pull
or change branches/worktrees unless explicitly requested.

Read the complete Unreleased section before editing it; add user-visible changes
there, reuse subsections and leave released sections immutable. Release requires
explicit `opi-release <version>`; public rollback uses a revert and tag deletion.

## Handoff

Lead with results, exact checks/outcomes and remaining gaps. Record test impact
as `add`, `update`, `delete`, `retain` or `none`. For review feedback, state
agreement/disagreement with governing evidence. Keep prose concise and technical;
no fluff or emojis in commits, issues, PR comments or code.
