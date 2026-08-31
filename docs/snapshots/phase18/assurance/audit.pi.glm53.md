# Phase 18 Audit

**Audit run ID**: `phase18-pi-glm53-68d74ec-20260830t200548z`
**Audit head**: `68d74ec0db78d0d198bd8ead9b3c8c31a364e65e`
**Reviewer ID**: `pi`
**Model ID**: `glm53`
**Reviewer identity**: pi coding agent (earendil-works pi runtime)
**Reviewer model ID**: `glm-5.3`
**Model identity source**: runtime-attested
**Independence**: fresh-context-same-family — fresh session context; same reviewer/model pair as the superseded member this run replaces; no prior audit, remediation, history, or sibling-peer content was read
**Baseline policy**: latest-committed-spec

**Verdict**: PASS-WITH-FINDINGS

Attestation detail: the runtime session record for this run carries `model_change → modelId: glm-5.3`, matching `defaultModel: glm-5.3` in the runtime settings; the identity is runtime-attested, not inferred.

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| `.opi-impl-state.json` | `9d2ecf977f940f03db3c5d3b17437ad4a3afbca6ad409fcebf306727848a358e` | current committed source; current_phase=19, phase_exit[18] complete |
| `docs/snapshots/phase18/opi-impl-state.json` | `cea5031074ac0d5667357863fbdf03bc76494295a6c38fac304dc1c851d7b42c` | current committed source; 20 tasks passing, 39-criteria trace, audit notes |
| `docs/opi-spec.md` | `cc7f8898f60c0d8abaa667f4b49b7affc721412e75dd3a67dcde37a783e1bc4c` | current committed source; matches ledger-registered SHA (no drift) |
| `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md` | `43b2759d327cbf0af8d35d4eba50839eef7aac473978b58fcb707b335dad8265` | current committed source; registered supplemental spec, matches stored hash |

Sealed export produced with `git archive` from `audit_head`; all checks ran against a `--shared` clone pinned at the same commit (byte-identical checkout, verified by recursive diff of `crates/opi-eval`) because the acceptance suite replays `git show <baseline-commit>:<anchor>`.

## Requirement Conformance

