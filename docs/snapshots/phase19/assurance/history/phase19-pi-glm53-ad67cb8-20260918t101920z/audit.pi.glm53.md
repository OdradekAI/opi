# Phase 19 Audit

**Audit run ID**: `phase19-pi-glm53-ad67cb8-20260918t101920z`
**Audit head**: `ad67cb8e1d9e824f270b549e4fdea0072cd7b539`
**Reviewer ID**: `pi`
**Model ID**: `glm53`
**Reviewer identity**: pi (pi-coding-agent terminal coding agent)
**Reviewer model ID**: `glm53`
**Model identity source**: operator-declared
**Independence**: fresh-context-same-family - fresh audit context begun from rotation admission; a prior pi/glm53 member exists in the live set and its content, history, and the sibling codex.gpt6 member were never read or searched
**Baseline policy**: latest-committed-spec
**Verdict**: FAIL

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| .opi-impl-state.json | d9b84dd2d85d64f0fa7b3462dbfd00f6f0a464b02393684e70840a9ecdc642a0 | current committed source; spec_files_sha256 entries all match committed bytes |
| docs/snapshots/phase19/opi-impl-state.json | 35b3378bd902cc976bb348d794dd1b5d3c5f160a58769c3aeb62fea175a7b7a9 | current committed source; 11 tasks recorded passing |
| docs/opi-spec.md | 1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db | current committed source; parent clauses CTRL-004..007, PRIN/PLACE/GATE mapping read in full |
| docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md | 5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a | current committed source; English document owns Phase meaning |
| docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.zh.md | d41c826412b0d71802327d60f953708f6cb2083436a2a2b0d99a2e52d851f194 | current committed source; identifiers P19-AC-001..013 and P19-D01..D08 verified in sync |

## Requirement Conformance

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| P19-AC-001 | Independent local directory operation | independent-directory prepare, fixture-free frozen plan, pi-only preparation tests pass; operator workspace outside the checkout | met | |
| P19-AC-002 | Native result identity binding | import_bound_native_job rejects oracle/wrong-agent/wrong-task/substituted/no-terminal/no-job/two-jobs with typed errors and no seal; all substitution tests pass | met | |
| P19-AC-003 | TB2.1 native lifecycle grading | six real trials graded by the pinned Harbor grader and verifier-accepted; real-grader correct/incorrect witness not durably evidenced | partially-met | P19-AUD-002 |
| P19-AC-004 | TB3 declared collection / separate verifier | real traversal by both agents plus deterministic declared/substituted rejections pass; real-grader witness not durably evidenced | partially-met | P19-AUD-002 |
| P19-AC-005 | DeepSWE committed patch in pristine verifier | real patch binding plus deterministic base-drift/undeclared/nop-zero tests pass; real-grader witness not durably evidenced | partially-met | P19-AUD-002 |
| P19-AC-006 | Provider projection per Agent | three-API enum with exactly-one credential reference; client matrices (ledger-recorded local-material run), missing-credential rejection, and six real zhipu glm-5.3-flash trials with sealed output-control witness | met | |
| P19-AC-007 | Resource preflight and explicit scope | full-inventory preflight, unmet-resource rejection without silent shrink, frozen subset, and plan contradiction tests pass | met | |
| P19-AC-008 | Native-metric-honest reporting | report contract, end-to-end, containment, pairing suites (28 tests) and native metric-meaning/test-count tests pass | met | |
| P19-AC-009 | Distinct offline verify/recompute/regrade | verify/report/regrade are separate commands; bundle recompute, offline regrade family, and every fail-closed regrade rejection pass with zero Agent/provider invocations | met | |
| P19-AC-010 | Credentials, supervision, cancellation, recovery | credential settlement removal, ambient-secret-free regrade child, bounded cancellation, retained failed attempts with fresh replacement identities; verifier-accepted | met | |
| P19-AC-011 | Deterministic credential-free defaults, explicit acceptance | 216 python tests OK and full default suite credential-free with identical results; real acceptance independently identified in docs/audits | met | |
| P19-AC-012 | Six real trials with native grading | offline verifier re-run at audit_head: accepted:true exit 0, six attributable trials across three families matching the committed acceptance audit | met | |
| P19-AC-013 | Independent placement, recorded 0.x changes | no internal crate dependencies either direction, publish=false, pi-only tests pass, bilingual READMEs, comprehensive Unreleased entries, doc-check PASS | met | |
| P19-EXIT-GATES | Repository verification union | doc-check PASS, fmt clean, clippy(-p) clean, but cargo test -p opi-eval fails on a faithful checkout (2 exec-bit failures); workspace-wide test/doc-test/rustdoc not run (fail-dominant crate failure already establishes the union broken) | not-met | P19-AUD-001 |

