# Phase 19 Audit

**Audit run ID**: `phase19-pi-glm53-9d805b4-20261004t094337z`
**Audit head**: `9d805b498a56a276b141cc0c0728ce8201dbe6f3`
**Reviewer ID**: `pi`
**Model ID**: `glm53`
**Reviewer identity**: Pi (pi coding agent harness)
**Reviewer model ID**: `glm53`
**Model identity source**: operator-declared
**Independence**: fresh-context-same-family — this peer began with no prior Phase-19 audit, remediation conclusion, history run, or sibling output in context; the baseline was sealed through the Step 2 reader projection and a `git archive` export of `audit_head`.
**Baseline policy**: latest-committed-spec
**Verdict**: FAIL

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| `.opi-impl-state.json` | `c65c10635c4bb8b1754625b94e8a0a15459eef6d65634610ba9b65ea9ba5337e` | current committed source |
| `docs/snapshots/phase19/opi-impl-state.json` | `35b3378bd902cc976bb348d794dd1b5d3c5f160a58769c3aeb62fea175a7b7a9` | current committed source |
| `docs/opi-spec.md` | `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db` | current committed source |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | current committed source |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.zh.md` | `d41c826412b0d71802327d60f953708f6cb2083436a2a2b0d99a2e52d851f194` | current committed source |

No stored-hash mismatches. All five sources were verified byte-identical in the
sealed export. HEAD equals `audit_head` at sealing time.

## Requirement Conformance

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| P19-AC-001 | §7 P19-AC-001; §4 workflow; 19.5/19.6 DoD | Auditor executed doctor/prepare from an independent /tmp directory with a pin-only cache; supervised acquisition cloned pinned corpora, installed pinned uv/Harbor, built pi v1.0.0 and the musl capture helper from audit_head, and published `opi-eval-native-material/7` with local origin and no GitHub metadata; assembled plan/run/report fixture tests green | met | |
| P19-AC-002 | §7 P19-AC-002; §5; 19.1 DoD | Production reader enforces fresh-job uniqueness, agent/eval-key/task/model/selection identity, reward domain and aggregate attribution, verifier-timeout unknown, lifecycle equality, declared-collection and delivered-bytes digest binding; fixture binding tests green; all three family witnesses bound real Harbor/Pier results (oracle=1/nop=0) | met | |
| P19-AC-003 | §7 P19-AC-003; §5 TB2.1 row | `native_tb21_artifact_witness` executed by this audit against the real pinned Harbor runner in local Docker: oracle witness 1, nop witness 0, same production handoff; live Opi/pi traces claimed but raw operator evidence unsupplied | partially-met | P19-AUD-001 |
| P19-AC-004 | §7 P19-AC-004; §5 TB3 row | `native_tb3_artifact_witness` executed: exactly the declared artifact crossed into the separate verifier, digests differ between witnesses, reward responds, canary excluded; live portion unsupplied | partially-met | P19-AUD-001 |
| P19-AC-005 | §7 P19-AC-005; §5 DeepSWE row | `native_deepswe_patch_witness` executed through pinned Pier: committed patch against declared base applied in pristine verifier, 1 vs 0, base/patch/verifier bound; wrong-base/substitution fixtures green; live portion unsupplied | partially-met | P19-AUD-001 |
| P19-AC-006 | §7 P19-AC-006; §4 config table; 19.2 DoD | `native_protocol_client_matrix` executed: both real built clients × three APIs on local scripted providers with retry/429/auth/protocol terminal shapes; turn-budget, output-control and operator-control-proof matrices executed; connection schema (credential_env/file exclusivity, mode enum, controls incl. top_p/context_window/required) enforced fail-closed; first-connection live evidence unsupplied | partially-met | P19-AUD-001 |
| P19-AC-007 | §7 P19-AC-007; §5 preflight; 19.6 DoD | Corpus enumeration parses per-task requirements without image pulls; unmet/unknown rejects freeze; plan_digest binds canonical bytes and is re-verified; doctor reports GPU unknown with reason; frozen-subset and preflight tests green | met | |
| P19-AC-008 | §7 P19-AC-008; §6; 19.7 DoD | Per-benchmark metric definitions and denominators, coverage counts (corpus/selected/planned/attempted/valid/agent_failure/invalid/unexecuted), subset/incomplete labelling, sealed-settlement validation, planned-coverage-before-effects (run-control/2), provider failure counted invalid not Agent outcome; fixture provider-failure report test green; declared native command is vacuous (P19-AUD-003) | met | P19-AUD-003 |
| P19-AC-009 | §7 P19-AC-009; §6; 19.7/19.8 DoD | verify is byte/seal-only; native regrade verifies bundles first, rejects outputs inside/aliasing the source, requires cached immutable material, grades in fresh verifier environments, persists derived records; `native_verifier_only_regrade_witness` executed for all three families (767 s) with byte-identical sources afterwards | met | |
| P19-AC-010 | §7 P19-AC-010; §4 credentials; §9 breach handling; 19.9 DoD | Canary scan over sealed bytes; 0700/0600 private credential staging; owned-cleanup removes only daemon-observed identities bound to private roots; recovery classifies from durable files only; replacement mints fresh identities; `native_owned_resource_cancellation` and `native_conformance_owned_cleanup_witness` executed and passed; supplementary deadline witness deterministically broken (P19-AUD-002) | met | P19-AUD-002, P19-AUD-004 |
| P19-AC-011 | §7 P19-AC-011; §8; 19.10 DoD | Workflows are thin callers of local native-smoke stages; offline acceptance verifier validates manifest/trials/oracle-rejection/digests/exits/preserved artifacts/scoped Opi-only view/upstream identities without Agent, credentials, or providers; 277 Python tests OK (1 opt-in Docker case skipped); dummy-credential non-activation tests green | met | |
| P19-AC-012 | §7 P19-AC-012; §8; 19.11 DoD | Deliverables exist (en+zh, required sections, no credential patterns in a bounded scan) and the offline verifier tooling is fixture-tested, but the redacted raw evidence root was not supplied; the six real trials remain unverified claims in this audit context | not-assessable | P19-AUD-001 |
| P19-AC-013 | §7 P19-AC-013; §3; §9; 19.10 DoD | No internal workspace dependencies; package list limited to src/READMEs/manifests; pi-only preparation performed by this auditor required no Opi; static lock preserves repository URL+commit for pi/Harbor/Pier; bilingual README sections parallel; doc-check PASS; Unreleased entries record the deliberate 0.x changes | met | |
| P19-VERIFY-UNION | §8 verification union | fmt, clippy (workspace, -D warnings), doc tests, cargo doc (-D warnings), doc-check all PASS at audit_head; workspace all-targets 1849 passed / 1 failed where the single failure (`artifact_audit_accepts_real_declared_commit_objects`) requires Git history absent from the history-less audit export — the referenced commit exists in the live repository; opi-eval all-targets 408/0 | met | |

## Standards Review

- **Dependency direction / placement**: `opi-eval` remains an Independent
  Companion with zero internal workspace dependencies; the Phase introduced no
  Agent Core, provider, or Opi-runtime seam (Ctrl: PRIN-003, PLACE-001,
  CTRL-007 respected at the manifest level).
- **Failure behavior**: typed `NativeBindingError` taxonomy with static
  redacted tokens; fail-closed staging (`create_private_stage`, fresh-output
  resolution in regrade); VerifierTimeoutError settles `Fact::Unknown` rather
  than a fabricated zero.
- **Documentation lockstep**: README/README.zh section structures parallel;
  the design spec and its Chinese counterpart preserve all 13 acceptance
  identifiers; `opi-doc-check.py` PASS.
- **Rust hygiene**: `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo fmt --check --all` PASS at audit_head.

## Spec Review

Every sealed requirement was traced to its current implementation surface and
exercised at the highest practical seam (spawned public CLI or library-adjacent
integration tests). Deterministic fixture evidence is complete (408 tests,
0 failures, 15 binaries; execution checker PASS on native_driver: 130
inventory / 118 executed / 12 ignored). Native-class evidence was produced by
this audit itself: three family artifact witnesses, the three-API client
protocol matrix, output-control/turn-budget/control-proof matrices, the
three-family verifier-only regrade witness, and two owned-cleanup witnesses —
all executed against the real pinned Harbor/Pier runners and the local Docker
engine with the loopback scripted provider, never a remote inference call.
The remaining gap is exclusively live/operator-class proof (P19-AC-012 and the
live halves of AC-003–AC-006), which the invocation did not supply.

## Security, Invariants, Integration, Test Quality, and Residuals

- **Security/authority**: credential references resolved only inside supervised
  agent launch; canary scan reads exactly the bytes sealing would cover;
  grader receives neither credentials nor undeclared files (TB3 undeclared
  entry is a typed binding rejection).
- **Invariants**: one settlement owner (`classify_native_trial`) — neither exit
  code nor boundary code alone determines eligibility; `NoOutcome` for
  cancelled/unstarted/effect-unknown; report consumes the sealed
  interpretation only (CTRL-005/CTRL-006 honored).
- **Integration**: workflows delegate to the same local preparation path;
  `materialize-configs` publishes receipts only after shared prepare and
  conformance succeed.
- **Test quality**: one deterministically broken native witness
  (P19-AUD-002); one vacuous declared verification command (P19-AUD-003); one
  stale ignore annotation (P19-AUD-004); one opt-in Docker Python test skipped
  by design (recorded, not a defect).
- **Residuals**: legacy `/1` run-control and material/6 read-only paths are
  explicitly retained for sealed evidence, documented in Unreleased.

## Minimum-change Conformance

- 19.1 `NativeTrialBinding` (benchmark/process.rs) and the sealed trial
  interpretation (runner/experiment.rs) exist as recorded shared decisions with
  the named consumers — **conforming**.
- 19.2 `project_connection`/`project_controls` own request identity;
  `native_opi_config`/`native_agent_env` legacy projections are gone —
  **conforming**.
- 19.5 `NativeMaterial::load` verified-local-material identity with
  prepare/doctor entry points — **conforming** (independently reproduced).
- 19.6 plan module + CLI facade as recorded; no second selection policy —
  **conforming**.
- 19.7/19.8 verify/regrade split as recorded — **conforming**.
- 19.9 run supervisor, owned cleanup, recovery/replacement — **conforming**
  on declared surfaces; the supplementary deadline witness is broken
  (P19-AUD-002).
- 19.10 CI alignment and offline acceptance verifier — **conforming**;
  19.11 deliverables exist — **not-assessable** beyond structure (P19-AUD-001).

## Findings

### P19-AUD-001: Live six-trial acceptance evidence is not independently verifiable from supplied proof

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-003, P19-AC-004, P19-AC-005, P19-AC-006, P19-AC-012
- Claim: the six real trials and live-provider portions cannot be confirmed or
  falsified because the redacted operator evidence root was not supplied.
- Evidence: acceptance deliverables contain claims only; invocation supplied
  no `evidence_root`.
- Refutation attempted: searched the committed tree for raw operator proof
  (none admissible — raw ledgers are excluded and the deliverable is not
  proof); the offline verifier's fixture tests pass, which proves tooling, not
  trials. No counter-evidence found; the gap is evidentiary, not a
  demonstrated failure.
- Suggested closure: supply the redacted evidence root and run
  `verify-local-acceptance.py` against it in a remediation/verification
  context.

### P19-AUD-002: remediation_native_stage_deadlines asserts a stderr marker no producer emits

- Axis: test-quality
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: P19-AC-010
- Claim: the ignored native witness fails deterministically because commit
  `0fa995d` removed the diagnostic client's `stage-witness-start` stderr
  emission without updating the deadline test's assertion.
- Evidence: `native_driver.rs:11021-11024`; `diagnostic_client_script` writes
  no stderr by design; `git log -S` shows the producer removed in `0fa995d`.
- Refutation attempted: full-tree search for any producer of the marker
  (none); reproduced twice on an idle host (fails at terminal-bench-2.1/
  positive, "Agent never started"); the declared AC-010 verification commands
  all pass, so the core criterion stands on other evidence.
- Suggested closure: update the deadline witness to observe agent start
  through an admissible capture vocabulary event (or reintroduce a compliant
  marker) and re-run with three-family material.

### P19-AUD-003: Declared AC-008 native verification command executes zero tests

- Axis: test-quality
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: P19-AC-008
- Claim: task 19.7's registered command `native_provider_failure_report --
  --ignored --exact` selects zero tests; the intended exact-client witness is
  `native_client_provider_failure_report`.
- Evidence: ledger task 19.7 verification[3]; test names at
  `native_driver.rs:7063` (non-ignored) and `:7140` (ignored, operator-pinned).
- Refutation attempted: the fixture twin passed in-suite, covering the
  denominator semantics; the vacuity affects the recorded command, not the
  behavior.
- Suggested closure: correct the ledger declaration or rename the exact-client
  witness so the declared filter executes it.

### P19-AUD-004: Stale ignore annotation on native_conformance_owned_cleanup_witness

- Axis: residuals
- Severity: Info
- Conformance effect: advisory
- Requirement IDs: P19-AC-010
- Claim: the ignore text references superseded material/6 while the test
  consumes material/7 and passes with it.
- Evidence: `native_driver.rs:3030`; auditor execution passed in 120.68 s with
  material/7.
- Suggested closure: update the annotation.

## Verification Commands

| Command | Result | Requirement/finding |
|---|---|---|
| `python3 .agents/skills/_shared/scripts/validate_assurance_artifact.py rotation docs/snapshots/phase19` | PASS | workflow admission |
| `python3 .agents/skills/opi-audit/scripts/read_audit_scope.py --phase 19` | PASS (baseline sealed, no mismatches) | workflow |
| `cargo test -p opi-eval --all-targets` (export) | PASS — 408 passed / 0 failed / 15 binaries | AC-001..013 fixture evidence |
| `cargo test -p opi-eval --test native_driver` (per-target) + checker | PASS — 118/0/12; checker PASS | anti-vacuity |
| `native_tb21_artifact_witness -- --ignored --exact` | PASS (84 s) | P19-AC-003 |
| `native_tb3_artifact_witness -- --ignored --exact` | PASS (187 s) | P19-AC-004 |
| `native_deepswe_patch_witness -- --ignored --exact` | PASS (85 s) | P19-AC-005 |
| `native_protocol_client_matrix -- --ignored --exact` | PASS (151 s) | P19-AC-006 |
| `native_client_turn_budgets -- --ignored --exact` | PASS | P19-AC-006 |
| `native_output_control_client_matrix -- --ignored --exact` | PASS | P19-AC-006 |
| `native_operator_control_proof_validates_exact_clients -- --ignored --exact` | PASS | P19-AC-006 |
| `native_verifier_only_regrade_witness -- --ignored --exact` | PASS (767 s) | P19-AC-009 |
| `native_owned_resource_cancellation -- --ignored --exact` | PASS | P19-AC-010 |
| `native_conformance_owned_cleanup_witness -- --ignored --exact` | PASS (121 s) | P19-AC-010 |
| `remediation_native_stage_deadlines -- --ignored --exact` | **FAIL** (reproduced ×2) | P19-AUD-002 |
| `native_client_provider_failure_report -- --ignored --exact` | NOT RUN — pins operator's historical client identities | P19-AUD-001/003 context |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py'` | OK — 277 tests (1 opt-in skip) | P19-AC-011 |
| `python3 scripts/opi-doc-check.py` | PASS | P19-AC-013 |
| `cargo fmt --check --all` | PASS | P19-VERIFY-UNION |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | P19-VERIFY-UNION |
| `cargo test --workspace --doc` | PASS | P19-VERIFY-UNION |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | PASS | P19-VERIFY-UNION |
| `cargo test --workspace --all-targets` | 1849 passed / 1 failed (export Git-history artifact, commit verified present in live repo) | P19-VERIFY-UNION |
| `cargo package -p opi-eval --list --allow-dirty` | PASS — src/READMEs/manifests only | P19-AC-013 |
| `opi-eval doctor/prepare` from independent /tmp dir | PASS — material/7 local origin, helper from audit_head | P19-AC-001 |
| `verify-local-acceptance.py --root <operator root>` | NOT RUN — no evidence root supplied | P19-AUD-001 |

