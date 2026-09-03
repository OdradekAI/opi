# Phase 18 Audit

**Audit run ID**: `phase18-codex-gpt56-a8bb454-20260903t021838z`
**Audit head**: `a8bb45426daf960d9e60024ce34542995c4dd2d1`
**Reviewer ID**: `codex`
**Model ID**: `gpt56`
**Reviewer identity**: Codex
**Reviewer model ID**: `gpt56`
**Model identity source**: operator-declared
**Independence**: fresh-context-same-family; this run was sealed before production inspection and did not consume prior assurance conclusions
**Baseline policy**: latest-committed-spec
**Verdict**: FAIL

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| `.opi-impl-state.json` | `9d2ecf977f940f03db3c5d3b17437ad4a3afbca6ad409fcebf306727848a358e` | current committed implementation ledger; registered hashes matched |
| `docs/snapshots/phase18/opi-impl-state.json` | `cea5031074ac0d5667357863fbdf03bc76494295a6c38fac304dc1c851d7b42c` | pointed sealed Phase state |
| `docs/opi-spec.md` | `cc7f8898f60c0d8abaa667f4b49b7affc721412e75dd3a67dcde37a783e1bc4c` | current committed normative specification |
| `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md` | `43b2759d327cbf0af8d35d4eba50839eef7aac473978b58fcb707b335dad8265` | currently registered Phase 18 supplemental source |

The audit sealed 39 mandatory requirements before production inspection. The stored registered-source hashes matched the current committed sources.

