# Active Assurance Set Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every Opi audit an independent current-spec conformance run, make remediation consume only that run while permitting bounded verification-blocking incidental repairs, and retain only one Git-versioned active assurance set per Phase.

**Architecture:** Replace timestamped immutable audit/remediation siblings with fixed paths under `docs/snapshots/phase<N>/assurance/`. Seal the latest committed specification files and implementation endpoint at audit start, prohibit audit/remediation history as input, identify findings by `(audit_run_id, id)`, and bind remediation to the exact findings and plan digests. Git history preserves superseded sets; the live tree contains only the current set.

**Tech Stack:** Markdown Codex skills, Python 3 standard library validators/tests, JSON/JSONL assurance records, Git status/history, Rust repository verification commands.

**Repository rule:** Do not commit during execution unless the user separately authorizes a commit. The commit steps normally required by the generic planning skill are replaced below by explicit user materialization gates.

**Notation:** `phase<N>`, `I<N>`, and angle-bracket values in schema examples are protocol metavariables that the implementation must document; executable verification commands below bind them to concrete paths or shell variables.

---

## Confirmed decisions

1. **Current-spec baseline:** `phase=<N>` locates the ledger and registered scope, but the verdict evaluates current committed implementation against the latest committed `docs/opi-spec.md` and registered supplemental specs at `audit_head`.
2. **Independent audit:** audit may use the root ledger, pointed Phase state, current specs, current production code/tests/docs, and repository standards. It must never read prior audit or remediation content, even after sealing findings; remove history comparison entirely.
3. **Single active set:** each Phase has one fixed `assurance/` directory. A later run replaces it only after the previous set and fixes are materialized in Git.
4. **Current-run remediation:** remediation consumes exactly one `audit_run_id` and `findings_sha256`; it does not compute lineage or consult older results.
5. **Bounded incidental repair:** apply may repair a verification-blocking defect discovered by the approved plan only when it is directly causal, remains within the batch surface, changes no public/spec/authority/dependency contract, and has its own red/green evidence.
6. **Full legacy cleanup:** remove all historical audit/remediation artifacts from the live tree. Keep Phase ledgers and non-assurance snapshots. Git history remains the archive; do not migrate old conclusions into the new active set.

Derived execution invariant: audit source inspection and checks operate on an isolated checkout/export of `audit_head`, not on uncommitted tracked files in the live worktree. The live worktree receives only the validated fixed assurance outputs. This makes “current committed implementation” observable even when unrelated work is present.

## Target file structure

An audit creates or replaces the first four fixed files:

```text
docs/snapshots/phase<N>/assurance/
├── audit.meta.json
├── audit.requirements.jsonl
├── audit.findings.jsonl
└── audit.md
```

A failed audit followed by remediation may add:

```text
├── remediation.plan.md
├── remediation.plan.dispositions.jsonl
├── remediation.result.md
└── remediation.result.dispositions.jsonl
```

A later audit removes the four remediation files while replacing the audit files. A PASS set therefore contains only the four audit files. Fixed paths are mutable across Git commits but sealed within a run by content digests and `audit_run_id`.

## Machine contracts

`audit.meta.json` is the run root:

```json
{
  "schema_version": 1,
  "audit_run_id": "phase17-21eaacf-20260824t010203z",
  "phase": 17,
  "audit_head": "21eaacfcbbb9f1179ce4cbd9bee5079e62f2d520",
  "reviewer_model": "reported model identity",
  "independence": "fresh-context-same-family",
  "baseline_policy": "latest-committed-spec",
  "baseline_sources": [
    {"path": ".opi-impl-state.json", "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},
    {"path": "docs/snapshots/phase17/opi-impl-state.json", "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"},
    {"path": "docs/opi-spec.md", "sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"}
  ],
  "requirements_sha256": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
  "findings_sha256": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
  "verdict": "FAIL"
}
```

Each requirement record is independently decidable:

```json
{
  "audit_run_id": "phase17-21eaacf-20260824t010203z",
  "id": "P17-A15",
  "mandatory": true,
  "criterion_source": {
    "path": "docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md",
    "sha256": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    "citation": "P17-A15"
  },
  "observable_behavior": "The acceptance suite passes on Linux, macOS, and Windows.",
  "production_surfaces": [".github/workflows/ci.yml"],
  "test_evidence": ["phase17_api_audit"],
  "checks": [{"command": "gh run list --commit 21eaacfcbbb9f1179ce4cbd9bee5079e62f2d520 --json databaseId,headSha,status,conclusion", "observed": "no run"}],
  "state": "not-assessable",
  "finding_ids": ["P17-AUD-001"]
}
```

