# Phase 19 Remediation Result

**Status**: COMPLETE
**Audit index SHA-256**: `7550892cafbe6db2df1b56cd3f2eadb2104303428019940d4dc341e621f48f9d`
**Plan SHA-256**: `bd72b16cd96d523c38116701c26d0a12ba7ded5398feb752e31ba1034e08b57f`
**Remediation head**: `131f003d4ae883e30df3ec85eae7469b0d8c60cc`
**Disposition artifact**: `remediation.result.dispositions.jsonl`
**Changed paths**: ["CHANGELOG.md","crates/opi-eval/README.md","crates/opi-eval/README.zh.md","crates/opi-eval/integrations/harbor/opi_eval_harbor.py","crates/opi-eval/integrations/harbor/opi_eval_regrade.py","crates/opi-eval/integrations/pier/opi_eval_pier.py","crates/opi-eval/integrations/pier/opi_eval_regrade.py","crates/opi-eval/scripts/native-smoke.sh","crates/opi-eval/scripts/test_native_integrations.py","crates/opi-eval/scripts/test_native_regrade.py","crates/opi-eval/scripts/test_native_smoke_output_control.py","crates/opi-eval/scripts/test_verify_native_smoke_artifact.py","crates/opi-eval/scripts/verify-native-smoke-artifact.py","crates/opi-eval/src/agent/opi.rs","crates/opi-eval/src/agent/pi.rs","crates/opi-eval/src/agent/process.rs","crates/opi-eval/src/benchmark/deepswe.rs","crates/opi-eval/src/benchmark/process.rs","crates/opi-eval/src/benchmark/task_lifecycle.rs","crates/opi-eval/src/benchmark/terminal_bench_21.rs","crates/opi-eval/src/benchmark/terminal_bench_30.rs","crates/opi-eval/src/cli/conformance.rs","crates/opi-eval/src/cli/run.rs","crates/opi-eval/src/main.rs","crates/opi-eval/src/regrade.rs","crates/opi-eval/src/report.rs","crates/opi-eval/src/runner/experiment.rs","crates/opi-eval/src/runner/material.rs","crates/opi-eval/src/runner/plan.rs","crates/opi-eval/src/runner/prepare.rs","crates/opi-eval/src/trajectory/mod.rs","crates/opi-eval/tests/agent_integration_conformance.rs","crates/opi-eval/tests/assembled_run.rs","crates/opi-eval/tests/benchmark_integration_conformance.rs","crates/opi-eval/tests/bundle_recompute.rs","crates/opi-eval/tests/end_to_end_report.rs","crates/opi-eval/tests/fail_closed_cli.rs","crates/opi-eval/tests/native_driver.rs","crates/opi-eval/tests/report_contract.rs"]

This completes disposition recording, not all repairs or Phase acceptance.
Of the 13 exact indexed source keys, 8 are Closed, 3 are Not closed,
1 is Cannot confirm, and 1 is Info/No action. Five bounded incidental repairs
are separately recorded. B5's real-stage witness now reproduces a verifier
process continuing beyond its deadline; D5 stops execution before B10 and B11. No Phase PASS is granted.

## Approval and scope

The explicit apply invocation supplies the exact plan digest above. Revalidation
retained READY-FOR-APPLY, the committed HEAD, the active index bytes and all
indexed member identities. The original 14 carried-in source paths were admitted
against the approved baseline before work; they were preserved and extended only
within the approved batches. The staged inventory is empty. There was no branch,
commit, push, manifest/lockfile change, dependency change, implementation-ledger
edit, or invocation of another explicit-only Opi workflow.

The machine Changed paths header is the exact union of the copied plan records
and accepted incidentals, as required by the disposition contract. It is an
attribution scope, not a claim that every planned file was edited: B10/B11 paths
remain in copied records although those batches did not execute. The actual
tracked worktree delta contains these 31 paths:

