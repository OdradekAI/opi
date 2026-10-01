# Phase 19 Audit

**Audit run ID**: `phase19-pi-glm53-ac80281-20260928t070222z`
**Audit head**: `ac802811f2f0216ceddc9d802906570d9f4d67e8`
**Reviewer ID**: `pi`
**Model ID**: `glm53`
**Reviewer identity**: pi coding agent (pi-coding-agent CLI session)
**Reviewer model ID**: `glm-5.3`
**Model identity source**: runtime-attested
**Independence**: fresh-context-same-family
**Baseline policy**: latest-committed-spec
**Verdict**: PASS-WITH-FINDINGS

Independence rationale: this audit ran in a fresh conversation with no prior
Phase-19 audit, remediation, history, or sibling-peer content in context; the
runtime-attested model identity is `glm-5.3` (`PI_MODEL`, provider
`zai-coding-cn`), selected by the operator, never by this workflow. All
evidence derives from the sealed `git archive` export of the audit head plus
the operator's durable local run roots.

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| `.opi-impl-state.json` | `c65c10635c4bb8b1754625b94e8a0a15459eef6d65634610ba9b65ea9ba5337e` | current committed root ledger (read via the scope reader projection only) |
| `docs/snapshots/phase19/opi-impl-state.json` | `35b3378bd902cc976bb348d794dd1b5d3c5f160a58769c3aeb62fea175a7b7a9` | pointed Phase-19 snapshot ledger (projection only) |
| `docs/opi-spec.md` | `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db` | normative parent |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | registered Phase-19 source (English authority) |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.zh.md` | `d41c826412b0d71802327d60f953708f6cb2083436a2a2b0d99a2e52d851f194` | registered Chinese counterpart |

Stored hash mismatches: none. The reader resolved HEAD once; HEAD equals the
audit head and the shared worktree was clean at admission (rotation PASS).

## Requirement Conformance

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| P19-AC-002 | Phase spec §7 row P19-AC-002 | `bound_native_identity` (fresh-job staleness snapshot, single-trial/single-eval enforcement, agent-name match on result and eval key, task match, oracle exclusion); `native_result_binding_rejects_every_substitution` green over 7 substitution modes via the built binary | met | |
| P19-AC-003-native | §7 P19-AC-003 native witness | live sealed TB2.1 trials graded through the production handoff (shared mode); witness test present but its pinned material is absent locally | met | |
| P19-AC-008-outcome | §7 P19-AC-008 settlement | `classify_native_trial` taxonomy; `native_trial_outcome_classifies_every_terminal_shape` green; live native zeros sealed as graded outcomes | met | |
| P19-AC-006-fixture | §7 P19-AC-006 fixture | `project_connection` (credential matched to declared reference, no ambient fallback); agent conformance matrix green for both adapters incl. typed terminal failures; real-client ignored matrix not runnable locally | met | |
| P19-AC-010-credentials | §7 P19-AC-010 credentials | `native_settlement_removes_actual_credentials` green (env+file backings, config/handoff removal, no credential in identity); authority_boundaries green | met | |
| P19-AC-004-native | §7 P19-AC-004 native witness | live TB3 trials seal separate-verifier mode with collected declared artifacts; witness test present, material absent | met | |
| P19-AC-005-native | §7 P19-AC-005 native witness | live DeepSWE trials bind base revision and committed patch in the pristine verifier; witness test present, material absent | met | |
| P19-AC-001-prepare | §7 P19-AC-001 prepare | `independent_directory_prepare*` and doctor tests green; sealed integrity records bind corpus/grader/upstream digests of the materials that produced the six trials | met | |
| P19-AC-007-material | §7 P19-AC-007 material | `local_material_integrity` and post-materialization drift rejection green | met | |
| P19-AC-007-selection | §7 P19-AC-007 selection | `full_inventory_preflight...` green (inventory, unknown reasons, counts); `frozen_subset_execution` green | met | |
| P19-AC-001-plan | §7 P19-AC-001 plan | `independent_plan_run` green; plan digest/manifest/trial re-verification reviewed in `run_experiment` | met | |
| P19-AC-008 | §7 P19-AC-008 reports | independent reconstruction of all three live families: metrics with definitions/denominators, coverage counts, comparable pairs, published outcome; provider-failure semantics covered by default tests | met | |
| P19-AC-009-verify | §7 P19-AC-009 verify | live seals verified; tamper → exit 1 digest-mismatch; byte-identical reruns; output containment (`create_new`, inside-root rejection); retained-attempt roots fail closed | met | P19-AUD-002 |
| P19-AC-009-regrade | §7 P19-AC-009 regrade | `offline_regrade` green (typed exit-2, no records, source unchanged); missing-material regrade verified fail-closed live; full native witness needs absent material | met | |
| P19-AC-010-lifecycle | §7 P19-AC-010 lifecycle | root-claim rejection, cooperative signal cancellation, `OwnedCleanup` exact-root authorization; race/reopen tests green; live roots preserve durable failed attempts with fresh replacement identities | met | |
| P19-AC-011 | §7 P19-AC-011 | dummy-credential isolation test green; 238 Python regression tests OK; both workflows call the same local scripts | met | |
| P19-AC-013 | §7 P19-AC-013 | no internal dependencies; pi-only test green; package excludes corpora/graders/images; bilingual README and Unreleased entries synchronized; doc-check PASS | met | |
| P19-AC-001-live | §7 P19-AC-001 live | three family run roots independently verified (seals + deterministic reconstruction); preserved run-report digests match the acceptance doc exactly | met | P19-AUD-001 |
| P19-AC-003-live | §7 P19-AC-003 live | both sealed TB2.1 trials bind product pi/opi, `zhipu:glm-5.3-flash`, delivered artifact digests, shared verifier, rewards 1.0/1.0 | met | P19-AUD-001 |
| P19-AC-004-live | §7 P19-AC-004 live | both sealed TB3 trials carry separate verifier mode and collected declared artifacts; native zeros retained | met | P19-AUD-001 |
| P19-AC-005-live | §7 P19-AC-005 live | both sealed DeepSWE trials bind base revision and committed patch; native zeros retained | met | P19-AUD-001 |
| P19-AC-006-live | §7 P19-AC-006 live | sealed output-control witness in every final bundle; agent records bind model and executable digests matching the acceptance doc | met | P19-AUD-001 |
| P19-AC-012 | §7 P19-AC-012 | six attributable native trials verified end-to-end offline; workspace union green at the audit head; staged acceptance-root rerun and Opi-only scoped view unavailable (see P19-AUD-001) | met | P19-AUD-001, P19-AUD-002 |
| P19-STD-001 | §8 verification union | fmt/clippy/all-target tests/doc tests/cargo doc/doc-check/diff-check green in the sealed export; one `.git`-dependent test verified in the live tree at the identical clean audit head | met | |
| P19-MCC-001 | AGENTS.md smallest-coherent-change via ledger inference notes | recorded reuse/surface/ceiling fields verified against current code; no duplicated mechanism, speculative seam, production TODO, or residual glue found | met | |

## Standards Review

- Workspace gates at the sealed export: `cargo fmt --check --all` exit 0;
  `cargo clippy --workspace --all-targets -- -D warnings` exit 0;
  `cargo test --workspace --all-targets` green (opi-eval: 362 passed, 11
  ignored; all other crates green) except
  `artifact_audit_accepts_real_declared_commit_objects`, which performs
  `git -C <workspace> cat-file` lookups and cannot resolve commit objects in
  a `.git`-less `git archive` export; it passes in the live tree at the
  identical clean audit-head commit (evidence recorded in P19-STD-001).
  `cargo test --workspace --doc` exit 0; `RUSTDOCFLAGS="-D warnings"
  cargo doc --workspace --no-deps` exit 0; `python scripts/opi-doc-check.py`
  PASS; `git diff --check` clean.
- Python regression suite `python -m unittest discover -s
  crates/opi-eval/scripts -p 'test_*.py'`: 238 tests OK, 1 skipped.
- Dependency direction: `crates/opi-eval/Cargo.toml` has no internal
  workspace dependencies (normal, dev, build, optional, or target-specific);
  task-owned paths across all 11 tasks are confined to the Companion, its two
  GitHub workflows, `CHANGELOG.md`, and `docs/audits/`.
- `crates/opi-eval/src/lib.rs` documents the crate-private module layout and
  the single documented `unsafe` FFI home (`process::tree`) under
  `#![deny(unsafe_code)]`.
