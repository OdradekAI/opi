# Phase 19 Remediation Apply Result

**Status**: COMPLETE
**Execution outcome**: BLOCKED; B1, B2 and B3 are Not closed.
**Audit index SHA-256**: `7550892cafbe6db2df1b56cd3f2eadb2104303428019940d4dc341e621f48f9d`
**Plan SHA-256**: `56b3599c0c7646c377d9c7bbe4716a297898303964d324cbf8b2bfdbd4273023`
**Remediation head**: `e169ab41d91b25870312088616ab1cfe759ef793`
**Disposition artifact**: `remediation.result.dispositions.jsonl`
**Changed paths**: ["CHANGELOG.md", "crates/opi-eval/README.md", "crates/opi-eval/README.zh.md", "crates/opi-eval/integrations/harbor/opi_eval_harbor.py", "crates/opi-eval/integrations/pier/opi_eval_pier.py", "crates/opi-eval/scripts/test_native_integrations.py", "crates/opi-eval/src/benchmark/process.rs", "crates/opi-eval/src/cli/regrade.rs", "crates/opi-eval/src/cli/run.rs", "crates/opi-eval/src/regrade.rs", "crates/opi-eval/src/report.rs", "crates/opi-eval/src/runner/experiment.rs", "crates/opi-eval/tests/bundle_recompute.rs", "crates/opi-eval/tests/end_to_end_report.rs", "crates/opi-eval/tests/native_driver.rs", "crates/opi-eval/tests/report_contract.rs", "crates/opi-eval/tests/report_output_containment.rs"]

`COMPLETE` is the required result-document status: all 13 approved dispositions
have final observations. It does not mean the three repair batches passed or
the Phase is conformant. The live indexed aggregate remains `FAIL`.

The contract requires each planned record's `changed_paths` to be copied
unchanged and this header to equal their union, even for undispatched batches.
The header therefore preserves plan attribution; it is not a claim that every
listed file was edited. The actual worktree inventory is recorded below.

## Approval and preservation

The explicit apply invocation approved exactly the plan digest above.
Admission verified that digest, the full committed HEAD, the active audit
index/member hashes, READY-FOR-APPLY status and observed red-before evidence.
The plan pair was the only carried-in dirty work: staged `[]`, unstaged
`["docs/snapshots/phase19/assurance/remediation.plan.md", "docs/snapshots/phase19/assurance/remediation.plan.dispositions.jsonl"]`,
untracked `[]`. Both result files were tracked and clean and were replaced
without consuming their prior conclusions. The carried-in plan bytes remain
unchanged. No commit, branch, push, ledger edit, manifest/lockfile change,
upstream pin change, registered-source edit or audit-member edit was made.

| Alias | Exact indexed run | Findings SHA-256 |
|---|---|---|
| C | `phase19-codex-gpt6-c18efa8-20260917t133157z` | `67c0ab6ca8b90178e3caf2d7cdd6cb4a6c9a6b2fe342c85d8cbd3ef105c95b36` |
| P | `phase19-pi-glm53-d61590b-20260917t152753z` | `b85fa7a405aee5d3da9ce410bcd9619cf55af85fb3bce75a4c8b1f9aae96bc0b` |

The sidecar copies every plan record exactly except `green_after.observed`
and final `remediation_status`. Source findings, evidence, severities, keys,
approved commands and path attribution are preserved. No incidental repair
was accepted. Iterations correcting the new B1 implementation's umask and
cancellation behavior remained part of B1; the external material defect was
diagnosed but not repaired.

## Dispositions

| Source / finding | Verification | Batch | Result |
|---|---|---|---|
| C / P19-AUD-001 | Refuted | - | Refuted |
| C / P19-AUD-002 | Confirmed | B2 | Not closed |
| C / P19-AUD-003 | Refuted | - | Refuted |
| C / P19-AUD-004 | Refuted | - | Refuted |
| C / P19-AUD-005 | Partially confirmed | B1 | Not closed |
| C / P19-AUD-006 | Refuted | - | Refuted |
| C / P19-AUD-007 | Refuted | - | Refuted |
| C / P19-AUD-008 | Refuted | - | Refuted |
| C / P19-AUD-009 | Refuted | - | Refuted |
| C / P19-AUD-010 | Cannot confirm | - | Cannot confirm |
| C / P19-AUD-011 | Confirmed | B3 | Not closed |
| C / P19-AUD-012 | Refuted | - | Refuted |
| P / P19-AUD-001 | Partially confirmed | - | Info/No action |

The eight Refuted findings retain their independent current-head
counterexamples; all eight named regressions passed in the final default
native_driver binary. C/010's 30 real acceptance obligations remain outside
claimed closure. The no-model witnesses and default tests do not supply
acceptance evidence. P/001's immutable Git statistics partially confirm the
consolidated-commit claim, without its stronger opi-ai attribution.