131 sealed records (109 `P18-*` requirements + 22 `P18-A*` scenarios), all mandatory. Full evidence per record lives in `audit.pi.glm53.requirements.jsonl`; the table cites the owning surface group.

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| P18-AUTH-001 | #Status and authority | `AUTH` group evidence (see sidecar) | met | — |
| P18-AUTH-002 | #Status and authority | `AUTH` group evidence (see sidecar) | met | — |
| P18-AUTH-003 | #Status and authority | `AUTH` group evidence (see sidecar) | met | — |
| P18-AUTH-004 | #Status and authority | `AUTH` group evidence (see sidecar) | met | — |
| P18-AUTH-005 | #Status and authority | `AUTH` group evidence (see sidecar) | met | — |
| P18-OUT-001 | #Outcome | `OUT` group evidence (see sidecar) | met | — |
| P18-OUT-002 | #Outcome | `OUT` group evidence (see sidecar) | met | — |
| P18-OUT-003 | #Outcome | `OUT` group evidence (see sidecar) | met | — |
| P18-OUT-004 | #Outcome | `OUT` group evidence (see sidecar) | met | — |
| P18-OUT-005 | #Outcome | `OUT` group evidence (see sidecar) | met | — |
| P18-OUT-006 | #Outcome | `OUT` group evidence (see sidecar) | met | — |
| P18-PLC-001 | #Architecture placement case | `PLC` group evidence (see sidecar) | met | — |
| P18-PLC-002 | #Architecture placement case | `PLC` group evidence (see sidecar) | met | — |
| P18-PLC-003 | #Architecture placement case | `PLC` group evidence (see sidecar) | met | — |
| P18-PLC-004 | #Architecture placement case | `PLC` group evidence (see sidecar) | met | — |
| P18-PLC-005 | #Architecture placement case | `PLC` group evidence (see sidecar) | met | — |
| P18-PLC-006 | #Architecture placement case | `PLC` group evidence (see sidecar) | met | — |
| P18-SEAM-001 | #Provisional package and seam discipline | `SEAM` group evidence (see sidecar) | met | — |
| P18-SEAM-002 | #Provisional package and seam discipline | `SEAM` group evidence (see sidecar) | met | — |
| P18-SEAM-003 | #Provisional package and seam discipline | `SEAM` group evidence (see sidecar) | met | — |
| P18-SEAM-004 | #Provisional package and seam discipline | `SEAM` group evidence (see sidecar) | met | — |
| P18-SEAM-005 | #Provisional package and seam discipline | `SEAM` group evidence (see sidecar) | met | — |
| P18-EXP-001 | #Resolved experiment identity and pairing | `EXP` group evidence (see sidecar) | met | — |
| P18-EXP-002 | #Resolved experiment identity and pairing | `EXP` group evidence (see sidecar) | met | — |
| P18-EXP-003 | #Resolved experiment identity and pairing | `EXP` group evidence (see sidecar) | met | — |
| P18-EXP-004 | #Resolved experiment identity and pairing | `EXP` group evidence (see sidecar) | met | — |
| P18-EXP-005 | #Resolved experiment identity and pairing | `EXP` group evidence (see sidecar) | met | — |
| P18-EXP-006 | #Resolved experiment identity and pairing | `EXP` group evidence (see sidecar) | met | — |
| P18-EXP-007 | #Resolved experiment identity and pairing | `EXP` group evidence (see sidecar) | met | — |
| P18-EXP-008 | #Resolved experiment identity and pairing | `EXP` group evidence (see sidecar) | met | — |
| P18-DUR-001 | #Trial durability and effect uncertainty | `DUR` group evidence (see sidecar) | met | — |
| P18-DUR-002 | #Trial durability and effect uncertainty | `DUR` group evidence (see sidecar) | met | — |
| P18-DUR-003 | #Trial durability and effect uncertainty | `DUR` group evidence (see sidecar) | met | — |
| P18-DUR-004 | #Trial durability and effect uncertainty | `DUR` group evidence (see sidecar) | met | — |
| P18-DUR-005 | #Trial durability and effect uncertainty | `DUR` group evidence (see sidecar) | met | — |
| P18-AGT-001 | #Agent harness process integrations | `AGT` group evidence (see sidecar) | met | — |
| P18-AGT-002 | #Agent harness process integrations | `AGT` group evidence (see sidecar) | met | — |
| P18-AGT-003 | #Agent harness process integrations | `AGT` group evidence (see sidecar) | met | — |
| P18-AGT-004 | #Agent harness process integrations | `AGT` group evidence (see sidecar) | met | — |
| P18-AGT-005 | #Agent harness process integrations | `AGT` group evidence (see sidecar) | met | — |
| P18-AGT-006 | #Agent harness process integrations | `AGT` group evidence (see sidecar) | met | — |
| P18-AGT-007 | #Agent harness process integrations | `AGT` group evidence (see sidecar) | met | — |
| P18-AGT-008 | #Agent harness process integrations | `AGT` group evidence (see sidecar) | met | — |
| P18-AGT-009 | #Agent harness process integrations | `AGT` group evidence (see sidecar) | met | — |
| P18-BMK-001 | #Benchmark task-package and native-verifier integrations | `BMK` group evidence (see sidecar) | met | — |
| P18-BMK-002 | #Benchmark task-package and native-verifier integrations | `BMK` group evidence (see sidecar) | met | — |
| P18-BMK-003 | #Benchmark task-package and native-verifier integrations | `BMK` group evidence (see sidecar) | met | — |
| P18-BMK-004 | #Benchmark task-package and native-verifier integrations | `BMK` group evidence (see sidecar) | met | — |
| P18-BMK-005 | #Benchmark task-package and native-verifier integrations | `BMK` group evidence (see sidecar) | met | — |
| P18-BMK-006 | #Benchmark task-package and native-verifier integrations | `BMK` group evidence (see sidecar) | met | — |
| P18-BMK-007 | #Benchmark task-package and native-verifier integrations | `BMK` group evidence (see sidecar) | met | — |
| P18-BMK-008 | #Benchmark task-package and native-verifier integrations | `BMK` group evidence (see sidecar) | met | — |
| P18-BMK-009 | #Benchmark task-package and native-verifier integrations | `BMK` group evidence (see sidecar) | met | — |
| P18-BMK-010 | #Benchmark task-package and native-verifier integrations | `BMK` group evidence (see sidecar) | met | — |
| P18-BMK-011 | #Benchmark task-package and native-verifier integrations | `BMK` group evidence (see sidecar) | met | — |
| P18-RDM-001 | #Post-Phase Eval coverage roadmap | `RDM` group evidence (see sidecar) | met | — |
| P18-RDM-002 | #Post-Phase Eval coverage roadmap | `RDM` group evidence (see sidecar) | met | — |
| P18-RDM-003 | #Post-Phase Eval coverage roadmap | `RDM` group evidence (see sidecar) | met | — |
| P18-RDM-004 | #Post-Phase Eval coverage roadmap | `RDM` group evidence (see sidecar) | met | — |
| P18-RDM-005 | #Post-Phase Eval coverage roadmap | `RDM` group evidence (see sidecar) | met | — |
| P18-RDM-006 | #Post-Phase Eval coverage roadmap | `RDM` group evidence (see sidecar) | met | — |
| P18-INT-001 | #Benchmark revision integrity | `INT` group evidence (see sidecar) | met | — |
| P18-INT-002 | #Benchmark revision integrity | `INT` group evidence (see sidecar) | met | — |
| P18-INT-003 | #Benchmark revision integrity | `INT` group evidence (see sidecar) | met | — |
| P18-INT-004 | #Benchmark revision integrity | `INT` group evidence (see sidecar) | met | — |
| P18-INT-005 | #Benchmark revision integrity | `INT` group evidence (see sidecar) | met | — |
| P18-BND-001 | #Content-addressed RunBundle | `BND` group evidence (see sidecar) | met | — |
| P18-BND-002 | #Content-addressed RunBundle | `BND` group evidence (see sidecar) | met | — |
| P18-BND-003 | #Content-addressed RunBundle | `BND` group evidence (see sidecar) | met | — |
| P18-BND-004 | #Content-addressed RunBundle | `BND` group evidence (see sidecar) | met | — |
| P18-BND-005 | #Content-addressed RunBundle | `BND` group evidence (see sidecar) | met | — |
| P18-BND-006 | #Content-addressed RunBundle | `BND` group evidence (see sidecar) | met | — |
| P18-TRJ-001 | #Trajectory and causal-span hypotheses | `TRJ` group evidence (see sidecar) | met | — |
| P18-TRJ-002 | #Trajectory and causal-span hypotheses | `TRJ` group evidence (see sidecar) | met | — |
| P18-TRJ-003 | #Trajectory and causal-span hypotheses | `TRJ` group evidence (see sidecar) | met | — |
| P18-TRJ-004 | #Trajectory and causal-span hypotheses | `TRJ` group evidence (see sidecar) | met | — |
| P18-TRJ-005 | #Trajectory and causal-span hypotheses | `TRJ` group evidence (see sidecar) | met | — |
| P18-FAL-001 | #Failure, cancellation, and classification | `FAL` group evidence (see sidecar) | met | — |
| P18-FAL-002 | #Failure, cancellation, and classification | `FAL` group evidence (see sidecar) | met | — |
| P18-FAL-003 | #Failure, cancellation, and classification | `FAL` group evidence (see sidecar) | met | — |
| P18-FAL-004 | #Failure, cancellation, and classification | `FAL` group evidence (see sidecar) | met | — |
| P18-FAL-005 | #Failure, cancellation, and classification | `FAL` group evidence (see sidecar) | met | — |
| P18-RPT-001 | #Offline recomputation and outcome-first reporting | `RPT` group evidence (see sidecar) | met | — |
| P18-RPT-002 | #Offline recomputation and outcome-first reporting | `RPT` group evidence (see sidecar) | met | — |
| P18-RPT-003 | #Offline recomputation and outcome-first reporting | `RPT` group evidence (see sidecar) | met | — |
| P18-RPT-004 | #Offline recomputation and outcome-first reporting | `RPT` group evidence (see sidecar) | met | — |
| P18-RPT-005 | #Offline recomputation and outcome-first reporting | `RPT` group evidence (see sidecar) | met | — |
| P18-RPT-006 | #Offline recomputation and outcome-first reporting | `RPT` group evidence (see sidecar) | met | — |
| P18-SEC-001 | #Privacy, authority, and supply-chain boundaries | `SEC` group evidence (see sidecar) | met | — |
| P18-SEC-002 | #Privacy, authority, and supply-chain boundaries | `SEC` group evidence (see sidecar) | met | — |
| P18-SEC-003 | #Privacy, authority, and supply-chain boundaries | `SEC` group evidence (see sidecar) | met | — |
| P18-SEC-004 | #Privacy, authority, and supply-chain boundaries | `SEC` group evidence (see sidecar) | met | — |
| P18-SEC-005 | #Privacy, authority, and supply-chain boundaries | `SEC` group evidence (see sidecar) | met | — |
| P18-SEC-006 | #Privacy, authority, and supply-chain boundaries | `SEC` group evidence (see sidecar) | met | — |
| P18-MIG-001 | #Compatibility, migration, and Minimal Runtime | `MIG` group evidence (see sidecar) | met | — |
| P18-MIG-002 | #Compatibility, migration, and Minimal Runtime | `MIG` group evidence (see sidecar) | met | — |
| P18-MIG-003 | #Compatibility, migration, and Minimal Runtime | `MIG` group evidence (see sidecar) | met | — |
| P18-MIG-004 | #Compatibility, migration, and Minimal Runtime | `MIG` group evidence (see sidecar) | met | — |
| P18-MIG-005 | #Compatibility, migration, and Minimal Runtime | `MIG` group evidence (see sidecar) | met | — |
| P18-MIG-006 | #Compatibility, migration, and Minimal Runtime | `MIG` group evidence (see sidecar) | met | — |
| P18-PLT-001 | #Platform scope | `PLT` group evidence (see sidecar) | met | — |
| P18-PLT-002 | #Platform scope | `PLT` group evidence (see sidecar) | met | — |
| P18-PLT-003 | #Platform scope | `PLT` group evidence (see sidecar) | met | — |
| P18-PLT-004 | #Platform scope | `PLT` group evidence (see sidecar) | met | — |
| P18-RBK-001 | #Risk thresholds and rollback | `RBK` group evidence (see sidecar) | met | — |
| P18-RBK-002 | #Risk thresholds and rollback | `RBK` group evidence (see sidecar) | met | — |
| P18-RBK-003 | #Risk thresholds and rollback | `RBK` group evidence (see sidecar) | met | — |
| P18-RBK-004 | #Risk thresholds and rollback | `RBK` group evidence (see sidecar) | met | — |
| P18-RBK-005 | #Risk thresholds and rollback | `RBK` group evidence (see sidecar) | met | — |
| P18-A01 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A02 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A03 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A04 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A05 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A06 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A07 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A08 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A09 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A10 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A11 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A12 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A13 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A14 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A15 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A16 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A17 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A18 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A19 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A20 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A21 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | — |
| P18-A22 | #Acceptance scenarios and verification | `SCEN` group evidence (see sidecar) | met | P18-AUD-001 |

