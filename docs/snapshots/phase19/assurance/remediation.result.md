# Phase 19 remediation apply result

**Status**: COMPLETE
**Audit index SHA-256**: `702853fe37c1779c38612f7979a7e995395d1890954c4d3d8d6205592625e6d3`
**Plan SHA-256**: `d7e2b928415d8653455d03ab1f1b24a931af75bc903ce4687b3bb08409e3d50c`
**Remediation head**: `e834271d7ea6d114d67d9fc7e5298919fc7cb11e`
**Changed paths**: ["CHANGELOG.md", "crates/opi-eval/README.md", "crates/opi-eval/README.zh.md", "crates/opi-eval/examples/local-evaluation/README.md", "crates/opi-eval/external-locks/static/linux-x86_64.json", "crates/opi-eval/scripts/build-agent-artifacts.sh", "crates/opi-eval/scripts/materialize-external-locks.sh", "crates/opi-eval/scripts/native-smoke.sh", "crates/opi-eval/scripts/test_prepare_acquisition.py", "crates/opi-eval/scripts/test_verify_native_smoke_ci.py", "crates/opi-eval/src/benchmark/process.rs", "crates/opi-eval/src/runner/material.rs", "crates/opi-eval/src/runner/prepare.rs", "crates/opi-eval/tests/native_driver.rs", "crates/opi-eval/tests/support/native_witness_material.rs"]
**Dispositions**: `remediation.result.dispositions.jsonl`
**Execution date**: 2026-09-25–2026-09-26 (Asia/Shanghai)

## Outcome and stop boundary

Closure is incomplete: B1 and B2 met their approved predicates; B3 is **Not closed**.
Eight independently refuted source findings remain Refuted. The COMPLETE header
means that this fixed result covers all 11 approved source keys; it does not mean
that B3, the Final Verification union, or Phase 19 passed. The live aggregate
verdict remains FAIL. No commit, branch, push, new audit, provider run, or other
explicit-only workflow was performed.

B3 exposed a new real-client packaging defect. The exact pi 0.85.1 runtime emitted
by each B1 cold preparation contains `dist/`, documentation and package metadata,
but no external dependency closure. The bundle imports `@earendil-works/chord`
(declared in its package.json as `^0.85.1`). Both the direct pinned Node/bundle test
and each package's own wrapper fail with `ERR_MODULE_NOT_FOUND` before pi sends a
request. No package bytes were patched to conceal that failure.

The causal packaging path is `crates/opi-eval/scripts/materialize-external-locks.sh`.
Repairing it also requires updating its producer SHA-256 in
`crates/opi-eval/external-locks/static/linux-x86_64.json`; preparation rejects producer
hash drift. The execution protocol's bounded incidental rule forbids a
`manifest/lockfile` change. Its instruction, “If any condition fails, stop and
create a new plan,” applies here. B1.1 also limited the approved producer change
to propagating its admitted model-data archive pin. This additional runtime
closure repair therefore needs a new explicit plan; the approved plan is unchanged.
No incidental repair was made or claimed.

## Active source coverage

- C: `phase19-codex-gpt6-d915e4a-20260918t084558z`, findings SHA-256
  `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac`.
- P: `phase19-pi-glm53-ad67cb8-20260918t101920z`, findings SHA-256
  `fa4d61c10efa6cb9d68146fab9f5f28eaf970393f941ba97119a1c68e5fd8301`.

| Source / finding | Verification | Batch | Remediation |
|---|---|---|---|
| C / P19-AUD-001 | Partially confirmed | B1 | Closed |
| C / P19-AUD-002 | Refuted | Exclusion | Refuted |
| C / P19-AUD-003 | Refuted | Exclusion | Refuted |
| C / P19-AUD-004 | Refuted | Exclusion | Refuted |
| C / P19-AUD-005 | Refuted | Exclusion | Refuted |
| C / P19-AUD-006 | Refuted | Exclusion | Refuted |
| C / P19-AUD-007 | Refuted | Exclusion | Refuted |
| C / P19-AUD-008 | Refuted | Exclusion | Refuted |
| C / P19-AUD-009 | Partially confirmed | B3 | Not closed |
| P / P19-AUD-001 | Refuted | Exclusion | Refuted |
| P / P19-AUD-002 | Partially confirmed | B2 | Closed |

The result sidecar copies each plan disposition verbatim except for
`green_after.observed` and `remediation_status`. There are no extra finding keys.

## Evidence locations and execution identity

External durable evidence root (Windows):
`D:/Luiz/Odradek/opi-remediation-evidence/phase19-apply-20260925-d7e2b928`.

