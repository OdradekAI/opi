# Phase 19 Audit

**Audit run ID**: `phase19-pi-glm53-2e82c99-20261002t204027z`
**Audit head**: `2e82c99aa32ca3a56acef3e2b969223161d370c7`
**Reviewer ID**: `pi`
**Model ID**: `glm53`
**Reviewer identity**: pi coding agent (pi CLI harness)
**Reviewer model ID**: `glm53`
**Model identity source**: operator-declared
**Independence**: fresh-context-same-family (operator-supplied model slug `glm53` recorded verbatim as reviewer model identity) — this peer sealed its baseline, requirements and findings in a fresh context with no prior audit, remediation or sibling-peer content in scope; the model family may match other reviewers' peers.
**Baseline policy**: latest-committed-spec
**Verdict**: PASS

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| `.opi-impl-state.json` | c65c10635c4bb8b1754625b94e8a0a15459eef6d65634610ba9b65ea9ba5337e | root ledger (reader-projected scope input) |
| `docs/snapshots/phase19/opi-impl-state.json` | 35b3378bd902cc976bb348d794dd1b5d3c5f160a58769c3aeb62fea175a7b7a9 | Phase 19 ledger (reader-projected scope input) |
| `docs/opi-spec.md` | 1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db | normative parent (STRAT-002, PHASE-001..006) |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | 5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a | Phase 19 delivery specification (owns AC criteria) |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.zh.md` | d41c826412b0d71802327d60f953708f6cb2083436a2a2b0d99a2e52d851f194 | Chinese counterpart; all 13 AC and 8 D identifiers preserved |

No stored hash mismatches. All inspection and checks ran in a `git archive` export of `audit_head` (no worktree); `audit_head` is a committed ancestor of HEAD.

## Requirement Conformance

One record per spec acceptance criterion (`P19-AC-001`..`P19-AC-013`, mapped to `P19-R-001`..`P19-R-013`); full evidence in `audit.pi.glm53.requirements.jsonl`.

| Requirement ID | Criterion | Current evidence (summary) | State | Finding IDs |
|---|---|---|---|---|
| P19-R-001 | P19-AC-001 | doctor/prepare/plan/run/verify/regrade/report CLI from independent directory; `independent_directory_prepare…`, `native_frozen_plan_runs_without_fixtures…` green; local-origin material replaces GitHub receipts | met | — |
| P19-R-002 | P19-AC-002 | `NativeTrialBinding` typed rejections; `native_result_binding_rejects_every_substitution` discriminates 7 substitution/oracle modes with no sealed bundle | met | — |
| P19-R-003 | P19-AC-003 | task.toml lifecycle parse + `LifecycleMismatch` binding rejection; paired single-job dispatch and terminal-shape settlement green; live TB2.1 capture in verified acceptance root | met | — |
| P19-R-004 | P19-AC-004 | declared/undeclared/missing/substituted collection binding; TB30 separate-verifier run + rejection tests green | met | — |
| P19-R-005 | P19-AC-005 | base/patch/verifier identity binding; DeepSWE pristine-verifier, base-drift and nop-committed-zero tests green | met | — |
| P19-R-006 | P19-AC-006 | named-connection projection (exact protocol/model/credential, loopback-unless-live, unenforceable caps rejected, required controls pre-validated); typed provider terminal causes; credential-missing rejection green | met | — |
| P19-R-007 | P19-AC-007 | full-inventory preflight, no silent shrink, frozen digest-bound subset execution, tampered-plan rejection green | met | — |
| P19-R-008 | P19-AC-008 | planned/attempted/valid/invalid/unexecuted denominators, subset labeling, protocol-safe pairing, single-subject edge-free; report contract suites green | met | — |
| P19-R-009 | P19-AC-009 | verify (bytes) / report (deterministic reconstruction) / regrade (fresh native verifier) distinct; fail-closed and tamper rejections green | met | — |
| P19-R-010 | P19-AC-010 | canary/credential privacy (private 0700/0600 staging, echo-never-sealed), signal cancellation, effect-unknown reopen, recovery and fresh-identity replacement green | met | — |
| P19-R-011 | P19-AC-011 | `dummy_credentials_do_not_activate_live_behavior` green; 272 Python script tests OK; CI workflows are drift-bound thin producers | met | — |
| P19-R-012 | P19-AC-012 | registered 2026-09-29 six-trial acceptance root re-verified at audit_head: `verify-local-acceptance.py` accepted:true/exit 0; `opi-artifact-audit.py` PASS; retained failed attempts disclosed | met | P19-AUD-001 |
| P19-R-013 | P19-AC-013 | no Opi dependency in any class; pi-only packaging test green; package list free of real corpora/graders/images; Unreleased records deliberate 0.x changes; bilingual docs pass doc-check | met | — |

## Standards Review

- **Dependency direction / placement**: `opi-eval` depends on no Opi crate (normal/dev/build/optional/target-specific verified in `Cargo.toml`); public library surface is `cli` + `experiment` only; every other module is crate-private. `#![deny(unsafe_code)]` with the single documented FFI home `process::tree` overriding locally. No speculative public trait or new crate.
- **Error handling**: typed `thiserror`-style errors at CLI boundaries; static redacted tokens for job-tree bytes; fail-closed `ResolveError`/`NativeBindingError`/`ProjectionError` taxonomies inspected in source.
- **Documentation lockstep**: `scripts/opi-doc-check.py` PASS at audit_head; English/Chinese acceptance audit pair and spec counterpart synchronized; README bilingual walkthrough consistent with the CLI.
- **Verification union at audit_head**: `cargo fmt --check --all` green; `cargo clippy --workspace --all-targets -- -D warnings` green; `cargo test -p opi-eval` fully green (172 lib + all integration binaries; native_driver 112 passed / 11 ignored requiring Docker or pinned external client material); `cargo test --workspace --all-targets` green except `opi-coding-agent::artifact_audit_script::artifact_audit_accepts_real_declared_commit_objects`, which fails solely because the sealed `git archive` export has no `.git` database — the identical script invocation against the live repository at the same HEAD passes (`artifact audit: PASS`, exit 0), refuting a product defect. `cargo test --workspace --doc` was not rerun by this peer (no doc tests in opi-eval; other crates outside Phase 19 scope).

