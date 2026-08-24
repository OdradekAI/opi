# Multi-Model Active Audit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the single unsuffixed Phase audit with an indexed generation of equal active reviewer/model audit groups, then make remediation validate and consume the strict union of their findings.

**Architecture:** `audit.index.json` is the only active-membership authority and binds every reviewer/model four-file group to one Phase, generation, and committed head. A new Python generation coordinator stages peer outputs independently, serializes completion/publication with a repository-scoped lock and compare-and-swap, and journal-recovers interrupted publication. The existing validator remains the schema and remediation admission authority, extended to load all indexed members and derive one strict aggregate verdict.

**Tech Stack:** Python 3 standard library, JSON/JSONL, Markdown workflow contracts, Git path-state inspection, `unittest`.

---

## Scope and working-tree guard

This plan implements the approved design in
`docs/superpowers/specs/2026-08-24-multi-model-active-audit-design.md`.
It is a deliberate 0.x assurance-format break. It does not change Rust,
Cargo, provider selection, or `docs/opi-spec.md`.

The current worktree already contains relevant uncommitted assurance changes.
They are part of the implementation base and must not be discarded, stashed,
or replaced by an isolated worktree made only from `HEAD`. Before every task,
inspect `git status --short`, preserve unrelated changes, and edit only the
paths named by that task.

Every commit step below is conditional. Run it only after the user explicitly
authorizes commits; otherwise leave the verified changes uncommitted and move
to the next task.

## File responsibility map

### Create

- `.agents/skills/_shared/scripts/assurance_generation.py`: cross-platform
  staging registry, reviewer/model slot completion, repository lock,
  compare-and-swap publication, recovery journal, and generation history.
- `scripts/test_opi_assurance_generation.py`: subprocess and recovery tests for
  generation initialization, concurrent completion, publication, rollback, and
  history.

### Modify

- `.agents/skills/_shared/scripts/validate_assurance_artifact.py`: index,
  member, aggregate verdict, multi-source disposition, plan, result, and
  rotation validation.
- `scripts/test_opi_assurance_skills.py`: indexed audit fixtures and focused
  multi-member validator/remediation tests.
- `.agents/skills/_shared/references/audit-set-contract.md`: durable index,
  member schema, aggregate verdict, staging, publication, recovery, rotation,
  and history contract.
- `.agents/skills/_shared/references/finding-contract.md`: reviewer/model
  finding path and source identity rules.
- `.agents/skills/_shared/references/remediation-disposition-contract.md`:
  generation/index plan binding and union coverage.
- `.agents/skills/opi-audit/SKILL.md`: reviewer/model identity inputs,
  generation participation, independent staging, and coordinated publication.
- `.agents/skills/opi-audit/agents/openai.yaml`: explicit reviewer/model audit
  invocation description; it must not select a model.
- `.agents/skills/opi-audit/references/finding-template.md`: dynamic four-file
  names and generation/report headers.
- `.agents/skills/opi-audit/references/audit-proof-obligations.md`: dynamic
  requirements sidecar naming.
- `.agents/skills/opi-audit/evals/evals.json`: multi-model generation admission
  and failure cases.
- `.agents/skills/opi-remediate/SKILL.md`: consume the index and all member
  metadata/findings sidecars.
- `.agents/skills/opi-remediate/agents/openai.yaml`: generation-wide
  remediation description.
- `.agents/skills/opi-remediate/references/cross-reference-matrix.md`: strict
  finding union, criterion matrix, duplicate closure, and severity conflicts.
- `.agents/skills/opi-remediate/references/remediation-plan-template.md`:
  generation/index headers and full source identity.
- `.agents/skills/opi-remediate/references/execution-protocol.md`: apply/result
  binding to the current index digest.
- `.agents/skills/opi-remediate/evals/evals.json`: cross-reviewer coverage and
  index-drift cases.
- `scripts/opi-doc-check.py`: source-derived tokens for the new assurance
  contract and removal of obsolete single-file prohibitions.
- `scripts/test_opi_doc_check.py`: exact mirror tests for the documentation
  contract.
- `.agents/skills/README.md` and `.agents/skills/README.zh.md`: bilingual
  lifecycle and durable-artifact ownership.
- `CHANGELOG.md`: one `Unreleased / Breaking Changes` entry for the durable
  path and remediation-input break.

Do not edit `.claude/skills/` separately. It is the compatibility symlink to
`.agents/skills/`.

### Task 1: Specify indexed audit fixtures and fail the old validator

**Files:**
- Modify: `scripts/test_opi_assurance_skills.py:15-744`
- Test: `scripts/test_opi_assurance_skills.py`

- [ ] **Step 1: Generalize fixture record identities**

Change `requirement_record` and `finding_record` to accept an explicit
`audit_run_id`. Change `finding_record` to accept `source_path`,
`source_model`, and `observed_at`. Keep the existing defaults so the old tests
remain readable:

```python
def requirement_record(
    *,
    audit_run_id: str = RUN_ID,
    requirement_id: str = "P17-A1",
    mandatory: bool = True,
    state: str = "met",
    finding_ids: list[str] | None = None,
) -> dict[str, object]:
    record = {
        "audit_run_id": audit_run_id,
        "id": requirement_id,
        "mandatory": mandatory,
        "criterion_source": {
            "path": "docs/opi-spec.md",
            "sha256": "a" * 64,
            "citation": requirement_id,
        },
        "observable_behavior": "The registered behavior is present.",
        "production_surfaces": ["crates/opi-agent/src/lib.rs"],
        "test_evidence": ["phase17_api_audit"],
        "checks": [{"command": "cargo test -p opi-agent phase17_api_audit", "observed": "PASS"}],
        "state": state,
        "finding_ids": finding_ids or [],
    }
    return record
```

