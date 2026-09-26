# Phase 19 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `702853fe37c1779c38612f7979a7e995395d1890954c4d3d8d6205592625e6d3`
**Remediation head**: `62758ce04ec19e7c1bfeb3ea776c7eed6d7fd0bd`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged `[]`; unstaged `[]`; untracked `[]` after the user-authorized commit.
**Unresolved decisions**: none

## Admission and current evidence

Audit-set validation passed before semantic inputs. Revision 4 is FAIL. All 85 indexed requirement records and the strict union of 11 findings were consumed. Source records and severities are unchanged. Current committed source controls verification; no history, prior remediation conclusion, alternate plan or eval artifact is an input.

Checks and inspection use a unique WSL ext4 `git archive` export: `/home/luiz/.local/state/opi-remediation-evidence/phase19-plan-20260926-1d1f6989/source`. Archive SHA-256: `5a0922e9adaac10d4f8fef21213e69e9db5dd387d517641220069ca6295b1044`. The ledger registers the normative spec and both Phase 19 language sources; current criterion digests match the indexed values. Ledger progress fields do not grant remediation authority.

Evidence root `E`: `D:/Luiz/Odradek/opi-remediation-evidence/phase19-plan-20260926-1d1f6989`, or WSL `/mnt/d/Luiz/Odradek/opi-remediation-evidence/phase19-plan-20260926-1d1f6989`. `baseline.json`, `current-source-bindings.json`, `upstream-source-bindings.json` and `logs/<label>.json/.stdout/.stderr` retain source/argv/environment/exit/output hashes. Existing external Cargo target: `/mnt/e/opi/cargo-targets/opi-8abcf38f9871653e`; incremental compilation enabled. No worktree or new target cache.

| Alias | Exact current run | Owning findings SHA-256 |
|---|---|---|
| C | `phase19-codex-gpt6-d915e4a-20260918t084558z` | `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac` |
| P | `phase19-pi-glm53-ad67cb8-20260918t101920z` | `fa4d61c10efa6cb9d68146fab9f5f28eaf970393f941ba97119a1c68e5fd8301` |

Aliases abbreviate full identities above; duplicate textual IDs remain separate. Current run/digest identity controls admission, title similarity cannot substitute for it, and no older source is consulted.

## Current Finding Verification

| Source / Finding ID | Verification | Source → final severity and rationale | Closure key / family | Batch | Decision |
|---|---|---|---|---|---|
| C / P19-AUD-001 | Partially confirmed | Major → Major; Major retained: selected pi acquisition publishes an unlaunchable package; the original not-wired claim is refuted by current source and acquisition tests. | `prepare.standalone-locked-pi-runtime` / `prepare.acquisition` | B1 | `fix:package-locked-pi-runtime-closure` |
| C / P19-AUD-002 | Refuted | Major → Major; Original Major severity is retained without rewriting the source; current independent evidence refutes the claimed defect. | `prepare.supervised-acquisition` / `prepare.acquisition` | — | `no-action:refuted` |
| C / P19-AUD-003 | Refuted | Major → Major; Original Major severity is retained without rewriting the source; current independent evidence refutes the claimed defect. | `controls.required-sampling-admission` / `controls.comparability` | — | `no-action:refuted` |
| C / P19-AUD-004 | Refuted | Major → Major; Original Major severity is retained without rewriting the source; current independent evidence refutes the claimed defect. | `report.preserve-output-support` / `controls.comparability` | — | `no-action:refuted` |
| C / P19-AUD-005 | Refuted | Major → Major; Original Major severity is retained without rewriting the source; current independent evidence refutes the claimed defect. | `recovery.read-only-planned-inventory` / `run.recovery` | — | `no-action:refuted` |
| C / P19-AUD-006 | Refuted | Major → Major; Original Major severity is retained without rewriting the source; current independent evidence refutes the claimed defect. | `replacement.fresh-resolved-paired-identities` / `run.recovery` | — | `no-action:refuted` |
| C / P19-AUD-007 | Refuted | Major → Major; Original Major severity is retained without rewriting the source; current independent evidence refutes the claimed defect. | `privacy.stop-affected-dispatch` / `run.privacy` | — | `no-action:refuted` |
| C / P19-AUD-008 | Refuted | Major → Major; Original Major severity is retained without rewriting the source; current independent evidence refutes the claimed defect. | `acceptance.reject-synthetic-evidence` / `evidence.acceptance` | — | `no-action:refuted` |
| C / P19-AUD-009 | Partially confirmed | Major → Major; Major retained: nine native filters remain ignored; exact pi startup blocks client proof and the full current dynamic/workspace union is not established. The indexed source already distinguishes supplied six live trials from these remaining obligations. | `native.current-client-lifecycle-and-gate-evidence` / `native.dynamic-evidence` | B3 | `fix:complete-current-no-model-native-evidence` |
| P / P19-AUD-001 | Refuted | Major → Major; Original Major severity is retained without rewriting the source; current independent evidence refutes the claimed defect. | `checkout.executable-script-modes` / `verification.faithful-export` | — | `no-action:refuted` |
| P / P19-AUD-002 | Partially confirmed | Major → Major; Major retained for mandatory dynamic proof. Current retained-witness/diagnostic code exists, but this plan-stage set lacks newly executed three-family native contrasts. This does not claim earlier witnesses never ran. | `native.three-family-artifact-sensitive-grading` / `native.dynamic-evidence` | B2 | `fix:retain-current-three-family-grader-witnesses` |