## Spec Review

All thirteen acceptance criteria are satisfied by current committed behavior plus registered, offline-verifiable operator evidence. The six-trial live obligation (AC-012) is met by the registered 2026-09-29 re-collection root (material/6, companion `d2502daf…`): this peer independently re-ran the committed offline verifier and the Opi-only artifact audit at `audit_head` — both accept. The earlier 2026-09-15 root stays registered as historical with its verifier-contract evolution disclosed, matching the spec's legacy-evidence policy (read-only legacy, no score relabeling). Native zeros and agent failures remain graded outcomes; comparability markers rather than exit-code fabrication settle incomplete pairs.

## Security, Invariants, Integration, Test Quality, and Residuals

- **Security/authority**: credential values live only in the supervised launch handoff (Opi config references `api_key_env`; pi `models.json` staged private with 0600/0700 and removed at settlement); canary sealing blocks publication; loopback-only routes without explicit `live`; unenforceable monetary/usage/output caps rejected at resolution rather than ignored.
- **Invariants/integration**: planned-coverage binding written before any effect; sealed-bundle immutability with tamper rejection; run-root single-owner claim; recovery classification distinguishes completed/cancelled/failed/effect-unknown/unstarted without fabricated scores; replacement groups take fresh identities while preserving prior evidence.
- **Test quality**: ~19.4k lines of integration tests across 12 binaries drive the spawned public CLI (stdout/stderr/exit + persisted artifacts), with negative matrices (substitution modes, tamper cases, plan contradictions, fail-closed regrade) rather than success-only assertions. Anti-vacuity spot-checks confirmed assertions observe typed failure kinds and sealed-artifact absence, not just exit codes.
- **Residuals**: no TODO/FIXME markers in `src/`; legacy score paths (host-agent branch, `newest_job_result`, `--preflight-only`, answer.txt glue) absent from production code. One advisory residual recorded as P19-AUD-001 (registry report pointers resolvable only through Git history).

