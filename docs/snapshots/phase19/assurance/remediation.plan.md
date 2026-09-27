# Phase 19 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `40c48da48a5192a8cfce2b32ce80dc91bf33e5435442e9facff169c0f4ec7b00`
**Remediation head**: `5bee4f28738412c6ae3e326e5bb3cf60e57230c2`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged=[]; unstaged=[]; untracked=[]; committed=none-for-this-task.
**Unresolved decisions**: none

## Admission and evidence boundary

The current schema-2 index (revision 6, aggregate FAIL) passed `audit-set` validation before any semantic finding input. Its exact raw-byte digest above binds both indexed members. The current scope projection reports no stored-source hash mismatches. The 15 source keys remain distinct, including the repeated textual IDs; each has exactly one disposition.

| Alias | Current audit run | Findings SHA-256 | Audit head |
|---|---|---|---|
| C (audit.codex.gpt6) | `phase19-codex-gpt6-e079780-20260926t093335z` | `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` | `e079780bda6681ec466f79bd15f3248baf0f2eb8` |
| P (audit.pi.glm53) | `phase19-pi-glm53-e079780-20260926t192024z` | `f2e7acaffe724a0fa6dbf28ca767b163a3cd664a905a17e6b17924d1fedd582c` | `e079780bda6681ec466f79bd15f3248baf0f2eb8` |

Source inspection and all behavioral probes used unique temporary `git archive` exports of the remediation head. No production edit, provider call, real credential read, or Git worktree was used. No historical audit, earlier remediation conclusion, or external evaluation artifact was consumed. Source finding evidence remains immutable in its indexed sidecar; fresh probes below independently establish only their named claims.

The coordinating export is `C:\Users\Luiz\AppData\Local\Temp\opi-remediate-p19-gkm1aqyc\source`. Reusable probe files are retained outside the repository with hashes in the evidence manifest below. A probe path is diagnostic evidence, not an alternate source or plan. Before an eventual repair, verify its hash and reconstruct the same isolated test if the temporary file is unavailable; re-establish its named red outcome before a production edit.

External incremental Cargo caches were retained. WSL checks explicitly selected the existing `/mnt/e/opi/cargo-targets/wsl-direct` and sanitized execution with `/usr/bin/env -i`, a known PATH, and offline mode. Parallel exports can replace same-named test executables in a shared target; counted evidence requires the intended test name and nonzero execution count, with full Cargo invocations serialized by `flock /tmp/opi-p19-remediate-cargo.lock` or a uniquely named temporary integration-test binary. Empty/zero-selected exploratory commands are not passes.

Plan-only commits may later store this exact pair without changing the source baseline. Any other intervening source change or active-index change requires a new plan under the source-baseline rule. Apply still requires a new explicit invocation containing the validator-emitted digest of this exact pair.

## Current Finding Verification

| Source run / Finding ID | Verification | Source → final severity; rationale | Closure key / family | Batch | Decision |
|---|---|---|---|---|---|
| C / P19-AUD-001 | Confirmed | Major → Major; Retain Major: admitted file credentials cannot reach the selected Opi process. | `connection.opi-file-credential-projection` / `native-connection-projection` | B1 | `fix:project-opi-file-credential` |
| C / P19-AUD-002 | Confirmed | Major → Major; Two current deterministic subprocess probes reproduce serial dispatch after cleanup uncertainty and identity rejection; both are safety/provenance failures covered by registered source section 9. | `native-lifecycle.stop-after-boundary-breach` / `native-lifecycle.ownership` | B2 | `fix:stop-dispatch-after-native-boundary-breach` |
| C / P19-AUD-003 | Confirmed | Major → Major; Retain Major: a linked sealed trial is silently omitted from successful byte verification. | `offline-verify.trial-link-rejection` / `offline-evidence-integrity` | B3 | `fix:reject-linked-trial-entry` |
| C / P19-AUD-004 | Confirmed | Major → Major; Retain Major: valid single-product native evidence cannot be regraded with its own selected material. | `native-regrade.selected-product-material` / `offline-native-regrade` | B4 | `fix:load-selected-regrade-products` |
| C / P19-AUD-005 | Confirmed | Major → Major; Retain Major: public full-run coverage reports the wrong selected-task denominator. | `report.full-selection-task-count` / `offline-report-coverage` | B5 | `fix:count-frozen-full-selection` |
| C / P19-AUD-006 | Cannot confirm | Major → Major; Retain source Major: the exact current sanitized check passes, but no full final-workspace gate or historical-failure refutation is claimed. | `verification.final-workspace-gates` / `verification.phase-exit` | — | `no-action:cannot-confirm` |
| C / P19-AUD-007 | Confirmed | Major → Major; Retain Major: the required actual-client public run-to-report acceptance path is missing. | `acceptance.actual-client-provider-outcome-run-report` / `acceptance.native-provider-outcomes` | B6 | `fix:actual-client-run-report-matrix` |
| C / P19-AUD-008 | Partially confirmed | Major → Major; Current reproduction proves loader-shaped evidence is scored as Agent failure. The exact old GNU binary/container observation is outside current input boundary and was not reread. | `native-settlement.loader-failure-unscored` / `native-settlement.outcome-classification` | B11 | `fix:bind-native-runtime-preflight` |
| C / P19-AUD-009 | Confirmed | Major → Major; Retain Major: the checker rejects the correct native Opi API representation in first-connection evidence. | `acceptance.native-api-attribution` / `acceptance.native-evidence` | B7 | `fix:interpret-product-native-api` |
| C / P19-AUD-010 | Confirmed | Major → Major; Current complete >1MiB streams lose their suffix during native reading, invalidating pi and offline live evidence. Source live measurements are not reused. | `native-evidence.complete-within-artifact-bound` / `native-evidence.capture` | B8 | `fix:retain-native-stream-within-existing-artifact-bound` |
| P / P19-AUD-001 | Partially confirmed | Major → Major; Retain Major: mandatory six-trial real acceptance remains an external verified-evidence obligation under the registered source. | `acceptance.six-real-native-trials` / `acceptance.native-evidence` | — | `no-action:deferred-by-registered-source` |
| P / P19-AUD-002 | Partially confirmed | Major → Major; Current complete >1MiB streams lose their suffix during native reading, invalidating pi and offline live evidence. Source live measurements are not reused. | `native-evidence.complete-within-artifact-bound` / `native-evidence.capture` | B8 | `fix:retain-native-stream-within-existing-artifact-bound` |
| P / P19-AUD-003 | Confirmed | Minor → Minor; Retain Minor: effective existing tests pass, but ledger-declared filters select zero tests. | `verification.ledger-selectors-nonvacuous` / `verification.acceptance-selectors` | — | `no-action:returned-to-shaping` |
| P / P19-AUD-004 | Partially confirmed | Minor → Minor; Retain Minor: invalid witness input has an opaque test-helper panic; historical duration/log claims are not re-established. | `verification.native-witness-material-diagnostics` / `verification.native-witness-inputs` | B9 | `fix:witness-material-preflight-diagnostics` |
| P / P19-AUD-005 | Partially confirmed | Minor → Minor; Retain advisory Minor from source. Current stand-in subprocess proves the diagnostic witness omits owned-container cleanup; exact earlier operator Docker instances and manual cleanup remain unconfirmed and were not accessed. | `native-witness.cleanup-owned-task-resources` / `native-lifecycle.ownership` | B10 | `fix:apply-owned-cleanup-to-native-witness` |

### Fresh evidence and counter-evidence

**C / P19-AUD-001 — Opi 丢弃 credential_file 解析出的凭据**

- `crates/opi-eval/src/experiment.rs:1135` — Opi adds an environment credential only when variable() is Some.
- `crates/opi-eval/src/experiment.rs:1263` — render_opi_config uses OPI_EVAL_AGENT_KEY for file-backed references.
- `crates/opi-eval/src/experiment.rs:1561` — FromFile returns None from variable().
- `crates/opi-eval/src/runner/experiment.rs:1249` — The actual launcher resolves declared file contents into FromFile, so the branch is admitted and reachable.
- `crates/opi-eval/src/runner/experiment.rs:1575` — Native handoff forwards exactly projection.agent_env; no later repair fills in the missing file credential.
- `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md:132` — Selected environment references and private local files are both valid credential sources; resolution is Agent-process scoped.
- Limit: Pi intentionally carries the resolved value in isolated models.json (experiment.rs:1141,1337) and leaves agent_env empty. Env-backed Opi works; existing tests at experiment.rs:1861 cover Env only. Native handoff identity explicitly strips agent_env at runner/experiment.rs:1586, retaining the required privacy boundary.
- Limit: No README contract change required: English/Chinese already admit credential_file. Record behavior correction in Unreleased.

**C / P19-AUD-002 — 完整性或清理失败后仍继续原生 trial 调度**

- `crates/opi-eval/src/runner/experiment.rs:769` — Dispatch checks only run_cancel; a failed receipt alone never cancels.
- `crates/opi-eval/src/runner/experiment.rs:2012` — Cleanup failure sets Infrastructure ledger failure but not run_cancel.
- `crates/opi-eval/src/runner/experiment.rs:2300` — Only canary leak cancels here; absent binding refuses seal at 2311 without stopping subsequent dispatch.
- `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md:342` — On budget, authority, provenance, or privacy breach stop new affected trials and cancel only owned execution.

**C / P19-AUD-003 — verify 将符号链接 trial 静默排除后仍返回成功**

- `crates/opi-eval/src/regrade.rs:117` — Trial enumeration silently skips every non-directory DirEntry; a symlink's is_dir() is false.
- `crates/opi-eval/src/regrade.rs:86` — Only the filtered IDs reach RunBundle::verify.
- `crates/opi-eval/src/regrade.rs:97` — Empty failures produces outcome=verified even when a linked trial was omitted.
- `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md:262` — P19-AC-009 requires offline byte verification, tamper rejection, and immutable source evidence.
- Limit: Report already includes directory/symlink entries and rejects linked ancestors through checked_input_path (report.rs:279-308,603-626). Bundle verification rejects links inside its artifact tree, but it is never invoked on the omitted trial. Neither path rescues verify enumeration. NativeRegrader also uses read_trial_ids at regrade.rs:298.
- Limit: No README contract change required: both counterparts already promise every sealed bundle is checked and tamper rejected. Record bug fix in Unreleased.

**C / P19-AUD-004 — pi-only 原生运行无法使用自己的材料重评分**

- `crates/opi-eval/src/regrade.rs:312` — Native regrade calls NativeMaterial::load before reading sealed trial product requirements.
- `crates/opi-eval/src/runner/material.rs:257` — load hardcodes load_for(path, &[opi, pi]).
- `crates/opi-eval/src/runner/material.rs:305` — load_for correctly requires an exact agent identity set, so pi-only material is rejected by the wrong caller selection.
- `crates/opi-eval/src/regrade.rs:470` — Per-trial regrade already resolves the product from sealed trial/subject evidence and selects material.agent(product); the required product source exists.
- `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md:70` — Independent Companion pi-only evaluation has complete value without an Opi executable.
- `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md:236` — Actual offline regrade consumes preserved Agent artifacts with the pinned native grader and emits new derived records.
- Limit: Run/native_integrity_identity already uses NativeMaterial::load_for with actual selected products (runner/experiment.rs:1282-1288). The native driver has paired regrade tests but its pi-only test at native_driver.rs:8240 covers hermetic run/report, not pi-only native regrade.
- Limit: No contract revision required: current bilingual README already describes selected subjects and verifier-only regrading. Add explanation only if implementation changes documented rejection classification; otherwise Unreleased suffices.