## Requirement Conformance

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| P18-OUT-001 | outcome | crates/opi-eval/src/agent/mod.rs; crates/opi-eval/src/agent/opi.rs | partially-met | P18-AUD-002 |
| P18-OUT-002 | outcome | crates/opi-eval/src/benchmark/mod.rs; crates/opi-eval/src/benchmark/process.rs | partially-met | P18-AUD-002 |
| P18-OUT-003 | outcome | crates/opi-eval/src/bundle/mod.rs; crates/opi-eval/src/runner/material.rs | met | — |
| P18-OUT-004 | outcome | crates/opi-eval/src/report.rs; crates/opi-eval/src/regrade.rs | met | — |
| P18-OUT-005 | outcome | crates/opi-eval/src/experiment.rs; crates/opi-eval/docs/seam-evidence-matrix.md | not-met | P18-AUD-001 |
| P18-OUT-006 | outcome | crates/opi-eval/Cargo.toml; Cargo.toml | met | — |
| P18-A01 | acceptance-scenarios-and-verification | crates/opi-eval/Cargo.toml; Cargo.toml | met | — |
| P18-A02 | acceptance-scenarios-and-verification | crates/opi-eval/src/agent/mod.rs; crates/opi-eval/src/agent/opi.rs | not-assessable | P18-AUD-002 |
| P18-A03 | acceptance-scenarios-and-verification | crates/opi-eval/src/agent/mod.rs; crates/opi-eval/src/agent/opi.rs | not-assessable | P18-AUD-002 |
| P18-A04 | acceptance-scenarios-and-verification | crates/opi-eval/src/agent/mod.rs; crates/opi-eval/src/agent/opi.rs | not-assessable | P18-AUD-002 |
| P18-A05 | acceptance-scenarios-and-verification | crates/opi-eval/src/agent/process.rs; crates/opi-eval/src/process.rs | met | — |
| P18-A06 | acceptance-scenarios-and-verification | crates/opi-eval/src/agent/process.rs; crates/opi-eval/src/process.rs | met | — |
| P18-A07 | acceptance-scenarios-and-verification | crates/opi-eval/src/runner/lifecycle.rs; crates/opi-eval/src/runner/experiment.rs | met | — |
| P18-A08 | acceptance-scenarios-and-verification | crates/opi-eval/src/benchmark/mod.rs; crates/opi-eval/src/benchmark/process.rs | not-assessable | P18-AUD-002 |
| P18-A09 | acceptance-scenarios-and-verification | crates/opi-eval/src/benchmark/mod.rs; crates/opi-eval/src/benchmark/process.rs | not-assessable | P18-AUD-002 |
| P18-A10 | acceptance-scenarios-and-verification | crates/opi-eval/src/benchmark/mod.rs; crates/opi-eval/src/benchmark/process.rs | not-assessable | P18-AUD-002 |
| P18-A11 | acceptance-scenarios-and-verification | crates/opi-eval/src/benchmark/mod.rs; crates/opi-eval/src/benchmark/process.rs | met | — |
| P18-A12 | acceptance-scenarios-and-verification | crates/opi-eval/src/experiment.rs; crates/opi-eval/src/comparison.rs | partially-met | P18-AUD-002 |
| P18-A13 | acceptance-scenarios-and-verification | crates/opi-eval/src/comparison.rs; crates/opi-eval/src/integrity.rs | met | — |
| P18-A14 | acceptance-scenarios-and-verification | crates/opi-eval/src/comparison.rs; crates/opi-eval/src/integrity.rs | met | — |
| P18-A15 | acceptance-scenarios-and-verification | crates/opi-eval/src/bundle/mod.rs; crates/opi-eval/src/runner/material.rs | met | — |
| P18-A16 | acceptance-scenarios-and-verification | crates/opi-eval/src/report.rs; crates/opi-eval/src/regrade.rs | met | — |
| P18-A17 | acceptance-scenarios-and-verification | crates/opi-eval/src/report.rs; crates/opi-eval/src/regrade.rs | met | — |
| P18-A18 | acceptance-scenarios-and-verification | crates/opi-eval/src/bundle/mod.rs; crates/opi-eval/src/report.rs | met | — |
| P18-A19 | acceptance-scenarios-and-verification | crates/opi-eval/Cargo.toml; Cargo.toml | met | — |
| P18-A20 | acceptance-scenarios-and-verification | crates/opi-eval/src/experiment.rs; crates/opi-eval/docs/seam-evidence-matrix.md | partially-met | P18-AUD-001 |
| P18-A21 | acceptance-scenarios-and-verification | docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| P18-A22 | acceptance-scenarios-and-verification | .github/workflows/ci.yml; .github/workflows/opi-eval-native-smoke.yml | partially-met | P18-AUD-002 |
| P18-RBK-001 | risk-thresholds-and-rollback | crates/opi-eval/Cargo.toml; Cargo.toml | met | — |
| P18-RBK-002 | risk-thresholds-and-rollback | crates/opi-eval/Cargo.toml; Cargo.toml | met | — |
| P18-RBK-003 | risk-thresholds-and-rollback | crates/opi-eval/Cargo.toml; Cargo.toml | met | — |
| P18-RBK-004 | risk-thresholds-and-rollback | crates/opi-eval/Cargo.toml; Cargo.toml | met | — |
| P18-RBK-005 | risk-thresholds-and-rollback | crates/opi-eval/Cargo.toml; Cargo.toml | met | — |
| P18-RDM-001 | post-phase-eval-coverage-roadmap | docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| P18-RDM-002 | post-phase-eval-coverage-roadmap | crates/opi-eval/src/experiment.rs; crates/opi-eval/src/agent/mod.rs | met | — |
| P18-RDM-003 | post-phase-eval-coverage-roadmap | docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| P18-RDM-004 | post-phase-eval-coverage-roadmap | docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| P18-RDM-005 | post-phase-eval-coverage-roadmap | docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| P18-RDM-006 | post-phase-eval-coverage-roadmap | docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |

## Standards Review

The current Companion remains unpublished, crate-private at its adapter and process seams, and dependency-neutral. `cargo tree` proves no inward Opi dependency and no reverse product dependency. The implementation uses typed states and errors, bounded process supervision, content-addressed bundles, and fail-closed validation. The only unsafe boundary is the platform process-tree implementation and is narrowly scoped under the crate-level unsafe-code denial. No standards-axis finding was identified.

## Spec Review

