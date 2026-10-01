# Phase 19 Audit

**Audit run ID**: `phase19-pi-glm53-bc9a5c7-20260930t231208z`
**Audit head**: `bc9a5c75ba74ed59b6a676023af813566300a388`
**Reviewer ID**: pi
**Model ID**: glm53
**Reviewer identity**: pi coding agent harness
**Reviewer model ID**: `glm53`
**Model identity source**: operator-declared
**Independence**: fresh-context-same-family — fresh context sealed from a git-archive export of the committed audit head; no prior audit, remediation conclusion, history run, or sibling peer output was read before installation; same model family as any GLM-operated peer cannot be excluded.
**Baseline policy**: latest-committed-spec
**Verdict**: PASS-WITH-FINDINGS

## Baseline Sources

Sealed through `read_audit_scope.py --phase 19` and re-hashed byte-exact inside the
`git archive` export used for every check below. No stored hash mismatches.

| Path | SHA-256 | Registration note |
|---|---|---|
| .opi-impl-state.json | c65c10635c4bb8b1754625b94e8a0a15459eef6d65634610ba9b65ea9ba5337e | current committed root ledger (read only through the scope reader projection) |
| docs/snapshots/phase19/opi-impl-state.json | 35b3378bd902cc976bb348d794dd1b5d3c5f160a58769c3aeb62fea175a7b7a9 | committed Phase 19 task graph (requirements, scenarios, verification declarations) |
| docs/opi-spec.md | 1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db | latest committed normative parent (STRAT-002, CTRL-004..007, PRIN/PLACE/PHASE clauses) |
| docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md | 5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a | registered Phase 19 delivery specification (P19-AC-001..013, P19-D01..D08) |
| docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.zh.md | d41c826412b0d71802327d60f953708f6cb2083436a2a2b0d99a2e52d851f194 | Chinese counterpart; identifiers and requirements verified equivalent |

## Requirement Conformance

One sealed requirement per registered task definition of done; criterion evidence cites
the committed Phase ledger. All states derive from current audit_head evidence.

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| 19.1 | task 19.1 DoD (P19-D01; AC-002, AC-003-native, AC-008-outcome) | NativeTrialBinding + typed NativeBindingError substitution matrix in benchmark/process.rs; classify_native_trial single sealed settlement in runner/experiment.rs; legacy newest_job_result/oracle preflight absent; native_driver 110 passed incl. native_result_binding_rejects_every_substitution and native_trial_outcome_classifies_every_terminal_shape | met | — |
| 19.2 | task 19.2 DoD (P19-D02; AC-006-fixture, AC-010-credentials) | experiment.rs named-connection schema with exactly-one credential reference, explicit live mode, extended controls with recorded origins and required-control/caps rejection; pi.rs retry-phase importer with typed terminal causes incl. 429 and JSON-at-exit-0; experiment_contract 34, agent_integration_conformance 2, authority_boundaries 4, fail-closed credential tests passed; exact-client matrix filters platform-gated (CI-exercised) | met | — |
| 19.3 | task 19.3 DoD (P19-D03; AC-004-native) | task_lifecycle.rs typed Harbor lifecycle admission; CollectedArtifact digest binding with UndeclaredArtifact/DeclaredArtifactMissing/DeliveredArtifactSubstituted/LifecycleMismatch rejections; native_tb30 positive/negative tests passed through the public run seam | met | — |
| 19.4 | task 19.4 DoD (P19-D04; AC-005-native) | deepswe.rs Pier collect-hook parse; NativeTrialBinding.base_revision/docker_image committed-patch identity; pristine no-network verifier; nop-zero semantics; native_deepswe positive/base-drift/nop tests passed | met | — |
| 19.5 | task 19.5 DoD (P19-D05; AC-001-prepare, AC-007-material) | cli/prepare.rs public doctor/prepare; runner/prepare.rs verification and image-digest freeze; material/6 local-origin manifest; independent_directory_prepare_builds_a_local_origin_material and local_material_integrity passed (no GitHub metadata, no repository paths) | met | — |
| 19.6 | task 19.6 DoD (P19-D05; AC-007-selection, AC-001-plan) | runner/plan.rs corpus enumeration/selection expansion/resource preflight; shared resolve path for run --config and run --plan; frozen_subset_execution and tampered_plan_rejects_before_any_trial passed with exact subset execution and digest-bound immutability | met | — |
| 19.7 | task 19.7 DoD (P19-D06; AC-008, AC-009-verify) | verify/report byte-vs-recompute separation; report.rs coverage_counts, per-benchmark metric definitions and denominators, planned binding before effects, no cross-benchmark average; native_provider_failure_report passed (provider failure out of denominator, native zero graded); determinism/tamper/containment suites passed | met | — |
| 19.8 | task 19.8 DoD (P19-D06; AC-009-regrade) | regrade.rs NativeRegrader verifies source bundles first, fresh verifier-only environments, credential-shaped env denylist, Pier network fail-closed, derived records without rewriting scores; offline_regrade and native_offline_regrade* fail-closed matrix passed | met | — |
| 19.9 | task 19.9 DoD (P19-D07; AC-010-lifecycle) | one supervisor with shared cancellation token; native_cleanup.rs confirmed owned-container cleanup gating success; recovery_report durable classification; replacement fresh paired identities; cancellation-race, effect-unknown, cleanup-uncertainty and replacement tests passed | met | — |
| 19.10 | task 19.10 DoD (P19-D07; AC-011, AC-013) | workflows delegate to the shared local prepare/materialize path; verify-local-acceptance.py offline verifier validated end-to-end by this audit; dummy-credential and pi-only packaging tests passed; bilingual READMEs synchronized (10/10 sections); CHANGELOG Unreleased records the 0.x schema cutover; doc-check PASS; 3 default Python tests error on hosts without /usr/bin/uv (P19-AUD-001) | met | P19-AUD-001 |
| 19.11 | task 19.11 DoD (P19-D08; AC-001/003/004/005/006-live, AC-012) | bilingual redacted acceptance record committed; evidence-roots registry records run trees, retained failed attempts and the 2026-09-29 re-collected root; this audit independently re-verified that root with the audit_head verifier and an audit_head-built companion (accepted:true, exit 0, six attributable glm-5.3-flash trials) and the Opi-only artifact audit (PASS); audit doc references only the superseded root (P19-AUD-002) | met | P19-AUD-002 |