## Verdict Rationale

Nine of fourteen sealed mandatory requirements are met on current audit_head
evidence, including native-class proof produced by this audit for the binding,
family-witness, client-projection, regrade, cleanup, and independence
criteria, and a full workspace gate run. Four mandatory requirements
(P19-AC-003/004/005/006) are partially met and P19-AC-012 is not assessable
because the live operator evidence root required by the criterion was not
supplied to this peer; the reciprocal blocking finding P19-AUD-001 records
that gap. Under the fail-dominant member rule — any mandatory state that is
not `met` — the member verdict is **FAIL**. The failure is evidentiary, not a
demonstrated product defect: no counter-evidence of non-conformance was found
for the live criteria, and all locally executable obligations passed.

<!-- opi-audit-history/1
{
  "audit\u005fhead": "9d805b498a56a276b141cc0c0728ce8201dbe6f3",
  "audit\u005frun\u005fid": "phase19-pi-glm53-9d805b4-20261004t094337z",
  "comparison": {
    "comparisons": [
      {
        "cause": "Difference in available operator-supplied raw proof between the two runs, plus stricter application of the raw-proof preflight (claims in acceptance deliverables are closure claims, not applicable proof). Requirement granularity also changed (P19-R-012 versus P19-AC-012 plus the live portions split across AC-003..006), which surfaces the gap explicitly instead of a single met verdict.",
        "current\u005fevidence": "At 9d805b4 the acceptance deliverables exist and the offline verifier tooling passes its fixture tests, but the invocation supplied no evidence\u005froot, so the six real trials and the live halves of AC-003/004/005/006 are unassessed raw proof rather than reproduced failures; the current sidecar records AC-012 not-assessable and AC-003..006 partially-met with reciprocal blocking finding P19-AUD-001. What remains uncertain: whether the prior run's independent verification would reproduce today against the same operator evidence root.",
        "current\u005ffinding\u005fid": "P19-AUD-001",
        "prior\u005fclaim": "The superseded run recorded its six-trial acceptance requirement (P19-R-012) as met with no reciprocal finding, and its report states the acceptance verification was 're-verified independently at audit\u005fhead' (2e82c99); the raw operator proof behind that claim is outside the committed tree and was not supplied to the current run either.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [
          {
            "checked": "Read the superseded report's finding and refutation sections; no raw execution proof is embedded in the record itself.",
            "limitations": "The prior run's underlying raw evidence root is not named by any verifiable in-tree artifact and was not resupplied; the claim cannot be confirmed or refuted from committed bytes.",
            "path": "docs/snapshots/phase19/assurance/history/phase19-pi-glm53-2e82c99-20261002t204027z/audit.pi.glm53.md",
            "sha256": "2f8e6d0b7fb8537c2d1b6d28f2d92aaf2717237feec406914eea60e7be6fe3df"
          }
        ],
        "relationship": "evidence-gap",
        "repair\u005fcommits": [],
        "verification": "Supply the redacted operator evidence root and run python crates/opi-eval/scripts/verify-local-acceptance.py --root \u003croot\u003e --out \u003cfresh-out\u003e plus scripts/opi-artifact-audit.py in a fresh remediation/verification context."
      },
      {
        "cause": "0fa995d (fix(eval): apply phase19 remediation batches B1-B10) replaced the stderr emission with a no-stderr design comment and missed the remediation\u005fnative\u005fstage\u005fdeadlines assertion that consumed it.",
        "current\u005fevidence": "At 2e82c99 the diagnostic client script still emitted 'stage-witness-start' to stderr (git show 2e82c99:crates/opi-eval/tests/support/native\u005fwitness\u005fmaterial.rs contains the producer) while the deadline test asserted it, giving a working state; commit 0fa995d (a descendant of 2e82c99, 2026-10-04) removed the producer without updating the assertion, and the current run fails deterministically twice on an idle host ('Agent never started' at terminal-bench-2.1/positive).",
        "current\u005ffinding\u005fid": "P19-AUD-002",
        "prior\u005fclaim": "No prior finding: at the prior head the deadline witness was internally consistent.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [
          {
            "checked": "Assertion at lines 11021-11024 requires 'stage-witness-start' in agent/stderr.log; live reproduction with three-family material/7 failed twice at terminal-bench-2.1/positive ('Agent never started') on an idle host.",
            "limitations": "Reproduction stdout/stderr were streamed through a filter and not retained byte-exact; the command and deterministic assertion mismatch are the durable record.",
            "path": "crates/opi-eval/tests/native\u005fdriver.rs",
            "sha256": "c06a76b88215787be0302418930c5e1a95da78aeeec03f06ef786a119d27995d"
          },
          {
            "checked": "diagnostic\u005fclient\u005fscript contains no stderr emission; git show 2e82c99 of the same file still contained the removed producer.",
            "limitations": "None within scope.",
            "path": "crates/opi-eval/tests/support/native\u005fwitness\u005fmaterial.rs",
            "sha256": "1d4559cfa2bf07c1041ff576f48fabfc032cccd14654373b07fddcb48c094ef1"
          }
        ],
        "relationship": "regression",
        "repair\u005fcommits": [],
        "verification": "Restore an admissible agent-start observation for the deadline witness (or a compliant marker) and re-run the exact ignored filter with three-family material; additionally gate the B-batch remediation flow on running touched ignored witnesses."
      },
      {
        "cause": "The prior run's verification-command review did not execute or count the declared ignored filters against the current test inventory; the current run's anti-vacuity check exposed the empty selection.",
        "current\u005fevidence": "The declared command selects zero tests at 9d805b4 (the only test named native\u005fprovider\u005ffailure\u005freport is non-ignored; the intended exact-client witness is native\u005fclient\u005fprovider\u005ffailure\u005freport); the defect existed in the same shape at the prior head and went unreported there.",
        "current\u005ffinding\u005fid": "P19-AUD-003",
        "prior\u005fclaim": "No prior finding on the declared-verification command hygiene, although the phase19 ledger at the prior head already carried the native\u005fprovider\u005ffailure\u005freport --ignored --exact command (six occurrences in docs/snapshots/phase19/opi-impl-state.json at 2e82c99).",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [
          {
            "checked": "Current committed ledger contains the vacuous command; git show 2e82c99 of the same path contains the same command shape.",
            "limitations": "Prior-run execution counts for this command are not recorded in its sidecars, so prior non-execution is inferred from the command's structural vacuity, not from a prior log.",
            "path": "docs/snapshots/phase19/opi-impl-state.json",
            "sha256": "35b3378bd902cc976bb348d794dd1b5d3c5f160a58769c3aeb62fea175a7b7a9"
          }
        ],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "cargo test -p opi-eval --test native\u005fdriver native\u005fprovider\u005ffailure\u005freport -- --ignored --exact reports 0 tests executed; correcting the ledger declaration (or renaming the exact-client witness) makes the filter non-vacuous."
      },
      {
        "cause": "Cosmetic annotation drift that survived the material/6 to /7 cutover; below the prior run's reporting threshold.",
        "current\u005fevidence": "At 9d805b4 the ignore text still references the superseded material/6 schema while the test consumes controlled material/7 and passes with it (executed by this audit in 120.68 s).",
        "current\u005ffinding\u005fid": "P19-AUD-004",
        "prior\u005fclaim": "No prior finding; the 'material/6' ignore annotation already existed at the prior head (four occurrences of material/6 in native\u005fdriver.rs at 2e82c99).",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [
          {
            "checked": "Ignore annotation at the native\u005fconformance\u005fowned\u005fcleanup\u005fwitness definition references material/6; the test executed and passed with controlled material/7 in 120.68 s.",
            "limitations": "None within scope.",
            "path": "crates/opi-eval/tests/native\u005fdriver.rs",
            "sha256": "c06a76b88215787be0302418930c5e1a95da78aeeec03f06ef786a119d27995d"
          }
        ],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Update the annotation text; no behavioral change expected."
      }
    ],
    "limitations": "(1) The prior run's P19-R-012 met verdict rests on an independent verification performed at its own head whose raw proof is neither in the tree nor resupplied; whether the live-evidence gap (current P19-AUD-001) also existed unverifiable at 2e82c99 cannot be established from committed bytes (insufficient information, bounded to this comparison). (2) Bounded lead for fresh independent verification, not a current finding: the superseded run's Info condition on docs/eval/evidence-roots.jsonl report pointers partially persists at audit\u005fhead - the 2026-09-27 longrun and 2026-09-29 recollection pointers still do not resolve in-tree (only README.md and the registry remain under docs/eval), while the acceptance pointer now resolves to docs/audits/phase19-local-native-acceptance.md. (3) Requirement identifiers differ between the runs (P19-R-\u002a versus P19-AC-\u002a), so comparison rows map by criterion substance rather than shared IDs; no strict-union key continuity is implied.",
    "schema": "opi-audit-history/1",
    "scope": "Compared the immediately superseded same-pair run phase19-pi-glm53-2e82c99-20261002t204027z (verdict PASS, audit\u005fhead 2e82c99aa32ca3a56acef3e2b969223161d370c7, 13 requirements P19-R-001..P19-R-013 all met, one Info finding) that this peer's complete moved to assurance/history/phase19-pi-glm53-2e82c99-20261002t204027z/; its bytes were last carried by commit c770e3368203aadd6d9ee5a9713d4d399041cc5b. No other history record, sibling member, or remediation material entered the comparison. Current behavior is anchored to audit\u005fhead 9d805b498a56a276b141cc0c0728ce8201dbe6f3. Requirement identifiers are not shared between the runs (P19-R-\u002a versus P19-AC-\u002a), so rows map by criterion substance.",
    "sources": [
      {
        "audit\u005frun\u005fid": "phase19-pi-glm53-2e82c99-20261002t204027z",
        "commit": "c770e3368203aadd6d9ee5a9713d4d399041cc5b",
        "id": "prior-meta",
        "path": "docs/snapshots/phase19/assurance/history/phase19-pi-glm53-2e82c99-20261002t204027z/audit.pi.glm53.meta.json",
        "sha256": "d64bf8be9db090b5a464c0cdb7ca6c415a01ebfc2159c1cd5bf1282dc09c66b7"
      },
      {
        "audit\u005frun\u005fid": "phase19-pi-glm53-2e82c99-20261002t204027z",
        "commit": "c770e3368203aadd6d9ee5a9713d4d399041cc5b",
        "id": "prior-requirements",
        "path": "docs/snapshots/phase19/assurance/history/phase19-pi-glm53-2e82c99-20261002t204027z/audit.pi.glm53.requirements.jsonl",
        "sha256": "44e12e1c912c7f7745e8e7e56ddffd32f78204917b5b0a85f3f6beb6030017b4"
      },
      {
        "audit\u005frun\u005fid": "phase19-pi-glm53-2e82c99-20261002t204027z",
        "commit": "c770e3368203aadd6d9ee5a9713d4d399041cc5b",
        "id": "prior-findings",
        "path": "docs/snapshots/phase19/assurance/history/phase19-pi-glm53-2e82c99-20261002t204027z/audit.pi.glm53.findings.jsonl",
        "sha256": "d8dd6dd9833a5575975171b815f790d5c1c26ae3df4d5b94b21a84663cb8c7d1"
      },
      {
        "audit\u005frun\u005fid": "phase19-pi-glm53-2e82c99-20261002t204027z",
        "commit": "c770e3368203aadd6d9ee5a9713d4d399041cc5b",
        "id": "prior-report",
        "path": "docs/snapshots/phase19/assurance/history/phase19-pi-glm53-2e82c99-20261002t204027z/audit.pi.glm53.md",
        "sha256": "2f8e6d0b7fb8537c2d1b6d28f2d92aaf2717237feec406914eea60e7be6fe3df"
      }
    ],
    "status": "compared"
  },
  "core": {
    "findings\u005fsha256": "377d730be9a09201c6056b3513739fa2f524ec684019ae516ef04182da56eeaf",
    "meta\u005fsha256": "257f3dc0849103c5f7770168aa7ccb041a3269f3ea3cdbf59bef3b87de6e7e5d",
    "report\u005fbytes": 17939,
    "report\u005fsha256": "5746c349acfe4b9318afade363d62145cbacce590225d61338914c3eac23cc9f",
    "requirements\u005fsha256": "94a60de1e21ca28f1e2e13f86af2217ceb34f273aeaeef6652b6004769eef2d9"
  },
  "schema": "opi-audit-history/1"
}
-->

