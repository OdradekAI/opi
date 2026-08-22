#!/usr/bin/env python3
"""Validate immutable Opi audit and remediation assurance artifacts."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any


SHA_RE = re.compile(r"^[0-9a-f]{40}$")
FINDINGS_NAME_RE = re.compile(
    r"^audit\.[a-z0-9][a-z0-9.-]*\.([0-9a-f]{7,40})\."
    r"[a-z0-9][a-z0-9.-]*\.findings\.jsonl$"
)
DISPOSITIONS_NAME_RE = re.compile(
    r"^remediation\.([0-9a-f]{7,40})\."
    r"[a-z0-9][a-z0-9.-]*\.(plan|result)\.dispositions\.jsonl$"
)
PLAN_NAME_RE = re.compile(
    r"^remediation\.([0-9a-f]{7,40})\."
    r"[a-z0-9][a-z0-9.-]*\.plan\.md$"
)

FINDING_FIELDS = {
    "id",
    "source_kind",
    "source_path",
    "source_model",
    "observed_at",
    "independence",
    "axis",
    "severity",
    "title",
    "claim",
    "evidence",
    "criterion_source",
    "reproduction",
    "confidence",
    "status",
}
DISPOSITION_FIELDS = {
    "source",
    "verified_at",
    "verification_status",
    "final_severity",
    "final_severity_rationale",
    "closure_key",
    "family_key",
    "lineage",
    "decision",
    "closure_batch",
    "change_kind",
    "green_after",
}

SOURCE_KINDS = {"audit", "eval"}
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
CONFIDENCE = {"high", "medium", "low"}
VERIFICATION = {"Confirmed", "Partially confirmed", "Cannot confirm", "Refuted"}
LINEAGE_KINDS = {
    "new",
    "recurrent-same-defect",
    "recurrent-adjacent-path",
    "regression",
    "carried-forward-deferred",
}
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


def missing_fields(record: dict[str, Any], fields: set[str]) -> list[str]:
    return sorted(field for field in fields if field not in record)


def nonempty_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def source_key(record: dict[str, Any]) -> tuple[str, str] | None:
    source = record.get("source")
    if not isinstance(source, dict):
        return None
    source_path = source.get("source_path")
    finding_id = source.get("id")
    if not nonempty_string(source_path) or not nonempty_string(finding_id):
        return None
    return (source_path, finding_id)


def check_record(value: Any) -> bool:
    return (
        isinstance(value, dict)
        and all(key in value for key in ("command", "expected", "observed"))
        and (value["command"] is None or nonempty_string(value["command"]))
        and nonempty_string(value["expected"])
        and nonempty_string(value["observed"])
    )


def require_sha(value: Any, field: str, errors: list[str], prefix: str) -> None:
    if not isinstance(value, str) or SHA_RE.fullmatch(value) is None:
        errors.append(f"{prefix}{field} must be a full lowercase commit SHA")


def validate_findings(path: Path) -> list[str]:
    errors: list[str] = []
    match = FINDINGS_NAME_RE.fullmatch(path.name)
    if match is None:
        errors.append(
            "expected immutable audit findings filename "
            "audit.<model>.<head7+>.<run-id>.findings.jsonl"
        )
    head_prefix = match.group(1) if match is not None else ""
    expected_report = path.name.removesuffix(".findings.jsonl") + ".md"
    if not path.with_name(expected_report).is_file():
        errors.append(f"missing immutable report sibling: {expected_report}")
    seen: set[tuple[str, str]] = set()
    for number, record in enumerate(
        load_jsonl(path, errors, allow_empty=True),
        start=1,
    ):
        prefix = f"record {number}: "
        missing = missing_fields(record, FINDING_FIELDS)
        if missing:
            errors.append(prefix + "missing fields: " + ", ".join(missing))
            continue
        require_sha(record["observed_at"], "observed_at", errors, prefix)
        observed_at = record.get("observed_at")
        if isinstance(observed_at, str) and head_prefix and not observed_at.startswith(head_prefix):
            errors.append(prefix + "observed_at does not match filename HEAD")
        if record.get("source_kind") not in SOURCE_KINDS:
            errors.append(prefix + "invalid source_kind")
        elif record.get("source_kind") != "audit":
            errors.append(prefix + "audit findings source_kind must be audit")
        if record.get("independence") not in INDEPENDENCE:
            errors.append(prefix + "invalid independence")
        if record.get("axis") not in AXES:
            errors.append(prefix + "invalid axis")
        if record.get("severity") not in SEVERITIES:
            errors.append(prefix + "invalid severity")
        if record.get("confidence") not in CONFIDENCE:
            errors.append(prefix + "invalid confidence")
        if record.get("status") != "unverified":
            errors.append(prefix + "status must be unverified")
        for field in ("id", "source_model", "title", "claim"):
            if not nonempty_string(record.get(field)):
                errors.append(prefix + f"{field} must be non-empty")
        source_path = record.get("source_path")
        expected_source = (
            rf"docs/snapshots/(phase[0-9]+)/{re.escape(expected_report)}"
        )
        source_match = (
            re.fullmatch(expected_source, source_path)
            if isinstance(source_path, str)
            else None
        )
        if source_match is None:
            errors.append(
                prefix + "source_path must name the repo-relative Phase snapshot report"
            )
        elif re.fullmatch(r"phase[0-9]+", path.parent.name) and (
            source_match.group(1) != path.parent.name
        ):
            errors.append(prefix + "source_path Phase must match artifact directory")
        evidence = record.get("evidence")
        if not isinstance(evidence, list) or not evidence:
            errors.append(prefix + "evidence must be a non-empty list")
        elif any(
            not isinstance(item, dict)
            or not nonempty_string(item.get("location"))
            or not nonempty_string(item.get("detail"))
            for item in evidence
        ):
            errors.append(
                prefix + "evidence entries require non-empty location and detail"
            )
        reproduction = record.get("reproduction")
        if not isinstance(reproduction, list) or not reproduction or any(
            not nonempty_string(item) for item in reproduction
        ):
            errors.append(prefix + "reproduction must contain a checkable command or case")
        criterion_source = record.get("criterion_source")
        if criterion_source is not None and not nonempty_string(criterion_source):
            errors.append(prefix + "criterion_source must be null or non-empty")
        key = (str(source_path), str(record.get("id")))
        if key in seen:
            errors.append(prefix + "duplicate (source_path, id)")
        seen.add(key)
    return errors


def validate_dispositions(path: Path) -> list[str]:
    errors: list[str] = []
    match = DISPOSITIONS_NAME_RE.fullmatch(path.name)
    if match is None:
        errors.append(
            "expected immutable remediation dispositions filename "
            "remediation.<head7+>.<round>.<plan|result>.dispositions.jsonl"
        )
    head_prefix = match.group(1) if match is not None else ""
    stage = match.group(2) if match is not None else ""
    expected_report = path.name.removesuffix(".dispositions.jsonl") + ".md"
    if not path.with_name(expected_report).is_file():
        errors.append(f"missing immutable report sibling: {expected_report}")
    seen: set[tuple[str, str]] = set()
    batch_keys: dict[str, str] = {}
    records = load_jsonl(path, errors)
    for number, record in enumerate(records, start=1):
        prefix = f"record {number}: "
        missing = missing_fields(record, DISPOSITION_FIELDS)
        if missing:
            errors.append(prefix + "missing fields: " + ", ".join(missing))
            continue
        require_sha(record["verified_at"], "verified_at", errors, prefix)
        verified_at = record.get("verified_at")
        if isinstance(verified_at, str) and head_prefix and not verified_at.startswith(head_prefix):
            errors.append(prefix + "verified_at does not match filename HEAD")
        if record.get("verification_status") not in VERIFICATION:
            errors.append(prefix + "invalid verification_status")
        if record.get("final_severity") not in SEVERITIES:
            errors.append(prefix + "invalid final_severity")
        if not nonempty_string(record.get("final_severity_rationale")):
            errors.append(prefix + "final_severity_rationale must be non-empty")
        if record.get("change_kind") not in CHANGE_KINDS:
            errors.append(prefix + "invalid change_kind")
        key = source_key(record)
        if key is None:
            errors.append(prefix + "source must contain non-empty source_path and id")
        else:
            if key in seen:
                errors.append(prefix + "duplicate source disposition")
            seen.add(key)
        for field in ("closure_key", "family_key", "decision"):
            if not nonempty_string(record.get(field)):
                errors.append(prefix + f"{field} must be non-empty")
        closure_batch = record.get("closure_batch")
        if closure_batch is not None and not nonempty_string(closure_batch):
            errors.append(prefix + "closure_batch must be null or non-empty")
        if nonempty_string(closure_batch) and nonempty_string(record.get("closure_key")):
            prior_key = batch_keys.setdefault(closure_batch, record["closure_key"])
            if prior_key != record["closure_key"]:
                errors.append(
                    f"closure batch {closure_batch} has multiple closure keys"
                )
        lineage = record.get("lineage")
        if not isinstance(lineage, dict) or not {
            "kind",
            "prior_occurrences",
            "prior_disposition",
        } <= lineage.keys():
            errors.append(prefix + "lineage is incomplete")
        else:
            if lineage.get("kind") not in LINEAGE_KINDS:
                errors.append(prefix + "invalid lineage kind")
            if not isinstance(lineage.get("prior_occurrences"), list):
                errors.append(prefix + "lineage prior_occurrences must be a list")
            if not nonempty_string(lineage.get("prior_disposition")):
                errors.append(prefix + "lineage prior_disposition must be non-empty")
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
    if stage == "result":
        plan_path = path.with_name(
            path.name.replace(
                ".result.dispositions.jsonl",
                ".plan.dispositions.jsonl",
            )
        )
        if not plan_path.is_file():
            errors.append(f"missing plan-stage disposition sibling: {plan_path.name}")
        else:
            plan_errors = validate_dispositions(plan_path)
            errors.extend(f"plan-stage: {error}" for error in plan_errors)
            plan_records = {
                key: record
                for record in load_jsonl(plan_path, errors)
                if (key := source_key(record)) is not None
            }
            result_records = {
                key: record
                for record in records
                if (key := source_key(record)) is not None
            }
            if set(plan_records) != set(result_records):
                errors.append("result dispositions must cover exactly the plan sources")
            stable_fields = (
                "verified_at",
                "verification_status",
                "final_severity",
                "final_severity_rationale",
                "closure_key",
                "family_key",
                "lineage",
                "decision",
                "closure_batch",
                "change_kind",
                "red_before",
                "red_before_not_applicable",
            )
            for key in sorted(set(plan_records) & set(result_records)):
                plan_record = plan_records[key]
                result_record = result_records[key]
                drifted = any(
                    plan_record.get(field) != result_record.get(field)
                    for field in stable_fields
                )
                plan_green = plan_record.get("green_after")
                result_green = result_record.get("green_after")
                if check_record(plan_green) and check_record(result_green):
                    drifted = drifted or any(
                        plan_green.get(field) != result_green.get(field)
                        for field in ("command", "expected")
                    )
                if drifted:
                    errors.append(
                        f"result {key[1]} drifts from plan-stage disposition"
                    )
                if (
                    result_record.get("change_kind") == "behavioral"
                    and result_record.get("remediation_status") == "Closed"
                ):
                    red = result_record.get("red_before")
                    green = result_record.get("green_after")
                    if not check_record(red) or not str(red.get("observed", "")).startswith("FAIL"):
                        errors.append(f"result {key[1]} Closed requires observed FAIL red-before")
                    if not check_record(green) or not str(green.get("observed", "")).startswith("PASS"):
                        errors.append(f"result {key[1]} Closed requires observed PASS green-after")
    return errors


def header_value(text: str, label: str) -> str | None:
    match = re.search(rf"(?m)^\*\*{re.escape(label)}\*\*:\s*(.+?)\s*$", text)
    return match.group(1).strip() if match else None


def validate_plan(path: Path) -> list[str]:
    errors: list[str] = []
    match = PLAN_NAME_RE.fullmatch(path.name)
    if match is None:
        errors.append(
            "expected immutable remediation plan filename "
            "remediation.<head7+>.<round>.plan.md"
        )
    head_prefix = match.group(1) if match is not None else ""
    try:
        text = path.read_text(encoding="utf-8-sig")
    except OSError as exc:
        return [f"cannot read {path}: {exc}"]
    status = header_value(text, "Status")
    if status not in PLAN_STATUSES:
        errors.append("Status must be DRAFT-UNRESOLVED or READY-FOR-APPLY")
    target = header_value(text, "Verification target")
    target_match = re.search(r"`([0-9a-f]{40})`", target or "")
    if target_match is None:
        errors.append("Verification target must contain a full committed SHA")
    elif head_prefix and not target_match.group(1).startswith(head_prefix):
        errors.append("Verification target does not match filename HEAD")
    expected_disposition = (
        path.name.removesuffix(".plan.md") + ".plan.dispositions.jsonl"
    )
    disposition = header_value(text, "Disposition artifact")
    if disposition != f"`{expected_disposition}`":
        errors.append("Disposition artifact must name the exact plan sibling")
    disposition_path = path.with_name(expected_disposition)
    if not disposition_path.is_file():
        errors.append(
            f"missing exact plan disposition sibling: {expected_disposition}"
        )
    unresolved = header_value(text, "Unresolved decisions")
    if unresolved is None:
        errors.append("missing Unresolved decisions header")
    elif status == "READY-FOR-APPLY" and unresolved.lower() != "none":
        errors.append("READY-FOR-APPLY requires no unresolved decisions")
    fix_starts = list(re.finditer(r"(?m)^#### Fix\s+[^\n]+$", text))
    if status == "READY-FOR-APPLY" and not fix_starts:
        errors.append("plan contains no Fix items")
    for index, start in enumerate(fix_starts):
        end = fix_starts[index + 1].start() if index + 1 < len(fix_starts) else len(text)
        block = text[start.start() : end]
        title = start.group(0)
        for field in ("Closure predicate", "Red-before", "Green-after"):
            field_match = re.search(
                rf"(?m)^- \*\*{re.escape(field)}\*\*:[ \t]*(.*?)[ \t]*$",
                block,
            )
            if field_match is None:
                errors.append(f"{title}: missing {field}")
            elif not field_match.group(1):
                errors.append(f"{title}: empty {field}")
    if disposition_path.is_file():
        disposition_errors = validate_dispositions(disposition_path)
        errors.extend(f"plan disposition: {error}" for error in disposition_errors)
        disposition_records = load_jsonl(disposition_path, errors)
        if status == "READY-FOR-APPLY":
            for record in disposition_records:
                key = source_key(record)
                finding_id = key[1] if key is not None else "<invalid>"
                if key is not None and (key[0] not in text or key[1] not in text):
                    errors.append(f"READY-FOR-APPLY missing source disposition {finding_id}")
                decision = record.get("decision")
                if nonempty_string(decision) and decision.startswith("pending:"):
                    errors.append(f"READY-FOR-APPLY has pending decision for {finding_id}")
                closure_batch = record.get("closure_batch")
                if closure_batch is None and not str(decision).startswith("no-action:"):
                    errors.append(f"READY-FOR-APPLY missing closure batch for {finding_id}")
                if record.get("change_kind") == "behavioral":
                    red_before = record.get("red_before")
                    if not check_record(red_before) or not str(
                        red_before.get("observed", "")
                    ).startswith("FAIL"):
                        errors.append(
                            f"READY-FOR-APPLY requires observed red-before for {finding_id}"
                        )
                    elif not nonempty_string(red_before.get("command")):
                        errors.append(
                            f"READY-FOR-APPLY requires concrete red-before for {finding_id}"
                        )
                green_after = record.get("green_after")
                if not check_record(green_after) or not nonempty_string(
                    green_after.get("command")
                ):
                    errors.append(
                        f"READY-FOR-APPLY requires concrete green-after for {finding_id}"
                    )
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("kind", choices=("findings", "dispositions", "plan"))
    parser.add_argument("path", type=Path)
    args = parser.parse_args()
    validators = {
        "findings": validate_findings,
        "dispositions": validate_dispositions,
        "plan": validate_plan,
    }
    errors = validators[args.kind](args.path)
    if errors:
        for error in errors:
            print(f"FAIL: {error}", file=sys.stderr)
        return 1
    print(f"{args.kind}: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