**C / P19-AUD-005 — full 选择的离线报告将 selected 计为零**

- `crates/opi-eval/src/runner/plan.rs:296` — Full selection expands to every canonical corpus task ID.
- `crates/opi-eval/src/runner/experiment.rs:582` — The expanded trials replace original declared trials before run-control generation.
- `crates/opi-eval/src/runner/experiment.rs:702` — Frozen run-control already records corpus_tasks and expanded planned_trials, while selection.task_ids preserves original request syntax.
- `crates/opi-eval/src/report.rs:841` — Report selected count always uses original selection.task_ids length, including legal full selections with no explicit IDs.
- `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md:226` — Reports must show corpus size, selected tasks and separate planned/attempted/valid counts; full and subset meanings stay honest.
- Limit: The producer and plan already correctly expand full scope and preserve all trial/task identities (runner/plan.rs:295-312; runner/experiment.rs:708-717). Existing subset test native_driver.rs:6095 asserts selected=1 but its full-dispatch counterpart at :6983 does not assert report selected count. This is a report interpretation defect, not a need for a new durable format.
- Limit: No README contract change needed: both already distinguish corpus/selected/trial counts and full/subset. Record correction in Unreleased.

**C / P19-AUD-006 — 最终工作区测试门禁未通过**

- At current head the exact WSL/Linux bash_tool_env_inheritance_reporting test passes: 1 passed, 94 filtered out, 0.05s; command below used absolute /usr/bin/env with a clean PATH and the existing shared external Cargo target.
- Local command resolution showed bare env resolves to a host PATH shim. Commands initially attempted with bare env did not execute Cargo and are explicitly not evidence; no shim contents or external eval artifacts were read.
- Current tests/tools_read_write_edit_bash.rs:2104 executes env through normal BashTool; src/tool/operations.rs:908 build_bash_command uses envs without clearing inherited environment. No source defect is demonstrated by the clean-PATH exact run.
- The complete final workspace gate was not rerun at plan stage. The earlier audit failure is not refuted.
- Current check: `/usr/bin/env -i HOME=/home/luiz PATH=/home/luiz/.cargo/bin:/usr/bin:/bin SHELL=/bin/bash CARGO_NET_OFFLINE=true CARGO_INCREMENTAL=1 CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/wsl-direct cargo test --offline --locked -p opi-coding-agent --test tools_read_write_edit_bash bash_tool_env_inheritance_reporting -- --exact --nocapture` → PASS: 1 passed, 0 failed, 94 filtered out; test finished 0.05s
- Limit: No full-gate claim or historical-failure refutation.

**C / P19-AUD-007 — 真实客户端失败矩阵尚未贯通公开 run-to-report 验收链**

- Scope reader project_task on committed export task 19.7 P19-AC-008 explicitly requires the same public run-to-report path with actual Opi/pi clients and local fixture provider for final provider failure, native zero and transient 429 success.
- native_driver.rs:2748 native_protocol_client_matrix runs actual clients directly via run_opi_client/run_pi_client, then inspects exit/messages/provider requests, never public run/report.
- native_driver.rs:5719 native_provider_failure_report runs public run/report but obtains material from stage("provider-error") and stage("zero"); stage at :669 constructs stand-in agents/integrations/runner.
- native_driver.rs:6256 remediation_004_native_provider_error_is_not_scored adds broader family/capture/terminal coverage but still uses stage and rewrites synthetic terminal artifacts. It is counterevidence to missing classification coverage, not the required actual-client assembled path.
- Limit: Exact local client material must be prepared for the new opt-in check; no old eval material was consulted and no paid/live provider was invoked.
- Limit: English/Chinese synchronization unnecessary for test-only edits.

**C / P19-AUD-008 — Opi 加载器失败被计为有效原生零分**

- `crates/opi-eval/src/runner/experiment.rs:1653` — Opi provider classifier recognizes exits 3/4; any other nonzero becomes agent-non-zero-exit.
- `crates/opi-eval/src/runner/experiment.rs:1453` — No native exception or NonZeroAgentExitCodeError yields graded-native after authority check.
- `crates/opi-eval/src/agent/process.rs:157` — agent-non-zero-exit and agent-timeout are ScoredAgentOutcome.
- `crates/opi-eval/integrations/harbor/opi_eval_harbor.py:470` — Every nonzero launch result raises NonZeroAgentExitCodeError; equivalent Pier path at 509.
- `crates/opi-eval/src/benchmark/process.rs:1010` — Native terminal /1 parses product, executable digest, exit_code, timed_out and delivered only; no trusted started/not-started observation.
- `crates/opi-eval/tests/native_driver.rs:1556` — Valid agent-nonzero counterexample must remain graded-native and agent_failed=true.
- `docs/opi-spec.md:305` — CTRL-005 prohibits scoring invalid/infrastructure-failed trials as Agent success or failure.
- `crates/opi-coding-agent/src/runner.rs:399` — run_json accumulates header and events into Arc<Mutex<String>> and awaits harness.prompt at 498 before returning captured stdout at 512-527; AgentStart is not an independently flushed startup witness.
- `crates/opi-coding-agent/src/main.rs:961` — The CLI writes result.stdout only after awaited runner operation finishes; a post-start crash/abort can therefore leave no stdout.
- `crates/opi-eval/src/agent/process.rs:134` — Contract explicitly includes the Agent own crash as a scored Agent outcome, so empty stdout cannot alone distinguish loader refusal from started crash.
- `fresh plan-stage loader probe` — CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/wsl-direct flock /tmp/opi-p19-remediate-cargo.lock cargo test --offline -p opi-eval --test native_driver plan_p19_loader_failure_is_unscored -- --exact --nocapture --test-threads=1 -> Named test ran 1 failed, exit 101; classification was {agent_failed:true,class:graded-native,reward:0,timed_out:false}, expected invalid-execution. Diagnostic-shaped evidence establishes the existing defect, not proof of the selected new contract.
- `crates/opi-eval/integrations/harbor/opi_eval_harbor.py:331` — setup verifies and uploads the exact runtime, then projects configuration at 367; this is the existing boundary for a pre-configuration compatibility probe. Pier has the same integration-owned boundary.
- `crates/opi-eval/integrations/harbor/opi_eval_harbor.py:101` — OwnedDockerEnvironment.exec already supervises execution and bounds group termination; equivalent Pier implementation supplies the 30-second preflight transport.
- `crates/opi-coding-agent/src/main.rs:14` — Clap parses --version before normal command/provider construction. The pinned pi materializer already executes its exact Node/bundled CLI --version in an isolated environment (materialize-external-locks.sh:223-232); the staged pi wrapper selects that package runtime.
- `crates/opi-eval/src/runner/experiment.rs:1563` — Current handoff/3 carries logical trial, product, executable SHA and optional runtime package; bind these to independently retained NativeSelection expectations rather than trusting echoed proof.
- `C:\Users\Luiz\AppData\Local\Temp\opi-remediate-p19-gkm1aqyc\startup-contract-red.log` — Fresh exact-baseline offline Python probe ran 2 test methods and 8 Harbor/Pier x Opi/pi subcases, all FAIL, exit 1. Refusal cases launched one task instead of zero; passed-preflight/nonzero-task cases observed no preflight. No shell, container, provider or real credential was used.
- Limit: No old evaluation artifact or exact GNU binary/image was opened; current evidence is a bounded stand-in native job. Fresh separately authorized evidence is still required for actual-client/official-image acceptance.
- Limit: The probe proves missing integration behavior with controlled transport results. It does not claim actual image compatibility or real-provider acceptance; a passed version probe does not prove later task startup or all lazily loaded dependencies.

**C / P19-AUD-009 — 离线验收器把 Opi 原生 API 枚举误当作 pi 协议字符串**

- `crates/opi-ai/src/lib.rs:62` — ApiKind derives Serialize without variant renaming; the Anthropic variant is serialized as Anthropic. No Opi schema change is needed.
- `crates/opi-eval/scripts/verify-local-acceptance.py:439` — Both products compare message.api directly with FIRST_CONNECTION[api]=anthropic-messages, rejecting the Opi native enum.
- `crates/opi-eval/scripts/test_verify_local_acceptance.py:190` — Existing synthetic Opi fixture copies the pi connection API string, hiding the production-shaped mismatch.
- `C:\Users\Luiz\AppData\Local\Temp\opi-remediate-p19-gkm1aqyc\native-api-red.log` — Fresh offline probe: 7 cases, correct Opi Anthropic rejected, 6 controls passed (pi positive and wrong/unknown API, provider, model negatives). Synthetic native-shaped events; not real-provider acceptance.

**C / P19-AUD-010 — 固定 1 MiB 封存上限使真实客户端循环无法完整进入验收材料**

- `crates/opi-eval/src/runner/experiment.rs:1715` — Native reader hard-codes CAP=1024*1024 and reads CAP+1 then truncates to CAP at 1750-1753.
- `crates/opi-eval/src/runner/experiment.rs:2077` — Full policy stages only capture.bytes, already truncated by native reader.
- `crates/opi-eval/src/agent/pi.rs:320` — Native interpretation calls PiImporter directly; it does not create a profile SpawnSpec.
- `crates/opi-eval/src/agent/pi.rs:395` — Cap-truncated pi input is rejected as import-evidence-incomplete.
- `crates/opi-eval/src/agent/pi.rs:224` — Profile caps apply to host process SpawnSpec. Those caps are not the native post-job read cap.
- `crates/opi-eval/scripts/verify-local-acceptance.py:418` — Acceptance requires non-truncated sealed logs plus terminal and tool events.
- `crates/opi-eval/src/experiment.rs:436` — Full is every capturable evidence stream.
- `crates/opi-eval/src/bundle/mod.rs:1164` — Existing MAX_ARTIFACT_BYTES=64MiB bounds one artifact; insert explicitly rejects oversize at 1202 rather than truncating.
- Limit: Probe uses a valid pi fixture padded with blank NDJSON lines and bounded fake native jobs. It proves loss/interpretation mechanically, not the six real task/client acceptance criteria. Fresh authorized real evidence remains separate.
- Limit: The indexed pi finding is a separate source identity despite title similarity. Its truncated-native behavior shares this proven root/predicate/red pair with codex010; its causal claim that embedded profile caps produced these native files is refuted by current native call path. Do not edit profiles based on this claim.

**P / P19-AUD-001 — Six-trial local acceptance evidence is rejected by the offline verifier**

- `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md:292` — Section 8 reserves real acceptance to a separately authorized local operator run. Until real evidence is present and verified, P19-AC-012 remains not met; credential presence/source registration gives no spending authority.
- `crates/opi-eval/scripts/verify-local-acceptance.py:418` — The offline acceptance predicate rejects truncation or terminal/model mismatch; fresh API probe independently confirms an additional rejection on valid Opi native-shaped API events.
- `current indexed pi findings P19-AUD-001` — The source reports rejection of supplied six-trial evidence. Those external eval artifacts were not admitted or read in this plan; original saved-run counts and no-replacement claims are not independently re-established.

**P / P19-AUD-002 — Pinned 1 MiB agent stdout capture ceiling invalidates real trials on TB3/DeepSWE**

