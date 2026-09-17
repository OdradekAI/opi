# pi core-loop alignment repair — 2026-09-13

The approved source repair is implemented in the current worktree. The full workspace gate remains blocked by the live-ledger spec hash after the approved normative revision. Agent Core has no intrinsic turn ceiling, the product defaults to no finite turn budget, and actual Opi and pi clients both complete the same offline 51-tool-turn task plus final response. An explicit Opi budget of two still rejects a third request. This is source-repair verification, not a new TB3 score or paid rerun.

## Authority and scope

The user's “同意” approved the previously presented repair and the paired normative section 7.2 revision under opi-eval's user-confirmed source-repair exception. The [approved plan](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/plan.md) and [baseline](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/baseline.json) retain the authorization and initial worktree inventory. No commit, staging, branch, implementation-ledger update, regrade, real credential resolution, or new paid trial occurred.

`opi-ai` continues to own provider-neutral request preparation and streaming. `opi-agent` owns generic completion, stop, queue and next-turn state transitions. `opi-coding-agent` owns optional execution-budget policy. `opi-eval` omits the previous `u32::MAX` projection workaround. This repair aligns the affected loop semantics with local pi 0.85.1; it is not a claim that every ai/agent feature is already equivalent.

The normative English/Chinese spec now requires stop evaluation on the completed state before queue polling, and preparation only for actual continuation. Historical Phase snapshots and prior evaluation reports remain historical evidence. The pinned local pi ai/agent package manifests report 0.85.1; their source hashes and six reference files are retained in [pi-reference.json](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/pi-reference.json). The reference directory is not an independent Git checkout, so Opi's HEAD is not represented as a pi revision.

## Resulting behavior

- `AgentLoopConfig.max_turns` is removed. `PrepareNextTurnContext.turn` is a per-invocation completed-turn `u64` counter, not a quota.
- After a completed turn: stop decision → steering and applicable follow-up selection → preparation only if continuing → atomic complete-state validation/application → next request. A prepared `Some` value cannot create an extra turn. Tool termination suppresses automatic tool continuation while allowing queued work.
- Preparation errors and cancellation preserve complete-state atomicity and undelivered queued input. The independent review also exposed initial steering loss on pre-dispatch failure; transform, conversion, authentication preparation and cancellation now return that input to its queue. Already-dispatched input is not replayed.
- `defaults.max_iterations: Option<u32>` defaults to `None`. Supplied integers apply per prompt/continuation invocation, with provider retries remaining part of one logical turn. Natural completion exactly at the boundary succeeds; a needed extra turn fails as `MaxTurnsExceeded`; zero fails before a provider request. The counter resets on the next invocation.
- Evidence records an explicit numeric quota when supplied and `Unknown(NotApplicable)` when absent. The diagnostic remains typed and advises adjusting the caller's budget.
- Current Opi projection omits `max_iterations`. Historical sealed projections containing the exact old workaround remain verifiable. Current source selection when Opi version is omitted is preserved. Output limits, context capacity and external deadlines are separate controls.

The product's existing persisted-session compaction remains product-owned; this repair does not relocate that coordinator into the core hooks.

## Candidate and actual-client evidence

Candidate root: `/home/luiz/opi-eval-runs/20260913t034303z-pi-core-loop-alignment-verified`. Source HEAD: `55fbc95009e6f47145196dbc539454ebcb1cf692`, package version 0.8.2 plus the captured worktree changes; this is not the stock 0.8.2 release. Both binaries use the release profile, workspace default features, and `x86_64-unknown-linux-musl`.

Source inventory SHA-256: `93cb48d760f9f508d215b0d5843943a1fac798cac0027bbd7b44d60d0054e79c`. Working-tree patch SHA-256: `82d9ed30d5659fce361982a5d1de10529bcebce1e9245879df53afb38b906701`. The [candidate identity](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/evidence/candidate/candidate-identity.json) also records Cargo.lock, toolchain, build command, source inventory, and immutable executable paths.

| Executable | SHA-256 |
|---|---|
| opi | `c89ac4a103c6cf0f433469ae45dc6baea03328c3937b8c412f67e8f0457ff8de` |
| opi-eval | `bcfc147973de76579013b442860f7470c17b1d00b8233d1b5224975242f81812` |

```text
default: requests=52, exit=Some(0)
explicit-two: requests=2, exit=Some(1)
projected: requests=52, exit=Some(0)
pi-projected: requests=52, exit=Some(0)
omitted-output: requests=0, exit=Some(2)
```