## B1 implementation and failed closure

The partial B1 change selects a private `OwnedDockerEnvironment` in the pinned
Harbor/Pier integration modules and binds its import path in persisted trial
configuration. Owned process groups retain command-exit observations through
timeout/cancellation; repeated cancellation settles bounded cleanup before
propagating. Exact-project escalation is used when termination is uncertain.
An effect-unknown marker cancels further dispatch and prevents sealing.
Teardown retains shared images. The local command wrapper preserves the
original command's umask. README changes are synchronized in both languages,
and the Unreleased entry describes the changed behavior.

The real closure check ran with newly byte-pinned module material:

```sh
OPI_EVAL_NATIVE_WITNESS_MATERIAL=/home/luiz/.cache/opi-remediate-plan19-fb8c5146d1fc/stage/material.json cargo test -p opi-eval --locked --test native_driver remediation_native_stage_deadlines -- --exact --ignored --nocapture --test-threads=1
```

Result: **FAIL**, Cargo exit 101 after 501.57 seconds (outer WSL invocation
reported exit 1). Evidence root: `/tmp/.tmpgp0X7F`; log: `b1-native-2.log`.
Material SHA-256:
`0252079a326ad393d98f96ce943ac9b7cac70497d8459e4453bb26fcf97fc2ac`.
The original red material and pinned modules were retained unchanged.

TB2.1 reached and passed the assertions for all nine cases and both no-model
subjects: positive, time-cap, agent-cap, verifier-cap, both-agent,
both-verifier, minimum-cap, strict-task and unbounded-task. Expected verifier
timeouts remained invalid with Unknown reward. TB3 positive then failed for
both subjects with `RewardFileNotFoundError`; native verifier stdout was:

```text
bash: line 1: /tests/test.sh: No such file or directory
```

The frozen separate-verifier task selects
`python@sha256:272efc3f94bec901e2bc7a5c9984b3d113c317839edca2714c6a388b9476f8fc`
and has no verifier-image build file providing that script. Pinned Harbor
`4407eb5227a2ff4f0d3f16b2eb48849382fdf276` constructs its separate verifier with
`skip_tests_upload=True` in `src/harbor/trial/trial.py:723-731`.
The direct offline image check also exited 1:

```sh
docker run --rm --pull never --network none python@sha256:272efc3f94bec901e2bc7a5c9984b3d113c317839edca2714c6a388b9476f8fc sh -c "test -f /tests/test.sh"
```

The previous approved red stopped in TB2.1 and never exercised this TB3
positive path. Replacing task/image packaging or adding an upload behavior
would exceed this plan's frozen task/image inputs and supervision-only
integration scope. No grader, task declaration, deadline or image was changed
to bypass the defect. The remaining eight TB3 cases and all DeepSWE cases
were not executed. B2 and B3 were not dispatched, as explicitly required by
the approved dependency order.

Both failed native result files are preserved under the evidence root:

| Subject | Relative native result path | SHA-256 |
|---|---|---|
| Opi | `terminal-bench-3.0/positive/run/trials/trial-opi-native/verifier-trace/jobs/2026-09-18__12-34-25/task-package__GybKsAB/result.json` | `a6b685caaf736ae7f43c06a0653eccce0db671de607d9f71a520b717d9fa9dda` |
| pi | `terminal-bench-3.0/positive/run/trials/trial-pi-native/verifier-trace/jobs/2026-09-18__12-34-49/task-package__wMCVXaP/result.json` | `454ab0c45fa16ef7a3ed9e8d28564e9627f47c9d3a3766cfe85297212da614fc` |

Exact compose working-directory label inspection covered 44 Agent/verifier
paths across this run and the earlier B1 iteration and found zero remaining
owned containers (`b1-failure-inspection.json`). No global prune, neighboring
container cleanup or cache removal was used. The first B1 iteration's normal
case exposed an over-restrictive command umask; it was corrected and followed
by the real run above. That first run is retained as `b1-native.log`.

Final source includes the repeated-stop cancellation propagation refinement
and additional native witness neighboring-container/shared-image assertions
that postdate the failing real matrix. The refinement passed focused Python
tests; the assertions compiled in the final native_driver binary but remain
inside an ignored native test. Neither has complete final-state real-matrix
coverage. The eighteen earlier TB2.1 subject observations must not be promoted
to a full final-source native closure claim.

## Actual change inventory

These nine B1-owned files were modified:

- `CHANGELOG.md`
- `crates/opi-eval/README.md`
- `crates/opi-eval/README.zh.md`
- `crates/opi-eval/integrations/harbor/opi_eval_harbor.py`
- `crates/opi-eval/integrations/pier/opi_eval_pier.py`
- `crates/opi-eval/scripts/test_native_integrations.py`
- `crates/opi-eval/src/benchmark/process.rs`
- `crates/opi-eval/src/runner/experiment.rs`
- `crates/opi-eval/tests/native_driver.rs`