The same root in WSL is `/mnt/d/Luiz/Odradek/opi-remediation-evidence/phase19-apply-20260925-d7e2b928`. It contains raw stdout/stderr, exact argv/cwd and
closed environment projections, exit codes, timings and SHA-256 records in
`logs/`; admission bytes in `baseline/` and `baseline.json`; B1/B2 candidate
snapshots; `b2-official-verified.json`; and `b3-pi-package-diagnosis.json`.

The faithful executable-bit-preserving candidate export is `/home/luiz/.local/state/opi-remediation-evidence/phase19-apply-20260925-d7e2b928/source`. It started
from a fresh Git archive of the remediation head and overlaid exactly the 17
admitted unstaged paths. New task edits were synchronized explicitly. No Git
worktree or per-session Cargo target was created. The retained incremental Cargo
cache is `/mnt/e/opi/cargo-targets/opi-8abcf38f9871653e`.

Real material/native roots are beneath `/home/luiz/.local/state/opi-remediation-evidence/phase19-apply-20260925-d7e2b928`:
`cold-v2/{terminal-bench-2.1,terminal-bench-3.0,deepswe-v1.1}`,
`official-witnesses/`, and `b3/`. The selected Docker Desktop engine was
`f7640520-9a9f-4acb-bd58-62c5b388982a`. No paid models or live provider credentials
were used. Official conformance diagnostics ran the fixed upstream graders;
client protocol checks used isolated loopback scripts and synthetic credentials.

## B1: selected pinned preparation — Closed

Retained all approved carried-in B1 repairs. The task additionally isolated the
resource-admission test from the ambient Docker endpoint with a recording stub
covering unknown, sufficient and insufficient resources. No required task was
silently removed.

The six discriminating probes passed: exact pi archive pin, selected-engine
storage identity, selected-image bindings, image acquisition, corpus/relative
preparation and closed-endpoint resource admission. Available/closed ambient
resource tests both passed. Source pins, target bytes/modes, unsupported images
and resource failures retain their rejection checks.

All three official selected families completed independent-directory
`doctor → prepare → validate → plan`, then warm `prepare → validate → plan`:
21 commands, all exit 0. Warm preparation acquired nothing and preserved the
selected task, complete corpus identity, requirements, clients and material
identities; only observation/output locations vary. Per-family
`warm-identity-check.json` records the comparisons. TB2.1 selected
`openssl-selfsigned-cert`, TB3 selected `batched-eval-parity`, and DeepSWE selected
`abs-module-cache-flags`.

This closes the approved preparation predicates. The later B3 failure expressly
limits the meaning of this success: successful preparation and byte binding did
not prove that the emitted pi runtime had all of its executable dependencies.

## B2: explicit native diagnostics — Closed

`native_job_config` now chooses registered `oracle`/`nop` names without a custom
import path. Its reader requires the exact selected diagnostic name and an
absent/null import path. Missing, wrong, malformed and dual selectors reject;
unsupported internal selectors fail before spawn. Integration import selection,
model/trial/cap/retry/environment behavior and the absence of diagnostic Agent
scores/terminal observations are preserved. Updated Unreleased and both README
languages together; this snapshot has no bilingual counterpart.

The two approved Rust probes passed with newly generated configs. The exact
Harbor and Pier constructors both instantiated the production configs and their
registered-name controls. Four process-module tests, two new focused selection
regressions and the existing substitution matrix passed. The default native
binary passed 78 tests, with nine opt-in tests ignored.

| Official task family | Exact ignored test | Raw oracle reward | Raw nop reward | Exit |
|---|---|---|---|---|
| TB2.1 | `native_tb21_artifact_witness` | 1 | 0 | 0 |
| TB3.0 | `native_tb3_artifact_witness` | 1 | 0 | 0 |
| DeepSWE v1.1 | `native_deepswe_patch_witness` | 1 | 0 | 0 |

Each command was `OPI_EVAL_NATIVE_WITNESS_MATERIAL=<its B1 cold material>
cargo test -p opi-eval --test native_driver <test> -- --ignored --exact --nocapture
--test-threads=1`. Raw logs are `b2-official-<family>`. Six retained
`witness-evidence.json` records bind CLI exit, executable/material hashes,
job/trial output and every retained file digest. All those hashes were rechecked.
TB3's declared `/app/evalbench/` and DeepSWE's `/logs/artifacts/model.patch`
changed digest between oracle/nop; no undeclared canary entered the bound
collection. Official tasks and upstream runners were not modified.

## B3: current native diagnostic union — Not closed

Implemented only the retention work reached before the blocker:

- Protocol/turn-budget roots now survive the test. Client configs, executable
  and package identities, stdout/stderr, argv, isolated environment, exits and
  tool-effect hashes are retained by the private test helper.
