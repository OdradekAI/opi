# Phase 19 remediation result

**Status**: COMPLETE
**Audit index SHA-256**: `7550892cafbe6db2df1b56cd3f2eadb2104303428019940d4dc341e621f48f9d`
**Plan SHA-256**: `0bba60e040eaae1c4ede2e3740a05f7a6293d61a73d3628b25c63a9134c1f5ce`
**Remediation head**: `42c8fe19f633a3993bf535376911d159ef438980`
**Disposition records**: `remediation.result.dispositions.jsonl`
**Changed paths**: ["CHANGELOG.md", "crates/opi-eval/README.md", "crates/opi-eval/README.zh.md", "crates/opi-eval/integrations/harbor/opi_eval_harbor.py", "crates/opi-eval/integrations/pier/opi_eval_pier.py", "crates/opi-eval/scripts/test_native_integrations.py", "crates/opi-eval/src/benchmark/process.rs", "crates/opi-eval/src/cli/regrade.rs", "crates/opi-eval/src/cli/run.rs", "crates/opi-eval/src/regrade.rs", "crates/opi-eval/src/report.rs", "crates/opi-eval/src/runner/experiment.rs", "crates/opi-eval/tests/bundle_recompute.rs", "crates/opi-eval/tests/end_to_end_report.rs", "crates/opi-eval/tests/native_driver.rs", "crates/opi-eval/tests/report_contract.rs", "crates/opi-eval/tests/report_output_containment.rs"]

## Result and authority

B1, B2 and B3 are Closed. The exact 13-source union is retained: three planned
closures, eight Refuted findings with maintained counterexamples, C/010 Cannot
confirm, and P/001 Info/No action. No incidental repair was admitted. This result
does not grant Phase PASS; the live audit index remains revision 2, aggregate FAIL.

The current explicit apply invocation approved the fixed plan digest above.
Admission and final checks validated the live indexed set, exact committed head
and unchanged plan bytes. Only the carried-in approved plan pair was dirty at
admission; both files remain byte-identical to that baseline. No staged/untracked
baseline work existed. The 17 implementation/documentation/test paths above are
owned by plan records; the fixed result pair is the required output, not an
additional implementation surface. No dependency, lockfile, registered source,
implementation ledger, Git branch or commit was changed.

Current indexed identities:

- C: `phase19-codex-gpt6-c18efa8-20260917t133157z`, findings
  `67c0ab6ca8b90178e3caf2d7cdd6cb4a6c9a6b2fe342c85d8cbd3ef105c95b36`.
- P: `phase19-pi-glm53-d61590b-20260917t152753z`, findings
  `b85fa7a405aee5d3da9ce410bcd9619cf55af85fb3bce75a4c8b1f9aae96bc0b`.

## Dependency-ordered closure evidence

### B1 — stage deadlines

The maintained real Docker witness first establishes positive controls for all
three families and both subjects, then verifies time-cap, agent-cap,
verifier-cap, both-agent, both-verifier, minimum-cap, strict-task and
unbounded-task cases. It checks native command evidence/digests, distinct stopped
command identities, cancellation/exit 137, bounded monotonic elapsed time,
missing delayed completion, honest outcomes and cleanup isolation. Pier uses
the approved 30-second verifier control; tighter Agent/task controls remain
separate. Native wall timestamps remain diagnostic, since WSL wall time can
regress; they are not used to falsify a monotonic duration assertion.
The final rerun also retained TB2.1/TB3 unbounded-task wall-clock regression
diagnostic; its monotonic deadline and command/cleanup assertions passed.

Initial green: `/tmp/.tmpolz6Fi`, candidate-v2 material, 54 subject cases.
After B2 changed the integration modules, a new frozen stage-final material
replaced no earlier manifest. Final green: `/tmp/.tmpFQ6syf`, 54 subject cases in
1938.39 seconds. Independent exact-label inspection found 108 owned task
roots and zero remaining owned containers or neighboring witness containers.
The shared immutable image and neighboring container survived each trial's
cleanup; the witness removed its own neighbor at completion.

### B2 — actual verifier-only regrading

Regrade now dispatches explicit `uv run --locked --offline python -m MODULE REQUEST`
argv into the pinned Harbor/Pier verifier entry. It restores only declared,
digest-bound sufficient inputs into a fresh native verifier, with no Agent job,
oracle/nop substitute or manufactured Agent metadata. It binds source
bundle/trial/result, task/package, runner/module/image/build context, restored
input bytes, fresh verifier identity, raw result digest and binary native reward.
Reports use native-regrade/2 and regrade-record/2. Cancellation and output/time
bounds apply; independent scoped cleanup must settle before further dispatch.
Original roots and scores remain unchanged, and existing output roots reject.

