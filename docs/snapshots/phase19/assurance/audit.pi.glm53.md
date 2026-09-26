# Phase 19 Audit

**Audit run ID**: `phase19-pi-glm53-e079780-20260926t192024z`
**Audit head**: `e079780bda6681ec466f79bd15f3248baf0f2eb8`
**Reviewer ID**: `pi`
**Model ID**: `glm53`
**Reviewer identity**: pi terminal coding agent (earendil-works pi)
**Reviewer model ID**: `glm-5.3`
**Model identity source**: request-config
**Independence**: fresh-context-same-family — fresh task context; no prior audit, remediation, history, or sibling-peer output read; same reviewer/model family as the superseded `audit.pi.glm53` entry
**Baseline policy**: latest-committed-spec
**Verdict**: FAIL

Operator-pinned baseline: the supplied evidence root (`E:\opi\evaluations\phase19-codex-gpt6-e079780-20260926`, acceptance tree `fresh-portable/`) seals `audit_head` `e079780` in its manifest and `identity.json`; `e079780` is a committed ancestor of HEAD and differs from HEAD only in assurance-set files.

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| .opi-impl-state.json | c65c10635c4bb8b1754625b94e8a0a15459eef6d65634610ba9b65ea9ba5337e | current committed source (reader projection only) |
| docs/snapshots/phase19/opi-impl-state.json | 35b3378bd902cc976bb348d794dd1b5d3c5f160a58769c3aeb62fea175a7b7a9 | current committed source (reader projection only) |
| docs/opi-spec.md | 1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db | current committed source |
| docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md | 5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a | current committed source |
| docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.zh.md | d41c826412b0d71802327d60f953708f6cb2083436a2a2b0d99a2e52d851f194 | current committed source |

No stored hash mismatches. All seals were written before production inspection; requirement identities were not revised afterwards.

## Requirement Conformance

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| P19-AC-002 | P19-AC-002 §7; 19.1 | `import_bound_native_job` + 7-mode substitution test (exact failure kinds, no seal) | met | — |
| P19-AC-003-native | P19-AC-003 §7; 19.1 | auditor-run `native_tb21_artifact_witness --ignored --exact`: pass | met | — |
| P19-AC-008-outcome | P19-AC-008 §7; 19.1 | `native_trial_outcome_classifies_every_terminal_shape` + report consumers | met | — |
| P19-AC-006-fixture | P19-AC-006 §7; 19.2 | auditor-run protocol/turn-budget/output-control client matrices: pass | met | — |
| P19-AC-010-credentials | P19-AC-010 §7; 19.2 | `authority_boundaries` suite; sealed receipts `credential-material:removed` | met | — |
| P19-AC-004-native | P19-AC-004 §7; 19.3 | auditor-run `native_tb3_artifact_witness --ignored --exact`: pass | met | — |
| P19-AC-005-native | P19-AC-005 §7; 19.4 | auditor-run `native_deepswe_patch_witness --ignored --exact`: pass | met | — |
| P19-AC-001-prepare | P19-AC-001 §7; 19.5 | `independent_directory_prepare`, `fail_closed_cli`, 228 python tests | met | — |
| P19-AC-007-material | P19-AC-007 §7; 19.5 | `local_material_integrity` + package/module drift rejection tests | met | — |
| P19-AC-007-selection | P19-AC-007 §7; 19.6 | `full_inventory_preflight`, `frozen_subset_execution` | met | — |
| P19-AC-001-plan | P19-AC-001 §7; 19.6 | `frozen_subset_execution` (plan+run from independent dir); declared filter drifted | met | P19-AUD-003 |
| P19-AC-008 | P19-AC-008 §7; 19.7 | report suites + auditor-run real-data reports (honest coverage) | met | — |
| P19-AC-009-verify | P19-AC-009 §7; 19.7 | `bundle_recompute` etc. + auditor-run `verify`/`report` on 3 real family roots: exit 0 | met | — |
| P19-AC-009-regrade | P19-AC-009 §7; 19.8 | `native_offline_regrade` (3 families) in default suite; declared flags drifted; long witness variant incomplete | met | P19-AUD-003, P19-AUD-004 |
| P19-AC-010-lifecycle | P19-AC-010 §7; 19.9 | cancellation/recovery suites + auditor-run `native_owned_resource_cancellation` | met | P19-AUD-005 |
| P19-AC-011 | P19-AC-011 §7; 19.10 | `fail_closed_cli`; 228 python tests deterministic | met | — |
| P19-AC-013 | P19-AC-013 §7; 19.10 | no internal deps, publish=false, package list, bilingual READMEs, Unreleased entries | met | — |
| P19-AC-001-live | P19-AC-001 §7; 19.11 | workflow assembly + offline reconstruction evidenced; acceptance verifier exits 2 | partially-met | P19-AUD-001 |
| P19-AC-003-live | P19-AC-003 §7; 19.11 | pi trial passes acceptance predicate; opi trial stream truncated (trace manifest sealed) | partially-met | P19-AUD-001 |
| P19-AC-004-live | P19-AC-004 §7; 19.11 | both TB3 trials crossed verifier boundary; pi trial invalid-execution | partially-met | P19-AUD-001 |
| P19-AC-005-live | P19-AC-005 §7; 19.11 | both DeepSWE trials graded natively; pi trial invalid-execution | partially-met | P19-AUD-001 |
| P19-AC-006-live | P19-AC-006 §7; 19.11 | output-control snapshots sealed in all six bundles; 5/6 live streams truncated | partially-met | P19-AUD-001, P19-AUD-002 |
| P19-AC-012 | P19-AC-012 §7; 19.11 | acceptance verifier rejects root; two pi trials invalid-execution; 5/6 streams truncated | not-met | P19-AUD-001, P19-AUD-002 |

