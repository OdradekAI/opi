"""Typed advisory suffixes bound to an already installed independent audit core.

This codec never reads historical paths or evaluates historical claims. It checks
shape, identity and byte integrity; the workflow owns evidence applicability.
"""

from __future__ import annotations

import hashlib
import json
import re
from typing import Any

SCHEMA = "opi-audit-history/1"
RESERVED = b"<!-- opi-audit-history/"
START = b"\n<!-- opi-audit-history/1\n"
END = b"\n-->\n"
SIDECARS = ("meta_sha256", "requirements_sha256", "findings_sha256")
RELATIONSHIPS = {
    "persistent", "missed-branch", "regression", "new-omission",
    "evidence-gap", "insufficient-information", "preserved-repair",
}


class HistoryError(ValueError):
    """An appendix is malformed or does not belong to the supplied audit core."""


def _object(value: Any, fields: set[str], label: str) -> dict:
    if not isinstance(value, dict) or set(value) != fields:
        raise HistoryError(f"{label}: expected exactly {', '.join(sorted(fields))}")
    return value


def _text(value: Any, label: str) -> str:
    if (not isinstance(value, str) or not value.strip()
            or any(ord(c) < 32 and c not in "\n\t" for c in value)):
        raise HistoryError(f"{label}: expected non-empty text without control characters")
    try:
        value.encode("utf-8")
    except UnicodeEncodeError as exc:
        raise HistoryError(f"{label}: text must be UTF-8 encodable") from exc
    return value


def _list(value: Any, label: str) -> list:
    if not isinstance(value, list):
        raise HistoryError(f"{label}: expected a list")
    return value


def _digest(value: Any, length: int, label: str) -> str:
    if not isinstance(value, str) or re.fullmatch(rf"[0-9a-f]{{{length}}}", value) is None:
        raise HistoryError(f"{label}: expected a lowercase {length}-character digest")
    return value


def validate_comparison(value: Any, finding_ids: set[str]) -> None:
    value = _object(value, {"schema", "status", "scope", "sources", "comparisons", "limitations"}, "comparison")
    if value["schema"] != SCHEMA:
        raise HistoryError("comparison schema is invalid")
    if value["status"] not in ("compared", "no-history"):
        raise HistoryError("comparison status must be compared or no-history")
    _text(value["scope"], "history search scope")
    _text(value["limitations"], "comparison limitations")
    sources = _list(value["sources"], "sources")
    rows = _list(value["comparisons"], "comparisons")
    if value["status"] == "no-history" and rows:
        raise HistoryError("no-history cannot contain comparison rows")
    if value["status"] == "compared" and (not sources or not rows):
        raise HistoryError("compared requires historical sources and comparison rows")
    source_runs: dict[str, str | None] = {}
    for source in sources:
        _object(source, {"id", "path", "commit", "sha256", "audit_run_id"}, "history source")
        source_id = _text(source["id"], "source id")
        if source_id in source_runs:
            raise HistoryError("duplicate history source id")
        _text(source["path"], "source path")
        _digest(source["commit"], 40, "source commit")
        _digest(source["sha256"], 64, "source SHA-256")
        if source["audit_run_id"] is not None:
            _text(source["audit_run_id"], "source audit run")
        source_runs[source_id] = source["audit_run_id"]
    current_ids: set[str] = set()
    for row in rows:
        _object(row, {"current_finding_id", "prior_findings", "relationship", "prior_claim",
                      "repair_commits", "current_evidence", "cause", "raw_evidence", "verification"}, "comparison row")
        relationship = _text(row["relationship"], "relationship")
        if relationship not in RELATIONSHIPS:
            raise HistoryError("invalid history relationship")
        current = row["current_finding_id"]
        if current is None:
            if relationship != "preserved-repair":
                raise HistoryError("only preserved-repair may omit a current finding")
        else:
            _text(current, "current finding")
            if current not in finding_ids:
                raise HistoryError(f"unknown current finding: {current}")
            if current in current_ids:
                raise HistoryError(f"duplicate current finding comparison: {current}")
            if relationship == "preserved-repair":
                raise HistoryError("preserved-repair must remain outside current finding scope")
            current_ids.add(current)
        prior = _list(row["prior_findings"], "prior findings")
        for ref in prior:
            _object(ref, {"source_id", "finding_id", "audit_run_id"}, "prior finding")
            source_id = _text(ref["source_id"], "prior source id")
            _text(ref["finding_id"], "prior finding id")
            _text(ref["audit_run_id"], "prior audit run")
            if source_id not in source_runs or source_runs[source_id] != ref["audit_run_id"]:
                raise HistoryError("prior finding source/run does not match historical source")
        for field in ("prior_claim", "current_evidence", "cause", "verification"):
            _text(row[field], field)
        for commit in _list(row["repair_commits"], "repair commits"):
            _digest(commit, 40, "repair commit")
        for evidence in _list(row["raw_evidence"], "raw evidence"):
            _object(evidence, {"path", "sha256", "checked", "limitations"}, "raw evidence")
            for field in ("path", "checked", "limitations"):
                _text(evidence[field], "raw evidence " + field)
            _digest(evidence["sha256"], 64, "raw evidence SHA-256")


def _literal(value: str) -> str:
    """Render record text without active HTML, Markdown links or section markers."""
    escaped = re.sub(r"[&<>\"'\\`*_{}\[\]()#+.!|~\-]", lambda m: f"&#{ord(m[0])};", value)
    return escaped.replace("\n", "<br>").replace("\t", "    ")


