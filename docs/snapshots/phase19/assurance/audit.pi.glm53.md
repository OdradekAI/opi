# Phase 19 Audit

**Audit run ID**: `phase19-pi-glm53-efe2f69-20261001t043357z`
**Audit head**: `efe2f6906fa47003bf806bddec0b15f2d20dd3c3`
**Reviewer ID**: `pi`
**Model ID**: `glm53`
**Reviewer identity**: pi coding agent (earendil-works pi-coding-agent harness)
**Reviewer model ID**: `glm53`
**Model identity source**: operator-declared
**Independence**: fresh-context-same-family — fresh context sealed before implementation inspection; no prior audit, remediation conclusion, sibling peer output, or assurance history was read before installation. The operator-supplied `model` slug is transcribed verbatim; no exact runtime model identity was inferred.
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

Stored v2 hash mismatches: none. The baseline was sealed from a `git archive`
export of `audit_head`; all five digests matched the reader projection. At
completion `git merge-base --is-ancestor efe2f69... HEAD` holds (audit_head is
HEAD; the pre-audit working-tree cleanup of the stale live `pi/glm53` member was
committed as `efe2f69` on explicit operator instruction before rotation).

## Requirement Conformance

26 sealed requirement records (see `audit.pi.glm53.requirements.jsonl`).
States: 23 `met` (five of them with recorded re-execution limitations on
ignored exact-native filters), 1 `partially-met`, 1 `not-met`, 0 `not-assessable`.

| Requirement ID | Criterion | Current evidence (condensed) | State | Finding IDs |
|---|---|---|---|---|
| P19-R-AC001-PREPARE | AC-001 prepare | doctor/prepare surfaces + preflight/acquisition tests green | met | |
| P19-R-AC001-PLAN | AC-001 plan | plan/run frozen-plan tests green | met | |
| P19-R-AC001-LIVE | AC-001 live | offline acceptance verifier `accepted:true` on registered 2026-09-29 root | met | |
| P19-R-AC002 | AC-002 | bound_native_identity rejection suite green | met | |
| P19-R-AC003-NATIVE | AC-003 witness | production-handoff fixture tests green; ignored witness filter not re-executed (material cache gone) | met (limitation) | |
| P19-R-AC003-LIVE | AC-003 live | acceptance root: both TB2.1 trials verified-native | met | |
| P19-R-AC004-NATIVE | AC-004 witness | declared-collection boundary tests green; ignored filter not re-executed | met (limitation) | |
| P19-R-AC004-LIVE | AC-004 live | acceptance root: both TB3.0 trials, native zeros | met | |
| P19-R-AC005-NATIVE | AC-005 witness | committed-patch pristine-verifier tests green; ignored filter not re-executed | met (limitation) | |
| P19-R-AC005-LIVE | AC-005 live | acceptance root: both DeepSWE trials | met | |
| P19-R-AC006-FIXTURE | AC-006 fixture | connection/control projection, retry/terminal classification code + green tests; exact-client matrix not re-executed (pi package cache cleaned) | met (limitation) | |
| P19-R-AC006-LIVE | AC-006 live | verifier-validated sealed first-connection evidence | met | |
| P19-R-AC007-MATERIAL | AC-007 material | drift/integrity rejection tests green | met | |
| P19-R-AC007-SELECTION | AC-007 selection | full-inventory/frozen-subset tests green | met | |
| P19-R-AC008-OUTCOME | AC-008 settlement | classify_native_trial + terminal-shape tests green | met | |
| P19-R-AC008-REPORT | AC-008 report | report contract/output-containment/pairing tests green | met | |
| P19-R-AC009-VERIFY | AC-009 verify | bundle recompute/tamper tests green; read-only acceptance staging accepted | met | |
| P19-R-AC009-REGRADE | AC-009 regrade | NativeRegrader code + offline_regrade tests green; ignored filter not re-executed | met (limitation) | |
| P19-R-AC010-CREDENTIALS | AC-010 credentials | credential-removal/canary/authority tests green | met | |
| P19-R-AC010-LIFECYCLE | AC-010 lifecycle | cancellation/recovery/replacement tests green; Docker cleanup witness not re-executed | met (limitation) | |
| P19-R-AC011 | AC-011 | dummy-credential test green; workflows are thin fail-closed producers; Python suite errors 3/251 on uv-less machines | partially-met | P19-AUD-001 |
| P19-R-AC012 | AC-012 | current verifier + audit_head companion accepted the six-trial root; artifact audit PASS | met | |
| P19-R-AC013 | AC-013 | package clean, bilingual docs, doc-check PASS, Unreleased records | met | |
| P19-R-GATES | spec §8 union | fmt FAIL (1 hunk); doc-check/clippy/tests/doc-tests/rustdoc green | not-met | P19-AUD-002 |
| P19-R-PLACE | placement/non-goals | no Opi deps, no LLM client, no embedded corpora/graders/images | met | |
| P19-R-MINCHANGE | proof obligations | reuse surfaces exist and consumed; legacy scoring paths removed | met | |