```text
CHANGELOG.md
crates/opi-eval/README.md
crates/opi-eval/README.zh.md
crates/opi-eval/integrations/harbor/opi_eval_harbor.py
crates/opi-eval/integrations/pier/opi_eval_pier.py
crates/opi-eval/scripts/native-smoke.sh
crates/opi-eval/scripts/test_native_integrations.py
crates/opi-eval/scripts/test_native_smoke_output_control.py
crates/opi-eval/scripts/test_verify_native_smoke_artifact.py
crates/opi-eval/scripts/verify-native-smoke-artifact.py
crates/opi-eval/src/agent/opi.rs
crates/opi-eval/src/agent/pi.rs
crates/opi-eval/src/agent/process.rs
crates/opi-eval/src/benchmark/deepswe.rs
crates/opi-eval/src/benchmark/process.rs
crates/opi-eval/src/benchmark/task_lifecycle.rs
crates/opi-eval/src/benchmark/terminal_bench_21.rs
crates/opi-eval/src/benchmark/terminal_bench_30.rs
crates/opi-eval/src/cli/conformance.rs
crates/opi-eval/src/cli/run.rs
crates/opi-eval/src/main.rs
crates/opi-eval/src/regrade.rs
crates/opi-eval/src/runner/experiment.rs
crates/opi-eval/src/runner/material.rs
crates/opi-eval/src/runner/plan.rs
crates/opi-eval/src/runner/prepare.rs
crates/opi-eval/src/trajectory/mod.rs
crates/opi-eval/tests/assembled_run.rs
crates/opi-eval/tests/fail_closed_cli.rs
crates/opi-eval/tests/native_driver.rs
crates/opi-eval/tests/report_contract.rs
```

The plan pair was carried in unchanged. The result pair is generated workflow
output, separate from that tracked code/document delta. The bilingual README
changes are synchronized; this fixed result has no existing Chinese counterpart.

## Finding outcomes

`C` denotes `phase19-codex-gpt6-c18efa8-20260917t133157z`; `P` denotes
`phase19-pi-glm53-d61590b-20260917t152753z`. Exact owning findings digests and
unchanged source decisions appear in every machine record.

| Source key | Batch | Result | Observed scope |
|---|---|---|---|
| C / P19-AUD-001 | B1 | Closed | Regrade child environment isolation. Real replay correctness remains B10. |
| C / P19-AUD-006 | B2 | Closed | Native frozen-plan entry and typed pre-effect input rejection. |
| C / P19-AUD-003 | B3 | Closed | Exact selected terminal product, executable digest and model identity. |
| C / P19-AUD-004 | B4 | Closed | Provider/transport errors excluded from scored task results; valid failures retained. |
| C / P19-AUD-005 | B5 | Not closed | Real Agent caps and timeout classification pass; the verifier script still completes after its deadline. |
| C / P19-AUD-009 | B6 | Closed | Existing run control and trial evidence survive rejected reruns and concurrent claims. |
| C / P19-AUD-012 | B7 | Closed | Unobserved native Agent duration is Unknown; measured host durations keep origin. |
| C / P19-AUD-007 | B8 | Closed | Effective role requirements, origins, selected-engine observations and resource/image rejection. |
| C / P19-AUD-008 | B9 | Closed | material/5, run-plan/2 and plan-preview/2, full pinned corpus and per-task binding/dispatch. |
| C / P19-AUD-002 | B10 | Not closed | Not started because B5's required real-stage termination gate fails. |
| C / P19-AUD-011 | B11 | Not closed | Not started; depends on B10 and the real-stage gate. |
| C / P19-AUD-010 | none | Cannot confirm | Preserve Partially confirmed/Major and all 30 linked requirements; excluded external acceptance evidence was not read. |
| P / P19-AUD-001 | none | Info/No action | Preserve Partially confirmed/Info; immutable mixed commit provenance confirmed, no history rewrite. |