The two fixed result files were replaced for this invocation. The two dirty
plan files are carried-in work, preserved byte-for-byte. Final staged and
untracked inventories remain empty. These other paths retained in the
contract-required attribution union were not edited:

- `crates/opi-eval/src/cli/regrade.rs`
- `crates/opi-eval/src/cli/run.rs`
- `crates/opi-eval/src/regrade.rs`
- `crates/opi-eval/src/report.rs`
- `crates/opi-eval/tests/bundle_recompute.rs`
- `crates/opi-eval/tests/end_to_end_report.rs`
- `crates/opi-eval/tests/report_contract.rs`
- `crates/opi-eval/tests/report_output_containment.rs`

## Verification

Checks ran in Ubuntu WSL from `/mnt/d/Luiz/Odradek/opi`, with the existing
external Cargo cache `/mnt/e/opi/cargo-targets/opi-8abcf38f9871653e`,
`CARGO_INCREMENTAL=1`, and explicit cache leases through `check.sh`.
No per-session target directory, Cargo clean or provider call was used.
Documentation/assurance validators ran from the Windows repository root.

| Exact command | Observed result |
|---|---|
| `cargo fmt --check --all` | PASS, exit 0 |
| `cargo clippy -p opi-eval --all-targets --locked -- -D warnings` | PASS, exit 0 |
| `cargo test -p opi-eval --all-targets --locked` | PASS, exit 0; 300 passed, 8 explicit native opt-in tests ignored, 0 failed; native_driver 61 passed / 8 ignored |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py'` | PASS, exit 0; 213 tests, OK with 1 Docker opt-in skip |
| `python3 crates/opi-eval/scripts/test_native_integrations.py --docker` | PASS, exit 0; 28 tests, no skips |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p test_native_integrations.py` | PASS in the earlier B1 state; 25 tests, 1 skip; superseded by final full discovery and explicit Docker run |
| `python3 crates/opi-eval/scripts/test_native_integrations.py NativeEnvironmentTest` | PASS at final source; 6 tests (see focused log); new cancellation/umask/teardown cases were observed red before their B1 fixes |
| `cargo test -p opi-eval --locked --test native_driver native_stage_configuration_drift_rejects_scores -- --exact --nocapture` | PASS, 1 test; also passed in final full suite |
| `cargo test -p opi-eval --locked --test native_driver native_unobserved_command_exit_stops_dispatch_and_sealing -- --exact --nocapture` | PASS, 1 test; also passed in final full suite |

The following approved Refuted-finding filters were covered by the final
all-targets superset, without redundant per-filter invocations:

| Source / finding | Approved filter command (passed within superset) |
|---|---|
| C / P19-AUD-001 | `cargo test -p opi-eval --locked --test native_driver remediation_001_regrade_child_has_no_ambient_secret -- --exact --nocapture` |
| C / P19-AUD-003 | `cargo test -p opi-eval --locked --test native_driver remediation_003_substituted_terminal_identity_rejects -- --exact --nocapture` |
| C / P19-AUD-004 | `cargo test -p opi-eval --locked --test native_driver remediation_004_native_provider_error_is_not_scored -- --exact --nocapture` |
| C / P19-AUD-006 | `cargo test -p opi-eval --locked --test native_driver remediation_006_plan_material_rejects_without_panic -- --exact --nocapture` |
| C / P19-AUD-007 | `cargo test -p opi-eval --locked --test native_driver remediation_007_prepare_rejects_impossible_storage -- --exact --nocapture` |
| C / P19-AUD-008 | `cargo test -p opi-eval --locked --test native_driver remediation_008_feasible_second_task_can_freeze -- --exact --nocapture` |
| C / P19-AUD-009 | `cargo test -p opi-eval --locked --test native_driver remediation_009_rejected_rerun_preserves_control -- --exact --nocapture` |
| C / P19-AUD-012 | `cargo test -p opi-eval --locked --test native_driver remediation_012_native_unobserved_time_is_unknown -- --exact --nocapture` |

The exact provenance command also passed, inspecting commit statistics only:

```sh
git show --format=fuller --stat 4bf5fc0 -- crates/opi-agent crates/opi-ai crates/opi-coding-agent docs/audits/phase19-local-native-acceptance.md docs/audits/phase19-local-native-acceptance.zh.md crates/opi-eval/scripts/verify-local-acceptance.py
```

Result/plan/audit revalidation and documentation/diff checks are recorded in
the final validation section below. Workspace clippy, workspace tests, doc
tests, rustdoc and real-provider acceptance were not run or claimed; this is
the approved unpublished-Companion gate, not Phase exit.

Test impact: `add` six Python environment tests and the Rust effect-unknown
dispatch/seal regression; `update` native stage/configuration witness and
stand-in config; `retain` eight counterexample regressions and other tests;
`delete` none. B2/B3 planned new tests were not added. The result workflow has
no Chinese counterpart; user-facing README documentation was synchronized.

## Preserved local evidence

The following files are under `E:/opi/remediate-apply19-56b3599c/` (WSL:
`/mnt/e/opi/remediate-apply19-56b3599c/`). Native artifact roots are outside the
repository and remain local evidence; no external report or eval archive was
substituted for the live indexed inputs.

| File | SHA-256 |
|---|---|
| `b1-native.log` | `c1dd601420a160834b820275c29220512c043c270d77fa6dcb0f3c39ec71197b` |
| `b1-native-2.log` | `d989127abbcb7aea527b563e1caebc95879ca8a9739ab2a4f187faccb3569fe4` |
| `b1-failure-inspection.json` | `88a4c76dff2455e87a6aa99b5b806c89094e1dd68ae7ba29f01627e0a06f8ccf` |
| `b1-config.log` | `eec4a6c566e94d48c2790ace2342f43c4740249a05254e012622cd2c11807f0a` |
| `b1-unknown.log` | `7c71bb558ca5ed297a0ec7733dacec36ad3d1c7e77c77c66cc7119c418297486` |
| `b1-python.log` | `758d228f58e7fd52239f812174c82ffe0870b0bbb1c396787cae50529495dc80` |
| `b1-python-focused-final.log` | `85f6b1abd459f6b5459758013e1707e70fde9b78585f461867a088cdbb8b302c` |
| `b1-python-docker-final.log` | `255ed6dc0c350dfd8d201daa4e315c1a7588a7c09f692d0a7ce29ddd77b6db85` |
| `apply-crate-tests.log` | `d53fe6aeffec5c5727bdfa6d8d0fd1621fad1e669ac040d341ad0161b05028f1` |
| `apply-python-tests.log` | `e65d73f399fcd79ba3997ec3a17d215ee2dabcca82594f73106337976d510f75` |
| `apply-clippy.log` | `1b8af9d16608d683ef396cb4b99e916c525d99f1819918a58188e0e282e622ea` |
| `fmt.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `check.sh` | `78f2ac00d834bc236026fd0fca90ef07670505f9a4dddf844ebc011424aca42e` |
| `prepare_stage.py` | `1af8a29ce904a535e560e3459c728dc479f26e58c40c09b6a313298a8c3ed577` |
| `refresh_stage.py` | `aac6e88a1652631f522eb0c8d4a27e8ee41896cee25f4ad67495aeb2e82630e7` |
| `inspect_stage_failure.py` | `a1d18ff7c13926ed672e6abb85fd7e86b7b973b8181c21eca70d20191dc35834` |