## Standards Review

- Workspace gates at audit_head: `cargo clippy --workspace --all-targets --
  D warnings` PASS; `cargo test --workspace --all-targets` green in the sealed
  export for 86 test binaries, with the single Git-object-dependent test
  (`artifact_audit_accepts_real_declared_commit_objects`, which resolves a real
  commit through the workspace `.git`) re-verified green in the live checkout at
  the same commit — the export lacks `.git` by construction; `cargo test
  --workspace --doc` 12 passed; `RUSTDOCFLAGS="-D warnings" cargo doc
  --workspace --no-deps` PASS; `python3 scripts/opi-doc-check.py` PASS.
- `cargo fmt --check --all` FAILS with exactly one hunk
  (`crates/opi-eval/tests/native_driver.rs:10054`); no rustfmt configuration
  exists that would preserve the committed form (finding P19-AUD-002).
- `git diff --check`: the tree at audit_head is clean; the audit's own commit
  contains only deletions.
- Dependency direction: `opi-eval` retains zero internal workspace
  dependencies and no HTTP/LLM client crate; nothing in Phase 19 touched Agent
  Core crates.
- Error handling: typed `NativeBindingError`/`ExecutionError` taxonomies with
  static redacted tokens; fail-closed refusals throughout the inspected
  binding, preflight, material, plan, regrade, and verifier paths.

## Spec Review

Every acceptance criterion of the registered Phase 19 design was traced to
current production surfaces and discriminating tests (table above). The
six-trial acceptance obligation (AC-012) is currently verifiable: the
committed `verify-local-acceptance.py`, run from the sealed export with a
companion built from `audit_head`, accepted the registered 2026-09-29
re-collection root (`accepted: true`, exit 0, six attributable trials,
companion verify/report on every family root), and
`scripts/opi-artifact-audit.py` passed on the Opi-only scoped view. The
2026-09-15 historical root's non-consumability under the current verifier is
recorded honestly in `docs/audits/phase19-local-native-acceptance.md` and
superseded by the registered re-collection.

## Security, Invariants, Integration, Test Quality, and Residuals

- **Security/authority**: credential references resolve only inside the
  supervised Agent launch; `NativeHandoff`'s `Drop` removes temporary
  credential material; canary scans cover grader/export boundaries; the
  acceptance verifier strips credential-shaped environment entries from the
  spawned companion.
- **Invariants/integration**: one settlement classifier
  (`classify_native_trial`) seals the trial interpretation; report consumers
  consume the same sealed meaning; run-level planned coverage is bound before
  effects; replacement/recovery preserve prior immutable attempts.
- **Test quality**: the dummy-credential test asserts the scripted reward
  token (not just exit 0) and byte-scans every durable file for the canary;
  binding tests inject each substitution kind; witness tests discriminate
  oracle=1 vs nop=0 with digest inequality and canary exclusion. Anti-vacuity
  holds on the inspected acceptance evidence.
