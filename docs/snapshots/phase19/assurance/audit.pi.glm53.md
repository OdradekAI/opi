# Phase 19 Audit

**Audit run ID**: `phase19-pi-glm53-7a0fe85-20261014t073512z`
**Audit head**: `7a0fe85ab00156bef687a69f5e413bf7e364573b`
**Reviewer ID**: `pi`
**Model ID**: `glm53`
**Reviewer identity**: pi coding agent harness
**Reviewer model ID**: `glm53`
**Model identity source**: operator-declared
**Independence**: fresh-context-same-family — fresh audit context sealed from a `git archive` export of `audit_head`; no prior audit/remediation conclusion, history run, or sibling peer output was read during the independent stages. `docs/audits/phase19-local-native-acceptance.md` and the live assurance directory contents were excluded from Steps 1–5.
**Baseline policy**: latest-committed-spec
**Verdict**: FAIL

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| .opi-impl-state.json | c65c10635c4bb8b1754625b94e8a0a15459eef6d65634610ba9b65ea9ba5337e | current committed source |
| docs/snapshots/phase19/opi-impl-state.json | 35b3378bd902cc976bb348d794dd1b5d3c5f160a58769c3aeb62fea175a7b7a9 | current committed source |
| docs/opi-spec.md | 1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db | current committed source |
| docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md | f507f29d91ddfc919ead6fa84df8d5f912b04af1d84ed5e68777e20c80c135c5 | stored-hash mismatch: both ledgers still record 5be1b609… (superseded revision); the committed bytes govern — DeepSWE scoped out, ≥4-trial exit minimum |
| docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.zh.md | 7d97f00f3a06723765e8cb2c8ea7ed13933defc8d7c230bdc0d165ad1284e7dc | stored-hash mismatch (d41c8264… recorded); en/zh identifier parity verified for all 13 criteria |

Requirements were sealed (19 records: 18 mandatory, 1 optional) from these sources before any production implementation was read. The registered graph (tasks 19.1–19.11) supplied scope and scenario verification routes, not conformance.

## Requirement Conformance

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| P19-AC-001 | design §7 AC-001 | doctor/prepare/plan/verify/report executed from an independent directory at audit_head (doctor exit 0 with full observations; two fresh prepare manifests from the operator cache); fail_closed_cli 8/8 | met | — |
| P19-AC-002 | design §7 AC-002 | NativeTrialBinding taxonomy reviewed; native_result_binding_rejects_every_substitution + conformance matrices passed; oracle attribution rejected on exit 0 | met | — |
| P19-AC-003 | design §7 AC-003 | native_tb21_artifact_witness PASSED at audit_head (fresh current-lock material, execution checker PASS); tb21-z2 live bundles verify + reconstruct deterministically | met | — |
| P19-AC-004 | design §7 AC-004 | binding and live z-batch evidence correct, but the declared witness command fails (nop cannot produce the agent-created declared artifact) | partially-met | P19-AUD-103 |
| P19-AC-005 (optional) | design §7 AC-005 deferral | DeepSWE code/evidence retained and excluded from current acceptance; deepswe witness passes; preserved bundles verify | met | — |
| P19-AC-006 | design §7 AC-006 | native_protocol_client_matrix PASSED (both exact clients × 3 protocols, loopback); 35 experiment_contract rejections; turn budgets; z-batch first-connection receipts | met | — |
| P19-AC-007 | design §7 AC-007 | plan module reviewed; full_inventory_preflight, frozen_subset_execution, tampered-plan and drift rejections passed; prepare rejects drift fail-closed (observed live) | met | — |
| P19-AC-008 | design §7 AC-008 | deterministic report contract 7/7 + real-bundle reconstruction with honest coverage; but native_provider_failure_report and remediation_native_stage_deadlines fail at audit_head | partially-met | P19-AUD-104, P19-AUD-105 |
| P19-AC-009 | design §7 AC-009 | verify/report on 5 real run roots deterministic and byte-identical; native_offline_regrade PASSED at audit_head; tamper/containment suites passed | met | — |
| P19-AC-010 | design §7 AC-010 | authority_boundaries 4/4; cancellation + owned-cleanup witnesses passed; canary/credential-removal tests passed; recovery/replacement tests passed | met | — |
| P19-AC-011 | design §7 AC-011 | dummy-credential test passed; workflows are thin callers; but the deterministic Python suite fails at audit_head (static-pin desync) | partially-met | P19-AUD-101 |
| P19-AC-012 | design §7 AC-012 | four attributable real trials (tb21-z2, tb30-z3) verify and reconstruct at audit_head; superseded attempts retained; but the prescribed acceptance-verifier route rejects the supplied schema-/1 root | partially-met | P19-AUD-102 |
| P19-AC-013 | design §7 AC-013 | no internal deps; package clean; pi-only works; Minimal Runtime unchanged; CHANGELOG Unreleased + bilingual README synchronized | met | — |
| P19-CTRL-003 | opi-spec CTRL-003 | canary gates, private credential staging, explicit capture refusal; suites passed | met | — |
| P19-CTRL-004 | opi-spec CTRL-004 | headline only from grader-sourced artifact; oracle only an explicit diagnostic | met | — |
| P19-CTRL-005 | opi-spec CTRL-005 | frozen-manifest pairing, visible exclusions, invalid-not-scored; real reports consistent | met | — |
| P19-CTRL-006 | opi-spec CTRL-006 | byte-stable offline reconstruction (real roots, twice each); identity reuse rejected | met | — |
| P19-CTRL-007 | opi-spec CTRL-007 | external corpora/runners/images via pinned locks; two real Agent + two grader integrations through shared conformance; crate unpublished | met | — |
| P19-GATES | design §8 | fmt/clippy/doc-check/opi-eval tests pass at audit_head; Python suite fails → union cannot pass on the final state | not-met | P19-AUD-101 |

## Standards Review

