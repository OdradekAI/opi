# Phase 17 Remediation Result

**Status**: COMPLETE
**Audit index SHA-256**: `3a431c990871dd9183b31a1376daee59a3e4f2d888b7393dfb208e14a3cdea3f`
**Plan SHA-256**: `d724f0fd26b1e43389af37d649ec952d9103a104f271eaa929aaef93f8dfb363`
**Changed paths**: ["CHANGELOG.md", "crates/opi-coding-agent/src/project_trust.rs", "crates/opi-coding-agent/tests/project_trust_store.rs"]

`COMPLETE` means this apply execution and its machine dispositions are fully
recorded. It does not mean every finding is closed or that Phase 17 conforms.

## Admission

- `audit-set`: PASS for the current live Phase 17 index.
- Plan dispositions: PASS.
- Approved plan: PASS with the exact invocation digest
  `d724f0fd26b1e43389af37d649ec952d9103a104f271eaa929aaef93f8dfb363`.
- Current `HEAD` remained
  `23f5754c6e9b1f46ea3151222fc1c1289ae5b64a`, matching the approved
  remediation head.
- The active index digest remained
  `3a431c990871dd9183b31a1376daee59a3e4f2d888b7393dfb208e14a3cdea3f`.
- The three approved production paths had no staged or unstaged overlap before
  apply. Carried-in assurance changes were preserved.

## Batch B1 result — P17-AUD-001

**Remediation status**: Not closed

The behavioral defect is repaired in the worktree: registered resolver votes
now combine monotonically so `Deny` dominates `Trust` regardless of
registration order, while all-`Undecided` still falls through. Explicit CLI
override precedence, an empty standard registry, store/default/ask fallback,
and sealing behavior remain unchanged.

The approved red-before test was reproduced in a fresh archive of the bound
head before production edits:

```text
cargo test -p opi-coding-agent --test project_trust_store conflicting_resolver_votes_deny_independent_of_registration_order
FAIL: left Trusted, right Untrusted
```

The same regression and the updated embedder precedence case both failed for
the expected reason after the tests were installed and before implementation.
After the minimal implementation change, both passed.

The finding remains `Not closed` because the approved verification union did
not become entirely green. The exact global command `git diff --check` failed
on a carried-in active audit report:

```text
docs/snapshots/phase17/assurance/audit.codex.gpt56.md:218: new blank line at EOF.
```

That report is outside Batch B1's causal and owned paths, and its bytes are
bound by the active audit index. Editing it as an incidental repair would both
violate the approved path boundary and invalidate the approval digest, so no
such edit was made. A scoped `git diff --check` over the three approved
production paths passed.

## P17-AUD-002 result

**Remediation status**: Info/No action

`cargo test -p opi-coding-agent --test phase17_api_audit` passed all 22 local
static checks on Windows. This confirms the platform-neutral acceptance and CI
matrix structure only. It does not provide Linux or macOS execution evidence,
and no current-head three-platform closure is claimed.

## Verification evidence

| Command | Result |
| --- | --- |
| `cargo test -p opi-coding-agent --test project_trust_store conflicting_resolver_votes_deny_independent_of_registration_order` | PASS after the observed red-before failure |
| `cargo test -p opi-coding-agent --test project_trust_store` | PASS: 27 passed |
| `cargo test -p opi-coding-agent --test project_trust_startup` | PASS: 11 passed |
| `cargo test -p opi-coding-agent --test interactive_trust` | PASS: 4 passed |
| `cargo test -p opi-coding-agent --test non_interactive_trust` | PASS: 2 passed |
| `cargo test -p opi-coding-agent --test rpc_trust` | PASS: 2 passed |
| `cargo test -p opi-coding-agent --test phase17_api_audit` | PASS: 22 passed locally on Windows |
| `cargo clippy -p opi-coding-agent --all-targets -- -D warnings` | PASS |
| `cargo fmt --check --all` | PASS |
| `python scripts/opi-doc-check.py` | PASS |
| `git diff --check` | FAIL on the carried-in active audit report named above |
| `git diff --check -- CHANGELOG.md crates/opi-coding-agent/src/project_trust.rs crates/opi-coding-agent/tests/project_trust_store.rs` | PASS |

Test impact: `update`. The existing `project_trust_store` integration binary
now rejects conflict-order authority widening; no new test binary was added.

## Worktree and materialization boundary

- `HEAD` is unchanged; this task created no commit.
- Task-owned production changes are exactly `CHANGELOG.md`,
  `crates/opi-coding-agent/src/project_trust.rs`, and
  `crates/opi-coding-agent/tests/project_trust_store.rs`.
- Fixed plan/result artifacts are remediation-owned outputs. Existing audit
  and history modifications remain carried-in and untouched.
- No staged or untracked paths were introduced.
- The behavior repair and current audit set must be materialized deliberately,
  and the global diff-check blocker must be resolved under a newly bound plan,
  before any fresh audit request. This result grants no Phase PASS.