## Minimum-change Conformance

Audited at `audit_head` against the ledger-projected task notes for 19.1–19.11: **conforming**.

- Recorded `reuse_search` entries match current code: reused experiment/lifecycle/supervision/binding surfaces are present; the recorded removals (`newest_job_result` authority, host-agent score branch, `--preflight-only`) are absent from `src/`.
- `surface_necessity` holds: each new seam (doctor/prepare/plan/verify/regrade CLI, `NativeTrialBinding`, `task_lifecycle`, material/plan modules) has production consumers (`main.rs` dispatch, runner, verifier scripts/workflows) and discriminating tests; `production_consumers`/`nonproduction_consumers` recorded per requirement. `net_deletion` verified for the named legacy paths; `residual_glue` limited to the documented pinned-integration and hidden `__native-capture` binary-local mode (no public CLI/config surface).
- `simplification_ceiling` respected: one report builder, one settlement owner, verify/regrade split by operation rather than duplicated scoring policy.

## Findings

### P19-AUD-001: Evidence-root registry report pointers dangle in-tree

- Axis: residuals
- Severity: Info
- Conformance effect: advisory
- Requirement IDs: P19-R-012
- Claim: `docs/eval/evidence-roots.jsonl` `report` fields reference `docs/eval/` files removed from the checkout by commit d42a7a0 (including the current qualifying 2026-09-29 acceptance root's recollection report), so in-tree resolution fails; only Git history at the removal commit resolves them.
- Evidence: registry entries vs `git ls-tree HEAD docs/eval/` (only README.md and the registry remain); removal commit message and `docs/eval/README.md` disclose the convention ("remain readable in Git history").
- Refutation attempted: n/a (advisory). The removal is deliberate, disclosed and changelog-recorded; acceptance verification itself does not depend on the in-tree report files (re-verified independently at audit_head).
- Suggested closure: make registry report pointers history-aware (e.g., pin the removal/resolving commit per entry) or regenerate conclusion-pair reports in-tree for currently qualifying roots.

## Verification Commands

All commands ran in the sealed `audit archive` export at `audit_head` unless noted.

| Command | Result | Requirement/finding |
|---|---|---|
| `cargo test -p opi-eval` | PASS (172 lib + all integration binaries; native_driver 112 passed / 11 ignored) | R-001..R-011, R-013 |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py'` | PASS (272 tests, 1 skipped) | R-011, R-012 |
| `python3 scripts/opi-doc-check.py` | PASS | R-013 |
| `cargo fmt --check --all` | PASS | standards |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | standards |
| `cargo test --workspace --all-targets` | PASS except `artifact_audit_accepts_real_declared_commit_objects` (export lacks `.git`; same inputs PASS against the live repo at the same HEAD — environment-induced, refuted as product defect) | standards |
| `python3 crates/opi-eval/scripts/verify-local-acceptance.py --root <registered 2026-09-29 acceptance root> --out … --companion <pinned d2502daf… binary>` | PASS — `accepted: true`, exit 0, six trials, three families | R-012 |
| `python3 scripts/opi-artifact-audit.py --workspace-root . <root>/opi-artifact-audit` | PASS | R-012 |
| `cargo package -p opi-eval --list --allow-dirty` | PASS — no Opi dependency, no real corpora/graders/images | R-013 |
| 11 ignored native filters (`native_tb21/tb3_artifact_witness`, `native_deepswe_patch_witness`, `native_protocol_client_matrix`, `native_offline_regrade`(witness variant), `native_owned_resource_cancellation`, exact-client matrix, stage deadlines, conformance-owned-cleanup witness, verifier-only regrade witness) | NOT RUN — require local Docker engine / pinned external client material unavailable in this audit context; covered indirectly by non-ignored stand-in tests at the same seams and by the offline-verified acceptance root | R-003..R-006, R-009, R-010 (limitation) |
| `cargo test --workspace --doc`, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | NOT RUN by this peer (opi-eval has no doc tests; remaining crates outside Phase 19 scope) | standards (limitation) |

## Verdict Rationale

All thirteen mandatory requirement records are `met` at `audit_head`: the deterministic public-seam suite is green, the offline acceptance evidence independently re-verifies, and placement/contract recording obligations hold. The single published finding is advisory (Info), so the mechanical member verdict is PASS. Limitations (ignored native filters needing Docker/pinned material; workspace doc tests not rerun) are recorded above and do not change any requirement state, because each affected criterion carries independent current evidence through non-ignored tests at the same production seams and the verified registered acceptance root.

<!-- opi-audit-history/1
{
  "audit\u005fhead": "2e82c99aa32ca3a56acef3e2b969223161d370c7",
  "audit\u005frun\u005fid": "phase19-pi-glm53-2e82c99-20261002t204027z",
  "comparison": {
    "comparisons": [
      {
        "cause": "The bilingual conclusion-pair contract change (removal commit d42a7a0) deliberately removed the detailed docs/eval/ operator reports and left the registry rows unchanged; no subsequent commit added removal-commit pinning or in-tree replacement reports.",
        "current\u005fevidence": "Independently re-derived at audit\u005fhead 2e82c99: git ls-tree HEAD docs/eval/ shows only README.md and evidence-roots.jsonl; the registry report paths for phase19-acceptance-20260929 and phase19-longrun-validation-20260927 still do not exist in-tree and no row pins a resolving commit. Disclosure remains in docs/eval/README.md and CHANGELOG Unreleased/Changed. Uncertain: nothing - both prior and current states were directly observed.",
        "current\u005ffinding\u005fid": "P19-AUD-001",
        "prior\u005fclaim": "Prior Info finding: docs/eval/evidence-roots.jsonl entries (2026-09-29 acceptance re-collection, 2026-09-27 longrun) carry report fields pointing at docs/eval/\u002a.md files removed from the checkout on 2026-10-01; README documents Git-history resolution but registry rows do not record the removal commit.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-pi-glm53-efe2f69-20261001t043357z",
            "finding\u005fid": "P19-AUD-003",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "json parse of every report field plus existence check of each referenced path in the audit\u005fhead export; git ls-tree HEAD docs/eval/",
            "limitations": "Reports remain resolvable through Git history at the removal commit; acceptance verification itself does not read the in-tree report files (verify-local-acceptance and artifact audit re-run green at audit\u005fhead)",
            "path": "docs/eval/evidence-roots.jsonl",
            "sha256": "11d41b4745959dadcbf166be0c0349cee8e672f141fd607cff60a0de892a65be"
          }
        ],
        "relationship": "persistent",
        "repair\u005fcommits": [],
        "verification": "Pin the resolving/removal commit per registry row (or regenerate in-tree conclusion pairs for currently qualifying roots), then confirm every report field resolves without a history lookup."
      },
      {
        "cause": "Remediation batch B9 replaced the hardcoded host path with fixture-owned synthetic bytes, removing the host-tool coupling.",
        "current\u005fevidence": "Verified at audit\u005fhead 2e82c99: the test now writes synthetic executable uv bytes inside its own fixture root and the mocked 'command -v uv' returns that fixture path; /usr/bin/uv does not exist in this audit environment and 'python3 -m unittest discover -s crates/opi-eval/scripts -p test\u005f\u002a.py' ran 272 tests OK (1 skipped) with zero errors. Uncertain: nothing material; the exact-client ignored native filters remain Docker/material-gated and are unrelated to this defect.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Prior Major blocking finding: test\u005fnative\u005fsmoke\u005foutput\u005fcontrol.py faked 'command -v uv' with a hardcoded /usr/bin/uv while the embedded materialize-configs stage copied the real binary, erroring three tests wherever uv is absent; the registered AC-011 unittest command failed at the prior audit\u005fhead. The remediation commit message claims this closed among eleven findings - not adopted as current closure.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-pi-glm53-efe2f69-20261001t043357z",
            "finding\u005fid": "P19-AUD-001",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "source read at audit\u005fhead plus full unittest discover rerun on a uv-less host (272 OK, 1 skipped)",
            "limitations": "None within scope; the repair is observable on any machine without /usr/bin/uv",
            "path": "crates/opi-eval/scripts/test\u005fnative\u005fsmoke\u005foutput\u005fcontrol.py",
            "sha256": "43d7d9541a6f209922c944b75b31c3d0e7ebca35a07680e3427463cabda521db"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "81d26aac8772b492e7ee4586fbbb7b331f78ce6b"
        ],
        "verification": "Keep the registered AC-011 command green on uv-less hosts (CI or a fresh container without uv installed)."
      },
      {
        "cause": "The same remediation commit applied the rustfmt rendering to the touched hunk.",
        "current\u005fevidence": "Verified at audit\u005fhead 2e82c99: 'cargo fmt --check --all' exits 0 (rerun by this peer in the sealed export); the formerly multi-line assert at native\u005fdriver.rs is now the rustfmt single-line form. Uncertain: nothing.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Prior Minor advisory finding: cargo fmt --check --all exited 1 at the prior audit\u005fhead with one unformatted hunk at crates/opi-eval/tests/native\u005fdriver.rs:10054, blocking the Phase 8-section verification union.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-pi-glm53-efe2f69-20261001t043357z",
            "finding\u005fid": "P19-AUD-002",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "cargo fmt --check --all at audit\u005fhead (exit 0) plus diff inspection of commit 81d26aac",
            "limitations": "None within scope",
            "path": "crates/opi-eval/tests/native\u005fdriver.rs",
            "sha256": "d996f5cab322cda6e5c451cdc170670d7955c28710288072bf3636f01434f598"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "81d26aac8772b492e7ee4586fbbb7b331f78ce6b"
        ],
        "verification": "The workspace fmt gate in CI/Phase exit continues to enforce this."
      }
    ],
    "limitations": "Comparison bounded to the single superseded same-pair run cited above; no other history record, sibling peer output or remediation material was ingested. The prior run sealed a finer-grained requirement set (26 records including P19-R-GATES, then not-met, and P19-R-AC011, then partially-met) whose failing causes map to the two prior findings verified here as repaired; this member independently sealed 13 AC-level requirements, all met at the current head. No newly noticed post-install defect absent from the current sidecar arose within this comparison's scope. The prior verdict FAIL and its closure claims are historical attributes only; the installed core verdict stands on current evidence.",
    "schema": "opi-audit-history/1",
    "scope": "Compared only the immediately superseded run of this same (pi, glm53) pair - audit\u005frun\u005fid phase19-pi-glm53-efe2f69-20261001t043357z, sealed at audit\u005fhead efe2f6906fa47003bf806bddec0b15f2d20dd3c3 and last carried by commit 2465aeb80ad84d06172acc142f6d5ffbceb201cc - against this member's current core sealed at audit\u005fhead 2e82c99aa32ca3a56acef3e2b969223161d370c7. Every other history record (the older phase19-pi-glm53-ac80281 run, the codex-gpt6 run, unrelated remediation material) and all concurrent sibling output stayed outside the comparison. Current behavior is anchored to the current audit\u005fhead only.",
    "sources": [
      {
        "audit\u005frun\u005fid": "phase19-pi-glm53-efe2f69-20261001t043357z",
        "commit": "2465aeb80ad84d06172acc142f6d5ffbceb201cc",
        "id": "prior-report",
        "path": "docs/snapshots/phase19/assurance/history/phase19-pi-glm53-efe2f69-20261001t043357z/audit.pi.glm53.md",
        "sha256": "44b2c3538ef101efedf67a53e2f08d7deb5a60c82716161f986dfb85ddf326fc"
      },
      {
        "audit\u005frun\u005fid": "phase19-pi-glm53-efe2f69-20261001t043357z",
        "commit": "2465aeb80ad84d06172acc142f6d5ffbceb201cc",
        "id": "prior-meta",
        "path": "docs/snapshots/phase19/assurance/history/phase19-pi-glm53-efe2f69-20261001t043357z/audit.pi.glm53.meta.json",
        "sha256": "d9bf70ecba230c708c20df2d61092a63e94be93a8905697620761ae5ad2c5bf0"
      },
      {
        "audit\u005frun\u005fid": "phase19-pi-glm53-efe2f69-20261001t043357z",
        "commit": "2465aeb80ad84d06172acc142f6d5ffbceb201cc",
        "id": "prior-requirements",
        "path": "docs/snapshots/phase19/assurance/history/phase19-pi-glm53-efe2f69-20261001t043357z/audit.pi.glm53.requirements.jsonl",
        "sha256": "9315c6f7894ac36ccced55b081e76f0162364200fd9c81398cb55723f38a5aba"
      },
      {
        "audit\u005frun\u005fid": "phase19-pi-glm53-efe2f69-20261001t043357z",
        "commit": "2465aeb80ad84d06172acc142f6d5ffbceb201cc",
        "id": "prior-findings",
        "path": "docs/snapshots/phase19/assurance/history/phase19-pi-glm53-efe2f69-20261001t043357z/audit.pi.glm53.findings.jsonl",
        "sha256": "fb086ccb76ea53156b0600c6b46f8e8c1b9fb8842515f3acff8669ec69923b70"
      }
    ],
    "status": "compared"
  },
  "core": {
    "findings\u005fsha256": "d8dd6dd9833a5575975171b815f790d5c1c26ae3df4d5b94b21a84663cb8c7d1",
    "meta\u005fsha256": "d64bf8be9db090b5a464c0cdb7ca6c415a01ebfc2159c1cd5bf1282dc09c66b7",
    "report\u005fbytes": 13473,
    "report\u005fsha256": "9b27aab9b61f26cc4df095c2847b9652584b273aa38647e4a1da23b4018d24b6",
    "requirements\u005fsha256": "44e12e1c912c7f7745e8e7e56ddffd32f78204917b5b0a85f3f6beb6030017b4"
  },
  "schema": "opi-audit-history/1"
}
-->