The generic experiment resolver, N-subject/edge shapes, Opi/pi importers, three benchmark adapters, immutable bundle machinery, integrity/comparison projection, and offline regrade/report paths remain present and pass their current local tests. The required artifact-derived minimum-seam result does not: `crates/opi-eval/docs/seam-evidence-matrix.md` and its current derivation/acceptance guard are absent. This is P18-AUD-001.

## Security, Invariants, Integration, Test Quality, and Residuals

Security and invariants: bundle recomputation, path/digest checks, output containment, typed unknowns, authority transitions, and failure preservation pass current tests. No raw-secret or machine-path exposure was observed in the inspected export.

Integration and test quality: all current local gates pass, including 109 opi-eval tests and the complete workspace suite. Four material opi-eval integration binaries are Unix-only and therefore execute zero tests on this Windows host. Historical native run 33271354427 and the 27-job terminal receipt remain successful and unexpired, but their heads precede 120/121 scoped file changes respectively. GitHub reports no run for the audit head. Therefore the required current real-process/native proof is unavailable (P18-AUD-002). The downloaded 528-byte native upload receipt was inspected; the 6.1 GB sealed artifact was not downloaded or revalidated during this run.

Residuals: the current CLI continues to label the crate unpublished/provisional and the roadmap keeps unsupported integrations not admitted. No advisory residual finding was added because the two evidence gaps are already blocking.

## Minimum-change Conformance

Each Phase task was traced to its current consumer/call site and checked for reuse, placement, necessity, and simplification ceiling. Current code generally reuses one crate-private process, bundle, comparison, and report path; no duplicate public seam was found.

| Task | Observable current consumer | Reuse / placement / necessity | Status |
|---|---|---|---|
| 18.1 | CLI/experiment resolver | `cli`, `experiment`, generic fixture | conforming |
| 18.2 | external-lock verifier | `external_lock`, crate-local materializer/verifier scripts | conforming |
| 18.3 | resolved Linux lock | resolved/static lock files and artifact verifier tests | conforming |
| 18.4 | Agent/benchmark subprocess callers | one crate-private `process`/`process::tree` seam | conforming |
| 18.5 | runner settlement/sealing | `bundle`, `runner::lifecycle`, `failure` | conforming |
| 18.5.1 | pair/report projection | `integrity` and `comparison`; no duplicate scoring seam | conforming |
| 18.6 | Opi adapter | current importer is exercised by fixtures, but current real Opi proof is absent | not-assessable: P18-AUD-002 |
| 18.7 | pi adapter | current importer is exercised by fixtures, but current real pi proof is absent | not-assessable: P18-AUD-002 |
| 18.8 | Terminal-Bench 2.1 adapter | single benchmark contract/profile; native proof is stale | not-assessable: P18-AUD-002 |
| 18.9 | Terminal-Bench 3.0 adapter | separate profile over shared contract; native proof is stale | not-assessable: P18-AUD-002 |
| 18.10 | DeepSWE v1.1 adapter | separate native adapter over shared contract; native proof is stale | not-assessable: P18-AUD-002 |
| 18.10.1 | conformance CLI | current `cli::conformance` plus Agent/benchmark conformance tests | conforming |
| 18.11 | bundle/report projection | single provisional `trajectory` module | conforming |
| 18.12 | `cli::run` | one runner with shared authority/lifecycle/material seams | partially conforming: P18-AUD-002 |
| 18.13 | regrade/report CLI | offline `regrade`, `report`, `comparison` paths | conforming |
| 18.14 | native-smoke workflow | one current workflow/script producer; no current-head receipt | partially conforming: P18-AUD-002 |
| 18.14.1 | runner/benchmark driver | current driver and verifier contract tests | conforming locally |
| 18.15 | native artifact | historical successful artifact remains, but is not bound to current bytes | not-assessable: P18-AUD-002 |
| 18.16 | seam review/CI contract | generic schema remains; required matrix and derivation guard are absent | nonconforming: P18-AUD-001 |
| 18.16.1 | three-platform receipt | unexpired historical receipt; 121 scoped files changed afterward | not-assessable: P18-AUD-002 |

