#!/usr/bin/env python3
"""Deterministic artifact audit for opi dogfood runs.

Analyzes saved local artifacts only (no network). Detects:
  - workspace-root path leakage in public NDJSON and session JSONL
    (message/tool records; the by-design session-header `cwd` is skipped);
  - all-zero runtime message timestamps;
  - session_summary.provider_turns != TurnStart count;
  - O(n^2) text_delta lines that carry a redundant cumulative `partial`;
  - report mentions of provider failures with no preserved raw artifact
    and no explicit disclosure phrase.

Known limitations (documented, not blocking):
  - Failure-word matching is a deliberately conservative trip-wire; it
    cannot fully distinguish an honest disclosure from a false claim, so
    explicit disclosure phrases are allow-listed.
  - The default session-dir glob matches the opi-run-sandbox shape; pass a
    custom dir layout for other shapes.
"""
import argparse
import json
import os
import pathlib
import re
import sys
from collections import Counter


FAILURE_WORDS = ["ProviderFailure", "HTTP 429", "HTTP 404", "rate limited"]
DISCLOSURE_PHRASES = [
    "not preserved",
    "overwritten before being preserved",
    "observed-unpreserved",
    "observed live",
    "did not preserve",
]


def read_text(path):
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except FileNotFoundError:
        return ""


def parse_json_lines(path):
    records = []
    for line_no, line in enumerate(read_text(path).splitlines(), start=1):
        if not line.strip():
            continue
        try:
            records.append((line_no, json.loads(line)))
        except json.JSONDecodeError as exc:
            records.append((line_no, {"_parse_error": str(exc), "_raw": line}))
    return records


def _norm(value):
    """Normalize a path-ish string for substring comparison across platforms.

    Collapses any run of forward/backslashes to a single "/" so JSON-escaped
    backslashes ("\\\\") and POSIX slashes compare equal (json.dumps doubles
    every backslash, so a single .replace would leave "//" and miss the match).
    """
    if not value:
        return ""
    norm = os.path.normcase(value)
    norm = re.sub(r"[\\/]+", "/", norm)
    return norm


def _root_forms(workspace_root):
    """All comparable forms of the workspace root to detect in rendered JSON."""
    forms = set()
    if workspace_root:
        forms.add(_norm(workspace_root))
        forms.add(_norm(workspace_root.replace("/", "\\")))
        forms.add(_norm(workspace_root.replace("\\", "/")))
    return [f for f in forms if f]


def _leaks_root(rendered, root_forms_norm):
    norm = _norm(rendered)
    return any(f and f in norm for f in root_forms_norm)


def analyze_ndjson(path, root_forms_norm):
    counts = Counter()
    timestamps = []
    provider_turns = 0
    summary_turns = None
    summary_provider_turns = None
    text_delta_lines = 0
    text_delta_bytes = 0
    partial_mentions = 0
    issues = []

    for line_no, record in parse_json_lines(path):
        if "_parse_error" in record:
            issues.append({
                "code": "invalid_json_line",
                "file": str(path),
                "line": line_no,
                "message": record["_parse_error"],
            })
            continue

        event_type = record.get("type")
        counts[event_type] += 1
        rendered = json.dumps(record, ensure_ascii=False)
        # NDJSON carries no session-cwd field, so any root occurrence here is a leak.
        if _leaks_root(rendered, root_forms_norm):
            issues.append({
                "code": "workspace_root_leak",
                "file": str(path),
                "line": line_no,
                "message": "NDJSON line contains the workspace root",
            })

        if event_type == "Agent":
            inner = record.get("event", {})
            inner_type = inner.get("type")
            counts[f"Agent.{inner_type}"] += 1
            if inner_type == "TurnStart":
                provider_turns += 1
            message = inner.get("message")
            if isinstance(message, dict) and isinstance(message.get("timestamp_ms"), int):
                timestamps.append(message["timestamp_ms"])
            assistant_event = inner.get("assistant_event")
            if isinstance(assistant_event, dict):
                # Wire emits snake_case "text_delta" (#[serde(rename=...)]).
                if assistant_event.get("type") == "text_delta":
                    text_delta_lines += 1
                    text_delta_bytes += len(json.dumps(assistant_event, ensure_ascii=False))
                    if "partial" in assistant_event:
                        partial_mentions += 1
        elif event_type == "session_summary":
            summary_turns = record.get("turns")
            summary_provider_turns = record.get("provider_turns")

    if timestamps and all(ts == 0 for ts in timestamps):
        issues.append({
            "code": "all_zero_timestamps",
            "file": str(path),
            "message": "all observed runtime message timestamps are zero",
        })

    if summary_provider_turns is not None and summary_provider_turns != provider_turns:
        issues.append({
            "code": "provider_turn_mismatch",
            "file": str(path),
            "message": f"summary provider_turns={summary_provider_turns} but TurnStart count={provider_turns}",
        })

    if text_delta_lines >= 50 and partial_mentions == text_delta_lines:
        issues.append({
            "code": "duplicated_text_delta_partials",
            "file": str(path),
            "message": f"{text_delta_lines} text_delta events carry assistant_event.partial (O(n^2) shape)",
        })

    return {
        "file": str(path),
        "counts": dict(counts),
        "provider_turns_seen": provider_turns,
        "summary_turns": summary_turns,
        "summary_provider_turns": summary_provider_turns,
        "timestamp_count": len(timestamps),
        "zero_timestamp_count": sum(1 for ts in timestamps if ts == 0),
        "text_delta_lines": text_delta_lines,
        "text_delta_bytes": text_delta_bytes,
        "partial_mentions": partial_mentions,
        "issues": issues,
    }


