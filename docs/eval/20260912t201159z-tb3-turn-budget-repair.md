# TB3 turn-budget source repair

The approved repair is verified offline. Native Opi projection now explicitly sets `[defaults].max_iterations = 4294967295`, the existing field's maximum finite `u32` value. It avoids the inherited 50-turn cutoff without changing the ordinary Opi default, Agent Core, native timeouts, cancellation or benchmark grades. This is a finite ceiling, not an unlimited mode or token budget.

Authorization: the user's “继续修复” followed the observed TB3 `max turns exceeded (50)` failure. The active `opi-eval` confirmed-repair branch covers reproduction, source repair, synthetic checks and candidate builds. No new paid trial, credential resolution, regrade, commit, publication or implementation-ledger update occurred.

The production change is limited to the Companion's Opi configuration projection and the matching Python offline acceptance comparison. The verifier accepts the new exact ceiling and historically sealed projections that omitted it; arbitrary values/types still reject. Skill preparation now discloses the finite Agent budget separately from native deadlines. New projections require fresh client proof, material and plan identities.

The actual Opi binary first failed at 50 provider requests with the original projection. With the repaired projection it completed 51 successful tool turns and one final response: 52 requests, exit 0. Counterfactual configurations still stop at 50 by default and at 2 when explicitly selected, both exit 1. The custom model without an output capability rejects before dispatch (0 requests, exit 2); supporting omitted model capabilities is outside this repair. Nine unit cases cover numeric/omitted/unknown output controls on all three APIs.

Verification: 273 Rust tests passed in `opi-eval --all-targets`; seven material-dependent tests are ignored in that command, with the actual-client turn-budget and output-control tests separately run and passed. The Python verifier passed all 37 tests, including historical/current projections and invalid turn-budget values. Native timeout/cancellation regressions passed. Formatting, targeted clippy and Linux musl builds passed; the doc-test command passed with zero doctests. Independent read-only review found no actionable issues.

Both frozen clients again proved `max_tokens = 131072` on the initial request and tool continuation with a 1000000-token declared context. The original TB3 run still verifies, and two reports reconstructed by the new Companion are identical to each other and to the previous report. All 2358 indexed historical files across four protected roots match their original hashes. The previous native rewards remain zero; these offline tests do not establish improved task success or resolve pi's prior connection failure.

Scope is worktree-only: HEAD unchanged, committed/staged changes none for this task. Task-only changes: `src/experiment.rs`, `scripts/verify-local-acceptance.py`, `scripts/test_verify_local_acceptance.py` under `crates/opi-eval`. Carried-in-and-touched: `tests/native_driver.rs`, paired crate README, `CHANGELOG.md`, the skill entrypoint and native-run reference. All other carried-in changes are preserved. Test impact: **add** two Rust regressions and one Python regression; **update** the synthetic evidence fixture; **retain** supervision and existing acceptance tests; delete none. No English/Chinese source counterpart is missing: paired READMEs and this report are synchronized; the skill reference and changelog have no separate Chinese counterpart.

Initial compile and test failures remain in the command evidence, including the real red reproduction and the acceptance-verifier mismatch caught by the full crate gate. The generic skill validator exits 1 because it does not support the repository's pre-existing `disable-model-invocation` frontmatter. That user-only invocation contract is preserved; `python3 scripts/opi-doc-check.py` and `git diff --check` pass (final records in evidence).

Cleanup: isolated fixture homes/listeners were closed by their test lifecycles and this operation's Cargo leases released. Shared caches (149.85 GiB observed), candidates and historical evidence are retained. Docker inspection is partial because the engine is stopped and the WSL Docker executable is unavailable; no real Docker resource was created or removed. See the cleanup record for exact retained scope.

Further live TB3 execution remains pending new preparation/authorization. The preserved probe input is not a frozen native plan. Actual TB3 limits remain 18000 seconds for the native job, 14400 for the Agent and 900 for the verifier; no total-token or monetary cap is claimed. Current Opi source selection without a version still means the latest local working tree at that future preparation time.