Finding identity no longer depends on a historical filename:

```json
{
  "audit_run_id": "phase17-21eaacf-20260824t010203z",
  "id": "P17-AUD-001",
  "source_kind": "audit",
  "source_path": "docs/snapshots/phase17/assurance/audit.codex.gpt56.md",
  "source_model": "gpt-5.6",
  "observed_at": "21eaacfcbbb9f1179ce4cbd9bee5079e62f2d520",
  "independence": "fresh-context-same-family",
  "axis": "test-quality",
  "severity": "Major",
  "conformance_effect": "blocks",
  "title": "Current-head three-platform evidence is absent",
  "claim": "The mandatory current-head CI route has not executed.",
  "evidence": [{"location": "GitHub Actions query", "detail": "No run exists."}],
  "requirement_ids": ["P17-A15", "P17-PLT-001"],
  "criterion_source": "registered Phase 17 platform requirements",
  "reproduction": ["gh run list --commit 21eaacfcbbb9f1179ce4cbd9bee5079e62f2d520 --json databaseId,headSha,status,conclusion"],
  "confidence": "high",
  "status": "unverified"
}
```

Requirement `state` is one of `met`, `partially-met`, `not-met`, or `not-assessable`. Finding `conformance_effect` is `blocks` or `advisory`. `Blocker` and `Major` findings must be `blocks`; every requirement linked from a blocking finding must have a state other than `met`. Advisory findings may be only `Minor` or `Info`. Finding and requirement links are reciprocal. These rules mechanically reject a report that marks a required missing check as both a Major finding and a met requirement.

Plan/result dispositions identify their one source set as:

```json
"source": {
  "audit_run_id": "phase17-21eaacf-20260824t010203z",
  "findings_sha256": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
  "id": "P17-AUD-001"
}
```

Remove `lineage`, `prior_occurrences`, and `prior_disposition`. Retain `closure_key` and `family_key` only for clustering findings inside the current set.

An incidental result record uses:

```json
{
  "record_kind": "incidental-repair",
  "id": "I1",
  "trigger_batch": "B2",
  "blocking_check": "cargo test --workspace --all-targets",
  "scope_rationale": "The collision prevents the approved B2 workspace gate from passing.",
  "guardrails": {
    "required_for_green": true,
    "within_causal_surface": true,
    "changes_public_api": false,
    "changes_durable_format": false,
    "changes_dependency_graph": false,
    "changes_spec_or_authority": false
  },
  "changed_paths": ["crates/opi-coding-agent/src/session_coordinator.rs"],
  "red_before": {"command": "cargo test --workspace --all-targets", "expected": "FAIL", "observed": "FAIL"},
  "green_after": {"command": "cargo test --workspace --all-targets", "expected": "PASS", "observed": "PASS"},
  "remediation_status": "Closed"
}
```

---

### Task 0: Preserve reproducible pre-change skill baselines

**Files:**
- Create ignored workspace: `.agents/skills/opi-audit-workspace/skill-snapshot/`
- Create ignored workspace: `.agents/skills/opi-remediate-workspace/skill-snapshot/`
- Copy into both snapshots: `.agents/skills/_shared/`

- [ ] **Step 1: Confirm the evaluation workspaces are absent and ignored**

Run:

```powershell
$auditWorkspace = '.agents/skills/opi-audit-workspace'
$remediateWorkspace = '.agents/skills/opi-remediate-workspace'
if ((Test-Path -LiteralPath $auditWorkspace) -or (Test-Path -LiteralPath $remediateWorkspace)) {
    throw 'existing skill evaluation workspace requires explicit reuse or a new path'
}
git check-ignore -v -- $auditWorkspace $remediateWorkspace
```

Expected: both paths match `.gitignore`; neither exists.

- [ ] **Step 2: Snapshot each original skill and its shared contracts before editing**

Run:

```powershell
New-Item -ItemType Directory -Path "$auditWorkspace/skill-snapshot" | Out-Null
New-Item -ItemType Directory -Path "$remediateWorkspace/skill-snapshot" | Out-Null
Copy-Item -LiteralPath '.agents/skills/opi-audit' -Destination "$auditWorkspace/skill-snapshot/opi-audit" -Recurse
Copy-Item -LiteralPath '.agents/skills/opi-remediate' -Destination "$remediateWorkspace/skill-snapshot/opi-remediate" -Recurse
Copy-Item -LiteralPath '.agents/skills/_shared' -Destination "$auditWorkspace/skill-snapshot/_shared" -Recurse
Copy-Item -LiteralPath '.agents/skills/_shared' -Destination "$remediateWorkspace/skill-snapshot/_shared" -Recurse
```