- `crates/opi-eval/src/runner/experiment.rs:1715` — Native reader hard-codes CAP=1024*1024 and reads CAP+1 then truncates to CAP at 1750-1753.
- `crates/opi-eval/src/runner/experiment.rs:2077` — Full policy stages only capture.bytes, already truncated by native reader.
- `crates/opi-eval/src/agent/pi.rs:320` — Native interpretation calls PiImporter directly; it does not create a profile SpawnSpec.
- `crates/opi-eval/src/agent/pi.rs:395` — Cap-truncated pi input is rejected as import-evidence-incomplete.
- `crates/opi-eval/src/agent/pi.rs:224` — Profile caps apply to host process SpawnSpec. Those caps are not the native post-job read cap.
- `crates/opi-eval/scripts/verify-local-acceptance.py:418` — Acceptance requires non-truncated sealed logs plus terminal and tool events.
- `crates/opi-eval/src/experiment.rs:436` — Full is every capturable evidence stream.
- `crates/opi-eval/src/bundle/mod.rs:1164` — Existing MAX_ARTIFACT_BYTES=64MiB bounds one artifact; insert explicitly rejects oversize at 1202 rather than truncating.
- Limit: Probe uses a valid pi fixture padded with blank NDJSON lines and bounded fake native jobs. It proves loss/interpretation mechanically, not the six real task/client acceptance criteria. Fresh authorized real evidence remains separate.
- Limit: The indexed pi finding is a separate source identity despite title similarity. Its truncated-native behavior shares this proven root/predicate/red pair with codex010; its causal claim that embedded profile caps produced these native files is refuted by current native call path. Do not edit profiles based on this claim.

**P / P19-AUD-003 — Ledger-declared verification filters drifted to vacuous commands**

- Committed scope projection task 19.6 P19-AC-001-plan declares assembled_run independent_plan_run; rg finds no such function. Exact Cargo selector executes 0 tests, 15 filtered out.
- Committed scope projection task 19.8 P19-AC-009-regrade declares native_offline_regrade with --ignored --exact. The function at native_driver.rs:5839 has #[test] and no #[ignore]; exact ignored selector executes 0 tests.
- Existing assembled_run.rs:1424 frozen_subset_execution covers independent-directory public plan/run and frozen cardinality; :242 hermetic_paired_group_runs_end_to_end covers paired execution. Native regrade actual test remains present.
- AGENTS.md Skills and workflow assigns ledger writes to opi-implement; opi-implement/SKILL.md explicitly prohibits edits to frozen snapshots. This invocation authorizes neither ledger ownership transfer nor snapshot rewriting.
- Corrected existing selectors verified in this export under shared flock and clean offline WSL environment: frozen_subset_execution PASS 1 executed (67.77s); hermetic_paired_group_runs_end_to_end PASS 1 executed (0.54s); native_offline_regrade without --ignored PASS 1 executed (5.02s). Existing behavior is covered; correction belongs to metadata ownership, not redundant implementation tests.
- Limit: Remediation has no authority to edit the ledger or frozen snapshot; the exclusion is explicit, not an ownership transfer.

**P / P19-AUD-004 — Witness-material helper panics opaquely on single-family material; two long ignored tests lack saved run evidence**

- support/native_witness_material.rs:91-106 iterates all three required families then unwraps material[benchmarks][family][tasks].as_object(), the first task and profile path. Missing TB3 after a valid TB2.1 family panics at :97.
- A scratch-only probe in the unique export uses stage("ok"), removes TB3 and DeepSWE entries, calls controlled_material(false) under catch_unwind, and asserts the diagnostic names terminal-bench-3.0. No Docker operations, models, credentials, network listener, or historical material are used.
- After flock serialization and touching the scratch test to ensure correct shared-cache binary, cargo executed 1 probe and failed: Option::unwrap at support/native_witness_material.rs:97, followed by named-family diagnostic assertion failure.
- Current native_witness_material::run_cli saves each command stdout/stderr/metadata JSON, while both witnesses retain temp roots and announce them. Source thus already supports saved evidence. The specific duration and absence-of-historical-log claim was not independently verified; old eval artifacts are excluded.
- Limit: No new claim about completion/duration of the two Docker witnesses or historical saved logs; no new retention subsystem is warranted.
- Limit: English/Chinese synchronization unnecessary for private test-helper/test edits.

**P / P19-AUD-005 — Failed native witness runs orphaned task environment containers**

- `crates/opi-eval/src/cli/conformance.rs:1447` — Diagnostic witness finishes by directly running BenchmarkExecution; on observed record it immediately builds report at 1481 without owned-resource cleanup.
- `crates/opi-eval/src/benchmark/process.rs:1394` — BenchmarkExecution supervises runner process tree and imports outcome; no container enumeration/removal occurs before returning at 1439.
- `crates/opi-eval/src/runner/experiment.rs:1966` — The ordinary native run invokes owned cleanup itself, so its good cleanup tests do not prove conformance-witness coverage.
- `crates/opi-eval/src/runner/experiment.rs:3656` — Current ordinary-run cleanup discovers exact com.docker.compose.project.working_dir labels; the source claim that it requires an opi-eval label is not true at remediation head.
- `crates/opi-eval/tests/native_driver.rs:1136` — Positive control native_settlement_cleans_root_bound_containers preserves neighbors and removes exact owned resources even when setup fails.
- `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md:342` — Cleanup is limited to identified trial-owned resources; neighboring/shared resources are not targets.
- Limit: No Docker workload ran and no old E: evaluation artifact was opened. The exact historic operator leaks remain unconfirmed; the current diagnostic witness cleanup gap has independent red-before evidence.

## Unresolved Decisions

none

### Resolved D1: user-selected option A — trusted runtime compatibility observation

The user explicitly selected option A in this invocation. B11 adopts an integration-owned `runtime_preflight`, using the existing task-environment execution boundary and exact material identities. This resolves the evidence-contract decision without changing the Agent Core event stream. It detects the cited persistent loader/runtime incompatibility; it does not prove that the later task process started or that every lazily loaded dependency will work.

1. Exact runtime -> verify: after verified runtime staging and before `_project_configuration`, run the staged Opi executable or staged pi wrapper/package with `--version`. The pi wrapper must select its packaged Node and bundled CLI, never host Node or bare `pi`. Use the same task environment, task user/cwd and loader-relevant nonsecret settings as the task command; supply no projected credentials, task prompt, provider call, network fetch or ambient user configuration. Revalidate the staged executable/package identities before task dispatch if intervening operations could alter them. Existing handoff/3 supplies identity; no new public handoff or runtime configuration is needed.
2. Bounded observation -> verify: use `OwnedDockerEnvironment.exec(timeout_sec=30)` within the enclosing lifecycle deadline, with observed/bounded process-group termination. Observe one result. `passed` requires observed exit 0, complete readable bounded output and valid identity. Observed nonzero is `failed`; timeout, transport/termination uncertainty, missing/invalid output or capture overflow is `unknown`. Failed and unknown both prevent task dispatch. Keep the existing stronger uncertainty/cancellation/cleanup behavior; a fabricated exit code is never evidence.
3. Private terminal /2 -> verify: synchronize both pinned writers and the Rust reader on `schema: opi-eval-native-agent-terminal/2`. Retain `product` and `agent_executable_sha256`; add logical `trial`, `runtime_preflight`, and boolean `task_dispatched`; make the task `exit_code` nullable. `runtime_preflight` has `kind: version`, `status: passed|failed|unknown`, its own nullable observed `exit_code`, exact executable SHA-256 and optional runtime package manifest SHA-256/entrypoint, exact command SHA-256, separate stdout/stderr SHA-256 plus completeness/truncation state, and a bounded static reason token for unknown. Failed/unknown preflight requires `task_dispatched=false`, task `exit_code=null`, task `timed_out=false` and no delivered Agent files. Passed preflight alone never supplies a task exit. Malformed or contradictory combinations reject.
4. Bind and preserve -> verify: compare logical trial, product and runtime identity with dispatch-owned `NativeSelection` expectations; validate the fixed version command and hashes against captured bytes. Reuse the existing one-new-job, selected-trial and stale-job guards, rather than accepting self-reported identity as authority. The host-side pinned integration writes proof; task output cannot authorize scoring. Keep preflight captures separate from Agent stdout/stderr under runtime-preflight artifact keys, safe-file checks, bounded capture and privacy/canary rules. Preserve typed proof and bounded diagnostics even when setup failure leaves no reward binding. A missing reward, terminal or Agent process observation must never be fabricated to make that path bind. Terminal-only capture retains the proof and permitted bounded diagnostics; full capture follows B8's existing artifact bound.
5. One settlement owner -> verify: failed/unknown preflight is attempted but invalid/unscored; if effects or cleanup are uncertain, retain the stronger existing no-outcome/failed-seal handling. Do not manufacture an AgentRecord with the preflight's exit. Bound passed preflight permits ordinary task processing: real nonzero/crash and task-timeout controls retain their current semantics, with required pi terminal evidence still enforced. Provider failures, unobserved task transport/termination, missing required native evidence and failed binding stay invalid. Propagate proof into sealed normalized `native/agent-terminal.json` where that observation exists and into settlement evidence; offline report reads settlement and does not add a stderr/empty-stream classifier.
6. Reader and version boundary -> verify: fresh native imports and the local acceptance checker require /2 proof; /1 or absent proof cannot be silently upgraded. Existing sealed bundle byte verification, offline reporting and verifier-only regrade retain their original versioned meaning, never invent startup proof and do not certify new live acceptance. Regrade does not rerun the Agent or its preflight. Update /2 fixture writers/readers together, including the DeepSWE selection constructor; retain explicit legacy rejection cases. Document the bounded 0.x private contract change in both READMEs and Unreleased. No public Agent SDK, manifest/dependency, implementation-ledger or registered-spec edit is admitted.
7. Discrimination -> verify: the same recorded Python probe must turn green for Harbor/Pier x Opi/pi, then new `native_runtime_preflight` Rust cases exercise sealed classification and public report with failed/unknown/passed proof, timeout and uncertain transport, identity/trial/command/log substitution, stale proof, missing /1 proof and contradictory task state. Pair nonzero/crash/timeout outcomes after passed proof with invalid provider/evidence controls. Update the original loader-shaped fixture to supply actually observed preflight refusal; diagnostic text alone is not proof. Run full modified Python and Rust suites and B6's actual-client assembled path. Real-provider acceptance remains separately authorized under P / P19-AUD-001.

## Closure Batches

Execute Major batches B1, B2, B3, B4, B5, B7, B8, B11, B6, then Minor B9 and B10. B8 shares a repair only because its two source records have the same native capture predicate; their identities and the incorrect profile-root attribution stay separate. Apply shared-file edits sequentially. The initial clean baseline still applies to this continued planning invocation; only its own fixed plan pair was revised after the user selected D1 option A.

### Batch B1: Project file credentials to the selected Opi process

**Closure predicate**: A matching File/FromFile projection for Opi carries the resolved value in agent_env[config.api_key_env], uses OPI_EVAL_AGENT_KEY, keeps it out of config/exported evidence, and retained Env/Pi behavior still passes.
**Dependencies**: none
**Verification union**: `cargo test --offline --locked -p opi-eval --all-targets (WSL/Linux; no ignored live tests)`; `cargo clippy --offline --locked -p opi-eval --all-targets -- -D warnings`; `cargo fmt --check --all`; `python scripts/opi-doc-check.py`; `git diff --check`

#### Fix B1.1: Project file credentials to the selected Opi process