Use the same explicit-argument pattern in `finding_record`; set
`source_path` from the argument and set `source_model` to the exact configured
model identity.

Use this signature and field assignment so later two-run tests never depend on
the global run ID:

```python
def finding_record(
    *,
    audit_run_id: str = RUN_ID,
    finding_id: str = "P17-AUD-001",
    source_path: str = "docs/snapshots/phase17/assurance/audit.codex.gpt56.md",
    source_model: str = "gpt-5.6",
    observed_at: str = AUDIT_HEAD,
    requirement_ids: list[str] | None = None,
    severity: str = "Major",
    conformance_effect: str = "blocks",
) -> dict[str, object]:
    return {
        "audit_run_id": audit_run_id,
        "id": finding_id,
        "source_kind": "audit",
        "source_path": source_path,
        "source_model": source_model,
        "observed_at": observed_at,
        "independence": "fresh-context-same-family",
        "axis": "spec",
        "severity": severity,
        "conformance_effect": conformance_effect,
        "title": "Durable session binding is absent",
        "claim": "New sessions do not persist the required runtime binding.",
        "evidence": [{
            "location": "crates/opi-agent/src/session.rs:42",
            "detail": "The serialized header has no runtime binding.",
        }],
        "requirement_ids": requirement_ids or ["P17-A1"],
        "criterion_source": "docs/opi-spec.md#INV-007",
        "reproduction": ["cargo test -p opi-agent --test session_contract"],
        "confidence": "high",
        "status": "unverified",
    }
```

- [ ] **Step 2: Replace the single-set writer with a member writer**

Add these fixture constants and helpers to `AssuranceFixture`:

```python
GENERATION_ID = "phase17-20260824t010203z"


def artifact_stem(reviewer_id: str, model_id: str) -> str:
    return f"audit.{reviewer_id}.{model_id}"


def member_run_id(reviewer_id: str, model_id: str) -> str:
    return f"phase17-{reviewer_id}-{model_id}-136c380-20260824t010203z"
```

Implement `write_audit_member(reviewer_id, model_id, reviewer_model_id,
requirements, findings, verdict)` so it writes exactly:

```text
audit.<reviewer-id>.<model-id>.meta.json
audit.<reviewer-id>.<model-id>.requirements.jsonl
audit.<reviewer-id>.<model-id>.findings.jsonl
audit.<reviewer-id>.<model-id>.md
```

The member metadata uses `schema_version: 2` and includes these exact identity
fields in addition to the existing Phase/baseline/digest/verdict fields:

```python
{
    "reviewer_id": reviewer_id,
    "reviewer_identity": reviewer_id.capitalize(),
    "model_id": model_id,
    "reviewer_model_id": reviewer_model_id,
    "model_identity_source": "operator-declared",
    "audit_generation_id": GENERATION_ID,
}
```

The report repeats `Audit generation ID`, `Audit run ID`, `Audit head`,
`Reviewer ID`, `Model ID`, and `Verdict` headers. Return an index-member dict
whose `digests` hash the exact four files.

- [ ] **Step 3: Add an active-index writer**

Add `write_audit_index(members)` to sort members by
`(reviewer_id, model_id)`, derive the strict aggregate verdict, and write
`audit.index.json`:

```python
def aggregate_verdict(member_verdicts: list[str]) -> str:
    if "FAIL" in member_verdicts:
        return "FAIL"
    if "PASS-WITH-FINDINGS" in member_verdicts:
        return "PASS-WITH-FINDINGS"
    return "PASS"
```

The fixture index has `schema_version: 1`, `phase: 17`, the fixed generation
and head constants, `revision: 1`, the derived `aggregate_verdict`, and the
sorted member list.

- [ ] **Step 4: Add red tests for indexed membership and verdicts**

Add focused tests with these exact outcomes:

```python
def test_two_active_members_derive_pass(self) -> None:
    codex = self.write_audit_member("codex", "gpt56", "gpt-5.6")
    claude = self.write_audit_member("claude", "glm53", "glm-5.3")
    self.write_audit_index([codex, claude])
    result = run_validator("audit-set", self.assurance_dir)
    self.assertEqual(0, result.returncode, result.stderr)


def test_one_failed_member_derives_fail(self) -> None:
    codex = self.write_audit_member("codex", "gpt56", "gpt-5.6")
    failed = self.write_audit_member(
        "claude",
        "glm53",
        "glm-5.3",
        requirements=[
            requirement_record(
                audit_run_id=member_run_id("claude", "glm53"),
                state="not-met",
                finding_ids=["P17-AUD-001"],
            )
        ],
        findings=[
            finding_record(
                audit_run_id=member_run_id("claude", "glm53"),
                source_path="docs/snapshots/phase17/assurance/audit.claude.glm53.md",
                source_model="glm-5.3",
            )
        ],
    )
    self.write_audit_index([codex, failed])
    result = run_validator("audit-set", self.assurance_dir)
    self.assertEqual(0, result.returncode, result.stderr)
```

