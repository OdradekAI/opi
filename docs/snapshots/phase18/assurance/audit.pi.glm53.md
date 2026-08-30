# Phase 18 Audit

**Audit run ID**: `phase18-pi-glm53-432ff13-20260830t114647z`
**Audit head**: `432ff13b0bb05bb8cc8efcfdb04a06d3e9e87dbb`
**Reviewer ID**: `pi`
**Model ID**: `glm53`
**Reviewer identity**: Pi
**Reviewer model ID**: `glm53`
**Model identity source**: operator-declared
**Independence**: fresh-context-same-family — fresh audit context; no prior Phase 18 assurance-set audit, remediation conclusion, history run, or sibling peer output was loaded; requirements were sealed from the registered sources before production inspection
**Baseline policy**: latest-committed-spec
**Verdict**: FAIL

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| `.opi-impl-state.json` | `9d2ecf977f940f03db3c5d3b17437ad4a3afbca6ad409fcebf306727848a358e` | current committed root state (current_phase 19; Phase 18 exit recorded) |
| `docs/snapshots/phase18/opi-impl-state.json` | `cea5031074ac0d5667357863fbdf03bc76494295a6c38fac304dc1c851d7b42c` | pointed Phase state (20 tasks, all passing) |
| `docs/opi-spec.md` | `cc7f8898f60c0d8abaa667f4b49b7affc721412e75dd3a67dcde37a783e1bc4c` | latest committed normative spec (stored hash matched) |
| `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md` | `43b2759d327cbf0af8d35d4eba50839eef7aac473978b58fcb707b335dad8265` | registered supplemental source (stored hash matched) |

Both ledger-stored `spec_files_sha256` values matched the committed bytes at
`audit_head`; no mismatch metadata evidence arose. The implementation was
inspected only inside a `git archive` export of `audit_head` (plus one
detached, byte-verified clone of the same commit used solely to exercise the
git-history-dependent A19 replay, since a `git archive` export carries no
`.git`).

## Requirement Conformance

131 requirements were sealed before implementation inspection: 109 `P18-*`
clauses and 22 `P18-A*` acceptance scenarios of the registered supplemental
source, all mandatory. States: **129 met, 2 partially-met** (`P18-FAL-003`,
`P18-RBK-001`). The two partially-met mandatory requirements both trace to
finding `P18-AUD-001`; per the mechanical rule the member verdict is **FAIL**.

Representative evidence (full per-requirement evidence in the sidecar):

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| P18-FAL-003 | Agent crashes/timeouts remain Agent outcomes; infra/grader/invalid never scored as Agent outcomes | infra/grader/invalid exclusion implemented and tested; but `runner/experiment.rs:1422-1432` maps every `AgentCompletion::Failed` to `TrialOutcome::InfrastructureFailure` and `comparison.rs` has no Agent-failure variant; `p18_a05`/`p18_a06` pin `infrastructure-failure:` coverage for agent-side failures incl. boundary `agent-process` | partially-met | P18-AUD-001 |
| P18-RBK-001 | every listed risk threshold blocks exit | threshold-by-threshold review: all absent except "one valid Agent failure reclassified", observed at the pairing/coverage layer (P18-AUD-001) | partially-met | P18-AUD-001 |
| P18-PLC-001/002, P18-A01 | Opi-free Companion; no reverse dependency | `cargo tree` forward: zero `opi-*` edges; invert: no dependents; not in `[workspace.dependencies]` | met | |
| P18-OUT-006, P18-A19, P18-MIG-004 | Minimal Runtime unchanged | `p18_a19` replay of the 13-command `1ad534b` baseline green (276.6 s) in the detached sealed clone; in the plain export the same test cannot run because the baseline verifier needs git history (environmental, recorded) | met | |
| P18-OUT-002, P18-A02–A04, A08–A10, A12, A22 | native three-revision artifact proof | accepted through the committed digest chain: ci-receipt workflow-bytes digest matches `audit_head` `ci.yml` (`4c0f3fcb…`), artifact-derived seam matrix `--verify` green binding run `33271354427`/digest `12892746…`/6 trials, artifact-verifier suite green; the 6.13 GiB artifact itself was not re-downloaded (limitation recorded) | met | |
| P18-INT-001 | immutable integrity admission | records, reclassification identity, and exclusion traceability implemented and tested; DeepSWE oracle preflight bar is structural only (advisory) | met | P18-AUD-003 |
| P18-MIG-003, P18-TRJ-001 | native/normalized/derived distinguishability; projection provenance | roles enforced and tested; verifier-native artifacts carry `agent-<product>` source identity (advisory) | met | P18-AUD-002 |
| P18-AGT-002 | shared conformance suite | hermetic suites green; native rerun of 13 cases per artifact chain; receipt records `cases_run: 12` (advisory) | met | P18-AUD-004 |

