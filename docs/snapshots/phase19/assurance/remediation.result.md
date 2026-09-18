# Phase 19 remediation result

**Status**: COMPLETE
**Audit index SHA-256**: `702853fe37c1779c38612f7979a7e995395d1890954c4d3d8d6205592625e6d3`
**Plan SHA-256**: `4a7b348b2560ec94dceb2a7bc1890b412337f4fded47d190a2eefcd996b3ea2a`
**Changed paths**: ["CHANGELOG.md", "crates/opi-eval/README.md", "crates/opi-eval/README.zh.md", "crates/opi-eval/examples/local-evaluation/README.md", "crates/opi-eval/external-locks/static/linux-x86_64.json", "crates/opi-eval/scripts/build-agent-artifacts.sh", "crates/opi-eval/scripts/materialize-external-locks.sh", "crates/opi-eval/scripts/native-smoke.sh", "crates/opi-eval/scripts/scripted-provider.py", "crates/opi-eval/scripts/test_prepare_acquisition.py", "crates/opi-eval/scripts/test_verify_local_acceptance.py", "crates/opi-eval/scripts/verify-local-acceptance.py", "crates/opi-eval/src/cli/prepare.rs", "crates/opi-eval/src/experiment.rs", "crates/opi-eval/src/main.rs", "crates/opi-eval/src/report.rs", "crates/opi-eval/src/runner/experiment.rs", "crates/opi-eval/src/runner/plan.rs", "crates/opi-eval/src/runner/prepare.rs", "crates/opi-eval/tests/assembled_run.rs", "crates/opi-eval/tests/experiment_contract.rs", "crates/opi-eval/tests/fixtures/external-locks/local-cache/scripts/materialize-external-locks.sh", "crates/opi-eval/tests/native_driver.rs", "crates/opi-eval/tests/pairing_and_integrity.rs", "crates/opi-eval/tests/report_contract.rs", "crates/opi-eval/tests/support/native_witness_material.rs"]
**Remediation head**: `2b6538a1b0bca0146ebaeac2d80352a5929fe22b`
**Result dispositions**: `remediation.result.dispositions.jsonl`

This apply concluded at the approved execution protocol's stop boundary: **8 Closed,
3 Not closed**, covering all 11 indexed source keys. COMPLETE describes the result
artifact contract; it does not mean every batch passed, real acceptance succeeded,
or Phase 19 is conformant. There are no incidental-repair records.

## Binding and preserved baseline

The active audit set and exact approved plan digest were revalidated before the
result was written. The current HEAD is unchanged. The carried-in modifications
to `remediation.plan.md` and `remediation.plan.dispositions.jsonl` were preserved
byte-for-byte; their combined approval digest is still the value above. Neither
indexed audit member, the specification, implementation ledger, Cargo manifests,
Cargo.lock, nor upstream commit/image/workspace-input pins were edited.

The result sidecar copies all planned records without changing source identity,
source evidence, verification decisions, closure scope, red-before observations,
or the green command/expected predicate. Only green observations and final
remediation status differ. Source IDs with identical textual IDs remain distinct.

## Closure results

| Batch | Indexed member / finding | Status | Applied outcome and evidence |
| --- | --- | --- | --- |
| B1 | pi / P19-AUD-001 | Closed | Both direct entrypoints carry Git mode 100755; exact Linux execution checks pass. |
| B2 | codex / P19-AUD-007 | Closed | Privacy failure cancels dispatch before the next wave while retaining planned missing coverage. |
| B3 | codex / P19-AUD-003 | Closed | Required unsupported sampling controls reject before native dispatch; optional controls remain explicitly unsupported. |
| B4 | codex / P19-AUD-004 | Closed | Offline reports reconstruct output-control coverage from sealed evidence without trusting mutable reports or changing original bundles. |
| B5 | codex / P19-AUD-005 | Closed | Recovery is an immutable classification over the frozen inventory, including unstarted trials. |
| B6 | codex / P19-AUD-006 | Closed | Replacement uses a validated resolved pair, fresh identities and preserved old roots. |
| B7 | codex / P19-AUD-008 | Closed | Acceptance rejects fixture substitution and requires the sealed live connection and client observations. |
| B8 | codex / P19-AUD-002 | Closed | Acquisition uses bounded supervision, environment filtering, cancellation and owned staging cleanup. |
| B9 | codex / P19-AUD-001 | Not closed | Selected-material acquisition and deterministic checks are implemented; mandatory real preparation failed on the approved source/lock input conflict below. |
| B10 | pi / P19-AUD-002 | Not closed | Not started: depends on successful B9/D3 preparation. |
| B11 | codex / P19-AUD-009 | Not closed | Not started: depends on B9/D3 and B10; current native union and final workspace gates remain unverified. |