## Post-audit historical reference — advisory

This section was appended after installation of the independent core. It cannot change findings, requirements, independence or verdict. Historical closure claims require current independent verification.

- Owning audit run: phase19&#45;pi&#45;glm53&#45;2e82c99&#45;20261002t204027z
- Audit head: 2e82c99aa32ca3a56acef3e2b969223161d370c7
- Core report bytes: 13473
- Core report SHA-256: 9b27aab9b61f26cc4df095c2847b9652584b273aa38647e4a1da23b4018d24b6
- Current findings SHA-256: d8dd6dd9833a5575975171b815f790d5c1c26ae3df4d5b94b21a84663cb8c7d1
- Comparison status: compared
- Inspected history: Compared only the immediately superseded run of this same &#40;pi, glm53&#41; pair &#45; audit&#95;run&#95;id phase19&#45;pi&#45;glm53&#45;efe2f69&#45;20261001t043357z, sealed at audit&#95;head efe2f6906fa47003bf806bddec0b15f2d20dd3c3 and last carried by commit 2465aeb80ad84d06172acc142f6d5ffbceb201cc &#45; against this member&#39;s current core sealed at audit&#95;head 2e82c99aa32ca3a56acef3e2b969223161d370c7&#46; Every other history record &#40;the older phase19&#45;pi&#45;glm53&#45;ac80281 run, the codex&#45;gpt6 run, unrelated remediation material&#41; and all concurrent sibling output stayed outside the comparison&#46; Current behavior is anchored to the current audit&#95;head only&#46;
- Limitations: Comparison bounded to the single superseded same&#45;pair run cited above; no other history record, sibling peer output or remediation material was ingested&#46; The prior run sealed a finer&#45;grained requirement set &#40;26 records including P19&#45;R&#45;GATES, then not&#45;met, and P19&#45;R&#45;AC011, then partially&#45;met&#41; whose failing causes map to the two prior findings verified here as repaired; this member independently sealed 13 AC&#45;level requirements, all met at the current head&#46; No newly noticed post&#45;install defect absent from the current sidecar arose within this comparison&#39;s scope&#46; The prior verdict FAIL and its closure claims are historical attributes only; the installed core verdict stands on current evidence&#46;