Also add one test each for `PASS-WITH-FINDINGS`, an absent member file,
different `audit_head`, filename/meta identity mismatch, wrong exact digest,
different baseline source bytes, duplicate reviewer/model pair, unsorted
members, orphan dynamic audit file, an unexpected root directory, and an old
unsuffixed four-file set. The first three aggregate cases pass; all structural
cases fail with `AUDIT-INCOMPLETE` diagnostics.

- [ ] **Step 5: Run the focused tests and verify red**

Run:

```text
python scripts/test_opi_assurance_skills.py AuditSetValidatorTests
```

Expected: FAIL because the current validator still requires
`audit.meta.json`, `audit.requirements.jsonl`, `audit.findings.jsonl`, and
`audit.md`.

### Task 2: Implement index-driven audit validation

**Files:**
- Modify: `.agents/skills/_shared/scripts/validate_assurance_artifact.py:15-570`
- Modify: `scripts/test_opi_assurance_skills.py:15-500`
- Test: `scripts/test_opi_assurance_skills.py`

- [ ] **Step 1: Introduce the exact index and filename schema**

Replace the fixed `AUDIT_FILES` constant with these definitions:

```python
SLUG_RE = re.compile(r"^[a-z0-9][a-z0-9-]*$")
AUDIT_STEM_RE = re.compile(
    r"^audit\.(?P<reviewer>[a-z0-9][a-z0-9-]*)\."
    r"(?P<model>[a-z0-9][a-z0-9-]*)$"
)
AUDIT_FILE_RE = re.compile(
    r"^(?P<stem>audit\.[a-z0-9][a-z0-9-]*\."
    r"[a-z0-9][a-z0-9-]*)\."
    r"(?P<kind>meta\.json|requirements\.jsonl|findings\.jsonl|md)$"
)
AUDIT_INDEX_FILE = "audit.index.json"
INDEX_FIELDS = {
    "schema_version",
    "phase",
    "audit_generation_id",
    "audit_head",
    "revision",
    "aggregate_verdict",
    "members",
}
INDEX_MEMBER_FIELDS = {
    "reviewer_id",
    "model_id",
    "artifact_stem",
    "audit_run_id",
    "verdict",
    "digests",
}
INDEX_DIGEST_FIELDS = {
    "meta_sha256",
    "requirements_sha256",
    "findings_sha256",
    "report_sha256",
}
```

Extend `META_FIELDS` with `reviewer_id`, `reviewer_identity`, `model_id`,
`reviewer_model_id`, `model_identity_source`, and `audit_generation_id`;
remove `reviewer_model`; require member `schema_version == 2`. Keep the new
index at schema version 1. Require `model_identity_source` to be exactly one of
`runtime-attested`, `request-config`, or `operator-declared`, and require both
human-readable identity fields to be non-empty.

- [ ] **Step 2: Add path and member loaders**

Add immutable helpers that derive all four paths from one validated stem:

```python
from dataclasses import dataclass


@dataclass(frozen=True)
class AuditMemberPaths:
    stem: str
    meta: Path
    requirements: Path
    findings: Path
    report: Path


def member_paths(directory: Path, stem: str) -> AuditMemberPaths:
    return AuditMemberPaths(
        stem=stem,
        meta=directory / f"{stem}.meta.json",
        requirements=directory / f"{stem}.requirements.jsonl",
        findings=directory / f"{stem}.findings.jsonl",
        report=directory / f"{stem}.md",
    )
```

Add `validate_index(directory, errors)` that rejects missing/extra fields,
invalid slugs, duplicate pairs, duplicate run IDs, unsorted members, invalid
digests, non-positive revisions, and invalid aggregate verdicts. It returns the
index dict and its exact raw SHA-256.

Add a pure `validate_staged_member(directory, phase, generation_id,
audit_head, reviewer_id, model_id)` helper that validates the four staged files
without requiring an index entry and returns metadata, records, verdict, and
four exact digests. `validate_audit_member` calls this helper and then verifies
the returned values against the indexed member. The generation coordinator
uses the same pure helper during `complete`, so staged and live validation
cannot drift.

- [ ] **Step 3: Parameterize member record validation by stem**

Change `validate_requirement_records` and `validate_finding_records` to accept
`paths: AuditMemberPaths`. Replace fixed sibling checks with the paths derived
from that stem. Require each finding's `source_path` to equal:

```python
f"docs/snapshots/phase{meta['phase']}/assurance/{paths.stem}.md"
```

Require `source_model == meta["reviewer_model_id"]`. Require filename slugs,
index values, metadata, and report headers to agree exactly.

Keep the standalone `requirements` and `findings` subcommands diagnostic by
deriving the stem from the supplied suffixed filename. Reject unsuffixed and
malformed names.

- [ ] **Step 4: Validate every indexed member independently**

Extract the existing reciprocal-link and member-verdict logic into
`validate_audit_member(directory, index, member, errors)`. It must:

1. require all four member files;
2. verify all four index digests over exact bytes;
3. validate metadata, requirements, findings, and report headers;
4. require Phase, generation, head, reviewer/model, run ID, and verdict to
   match the index;
5. derive that member's verdict from only its own requirements/findings;
6. return the loaded metadata and findings for later remediation use.