- The owned-resource test selects the admitted immutable Python image with
  `--pull never`, retains its root/output and checks neighboring-container and
  shared-image survival. This updated ignored test has not run yet.

The first command, `OPI_EVAL_NATIVE_CLIENT_MATERIAL=<b3/clients.json>
cargo test -p opi-eval --test native_driver native_protocol_client_matrix --
--ignored --exact --nocapture --test-threads=1`, exited **101**. Evidence is in
`logs/b3-native_protocol_client_matrix.*` and `/home/luiz/.local/state/opi-remediation-evidence/phase19-apply-20260925-d7e2b928/b3/.tmpiobuIY`.
The pi Anthropic case exited 1 because `@earendil-works/chord` was absent. Earlier
Opi loopback cases do not establish a complete protocol matrix.

To distinguish packaging failure from client-test configuration, each B1 pi
wrapper was run as `<exact package>/pi --version` in its own fresh directory,
with isolated HOME/config and no credentials. All three exited **1** with the
same missing-package error. Logs are `b3-pi-launch-<family>.*`; byte identities
and the missing dependency directory are recorded in
`b3-pi-package-diagnosis.json`.

Stopped here. The five remaining B3 ignored filters, the controlled task/material
builder, real grader privacy inspections, deadline/regrade matrices and the
three-family independent assembled CLI rows were not executed/completed. No
bound was increased and no synthetic substitute was counted as native success.
The workspace-wide Clippy/test/doctest/rustdoc gates from Final Verification
were not run during this apply; there is no claim that its complete union passed.

## Current-source handoff checks

The following checks verify the retained changes and refutation counterevidence;
they do not close the failed native matrix. The default Rust union yielded
153 passed/0 ignored, 2 passed/0 ignored, 15 passed/0 ignored, 78 passed/9 ignored, in lib, agent-integration, assembled-run and native-driver order.
The three Python suites passed 7, 35 and 42 tests respectively.

| Raw log | Exact command | Exit |
|---|---|---|
| `handoff-current-eval-union` | `cargo test -p opi-eval --lib --test native_driver --test assembled_run --test agent_integration_conformance -- --test-threads=4` | 0 |
| `handoff-prepare-tests` | `python3 crates/opi-eval/scripts/test_prepare_acquisition.py` | 0 |
| `handoff-smoke-ci-tests` | `python3 crates/opi-eval/scripts/test_verify_native_smoke_ci.py` | 0 |
| `handoff-acceptance-tests` | `python3 crates/opi-eval/scripts/test_verify_local_acceptance.py` | 0 |
| `handoff-fmt` | `cargo fmt --check --all` | 0 |
| `handoff-eval-clippy` | `cargo clippy -p opi-eval --all-targets -- -D warnings` | 0 |
| `handoff-script-modes` | `git ls-tree HEAD crates/opi-eval/scripts/scripted-provider.py crates/opi-eval/tests/fixtures/external-locks/local-cache/scripts/materialize-external-locks.sh` | 0 |

`python scripts/opi-doc-check.py`, `git diff --check`, the active audit-set/plan
validators and the fixed result validator are run after this report is installed;
their final command records are retained under `logs/handoff-*`.

Test impact: **add** two diagnostic-selection regressions and the shared private
evidence helper; **update** the isolated resource test, affected diagnostic
fixtures and opt-in retention/owned-image checks; **retain** official reward
contrasts and all refutation regressions; **delete** none.

## Preservation and materialization boundary

Worktree-only delivery: committed=none-for-this-task; staged=none. HEAD remains
`e834271d7ea6d114d67d9fc7e5298919fc7cb11e`. The 17-path admission inventory is preserved in `baseline.json`.
Carried-in paths touched during this apply are the two READMEs, CHANGELOG,
`native_driver.rs` and the authorized fixed result pair. The newly modified
tracked source is `benchmark/process.rs`; the only new source file is
`tests/support/native_witness_material.rs`. All other admission bytes, including
the fixed plan pair, producer scripts and static lock, remain unchanged.
Final inventory/hashes are in `preservation-final.json`. The original fixed
result bytes were backed up opaquely, never consumed as evidence.

The Changed paths header is the exact union attributed by approved dispositions;
it includes retained B1 work and the partially implemented B3 helper, not a claim
that every listed path was first edited in this apply. The fixed result pair is
an authorized assurance output, separate from source-path attribution.

A new explicit plan is required for the pi runtime dependency closure and its
static producer pin before B3 can resume. A fresh audit/reviewer rerun additionally
requires fixes and the live assurance set committed, assurance clean, external
evidence resolved and any owning-workflow return complete. Nothing was committed
automatically and no Phase PASS is granted.