## Post-audit historical reference — advisory

This section was appended after installation of the independent core. It cannot change findings, requirements, independence or verdict. Historical closure claims require current independent verification.

- Owning audit run: phase19&#45;pi&#45;glm53&#45;9d805b4&#45;20261004t094337z
- Audit head: 9d805b498a56a276b141cc0c0728ce8201dbe6f3
- Core report bytes: 17939
- Core report SHA-256: 5746c349acfe4b9318afade363d62145cbacce590225d61338914c3eac23cc9f
- Current findings SHA-256: 377d730be9a09201c6056b3513739fa2f524ec684019ae516ef04182da56eeaf
- Comparison status: compared
- Inspected history: Compared the immediately superseded same&#45;pair run phase19&#45;pi&#45;glm53&#45;2e82c99&#45;20261002t204027z &#40;verdict PASS, audit&#95;head 2e82c99aa32ca3a56acef3e2b969223161d370c7, 13 requirements P19&#45;R&#45;001&#46;&#46;P19&#45;R&#45;013 all met, one Info finding&#41; that this peer&#39;s complete moved to assurance/history/phase19&#45;pi&#45;glm53&#45;2e82c99&#45;20261002t204027z/; its bytes were last carried by commit c770e3368203aadd6d9ee5a9713d4d399041cc5b&#46; No other history record, sibling member, or remediation material entered the comparison&#46; Current behavior is anchored to audit&#95;head 9d805b498a56a276b141cc0c0728ce8201dbe6f3&#46; Requirement identifiers are not shared between the runs &#40;P19&#45;R&#45;&#42; versus P19&#45;AC&#45;&#42;&#41;, so rows map by criterion substance&#46;
- Limitations: &#40;1&#41; The prior run&#39;s P19&#45;R&#45;012 met verdict rests on an independent verification performed at its own head whose raw proof is neither in the tree nor resupplied; whether the live&#45;evidence gap &#40;current P19&#45;AUD&#45;001&#41; also existed unverifiable at 2e82c99 cannot be established from committed bytes &#40;insufficient information, bounded to this comparison&#41;&#46; &#40;2&#41; Bounded lead for fresh independent verification, not a current finding: the superseded run&#39;s Info condition on docs/eval/evidence&#45;roots&#46;jsonl report pointers partially persists at audit&#95;head &#45; the 2026&#45;09&#45;27 longrun and 2026&#45;09&#45;29 recollection pointers still do not resolve in&#45;tree &#40;only README&#46;md and the registry remain under docs/eval&#41;, while the acceptance pointer now resolves to docs/audits/phase19&#45;local&#45;native&#45;acceptance&#46;md&#46; &#40;3&#41; Requirement identifiers differ between the runs &#40;P19&#45;R&#45;&#42; versus P19&#45;AC&#45;&#42;&#41;, so comparison rows map by criterion substance rather than shared IDs; no strict&#45;union key continuity is implied&#46;