The full member names and findings digests are retained in the sidecar. No source
severity was downgraded. No current or historical live trial was ingested as a
substitute for the missing native evidence.

## B9/D3 blocking evidence

Public prepare ran from `/tmp/opi-p19-native-apply.jr3czuc0/unrelated` with this exact invocation:

```sh
/mnt/e/opi/cargo-targets/opi-8abcf38f9871653e/debug/opi-eval prepare --config /tmp/opi-p19-native-apply.jr3czuc0/inputs/terminal-bench-2.1.toml --out /tmp/opi-p19-native-apply.jr3czuc0/materials/terminal-bench-2.1 --opi-source /mnt/d/Luiz/Odradek/opi --opi-commit 2b6538a1b0bca0146ebaeac2d80352a5929fe22b
```

The Companion exited **2** after its supervised materializer exited **1**:
`prepare: Opi Cargo.lock differs from the declared workspace input`.
The retained acquisition log records `cleanup: NotRequired`,
`staging_removed: true`, and both truncation fields `false`. No material manifest
was published. No Opi/pi build, benchmark container, native grader or model request
was started by this real-material attempt. A verified uv download remains in the
private cache. Existing immutable images were inspected successfully; no image
pull, removal or shared-image modification was needed.

| Input | Exact value |
| --- | --- |
| Approved Opi commit | `2b6538a1b0bca0146ebaeac2d80352a5929fe22b` |
| Committed Cargo.lock Git blob | `b5322d4d226622d1f16567254351881e1abbf62c` |
| Committed Cargo.lock bytes | 112436 |
| Committed Cargo.lock SHA-256 | `261a3ec501be2e045dc6c9b2b9a940adc1c56305f477008eea940f26545925f3` |
| LF-normalized Cargo.lock SHA-256 | `261a3ec501be2e045dc6c9b2b9a940adc1c56305f477008eea940f26545925f3` |
| Unchanged static-lock workspace input | `d19619f4c05b0bb00f324345f8103d615be6bcae0462cff5b9b8dbf3cce0cd2a` |
| Current Companion executable SHA-256 | `7c701e4323f6e88f8121db9279e42cad8331e1d7b2d9933095858163ea9f4e56` |

This is an exact input-version conflict, not a CRLF artifact. D3 fixes the Opi
commit while B9 explicitly preserves Cargo.lock and workspace-input pins. Changing
either approved input is outside this apply. The [execution protocol](../../../../.agents/skills/opi-remediate/references/execution-protocol.md)
requires: "Proceed only after the prior batch passes" and returns evidence that
changes planned scope to `mode=plan`. Therefore the input pair needs a new plan;
this result neither changes the approved plan retroactively nor treats the input
conflict as an authorized incidental repair.

Raw native evidence is retained under `/tmp/opi-p19-native-apply.jr3czuc0`:
`inputs/prepare-terminal-bench-2.1-command.json`, `inputs/prepare-results.json`,
`inputs/source-lock-comparison.json`, `inputs/companion.json`,
`inputs/image-acquisition.json`, `logs/prepare-terminal-bench-2.1.stderr`, and
`materials/terminal-bench-2.1/acquisition.log`. Other family configurations were
prepared, but their public prepare invocations were not run after this failure.

## Verification actually executed

Rust and shell/Python runtime checks used the unique ext4 archive export
`/tmp/opi-p19-remediation-apply.sbE3TAxT`, overlaid only with approved changes and intended executable
modes. Cargo retained `CARGO_INCREMENTAL=1` and the shared external target
`/mnt/e/opi/cargo-targets/opi-8abcf38f9871653e`; no cache was deleted. Windows
DrvFs modes were not used to establish Linux execution correctness.

Log names below are relative to that export. Full command/expected predicates
remain in the result sidecar; raw logs, current source hashes, Git index modes,
Companion identity and the source diff are hashed by the retained evidence index.