B8 compares actual selected-engine CPU/memory and separately observed engine,
cache and output storage. Unobserved engine storage and GPU capacity remain
Unknown; an nvidia runtime registration is not a device count. Pinned Harbor/Pier
requirements retain role, default/declaration origin, network, lifecycle, collect
and deadlines. Invalid units and unsupported requirements fail closed. Preview
never starts task images or silently drops infeasible tasks.

B9 verifies the complete source corpus against its declared pinned commit/tree,
keeps source identity separate from immutable-image materialization, and verifies
each selected canonical task's native name, profile bytes and package digest.
The three-family two-task fixture matrix exercises direct and frozen entries
from independent directories, producing 24 sealed deterministic trials. Missing
images, selected bindings, stale corpus bytes, swapped identities and old schemas
reject. Existing sealed bundles are preserved; material/4 and plan/1 must be
regenerated. These fixtures do not establish actual native grading correctness.

## B5 native witness boundary

The user confirmed no reusable material was available. This continuation prepared
independent locked Harbor/Pier environments without reading historical eval
archives or acceptance outputs. The exact Pier commit was already present in the
named source object databases despite their different checkout HEADs. Independent
clones now bind Harbor `4407eb5227a2ff4f0d3f16b2eb48849382fdf276` and Pier
`c1ebc6d145b40fae8425215e3fca528945065124`; locked dependency synchronization
left both upstream source trees and lockfiles unchanged.

The no-model witness uses actual Docker, real pinned uv and runners, and
independently Git/byte-bound controlled delay tasks. Its deterministic client
fixtures exercise the Opi/pi integration command paths without running either
real client or calling a provider. It is stage conformance evidence, not official
benchmark or C/010 acceptance. The fixture declares its own CPU/memory/storage
requirements; no original benchmark task requirement was reduced.

Reusable WSL material is retained at
`/home/luiz/.cache/opi-native-19-mzcSc2/witness-material-cancel/material.json`
(SHA-256 `b01c5956a37f364a4be05d3931c11a8f03d825579c0d2ab02831792cce2ee31c`).
It byte-binds the updated repository integrations and cached immutable image
`python@sha256:272efc3f94bec901e2bc7a5c9984b3d113c317839edca2714c6a388b9476f8fc`.
Independent runner source and environments are siblings at `harbor/` and
`pier/`. The controlled-material generator is
`/home/luiz/.cache/opi-native-19-mzcSc2/prepare_witness.py`
(SHA-256 `9d09f99fb6c7700bd6485ea410cd06b9ba0e3e7eedf55a2a816d4ad359bcd264`).
The first material generation remains unchanged under `witness-material/`;
the final manifest substitutes fresh byte-bound integration copies.

Docker Desktop's selected Linux engine remains
`f7640520-9a9f-4acb-bd58-62c5b388982a`, with 32 CPUs and 16611233792 bytes memory.
An operator read through the `docker-desktop` WSL namespace observed its actual
storage at `/proc/185/root/var/lib/docker` (993441696 KiB available at inspection).
This does not change the product's Unknown storage observation from Ubuntu or
add a new engine-observation transport. GPU capacity remains Unknown.
The ambient WSL `env` resolves to a historical stub; preparation therefore used
`/usr/bin/env -i` and real absolute tools. The native run's closed environment
uses `/usr/local/bin:/usr/bin:/bin`, never that stub.

The implemented ignored test `remediation_native_stage_deadlines` was explicitly
run with `--ignored --exact`; no ignored or zero-test result counts as PASS.
Fresh failures and repairs from these runs:

1. A first preparation attempt (`/tmp/.tmpe2Z9Zz`) used a nonexistent workdir.
   The controlled fixture was corrected to `/tmp`; this is not behavioral red
   evidence.
2. `/tmp/.tmp6Wvdg8` completed a real normal trial but rejected its configuration:
   Harbor persists TrialConfig with `exclude_defaults=True` and omits the
   default global multiplier 1. The reader now accepts that exact omission;
   explicit null/changed multipliers and stage-cap drift still reject.