- **Rust correctness and dependency direction**: opi-eval declares only external dependencies (the `url` addition via root workspace deps is a benign parsing dependency, reviewed as code); `experiment_contract::independent_companion_dependency_boundary` enforces the boundary by test. No Agent Core seam was introduced; capture runtime stays binary-local with the pure filter shared into lib tests.
- **Failure behavior**: typed errors throughout (`NativeBindingError`, `PrepareError::Rejected`, `PlanError`, `RunError::Native`); the authority ledger mechanically stops later transitions after boundary failures. Fail-closed direction verified live (material digest drift, manifest schema, capture ceiling).
- **Documentation lockstep**: bilingual READMEs document the same eight command surfaces; zh explicitly separates byte verification from actual regrading; CHANGELOG Unreleased records the deliberate 0.x changes. `opi-doc-check.py` PASS.
- **Metadata drift (narrative, no product defect)**: both implementation ledgers still store the superseded design-spec digests (5be1b609…/d41c8264…) after `d54ae48` revised the source; registration authority (`opi-implement`) was not re-run. Also, one remediation commit touched `crates/opi-coding-agent/tests/json_streaming_cli.rs` (+19 lines, test-only D-Bus isolation) outside any recorded task-owned path — benign and pro-determinism, noted for the record.

## Spec Review

The committed design revision governs: DeepSWE deferred (AC-005 not a current obligation; existing interfaces, code and evidence retained — verified), Phase-exit minimum four real trials across TB2.1/TB3 for both Agents on the first connection. The substance of P19-AC-012 is independently verified at audit_head: tb21-z2 (openssl-selfsigned-cert, pi 1 / opi 1) and tb30-z3 (foodstuff-beta-activity, pi 0 / opi 0) — all four bundles seal-verify with the audit_head binary and reconstruct byte-identically; zero scores are retained as valid graded outcomes; superseded invalid attempts stay in evidence with explicit reasons. What fails is the *prescribed structured verification route* (findings P19-AUD-102) and three committed native verification commands (P19-AUD-103/104/105). Oracle remains an explicitly selected diagnostic only; no cross-benchmark headline exists; subset scopes are labelled.

## Security, Invariants, Integration, Test Quality, and Residuals

- **Security/authority**: credential references resolve only inside the supervised Agent launch; canary leakage blocks bundle and report; the generated verifier itself proves the staged credential's absence; private staging directories are removed. The 64 MiB capture ceiling is a deliberate privacy/cost bound, but it currently converts over-ceiling valid-timeout trials into invalid ones (P19-AUD-105).
- **Invariants/integration**: one sealed trial interpretation (GradedNative / InvalidExecution / NoOutcome) consumes the binding, the agent record and the lifecycle; provider failures never score; run-control binds the planned denominator before any effect; planned coverage survives total bundle loss.
- **Test quality**: 230 deterministic Rust tests + 194 lib tests pass; 9 of 14 ignored native-class tests were executed at audit_head and pass (tb21 witness, tb3 binding cases, deepswe witness, offline regrade, protocol matrix, turn budgets, control proof, cancellation, owned cleanup); 3 fail (findings above); 2 could not run for lack of the never-preserved combined three-family/client-regrade material (`native_offline_regrade_all_families`, `native_real_client_offline_regrade`) — recorded as a supply limitation, with their deterministic equivalents passing.
- **Residuals**: no dead branches or speculative seams found in the reviewed modules; the DeepSWE family is intentionally retained-but-excluded.

## Minimum-change Conformance

| Task | Status | Evidence |
|---|---|---|
| 19.1 | conforming | NativeTrialBinding + run_trial settlement at the recorded modules; closure tests present and passing |
| 19.2 | conforming | connection/control projection in experiment.rs; protocol matrix passes with exact clients |
| 19.3 | drifted | witness expectation contradicts its own binding contract (P19-AUD-103) |
| 19.4 | conforming | Pier family parse + pristine-verifier witness passes (family now scoped out; retention verified) |
| 19.5 | drifted | prepare works at audit_head, but the producer/workspace-input hash sync obligation was not maintained (P19-AUD-101) |
| 19.6 | conforming | plan module + CLI facade at recorded paths; tests pass |
| 19.7 | conforming | verify/report split with honest coverage; tests pass |
| 19.8 | conforming | native offline regrade passes at audit_head |
| 19.9 | conforming | cancellation/owned-cleanup witnesses pass; recovery/replacement tests pass |
| 19.10 | drifted | CI thin callers hold, but the Python regression gate fails at the final state and the verifier realignment orphaned the supplied evidence root (P19-AUD-101/102) |
| 19.11 | drifted | acceptance substance verified at bundle level, but the prescribed verification command fails against the supplied root (P19-AUD-102) |

## Findings

### P19-AUD-101: Static external lock workspace_input pin desynchronized from Cargo.lock at the final state

- Axis: standards
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-011, P19-GATES
- Claim: at audit_head the static lock records `cargo_lock_sha256=261a3ec5…` while the committed `Cargo.lock` is `e2adf702…` (changed by `d54ae48` adding the `url` dependency), so the declared Python regression gate fails and cold Opi materialization from the current workspace is rejected fail-closed.
- Evidence: failing `test_repository_workspace_hash…` (294 ran, 1 failure); hash comparison both sides at 3908fba (match) vs 7a0fe85 (differ); `materialize-external-locks.sh:324` rejection message.
- Refutation attempted: confirmed the failure is deterministic (hash equality), scoped to the two bytes in question, and absent at the operator's evidence commit; no alternate lock-update path exists in the tree (last lock change e40c46f).
- Suggested closure: update `workspace_input.cargo_lock_sha256` to the committed `Cargo.lock` digest (re-running the prepare/admission chain for the pin) so the Python suite and cold Opi preparation pass at the final state.

### P19-AUD-102: Acceptance evidence handoff stale: schema-/1 root rejected by the audit_head acceptance verifier; final z-batch never re-packaged