- **Finding source(s)**: `phase19-codex-gpt6-e079780-20260926t093335z` + `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` + `P19-AUD-001`
- **Decision**: fix:project-opi-file-credential
- **Verification status**: C/P19-AUD-001: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-eval/src/experiment.rs`, `crates/opi-eval/tests/native_driver.rs`
- **Change kind**: behavioral
- **Change**: Populate Opi agent_env under the same environment name used by its rendered config for both Env and File values (existing OPI_EVAL_AGENT_KEY fallback for File). Keep secret out of config bytes and exportable handoff identity. Add regression coverage for Env/File on both clients, mismatch rejection, and process-only file credential handoff/cleanup using dummy values. No new config key or public API.
- **Closure predicate**: A matching File/FromFile projection for Opi carries the resolved value in agent_env[config.api_key_env], uses OPI_EVAL_AGENT_KEY, keeps it out of config/exported evidence, and retained Env/Pi behavior still passes.
- **Red-before**: `CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/wsl-direct flock /tmp/opi-p19-remediate-cargo.lock cargo test --offline --locked -p opi-eval --test p19_projection_probe p19_plan_probe_file_credential_projection -- --exact --nocapture --test-threads=1` → FAIL: Cargo exit 101; exactly 1 executed test, 1 failed. Config api_key_env=OPI_EVAL_AGENT_KEY, env_names=[], assertion left None versus Some(dummy-file-key).; expected: FAIL: Opi file credential is absent from the environment selected by its config.
- **Green-after**: `CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/wsl-direct flock /tmp/opi-p19-remediate-cargo.lock cargo test --offline --locked -p opi-eval --test p19_projection_probe p19_plan_probe_file_credential_projection -- --exact --nocapture --test-threads=1` → expected PASS: exactly one test; selected environment key contains dummy-file-key and config excludes the value.; observed `not-run`.
- **Test impact**: add: file-backed Opi credential projection and private handoff regression; update: projection matrix to include Env/File for both clients; delete: none; retain: environment credential matching and privacy/cleanup tests

### Batch B2: Stop native dispatch after integrity or cleanup boundary failure

**Closure predicate**: In a serial two-trial native run, cleanup uncertainty or rejected identity in the first attempt settles that attempt and prevents the second stage=start; legitimate Agent failures remain graded and permit the next trial.
**Dependencies**: none
**Verification union**: `cargo test --offline -p opi-eval --test native_driver`; `cargo clippy --offline -p opi-eval --all-targets -- -D warnings`; `cargo fmt --check --all`; `python scripts/opi-doc-check.py`; `git diff --check`

#### Fix B2.1: Stop native dispatch after integrity or cleanup boundary failure

- **Finding source(s)**: `phase19-codex-gpt6-e079780-20260926t093335z` + `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` + `P19-AUD-002`
- **Decision**: fix:stop-dispatch-after-native-boundary-breach
- **Verification status**: C/P19-AUD-002: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/tests/native_driver.rs`
- **Change kind**: behavioral
- **Change**: Propagate cleanup/credential-removal uncertainty and integrity/provenance failure to the existing run-level cancellation owner before the next wave. Preserve the failed attempt receipt and missing/unexecuted planned coverage. Do not cancel merely because an ordinary native task outcome has reward zero or a legitimate Agent nonzero exit.
- **Closure predicate**: In a serial two-trial native run, cleanup uncertainty or rejected identity in the first attempt settles that attempt and prevents the second stage=start; legitimate Agent failures remain graded and permit the next trial.
- **Red-before**: `CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/wsl-direct flock /tmp/opi-p19-remediate-cargo.lock cargo test --offline -p opi-eval --test native_driver plan_p19_cleanup_stops_dispatch -- --exact --nocapture --test-threads=1 ; CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/wsl-direct flock /tmp/opi-p19-remediate-cargo.lock cargo test --offline -p opi-eval --test native_driver plan_p19_identity_stops_dispatch -- --exact --nocapture --test-threads=1` → FAIL: Both named tests ran individually, 1 failed each, Cargo exit 101. Each observed trial-opi-native settled=failed followed by trial-pi-native stage=start; assertion left=2 attempted receipts, right=1.; expected: Both fail because the next trial starts after the boundary breach.
- **Green-after**: `CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/wsl-direct flock /tmp/opi-p19-remediate-cargo.lock cargo test --offline -p opi-eval --test native_driver plan_p19_cleanup_stops_dispatch -- --exact --nocapture --test-threads=1 ; CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/wsl-direct flock /tmp/opi-p19-remediate-cargo.lock cargo test --offline -p opi-eval --test native_driver plan_p19_identity_stops_dispatch -- --exact --nocapture --test-threads=1` → expected PASS each; one attempted receipt, no second dispatch, retained failed settlement and visible unexecuted coverage.; observed `not-run`.
- **Test impact**: add: dispatch-after-cleanup-failure and dispatch-after-identity-rejection cases; update: native_result_binding_rejects_every_substitution and cleanup tests currently expect both trial receipts; adjust fixture isolation/expectations without losing each rejection case; retain: valid native zero/nonzero/timeouts; signal cancellation; unknown termination; recovery evidence

### Batch B3: Reject unsafe entries in the trial inventory

**Closure predicate**: Replacing a sealed trial directory with a symlink to its unchanged original bytes cannot produce verify exit 0/outcome verified; the linked trial is explicitly rejected, no source bytes are rewritten, and ordinary sealed/unsealed cases retain their existing outcomes.
**Dependencies**: none
**Verification union**: `cargo test --offline --locked -p opi-eval --all-targets (WSL/Linux; no ignored live tests)`; `cargo clippy --offline --locked -p opi-eval --all-targets -- -D warnings`; `cargo fmt --check --all`; `python scripts/opi-doc-check.py`; `git diff --check`; `cargo test --offline --locked -p opi-eval --test bundle_recompute (included in all-targets; run on WSL/Linux so symlink cases execute)`; `Windows platform-specific link/junction regression if reparse detection is introduced; use existing non-privileged junction fixture style`

#### Fix B3.1: Reject unsafe entries in the trial inventory

- **Finding source(s)**: `phase19-codex-gpt6-e079780-20260926t093335z` + `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` + `P19-AUD-003`
- **Decision**: fix:reject-linked-trial-entry
- **Verification status**: C/P19-AUD-003: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-eval/src/regrade.rs`, `crates/opi-eval/tests/bundle_recompute.rs`
- **Change kind**: behavioral
- **Change**: Make trial enumeration fail closed on linked trial entries instead of silently filtering them. Inspect link metadata (including Windows reparse points) before following descendants; a symlink/junction substitution must cause a documented existing nonzero verify outcome. Keep ordinary unsealed directories visible. Exercise the shared native-regrade enumeration path without broadening into a new inventory format. Retain read-only behavior.
- **Closure predicate**: Replacing a sealed trial directory with a symlink to its unchanged original bytes cannot produce verify exit 0/outcome verified; the linked trial is explicitly rejected, no source bytes are rewritten, and ordinary sealed/unsealed cases retain their existing outcomes.
- **Red-before**: `CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/wsl-direct flock /tmp/opi-p19-remediate-cargo.lock cargo test --offline --locked -p opi-eval --test p19_projection_probe p19_plan_probe_verify_trial_symlink -- --exact --nocapture --test-threads=1` → FAIL: Cargo exit 101; exactly 1 executed test, 1 failed. Before substitution verify has 2 bundles/exit 0. Afterwards verify exit 0, bundles contains only trial-pi-native, failures=[], outcome=verified.; expected: FAIL: replacing one of two sealed trial directories with a link still succeeds.
- **Green-after**: `CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/wsl-direct flock /tmp/opi-p19-remediate-cargo.lock cargo test --offline --locked -p opi-eval --test p19_projection_probe p19_plan_probe_verify_trial_symlink -- --exact --nocapture --test-threads=1` → expected PASS: exactly one test; whole-trial link substitution has nonzero verify exit, while pre-substitution 2-bundle verification succeeds.; observed `not-run`.
- **Test impact**: add: whole-trial symlink substitution regression, Windows junction variant for corresponding metadata check; update: none; delete: none; retain: sealed artifact tamper tests, unsealed bundle verification, report input containment

### Batch B4: Regrade only the products required by the source trials

**Closure predicate**: A valid sealed pi-only native run regrades successfully with its pi-only material, producing exactly its derived trial record with no Opi dependency or Agent/provider invocation; corresponding selected-product identity/material drift remains rejected.
**Dependencies**: none
**Verification union**: `cargo test --offline --locked -p opi-eval --all-targets (WSL/Linux; no ignored live tests)`; `cargo clippy --offline --locked -p opi-eval --all-targets -- -D warnings`; `cargo fmt --check --all`; `python scripts/opi-doc-check.py`; `git diff --check`

#### Fix B4.1: Regrade only the products required by the source trials

- **Finding source(s)**: `phase19-codex-gpt6-e079780-20260926t093335z` + `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` + `P19-AUD-004`
- **Decision**: fix:load-selected-regrade-products
- **Verification status**: C/P19-AUD-004: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-eval/src/regrade.rs`, `crates/opi-eval/tests/native_driver.rs`
- **Change kind**: behavioral
- **Change**: Derive the exact required product set from verified sealed source trial/subject identities and call existing NativeMaterial::load_for with that set instead of the two-product default. Preserve per-trial tamper/identity rejection, verifier-only execution, missing selected material rejection, source immutability and cleanup semantics. Add pi-only and Opi-only regrade cases with no unselected executable/material, keep paired success and substitution negatives. No material schema or new public seam.
- **Closure predicate**: A valid sealed pi-only native run regrades successfully with its pi-only material, producing exactly its derived trial record with no Opi dependency or Agent/provider invocation; corresponding selected-product identity/material drift remains rejected.
- **Red-before**: `CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/wsl-direct flock /tmp/opi-p19-remediate-cargo.lock cargo test --offline --locked -p opi-eval --test p19_projection_probe p19_plan_probe_pi_only_regrade -- --exact --nocapture --test-threads=1` → FAIL: Cargo exit 101; exactly 1 executed test, 1 failed. Validation, native pi-only run, and offline report succeeded; regrade exit 2 with 'agent set must be exactly [opi, pi], found [pi]'.; expected: FAIL: valid pi-only run cannot regrade with its own valid selected material.
- **Green-after**: `CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/wsl-direct flock /tmp/opi-p19-remediate-cargo.lock cargo test --offline --locked -p opi-eval --test p19_projection_probe p19_plan_probe_pi_only_regrade -- --exact --nocapture --test-threads=1` → expected PASS: exactly one test; pi-only regrade exit 0 and one derived trial.; observed `not-run`.
- **Test impact**: add: native pi-only and Opi-only offline regrade with absent unselected material; update: selected-product material drift/rejection coverage; delete: none; retain: paired all-family native_offline_regrade, native_offline_regrade_rejects_every_fail_closed_case, credential-free/source-immutable/cleanup regrade tests

### Batch B5: Report the frozen full-selection task count

**Closure predicate**: For a legal full selection, selected equals the complete frozen corpus task count, independent of subject count, repetitions, attempted trials, or surviving bundles; subset selected count and separate planned/attempted/valid/unexecuted counts remain correct.
**Dependencies**: none
**Verification union**: `cargo test --offline --locked -p opi-eval --all-targets (WSL/Linux; no ignored live tests)`; `cargo clippy --offline --locked -p opi-eval --all-targets -- -D warnings`; `cargo fmt --check --all`; `python scripts/opi-doc-check.py`; `git diff --check`