## Final artifact validation

The following commands completed after writing the result pair:

| Exact command | Observed result |
|---|---|
| `python .agents/skills/_shared/scripts/validate_assurance_artifact.py dispositions docs/snapshots/phase19/assurance/remediation.result.dispositions.jsonl` | PASS; exact 13-record result coverage |
| `python .agents/skills/_shared/scripts/validate_assurance_artifact.py result docs/snapshots/phase19/assurance/remediation.result.md` | PASS; statuses remain three Not closed, eight Refuted, one Cannot confirm and one Info/No action |
| `python .agents/skills/_shared/scripts/validate_assurance_artifact.py audit-set docs/snapshots/phase19/assurance` | PASS; this validates the unchanged indexed artifacts, whose aggregate verdict remains FAIL |
| `python .agents/skills/_shared/scripts/validate_assurance_artifact.py plan docs/snapshots/phase19/assurance/remediation.plan.md` | PASS; emitted the exact approved digest `56b3599c0c7646c377d9c7bbe4716a297898303964d324cbf8b2bfdbd4273023` |
| `python scripts/opi-doc-check.py` | PASS |
| `git diff --check` | PASS, exit 0 |
| `git status --short` | Only nine B1 files, two carried-in plan files and two renewed result files are modified; no staged or untracked paths |

Validation certifies the result's identity and coverage, not native closure.

## Materialization boundary

Apply stops with partial B1 changes and validated result coverage. The
execution protocol requires: "New evidence that changes public API, durable
format, dependencies, specification, authority, or planned scope returns to
`mode=plan`." A new plan must admit suitable separate-verifier image/task
packaging and establish all required native baseline cases before approving
another apply. This invocation does not revise the approved plan, dispatch
B2/B3, invoke another explicit-only skill or grant Phase PASS.

No changes were committed. A fresh audit/reviewer run remains inadmissible
until fixes plus the current live assurance set are committed, the assurance
directory is clean, external acceptance evidence is resolved, and any required
owning-workflow return is complete.