- [ ] **Step 3: Verify snapshot creation did not change tracked scope**

Run `git status --short` and record its output. Expected: the workspaces remain ignored and the only pre-existing untracked files are the four Phase 17 audit artifacts plus this plan.

---

### Task 1: Pin the active-set validator contract with failing tests

**Files:**
- Modify: `scripts/test_opi_assurance_skills.py`
- Test: `scripts/test_opi_assurance_skills.py`

- [ ] **Step 1: Replace immutable-name fixtures with active-set helpers**

Add helpers that create `phase17/assurance/`, write `audit.meta.json`, `audit.requirements.jsonl`, `audit.findings.jsonl`, and `audit.md`, and calculate SHA-256 over exact file bytes:

```python
import hashlib


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_jsonl(path: Path, records: list[dict[str, object]]) -> None:
    path.write_text(
        "".join(json.dumps(record, sort_keys=True) + "\n" for record in records),
        encoding="utf-8",
    )
```

- [ ] **Step 2: Add audit-set success and verdict-consistency tests**

Add these exact tests and assertions:

- `test_current_audit_set_passes_when_meta_digests_and_verdict_match`: construct a four-file set with recomputed digests and assert exit 0 plus `audit-set: PASS`.
- `test_fail_is_required_when_one_mandatory_requirement_is_not_assessable`: mark one mandatory requirement `not-assessable`, declare `PASS-WITH-FINDINGS`, and assert rejection for verdict mismatch.
- `test_pass_with_findings_requires_all_mandatory_requirements_met`: mark every mandatory requirement `met`, include one Major finding, and assert the derived verdict is `PASS-WITH-FINDINGS`.
- `test_finding_requirement_ids_must_exist_in_requirement_sidecar`: reference a missing requirement ID and assert rejection.
- `test_every_finding_and_requirement_must_share_audit_run_id`: change one record's run ID and assert rejection.
- `test_findings_identity_is_audit_run_id_plus_id_not_source_path`: use the fixed source path with unique IDs and assert acceptance; duplicate `(audit_run_id, id)` and assert rejection.
- `test_blocking_finding_rejects_met_linked_requirement`: link a Major blocking finding to a mandatory `met` requirement and assert rejection.
- `test_audit_set_rejects_timestamped_legacy_sibling_names`: add a Phase-level timestamped audit sibling and assert rotation/audit-set rejection.

The second test must reproduce the current GLM contradiction: meta says `PASS-WITH-FINDINGS`, `P17-A15` says `met`, while its linked finding says the required check did not execute. Represent the requirement honestly as `not-assessable` and assert the validator rejects the false verdict.

- [ ] **Step 3: Add current-run remediation identity tests**

- `test_plan_source_requires_exact_audit_run_id_and_findings_digest`: assert exact match passes and either mismatch fails.
- `test_plan_cannot_reference_a_different_or_older_audit_run`: supply a well-formed older ID and assert rejection.
- `test_disposition_contract_contains_no_lineage_fields`: assert `lineage`, `prior_occurrences`, and `prior_disposition` are rejected.
- `test_plan_digest_changes_when_plan_or_dispositions_change`: mutate each input independently and assert the emitted digest changes.

- [ ] **Step 4: Add bounded-incidental and result drift tests**

- `test_result_accepts_verification_blocking_bounded_incidental_repair`: provide all guardrails plus observed red/green and assert acceptance.
- `test_incidental_repair_rejects_public_api_change`: set `changes_public_api` true and assert rejection.
- `test_incidental_repair_rejects_dependency_or_spec_change`: name a manifest and a registered spec in separate subtests and assert rejection.
- `test_incidental_repair_requires_observed_fail_and_pass`: omit each observation in separate subtests and assert rejection.
- `test_unrecorded_result_change_is_rejected`: report a changed path absent from planned and incidental records and assert rejection.

- [ ] **Step 5: Add rotation admission tests in a temporary Git repository**

Create a temporary repository with `git init`, configure a local test identity, commit an assurance set, and test:

- `test_rotation_passes_when_active_set_is_tracked_and_clean`: commit the fixed set and assert acceptance.
- `test_rotation_rejects_uncommitted_active_set_changes`: modify a tracked assurance file and assert rejection.
- `test_rotation_rejects_untracked_active_set_files`: add an untracked assurance file and assert rejection.
- `test_rotation_rejects_legacy_audit_or_remediation_siblings`: add each forbidden sibling class in subtests and assert rejection.