### Pinned historical sources

- prior&#45;meta: docs/snapshots/phase19/assurance/history/phase19&#45;pi&#45;glm53&#45;2e82c99&#45;20261002t204027z/audit&#46;pi&#46;glm53&#46;meta&#46;json; commit c770e3368203aadd6d9ee5a9713d4d399041cc5b; SHA-256 d64bf8be9db090b5a464c0cdb7ca6c415a01ebfc2159c1cd5bf1282dc09c66b7; audit run phase19&#45;pi&#45;glm53&#45;2e82c99&#45;20261002t204027z
- prior&#45;requirements: docs/snapshots/phase19/assurance/history/phase19&#45;pi&#45;glm53&#45;2e82c99&#45;20261002t204027z/audit&#46;pi&#46;glm53&#46;requirements&#46;jsonl; commit c770e3368203aadd6d9ee5a9713d4d399041cc5b; SHA-256 44e12e1c912c7f7745e8e7e56ddffd32f78204917b5b0a85f3f6beb6030017b4; audit run phase19&#45;pi&#45;glm53&#45;2e82c99&#45;20261002t204027z
- prior&#45;findings: docs/snapshots/phase19/assurance/history/phase19&#45;pi&#45;glm53&#45;2e82c99&#45;20261002t204027z/audit&#46;pi&#46;glm53&#46;findings&#46;jsonl; commit c770e3368203aadd6d9ee5a9713d4d399041cc5b; SHA-256 d8dd6dd9833a5575975171b815f790d5c1c26ae3df4d5b94b21a84663cb8c7d1; audit run phase19&#45;pi&#45;glm53&#45;2e82c99&#45;20261002t204027z
- prior&#45;report: docs/snapshots/phase19/assurance/history/phase19&#45;pi&#45;glm53&#45;2e82c99&#45;20261002t204027z/audit&#46;pi&#46;glm53&#46;md; commit c770e3368203aadd6d9ee5a9713d4d399041cc5b; SHA-256 2f8e6d0b7fb8537c2d1b6d28f2d92aaf2717237feec406914eea60e7be6fe3df; audit run phase19&#45;pi&#45;glm53&#45;2e82c99&#45;20261002t204027z

