# Verification Tiers Reference

Each task carries a `tier` field; the skill selects one authoritative mechanical
gate from this table. D.3 adds only acceptance/platform checks missing from that
gate. Documentation-only tasks must be promoted when they touch runtime Rust,
Cargo manifests, or generated build artifacts.

## `workspace` Tier

Use for dependency graph changes, cross-crate integration harnesses, and tasks
whose primary crate is `workspace` or `cross-crate`.

Gate: `scripts/opi-impl-smoke.sh full` (or `.ps1 full` on Windows). It runs
format, all-target clippy, rustdoc, and the workspace test exactly once. There
is no preceding `cargo build --workspace` and no D.3 rerun.

## `documentation` Tier

Use for documentation/alignment tasks whose source spec explicitly says no
runtime behavior or code migration is allowed.

Gates:
1. `python scripts/opi-doc-check.py` exits 0.
2. `git diff --check` exits 0.
3. Task-owned paths are exact documentation paths, not broad `docs/**` globs.
4. English and localized counterparts are updated together when both exist.
5. Any source-spec or `acceptance_scenarios[].verification` command that proves
   more than prose presence exits with the expected result.
6. `git diff --name-only` shows no Rust source, Cargo manifest, lockfile,
   runtime script, fixture, snapshot, or generated build artifact changes. If
   it does, reclassify the task before implementation continues.

## `library` Tier

Use for focused `opi-ai`, `opi-agent`, or `opi-tui` library changes that do not
add provider wire formats, CLI runtime behavior, or visual snapshot surfaces.

Gates:
1. Record test impact as `add`, `update`, `delete`, `retain`, or `none`.
   Features and bug fixes normally require `add`/`update`; a behavior-preserving
   internal refactor may use `retain`; test-only cleanup may use `delete`;
   docs/skills/metadata may use `none`.
2. Run `scripts/opi-impl-smoke.sh scoped --crate <crate> [--test <name> ...]`
   (or the PowerShell sibling). Name every affected integration binary; with no
   `--test`, the gate runs lib tests only. The script owns format, production
   clippy, named-test clippy/test, and crate rustdoc.
3. No `unwrap`/`expect` in changed non-test code (focused grep/diff check).

Do not precede or follow the scoped gate with `cargo build --workspace`, bare
`cargo test -p <crate>`, or another clippy/doc run. Cross-crate compile behavior
belongs to a workspace-tier task or an explicit acceptance command.

## `cli-tool` Tier

Use for built-in tools such as `read`, `write`, `edit`, `bash`, `glob`, `grep`,
`find`, and `ls`.

Gates: All `library` gates, plus:
1. Behavioral tests in `crates/opi-coding-agent/tests/` using `tempfile` crate
2. For `bash`: tests for timeout, cwd capture, cancellation
3. For mutating tools: test asserting Phase-1 safety boundary is reported
   before execution (per opi-spec §8.4)

## `cli-runtime` Tier

Use for CLI parsing, config, prompt/context loading, session commands, JSON
mode, tool selection flags, shell completions, and binary subprocess behavior.

Gates: All `library` gates, plus:
1. E2E test booting `MockProvider` + `opi` binary subprocess with scripted prompts
2. Assertions on stdout, stderr, and exit code

**MockProvider precondition:** REFUSE to run if no `MockProvider` symbol exists.
Grep `crates/opi-ai/src/test_support.rs` (or feature-gated path). If absent:
> "Task `<id>` depends on MockProvider scaffolding (task 1.17). Run 1.17 first."

## `tui` Tier

Use for ratatui rendering, keybindings, themes, fuzzy pickers, diff rendering,
terminal image rendering, and snapshot surfaces.

Gates: All `library` gates, plus:
1. Ratatui snapshot tests at fixed sizes (80×24 and 120×40) using `insta`
2. Snapshot diffs require explicit user approval — NEVER auto-accept

## Provider-Contract Addendum

Apply to enterprise providers and HTTP client work: Bedrock, Azure OpenAI,
Vertex, proxy support, and connection pooling.

Additional gates:
1. Fixture or `wiremock` tests cover success, streamed deltas, tool calls when
   applicable, usage, provider errors, and error mapping.