## Standards Review

- Dependency direction: opi-eval depends on no Opi crate in any dependency class
  (Cargo.toml inspection plus pi_only_without_opi_runtime_and_packaging). No Eval LLM
  client exists (no HTTP dependency in the manifest).
- Unsafe code is denied crate-wide and isolated in process/tree.rs, the single
  documented unsafe-FFI home, with per-block SAFETY comments (Unix process groups,
  Windows Job Objects).
- `cargo clippy -p opi-eval --all-targets -- -D warnings` passes with zero warnings at
  the audit_head export.
- Documentation lockstep: opi-eval README.md/README.zh.md carry the same 10 top-level
  sections; docs/eval/README.md documents the evidence-registry and conclusion-pair
  contracts; `scripts/opi-doc-check.py` passes.
- CHANGELOG Unreleased carries the deliberate 0.x cutover entries for opi-eval
  (native-material/6, terminal-preflight/2, regrade/2, report/2, plan/2 schemas).

## Spec Review

Each registered DoD obligation was traced to its production surface and a discriminating
public-seam test (see the requirement table). Key falsifiable behaviors verified at
audit_head: identity/artifact/oracle substitution rejection even on exit 0; sealed single
trial classification where neither exit code nor boundary code alone decides score
eligibility; declared-artifact TB3 and committed-patch DeepSWE handoffs with digest
binding and canary exclusion; local-origin material preparation without GitHub receipt
dependence; digest-bound frozen subset execution without implicit filtering; provider
failures preserved as attempted/invalid while native zeros stay graded; offline regrade
with fresh verifier environments and fail-closed missing-material rejection; owned-resource
cleanup evidence gating successful cancellation; dummy credentials not activating live
behavior; and the six-trial acceptance evidence independently re-verified offline.
Oracle remains an explicitly selected conformance diagnostic only.

## Security, Invariants, Integration, Test Quality, and Residuals

- Security/authority: credentials resolve only inside the supervised Agent launch
  (stage_native_handoff); native_settlement_removes_actual_credentials passed; the regrade
  spawn projection denies credential-shaped environment keys; the acceptance verifier
  strips credential markers before invoking the Companion; canary sealing blocks
  publication; remote base URLs require explicit live mode.
- Invariants/integration: immutable digest-bound plans and bundles; tampered plans and
  bundles reject before effects; recovery preserves prior evidence byte-for-byte and
  replaces under fresh paired identities; unsealed settlements never upgrade to scores.
- Test quality: assertions observe required behavior rather than success-only (verified by
  reading native_provider_failure_report, frozen_subset_execution,
  native_settlement_cleanup_uncertainty_blocks_success,
  native_result_binding_rejects_every_substitution, independent_directory_prepare test
  bodies). Eleven native filters are platform-gated (pinned external material and/or
  Docker) and were not executed in this audit context; compensating public-seam coverage
  passed and the CI native smoke is the declared executor for those witnesses.
- Residuals: no TODO/FIXME/unimplemented markers in crates/opi-eval/src; no dead public
  surface (clippy -D warnings over all targets); no competing grading path (single
  classifier, single report builder).

## Minimum-change Conformance

Audited against the recorded reuse_search/surface_necessity/simplification_ceiling notes
in the committed ledger projection. Status: conforming for all eleven tasks.

- reuse_search: recorded reuse (RunBundle::verify, BenchmarkExecution, family adapters,
  ReportBuilder, ProcessSupervisor, lifecycle journal, scripted-provider) is what the
  current code uses; no duplicated mechanism found (one classifier, one report builder,
  one supervisor).
- surface_necessity: every new public operation (doctor, prepare, plan, verify, report,
  regrade, run --plan/--recover/--replacement-for) reaches production consumers through
  main.rs and has discriminating tests; no speculative trait, crate, or config key
  observed; the native-trial-artifact-binding and verified-local-material-identity shared
  decisions have their recorded multiple consumers.
- simplification_ceiling / net_deletion: legacy paths recorded for removal are absent
  (newest_job_result authority, host-agent/default-job score route, oracle preflight,
  mandatory GitHub receipt for local material); forbidden-scope items are absent (no
  embedded datasets/graders/images, no Opi dependency, no statistical-superiority claim,
  no 18 mandatory live trials — six-trial scope verified).
- residual_glue: none beyond the documented integration/verifier boundaries.

## Findings

### P19-AUD-001: Default Python regression tests depend on an undeclared host uv binary

- Axis: test-quality
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: 19.10
- Claim: The three materialization tests in test_native_smoke_output_control.py error on
  hosts without /usr/bin/uv because the mocked `command -v uv` fabricates that path while
  the embedded materialize stage performs a real shutil.copy2 of the resolved binary; the
  default registered verification command fails on such hosts (observed 3 errors of 251).