### Pinned historical sources

- prior&#45;report: docs/snapshots/phase19/assurance/history/phase19&#45;pi&#45;glm53&#45;efe2f69&#45;20261001t043357z/audit&#46;pi&#46;glm53&#46;md; commit 2465aeb80ad84d06172acc142f6d5ffbceb201cc; SHA-256 44b2c3538ef101efedf67a53e2f08d7deb5a60c82716161f986dfb85ddf326fc; audit run phase19&#45;pi&#45;glm53&#45;efe2f69&#45;20261001t043357z
- prior&#45;meta: docs/snapshots/phase19/assurance/history/phase19&#45;pi&#45;glm53&#45;efe2f69&#45;20261001t043357z/audit&#46;pi&#46;glm53&#46;meta&#46;json; commit 2465aeb80ad84d06172acc142f6d5ffbceb201cc; SHA-256 d9bf70ecba230c708c20df2d61092a63e94be93a8905697620761ae5ad2c5bf0; audit run phase19&#45;pi&#45;glm53&#45;efe2f69&#45;20261001t043357z
- prior&#45;requirements: docs/snapshots/phase19/assurance/history/phase19&#45;pi&#45;glm53&#45;efe2f69&#45;20261001t043357z/audit&#46;pi&#46;glm53&#46;requirements&#46;jsonl; commit 2465aeb80ad84d06172acc142f6d5ffbceb201cc; SHA-256 9315c6f7894ac36ccced55b081e76f0162364200fd9c81398cb55723f38a5aba; audit run phase19&#45;pi&#45;glm53&#45;efe2f69&#45;20261001t043357z
- prior&#45;findings: docs/snapshots/phase19/assurance/history/phase19&#45;pi&#45;glm53&#45;efe2f69&#45;20261001t043357z/audit&#46;pi&#46;glm53&#46;findings&#46;jsonl; commit 2465aeb80ad84d06172acc142f6d5ffbceb201cc; SHA-256 fb086ccb76ea53156b0600c6b46f8e8c1b9fb8842515f3acff8669ec69923b70; audit run phase19&#45;pi&#45;glm53&#45;efe2f69&#45;20261001t043357z