3. `/tmp/.tmpdxLs2f` reproduced missing terminal identity on real Agent-stage
   cancellation. I5 now terminates the integration-owned launch process group,
   observes the actual exit and preserves cancellation. Focused subprocess
   tests went red/green; outer host kill still creates no invented terminal.
4. `/tmp/.tmpVSEqYH` observed both Agent exits 137, but pi's interrupted stream
   was incorrectly excluded. The private projection now takes the bound
   runner's AgentTimeoutError as the timeout cause. Its focused test went
   red/green.
5. `/tmp/.tmpDdiW9P` reproduced real VerifierTimeoutError with empty reward
   statistics and a null verifier result. Binding now preserves an Unknown
   reward only with exact exception attribution and unchanged identity/artifact
   checks. Focused positive and substitution tests went red/green.

The final actual witness, `/tmp/.tmpmOZkTd`, **FAILS**. TB2.1 normal controls,
standalone `time_limit_seconds`, and standalone `agent_timeout_seconds` pass
for both subjects. The verifier-only case correctly settles both trials as
invalid `verifier-timeout` with no score, but the real verifier process continues
after its deadline. For the Opi-controlled trial:

- native verifier timing: 03:26:53.282288Z to 03:26:54.319462Z (1-second cap);
- mounted `verifier/stage-start`: 03:26:53.482238Z;
- mounted `verifier/stage-end`: 03:26:57.487049Z, after the actual 4-second sleep;
- native job finished: 03:26:58.636577Z.

The pi-controlled trial shows the same delayed completion. The final assertion
`!verifier/stage-end.exists()` fails. An independent Docker query confirms no
remaining containers under all 16 exact owned Compose directories in this run;
absence after job cleanup does not prove stage-time termination.

Pinned Harbor `trial/trial.py:670` cancels `verifier.verify()` through
`asyncio.wait_for`; `verifier/verifier.py:199` awaits environment exec without
command-level termination. Docker `_collect_buffered_output` has no
CancelledError cleanup. The script can finish during later environment cleanup.
The existing maximum/override timeout configuration cannot fix this process
lifecycle gap. Pier's current environment config also has no Harbor-style extra
Compose override surface; changing task grader bytes, adding an execution
environment adapter, patching/upgrading upstream, or substituting a whole-job
timeout is not the approved B5 dispatch/configuration repair.

B5 therefore remains Not closed. Combined bounds, minimum-cap, stricter/unbounded
defaults and the TB3/Pier real matrices remain unexecuted after fail-fast.
D5 stops B10 and B11. Their helpers, schemas and real regrade/report closure
checks remain unimplemented. A new fixed plan must explicitly scope the verifier
execution/cancellation boundary before dependent implementation resumes.

## Verification evidence

Rust commands ran in WSL/Linux with Cargo 1.97.1, incremental compilation enabled
and the existing `/mnt/e/opi/cargo-targets/opi-8abcf38f9871653e` cache. Each Cargo
shell registered and released its lease through `scripts/opi-cargo-cache.py`.
No cache was cleaned and no session-specific target directory was created.

The exact behavioral checks below passed (one focused test per command), then
ran again as part of the final native-driver binary. B5 here is only its initial
offline/configuration gate.

```sh
cargo test -p opi-eval --locked --test native_driver remediation_001_regrade_child_has_no_ambient_secret -- --exact --nocapture --test-threads=1
cargo test -p opi-eval --locked --test native_driver remediation_006_plan_material_rejects_without_panic -- --exact --nocapture --test-threads=1
cargo test -p opi-eval --locked --test native_driver remediation_003_substituted_terminal_identity_rejects -- --exact --nocapture --test-threads=1
cargo test -p opi-eval --locked --test native_driver remediation_004_native_provider_error_is_not_scored -- --exact --nocapture --test-threads=1
cargo test -p opi-eval --locked --test native_driver remediation_005_native_stage_deadlines_reach_runner -- --exact --nocapture --test-threads=1
cargo test -p opi-eval --locked --test native_driver remediation_009_rejected_rerun_preserves_control -- --exact --nocapture --test-threads=1
cargo test -p opi-eval --locked --test native_driver remediation_012_native_unobserved_time_is_unknown -- --exact --nocapture --test-threads=1
cargo test -p opi-eval --locked --test native_driver remediation_007_prepare_rejects_impossible_storage -- --exact --nocapture --test-threads=1
cargo test -p opi-eval --locked --test native_driver remediation_008_feasible_second_task_can_freeze -- --exact --nocapture --test-threads=1
```