- Evidence: test_native_smoke_output_control.py:67-69 (hardcoded "/usr/bin/uv" mock
  return); native-smoke.sh:897-903 (real copy of the resolved uv into the cache);
  observed unittest run in the audit_head export on WSL Ubuntu without uv.
- Refutation attempted: searched for skip guards (none — the tests error rather than
  skip); checked whether uv is a declared prerequisite of the default unit suite (the
  test docstring claims only external commands are replaced; README prerequisites cover
  the native workflow, not the hermetic unit suite); confirmed the production
  materialization behavior is correct on uv-equipped hosts and this is purely a
  host-conditional default-test failure. Severity stays Minor/advisory.
- Suggested closure: make the mock point at (or create) a synthetic uv executable inside
  the test's temporary directory so the suite stays hermetic, or skip with an explicit
  message when the resolved uv is absent.

### P19-AUD-002: Acceptance audit doc references only the superseded 2026-09-15 root

- Axis: standards
- Severity: Info
- Conformance effect: advisory
- Requirement IDs: 19.11
- Claim: docs/audits/phase19-local-native-acceptance.md and .zh.md record the 2026-09-15
  root as the formal handoff and note the current verifier rejects it, but do not
  cross-reference the 2026-09-29 re-collected root the current verifier accepts
  (registered only in docs/eval/evidence-roots.jsonl; its owning report was removed from
  the checkout per the documented conclusion-pair contract). A reader of the audit
  documents alone cannot locate the currently consumable acceptance evidence.
- Evidence: docs/audits/phase19-local-native-acceptance.md:18-30; docs/eval/
  evidence-roots.jsonl final record; this audit's independent re-verification of the
  re-collected root (accepted:true, exit 0) confirming the registry claim is accurate.
- Suggested closure: add one cross-reference line in both acceptance audit documents
  pointing to the registry record of the currently accepted re-collected root.

## Verification Commands

