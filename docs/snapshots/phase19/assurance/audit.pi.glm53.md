# Phase 19 Audit

**Audit run ID**: `phase19-pi-glm53-d61590b-20260917t152753z`
**Audit head**: `d61590b9aeae1685d605a4b86dd7a24f1151de1c`
**Reviewer ID**: `pi`
**Model ID**: `glm53`
**Reviewer identity**: pi coding agent (earendil-works pi TUI harness)
**Reviewer model ID**: `glm-5.3`
**Model identity source**: runtime-attested
**Independence**: independent-family — fresh pi runtime context sealed before implementation inspection; no prior audit, remediation, history, or sibling-peer conclusion was loaded; the only other live-set member (codex/gpt6) is a different model family, and the pi session log itself attests `modelId: glm-5.3` (provider `zai-coding-cn`)
**Baseline policy**: latest-committed-spec
**Verdict**: PASS

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| .opi-impl-state.json | d9b84dd2d85d64f0fa7b3462dbfd00f6f0a464b02393684e70840a9ecdc642a0 | current committed bytes equal the stored root-ledger hash |
| docs/snapshots/phase19/opi-impl-state.json | 35b3378bd902cc976bb348d794dd1b5d3c5f160a58769c3aeb62fea175a7b7a9 | current committed bytes equal the stored phase-ledger hash |
| docs/opi-spec.md | 1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db | current committed bytes equal the stored spec hash |
| docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md | 5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a | registered supplemental source; hash matches the ledger |
| docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.zh.md | d41c826412b0d71802327d60f953708f6cb2083436a2a2b0d99a2e52d851f194 | registered Chinese counterpart; identifiers and requirements identical to the English source |

All five registered sources were exported byte-identically with `git archive` from the sealed `audit_head` into a unique temporary directory; every inspection and check below ran in that export.

## Requirement Conformance

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| P19-AC-001 | design §7 | doctor/prepare/plan/run from external directories; independent-directory prepare tests; operator evidence root re-verified offline with an audit_head-built companion (accepted, exit 0) | met | |
| P19-AC-002 | design §7 | `import_bound_native_job` authority chain (one-new-job, aggregate shape, agent/task/selection identity incl. eval-key oracle rejection, reward domain, declared-collection and delivered-byte cross-checks); public-CLI substitution matrix green | met | |
| P19-AC-003 | design §7 | TB2.1 settle through the bound import; deterministic correct/incorrect witnesses through the production handoff; both real tb21 trials verified-native reward 1/1 in the accepted evidence root | met | |
| P19-AC-004 | design §7 | typed task-lifecycle parse with pre-spawn rejections; declared-collection binding (undeclared/missing/substituted/lifecycle-drift rejections); both real tb30 trials crossed the separate-verifier boundary | met | |
| P19-AC-005 | design §7 | pier committed-patch handoff with base/image identity; nop-zero and base-drift rejections; both real deepswe patches graded in the pristine verifier | met | |
| P19-AC-006 | design §7 | `experiment::project_connection` single owner of request identity (opi TOML `api_key_env`/pi models.json, per-api auth, per-control origins); credential/control/mode pre-effect rejections; real first-connection tool-loop evidence and sealed output-control witness in all six bundles | met | |
| P19-AC-007 | design §7 | metadata-only corpus enumeration, unmet-resource rejection without silent shrink, digest-bound frozen plans with exact-trial equality on re-expansion | met | |
| P19-AC-008 | design §7 | sealed-settlement consumption; coverage_counts with planned denominators; native zero and benchmark-valid timeout stay graded; provider failures attempted/invalid; protocol-group guard; no cross-benchmark average | met | |
| P19-AC-009 | design §7 | verify (byte/seal) vs regrade (fresh native verifier) split; fail-closed drift/artifact/network rejections; credential-stripped verifier env; derived records without source rewrites; zero Agent/provider invocations | met | |
| P19-AC-010 | design §7 | signal cancellation bounds dispatch and removes owned containers/credentials with honest uncertainty; recovery/replacement immutable with fresh identities; canary sealing guard | met | |
| P19-AC-011 | design §7 | dummy-credential determinism test green; CI workflows are thin callers; offline acceptance verifier separately identified | met | |
| P19-AC-012 | design §7 | six attributable real trials (tb21 1/1, tb30 0/0, deepswe 0/0, all comparable) over glm-5.3-flash; retained failed attempts and replacement identities; offline verifier and scoped artifact audit re-run green at audit_head | met | |
| P19-AC-013 | design §7 | no internal dependency; no product crate links opi-eval; pi-only acceptance; bilingual README lockstep (20/20 sections); Unreleased entries for the deliberate 0.x contract changes | met | P19-AUD-001 |
| P19-SCOPE-001 | design §1/§3 | delivery confined to opi-eval + docs/script/workflow paths; no Eval LLM client dependency; no docker-socket/broad host mounts; other-crate changes in the window belong to parallel-repair commits outside task-owned paths | met | |

