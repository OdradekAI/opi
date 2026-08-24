# Phase 17 Remediation Result

**Status**: COMPLETE
**Audit index SHA-256**: `bc8041d8e8aa26d9067b02f006263ecf922c23d8263e13e1ac60b8b434194ed1`
**Plan SHA-256**: `09ac20ef83029cb2d9084e4ad859dea382432303d87922a7a3bd150ab7767fb1`
**Changed paths**: ["crates/opi-agent/src/agent_loop.rs","crates/opi-coding-agent/tests/phase17_tool_authority.rs"]

## Result

- `P17-AUD-001`: Closed. Terminal tool evidence now derives its execution
  outcome from the actual lower-boundary `Tool::execute` result before the
  presentation-only after-call replacement. The replacement still owns
  diagnostics, events, messages, and model context.
- `P17-AUD-002`: Refuted. Independent fixed-ref comparison and GitHub run
  `32733627895` establish byte-identical implementation/CI inputs and
  successful Phase 17 acceptance on Ubuntu, macOS, and Windows.
- `P17-AUD-003`: Info/No action. The trace-compatible RPC constructor remains
  intentional and its supported command path remains covered.

No incidental repair was admitted. The first workspace all-targets run exposed
the known unrelated RPC thinking-level timeout under parallel load. Its exact
isolated test passed, no repair was made, and a fresh complete workspace
all-targets rerun passed, including all 80 `rpc_jsonl` tests.

## Verification

    cargo test -p opi-coding-agent --test phase17_tool_authority phase17_after_call_replace_keeps_later_authorization_unchanged -- --exact --nocapture
    RED: FAIL, outcomes [Failed, Failed] instead of [Succeeded, Succeeded]
    GREEN: PASS, 1 passed, 0 failed

    cargo fmt --check --all
    PASS

    cargo clippy --workspace --all-targets -- -D warnings
    PASS

    cargo test --workspace --all-targets
    First run: FAIL, one unrelated rpc_jsonl timeout
    Isolated rpc_set_thinking_level_off_medium_high_change_runtime_config: PASS
    Fresh complete rerun: PASS

    cargo test --workspace --doc
    PASS

    $env:RUSTDOCFLAGS = "-D warnings"; cargo doc --workspace --no-deps
    PASS

    python scripts/opi-doc-check.py
    PASS

    git diff --quiet 87377fcf750a5d0a38919bf82e740b7baefe8a8b..68507a86b5e99a226bb65b219f274f4f729fd88c -- .github crates Cargo.toml Cargo.lock scripts
    PASS

    gh run view 32733627895 --repo OdradekAI/opi --json headSha,status,conclusion,jobs
    PASS: head 87377fcf750a5d0a38919bf82e740b7baefe8a8b, completed,
    success, three successful Phase 17 acceptance jobs

    git diff --check
    PASS

## Materialization Boundary

The approved fixes and fixed remediation artifacts are materialized only in the
current worktree. They are not committed or published. A fresh audit or
reviewer rerun is not admitted until the fixes and active assurance set are
committed and the assurance directory is clean.
