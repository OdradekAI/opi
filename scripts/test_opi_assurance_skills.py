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
GENERATION_ID = "phase17-20260824t010203z"


def artifact_stem(reviewer_id: str, model_id: str) -> str:
    return f"audit.{reviewer_id}.{model_id}"


def member_run_id(reviewer_id: str, model_id: str) -> str:
    return f"phase17-{reviewer_id}-{model_id}-136c380-20260824t010203z"


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
    audit_run_id: str = RUN_ID,
    requirement_id: str = "P17-A1",
    mandatory: bool = True,
    state: str = "met",
    finding_ids: list[str] | None = None,
) -> dict[str, object]:
    return {
        "audit_run_id": audit_run_id,
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
        for args in (
            ("init",),
            ("config", "user.email", "test@example.com"),
            ("config", "user.name", "Assurance Test"),
        ):
            result = run_git(self.root, *args)
            self.assertEqual(0, result.returncode, result.stderr)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write_legacy_audit_set(
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

    def write_audit_member(
        self,
        reviewer_id: str,
        model_id: str,
        reviewer_model_id: str,
        *,
        requirements: list[dict[str, object]] | None = None,
        findings: list[dict[str, object]] | None = None,
        verdict: str | None = None,
        audit_head: str = AUDIT_HEAD,
        audit_run_id: str | None = None,
        baseline_hashes: list[str] | None = None,
    ) -> dict[str, object]:
        stem = artifact_stem(reviewer_id, model_id)
        run_id = audit_run_id or member_run_id(reviewer_id, model_id)
        requirement_records = requirements or [requirement_record(audit_run_id=run_id)]
        finding_records = findings or []
        requirements_path = self.assurance_dir / f"{stem}.requirements.jsonl"
        findings_path = self.assurance_dir / f"{stem}.findings.jsonl"
        report_path = self.assurance_dir / f"{stem}.md"
        meta_path = self.assurance_dir / f"{stem}.meta.json"
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
        hashes = baseline_hashes or ["b" * 64, "c" * 64, "d" * 64]
        meta = {
            "schema_version": 3,
            "audit_run_id": run_id,
            "phase": 17,
            "audit_head": audit_head,
            "reviewer_id": reviewer_id,
            "reviewer_identity": reviewer_id.capitalize(),
            "model_id": model_id,
            "reviewer_model_id": reviewer_model_id,
            "model_identity_source": "operator-declared",
            "independence": "fresh-context-same-family",
            "baseline_policy": "latest-committed-spec",
            "baseline_sources": [
                {"path": ".opi-impl-state.json", "sha256": hashes[0]},
                {
                    "path": "docs/snapshots/phase17/opi-impl-state.json",
                    "sha256": hashes[1],
                },
                {"path": "docs/opi-spec.md", "sha256": hashes[2]},
            ],
            "requirements_sha256": sha256(requirements_path),
            "findings_sha256": sha256(findings_path),
            "verdict": derived,
        }
        write_json(meta_path, meta)
        report_path.write_text(
            "# Phase 17 Audit\n\n"
            f"**Audit run ID**: `{run_id}`\n"
            f"**Audit head**: `{audit_head}`\n"
            f"**Reviewer ID**: `{reviewer_id}`\n"
            f"**Model ID**: `{model_id}`\n"
            f"**Reviewer identity**: `{reviewer_id.capitalize()}`\n"
            f"**Reviewer model ID**: `{reviewer_model_id}`\n"
            "**Model identity source**: `operator-declared`\n"
            f"**Verdict**: {derived}\n",
            encoding="utf-8",
        )
        return {
            "reviewer_id": reviewer_id,
            "model_id": model_id,
            "artifact_stem": stem,
            "audit_run_id": run_id,
            "audit_head": audit_head,
            "verdict": derived,
            "digests": {
                "meta_sha256": sha256(meta_path),
                "requirements_sha256": sha256(requirements_path),
                "findings_sha256": sha256(findings_path),
                "report_sha256": sha256(report_path),
            },
        }

    def write_audit_index(self, members: list[dict[str, object]]) -> Path:
        sorted_members = sorted(
            members,
            key=lambda member: (str(member["reviewer_id"]), str(member["model_id"])),
        )
        verdicts = [str(member["verdict"]) for member in sorted_members]
        aggregate = (
            "FAIL"
            if "FAIL" in verdicts
            else "PASS-WITH-FINDINGS"
            if "PASS-WITH-FINDINGS" in verdicts
            else "PASS"
        )
        path = self.assurance_dir / "audit.index.json"
        write_json(
            path,
            {
                "schema_version": 2,
                "phase": 17,
                "revision": 1,
                "aggregate_verdict": aggregate,
                "members": sorted_members,
            },
        )
        return path

    def write_audit_set(
        self,
        *,
        requirements: list[dict[str, object]] | None = None,
        findings: list[dict[str, object]] | None = None,
        verdict: str | None = None,
    ) -> dict[str, Path]:
        member = self.write_audit_member(
            "codex",
            "gpt56",
            "gpt-5.6",
            requirements=requirements,
            findings=findings,
            verdict=verdict,
            audit_run_id=RUN_ID,
        )
        self.write_audit_index([member])
        stem = artifact_stem("codex", "gpt56")
        return {
            "meta": self.assurance_dir / f"{stem}.meta.json",
            "requirements": self.assurance_dir / f"{stem}.requirements.jsonl",
            "findings": self.assurance_dir / f"{stem}.findings.jsonl",
            "report": self.assurance_dir / f"{stem}.md",
            "index": self.assurance_dir / "audit.index.json",
        }

    def write_plan(
        self,
        *,
        disposition: dict[str, object] | None = None,
        dispositions: list[dict[str, object]] | None = None,
        status: str = "READY-FOR-APPLY",
    ) -> tuple[Path, Path]:
        index_path = self.assurance_dir / "audit.index.json"
        index = json.loads(index_path.read_text(encoding="utf-8"))
        first_member = index["members"][0]
        first_meta = json.loads(
            (
                self.assurance_dir
                / f"{first_member['artifact_stem']}.meta.json"
            ).read_text(encoding="utf-8")
        )
        records = dispositions or [
            disposition
            or disposition_record(
                first_meta["findings_sha256"],
                audit_run_id=first_meta["audit_run_id"],
            )
        ]
        disposition_path = self.assurance_dir / "remediation.plan.dispositions.jsonl"
        plan_path = self.assurance_dir / "remediation.plan.md"
        write_jsonl(disposition_path, records)
        source_lines = "\n".join(
            f"Source finding: {record['source']['audit_run_id']}/{record['source']['id']}"
            for record in records
            if record.get("record_kind") == "finding-disposition"
        )
        plan_path.write_text(
            "# Phase 17 Remediation Plan\n\n"
            f"**Status**: {status}\n"
            f"**Audit index SHA-256**: `{sha256(index_path)}`\n"
            f"**Remediation head**: `{REMEDIATION_HEAD}`\n"
            "**Disposition artifact**: `remediation.plan.dispositions.jsonl`\n"
            "**Unresolved decisions**: none\n\n"
            f"{source_lines}\n\n"
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
        index_path = self.assurance_dir / "audit.index.json"
        result_path.write_text(
            "# Phase 17 Remediation Result\n\n"
            "**Status**: COMPLETE\n"
            f"**Audit index SHA-256**: `{sha256(index_path)}`\n"
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

    def test_standalone_findings_rejects_invalid_member_filename(self) -> None:
        path = self.assurance_dir / "audit.codex.136c380.run1.findings.jsonl"
        write_jsonl(path, [])

        result = run_validator("findings", path)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("filename must match audit.<reviewer>.<model>", result.stderr)

    def test_standalone_findings_accepts_indexed_member_filename(self) -> None:
        paths = self.write_audit_set()

        result = run_validator("findings", paths["findings"])

        self.assertEqual(0, result.returncode, result.stderr)

    def test_report_verdict_must_match_meta(self) -> None:
        paths = self.write_audit_set()
        report = paths["report"]
        report.write_text(
            report.read_text(encoding="utf-8").replace("PASS", "FAIL"),
            encoding="utf-8",
        )

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("report Verdict does not match metadata", result.stderr)


class IndexedAuditSetValidatorTests(AssuranceFixture):
    def test_two_active_members_derive_pass(self) -> None:
        codex = self.write_audit_member("codex", "gpt56", "gpt-5.6")
        claude = self.write_audit_member("claude", "glm53", "glm-5.3")
        self.write_audit_index([codex, claude])

        result = run_validator("audit-set", self.assurance_dir)

        self.assertEqual(0, result.returncode, result.stderr)

    def test_unexpected_root_directory_is_rejected(self) -> None:
        member = self.write_audit_member("codex", "gpt56", "gpt-5.6")
        self.write_audit_index([member])
        (self.assurance_dir / "scratch").mkdir()

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("unexpected directory: scratch", result.stderr)

    def test_one_failed_member_derives_fail(self) -> None:
        codex = self.write_audit_member("codex", "gpt56", "gpt-5.6")
        run_id = member_run_id("claude", "glm53")
        failed = self.write_audit_member(
            "claude",
            "glm53",
            "glm-5.3",
            requirements=[
                requirement_record(
                    audit_run_id=run_id,
                    state="not-met",
                    finding_ids=["P17-AUD-001"],
                )
            ],
            findings=[
                finding_record(
                    audit_run_id=run_id,
                    source_path=(
                        "docs/snapshots/phase17/assurance/"
                        "audit.claude.glm53.md"
                    ),
                    source_model="glm-5.3",
                )
            ],
        )
        index_path = self.write_audit_index([codex, failed])

        result = run_validator("audit-set", self.assurance_dir)

        self.assertEqual(0, result.returncode, result.stderr)
        index = json.loads(index_path.read_text(encoding="utf-8"))
        self.assertEqual("FAIL", index["aggregate_verdict"])

    def test_pass_with_findings_is_preserved_in_aggregate(self) -> None:
        run_id = member_run_id("codex", "gpt56")
        member = self.write_audit_member(
            "codex",
            "gpt56",
            "gpt-5.6",
            requirements=[
                requirement_record(audit_run_id=run_id),
                requirement_record(
                    audit_run_id=run_id,
                    requirement_id="P17-O1",
                    mandatory=False,
                    state="not-met",
                    finding_ids=["P17-AUD-001"],
                ),
            ],
            findings=[
                finding_record(
                    audit_run_id=run_id,
                    source_path=(
                        "docs/snapshots/phase17/assurance/"
                        "audit.codex.gpt56.md"
                    ),
                    source_model="gpt-5.6",
                    requirement_ids=["P17-O1"],
                )
            ],
        )
        index_path = self.write_audit_index([member])

        result = run_validator("audit-set", self.assurance_dir)

        self.assertEqual(0, result.returncode, result.stderr)
        index = json.loads(index_path.read_text(encoding="utf-8"))
        self.assertEqual("PASS-WITH-FINDINGS", index["aggregate_verdict"])

    def test_missing_indexed_member_file_is_rejected(self) -> None:
        member = self.write_audit_member("codex", "gpt56", "gpt-5.6")
        self.write_audit_index([member])
        (self.assurance_dir / "audit.codex.gpt56.md").unlink()

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("missing member file: audit.codex.gpt56.md", result.stderr)

    def test_member_head_must_match_index_member_entry(self) -> None:
        member = self.write_audit_member(
            "codex",
            "gpt56",
            "gpt-5.6",
            audit_head=REMEDIATION_HEAD,
        )
        index_path = self.write_audit_index([member])
        index = json.loads(index_path.read_text(encoding="utf-8"))
        index["members"][0]["audit_head"] = AUDIT_HEAD
        write_json(index_path, index)

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("metadata audit_head does not match index", result.stderr)

    def test_mixed_member_heads_are_accepted(self) -> None:
        codex = self.write_audit_member("codex", "gpt56", "gpt-5.6")
        claude = self.write_audit_member(
            "claude",
            "glm53",
            "glm-5.3",
            audit_head=REMEDIATION_HEAD,
            baseline_hashes=["e" * 64, "f" * 64, "9" * 64],
        )
        self.write_audit_index([codex, claude])

        result = run_validator("audit-set", self.assurance_dir)

        self.assertEqual(0, result.returncode, result.stderr)

    def test_member_entry_requires_audit_head(self) -> None:
        member = self.write_audit_member("codex", "gpt56", "gpt-5.6")
        index_path = self.write_audit_index([member])
        index = json.loads(index_path.read_text(encoding="utf-8"))
        del index["members"][0]["audit_head"]
        write_json(index_path, index)

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("index member 1: missing fields: audit_head", result.stderr)

    def test_index_schema_1_is_rejected(self) -> None:
        member = self.write_audit_member("codex", "gpt56", "gpt-5.6")
        index_path = self.assurance_dir / "audit.index.json"
        write_json(
            index_path,
            {
                "schema_version": 1,
                "phase": 17,
                "audit_generation_id": GENERATION_ID,
                "audit_head": AUDIT_HEAD,
                "revision": 1,
                "aggregate_verdict": member["verdict"],
                "members": [member],
            },
        )

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("schema_version must be 2", result.stderr)

    def test_member_meta_schema_2_is_rejected(self) -> None:
        member = self.write_audit_member("codex", "gpt56", "gpt-5.6")
        index_path = self.write_audit_index([member])
        meta_path = self.assurance_dir / "audit.codex.gpt56.meta.json"
        meta = json.loads(meta_path.read_text(encoding="utf-8"))
        meta["schema_version"] = 2
        meta["audit_generation_id"] = GENERATION_ID
        write_json(meta_path, meta)
        index = json.loads(index_path.read_text(encoding="utf-8"))
        index["members"][0]["digests"]["meta_sha256"] = sha256(meta_path)
        write_json(index_path, index)

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("member metadata schema_version must be 3", result.stderr)

    def test_filename_and_metadata_reviewer_identity_must_match(self) -> None:
        member = self.write_audit_member("codex", "gpt56", "gpt-5.6")
        index_path = self.write_audit_index([member])
        meta_path = self.assurance_dir / "audit.codex.gpt56.meta.json"
        meta = json.loads(meta_path.read_text(encoding="utf-8"))
        meta["reviewer_id"] = "claude"
        write_json(meta_path, meta)
        index = json.loads(index_path.read_text(encoding="utf-8"))
        index["members"][0]["digests"]["meta_sha256"] = sha256(meta_path)
        write_json(index_path, index)

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("metadata reviewer_id does not match index", result.stderr)

    def test_index_digest_must_match_exact_member_bytes(self) -> None:
        member = self.write_audit_member("codex", "gpt56", "gpt-5.6")
        index_path = self.write_audit_index([member])
        index = json.loads(index_path.read_text(encoding="utf-8"))
        index["members"][0]["digests"]["report_sha256"] = "f" * 64
        write_json(index_path, index)

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("report_sha256 does not match", result.stderr)

    def test_full_model_identity_header_must_match_member_metadata(self) -> None:
        member = self.write_audit_member("codex", "gpt56", "gpt-5.6")
        index_path = self.write_audit_index([member])
        report_path = self.assurance_dir / "audit.codex.gpt56.md"
        report_path.write_text(
            report_path.read_text(encoding="utf-8").replace(
                "**Reviewer model ID**: `gpt-5.6`",
                "**Reviewer model ID**: `gpt-5.5`",
            ),
            encoding="utf-8",
        )
        index = json.loads(index_path.read_text(encoding="utf-8"))
        index["members"][0]["digests"]["report_sha256"] = sha256(report_path)
        write_json(index_path, index)

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("report Reviewer model ID does not match metadata", result.stderr)

    def test_members_must_be_sorted(self) -> None:
        codex = self.write_audit_member("codex", "gpt56", "gpt-5.6")
        claude = self.write_audit_member("claude", "glm53", "glm-5.3")
        index_path = self.write_audit_index([codex, claude])
        index = json.loads(index_path.read_text(encoding="utf-8"))
        index["members"].reverse()
        write_json(index_path, index)

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("members must be sorted", result.stderr)

    def test_duplicate_reviewer_model_pair_is_rejected(self) -> None:
        member = self.write_audit_member("codex", "gpt56", "gpt-5.6")
        index_path = self.write_audit_index([member])
        index = json.loads(index_path.read_text(encoding="utf-8"))
        index["members"].append(copy.deepcopy(index["members"][0]))
        write_json(index_path, index)

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("duplicate reviewer/model pair", result.stderr)

    def test_unindexed_dynamic_file_is_rejected(self) -> None:
        member = self.write_audit_member("codex", "gpt56", "gpt-5.6")
        self.write_audit_index([member])
        (self.assurance_dir / "audit.claude.glm53.md").write_text(
            "orphan\n",
            encoding="utf-8",
        )

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("unindexed files: audit.claude.glm53.md", result.stderr)

    def test_baseline_source_digests_may_differ_across_heads(self) -> None:
        codex = self.write_audit_member("codex", "gpt56", "gpt-5.6")
        claude = self.write_audit_member(
            "claude",
            "glm53",
            "glm-5.3",
            audit_head=REMEDIATION_HEAD,
            baseline_hashes=["e" * 64, "f" * 64, "9" * 64],
        )
        self.write_audit_index([codex, claude])

        result = run_validator("audit-set", self.assurance_dir)

        self.assertEqual(0, result.returncode, result.stderr)

    def test_baseline_paths_must_match_across_members(self) -> None:
        codex = self.write_audit_member("codex", "gpt56", "gpt-5.6")
        claude = self.write_audit_member("claude", "glm53", "glm-5.3")
        meta_path = self.assurance_dir / "audit.claude.glm53.meta.json"
        meta = json.loads(meta_path.read_text(encoding="utf-8"))
        meta["baseline_sources"][0]["path"] = "docs/snapshots/phase16/opi-impl-state.json"
        write_json(meta_path, meta)
        claude["digests"]["meta_sha256"] = sha256(meta_path)
        self.write_audit_index([codex, claude])

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn(
            "baseline paths do not match the other active members", result.stderr
        )

    def test_legacy_unsuffixed_audit_set_is_rejected(self) -> None:
        self.write_legacy_audit_set()

        result = run_validator("audit-set", self.assurance_dir)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("active audit set requires audit.index.json", result.stderr)


class RemediationValidatorTests(AssuranceFixture):
    def setUp(self) -> None:
        super().setUp()
        requirement = requirement_record(
            state="not-met",
            finding_ids=["P17-AUD-001"],
        )
        self.write_audit_set(requirements=[requirement], findings=[finding_record()])

    def write_two_member_failed_set(
        self,
    ) -> tuple[dict[str, object], dict[str, object]]:
        for path in self.assurance_dir.iterdir():
            if path.is_file():
                path.unlink()
        members: list[dict[str, object]] = []
        for reviewer_id, model_id, reviewer_model_id in (
            ("claude", "glm53", "glm-5.3"),
            ("codex", "gpt56", "gpt-5.6"),
        ):
            run_id = member_run_id(reviewer_id, model_id)
            members.append(
                self.write_audit_member(
                    reviewer_id,
                    model_id,
                    reviewer_model_id,
                    requirements=[
                        requirement_record(
                            audit_run_id=run_id,
                            state="not-met",
                            finding_ids=["P17-AUD-001"],
                        )
                    ],
                    findings=[
                        finding_record(
                            audit_run_id=run_id,
                            source_path=(
                                "docs/snapshots/phase17/assurance/"
                                f"audit.{reviewer_id}.{model_id}.md"
                            ),
                            source_model=reviewer_model_id,
                        )
                    ],
                )
            )
        self.write_audit_index(members)
        metadata = []
        for member in members:
            metadata.append(
                json.loads(
                    (
                        self.assurance_dir
                        / f"{member['artifact_stem']}.meta.json"
                    ).read_text(encoding="utf-8")
                )
            )
        return metadata[0], metadata[1]

    def dispositions_for(
        self, *metadata: dict[str, object]
    ) -> list[dict[str, object]]:
        return [
            disposition_record(
                str(meta["findings_sha256"]),
                audit_run_id=str(meta["audit_run_id"]),
            )
            for meta in metadata
        ]

    def test_plan_covers_strict_union_of_duplicate_textual_ids(self) -> None:
        first, second = self.write_two_member_failed_set()
        plan_path, _ = self.write_plan(
            dispositions=self.dispositions_for(first, second)
        )

        result = run_validator("plan", plan_path)

        self.assertEqual(0, result.returncode, result.stderr)

    def test_plan_omitting_one_run_identity_fails_union_coverage(self) -> None:
        first, second = self.write_two_member_failed_set()
        plan_path, _ = self.write_plan(dispositions=self.dispositions_for(first))

        result = run_validator("plan", plan_path)

        self.assertNotEqual(0, result.returncode)
        self.assertIn(
            f"{second['audit_run_id']}/P17-AUD-001",
            result.stderr,
        )

    def test_source_digest_must_match_owning_member(self) -> None:
        first, second = self.write_two_member_failed_set()
        records = self.dispositions_for(first, second)
        records[0]["source"]["findings_sha256"] = second["findings_sha256"]
        plan_path, _ = self.write_plan(dispositions=records)

        result = run_validator("plan", plan_path)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("source findings_sha256 does not match owning audit", result.stderr)

    def test_source_run_absent_from_index_is_rejected(self) -> None:
        first, second = self.write_two_member_failed_set()
        records = self.dispositions_for(first, second)
        records[0]["source"]["audit_run_id"] = "phase17-absent-run"
        plan_path, _ = self.write_plan(dispositions=records)

        result = run_validator("plan", plan_path)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("source audit_run_id is not in current index", result.stderr)

    def test_index_change_invalidates_existing_plan_binding(self) -> None:
        first, second = self.write_two_member_failed_set()
        plan_path, _ = self.write_plan(
            dispositions=self.dispositions_for(first, second)
        )
        index_path = self.assurance_dir / "audit.index.json"
        index = json.loads(index_path.read_text(encoding="utf-8"))
        report_path = self.assurance_dir / "audit.codex.gpt56.md"
        report_path.write_text(
            report_path.read_text(encoding="utf-8") + "\n",
            encoding="utf-8",
        )
        codex = next(
            member
            for member in index["members"]
            if member["reviewer_id"] == "codex"
        )
        codex["digests"]["report_sha256"] = sha256(report_path)
        write_json(index_path, index)

        result = run_validator("plan", plan_path)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("Audit index SHA-256 does not match current audit", result.stderr)

    def test_result_preserves_exact_two_run_source_set(self) -> None:
        first, second = self.write_two_member_failed_set()
        plan_path, dispositions_path = self.write_plan(
            dispositions=self.dispositions_for(first, second)
        )
        result_path, _ = self.write_result(plan_path, dispositions_path)

        result = run_validator("result", result_path)

        self.assertEqual(0, result.returncode, result.stderr)

    def test_result_omitting_one_plan_source_fails_exact_coverage(self) -> None:
        first, second = self.write_two_member_failed_set()
        plan_path, dispositions_path = self.write_plan(
            dispositions=self.dispositions_for(first, second)
        )
        result_path, result_dispositions = self.write_result(
            plan_path, dispositions_path
        )
        records = [
            json.loads(line)
            for line in result_dispositions.read_text(encoding="utf-8").splitlines()
        ]
        write_jsonl(result_dispositions, records[:1])

        result = run_validator("result", result_path)

        self.assertNotEqual(0, result.returncode)
        self.assertIn(
            "result dispositions must cover exactly the plan sources",
            result.stderr,
        )

    def test_shared_closure_batch_accepts_two_source_keys(self) -> None:
        first, second = self.write_two_member_failed_set()
        records = self.dispositions_for(first, second)
        self.assertEqual(records[0]["closure_key"], records[1]["closure_key"])
        self.assertEqual(records[0]["closure_batch"], records[1]["closure_batch"])
        plan_path, _ = self.write_plan(dispositions=records)

        result = run_validator("plan", plan_path)

        self.assertEqual(0, result.returncode, result.stderr)

    def test_shared_batch_rejects_different_closure_keys(self) -> None:
        first, second = self.write_two_member_failed_set()
        records = self.dispositions_for(first, second)
        records[1]["closure_key"] = "different.behavior"
        plan_path, _ = self.write_plan(dispositions=records)

        result = run_validator("plan", plan_path)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("closure batch B1 has multiple closure keys", result.stderr)

    def test_plan_source_requires_exact_audit_run_id_and_findings_digest(self) -> None:
        meta = json.loads(
            (self.assurance_dir / "audit.codex.gpt56.meta.json").read_text(
                encoding="utf-8"
            )
        )
        record = disposition_record(meta["findings_sha256"])
        record["source"]["findings_sha256"] = "f" * 64
        plan_path, _ = self.write_plan(disposition=record)

        result = run_validator("plan", plan_path)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("source findings_sha256 does not match owning audit", result.stderr)

    def test_plan_cannot_reference_a_different_audit_run(self) -> None:
        meta = json.loads(
            (self.assurance_dir / "audit.codex.gpt56.meta.json").read_text(
                encoding="utf-8"
            )
        )
        record = disposition_record(
            meta["findings_sha256"],
            audit_run_id="phase17-older-run",
        )
        plan_path, _ = self.write_plan(disposition=record)

        result = run_validator("plan", plan_path)

        self.assertNotEqual(0, result.returncode)
        self.assertIn("source audit_run_id is not in current index", result.stderr)

    def test_disposition_contract_rejects_lineage_fields(self) -> None:
        meta = json.loads(
            (self.assurance_dir / "audit.codex.gpt56.meta.json").read_text(
                encoding="utf-8"
            )
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
            (self.assurance_dir / "audit.codex.gpt56.meta.json").read_text(
                encoding="utf-8"
            )
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
        (self.assurance_dir / "audit.codex.gpt56.md").write_text(
            "changed\n",
            encoding="utf-8",
        )

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