- Axis: integration
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-012
- Claim: `verify-local-acceptance.py` at audit_head requires manifest schema `/2` (per-family controls, DeepSWE excluded, four-trial minimum) but the only supplied redacted root is schema `/1` bound to the r2-generation (superseded tasks, companion 3908fba); the final z-batch satisfying the current criterion was never re-packaged, and the supplied TB2.1 material generation pins a stale static lock.
- Evidence: verifier exit 2 with schema rejection; manifest contents; no `/2` manifest anywhere under the root; r7 tb21 material digest-drift failure vs the same test passing with a freshly prepared material.
- Refutation attempted: bundle-level verification proves the underlying evidence still applies at audit_head (all six bundles verify; reports reconstruct deterministically), isolating the defect to the handoff contract rather than the evidence; the scoped `opi-artifact-audit` view is byte-identical to the original captures (spot-verified) and passes the generic artifact audit.
- Suggested closure: re-package the final z-batch (plus retained scoped-out DeepSWE evidence) into a schema-/2 evidence root with per-family control blocks, and refresh the tb21 material generation to the current lock; then re-run the prescribed verifier to a green result.

### P19-AUD-103: native_tb3_artifact_witness fails at audit_head: nop expectation contradicts the declared-artifact binding

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-004
- Claim: the committed TB3 witness command cannot pass: nop cannot produce the pinned task's only (agent-created) declared artifact, and the production binding — correctly, per the 19.3 contract and the operator's live z-batch semantics — rejects `declared-artifact-missing`, so the test's exit-0/collected-artifact expectation fails.
- Evidence: executed failure with the current-lock material (oracle passes, nop rejected after the real grader ran); task.toml artifacts declaration; conformance staging selects only upstream agents.
- Refutation attempted: no task/staging route creates the artifact for nop; the DeepSWE sibling deliberately carves out nop-committed-nothing because a patch is always collectable — TB3 files are not; the binding's rejection is the documented contract.
- Suggested closure: stage an incorrect witness that emits a wrong-content declared artifact (or align the expectation with the invalid classification), keeping the changed-artifact/grader-responds proof.

### P19-AUD-104: native_provider_failure_report fails at audit_head: accepted-reason enumeration omits the deliberate retry-exhaustion spelling

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-008
- Claim: the test's accepted classification reasons lack `agent-provider-failure:connection`, the intentional retry-exhaustion settlement of the real pi client (unit-tested in `pi.rs`); the observed run behavior is correct (invalid, unscored) but the committed command fails.
- Evidence: executed failure at rate-limit/pi with the full receipt; pi.rs implementation and its own unit test asserting that spelling.
- Refutation attempted: retry-success and auth-fail cases pass; the deterministic denominator equivalents pass; the mismatch is exactly the one spelling.
- Suggested closure: add the spelling (or match the `agent-provider-failure:` prefix) to the accepted list.

### P19-AUD-105: Stage-deadline witness fails: 64 MiB native capture ceiling turns graded benchmark-valid timeouts into invalid trials

- Axis: invariants
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-008
- Claim: the time-cap case settles both trials `native-capture-refused` (invalid) instead of graded timeouts because the captured stream exceeds the fixed 64 MiB ceiling; the same ceiling refused the operator's two real DeepSWE Opi attempts by 377/185 bytes, so the "benchmark-valid timeout retains its native metric" semantics are unreachable for over-ceiling streams.
- Evidence: two deterministic reproductions with retained run reports (valid=0/invalid=2, exit 1); operator-record capture-refusal entries; STREAM_LIMIT_BYTES and the classification path.
- Refutation attempted: positive cases pass and the capture refusal is honest (explicit, evidence-boundary, no partial export); environment-sensitivity is acknowledged (stream volume × deadline), but the real DeepSWE refusals show the collision is not hypothetical.
- Suggested closure: decide the intended semantics for over-ceiling streams (raise/configure the bound with provenance, or truncate-with-proof while preserving timeout grading), then make the witness and the live path agree.

## Verification Commands

| Command | Result | Requirement/finding |
|---|---|---|
| cargo test -p opi-eval --test {12 deterministic targets} | PASS (106 tests) | AC-001..013, CTRL-* |
| cargo test -p opi-eval --test native_driver (default) | PASS (124 passed, 14 ignored) | AC-002..010 |
| cargo test -p opi-eval --lib | PASS (194) | units |
| python3 -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py' | FAIL (294 ran, 1 failure, 1 skip) | P19-AUD-101 |
| cargo fmt --check --all; cargo clippy -p opi-eval --all-targets -- -D warnings; python3 scripts/opi-doc-check.py | PASS | P19-GATES |
| native_tb21_artifact_witness --ignored --exact (fresh material) | PASS (checker PASS) | AC-003 |
| native_tb3_artifact_witness --ignored --exact (r9 material) | FAIL (nop case) | P19-AUD-103 |
| native_deepswe_patch_witness --ignored --exact | PASS | AC-005 |
| native_offline_regrade --ignored --exact | PASS | AC-009 |
| native_protocol_client_matrix --ignored --exact | PASS | AC-006 |
| native_provider_failure_report --ignored --exact | FAIL (rate-limit/pi) | P19-AUD-104 |
| remediation_native_stage_deadlines --ignored --exact (×2) | FAIL (time-cap) | P19-AUD-105 |
| native_owned_resource_cancellation; native_conformance_owned_cleanup_witness; native_client_turn_budgets; native_operator_control_proof_validates_exact_clients --ignored --exact | PASS | AC-010, AC-006 |
| opi-eval doctor/prepare (tb21-z, tb30-z configs) | PASS | AC-001, AC-007 |
| opi-eval verify + report ×2 on tb21-r5, tb30-r1, deepswe-r1, tb21-z2, tb30-z3 | PASS (byte-identical reports) | AC-009, AC-012, CTRL-006 |
| python3 crates/opi-eval/scripts/verify-local-acceptance.py --root evidence-root --companion <audit_head build> | FAIL (exit 2, schema /1 vs /2) | P19-AUD-102 |
| python3 scripts/opi-artifact-audit.py evidence-root/opi-artifact-audit --workspace-root . | PASS | AC-012 scoped view |
| cargo package -p opi-eval --list --allow-dirty | PASS (clean inventory) | AC-013, CTRL-007 |

