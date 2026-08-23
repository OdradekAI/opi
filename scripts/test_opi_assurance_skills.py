from __future__ import annotations

import copy
import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
VALIDATOR = (
    ROOT
    / ".agents"
    / "skills"
    / "_shared"
    / "scripts"
    / "validate_assurance_artifact.py"
)
AUDIT_HEAD = "136c380f0c5eea541190cc1a0f5c1d62f983b4e8"
REMEDIATION_HEAD = "236c380f0c5eea541190cc1a0f5c1d62f983b4e8"
RUN_ID = "phase17-136c380-20260824t010203z"


def run_validator(kind: str, path: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(VALIDATOR), kind, str(path)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )


def run_git(repo: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True,
        text=True,
        check=False,
    )


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path: Path, value: dict[str, object]) -> None:
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def write_jsonl(path: Path, records: list[dict[str, object]]) -> None:
    path.write_text(
        "".join(json.dumps(record, sort_keys=True) + "\n" for record in records),
        encoding="utf-8",
    )


def requirement_record(
    *,
    requirement_id: str = "P17-A1",
    mandatory: bool = True,
    state: str = "met",
    finding_ids: list[str] | None = None,
) -> dict[str, object]:
    return {
        "audit_run_id": RUN_ID,
        "id": requirement_id,
        "mandatory": mandatory,
        "criterion_source": {
            "path": "docs/opi-spec.md",
            "sha256": "a" * 64,
            "citation": "P17-A1",
        },
        "observable_behavior": "The registered behavior is present.",
        "production_surfaces": ["crates/opi-agent/src/lib.rs"],
        "test_evidence": ["phase17_api_audit"],
        "checks": [
            {
                "command": "cargo test -p opi-agent phase17_api_audit",
                "observed": "PASS",
            }
        ],
        "state": state,
        "finding_ids": finding_ids or [],
    }


def finding_record(
    *,
    finding_id: str = "P17-AUD-001",
    requirement_ids: list[str] | None = None,
    severity: str = "Major",
    conformance_effect: str = "blocks",
) -> dict[str, object]:
    return {
        "audit_run_id": RUN_ID,
        "id": finding_id,
        "source_kind": "audit",
        "source_path": "docs/snapshots/phase17/assurance/audit.md",
        "source_model": "codex",
        "observed_at": AUDIT_HEAD,
        "independence": "fresh-context-same-family",
        "axis": "spec",
        "severity": severity,
        "conformance_effect": conformance_effect,
        "title": "Durable session binding is absent",
        "claim": "New sessions do not persist the required runtime binding.",
        "evidence": [
            {
                "location": "crates/opi-agent/src/session.rs:42",
                "detail": "The serialized header has no runtime binding.",
            }
        ],
        "requirement_ids": requirement_ids or ["P17-A1"],
        "criterion_source": "docs/opi-spec.md#INV-007",
        "reproduction": ["cargo test -p opi-agent --test session_contract"],
        "confidence": "high",
        "status": "unverified",
    }


def disposition_record(
    findings_sha256: str,
    *,
    finding_id: str = "P17-AUD-001",
    audit_run_id: str = RUN_ID,
) -> dict[str, object]:
    return {
        "record_kind": "finding-disposition",
        "source": {
            "audit_run_id": audit_run_id,
            "findings_sha256": findings_sha256,
            "id": finding_id,
        },
        "verified_at": REMEDIATION_HEAD,
        "verification_status": "Confirmed",
        "final_severity": "Major",
        "final_severity_rationale": "The mandatory durable binding is absent.",
        "closure_key": "session.runtime-binding",
        "family_key": "session.durability",
        "decision": "fix:persist-runtime-binding",
        "closure_batch": "B1",
        "change_kind": "behavioral",
        "changed_paths": ["crates/opi-agent/src/session.rs"],
        "red_before": {
            "command": "cargo test -p opi-agent --test session_contract binding",
            "expected": "FAIL because the binding is absent",
            "observed": "FAIL: binding was None",
        },
        "green_after": {
            "command": "cargo test -p opi-agent --test session_contract binding",
            "expected": "PASS",
            "observed": "not-run",
        },
    }