Replace `validate_audit_set` with an index-driven loop. Allow only
`audit.index.json`, indexed dynamic member files, the four fixed remediation
files, and the `history/` directory at the assurance root. Any unindexed
dynamic file is an orphan, any other root directory is unexpected, and any old
unsuffixed file is a legacy-format error. After loading the members, require
their `baseline_policy` and exact ordered `baseline_sources` arrays to match;
same head with different registered baselines is invalid.

- [ ] **Step 5: Derive and validate the aggregate verdict**

Use this exact precedence after every member validates:

```python
def derive_aggregate_verdict(verdicts: list[str]) -> str:
    if any(verdict == "FAIL" for verdict in verdicts):
        return "FAIL"
    if any(verdict == "PASS-WITH-FINDINGS" for verdict in verdicts):
        return "PASS-WITH-FINDINGS"
    return "PASS"
```

Reject an empty member roster. Reject an index whose stored aggregate differs
from the derived result. Validator errors remain printed as `FAIL:` lines; the
audit workflow maps any such failure to `AUDIT-INCOMPLETE` and publishes no new
verdict.

Update `validate_rotation` to recognize an index plus complete dynamic groups,
the all-or-none fixed remediation group, and the `history/` directory. It still
inspects Git path state without reading old conclusions and still rejects every
legacy Phase-level sibling or dirty assurance path.

- [ ] **Step 6: Run indexed audit tests green**

Run:

```text
python scripts/test_opi_assurance_skills.py AuditSetValidatorTests
```

Expected: PASS, including both peer reports and every fail-closed case.

- [ ] **Step 7: Conditionally commit the validator slice**

Only with explicit commit authorization:

```text
git add -- .agents/skills/_shared/scripts/validate_assurance_artifact.py scripts/test_opi_assurance_skills.py
git commit -m "feat: validate multi-model audit generations"
```

### Task 3: Add staged generation coordination and transactional publication

**Files:**
- Create: `.agents/skills/_shared/scripts/assurance_generation.py`
- Create: `scripts/test_opi_assurance_generation.py`
- Modify: `.agents/skills/_shared/scripts/validate_assurance_artifact.py:1070-1106`
- Test: `scripts/test_opi_assurance_generation.py`

- [ ] **Step 1: Write red CLI tests for generation initialization**

Create `scripts/test_opi_assurance_generation.py` with temporary Git repository
fixtures and a `run_generation(*args)` subprocess helper. Test this concrete
initialization:

```text
python .agents/skills/_shared/scripts/assurance_generation.py init docs/snapshots/phase17 C:/tmp/phase17-generation --audit-head 136c380f0c5eea541190cc1a0f5c1d62f983b4e8 --generation-id phase17-20260824t010203z --member codex:gpt56 --member claude:glm53 --expected-index absent
```

In the test, replace `C:/tmp/phase17-generation` with the fixture's absolute
temporary path. Assert that `generation.json` has revision 0, the exact sorted
roster, both states `pending`, the expected prior index value, and an empty
`active/` directory. Assert duplicate slots and a staging path inside the live
`assurance/` directory are rejected.

- [ ] **Step 2: Run the generation test and verify red**

Run:

```text
python scripts/test_opi_assurance_generation.py GenerationInitTests
```

Expected: FAIL because `assurance_generation.py` does not exist.

- [ ] **Step 3: Implement the staging manifest and CLI parser**

Create the new script with `init`, `complete`, `fail`, `publish`, and `recover`
subcommands. The durable active index remains owned by the validator; the
staging `generation.json` is scratch with this exact shape:

```json
{
  "schema_version": 1,
  "phase": 17,
  "audit_generation_id": "phase17-20260824t010203z",
  "audit_head": "136c380f0c5eea541190cc1a0f5c1d62f983b4e8",
  "revision": 0,
  "expected_index_sha256": "absent",
  "members": [
    {"reviewer_id": "claude", "model_id": "glm53", "state": "pending"},
    {"reviewer_id": "codex", "model_id": "gpt56", "state": "pending"}
  ]
}
```

Use atomic JSON writes implemented as a same-directory temporary file followed
by `os.replace`. Resolve the repository root and Git common directory with
`git rev-parse --show-toplevel` and `git rev-parse --git-common-dir`.

`fail --reviewer <id> --model <id> --expected-revision <N> --reason <text>`
changes only a pending member to `failed`, records a non-empty
`failure_reason`, and increments revision under the same lock/CAS rules.
Publication refuses every manifest containing a pending or failed member.

- [ ] **Step 4: Implement a cross-platform repository assurance lock**

Use only the Python standard library. The lock file lives under the resolved
Git common directory at `opi-assurance-locks/phase17.lock`. Lock byte zero with
`msvcrt.locking(..., LK_NBLCK, 1)` on Windows and
`fcntl.flock(..., LOCK_EX | LOCK_NB)` elsewhere. Always unlock and close in
`finally`. Failure to acquire the lock returns a nonzero exit and does not
retry or mutate files.

- [ ] **Step 5: Add red tests for independent completion and CAS**

Have the fixture write valid `codex:gpt56` and `claude:glm53` groups below
`<staging>/active/`. Add tests that:

- complete Codex with `--expected-revision 0`, producing revision 1;
- complete Claude with `--expected-revision 1`, producing revision 2;
- reject a second completion of the Codex slot;
- reject `--expected-revision 0` after revision has advanced;
- mark one pending slot failed with an exact reason and prove publish refuses;
- reject a member whose metadata head or reviewer/model identity differs;
- prove the failed completion leaves the manifest bytes unchanged.