- Documentation lockstep: bilingual READMEs document the full
  doctor/prepare/plan/run/verify/report/regrade surface consistently;
  generated `--help` matches; Unreleased entries record the deliberate 0.x
  schema cutover (native-material/5, run-plan/2, plan-preview/2,
  normalized-report/2, reporter/2, native-regrade/2, regrade-record/2,
  owned-resources/2, native-handoff/3, agent-terminal preflight).

## Spec Review

- AC-002/AC-008-outcome: the production result reader and the single
  settlement classifier at the `run_trial` join point implement the exact
  DoD taxonomy; the discriminating tests drive the built binary and assert
  typed failure kinds, boundaries, and sealed-bundle absence on rejection.
- AC-003/004/005 (native and live): the family handoffs are bound in
  `NativeTrialBinding` (environment mode, collected artifact digests, base
  revision for Pier); the six live trials' sealed bundles independently
  confirm shared/separate/pristine verifier semantics with per-trial
  attribution.
- AC-006/AC-010: connection projection consumes only the declared credential
  reference inside the supervised launch boundary; settlement removes
  temporary credential/config material; unenforceable required bounds are
  rejected before effects.
- AC-001/AC-007: independent-directory doctor/prepare/plan/run verified by
  default tests and by the live evidence chain (sealed integrity records
  carry the material identities that produced the trials).
- AC-008/AC-009: reports reconstructed deterministically from the live roots
  with full metric/coverage semantics; verify/regrade fail closed on tamper,
  missing material, and non-native roots; output containment enforced.
- AC-011/AC-013: dummy-credential isolation, credential-free defaults, thin
  CI callers, independent placement, and synchronized documentation all
  verified.
- AC-012: six real attributable native trials (one per family per agent over
  `zhipu:glm-5.3-flash`), retained failed attempts, replacement identities,
  and reproducible reports verified directly from the durable run roots; the
  staged acceptance-evidence rerun is not reproducible locally (P19-AUD-001).

## Security, Invariants, Integration, Test Quality, and Residuals

- Security/authority: no credential value appears in any exportable artifact
  (canary scan enforced at seal and report); the acceptance verifier strips
  credential-marked environment variables before invoking the companion;
  handoff identity records exclude `agent_env`.
- Invariants: bundle sealing is content-addressed and mutation-rejecting
  (verified live with a flipped byte); unsealed retained attempts stay
  visible in denominators rather than being silently skipped; effect-unknown
  states are never reclassified.
- Integration: root-claim admission (`create_new` run-control) prevents
  concurrent/repeated execution into one root; replacement requires a fresh
  root at the current code (legacy same-root replacements fail closed in
  reconstruction — P19-AUD-002).