Not run: full `cargo test/clippy/doc --workspace` beyond opi-eval (no production code outside opi-eval changed since the operator's evidence commit; one test-only file elsewhere); `native_offline_regrade_all_families` and `native_real_client_offline_regrade` (the required combined three-family/client-regrade source material was never preserved in the supplied root).

## Verdict Rationale

Mandatory requirements P19-AC-004, P19-AC-008, P19-AC-011, P19-AC-012 and P19-GATES are not fully met at audit_head: five Major findings record a desynchronized workspace-input pin that fails the declared deterministic gate (P19-AUD-101), an acceptance handoff that the current verifier rejects (P19-AUD-102), and three committed native verification commands that fail when actually executed (P19-AUD-103/104/105). The verdict is mechanical: any mandatory state other than `met` yields FAIL. The core product loop is nevertheless strong — the four real admitted-family trials verify and reconstruct at audit_head, and nine of the executed native witnesses pass — so the gaps are concentrated in final-state synchronization and committed-test integrity rather than in scoring or attribution semantics.

<!-- opi-audit-history/1
{
  "audit\u005fhead": "7a0fe85ab00156bef687a69f5e413bf7e364573b",
  "audit\u005frun\u005fid": "phase19-pi-glm53-7a0fe85-20261014t073512z",
  "comparison": {
    "comparisons": [
      {
        "cause": "The remediation commit d54ae484f535cc417209b921e5c4fab0019f0522 changed the workspace dependency graph without refreshing the pinned workspace input digest.",
        "current\u005fevidence": "At 7a0fe85 the static lock records cargo\u005flock\u005fsha256 261a3ec5 while Cargo.lock hashes e2adf702; the Python regression test\u005frepository\u005fworkspace\u005finput\u005fmatches\u005fstatic\u005fpin fails (294 ran, 1 failure) and materialize-external-locks.sh rejects cold Opi preparation from the current workspace.",
        "current\u005ffinding\u005fid": "P19-AUD-101",
        "prior\u005fclaim": "No prior same-pair finding covered the static lock workspace\u005finput pin; the superseded run's audit\u005fhead predates the desynchronizing change (d54ae48 added the url dependency and moved Cargo.lock to e2adf702 without updating the pin).",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [
          {
            "checked": "Compared the operator lock copy against audit\u005fhead repository bytes via git show; confirmed the workspace\u005finput pin still records the 3908fba-era Cargo.lock digest.",
            "limitations": "operator-supplied copy of the 3908fba-era lock; the audit\u005fhead repository bytes were compared directly via git show",
            "path": "/home/luiz/phase19-acceptance-r2/evidence-root/static-lock/linux-x86\u005f64.json",
            "sha256": "faaf03d1c7cea822078d71c9c849ff38e0571ea989a863d2456ac1ec2abba4ea"
          }
        ],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Rerun python -m unittest discover -s crates/opi-eval/scripts -p 'test\u005f\u002a.py' after updating the pin; confirm 0 failures and a green cold prepare of the Opi agent from the current workspace."
      },
      {
        "cause": "Bounded inference: the acceptance verifier contract and the operator evidence were last synchronized at the 3908fba-era re-run; the final remediation commits revised the verifier without a matching evidence-root refresh.",
        "current\u005fevidence": "The gap changed shape rather than closing: raw operator evidence now exists (r2 root plus the final z-batch runs) and all four admitted-family trials verify and reconstruct deterministically with the audit\u005fhead binary, but the structured route fails because d54ae48 realigned verify-local-acceptance.py to manifest schema /2 (DeepSWE excluded, four-trial minimum, per-family controls) while the supplied root still carries the schema-/1 r2-generation manifest and the z-batch was never re-packaged; the supplied TB2.1 material generation also pins a stale static lock.",
        "current\u005ffinding\u005fid": "P19-AUD-102",
        "prior\u005fclaim": "Prior P19-AUD-001 (Major, spec): P19-AC-012 and the live portions of AC-003/004/005/006 could not be confirmed or falsified because the redacted operator evidence root was not supplied; only unverified claims in docs/audits/phase19-local-native-acceptance.md(.zh.md) and fixture-tested verifier tooling existed. The old closure claim is not adopted here.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-pi-glm53-9d805b4-20261004t094337z",
            "finding\u005fid": "P19-AUD-001",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Read the manifest; executed verify-local-acceptance.py at audit\u005fhead against it (exit 2, schema /1 vs /2); cross-checked trial ids against the z-batch run roots.",
            "limitations": "schema /1 manifest bound to companion commit 3908fba and superseded task selections; not acceptable input to the audit\u005fhead verifier",
            "path": "/home/luiz/phase19-acceptance-r2/evidence-root/manifest.json",
            "sha256": "31d290e2cefb4c617ea6801f678e7a15772f1e16cbb531975f0ac17aae8a4535"
          },
          {
            "checked": "Read the raw record; used only batch entries, exits and capture-refusal byte counts; independently re-verified the referenced run bundles with the audit\u005fhead binary.",
            "limitations": "operator narrative record; used only for raw execution facts (batch entries, exits, capture refusals), not as closure evidence",
            "path": "/home/luiz/phase19-acceptance-r2/operator-record.json",
            "sha256": "95afee6d45b3088f99ed0835188107ab0c835855bcfc29d80dea0491e0477ea3"
          }
        ],
        "relationship": "missed-branch",
        "repair\u005fcommits": [],
        "verification": "Re-package the final z-batch into a schema-/2 root (per-family control blocks, refreshed tb21 material) and rerun verify-local-acceptance.py --root \u003cnew root\u003e to exit 0."
      },
      {
        "cause": "The d54ae48 witness revision pinned foodstuff-beta-activity and asserted collected.len()==1 for both cases without reconciling the nop expectation with the DeclaredArtifactMissing contract the same remediation enforced.",
        "current\u005fevidence": "At 7a0fe85 native\u005ftb3\u005fartifact\u005fwitness fails on the nop case: the pinned task declares only the agent-created /app/results.txt, the upstream nop agent never creates it, and the binding correctly rejects declared-artifact-missing while the test expects a bound reward-zero result.",
        "current\u005ffinding\u005fid": "P19-AUD-103",
        "prior\u005fclaim": "No prior same-pair finding covered the TB3 nop witness; the superseded run's material generation pinned different TB3 tasks and the current witness shape (foodstuff-beta-activity pin plus collected-artifact assertions) landed in d54ae48, after 9d805b4.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [
          {
            "checked": "Read the task declaration (artifacts, separate verifier, timeouts) during the executed witness failure analysis.",
            "limitations": "cache-resident pinned task bytes",
            "path": "/home/luiz/phase19-acceptance-r2/cache/corpora/terminal-bench-3.0/tasks/foodstuff-beta-activity/task.toml",
            "sha256": "5090e2cf2308f9d0e516c785c106831f56e43c9c42e942d984dc00df81a4db24"
          }
        ],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Stage an incorrect witness that emits a wrong-content declared artifact (or align the expectation with the invalid classification) and rerun the exact ignored filter to green."
      },
      {
        "cause": "d54ae48 renamed and re-shaped the witness (removing the native\u005fclient\u005fprovider\u005ffailure\u005freport split) without absorbing the retry-exhaustion reason prefix that b392912 had introduced and unit-tested.",
        "current\u005fevidence": "The same command now executes and fails behaviorally: the rate-limit/pi case classifies correctly as invalid-execution/agent-provider-failure:connection (unscored), but the accepted-reason enumeration omits that deliberate spelling, so the committed command is red rather than vacuous.",
        "current\u005ffinding\u005fid": "P19-AUD-104",
        "prior\u005fclaim": "Prior P19-AUD-003 (Minor, test-quality): the registered AC-008 command selected zero tests because the only test named native\u005fprovider\u005ffailure\u005freport was not ignored while the intended witness was named native\u005fclient\u005fprovider\u005ffailure\u005freport, making the recorded command vacuous. That selection defect is repaired at audit\u005fhead (the test is ignored and executes under --ignored --exact); the old claim is closed by current observation, not adopted.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-pi-glm53-9d805b4-20261004t094337z",
            "finding\u005fid": "P19-AUD-003",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Read the raw record; used only batch entries, exits and capture-refusal byte counts; independently re-verified the referenced run bundles with the audit\u005fhead binary.",
            "limitations": "used only for the raw fact that provider-side terminal failures stayed unscored in the accepted z-batch; the reason-spelling mismatch is established from audit\u005fhead source and executed test output",
            "path": "/home/luiz/phase19-acceptance-r2/operator-record.json",
            "sha256": "95afee6d45b3088f99ed0835188107ab0c835855bcfc29d80dea0491e0477ea3"
          }
        ],
        "relationship": "missed-branch",
        "repair\u005fcommits": [
          "d54ae484f535cc417209b921e5c4fab0019f0522"
        ],
        "verification": "Add the agent-provider-failure prefix to the accepted reason list and rerun the exact ignored filter with admitted clients."
      },
      {
        "cause": "Bounded inference: the deadline witness and the capture ceiling were tuned independently; high-volume timeout streams cannot both stay under the ceiling and demonstrate graded-timeout semantics.",
        "current\u005fevidence": "The same test still fails, now through a different mechanism reproduced twice: the time-cap case's captured stream exceeds the fixed 64 MiB STREAM\u005fLIMIT\u005fBYTES ceiling, both trials settle invalid with native-capture-refused, and the run exits 1 while the test expects two valid graded timeouts; the operator's real DeepSWE Opi attempts were refused by the same ceiling (377 B and 185 B over).",
        "current\u005ffinding\u005fid": "P19-AUD-105",
        "prior\u005fclaim": "Prior P19-AUD-002 (Minor, test-quality): remediation\u005fnative\u005fstage\u005fdeadlines failed at 9d805b4 because it required a 'stage-witness-start' stderr marker that no producer emitted after 0fa995d. That specific marker defect is repaired at audit\u005fhead (the positive cases now pass); the old claim is closed by current observation, not adopted.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-pi-glm53-9d805b4-20261004t094337z",
            "finding\u005fid": "P19-AUD-002",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Read the raw record; used only batch entries, exits and capture-refusal byte counts; independently re-verified the referenced run bundles with the audit\u005fhead binary.",
            "limitations": "corroborating live refusal byte counts only; the audit\u005fhead reproductions ran in private temp roots and are summarized in the installed core's checks",
            "path": "/home/luiz/phase19-acceptance-r2/operator-record.json",
            "sha256": "95afee6d45b3088f99ed0835188107ab0c835855bcfc29d80dea0491e0477ea3"
          }
        ],
        "relationship": "missed-branch",
        "repair\u005fcommits": [
          "d54ae484f535cc417209b921e5c4fab0019f0522"
        ],
        "verification": "Decide the intended over-ceiling semantics (bound provenance or truncate-with-proof preserving timeout grading), then rerun the exact ignored filter deterministically to green."
      },
      {
        "cause": "The annotation was refreshed when the material schema advanced to /7.",
        "current\u005fevidence": "At 7a0fe85 the ignore annotations reference material/7 and the witness passes when executed with a current material (native\u005fconformance\u005fowned\u005fcleanup\u005fwitness passed in 110.2s during this run); no current finding exists.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Prior P19-AUD-004 (Info, residuals): the ignore annotation on native\u005fconformance\u005fowned\u005fcleanup\u005fwitness referenced the superseded material/6 schema while the test consumes controlled material/7.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-pi-glm53-9d805b4-20261004t094337z",
            "finding\u005fid": "P19-AUD-004",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Consumed as the witness input for the executed native\u005fconformance\u005fowned\u005fcleanup\u005fwitness and native\u005ftb3\u005fartifact\u005fwitness runs at audit\u005fhead.",
            "limitations": "witness input consumed during execution",
            "path": "/home/luiz/phase19-acceptance-r2/materials-r9/tb30/material.json",
            "sha256": "02e7cc157ca7e7b80b27b4b42936c1a0dbafbdb517d5f08cb95292fa9e53a3e9"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "d54ae484f535cc417209b921e5c4fab0019f0522"
        ],
        "verification": "None required beyond keeping annotations aligned with the consumed material schema."
      }
    ],
    "limitations": "The comparison covers only the superseded same-pair run cited above; no other history record or sibling output was ingested. Two raw\u005fevidence entries name consumed witness inputs whose per-file digests were not separately recorded (marked unhashed); their effect is established by the executed tests recorded in the installed core's checks. A post-install observation recorded as a bounded lead for fresh independent verification (not a current finding): both implementation ledgers still store the superseded design-spec digests (5be1b609…/d41c8264…) after the d54ae48 source revision, so registration metadata lags the registered sources.",
    "schema": "opi-audit-history/1",
    "scope": "Compared only the immediately superseded same-pair run phase19-pi-glm53-9d805b4-20261004t094337z (audit\u005fhead 9d805b498a56a276b141cc0c0728ce8201dbe6f3, FAIL), whose bytes this run's complete moved to assurance/history/phase19-pi-glm53-9d805b4-20261004t094337z/ and whose last carrying commit is 53aaf804c8f46049a3718868ac8906d77d2654d7 (verified by byte-identical digests of all four files). Every other history record (the older phase19-pi-glm53-2e82c99-20261002t204027z run, all codex runs, unrelated remediation material) and all concurrent sibling output stayed outside the comparison. Current behavior remains anchored to audit\u005fhead 7a0fe85ab00156bef687a69f5e413bf7e364573b.",
    "sources": [
      {
        "audit\u005frun\u005fid": "phase19-pi-glm53-9d805b4-20261004t094337z",
        "commit": "53aaf804c8f46049a3718868ac8906d77d2654d7",
        "id": "prior-findings",
        "path": "docs/snapshots/phase19/assurance/history/phase19-pi-glm53-9d805b4-20261004t094337z/audit.pi.glm53.findings.jsonl",
        "sha256": "377d730be9a09201c6056b3513739fa2f524ec684019ae516ef04182da56eeaf"
      },
      {
        "audit\u005frun\u005fid": "phase19-pi-glm53-9d805b4-20261004t094337z",
        "commit": "53aaf804c8f46049a3718868ac8906d77d2654d7",
        "id": "prior-meta",
        "path": "docs/snapshots/phase19/assurance/history/phase19-pi-glm53-9d805b4-20261004t094337z/audit.pi.glm53.meta.json",
        "sha256": "257f3dc0849103c5f7770168aa7ccb041a3269f3ea3cdbf59bef3b87de6e7e5d"
      }
    ],
    "status": "compared"
  },
  "core": {
    "findings\u005fsha256": "b37a36c99d81ea6dce462f067147abfeef0eb9d1519436bcf2052c5ffac75356",
    "meta\u005fsha256": "dd576698d3f542756eef50e30b0162c9fd1a6e3a75b0ed5e11ff064c8294de4a",
    "report\u005fbytes": 20273,
    "report\u005fsha256": "ce14b94aea0690d8169e9efad25fc75c79cb52b03544006f9c828bec7002cf48",
    "requirements\u005fsha256": "9b97b0d6e69f8ded9892dd0c20d6f66af61ff79852a7178d921917690ee7de4e"
  },
  "schema": "opi-audit-history/1"
}
-->