Additional matrices and final gates:

| Command or case within the final binary | Result |
|---|---|
| `material_five_*` cases in the final `native_driver` binary below | PASS: 2 expanded binding/dispatch matrices; their current versions also passed in the final full binary. |
| `prepare_` cases in the final `native_driver` binary below | PASS: 6 producer/preflight cases, including the storage-reason assertion. |
| `cargo test -p opi-eval --locked --lib` | PASS: 146, including effective-role resource/default/origin matrices and immutable-image/source binding checks. |
| `cargo test -p opi-eval --locked --test native_driver --test assembled_run --test fail_closed_cli --test benchmark_integration_conformance --test agent_integration_conformance --test report_contract --test end_to_end_report --test bundle_recompute` | PASS: 100, 0 failed, 8 ignored material gates. Binary counts: native_driver 60+8 ignored; assembled_run 12; fail_closed_cli 6; bundle_recompute 6; end_to_end_report 6; report_contract 6; each conformance binary 2. |
| `cargo clippy -p opi-eval --all-targets --locked -- -D warnings` | PASS after final Rust edits. |
| `cargo fmt --check --all` | PASS after final Rust edits. |
| `PYTHONDONTWRITEBYTECODE=1 python3 crates/opi-eval/scripts/test_native_integrations.py` (WSL) | PASS: 21, 1 skipped opt-in Docker outer-kill test. This is not a stage-deadline witness. |
| `PYTHONDONTWRITEBYTECODE=1 python3 crates/opi-eval/scripts/test_native_integrations.py NativeIntegrationTest.test_docker_outer_kill_preserves_partial_capture_without_terminal --docker` | PASS: 1 test, four family/product cases using real Docker. |
| `python crates/opi-eval/scripts/test_native_integrations.py` (Windows) | PASS: 15, 7 platform/opt-in skips. |
| `python crates/opi-eval/scripts/test_verify_native_smoke_artifact.py` | PASS: 17, including material/5 malformed/missing binding cases. |
| `python crates/opi-eval/scripts/test_native_smoke_output_control.py` | PASS: 2 producer/output-control cases. |
| `python crates/opi-eval/scripts/test_verify_native_smoke_ci.py` | PASS: 34. |
| `python scripts/opi-doc-check.py` | PASS, including a re-run after emitting this pair. |
| `git diff --check` | PASS, including a re-run after emitting this pair. |
| `python .agents/skills/_shared/scripts/validate_assurance_artifact.py plan docs/snapshots/phase19/assurance/remediation.plan.md` | PASS with the unchanged approved plan digest. |
| `python .agents/skills/_shared/scripts/validate_assurance_artifact.py result docs/snapshots/phase19/assurance/remediation.result.md` | PASS: exact source coverage, copied decisions, bounded incidentals and attributed paths. |
| `python .agents/skills/_shared/scripts/validate_assurance_artifact.py audit-set docs/snapshots/phase19/assurance` | PASS: current live indexed set and fixed remediation group. |

The current integration log is `/tmp/opi-remediate-integration-6hyoxf.log` in
WSL: Cargo and the wrapper returned 0, with 100 passing tests across eight
binaries. A shell-only attempt failed before Cargo because a non-login WSL
shell did not expose Cargo; rerunning through the configured login shell
produced this verified result. The prior continuation's 99-test result and I4
red/green remain earlier evidence, not the current test count.