Run:

```text
python scripts/test_opi_assurance_generation.py GenerationCompletionTests
```

Expected: FAIL until `complete` invokes the validator's member validation,
computes the four exact digests, records run ID/verdict/digests in that member,
and atomically increments the manifest revision under the assurance lock.

- [ ] **Step 6: Implement completion with no lost updates**

`complete` must acquire the lock, re-read `generation.json`, compare the exact
revision, validate only the named slot's four files, and replace that member's
pending entry with:

```python
{
    "reviewer_id": reviewer_id,
    "model_id": model_id,
    "state": "complete",
    "artifact_stem": f"audit.{reviewer_id}.{model_id}",
    "audit_run_id": meta["audit_run_id"],
    "verdict": meta["verdict"],
    "digests": digests,
}
```

Increment revision by one. Do not modify another member entry. A stale revision
or non-pending slot fails before writing.

- [ ] **Step 7: Add red publication and recovery tests**

Add tests for:

1. all-complete staging publishes two active groups and a sorted index;
2. any pending/failed member refuses publication;
3. active-index SHA mismatch refuses publication without changing live bytes;
4. a complete prior generation and its four remediation files move together
   to `history/<old-generation-id>/`;
5. an injected failure before index switch restores the exact prior active
   bytes;
6. an injected failure after live validation finishes the new generation on
   `recover`;
7. a pre-existing history target refuses publication without overwriting it;
8. a same-slot concurrent completion has one success and one lock/CAS failure;
9. invoking the live validator with a prepared recovery journal restores the
   prior generation before validation reads the active files.

Expose no production `--fail-after` flag. Unit-test interruption by importing
the module and mocking the internal `install_new_files` or `finalize_history`
function to raise `OSError` after the journal state has been written.

- [ ] **Step 8: Implement journaled publication and recovery**

Store the recovery journal and prior-byte backup below the Git common directory
under `opi-assurance-transactions/<transaction-id>/`. Validate that every
resolved delete/copy target stays within either the exact Phase assurance root
or that transaction directory.

Use journal states `prepared`, `installing`, and `switched`:

- `prepared`: prior bytes and the intended new index are backed up;
- `installing`: dynamic active audit/remediation files are being replaced;
- `switched`: the new index was written last and live validation passed.

Before `switched`, recovery restores the exact prior root files and removes
only transaction-owned new files. At `switched`, recovery completes the
history copy and removes the journal/backup. Every subcommand runs recovery
after acquiring the lock and before reading live membership.

`publish` builds `audit.index.json` from the complete staging manifest, sets
active revision to prior active revision plus one (or 1 for the first
generation), derives the aggregate verdict, validates staging, compares the
expected prior index SHA-256, performs the journaled switch, validates live,
and prints:

```text
publish: PASS audit_generation_id=phase17-20260824t010203z aggregate_verdict=PASS
```

Expose `AssuranceLock` and `recover_locked` from the generation module without
importing the validator at module import time. In
`validate_assurance_artifact.py`, wrap live `audit-set`, `rotation`,
`dispositions`, `plan`, and `result` CLI validation with that lock and recovery.
Do not wrap an external staging directory. The publisher, which already holds
the lock, calls the validator's pure validation functions directly so it cannot
deadlock itself.

- [ ] **Step 9: Run generation coordination tests green**

Run:

```text
python scripts/test_opi_assurance_generation.py
```

Expected: PASS for init, completion, CAS, publication, interruption recovery,
history, and concurrency cases.

- [ ] **Step 10: Conditionally commit generation coordination**

Only with explicit commit authorization:

```text
git add -- .agents/skills/_shared/scripts/assurance_generation.py .agents/skills/_shared/scripts/validate_assurance_artifact.py scripts/test_opi_assurance_generation.py
git commit -m "feat: publish concurrent audit generations"
```

### Task 4: Make remediation consume the strict findings union

**Files:**
- Modify: `.agents/skills/_shared/scripts/validate_assurance_artifact.py:574-1050`
- Modify: `scripts/test_opi_assurance_skills.py:480-650`
- Test: `scripts/test_opi_assurance_skills.py`

- [ ] **Step 1: Add red multi-source remediation fixtures**

Change `AssuranceFixture.write_plan` and `write_result` to read
`audit.index.json`, calculate its exact SHA-256, and write these headers:

```python
index = json.loads((self.assurance_dir / "audit.index.json").read_text(encoding="utf-8"))
index_sha256 = sha256(self.assurance_dir / "audit.index.json")
headers = (
    f"**Audit generation ID**: `{index['audit_generation_id']}`\n"
    f"**Audit index SHA-256**: `{index_sha256}`\n"
)
```

Remove the single `Audit run ID` and `Findings SHA-256` headers from plan/result
fixtures.

Write two failed member audits whose finding IDs are both `P17-AUD-001` but
whose run IDs and findings digests differ. Add one disposition per
`(audit_run_id, finding_id)`.

- [ ] **Step 2: Add exact union and drift tests**

Add tests proving:

- a plan with both source identities passes;
- omitting either source identity fails coverage;
- duplicate textual IDs from different runs remain distinct;
- a source digest must match its owning member, not another member;
- a source run absent from the current index is rejected;
- changing any indexed member byte invalidates the plan's index digest;
- result dispositions cover exactly the plan source-key set;
- one closure batch may contain both source keys when their closure key is
  identical;