### Comparison 1: P19&#45;AUD&#45;001

- Relationship: persistent
- Prior claim (attributed): Prior Info finding: docs/eval/evidence&#45;roots&#46;jsonl entries &#40;2026&#45;09&#45;29 acceptance re&#45;collection, 2026&#45;09&#45;27 longrun&#41; carry report fields pointing at docs/eval/&#42;&#46;md files removed from the checkout on 2026&#45;10&#45;01; README documents Git&#45;history resolution but registry rows do not record the removal commit&#46;
- Current evidence: Independently re&#45;derived at audit&#95;head 2e82c99: git ls&#45;tree HEAD docs/eval/ shows only README&#46;md and evidence&#45;roots&#46;jsonl; the registry report paths for phase19&#45;acceptance&#45;20260929 and phase19&#45;longrun&#45;validation&#45;20260927 still do not exist in&#45;tree and no row pins a resolving commit&#46; Disclosure remains in docs/eval/README&#46;md and CHANGELOG Unreleased/Changed&#46; Uncertain: nothing &#45; both prior and current states were directly observed&#46;
- Cause / missed boundary: The bilingual conclusion&#45;pair contract change &#40;removal commit d42a7a0&#41; deliberately removed the detailed docs/eval/ operator reports and left the registry rows unchanged; no subsequent commit added removal&#45;commit pinning or in&#45;tree replacement reports&#46;
- Suggested current verification: Pin the resolving/removal commit per registry row &#40;or regenerate in&#45;tree conclusion pairs for currently qualifying roots&#41;, then confirm every report field resolves without a history lookup&#46;
- Prior finding: phase19&#45;pi&#45;glm53&#45;efe2f69&#45;20261001t043357z / P19&#45;AUD&#45;003; source prior&#45;findings
- Candidate raw proof: docs/eval/evidence&#45;roots&#46;jsonl; SHA-256 11d41b4745959dadcbf166be0c0349cee8e672f141fd607cff60a0de892a65be; checked: json parse of every report field plus existence check of each referenced path in the audit&#95;head export; git ls&#45;tree HEAD docs/eval/; limits: Reports remain resolvable through Git history at the removal commit; acceptance verification itself does not read the in&#45;tree report files &#40;verify&#45;local&#45;acceptance and artifact audit re&#45;run green at audit&#95;head&#41;