All commands ran inside the sealed audit_head export
(sha256-verified against the reader's baseline digests) with the repository's external
Cargo cache workflow.

| Command | Result | Requirement/finding |
|---|---|---|
| sha256sum of the five baseline sources in the git-archive export | PASS (byte-exact match) | baseline seal |
| cargo build -p opi-eval --bin opi-eval | PASS (24s) | companion for offline verification |
| cargo test -p opi-eval (lib + 12 integration binaries + doc-tests) | PASS (161+2+15+4+2+7+6+34+6+110+2+6+14; 11 platform-gated ignored) | 19.1–19.10 |
| cargo clippy -p opi-eval --all-targets -- -D warnings | PASS | standards |
| python3 -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py' | 247 pass, 1 skip, 3 errors (missing /usr/bin/uv) | 19.10, P19-AUD-001 |
| python3 scripts/opi-doc-check.py | PASS | 19.10 |
| python3 crates/opi-eval/scripts/verify-local-acceptance.py --root <registered 2026-09-29 root> --out <temp> --companion <audit_head-built binary> | PASS (accepted:true, exit 0, six trials, three families) | 19.11 |
| python3 scripts/opi-artifact-audit.py <root>/opi-artifact-audit --workspace-root <export> | PASS | 19.11 |
| grep for newest_job_result / run_oracle_preflight / TODO / FIXME in crates/opi-eval/src | no production matches | residuals, minimum-change |

## Verdict Rationale

All eleven sealed mandatory requirements are met on current audit_head evidence: every
task DoD traces to production surfaces and discriminating public-seam tests that pass in
the sealed export, and the six-trial acceptance evidence was independently re-verified
offline with the committed verifier and an audit_head-built companion. Two advisory
findings remain (one Minor host-hermeticity gap in the default Python suite, one Info
documentation cross-reference gap); neither blocks conformance, so the mechanical member
verdict is PASS-WITH-FINDINGS. Limitations: the eleven platform-gated native filters
(pinned external material and/or Docker) were not executed in this context; their
compensating public-seam coverage passed and the acceptance evidence exercises the same
production paths end-to-end.

<!-- opi-audit-history/1
{
  "audit\u005fhead": "bc9a5c75ba74ed59b6a676023af813566300a388",
  "audit\u005frun\u005fid": "phase19-pi-glm53-bc9a5c7-20260930t231208z",
  "comparison": {
    "comparisons": [
      {
        "cause": "The unit tests were authored on a uv-equipped host; the hermetic mock fabricates a host path that the embedded materializer then reads with real file I/O, so the dependence never failed for its authors. Bounded inference from the mock and copy call sites; no other undeclared host reads were found in the suite.",
        "current\u005fevidence": "At audit\u005fhead, test\u005fnative\u005fsmoke\u005foutput\u005fcontrol.py mocks \u0060command -v uv\u0060 to return the hardcoded /usr/bin/uv while the materialize-configs stage in native-smoke.sh performs a real shutil.copy2 of that path; on this audit's WSL Ubuntu host without uv, python -m unittest discover produced 3 errors of 251 in MaterializeOutputControlTest. The historical script-modes defect is a different host-brittleness branch and stays refuted; this branch is a distinct, currently present defect.",
        "current\u005ffinding\u005fid": "P19-AUD-001",
        "prior\u005fclaim": "ad67cb8 run claimed non-executable script modes broke the default suite on faithful checkouts (Major); remediation record 265078f verified that claim Refuted on a fresh ext4 git archive preserving mode 0755. No prior Phase 19 finding names the undeclared host uv binary dependence now observed.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-pi-glm53-ad67cb8-20260918t101920z",
            "finding\u005fid": "P19-AUD-001",
            "source\u005fid": "s1"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "mock at command() returns /usr/bin/uv without creating it; no skip guard exists; observed 3 FileNotFoundError/FileExistsError tracebacks in this audit's export run",
            "limitations": "observed on one uv-less WSL Ubuntu host; a uv-equipped host was not re-executed in this audit",
            "path": "crates/opi-eval/scripts/test\u005fnative\u005fsmoke\u005foutput\u005fcontrol.py",
            "sha256": "b6362619c612cb16156a9cc1512961c2e9f51af5a66aadc63f2a433b4b1e3e4d"
          },
          {
            "checked": "materialize-configs resolves uv via \u0060command -v uv\u0060 and copies the resolved real binary into cache/tools/uv",
            "limitations": "production behavior on uv-equipped hosts is unaffected; the defect is the default test suite's host dependence only",
            "path": "crates/opi-eval/scripts/native-smoke.sh",
            "sha256": "5abe7291e3cbcaf6cd5c6e1d6db5fa498e5a62dee331605bdca8d808eeddcd33"
          }
        ],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "python3 -m unittest discover -s crates/opi-eval/scripts -p 'test\u005f\u002a.py' on a host without /usr/bin/uv (expect the 3 errors), then point the mock at a synthetic executable inside the test tempdir (or add an explicit uv-absent skip) and rerun expecting 0 errors."
      },
      {
        "cause": "The bilingual acceptance audit documents were finalized before the re-collection and were not revisited when the registry gained the accepted root and the conclusion-pair removal landed.",
        "current\u005fevidence": "The e079780 substance was closed by the user-authorized 2026-09-29 re-collection registered at ed945ed0: this audit independently re-verified that root with the audit\u005fhead verifier and an audit\u005fhead-built companion (accepted:true, exit 0, six trials) and the Opi-only artifact audit passed. The residual defect is documentation-only: docs/audits/phase19-local-native-acceptance.md and .zh.md still reference only the superseded 2026-09-15 root and its current-verifier rejection, without a cross-reference to the registry record of the currently accepted root; the recollection's owning report was removed from the checkout under the documented conclusion-pair contract.",
        "current\u005ffinding\u005fid": "P19-AUD-002",
        "prior\u005fclaim": "e079780 run claimed the six-trial acceptance root was rejected by the offline verifier (Major); ac80281 run claimed the staged evidence root was no longer reproducible (Minor, remediated Cannot-confirm with per-execution staging documented). Both concerned acceptance-evidence consumability; neither named the documentation cross-reference residue.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-pi-glm53-e079780-20260926t192024z",
            "finding\u005fid": "P19-AUD-001",
            "source\u005fid": "s2"
          },
          {
            "audit\u005frun\u005fid": "phase19-pi-glm53-ac80281-20260928t070222z",
            "finding\u005fid": "P19-AUD-001",
            "source\u005fid": "s3"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "grep for 2026-09-29 / re-collection in both language files: no match; the only registry pointer is at line 26",
            "limitations": "documentation traceability only; acceptance evidence itself verified accepted",
            "path": "docs/audits/phase19-local-native-acceptance.md",
            "sha256": "6154eddd82d516b3ecca15a3400265a3d7baca75225bd717578f61363b100381"
          },
          {
            "checked": "final record registers the material/6 re-collected root with status accepted by the current verifier; manifest sha256 matches the root this audit re-verified (9897f08f...)",
            "limitations": "registry status strings are historical claims; this audit re-executed the verifier rather than trusting them",
            "path": "docs/eval/evidence-roots.jsonl",
            "sha256": "11d41b4745959dadcbf166be0c0349cee8e672f141fd607cff60a0de892a65be"
          }
        ],
        "relationship": "new-omission",
        "repair\u005fcommits": [
          "ed945ed08a396192741cc8d59429c673e1e7e718"
        ],
        "verification": "Add one cross-reference line in docs/audits/phase19-local-native-acceptance.md and .zh.md pointing to the accepted re-collection's registry record, then python3 scripts/opi-doc-check.py."
      },
      {
        "cause": "not-a-defect row: records the verified retained fix for an old-only finding.",
        "current\u005fevidence": "runner/experiment.rs at audit\u005fhead refreshes trial identities for every re-executed trial; replacement\u005ffresh\u005fresolved\u005fpairs\u005fpreserve\u005foriginal\u005fscope passed in this audit's independent full-suite run at audit\u005fhead (assembled\u005frun binary, 15/15). The repair is retained.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "codex-3e8fee6 claimed replacement reran unrelated groups under their old trial identities (Major); remediation decision fix:fresh-trial-id-for-every-replacement-execution, verification\u005fstatus Confirmed at aec029b.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-002",
            "source\u005fid": "s4"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "replacement path dispatches retained groups and refreshes each re-executed trial's identity; daaf2cf applied the change (23-line diff in this file)",
            "limitations": "hermetic seam only; real native replacement evidence belongs to operator runs",
            "path": "crates/opi-eval/src/runner/experiment.rs",
            "sha256": "655e65df2bd7f99c5a66e7c97b4ba715ba110b83b6eac3a3690bebda6851249c"
          },
          {
            "checked": "replacement\u005ffresh\u005fresolved\u005fpairs\u005fpreserve\u005foriginal\u005fscope executed green in this audit's full-suite run",
            "limitations": "none observed",
            "path": "crates/opi-eval/tests/assembled\u005frun.rs",
            "sha256": "bea21545fcb169cf87488ab676d73677decd5e1e99de029cfdba5c5849979045"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "daaf2cfb540cb9890d930973cff725802a742a93"
        ],
        "verification": "Keep replacement\u005ffresh\u005fresolved\u005fpairs\u005fpreserve\u005foriginal\u005fscope in the workspace test union; regression would reappear as an old-identity rerun."
      },
      {
        "cause": "not-a-defect row: records the verified retained fix for an old-only finding.",
        "current\u005fevidence": "experiment.rs at audit\u005fhead rejects a required control the selected native client cannot prove effective before any effect, with reasoning named explicitly unenforceable in native execution; the experiment\u005fcontract suite (34 tests) passed in this audit's independent run, and the bilingual README documents the reject-before-effects contract. The repair is retained.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "codex-3e8fee6 claimed a required reasoning control was admitted before effects and Pi launched with thinking off (Major); remediation decision fix:reject-required-native-reasoning-before-effects, verification\u005fstatus Confirmed at aec029b.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-006",
            "source\u005fid": "s4"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "required-control admission loop rejects values the selected client cannot prove effective; daaf2cf applied a 169-line change in this file including the reasoning rule",
            "limitations": "fixture-level admission checks; live-client enforcement evidence lives in the acceptance root",
            "path": "crates/opi-eval/src/experiment.rs",
            "sha256": "b0cc4252f4701d2900098a83c408c0b5df59bf01f23c9daab311f56b59bcf952"
          },
          {
            "checked": "34/34 passed in this audit's full-suite run at audit\u005fhead",
            "limitations": "none observed",
            "path": "crates/opi-eval/tests/experiment\u005fcontract.rs",
            "sha256": "3b4797e6d68d887df21ea07c741f0f11c1b383387e3aca226028948cd2b8d2bf"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "daaf2cfb540cb9890d930973cff725802a742a93"
        ],
        "verification": "Keep the required-control admission cases in experiment\u005fcontract and the required\u005freasoning native\u005fdriver filters in the verification union."
      }
    ],
    "limitations": "- Comparison ingested only sources pinned to commits available at audit\u005fhead bc9a5c75; the two earlier superseded runs (codex d915e4a, codex e079780) were reviewed at finding-inventory level and produced no row because their findings map to repairs already covered by later confirmed dispositions or to areas this audit re-verified wholesale.\n- codex-3e8fee6 P19-AUD-011 (pinned benchmark-native offline regrade of the original operator inputs) has no established repair (no-action/Cannot-confirm in s5): the registered diagnosis records that regrading the original 2026-09-15 material/4 bundles is blocked by the /4-seal versus /6-producer image-digest identity break. Post-install lead for fresh independent verification; not a current finding of this run, whose regrade evidence is the audit\u005fhead fail-closed public-seam matrix.\n- codex-3e8fee6 P19-AUD-013's historical Cannot-confirm (six-trial inputs unavailable) was superseded by the 2026-09-29 re-collection that this audit independently re-verified (accepted:true, exit 0); recorded here because no current finding of this run carries that history.\n- This run's P19-AUD-001 was observed on a uv-less WSL host; the uv-equipped-host path was not re-executed in this audit.\n- Raw remediation-era logs (logs/current-eval-union.\u002a, logs/script-modes.\u002a) named by older dispositions were not re-read or rerun; preserved-repair rows rely on this audit's independent audit\u005fhead test execution instead.",
    "schema": "opi-audit-history/1",
    "scope": "All five superseded Phase 19 audit runs under docs/snapshots/phase19/assurance/history/ (ad67cb8, e079780, ac80281 pi-glm53; d915e4a, e079780 codex-gpt6) reviewed by finding inventory, plus the two committed remediation result records (265078f, daaf2cf) and the current codex-gpt6 member report committed at ac80281, all pinned to commits available at audit\u005fhead bc9a5c75ba74ed59b6a676023af813566300a388. Concurrent sibling output installed after audit\u005fhead was not ingested. Comparison rows cover the two current findings of this run and two old-only repairs verified by direct current test evidence; other historical findings were checked against this run's passing audit\u005fhead suite only at inventory level.",
    "sources": [
      {
        "audit\u005frun\u005fid": "phase19-pi-glm53-ad67cb8-20260918t101920z",
        "commit": "5803c8758006593efd4efeba214ae8b73c903580",
        "id": "s1",
        "path": "docs/snapshots/phase19/assurance/history/phase19-pi-glm53-ad67cb8-20260918t101920z/audit.pi.glm53.findings.jsonl",
        "sha256": "fa4d61c10efa6cb9d68146fab9f5f28eaf970393f941ba97119a1c68e5fd8301"
      },
      {
        "audit\u005frun\u005fid": "phase19-pi-glm53-e079780-20260926t192024z",
        "commit": "aec029b2226f23dd76d33cc29eeacda2ac78bc77",
        "id": "s2",
        "path": "docs/snapshots/phase19/assurance/history/phase19-pi-glm53-e079780-20260926t192024z/audit.pi.glm53.findings.jsonl",
        "sha256": "f2e7acaffe724a0fa6dbf28ca767b163a3cd664a905a17e6b17924d1fedd582c"
      },
      {
        "audit\u005frun\u005fid": "phase19-pi-glm53-ac80281-20260928t070222z",
        "commit": "aec029b2226f23dd76d33cc29eeacda2ac78bc77",
        "id": "s3",
        "path": "docs/snapshots/phase19/assurance/audit.pi.glm53.findings.jsonl",
        "sha256": "8e47b6f1d2a5d74d6514568731c03e7a4ba99f31c638de369c6125a91f7b8435"
      },
      {
        "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
        "commit": "ac802811f2f0216ceddc9d802906570d9f4d67e8",
        "id": "s4",
        "path": "docs/snapshots/phase19/assurance/audit.codex.gpt6.findings.jsonl",
        "sha256": "ccf574271b308b2bb72e13df1721824a9bda153e08b813e0679320b65ceb4ca1"
      },
      {
        "audit\u005frun\u005fid": null,
        "commit": "daaf2cfb540cb9890d930973cff725802a742a93",
        "id": "s5",
        "path": "docs/snapshots/phase19/assurance/remediation.result.dispositions.jsonl",
        "sha256": "f9d62f9eff55cb932c48129df9c2264dfb2c8fbba64176090df3f8c64ff87d14"
      },
      {
        "audit\u005frun\u005fid": null,
        "commit": "265078fd04442bc317a79175614064e3e243a7ae",
        "id": "s6",
        "path": "docs/snapshots/phase19/assurance/remediation.result.dispositions.jsonl",
        "sha256": "40c39384d0d9387c8649920005cdc2d3f9d68e73b428aa74016349692caa0696"
      }
    ],
    "status": "compared"
  },
  "core": {
    "findings\u005fsha256": "380291b2803be54445d704b50f6e9d3d63647ca7a1e37373c0d256cf56632345",
    "meta\u005fsha256": "dc15283a07e1f4a36f78277552e1832caddd4daeac3098d9b82bb0698665a1da",
    "report\u005fbytes": 16094,
    "report\u005fsha256": "f2986830d7b12a2277a16be9bfa4422ac1d504acd5faf0578f15d96dc150773a",
    "requirements\u005fsha256": "0e17540650c20c7585cb9009916af209196cb13c0336bff32c86fbcf70199e7c"
  },
  "schema": "opi-audit-history/1"
}
-->