- different closure keys still cannot share one batch.

- [ ] **Step 3: Run remediation tests and verify red**

Run:

```text
python scripts/test_opi_assurance_skills.py RemediationValidatorTests
```

Expected: FAIL because plan/result validation still loads one fixed meta and
one fixed findings sidecar and reduces coverage to finding ID alone.

- [ ] **Step 4: Introduce a generation-wide remediation context**

Add this immutable context near `source_key`:

```python
@dataclass(frozen=True)
class RemediationAuditContext:
    generation_id: str
    index_sha256: str
    metadata_by_run: dict[str, dict[str, Any]]
    findings_by_key: dict[tuple[str, str], dict[str, Any]]
```

Implement `load_remediation_context(directory, errors)` by validating the
whole active set, hashing `audit.index.json`, then loading every indexed
member's meta/findings sidecar. Reject duplicate `(audit_run_id, finding_id)`
across members.

- [ ] **Step 5: Validate dispositions against their owning member**

Change `validate_finding_disposition` and `validate_disposition_records` to
take `context: RemediationAuditContext`. For each source, find metadata by
`audit_run_id`, compare `findings_sha256` with that member, and require the full
source key in `context.findings_by_key`.

For incidental repairs, calculate protected baseline paths from the union of
all current member metadata. The members have the same registered baseline,
but validation must not silently choose the first one.

- [ ] **Step 6: Replace single-run plan/result headers and coverage**

In `validate_plan` and `validate_result`, require `Audit generation ID` and
`Audit index SHA-256` to match the current context. Compute missing and extra
coverage as sets of `(audit_run_id, finding_id)`, never as bare finding IDs.
Error messages include both components, for example:

```text
plan is missing current finding phase17-claude-glm53-136c380-20260824t010203z/P17-AUD-001
```

Keep the plan digest algorithm unchanged:

```text
sha256(remediation.plan.md exact bytes + NUL + remediation.plan.dispositions.jsonl exact bytes)
```

- [ ] **Step 7: Run remediation and complete assurance tests green**

Run:

```text
python scripts/test_opi_assurance_skills.py RemediationValidatorTests
python scripts/test_opi_assurance_skills.py
```

Expected: both commands PASS.

- [ ] **Step 8: Conditionally commit remediation aggregation**

Only with explicit commit authorization:

```text
git add -- .agents/skills/_shared/scripts/validate_assurance_artifact.py scripts/test_opi_assurance_skills.py
git commit -m "feat: remediate all active audit findings"
```

### Task 5: Rewrite assurance contracts, templates, skills, and evals

**Files:**
- Modify: `.agents/skills/_shared/references/audit-set-contract.md`
- Modify: `.agents/skills/_shared/references/finding-contract.md`
- Modify: `.agents/skills/_shared/references/remediation-disposition-contract.md`
- Modify: `.agents/skills/opi-audit/SKILL.md`
- Modify: `.agents/skills/opi-audit/agents/openai.yaml`
- Modify: `.agents/skills/opi-audit/references/finding-template.md`
- Modify: `.agents/skills/opi-audit/references/audit-proof-obligations.md`
- Modify: `.agents/skills/opi-audit/evals/evals.json`
- Modify: `.agents/skills/opi-remediate/SKILL.md`
- Modify: `.agents/skills/opi-remediate/agents/openai.yaml`
- Modify: `.agents/skills/opi-remediate/references/cross-reference-matrix.md`
- Modify: `.agents/skills/opi-remediate/references/remediation-plan-template.md`
- Modify: `.agents/skills/opi-remediate/references/execution-protocol.md`
- Modify: `.agents/skills/opi-remediate/evals/evals.json`
- Test: `scripts/test_opi_doc_check.py`

- [ ] **Step 1: Rewrite the active audit set contract from the approved design**

The shared contract must define, without retaining the old fixed paths:

- the exact `audit.index.json` schema and sorted member roster;
- meta schema version 2 and the identity-source enum;
- `audit.<reviewer-id>.<model-id>.*` four-file groups;
- member and aggregate verdict derivation;
- one generation/head/baseline invariant;
- independent peer contexts that cannot read one another's output;
- staging init/complete/publish commands;
- lock, revision/CAS, journal recovery, and `AUDIT-INCOMPLETE` behavior;
- generation-wide rotation into `history/<audit-generation-id>/`;
- no unsuffixed compatibility aliases and no prior-conclusion input.

Use concrete examples `audit.codex.gpt56.*` and
`audit.claude.glm53.*`. State that `<model-id>` identifies the runtime-selected
model and does not configure model selection.

- [ ] **Step 2: Update the finding and report contracts**

Change the finding contract's example to
`docs/snapshots/phase17/assurance/audit.codex.gpt56.md`. Require
`source_model == reviewer_model_id` from the owning meta. Keep finding identity
as `(audit_run_id, id)`.

Change the report template to list the suffixed group and add exact headers for
generation, reviewer ID, model ID, full model identity, identity source, run,
head, and verdict. Change the proof-obligation reference from the literal
`audit.requirements.jsonl` to the current member's suffixed requirements
sidecar.

- [ ] **Step 3: Rewrite remediation binding and templates**

The disposition contract must say that remediation consumes every indexed
member, findings are a strict union, duplicate repairs still receive one
disposition per source key, and severity disagreement schedules at the highest
source severity without rewriting source records.