2. Credential precedence tests never require live cloud credentials.
3. Secret redaction tests assert API keys, OAuth tokens, proxy credentials, and
   cloud credentials do not appear in logs, errors, session files, or snapshots.
4. No live provider tests run unless they are `#[ignore]` and explicitly
   invoked outside this skill.
5. Shared HTTP client/proxy behavior is tested without real network calls.

## Multimodal Addendum

Apply to image input, image tool results, and terminal image rendering.

Additional gates:
1. Serialization tests cover image metadata, MIME type, size limits, and
   provider capability rejection.
2. Tool-result tests cover text-only fallback and non-UTF-8/binary-safe handling.
3. TUI tests use deterministic snapshots or golden terminal protocol output; no
   visual snapshot is accepted without explicit user approval.

## Product Acceptance Addendum

Apply to any task with non-empty `acceptance_scenarios`, and to any task whose
DoD claims runtime/startup/CLI/session/adapter/provider behavior.

Additional gates:

1. Run every command listed in each owned `acceptance_scenarios[].verification`
   item.
2. Inspect code paths and tests to prove each
   `acceptance_scenarios[].production_call_sites` entry is exercised by the
   verification. Direct helper, parser, protocol, mock bridge, or registry-only
   tests are substrate evidence unless they enter through the production
   call-site named in the scenario.
3. For CLI/runtime scenarios, include at least one subprocess, harness, RPC, or
   integration test that starts at the public command/API boundary. Unit tests
   may supplement but cannot replace this.
4. If a task cannot close an acceptance scenario yet, mark or keep the task as
   `substrate_only = true`, leave the scenario `open`, and ensure a later
   vertical-slice task owns closure.
5. Before Phase E, the planned commit evidence must include `Opi-Acceptance`
   for every closed scenario.
6. Runtime/CLI/NDJSON/session claims must also satisfy the Artifact Truthfulness
   Gate in `artifact-truthfulness.md`; helper tests alone are substrate
   coverage unless a production command or saved artifact exercises the claimed
   surface.

## Phase-Specific Addenda

Apply these in addition to the task's tier and the Product Acceptance Addendum.

### Phase 6 Alignment Hardening

Additional gates:
1. Documentation tasks that touch English user docs also update localized
   counterparts or explicitly cite why no localized counterpart exists.
2. Phase 6 baseline audit is additive under `docs/snapshots/phase6/`; do not
   rewrite Phase 5 historical audits.
3. Package runtime tasks include at least one local-package startup path and
   degraded-path coverage for invalid/missing/unsupported adapter states.
4. Docs guard tests cover both completed Phase 5 MVP claims and forbidden
   overclaims: npm, marketplace/gallery, update/enable/disable, permission
   enforcement, hot reload, TypeScript extension API compatibility, pi session
   v3 compatibility, pi-web-ui parity, and broad OAuth/provider parity.

### Phase 7 Reliability and Observability

Additional gates:
1. Diagnostic payload tests cover severity, stable code, source, message,
   redacted details, and optional action.
2. Redaction tests cover API keys, bearer tokens, environment values, prompt
   content, tool output, provider URLs, and absolute paths outside the relevant
   workspace.
3. `opi doctor`, `opi doctor --json`, scope selection, and exit-code policy are
   covered without paid provider calls or network requirements.
4. Trace envelope tests cover schema version, run/turn id, sequence,
   timestamp, source, kind, diagnostic linkage, and summary/verbose redaction.
5. Docs state observability is local and explicit; no telemetry, analytics,
   remote trace service, or web dashboard is added.

### Phase 8 Agent Runtime Stabilization

Additional gates:
1. Contract tests cover event order for no-tool, one-tool, parallel, sequential,
   mixed scheduling, validation failure, hook block, hook modification,
   cancellation, compaction, and steering/follow-up ordering.
2. Hook tests cover every `AgentHooks` method and failure semantics.
3. SDK/RPC contract tests cover busy-state rejection, abort, steer,
   follow-up, set_model, thinking level, compact, session_info, and
   extension_command behavior.
4. Public `opi-agent` API review classifies touched surfaces as supported 0.x,
   unstable internal, or candidate removal in docs.
