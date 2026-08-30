# Phase 18 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `e10b49f77361516f750166624f02ec0bb508be4018ac93efaef30297d871baa0`
**Remediation head**: `b9af27fd7944b00566d8dd7443936d9a5031f0e2`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged: none; unstaged: none; untracked: none
**Unresolved decisions**: none

The red-before observations below are bound to the remediation head by an empty
diff across every affected production and test path from both indexed audit
heads. Focused current-head tests were also run from an isolated `git archive`;
where an existing test encodes the defect, the plan records that inverse
expectation and requires the apply run to make the same behavioral check red
before changing production code.

## Current Finding Verification

| Source run / Finding ID | Verification | Source and final severity + rationale | Closure key/family | Batch | Decision |
|---|---|---|---|---|---|
| `phase18-codex-gpt56-dbd984e-20260830t152407z` / `P18-AUD-001` | Confirmed | Major -> Major: incomplete retained-byte closure still verifies after unreserved-file or sidecar mutation. | `bundle.retained-byte-closure` / `bundle.durability` | B1 | `fix:seal-complete-trial-closure` |
| `phase18-codex-gpt56-dbd984e-20260830t152407z` / `P18-AUD-002` | Confirmed | Major -> Major: reporting still trusts mutable side files, publishes after verification failure, and can overwrite sealed output. | `report.sealed-input-and-output-isolation` / `report.offline-integrity` | B2 | `fix:derive-report-from-sealed-inputs` |
| `phase18-codex-gpt56-dbd984e-20260830t152407z` / `P18-AUD-003` | Confirmed | Major -> Major: valid Agent non-zero exits and timeouts still stop native grading and become infrastructure failures. | `agent-failure.native-graded-outcome` / `failure.classification` | B3 | `fix:score-agent-owned-failures` |
| `phase18-codex-gpt56-dbd984e-20260830t152407z` / `P18-AUD-004` | Confirmed | Major -> Major: the natural-exit branch still abandons inherited pipes and records cleanup as not required without tree verification. | `process.natural-exit-tree-cleanup` / `process.supervision` | B4 | `fix:verify-natural-exit-tree-cleanup` |
| `phase18-codex-gpt56-dbd984e-20260830t152407z` / `P18-AUD-005` | Confirmed | Major -> Major: Pier rewards still lack the native 0..=1 domain check and DeepSWE accepts any structurally verified oracle result. | `deepswe.reward-domain-and-positive-oracle` / `benchmark.oracle-integrity` | B5 | `fix:enforce-deepswe-reward-contract` |
| `phase18-codex-gpt56-dbd984e-20260830t152407z` / `P18-AUD-006` | Confirmed | Major -> Major: trial intents still select the first declared edge instead of the unique owning pair. | `trial.intent-edge-identity` / `experiment.pairing` | B6 | `fix:bind-intent-to-owning-edge` |
| `phase18-pi-glm53-432ff13-20260830t114647z` / `P18-AUD-001` | Confirmed | Major -> Major: the comparison vocabulary and runner still reclassify Agent-owned failure as infrastructure. | `agent-failure.native-graded-outcome` / `failure.classification` | B3 | `fix:score-agent-owned-failures` |
| `phase18-pi-glm53-432ff13-20260830t114647z` / `P18-AUD-002` | Confirmed | Minor -> Minor: verifier stdout, stderr, and native reports still reuse the Agent source identity. | `verifier-artifact.source-identity` / `bundle.provenance` | B7 | `fix:attribute-verifier-artifacts-to-grader` |
| `phase18-pi-glm53-432ff13-20260830t114647z` / `P18-AUD-003` | Confirmed | Minor -> Minor: the zero-reward DeepSWE oracle path is still accepted. | `deepswe.reward-domain-and-positive-oracle` / `benchmark.oracle-integrity` | B5 | `fix:enforce-deepswe-reward-contract` |
| `phase18-pi-glm53-432ff13-20260830t114647z` / `P18-AUD-004` | Confirmed | Minor -> Minor: the producer still executes 13 cases and writes `cases_run=12`. | `native-conformance.receipt-count` / `native-smoke.metadata` | B8 | `fix:derive-conformance-case-count` |

## Unresolved Decisions

| ID | Required decision | Why evidence cannot decide | Alternatives | Authority needed |
|---|---|---|---|---|
| none | none | All closure predicates follow from the registered Phase 18 specification and current crate contracts. | none | none |

## Closure Batches

### Batch B1: Seal the complete retained trial closure