## Standards Review

- **Repository rules**: `scripts/opi-doc-check.py` PASS at audit_head; bilingual README pair (`README.md`/`README.zh.md`) present for the crate; CHANGELOG carries the Phase 18 entries under `[Unreleased]`; the crate manifest inherits workspace metadata without duplication.
- **Rust correctness**: `#![deny(unsafe_code)]` with one documented FFI home (`process::tree`, OS process-tree termination with no safe alternative); typed `thiserror`-style failures throughout; enums for closed lifecycles; no TODO/FIXME/`unimplemented!` in `crates/opi-eval/src`.
- **Dependency direction**: zero `opi-*` edges under `--all-features --target all --edges normal,build,dev`; not in `[workspace.dependencies]`; no reverse dependency or call-site in any product crate (grep + `cargo metadata` assertion in the A19 test).
- **Failure behavior**: every typed failure carries a `FailureBoundaryCode`; the authority ledger mechanically stops later transitions (four `authority_boundaries` tests).
- **Public contract consistency**: library surface is `experiment::ResolvedExperiment` + `cli` only; all other modules crate-private behind documented provisional-seam comments; `publish = false`.

## Spec Review

All 109 `P18-*` requirements and 22 scenarios were sealed before implementation inspection and traced to current code, tests, scripts, CI receipts, and the artifact-bound seam matrix. Highlights:

- Fail-closed resolution with explicit control markers; canonical digest identity; N-subject/directed-edge contract proven with both real adapters (experiment contract tests + native artifact binding).
- Intent-before-effect durability with a `DurableIntentProof` type that only `RunBundle::publish_intent` can mint; effect-unknown recovery is a closed classification (A07).
- Bundle sealing is content-addressed, reservation-closed, symlink/path-grammar/oversize-rejecting, and mutation-invalidating without repair (A15, BND-001).
- Native-verifier authority preserved per revision (Harbor CTRF for TB 2.1, separate-container declared-original-paths for TB 3.0, Pier separate-pristine no-network collected-patch for DeepSWE v1.1); reward stays native; zero-reward is a valid outcome that stays zero.
- Reports recompute from verified sealed bundles only, are byte-stable, keep every exclusion in the denominator, and label the evidence `conformance-evidence`.

## Security, Invariants, Integration, Test Quality, and Residuals

- **Security/authority**: activation is explicit; the static external lock pins workflow SHA-256, producer script digests, GitHub Action commits, and image digests; spawn specs are structured argv/env vectors (`/bin/sh` appears only in test fixtures); canary leakage blocks sealing and publication (A18); no stronger-sandbox claims.
- **Invariants/integration**: forward-only trial ladder; single-writer bundles; atomic manifest publication; offline operations never spawn or mutate; integration covered by 13 test binaries driving the real CLI binary and adapters.
- **Test quality**: 203 passing tests at audit_head with discriminating assertions (settlement tables asserted independently of driver `met` flags; negative matrices for every fail-closed boundary); hermetic suites refuse to stand in for native evidence (`hermetic_only_conformance_cases_are_refused_in_native_mode`).
- **Residuals**: no dual paths, aliases, or compatibility bridges (rollback-contract test); `#[allow(dead_code)]` is confined to documented crate-private provisional seams; the ledger's two non-blocking flags (hermetic negative gap for TB 3.0 artifact-boundary enforcement; shared decisions closed through integration binaries rather than dedicated per-interface tests) were re-examined and are covered by the declared policy validation plus the native canary-oracle negative preflight record and by the integration binaries' behavioral assertions respectively.