5. No plan mode, sub-agent system, todo system, permission popup, MCP runtime,
   package ecosystem expansion, or web UI product enters core.

### Phase 9 pi 0.80.2 Baseline Realignment

Additional gates:
1. Documentation tier is required unless a reviewed source update changes
   Phase 9 scope.
2. English and Chinese normative docs name `.repo/pi-0.80.2` as the current
   studied upstream baseline.
3. Stale `.repo/pi-0.75.3` references appear only as historical notes, not as
   current baseline statements.
4. Docs guards reject current-scope overclaims for OAuth parity, image
   generation, custom extension UI parity, npm/gallery/update/enable/disable,
   web/share, and `pi` session compatibility.
5. No Rust code, runtime behavior, provider auth, image generation, custom UI,
   package ecosystem, web/share flow, `pi` session compatibility, or
   `opi-types` crate is added.

### Phase 10 Core Architecture Deepening

Additional gates:
1. Provider collection/auth work includes tests for model lookup, provider
   dispatch, auth status/diagnostics, compatibility metadata, and existing
   provider construction through `opi-coding-agent`.
2. Generic harness work includes contract tests for phase guards, turn
   snapshots, save points, busy rejection, runtime config mutation timing,
   cancellation, and ordered pending session writes.
3. Session repo/facade work preserves existing session readability and proves
   deterministic branch/leaf reconstruction.
4. `CodingHarness` remains the coding-agent product wrapper; generic behavior
   moved into `opi-agent` is exercised through at least one production CLI,
   JSON, RPC, or interactive path.
5. Hook-boundary changes keep current core/product hooks narrow and document
   future provider/UI/session lifecycle paths without implementing them.
6. No OAuth login, subscription auth, broad provider catalog expansion, image
   generation, custom TUI extension protocol, npm/gallery, browser/web UI,
   `pi` TypeScript API compatibility, `pi` session file compatibility,
   `opi-types`, or whole-loop rewrite is added.

### Phase 11 Tooling Quality

Additional gates:
1. Built-in tool results expose consistent `content`, `details`, `is_error`,
   diagnostics, truncation, and path metadata.
2. Filesystem tool tests cover Windows paths, drive prefixes, Unicode,
   line endings, large files/output, binary/encoding errors, symlinks where
   supported, ignore handling, sorting, limits, and cancellation.
3. Mutating tool tests prove create/overwrite/edit conflict behavior with diff
   or audit summaries and no silent partial writes.
4. `bash` tests cover timeout, cancellation, cwd/env reporting, exit code,
   truncation, mutating classification, and sequential execution.
5. No permission popup, persistent background bash, remote execution, IDE
   index, language server, automatic formatting, sandbox, or workflow tool is
   added to core.

### Phase 12 Provider Correctness

Additional gates:
1. Every existing provider family has fixture coverage for request
   serialization, streaming lifecycle, usage, finish reasons, errors, and
   cancellation where supported.
2. Provider errors map into the documented taxonomy: auth, config, request,
   network, rate_limit, provider, stream, capability, and cancelled.
3. Tool calls, thinking/reasoning, image input, usage/cost, retry, proxy, and
   OpenAI-compatible profile flags are tested with fixtures or `wiremock`.
4. OpenAI-compatible breadth remains config-driven unless the reviewed source
   explains why a first-class adapter is required.
5. No live provider calls run by default; no OAuth, subscription auth, image
   generation, browser usage feature, provider streaming adapter protocol, or
   broad provider catalog expansion is added.

### Phase 13 Session Tree and Context Reconstruction

Additional gates:
1. New session entries round-trip and rebuild context deterministically, or are
   explicitly deferred with source citations.
2. Context-building tests cover active leaf resolution, corrupt trailing lines,
   model/thinking changes, compaction, branch summaries, custom messages, and
   extension state restoration when implemented.
3. Export tests cover local markdown or JSON output, active-branch/full-tree
   selection, tool/thinking inclusion controls, and redaction.
4. All session tests isolate data with temp directories or `OPI_SESSIONS_DIR`.
5. Docs state session files are sensitive and context is bounded to session
   files/explicit exports. No vector memory, global user profile, cloud sync,
   session sharing service, web UI product, or pi session v3 compatibility
   claim is added.

## D.3 Gap-Only Gates