**Closure predicate**: A sealed trial covers exactly its reserved artifacts and canonical control evidence; verification rejects missing, additional, mutated, or sidecar-divergent bytes.
**Dependencies**: none
**Verification union**: `cargo test -p opi-eval --lib bundle`; `cargo test -p opi-eval --test bundle_recompute`; `cargo test -p opi-eval --test phase18_assembled_smoke`

#### Fix B1.1: Make the bundle identity own all authoritative trial inputs and outputs

- **Finding source(s)**: `phase18-codex-gpt56-dbd984e-20260830t152407z` + `83e52033f3bd1eb534409ac144f502d5691e33f2d32e7ea282beec571bd06194` + `P18-AUD-001`
- **Decision**: `fix:seal-complete-trial-closure`
- **Verification status**: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-eval/src/bundle/mod.rs`, `crates/opi-eval/src/integrity.rs`, `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/src/trajectory/mod.rs`, `crates/opi-eval/tests/bundle_recompute.rs`, `crates/opi-eval/tests/phase18_assembled_smoke.rs`
- **Change kind**: behavioral
- **Change**: Add canonical byte accessors for the integrity record and provisional trajectory; reserve and stage the resolved experiment, integrity record, trajectory, normalized settled output, authority evidence, and all expected native artifacts before sealing. Make sealing require the staged-key set to equal the intent reservation and require the expected output to exist. Make verification read-only, compare manifest intent/settlement with their durable sidecars, enumerate the artifact tree, and reject any unmanifested, missing, non-file, or digest-mismatched entry. Treat the post-seal receipt as derived convenience data, not an input to bundle identity.
- **Closure predicate**: Adding `artifacts/native/rogue.txt`, corrupting `intent.json` or `settlement.json`, omitting the expected output, or leaving any intended artifact unstaged makes `RunBundle::verify` fail at `TrialDurability`; an unchanged complete bundle verifies byte-stably.
- **Red-before**: `python -c 'from pathlib import Path; s=Path("crates/opi-eval/src/bundle/mod.rs").read_text(encoding="utf-8"); v=s[s.index("pub(crate) fn verify"):s.index("fn read_covered",s.index("pub(crate) fn verify"))]; missing=[x for x in ("read_dir","intent.json","settlement.json") if x not in v]; assert not missing, missing'` -> FAIL with `['read_dir', 'intent.json', 'settlement.json']`; the indexed mutation reproductions also remain current because the affected-path diff is empty.
- **Green-after**: Run the same predicate plus `cargo test -p opi-eval --test bundle_recompute`; expect PASS with explicit rogue-file, sidecar-drift, reservation-equality, expected-output, and stable re-verification cases.

### Batch B2: Derive reports only from verified sealed inputs

**Closure predicate**: Reporting publishes only when every contributing bundle verifies, derives all headline and coverage facts from sealed content, and never overwrites a run artifact or prior output.
**Dependencies**: B1, B7
**Verification union**: `cargo test -p opi-eval --test bundle_recompute`; `cargo test -p opi-eval --test end_to_end_report`; `cargo test -p opi-eval --test report_contract`

#### Fix B2.1: Fail closed on mutation and isolate report output

- **Finding source(s)**: `phase18-codex-gpt56-dbd984e-20260830t152407z` + `83e52033f3bd1eb534409ac144f502d5691e33f2d32e7ea282beec571bd06194` + `P18-AUD-002`
- **Decision**: `fix:derive-report-from-sealed-inputs`
- **Verification status**: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-eval/src/cli/report.rs`, `crates/opi-eval/src/report.rs`, `crates/opi-eval/tests/bundle_recompute.rs`, `crates/opi-eval/tests/end_to_end_report.rs`, `crates/opi-eval/tests/report_contract.rs`
- **Change kind**: behavioral
- **Change**: Reconstruct trials, pair coverage, integrity provenance, rewards, and diagnostics from B1's verified bundle artifacts and manifest identities only. Return a typed non-published outcome and non-zero CLI exit on any bundle verification or sealed-input parse failure. Reject `--out` inside the run root and open external output with create-new semantics so neither sealed bytes nor prior reports can be replaced.
- **Closure predicate**: A covered-byte mutation prevents publication and exits non-zero; mutations of outer `run-report.json` or `receipt.json` cannot affect output; an in-run-root or existing `--out` target is rejected without changing its bytes.
- **Red-before**: `python -c 'from pathlib import Path; s=Path("crates/opi-eval/src/report.rs").read_text(encoding="utf-8"); bad=[x for x in ("run-report.json","receipt.json") if x in s]; assert not bad,bad'` -> FAIL with `['run-report.json', 'receipt.json']`; the indexed CLI mutation and overwrite reproductions remain current by empty affected-path diff.
- **Green-after**: Run the same predicate plus `cargo test -p opi-eval --test end_to_end_report` and `cargo test -p opi-eval --test report_contract`; expect PASS for mutation refusal, sealed-only derivation, output isolation, byte stability, and valid publication.