## Minimum-change Conformance

All 20 ledger tasks record `reuse_search`, `surface_necessity`, `simplification_ceiling` (plus `placement`, `forbidden_scope`, `shared_decision`, `test_impact`) in `inference_notes`, with `production_call_sites` per task. Spot verification against current code: 18.1's recorded ceiling (one publish-disabled package, one `validate` entry, no plugin SDK) matches the current surface (no SDK; the later run/regrade/report/conformance commands were added by their own recorded tasks); 18.12/18.13/18.14.1 call sites resolve to the current runner/report/driver modules; no task introduced a seam beyond its recorded ceiling. Status: **conforming**.

## Findings

### P18-AUD-001: Workspace gate test assumes a proxy-free environment

- Axis: test-quality
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: P18-A22
- Claim: config_tests::build_http_client_without_proxy_succeeds fails whenever ambient HTTP(S)_PROXY/ALL_PROXY variables are set, because it asserts build_http_client(None) yields no proxy URL without scrubbing or forking the ambient proxy environment.
- Evidence: `crates/opi-coding-agent/tests/config_tests.rs:721` — Test calls build_http_client(None) and asserts client.proxy_config().url.is_none(); with HTTP_PROXY=http://127.0.0.1:19828 exported the product correctly adopts the ambient proxy and the assertion fails.
- Evidence: `audit sandbox reproduction` — cargo test -p opi-coding-agent --test config_tests FAILED (1/49) with ambient proxy vars; PASSED (49/49) with proxy vars unset. Product crates are byte-identical to three-platform CI-green commit 0f5a3fa.
- Refutation attempted: Searched the test and helpers for proxy scrubbing (none); confirmed the product adopting ambient proxies is intended Phase 3 behavior, so the gap is test isolation, not product behavior; CI environments are proxy-free, which explains green CI; no P18 requirement is unmet because the sanctioned gate environment passes and the workspace gate passes with the environment cleaned.
- Suggested closure: scrub or fork the ambient proxy environment inside the test (or assert against an explicit no-proxy env), so the workspace gate is reproducible behind a corporate proxy.