#### Fix B5.1: Report the frozen full-selection task count

- **Finding source(s)**: `phase19-codex-gpt6-e079780-20260926t093335z` + `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` + `P19-AUD-005`
- **Decision**: fix:count-frozen-full-selection
- **Verification status**: C/P19-AUD-005: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-eval/src/report.rs`, `crates/opi-eval/tests/bundle_recompute.rs`, `crates/opi-eval/tests/native_driver.rs`
- **Change kind**: behavioral
- **Change**: Compute full-scope selected task count from the existing frozen corpus/task inventory rather than the raw optional request task_ids array. The minimal path is full -> run-control.corpus_tasks, subset -> explicit selected IDs length; preserve separate trial counts and no-selection/null behavior. Add full/subset coverage regressions with multiple subjects/repetitions and missing or zero sealed bundles. Do not revise run-control or report schemas.
- **Closure predicate**: For a legal full selection, selected equals the complete frozen corpus task count, independent of subject count, repetitions, attempted trials, or surviving bundles; subset selected count and separate planned/attempted/valid/unexecuted counts remain correct.
- **Red-before**: `CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/wsl-direct flock /tmp/opi-p19-remediate-cargo.lock cargo test --offline --locked -p opi-eval --test p19_projection_probe p19_plan_probe_full_selection_report -- --exact --nocapture --test-threads=1` → FAIL: Cargo exit 101; exactly 1 executed test, 1 failed. Successful full run/offline report produced corpus=1, selected=0, planned=2, attempted=2, valid=2, invalid=0, unexecuted=0.; expected: FAIL: a one-task successful full run reports selected=0.
- **Green-after**: `CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/wsl-direct flock /tmp/opi-p19-remediate-cargo.lock cargo test --offline --locked -p opi-eval --test p19_projection_probe p19_plan_probe_full_selection_report -- --exact --nocapture --test-threads=1` → expected PASS: exactly one test; corpus=selected=1 and planned=attempted=valid=2 for the same full run.; observed `not-run`.
- **Test impact**: add: full selection report denominator regression including repetitions and total bundle loss; update: existing subset/full native coverage matrix assertions; delete: none; retain: planned_coverage_survives_total_bundle_loss, subset/pair/native failure coverage tests

### Batch B6: Exercise actual clients through the public failure-to-report path

**Closure predicate**: One opt-in test invocation proves both actual clients, local provider fixture, sealed settlement and public run/report agree on each final provider outcome, native zero, and retry-success, with executed test count and exact binary/material identities retained.
**Dependencies**: B2, B8 and B11; actual-client acceptance uses the repaired lifecycle, capture and preflight contract
**Verification union**: `cargo test --offline --locked -p opi-eval --test native_driver native_client_provider_failure_report -- --ignored --exact --nocapture --test-threads=1`; `cargo test --offline --locked -p opi-eval --test native_driver`; `cargo clippy --offline --locked -p opi-eval --test native_driver -- -D warnings`; `cargo fmt --check --all`; `git diff --check`

#### Fix B6.1: Exercise actual clients through the public failure-to-report path

- **Finding source(s)**: `phase19-codex-gpt6-e079780-20260926t093335z` + `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` + `P19-AUD-007`
- **Decision**: fix:actual-client-run-report-matrix
- **Verification status**: C/P19-AUD-007: Confirmed
- **File(s)**: `crates/opi-eval/tests/native_driver.rs`
- **Change kind**: test-only
- **Change**: Add an explicitly opt-in local fixture integration test, proposed native_client_provider_failure_report. Reuse production configuration projection, current exact-client material and local scripted provider, but drive actual Opi/pi through public opi-eval run then offline report. Preserve real client terminal evidence and provider request logs in the same trial bundles; assert final auth/rate-limit/protocol failures attempted/invalid/unscored, native zero valid/denominator/Agent failure, and transient 429 followed by success valid/final success. Retain the existing stand-in tests. Use the pinned native runner/integration and grading route with the local no-model provider fixture; do not synthesize terminal records for the real-client assertions.
- **Closure predicate**: One opt-in test invocation proves both actual clients, local provider fixture, sealed settlement and public run/report agree on each final provider outcome, native zero, and retry-success, with executed test count and exact binary/material identities retained.
- **Red-before**: N/A — This repairs a missing assembled acceptance test; current production misbehavior is not alleged. Static call-path evidence distinguishes the existing disjoint tests, so no invented red production assertion is appropriate.
- **Green-after**: `OPI_EVAL_NATIVE_CLIENT_MATERIAL=<prepared-exact-client-material> OPI_EVAL_NATIVE_WITNESS_MATERIAL=<prepared-three-family-material> cargo test --offline --locked -p opi-eval --test native_driver native_client_provider_failure_report -- --ignored --exact --nocapture --test-threads=1` → expected 1 executed test passes for actual Opi/pi across final failure, native zero and retry-success through public run/report; no remote endpoint or real credential.; observed `not-run`.
- **Test impact**: add; retain existing stand-in report and protocol tests

### Batch B7: Interpret each product native API representation

**Closure predicate**: Byte-bound Opi Anthropic events and pi anthropic-messages events pass for the selected first connection; unknown/wrong API, provider/model substitution and broken tool/terminal evidence reject.
**Dependencies**: none
**Verification union**: `python crates/opi-eval/scripts/test_verify_local_acceptance.py`; `cargo test --offline --locked -p opi-eval --test native_driver p19_plan_acceptance_rejects_fixture_evidence -- --exact`; `python scripts/opi-doc-check.py`; `git diff --check`

#### Fix B7.1: Interpret each product native API representation

- **Finding source(s)**: `phase19-codex-gpt6-e079780-20260926t093335z` + `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` + `P19-AUD-009`
- **Decision**: fix:interpret-product-native-api
- **Verification status**: C/P19-AUD-009: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-eval/scripts/test_verify_local_acceptance.py`, `crates/opi-eval/scripts/verify-local-acceptance.py`
- **Change kind**: behavioral
- **Change**: Interpret the native API field according to the selected product in the existing offline checker. For the fixed first Anthropic connection, require Opi ApiKind Anthropic and pi anthropic-messages while retaining exact provider/model and seal checks. Correct Opi synthetic fixtures to native serialization; add product-specific positive and mismatch negatives. Do not alter Opi serialization, loosen arbitrary aliases, import an Opi crate dependency, or treat unit fixtures as real acceptance.
- **Closure predicate**: Byte-bound Opi Anthropic events and pi anthropic-messages events pass for the selected first connection; unknown/wrong API, provider/model substitution and broken tool/terminal evidence reject.
- **Red-before**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-remediate-p19-gkm1aqyc/probe_native_api.py" "C:\Users\Luiz\AppData\Local\Temp\opi-remediate-p19-gkm1aqyc/source"` → FAIL (exit 1): 7 tests, 1 error and 6 passed; Opi Anthropic raised Rejection: provider response differs from the first connection.; expected: FAIL only the valid Opi-native Anthropic admission; retain pi and mismatch controls.
- **Green-after**: `python <evidence-root>/probe_native_api.py <apply-export>; python crates/opi-eval/scripts/test_verify_local_acceptance.py` → expected All 7 native-shape cases and the complete offline verifier suite pass; rejected cases remain rejected.; observed `not-run`.
- **Test impact**: update existing synthetic Opi shape; add native API mapping/mismatch cases; retain seal, first-connection, terminal and tool-pair negatives.

### Batch B8: Preserve full native capture within the existing artifact bound

**Closure predicate**: For each product, a complete native stream over 1MiB but at or below the existing 64MiB artifact ceiling seals byte-identically with its full digest and non-truncated status; above that ceiling the attempt is explicitly invalid/unscored, retains private diagnostics, and cannot claim complete full evidence.
**Dependencies**: none
**Verification union**: `cargo test --offline -p opi-eval --test native_driver`; `cargo clippy --offline -p opi-eval --all-targets -- -D warnings`; `cargo fmt --check --all`; `python scripts/opi-doc-check.py`; `git diff --check`; `cargo test --offline -p opi-eval --lib bundle`

#### Fix B8.1: Preserve full native capture within the existing artifact bound

- **Finding source(s)**: `phase19-codex-gpt6-e079780-20260926t093335z` + `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` + `P19-AUD-010`; `phase19-pi-glm53-e079780-20260926t192024z` + `f2e7acaffe724a0fa6dbf28ca767b163a3cd664a905a17e6b17924d1fedd582c` + `P19-AUD-002`
- **Decision**: fix:retain-native-stream-within-existing-artifact-bound
- **Verification status**: C/P19-AUD-010: Confirmed; P/P19-AUD-002: Partially confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `crates/opi-eval/src/bundle/mod.rs`, `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/tests/native_driver.rs`
- **Change kind**: behavioral
- **Change**: Use the existing 64MiB per-artifact admission ceiling as the native capture bound through a crate-private shared constant, read at most bound+1, preserve and seal the entire safe stream when within bound, and explicitly classify capacity overflow as evidence failure without scoring or presenting a prefix as full evidence. Keep original private raw logs for diagnostics; keep link/special-file, privacy and integrity checks. Preserve terminal-only policy. Do not change unrelated agent/benchmark profile caps or relax the acceptance checker.
- **Closure predicate**: For each product, a complete native stream over 1MiB but at or below the existing 64MiB artifact ceiling seals byte-identically with its full digest and non-truncated status; above that ceiling the attempt is explicitly invalid/unscored, retains private diagnostics, and cannot claim complete full evidence.
- **Red-before**: `CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/wsl-direct flock /tmp/opi-p19-remediate-cargo.lock cargo test --offline -p opi-eval --test native_driver plan_p19_full_capture_preserves_long_native_stream -- --exact --nocapture --test-threads=1` → FAIL: Named test ran 1 failed, exit 101. Each sealed log was 1048576 bytes and stdout_truncated=true. Opi still classified graded-native reward1; pi classified invalid-execution/import-evidence-incomplete.; expected: FAIL: both >2MiB native streams lose their suffix during full capture.
- **Green-after**: `CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/wsl-direct flock /tmp/opi-p19-remediate-cargo.lock cargo test --offline -p opi-eval --test native_driver plan_p19_full_capture_preserves_long_native_stream -- --exact --nocapture --test-threads=1` → expected PASS: both complete >1MiB streams are retained without truncation and the pi stream remains valid. Add exact-64MiB/over-bound, privacy, identity, unsafe-file and terminal-only controls in the affected native_driver binary.; observed `not-run`.
- **Test impact**: add: complete >1MiB stream for Opi/pi, exact 64MiB admission and over-bound explicit failure; update: full-capture assertions to compare exact original bytes/digest rather than a length only; retain: terminal-only policy, bounded cancellation capture, unsafe link/special-file and canary rejection

### Batch B9: Name invalid witness-material inputs before effects

**Closure predicate**: A single-family/empty-task/missing-profile witness manifest fails before materialization with a named family/field diagnostic, while valid three-family input passes preflight and existing callers consume the same profiles.
**Dependencies**: none
**Verification union**: `cargo test --offline --locked -p opi-eval --test native_driver witness_material -- --nocapture --test-threads=1`; `cargo test --offline --locked -p opi-eval --test native_driver`; `cargo clippy --offline --locked -p opi-eval --test native_driver -- -D warnings`; `cargo fmt --check --all`; `git diff --check`