## Standards Review

Workspace topology and dependency direction hold at `audit_head`: `opi-eval` declares no internal dependency, no product crate links it, and the crate has no HTTP/LLM client dependency (provider traffic belongs to the pinned external Agents). Error taxonomy is typed (`thiserror`-style enums with static redacted tokens at the binding boundary). Documentation lockstep holds: the crate READMEs carry 20/20 section parity, the acceptance audit exists bilingually, `scripts/opi-doc-check.py` passes, and `CHANGELOG.md` records the deliberate 0.x contract changes (native binding cutover, native-material /4, handoff /3, output-control evidence) under the existing Unreleased subsections. Legacy paths claimed deleted are absent from production code (`newest_job_result`, `import_harbor_result`/`import_pier_job_result` admission, `--preflight-only`, `native_opi_config`/`native_agent_env`); the only `answer.txt` occurrences are fixture bytes inside test modules.

## Spec Review

Each of the thirteen sealed acceptance criteria plus the scope obligation derived from the §1 parent-clause mapping was traced from its sealed observable behavior to production surfaces and discriminating tests (table above). The classification semantics of §6 hold in code: graded native outcomes (including zero and benchmark-valid timeout) keep their metric and denominator; provider/access/adapter/identity/infra/invalid-grading failures preserve attempted coverage without an Agent score; cancelled/unstarted/effect-unknown invent nothing; oracle is an explicitly selected diagnostic that never fills a pair.

## Security, Invariants, Integration, Test Quality, and Residuals

- Security/authority: credentials resolve only inside the supervised Agent launch, ride only the private handoff outside the sealed bundle and task environment, and are removed before the verifier phase; the offline regrade projects a credential-stripped environment and rejects credential-shaped verifier env keys; canary secrets block sealing/publication; no docker-socket or broad host mounts exist in the crate or integrations.
- Invariants/integration: the run-control planned coverage binds before any effect; tampered or drifted run-control blocks publication; plan re-verification requires digest, material, and exact-trial equality; family adapters import through the one bound reader; verify/report/regrade stay distinct offline operations.
- Test quality: the evidence tests assert observable rejections (typed failure kinds, non-zero exits, absence of sealed bundles, denominators, no credential bytes under the run root) rather than construction success; the substitution matrix covers seven distinct identity attacks at the spawned-CLI seam.
- Residuals: two `#[allow(dead_code)]` serde structs in `task_lifecycle.rs` implement the documented deny-unknown-fields admission pattern (fields declared to admit the pinned model, values intentionally unread) — not a defect. Seven ignored Rust filters are platform/material-gated witnesses with registered exact filters; their deterministic equivalents and the operator's real trials cover the same seams, and the limitation is recorded per requirement.

## Minimum-change Conformance