| Command actually run | Result | Raw log |
| --- | --- | --- |
| `cargo test -p opi-eval --locked --test agent_integration_conformance agent_conformance_matrix_settles_every_pinned_case -- --exact --nocapture` | PASS, 1 | `b1-conformance-current.log` |
| `cargo test -p opi-eval --locked --test native_driver supervised_acquisition_runs_the_pinned_materializer -- --exact --nocapture` | PASS, 1 | `b1-acquisition.log` |
| `cargo test -p opi-eval --locked --test native_driver p19_plan_privacy_failure_stops_dispatch -- --exact --nocapture` | PASS, 1 | `b2-green.log` |
| `cargo test -p opi-eval --locked --test native_driver p19_plan_required_sampling_rejects_before_dispatch -- --exact --nocapture` | PASS, 1 | `b3-required.log` |
| `cargo test -p opi-eval --locked --test experiment_contract` | PASS, 34 | `b3-experiment.log` |
| `cargo test -p opi-eval --locked --test native_driver p19_plan_offline_preserves_unsupported_output -- --exact --nocapture` | PASS, 1 | `b4-focused.log` |
| `cargo test -p opi-eval --locked --test report_contract --test report_output_containment --test end_to_end_report` | PASS, 26 total | `b4-report.log` |
| `cargo test -p opi-eval --locked --lib report::tests` | PASS, 4 | `b4-unit.log` |
| `cargo test -p opi-eval --locked --test assembled_run recovery_ -- --nocapture` | PASS, 2 | `b5-recovery.log` |
| `cargo test -p opi-eval --locked --test assembled_run --test pairing_and_integrity` | PASS, 14 + 2 | `b5-suite-final.log` |
| `cargo test -p opi-eval --locked --test assembled_run replacement_ -- --nocapture` | PASS, 1 | `b6-focused.log` |
| `cargo test -p opi-eval --locked --test assembled_run` | PASS, 15 | `b6-suite.log` |
| `cargo test -p opi-eval --locked --test native_driver p19_plan_acceptance_rejects_fixture_evidence -- --exact --nocapture` | PASS, 1 | `b7-focused.log` |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p test_verify_local_acceptance.py` | PASS, 42 | `b7-python.log` |
| `cargo test -p opi-eval --locked --test native_driver p19_plan_acquisition_cancel_cleans_child -- --exact --nocapture` | PASS, 1; final owning binary also passes | `b8-focused.log`, `b9-native-driver.log` |
| `cargo test -p opi-eval --locked --lib runner::prepare` | PASS, 7 | `b8-unit-final.log` |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p test_prepare_acquisition.py` | PASS, 2, with family/product and negative subcases | `b9-python-final.log` |
| `cargo test -p opi-eval --locked --test native_driver prepare_ -- --nocapture` | PASS, 6 before the added cold-source regression | `b9-prepare-initial.log` |
| `cargo test -p opi-eval --locked --test native_driver prepare_cold_selected_products_from_explicit_committed_source -- --exact --nocapture` | PASS, 1; also covered by final owning binary | `b9-cold.log` |
| `cargo test -p opi-eval --locked --test native_driver` | PASS, 74; 9 ignored | `b9-native-driver.log` |
| `cargo clippy -p opi-eval --all-targets -- -D warnings` | PASS | `b9-clippy.log` |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p test_verify_external_lock_ci.py` | PASS, 15 | `b9-lock-ci.log` |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p test_verify_native_smoke_ci.py` | PASS, 34 | `b9-smoke-ci-final.log` |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p test_verify_native_smoke_artifact.py` | PASS, 17 | `b9-smoke-artifact.log` |
| `cargo build -p opi-eval --locked --message-format=json` | PASS; exact compiler artifact used for D3 | `b9-companion-build.log`, `b9-companion-build.jsonl` |

Earlier failed development attempts remain in the same evidence index. These
include the discriminating B2 red regression; an initial stale cached test binary
with a prior export path (recompiled before the recorded B1 green); and corrected
current-change test/compile/lint failures. They are not substituted for the final
PASS logs. No extra pre-existing production defect was repaired as an incidental.

**Evidence index**: `/tmp/opi-p19-native-apply.jr3czuc0/inputs/apply-evidence-index.json`

**Evidence index SHA-256**: `6c1887e6b8c1c4fc36f2467b8740f34c69f8bfa4fdae7d06045a81f28f8d96e7`

Final result validation (also retained in the private evidence root as
`inputs/result-validation.json`):

| Exact command | Result |
| --- | --- |
| `python .agents/skills/_shared/scripts/validate_assurance_artifact.py audit-set docs/snapshots/phase19/assurance` | PASS |
| `python .agents/skills/_shared/scripts/validate_assurance_artifact.py plan docs/snapshots/phase19/assurance/remediation.plan.md` | PASS; approved digest unchanged |
| `python .agents/skills/_shared/scripts/validate_assurance_artifact.py dispositions docs/snapshots/phase19/assurance/remediation.result.dispositions.jsonl` | PASS |
| `python .agents/skills/_shared/scripts/validate_assurance_artifact.py result docs/snapshots/phase19/assurance/remediation.result.md` | PASS |
| `python scripts/opi-doc-check.py` | PASS |
| `git diff --check` | PASS |
| `git diff --cached --check` | PASS |

The full workspace clippy/test/rustdoc/format gate union and all nine ignored
native checks were **NOT RUN** at the B9 stop boundary. Deterministic fixture
checks do not establish real-material or live-provider acceptance.

## Actual applied paths and test impact

The required `Changed paths` header is the union of all copied planned result
records, including the unstarted B10/B11 scope. It is not an assertion that every
listed path was edited. In particular,
`crates/opi-eval/tests/support/native_witness_material.rs` was not created.
Shared paths were edited only for B1-B9; no B10/B11 retention/support changes were
made. The actual applied source/documentation path inventory is:

- `CHANGELOG.md`
- `crates/opi-eval/README.md`
- `crates/opi-eval/README.zh.md`
- `crates/opi-eval/examples/local-evaluation/README.md`
- `crates/opi-eval/external-locks/static/linux-x86_64.json`
- `crates/opi-eval/scripts/build-agent-artifacts.sh`
- `crates/opi-eval/scripts/materialize-external-locks.sh`
- `crates/opi-eval/scripts/native-smoke.sh`
- `crates/opi-eval/scripts/scripted-provider.py`
- `crates/opi-eval/scripts/test_prepare_acquisition.py`
- `crates/opi-eval/scripts/test_verify_local_acceptance.py`
- `crates/opi-eval/scripts/verify-local-acceptance.py`
- `crates/opi-eval/src/cli/prepare.rs`
- `crates/opi-eval/src/experiment.rs`
- `crates/opi-eval/src/main.rs`
- `crates/opi-eval/src/report.rs`
- `crates/opi-eval/src/runner/experiment.rs`
- `crates/opi-eval/src/runner/plan.rs`
- `crates/opi-eval/src/runner/prepare.rs`
- `crates/opi-eval/tests/assembled_run.rs`
- `crates/opi-eval/tests/experiment_contract.rs`
- `crates/opi-eval/tests/fixtures/external-locks/local-cache/scripts/materialize-external-locks.sh`
- `crates/opi-eval/tests/native_driver.rs`
- `crates/opi-eval/tests/pairing_and_integrity.rs`
- `crates/opi-eval/tests/report_contract.rs`

The fixed result Markdown and sidecar are workflow outputs in addition to that
inventory. The two carried-in plan files remain separate and unchanged.

Test impact: **add** focused privacy, sampling, offline-control, recovery,
replacement, acceptance-rejection, supervised-acquisition and cold-source
regressions, plus `test_prepare_acquisition.py`; **update** affected contract,
assembly, report, pairing and acceptance-checker tests; **retain** the conformance
coverage and existing nine ignored native tests. No intentional product behavior
outside the approved repairs was removed. The two script mode changes are staged;
other task edits and the new test remain unstaged/untracked. No commit was made.
The English/Chinese crate README contract was synchronized; the local walkthrough
has no Chinese counterpart. User-visible changes are under Unreleased.

## Materialization handoff

B1-B8 have the recorded closure evidence; B9's deterministic implementation is
reviewable but its required real replay remains failed. A new explicitly invoked
plan must resolve the exact Opi source/workspace-input pair before B9/D3 can pass
and B10/B11 can proceed. This apply does not invoke planning, evaluation, audit,
release or any other explicit-only workflow automatically.

No Phase PASS or real-acceptance claim is granted. A fresh audit/reviewer rerun is
not admitted yet: the input conflict and owning-workflow return must be resolved,
then the fixes plus current live audit set must be committed by explicit user
instruction and the assurance directory must be clean. Retained private evidence
has not been copied into source bundles or used as an implementation ledger.