def analyze_session(path, root_forms_norm):
    issues = []
    records = parse_json_lines(path)
    for line_no, record in records:
        # The session header (type == "session") legitimately stores cwd.
        if isinstance(record, dict) and record.get("type") == "session":
            continue
        rendered = json.dumps(record, ensure_ascii=False)
        if _leaks_root(rendered, root_forms_norm):
            issues.append({
                "code": "session_workspace_root_leak",
                "file": str(path),
                "line": line_no,
                "message": "session message/leaf record contains the workspace root",
            })
    return {"file": str(path), "records": len(records), "issues": issues}


def analyze_failure_evidence(artifact_dir):
    report_blob = "\n".join(
        read_text(path)
        for path in [artifact_dir / "RUN_SUMMARY.md", artifact_dir / "REVIEW_REPORT.md"]
    )
    report_lower = report_blob.lower()
    mentioned = [w for w in FAILURE_WORDS if w.lower() in report_lower]
    disclosed = any(phrase in report_lower for phrase in DISCLOSURE_PHRASES)

    preserved = []
    for path in artifact_dir.glob("run*.ndjson"):
        body = read_text(path).lower()
        preserved.extend(w for w in FAILURE_WORDS if w.lower() in body)
    stderr = read_text(artifact_dir / "run.stderr.log").lower()
    preserved.extend(w for w in FAILURE_WORDS if w.lower() in stderr)

    issues = []
    if mentioned and not preserved and not disclosed:
        issues.append({
            "code": "failure_claim_without_preserved_artifact",
            "message": (
                "report mentions failure markers "
                f"{sorted(set(mentioned))} but preserved run artifacts do not "
                "contain them and no disclosure phrase was found"
            ),
        })
    return {
        "mentioned": sorted(set(mentioned)),
        "preserved": sorted(set(preserved)),
        "disclosed": disclosed,
        "issues": issues,
    }


def _session_files(artifact_dir):
    # Broad glob: top-level sessions/, sibling attempts, and nested shapes.
    seen = set()
    for path in artifact_dir.rglob("sessions*"):
        if path.is_dir():
            for f in path.glob("*.jsonl"):
                seen.add(f.resolve())
    return sorted(seen)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("artifact_dir")
    parser.add_argument("--workspace-root", default="")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    artifact_dir = pathlib.Path(args.artifact_dir)
    root_forms_norm = _root_forms(args.workspace_root)

    ndjson_reports = [
        analyze_ndjson(path, root_forms_norm)
        for path in sorted(artifact_dir.glob("run*.ndjson"))
    ]
    session_reports = [analyze_session(path, root_forms_norm) for path in _session_files(artifact_dir)]
    failure_report = analyze_failure_evidence(artifact_dir)

    issues = []
    for report in ndjson_reports:
        issues.extend(report["issues"])
    for report in session_reports:
        issues.extend(report["issues"])
    issues.extend(failure_report["issues"])

    report = {
        "artifact_dir": str(artifact_dir),
        "files": sorted(
            str(path.relative_to(artifact_dir))
            for path in artifact_dir.rglob("*")
            if path.is_file()
        ),
        "ndjson": ndjson_reports,
        "sessions": session_reports,
        "failure_evidence": failure_report,
        "issues": issues,
        "ok": not issues,
    }

    if args.json:
        print(json.dumps(report, indent=2, ensure_ascii=False))
    else:
        status = "PASS" if report["ok"] else "FAIL"
        print(f"artifact audit: {status}")
        for issue in issues:
            location = issue.get("file", "<artifact>")
            line = issue.get("line")
            suffix = f":{line}" if line else ""
            print(f"- {issue['code']} {location}{suffix}: {issue['message']}")
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