- [ ] **Step 6: Run the new tests and observe the expected red state**

Run:

```powershell
python scripts/test_opi_assurance_skills.py
```

Expected: FAIL because the validator still recognizes timestamped immutable names, has no `audit-set`/`result`/`rotation` commands, and still requires lineage.

---

### Task 2: Implement fixed-path audit-set validation and independent source identity

**Files:**
- Modify: `.agents/skills/_shared/references/finding-contract.md`
- Create: `.agents/skills/_shared/references/audit-set-contract.md`
- Modify: `.agents/skills/_shared/scripts/validate_assurance_artifact.py`
- Modify: `scripts/test_opi_assurance_skills.py`

- [ ] **Step 1: Replace the finding identity contract**

Document fixed `source_path`, mandatory `audit_run_id`, `requirement_ids`, full-SHA `observed_at`, and source identity `(audit_run_id, id)`. Remove immutable filename and historical `(source_path, id)` language.

- [ ] **Step 2: Define the audit meta and requirement schemas**

Write `audit-set-contract.md` with the exact structures shown in this plan, the four fixed paths, latest-committed-spec baseline semantics, digest calculation over raw bytes, and verdict derivation:

```text
FAIL                if any mandatory state != met
PASS-WITH-FINDINGS  if all mandatory states == met and any non-Info finding exists
PASS                if all mandatory states == met and no non-Info finding exists
```

Info findings never change requirement state and do not make the verdict actionable.

- [ ] **Step 3: Add validator commands**

Extend the CLI choices to:

```python
choices=("audit-set", "findings", "requirements", "dispositions", "plan", "result", "rotation")
```

Implement four focused functions:

- `validate_audit_set(directory: Path) -> list[str]` validates the four-file set and derived verdict.
- `validate_requirements(path: Path, meta: dict[str, Any]) -> list[str]` validates requirement schema, run identity, cross-references, and state.
- `validate_result(path: Path) -> list[str]` validates planned coverage, incidental records, and changed-path coverage.
- `validate_rotation(phase_directory: Path) -> list[str]` validates only Git path state and forbidden sibling names.

`validate_audit_set` must load all four siblings, recompute both sidecar digests, enforce one run/head/phase, cross-check reciprocal finding/requirement IDs and conformance effects, and derive the verdict. It may tolerate the four known fixed remediation siblings without reading them during audit rotation, but rejects every other file in `assurance/`.

`validate_rotation` must inspect only Git path state, not old report content. It passes when the `assurance/` directory is absent or fully tracked and clean at HEAD. It fails on staged, unstaged, or untracked assurance paths and on any legacy audit/remediation sibling present in HEAD, the index, the worktree, or Git status; an uncommitted deletion therefore does not satisfy rotation.

- [ ] **Step 4: Preserve standalone sidecar diagnostics**

Keep `findings` and `requirements` subcommands for focused error reporting, but require the fixed filenames `audit.findings.jsonl` and `audit.requirements.jsonl`. A successful audit must run `audit-set`, not only the sidecars.

- [ ] **Step 5: Run focused validator tests**

Run:

```powershell
python scripts/test_opi_assurance_skills.py
```

Expected: audit-set, identity, verdict, and rotation tests PASS; remediation/incidental tests remain red until Task 3.

---

### Task 3: Implement current-run remediation and bounded incidental validation

**Files:**
- Modify: `.agents/skills/_shared/references/remediation-disposition-contract.md`
- Modify: `.agents/skills/_shared/scripts/validate_assurance_artifact.py`
- Delete: `.agents/skills/_shared/scripts/compare_finding_lineage.py`
- Modify: `scripts/test_opi_assurance_skills.py`

- [ ] **Step 1: Remove historical lineage from dispositions**

Delete `lineage` from required fields and delete all recurrence classifications. Retain source verification, final severity, current-set `closure_key`/`family_key`, decision, batch, red-before, and green-after.

- [ ] **Step 2: Bind plans to the current audit set**

Require every plan disposition source to match the current `audit.meta.json` run ID and exact `audit.findings.jsonl` SHA-256. Require every current finding to appear exactly once in a fix or exclusion; reject extra/older source identities.

- [ ] **Step 3: Compute and report the approval digest**

Define:

```python
def plan_digest(plan: Path, dispositions: Path) -> str:
    digest = hashlib.sha256()
    digest.update(plan.read_bytes())
    digest.update(b"\0")
    digest.update(dispositions.read_bytes())
    return digest.hexdigest()
```