## Standards Review

opi-eval declares no internal workspace dependencies, `publish = false`, edition/rust-version via workspace: Independent Companion placement holds (PLACE-001..004, CTRL-007). `cargo clippy -p opi-eval --all-targets -- -D warnings` and `cargo fmt --check -p opi-eval` are clean at the sealed head; `scripts/opi-doc-check.py` passes. Zero TODO/FIXME/todo! markers in `crates/opi-eval/src`. CHANGELOG `[Unreleased]` records the deliberate 0.x schema cutover (material/5, plan/2, report/2, regrade/2, prepare args) with updated callers — no compatibility shims. Bilingual READMEs ship in the package list.

## Spec Review

Deterministic obligations (19.1–19.10) trace to typed production seams and discriminating tests (see table). The live obligations (19.11) fail on the supplied evidence: `verify-local-acceptance.py` exits 2. The product's own reporting behaves conformantly under CTRL-005/CTRL-006 — invalid trials are classified `invalid-execution`, pairs marked non-comparable, no invented scores, offline verify/report succeed on every family root — but P19-AC-012 requires six *valid* real trials and the root carries at most four (two pi trials are product-classified invalid; five of six event streams are cap-truncated). Per the spec ("Until actual evidence is present and verified, P19-AC-012 remains not met and Phase exit is unavailable"), Phase exit evidence is unavailable at this head.

## Security, Invariants, Integration, Test Quality, and Residuals

- **Security/authority**: credential canaries, env-marker filtering, fail-closed CLI, and credential-material removal are test-enforced (`authority_boundaries`, `fail_closed_cli`) and visible in real receipts. No secret-shaped content crossed into my companion invocations (verifier filters credential markers from the spawned environment).
- **Invariants**: native binding (`NativeBindingError`) rejects every substitution class even on exit 0; the single classifier lives at the `run_trial` join; reward domains are typed.
- **Integration**: the one residual integration defect is the failure-path container orphaning (P19-AUD-005): harbor compose-down fails after its temp project dir is removed and the leaked environment container carries no trial-owned label, so owned-container cleanup cannot see it. Passing/cancelling paths leave zero containers (auditor-verified `docker ps -aq` empty after each run).
- **Test quality**: 328 Rust tests pass (0 failed, 9 ignored); I additionally ran seven of the nine ignored native tests to green (tb21/tb3/deepswe witnesses, protocol/turn-budget/output-control client matrices, owned-resource cancellation) against the operator's pinned materials. Two ledger-declared verification commands are vacuous as written (P19-AUD-003). The two longest ignored tests panic opaquely on single-family material and exceeded the audit time budget (P19-AUD-004).
- **Residuals**: no dead branches or duplicate grading paths found; one classifier, one supervisor, one verifier-only regrade entry.