## Standards Review

Workspace topology conforms: `opi-eval` is a `publish = false` workspace
member with workspace-external dependencies only, absent from
`[workspace.dependencies]`; no Opi crate links it and no product activates it.
The crate's public surface is exactly the recorded provisional entry seam
(`cli`, `experiment::ResolvedExperiment`); all other modules are crate-private
behind `#[allow(dead_code)]`. `#![deny(unsafe_code)]` holds crate-wide with the
single documented FFI home (`process::tree`) carrying scoped allows. No
TODO/FIXME/unimplemented markers exist in the Phase 18 production or script
surfaces. Scoped clippy (`-D warnings`), fmt, and rustdoc are clean. The
documentation topology (AGENTS.md, both README variants, both crate README
variants, CONTEXT.md without provisional eval terms) is in lockstep and
`opi-doc-check.py` passes, including the 16-entry GLM-5.3 roadmap contract.
`CHANGELOG.md` `[Unreleased]` records only the 18.16 assurance additions; the
Companion crate itself, being unpublished and behavior-neutral for the `opi`
binary, has no user-facing entry — noted as an observation, not a finding.

## Spec Review

The delivered prototype implements the registered outcome shape: N-harness
experiment resolution with directed edges (three-subject/fourth-benchmark
fixture resolves), fail-closed Opi/pi process adapters with authoritative
completion predicates, three benchmark-revision adapters over pinned official
packages with harbor/pier native-result authorities, durable intent and
effect-unknown recovery, content-addressed sealed bundles with mutation
rejection, authority-transition gating with sealed call-count evidence, offline
regrade/report with byte stability and conformance-only labeling, and the
artifact-derived seam matrix. Parent-clause obligations (CTRL-004..007,
GOAL-004, PLACE-002, PRIN-004/005) are honored through these
operationalizations, with one exception detailed in P18-AUD-001: the failure
classification at the pairing/coverage layer violates the FAL-003 agent-outcome
clause and the corresponding RBK-001 exit threshold. The spec-internal tension
with FAL-002 (no conversion of a boundary failure into zero; transitions stop)
explains refusing grade dispatch after Agent-process failures, but it does not
license labeling the Agent's own failure as infrastructure in coverage; a
distinct Agent-failure classification (non-comparable or scored-failed, but
attributable to the Agent) was the registerable behavior. No ADR, spec
revision, or registered deferral legitimizes the shipped mapping.

## Security, Invariants, Integration, Test Quality, and Residuals

- **Security/authority**: external execution locks are admitted by digest
  (static and resolved, deny-unknown-fields); spawns use structured argv/env
  with closed projections; the scripted provider is stdlib-only with no
  credential surface (suite-enforced); canary gates block sealing and
  publication; no ambient PATH executable resolution; workflows are
  manual-dispatch with pinned actions and candidate-byte binding.
- **Invariants/integration**: lifecycle ladder is forward-only with durable
  intent-before-effect; sealing is atomic and content-addressed; verification
  never repairs; identity reuse is refused; replacements take fresh group
  identities; the three-platform CI receipt is digest-bound to the committed
  workflow bytes (independently re-hashed in this audit).