`python .agents/skills/_shared/scripts/validate_assurance_artifact.py plan docs/snapshots/phase17/assurance/remediation.plan.md` prints `plan: PASS plan_sha256=` followed by the computed 64-character lowercase digest. Apply approval must name both the fixed path and that emitted digest.

- [ ] **Step 4: Validate planned and incidental result records**

Allow `record_kind: finding-disposition` for planned records and `record_kind: incidental-repair` only in result dispositions. Enforce all six guardrail booleans, changed paths, trigger batch, blocking check, and observed FAIL/PASS. Reject incidental records naming Cargo manifests, lockfiles, registered specs, `.opi-impl-state.json`, public schemas, or a declared public API change.

- [ ] **Step 5: Validate exact result coverage**

The result must contain every plan source exactly once. Every changed path reported by the result must belong to either a planned record or an accepted incidental record. A narrative-only “verification-discovered correction” therefore fails validation.

- [ ] **Step 6: Remove lineage tests and make the suite green**

Delete `CompareFindingLineageTests`, the `LINEAGE` constant, and all recurrence fixtures. Run:

```powershell
python scripts/test_opi_assurance_skills.py
```

Expected: all validator, rotation, current-source, plan-digest, and bounded-incidental tests PASS.

---

### Task 4: Rewrite `opi-audit` as an independent current-spec audit

**Files:**
- Modify: `.agents/skills/opi-audit/SKILL.md`
- Modify: `.agents/skills/opi-audit/references/audit-proof-obligations.md`
- Modify: `.agents/skills/opi-audit/references/finding-template.md`
- Modify: `.agents/skills/opi-audit/evals/evals.json`

- [ ] **Step 1: Replace immutable-run outputs with the active-set contract**

Name the four fixed outputs and require `validate_assurance_artifact.py rotation <phase-directory>` before reading requirements. State that rotation checks path state only and never loads old report content.

- [ ] **Step 2: Define, seal, and isolate the latest committed baseline**

At `audit_head`, hash the root ledger, pointed Phase state, `docs/opi-spec.md`, and every currently registered supplemental source. Use their current committed contents even if a stored historical hash differs. Record a hash mismatch as current metadata evidence when applicable, but never substitute an older source revision. Inspect source and execute checks in a temporary detached worktree/export of `audit_head`; do not read tracked implementation/spec content from the live worktree.

- [ ] **Step 3: Add a strict history denylist**

Prohibit reading or searching `docs/snapshots/phase*/audit*`, `remediation*`, or the existing `assurance/` contents. Remove “history comparison”, recurrence annotation, and consensus language. If a broad search surfaces old conclusions before sealing, invalidate and restart the run.

- [ ] **Step 4: Make requirements machine-readable before implementation inspection**

Write `audit.requirements.jsonl` before reading production implementation. After inspection, fill evidence/check/state/finding IDs without changing the sealed requirement set. A mandatory item with unavailable required evidence is `not-assessable`.

- [ ] **Step 5: Stage and validate all four outputs outside the live Phase**

Create a unique OS temporary directory, write the four fixed basenames there, and run:

```text
python .agents/skills/_shared/scripts/validate_assurance_artifact.py audit-set C:\absolute\temporary\assurance
```

The displayed temporary path is illustrative; bind the actual absolute path returned by the temporary-directory API. Publish nothing to the Phase when validation fails. Report `AUDIT-INCOMPLETE`, keep no claimed PASS/FAIL verdict, and retain the previously committed active set unchanged.

- [ ] **Step 6: Publish one complete replacement and rotate remediation files**

After the staged set validates, replace the four audit files and remove the four fixed remediation plan/result files from the previous set in one controlled patch. Because rotation admission proved the previous files were committed, Git preserves them. Validate the live fixed set again with `python .agents/skills/_shared/scripts/validate_assurance_artifact.py audit-set docs/snapshots/phase17/assurance`. Publish the verdict only after this second validation. A PASS run leaves only the four audit files.

- [ ] **Step 7: Replace audit evals with boundary cases**

Include at least these prompts and objective expectations:

1. A previous assurance set exists: audit checks only Git cleanliness, never reads it, and replaces it.
2. The current spec changed after Phase exit: the latest committed spec is hashed and applied.
3. One mandatory current-head CI check is absent: requirement is `not-assessable`, verdict `FAIL`.
4. A malformed finding sidecar: audit reports `AUDIT-INCOMPLETE`, not a verdict.
5. Old audit files are present: rotation admission refuses before auditing.