- Test quality: the reviewed acceptance tests assert typed classifications,
  pair comparability, sealed-bundle presence/absence, and byte-level
  identity rather than construction success; ignored tests are explicitly
  gated on external material with documented env contracts.
- Residuals: no production `TODO`/`FIXME`/dead branches found in
  `crates/opi-eval/src`; the only ignored-test population is materially
  justified; no duplicated scoring/materialization policy found in the
  workflows.

## Minimum-change Conformance

All 11 admitted tasks carry complete `reuse_search`, `surface_necessity`,
and `simplification_ceiling` records. Spot-checks at the audit head found
every recorded reused surface present with the described behavior
(`run_trial` join point, `NativeTrialBinding`, `replace_group`,
`ReportBuilder`, `OfflineRegrader`, `OwnedCleanup`, `project_connection`,
plan freeze/verify). Introduced public seams trace to production consumers
(the CLI facade) with discriminating tests. Status per task: **conforming**
(19.1–19.11).

## Findings

### P19-AUD-001: Staged local-acceptance evidence root is no longer reproducible

- Axis: standards
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: P19-AC-001-live, P19-AC-003-live, P19-AC-004-live,
  P19-AC-005-live, P19-AC-006-live, P19-AC-012
- Claim: the redacted staged evidence root consumed by
  `verify-local-acceptance.py` and `scripts/opi-artifact-audit.py` is absent
  from this machine, so the acceptance criteria's named offline verification
  commands cannot be re-run; only the operator's primary run roots remain.
- Evidence: bounded search found no `phase19-operator/evidence-root` or
  `opi-artifact-audit` view; the primary run roots remain and their preserved
  run-report digests match `docs/audits/phase19-local-native-acceptance.md`
  exactly.
- Refutation attempted: the six trials were re-verified independently
  through the audit-head companion (seals, deterministic reports, per-trial
  attribution), which substantiates the acceptance substance; the missing
  artifact is the staged copy and scoped view, not the primary evidence, so
  the claim is retained as an evidence-reproducibility gap only.
- Suggested closure: re-stage the redacted evidence root (manifest, scoped
  Opi-only view, upstream identity cache) from the retained primary roots and
  re-run the two verifier scripts, recording the new output; or record the
  retention decision explicitly.

### P19-AUD-002: Durable roots retaining superseded attempts cannot publish report reconstruction

- Axis: integration
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: P19-AC-009-verify, P19-AC-012
- Claim: run roots that retain superseded sealed or unsealed attempts — the
  layout the acceptance runs used before the 2026-09-19 fresh-root
  enforcement and documented as "retained failures included" — fail `report`
  with control-drift/undeclared-trial/run-control-drift (exit 1), and
  `verify` lists unsealed retained attempts as io failures under outcome
  `mutation-detected`; reconstructing the documented six-trial reports
  requires an undocumented per-execution staging step.
- Evidence: whole-root report runs exit 1 on all three operator roots;
  filtered roots holding exactly each `run-control.json` planned set verify
  and publish with outcomes matching the preserved run reports;
  `report.rs` enforces one sealed experiment contract per root;
  `regrade.rs` surfaces unsealed attempts as io failures.
- Refutation attempted: the rule is deliberate fail-closed behavior
  (never-silently-normalized mixing; EVAL-EXP-006 denominator honesty), the
  current code rejects execution into evidence-bearing roots so new evidence
  cannot create this layout, and per-execution reconstruction is fully
  reproducible; the defect reduces to an undocumented operator staging
  requirement, hence Minor.
- Suggested closure: document the per-execution staging step for
  retained-attempt roots in the Companion README/acceptance handoff (or teach
  `report` to select the run-control-declared execution explicitly).

## Verification Commands

| Command | Result | Requirement/finding |
|---|---|---|
| `python3 .agents/skills/_shared/scripts/validate_assurance_artifact.py rotation docs/snapshots/phase19` | PASS | admission |
| `python3 .agents/skills/opi-audit/scripts/read_audit_scope.py --phase 19` | PASS (no hash mismatches) | baseline seal |
| `cargo test -p opi-eval --all-targets` (export) | PASS (362 ok, 11 ignored) | AC-002/006/007/008/009/010/011/013 |
| `cargo test --workspace --all-targets --exclude opi-eval` (export, `--no-fail-fast`) | PASS except one `.git`-dependent test; that test PASS in live tree at audit head | P19-STD-001 |
| `cargo fmt --check --all` (export) | PASS | P19-STD-001 |
| `cargo clippy --workspace --all-targets -- -D warnings` (export) | PASS | P19-STD-001 |
| `cargo test --workspace --doc` (export) | PASS | P19-STD-001 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` (export) | PASS | P19-STD-001 |
| `python3 scripts/opi-doc-check.py`; `git diff --check` | PASS / clean | P19-STD-001, AC-013 |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py'` (export) | PASS (238 ok, 1 skipped) | AC-011 |
| `cargo package -p opi-eval --list --allow-dirty` | contents confined to fixtures/integrations/profiles/locks | AC-013 |
| built-binary `verify`/`report` on live tb21/tb30/deepswe roots | finals verify+publish; whole roots fail closed (mixed contracts) | AC-009-verify, P19-AUD-002 |
| built-binary tamper test (flipped byte) | verify exit 1 digest-mismatch; report exit 1 | AC-009-verify |
| built-binary report determinism rerun | byte-identical SHA-256 | AC-009-verify |
| built-binary `regrade` missing material | exit 2, no effects | AC-009-regrade |
| ignored native witness/client/cancellation tests | NOT RUN (pinned external material absent locally) | AC-003/004/005-native, AC-006-fixture, AC-008, AC-009-regrade, AC-010 limitations |
| `verify-local-acceptance.py` / `opi-artifact-audit.py` reruns | NOT RUN (staged evidence root absent) | P19-AUD-001 |

