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
| Never run live provider tests from this skill | Non-deterministic, costs money, hits rate limits. Belong in `#[ignore]`-gated tests run manually. |
| Never commit ledger/tmp/draft files | High-churn runtime artifacts. Pollutes history, creates merge conflicts. |
| Never skip `[workspace.dependencies]` for internal deps | Lockstep versioning requires workspace table. Bare path deps break `cargo publish`. |
| Never execute a stale ledger after `opi-spec.md` changed | The ledger is an implementation cache. If the spec hash changed, task title, DoD, dependencies, and phase scope may now mean something different. |
| Never silently default v1 fields when migrating to v2 | Defaults mask the case where a v1 task was inferred under old rules and would now be re-classified. Migration must re-evaluate each new field per v2 semantics and demote to `failing` when the old evidence does not match. |
| Never add unregistered design/plan docs, snapshot files, `CLAUDE.md`, `AGENTS.md`, or skill source to `spec_files` | Only reviewed supplemental source files listed in `skill.md` are normative for Phase 14-16 ledger drift checks. Arbitrary process docs and skill files create circular reinit failures. |
| Never execute a composite spec row as a single monolithic task | One commit, one DoD, one evaluator, and a 5-iteration cap cannot reliably cover N independent extension examples. Reinit MUST decompose composite rows into dotted sub-tasks; attempts to bypass decomposition fail loudly. |
| Never require unrelated user changes to become clean | This repository may be shared with users or other agents. The harness owns only the selected task's files and must not pressure cleanup of unrelated work. |
| Never reintroduce MCP, permission profiles, sub-agents, plan mode, or todos as Phase 3 core work | The current spec keeps these as extension/package examples or later surfaces; putting them back in core recreates the drift the harness is supposed to prevent. |
| Never satisfy DoD with placeholder stubs/TODOs | Stubs pass gates but don't deliver value. Poisons downstream tasks depending on real behavior. |
| Never close a product scenario with component-only tests | Parser, protocol, helper, bridge, and mock-registry tests prove substrate only. Product scenarios require a production CLI/startup/runtime/session/API path. |
| Never mark an unused runtime integration as passing | A function that is only called by tests is not integrated. Runtime/startup claims need production call sites and tests that exercise them. |
| Never archive a phase from ledger status alone | The ledger can encode weak DoDs. Phase exit must independently rebuild current source-spec criteria and trace them to code and tests. |
| Never leave vague DoD verbs unexpanded | Words like `works`, `supports`, `loads`, `integrates`, `bridges`, and `handles` hide missing observable behavior. Expand before task execution. |
| Never satisfy a phase by implementing its Non-Goals | Phase designs use Non-Goals to preserve product scope. npm, marketplace/gallery, telemetry, OAuth, sandboxing, pi-web-ui parity, pi session compatibility, background bash, vector memory, and workflow-heavy core features require separate reviewed designs. |
| Never treat handoff/backlog lists as current executable scope | Future Ecosystem and phase handoff sections are dependency hints, not task authorization. Converting them to tasks requires a reviewed source update and `--reinit`. |
| Never broaden into cross-task refactors without graph update | Scope creep invalidates adjacent task assumptions. Graph must reflect reality. |
| Never clean/restore/discard user changes from failure gate | Working tree may contain in-progress manual fixes. Automated cleanup destroys expensive context. |
| Never let sub-agent completion order decide result order | Non-deterministic ordering = unreproducible results. `parallelize` array defines canonical order. |

The skill refuses to act if any rule would be violated, even if the user
requests it during an interactive failure-decision gate.