### Comparison 1: P19&#45;AUD&#45;001

- Relationship: evidence&#45;gap
- Prior claim (attributed): The superseded run recorded its six&#45;trial acceptance requirement &#40;P19&#45;R&#45;012&#41; as met with no reciprocal finding, and its report states the acceptance verification was &#39;re&#45;verified independently at audit&#95;head&#39; &#40;2e82c99&#41;; the raw operator proof behind that claim is outside the committed tree and was not supplied to the current run either&#46;
- Current evidence: At 9d805b4 the acceptance deliverables exist and the offline verifier tooling passes its fixture tests, but the invocation supplied no evidence&#95;root, so the six real trials and the live halves of AC&#45;003/004/005/006 are unassessed raw proof rather than reproduced failures; the current sidecar records AC&#45;012 not&#45;assessable and AC&#45;003&#46;&#46;006 partially&#45;met with reciprocal blocking finding P19&#45;AUD&#45;001&#46; What remains uncertain: whether the prior run&#39;s independent verification would reproduce today against the same operator evidence root&#46;
- Cause / missed boundary: Difference in available operator&#45;supplied raw proof between the two runs, plus stricter application of the raw&#45;proof preflight &#40;claims in acceptance deliverables are closure claims, not applicable proof&#41;&#46; Requirement granularity also changed &#40;P19&#45;R&#45;012 versus P19&#45;AC&#45;012 plus the live portions split across AC&#45;003&#46;&#46;006&#41;, which surfaces the gap explicitly instead of a single met verdict&#46;
- Suggested current verification: Supply the redacted operator evidence root and run python crates/opi&#45;eval/scripts/verify&#45;local&#45;acceptance&#46;py &#45;&#45;root &#60;root&#62; &#45;&#45;out &#60;fresh&#45;out&#62; plus scripts/opi&#45;artifact&#45;audit&#46;py in a fresh remediation/verification context&#46;
- Candidate raw proof: docs/snapshots/phase19/assurance/history/phase19&#45;pi&#45;glm53&#45;2e82c99&#45;20261002t204027z/audit&#46;pi&#46;glm53&#46;md; SHA-256 2f8e6d0b7fb8537c2d1b6d28f2d92aaf2717237feec406914eea60e7be6fe3df; checked: Read the superseded report&#39;s finding and refutation sections; no raw execution proof is embedded in the record itself&#46;; limits: The prior run&#39;s underlying raw evidence root is not named by any verifiable in&#45;tree artifact and was not resupplied; the claim cannot be confirmed or refuted from committed bytes&#46;