## Verdict Rationale

Every sealed mandatory requirement is met with current audit-head evidence or
an explicit recorded limitation. Two advisory Minor findings remain
(evidence-root reproducibility; retained-attempt root reconstruction), so the
mechanical member verdict is PASS-WITH-FINDINGS. The limitations that could
not be exercised locally (ignored native witness tests requiring pinned
external material; the staged acceptance-evidence rerun) are covered by
stronger live sealed evidence for the same production paths, and are recorded
as check-level limitations rather than requirement gaps.

<!-- opi-audit-history/1
{
  "audit\u005fhead": "ac802811f2f0216ceddc9d802906570d9f4d67e8",
  "audit\u005frun\u005fid": "phase19-pi-glm53-ac80281-20260928t070222z",
  "comparison": {
    "comparisons": [
      {
        "cause": "Operator-side artifact retention: the acceptance workflow never required keeping the staged redacted copy after verification; only the primary run roots remain.",
        "current\u005fevidence": "The final operator runs (2026-09-15) sealed six attributable trials and docs/audits/phase19-local-native-acceptance.md records accepted:true with an honest manifest correction; independently at audit\u005fhead the primary run roots verify and their preserved run-report digests match the doc exactly. The prior defect is gone; the new gap is that the staged evidence root itself was deleted from the machine, so the named offline verifier and Opi-only scoped-view reruns are impossible.",
        "current\u005ffinding\u005fid": "P19-AUD-001",
        "prior\u005fclaim": "At e079780 the supplied redacted acceptance root fresh-portable/ failed verify-local-acceptance.py with fewer than six valid trials; disposition 'Deferred by registered source' on acceptance.six-real-native-trials awaited the operator's final runs.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-pi-glm53-e079780-20260926t192024z",
            "finding\u005fid": "P19-AUD-001",
            "source\u005fid": "S1"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "digest matches docs/audits/phase19-local-native-acceptance.md; all six final bundles verify through the audit-head companion",
            "limitations": "private operator path outside the repository; the staged root bytes are unavailable so the verifier rerun claim rests on the recorded doc plus independently verified primary evidence",
            "path": "/home/luiz/.cache/opi/phase19-setup-repair/runs/tb21/run-report.json",
            "sha256": "15bb5a4bb50498f6784201b4db9054d00bf0ad75376747d8162ce8013f31ca61"
          }
        ],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Re-stage the redacted evidence root (manifest, scoped opi-artifact-audit view, upstream identity cache) from the retained primary roots, rerun verify-local-acceptance.py and scripts/opi-artifact-audit.py, and record the fresh outputs."
      },
      {
        "cause": "The fresh-root replacement repair deliberately preserved prior attempts durably while report kept enforcing one sealed experiment contract per root; the interaction leaves legacy retained-attempt roots unpublished without a documented staging step.",
        "current\u005fevidence": "At audit\u005fhead recovery is read-only (recovery\u005freport returns before any durable write; run-control identity checked), replacement requires a fresh root (execution into an evidence-bearing root rejects), and replacement identities derive from a time/pid/root token. The repairs hold. The new gap: durable roots created before the fresh-root enforcement - the documented acceptance layout with retained failed attempts - fail whole-root report reconstruction (control-drift/undeclared-trial) and require an undocumented per-execution staging copy.",
        "current\u005ffinding\u005fid": "P19-AUD-002",
        "prior\u005fclaim": "Recovery overwrote the original run report losing trial detail (005), and replacement mis-resolved frozen-selection trials while reusing fixed .r2 identities across roots (006).",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-d915e4a-20260918t084558z",
            "finding\u005fid": "P19-AUD-005",
            "source\u005fid": "S3"
          },
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-d915e4a-20260918t084558z",
            "finding\u005fid": "P19-AUD-006",
            "source\u005fid": "S3"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "one-contract-per-root rule (EVAL-EXP-006) reviewed at audit\u005fhead export bytes",
            "limitations": "behavioral evidence is the built-binary run outcomes recorded in the finding, not these bytes alone",
            "path": "crates/opi-eval/src/report.rs",
            "sha256": "1ef6ed7fe21c4a4718302315a6b439ec742bd406352b6950e3b152b8e40b7073"
          },
          {
            "checked": "root-claim rejection and read-only recovery path reviewed at audit\u005fhead export bytes",
            "limitations": "none beyond the recorded checks at audit\u005fhead",
            "path": "crates/opi-eval/src/runner/experiment.rs",
            "sha256": "550e89d5ba14f2191c3bbc6c12a2e80b7a0e325eb8f97079710d867d20fdc2d6"
          }
        ],
        "relationship": "new-omission",
        "repair\u005fcommits": [
          "2e2bd806d25e95a722b72ca7f4482753cb6d048b"
        ],
        "verification": "Either document the per-execution staging step for retained-attempt roots in the Companion README/acceptance handoff, or extend report to reconstruct the run-control-declared execution explicitly; add a regression test that publishes from a root containing one sealed superseded attempt."
      },
      {
        "cause": "Closure offline-verify.trial-link-rejection per the committed remediation result (S4).",
        "current\u005fevidence": "read\u005ftrial\u005fids rejects symlink and Windows reparse-point trial entries with InvalidData before verification; live verify runs enumerated every durable trial including unsealed attempts (io failures surfaced, none silently skipped).",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Replacing a sealed trial directory with a symlink let verify silently drop it and still return verified/exit 0.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-e079780-20260926t093335z",
            "finding\u005fid": "P19-AUD-003",
            "source\u005fid": "S2"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "symlink/reparse rejection code present at audit\u005fhead export bytes",
            "limitations": "none beyond the recorded checks at audit\u005fhead",
            "path": "crates/opi-eval/src/regrade.rs",
            "sha256": "6ef4e0c028932f14494f68e88b1de3888c812d6ddac2f510afb84a34c9eacb28"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "a7961baeea280a50df9c0ad86a8bed9d56078be0"
        ],
        "verification": "Retain the link-rejection invariant; bundle\u005frecompute regression keeps covering it."
      },
      {
        "cause": "Closure native-settlement.loader-failure-unscored per the committed remediation result (S4), with changed paths across integrations, benchmark process, experiment and verifier scripts.",
        "current\u005fevidence": "The integrations now run an exact staged-runtime --version preflight (opi-eval-native-agent-terminal/2) before configuration projection; failed or unknown preflight prevents dispatch and stays attempted/invalid/unscored, and classify\u005fnative\u005ftrial keeps binding-absent completions out of the graded class. Unreleased records the cutover.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "An Opi loader failure (glibc) before Agent start was classified as graded-native reward=0, polluting the valid denominator.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-e079780-20260926t093335z",
            "finding\u005fid": "P19-AUD-008",
            "source\u005fid": "S2"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "preflight and classifier paths reviewed at audit\u005fhead; native\u005fruntime\u005fpreflight\u005frejects\u005fsubstituted\u005fproof test green in the export run",
            "limitations": "none beyond the recorded checks at audit\u005fhead",
            "path": "crates/opi-eval/src/runner/experiment.rs",
            "sha256": "550e89d5ba14f2191c3bbc6c12a2e80b7a0e325eb8f97079710d867d20fdc2d6"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "a7961baeea280a50df9c0ad86a8bed9d56078be0"
        ],
        "verification": "Keep the runtime-preflight proof as an admission requirement for fresh native imports; the sealed acceptance bundles carry the pinned musl Opi build."
      },
      {
        "cause": "Remediation cycle closures per the committed remediation result (S4).",
        "current\u005fevidence": "Closure native-evidence.complete-within-artifact-bound redefined the contract: native evidence must complete within the artifact bound, and the six real acceptance trials sealed completely under the frozen 1 MiB caps (all six bundles verify and reconstruct at audit\u005fhead).",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "The pinned 1 MiB agent stdout/stderr capture ceiling would invalidate real TB3/DeepSWE trials (incomplete capture fail-closed to invalid, or truncated streams).",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-pi-glm53-e079780-20260926t192024z",
            "finding\u005fid": "P19-AUD-002",
            "source\u005fid": "S1"
          },
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-e079780-20260926t093335z",
            "finding\u005fid": "P19-AUD-010",
            "source\u005fid": "S2"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "both TB3 trials sealed and graded under the frozen caps; bundles verify through the audit-head companion",
            "limitations": "private operator path; caps remain pinned at 1 MiB in profiles/agents/\u002a.toml, so the bound holds per the closure predicate rather than being raised",
            "path": "/home/luiz/.cache/opi/phase19-setup-repair/runs/tb30/run-report.json",
            "sha256": "7ea5abb767d3632f4bb392c80fa4f3458eff30b80279fdc7f4e56ab1772db39a"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "265078f5e9a946338a67ac8ab1c50cbec3d23f0a",
          "a7961baeea280a50df9c0ad86a8bed9d56078be0"
        ],
        "verification": "Any future task whose streams exceed the bound must fail closed rather than truncate; keep complete-within-artifact-bound as the admission predicate."
      },
      {
        "cause": "Recovery became a read-only classification; report writes moved behind explicit report/regrade operations with output containment.",
        "current\u005fevidence": "At audit\u005fhead the recover branch returns recovery\u005freport before any durable write; the operator roots' preserved run-report.json bytes are intact after multiple recovery/replacement operations (digests match the acceptance doc).",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "run --recover overwrote run-report.json, clearing trial detail and marking completed runs incomplete.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-d915e4a-20260918t084558z",
            "finding\u005fid": "P19-AUD-005",
            "source\u005fid": "S3"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "preserved bytes intact after the retained failure/replacement history; matches the acceptance doc digest",
            "limitations": "private operator path",
            "path": "/home/luiz/.cache/opi/phase19-setup-repair/runs/deepswe/run-report.json",
            "sha256": "674b198cb77ba7c01c723c719f4cc66cec2210f97c4ee1e7d34066ad0d08e5d2"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "2e2bd806d25e95a722b72ca7f4482753cb6d048b"
        ],
        "verification": "Keep recovery read-only; any future recovery-side write must reintroduce explicit review."
      }
    ],
    "limitations": "Exact per-finding code commits beyond the pinned remediation-cycle commits (265078f, a7961ba) and the recover-era commit 2e2bd80 were not individually attributed; the superseded pi run at ad67cb8 and codex run at d915e4a items outside the compared rows are covered by their successors and were not separately re-derived; the concurrent live sibling member (codex/gpt6, 2026-09-28T05:02Z) was excluded as concurrent output; historical remediation red/green logs referenced by S4 live in operator-private temp paths and were not re-read.",
    "schema": "opi-audit-history/1",
    "scope": "Phase-19 same-Phase audit history pinned to commits available at audit\u005fhead ac802811f2f0216ceddc9d802906570d9f4d67e8: the four superseded history runs (two sealed at d915e4a22509c0f60c2651dbccfd054f0abc73d4, two at e079780bda6681ec466f79bd15f3248baf0f2eb8, both committed ancestors) and the committed remediation plan/result pair at a7961baeea280a50df9c0ad86a8bed9d56078be0. The live sibling member (codex/gpt6 sealed 2026-09-28T05:02Z) is concurrent output and excluded.",
    "sources": [
      {
        "audit\u005frun\u005fid": "phase19-pi-glm53-e079780-20260926t192024z",
        "commit": "e079780bda6681ec466f79bd15f3248baf0f2eb8",
        "id": "S1",
        "path": "docs/snapshots/phase19/assurance/history/phase19-pi-glm53-e079780-20260926t192024z/audit.pi.glm53.findings.jsonl",
        "sha256": "f2e7acaffe724a0fa6dbf28ca767b163a3cd664a905a17e6b17924d1fedd582c"
      },
      {
        "audit\u005frun\u005fid": "phase19-codex-gpt6-e079780-20260926t093335z",
        "commit": "e079780bda6681ec466f79bd15f3248baf0f2eb8",
        "id": "S2",
        "path": "docs/snapshots/phase19/assurance/history/phase19-codex-gpt6-e079780-20260926t093335z/audit.codex.gpt6.findings.jsonl",
        "sha256": "7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8"
      },
      {
        "audit\u005frun\u005fid": "phase19-codex-gpt6-d915e4a-20260918t084558z",
        "commit": "d915e4a22509c0f60c2651dbccfd054f0abc73d4",
        "id": "S3",
        "path": "docs/snapshots/phase19/assurance/history/phase19-codex-gpt6-d915e4a-20260918t084558z/audit.codex.gpt6.findings.jsonl",
        "sha256": "01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac"
      },
      {
        "audit\u005frun\u005fid": null,
        "commit": "a7961baeea280a50df9c0ad86a8bed9d56078be0",
        "id": "S4",
        "path": "docs/snapshots/phase19/assurance/remediation.result.dispositions.jsonl",
        "sha256": "a6cadab79fe2c77bdfddd4303dd4894afbc482d50bd9072aa59132bcc9c63918"
      }
    ],
    "status": "compared"
  },
  "core": {
    "findings\u005fsha256": "8e47b6f1d2a5d74d6514568731c03e7a4ba99f31c638de369c6125a91f7b8435",
    "meta\u005fsha256": "37203cbc4caa04822e61589fb112858afbbb595b80a66c5bba7e9378088a2978",
    "report\u005fbytes": 18409,
    "report\u005fsha256": "1a1b57f107772b19a8e674e312e4f83e861c6f9883f9708334e4b0b2dc944dd5",
    "requirements\u005fsha256": "ba9ef009c749e312b0fd522f6765d2447a457890deae8fc6733a5b280d8e5fd8"
  },
  "schema": "opi-audit-history/1"
}
-->