## Post-audit historical reference — advisory

This section was appended after installation of the independent core. It cannot change findings, requirements, independence or verdict. Historical closure claims require current independent verification.

- Owning audit run: phase19&#45;pi&#45;glm53&#45;7a0fe85&#45;20261014t073512z
- Audit head: 7a0fe85ab00156bef687a69f5e413bf7e364573b
- Core report bytes: 20273
- Core report SHA-256: ce14b94aea0690d8169e9efad25fc75c79cb52b03544006f9c828bec7002cf48
- Current findings SHA-256: b37a36c99d81ea6dce462f067147abfeef0eb9d1519436bcf2052c5ffac75356
- Comparison status: compared
- Inspected history: Compared only the immediately superseded same&#45;pair run phase19&#45;pi&#45;glm53&#45;9d805b4&#45;20261004t094337z &#40;audit&#95;head 9d805b498a56a276b141cc0c0728ce8201dbe6f3, FAIL&#41;, whose bytes this run&#39;s complete moved to assurance/history/phase19&#45;pi&#45;glm53&#45;9d805b4&#45;20261004t094337z/ and whose last carrying commit is 53aaf804c8f46049a3718868ac8906d77d2654d7 &#40;verified by byte&#45;identical digests of all four files&#41;&#46; Every other history record &#40;the older phase19&#45;pi&#45;glm53&#45;2e82c99&#45;20261002t204027z run, all codex runs, unrelated remediation material&#41; and all concurrent sibling output stayed outside the comparison&#46; Current behavior remains anchored to audit&#95;head 7a0fe85ab00156bef687a69f5e413bf7e364573b&#46;
- Limitations: The comparison covers only the superseded same&#45;pair run cited above; no other history record or sibling output was ingested&#46; Two raw&#95;evidence entries name consumed witness inputs whose per&#45;file digests were not separately recorded &#40;marked unhashed&#41;; their effect is established by the executed tests recorded in the installed core&#39;s checks&#46; A post&#45;install observation recorded as a bounded lead for fresh independent verification &#40;not a current finding&#41;: both implementation ledgers still store the superseded design&#45;spec digests &#40;5be1b609…/d41c8264…&#41; after the d54ae48 source revision, so registration metadata lags the registered sources&#46;

