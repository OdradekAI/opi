# Anti-Pattern Guards Reference

These are explicit rules. Each maps to a documented failure mode. The **Why**
column explains reasoning so you can apply judgment in edge cases.

| Rule | Why |
|---|---|
| Never delete or weaken tests to make them pass | A passing suite that doesn't catch regressions creates false confidence. Fix the implementation, not the test. |
| Never `git push --force` | Rewrites shared history. Others may have fetched old refs; causes silent data loss and broken bisects. |
| Never bypass clippy with crate-wide `#[allow]` | Suppresses future warnings too. Targeted `#[allow]` on specific item with comment is OK; blanket suppression hides real issues. |
| Never commit with broken smoke | Smoke is cheapest proof prior work holds. Broken baseline means next invocation can't distinguish old from new breakage. |
| Never commit unstaged secrets | Secrets in git history are effectively public. Rotation cost far exceeds checking cost. |
| Never bypass git hooks (`--no-verify`) | Hooks encode project invariants. Bypassing means commit may fail CI later. |
| Never `git reset --hard` + force push for rollback | Destroys history for all collaborators. Use `git revert` instead. |
| Never `--amend` on already-pushed commits | Rewrites public SHA. Anyone who fetched original now has diverged history. |
| Never self-grade verification | LLMs rationalize success. Mechanical gates (exit codes, grep) are deterministic and auditable. |
| Never auto-accept TUI snapshot changes | Snapshot diffs are visual regressions until proven otherwise. Only human can judge intent. |
| Never silently rewrite inferred task graph metadata | Graph is a reviewed contract. Silent changes reorder execution, skip gates, break confirmed assumptions. |
| Never amend a normative source from plan admission or task execution | Missing facts return to research/realignment; unresolved product meaning returns to human-led shaping. Editing the source inside the harness collapses author and reviewer roles. |
| Never let a plan reviewer mutate the draft it reviews | Adversarial review must report independently. Auto-folding its own findings removes the fixed artifact needed for a credible verdict. |
| Never run a second task, worktree, commit, or ticket state machine inside `opi-implement` | The canonical ledger, task commit, and ledger checkpoint already own delivery state. A nested generic workflow creates contradictory recovery evidence. |
| Never write a test at an unconfirmed seam | Tests at private or accidental seams couple the suite to implementation and can make a task look covered without proving its public behavior. |
| Never disguise a horizontal task graph as dependency sequencing | Infrastructure-by-layer tasks defer integration risk. Use demonstrable vertical slices, or explicitly justified expand-contract steps for wide refactors. |
| Never run live provider tests from this skill | Non-deterministic, costs money, hits rate limits. Belong in `#[ignore]`-gated tests run manually. |
| Never mix the canonical ledger into a task commit or commit transient ledger files | The task SHA is not known until the task commit exists, so the canonical ledger needs a separate checkpoint commit. Tmp, draft, candidate, backup, and corrupt files are nondurable artifacts and must remain ignored. |
| Never resolve a canonical-ledger conflict by choosing one side | Parallel branches carry independent task evidence. Reconcile both branches' `Opi-*` footers through the plan path or valid progress is silently lost. |
| Never remove a worktree with dirty or uncontained ledger state | A worktree-local ignored ledger caused the Phase 14 recovery incident. Cleanup must prove the canonical ledger is clean, no temp remains, and required checkpoints reached the destination branch. |
| Never skip `[workspace.dependencies]` for internal deps | Lockstep versioning requires workspace table. Bare path deps break `cargo publish`. |
| Never execute a stale ledger after `opi-spec.md` changed | The ledger is an implementation cache. If the spec hash changed, task title, DoD, dependencies, and phase scope may now mean something different. |
| Never silently default v1 fields when migrating to v2 | Defaults mask the case where a v1 task was inferred under old rules and would now be re-classified. Migration must re-evaluate each new field per v2 semantics and demote to `failing` when the old evidence does not match. |
| Never add unregistered design/plan docs, snapshot files, `CLAUDE.md`, `AGENTS.md`, or skill source to `spec_files` | Only reviewed supplemental source files listed in `skill.md` are normative for the active supplemental-phase ledger drift check. Arbitrary process docs and skill files create circular reinit failures. |
| Never execute a composite spec row as a single monolithic task | One commit, one DoD, one evaluator, and a 5-iteration cap cannot reliably cover N independent extension examples. Reinit MUST decompose composite rows into dotted sub-tasks; attempts to bypass decomposition fail loudly. |
| Never require unrelated user changes to become clean | This repository may be shared with users or other agents. The harness owns only the selected task's files and must not pressure cleanup of unrelated work. |
| Never reintroduce MCP, permission profiles, sub-agents, plan mode, or todos as Phase 3 core work | The current spec keeps these as extension/package examples or later surfaces; putting them back in core recreates the drift the harness is supposed to prevent. |
| Never satisfy DoD with placeholder stubs/TODOs | Stubs pass gates but don't deliver value. Poisons downstream tasks depending on real behavior. |
| Never close a product scenario with component-only tests | Parser, protocol, helper, bridge, and mock-registry tests prove substrate only. Product scenarios require a production CLI/startup/runtime/session/API path. |
| Never mark an unused runtime integration as passing | A function that is only called by tests is not integrated. Runtime/startup claims need production call sites and tests that exercise them. |
| Never archive a phase from ledger status alone | The ledger can encode weak DoDs. Phase exit must independently rebuild current source-spec criteria and trace them to code and tests. |
| Never leave vague DoD verbs unexpanded | Words like `works`, `supports`, `loads`, `integrates`, `bridges`, and `handles` hide missing observable behavior. Expand before task execution. |
| Never satisfy a phase by implementing its Non-Goals | Phase designs use Non-Goals to preserve product scope. npm, marketplace/gallery, telemetry, OAuth, sandboxing, pi-web-ui parity, pi session compatibility, background bash, vector memory, and workflow-heavy core features require separate reviewed designs. |
| Never treat handoff/backlog lists as current executable scope | Future Ecosystem and phase handoff sections are dependency hints, not task authorization. Converting them to tasks requires a reviewed source update and a `plan` re-run. |
| Never broaden into cross-task refactors without graph update | Scope creep invalidates adjacent task assumptions. Graph must reflect reality. |
| Never clean/restore/discard user changes from failure gate | Working tree may contain in-progress manual fixes. Automated cleanup destroys expensive context. |
| Never let sub-agent completion order decide result order | Non-deterministic ordering = unreproducible results. `parallelize` array defines canonical order. |
| Never run the verify engine after its gate has fired | Plan review runs only before P.4 confirmation (the graph is not yet a contract). Exec verify gates Phase D (must-fix routes to C); phase-exit verify gates F.1b archive. Re-running a stage to erase its own finding silently rewrites confirmed or shipped state. |

The skill refuses to act if any rule would be violated, even if the user
requests it during an interactive failure-decision gate.