#### Fix B9.1: Name invalid witness-material inputs before effects

- **Finding source(s)**: `phase19-pi-glm53-e079780-20260926t192024z` + `f2e7acaffe724a0fa6dbf28ca767b163a3cd664a905a17e6b17924d1fedd582c` + `P19-AUD-004`
- **Decision**: fix:witness-material-preflight-diagnostics
- **Verification status**: P/P19-AUD-004: Partially confirmed
- **File(s)**: `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/support/native_witness_material.rs`
- **Change kind**: test-only
- **Change**: Preflight all required three-family task/profile inputs at the beginning of controlled_material, before retained directories, git/tar work or Docker. Reject missing family, empty task map or missing profile path with a diagnostic naming the family and required field; reuse the validated profile paths. Add focused no-Docker invalid-material cases and a valid three-family preflight case. Keep existing evidence retention behavior unchanged.
- **Closure predicate**: A single-family/empty-task/missing-profile witness manifest fails before materialization with a named family/field diagnostic, while valid three-family input passes preflight and existing callers consume the same profiles.
- **Red-before**: `/usr/bin/env -i HOME=/home/luiz PATH=/home/luiz/.cargo/bin:/usr/bin:/bin SHELL=/bin/bash CARGO_NET_OFFLINE=true CARGO_INCREMENTAL=1 CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/wsl-direct cargo test --offline --locked -p opi-eval --test native_driver plan_probe_single_family_witness_reports_missing_family -- --exact --nocapture (scratch-only test; serialized by flock /tmp/opi-p19-remediate-cargo.lock)` → FAIL: 1 executed, 0 passed, 1 failed, 87 filtered; Option::unwrap panic at support/native_witness_material.rs:97; diagnostic must name the missing family assertion fails; 0.36s.; expected: FAIL because the single-family material is rejected by opaque Option::unwrap instead of naming terminal-bench-3.0.
- **Green-after**: `cargo test --offline --locked -p opi-eval --test native_driver witness_material -- --nocapture --test-threads=1` → expected New missing-family, empty-task, missing-profile and valid-three-family preflight tests execute and pass, with no Docker/provider operations and no input mutation.; observed `not-run`.
- **Test impact**: add negative/valid preflight cases; update helper; retain opt-in witnesses

### Batch B10: Clean exact witness-owned containers on conformance failure

**Closure predicate**: A native nop/oracle witness whose runner leaves an exact owned task-environment container is settled only after attempting and verifying that owned cleanup; a neighboring container is retained, and cleanup uncertainty is explicit non-success.
**Dependencies**: B2 for shared cancellation/cleanup failure propagation
**Verification union**: `cargo test --offline -p opi-eval --test native_driver`; `cargo clippy --offline -p opi-eval --all-targets -- -D warnings`; `cargo fmt --check --all`; `python scripts/opi-doc-check.py`; `git diff --check`

#### Fix B10.1: Clean exact witness-owned containers on conformance failure

- **Finding source(s)**: `phase19-pi-glm53-e079780-20260926t192024z` + `f2e7acaffe724a0fa6dbf28ca767b163a3cd664a905a17e6b17924d1fedd582c` + `P19-AUD-005`
- **Decision**: fix:apply-owned-cleanup-to-native-witness
- **Verification status**: P/P19-AUD-005: Partially confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-eval/src/cli/conformance.rs`, `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/src/runner/mod.rs`, `crates/opi-eval/src/runner/native_cleanup.rs`, `crates/opi-eval/tests/native_driver.rs`
- **Change kind**: behavioral
- **Change**: Give the conformance witness the same exact task-root ownership and Docker context cleanup used by ordinary native trials, including observed setup/failure/timeout exits. Extract the minimum existing crate-private cleanup mechanism into native_cleanup for the two real consumers; persist identified resource cleanup evidence before reporting. A failed/unconfirmed cleanup must remain non-success. Never infer ownership from project-name prefixes or remove a neighbor/shared resource.
- **Closure predicate**: A native nop/oracle witness whose runner leaves an exact owned task-environment container is settled only after attempting and verifying that owned cleanup; a neighboring container is retained, and cleanup uncertainty is explicit non-success.
- **Red-before**: `CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/wsl-direct flock /tmp/opi-p19-remediate-cargo.lock cargo test --offline -p opi-eval --test native_driver plan_p19_failed_witness_cleans_owned_resources -- --exact --nocapture --test-threads=1` → FAIL: Named test ran 1 failed, Cargo exit101. Fake state retained one environment container rooted under this witness task-package/environment and one -neighbor. Conformance returned failed/native-binding-no-new-job with compose-down getwd diagnostic; owned cleanup was never attempted.; expected: FAIL: failed nop witness leaves its exact owned fake container.
- **Green-after**: `CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/wsl-direct flock /tmp/opi-p19-remediate-cargo.lock cargo test --offline -p opi-eval --test native_driver plan_p19_failed_witness_cleans_owned_resources -- --exact --nocapture --test-threads=1` → expected PASS: exact owned fake container removed and neighbor retained; add successful witness, timeout, failed inspect/remove, and Docker context controls.; observed `not-run`.
- **Test impact**: add: failed witness owned cleanup with neighbor preservation, cleanup uncertainty and context; retain: ordinary native run cleanup and shared cache preservation

### Batch B11: Bind exact-runtime preflight before native task dispatch

**Closure predicate**: For Harbor and Pier with Opi and pi, a failed or unknown exact-runtime preflight yields zero task launches, no task exit/delivered output invented, preserved attempted coverage and bounded diagnostics, and no graded Agent outcome. A bound passed preflight permits normal task execution: valid zero/nonzero/crash/timeout controls retain existing graded semantics while provider failures, transport/termination uncertainty, missing evidence, malformed proof and substituted trial/runtime/capture identities remain unscored. Reporting agrees with sealed settlement; fresh acceptance requires the bound proof. This proves runtime compatibility, not that a later task process started or that every lazy task dependency can load.
**Dependencies**: B8 for bounded proof capture; B7 before shared offline-checker changes
**Verification union**: `python <evidence-root>/probe_startup_contract.py <apply-export>`; `python crates/opi-eval/scripts/test_native_integrations.py`; `python crates/opi-eval/scripts/test_verify_local_acceptance.py`; `python crates/opi-eval/scripts/test_verify_native_smoke_artifact.py`; `cargo test --offline --locked -p opi-eval --lib`; `cargo test --offline --locked -p opi-eval --test native_driver native_runtime_preflight -- --nocapture --test-threads=1`; `cargo test --offline --locked -p opi-eval --test native_driver`; `cargo fmt --check --all`; `python scripts/opi-doc-check.py`; `git diff --check`

#### Fix B11.1: Bind exact-runtime preflight before native task dispatch

- **Finding source(s)**: `phase19-codex-gpt6-e079780-20260926t093335z` + `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` + `P19-AUD-008`
- **Decision**: fix:bind-native-runtime-preflight
- **Verification status**: C/P19-AUD-008: Partially confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `crates/opi-eval/integrations/harbor/opi_eval_harbor.py`, `crates/opi-eval/integrations/pier/opi_eval_pier.py`, `crates/opi-eval/scripts/test_native_integrations.py`, `crates/opi-eval/scripts/test_verify_local_acceptance.py`, `crates/opi-eval/scripts/test_verify_native_smoke_artifact.py`, `crates/opi-eval/scripts/verify-local-acceptance.py`, `crates/opi-eval/src/benchmark/deepswe.rs`, `crates/opi-eval/src/benchmark/process.rs`, `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/tests/native_driver.rs`
- **Change kind**: behavioral
- **Change**: Implement the user-selected D1 option A as the bounded runtime_preflight contract in Resolved D1. Both pinned integrations run the exact staged executable or pi wrapper/package with --version inside the same task environment before task dispatch and before credential/config projection. Use the existing supervised exec with a 30-second bound; failed or unknown preflight prevents task dispatch and settles as attempted but invalid/unscored. Version the private native terminal as /2 and bind logical trial, executable/package identity, command and separate capture digests against dispatch-owned expectations. Distinguish preflight exit from nullable task exit and task_dispatched. Preserve successful-preflight ordinary nonzero/crash/timeout semantics, existing provider-failure and uncertainty rejection, and offline report consumption of sealed classification. Synchronize normalized sealed proof, both writers, importer/classifier, acceptance checker, native fixtures and bilingual behavior documentation. deepswe.rs changes are limited to the existing SelectionKind::Integration test constructor. No Agent Core hook, stderr heuristic, public SDK, dependency or new configuration knob.
- **Closure predicate**: For Harbor and Pier with Opi and pi, a failed or unknown exact-runtime preflight yields zero task launches, no task exit/delivered output invented, preserved attempted coverage and bounded diagnostics, and no graded Agent outcome. A bound passed preflight permits normal task execution: valid zero/nonzero/crash/timeout controls retain existing graded semantics while provider failures, transport/termination uncertainty, missing evidence, malformed proof and substituted trial/runtime/capture identities remain unscored. Reporting agrees with sealed settlement; fresh acceptance requires the bound proof. This proves runtime compatibility, not that a later task process started or that every lazy task dependency can load.
- **Red-before**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-remediate-p19-gkm1aqyc\probe_startup_contract.py" "C:\Users\Luiz\AppData\Local\Temp\opi-remediate-p19-gkm1aqyc\source"` → FAIL (exit 1): 2 methods, 8 subcase failures. Four refusal cases launched one task; four passed-preflight/nonzero-task cases recorded zero runtime probes.; expected: FAIL across both integration families and products because compatibility refusal is not observed before task dispatch; positive preflight and later nonzero task exit are not distinguished.
- **Green-after**: `python <evidence-root>/probe_startup_contract.py <apply-export>; python crates/opi-eval/scripts/test_native_integrations.py; cargo test --offline --locked -p opi-eval --test native_driver native_runtime_preflight -- --nocapture --test-threads=1; python crates/opi-eval/scripts/test_verify_local_acceptance.py` → expected Same 2-method/8-subcase probe passes. Complete integration suite and new native_runtime_preflight end-to-end cases cover failed/unknown/passed, mismatched or stale proof, nullable task exit, cleanup, preserved nonzero/crash/timeout and invalid provider/uncertainty controls. Loader fixture supplies observed preflight failure instead of relying on diagnostic text. Offline report and acceptance match the sealed proof; every named filter executes nonzero tests.; observed `not-run`.
- **Test impact**: add runtime-preflight dispatch/classification and tamper/unknown/timeout tests; update /2 integration/native-driver/benchmark/deepswe/acceptance/smoke fixtures; retain valid native zero, task nonzero/crash/timeout, provider-error, cleanup, stale-job, identity and capture-policy controls; delete none.

## Final Verification

These are future checks for the final repaired state, not plan-stage pass claims. Execute every added/modified test binary and named filter; zero tests or an ignored filter with no selected test cannot close a finding. Reuse completed checks for the identical relevant state and rerun only invalidated evidence. The final union includes the current workspace-gate finding even though no Bash source fix is justified.

1. Each resolved closure probe -> verify: its recorded red case turns green with the stated counterexamples retained.
2. Modified Rust tests -> verify: the workspace all-target gate below runs all affected default binaries; run each affected ignored native test explicitly on WSL/Linux with pinned offline materials and actual local clients.
3. Script/native seams -> verify: the complete offline acceptance-verifier, native-integration and native-smoke-artifact Python suites, and public CLI rejection/cleanup negatives execute; run native process-group tests on WSL/Linux so platform skips cannot stand in for execution.
4. Final repository state -> verify:

```sh
python scripts/opi-doc-check.py
git diff --check
cargo fmt --check --all
cargo clippy --offline --locked --workspace --all-targets -- -D warnings
cargo test --offline --locked --workspace --all-targets
cargo test --offline --locked --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --offline --locked --workspace --no-deps
python crates/opi-eval/scripts/test_verify_local_acceptance.py
python crates/opi-eval/scripts/test_native_integrations.py
python crates/opi-eval/scripts/test_verify_native_smoke_artifact.py
```

Run the newly added `native_client_provider_failure_report` opt-in test with its exact prepared local-client material input; assert one executed test and both clients/cases. Native witnesses affected by the helper or conformance-cleanup changes must also run explicitly with a verified three-family material manifest: `native_verifier_only_regrade_witness`, `remediation_native_stage_deadlines`, and the corrected affected witness failure path. Preserve exact commands, exit codes, material identities and output logs. Their duration and completion are not inferred from the fast fixture probes.

Do not introduce a new dependency, Cargo feature, public Agent Core seam or unbounded capture. User-selected D1 admits the bounded private terminal /2 contract in B11; synchronize its writers/readers/fixtures and bilingual documentation as specified above. For other batches, synchronize English/Chinese README text only where current user-facing promises change, and add user-visible fixes to the complete current Unreleased section. This fixed workflow plan has no separate Chinese counterpart; source specifications and their translations are unchanged.

## Exclusions

| Source run / Finding ID | Disposition | Current evidence / authority |
|---|---|---|
| C / P19-AUD-006 | Cannot confirm | No source or test change justified from the exact current check. Preserve final workspace verification as an apply-stage obligation, without asserting Phase conformance. |
| P / P19-AUD-001 | Deferred by registered source | Exclude from code-closure claims as Deferred by registered source. After applicable fixes, the Evaluation product owner/local operator must separately authorize and freeze real run scope and preserve fresh six-trial evidence; a later admitted offline verification must pass the unmodified corrected checker. This plan neither calls providers nor declares real acceptance. |
| P / P19-AUD-003 | Returned to shaping | No ledger edits under remediation. Return the metadata disposition to the owning opi-implement authority: document/admit correct executable checks without rewriting a frozen snapshot. Proposed effective selectors are frozen_subset_execution plus hermetic_paired_group_runs_end_to_end, and native_offline_regrade without --ignored. |

The excluded keys still have one exact disposition and retain their source severity. None is a closed finding. The registered source (§8, P19-AC-012) reserves six real-provider trials to a separately authorized local operator run with frozen tasks, builds/materials, controls, bounds and credential references. A future unmodified corrected offline verifier must accept that supplied evidence. This plan does not authorize that run or reinterpret deterministic/native-fixture evidence as real acceptance. Ledger metadata returns to `opi-implement` ownership under `AGENTS.md`; remediation does not rewrite frozen snapshots.

## Exact Criterion Matrix

Criterion identity is the exact `(path, sha256, citation)` tuple. Both members reference the same bytes but use different citation strings, so their rows remain unaligned rather than being silently merged by similar requirement IDs. Each column retains the original requirement ID/state and finding links from its sidecar. A peer `met` state is counter-evidence to assess, never a vote that refutes another peer. The active fail-dominant verdict remains FAIL.

- S1: `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`; SHA-256 `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`.
- S2: `docs/opi-spec.md`; SHA-256 `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db`.

| Exact source / citation | C original requirement / state / findings | P original requirement / state / findings |
|---|---|---|
| S1: Section 7; task 19.1 / P19-AC-002 | `P19-AC-002` / met / none (sidecar line 1) | — |
| S1: Section 7; task 19.1 / P19-AC-003-native | `P19-AC-003-native` / met / none (sidecar line 2) | — |
| S1: Section 7; task 19.1 / P19-AC-008-outcome | `P19-AC-008-outcome` / partially-met / P19-AUD-008 (sidecar line 3) | — |
| S1: Section 7; task 19.2 / P19-AC-006-fixture | `P19-AC-006-fixture` / met / none (sidecar line 4) | — |
| S1: Section 7; task 19.2 / P19-AC-010-credentials | `P19-AC-010-credentials` / met / none (sidecar line 5) | — |
| S1: Section 7; task 19.3 / P19-AC-004-native | `P19-AC-004-native` / met / none (sidecar line 6) | — |
| S1: Section 7; task 19.4 / P19-AC-005-native | `P19-AC-005-native` / met / none (sidecar line 7) | — |
| S1: Section 7; task 19.5 / P19-AC-001-prepare | `P19-AC-001-prepare` / met / none (sidecar line 8) | — |
| S1: Section 7; task 19.5 / P19-AC-007-material | `P19-AC-007-material` / met / none (sidecar line 9) | — |
| S1: Section 7; task 19.6 / P19-AC-007-selection | `P19-AC-007-selection` / met / none (sidecar line 10) | — |
| S1: Section 7; task 19.6 / P19-AC-001-plan | `P19-AC-001-plan` / met / none (sidecar line 11) | — |
| S1: Section 7; task 19.7 / P19-AC-008 | `P19-AC-008` / partially-met / P19-AUD-005, P19-AUD-007 (sidecar line 12) | — |
| S1: Section 7; task 19.7 / P19-AC-009-verify | `P19-AC-009-verify` / partially-met / P19-AUD-003 (sidecar line 13) | — |
| S1: Section 7; task 19.8 / P19-AC-009-regrade | `P19-AC-009-regrade` / partially-met / P19-AUD-004 (sidecar line 14) | — |
| S1: Section 7; task 19.9 / P19-AC-010-lifecycle | `P19-AC-010-lifecycle` / partially-met / P19-AUD-002 (sidecar line 15) | — |
| S1: Section 7; task 19.10 / P19-AC-011 | `P19-AC-011` / met / none (sidecar line 16) | — |
| S1: Section 7; task 19.10 / P19-AC-013 | `P19-AC-013` / partially-met / P19-AUD-004 (sidecar line 17) | — |
| S1: Section 7; task 19.11 / P19-AC-001-live | `P19-AC-001-live` / partially-met / P19-AUD-010 (sidecar line 18) | — |
| S1: Section 7; task 19.11 / P19-AC-003-live | `P19-AC-003-live` / partially-met / P19-AUD-009, P19-AUD-010 (sidecar line 19) | — |
| S1: Section 7; task 19.11 / P19-AC-004-live | `P19-AC-004-live` / partially-met / P19-AUD-009, P19-AUD-010 (sidecar line 20) | — |
| S1: Section 7; task 19.11 / P19-AC-005-live | `P19-AC-005-live` / partially-met / P19-AUD-009, P19-AUD-010 (sidecar line 21) | — |
| S1: Section 7; task 19.11 / P19-AC-006-live | `P19-AC-006-live` / partially-met / P19-AUD-009 (sidecar line 22) | — |
| S1: Section 7; task 19.11 / P19-AC-012 | `P19-AC-012` / partially-met / P19-AUD-006, P19-AUD-009, P19-AUD-010 (sidecar line 23) | — |
| S1: Section 4 / task 19.5 | `P19-001-doctor` / met / none (sidecar line 24) | — |
| S1: Section 4-5 / task 19.5 | `P19-001-acquire` / met / none (sidecar line 25) | — |
| S1: Section 5-6 / task 19.1 | `P19-002-settlement` / partially-met / P19-AUD-008 (sidecar line 26) | — |
| S1: Section 6 / task 19.1 | `P19-002-failed-capture` / met / none (sidecar line 27) | — |
| S1: Section 5 / task 19.1 | `P19-003-no-legacy-route` / met / none (sidecar line 28) | — |
| S1: Section 5 / task 19.3 | `P19-004-declarations` / met / none (sidecar line 29) | — |
| S1: Section 5 / task 19.4 | `P19-005-patch` / met / none (sidecar line 30) | — |
| S1: Section 4 / task 19.2 | `P19-006-config` / partially-met / P19-AUD-001 (sidecar line 31); `P19-006-controls` / met / none (sidecar line 32) | — |
| S1: Section 4,6 / task 19.2 | `P19-006-terminal` / met / none (sidecar line 33) | — |
| S1: Section 4,6 | `P19-006-unknown` / met / none (sidecar line 34) | — |
| S1: Section 5 / task 19.6 | `P19-007-inventory` / met / none (sidecar line 35) | — |
| S1: Section 4-5 / task 19.6 | `P19-007-plan` / met / none (sidecar line 36) | — |
| S1: Section 4 / tasks 19.2,19.6,19.9 | `P19-007-bounds` / met / none (sidecar line 37) | — |
| S1: Section 6 / task 19.7 | `P19-008-coverage` / met / none (sidecar line 38); `P19-008-metrics` / partially-met / P19-AUD-005, P19-AUD-008 (sidecar line 39); `P19-008-pairs` / met / none (sidecar line 40) | — |
| S1: Section 6 / task 19.8 | `P19-009-derived` / partially-met / P19-AUD-004 (sidecar line 41) | — |
| S1: Section 4 / tasks 19.2,19.9 | `P19-010-isolation` / met / none (sidecar line 42) | — |
| S1: Section 9 / task 19.9 | `P19-010-cancel` / partially-met / P19-AUD-002 (sidecar line 43) | — |
| S1: Section 4,6 / task 19.9 | `P19-010-recovery` / met / none (sidecar line 44) | — |
| S1: Section 8 / task 19.10 | `P19-011-ci-policy` / met / none (sidecar line 45) | — |
| S1: Section 8 / tasks 19.10-19.11 | `P19-012-offline-verifier` / partially-met / P19-AUD-009 (sidecar line 46); `P19-012-provenance` / met / none (sidecar line 47) | — |
| S1: Section 8 / task 19.11 | `P19-012-final-gates` / partially-met / P19-AUD-006 (sidecar line 48) | — |
| S1: Section 3 / P19-AC-013 | `P19-013-placement` / met / none (sidecar line 49) | — |
| S1: Section 9 / P19-AC-013 | `P19-013-contracts` / met / none (sidecar line 50) | — |
| S1: Section 5 | `P19-OPTIONAL-ORACLE` / met / none (sidecar line 51) | — |
| S1: Section 4 | `P19-OPTIONAL-CONNECTIONS` / met / none (sidecar line 52) | — |
| S2: PRIN-001, PRIN-002, PRIN-003, PLACE-001, PLACE-004 | `P19-ARCH-MINIMUM` / met / none (sidecar line 53) | — |
| S1: Section 7 P19-AC-002; task 19.1 acceptance scenario P19-AC-002 | — | `P19-AC-002` / met / none (sidecar line 1) |
| S1: Section 7 P19-AC-003; task 19.1 acceptance scenario P19-AC-003-native | — | `P19-AC-003-native` / met / none (sidecar line 2) |
| S1: Section 7 P19-AC-008; task 19.1 acceptance scenario P19-AC-008-outcome | — | `P19-AC-008-outcome` / met / none (sidecar line 3) |
| S1: Section 7 P19-AC-006; task 19.2 acceptance scenario P19-AC-006-fixture | — | `P19-AC-006-fixture` / met / none (sidecar line 4) |
| S1: Section 7 P19-AC-010; task 19.2 acceptance scenario P19-AC-010-credentials | — | `P19-AC-010-credentials` / met / none (sidecar line 5) |
| S1: Section 7 P19-AC-004; task 19.3 acceptance scenario P19-AC-004-native | — | `P19-AC-004-native` / met / none (sidecar line 6) |
| S1: Section 7 P19-AC-005; task 19.4 acceptance scenario P19-AC-005-native | — | `P19-AC-005-native` / met / none (sidecar line 7) |
| S1: Section 7 P19-AC-001; task 19.5 acceptance scenario P19-AC-001-prepare | — | `P19-AC-001-prepare` / met / none (sidecar line 8) |
| S1: Section 7 P19-AC-007; task 19.5 acceptance scenario P19-AC-007-material | — | `P19-AC-007-material` / met / none (sidecar line 9) |
| S1: Section 7 P19-AC-007; task 19.6 acceptance scenario P19-AC-007-selection | — | `P19-AC-007-selection` / met / none (sidecar line 10) |
| S1: Section 7 P19-AC-001; task 19.6 acceptance scenario P19-AC-001-plan | — | `P19-AC-001-plan` / met / P19-AUD-003 (sidecar line 11) |
| S1: Section 7 P19-AC-008; task 19.7 acceptance scenario P19-AC-008 | — | `P19-AC-008` / met / none (sidecar line 12) |
| S1: Section 7 P19-AC-009; task 19.7 acceptance scenario P19-AC-009-verify | — | `P19-AC-009-verify` / met / none (sidecar line 13) |
| S1: Section 7 P19-AC-009; task 19.8 acceptance scenario P19-AC-009-regrade | — | `P19-AC-009-regrade` / met / P19-AUD-003, P19-AUD-004 (sidecar line 14) |
| S1: Section 7 P19-AC-010; task 19.9 acceptance scenario P19-AC-010-lifecycle | — | `P19-AC-010-lifecycle` / met / P19-AUD-005 (sidecar line 15) |
| S1: Section 7 P19-AC-011; task 19.10 acceptance scenario P19-AC-011 | — | `P19-AC-011` / met / none (sidecar line 16) |
| S1: Section 7 P19-AC-013; task 19.10 acceptance scenario P19-AC-013 | — | `P19-AC-013` / met / none (sidecar line 17) |
| S1: Section 7 P19-AC-001; task 19.11 acceptance scenario P19-AC-001-live | — | `P19-AC-001-live` / partially-met / P19-AUD-001 (sidecar line 18) |
| S1: Section 7 P19-AC-003; task 19.11 acceptance scenario P19-AC-003-live | — | `P19-AC-003-live` / partially-met / P19-AUD-001 (sidecar line 19) |
| S1: Section 7 P19-AC-004; task 19.11 acceptance scenario P19-AC-004-live | — | `P19-AC-004-live` / partially-met / P19-AUD-001 (sidecar line 20) |
| S1: Section 7 P19-AC-005; task 19.11 acceptance scenario P19-AC-005-live | — | `P19-AC-005-live` / partially-met / P19-AUD-001 (sidecar line 21) |
| S1: Section 7 P19-AC-006; task 19.11 acceptance scenario P19-AC-006-live | — | `P19-AC-006-live` / partially-met / P19-AUD-001, P19-AUD-002 (sidecar line 22) |
| S1: Section 7 P19-AC-012; task 19.11 acceptance scenario P19-AC-012 | — | `P19-AC-012` / not-met / P19-AUD-001, P19-AUD-002 (sidecar line 23) |