Replace plan/result `Audit run ID` and `Findings SHA-256` headers with `Audit
generation ID` and `Audit index SHA-256`. Retain each disposition's exact
`audit_run_id`, `findings_sha256`, and finding ID. Update the plan template and
execution protocol with the same headers and invalidation rule.

- [ ] **Step 4: Update `opi-audit` invocation and execution**

Require `phase=<N>`, `reviewer=<reviewer-id>`, `model=<model-id>`, the exact
runtime model identity, its identity source, and a staged generation path.
Document that a single-member audit still uses a one-member index. Replace
sequential repository execution with parallel independent inspection and
serialized completion/publication. Route init, complete, publish, and recovery
through `assurance_generation.py`; do not describe manual multi-file copying.

Update `agents/openai.yaml` to mention reviewer/model identity in the prompt,
but do not add a model selector field to the YAML.

- [ ] **Step 5: Update `opi-remediate` and the closure matrix**

Require complete active-index validation, consume every indexed meta/findings
sidecar, bind plan/apply to the index digest, and preserve all source keys.
Document the criterion matrix, strict finding union, shared fix evidence,
highest-severity scheduling, and evidence-only refutation. Remove statements
that remediation consumes only `audit.meta.json` and `audit.findings.jsonl`.

- [ ] **Step 6: Replace skill eval cases**

Update audit evals to cover a two-member roster, inability to inspect sibling
conclusions, a malformed member leaving the prior generation active, and an
index CAS failure. Update remediation evals to cover same textual finding IDs
from two run IDs, complete union coverage, and an index byte change invalidating
an approved plan. Keep existing latest-committed-spec, missing-platform
evidence, incidental-repair, and materialization cases.

Validate both JSON files:

```text
python -c "import json,pathlib; [json.loads(pathlib.Path(p).read_text(encoding='utf-8')) for p in ('.agents/skills/opi-audit/evals/evals.json','.agents/skills/opi-remediate/evals/evals.json')]"
```

Expected: exit 0 with no output.

- [ ] **Step 7: Conditionally commit workflow contracts**

Only with explicit commit authorization, stage every path from this task by
explicit name and commit:

```text
git add -- .agents/skills/_shared/references/audit-set-contract.md .agents/skills/_shared/references/finding-contract.md .agents/skills/_shared/references/remediation-disposition-contract.md .agents/skills/opi-audit/SKILL.md .agents/skills/opi-audit/agents/openai.yaml .agents/skills/opi-audit/references/finding-template.md .agents/skills/opi-audit/references/audit-proof-obligations.md .agents/skills/opi-audit/evals/evals.json .agents/skills/opi-remediate/SKILL.md .agents/skills/opi-remediate/agents/openai.yaml .agents/skills/opi-remediate/references/cross-reference-matrix.md .agents/skills/opi-remediate/references/remediation-plan-template.md .agents/skills/opi-remediate/references/execution-protocol.md .agents/skills/opi-remediate/evals/evals.json
git commit -m "docs: define multi-model assurance workflow"
```

Before committing, run `git status --short` and ensure the index contains only
the explicitly staged Task 5 paths.

### Task 6: Update source-derived documentation checks

**Files:**
- Modify: `scripts/test_opi_doc_check.py:100-650`
- Modify: `scripts/opi-doc-check.py:95-245`
- Test: `scripts/test_opi_doc_check.py`

- [ ] **Step 1: Change required and forbidden assurance tokens in the test**

In `ASSURANCE_WORKFLOW_REQUIRED`, require the owning documents to contain these
new semantic tokens where applicable:

```python
(
    "audit.index.json",
    "audit.<reviewer-id>.<model-id>",
    "audit_generation_id",
    "reviewer_id",
    "model_id",
    "model_identity_source",
    "Audit generation ID",
    "Audit index SHA-256",
    "strict union",
    "assurance_generation.py",
    "AUDIT-INCOMPLETE",
)
```

Remove old required tokens that assert one fixed meta/findings pair. Remove
`audit.<model>` from the forbidden-token map because suffixed active names are
now intentional. Add unsuffixed path statements and majority-vote wording to
the forbidden map.

- [ ] **Step 2: Run the documentation-contract test and verify red**

Run:

```text
python scripts/test_opi_doc_check.py SkillContractTests
```

Expected: FAIL because `scripts/opi-doc-check.py` still enforces the old
single-set tokens.

- [ ] **Step 3: Mirror the exact contract in `opi-doc-check.py`**

Update `ASSURANCE_WORKFLOW_CONTRACT` and
`ASSURANCE_WORKFLOW_FORBIDDEN` to exactly match the test constants. Keep the
branch-scoped remediation reference checks unchanged. Do not add prose wording
or report titles to Rust tests.

- [ ] **Step 4: Run focused documentation tests green**

Run:

```text
python scripts/test_opi_doc_check.py SkillContractTests
python scripts/test_opi_doc_check.py
```

Expected: both commands PASS.

- [ ] **Step 5: Conditionally commit documentation checks**

Only with explicit commit authorization:

```text
git add -- scripts/opi-doc-check.py scripts/test_opi_doc_check.py
git commit -m "test: enforce multi-model assurance docs"
```

### Task 7: Synchronize bilingual workflow documentation and changelog