- **Residuals**: no TODO/FIXME placeholders in production code; the only
  `answer.txt` remnants are the explicitly selected conformance oracle
  diagnostic and test fixtures, matching the recorded simplification ceiling.
  Registry report-link drift recorded as P19-AUD-003 (Info).
- **Re-execution limitations** (recorded, not defects): the frozen
  `/tmp/opi-p19-b6-cache-20260927` material/client cache no longer exists, so
  the ignored exact-native filters (`native_tb21/tb3/deepswe_*_witness`,
  `native_protocol_client_matrix`, `native_client_turn_budgets`,
  `native_offline_regrade`, `native_owned_resource_cancellation`,
  `remediation_native_stage_deadlines`) could not be re-run at audit time;
  their seams are covered by non-ignored fixture-backed tests, code
  inspection, and the offline-verified sealed acceptance evidence.

## Minimum-change Conformance

Status: **conforming** for all eleven audited tasks (19.1–19.11). Recorded
`reuse_search` surfaces (`NativeTrialBinding`, `ProcessSupervisor`,
`RunBundle::verify`, `ReportBuilder`, family adapters, `task_lifecycle`,
`NativeMaterial`) exist at audit_head and are consumed by the recorded
production callers and closure tests. Recorded `surface_necessity` matches: no
new crate, trait, or public SDK; new operations are CLI facades over private
modules. Recorded `net_deletion` claims verified: no `newest_job_result`,
`run_oracle_preflight`, or host-agent score route remains in production code.
19.1/19.10's recorded production/non-production consumers check out against
current callers.

## Findings

### P19-AUD-001: Script regression test couples mocked uv resolution to /usr/bin/uv and transports real bytes

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-R-AC011
- Claim: `test_native_smoke_output_control.py` fakes `command -v uv` with a
  hardcoded `/usr/bin/uv` while the embedded `native-smoke.sh`
  materialize-configs stage `shutil.copy2()`s the real binary at that path, so
  its three tests error wherever uv is absent or installed elsewhere; the
  registered P19-AC-011 mechanical verification command fails at audit_head on
  a WSL platform without `/usr/bin/uv`.
- Evidence: test intercepts the probe and unconditionally returns
  `/usr/bin/uv\n`; embedded script lines 58–65 copy the real file;
  `python3 -m unittest discover` ran 251 tests with 3 errors (both subtests of
  `test_failed_or_incomplete_probe_never_freezes_material` and
  `test_proof_precedes_material_validation_and_config_freeze`), all cascading
  from the missing `/usr/bin/uv`; `/usr/bin/uv`, `/usr/local/bin/uv`, and
  `~/.local/bin/uv` are absent in this environment.
- Refutation attempted: (1) "environmental, uv should be installed" — the
  test's own docstring claims only external commands are replaced and no
  documented prerequisite requires uv for the Python suite; the probe is
  mocked, signalling intended hermeticity that the real-byte copy breaks; (2)
  "CI is green" — verified CI never executes this suite, so CI cannot refute
  the local verification-route failure; (3) "isolated breakage" — 248/251
  tests pass, isolating the defect precisely to the mocked-probe/real-copy
  asymmetry. None of the counter-evidence defeats the claim.
- Suggested closure: make the uv transport hermetic in the test (stage a
  synthetic executable and return its path from the mocked probe), or skip
  with an explicit limitation when no real uv exists; the suite must then pass
  251/251 on a uv-less machine.

### P19-AUD-002: cargo fmt --check --all fails at audit_head

- Axis: standards
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: P19-R-GATES
- Claim: the committed state fails the Phase 19 verification-union format
  gate — `cargo fmt --check --all` exits 1 with a single unformatted hunk at
  `crates/opi-eval/tests/native_driver.rs:10054` — so the spec §8 union cannot
  pass on the final relevant state and Phase exit is blocked until it is
  formatted.