- **Test quality**: suites assert discriminating behavior at production seams
  (real `opi-eval` binary end to end, real bundled adapters, typed failure
  tokens, call-count authority proofs). Two advisory gaps: the DeepSWE oracle
  bar (P18-AUD-003) and the miscounted native conformance receipt
  (P18-AUD-004).
- **Residuals**: verifier-native artifacts staged under `agent-<product>`
  source identity (P18-AUD-002); ledger-only naming drift
  (`agent::execution`/`benchmark::execution` interface records vs shipped
  `agent/process.rs`/`benchmark/process.rs` modules) flagged four times
  in-session and never reconciled; `--fixtures` remains a required-but-unused
  argument in native run mode; per-revision CTRF/profile duplication persists
  inside the recorded simplification ceiling (revisit trigger not yet fired).
  None of these alter behavior incorrectly.

## Minimum-change Conformance

All 20 admitted tasks were checked against their recorded `reuse_search`,
`surface_necessity`, and `simplification_ceiling` at `audit_head`:
**conforming** for every task. The introduced public seam (`opi_eval::cli`,
`opi_eval::experiment`) has production consumers (the `opi-eval` binary and
same-package integration tests), no non-production consumers outside tests, no
net deletion (new crate), and residual glue limited to the `--fixtures` native-
mode wart noted above. Recorded deviations were disclosed in the ledger at the
time (18.12's late C.1a glob append; 18.15's forty-seven interim producer
repair commits tracked by task footers; 18.16.1's absorbed pre-existing CI
debt) and do not contradict their recorded decisions.

## Findings

### P18-AUD-001: Agent-side failures are reclassified as infrastructure failures in pairing coverage

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P18-FAL-003, P18-RBK-001
- Claim: Agent crashes, Agent-owned timeouts, and rejected agent streams are
  recorded as `TrialOutcome::InfrastructureFailure`, so coverage labels them
  `infrastructure-failure:<trial>` and no Agent-failure outcome class exists.
- Evidence: `runner/experiment.rs:1422-1432`; `comparison.rs:49-56`;
  `phase18_assembled_smoke.rs:216-221` and `:306-311` (tests pin the label,
  including for boundary `agent-process` timeouts).
- Refutation attempted: searched all `TrialOutcome` construction sites (one,
  covering both hermetic and native modes); confirmed FAL-002's
  no-conversion-to-zero clause motivates only the refused grade dispatch, not
  the coverage label; confirmed receipts retain the true boundary (evidence
  stays honest) while the scoring/coverage classification — the clause's
  domain — is wrong; confirmed no ADR, spec revision, or registered deferral
  legitimizes the mapping; the native artifact's six trials all completed, so
  the mislabel is unexercised there but reachable on any future failure.
- Suggested closure: introduce an Agent-outcome failure classification in the
  pairing vocabulary (visible, attributable to the Agent, excluded from
  infrastructure exclusions), map `AgentCompletion::Failed` with
  Agent-owned boundaries to it, and re-pin the A05/A06 coverage assertions.

### P18-AUD-002: Verifier-native artifacts are staged under the agent-product source identity

- Axis: residuals
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: P18-MIG-003, P18-TRJ-001
- Claim: grader-produced bytes in sealed manifests carry `agent-<product>`
  source attribution; the report compensates by role-suffix matching.
- Evidence: `runner/experiment.rs:1196-1198` (single staging source);
  `report.rs:336-344`; ledger 18.13 flag 4 unresolved.
- Refutation attempted: roles remain distinguishable (MIG-003 holds); the
  mislabel affects source provenance readability, not classification.
- Suggested closure: stage verifier artifacts under a grader source identity.

### P18-AUD-003: DeepSWE oracle preflight bar is structural, not reward-positive

- Axis: test-quality
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: P18-INT-001
- Claim: a zero-reward DeepSWE reference solution would pass the oracle
  preflight and the task would be admitted.
- Evidence: `runner/experiment.rs:776-784` (`"deepswe-v1.1" => true`);
  `benchmark/process.rs:199-262` shows rewards are readable.