### Pinned historical sources

- prior&#45;findings: docs/snapshots/phase19/assurance/history/phase19&#45;pi&#45;glm53&#45;9d805b4&#45;20261004t094337z/audit&#46;pi&#46;glm53&#46;findings&#46;jsonl; commit 53aaf804c8f46049a3718868ac8906d77d2654d7; SHA-256 377d730be9a09201c6056b3513739fa2f524ec684019ae516ef04182da56eeaf; audit run phase19&#45;pi&#45;glm53&#45;9d805b4&#45;20261004t094337z
- prior&#45;meta: docs/snapshots/phase19/assurance/history/phase19&#45;pi&#45;glm53&#45;9d805b4&#45;20261004t094337z/audit&#46;pi&#46;glm53&#46;meta&#46;json; commit 53aaf804c8f46049a3718868ac8906d77d2654d7; SHA-256 257f3dc0849103c5f7770168aa7ccb041a3269f3ea3cdbf59bef3b87de6e7e5d; audit run phase19&#45;pi&#45;glm53&#45;9d805b4&#45;20261004t094337z

### Comparison 1: P19&#45;AUD&#45;101

- Relationship: new&#45;omission
- Prior claim (attributed): No prior same&#45;pair finding covered the static lock workspace&#95;input pin; the superseded run&#39;s audit&#95;head predates the desynchronizing change &#40;d54ae48 added the url dependency and moved Cargo&#46;lock to e2adf702 without updating the pin&#41;&#46;
- Current evidence: At 7a0fe85 the static lock records cargo&#95;lock&#95;sha256 261a3ec5 while Cargo&#46;lock hashes e2adf702; the Python regression test&#95;repository&#95;workspace&#95;input&#95;matches&#95;static&#95;pin fails &#40;294 ran, 1 failure&#41; and materialize&#45;external&#45;locks&#46;sh rejects cold Opi preparation from the current workspace&#46;
- Cause / missed boundary: The remediation commit d54ae484f535cc417209b921e5c4fab0019f0522 changed the workspace dependency graph without refreshing the pinned workspace input digest&#46;
- Suggested current verification: Rerun python &#45;m unittest discover &#45;s crates/opi&#45;eval/scripts &#45;p &#39;test&#95;&#42;&#46;py&#39; after updating the pin; confirm 0 failures and a green cold prepare of the Opi agent from the current workspace&#46;
- Candidate raw proof: /home/luiz/phase19&#45;acceptance&#45;r2/evidence&#45;root/static&#45;lock/linux&#45;x86&#95;64&#46;json; SHA-256 faaf03d1c7cea822078d71c9c849ff38e0571ea989a863d2456ac1ec2abba4ea; checked: Compared the operator lock copy against audit&#95;head repository bytes via git show; confirmed the workspace&#95;input pin still records the 3908fba&#45;era Cargo&#46;lock digest&#46;; limits: operator&#45;supplied copy of the 3908fba&#45;era lock; the audit&#95;head repository bytes were compared directly via git show

