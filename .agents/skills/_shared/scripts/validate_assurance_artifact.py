#!/usr/bin/env python3
"""Validate the current Opi audit and remediation assurance set."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path, PurePosixPath
from typing import Any


FULL_SHA_RE = re.compile(r"^[0-9a-f]{40}$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
RUN_ID_RE = re.compile(r"^phase([1-9][0-9]*)-[a-z0-9][a-z0-9.-]*$")
PHASE_RE = re.compile(r"^phase([1-9][0-9]*)$")
INCIDENTAL_ID_RE = re.compile(r"^I[1-9][0-9]*$")

AUDIT_FILES = {
    "audit.meta.json",
    "audit.requirements.jsonl",
    "audit.findings.jsonl",
    "audit.md",
}
REMEDIATION_FILES = {
    "remediation.plan.md",
    "remediation.plan.dispositions.jsonl",
    "remediation.result.md",
    "remediation.result.dispositions.jsonl",
}
ALLOWED_ASSURANCE_FILES = AUDIT_FILES | REMEDIATION_FILES

META_FIELDS = {
    "schema_version",
    "audit_run_id",
    "phase",
    "audit_head",
    "reviewer_model",
    "independence",
    "baseline_policy",
    "baseline_sources",
    "requirements_sha256",
    "findings_sha256",
    "verdict",
}
REQUIREMENT_FIELDS = {
    "audit_run_id",
    "id",
    "mandatory",
    "criterion_source",
    "observable_behavior",
    "production_surfaces",
    "test_evidence",
    "checks",
    "state",
    "finding_ids",
}
FINDING_FIELDS = {
    "audit_run_id",
    "id",
    "source_kind",
    "source_path",
    "source_model",
    "observed_at",
    "independence",
    "axis",
    "severity",
    "conformance_effect",
    "title",
    "claim",
    "evidence",
    "requirement_ids",
    "criterion_source",
    "reproduction",
    "confidence",
    "status",
}
DISPOSITION_FIELDS = {
    "record_kind",
    "source",
    "verified_at",
    "verification_status",
    "final_severity",
    "final_severity_rationale",
    "closure_key",
    "family_key",
    "decision",
    "closure_batch",
    "change_kind",
    "changed_paths",
    "green_after",
}
INCIDENTAL_FIELDS = {
    "record_kind",
    "id",
    "trigger_batch",
    "blocking_check",
    "scope_rationale",
    "guardrails",
    "changed_paths",
    "red_before",
    "green_after",
    "remediation_status",
}
GUARDRAIL_FIELDS = {
    "required_for_green",
    "within_causal_surface",
    "changes_public_api",
    "changes_durable_format",
    "changes_dependency_graph",
    "changes_spec_or_authority",
}
HISTORICAL_FIELDS = {"lineage", "prior_occurrences", "prior_disposition"}

INDEPENDENCE = {
    "independent-family",
    "fresh-context-same-family",
    "unknown",
}
AXES = {
    "standards",
    "spec",
    "security",
    "test-quality",
    "invariants",
    "integration",
    "residuals",
    "runtime-fidelity",
}
SEVERITIES = {"Blocker", "Major", "Minor", "Info"}
CONFORMANCE_EFFECTS = {"blocks", "advisory"}
CONFIDENCE = {"high", "medium", "low"}
REQUIREMENT_STATES = {"met", "partially-met", "not-met", "not-assessable"}
VERDICTS = {"FAIL", "PASS-WITH-FINDINGS", "PASS"}
VERIFICATION = {"Confirmed", "Partially confirmed", "Cannot confirm", "Refuted"}
CHANGE_KINDS = {"behavioral", "test-only", "documentation", "metadata"}
PLAN_STATUSES = {"DRAFT-UNRESOLVED", "READY-FOR-APPLY"}
REMEDIATION_STATUSES = {
    "Closed",
    "Not closed",
    "Deferred by registered source",
    "Returned to shaping",
    "Info/No action",
    "Refuted",
    "Cannot confirm",
}


def nonempty_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def string_list(value: Any, *, allow_empty: bool = True) -> bool:
    return (
        isinstance(value, list)
        and (allow_empty or bool(value))
        and all(nonempty_string(item) for item in value)
    )


def missing_fields(record: dict[str, Any], fields: set[str]) -> list[str]:
    return sorted(field for field in fields if field not in record)


def read_json(path: Path, errors: list[str]) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8-sig"))
    except OSError as exc:
        errors.append(f"cannot read {path}: {exc}")
        return {}
    except json.JSONDecodeError as exc:
        errors.append(f"invalid JSON in {path.name}: {exc.msg}")
        return {}
    if not isinstance(value, dict):
        errors.append(f"{path.name} must contain one JSON object")
        return {}
    return value


def load_jsonl(
    path: Path,
    errors: list[str],
    *,
    allow_empty: bool = False,
) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    try:
        lines = path.read_text(encoding="utf-8-sig").splitlines()
    except OSError as exc:
        errors.append(f"cannot read {path}: {exc}")
        return records
    for number, line in enumerate(lines, start=1):
        if not line.strip():
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError as exc:
            errors.append(f"line {number}: invalid JSON: {exc.msg}")
            continue
        if not isinstance(value, dict):
            errors.append(f"line {number}: record must be an object")
            continue
        records.append(value)
    if not records and not allow_empty:
        errors.append("artifact contains no records")
    return records


def raw_sha256(path: Path, errors: list[str]) -> str | None:
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError as exc:
        errors.append(f"cannot read {path}: {exc}")
        return None


def check_record(value: Any) -> bool:
    return (
        isinstance(value, dict)
        and all(key in value for key in ("command", "expected", "observed"))
        and nonempty_string(value.get("command"))
        and nonempty_string(value.get("expected"))
        and nonempty_string(value.get("observed"))
    )


def header_value(text: str, label: str) -> str | None:
    match = re.search(rf"(?m)^\*\*{re.escape(label)}\*\*:\s*(.+?)\s*$", text)
    return match.group(1).strip() if match else None


def unquote_code(value: str | None) -> str | None:
    if value is None:
        return None
    match = re.fullmatch(r"`([^`]+)`", value)
    return match.group(1) if match else value


def validate_meta(meta: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    missing = missing_fields(meta, META_FIELDS)
    if missing:
        errors.append("audit.meta.json missing fields: " + ", ".join(missing))
        return errors
    if meta.get("schema_version") != 1:
        errors.append("audit.meta.json schema_version must be 1")
    phase = meta.get("phase")
    if not isinstance(phase, int) or isinstance(phase, bool) or phase < 1:
        errors.append("audit.meta.json phase must be a positive integer")
    run_id = meta.get("audit_run_id")
    match = RUN_ID_RE.fullmatch(run_id) if isinstance(run_id, str) else None
    if match is None:
        errors.append("audit.meta.json audit_run_id has invalid format")
    elif isinstance(phase, int) and int(match.group(1)) != phase:
        errors.append("audit.meta.json audit_run_id Phase does not match phase")
    if not isinstance(meta.get("audit_head"), str) or FULL_SHA_RE.fullmatch(
        meta["audit_head"]
    ) is None:
        errors.append("audit.meta.json audit_head must be a full lowercase commit SHA")
    if not nonempty_string(meta.get("reviewer_model")):
        errors.append("audit.meta.json reviewer_model must be non-empty")
    if meta.get("independence") not in INDEPENDENCE:
        errors.append("audit.meta.json independence is invalid")
    if meta.get("baseline_policy") != "latest-committed-spec":
        errors.append("audit.meta.json baseline_policy must be latest-committed-spec")
    for field in ("requirements_sha256", "findings_sha256"):
        value = meta.get(field)
        if not isinstance(value, str) or SHA256_RE.fullmatch(value) is None:
            errors.append(f"audit.meta.json {field} must be a lowercase SHA-256")
    if meta.get("verdict") not in VERDICTS:
        errors.append("audit.meta.json verdict is invalid")
    sources = meta.get("baseline_sources")
    source_paths: set[str] = set()
    if not isinstance(sources, list) or not sources:
        errors.append("audit.meta.json baseline_sources must be a non-empty list")
    else:
        for number, source in enumerate(sources, start=1):
            if not isinstance(source, dict):
                errors.append(f"baseline source {number} must be an object")
                continue
            path = source.get("path")
            digest = source.get("sha256")
            if not nonempty_string(path):
                errors.append(f"baseline source {number} path must be non-empty")
            else:
                source_paths.add(path)
            if not isinstance(digest, str) or SHA256_RE.fullmatch(digest) is None:
                errors.append(f"baseline source {number} sha256 must be lowercase SHA-256")
    if isinstance(phase, int):
        required_sources = {
            ".opi-impl-state.json",
            f"docs/snapshots/phase{phase}/opi-impl-state.json",
            "docs/opi-spec.md",
        }
        missing_sources = sorted(required_sources - source_paths)
        if missing_sources:
            errors.append(
                "audit.meta.json baseline_sources missing: " + ", ".join(missing_sources)
            )
    return errors


def validate_requirement_records(
    path: Path,
    meta: dict[str, Any] | None,
) -> tuple[list[dict[str, Any]], list[str]]:
    errors: list[str] = []
    if path.name != "audit.requirements.jsonl":
        errors.append("expected fixed filename audit.requirements.jsonl")
    records = load_jsonl(path, errors)
    seen: set[str] = set()
    for number, record in enumerate(records, start=1):
        prefix = f"requirement {number}: "
        missing = missing_fields(record, REQUIREMENT_FIELDS)
        if missing:
            errors.append(prefix + "missing fields: " + ", ".join(missing))
            continue
        if meta is not None and record.get("audit_run_id") != meta.get("audit_run_id"):
            errors.append(prefix + "audit_run_id does not match audit.meta.json")
        requirement_id = record.get("id")
        if not nonempty_string(requirement_id):
            errors.append(prefix + "id must be non-empty")
        elif requirement_id in seen:
            errors.append(prefix + "duplicate requirement id")
        else:
            seen.add(requirement_id)
        if not isinstance(record.get("mandatory"), bool):
            errors.append(prefix + "mandatory must be boolean")
        criterion = record.get("criterion_source")
        if not isinstance(criterion, dict):
            errors.append(prefix + "criterion_source must be an object")
        else:
            for field in ("path", "citation"):
                if not nonempty_string(criterion.get(field)):
                    errors.append(prefix + f"criterion_source {field} must be non-empty")
            digest = criterion.get("sha256")
            if not isinstance(digest, str) or SHA256_RE.fullmatch(digest) is None:
                errors.append(prefix + "criterion_source sha256 must be lowercase SHA-256")
        if not nonempty_string(record.get("observable_behavior")):
            errors.append(prefix + "observable_behavior must be non-empty")
        for field in ("production_surfaces", "test_evidence", "finding_ids"):
            if not string_list(record.get(field)):
                errors.append(prefix + f"{field} must be a list of non-empty strings")
        checks = record.get("checks")
        if not isinstance(checks, list) or any(
            not isinstance(check, dict)
            or not nonempty_string(check.get("command"))
            or not nonempty_string(check.get("observed"))
            for check in checks
        ):
            errors.append(prefix + "checks must contain command and observed")
        if record.get("state") not in REQUIREMENT_STATES:
            errors.append(prefix + "invalid state")
    if records and not any(record.get("mandatory") is True for record in records):
        errors.append("requirements must include at least one mandatory record")
    return records, errors


def validate_requirements(path: Path) -> list[str]:
    meta_path = path.with_name("audit.meta.json")
    meta = read_json(meta_path, []) if meta_path.is_file() else None
    _, errors = validate_requirement_records(path, meta)
    return errors


def validate_finding_records(
    path: Path,
    meta: dict[str, Any] | None,
) -> tuple[list[dict[str, Any]], list[str]]:
    errors: list[str] = []
    if path.name != "audit.findings.jsonl":
        errors.append("expected fixed filename audit.findings.jsonl")
    if not path.with_name("audit.md").is_file():
        errors.append("missing fixed report sibling: audit.md")
    records = load_jsonl(path, errors, allow_empty=True)
    seen: set[tuple[str, str]] = set()
    for number, record in enumerate(records, start=1):
        prefix = f"finding {number}: "
        missing = missing_fields(record, FINDING_FIELDS)
        if missing:
            errors.append(prefix + "missing fields: " + ", ".join(missing))
            continue
        run_id = record.get("audit_run_id")
        if meta is not None and run_id != meta.get("audit_run_id"):
            errors.append(prefix + "audit_run_id does not match audit.meta.json")
        finding_id = record.get("id")
        if not nonempty_string(finding_id):
            errors.append(prefix + "id must be non-empty")
        key = (str(run_id), str(finding_id))
        if key in seen:
            errors.append(prefix + "duplicate (audit_run_id, id)")
        seen.add(key)
        if record.get("source_kind") != "audit":
            errors.append(prefix + "source_kind must be audit")
        phase = meta.get("phase") if meta is not None else None
        expected_source = (
            f"docs/snapshots/phase{phase}/assurance/audit.md"
            if isinstance(phase, int)
            else None
        )
        source_path = record.get("source_path")
        if expected_source is not None and source_path != expected_source:
            errors.append(prefix + "source_path must name the fixed Phase audit.md")
        elif expected_source is None and not (
            isinstance(source_path, str)
            and re.fullmatch(r"docs/snapshots/phase[1-9][0-9]*/assurance/audit\.md", source_path)
        ):
            errors.append(prefix + "source_path must name the fixed Phase audit.md")
        if not nonempty_string(record.get("source_model")):
            errors.append(prefix + "source_model must be non-empty")
        observed_at = record.get("observed_at")
        if not isinstance(observed_at, str) or FULL_SHA_RE.fullmatch(observed_at) is None:
            errors.append(prefix + "observed_at must be a full lowercase commit SHA")
        elif meta is not None and observed_at != meta.get("audit_head"):
            errors.append(prefix + "observed_at does not match audit_head")
        if record.get("independence") not in INDEPENDENCE:
            errors.append(prefix + "invalid independence")
        if record.get("axis") not in AXES:
            errors.append(prefix + "invalid axis")
        severity = record.get("severity")
        effect = record.get("conformance_effect")
        if severity not in SEVERITIES:
            errors.append(prefix + "invalid severity")
        if effect not in CONFORMANCE_EFFECTS:
            errors.append(prefix + "invalid conformance_effect")
        if severity in {"Blocker", "Major"} and effect != "blocks":
            errors.append(prefix + "Blocker and Major findings must block conformance")
        if effect == "advisory" and severity not in {"Minor", "Info"}:
            errors.append(prefix + "advisory findings must be Minor or Info")
        for field in ("title", "claim"):
            if not nonempty_string(record.get(field)):
                errors.append(prefix + f"{field} must be non-empty")
        evidence = record.get("evidence")
        if not isinstance(evidence, list) or not evidence or any(
            not isinstance(item, dict)
            or not nonempty_string(item.get("location"))
            or not nonempty_string(item.get("detail"))
            for item in evidence
        ):
            errors.append(prefix + "evidence must contain location and detail")
        if not string_list(record.get("requirement_ids"), allow_empty=False):
            errors.append(prefix + "requirement_ids must be a non-empty string list")
        criterion = record.get("criterion_source")
        if criterion is not None and not nonempty_string(criterion):
            errors.append(prefix + "criterion_source must be null or non-empty")
        if not string_list(record.get("reproduction"), allow_empty=False):
            errors.append(prefix + "reproduction must contain a checkable command or case")
        if record.get("confidence") not in CONFIDENCE:
            errors.append(prefix + "invalid confidence")
        if record.get("status") != "unverified":
            errors.append(prefix + "status must be unverified")
    return records, errors


def validate_findings(path: Path) -> list[str]:
    meta_path = path.with_name("audit.meta.json")
    meta = read_json(meta_path, []) if meta_path.is_file() else None
    _, errors = validate_finding_records(path, meta)
    return errors


def validate_audit_set(directory: Path) -> list[str]:
    errors: list[str] = []
    if not directory.is_dir():
        return [f"audit set directory does not exist: {directory}"]
    names = {path.name for path in directory.iterdir() if path.is_file()}
    missing = sorted(AUDIT_FILES - names)
    if missing:
        errors.append("active audit set missing: " + ", ".join(missing))
    unknown = sorted(names - ALLOWED_ASSURANCE_FILES)
    if unknown:
        errors.append("assurance directory contains unexpected files: " + ", ".join(unknown))
    if missing:
        return errors

    meta_path = directory / "audit.meta.json"
    requirements_path = directory / "audit.requirements.jsonl"
    findings_path = directory / "audit.findings.jsonl"
    report_path = directory / "audit.md"
    meta = read_json(meta_path, errors)
    errors.extend(validate_meta(meta))
    requirements, requirement_errors = validate_requirement_records(
        requirements_path,
        meta,
    )
    findings, finding_errors = validate_finding_records(findings_path, meta)
    errors.extend(requirement_errors)
    errors.extend(finding_errors)

    requirements_digest = raw_sha256(requirements_path, errors)
    findings_digest = raw_sha256(findings_path, errors)
    if requirements_digest is not None and meta.get("requirements_sha256") != requirements_digest:
        errors.append("requirements_sha256 does not match audit.requirements.jsonl")
    if findings_digest is not None and meta.get("findings_sha256") != findings_digest:
        errors.append("findings_sha256 does not match audit.findings.jsonl")

    requirements_by_id = {
        record["id"]: record for record in requirements if nonempty_string(record.get("id"))
    }
    findings_by_id = {
        record["id"]: record for record in findings if nonempty_string(record.get("id"))
    }
    for finding in findings:
        finding_id = finding.get("id")
        for requirement_id in finding.get("requirement_ids", []):
            requirement = requirements_by_id.get(requirement_id)
            if requirement is None:
                errors.append(f"finding {finding_id}: unknown requirement {requirement_id}")
                continue
            if finding_id not in requirement.get("finding_ids", []):
                errors.append(
                    f"finding {finding_id}: requirement {requirement_id} lacks reciprocal finding_id"
                )
            if finding.get("conformance_effect") == "blocks" and requirement.get("state") == "met":
                errors.append(
                    f"finding {finding_id}: blocking finding links met requirement {requirement_id}"
                )
    for requirement in requirements:
        requirement_id = requirement.get("id")
        finding_ids = requirement.get("finding_ids", [])
        for finding_id in finding_ids:
            finding = findings_by_id.get(finding_id)
            if finding is None:
                errors.append(f"requirement {requirement_id}: unknown finding {finding_id}")
            elif requirement_id not in finding.get("requirement_ids", []):
                errors.append(
                    f"requirement {requirement_id}: finding {finding_id} lacks reciprocal requirement_id"
                )
        if (
            requirement.get("mandatory") is True
            and requirement.get("state") != "met"
            and not finding_ids
        ):
            errors.append(
                f"mandatory requirement {requirement_id} is not met but has no finding"
            )

    mandatory_failed = any(
        record.get("mandatory") is True and record.get("state") != "met"
        for record in requirements
    )
    actionable = any(record.get("severity") != "Info" for record in findings)
    expected_verdict = (
        "FAIL"
        if mandatory_failed
        else "PASS-WITH-FINDINGS"
        if actionable
        else "PASS"
    )
    if meta.get("verdict") != expected_verdict:
        errors.append(f"verdict must be {expected_verdict}")

    try:
        report = report_path.read_text(encoding="utf-8-sig")
    except OSError as exc:
        errors.append(f"cannot read {report_path}: {exc}")
    else:
        report_values = {
            "Audit run ID": meta.get("audit_run_id"),
            "Audit head": meta.get("audit_head"),
            "Verdict": meta.get("verdict"),
        }
        for label, expected in report_values.items():
            actual = unquote_code(header_value(report, label))
            if actual != expected:
                errors.append(f"audit.md {label} does not match audit.meta.json")
    return errors


def source_key(record: dict[str, Any]) -> tuple[str, str] | None:
    source = record.get("source")
    if not isinstance(source, dict):
        return None
    audit_run_id = source.get("audit_run_id")
    finding_id = source.get("id")
    if not nonempty_string(audit_run_id) or not nonempty_string(finding_id):
        return None
    return (audit_run_id, finding_id)


def validate_finding_disposition(
    record: dict[str, Any],
    *,
    stage: str,
    meta: dict[str, Any] | None,
    prefix: str,
) -> list[str]:
    errors: list[str] = []
    missing = missing_fields(record, DISPOSITION_FIELDS)
    if missing:
        errors.append(prefix + "missing fields: " + ", ".join(missing))
        return errors
    if HISTORICAL_FIELDS & record.keys():
        errors.append(prefix + "historical lineage fields are forbidden")
    if record.get("record_kind") != "finding-disposition":
        errors.append(prefix + "record_kind must be finding-disposition")
    source = record.get("source")
    if not isinstance(source, dict):
        errors.append(prefix + "source must be an object")
    else:
        if HISTORICAL_FIELDS & source.keys():
            errors.append(prefix + "historical lineage fields are forbidden")
        if not nonempty_string(source.get("audit_run_id")) or not nonempty_string(
            source.get("id")
        ):
            errors.append(prefix + "source requires audit_run_id and id")
        digest = source.get("findings_sha256")
        if not isinstance(digest, str) or SHA256_RE.fullmatch(digest) is None:
            errors.append(prefix + "source findings_sha256 must be lowercase SHA-256")
        if meta is not None:
            if source.get("audit_run_id") != meta.get("audit_run_id"):
                errors.append(prefix + "source audit_run_id does not match current audit")
            if source.get("findings_sha256") != meta.get("findings_sha256"):
                errors.append(prefix + "source findings_sha256 does not match current audit")
    verified_at = record.get("verified_at")
    if not isinstance(verified_at, str) or FULL_SHA_RE.fullmatch(verified_at) is None:
        errors.append(prefix + "verified_at must be a full lowercase commit SHA")
    if record.get("verification_status") not in VERIFICATION:
        errors.append(prefix + "invalid verification_status")
    if record.get("final_severity") not in SEVERITIES:
        errors.append(prefix + "invalid final_severity")
    for field in ("final_severity_rationale", "closure_key", "family_key", "decision"):
        if not nonempty_string(record.get(field)):
            errors.append(prefix + f"{field} must be non-empty")
    closure_batch = record.get("closure_batch")
    if closure_batch is not None and not nonempty_string(closure_batch):
        errors.append(prefix + "closure_batch must be null or non-empty")
    if record.get("change_kind") not in CHANGE_KINDS:
        errors.append(prefix + "invalid change_kind")
    if not string_list(record.get("changed_paths")):
        errors.append(prefix + "changed_paths must be a list of non-empty paths")
    if record.get("change_kind") == "behavioral" and "red_before" not in record:
        errors.append(prefix + "behavioral disposition requires red_before")
    if record.get("change_kind") != "behavioral" and not (
        "red_before" in record or "red_before_not_applicable" in record
    ):
        errors.append(prefix + "non-behavioral disposition must explain red-before")
    red_before = record.get("red_before")
    if red_before is not None and not check_record(red_before):
        errors.append(prefix + "red_before must contain command, expected, observed")
    red_na = record.get("red_before_not_applicable")
    if red_na is not None and not nonempty_string(red_na):
        errors.append(prefix + "red_before_not_applicable must be non-empty")
    green_after = record.get("green_after")
    if not check_record(green_after):
        errors.append(prefix + "green_after must contain command, expected, observed")
    elif stage == "plan" and green_after.get("observed") != "not-run":
        errors.append(prefix + "plan disposition green_after observed must be not-run")
    elif stage == "result" and green_after.get("observed") == "not-run":
        errors.append(prefix + "result disposition requires observed green_after")
    if stage == "result" and record.get("remediation_status") not in REMEDIATION_STATUSES:
        errors.append(prefix + "result disposition requires remediation_status")
    if stage == "plan" and "remediation_status" in record:
        errors.append(prefix + "plan disposition cannot claim remediation_status")
    return errors


def protected_incidental_path(path: str, baseline_paths: set[str]) -> bool:
    normalized = path.replace("\\", "/")
    pure = PurePosixPath(normalized)
    if pure.name in {"Cargo.toml", "Cargo.lock", ".opi-impl-state.json"}:
        return True
    if normalized in {"docs/opi-spec.md", "docs/CONTEXT.md"}:
        return True
    if normalized in baseline_paths and normalized.endswith((".md", ".json")):
        return True
    return "/schemas/" in f"/{normalized}" or normalized.endswith(".schema.json")


def validate_incidental(
    record: dict[str, Any],
    *,
    meta: dict[str, Any],
    prefix: str,
) -> list[str]:
    errors: list[str] = []
    missing = missing_fields(record, INCIDENTAL_FIELDS)
    if missing:
        return [prefix + "missing fields: " + ", ".join(missing)]
    if record.get("record_kind") != "incidental-repair":
        errors.append(prefix + "record_kind must be incidental-repair")
    if not isinstance(record.get("id"), str) or INCIDENTAL_ID_RE.fullmatch(
        record["id"]
    ) is None:
        errors.append(prefix + "id must use I<N>")
    for field in ("trigger_batch", "blocking_check", "scope_rationale"):
        if not nonempty_string(record.get(field)):
            errors.append(prefix + f"{field} must be non-empty")
    guardrails = record.get("guardrails")
    if not isinstance(guardrails, dict) or not GUARDRAIL_FIELDS <= guardrails.keys():
        errors.append(prefix + "incidental repair guardrails are incomplete")
    else:
        required = (
            guardrails.get("required_for_green") is True
            and guardrails.get("within_causal_surface") is True
        )
        protected = any(
            guardrails.get(field) is not False
            for field in (
                "changes_public_api",
                "changes_durable_format",
                "changes_dependency_graph",
                "changes_spec_or_authority",
            )
        )
        if not required:
            errors.append(prefix + "incidental repair is not required and causally bounded")
        if protected:
            errors.append(prefix + "incidental repair changes a protected contract")
    changed_paths = record.get("changed_paths")
    if not string_list(changed_paths, allow_empty=False):
        errors.append(prefix + "incidental repair changed_paths must be non-empty")
    else:
        baseline_paths = {
            source.get("path")
            for source in meta.get("baseline_sources", [])
            if isinstance(source, dict) and isinstance(source.get("path"), str)
        }
        for path in changed_paths:
            if protected_incidental_path(path, baseline_paths):
                errors.append(prefix + f"incidental repair names protected path: {path}")
    red = record.get("red_before")
    green = record.get("green_after")
    if not (
        check_record(red)
        and str(red.get("observed", "")).startswith("FAIL")
        and check_record(green)
        and str(green.get("observed", "")).startswith("PASS")
    ):
        errors.append(prefix + "incidental repair requires observed FAIL and PASS")
    if record.get("remediation_status") != "Closed":
        errors.append(prefix + "incidental repair remediation_status must be Closed")
    return errors


def disposition_stage(path: Path) -> str | None:
    if path.name == "remediation.plan.dispositions.jsonl":
        return "plan"
    if path.name == "remediation.result.dispositions.jsonl":
        return "result"
    return None


def validate_disposition_records(
    path: Path,
    *,
    meta: dict[str, Any] | None,
) -> tuple[list[dict[str, Any]], list[str]]:
    errors: list[str] = []
    stage = disposition_stage(path)
    if stage is None:
        errors.append(
            "expected fixed filename remediation.plan.dispositions.jsonl or "
            "remediation.result.dispositions.jsonl"
        )
        stage = "plan"
    expected_report = f"remediation.{stage}.md"
    if not path.with_name(expected_report).is_file():
        errors.append(f"missing fixed report sibling: {expected_report}")
    records = load_jsonl(path, errors)
    seen_sources: set[tuple[str, str]] = set()
    seen_incidentals: set[str] = set()
    batch_keys: dict[str, str] = {}
    for number, record in enumerate(records, start=1):
        prefix = f"record {number}: "
        kind = record.get("record_kind")
        if kind == "incidental-repair":
            if stage != "result":
                errors.append(prefix + "incidental repairs are allowed only in results")
                continue
            if meta is None:
                errors.append(prefix + "incidental repair requires current audit metadata")
                continue
            errors.extend(validate_incidental(record, meta=meta, prefix=prefix))
            incidental_id = record.get("id")
            if isinstance(incidental_id, str):
                if incidental_id in seen_incidentals:
                    errors.append(prefix + "duplicate incidental repair id")
                seen_incidentals.add(incidental_id)
            continue
        errors.extend(
            validate_finding_disposition(
                record,
                stage=stage,
                meta=meta,
                prefix=prefix,
            )
        )
        key = source_key(record)
        if key is not None:
            if key in seen_sources:
                errors.append(prefix + "duplicate source disposition")
            seen_sources.add(key)
        batch = record.get("closure_batch")
        closure_key = record.get("closure_key")
        if nonempty_string(batch) and nonempty_string(closure_key):
            previous = batch_keys.setdefault(batch, closure_key)
            if previous != closure_key:
                errors.append(f"closure batch {batch} has multiple closure keys")
    return records, errors


def validate_dispositions(path: Path) -> list[str]:
    meta_path = path.with_name("audit.meta.json")
    meta = read_json(meta_path, []) if meta_path.is_file() else None
    _, errors = validate_disposition_records(path, meta=meta)
    return errors


def plan_digest(plan: Path, dispositions: Path) -> str:
    digest = hashlib.sha256()
    digest.update(plan.read_bytes())
    digest.update(b"\0")
    digest.update(dispositions.read_bytes())
    return digest.hexdigest()


def read_text(path: Path, errors: list[str]) -> str:
    try:
        return path.read_text(encoding="utf-8-sig")
    except OSError as exc:
        errors.append(f"cannot read {path}: {exc}")
        return ""


def validate_plan(path: Path) -> list[str]:
    errors: list[str] = []
    if path.name != "remediation.plan.md":
        errors.append("expected fixed filename remediation.plan.md")
    directory = path.parent
    meta_path = directory / "audit.meta.json"
    findings_path = directory / "audit.findings.jsonl"
    disposition_path = directory / "remediation.plan.dispositions.jsonl"
    meta = read_json(meta_path, errors)
    if meta:
        errors.extend(validate_meta(meta))
    text = read_text(path, errors)
    status = header_value(text, "Status")
    if status not in PLAN_STATUSES:
        errors.append("Status must be DRAFT-UNRESOLVED or READY-FOR-APPLY")
    header_expectations = {
        "Audit run ID": meta.get("audit_run_id"),
        "Findings SHA-256": meta.get("findings_sha256"),
    }
    for label, expected in header_expectations.items():
        if unquote_code(header_value(text, label)) != expected:
            errors.append(f"{label} does not match current audit")
    remediation_head = unquote_code(header_value(text, "Remediation head"))
    if not isinstance(remediation_head, str) or FULL_SHA_RE.fullmatch(remediation_head) is None:
        errors.append("Remediation head must contain a full committed SHA")
    if header_value(text, "Disposition artifact") != "`remediation.plan.dispositions.jsonl`":
        errors.append("Disposition artifact must name the fixed plan sibling")
    unresolved = header_value(text, "Unresolved decisions")
    if unresolved is None:
        errors.append("missing Unresolved decisions header")
    elif status == "READY-FOR-APPLY" and unresolved.lower() != "none":
        errors.append("READY-FOR-APPLY requires no unresolved decisions")
    records, disposition_errors = validate_disposition_records(
        disposition_path,
        meta=meta,
    )
    errors.extend(f"plan disposition: {error}" for error in disposition_errors)

    finding_errors: list[str] = []
    findings = load_jsonl(findings_path, finding_errors, allow_empty=True)
    errors.extend(f"current findings: {error}" for error in finding_errors)
    finding_ids = {
        record.get("id") for record in findings if nonempty_string(record.get("id"))
    }
    disposition_ids = {
        key[1]
        for record in records
        if record.get("record_kind") == "finding-disposition"
        and (key := source_key(record)) is not None
    }
    missing_sources = sorted(finding_ids - disposition_ids)
    extra_sources = sorted(disposition_ids - finding_ids)
    for finding_id in missing_sources:
        errors.append(f"plan is missing current finding {finding_id}")
    for finding_id in extra_sources:
        errors.append(f"plan references non-current finding {finding_id}")

    fix_starts = list(re.finditer(r"(?m)^#### Fix\s+[^\n]+$", text))
    if status == "READY-FOR-APPLY" and findings and not fix_starts:
        errors.append("plan contains no Fix items")
    for index, start in enumerate(fix_starts):
        end = fix_starts[index + 1].start() if index + 1 < len(fix_starts) else len(text)
        block = text[start.start() : end]
        title = start.group(0)
        for field in ("Closure predicate", "Red-before", "Green-after"):
            match = re.search(
                rf"(?m)^- \*\*{re.escape(field)}\*\*:[ \t]*(.*?)[ \t]*$",
                block,
            )
            if match is None:
                errors.append(f"{title}: missing {field}")
            elif not match.group(1):
                errors.append(f"{title}: empty {field}")
    if status == "READY-FOR-APPLY":
        for record in records:
            if record.get("record_kind") != "finding-disposition":
                continue
            key = source_key(record)
            finding_id = key[1] if key is not None else "<invalid>"
            if key is not None and finding_id not in text:
                errors.append(f"READY-FOR-APPLY missing source disposition {finding_id}")
            decision = record.get("decision")
            if nonempty_string(decision) and decision.startswith("pending:"):
                errors.append(f"READY-FOR-APPLY has pending decision for {finding_id}")
            if record.get("closure_batch") is None and not str(decision).startswith(
                "no-action:"
            ):
                errors.append(f"READY-FOR-APPLY missing closure batch for {finding_id}")
            if record.get("change_kind") == "behavioral":
                red = record.get("red_before")
                if not check_record(red) or not str(red.get("observed", "")).startswith(
                    "FAIL"
                ):
                    errors.append(
                        f"READY-FOR-APPLY requires observed FAIL red-before for {finding_id}"
                    )
            green = record.get("green_after")
            if not check_record(green) or not nonempty_string(green.get("command")):
                errors.append(
                    f"READY-FOR-APPLY requires concrete green-after for {finding_id}"
                )
    return errors


def stable_disposition_drift(
    plan_record: dict[str, Any],
    result_record: dict[str, Any],
) -> bool:
    stable_fields = (
        "record_kind",
        "source",
        "verified_at",
        "verification_status",
        "final_severity",
        "final_severity_rationale",
        "closure_key",
        "family_key",
        "decision",
        "closure_batch",
        "change_kind",
        "changed_paths",
        "red_before",
        "red_before_not_applicable",
    )
    if any(plan_record.get(field) != result_record.get(field) for field in stable_fields):
        return True
    plan_green = plan_record.get("green_after")
    result_green = result_record.get("green_after")
    return not (
        check_record(plan_green)
        and check_record(result_green)
        and all(
            plan_green.get(field) == result_green.get(field)
            for field in ("command", "expected")
        )
    )


def validate_result(path: Path) -> list[str]:
    errors: list[str] = []
    if path.name != "remediation.result.md":
        errors.append("expected fixed filename remediation.result.md")
    directory = path.parent
    meta = read_json(directory / "audit.meta.json", errors)
    if meta:
        errors.extend(validate_meta(meta))
    plan_path = directory / "remediation.plan.md"
    plan_dispositions_path = directory / "remediation.plan.dispositions.jsonl"
    result_dispositions_path = directory / "remediation.result.dispositions.jsonl"
    plan_errors = validate_plan(plan_path)
    errors.extend(f"plan: {error}" for error in plan_errors)
    plan_records, plan_record_errors = validate_disposition_records(
        plan_dispositions_path,
        meta=meta,
    )
    errors.extend(f"plan disposition: {error}" for error in plan_record_errors)
    result_records, result_record_errors = validate_disposition_records(
        result_dispositions_path,
        meta=meta,
    )
    errors.extend(f"result disposition: {error}" for error in result_record_errors)
    text = read_text(path, errors)
    if header_value(text, "Status") != "COMPLETE":
        errors.append("result Status must be COMPLETE")
    for label, expected in (
        ("Audit run ID", meta.get("audit_run_id")),
        ("Findings SHA-256", meta.get("findings_sha256")),
    ):
        if unquote_code(header_value(text, label)) != expected:
            errors.append(f"result {label} does not match current audit")
    try:
        expected_plan_digest = plan_digest(plan_path, plan_dispositions_path)
    except OSError as exc:
        errors.append(f"cannot compute plan digest: {exc}")
        expected_plan_digest = None
    if unquote_code(header_value(text, "Plan SHA-256")) != expected_plan_digest:
        errors.append("result Plan SHA-256 does not match current plan")

    plan_by_source = {
        key: record
        for record in plan_records
        if record.get("record_kind") == "finding-disposition"
        and (key := source_key(record)) is not None
    }
    result_by_source = {
        key: record
        for record in result_records
        if record.get("record_kind") == "finding-disposition"
        and (key := source_key(record)) is not None
    }
    if set(plan_by_source) != set(result_by_source):
        errors.append("result dispositions must cover exactly the plan sources")
    for key in sorted(set(plan_by_source) & set(result_by_source)):
        result_record = result_by_source[key]
        if stable_disposition_drift(plan_by_source[key], result_record):
            errors.append(f"result {key[1]} drifts from plan-stage disposition")
        if (
            result_record.get("change_kind") == "behavioral"
            and result_record.get("remediation_status") == "Closed"
        ):
            red = result_record.get("red_before")
            green = result_record.get("green_after")
            if not check_record(red) or not str(red.get("observed", "")).startswith(
                "FAIL"
            ):
                errors.append(f"result {key[1]} Closed requires observed FAIL red-before")
            if not check_record(green) or not str(green.get("observed", "")).startswith(
                "PASS"
            ):
                errors.append(f"result {key[1]} Closed requires observed PASS green-after")

    changed_header = header_value(text, "Changed paths")
    try:
        reported_paths = json.loads(changed_header or "")
    except json.JSONDecodeError:
        reported_paths = None
    if not string_list(reported_paths):
        errors.append("Changed paths must be a JSON array of repo-relative paths")
        reported_set: set[str] = set()
    else:
        reported_set = set(reported_paths)
    attributed_set = {
        changed_path
        for record in result_records
        for changed_path in record.get("changed_paths", [])
        if isinstance(changed_path, str)
    }
    for changed_path in sorted(reported_set - attributed_set):
        errors.append(f"unattributed changed path: {changed_path}")
    for changed_path in sorted(attributed_set - reported_set):
        errors.append(f"recorded changed path missing from result report: {changed_path}")
    return errors


def run_git(path: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(path), *args],
        capture_output=True,
        text=True,
        check=False,
    )


def legacy_name(path: str, phase_relative: str) -> bool:
    normalized = path.replace("\\", "/")
    prefix = phase_relative.rstrip("/") + "/"
    if not normalized.startswith(prefix):
        return False
    remainder = normalized[len(prefix) :]
    if remainder.startswith("assurance/") or "/" in remainder:
        return False
    return remainder.startswith("audit") or remainder.startswith("remediation")


def validate_rotation(phase_directory: Path) -> list[str]:
    errors: list[str] = []
    phase_match = PHASE_RE.fullmatch(phase_directory.name)
    if phase_match is None:
        errors.append("rotation path must be a phase<N> directory")
    root_result = run_git(phase_directory, "rev-parse", "--show-toplevel")
    if root_result.returncode != 0:
        return errors + ["rotation path is not inside a Git repository"]
    repo_root = Path(root_result.stdout.strip()).resolve()
    try:
        phase_relative = phase_directory.resolve().relative_to(repo_root).as_posix()
    except ValueError:
        return errors + ["rotation path is outside the Git repository"]

    legacy_paths: set[str] = set()
    head = run_git(repo_root, "ls-tree", "-r", "--name-only", "HEAD", "--", phase_relative)
    if head.returncode == 0:
        legacy_paths.update(
            path
            for path in head.stdout.splitlines()
            if legacy_name(path, phase_relative)
        )
    index = run_git(repo_root, "ls-files", "--cached", "--", phase_relative)
    if index.returncode == 0:
        legacy_paths.update(
            path
            for path in index.stdout.splitlines()
            if legacy_name(path, phase_relative)
        )
    if phase_directory.is_dir():
        for path in phase_directory.iterdir():
            if path.is_file() and (
                path.name.startswith("audit") or path.name.startswith("remediation")
            ):
                legacy_paths.add(f"{phase_relative}/{path.name}")
    if legacy_paths:
        errors.append("legacy assurance artifact blocks rotation: " + ", ".join(sorted(legacy_paths)))

    assurance_relative = f"{phase_relative}/assurance"
    status = run_git(
        repo_root,
        "status",
        "--porcelain=v1",
        "--untracked-files=all",
        "--",
        assurance_relative,
    )
    if status.returncode != 0:
        errors.append("cannot inspect active assurance Git state")
    elif status.stdout.strip():
        errors.append("active assurance set is not clean")

    assurance_dir = phase_directory / "assurance"
    if assurance_dir.is_dir():
        names = {path.name for path in assurance_dir.iterdir() if path.is_file()}
        unknown = sorted(names - ALLOWED_ASSURANCE_FILES)
        if unknown:
            errors.append("active assurance set has unexpected files: " + ", ".join(unknown))
        present_audit = names & AUDIT_FILES
        if present_audit and present_audit != AUDIT_FILES:
            errors.append("active assurance set has an incomplete audit file group")
        present_remediation = names & REMEDIATION_FILES
        if present_remediation and present_remediation != REMEDIATION_FILES:
            errors.append("active assurance set has an incomplete remediation file group")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "kind",
        choices=(
            "audit-set",
            "findings",
            "requirements",
            "dispositions",
            "plan",
            "result",
            "rotation",
        ),
    )
    parser.add_argument("path", type=Path)
    args = parser.parse_args()
    validators = {
        "audit-set": validate_audit_set,
        "findings": validate_findings,
        "requirements": validate_requirements,
        "dispositions": validate_dispositions,
        "plan": validate_plan,
        "result": validate_result,
        "rotation": validate_rotation,
    }
    errors = validators[args.kind](args.path)
    if errors:
        for error in errors:
            print(f"FAIL: {error}", file=sys.stderr)
        return 1
    if args.kind == "plan":
        dispositions = args.path.with_name("remediation.plan.dispositions.jsonl")
        print(f"plan: PASS plan_sha256={plan_digest(args.path, dispositions)}")
    else:
        print(f"{args.kind}: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