- Refutation attempted: pier's aggregate lacks a pass counter and pass_at_k is
  undefined for multi-metric rewards — but per-metric rewards are parsed and
  the real preflight earned 1.0, so a positive bar was implementable.
- Suggested closure: require all parsed reference-solution rewards > 0 for the
  DeepSWE preflight.

### P18-AUD-004: Native conformance-rerun receipt hardcases cases_run=12 for 13 executed cases

- Axis: residuals
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: P18-AGT-002
- Claim: the sealed artifact's conformance receipt misreports the executed
  case count.
- Evidence: `scripts/phase18-native-smoke.sh:1173-1186` (13 case specs) vs
  `:1206-1208` (`cases_run: 12`); ledger 18.15 flag deferred to 18.16, not
  applied.
- Refutation attempted: none available — the list and constant are adjacent in
  the committed producer.
- Suggested closure: derive `cases_run` from the executed list.

## Verification Commands

All commands ran inside the sealed export at `audit_head` (the A19 replay in
the byte-verified detached clone of the same commit), with the repository's
external Cargo cache workflow and the recorded host environment (proxies
unset, `RUST_TEST_THREADS=4`).

| Command | Result | Requirement/finding |
|---|---|---|
| `python3 .agents/skills/_shared/scripts/validate_assurance_artifact.py rotation docs/snapshots/phase18` | PASS | admission |
| `python3 scripts/opi-doc-check.py` | PASS | P18-A21, P18-RDM-001/003/005, AUTH-004 |
| 11× `python3 scripts/test_*.py` (doc-check, materialization ci+artifact, scripted provider, native ci+artifact, eval smoke, seam matrix, ci verifier, baseline capture, impl smoke) | PASS (eval smoke 3/3 with cargo on PATH) | P18-SEC-003, PLT-002, AGT-006, OUT-005, A22 substrate |
| `cargo tree -p opi-eval --all-features --target all --edges normal,build,dev` | PASS (0 opi-* edges) | P18-PLC-001, P18-A01 |
| `cargo tree --workspace --invert opi-eval` | PASS (no dependents) | P18-PLC-002, P18-OUT-006 |
| `cargo test -p opi-eval --no-fail-fast` | PASS 136 lib + 46 integration; only `p18_a19` failed for the environmental no-`.git` reason | all crate-level requirements |
| `cargo test -p opi-eval --test phase18_acceptance` (detached clone at `audit_head`) | PASS 3/3 incl. A19 replay (276.6 s) | P18-A19, P18-OUT-006, P18-MIG-004 |
| `cargo run -p opi-eval -- validate --config …generic-three-subject-fourth-benchmark.toml` | PASS (3 subjects, 2 edges, 4 trials) | P18-A20, P18-EXP-007, P18-RDM-002 |
| `cargo clippy -p opi-eval --all-targets -- -D warnings`; `cargo fmt --check -p opi-eval`; `RUSTDOCFLAGS="-D warnings" cargo doc -p opi-eval --no-deps` | PASS / PASS / PASS | standards axis |
| `sha256sum .github/workflows/ci.yml` vs ci-receipt `workflow_sha256` | MATCH `4c0f3fcb…` | P18-PLT-001, A22 |

## Verdict Rationale

Two mandatory requirements are partially met (`P18-FAL-003`, `P18-RBK-001`)
because Agent-side failures — including Agent-owned timeouts on valid tasks —
are reclassified as infrastructure failures in pairing coverage, contradicting
the registered clause and the exit-blocking risk threshold, with no registered
deferral. Under the mechanical member rule (any mandatory state other than
`met`), the member verdict is **FAIL**. All other 129 requirements are met on
current `audit_head` evidence, with two recorded limitations: the 6.13 GiB
native artifact was accepted through its committed digest chain rather than
re-downloaded, and the A19 baseline replay requires a git-equipped checkout
(verified in a byte-identical detached clone; the `git archive` export cannot
host it).