## Verification Commands

| Command | Result | Requirement/finding |
|---|---|---|
| `python3 scripts/opi-doc-check.py` | PASS | AUTH-004, SEAM-005, documentation lockstep |
| `cargo fmt --check --all` | PASS | A22 |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | A22 |
| `cargo test -p opi-eval --all-targets` | PASS (144 unit + 59 integration, 0 failed) | all P18 groups |
| `cargo test -p opi-eval --doc` | PASS (0 doc tests in crate) | A22 |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p opi-eval --no-deps` | PASS | SEAM-001 wording, A22 |
| `cargo tree -p opi-eval --all-features --target all --edges normal,build,dev` | PASS (zero opi-* edges) | PLC-001/002, A01 |
| `cargo run -q -p opi-eval -- validate --config crates/opi-eval/tests/fixtures/experiment/minimal.toml` | PASS (exit 0, canonical digest printed) | EXP-001, A01 |
| `python3 scripts/test_derive_phase18_seam_matrix.py` | PASS (7/7) | SEAM-003, OUT-005 |
| `cargo test --workspace --all-targets --no-fail-fast` | FAIL(env)→PASS on rerun: 4 opi-coding-agent suites failed only under ambient proxy vars/parallel load; each passes with proxy unset and --test-threads=1; product crates byte-identical to CI-green 0f5a3fa | A22, P18-AUD-001 |
| `cargo test --workspace --doc` | PASS (12/0) | A22 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | PASS | A22 |
| `python3 scripts/verify-phase18-native-artifact.py --criterion all-native ...` | NOT RUN (6.1 GB artifact not re-downloaded); binding verified: GitHub API run 33271354427 head_sha 27344e3 matches committed matrix | A02/A03/A04/A08–A10/A12/BMK-003 |

## Verdict Rationale

Every one of the 131 sealed mandatory requirements is `met` at `audit_head` with current code, test, script, receipt, or artifact-bound evidence; no mandatory state is `not-met`, `partially-met`, or `not-assessable`. One advisory Minor finding exists (P18-AUD-001, test-environment isolation on a pre-Phase-18 surface, byte-identical to CI-green code), so the mechanical member verdict is **PASS-WITH-FINDINGS**. Limitations recorded honestly: the 6.1 GB native artifact was not re-downloaded (its binding chain was re-verified against the live GitHub API and committed digests), and the native smoke artifact binds candidate `27344e3` — remediation commits after it touched only `opi-eval`, scripts, and snapshot paths, with no requirement mandating an artifact refresh.