## Post-audit historical reference — advisory

This section was appended after installation of the independent core. It cannot change findings, requirements, independence or verdict. Historical closure claims require current independent verification.

- Owning audit run: phase19&#45;pi&#45;glm53&#45;bc9a5c7&#45;20260930t231208z
- Audit head: bc9a5c75ba74ed59b6a676023af813566300a388
- Core report bytes: 16094
- Core report SHA-256: f2986830d7b12a2277a16be9bfa4422ac1d504acd5faf0578f15d96dc150773a
- Current findings SHA-256: 380291b2803be54445d704b50f6e9d3d63647ca7a1e37373c0d256cf56632345
- Comparison status: compared
- Inspected history: All five superseded Phase 19 audit runs under docs/snapshots/phase19/assurance/history/ &#40;ad67cb8, e079780, ac80281 pi&#45;glm53; d915e4a, e079780 codex&#45;gpt6&#41; reviewed by finding inventory, plus the two committed remediation result records &#40;265078f, daaf2cf&#41; and the current codex&#45;gpt6 member report committed at ac80281, all pinned to commits available at audit&#95;head bc9a5c75ba74ed59b6a676023af813566300a388&#46; Concurrent sibling output installed after audit&#95;head was not ingested&#46; Comparison rows cover the two current findings of this run and two old&#45;only repairs verified by direct current test evidence; other historical findings were checked against this run&#39;s passing audit&#95;head suite only at inventory level&#46;
- Limitations: &#45; Comparison ingested only sources pinned to commits available at audit&#95;head bc9a5c75; the two earlier superseded runs &#40;codex d915e4a, codex e079780&#41; were reviewed at finding&#45;inventory level and produced no row because their findings map to repairs already covered by later confirmed dispositions or to areas this audit re&#45;verified wholesale&#46;<br>&#45; codex&#45;3e8fee6 P19&#45;AUD&#45;011 &#40;pinned benchmark&#45;native offline regrade of the original operator inputs&#41; has no established repair &#40;no&#45;action/Cannot&#45;confirm in s5&#41;: the registered diagnosis records that regrading the original 2026&#45;09&#45;15 material/4 bundles is blocked by the /4&#45;seal versus /6&#45;producer image&#45;digest identity break&#46; Post&#45;install lead for fresh independent verification; not a current finding of this run, whose regrade evidence is the audit&#95;head fail&#45;closed public&#45;seam matrix&#46;<br>&#45; codex&#45;3e8fee6 P19&#45;AUD&#45;013&#39;s historical Cannot&#45;confirm &#40;six&#45;trial inputs unavailable&#41; was superseded by the 2026&#45;09&#45;29 re&#45;collection that this audit independently re&#45;verified &#40;accepted:true, exit 0&#41;; recorded here because no current finding of this run carries that history&#46;<br>&#45; This run&#39;s P19&#45;AUD&#45;001 was observed on a uv&#45;less WSL host; the uv&#45;equipped&#45;host path was not re&#45;executed in this audit&#46;<br>&#45; Raw remediation&#45;era logs &#40;logs/current&#45;eval&#45;union&#46;&#42;, logs/script&#45;modes&#46;&#42;&#41; named by older dispositions were not re&#45;read or rerun; preserved&#45;repair rows rely on this audit&#39;s independent audit&#95;head test execution instead&#46;

