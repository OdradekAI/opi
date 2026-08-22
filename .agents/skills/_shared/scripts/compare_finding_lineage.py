#!/usr/bin/env python3
"""Classify verified findings against immutable remediation history."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


DEFERRED = {
    "Deferred",
    "Deferred by registered source",
    "Returned to shaping",
}


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for number, line in enumerate(
        path.read_text(encoding="utf-8-sig").splitlines(),
        start=1,
    ):
        if not line.strip():
            continue
        value = json.loads(line)
        if not isinstance(value, dict):
            raise ValueError(f"{path}:{number}: record must be an object")
        records.append(value)
    return records


def occurrence(record: dict[str, Any]) -> dict[str, str]:
    source = record.get("source")
    if isinstance(source, dict):
        return {
            "source_path": str(source.get("source_path", "")),
            "id": str(source.get("id", "")),
            "verified_at": str(record.get("verified_at", "")),
        }
    return {
        "source_path": str(record.get("source_path", "")),
        "id": str(record.get("id", "")),
        "verified_at": str(record.get("verified_at", "")),
    }


def classify(
    current: dict[str, Any],
    history: list[dict[str, Any]],
) -> dict[str, Any]:
    closure_key = current.get("closure_key")
    family_key = current.get("family_key")
    regression_of = current.get("regression_of")
    exact = [item for item in history if item.get("closure_key") == closure_key]
    family = [item for item in history if item.get("family_key") == family_key]
    closed_regressions = [
        item
        for item in history
        if item.get("closure_key") == regression_of
        and item.get("remediation_status") == "Closed"
    ]
    if regression_of and closed_regressions:
        kind = "regression"
        matched = closed_regressions
    elif exact:
        dispositions = {str(item.get("remediation_status", "unknown")) for item in exact}
        kind = (
            "carried-forward-deferred"
            if dispositions and dispositions <= DEFERRED
            else "recurrent-same-defect"
        )
        matched = exact
    elif family:
        kind = "recurrent-adjacent-path"
        matched = family
    else:
        kind = "new"
        matched = []
    dispositions = sorted(
        {str(item.get("remediation_status", "unknown")) for item in matched}
    )
    result = dict(current)
    result["lineage"] = {
        "kind": kind,
        "prior_occurrences": [occurrence(item) for item in matched],
        "prior_disposition": ", ".join(dispositions) if dispositions else "none",
    }
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--current", required=True, type=Path)
    parser.add_argument("--history", type=Path, nargs="*", default=[])
    args = parser.parse_args()
    try:
        current = load_jsonl(args.current)
        history = [
            record
            for history_path in args.history
            for record in load_jsonl(history_path)
        ]
        for record in current:
            if not isinstance(record.get("closure_key"), str) or not isinstance(
                record.get("family_key"), str
            ):
                raise ValueError("current records require closure_key and family_key")
            print(json.dumps(classify(record, history), sort_keys=True))
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