The stage-deadline witness below was added and explicitly run against prepared
material. Its final result is FAIL as detailed above. The real regrade witness and
Python replay test remain NOT RUN and unimplemented. No full final-plan union,
workspace-wide gate or Phase exit is claimed.

```sh
OPI_EVAL_NATIVE_WITNESS_MATERIAL=/home/luiz/.cache/opi-native-19-mzcSc2/witness-material-cancel/material.json cargo test -p opi-eval --locked --test native_driver remediation_native_stage_deadlines -- --exact --ignored --nocapture --test-threads=1
# NOT RUN: B10 is dependency-blocked.
cargo test -p opi-eval --locked --test native_driver remediation_native_regrade_witnesses -- --exact --ignored --nocapture --test-threads=1
```

Additional current-state focused checks passed:

```sh
cargo test -p opi-eval --locked --test native_driver native_stage_configuration_drift_rejects_scores -- --exact --nocapture --test-threads=1
cargo test -p opi-eval --locked --test native_driver native_runner_timeout_keeps_partial_client_evidence_graded -- --exact --nocapture --test-threads=1
cargo test -p opi-eval --locked --lib native_verifier_timeout_binds_without_inventing_reward -- --nocapture
cargo test -p opi-eval --locked --test native_driver native_job_and_verifier_timeouts_keep_distinct_settlements -- --exact --nocapture --test-threads=1
PYTHONDONTWRITEBYTECODE=1 python3 crates/opi-eval/scripts/test_native_integrations.py NativeIntegrationTest.test_stage_cancellation_observes_exit_and_stops_descendants
```

The multiplier serialization, partial-client timeout, ungraded-verifier binding,
and I5 subprocess tests each have an observed failing run before their repair.
The final library run, clippy and formatting passed on this continuation's Rust
state. The default integration union excludes eight explicitly ignored material
gates; it cannot replace the failing real-stage command.

## Bounded incidentals and test impact

I1 adapts a DeepSWE test constructor to B3's private identity payload. I2 supplies
B5's two optional limits to the existing conformance request. I3 migrates the
same diagnostic caller to B9's private one-task map, preserving explicit
one-task support. I4 updates a test assertion to B6's earlier run-root rejection.
I5 preserves actual Agent exit and terminates the owned launch process group
before cleanup on runner cancellation; its real four-case subprocess regression
and Docker outer-kill test pass. Their machine records contain causal scope, exact failing/passing
commands, paths and all required literal guardrails. None changes public API,
durable formats, dependency graph, specification, authority, manifests, lockfiles,
ledger or public schemas beyond the separately approved planned work.

Test impact: **add** actual stage witness, cancellation/partial-client and
ungraded-verifier regressions, stage-cap propagation/drift, root-preservation/concurrency,
unknown-duration, complete-resource and per-task-binding matrices; **update**
material fixtures, producer/consumer checks and the reused-root assertion;
**retain** conformance, oracle isolation, one-task/pi-only, cancellation, bundle,
report and credential/provider classification controls; **delete** none. The
remaining native witness conditions and B10/B11 regressions are explicit remaining work.

## Materialization handoff

The result is a worktree-only handoff. The approved plan and both live audit
members retain their exact bytes. Fixes and assurance outputs are uncommitted,
the assurance directory is not clean, and external acceptance remains unresolved.
A later independent audit or reviewer re-run is not yet admitted: it requires
materialized fixes plus the current live set, a clean assurance directory,
resolution of external evidence, and any owning-workflow return. This workflow
neither invokes that audit nor grants Phase conformance.

Resume dependent implementation only after a new fixed plan explicitly scopes
verifier command cancellation/termination and its exact runner/task identity
boundary, then B5's complete real matrix passes. The current approved plan and
upstream pins were not changed retroactively. An environment adapter, upstream
patch/pin change or task-wrapper strategy needs that planning decision; none was
silently introduced under this apply.