### Pinned historical sources

- s1: docs/snapshots/phase19/assurance/history/phase19&#45;pi&#45;glm53&#45;ad67cb8&#45;20260918t101920z/audit&#46;pi&#46;glm53&#46;findings&#46;jsonl; commit 5803c8758006593efd4efeba214ae8b73c903580; SHA-256 fa4d61c10efa6cb9d68146fab9f5f28eaf970393f941ba97119a1c68e5fd8301; audit run phase19&#45;pi&#45;glm53&#45;ad67cb8&#45;20260918t101920z
- s2: docs/snapshots/phase19/assurance/history/phase19&#45;pi&#45;glm53&#45;e079780&#45;20260926t192024z/audit&#46;pi&#46;glm53&#46;findings&#46;jsonl; commit aec029b2226f23dd76d33cc29eeacda2ac78bc77; SHA-256 f2e7acaffe724a0fa6dbf28ca767b163a3cd664a905a17e6b17924d1fedd582c; audit run phase19&#45;pi&#45;glm53&#45;e079780&#45;20260926t192024z
- s3: docs/snapshots/phase19/assurance/audit&#46;pi&#46;glm53&#46;findings&#46;jsonl; commit aec029b2226f23dd76d33cc29eeacda2ac78bc77; SHA-256 8e47b6f1d2a5d74d6514568731c03e7a4ba99f31c638de369c6125a91f7b8435; audit run phase19&#45;pi&#45;glm53&#45;ac80281&#45;20260928t070222z
- s4: docs/snapshots/phase19/assurance/audit&#46;codex&#46;gpt6&#46;findings&#46;jsonl; commit ac802811f2f0216ceddc9d802906570d9f4d67e8; SHA-256 ccf574271b308b2bb72e13df1721824a9bda153e08b813e0679320b65ceb4ca1; audit run phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z
- s5: docs/snapshots/phase19/assurance/remediation&#46;result&#46;dispositions&#46;jsonl; commit daaf2cfb540cb9890d930973cff725802a742a93; SHA-256 f9d62f9eff55cb932c48129df9c2264dfb2c8fbba64176090df3f8c64ff87d14; audit run not an audit report
- s6: docs/snapshots/phase19/assurance/remediation&#46;result&#46;dispositions&#46;jsonl; commit 265078fd04442bc317a79175614064e3e243a7ae; SHA-256 40c39384d0d9387c8649920005cdc2d3f9d68e73b428aa74016349692caa0696; audit run not an audit report