D.1 already owns mechanical format, compile/lint, test, and rustdoc proof. D.3
runs only commands still missing for the task's acceptance scenarios:

- a public CLI/harness/subprocess or production-call-site check not covered by
  the named D.1 test;
- a generator/checksum/artifact verifier;
- an authoritative OS/target check that cannot run locally;
- an explicit external acceptance command from the reviewed source.

Build the union of D.0, D.1, and D.3 commands before execution and deduplicate
exact commands and equivalent supersets. A workspace-tier D.1 `smoke full` is
never repeated in D.3. A focused task is promoted to workspace tier only for a
real cross-crate semantic contract, not as a precaution.

Commit-staging gates (every non-documentation tier, unchanged):

1. Capture `baseline_dirty_files` at Phase B before implementation starts.
2. Before commit-stage, every entry in
   `git status --porcelain --untracked-files=all` MUST satisfy ONE of:
   - present in `baseline_dirty_files` AND unchanged by this task AND not
     matched by `task_owned_paths` (untouched baseline, leave alone);
   - matched by `task_owned_paths` (intentional task file, will be staged);
   - matched by `task_owned_paths` AND also present in `baseline_dirty_files`
     → REFUSE; print the overlap and ask the user to either split the file
     manually or explicitly confirm the baseline edit is task-owned.
3. Stage only paths matched by `task_owned_paths` AND changed since
   `start_commit`. Never use `git add -A` or `git add .`.
4. Pre-commit: `HEAD` must equal `tasks[].start_commit` unless the only new
   commit is a reviewed manual task commit handled by `--resume-from-manual`.
5. Post-commit: `HEAD^` must equal `start_commit`; no path matched by
   `task_owned_paths` may remain dirty. Files in `baseline_dirty_files` that
   were not modified by the task remain as-is.
6. Commit message includes `Opi-*` evidence footers.

### `--resume-from-manual`

Skip commit creation only if:
- Exactly one candidate manual commit since `start_commit`
- No task-owned dirty files remain outside the candidate manual commit;
  unrelated baseline dirty files are allowed and must not be staged.
- Phase D passes
- Commit already contains `Opi-*` footers

If footer missing: print required footer text and stop (do NOT amend).

## Task Graph Verification Checks

Before confirming an init or reinit graph:

1. Every `behavioral_tests` path must be covered by `task_owned_paths`.
2. If `behavioral_tests` spans multiple crates, use `workspace` tier or include per-crate `cargo test`, `cargo clippy`, and rustdoc gates for every referenced crate.
3. If any behavioral or snapshot test lives under `crates/opi-tui/tests/`, set `snapshot_tests` for the affected snapshot path and mark snapshot acceptance as explicit human approval.
4. Direct spec rows use `parent_spec_row = null`; only dotted sub-task IDs use a parent row string.
5. Rows with open crate labels such as `examples / package template` must include the concrete test paths they declare, even when implementation files live under `examples/**`.
6. Example/package tasks must not own broad `docs/**`; use a task-specific
   docs subtree such as `docs/extension-examples/**`.
7. Reviewed documentation/alignment tasks may own exact documentation files
   required by their DoD, including `docs/opi-spec.md` and localized
   counterparts. They still must not own broad `docs/**`.
8. Public protocol or extension substrate tasks must include documentation
   requirements in their DoD when they introduce RPC, SDK, extension,
   provider/model registration, adapter protocol, transport, or proxy surfaces.
9. No task may include `docs/opi-spec.md` in `task_owned_paths` unless it is a
   reviewed documentation/alignment task whose DoD explicitly requires updating
   `docs/opi-spec.md` and the localized counterpart. Use exact file paths only.
10. Every source-spec goal, success criterion, exit criterion, or named user
    workflow for the active phase must be covered by at least one
    `acceptance_scenarios` entry, or be explicitly deferred by a current spec
    citation.
11. A runtime/startup/CLI/session/adapter/provider acceptance scenario must list
    production call sites. If no production call site exists yet, the owning
    task must be `substrate_only = true` and a later vertical-slice task must
    close the scenario.
12. Vague DoD verbs (`works`, `supports`, `loads`, `integrates`, `bridges`,
    `productizes`, `handles`) must be expanded into observable assertions before
    graph confirmation.