### Comparison 2: P19&#45;AUD&#45;002

- Relationship: regression
- Prior claim (attributed): No prior finding: at the prior head the deadline witness was internally consistent&#46;
- Current evidence: At 2e82c99 the diagnostic client script still emitted &#39;stage&#45;witness&#45;start&#39; to stderr &#40;git show 2e82c99:crates/opi&#45;eval/tests/support/native&#95;witness&#95;material&#46;rs contains the producer&#41; while the deadline test asserted it, giving a working state; commit 0fa995d &#40;a descendant of 2e82c99, 2026&#45;10&#45;04&#41; removed the producer without updating the assertion, and the current run fails deterministically twice on an idle host &#40;&#39;Agent never started&#39; at terminal&#45;bench&#45;2&#46;1/positive&#41;&#46;
- Cause / missed boundary: 0fa995d &#40;fix&#40;eval&#41;: apply phase19 remediation batches B1&#45;B10&#41; replaced the stderr emission with a no&#45;stderr design comment and missed the remediation&#95;native&#95;stage&#95;deadlines assertion that consumed it&#46;
- Suggested current verification: Restore an admissible agent&#45;start observation for the deadline witness &#40;or a compliant marker&#41; and re&#45;run the exact ignored filter with three&#45;family material; additionally gate the B&#45;batch remediation flow on running touched ignored witnesses&#46;
- Candidate raw proof: crates/opi&#45;eval/tests/native&#95;driver&#46;rs; SHA-256 c06a76b88215787be0302418930c5e1a95da78aeeec03f06ef786a119d27995d; checked: Assertion at lines 11021&#45;11024 requires &#39;stage&#45;witness&#45;start&#39; in agent/stderr&#46;log; live reproduction with three&#45;family material/7 failed twice at terminal&#45;bench&#45;2&#46;1/positive &#40;&#39;Agent never started&#39;&#41; on an idle host&#46;; limits: Reproduction stdout/stderr were streamed through a filter and not retained byte&#45;exact; the command and deterministic assertion mismatch are the durable record&#46;
- Candidate raw proof: crates/opi&#45;eval/tests/support/native&#95;witness&#95;material&#46;rs; SHA-256 1d4559cfa2bf07c1041ff576f48fabfc032cccd14654373b07fddcb48c094ef1; checked: diagnostic&#95;client&#95;script contains no stderr emission; git show 2e82c99 of the same file still contained the removed producer&#46;; limits: None within scope&#46;