### Comparison 2: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Prior Major blocking finding: test&#95;native&#95;smoke&#95;output&#95;control&#46;py faked &#39;command &#45;v uv&#39; with a hardcoded /usr/bin/uv while the embedded materialize&#45;configs stage copied the real binary, erroring three tests wherever uv is absent; the registered AC&#45;011 unittest command failed at the prior audit&#95;head&#46; The remediation commit message claims this closed among eleven findings &#45; not adopted as current closure&#46;
- Current evidence: Verified at audit&#95;head 2e82c99: the test now writes synthetic executable uv bytes inside its own fixture root and the mocked &#39;command &#45;v uv&#39; returns that fixture path; /usr/bin/uv does not exist in this audit environment and &#39;python3 &#45;m unittest discover &#45;s crates/opi&#45;eval/scripts &#45;p test&#95;&#42;&#46;py&#39; ran 272 tests OK &#40;1 skipped&#41; with zero errors&#46; Uncertain: nothing material; the exact&#45;client ignored native filters remain Docker/material&#45;gated and are unrelated to this defect&#46;
- Cause / missed boundary: Remediation batch B9 replaced the hardcoded host path with fixture&#45;owned synthetic bytes, removing the host&#45;tool coupling&#46;
- Suggested current verification: Keep the registered AC&#45;011 command green on uv&#45;less hosts &#40;CI or a fresh container without uv installed&#41;&#46;
- Prior finding: phase19&#45;pi&#45;glm53&#45;efe2f69&#45;20261001t043357z / P19&#45;AUD&#45;001; source prior&#45;findings
- Repair commits: 81d26aac8772b492e7ee4586fbbb7b331f78ce6b
- Candidate raw proof: crates/opi&#45;eval/scripts/test&#95;native&#95;smoke&#95;output&#95;control&#46;py; SHA-256 43d7d9541a6f209922c944b75b31c3d0e7ebca35a07680e3427463cabda521db; checked: source read at audit&#95;head plus full unittest discover rerun on a uv&#45;less host &#40;272 OK, 1 skipped&#41;; limits: None within scope; the repair is observable on any machine without /usr/bin/uv