### Batch B3: Keep Agent-owned failures in the native-graded outcome class

**Closure predicate**: On a valid task, Agent non-zero exit/crash and Agent-owned timeout dispatch the pinned native grader and remain Agent outcomes; spawn, cancellation, adapter, evidence, integrity, grader, and infrastructure failures retain their distinct boundary behavior.
**Dependencies**: none
**Verification union**: `cargo test -p opi-eval --lib agent::process`; `cargo test -p opi-eval --test authority_boundaries`; `cargo test -p opi-eval --test phase18_assembled_smoke`

#### Fix B3.1: Separate scored Agent failure from authority-boundary failure

- **Finding source(s)**: `phase18-codex-gpt56-dbd984e-20260830t152407z` + `83e52033f3bd1eb534409ac144f502d5691e33f2d32e7ea282beec571bd06194` + `P18-AUD-003`; `phase18-pi-glm53-432ff13-20260830t114647z` + `a4c66e4ed4c1465766d75e66bb271bc51042208981507e044e5c511c23046d5b` + `P18-AUD-001`
- **Decision**: `fix:score-agent-owned-failures`
- **Verification status**: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-eval/src/agent/process.rs`, `crates/opi-eval/src/authority.rs`, `crates/opi-eval/src/comparison.rs`, `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/tests/authority_boundaries.rs`, `crates/opi-eval/tests/phase18_assembled_smoke.rs`
- **Change kind**: behavioral
- **Change**: Introduce an internal closed classification that distinguishes scored Agent outcomes (non-zero exit/crash and Agent-owned timeout) from actual authority-boundary failures (spawn refusal, cancellation source, adapter/evidence rejection, and infrastructure). Do not fail the authority ledger for scored Agent outcomes; dispatch the native verifier over the settled workspace, retain the Agent completion in the trial fact, and include the native reward in the Agent success/failure denominator. Preserve fail-closed refusal for every non-Agent boundary.
- **Closure predicate**: Non-zero and timeout cases each execute one native grade dispatch and pair as Agent outcomes, while adapter/evidence/spawn/cancellation cases execute zero unauthorized downstream grades and retain their exact boundary label.
- **Red-before**: `python -c 'from pathlib import Path; s=Path("crates/opi-eval/src/runner/experiment.rs").read_text(encoding="utf-8"); assert "ledger.fail(failure.boundary);" not in s,"all AgentCompletion::Failed values stop grade dispatch"'` -> FAIL; current `authority_boundaries` and `phase18_assembled_smoke` tests pass only because they assert the inverse infrastructure classification.
- **Green-after**: Run the same predicate plus `cargo test -p opi-eval --test authority_boundaries` and `cargo test -p opi-eval --test phase18_assembled_smoke`; expect PASS for scored non-zero/timeout and refused adapter/evidence/spawn/cancellation paths.

### Batch B4: Verify descendant cleanup after natural child exit

**Closure predicate**: A direct child's natural exit cannot return until its process tree is empty or cleanup failure is reported; descendants holding inherited pipes are terminated and verified within bounds.
**Dependencies**: none
**Verification union**: `cargo test -p opi-eval --lib process::tests`; `cargo test -p opi-eval --test phase18_assembled_smoke`

#### Fix B4.1: Apply the tree-cleanup state machine to natural exit

- **Finding source(s)**: `phase18-codex-gpt56-dbd984e-20260830t152407z` + `83e52033f3bd1eb534409ac144f502d5691e33f2d32e7ea282beec571bd06194` + `P18-AUD-004`
- **Decision**: `fix:verify-natural-exit-tree-cleanup`
- **Verification status**: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-eval/src/process.rs`, `crates/opi-eval/src/process/tree.rs`
- **Change kind**: behavioral
- **Change**: Retain whether bounded stream settlement reached EOF, query the process-tree guard after the direct child exits, and run the same terminate/reap/verify sequence when descendants or inherited-pipe holders remain. Report `NotRequired` only after observed tree emptiness; otherwise emit verified `TreeTerminated` or `TreeTerminationFailed`. Add Unix coverage using a background descendant that inherits stdout and outlives its parent.
- **Closure predicate**: The background descendant is gone before return and cleanup is verified; ordinary no-descendant exits remain bounded and report no cleanup work.
- **Red-before**: `python -c 'from pathlib import Path; s=Path("crates/opi-eval/src/process.rs").read_text(encoding="utf-8"); b=s[s.index("match decided"):s.index("kill_decision =>")]; assert "guard.terminate()" in b,"natural-exit branch never terminates/verifies descendants"'` -> FAIL; the indexed live-descendant reproduction remains current by empty affected-path diff.
- **Green-after**: Run the same predicate plus `cargo test -p opi-eval --lib process::tests`; expect PASS including `natural_exit_terminates_inherited_pipe_descendant` on Unix.