Real no-model witness `/tmp/.tmpA281vu` passed with all three families and both
subject identities: six correct inputs returned 1, six incorrect inputs returned
0. DeepSWE used a real small Git base and committed patch. Timeout, missing or
tampered input/bundle, task/base/network drift, missing offline cache, existing
output and uncertain cleanup all rejected as asserted. A final cleanup-query
failure prevented the second dispatch. Provider invocation/log canaries stayed
absent and source tree/material hashes stayed unchanged. Independent inspection
of 18 fresh verifier settlements found no remaining containers. Maintained tests
also cover SIGTERM, child termination, relative source paths and explicit argv.

The witness image was separately prepared from immutable Ubuntu with Git/Python;
package acquisition happened only during preparation. Regrading requires cached
images, uses `--no-build --pull never`, and performs no package downloads. The
exact recipe, inventory and runtime image identities are retained below.

### B3 — metric meanings and observations

Normalized report/reporter /2 distinguishes TB2.1 native task success, TB3 task
success after declared collection, and DeepSWE resolved task after committed-patch
verification. Primary reward remains binary. Per-trial native `passed / total`
has separate numerator, denominator, raw grader digest and observation coverage.
Missing counters, zero totals, invalid counters and invalid trials have explicit
unknown reasons. Opaque native reward fields retain their original names;
abbreviations are not guessed. The run receipt projects the same observation;
offline reporting reconstructs it from verified sealed raw evidence.

Positive, zero and partial counts, missing/zero totals, opaque fields,
provider/verifier failures, unknown telemetry, corpus/subset/planned/attempted/
unexecuted and pair denominators passed. Partial test success coexists with task
reward zero. Existing bundles are read without rewriting/relabeling; deterministic
offline reconstruction, containment and immutability checks passed. No trial or
benchmark-family fraction is pooled into a new headline.

## Verification

Rust commands ran in WSL Ubuntu from `/mnt/d/Luiz/Odradek/opi`, using
`CARGO_TARGET_DIR=/mnt/e/opi/cargo-targets/opi-8abcf38f9871653e`,
`CARGO_INCREMENTAL=1` and explicit leases through the retained `check.sh` wrapper.
No cache was deleted and no per-session target directory was created. Logs below
are under `E:/opi/remediate-apply19-0bba60e0/`; the commands omit only this log/cache
wrapper. Final verification was deduplicated at each affected surface: B3 changed
derived Rust observations, while B2's final real witness and Python checks bind
the unchanged verifier runtime/modules. B1 was rerun explicitly after those
module changes. Default ignored tests are not counted as native verification.