The actual-client fixture used the same loopback provider and synthetic credentials. Opi default and projected configurations and pinned pi all made 52 requests and completed. Opi's explicit two-turn budget stopped at two requests. An Opi custom model with omitted output capability was rejected before dispatch. Every admitted request retained a per-response output limit of 131072 with declared context capacity 1000000. The separate output-control matrix passed for both clients, binding the exact executables and pi runtime package to their observed requests.

## Verification and review

Workspace all-target tests: **4452 passed**, **1 failed** (the spec-ledger hash guard), **12 ignored**; documentation tests: **12 passed**. The 37 offline Python acceptance-verifier tests passed. Both explicitly selected actual-client test binaries/filters passed. Ignored live/client-material tests were not implicitly activated by credentials.

Meaningful pre-fix failures reproduced the default 50-turn stop, old preparation/queue ordering, and lost initial input. Initial fixture compilation errors and an incorrect expected authentication-error variant are retained in the command records but are not counted as semantic reproduction. All final checks below passed except the explicitly recorded live-ledger spec-hash guard. The [core report](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/core-report.md) and [independent review](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/independent-review.md) retain focused commands and finding resolution; no actionable review finding remains.

All commands ran from `/mnt/d/Luiz/Odradek/opi` through the recorded sanitized, leased wrapper with incremental compilation and external Cargo cache. Exact argv, cwd, start/end, exit and logs are retained:

The first workspace run stopped before one test could start: its cached ELF contained 8189 invalid relative-relocation records and failed identically after a byte-identical copy to Linux storage. The separately built focused test passed all three cases. Only that test source timestamp was refreshed to make Cargo relink; source bytes and dependency settings were unchanged and no cache was deleted. The final workspace rerun loaded that test successfully. [Loader diagnosis](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/loader-diagnosis.json) and [relink record](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/startup-test-relink.json) retain this build-artifact incident; its underlying cause is not established.

The next parallel workspace run passed the relinked test, then two unchanged OAuth tests with 100 ms wall-clock deadlines failed while sending their synthetic authorization code. All 122 tests in that binary passed serially with identical assertions. The final complete workspace run used `RUST_TEST_THREADS=1`; no OAuth test or production behavior was changed. Earlier unsuccessful workspace attempts remain in the verification inventory.

A later RPC fixture assumed prompt acceptance meant a request was already dispatched. With initial steering now delivered before dispatch, its immediate steering message could correctly join the first request. The fixture now holds an existing mock stream open, waits for `MessageStart`, sends steering, and then releases the stream. It asserts exactly two requests with steering in the second; all 80 RPC tests passed and the correction received independent review. The final candidate captures this complete tested source; its runtime binaries are byte-identical to the earlier offline candidate, which is retained separately.