- Evidence: one `Diff in` hunk workspace-wide; rustfmt 1.9.0 on the pinned
  1.97.1 toolchain; no `rustfmt.toml` overrides exist; the collapsed form is
  67 characters, so no configuration preserves the committed expansion.
- Suggested closure: apply `cargo fmt` (or format the single assert) and
  re-run `cargo fmt --check --all`.

### P19-AUD-003: Evidence-root registry cites report files removed from the checkout

- Axis: residuals
- Severity: Info
- Conformance effect: advisory
- Requirement IDs: P19-R-AC012
- Claim: `docs/eval/evidence-roots.jsonl` rows for the 2026-09-29 acceptance
  re-collection and the 2026-09-27 longrun validation carry `report` fields
  pointing at `docs/eval/*.md` files removed from the checkout on 2026-10-01
  under the new conclusion-pair contract; `docs/eval/README.md` documents the
  removal and Git-history resolution, but the registry rows do not record the
  removal commit.
- Evidence: both cited paths absent at audit_head; README documents the
  2026-10-01 removal and history resolution.
- Suggested closure: record the removal commit (or a history permalink) in
  the affected registry rows when the registry is next appended to.

## Verification Commands

| Command | Result | Requirement/finding |
|---|---|---|
| `python3 .agents/skills/_shared/scripts/validate_assurance_artifact.py rotation docs/snapshots/phase19` | PASS (after operator-instructed commit `efe2f69` cleaned the stale live member) | workflow admission |
| `python3 .agents/skills/opi-audit/scripts/read_audit_scope.py --phase 19` | PASS; 5 baseline hashes matched | baseline |
| `sha256sum` over 5 baseline sources in export | match | baseline |
| `python3 scripts/opi-doc-check.py` | PASS | P19-R-GATES, AC-013 |
| `cargo fmt --check --all` | **FAIL (1 hunk)** | P19-R-GATES / P19-AUD-002 |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | P19-R-GATES |
| `cargo test -p opi-eval --all-targets` | PASS (all binaries; native_driver 110 passed/11 ignored) | AC-001..AC-011 |
| `cargo test --workspace --all-targets` | 86 binaries ok in export; 1 `.git`-dependent test re-verified ok live at audit_head | P19-R-GATES |
| `cargo test --workspace --doc` | PASS (12) | P19-R-GATES |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | PASS | P19-R-GATES |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py'` | **251 ran: 248 pass, 3 errors, 1 opt-in skip** | P19-R-AC011 / P19-AUD-001 |
| `python3 crates/opi-eval/scripts/verify-local-acceptance.py --root <2026-09-29 registered root> --out <fresh> --companion <audit_head build>` | `accepted: true`, exit 0 | P19-R-AC012, live criteria |
| `python3 scripts/opi-artifact-audit.py <root>/opi-artifact-audit --workspace-root .` | PASS | P19-R-AC012 |
| `cargo package -p opi-eval --list --allow-dirty` | 185 files; no corpora/graders/images | P19-R-AC013/PLACE |
| Ignored exact-native filters (`*_witness`, `native_protocol_client_matrix`, `native_offline_regrade`, `native_owned_resource_cancellation`) | NOT RUN (pinned material/client caches under `/tmp` cleaned) | limitations on native witness criteria |

All checks ran inside the `git archive` export of `audit_head` except the
single `.git`-dependent coding-agent test (re-run in the clean live checkout
at the identical commit) and the acceptance-root operations, whose evidence
roots are operator-external registered paths consumed read-only.

## Verdict Rationale

One mandatory requirement is not met: P19-R-GATES fails because
`cargo fmt --check --all` exits 1 at audit_head (P19-AUD-002). One mandatory
requirement is partially met: P19-R-AC011's Python regression route errors on
uv-less machines (blocking P19-AUD-001). The mechanical member verdict is
therefore **FAIL**. Every other sealed obligation — including the six-trial
acceptance verification, native binding/settlement/report semantics,
credential and lifecycle boundaries, placement independence, and minimum-change
conformance — is met with current evidence, subject to the recorded
re-execution limitations on ignored exact-native filters.