## Probe and planning evidence manifest

The following fresh evidence was produced at the remediation head. Historical audit reproduction paths are source claims only; they were not used as probe implementations.

- `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-p19-gkm1aqyc/probe_native_api.py` — SHA-256 `a4e10d36a13be419fb96894ba78f2294f5448697157ec21f5335fb7be5e3d6a2`; C009: seven native-shape attribution cases; run against an explicitly selected source export.
- `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-p19-gkm1aqyc/native-api-red.log` — SHA-256 `0dcf1a2bba642fb59f8fb15561a358db0c2c996d0edcc458b29de18e804404ce`; C009 observed red: one rejection error plus six passing controls.
- `C:/Users/Luiz/AppData/Local/Temp/opi-p19-projection-5c62d8c65df54214bfa345c67258dbd8/crates/opi-eval/tests/p19_plan_probes.inc` — SHA-256 `68d00b928793c5e77b331deb47e5734ec593bf17b3e087c85efc2cd1039fd6e2`; C001/C003/C004/C005 isolated regression setup. Export remediation_head with git archive to a new temporary directory, copy p19_plan_probes.inc into crates/opi-eval/tests, copy that export's committed tests/native_driver.rs to tests/p19_projection_probe.rs and append LF + include!("p19_plan_probes.inc"); + LF. Execute each red_before command from that export under WSL/Linux with /usr/bin/env -i HOME=/home/luiz PATH=/home/luiz/.cargo/bin:/usr/local/bin:/usr/bin:/bin and existing external CARGO_TARGET_DIR. For green-after use the same fragment on applied source in a separate temporary export, not this immutable baseline export. Unique test target name avoids sibling temporary probes sharing native_driver executable artifacts.
- `C:/Users/Luiz/AppData/Local/Temp/opi-p19-projection-5c62d8c65df54214bfa345c67258dbd8/crates/opi-eval/tests/p19_projection_probe.rs` — SHA-256 `af859edc229a24813d014367f4cd5bb1f28efccd6956b03ec5c35bd87a3aeba1`; Projection/offline-operation reproducible probe setup.
- `C:/Users/Luiz/AppData/Local/Temp/opi-p19-projection-5c62d8c65df54214bfa345c67258dbd8/run-probes.sh` — SHA-256 `209a3fefa1704e45e481045f6405eaf5d5e12e45062f2eb1522cd70ae8e8b106`; Projection/offline-operation reproducible probe setup.
- `C:/Users/Luiz/AppData/Local/Temp/opi-p19-projection-5c62d8c65df54214bfa345c67258dbd8/red-file_credential_projection.log` — SHA-256 `07d250cf944aef9a8e47e8d1f86c2e41e101b9bd54895741386e87f518a1c86a`; Final exact single-test red output; matching .exit file records Cargo exit 101.
- `C:/Users/Luiz/AppData/Local/Temp/opi-p19-projection-5c62d8c65df54214bfa345c67258dbd8/red-full_selection_report.log` — SHA-256 `56a42122a2d4affad915e74fd9baab1b7f49f879f0ab8a08247aff000233481d`; Final exact single-test red output; matching .exit file records Cargo exit 101.
- `C:/Users/Luiz/AppData/Local/Temp/opi-p19-projection-5c62d8c65df54214bfa345c67258dbd8/red-pi_only_regrade.log` — SHA-256 `dee96bed64aed13f7045ba5b7fa4ce4f318e66e82d6813847d7d858d7ac276d5`; Final exact single-test red output; matching .exit file records Cargo exit 101.
- `C:/Users/Luiz/AppData/Local/Temp/opi-p19-projection-5c62d8c65df54214bfa345c67258dbd8/red-verify_trial_symlink.log` — SHA-256 `a20fcc5aed4b0e21f3fda4e0ef5cfa8ed59457d09db27d611d23749d3973dc1d`; Final exact single-test red output; matching .exit file records Cargo exit 101.
- `C:/Users/Luiz/AppData/Local/Temp/opi-p19-lifecycle-10898ea8c62d415a810cc38c02c2ea0a/plan_p19_probes.rs` — SHA-256 `2e69c173fb66121f0cc1253cc7c2c632c38f9a68e0907955235c0a6a8bc8503c`; C002/C008/C010/P002/P005 exact red proof. Append UTF-8 bytes of plan_p19_probes.rs and plan_p19_witness_probe.rs to crates/opi-eval/tests/native_driver.rs in a fresh export of remediation_head. Use shared external target and lock across each full cargo command. Do not install tests into shared checkout during plan. Both snippets depend only on current committed native_driver helpers.
- `C:/Users/Luiz/AppData/Local/Temp/opi-p19-lifecycle-10898ea8c62d415a810cc38c02c2ea0a/plan_p19_witness_probe.rs` — SHA-256 `34aef29ef265b0d7d859db46e24da5daae4e16e0e014fd75fc50c48c67da1f47`; C002/C008/C010/P002/P005 exact red proof. Append UTF-8 bytes of plan_p19_probes.rs and plan_p19_witness_probe.rs to crates/opi-eval/tests/native_driver.rs in a fresh export of remediation_head. Use shared external target and lock across each full cargo command. Do not install tests into shared checkout during plan. Both snippets depend only on current committed native_driver helpers.
- `C:/Users/Luiz/AppData/Local/Temp/opi-p19-lifecycle-10898ea8c62d415a810cc38c02c2ea0a/plan_p19_exact_red.py` — SHA-256 `ef97fa868285cb7cf37d405b5feb1db4880200836b35d3966985590d312b616e`; Final serialized single-test command runner/log; five named failures.
- `C:/Users/Luiz/AppData/Local/Temp/opi-p19-lifecycle-10898ea8c62d415a810cc38c02c2ea0a/plan-p19-exact-red.log` — SHA-256 `66eae0a7ce35f459d88b6b6a36e52d4a9773a9597c75b51f3d509f305df9e1c6`; Final serialized single-test command runner/log; five named failures.
- `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-p19-gkm1aqyc/witness-input-probe.rs` — SHA-256 `3760cce2026259b55317a5e1e743d1da8ea7669b015f2ef87c9dc138ce07e785`; P004: append this test-only fragment to committed native_driver.rs in an isolated export; run the exact recorded test under the shared Cargo lock.
- `C:/Users/Luiz/AppData/Local/Temp/opi-p19-verification-7b0b59a78306434d8afbb1b399dbfc58/verification-handoff.json` — SHA-256 `ed7004e9e66766225be8736214e11e840997d764dacdd996ed444bd47eeb4bac`; Exact current WSL check/selector outcomes and single-family witness red command/diagnostic; source of this invocation evidence only.
- `C:\Users\Luiz\AppData\Local\Temp\opi-remediate-p19-gkm1aqyc\probe_startup_contract.py` — SHA-256 `06ff709a3876628e9cccd578f815c6968dd2ba852722550fd65f09dc22414236`; D1 option A unchanged-source two-method/eight-subcase runtime-preflight regression probe
- `C:\Users\Luiz\AppData\Local\Temp\opi-remediate-p19-gkm1aqyc\startup-contract-red.log` — SHA-256 `ab30daa625f234732f77b870df23d051ed2ff4c909dab352c10e8b3a545e416e`; D1 option A observed red: 8 failed subcases, exit 1

## Planning completion and materialization

Observable completion: fixed plan plus strict-union dispositions -> verify: dispositions validator and plan validator pass; docs -> verify: doc checker passes in the sealed export with this plan pair; task diff -> verify: only the two fixed plan files are modified and `git diff --check` passes. The digest emitted by the plan validator is reported externally and is not embedded here.

Actual task test impact: none in the repository; fresh tests/probes were added only to temporary exports. Planned impact is add/update/retain as recorded per batch; no test deletion is proposed. Actual committed changes=none-for-this-task, staged=[], untracked=[]; the outgoing uncommitted paths are the fixed Markdown/JSONL plan pair. D1 is resolved by the user-selected option A; this READY-FOR-APPLY pair requires a new explicit digest-bound apply invocation before production edits. No Phase-conformance or closure result is claimed.