| Command | Exit | Record |
|---|---|---|
| `cargo fmt --check --all` | 0 | [workspace-fmt-verified](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/workspace-fmt-verified.json) |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | 0 | [workspace-clippy-verified](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/workspace-clippy-verified.json) |
| `/usr/bin/env RUST_TEST_THREADS=1 cargo test --locked --workspace --all-targets --no-fail-fast` | 101 | [workspace-tests-all-recorded](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/workspace-tests-all-recorded.json) |
| `cargo test --locked --workspace --doc` | 0 | [workspace-doctests-final](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/workspace-doctests-final.json) |
| `/usr/bin/env RUSTDOCFLAGS=-D warnings cargo doc --locked --workspace --no-deps` | 0 | [workspace-doc-final](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/workspace-doc-final.json) |
| `cargo test --locked -p opi-eval --all-targets` | 0 | [eval-all-targets](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/eval-all-targets.json) |
| `python3 crates/opi-eval/scripts/test_verify_local_acceptance.py` | 0 | [python-acceptance](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/python-acceptance.json) |
| `/usr/bin/env OPI_EVAL_NATIVE_CLIENT_MATERIAL=/home/luiz/opi-eval-runs/20260913t034303z-pi-core-loop-alignment-verified/clients.json cargo test --locked -p opi-eval --test native_driver native_client_turn_budgets -- --ignored --exact --nocapture` | 0 | [candidate-turn-budget-verified](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/candidate-turn-budget-verified.json) |
| `/usr/bin/env OPI_EVAL_NATIVE_CLIENT_MATERIAL=/home/luiz/opi-eval-runs/20260913t034303z-pi-core-loop-alignment-verified/clients.json OPI_EVAL_OUTPUT_CONTROL_OUT=/home/luiz/opi-eval-runs/20260913t034303z-pi-core-loop-alignment-verified/output-control cargo test --locked -p opi-eval --test native_driver native_output_control_client_matrix -- --ignored --exact` | 0 | [candidate-output-control-verified](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/candidate-output-control-verified.json) |
| `/usr/bin/env CC_x86_64_unknown_linux_musl=gcc cargo build --locked --release --target x86_64-unknown-linux-musl -p opi-coding-agent --bin opi -p opi-eval --bin opi-eval --message-format=json-render-diagnostics` | 0 | [release-verified](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/release-verified.json) |
| `/home/luiz/opi-eval-runs/20260913t034303z-pi-core-loop-alignment-verified/builds/opi-eval verify --root /home/luiz/opi-eval-runs/20260911t141317z-tb3-repair/runs/terminal-bench-3.0` | 0 | [historical-verify-verified](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/historical-verify-verified.json) |
| `/home/luiz/opi-eval-runs/20260913t034303z-pi-core-loop-alignment-verified/builds/opi-eval report --root /home/luiz/opi-eval-runs/20260911t141317z-tb3-repair/runs/terminal-bench-3.0 --out /mnt/e/opi/evaluations/20260913t034303z-pi-core-loop-alignment/historical-report-a-verified.json` | 0 | [historical-report-a-verified](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/historical-report-a-verified.json) |
| `/home/luiz/opi-eval-runs/20260913t034303z-pi-core-loop-alignment-verified/builds/opi-eval report --root /home/luiz/opi-eval-runs/20260911t141317z-tb3-repair/runs/terminal-bench-3.0 --out /mnt/e/opi/evaluations/20260913t034303z-pi-core-loop-alignment/historical-report-b-verified.json` | 0 | [historical-report-b-verified](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/checks/historical-report-b-verified.json) |

The final documentation and diff checks are recorded as `checks/doc-check-final.json` and `checks/diff-check-final.json` in the evidence root; the machine [verification summary](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/verification-summary.json) inventories all attempted commands, including superseded failures.

Test impact: **add** `continuation_contract.rs` and `turn_budget.rs`; **update** affected core/product hook, queue, config, evidence and caller fixtures plus the native client turn-budget fixture; **retain** authority, cancellation, retry, provider failure, session-prefix, consumption and historical-evidence protections. No test file was deleted. `opi-ai` and Cargo dependency declarations were unchanged by this task.

## Historical evidence, cleanup and handoff

The remaining workspace failure is `live_ledger_spec_hash_matches_current_spec`. The ledger records `cc7f8898f60c0d8abaa667f4b49b7affc721412e75dd3a67dcde37a783e1bc4c`, while the approved current English spec normalizes to `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db`. [AGENTS.md](D:/Luiz/Odradek/opi/AGENTS.md:103) states “`opi-implement` alone owns `.opi-impl-state.json`”; line 107 prohibits hand editing it. The [one-field proposed patch](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/ledger-hash-proposal.patch) is concrete and remains unapplied. It changes only `spec_files_sha256["docs/opi-spec.md"]`, with no Phase/task/progress changes. The owning workflow must apply this synchronization and rerun the ledger test and final gate; this repair does not bypass or weaken the guard.

The five protected historical evidence indexes and all 3365 indexed files verified unchanged before and after the repair. The new Companion verified the prior TB3 root and reconstructed its report twice to separate paths with byte-identical results; SHA-256 remains `9c8e8eba8f0170d0f62452630115a32f108da14cf14731f50a17e876d4aa1bdf`. No prior trial was altered or regraded.

Private repair evidence: `E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/evidence`; its `evidence-index.json` binds the candidate, source, upstream reference, command records and paired reports. It contains local paths and synthetic fixture credentials and is not declared a public export. The separately retained index identity is `E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/final-index-identity.json`.

Test-owned temporary directories and loopback children were released by fixture lifecycles; command leases were ended. The new candidate, shared Cargo cache and all historical material remain. Docker was unavailable in WSL, so shared image/container/volume/network cleanup remains unverified; this operation made no Docker mutations. The [cleanup record](E:/opi/evaluations/20260913t034303z-pi-core-loop-alignment/cleanup.json) distinguishes completed owned scratch from that inventory gap.

The candidate is retained; synchronize the live-ledger spec hash through its owning workflow before claiming a clean workspace gate. Any future live run needs new material/control-proof/plan identities and authorization for that exact scope. Offline completion does not establish a new TB3 outcome or performance comparison.