### Batch B5: Enforce the DeepSWE native reward and oracle bar

**Closure predicate**: Pier import accepts only finite integral rewards in the native 0..=1 domain, and DeepSWE oracle preflight passes only with an explicitly known positive native reward.
**Dependencies**: none
**Verification union**: `cargo test -p opi-eval --lib benchmark::process`; `cargo test -p opi-eval --test native_driver`

#### Fix B5.1: Reject invalid Pier rewards and zero-reward oracle results

- **Finding source(s)**: `phase18-codex-gpt56-dbd984e-20260830t152407z` + `83e52033f3bd1eb534409ac144f502d5691e33f2d32e7ea282beec571bd06194` + `P18-AUD-005`; `phase18-pi-glm53-432ff13-20260830t114647z` + `a4c66e4ed4c1465766d75e66bb271bc51042208981507e044e5c511c23046d5b` + `P18-AUD-003`
- **Decision**: `fix:enforce-deepswe-reward-contract`
- **Verification status**: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-eval/src/benchmark/process.rs`, `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/tests/native_driver.rs`
- **Change kind**: behavioral
- **Change**: Apply the existing native reward-domain predicate to every Pier reward before conversion to `u64`; retain the native reward fact and require a `Known` value greater than zero for DeepSWE oracle admission. Add focused negative, above-one, fractional, unknown, zero-oracle, and positive-oracle cases.
- **Closure predicate**: `-1`, values above `1`, fractional values, and unknown reward fail import or preflight; `0` may be a valid measured trial reward but fails oracle admission; `1` passes.
- **Red-before**: `python -c 'from pathlib import Path; b=Path("crates/opi-eval/src/benchmark/process.rs").read_text(encoding="utf-8"); e=Path("crates/opi-eval/src/runner/experiment.rs").read_text(encoding="utf-8"); d=e[e.index("match inputs.adapter_key.as_str()"):e.index("};",e.index("match inputs.adapter_key.as_str()"))]; assert b.count("!(0.0..=1.0).contains(&reward)")>=2 and "=> true" not in d,"Pier lacks 0..=1 validation or DeepSWE preflight accepts any Verified result"'` -> FAIL.
- **Green-after**: Run the same predicate plus `cargo test -p opi-eval --lib benchmark::process` and `cargo test -p opi-eval --test native_driver`; expect PASS for the complete domain and oracle matrix.

### Batch B6: Bind each durable intent to its owning comparison edge

**Closure predicate**: Every trial has exactly one edge determined by its subject/task/group pairing, and its durable `PairIdentity` equals that edge; ambiguous or unpaired trial shapes are rejected before process effects.
**Dependencies**: none
**Verification union**: `cargo test -p opi-eval --test experiment_contract`; `cargo test -p opi-eval --test phase18_assembled_smoke`

#### Fix B6.1: Resolve the unique owning edge before intent publication

- **Finding source(s)**: `phase18-codex-gpt56-dbd984e-20260830t152407z` + `83e52033f3bd1eb534409ac144f502d5691e33f2d32e7ea282beec571bd06194` + `P18-AUD-006`
- **Decision**: `fix:bind-intent-to-owning-edge`
- **Verification status**: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/tests/fixtures/experiment/phase18-multi-edge.toml`, `crates/opi-eval/tests/phase18_assembled_smoke.rs`
- **Change kind**: behavioral
- **Change**: Resolve a trial's owning edge from the declared endpoint subject plus the matching counterpart trial in the same task/group; require exactly one match before intent publication and use that edge ID as `PairIdentity`. Add a fully admitted multi-edge fixture whose two groups exercise distinct edges and inspect every sealed intent.
- **Closure predicate**: Each multi-edge fixture intent names its own edge, no intent defaults to the first edge, and zero/multiple owner matches fail before Agent dispatch.
- **Red-before**: `python -c 'import re; from pathlib import Path; s=Path("crates/opi-eval/src/runner/experiment.rs").read_text(encoding="utf-8"); assert re.search(r"\.edges\(\)\s*\.first\(\)",s) is None,"trial intent always selects first edge"'` -> FAIL.
- **Green-after**: Run the same predicate plus `cargo test -p opi-eval --test phase18_assembled_smoke multi_edge`; expect PASS with distinct sealed pair identities and pre-dispatch ambiguity refusal.