<!-- opi-audit-history/1
{
  "audit\u005fhead": "efe2f6906fa47003bf806bddec0b15f2d20dd3c3",
  "audit\u005frun\u005fid": "phase19-pi-glm53-efe2f69-20261001t043357z",
  "comparison": {
    "comparisons": [],
    "limitations": "The no-superseded-run boundary rests on live-set membership at installation time only; the deleted prior live member's bytes were never read (independence boundary) and exist only in Git history predating this run, so no statement is made about their content. No newly noticed post-install defect arose within this supplement's scope.",
    "schema": "opi-audit-history/1",
    "scope": "This peer's complete (revision 13, audit\u005frun\u005fid phase19-pi-glm53-efe2f69-20261001t043357z) installed a fresh pi/glm53 member into a live set that contained no prior pi/glm53 entry: the earlier live member of this pair had been removed by the operator-committed working-tree cleanup efe2f69 before rotation admission, without a history move by this run. No same-pair run was superseded, so no comparison source exists for this installation. Pre-existing history records for this pair (phase19-pi-glm53-ac80281-20260928t070222z) and every other history record were not moved by this complete and stay outside the comparison boundary per contract.",
    "sources": [],
    "status": "no-history"
  },
  "core": {
    "findings\u005fsha256": "fb086ccb76ea53156b0600c6b46f8e8c1b9fb8842515f3acff8669ec69923b70",
    "meta\u005fsha256": "d9bf70ecba230c708c20df2d61092a63e94be93a8905697620761ae5ad2c5bf0",
    "report\u005fbytes": 16581,
    "report\u005fsha256": "12cfdfc3ba8abef51eebdcdfebfdeefadf4cbfd635cc5d47aa8d46c1551b8c71",
    "requirements\u005fsha256": "9315c6f7894ac36ccced55b081e76f0162364200fd9c81398cb55723f38a5aba"
  },
  "schema": "opi-audit-history/1"
}
-->

## Post-audit historical reference — advisory

This section was appended after installation of the independent core. It cannot change findings, requirements, independence or verdict. Historical closure claims require current independent verification.

- Owning audit run: phase19&#45;pi&#45;glm53&#45;efe2f69&#45;20261001t043357z
- Audit head: efe2f6906fa47003bf806bddec0b15f2d20dd3c3
- Core report bytes: 16581
- Core report SHA-256: 12cfdfc3ba8abef51eebdcdfebfdeefadf4cbfd635cc5d47aa8d46c1551b8c71
- Current findings SHA-256: fb086ccb76ea53156b0600c6b46f8e8c1b9fb8842515f3acff8669ec69923b70
- Comparison status: no&#45;history
- Inspected history: This peer&#39;s complete &#40;revision 13, audit&#95;run&#95;id phase19&#45;pi&#45;glm53&#45;efe2f69&#45;20261001t043357z&#41; installed a fresh pi/glm53 member into a live set that contained no prior pi/glm53 entry: the earlier live member of this pair had been removed by the operator&#45;committed working&#45;tree cleanup efe2f69 before rotation admission, without a history move by this run&#46; No same&#45;pair run was superseded, so no comparison source exists for this installation&#46; Pre&#45;existing history records for this pair &#40;phase19&#45;pi&#45;glm53&#45;ac80281&#45;20260928t070222z&#41; and every other history record were not moved by this complete and stay outside the comparison boundary per contract&#46;
- Limitations: The no&#45;superseded&#45;run boundary rests on live&#45;set membership at installation time only; the deleted prior live member&#39;s bytes were never read &#40;independence boundary&#41; and exist only in Git history predating this run, so no statement is made about their content&#46; No newly noticed post&#45;install defect arose within this supplement&#39;s scope&#46;

### Pinned historical sources

No relevant historical source found within the stated scope.