| Surface | Exact command | Observed result |
|---|---|---|
| B1 initial matrix | `OPI_EVAL_NATIVE_WITNESS_MATERIAL=/home/luiz/.cache/opi-remediate-plan19-42c8fe1-43b5a495f4/candidate-v2/material.json cargo test -p opi-eval --locked --test native_driver remediation_native_stage_deadlines -- --exact --ignored --nocapture --test-threads=1` | PASS; 54/54 subjects; 1938.08 s; b1-matrix-r3.log |
| B1 final modules | `OPI_EVAL_NATIVE_WITNESS_MATERIAL=/home/luiz/.cache/opi-remediate-plan19-42c8fe1-43b5a495f4/stage-final/material.json cargo test -p opi-eval --locked --test native_driver remediation_native_stage_deadlines -- --exact --ignored --nocapture --test-threads=1` | PASS; 54/54 subjects; 1938.39 s; b1-matrix-final.log |
| B2 argv | `cargo test -p opi-eval --locked --test native_driver plan_002_regrade_dispatches_a_native_entrypoint -- --exact --nocapture` | PASS; 1 passed; b2-argv-r2.log |
| B2 maintained regressions | `cargo test -p opi-eval --locked --test native_driver native_offline_regrade` | PASS; 5 passed; b2-offline-final.log |
| B2 real verifier | `OPI_EVAL_NATIVE_WITNESS_MATERIAL=/home/luiz/.cache/opi-remediate-plan19-42c8fe1-43b5a495f4/regrade/material.json cargo test -p opi-eval --locked --test native_driver native_verifier_only_regrade_witness -- --exact --ignored --nocapture --test-threads=1` | PASS; 1 passed; 702.01 s; b2-native-final-r1.log |
| B3 meaning | `cargo test -p opi-eval --locked --test native_driver plan_011_native_metric_meanings_are_distinct -- --exact --nocapture` | PASS; 1 passed, three meanings; b3-metric-green.log |
| B3 report union | `cargo test -p opi-eval --locked --test report_contract --test end_to_end_report --test bundle_recompute --test report_output_containment` | PASS; 32 passed; b3-reports-r2.log |
| B3 producer/counters | `cargo test -p opi-eval --locked --test native_driver native_test_counts_ -- --nocapture` | PASS; 2 passed; b3-native-counts-r1.log; subsequent subset test also passed in final all-targets |
| Retained counterexamples | `cargo test -p opi-eval --locked --test native_driver remediation_ -- --nocapture` | PASS; 9 passed, 1 ignored; retained-regressions.log |
| Final Rust union | `cargo test -p opi-eval --all-targets --locked` | PASS; 309 passed, 0 failed, 9 ignored; final-all-targets.log |
| Lint | `cargo clippy -p opi-eval --all-targets --locked -- -D warnings` | PASS; final-clippy.log |
| Format | `cargo fmt --check --all` | PASS; final-fmt.log |
| Python union | `python3 -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py'` | PASS; 216 tests, 1 skipped; b2-python-r1.log |
| Python Docker | `python3 crates/opi-eval/scripts/test_native_integrations.py --docker` | PASS; 31 tests; b2-python-docker.log |
| Documentation | `python scripts/opi-doc-check.py` | PASS after result materialization |
| Whitespace | `git diff --check` | PASS after result materialization |
| Result binding | `python .agents/skills/_shared/scripts/validate_assurance_artifact.py result docs/snapshots/phase19/assurance/remediation.result.md` | PASS; exact 13-source coverage and unchanged approved dispositions |

### Red-before and retained implementation iterations

The approved red-before remains unchanged in each disposition. Fresh apply red
also reproduced C/002 (zero child arguments, exit 77; `b2-argv-red.log`) and C/011
(all three revisions had the same generic meaning; `b3-metric-red.log`). The
additional counter test initially failed against report /1 (`b3-counts-red.log`).

All diagnostic attempts remain in the external evidence directory. B1 r1/r2
corrected newly written witness assertions for inner-group termination and
regressing wall timestamps; independent inspections found no owned residue.
B2 iteration corrected private-entry input layout, relative root resolution,
the sealed native `max_timeout_sec` field and Pier's basename collection path;
the final real witness then passed. The initial test syntax error and fixture
preparation failures were retained, not counted as green. B3's first report run
exposed a new test assertion using the wrong existing reward token; correcting
the test to `known:0(native-trial-result)` preserved that contract and passed.
These were explained iterations inside the approved batches, not scope expansion.

Preparation also retained the helper build cache miss caused by an added
`--network=none`; the corrected exact `--pull=false` build reused cached layers.
The helper index attestation changed while its executable manifest/config stayed
the same. A custom local image digest used in `FROM` attempted registry resolution
and was rejected; preparation instead reused the immutable Ubuntu recipe before
freezing explicit runtime digests. None of these attempts was runtime regrade
acquisition or a change to an official task, dependency or lockfile.

## Frozen native material and raw evidence

Pinned runners: Harbor `4407eb5227a2ff4f0d3f16b2eb48849382fdf276`; Pier
`c1ebc6d145b40fae8425215e3fca528945065124`. Material base directory:
`/home/luiz/.cache/opi-remediate-plan19-42c8fe1-43b5a495f4/`.

| Material | SHA-256 |
|---|---|
| `candidate-v2/material.json` | `4de2cdab3b6a3b5d2569bc9dda5798f0be78e1d1332eeab07e5c63f6a872ef0a` |
| `stage-final/material.json` | `242765a5ace340f1e97958a5600d12584b3a436874743c6c2a52d909cda6cb6d` |
| `regrade/material.json` | `88d9d11bf134369af26284e73fcf9f12f2dbba12e1cfd6999105778332f68bac` |
| Regrade image recipe | `50dcc4cb31f83160926c1f628fddd4694e740df561926085cecb04c6a0420eea` |
| Regrade package inventory | `9d0adb3bd1cc1be1ea4691cf4d04ed4646fb90cc304f62891faa05f67aee5f4e` |