All sources have Major priority. B1 package usability, B2 native artifact sensitivity and B3 remaining client/lifecycle/current-gate evidence do not share one root change and closure predicate, so they remain separate batches. Eight claims are refuted by fresh counterexamples.

## Unresolved Decisions

none

## Closure Batches

### Batch B1: Standalone locked pi runtime package

**Closure predicate**: A fresh selected pi package starts outside its source tree with pinned Node, resolves required runtime dependencies, and is not published if its closure is missing, escaping or inconsistent. Public cold/warm preparation preserves selected identities.
**Dependencies**: none.
**Verification union**: cold package/startup/import smoke; acquisition tests; three-family public cold/warm CLI matrix; producer-pin and documentation checks.

#### Fix B1.1: Include and validate the existing locked runtime closure

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z / 01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac / P19-AUD-001`.
- **Decision**: `fix:package-locked-pi-runtime-closure`.
- **Verification status**: Partially confirmed.
- **File(s)**: `crates/opi-eval/scripts/materialize-external-locks.sh`, `crates/opi-eval/scripts/test_prepare_acquisition.py`, `crates/opi-eval/external-locks/static/linux-x86_64.json`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Complete the existing private pi packaging branch from already locked/built inputs; validate startup before cache installation. Update the materializer producer digest, owning script tests, bilingual preparation guidance and Unreleased history.
- **Closure predicate**: Standalone `pi --version` and required external-import/native-binary smoke succeed without source/global module lookup; malformed closure rejects before installation.
- **Red-before**: `bash /home/luiz/.local/state/opi-remediation-evidence/phase19-plan-20260926-1d1f6989/pi-cold/cache/pins/scripts/materialize-external-locks.sh prepare --cache /home/luiz/.local/state/opi-remediation-evidence/phase19-plan-20260926-1d1f6989/pi-cold/cache --plan /home/luiz/.local/state/opi-remediation-evidence/phase19-plan-20260926-1d1f6989/pi-cold/missing-plan.json; /home/luiz/.local/state/opi-remediation-evidence/phase19-plan-20260926-1d1f6989/pi-cold/cache/agents/pi/pi --version` — cold acquisition exit 0, then startup exit 1 with `ERR_MODULE_NOT_FOUND` for `@earendil-works/chord`, Node `v22.19.0`, 0.13 s. Raw logs and the 1,064-file package inventory are retained under E; production source was unchanged.
- **Green-after**: Replay the same acquisition/startup recipe with candidate producer pins and a new empty cache; run the package/import smoke, updated acquisition tests and three-family public cold/warm checks below; all must pass.

The exact upstream source is pi `d981de1229ef899957bbe968bc8dcda02a21f477` (0.85.1). Its committed `scripts/build-coding-agent-bundle.mjs` explicitly leaves chord, photon-node and lazy jiti imports external. The lock resolves `@earendil-works/chord` to the built workspace at 0.85.1, `@silvia-odwyer/photon-node` to 0.3.4, `jiti` to 2.7.0, and chord's `esbuild` dependency to 0.28.1 with `@esbuild/linux-x64` at 0.28.1. Current packaging copies Node, coding-agent dist/docs/examples and metadata, omitting this closure.

Use the existing locked `npm ci --ignore-scripts` and `build:offline` output. Copy this concrete five-package runtime closure: the built chord distribution from verified workspace source and the four locked installed packages. Preserve required package metadata, distribution/data files, license notices and executable modes. Resolve the known chord workspace link only inside the verified source root; reject missing/unexpected targets, escaping links, special files and inconsistent identities. Preserve the final link-free runtime package contract and hash every copied byte in its existing manifest. Do not copy `.bin`, the whole development tree or unrelated platform packages. Optional `bufferutil`, `utf-8-validate` and `supports-color` are absent in the locked root and upstream declares fallbacks; do not add them.

Before cache installation, run a bounded credential-free startup/version check with the copied Node/bundle, fresh HOME/config, no NODE_PATH/NODE_OPTIONS and working directory outside the source checkout. Validation must resolve chord's exported runtime entries, import photon-node and jiti, and execute a trivial esbuild transformation with the copied Linux x64 binary; this exercises lazy dependencies without a model. Do not run installation scripts or fetch new dependencies. Negative script cases cover missing root/transitive/platform packages, wrong identity, workspace-link escape and startup failure; none may leave an installed `agents/pi` entry. Retain selected-products and warm-cache assertions.

The static external-lock edit is explicitly admitted: recompute the LF-normalized materializer producer SHA-256. This is not an incidental lock repair. Keep upstream source/blob versions, Node archive/version, model-data archive, runner/corpus/image pins, workspace Cargo input and npm dependency graph unchanged. No Cargo manifest/lock, public schema or durable format change is admitted. The package grows by already declared JS/WASM/esbuild runtime bytes; Linux x64 scope stays unchanged, install hooks remain disabled, and no new native build is introduced. A required dependency outside this verified locked closure requires replanning.

Public cold/warm check: use `terminal-bench-2.1/openssl-selfsigned-cert`, `terminal-bench-3.0/batched-eval-parity`, and `deepswe-v1.1/abs-module-cache-flags`. Seed fresh selected-material caches only with candidate pins/scripts/profiles/integrations. Keep selected Opi/pi identities explicit; build Opi from the exact local committed source rather than a PATH executable. From an independent directory per family, run:

```sh
opi-eval doctor --config experiment.toml
opi-eval prepare --config experiment.toml --out cold --opi-source <local-repository> --opi-commit 62758ce04ec19e7c1bfeb3ea776c7eed6d7fd0bd
opi-eval validate --config experiment.toml --native-material cold/material.json
opi-eval plan --config experiment.toml --native-material cold/material.json --out cold-plan.json
opi-eval prepare --config experiment.toml --out warm
opi-eval validate --config experiment.toml --native-material warm/material.json
opi-eval plan --config experiment.toml --native-material warm/material.json --out warm-plan.json
```

Bind candidate Companion/producer bytes separately from the selected committed Opi build; do not identify an uncommitted Companion as that Opi executable. Assert empty warm acquisition, unchanged selected task/Agent/runner/corpus identities, standalone pi startup and no unselected Agent build. Dynamic host observations/output paths need not be byte-identical. Preserve unknown/insufficient-resource rejection; do not fabricate Docker storage or loosen task limits.

### Batch B2: Current native artifact contrasts for every family

**Closure predicate**: All three official pinned graders respond to correct/incorrect artifacts, with six retained raw trials and exact material/current producer identities.
**Dependencies**: B1 supplies validated materials.
**Verification union**: three existing ignored artifact-witness filters plus raw-result/hash inspection.

#### Fix B2.1: Execute and retain the current official grader witnesses

- **Finding source(s)**: `phase19-pi-glm53-ad67cb8-20260918t101920z / fa4d61c10efa6cb9d68146fab9f5f28eaf970393f941ba97119a1c68e5fd8301 / P19-AUD-002`.
- **Decision**: `fix:retain-current-three-family-grader-witnesses`.
- **Verification status**: Partially confirmed.
- **File(s)**: no production/test edit required; existing `crates/opi-eval/tests/native_driver.rs` filters, external raw outputs and fixed result artifacts.
- **Change kind**: test-only (execution/evidence).
- **Change**: Run existing retained witnesses against exact B1 family material and independently inspect all native results.
- **Closure predicate**: Every family retains diagnostic oracle reward 1 and nop reward 0, correct task/runner identity and artifact-sensitive grading.
- **Red-before**: N/A for execution-only work. Fresh default tests pass diagnostic generator/reader checks but list the real-native filters as ignored; synthetic PASS cannot prove native grading.
- **Green-after**: Each exact command below passes; inspect six raw results, material hashes, argv, exits and inventories.

```sh
OPI_EVAL_NATIVE_WITNESS_MATERIAL=<tb21-material> cargo test -p opi-eval --test native_driver native_tb21_artifact_witness -- --ignored --exact --nocapture --test-threads=1
OPI_EVAL_NATIVE_WITNESS_MATERIAL=<tb3-material> cargo test -p opi-eval --test native_driver native_tb3_artifact_witness -- --ignored --exact --nocapture --test-threads=1
OPI_EVAL_NATIVE_WITNESS_MATERIAL=<deepswe-material> cargo test -p opi-eval --test native_driver native_deepswe_patch_witness -- --ignored --exact --nocapture --test-threads=1
```

Retain TB2.1's shared-trial lifecycle and explicit diagnostic selections. TB3 transfers only `/app/evalbench/`; DeepSWE transfers only `/logs/artifacts/model.patch` against the declared base into the pristine verifier. Paired declared artifact digests differ while reward changes 1 to 0. No diagnostic fills an Agent score or real-acceptance slot. Current registered-name dispatch is independently established; no dispatch redesign is planned.

### Batch B3: Remaining current client, lifecycle and gate evidence

**Closure predicate**: Exact clients/native paths satisfy the no-model dynamic checks, the independent-directory public workflow is reproducible, and every final workspace gate has current byte-bound raw evidence.
**Dependencies**: B1 then B2. Reference B2 artifacts without merging its distinct source finding/predicate.
**Verification union**: six remaining ignored filters; independent-directory CLI matrix; script/workspace checks.

#### Fix B3.1: Supply bounded diagnostic material and retain executions

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z / 01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac / P19-AUD-009`.
- **Decision**: `fix:complete-current-no-model-native-evidence`.
- **Verification status**: Partially confirmed.
- **File(s)**: `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/support/native_witness_material.rs`.
- **Change kind**: test-only.
- **Change**: Extend the existing private helper as needed to construct reproducible controlled Git/Python material for existing stage/regrade filters, preserve client/material/runner/container identities and execute the no-model matrix.
- **Closure predicate**: Raw observations prove actual-client protocol/control behavior, all-family narrowing deadlines and artifact-sensitive regrades, owned-only cancellation/credential isolation, immutable originals and current gates.
- **Red-before**: N/A for private diagnostic material/evidence. Fresh default output skips nine native filters and B1 independently reproduces exact-client startup failure. New production defects need their own admissible red/green repair or replanning.
- **Green-after**: All exact filters and CLI/workspace checks below pass with retained evidence. An ignored/not-run check, stand-in grader or source assertion is not a native PASS.