## Post-audit historical reference — advisory

This section was appended after installation of the independent core. It cannot change findings, requirements, independence or verdict. Historical closure claims require current independent verification.

- Owning audit run: phase19&#45;pi&#45;glm53&#45;ac80281&#45;20260928t070222z
- Audit head: ac802811f2f0216ceddc9d802906570d9f4d67e8
- Core report bytes: 18409
- Core report SHA-256: 1a1b57f107772b19a8e674e312e4f83e861c6f9883f9708334e4b0b2dc944dd5
- Current findings SHA-256: 8e47b6f1d2a5d74d6514568731c03e7a4ba99f31c638de369c6125a91f7b8435
- Comparison status: compared
- Inspected history: Phase&#45;19 same&#45;Phase audit history pinned to commits available at audit&#95;head ac802811f2f0216ceddc9d802906570d9f4d67e8: the four superseded history runs &#40;two sealed at d915e4a22509c0f60c2651dbccfd054f0abc73d4, two at e079780bda6681ec466f79bd15f3248baf0f2eb8, both committed ancestors&#41; and the committed remediation plan/result pair at a7961baeea280a50df9c0ad86a8bed9d56078be0&#46; The live sibling member &#40;codex/gpt6 sealed 2026&#45;09&#45;28T05:02Z&#41; is concurrent output and excluded&#46;
- Limitations: Exact per&#45;finding code commits beyond the pinned remediation&#45;cycle commits &#40;265078f, a7961ba&#41; and the recover&#45;era commit 2e2bd80 were not individually attributed; the superseded pi run at ad67cb8 and codex run at d915e4a items outside the compared rows are covered by their successors and were not separately re&#45;derived; the concurrent live sibling member &#40;codex/gpt6, 2026&#45;09&#45;28T05:02Z&#41; was excluded as concurrent output; historical remediation red/green logs referenced by S4 live in operator&#45;private temp paths and were not re&#45;read&#46;