def incidental_record(*, changed_path: str) -> dict[str, object]:
    return {
        "record_kind": "incidental-repair",
        "id": "I1",
        "trigger_batch": "B1",
        "blocking_check": "cargo test --workspace --all-targets",
        "scope_rationale": "The collision blocks the approved B1 workspace gate.",
        "guardrails": {
            "required_for_green": True,
            "within_causal_surface": True,
            "changes_public_api": False,
            "changes_durable_format": False,
            "changes_dependency_graph": False,
            "changes_spec_or_authority": False,
        },
        "changed_paths": [changed_path],
        "red_before": {
            "command": "cargo test --workspace --all-targets",
            "expected": "FAIL",
            "observed": "FAIL: helper name collision",
        },
        "green_after": {
            "command": "cargo test --workspace --all-targets",
            "expected": "PASS",
            "observed": "PASS",
        },
        "remediation_status": "Closed",
    }


class AssuranceFixture(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.phase_dir = self.root / "docs" / "snapshots" / "phase17"
        self.assurance_dir = self.phase_dir / "assurance"
        self.assurance_dir.mkdir(parents=True)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write_audit_set(
        self,
        *,
        requirements: list[dict[str, object]] | None = None,
        findings: list[dict[str, object]] | None = None,
        verdict: str | None = None,
    ) -> dict[str, Path]:
        requirement_records = requirements or [requirement_record()]
        finding_records = findings or []
        requirements_path = self.assurance_dir / "audit.requirements.jsonl"
        findings_path = self.assurance_dir / "audit.findings.jsonl"
        report_path = self.assurance_dir / "audit.md"
        meta_path = self.assurance_dir / "audit.meta.json"
        write_jsonl(requirements_path, requirement_records)
        write_jsonl(findings_path, finding_records)
        derived = verdict
        if derived is None:
            if any(
                record.get("mandatory") and record.get("state") != "met"
                for record in requirement_records
            ):
                derived = "FAIL"
            elif any(record.get("severity") != "Info" for record in finding_records):
                derived = "PASS-WITH-FINDINGS"
            else:
                derived = "PASS"
        meta = {
            "schema_version": 1,
            "audit_run_id": RUN_ID,
            "phase": 17,
            "audit_head": AUDIT_HEAD,
            "reviewer_model": "codex",
            "independence": "fresh-context-same-family",
            "baseline_policy": "latest-committed-spec",
            "baseline_sources": [
                {"path": ".opi-impl-state.json", "sha256": "b" * 64},
                {
                    "path": "docs/snapshots/phase17/opi-impl-state.json",
                    "sha256": "c" * 64,
                },
                {"path": "docs/opi-spec.md", "sha256": "d" * 64},
            ],
            "requirements_sha256": sha256(requirements_path),
            "findings_sha256": sha256(findings_path),
            "verdict": derived,
        }
        write_json(meta_path, meta)
        report_path.write_text(
            "# Phase 17 Audit\n\n"
            f"**Audit run ID**: `{RUN_ID}`\n"
            f"**Audit head**: `{AUDIT_HEAD}`\n"
            f"**Verdict**: {derived}\n",
            encoding="utf-8",
        )
        return {
            "meta": meta_path,
            "requirements": requirements_path,
            "findings": findings_path,
            "report": report_path,
        }

    def write_plan(
        self,
        *,
        disposition: dict[str, object] | None = None,
        status: str = "READY-FOR-APPLY",
    ) -> tuple[Path, Path]:
        meta = json.loads(
            (self.assurance_dir / "audit.meta.json").read_text(encoding="utf-8")
        )
        record = disposition or disposition_record(meta["findings_sha256"])
        disposition_path = self.assurance_dir / "remediation.plan.dispositions.jsonl"
        plan_path = self.assurance_dir / "remediation.plan.md"
        write_jsonl(disposition_path, [record])
        plan_path.write_text(
            "# Phase 17 Remediation Plan\n\n"
            f"**Status**: {status}\n"
            f"**Audit run ID**: `{meta['audit_run_id']}`\n"
            f"**Findings SHA-256**: `{meta['findings_sha256']}`\n"
            f"**Remediation head**: `{REMEDIATION_HEAD}`\n"
            "**Disposition artifact**: `remediation.plan.dispositions.jsonl`\n"
            "**Unresolved decisions**: none\n\n"
            "Source finding: P17-AUD-001\n\n"
            "#### Fix B1.1: Persist the runtime binding\n\n"
            "- **Closure predicate**: reconstructed sessions retain the binding\n"
            "- **Red-before**: `cargo test binding` -> FAIL\n"
            "- **Green-after**: `cargo test binding` -> PASS\n",
            encoding="utf-8",
        )
        return plan_path, disposition_path

    def write_result(
        self,
        plan_path: Path,
        plan_disposition_path: Path,
        *,
        incidentals: list[dict[str, object]] | None = None,
        reported_paths: list[str] | None = None,
    ) -> tuple[Path, Path]:
        plan_records = [
            json.loads(line)
            for line in plan_disposition_path.read_text(encoding="utf-8").splitlines()
            if line
        ]
        result_records = copy.deepcopy(plan_records)
        for record in result_records:
            record["green_after"]["observed"] = "PASS"
            record["remediation_status"] = "Closed"
        result_records.extend(incidentals or [])
        disposition_path = self.assurance_dir / "remediation.result.dispositions.jsonl"
        result_path = self.assurance_dir / "remediation.result.md"
        write_jsonl(disposition_path, result_records)
        paths = reported_paths
        if paths is None:
            paths = sorted(
                {
                    changed_path
                    for record in result_records
                    for changed_path in record.get("changed_paths", [])
                }
            )
        digest = hashlib.sha256()
        digest.update(plan_path.read_bytes())
        digest.update(b"\0")
        digest.update(plan_disposition_path.read_bytes())
        meta = json.loads(
            (self.assurance_dir / "audit.meta.json").read_text(encoding="utf-8")
        )
        result_path.write_text(
            "# Phase 17 Remediation Result\n\n"
            "**Status**: COMPLETE\n"
            f"**Audit run ID**: `{meta['audit_run_id']}`\n"
            f"**Findings SHA-256**: `{meta['findings_sha256']}`\n"
            f"**Plan SHA-256**: `{digest.hexdigest()}`\n"
            f"**Changed paths**: {json.dumps(paths)}\n",
            encoding="utf-8",
        )
        return result_path, disposition_path


class AuditSetValidatorTests(AssuranceFixture):
    def test_current_audit_set_passes_when_meta_digests_and_verdict_match(self) -> None:
        self.write_audit_set()

        result = run_validator("audit-set", self.assurance_dir)

        self.assertEqual(0, result.returncode, result.stderr)
        self.assertIn("audit-set: PASS", result.stdout)

    def test_fail_is_required_when_one_mandatory_requirement_is_not_assessable(
        self,
    ) -> None:
        requirement = requirement_record(
            state="not-assessable",
            finding_ids=["P17-AUD-001"],
        )
        finding = finding_record()
        self.write_audit_set(
            requirements=[requirement],
            findings=[finding],
            verdict="PASS-WITH-FINDINGS",
        )

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("verdict must be FAIL", result.stderr)

    def test_pass_with_findings_allows_a_major_optional_nonconformance(self) -> None:
        mandatory = requirement_record()
        optional = requirement_record(
            requirement_id="P17-O1",
            mandatory=False,
            state="not-met",
            finding_ids=["P17-AUD-001"],
        )
        finding = finding_record(requirement_ids=["P17-O1"])
        self.write_audit_set(requirements=[mandatory, optional], findings=[finding])

        result = run_validator("audit-set", self.assurance_dir)

        self.assertEqual(0, result.returncode, result.stderr)

    def test_finding_requirement_ids_must_exist_in_requirement_sidecar(self) -> None:
        self.write_audit_set(
            findings=[
                finding_record(
                    requirement_ids=["P17-MISSING"],
                    severity="Minor",
                    conformance_effect="advisory",
                )
            ],
            verdict="PASS-WITH-FINDINGS",
        )

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("unknown requirement P17-MISSING", result.stderr)

    def test_every_finding_and_requirement_must_share_audit_run_id(self) -> None:
        requirement = requirement_record(
            state="not-met",
            finding_ids=["P17-AUD-001"],
        )
        finding = finding_record()
        finding["audit_run_id"] = "phase17-older-run"
        self.write_audit_set(requirements=[requirement], findings=[finding])

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("audit_run_id does not match audit.meta.json", result.stderr)

    def test_finding_identity_is_audit_run_id_plus_id(self) -> None:
        first = finding_record(
            finding_id="P17-AUD-001",
            requirement_ids=["P17-O1"],
        )
        second = finding_record(
            finding_id="P17-AUD-001",
            requirement_ids=["P17-O1"],
        )
        optional = requirement_record(
            requirement_id="P17-O1",
            mandatory=False,
            state="not-met",
            finding_ids=["P17-AUD-001"],
        )
        self.write_audit_set(
            requirements=[requirement_record(), optional],
            findings=[first, second],
        )

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("duplicate (audit_run_id, id)", result.stderr)

    def test_blocking_finding_rejects_met_linked_requirement(self) -> None:
        requirement = requirement_record(finding_ids=["P17-AUD-001"])
        self.write_audit_set(
            requirements=[requirement],
            findings=[finding_record()],
            verdict="PASS-WITH-FINDINGS",
        )

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("blocking finding links met requirement P17-A1", result.stderr)

    def test_standalone_findings_requires_fixed_filename(self) -> None:
        path = self.assurance_dir / "audit.codex.136c380.run1.findings.jsonl"
        write_jsonl(path, [])

        result = run_validator("findings", path)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("expected fixed filename audit.findings.jsonl", result.stderr)

    def test_report_verdict_must_match_meta(self) -> None:
        self.write_audit_set()
        report = self.assurance_dir / "audit.md"
        report.write_text(
            report.read_text(encoding="utf-8").replace("PASS", "FAIL"),
            encoding="utf-8",
        )

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("audit.md Verdict does not match audit.meta.json", result.stderr)


class RemediationValidatorTests(AssuranceFixture):
    def setUp(self) -> None:
        super().setUp()
        requirement = requirement_record(
            state="not-met",
            finding_ids=["P17-AUD-001"],
        )
        self.write_audit_set(requirements=[requirement], findings=[finding_record()])

    def test_plan_source_requires_exact_audit_run_id_and_findings_digest(self) -> None:
        meta = json.loads(
            (self.assurance_dir / "audit.meta.json").read_text(encoding="utf-8")
        )
        record = disposition_record(meta["findings_sha256"])
        record["source"]["findings_sha256"] = "f" * 64
        plan_path, _ = self.write_plan(disposition=record)

        result = run_validator("plan", plan_path)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("source findings_sha256 does not match current audit", result.stderr)

    def test_plan_cannot_reference_a_different_audit_run(self) -> None:
        meta = json.loads(
            (self.assurance_dir / "audit.meta.json").read_text(encoding="utf-8")
        )
        record = disposition_record(
            meta["findings_sha256"],
            audit_run_id="phase17-older-run",
        )
        plan_path, _ = self.write_plan(disposition=record)

        result = run_validator("plan", plan_path)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("source audit_run_id does not match current audit", result.stderr)

    def test_disposition_contract_rejects_lineage_fields(self) -> None:
        meta = json.loads(
            (self.assurance_dir / "audit.meta.json").read_text(encoding="utf-8")
        )
        record = disposition_record(meta["findings_sha256"])
        record["lineage"] = {"kind": "new"}
        plan_path, _ = self.write_plan(disposition=record)

        result = run_validator("plan", plan_path)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("historical lineage fields are forbidden", result.stderr)

    def test_plan_digest_changes_when_plan_or_dispositions_change(self) -> None:
        plan_path, dispositions_path = self.write_plan()

        first = run_validator("plan", plan_path)
        plan_path.write_text(
            plan_path.read_text(encoding="utf-8") + "\n",
            encoding="utf-8",
        )
        second = run_validator("plan", plan_path)
        dispositions_path.write_text(
            dispositions_path.read_text(encoding="utf-8") + "\n",
            encoding="utf-8",
        )
        third = run_validator("plan", plan_path)

        self.assertEqual(0, first.returncode, first.stderr)
        self.assertEqual(0, second.returncode, second.stderr)
        self.assertEqual(0, third.returncode, third.stderr)
        digests = [
            result.stdout.strip().split("plan_sha256=")[1]
            for result in (first, second, third)
        ]
        self.assertEqual(3, len(set(digests)))

    def test_ready_plan_requires_observed_red_before(self) -> None:
        meta = json.loads(
            (self.assurance_dir / "audit.meta.json").read_text(encoding="utf-8")
        )
        record = disposition_record(meta["findings_sha256"])
        record["red_before"]["observed"] = "not-run"
        plan_path, _ = self.write_plan(disposition=record)

        result = run_validator("plan", plan_path)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("READY-FOR-APPLY requires observed FAIL red-before", result.stderr)

    def test_result_accepts_verification_blocking_bounded_incidental_repair(self) -> None:
        plan_path, dispositions_path = self.write_plan()
        result_path, _ = self.write_result(
            plan_path,
            dispositions_path,
            incidentals=[
                incidental_record(changed_path="crates/opi-agent/src/worker.rs")
            ],
        )

        result = run_validator("result", result_path)

        self.assertEqual(0, result.returncode, result.stderr)

    def test_incidental_repair_rejects_public_api_change(self) -> None:
        plan_path, dispositions_path = self.write_plan()
        incidental = incidental_record(changed_path="crates/opi-agent/src/worker.rs")
        incidental["guardrails"]["changes_public_api"] = True
        result_path, _ = self.write_result(
            plan_path,
            dispositions_path,
            incidentals=[incidental],
        )

        result = run_validator("result", result_path)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("incidental repair changes a protected contract", result.stderr)

    def test_incidental_repair_rejects_dependency_or_spec_change(self) -> None:
        for changed_path in ("Cargo.toml", "docs/opi-spec.md"):
            with self.subTest(changed_path=changed_path):
                plan_path, dispositions_path = self.write_plan()
                result_path, _ = self.write_result(
                    plan_path,
                    dispositions_path,
                    incidentals=[incidental_record(changed_path=changed_path)],
                )

                result = run_validator("result", result_path)

                self.assertNotEqual(0, result.returncode)
                self.assertIn("incidental repair names protected path", result.stderr)

    def test_incidental_repair_requires_observed_fail_and_pass(self) -> None:
        for check_name, observed in (
            ("red_before", "not-run"),
            ("green_after", "not-run"),
        ):
            with self.subTest(check_name=check_name):
                plan_path, dispositions_path = self.write_plan()
                incidental = incidental_record(
                    changed_path="crates/opi-agent/src/worker.rs"
                )
                incidental[check_name]["observed"] = observed
                result_path, _ = self.write_result(
                    plan_path,
                    dispositions_path,
                    incidentals=[incidental],
                )

                result = run_validator("result", result_path)

                self.assertNotEqual(0, result.returncode)
                self.assertIn(
                    "incidental repair requires observed FAIL and PASS",
                    result.stderr,
                )

    def test_unrecorded_result_change_is_rejected(self) -> None:
        plan_path, dispositions_path = self.write_plan()
        result_path, _ = self.write_result(
            plan_path,
            dispositions_path,
            reported_paths=[
                "crates/opi-agent/src/session.rs",
                "crates/opi-agent/src/unrecorded.rs",
            ],
        )

        result = run_validator("result", result_path)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("unattributed changed path", result.stderr)

    def test_result_disposition_cannot_drift_from_plan(self) -> None:
        plan_path, dispositions_path = self.write_plan()
        result_path, result_dispositions = self.write_result(
            plan_path,
            dispositions_path,
        )
        records = [
            json.loads(line)
            for line in result_dispositions.read_text(encoding="utf-8").splitlines()
        ]
        records[0]["final_severity"] = "Minor"
        write_jsonl(result_dispositions, records)

        result = run_validator("result", result_path)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("drifts from plan-stage disposition", result.stderr)


class RotationValidatorTests(AssuranceFixture):
    def initialize_repo(self) -> Path:
        repo = self.root
        for args in (
            ("init",),
            ("config", "user.email", "test@example.com"),
            ("config", "user.name", "Assurance Test"),
        ):
            result = run_git(repo, *args)
            self.assertEqual(0, result.returncode, result.stderr)
        return repo

    def commit_active_set(self) -> Path:
        repo = self.initialize_repo()
        self.write_audit_set()
        add = run_git(repo, "add", "--", "docs/snapshots/phase17/assurance")
        self.assertEqual(0, add.returncode, add.stderr)
        commit = run_git(repo, "commit", "-m", "test: add active assurance set")
        self.assertEqual(0, commit.returncode, commit.stderr)
        return repo

    def test_rotation_passes_when_active_set_is_tracked_and_clean(self) -> None:
        self.commit_active_set()

        result = run_validator("rotation", self.phase_dir)

        self.assertEqual(0, result.returncode, result.stderr)

    def test_rotation_rejects_uncommitted_active_set_changes(self) -> None:
        self.commit_active_set()
        (self.assurance_dir / "audit.md").write_text("changed\n", encoding="utf-8")

        result = run_validator("rotation", self.phase_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("active assurance set is not clean", result.stderr)

    def test_rotation_rejects_untracked_active_set_files(self) -> None:
        self.commit_active_set()
        (self.assurance_dir / "notes.md").write_text("untracked\n", encoding="utf-8")

        result = run_validator("rotation", self.phase_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("active assurance set is not clean", result.stderr)

    def test_rotation_rejects_legacy_audit_or_remediation_siblings(self) -> None:
        for name in ("audit.codex.old.md", "remediation.old.plan.md"):
            with self.subTest(name=name):
                nested = self.root / name.replace(".", "-")
                nested.mkdir()
                phase_dir = nested / "docs" / "snapshots" / "phase17"
                phase_dir.mkdir(parents=True)
                repo = nested
                for args in (
                    ("init",),
                    ("config", "user.email", "test@example.com"),
                    ("config", "user.name", "Assurance Test"),
                ):
                    command = run_git(repo, *args)
                    self.assertEqual(0, command.returncode, command.stderr)
                legacy = phase_dir / name
                legacy.write_text("legacy\n", encoding="utf-8")
                add = run_git(repo, "add", "--", f"docs/snapshots/phase17/{name}")
                self.assertEqual(0, add.returncode, add.stderr)
                commit = run_git(repo, "commit", "-m", "test: add legacy artifact")
                self.assertEqual(0, commit.returncode, commit.stderr)
                legacy.unlink()

                result = run_validator("rotation", phase_dir)

                self.assertNotEqual(0, result.returncode)
                self.assertIn("legacy assurance artifact", result.stderr)


if __name__ == "__main__":
    unittest.main()