Use unchanged pinned Harbor/Pier revisions and immutable image `python@sha256:272efc3f94bec901e2bc7a5c9984b3d113c317839edca2714c6a388b9476f8fc`. Private diagnostic task/corpus Git commits and material/profile hashes must be real and recomputed; do not substitute them into production static pins. Reuse current fixture binding helpers and production admission. Diagnostic material remains distinct from official corpus scores and real-provider acceptance. No new public schema/framework/crate/feature is needed.

Client material records exact B1 `opi`, `opi_package`, `pi`, `pi_package`, `node`, `pi_bundle` and experiment paths plus recomputed executable/package hashes. Exercise all three protocols through production projection, including authentication, tool effects, retry/error settlement and output/turn controls. Use isolated native config and loopback dummy credentials, without real auth files or ambient provider activation. Preserve requests, stdout/stderr/exits and failed attempts. The output-control experiment must have its explicit numeric output cap and admitted controls, with a fresh absolute output path.

```sh
OPI_EVAL_NATIVE_CLIENT_MATERIAL=<clients.json> cargo test -p opi-eval --test native_driver native_protocol_client_matrix -- --ignored --exact --nocapture --test-threads=1
OPI_EVAL_NATIVE_CLIENT_MATERIAL=<clients.json> cargo test -p opi-eval --test native_driver native_client_turn_budgets -- --ignored --exact --nocapture --test-threads=1
OPI_EVAL_NATIVE_CLIENT_MATERIAL=<clients.json> OPI_EVAL_OUTPUT_CONTROL_OUT=<fresh-absolute-path> cargo test -p opi-eval --test native_driver native_output_control_client_matrix -- --ignored --exact --nocapture --test-threads=1
cargo test -p opi-eval --test native_driver native_owned_resource_cancellation -- --ignored --exact --nocapture --test-threads=1
OPI_EVAL_NATIVE_WITNESS_MATERIAL=<delay-material> cargo test -p opi-eval --test native_driver remediation_native_stage_deadlines -- --ignored --exact --nocapture --test-threads=1
OPI_EVAL_NATIVE_WITNESS_MATERIAL=<regrade-material> cargo test -p opi-eval --test native_driver native_verifier_only_regrade_witness -- --ignored --exact --nocapture --test-threads=1
```