### Comparison 2: P19&#45;AUD&#45;102

- Relationship: missed&#45;branch
- Prior claim (attributed): Prior P19&#45;AUD&#45;001 &#40;Major, spec&#41;: P19&#45;AC&#45;012 and the live portions of AC&#45;003/004/005/006 could not be confirmed or falsified because the redacted operator evidence root was not supplied; only unverified claims in docs/audits/phase19&#45;local&#45;native&#45;acceptance&#46;md&#40;&#46;zh&#46;md&#41; and fixture&#45;tested verifier tooling existed&#46; The old closure claim is not adopted here&#46;
- Current evidence: The gap changed shape rather than closing: raw operator evidence now exists &#40;r2 root plus the final z&#45;batch runs&#41; and all four admitted&#45;family trials verify and reconstruct deterministically with the audit&#95;head binary, but the structured route fails because d54ae48 realigned verify&#45;local&#45;acceptance&#46;py to manifest schema /2 &#40;DeepSWE excluded, four&#45;trial minimum, per&#45;family controls&#41; while the supplied root still carries the schema&#45;/1 r2&#45;generation manifest and the z&#45;batch was never re&#45;packaged; the supplied TB2&#46;1 material generation also pins a stale static lock&#46;
- Cause / missed boundary: Bounded inference: the acceptance verifier contract and the operator evidence were last synchronized at the 3908fba&#45;era re&#45;run; the final remediation commits revised the verifier without a matching evidence&#45;root refresh&#46;
- Suggested current verification: Re&#45;package the final z&#45;batch into a schema&#45;/2 root &#40;per&#45;family control blocks, refreshed tb21 material&#41; and rerun verify&#45;local&#45;acceptance&#46;py &#45;&#45;root &#60;new root&#62; to exit 0&#46;
- Prior finding: phase19&#45;pi&#45;glm53&#45;9d805b4&#45;20261004t094337z / P19&#45;AUD&#45;001; source prior&#45;findings
- Candidate raw proof: /home/luiz/phase19&#45;acceptance&#45;r2/evidence&#45;root/manifest&#46;json; SHA-256 31d290e2cefb4c617ea6801f678e7a15772f1e16cbb531975f0ac17aae8a4535; checked: Read the manifest; executed verify&#45;local&#45;acceptance&#46;py at audit&#95;head against it &#40;exit 2, schema /1 vs /2&#41;; cross&#45;checked trial ids against the z&#45;batch run roots&#46;; limits: schema /1 manifest bound to companion commit 3908fba and superseded task selections; not acceptable input to the audit&#95;head verifier
- Candidate raw proof: /home/luiz/phase19&#45;acceptance&#45;r2/operator&#45;record&#46;json; SHA-256 95afee6d45b3088f99ed0835188107ab0c835855bcfc29d80dea0491e0477ea3; checked: Read the raw record; used only batch entries, exits and capture&#45;refusal byte counts; independently re&#45;verified the referenced run bundles with the audit&#95;head binary&#46;; limits: operator narrative record; used only for raw execution facts &#40;batch entries, exits, capture refusals&#41;, not as closure evidence

### Comparison 3: P19&#45;AUD&#45;103

- Relationship: new&#45;omission
- Prior claim (attributed): No prior same&#45;pair finding covered the TB3 nop witness; the superseded run&#39;s material generation pinned different TB3 tasks and the current witness shape &#40;foodstuff&#45;beta&#45;activity pin plus collected&#45;artifact assertions&#41; landed in d54ae48, after 9d805b4&#46;
- Current evidence: At 7a0fe85 native&#95;tb3&#95;artifact&#95;witness fails on the nop case: the pinned task declares only the agent&#45;created /app/results&#46;txt, the upstream nop agent never creates it, and the binding correctly rejects declared&#45;artifact&#45;missing while the test expects a bound reward&#45;zero result&#46;
- Cause / missed boundary: The d54ae48 witness revision pinned foodstuff&#45;beta&#45;activity and asserted collected&#46;len&#40;&#41;==1 for both cases without reconciling the nop expectation with the DeclaredArtifactMissing contract the same remediation enforced&#46;
- Suggested current verification: Stage an incorrect witness that emits a wrong&#45;content declared artifact &#40;or align the expectation with the invalid classification&#41; and rerun the exact ignored filter to green&#46;
- Candidate raw proof: /home/luiz/phase19&#45;acceptance&#45;r2/cache/corpora/terminal&#45;bench&#45;3&#46;0/tasks/foodstuff&#45;beta&#45;activity/task&#46;toml; SHA-256 5090e2cf2308f9d0e516c785c106831f56e43c9c42e942d984dc00df81a4db24; checked: Read the task declaration &#40;artifacts, separate verifier, timeouts&#41; during the executed witness failure analysis&#46;; limits: cache&#45;resident pinned task bytes