def _render(envelope: dict) -> bytes:
    serialized = json.dumps(envelope, ensure_ascii=False, indent=2, sort_keys=True)
    serialized = re.sub(r"[<>&*`_]", lambda m: f"\\u{ord(m[0]):04x}", serialized)
    value = envelope["comparison"]
    lines = [
        "", "## Post-audit historical reference — advisory", "",
        "This section was appended after installation of the independent core. "
        "It cannot change findings, requirements, independence or verdict. "
        "Historical closure claims require current independent verification.", "",
        f"- Owning audit run: {_literal(envelope['audit_run_id'])}",
        f"- Audit head: {_literal(envelope['audit_head'])}",
        f"- Core report bytes: {envelope['core']['report_bytes']}",
        f"- Core report SHA-256: {envelope['core']['report_sha256']}",
        f"- Current findings SHA-256: {envelope['core']['findings_sha256']}",
        f"- Comparison status: {_literal(value['status'])}",
        f"- Inspected history: {_literal(value['scope'])}",
        f"- Limitations: {_literal(value['limitations'])}",
        "", "### Pinned historical sources", "",
    ]
    if not value["sources"]:
        lines.append("No relevant historical source found within the stated scope.")
    for source in value["sources"]:
        lines.append(f"- {_literal(source['id'])}: {_literal(source['path'])}; "
                     f"commit {source['commit']}; SHA-256 {source['sha256']}; "
                     f"audit run {_literal(source['audit_run_id'] or 'not an audit report')}")
    for number, row in enumerate(value["comparisons"], 1):
        lines.extend(["", f"### Comparison {number}: {_literal(row['current_finding_id'] or 'preserved historical repair')}", ""])
        fields = [("Relationship", row["relationship"]), ("Prior claim (attributed)", row["prior_claim"]),
                  ("Current evidence", row["current_evidence"]), ("Cause / missed boundary", row["cause"]),
                  ("Suggested current verification", row["verification"])]
        for label, text in fields:
            lines.append(f"- {label}: {_literal(text)}")
        for prior in row["prior_findings"]:
            lines.append(f"- Prior finding: {_literal(prior['audit_run_id'])} / "
                         f"{_literal(prior['finding_id'])}; source {_literal(prior['source_id'])}")
        if row["repair_commits"]:
            lines.append("- Repair commits: " + ", ".join(row["repair_commits"]))
        for evidence in row["raw_evidence"]:
            lines.append(f"- Candidate raw proof: {_literal(evidence['path'])}; SHA-256 {evidence['sha256']}; "
                         f"checked: {_literal(evidence['checked'])}; limits: {_literal(evidence['limitations'])}")
    return START + serialized.encode("utf-8") + END + ("\n".join(lines) + "\n").encode("utf-8")


def build_report(core: bytes, comparison: dict, member: dict, finding_ids: set[str]) -> bytes:
    if RESERVED in core:
        raise HistoryError("independent core must not contain a reserved history envelope")
    validate_comparison(comparison, finding_ids)
    digest = hashlib.sha256(core).hexdigest()
    if digest != member["digests"]["report_sha256"]:
        raise HistoryError("independent core report digest does not match installed member")
    envelope = {
        "schema": SCHEMA, "audit_run_id": member["audit_run_id"], "audit_head": member["audit_head"],
        "core": {"report_bytes": len(core), "report_sha256": digest,
                 **{field: member["digests"][field] for field in SIDECARS}},
        "comparison": comparison,
    }
    return core + _render(envelope)


def _unique_object(pairs: list[tuple[str, Any]]) -> dict:
    result = {}
    for key, value in pairs:
        if key in result:
            raise HistoryError(f"duplicate history JSON field: {key}")
        result[key] = value
    return result


def split_report(report: bytes, member: dict, finding_ids: set[str]) -> tuple[bytes, dict | None]:
    if RESERVED not in report:
        return report, None
    if report.count(RESERVED) != 1 or START not in report:
        raise HistoryError("duplicate or malformed reserved history envelope")
    core, tail = report.split(START, 1)
    if END not in tail:
        raise HistoryError("unterminated history envelope")
    payload, _ = tail.split(END, 1)
    try:
        envelope = json.loads(payload.decode("utf-8"), object_pairs_hook=_unique_object)
    except (UnicodeDecodeError, json.JSONDecodeError, RecursionError) as exc:
        raise HistoryError(f"invalid history JSON: {exc}") from exc
    _object(envelope, {"schema", "audit_run_id", "audit_head", "core", "comparison"}, "history envelope")
    if envelope["schema"] != SCHEMA:
        raise HistoryError("invalid history envelope schema")
    for field in ("audit_run_id", "audit_head"):
        if envelope[field] != member[field]:
            raise HistoryError(f"history {field} does not match owning member")
    binding = _object(envelope["core"], {"report_bytes", "report_sha256", *SIDECARS}, "core binding")
    if type(binding["report_bytes"]) is not int or binding["report_bytes"] != len(core):
        raise HistoryError("independent core byte length mismatch")
    if binding["report_sha256"] != hashlib.sha256(core).hexdigest():
        raise HistoryError("independent core report digest mismatch")
    for field in SIDECARS:
        if binding[field] != member["digests"].get(field):
            raise HistoryError(f"independent core {field} does not match owning member")
    validate_comparison(envelope["comparison"], finding_ids)
    if report != core + _render(envelope):
        raise HistoryError("history suffix is not the canonical rendered envelope")
    return core, envelope