Retain positive, time-cap, agent-cap, verifier-cap, both-agent, both-verifier, minimum-cap, strict-task and unbounded-task deadline cases for all families. Keep admitted bounds, including Pier's 30-second verifier cap and current outer bound; run positive controls before negative cases. Observe native exception/config/settlement and owned-container cleanup. Do not increase a cap, delete a case, replace the runner or alter official tasks to hide failure.

Regrade includes positive/negative artifacts for every family, TB2.1 timeout, missing/tampered artifacts/bundles, task/base/network drift, missing cache, existing output and uncertain cleanup. Check fresh verifier identities, no Agent/provider activity during regrade, unchanged source/material bytes, absent canaries and independent engine cleanup. Cancellation retains raw evidence and proves an independent neighbor/shared image survive, then cleans only the test-owned neighbor. Missing observations remain Not closed.

From an independent directory per family, use the current CLI and frozen controlled material to exercise `doctor`, `prepare`, `validate`, `plan`, `run --plan`, `verify`, `report` and `regrade`. Where official tasks need a model, use separately labelled diagnostic material; it cannot close the six-live-trial criterion. No `--fixtures` or GitHub metadata supplies this production native workflow. Retain exact config/plan/material hashes, argv, exits, native records and original/derived reports. Offline operations cannot increase provider requests or launch an Agent.