- [ ] **Step 8: Run documentation contract tests after Task 6 updates the checker**

Do not run `opi-doc-check.py` yet; its old expectations intentionally fail until Task 6.

---

### Task 5: Rewrite `opi-remediate` around one current audit set

**Files:**
- Modify: `.agents/skills/opi-remediate/SKILL.md`
- Rewrite: `.agents/skills/opi-remediate/references/cross-reference-matrix.md`
- Modify: `.agents/skills/opi-remediate/references/remediation-plan-template.md`
- Modify: `.agents/skills/opi-remediate/references/execution-protocol.md`
- Modify: `.agents/skills/opi-remediate/evals/evals.json`

- [ ] **Step 1: Simplify invocation and source admission**

Use:

```text
$opi-remediate mode=plan phase=<N>
$opi-remediate mode=apply phase=<N> plan_sha256=<64-hex>
```

The source is always the fixed active audit set. Remove `sources=<path,...>`, historical defaulting, rounds, and lineage comparison.

- [ ] **Step 2: Replace cross-history lineage with current-set closure grouping**

Rewrite `cross-reference-matrix.md` to cover only current finding verification, closure predicates, family grouping inside the current set, conflicting current claims, and coverage across current reviewers when a single audit deliberately contains more than one reviewer. It must not reference previous runs.

- [ ] **Step 3: Fix plan and result paths**

Use the four fixed remediation paths. The plan header records audit run ID, findings digest, remediation head, and unresolved decisions. The validator supplies the approval digest; the user approves that digest.

- [ ] **Step 4: Add the bounded incidental repair loop**

When a recorded verification command exposes a new blocking defect:

1. prove it is required for the current green-after;
2. check every guardrail;
3. observe a focused red-before;
4. make the minimum repair;
5. record `I<N>` in result dispositions;
6. run focused green and resume the planned union.

If any guardrail fails, stop and create a new plan rather than editing. Non-blocking observations are reported but not fixed.

- [ ] **Step 5: Enforce materialization before a later audit**

The result says a later audit is allowed only after fixes and the complete active set are committed together and the assurance directory is clean. It must not say merely “materialize and audit” while required external evidence or an owning-workflow return remains unresolved.

- [ ] **Step 6: Replace remediation evals**

Add cases for:

1. plan tries to consume a different audit run or digest — refuse;
2. result adds a narrative-only unplanned correction — refuse;
3. a workspace gate exposes a directly causal private implementation defect — accept as bounded incidental with evidence;
4. an incidental fix needs a public constructor, Cargo dependency, schema, spec, or authority decision — require a new plan;
5. active set is uncommitted — do not hand off to a new audit.

---

### Task 6: Synchronize manuals and source-derived documentation checks

**Files:**
- Modify: `.agents/skills/README.md`
- Modify: `.agents/skills/README.zh.md`
- Modify: `scripts/opi-doc-check.py`
- Modify: `scripts/test_opi_doc_check.py`

- [ ] **Step 1: Update both user manuals in lockstep**

Describe current-spec independent audit, one current remediation source, fixed active paths, Git-history retention, approval digest, bounded incidental repairs, and materialization-before-rotation. Remove immutable timestamped artifacts, history comparison, and lineage wording.

- [ ] **Step 2: Rewrite `ASSURANCE_WORKFLOW_CONTRACT` tokens**

Pin tokens that prove the new behavior, including:

```python
"audit.meta.json"
"audit.requirements.jsonl"
"audit.findings.jsonl"
"latest-committed-spec"
"validate_assurance_artifact.py audit-set"
"audit_run_id"
"findings_sha256"
"plan_sha256"
"bounded incidental repair"
"validate_assurance_artifact.py rotation"
```

Remove checks for timestamp names, `compare_finding_lineage.py`, and recurrence labels.

- [ ] **Step 3: Update documentation-check unit fixtures**

Mirror the contract-token changes in `scripts/test_opi_doc_check.py` and add a negative test proving reintroduction of `compare_finding_lineage.py` or timestamped audit names is detected.

- [ ] **Step 4: Run focused documentation tests**

Run:

```powershell
python scripts/test_opi_doc_check.py
python scripts/opi-doc-check.py
```

Expected: both PASS.

---

### Task 7: Remove legacy assurance artifacts from every Phase