### Comparison 3: P19&#45;AUD&#45;003

- Relationship: new&#45;omission
- Prior claim (attributed): No prior finding on the declared&#45;verification command hygiene, although the phase19 ledger at the prior head already carried the native&#95;provider&#95;failure&#95;report &#45;&#45;ignored &#45;&#45;exact command &#40;six occurrences in docs/snapshots/phase19/opi&#45;impl&#45;state&#46;json at 2e82c99&#41;&#46;
- Current evidence: The declared command selects zero tests at 9d805b4 &#40;the only test named native&#95;provider&#95;failure&#95;report is non&#45;ignored; the intended exact&#45;client witness is native&#95;client&#95;provider&#95;failure&#95;report&#41;; the defect existed in the same shape at the prior head and went unreported there&#46;
- Cause / missed boundary: The prior run&#39;s verification&#45;command review did not execute or count the declared ignored filters against the current test inventory; the current run&#39;s anti&#45;vacuity check exposed the empty selection&#46;
- Suggested current verification: cargo test &#45;p opi&#45;eval &#45;&#45;test native&#95;driver native&#95;provider&#95;failure&#95;report &#45;&#45; &#45;&#45;ignored &#45;&#45;exact reports 0 tests executed; correcting the ledger declaration &#40;or renaming the exact&#45;client witness&#41; makes the filter non&#45;vacuous&#46;
- Candidate raw proof: docs/snapshots/phase19/opi&#45;impl&#45;state&#46;json; SHA-256 35b3378bd902cc976bb348d794dd1b5d3c5f160a58769c3aeb62fea175a7b7a9; checked: Current committed ledger contains the vacuous command; git show 2e82c99 of the same path contains the same command shape&#46;; limits: Prior&#45;run execution counts for this command are not recorded in its sidecars, so prior non&#45;execution is inferred from the command&#39;s structural vacuity, not from a prior log&#46;

### Comparison 4: P19&#45;AUD&#45;004

- Relationship: new&#45;omission
- Prior claim (attributed): No prior finding; the &#39;material/6&#39; ignore annotation already existed at the prior head &#40;four occurrences of material/6 in native&#95;driver&#46;rs at 2e82c99&#41;&#46;
- Current evidence: At 9d805b4 the ignore text still references the superseded material/6 schema while the test consumes controlled material/7 and passes with it &#40;executed by this audit in 120&#46;68 s&#41;&#46;
- Cause / missed boundary: Cosmetic annotation drift that survived the material/6 to /7 cutover; below the prior run&#39;s reporting threshold&#46;
- Suggested current verification: Update the annotation text; no behavioral change expected&#46;
- Candidate raw proof: crates/opi&#45;eval/tests/native&#95;driver&#46;rs; SHA-256 c06a76b88215787be0302418930c5e1a95da78aeeec03f06ef786a119d27995d; checked: Ignore annotation at the native&#95;conformance&#95;owned&#95;cleanup&#95;witness definition references material/6; the test executed and passed with controlled material/7 in 120&#46;68 s&#46;; limits: None within scope&#46;