### Comparison 1: P19&#45;AUD&#45;001

- Relationship: new&#45;omission
- Prior claim (attributed): ad67cb8 run claimed non&#45;executable script modes broke the default suite on faithful checkouts &#40;Major&#41;; remediation record 265078f verified that claim Refuted on a fresh ext4 git archive preserving mode 0755&#46; No prior Phase 19 finding names the undeclared host uv binary dependence now observed&#46;
- Current evidence: At audit&#95;head, test&#95;native&#95;smoke&#95;output&#95;control&#46;py mocks &#96;command &#45;v uv&#96; to return the hardcoded /usr/bin/uv while the materialize&#45;configs stage in native&#45;smoke&#46;sh performs a real shutil&#46;copy2 of that path; on this audit&#39;s WSL Ubuntu host without uv, python &#45;m unittest discover produced 3 errors of 251 in MaterializeOutputControlTest&#46; The historical script&#45;modes defect is a different host&#45;brittleness branch and stays refuted; this branch is a distinct, currently present defect&#46;
- Cause / missed boundary: The unit tests were authored on a uv&#45;equipped host; the hermetic mock fabricates a host path that the embedded materializer then reads with real file I/O, so the dependence never failed for its authors&#46; Bounded inference from the mock and copy call sites; no other undeclared host reads were found in the suite&#46;
- Suggested current verification: python3 &#45;m unittest discover &#45;s crates/opi&#45;eval/scripts &#45;p &#39;test&#95;&#42;&#46;py&#39; on a host without /usr/bin/uv &#40;expect the 3 errors&#41;, then point the mock at a synthetic executable inside the test tempdir &#40;or add an explicit uv&#45;absent skip&#41; and rerun expecting 0 errors&#46;
- Prior finding: phase19&#45;pi&#45;glm53&#45;ad67cb8&#45;20260918t101920z / P19&#45;AUD&#45;001; source s1
- Candidate raw proof: crates/opi&#45;eval/scripts/test&#95;native&#95;smoke&#95;output&#95;control&#46;py; SHA-256 b6362619c612cb16156a9cc1512961c2e9f51af5a66aadc63f2a433b4b1e3e4d; checked: mock at command&#40;&#41; returns /usr/bin/uv without creating it; no skip guard exists; observed 3 FileNotFoundError/FileExistsError tracebacks in this audit&#39;s export run; limits: observed on one uv&#45;less WSL Ubuntu host; a uv&#45;equipped host was not re&#45;executed in this audit
- Candidate raw proof: crates/opi&#45;eval/scripts/native&#45;smoke&#46;sh; SHA-256 5abe7291e3cbcaf6cd5c6e1d6db5fa498e5a62dee331605bdca8d808eeddcd33; checked: materialize&#45;configs resolves uv via &#96;command &#45;v uv&#96; and copies the resolved real binary into cache/tools/uv; limits: production behavior on uv&#45;equipped hosts is unaffected; the defect is the default test suite&#39;s host dependence only

### Comparison 2: P19&#45;AUD&#45;002