**Files:**
- Delete: the 61 tracked files under `docs/snapshots/phase1` through `phase17` whose basename starts with `audit` or `remediation`
- Delete untracked:
  - `docs/snapshots/phase17/audit.codex-gpt-5.21eaacf.20260823t142359z.md`
  - `docs/snapshots/phase17/audit.codex-gpt-5.21eaacf.20260823t142359z.findings.jsonl`
  - `docs/snapshots/phase17/audit.glm-5-3-1m.21eaacf.20260823t142211z.md`
  - `docs/snapshots/phase17/audit.glm-5-3-1m.21eaacf.20260823t142211z.findings.jsonl`
- Retain: every `docs/snapshots/phase<N>/opi-impl-state.json` and all non-assurance evidence

- [ ] **Step 1: Recompute and verify the exact deletion inventory**

Run:

```powershell
$assurancePaths = @(rg --files docs/snapshots | rg '(^|[\\/])(audit[^\\/]*|remediation[^\\/]*)($|\.)')
$tracked = @($assurancePaths | Where-Object { git ls-files --error-unmatch -- $_ 2>$null })
$untracked = @($assurancePaths | Where-Object { -not (git ls-files --error-unmatch -- $_ 2>$null) })
"all=$($assurancePaths.Count) tracked=$($tracked.Count) untracked=$($untracked.Count)"
```

Expected before deletion: `all=65 tracked=61 untracked=4`. Resolve each path and verify it remains under `D:\Luiz\Odradek\opi\docs\snapshots\phase1` through `phase17`. Verify repository recovery with `git show HEAD:docs/snapshots/phase1/audit.gpt5.5.md`.

- [ ] **Step 2: Delete the 61 tracked artifacts with explicit `apply_patch` deletions**

Do not use `git clean`, recursive deletion, a wildcard deletion command, or modify any `opi-impl-state.json`.

- [ ] **Step 3: Delete the four exact untracked invalid current-audit paths**

Use explicit literal paths only after rechecking they are still the same four files. Report that these untracked files were never materialized in Git and are not recoverable from repository history.

- [ ] **Step 4: Prove the live tree contains no legacy assurance artifact**

Run:

```powershell
$remaining = @(rg --files docs/snapshots | rg '(^|[\\/])(audit[^\\/]*|remediation[^\\/]*)($|\.)')
if ($remaining.Count -ne 0) { $remaining; exit 1 }
```

Expected: exit 0 and no output.

- [ ] **Step 5: Confirm retained Phase history owners**

Run:

```powershell
Get-ChildItem docs/snapshots -Directory | ForEach-Object {
    $state = Join-Path $_.FullName 'opi-impl-state.json'
    if (-not (Test-Path -LiteralPath $state)) { throw "missing $state" }
}
```

Expected: PASS for every registered Phase directory.

---

### Task 8: Evaluate the revised skills against the Task 0 baselines

**Files:**
- Modify: `.agents/skills/opi-audit/evals/evals.json`
- Modify: `.agents/skills/opi-remediate/evals/evals.json`
- Use ignored sibling workspaces: `.agents/skills/opi-audit-workspace/` and `.agents/skills/opi-remediate-workspace/`

- [ ] **Step 1: Verify the Task 0 baselines before running evals**

Require both `skill-snapshot/` directories and their `_shared/` copies. Refuse to reconstruct a baseline from the post-change skill or to continue without the pre-change snapshot.

- [ ] **Step 2: Build one isolated repository fixture per eval run**

For each old/new run, expand `git archive HEAD` into that run's ignored `repo/` directory. Overlay either the Task 0 skill plus `_shared` baseline or the revised skill plus revised `_shared`; apply only the fixture mutations named by that eval. Never point an eval at the root worktree, so fixed assurance paths cannot collide with real or sibling runs. For the first revised audit case, the concrete setup is:

```powershell
$runRoot = '.agents/skills/opi-audit-workspace/iteration-1/eval-0/with_skill'
$runRepo = "$runRoot/repo"
New-Item -ItemType Directory -Path $runRepo | Out-Null
git archive --format=zip --output "$runRoot/repo.zip" HEAD -- . ':(exclude).agents/skills/opi-audit' ':(exclude).agents/skills/_shared'
Expand-Archive -LiteralPath "$runRoot/repo.zip" -DestinationPath $runRepo
Copy-Item -LiteralPath '.agents/skills/opi-audit' -Destination "$runRepo/.agents/skills" -Recurse
Copy-Item -LiteralPath '.agents/skills/_shared' -Destination "$runRepo/.agents/skills" -Recurse
Copy-Item -LiteralPath '.agents/skills/README.md' -Destination "$runRepo/.agents/skills/README.md" -Force
Copy-Item -LiteralPath '.agents/skills/README.zh.md' -Destination "$runRepo/.agents/skills/README.zh.md" -Force
Copy-Item -LiteralPath 'scripts/opi-doc-check.py' -Destination "$runRepo/scripts/opi-doc-check.py" -Force
Copy-Item -LiteralPath 'scripts/test_opi_doc_check.py' -Destination "$runRepo/scripts/test_opi_doc_check.py" -Force
Copy-Item -LiteralPath 'scripts/test_opi_assurance_skills.py' -Destination "$runRepo/scripts/test_opi_assurance_skills.py" -Force
```