### Pinned historical sources

- S1: docs/snapshots/phase19/assurance/history/phase19&#45;pi&#45;glm53&#45;e079780&#45;20260926t192024z/audit&#46;pi&#46;glm53&#46;findings&#46;jsonl; commit e079780bda6681ec466f79bd15f3248baf0f2eb8; SHA-256 f2e7acaffe724a0fa6dbf28ca767b163a3cd664a905a17e6b17924d1fedd582c; audit run phase19&#45;pi&#45;glm53&#45;e079780&#45;20260926t192024z
- S2: docs/snapshots/phase19/assurance/history/phase19&#45;codex&#45;gpt6&#45;e079780&#45;20260926t093335z/audit&#46;codex&#46;gpt6&#46;findings&#46;jsonl; commit e079780bda6681ec466f79bd15f3248baf0f2eb8; SHA-256 7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8; audit run phase19&#45;codex&#45;gpt6&#45;e079780&#45;20260926t093335z
- S3: docs/snapshots/phase19/assurance/history/phase19&#45;codex&#45;gpt6&#45;d915e4a&#45;20260918t084558z/audit&#46;codex&#46;gpt6&#46;findings&#46;jsonl; commit d915e4a22509c0f60c2651dbccfd054f0abc73d4; SHA-256 01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac; audit run phase19&#45;codex&#45;gpt6&#45;d915e4a&#45;20260918t084558z
- S4: docs/snapshots/phase19/assurance/remediation&#46;result&#46;dispositions&#46;jsonl; commit a7961baeea280a50df9c0ad86a8bed9d56078be0; SHA-256 a6cadab79fe2c77bdfddd4303dd4894afbc482d50bd9072aa59132bcc9c63918; audit run not an audit report