### Comparison 3: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Prior Minor advisory finding: cargo fmt &#45;&#45;check &#45;&#45;all exited 1 at the prior audit&#95;head with one unformatted hunk at crates/opi&#45;eval/tests/native&#95;driver&#46;rs:10054, blocking the Phase 8&#45;section verification union&#46;
- Current evidence: Verified at audit&#95;head 2e82c99: &#39;cargo fmt &#45;&#45;check &#45;&#45;all&#39; exits 0 &#40;rerun by this peer in the sealed export&#41;; the formerly multi&#45;line assert at native&#95;driver&#46;rs is now the rustfmt single&#45;line form&#46; Uncertain: nothing&#46;
- Cause / missed boundary: The same remediation commit applied the rustfmt rendering to the touched hunk&#46;
- Suggested current verification: The workspace fmt gate in CI/Phase exit continues to enforce this&#46;
- Prior finding: phase19&#45;pi&#45;glm53&#45;efe2f69&#45;20261001t043357z / P19&#45;AUD&#45;002; source prior&#45;findings
- Repair commits: 81d26aac8772b492e7ee4586fbbb7b331f78ce6b
- Candidate raw proof: crates/opi&#45;eval/tests/native&#95;driver&#46;rs; SHA-256 d996f5cab322cda6e5c451cdc170670d7955c28710288072bf3636f01434f598; checked: cargo fmt &#45;&#45;check &#45;&#45;all at audit&#95;head &#40;exit 0&#41; plus diff inspection of commit 81d26aac; limits: None within scope