## Standards Review

Workspace hygiene at the sealed head is strong: clean worktree, edition-2024 manifests, no internal dependency in either direction for opi-eval, publish=false, and error typing via thiserror throughout the inspected modules. Repository standards are violated in one durable way recorded as P19-AUD-001: executable content committed without the executable bit (one regression at c8d6dea on a previously-correct file, one since introduction) is invisible on the development mount, which silently downgraded every recorded verification run that included those two tests.

## Spec Review

The 13 acceptance criteria are implemented with unusual care on the binding side: the native result reader enforces agent/task/model/config-selection identity, oracle exclusion through the eval key, stale-job defense, declared-artifact manifest equality, and delivered-artifact byte cross-checks, each as typed rejections with no seal. The reporting, offline-operations, and supervision criteria are covered by discriminating tests that pass at the sealed head. Two spec-level gaps remain: the verification-union claim does not survive a faithful checkout (P19-AC/EXIT-GATES), and the correct/incorrect artifact-witness routes for the three benchmark families have no durable execution evidence, so the 13/13 exit closure overstates AC-003/004/005 (P19-AUD-002).

## Security, Invariants, Integration, Test Quality, and Residuals

Security/authority: credential references are exactly-one and fail-closed; settlement removes actual credentials; regrade children run without ambient secrets; canary gating blocks sealing; grader inputs are restricted to declared artifacts. Invariants/integration: typed lifecycle states distinguish valid zero, invalid-execution, cancelled, and effect-unknown; verifier timeout stays Unknown without manufacturing a score; run roots admit one owner. Test quality: assertions observe behavior (typed failure kinds, bundle absence, byte digests), and the acceptance verifier was independently re-executed during this audit. Residuals: nine ignored tests exist; the ledger records explicit local-material runs for the client matrix and the 19.9 Docker witness, and the three family witnesses plus stage-deadline/verifier-only-regrade filters are exactly the unevidenced set named in P19-AUD-002. The acceptance audit honestly discloses a manifest exit-code correction (attempt-3 stale exits corrected to the final batch, verifier re-accepted).

## Minimum-change Conformance

