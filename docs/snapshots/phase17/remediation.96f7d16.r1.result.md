# Phase 17 Remediation Result

**Status**: REMEDIATION-CHECKS-PASSED  
**Approved plan**: \`docs/snapshots/phase17/remediation.96f7d16.r1.plan.md\`  
**Starting endpoint**: committed \`96f7d161045c94113ec9f02f5ad3ff4c8121cea5\`  
**Resulting endpoint**: the same committed endpoint with the approved remediation changes uncommitted  
**Approval provenance**: User explicitly approved the exact plan in this task: \`[$opi-remediate](D:\Luiz\Odradek\opi\\.agents\skills\opi-remediate\SKILL.md) mode=apply plan=docs/snapshots/phase17/remediation.96f7d16.r1.plan.md\`, received 2026-08-23 (Asia/Shanghai).  
**Dirty-worktree baseline**: staged none; unstaged none; carried untracked audit artifacts were \`audit.codex.md\`, both Phase 17 audit reports and findings sidecars, and the approved plan plus plan dispositions.  
**Unresolved decisions**: none

## Closure batches

| Batch | Outcome | Evidence |
|---|---|---|
| B1 | Closed | v2 session headers persist an immutable runtime-input binding; required entries fail closed, while only explicit ignorable observations recover. |
| B2 | Closed | A complete malformed Bedrock frame emits one typed stream error and terminates decoding. |
| B3 | Closed | An Allow whose evidence health changes is reauthorized before a tool can launch. |
| B4 | Closed | Session CLI integration tests resolve the Cargo-provided binary under the external target directory. |
| B5 | Closed | The complete registered Phase 17 reverse range yielded the exact pre-Phase tree and passed the registered runtime profile. |
| B6 | Closed | The false Unreleased accessor-removal wording was removed. |
| B7 | Closed | Tool visibility is recomputed independently for consecutive provider requests. |
| B8 | Closed | Untrusted prompt, hook, tool-output, retrieval-shaped, skill-shaped, and child-shaped content cannot forge authority. |
| B9 | Closed | Empty evidence manifests return a typed finalization error rather than unwinding. |
| B10 | Closed | The Retry contract fixture is structurally valid before parent-link assertions. |
| B11 | Closed | Early unknown-tool and invalid-schema paths assert zero hook/authorizer consultation. |
| B12 | Closed | The unused direct \`anyhow\` dependency was removed and Cargo regenerated the lockfile. |
| B13 | Closed | The three cited source comments retain contract prose without sprint-history labels. |

## Applied verification

- B1: \`cargo test -p opi-agent --test session_storage --test session_branching --test session_facade --test image_input_session\` — PASS; \`cargo test -p opi-coding-agent --test session_cli --test phase17_legacy_migration --test phase17_tool_authority --test phase17_product_evidence\` — PASS.
- B2: \`cargo test -p opi-ai --test bedrock_fixtures crc_invalid_complete_frame_emits_one_error_and_stops -- --exact\` — PASS; Bedrock unit tests and fixture suite — PASS.
- B3/B9/B10/B11: focused \`evidence_runtime\`, \`evidence_contract\`, \`tool_authority\`, and \`tool_validation\` checks — PASS.
- B4: full \`cargo test -p opi-coding-agent --test session_cli\` under \`CARGO_TARGET_DIR=E:\opi\cargo-targets\opi-remediate-phase17-96f7d16-bd26579c344448aa920b7a2c140eb10b-008bdaa87b0bb4c8\` — PASS.
- B5: detached worktree \`C:\Users\Luiz\AppData\Local\Temp\opi-remediate-phase17-rollback-96f7d16-r1\` at \`40f2e6ee4866f1cd44eefb952b8f40afcbb029ac\`; \`git revert --no-commit 4c4a8404b8a05e09f3479bc320c6f361ed2c9437..40f2e6ee4866f1cd44eefb952b8f40afcbb029ac\` — exit 0; \`git diff --exit-code 4c4a8404b8a05e09f3479bc320c6f361ed2c9437\` — exit 0; \`provider_collection\` (31), \`session_storage\` + \`hooks_queues\` (35 + 14), and \`session_runtime\` + \`non_interactive_policy\` (58 + 10) — PASS. The owned probe worktree was removed.
- B6/B13: \`python scripts/opi-doc-check.py\` — PASS.
- Final union: \`cargo fmt --check --all\`, \`cargo clippy --workspace --all-targets -- -D warnings\`, \`cargo test --workspace --all-targets\`, \`cargo test --workspace --doc\`, and \`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps\` — PASS.
- Scope checks: \`git diff --check\` — PASS. Cargo metadata/lock review contains only the required \`sha2\` addition for \`opi-agent\` and direct \`anyhow\` removal from \`opi-coding-agent\`.

## Changed paths

- Session durability and evidence binding: \`crates/opi-agent/src/session.rs\`, \`crates/opi-agent/src/evidence.rs\`, \`crates/opi-coding-agent/src/session_coordinator.rs\`, \`crates/opi-coding-agent/src/session_cli.rs\`, and \`crates/opi-coding-agent/src/harness.rs\`, with affected session/resume fixtures.
- Provider integrity and post-Allow authorization: \`crates/opi-ai/src/bedrock/event_stream.rs\`, \`crates/opi-ai/src/bedrock/mod.rs\`, and \`crates/opi-agent/src/agent_loop.rs\`, with their focused tests.
- Product authority and evidence contract coverage: \`crates/opi-coding-agent/tests/phase17_tool_authority.rs\`, \`crates/opi-coding-agent/src/evidence.rs\`, and focused agent tests.
- Dependency/documentation corrections: \`Cargo.toml\`, \`Cargo.lock\`, the affected crate manifests, \`CHANGELOG.md\`, and the three source-comment modules.

## Test impact

| Batch | Impact |
|---|---|
| B1 | add, update |
| B2 | add, update |
| B3 | update |
| B4 | update |
| B5 | none |
| B6 | none |
| B7 | add |
| B8 | add |
| B9 | update |
| B10 | update |
| B11 | update |
| B12 | none |
| B13 | none |

## Remaining dispositions

The result disposition sidecar retains all 37 source identities. The 13 approved action records are \`Closed\`; exclusions remain \`Info/No action\`, \`Refuted\`, \`Cannot confirm\`, or \`Returned to shaping\` exactly as the approved plan requires.

No commit was created. These are remediation checks, not a Phase PASS. After the user’s ordinary commit/materialization gate, request a fresh independent \`$opi-audit\` for the new committed endpoint.

