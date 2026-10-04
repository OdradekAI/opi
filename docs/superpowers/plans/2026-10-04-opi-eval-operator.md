# Opi Eval Operator Implementation Plan

> **For agentic workers:** Use superpowers:subagent-driven-development for bounded independent implementation and review. User approval in this conversation authorizes skill maintenance and the scoped native reasoning repair; this plan does not admit unrelated product work.

**Goal:** Configure evaluations once, preserve versioned pi baselines, build current-worktree Opi candidates, compare verified evidence offline, and enforce selected thinking levels.

**Architecture:** A private operator TOML resolves to existing Companion experiments. Skill-local Python helpers own configuration, immutable per-run material views, exact retry plans, snapshot discovery, comparisons and handoffs. Rust and pinned Harbor/Pier integrations own effective native thinking projection and fail-closed execution proof.

**Tech Stack:** Python standard library (tomllib, unittest, subprocess), existing Rust opi-eval and native integrations; no new dependencies.

## Global constraints

- Preserve all carried-in staged, unstaged and untracked changes; no commits, branches, paid provider calls or ledger writes.
- Respect docs/opi-spec.md CTRL-002 through CTRL-007 and the existing evidence/privacy contracts.
- Cargo caches remain shared with incremental compilation; runtime packages used by a frozen run remain immutable.
- Candidate builds use the current working tree, including relevant dirty inputs; failed builds never use a previous executable.
- No matching baseline means a truthful standalone result, never automatic pi dispatch. Cross-run comparisons bind both original evidence identities and remain labelled cross-run.
- Thinking is one requested control with effective projection and actual-client proof; unsupported required values reject before live dispatch.

## Task 1: Unified operator and evidence flow

**Files:** `.agents/skills/opi-eval/config.example.toml`, `.agents/skills/opi-eval/scripts/`, `scripts/test_opi_eval_operator.py`.

**Interfaces:** CLI `evaluate.py init|resolve|baseline|candidate|compare`; `--config` selects the sole editable profile. Live actions have separate prepare/run stages and use frozen plans. Machine output is a sanitized effective profile, per-run attempt records, snapshot/comparison manifests and handoff paths.

- [x] Define failing tests for strict TOML validation, secret-free resolution, thinking requirements, immutable outputs, baseline/task identity matching, unknown metrics, deterministic choice, exact retries and batch deadline.
- [x] Run `python scripts/test_opi_eval_operator.py` and observe missing-behavior failures.
- [x] Implement configuration and operation helpers around the actual current native material/plan/report formats; never fake supported CLI flags or scores.
- [x] Verify: run the complete Python test binary and representative subprocess refusal with synthetic credentials and temporary directories.

## Task 2: Effective native thinking

**Files:** `crates/opi-eval/src/experiment.rs`, applicable runner/control/proof files, both pinned integration modules, focused Rust and Python tests.

**Interfaces:** Existing experiment `[model_controls].reasoning` carries an explicit level. Projection config/handoff must bind and enforce the selected level in both clients; required unsupported levels reject. Existing omitted historical runs remain readable.

- [x] Add tests exposing the current fixed-off launch and unsupported projection; run them to observe failure.
- [x] Implement supported per-client projection, guarded native launch and truthful reporting, with no private reasoning capture.
- [x] Verify: exact-client loopback requests for first request and tool continuation, positive levels and off, unsupported level rejection, native integration conformance, focused crate tests and affected-target clippy.

## Task 3: Skill and handoff contracts

**Files:** `.agents/skills/opi-eval/SKILL.md`, its native/baseline/report references and UI metadata, `.agents/skills/README.md` and `.zh.md`, `.gitignore`, complete Unreleased section of `CHANGELOG.md`, crate README pair as needed.

- [x] Make baseline/candidate/compare and the private configuration the normal user entry; preserve acceptance, verify, fidelity and confirmed repair boundaries.
- [x] Document config precedence, credential handling, source selection, actual budget/retry enforcement, snapshot eligibility and explicit-only caller authorization.
- [x] Correct obsolete build/fixtures/recovery/replacement instructions; distinguish seals from comparison keys and native scores from unknown telemetry.
- [x] Verify: `python scripts/opi-doc-check.py`, `git diff --check`, skill validation, independent scenario review and outgoing worktree inventory.

## Completion

All three tasks are implemented and their required offline checks pass. Test impact is recorded as add/update/retain. No live benchmark result is claimed from mocks. Any endpoint/client incompatibility is named rather than silently dropping thinking controls.

## Verification evidence

- Operator: all 44 tests pass on Windows and Ubuntu WSL, including precise retries, secret-free exports, verified baseline admission, cancellation, and immutable internal source links.
- Current Rust CLI: both generated pi and Opi default experiments validate; both required unsupported thinking projections reject. Four offline checks are registered in `docs/eval/evidence-roots.jsonl`.
- Native thinking: actual pinned pi 1.0.0 and current-worktree Opi pass all 15 API/level loopback cases, including first request and tool continuation. Original failures and fresh retakes are retained in the registered repair-proof root; these are not benchmark scores.
- Rust: `opi-eval` library (178), experiment contract (35), focused native thinking/driver tests, actual-client proof test, affected-target clippy, and workspace formatting pass.
- Python native integrations: 40 tests pass with one opt-in Docker test skipped; scripted-provider tests (25) and control-proof tests (2) pass.
- Documentation and whitespace checks pass. The generic skill validator rejects the pre-existing `disable-model-invocation` frontmatter key because its whitelist omits that key; the repository checker accepts the required explicit-invocation policy, which is preserved.
- Independent review has no unresolved P1/P2 findings. No paid benchmark, commit, branch, manifest/dependency change, or implementation-ledger write occurred. Carried-in audit changes remain intact.
- Test impact: **add** operator, proof and native thinking tests; **update** experiment/integration/provider tests; **retain** historical evidence and privacy contracts; **delete** none.