| Command (Linux cwd: `/mnt/d/Luiz/Odradek/opi`) | Exit | Record |
|---|---:|---|
| `cargo fmt --check --all` | 0 | [fmt](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/fmt.json) |
| `cargo clippy --locked -p opi-eval --all-targets -- -D warnings` | 0 | [clippy](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/clippy.json) |
| `cargo test --locked -p opi-eval --all-targets` | 0 | [crate-tests-final](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/crate-tests-final.json) |
| `cargo test --locked -p opi-eval --doc` | 0 | [crate-doctests](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/crate-doctests.json) |
| `python3 crates/opi-eval/scripts/test_verify_local_acceptance.py` | 0 | [green-acceptance](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/green-acceptance.json) |
| `/usr/bin/env CC_x86_64_unknown_linux_musl=gcc cargo build --locked --release --target x86_64-unknown-linux-musl -p opi-coding-agent --bin opi -p opi-eval --bin opi-eval --message-format=json-render-diagnostics` | 0 | [release-final](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/release-final.json) |
| `/usr/bin/env OPI_EVAL_OPI_BINARY=$CANDIDATE/builds/opi cargo test --locked -p opi-eval --test native_driver native_opi_turn_budget -- --ignored --exact --nocapture` | 0 | [candidate-turn-budget](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/candidate-turn-budget.json) |
| `/usr/bin/env OPI_EVAL_NATIVE_CLIENT_MATERIAL=$CANDIDATE/clients.json OPI_EVAL_OUTPUT_CONTROL_OUT=$CANDIDATE/output-control cargo test --locked -p opi-eval --test native_driver native_output_control_client_matrix -- --ignored --exact` | 0 | [candidate-output-control](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/candidate-output-control.json) |
| `$CANDIDATE/builds/opi-eval verify --root /home/luiz/opi-eval-runs/20260911t141317z-tb3-repair/runs/terminal-bench-3.0` | 0 | [historical-verify](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/historical-verify.json) |
| `$CANDIDATE/builds/opi-eval report --root /home/luiz/opi-eval-runs/20260911t141317z-tb3-repair/runs/terminal-bench-3.0 --out /mnt/e/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/historical-report-a.json` | 0 | [historical-report-a](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/historical-report-a.json) |
| `$CANDIDATE/builds/opi-eval report --root /home/luiz/opi-eval-runs/20260911t141317z-tb3-repair/runs/terminal-bench-3.0 --out /mnt/e/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/historical-report-b.json` | 0 | [historical-report-b](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/checks/historical-report-b.json) |

- `$CANDIDATE = /home/luiz/opi-eval-runs/20260912t201159z-tb3-turn-budget-repair`
- Source HEAD: `55fbc95009e6f47145196dbc539454ebcb1cf692`; package `0.8.2`, current dirty working tree.
- Source inventory (873 files): `95b8f2a8988a7a5a9ce36f5a97707a4fb0fafe76cc48b1f9e9997c13e60fa647`.
- Working-tree patch: `5e722541c88c68ffa80f55a20637ad9ee145c04c4cbe8bf014fbf8278618b7d8`.
- Cargo.lock: `261a3ec501be2e045dc6c9b2b9a940adc1c56305f477008eea940f26545925f3`.
- Opi SHA-256: `62ea32cae8f95448cfb502dcd3ea0b96e793c14c366c8b046a4974617c94a955`.
- Companion SHA-256: `3d051747ff1c9c77ba6d872d2699bf4d4a1b2f49f1c410a5dfd4832b0cab06ea`.
- Opi output-control proof: `f3152169f9f3873a041185e1f0e2e84e488569c252f83880e63b6e5cbc902dde`.
- pi output-control proof: `310e6395b153a02b656af258536e598bea6188a03ec0fa2d3bd1aaf2806536a5`.
- Reconstructed historical report SHA-256: `9c8e8eba8f0170d0f62452630115a32f108da14cf14731f50a17e876d4aa1bdf`.

[Candidate identity](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/candidate/candidate-identity.json) · [Evidence index](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/evidence-index.json) · [Cleanup record](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/operator/cleanup.json) · [Worktree scope](E:/opi/evaluations/20260912t201159z-tb3-turn-budget-repair/evidence/operator/final-worktree.json)