| Task | Recorded reuse/surface/ceiling vs current code | Status |
|---|---|---|
| 19.1 | import_bound_native_job, NativeTrialBinding, RunBundle seal, oracle-as-diagnostic all present as recorded | conforming |
| 19.2 | named connections, three-API enum, exactly-one credential reference; fixed scripted/OpenAI-only projection gone from production config (remnants only in #[cfg(test)]) | conforming |
| 19.3 | terminal_bench_30.rs + task_lifecycle.rs declared collect/separate verifier as recorded | conforming |
| 19.4 | deepswe.rs committed-patch binding and Pier base identity as recorded | conforming |
| 19.5 | surfaces match, but its scoped-smoke PASS record includes supervised_acquisition_runs_the_pinned_materializer, which fails on faithful checkouts (P19-AUD-001) | drifted |
| 19.6 | frozen plan, subset selection, resource checks as recorded | conforming |
| 19.7 | verify/report split and regrade handoff to 19.8 as recorded | conforming |
| 19.8 | NativeRegrader with fail-closed rejections as recorded | conforming |
| 19.9 | supervisor/lifecycle ownership, recorded explicit Docker-witness run consistent with the ignored filter | conforming |
| 19.10 | thin workflows, offline acceptance verifier, deterministic CI as recorded | conforming |
| 19.11 | offline verifier acceptance reproduces (accepted:true), but the recorded smoke-full/workspace-test PASS does not reproduce on a faithful checkout (P19-AUD-001) | drifted |

## Findings

### P19-AUD-001: Non-executable script modes break the default test suite on faithful checkouts

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-EXIT-GATES
- Claim: Two scripts that production paths execute directly are committed with git mode 100644, so `cargo test -p opi-eval --all-targets` (and therefore the workspace union and the CI test job) fails on any checkout whose filesystem honors git modes; recorded phase-exit gate PASSes were masked by the all-executable DrvFs working tree.
- Evidence: scripted-provider.py lost 100755 at c8d6dea (R095 rename) and is direct-executed by the conformance helper (src/cli/conformance.rs:338), producing provider-fixture-response-drift with met:false; tests/fixtures/external-locks/local-cache/scripts/materialize-external-locks.sh is 100644 since introduction and is direct-executed by runner::prepare (src/runner/prepare.rs:930), failing supervised acquisition with os error 13; both failures reproduced twice in the sealed ext4 export plus a direct CLI reproduction; /mnt/d DrvFs shows both files rwxrwxrwx regardless of git mode.
- Refutation attempted: no chmod step exists in any workflow; actions/checkout preserves 644; both call sites are direct exec, not interpreter-invoked; `python3 <script>` produces the exact expected response bytes, isolating mode as the sole cause; CI ci.yml runs the affected tests on ubuntu-latest and macos-latest where they cannot pass in this state.
- Suggested closure: restore the executable bit on both files (git update-index --chmod=+x) or route both invocations through the interpreter, then re-run the workspace test union on a mode-faithful filesystem and record the result.

### P19-AUD-002: No durable evidence that per-family correct/incorrect artifact witnesses ran through the real native graders

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-003, P19-AC-004, P19-AC-005
- Claim: The registered mechanical verification routes for P19-AC-003/004/005 require a controlled correct/incorrect artifact witness through the same production handoff and the real pinned native grader; no executed witness run is recorded anywhere in the committed handoff, the operator evidence root, or CI dispatch records, so the 13/13 phase-exit closure overstates these criteria.
- Evidence: native_tb21_artifact_witness / native_tb3_artifact_witness / native_deepswe_patch_witness are ignored filters with no recorded execution; the operator evidence-root manifest contains zero witness/conformance/oracle entries; the CI native-smoke workflow contains the witness steps but is manually dispatched with no committed dispatch or receipt record; the six real trials grade tb21 1/1 and tb30/deepswe 0/0, giving no within-family correct/incorrect contrast; ledger tasks 19.1/19.3/19.4 themselves recorded these witnesses as open pending pinned material, and 19.11 closed the criteria via the six trials alone.
- Refutation attempted: searched the phase19 ledger, both acceptance audit documents, the operator workspace (runs, reports, material, output-control, evidence root), and all committed smoke fixtures and workflows for witness-execution records; deterministic stand-in-verifier witnesses pass and prove the binding path, but they do not exercise the pinned real graders, and grader-sensitivity to artifact variation therefore remains unproven by the real graders.
- Suggested closure: run the three family witness filters with the pinned material (or dispatch the native-smoke workflow) and commit the redacted witness records, then re-derive AC-003/004/005 closure from that evidence.

## Verification Commands

| Command | Result | Requirement/finding |
|---|---|---|
| python .agents/skills/_shared/scripts/validate_assurance_artifact.py rotation docs/snapshots/phase19 | PASS | rotation admission |
| git archive ad67cb8 to ext4 export; committed-source hash comparison against .opi-impl-state.json | match | baseline seal |
| python3 scripts/opi-doc-check.py (export) | PASS | P19-AC-013, P19-EXIT-GATES |
| cargo test -p opi-eval --all-targets --no-fail-fast (export) | FAIL: 307 passed, 2 failed | P19-EXIT-GATES, P19-AUD-001 |
| cargo clippy -p opi-eval --all-targets -- -D warnings (export) | PASS | P19-EXIT-GATES |
| cargo fmt --check --all (export) | PASS | P19-EXIT-GATES |
| python3 -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py' (export) | OK (216, 1 skip) | P19-AC-011 |
| python3 crates/opi-eval/scripts/verify-local-acceptance.py --root <operator evidence root> --companion <operator bin> (from export) | accepted:true, exit 0 | P19-AC-012 |
| opi-eval conformance --suite agent --adapter opi --case provider-fixture (export binary, direct) | exit 1, provider-fixture-response-drift | P19-AUD-001 reproduction |
| git ls-files -s on both scripts | 100644 both | P19-AUD-001 |
| cargo test --workspace / --doc / cargo doc --workspace | NOT RUN (fail-dominant crate failure already establishes the union broken) | P19-EXIT-GATES limitation |
| git diff --check | NOT RUN (clean worktree at seal; vacuous) | P19-EXIT-GATES limitation |

## Verdict Rationale

Two mandatory requirements are not fully met at the sealed head: P19-EXIT-GATES is not-met because the default opi-eval test suite fails on any mode-faithful checkout (P19-AUD-001, Major, blocking), and P19-AC-003/004/005 are partially-met because their registered witness routes lack durable real-grader execution evidence (P19-AUD-002, Major, blocking). The remaining eleven criteria are met with discriminating, credential-free evidence, including an independently re-executed offline acceptance verification of the six real trials. The member verdict is therefore FAIL.