### Comparison 1: P19&#45;AUD&#45;001

- Relationship: new&#45;omission
- Prior claim (attributed): At e079780 the supplied redacted acceptance root fresh&#45;portable/ failed verify&#45;local&#45;acceptance&#46;py with fewer than six valid trials; disposition &#39;Deferred by registered source&#39; on acceptance&#46;six&#45;real&#45;native&#45;trials awaited the operator&#39;s final runs&#46;
- Current evidence: The final operator runs &#40;2026&#45;09&#45;15&#41; sealed six attributable trials and docs/audits/phase19&#45;local&#45;native&#45;acceptance&#46;md records accepted:true with an honest manifest correction; independently at audit&#95;head the primary run roots verify and their preserved run&#45;report digests match the doc exactly&#46; The prior defect is gone; the new gap is that the staged evidence root itself was deleted from the machine, so the named offline verifier and Opi&#45;only scoped&#45;view reruns are impossible&#46;
- Cause / missed boundary: Operator&#45;side artifact retention: the acceptance workflow never required keeping the staged redacted copy after verification; only the primary run roots remain&#46;
- Suggested current verification: Re&#45;stage the redacted evidence root &#40;manifest, scoped opi&#45;artifact&#45;audit view, upstream identity cache&#41; from the retained primary roots, rerun verify&#45;local&#45;acceptance&#46;py and scripts/opi&#45;artifact&#45;audit&#46;py, and record the fresh outputs&#46;
- Prior finding: phase19&#45;pi&#45;glm53&#45;e079780&#45;20260926t192024z / P19&#45;AUD&#45;001; source S1
- Candidate raw proof: /home/luiz/&#46;cache/opi/phase19&#45;setup&#45;repair/runs/tb21/run&#45;report&#46;json; SHA-256 15bb5a4bb50498f6784201b4db9054d00bf0ad75376747d8162ce8013f31ca61; checked: digest matches docs/audits/phase19&#45;local&#45;native&#45;acceptance&#46;md; all six final bundles verify through the audit&#45;head companion; limits: private operator path outside the repository; the staged root bytes are unavailable so the verifier rerun claim rests on the recorded doc plus independently verified primary evidence

### Comparison 2: P19&#45;AUD&#45;002

- Relationship: new&#45;omission
- Prior claim (attributed): Recovery overwrote the original run report losing trial detail &#40;005&#41;, and replacement mis&#45;resolved frozen&#45;selection trials while reusing fixed &#46;r2 identities across roots &#40;006&#41;&#46;
- Current evidence: At audit&#95;head recovery is read&#45;only &#40;recovery&#95;report returns before any durable write; run&#45;control identity checked&#41;, replacement requires a fresh root &#40;execution into an evidence&#45;bearing root rejects&#41;, and replacement identities derive from a time/pid/root token&#46; The repairs hold&#46; The new gap: durable roots created before the fresh&#45;root enforcement &#45; the documented acceptance layout with retained failed attempts &#45; fail whole&#45;root report reconstruction &#40;control&#45;drift/undeclared&#45;trial&#41; and require an undocumented per&#45;execution staging copy&#46;
- Cause / missed boundary: The fresh&#45;root replacement repair deliberately preserved prior attempts durably while report kept enforcing one sealed experiment contract per root; the interaction leaves legacy retained&#45;attempt roots unpublished without a documented staging step&#46;
- Suggested current verification: Either document the per&#45;execution staging step for retained&#45;attempt roots in the Companion README/acceptance handoff, or extend report to reconstruct the run&#45;control&#45;declared execution explicitly; add a regression test that publishes from a root containing one sealed superseded attempt&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;d915e4a&#45;20260918t084558z / P19&#45;AUD&#45;005; source S3
- Prior finding: phase19&#45;codex&#45;gpt6&#45;d915e4a&#45;20260918t084558z / P19&#45;AUD&#45;006; source S3
- Repair commits: 2e2bd806d25e95a722b72ca7f4482753cb6d048b
- Candidate raw proof: crates/opi&#45;eval/src/report&#46;rs; SHA-256 1ef6ed7fe21c4a4718302315a6b439ec742bd406352b6950e3b152b8e40b7073; checked: one&#45;contract&#45;per&#45;root rule &#40;EVAL&#45;EXP&#45;006&#41; reviewed at audit&#95;head export bytes; limits: behavioral evidence is the built&#45;binary run outcomes recorded in the finding, not these bytes alone
- Candidate raw proof: crates/opi&#45;eval/src/runner/experiment&#46;rs; SHA-256 550e89d5ba14f2191c3bbc6c12a2e80b7a0e325eb8f97079710d867d20fdc2d6; checked: root&#45;claim rejection and read&#45;only recovery path reviewed at audit&#95;head export bytes; limits: none beyond the recorded checks at audit&#95;head