All eleven admitted tasks (19.1–19.11) were checked against their recorded `reuse_search`, `surface_necessity`, and `simplification_ceiling` at `audit_head`: every recorded interface exists at the recorded module (`benchmark::process::import_bound_native_job`, `experiment::project_connection`, `task_lifecycle` family parse, `NativeMaterial::load/load_for` local-origin manifests, `runner::plan` selection, verify/regrade split, `NativeRegrader`, cancellation/owned-resources, offline acceptance verifier, bilingual acceptance docs). Recorded net deletions are verified absent from production code. Status per task: **conforming**. The five shared decisions close at their owning interfaces with consumers exercising them. One provenance observation (consolidated landing of 19.11 task-owned paths) is recorded as finding P19-AUD-001.

## Findings

### P19-AUD-001: Phase 19 acceptance artifacts share a consolidated commit with unrelated parallel repairs

- Axis: standards
- Severity: Info
- Conformance effect: advisory
- Requirement IDs: P19-AC-013
- Claim: the 19.11 task-owned acceptance artifacts landed inside consolidated commit 4bf5fc0 that also carries unrelated opi-agent/opi-ai/opi-coding-agent changes, so per-task commit provenance for 19.11 cannot be isolated without the ledger note.
- Evidence: `git log c1218ac..d61590b --name-only` shows 221 opi-eval paths alongside 24 opi-agent, 10 opi-ai, and 36 opi-coding-agent paths in the same commits; every Phase 19 `task_owned_paths` list names only opi-eval and docs paths; the phase ledger's 19.11 session note records the consolidated landing as user-authorized.
- Refutation attempted: verified no Phase 19 task owns any non-opi-eval path and the Unreleased changelog records the actual changed contracts, so the delivery's documentation and independence are unaffected — the observation narrows to commit-granularity provenance only.
- Suggested closure: land task-owned assurance artifacts in dedicated commits when a clean tree permits; no code change required.

## Verification Commands

| Command | Result | Requirement/finding |
|---|---|---|
| `python3 scripts/opi-doc-check.py` (sealed export) | PASS | AC-013, P19-SCOPE-001 |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py'` (sealed export) | PASS (205 OK, 1 skipped) | AC-011, AC-012 tooling |
| `cargo test -p opi-eval` (sealed export, audit_head) | PASS (273 passed, 0 failed, 7 ignored) | AC-001..AC-011, AC-013 |
| `python3 crates/opi-eval/scripts/verify-local-acceptance.py --root phase19-operator/evidence-root --out <fresh> --companion <audit_head build>` | PASS (accepted true, exit 0; six trials, three families) | AC-012, AC-001 |
| `python3 scripts/opi-artifact-audit.py --workspace-root . <root>/opi-artifact-audit` | PASS | AC-012 |
| `git merge-base --is-ancestor d61590b HEAD` | PASS (re-checked at installation) | context boundary |

Ignored platform/material-gated filters (`native_tb21_artifact_witness`, `native_tb3_artifact_witness`, `native_deepswe_patch_witness`, `native_protocol_client_matrix`, `native_output_control_client_matrix`, `native_owned_resource_cancellation`) were not re-run by this audit; the ledger records their explicit local executions (19.2 protocol matrix PASSED 142 s; 19.9 Docker witness PASSED) and their deterministic production-seam equivalents are green in the run above.

## Verdict Rationale

All fourteen sealed mandatory requirements are `met` at `audit_head` with current evidence: the full opi-eval suite passes in the sealed export, the offline acceptance verifier and scoped artifact audit re-accept the operator's six-trial evidence with a companion built from `audit_head`, and the spec's classification, identity, authority, and reporting semantics were traced in production code with discriminating tests. One advisory Info finding (P19-AUD-001, consolidated-commit provenance) is published; it does not block conformance. Per the fail-dominant member rule (no mandatory failure, no non-Info finding), this member's verdict is **PASS**.