Immutable preparation base:
`ubuntu@sha256:b3cc40b72b93588182b5410f723c7aaf142363311c2aa993d8a453ddcbb3ae15`.
Regrade fixture image:
`opi-remediate-regrade-fixture-42c8fe1@sha256:b84ea696e9025b4bc9f745eddfb6d094968db746eb8c4b46ed8cf1fc05709111`.
Agent image:
`opi-remediate-regrade-agent-42c8fe1@sha256:148940025accd2376bb692e7c77deb9acb458fe192cef703c5f25480a7635f06`.
Verifier image:
`opi-remediate-regrade-verifier-42c8fe1@sha256:a73c334db0826f11c6e07fa32478a7634b4e3712adfdb122087df2033a88c861`.
DeepSWE fixture Git base: `53d725c42181cf61ec5e3983a34478ef175740a8`.
The material directory's `regrade/preparation-evidence.json`, image identities,
package inventory and build logs retain preparation provenance. Final modules
match both frozen manifests (`verify_final_material.py`).

| External evidence file | SHA-256 |
|---|---|
| `b1-matrix-r3.log` | `50329806aadac6a754486734a7b275a09efed381cb75e54b99af564fba0bf212` |
| `b1-matrix-final.log` | `488c639d66e23ea9cc514af6a0f839c275fcb9f0723b58b9063c7b69358fe7ec` |
| `b1-final-evidence.json` | `4a1d94f70d9018dc584e20fbeaa0501e23131e0b5a7c2f8dc0d83f22da2f415c` |
| `b2-offline-final.log` | `d537089833702b28a2862787a83143dad8cb9cf148a6ad67e77a7cfb2fe384d1` |
| `b2-native-final-r1.log` | `654f68ee143806b1757a1e3576404abe4fbf1c956f9939ec5d6c26306ce082b9` |
| `b2-native-evidence.json` | `94562d2ba4e56e237a08b4c9e6fd5ae32589822d7419407a3341e65fba81f92c` |
| `b3-metric-red.log` | `0d9859ba2509a5324e2058ef9df470991e13e90f0009fc730e971809b187aaa5` |
| `b3-metric-green.log` | `72a5108d7e9be7c0f2a61ddf5c688ada90367368f8d252188e7b9d858d0f4e44` |
| `b3-reports-r2.log` | `ecffc9c6fff33c6071316b822e109f3d9ff19d886c93112c3f9ecdffc4506560` |
| `final-all-targets.log` | `d32e4894dba93c6e98113ae98c6ebdb0dd10a3d63ddb0e5b6268b17a8a4db320` |
| `final-clippy.log` | `b4bed1141b6229c3c97a0c91ce2005f2f2f3994da2bb0825c2b404ebfcadb493` |
| `final-fmt.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `b2-python-r1.log` | `2c92164e9ad2c1ca01e09c86cf76a833d0a4a6c94cbb783c7faaf5e9b94dd6f3` |
| `b2-python-docker.log` | `bf4251a161c8131863c1716a756bbe3c12570b10f16c4e3d2c49aeec7d475325` |
| `retained-regressions.log` | `df10dd9aed3c455d1175d9b59a2c4afcc869a177f4fd3e24cf15b96f862257cd` |
| `final-owned-source-hashes.json` | `d5be79678cbe45750620fc184b82c559cac424b746f439bd3b6733e5c5b7544e` |

## Exclusions and materialization handoff

C/001,003,004,006,007,008,009,012 remain Refuted, with current regressions
retained and rerun. C/010 remains Cannot confirm: its thirty acceptance
obligations were reviewed against the registered P19-AC-001..013, but peer met
states are not closure. No historical acceptance/eval artifact was used as
evidence, no paid provider acceptance or real Phase exit was performed, and no
workspace clippy/test/doc-test/rustdoc gate is claimed. P/001 remains informational:
selected commit statistics were checked without consuming historical acceptance
contents; no history rewrite was attempted.

No commit was performed. A fresh audit/reviewer rerun requires user-authorized
materialization of the fixes plus current live set, a clean assurance directory,
resolved external evidence and any owning-workflow return. External native
evidence and caches remain local; a recipient must preserve or transfer them to
reproduce these claims. This workflow invokes no other explicit-only skill and
does not grant Phase PASS.

Test impact: **add** argv, real regrade, relative-source/cancellation/cleanup,
metric/counter/subset regressions; **update** stage witness, Python integration,
regrade and report tests; **retain** the eight counterexample findings and other
existing tests; **delete** none. README updates are bilingual. The fixed result
artifact has no Chinese counterpart.