## Final Verification

Run the union once on final relevant candidate bytes; reuse unchanged byte-bound evidence. Every modified test runs in its exact binary/filter. Final workspace gates are required by the indexed current-evidence obligation and are not claimed passed during planning.

```sh
python3 crates/opi-eval/scripts/test_prepare_acquisition.py
python3 -m unittest discover -s crates/opi-eval/scripts -p "test_*.py"
cargo test -p opi-eval --lib --test native_driver --test assembled_run --test agent_integration_conformance -- --test-threads=4
python scripts/opi-doc-check.py
git diff --check
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

Package smoke and all B2/B3 ignored commands are additional requirements. Use WSL/Linux and the existing incremental external cache. Retain the exact final source/test/producer/material digest set and raw logs, reporting failures and unrun checks. No `cargo clean` or new target cache.

Test impact planned: **add** missing/transitive/platform/escape/startup-failure packaging cases and private diagnostic material construction; **update** owning acquisition/native test helpers; **retain** current regressions and bounds; **delete** none. No production Rust behavior change is admitted. READMEs update English/Chinese together; fixed assurance artifacts have no Chinese counterpart.

## Scope and authority boundary

Do not revise normative/registered specifications, edit implementation ledgers, publish/release/push or authorize model spend. Indexed C009 already recognizes supplied six live trials and asks for remaining current native/gate proof. This plan does not re-ingest eval artifacts, rerun paid trials or upgrade saved live evidence to new source identities. An independent audit still owns Phase conformance. New product meaning, public contracts, dependency graphs, broader authority or weakened requirements return to shaping/planning.

## Plan-stage checks

| Retained label / command | Observed |
|---|---|
| Current audit-set validator | PASS before semantic inputs and before plan installation |
| `current-eval-union` | PASS: 153 library + 2 Agent conformance + 15 assembled + 78 native-driver = 248; 9 native filters ignored |
| `prepare-unit` | PASS: 7 tests including selected product/family combinations |
| `smoke-ci-unit` | PASS: 35 tests |
| `acceptance-unit` | PASS: 42 tests |
| `script-modes` | Both faithful exported scripts are 0755 |
| `pi-upstream-fetch`, `pi-upstream-checkout` | Exact pinned commit and locked blob hashes verified |
| `pi-cold-acquisition` | Harness configuration error: npm global/user config both named `/dev/null`; excluded from product defect attribution |
| `pi-cold-acquisition-retry` | PASS with unchanged production script and separate empty npm config files; selected pi package produced |
| `pi-cold-startup` | FAIL: missing chord, exit 1; B1 red-before |

No B2/B3 native execution or final workspace gate is claimed complete here. Source/package/startup evidence is fresh for the committed remediation head. The preliminary npm configuration error remains preserved separately.


## Exclusions

| Finding ID | Disposition | Current evidence / authority |
|---|---|---|
| C / P19-AUD-002 | Refuted | Current ProcessSupervisor deadline/output/failure and public cancellation checks pass; the owned child is reaped. Source: crates/opi-eval/src/runner/prepare.rs:365; crates/opi-eval/tests/native_driver.rs:5036; logs/current-eval-union.* |
| C / P19-AUD-003 | Refuted | Required unsupported sampling rejects before dispatch; optional sampling remains explicitly unsupported. Both public regressions pass. Source: crates/opi-eval/tests/native_driver.rs:1233,1333; logs/current-eval-union.* |
| C / P19-AUD-004 | Refuted | The same sealed-bundle offline report regression preserves unsupported output-control status and passes. Source: crates/opi-eval/tests/native_driver.rs:2498; logs/current-eval-union.* |
| C / P19-AUD-005 | Refuted | Recovery checks preserve every original byte and planned trial and reject frozen inventory identity drift; both pass. Source: crates/opi-eval/tests/assembled_run.rs:682,759; logs/current-eval-union.* |
| C / P19-AUD-006 | Refuted | Resolved selection and independent replacement identities retain frozen paired scope; the current public test passes. Source: crates/opi-eval/tests/assembled_run.rs:44; logs/current-eval-union.* |
| C / P19-AUD-007 | Refuted | The public serial privacy-failure regression passes and asserts later native dispatch does not start. Source: crates/opi-eval/tests/native_driver.rs:1361; logs/current-eval-union.* |
| C / P19-AUD-008 | Refuted | The native-fixture rejection regression and all 42 acceptance-checker tests pass; synthetic/scripted evidence cannot satisfy real acceptance. Source: crates/opi-eval/tests/native_driver.rs:8052; crates/opi-eval/scripts/test_verify_local_acceptance.py; logs/current-eval-union.*; logs/acceptance-unit.* |
| P / P19-AUD-001 | Refuted | Fresh ext4 git archive preserves 0755 for both scripts; agent_conformance_matrix_settles_every_pinned_case and supervised_acquisition_runs_the_pinned_materializer pass. logs/script-modes.* and logs/current-eval-union.* |

No exclusion relies on reviewer voting. C’s split requirement states and P’s broader rows remain separate; dynamic claims cannot be refuted by a peer’s met state.

## Indexed criterion matrix

All 85 rows preserve the exact criterion identity and owning state. `S` means `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`, SHA-256 `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`; `N` means `docs/opi-spec.md`, SHA-256 `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db`. C’s split citations and P’s table-row citations are not exact `(path, sha256, citation)` matches; they remain distinct. Coverage is descriptive, not a revised verdict.

| Source requirement | Criterion identity (path/digest alias + exact citation) | Original state | Reciprocal finding(s) |
|---|---|---|---|
| C / P19-AC-001-01 | S: P19-AC-001; sections 3–10 | met | — |
| C / P19-AC-001-02 | S: P19-AC-001; sections 3–10 | met | — |
| C / P19-AC-001-03 | S: P19-AC-001; sections 3–10 | partially-met | P19-AUD-001 |
| C / P19-AC-001-04 | S: P19-AC-001; sections 3–10 | partially-met | P19-AUD-009 |
| C / P19-AC-002-01 | S: P19-AC-002; sections 3–10 | met | — |
| C / P19-AC-002-02 | S: P19-AC-002; sections 3–10 | met | — |
| C / P19-AC-002-03 | S: P19-AC-002; sections 3–10 | met | — |
| C / P19-AC-002-04 | S: P19-AC-002; sections 3–10 | met | — |
| C / P19-AC-003-01 | S: P19-AC-003; sections 3–10 | met | — |
| C / P19-AC-003-02 | S: P19-AC-003; sections 3–10 | not-assessable | P19-AUD-009 |
| C / P19-AC-003-03 | S: P19-AC-003; sections 3–10 | met | — |
| C / P19-AC-004-01 | S: P19-AC-004; sections 3–10 | met | — |
| C / P19-AC-004-02 | S: P19-AC-004; sections 3–10 | met | — |
| C / P19-AC-004-03 | S: P19-AC-004; sections 3–10 | not-assessable | P19-AUD-009 |
| C / P19-AC-004-04 | S: P19-AC-004; sections 3–10 | met | — |
| C / P19-AC-005-01 | S: P19-AC-005; sections 3–10 | met | — |
| C / P19-AC-005-02 | S: P19-AC-005; sections 3–10 | met | — |
| C / P19-AC-005-03 | S: P19-AC-005; sections 3–10 | not-assessable | P19-AUD-009 |
| C / P19-AC-005-04 | S: P19-AC-005; sections 3–10 | met | — |
| C / P19-AC-006-01 | S: P19-AC-006; sections 3–10 | met | — |
| C / P19-AC-006-02 | S: P19-AC-006; sections 3–10 | partially-met | P19-AUD-003 |
| C / P19-AC-006-03 | S: P19-AC-006; sections 3–10 | not-assessable | P19-AUD-009 |
| C / P19-AC-006-04 | S: P19-AC-006; sections 3–10 | partially-met | P19-AUD-003 |
| C / P19-AC-006-05 | S: P19-AC-006; sections 3–10 | met | — |
| C / P19-AC-006-06 | S: P19-AC-006; sections 3–10 | met | — |
| C / P19-AC-006-07 | S: P19-AC-006; sections 3–10 | met | — |
| C / P19-AC-007-01 | S: P19-AC-007; sections 3–10 | met | — |
| C / P19-AC-007-02 | S: P19-AC-007; sections 3–10 | met | — |
| C / P19-AC-007-03 | S: P19-AC-007; sections 3–10 | met | — |
| C / P19-AC-007-04 | S: P19-AC-007; sections 3–10 | met | — |
| C / P19-AC-007-05 | S: P19-AC-007; sections 3–10 | met | — |
| C / P19-AC-008-01 | S: P19-AC-008; sections 3–10 | met | — |
| C / P19-AC-008-02 | S: P19-AC-008; sections 3–10 | met | — |
| C / P19-AC-008-03 | S: P19-AC-008; sections 3–10 | met | — |
| C / P19-AC-008-04 | S: P19-AC-008; sections 3–10 | met | — |
| C / P19-AC-008-05 | S: P19-AC-008; sections 3–10 | met | — |
| C / P19-AC-008-06 | S: P19-AC-008; sections 3–10 | partially-met | P19-AUD-003, P19-AUD-004 |
| C / P19-AC-008-07 | S: P19-AC-008; sections 3–10 | met | — |
| C / P19-AC-008-08 | S: P19-AC-008; sections 3–10 | partially-met | P19-AUD-004 |
| C / P19-AC-009-01 | S: P19-AC-009; sections 3–10 | met | — |
| C / P19-AC-009-02 | S: P19-AC-009; sections 3–10 | met | — |
| C / P19-AC-009-03 | S: P19-AC-009; sections 3–10 | met | — |
| C / P19-AC-009-04 | S: P19-AC-009; sections 3–10 | met | — |
| C / P19-AC-009-05 | S: P19-AC-009; sections 3–10 | met | — |
| C / P19-AC-009-06 | S: P19-AC-009; sections 3–10 | not-assessable | P19-AUD-009 |
| C / P19-AC-010-01 | S: P19-AC-010; sections 3–10 | met | — |
| C / P19-AC-010-02 | S: P19-AC-010; sections 3–10 | met | — |
| C / P19-AC-010-03 | S: P19-AC-010; sections 3–10 | partially-met | P19-AUD-002, P19-AUD-003 |
| C / P19-AC-010-04 | S: P19-AC-010; sections 3–10 | partially-met | P19-AUD-002, P19-AUD-007 |
| C / P19-AC-010-05 | S: P19-AC-010; sections 3–10 | met | — |
| C / P19-AC-010-06 | S: P19-AC-010; sections 3–10 | not-met | P19-AUD-005 |
| C / P19-AC-010-07 | S: P19-AC-010; sections 3–10 | partially-met | P19-AUD-006 |
| C / P19-AC-010-08 | S: P19-AC-010; sections 3–10 | not-assessable | P19-AUD-009 |
| C / P19-AC-011-01 | S: P19-AC-011; sections 3–10 | met | — |
| C / P19-AC-011-02 | S: P19-AC-011; sections 3–10 | partially-met | P19-AUD-001 |
| C / P19-AC-011-03 | S: P19-AC-011; sections 3–10 | partially-met | P19-AUD-008 |
| C / P19-AC-011-04 | S: P19-AC-011; sections 3–10 | met | — |
| C / P19-AC-011-05 | S: P19-AC-011; sections 3–10 | partially-met | P19-AUD-009 |
| C / P19-AC-012-01 | S: P19-AC-012; sections 3–10 | met | — |
| C / P19-AC-012-02 | S: P19-AC-012; sections 3–10 | met | — |
| C / P19-AC-012-03 | S: P19-AC-012; sections 3–10 | partially-met | P19-AUD-008 |
| C / P19-AC-012-04 | S: P19-AC-012; sections 3–10 | not-assessable | P19-AUD-009 |
| C / P19-AC-013-01 | S: P19-AC-013; sections 3–10 | met | — |
| C / P19-AC-013-02 | S: P19-AC-013; sections 3–10 | met | — |
| C / P19-AC-013-03 | S: P19-AC-013; sections 3–10 | met | — |
| C / P19-AC-013-04 | S: P19-AC-013; sections 3–10 | met | — |
| C / P19-AC-013-05 | S: P19-AC-013; sections 3–10 | met | — |
| C / P19-PRIN-002 | N: PRIN-002 | met | — |
| C / P19-PRIN-004 | N: PRIN-004 | partially-met | P19-AUD-003, P19-AUD-004, P19-AUD-007, P19-AUD-008 |
| C / P19-PHASE-001 | N: PHASE-001 | met | — |
| C / P19-AUTH-004 | N: AUTH-004 | met | — |
| P / P19-AC-001 | S: Section 7, table row P19-AC-001 | met | — |
| P / P19-AC-002 | S: Section 7, table row P19-AC-002 | met | — |
| P / P19-AC-003 | S: Section 7, table row P19-AC-003 | partially-met | P19-AUD-002 |
| P / P19-AC-004 | S: Section 7, table row P19-AC-004 | partially-met | P19-AUD-002 |
| P / P19-AC-005 | S: Section 7, table row P19-AC-005 | partially-met | P19-AUD-002 |
| P / P19-AC-006 | S: Section 7, table row P19-AC-006 | met | — |
| P / P19-AC-007 | S: Section 7, table row P19-AC-007 | met | — |
| P / P19-AC-008 | S: Section 7, table row P19-AC-008 | met | — |
| P / P19-AC-009 | S: Section 7, table row P19-AC-009 | met | — |
| P / P19-AC-010 | S: Section 7, table row P19-AC-010 | met | — |
| P / P19-AC-011 | S: Section 7, table row P19-AC-011 | met | — |
| P / P19-AC-012 | S: Section 7, table row P19-AC-012 | met | — |
| P / P19-AC-013 | S: Section 7, table row P19-AC-013 | met | — |
| P / P19-EXIT-GATES | S: Section 8, verification union | not-met | P19-AUD-001 |