- Relationship: new&#45;omission
- Prior claim (attributed): e079780 run claimed the six&#45;trial acceptance root was rejected by the offline verifier &#40;Major&#41;; ac80281 run claimed the staged evidence root was no longer reproducible &#40;Minor, remediated Cannot&#45;confirm with per&#45;execution staging documented&#41;&#46; Both concerned acceptance&#45;evidence consumability; neither named the documentation cross&#45;reference residue&#46;
- Current evidence: The e079780 substance was closed by the user&#45;authorized 2026&#45;09&#45;29 re&#45;collection registered at ed945ed0: this audit independently re&#45;verified that root with the audit&#95;head verifier and an audit&#95;head&#45;built companion &#40;accepted:true, exit 0, six trials&#41; and the Opi&#45;only artifact audit passed&#46; The residual defect is documentation&#45;only: docs/audits/phase19&#45;local&#45;native&#45;acceptance&#46;md and &#46;zh&#46;md still reference only the superseded 2026&#45;09&#45;15 root and its current&#45;verifier rejection, without a cross&#45;reference to the registry record of the currently accepted root; the recollection&#39;s owning report was removed from the checkout under the documented conclusion&#45;pair contract&#46;
- Cause / missed boundary: The bilingual acceptance audit documents were finalized before the re&#45;collection and were not revisited when the registry gained the accepted root and the conclusion&#45;pair removal landed&#46;
- Suggested current verification: Add one cross&#45;reference line in docs/audits/phase19&#45;local&#45;native&#45;acceptance&#46;md and &#46;zh&#46;md pointing to the accepted re&#45;collection&#39;s registry record, then python3 scripts/opi&#45;doc&#45;check&#46;py&#46;
- Prior finding: phase19&#45;pi&#45;glm53&#45;e079780&#45;20260926t192024z / P19&#45;AUD&#45;001; source s2
- Prior finding: phase19&#45;pi&#45;glm53&#45;ac80281&#45;20260928t070222z / P19&#45;AUD&#45;001; source s3
- Repair commits: ed945ed08a396192741cc8d59429c673e1e7e718
- Candidate raw proof: docs/audits/phase19&#45;local&#45;native&#45;acceptance&#46;md; SHA-256 6154eddd82d516b3ecca15a3400265a3d7baca75225bd717578f61363b100381; checked: grep for 2026&#45;09&#45;29 / re&#45;collection in both language files: no match; the only registry pointer is at line 26; limits: documentation traceability only; acceptance evidence itself verified accepted
- Candidate raw proof: docs/eval/evidence&#45;roots&#46;jsonl; SHA-256 11d41b4745959dadcbf166be0c0349cee8e672f141fd607cff60a0de892a65be; checked: final record registers the material/6 re&#45;collected root with status accepted by the current verifier; manifest sha256 matches the root this audit re&#45;verified &#40;9897f08f&#46;&#46;&#46;&#41;; limits: registry status strings are historical claims; this audit re&#45;executed the verifier rather than trusting them

### Comparison 3: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): codex&#45;3e8fee6 claimed replacement reran unrelated groups under their old trial identities &#40;Major&#41;; remediation decision fix:fresh&#45;trial&#45;id&#45;for&#45;every&#45;replacement&#45;execution, verification&#95;status Confirmed at aec029b&#46;
- Current evidence: runner/experiment&#46;rs at audit&#95;head refreshes trial identities for every re&#45;executed trial; replacement&#95;fresh&#95;resolved&#95;pairs&#95;preserve&#95;original&#95;scope passed in this audit&#39;s independent full&#45;suite run at audit&#95;head &#40;assembled&#95;run binary, 15/15&#41;&#46; The repair is retained&#46;
- Cause / missed boundary: not&#45;a&#45;defect row: records the verified retained fix for an old&#45;only finding&#46;
- Suggested current verification: Keep replacement&#95;fresh&#95;resolved&#95;pairs&#95;preserve&#95;original&#95;scope in the workspace test union; regression would reappear as an old&#45;identity rerun&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;002; source s4
- Repair commits: daaf2cfb540cb9890d930973cff725802a742a93
- Candidate raw proof: crates/opi&#45;eval/src/runner/experiment&#46;rs; SHA-256 655e65df2bd7f99c5a66e7c97b4ba715ba110b83b6eac3a3690bebda6851249c; checked: replacement path dispatches retained groups and refreshes each re&#45;executed trial&#39;s identity; daaf2cf applied the change &#40;23&#45;line diff in this file&#41;; limits: hermetic seam only; real native replacement evidence belongs to operator runs
- Candidate raw proof: crates/opi&#45;eval/tests/assembled&#95;run&#46;rs; SHA-256 bea21545fcb169cf87488ab676d73677decd5e1e99de029cfdba5c5849979045; checked: replacement&#95;fresh&#95;resolved&#95;pairs&#95;preserve&#95;original&#95;scope executed green in this audit&#39;s full&#45;suite run; limits: none observed

### Comparison 4: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): codex&#45;3e8fee6 claimed a required reasoning control was admitted before effects and Pi launched with thinking off &#40;Major&#41;; remediation decision fix:reject&#45;required&#45;native&#45;reasoning&#45;before&#45;effects, verification&#95;status Confirmed at aec029b&#46;
- Current evidence: experiment&#46;rs at audit&#95;head rejects a required control the selected native client cannot prove effective before any effect, with reasoning named explicitly unenforceable in native execution; the experiment&#95;contract suite &#40;34 tests&#41; passed in this audit&#39;s independent run, and the bilingual README documents the reject&#45;before&#45;effects contract&#46; The repair is retained&#46;
- Cause / missed boundary: not&#45;a&#45;defect row: records the verified retained fix for an old&#45;only finding&#46;
- Suggested current verification: Keep the required&#45;control admission cases in experiment&#95;contract and the required&#95;reasoning native&#95;driver filters in the verification union&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;006; source s4
- Repair commits: daaf2cfb540cb9890d930973cff725802a742a93
- Candidate raw proof: crates/opi&#45;eval/src/experiment&#46;rs; SHA-256 b0cc4252f4701d2900098a83c408c0b5df59bf01f23c9daab311f56b59bcf952; checked: required&#45;control admission loop rejects values the selected client cannot prove effective; daaf2cf applied a 169&#45;line change in this file including the reasoning rule; limits: fixture&#45;level admission checks; live&#45;client enforcement evidence lives in the acceptance root
- Candidate raw proof: crates/opi&#45;eval/tests/experiment&#95;contract&#46;rs; SHA-256 3b4797e6d68d887df21ea07c741f0f11c1b383387e3aca226028948cd2b8d2bf; checked: 34/34 passed in this audit&#39;s full&#45;suite run at audit&#95;head; limits: none observed