## Minimum-change Conformance

`reuse_search`/`surface_necessity`/`simplification_ceiling` are not recorded in the ledger projection (`not-recorded`); I performed direct current inspection instead. All 11 tasks: `conforming` — each task's owned paths exist at the head, changes trace to its scenario obligations, no speculative abstraction/feature flag was added, the 0.x cutover replaced (not appended) the former default/oracle scoring path, and no second classifier or competing grading route remains. 19.11's code surface (the offline verifier) conforms; its *evidence* outcome is the not-met requirement above, which is a finding, not a scope drift.

## Findings

### P19-AUD-001: Six-trial local acceptance evidence is rejected by the offline verifier

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-012, P19-AC-001-live, P19-AC-003-live, P19-AC-004-live, P19-AC-005-live, P19-AC-006-live
- Claim: the supplied acceptance root does not verify; fewer than six valid real trials exist; Phase exit evidence is unavailable at `e079780`.
- Evidence: pi DeepSWE/TB3 trials sealed with `failure_kind=import-evidence-incomplete`, `stdout_truncated=true`, stdout exactly 1048576 bytes; run reports classify both pi trials `invalid-execution` with non-comparable pairs; 5/6 streams truncated; `attempt-history.json` shows prior attempts with the same causes and no valid replacements.
- Refutation attempted: ran the built Companion `verify`/`report` on all three family roots (all exit 0 — bundles and offline reconstruction are sound, so the rejection is not a bundle-integrity artifact); replayed the verifier's agent-observation predicate per trial (tb21-pi passes; the failures are exactly truncation/terminal-fact violations, not verifier over-strictness for the pi trials the product itself classifies invalid); confirmed the manifest's six-trial/binding structure is otherwise intact. The product's own invalid-execution classification defeats any claim that the two pi trials are valid real trials.
- Suggested closure: complete six valid real trials (after P19-AUD-002) and re-submit a redacted root that `verify-local-acceptance.py` accepts.

### P19-AUD-002: Pinned 1 MiB agent stdout capture ceiling invalidates real trials on TB3/DeepSWE

- Axis: invariants
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-012, P19-AC-006-live
- Claim: the repo-pinned 1 MiB agent event-stream cap structurally truncates real Agent streams on the selected TB3/DeepSWE tasks; the pi adapter then fail-closes the trial to invalid-execution, so the acceptance loop cannot complete without a repository change.
- Evidence: `profiles/agents/pi.toml:28` / `profiles/agents/opi.toml:25` (`stdout_cap_bytes = 1048576`), embedded via `include_str!`; `agent/pi.rs` `parse_stream` rejects truncated streams as `import-evidence-incomplete`; five of six real streams are exactly 1048576 bytes.
- Refutation attempted: searched for a runtime override of the agent spawn caps (none — `SpawnSpec` caps come only from the embedded declarative profile); considered that opi's authoritative evidence is its on-disk trace manifest (true, and why opi trials still grade) — but the pi event stream *is* the trajectory evidence, so truncation loses the required binding for pi; the fail-closed classification itself is conformant, the ceiling is the defect.
- Suggested closure: raise the agent stream caps for real trials and/or stream pi events to a file artifact instead of a bounded stdout capture, then re-run acceptance.

### P19-AUD-003: Ledger-declared verification filters drifted to vacuous commands

- Axis: test-quality
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: P19-AC-001-plan, P19-AC-009-regrade
- Claim: `independent_plan_run` names no existing test, and `native_offline_regrade` is declared with `--ignored` although the test is no longer ignored; both declared commands select zero tests as written.
- Evidence: filter runs report `0 passed; N filtered out`; the behaviors are covered by `frozen_subset_execution`, `hermetic_paired_group_runs_end_to_end`, and the default-suite `native_offline_regrade`.
- Suggested closure: correct the ledger's verification routes in the owning workflow.

### P19-AUD-004: Witness-material helper panics opaquely on single-family material; two long ignored tests lack saved run evidence