**Files:**
- Modify: `.agents/skills/README.md:70-140`
- Modify: `.agents/skills/README.zh.md:70-140`
- Modify: `CHANGELOG.md:8-130`
- Test: `scripts/opi-doc-check.py`

- [ ] **Step 1: Update the English workflow manual**

Change the audit row, remediation row, assurance model, and durable artifact
ownership table to state:

- a Phase has one active generation containing all indexed peer audit groups;
- reviewer/model groups are equal and no canonical report exists;
- aggregate verdict is fail-dominant;
- remediation consumes the strict union and binds approval to the active index
  digest;
- superseded generations live under `assurance/history/` and are never
  semantic input;
- model IDs disclose actual runtime identity and never select a provider model.

- [ ] **Step 2: Make the equivalent Chinese changes**

Use the same propositions in `.agents/skills/README.zh.md`: one generation,
all indexed peer groups, strict union, fail-dominant verdict, index digest,
history exclusion, and identity-not-selection. Do not merely translate file
names; keep the authority and failure behavior equivalent.

- [ ] **Step 3: Add the `Unreleased / Breaking Changes` entry**

Append one bullet to the existing `### Breaking Changes` subsection; do not
create another subsection:

```markdown
- Project assurance: a Phase audit is now an indexed generation of
  `audit.<reviewer-id>.<model-id>.*` peer groups rather than one unsuffixed
  four-file set. Remediation consumes every indexed finding and binds approval
  to the exact `audit.index.json` digest; legacy unsuffixed active sets require
  an explicitly identified rerun or migration and are not dual-read.
```

- [ ] **Step 4: Run the documentation gate**

Run:

```text
python scripts/opi-doc-check.py
```

Expected: `opi documentation contracts: PASS`.

- [ ] **Step 5: Conditionally commit manuals and changelog**

Only with explicit commit authorization:

```text
git add -- .agents/skills/README.md .agents/skills/README.zh.md CHANGELOG.md
git commit -m "docs: document multi-model active audits"
```

### Task 8: Verify migration boundaries and the complete change

**Files:**
- Test: `scripts/test_opi_assurance_skills.py`
- Test: `scripts/test_opi_assurance_generation.py`
- Test: `scripts/test_opi_doc_check.py`
- Verify: every path listed in the file responsibility map

- [ ] **Step 1: Verify legacy behavior is an explicit break**

Run the focused test that constructs only `audit.meta.json`,
`audit.requirements.jsonl`, `audit.findings.jsonl`, and `audit.md`.
Expected: nonzero exit with a diagnostic requiring `audit.index.json` and
reviewer/model suffixed groups. Verify there is no fallback alias and no
automatic inference from `Codex (GPT-5)` to `gpt56`.

- [ ] **Step 2: Run all focused Python tests**

Run:

```text
python scripts/test_opi_assurance_skills.py
python scripts/test_opi_assurance_generation.py
python scripts/test_opi_doc_check.py
```

Expected: all tests PASS with zero failures and zero errors.

- [ ] **Step 3: Run the repository documentation contract**

Run:

```text
python scripts/opi-doc-check.py
```

Expected: `opi documentation contracts: PASS`.

- [ ] **Step 4: Check formatting and outgoing scope**

Run:

```text
git diff --check
git status --short
```

Expected: `git diff --check` prints nothing. `git status --short` contains only
the preserved pre-existing changes plus files named by this plan. Record test
impact as `update` for `scripts/test_opi_assurance_skills.py` and
`scripts/test_opi_doc_check.py`, and `add` for
`scripts/test_opi_assurance_generation.py`.

- [ ] **Step 5: Report the Phase 17 artifact migration gate**

Do not rename the existing Phase 17 audit as part of contract implementation
unless the user separately declares its exact full model identity, filename
slug, and identity source. Report that the new validator intentionally rejects
the legacy unindexed set and that a fresh multi-model generation is the
preferred migration path.

- [ ] **Step 6: Conditionally create the final commit**

Only if the user has explicitly authorized commits and earlier task commits
were intentionally skipped, stage each implementation path explicitly and
commit:

```text
git add -- .agents/skills/_shared/scripts/assurance_generation.py .agents/skills/_shared/scripts/validate_assurance_artifact.py scripts/test_opi_assurance_generation.py scripts/test_opi_assurance_skills.py .agents/skills/_shared/references/audit-set-contract.md .agents/skills/_shared/references/finding-contract.md .agents/skills/_shared/references/remediation-disposition-contract.md .agents/skills/opi-audit/SKILL.md .agents/skills/opi-audit/agents/openai.yaml .agents/skills/opi-audit/references/finding-template.md .agents/skills/opi-audit/references/audit-proof-obligations.md .agents/skills/opi-audit/evals/evals.json .agents/skills/opi-remediate/SKILL.md .agents/skills/opi-remediate/agents/openai.yaml .agents/skills/opi-remediate/references/cross-reference-matrix.md .agents/skills/opi-remediate/references/remediation-plan-template.md .agents/skills/opi-remediate/references/execution-protocol.md .agents/skills/opi-remediate/evals/evals.json scripts/opi-doc-check.py scripts/test_opi_doc_check.py .agents/skills/README.md .agents/skills/README.zh.md CHANGELOG.md
git commit -m "feat: support multi-model active audits"
```

Never stage the existing `docs/snapshots/phase17/assurance/` artifacts without
the separate identity/migration authorization described in Step 5.