### Comparison 4: P19&#45;AUD&#45;104

- Relationship: missed&#45;branch
- Prior claim (attributed): Prior P19&#45;AUD&#45;003 &#40;Minor, test&#45;quality&#41;: the registered AC&#45;008 command selected zero tests because the only test named native&#95;provider&#95;failure&#95;report was not ignored while the intended witness was named native&#95;client&#95;provider&#95;failure&#95;report, making the recorded command vacuous&#46; That selection defect is repaired at audit&#95;head &#40;the test is ignored and executes under &#45;&#45;ignored &#45;&#45;exact&#41;; the old claim is closed by current observation, not adopted&#46;
- Current evidence: The same command now executes and fails behaviorally: the rate&#45;limit/pi case classifies correctly as invalid&#45;execution/agent&#45;provider&#45;failure:connection &#40;unscored&#41;, but the accepted&#45;reason enumeration omits that deliberate spelling, so the committed command is red rather than vacuous&#46;
- Cause / missed boundary: d54ae48 renamed and re&#45;shaped the witness &#40;removing the native&#95;client&#95;provider&#95;failure&#95;report split&#41; without absorbing the retry&#45;exhaustion reason prefix that b392912 had introduced and unit&#45;tested&#46;
- Suggested current verification: Add the agent&#45;provider&#45;failure prefix to the accepted reason list and rerun the exact ignored filter with admitted clients&#46;
- Prior finding: phase19&#45;pi&#45;glm53&#45;9d805b4&#45;20261004t094337z / P19&#45;AUD&#45;003; source prior&#45;findings
- Repair commits: d54ae484f535cc417209b921e5c4fab0019f0522
- Candidate raw proof: /home/luiz/phase19&#45;acceptance&#45;r2/operator&#45;record&#46;json; SHA-256 95afee6d45b3088f99ed0835188107ab0c835855bcfc29d80dea0491e0477ea3; checked: Read the raw record; used only batch entries, exits and capture&#45;refusal byte counts; independently re&#45;verified the referenced run bundles with the audit&#95;head binary&#46;; limits: used only for the raw fact that provider&#45;side terminal failures stayed unscored in the accepted z&#45;batch; the reason&#45;spelling mismatch is established from audit&#95;head source and executed test output

### Comparison 5: P19&#45;AUD&#45;105

- Relationship: missed&#45;branch
- Prior claim (attributed): Prior P19&#45;AUD&#45;002 &#40;Minor, test&#45;quality&#41;: remediation&#95;native&#95;stage&#95;deadlines failed at 9d805b4 because it required a &#39;stage&#45;witness&#45;start&#39; stderr marker that no producer emitted after 0fa995d&#46; That specific marker defect is repaired at audit&#95;head &#40;the positive cases now pass&#41;; the old claim is closed by current observation, not adopted&#46;
- Current evidence: The same test still fails, now through a different mechanism reproduced twice: the time&#45;cap case&#39;s captured stream exceeds the fixed 64 MiB STREAM&#95;LIMIT&#95;BYTES ceiling, both trials settle invalid with native&#45;capture&#45;refused, and the run exits 1 while the test expects two valid graded timeouts; the operator&#39;s real DeepSWE Opi attempts were refused by the same ceiling &#40;377 B and 185 B over&#41;&#46;
- Cause / missed boundary: Bounded inference: the deadline witness and the capture ceiling were tuned independently; high&#45;volume timeout streams cannot both stay under the ceiling and demonstrate graded&#45;timeout semantics&#46;
- Suggested current verification: Decide the intended over&#45;ceiling semantics &#40;bound provenance or truncate&#45;with&#45;proof preserving timeout grading&#41;, then rerun the exact ignored filter deterministically to green&#46;
- Prior finding: phase19&#45;pi&#45;glm53&#45;9d805b4&#45;20261004t094337z / P19&#45;AUD&#45;002; source prior&#45;findings
- Repair commits: d54ae484f535cc417209b921e5c4fab0019f0522
- Candidate raw proof: /home/luiz/phase19&#45;acceptance&#45;r2/operator&#45;record&#46;json; SHA-256 95afee6d45b3088f99ed0835188107ab0c835855bcfc29d80dea0491e0477ea3; checked: Read the raw record; used only batch entries, exits and capture&#45;refusal byte counts; independently re&#45;verified the referenced run bundles with the audit&#95;head binary&#46;; limits: corroborating live refusal byte counts only; the audit&#95;head reproductions ran in private temp roots and are summarized in the installed core&#39;s checks

### Comparison 6: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Prior P19&#45;AUD&#45;004 &#40;Info, residuals&#41;: the ignore annotation on native&#95;conformance&#95;owned&#95;cleanup&#95;witness referenced the superseded material/6 schema while the test consumes controlled material/7&#46;
- Current evidence: At 7a0fe85 the ignore annotations reference material/7 and the witness passes when executed with a current material &#40;native&#95;conformance&#95;owned&#95;cleanup&#95;witness passed in 110&#46;2s during this run&#41;; no current finding exists&#46;
- Cause / missed boundary: The annotation was refreshed when the material schema advanced to /7&#46;
- Suggested current verification: None required beyond keeping annotations aligned with the consumed material schema&#46;
- Prior finding: phase19&#45;pi&#45;glm53&#45;9d805b4&#45;20261004t094337z / P19&#45;AUD&#45;004; source prior&#45;findings
- Repair commits: d54ae484f535cc417209b921e5c4fab0019f0522
- Candidate raw proof: /home/luiz/phase19&#45;acceptance&#45;r2/materials&#45;r9/tb30/material&#46;json; SHA-256 02e7cc157ca7e7b80b27b4b42936c1a0dbafbdb517d5f08cb95292fa9e53a3e9; checked: Consumed as the witness input for the executed native&#95;conformance&#95;owned&#95;cleanup&#95;witness and native&#95;tb3&#95;artifact&#95;witness runs at audit&#95;head&#46;; limits: witness input consumed during execution