### Comparison 3: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Replacing a sealed trial directory with a symlink let verify silently drop it and still return verified/exit 0&#46;
- Current evidence: read&#95;trial&#95;ids rejects symlink and Windows reparse&#45;point trial entries with InvalidData before verification; live verify runs enumerated every durable trial including unsealed attempts &#40;io failures surfaced, none silently skipped&#41;&#46;
- Cause / missed boundary: Closure offline&#45;verify&#46;trial&#45;link&#45;rejection per the committed remediation result &#40;S4&#41;&#46;
- Suggested current verification: Retain the link&#45;rejection invariant; bundle&#95;recompute regression keeps covering it&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;e079780&#45;20260926t093335z / P19&#45;AUD&#45;003; source S2
- Repair commits: a7961baeea280a50df9c0ad86a8bed9d56078be0
- Candidate raw proof: crates/opi&#45;eval/src/regrade&#46;rs; SHA-256 6ef4e0c028932f14494f68e88b1de3888c812d6ddac2f510afb84a34c9eacb28; checked: symlink/reparse rejection code present at audit&#95;head export bytes; limits: none beyond the recorded checks at audit&#95;head

### Comparison 4: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): An Opi loader failure &#40;glibc&#41; before Agent start was classified as graded&#45;native reward=0, polluting the valid denominator&#46;
- Current evidence: The integrations now run an exact staged&#45;runtime &#45;&#45;version preflight &#40;opi&#45;eval&#45;native&#45;agent&#45;terminal/2&#41; before configuration projection; failed or unknown preflight prevents dispatch and stays attempted/invalid/unscored, and classify&#95;native&#95;trial keeps binding&#45;absent completions out of the graded class&#46; Unreleased records the cutover&#46;
- Cause / missed boundary: Closure native&#45;settlement&#46;loader&#45;failure&#45;unscored per the committed remediation result &#40;S4&#41;, with changed paths across integrations, benchmark process, experiment and verifier scripts&#46;
- Suggested current verification: Keep the runtime&#45;preflight proof as an admission requirement for fresh native imports; the sealed acceptance bundles carry the pinned musl Opi build&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;e079780&#45;20260926t093335z / P19&#45;AUD&#45;008; source S2
- Repair commits: a7961baeea280a50df9c0ad86a8bed9d56078be0
- Candidate raw proof: crates/opi&#45;eval/src/runner/experiment&#46;rs; SHA-256 550e89d5ba14f2191c3bbc6c12a2e80b7a0e325eb8f97079710d867d20fdc2d6; checked: preflight and classifier paths reviewed at audit&#95;head; native&#95;runtime&#95;preflight&#95;rejects&#95;substituted&#95;proof test green in the export run; limits: none beyond the recorded checks at audit&#95;head

### Comparison 5: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): The pinned 1 MiB agent stdout/stderr capture ceiling would invalidate real TB3/DeepSWE trials &#40;incomplete capture fail&#45;closed to invalid, or truncated streams&#41;&#46;
- Current evidence: Closure native&#45;evidence&#46;complete&#45;within&#45;artifact&#45;bound redefined the contract: native evidence must complete within the artifact bound, and the six real acceptance trials sealed completely under the frozen 1 MiB caps &#40;all six bundles verify and reconstruct at audit&#95;head&#41;&#46;
- Cause / missed boundary: Remediation cycle closures per the committed remediation result &#40;S4&#41;&#46;
- Suggested current verification: Any future task whose streams exceed the bound must fail closed rather than truncate; keep complete&#45;within&#45;artifact&#45;bound as the admission predicate&#46;
- Prior finding: phase19&#45;pi&#45;glm53&#45;e079780&#45;20260926t192024z / P19&#45;AUD&#45;002; source S1
- Prior finding: phase19&#45;codex&#45;gpt6&#45;e079780&#45;20260926t093335z / P19&#45;AUD&#45;010; source S2
- Repair commits: 265078f5e9a946338a67ac8ab1c50cbec3d23f0a, a7961baeea280a50df9c0ad86a8bed9d56078be0
- Candidate raw proof: /home/luiz/&#46;cache/opi/phase19&#45;setup&#45;repair/runs/tb30/run&#45;report&#46;json; SHA-256 7ea5abb767d3632f4bb392c80fa4f3458eff30b80279fdc7f4e56ab1772db39a; checked: both TB3 trials sealed and graded under the frozen caps; bundles verify through the audit&#45;head companion; limits: private operator path; caps remain pinned at 1 MiB in profiles/agents/&#42;&#46;toml, so the bound holds per the closure predicate rather than being raised

### Comparison 6: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): run &#45;&#45;recover overwrote run&#45;report&#46;json, clearing trial detail and marking completed runs incomplete&#46;
- Current evidence: At audit&#95;head the recover branch returns recovery&#95;report before any durable write; the operator roots&#39; preserved run&#45;report&#46;json bytes are intact after multiple recovery/replacement operations &#40;digests match the acceptance doc&#41;&#46;
- Cause / missed boundary: Recovery became a read&#45;only classification; report writes moved behind explicit report/regrade operations with output containment&#46;
- Suggested current verification: Keep recovery read&#45;only; any future recovery&#45;side write must reintroduce explicit review&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;d915e4a&#45;20260918t084558z / P19&#45;AUD&#45;005; source S3
- Repair commits: 2e2bd806d25e95a722b72ca7f4482753cb6d048b
- Candidate raw proof: /home/luiz/&#46;cache/opi/phase19&#45;setup&#45;repair/runs/deepswe/run&#45;report&#46;json; SHA-256 674b198cb77ba7c01c723c719f4cc66cec2210f97c4ee1e7d34066ad0d08e5d2; checked: preserved bytes intact after the retained failure/replacement history; matches the acceptance doc digest; limits: private operator path