13. For phases 14-18, `spec_files` must match the reviewed source registry in
    `skill.md` for the active phase; arbitrary docs under
    `docs/superpowers/specs/` are not normative.
14. Phase non-goals must appear as `forbidden_scope` inference notes or
    phase-specific verification checks before graph confirmation.

## Risk Evaluator Gate

A task has `evaluator_required = true` only when it changes semantic high-risk
behavior that benefits from adversarial judgment:

- security, command/tool safety, authentication, authorization, permissions,
  credential handling, or destructive-operation boundaries;
- public API/protocol/schema compatibility or cross-crate semantic contracts;
- session persistence, branch reconstruction, durability, recovery, or
  user-data export/redaction;
- provider wire formats, model-visible event/tool behavior, cancellation/event
  ordering, or runtime behavior with ambiguous acceptance criteria;
- a release-critical migration whose failure is not fully characterized by a
  deterministic mechanical gate.

Tier name, multiple touched files, TUI work, diagnostics, and configuration do
not automatically require an evaluator. Set the flag only when the actual
change meets a criterion above.

`evaluator_required` is static (confirmed at init). Phase D MUST NOT dynamically
promote a task. Phase-exit evaluation is separate (Phase F).

D.2 skip rule: every task with `evaluator_required = false` skips exec-verify.
This normally includes deterministic docs/skills/metadata, test-only changes,
mechanical generation/version edits, dependency-neutral cleanup, and
behavior-preserving internal refactors with focused existing tests. D.0/D.1/D.3
remain mandatory. Phase-exit evaluation is separate and still runs once.

The evaluator receives: DoD, diff from `start_commit`, new/changed tests,
verification outputs, planned commit evidence, acceptance scenarios, production
call-site traces, and current source-spec success/exit criteria. It answers:
1. Does diff satisfy DoD without scope creep?
2. Do tests exercise behavior (not just implementation details)?
3. Public API/protocol/security risks not covered by mechanical gates?
4. Do closed acceptance scenarios start at the promised user/API boundary and
   reach the runtime effect claimed by the design?
5. Are all runtime claims wired through production call sites rather than only
   tested through helper functions?
6. Is evidence footer truthful and sufficient, including `Opi-Acceptance` when
   scenarios are closed?

If evaluator fails → back to Phase C with findings as input. Generator may NOT
self-approve the finding away.

### Verify engine exec stage (Phase D.2)

The D.2 evaluator is realized by the verify engine's exec stage (see
`references/verify-engine.md`):

- `evaluator_required = true` tasks → full 6-lens deep Workflow at
  `.claude/skills/opi-implement/scripts/exec.workflow.js` (L-D1 implementation-matches-DoD, L-D2
  tests-non-vacuous, L-D3 production-call-site-proven, L-D4
  evidence-truthfulness, L-D5 non-goal-leak, L-D6 workspace-deps-honored), with
  adversarial verify before any must-fix disposition.
- All other tasks → no D.2 run and no `verify_runs` exec entry.

Must-fix findings BLOCK Phase D pass, route to Phase C, and increment
`iteration_count` against `max_iterations` (5). The engine records a
`verify_runs` ledger entry (`stage = "exec"`) and never auto-edits code.

## Phase-Exit Verify Gate (Phase F.1a)

The F.1a phase-exit evaluator produces its criteria trace (working machinery,
phases 1-13); the verify engine's phase-exit stage then adversarially audits
that trace (see `references/verify-engine.md`). It always runs the full 5-lens
deep Workflow at `.claude/skills/opi-implement/scripts/phase-exit.workflow.js` (L-F1 traced-to-code, L-F2
traced-to-test, L-F3 non-goals-respected, L-F4 residuals-exactly-cited, L-F5
substrate-vs-product-honest), once per phase.

Findings that survive adversarial verify upsert
`phase_exit[N].criteria_trace[C].status = not-met` (with the finding's
`source_citation` as evidence pointer); `F.1b`'s existing REFUSE rule then fires
on that row, blocking phase archive. Flagged findings surface in the report only
and do not mutate the trace. The engine records a `verify_runs` ledger entry
(`stage = "phase-exit"`).