## Findings

### P18-AUD-001: The current conformance result has no inspectable seam-evidence matrix

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P18-OUT-005, P18-A20
- Claim: the audit head lacks the result needed to identify what the real cross-agent/native evidence actually proved.
- Evidence: the normative rows at lines 178 and 992 require the matrix; Test-Path and git-ls-tree prove it absent; the sealed Phase record at lines 3887, 3924, and 4269-4271 identifies the deleted path and historical derivation run.
- Refutation attempted: the historical Phase record and successful native run were checked. They show that a matrix once existed, but neither retains an inspectable current matrix or its current derivation guard, so they do not refute the current-tree gap.
- Suggested closure: restore or regenerate a current artifact-derived matrix, retain its derivation verifier and acceptance guard, and prove that only the evidence-supported seam is settled while the rest remains provisional.

### P18-AUD-002: Real-process and native-verifier evidence does not bind the audit head

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P18-OUT-001, P18-OUT-002, P18-A02, P18-A03, P18-A04, P18-A08, P18-A09, P18-A10, P18-A12, P18-A22
- Claim: current source and fixture tests cannot establish the required real Opi/pi and native benchmark outcomes after the retained receipts' evaluated bytes changed broadly.
- Evidence: native run 33271354427 is successful at `27344e3a...`, the terminal receipt is successful at `3b4a39d9...`, 120/121 scoped files changed afterward, and exact-head run lookup returns `[]`.
- Refutation attempted: current unit/integration tests, all verifier-script tests, the unexpired terminal receipt, native run metadata, and upload-receipt digest were checked. They establish local contract quality and historical execution, but not exact-head real-process/native behavior; four current Unix-only binaries also run zero tests on this host.
- Suggested closure: run the sole pinned Linux native-smoke producer and the required three-platform gate against a commit containing the current Eval/workflow bytes, validate/download the resulting receipts and artifact as required, then bind them to the audited head or a reviewed descendant.

## Verification Commands

| Command | Result | Requirement/finding |
|---|---|---|
| `python .agents/skills/_shared/scripts/validate_assurance_artifact.py rotation docs/snapshots/phase18` | PASS | independent admission |
| `python scripts/opi-cargo-cache.py status` | PASS | cache contract |
| `cargo test -p opi-eval --all-targets` | PASS: 109 executed, 0 failed | focused Companion surface |
| five `crates/opi-eval/scripts/test_*.py` verifier suites | PASS: 132 tests | external-lock/native producer validation |
| `cargo tree -p opi-eval --edges normal` | PASS: no opi-* dependency | P18-A01 |
| `cargo tree --workspace --invert opi-eval` | PASS: no reverse dependency | P18-A01/P18-A19 |
| `python3 scripts/opi-doc-check.py` in WSL over the same export | PASS | documentation and roadmap contracts |
| `cargo fmt --check --all` | PASS | workspace format |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | workspace lint |
| `cargo test --workspace --all-targets` | PASS | workspace runtime gates |
| `cargo test --workspace --doc` | PASS | doctests |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | PASS | rustdoc |
| generic three-subject/fourth-benchmark `opi-eval validate` | PASS | P18-A20 generic half |
| `Test-Path .../seam-evidence-matrix.md` | FAIL: False | P18-AUD-001 |
| exact-audit-head `gh run list` | NOT ASSESSABLE: `[]` | P18-AUD-002 |

The first Windows documentation-check attempt failed because `git archive` could not materialize the `.claude/skills` symlink under the extracted path; WSL verified the same sealed export successfully. The first workspace-test attempt similarly exposed the archive's missing Git object database in one test that validates a hard-coded real commit; after attaching a read-only object database, that exact test and the full unchanged suite passed.

## Verdict Rationale

The verdict is mechanically **FAIL** because mandatory requirements have `not-met`, `partially-met`, or `not-assessable` states linked reciprocally to two Major blocking findings. Local code quality and workspace gates are green, but neither an absent required conformance result nor stale native/CI receipts can be treated as current proof.