Use the same layout with `old_skill` and the Task 0 snapshot for the baseline; use a distinct `eval-ID` directory for every case. Do not reuse or recursively clear a run repository.

- [ ] **Step 3: Run each revised eval prompt against revised and old skills**

Run one audit/remediation eval at a time, including old/new counterparts, to honor the no-concurrent-audit decision. Capture outputs, token totals, duration, and assertion results using `.agents/skills/opi-audit-workspace/iteration-1/` and `.agents/skills/opi-remediate-workspace/iteration-1/`.

- [ ] **Step 4: Grade objective assertions**

Assertions must cover: no history reads, current-spec hashes, fixed paths, verdict derivation, exact run/digest binding, bounded incidental guardrails, and materialization rotation. Do not score prose style.

- [ ] **Step 5: Generate the static review viewers**

Run:

```powershell
python 'C:\Users\Luiz\.codex\skills\skill-creator\eval-viewer\generate_review.py' '.agents/skills/opi-audit-workspace/iteration-1' --skill-name opi-audit --benchmark '.agents/skills/opi-audit-workspace/iteration-1/benchmark.json' --static '.agents/skills/opi-audit-workspace/iteration-1/review.html'
python 'C:\Users\Luiz\.codex\skills\skill-creator\eval-viewer\generate_review.py' '.agents/skills/opi-remediate-workspace/iteration-1' --skill-name opi-remediate --benchmark '.agents/skills/opi-remediate-workspace/iteration-1/benchmark.json' --static '.agents/skills/opi-remediate-workspace/iteration-1/review.html'
```

Present both HTML files to the user before any second revision. Do not run evals concurrently, even though their repositories are isolated.

- [ ] **Step 6: Apply only feedback-authorized revisions**

If the user requests changes, repeat the eval iteration. Otherwise keep the first passing revision.

---

### Task 9: Run the final verification union and hand off without committing

**Files:**
- Verify every file changed or deleted by Tasks 1–8

- [ ] **Step 1: Run exact assurance and documentation tests**

```powershell
python scripts/test_opi_assurance_skills.py
python scripts/test_opi_doc_check.py
python scripts/opi-doc-check.py
```

Expected: PASS.

- [ ] **Step 2: Run whitespace and formatting checks for outgoing changes**

```powershell
git diff --check
```

Expected: PASS.

- [ ] **Step 3: Verify cleanup and worktree scope**

```powershell
git status --short
git diff --name-status
git diff -- .agents/skills scripts docs/snapshots docs/superpowers/plans
```

Expected: only planned skill/reference/script/manual changes, the plan, and the 61 tracked artifact deletions; no Rust/Cargo/runtime changes; no legacy audit/remediation paths; no unrelated user changes touched.

- [ ] **Step 4: Record verification and test impact**

Handoff must report exact commands/results and:

```text
Test impact: update (assurance validator and documentation contract tests)
Runtime Rust impact: none
Historical assurance files: 61 tracked removed, 4 untracked invalid files removed
Recovery: tracked files remain in Git history; four untracked files are not recoverable
Commit: not created; explicit user gate remains
```

## Self-review checklist

- Spec coverage: all six confirmed decisions map to Tasks 2–7.
- Independence: no task retains audit history comparison or remediation lineage.
- Current spec: audit baseline explicitly hashes latest committed spec contents.
- Source isolation: audit evidence comes from the sealed Git tree, not unrelated live-worktree edits.
- Accumulation: fixed active paths plus full legacy cleanup leave no timestamped siblings.
- Approval: remediation remains bound by exact plan digest despite permitted incidental repair.
- Safety: incidental repairs cannot change public API, durable formats, dependencies, specs, or authority.
- Materialization: rotation refuses uncommitted active sets.
- Transactionality: a failed staged audit cannot overwrite the prior committed active set or publish a verdict.
- Verification: requirement state and verdict are machine-cross-checked; malformed artifacts cannot publish a verdict.
- Repository policy: no automatic commit, no `.opi-impl-state.json` edit, no recursive/wildcard deletion.