- Axis: test-quality
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: P19-AC-009-regrade
- Claim: `native_witness_material.rs` unwraps the three-family benchmark map without a diagnostic, so single-family material panics with a bare `Option::unwrap()` message; `native_verifier_only_regrade_witness` exceeded a 1500 s auditor timeout and no preserved operator log was located for either long test.
- Evidence: panic at `tests/support/native_witness_material.rs:97` under tb21-only material; timed-out auditor run under combined material.
- Suggested closure: fail with a named error listing missing families; preserve a run log for the two long ignored tests or bound their material requirements.

### P19-AUD-005: Failed native witness runs orphaned task environment containers

- Axis: integration
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: P19-AC-010-lifecycle
- Claim: on the failed nop-witness attempts, harbor compose-down failed after its temp project directory removal and the environment container survived; it carries no opi-eval trial-owned label, so owned-container cleanup reports none-found.
- Evidence: two captured containers `Status=Running` after the failed attempts (`native-tb21-nop-cleanup.json`, `native-tb21-nop-retry-cleanup.json`); compose-down `getwd: no such file or directory` in the preserved stderr; both later removed manually.
- Suggested closure: surface runner-reported compose-down failures as retained cleanup diagnostics and consider labeling or sweeping task-environment containers created under the trial root.

## Verification Commands

| Command | Result | Requirement/finding |
|---|---|---|
| `python3 .agents/skills/_shared/scripts/validate_assurance_artifact.py rotation docs/snapshots/phase19` | PASS | admission |
| `python3 .agents/skills/opi-audit/scripts/read_audit_scope.py --phase 19 --head e079780…` | PASS (no mismatches) | baseline seal |
| `cargo test -p opi-eval --all-targets` (sealed export) | PASS 328/0/9 ignored | 19.1–19.10 requirements |
| `cargo test -p opi-eval --test native_driver native_tb21_artifact_witness -- --ignored --exact` | PASS (235.54s) | P19-AC-003-native |
| `cargo test -p opi-eval --test native_driver native_tb3_artifact_witness -- --ignored --exact` | PASS (675.32s) | P19-AC-004-native |
| `cargo test -p opi-eval --test native_driver native_deepswe_patch_witness -- --ignored --exact` | PASS (327.56s) | P19-AC-005-native |
| `cargo test -p opi-eval --test native_driver native_protocol_client_matrix -- --ignored --exact` | PASS (222.45s) | P19-AC-006-fixture |
| `cargo test -p opi-eval --test native_driver native_client_turn_budgets -- --ignored --exact` | PASS (27.45s) | P19-AC-006-fixture |
| `cargo test -p opi-eval --test native_driver native_output_control_client_matrix -- --ignored --exact` | PASS (18.99s) | P19-AC-006-fixture |
| `cargo test -p opi-eval --test native_driver native_owned_resource_cancellation -- --ignored --exact` | PASS (2.87s) | P19-AC-010-lifecycle |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py'` | PASS 228 (1 skip) | P19-AC-011, P19-AC-001-prepare |
| `python3 scripts/opi-doc-check.py` | PASS | P19-AC-013 |
| `cargo clippy -p opi-eval --all-targets -- -D warnings` / `cargo fmt --check -p opi-eval` | clean | standards |
| `cargo metadata --no-deps` / `cargo package -p opi-eval --list --allow-dirty` | PASS (185 files) | P19-AC-013 |
| `python3 crates/opi-eval/scripts/verify-local-acceptance.py --root <fresh-portable> --out <audit-out> --companion <built opi-eval>` | **REJECTED (exit 2)** | P19-AUD-001, P19-AC-012 |
| `opi-eval verify` / `opi-eval report` on the three `fresh-portable/runs/<family>` roots | PASS (exit 0 each) | P19-AC-009-verify, AC-008 |
| `cargo test -p opi-eval --test native_driver native_verifier_only_regrade_witness -- --ignored --exact` | NOT COMPLETED (>1500s) | P19-AUD-004 |

## Verdict Rationale

Mandatory requirement `P19-AC-012` is `not-met` and five live scenarios are `partially-met` because the supplied six-trial acceptance evidence is rejected by the declared offline verifier (root cause: the repo-pinned 1 MiB agent stream cap truncates five of six real streams and invalidates the two pi TB3/DeepSWE trials). Every deterministic 19.1–19.10 obligation is met with current, auditor-re-run evidence. With a mandatory requirement not met, the member verdict is FAIL.