### Batch B7: Attribute verifier artifacts to the grader

**Closure predicate**: Every sealed manifest entry identifies the component that produced its bytes; verifier stdout, stderr, and native report entries carry a grader/verifier source, never the Agent source.
**Dependencies**: none
**Verification union**: `cargo test -p opi-eval --test bundle_recompute`; `cargo test -p opi-eval --test report_contract`

#### Fix B7.1: Split Agent and verifier source identities

- **Finding source(s)**: `phase18-pi-glm53-432ff13-20260830t114647z` + `a4c66e4ed4c1465766d75e66bb271bc51042208981507e044e5c511c23046d5b` + `P18-AUD-002`
- **Decision**: `fix:attribute-verifier-artifacts-to-grader`
- **Verification status**: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-eval/src/report.rs`, `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/tests/bundle_recompute.rs`, `crates/opi-eval/tests/report_contract.rs`
- **Change kind**: behavioral
- **Change**: Create a distinct source identity from the pinned benchmark adapter/grader identity and use it for verifier streams and imported native reports. Make report artifact selection require the native grader role and grader source together instead of role-suffix recovery.
- **Closure predicate**: Manifest provenance assigns all Agent artifacts to `agent-<product>` and all verifier/native grader artifacts to the exact grader source; source-role mismatches fail sealed report reconstruction.
- **Red-before**: `python -c 'from pathlib import Path; s=Path("crates/opi-eval/src/runner/experiment.rs").read_text(encoding="utf-8"); assert "SourceIdentity::new(&format!(\"grader-" in s,"no grader-owned SourceIdentity exists"'` -> FAIL.
- **Green-after**: Run the same predicate plus `cargo test -p opi-eval --test bundle_recompute` and `cargo test -p opi-eval --test report_contract`; expect PASS for source attribution and source-sensitive native report selection.

### Batch B8: Emit the actual conformance case count

**Closure predicate**: The conformance-rerun receipt's `cases_run` equals the number of cases actually executed successfully in that invocation.
**Dependencies**: none
**Verification union**: `python scripts/test_verify_phase18_native_ci.py`; `python scripts/test_verify_phase18_native_artifact.py`

#### Fix B8.1: Count successful cases instead of hardcoding receipt metadata

- **Finding source(s)**: `phase18-pi-glm53-432ff13-20260830t114647z` + `a4c66e4ed4c1465766d75e66bb271bc51042208981507e044e5c511c23046d5b` + `P18-AUD-004`
- **Decision**: `fix:derive-conformance-case-count`
- **Verification status**: Confirmed
- **File(s)**: `CHANGELOG.md`, `scripts/phase18-native-smoke.sh`, `scripts/test_verify_phase18_native_ci.py`
- **Change kind**: metadata
- **Change**: Initialize a counter before the conformance loop, increment it only after each successful case, and serialize that value into the stage receipt. Extend the script contract test to compare the declared case list with the emitted counter path rather than pinning a second independent literal.
- **Closure predicate**: The present 13-case list emits `cases_run=13`, and adding or removing a case changes the receipt without another constant edit.
- **Red-before**: `python -c 'import pathlib,re; s=pathlib.Path("scripts/phase18-native-smoke.sh").read_text(encoding="utf-8"); b=s.split("for case_spec in",1)[1].split("; do",1)[0]; actual=b.count("\"agent ")+b.count("\"benchmark "); claimed=int(re.search(r"cases_run\": (\d+)",s).group(1)); assert actual==claimed,(actual,claimed)'` -> FAIL with `(13, 12)`.
- **Green-after**: Run the same predicate plus `python scripts/test_verify_phase18_native_ci.py`; expect PASS and receipt count 13.

## Final Verification

    cargo test -p opi-eval --lib
    cargo test -p opi-eval --test bundle_recompute
    cargo test -p opi-eval --test end_to_end_report
    cargo test -p opi-eval --test report_contract
    cargo test -p opi-eval --test authority_boundaries
    cargo test -p opi-eval --test phase18_assembled_smoke
    cargo test -p opi-eval --test experiment_contract
    cargo test -p opi-eval --test native_driver
    python scripts/test_verify_phase18_native_ci.py
    python scripts/test_verify_phase18_native_artifact.py
    python scripts/opi-doc-check.py
    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

## Exclusions

| Finding ID | Disposition | Current evidence/authority |
|---|---|---|
| none | none | All 10 current source findings are confirmed and assigned to a closure batch. |
