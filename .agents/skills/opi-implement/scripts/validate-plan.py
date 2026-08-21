#!/usr/bin/env python3
"""Validate deterministic opi-implement plan-admission evidence."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path


REQUIRED_NOTE_FIELDS = (
    "reuse_search",
    "placement",
    "surface_necessity",
    "simplification_ceiling",
)
REQUIRED_REASON_CLAUSES = {
    "reuse_search": ("searched", "reused", "gap"),
    "placement": ("target", "existing_home", "cannot_fit_fully"),
    "surface_necessity": (
        "public_api",
        "config",
        "state",
        "dependency_edge",
    ),
    "simplification_ceiling": (
        "ceiling",
        "revisit_when",
        "simplification_trigger",
    ),
}
PLACEMENT_TARGETS = {
    "core",
    "reference-product",
    "extension",
    "plugin",
    "package",
    "independent-companion",
    "assurance",
}
SIMPLIFICATION_TRIGGERS = {
    "none",
    "unused",
    "duplicate",
    "superseded",
    "delete",
    "merge",
    "replace",
    "dependency-substitution",
}


def reason_clauses(reason: str, name: str) -> list[str]:
    matches = re.finditer(
        rf"(?:^|;)\s*{re.escape(name)}\s*=\s*(?P<value>[^;]*?)\s*(?=;|$)",
        reason,
    )
    return [match.group("value").strip() for match in matches]


def reason_clause(reason: str, name: str) -> str | None:
    values = reason_clauses(reason, name)
    if len(values) != 1 or not values[0]:
        return None
    return values[0]


def validate_plan(ledger: object) -> list[str]:
    if not isinstance(ledger, dict):
        return ["plan ledger must be a JSON object"]
    if ledger.get("schema_version") != 2:
        return ["plan ledger schema_version must be 2"]

    tasks = ledger.get("tasks")
    if not isinstance(tasks, list):
        return ["plan ledger tasks must be an array"]

    errors: list[str] = []
    for task_index, task in enumerate(tasks):
        if not isinstance(task, dict):
            errors.append(f"task[{task_index}] must be an object")
            continue
        if task.get("status") == "archived":
            continue
        notes = task.get("inference_notes")
        if not isinstance(notes, list):
            errors.append(f"task[{task_index}] inference_notes must be an array")
            continue
        required_notes: dict[str, dict[str, object]] = {}
        for field in REQUIRED_NOTE_FIELDS:
            matching_notes = [
                note
                for note in notes
                if isinstance(note, dict) and note.get("field") == field
            ]
            if len(matching_notes) != 1:
                errors.append(
                    f"task[{task_index}] requires exactly one {field} note"
                )
                continue

            note = matching_notes[0]
            required_notes[field] = note
            source = note.get("source")
            if not isinstance(source, str) or not source.strip():
                errors.append(
                    f"task[{task_index}] {field} requires non-empty source"
                )

            reason = note.get("reason")
            if not isinstance(reason, str) or not reason.strip():
                errors.append(
                    f"task[{task_index}] {field} requires non-empty reason"
                )
                continue
            for clause in REQUIRED_REASON_CLAUSES[field]:
                values = reason_clauses(reason, clause)
                if not values:
                    errors.append(
                        f"task[{task_index}] {field} requires {clause}="
                    )
                elif len(values) != 1:
                    errors.append(
                        f"task[{task_index}] {field} requires exactly one "
                        f"{clause}="
                    )
                elif not values[0]:
                    errors.append(
                        f"task[{task_index}] {field} requires non-empty "
                        f"{clause}="
                    )

        placement = required_notes.get("placement")
        if placement is not None and isinstance(placement.get("reason"), str):
            target = reason_clause(placement["reason"], "target")
            if target is not None and target not in PLACEMENT_TARGETS:
                errors.append(
                    f"task[{task_index}] placement target '{target}' "
                    "is not recognized"
                )

        ceiling = required_notes.get("simplification_ceiling")
        reuse = required_notes.get("reuse_search")
        if ceiling is None or not isinstance(ceiling.get("reason"), str):
            continue
        trigger = reason_clause(ceiling["reason"], "simplification_trigger")
        if trigger is None:
            continue
        if trigger not in SIMPLIFICATION_TRIGGERS:
            errors.append(
                f"task[{task_index}] simplification_trigger '{trigger}' "
                "is not recognized"
            )
            continue
        if trigger == "none":
            continue

        reuse_reason = ""
        if reuse is not None and isinstance(reuse.get("reason"), str):
            reuse_reason = reuse["reason"]
        for clause in ("production_consumers", "nonproduction_consumers"):
            values = reason_clauses(reuse_reason, clause)
            if not values:
                errors.append(
                    f"task[{task_index}] simplification_trigger={trigger} "
                    f"requires {clause}="
                )
            elif len(values) != 1:
                errors.append(
                    f"task[{task_index}] simplification_trigger={trigger} "
                    f"requires exactly one {clause}="
                )
            elif not values[0]:
                errors.append(
                    f"task[{task_index}] simplification_trigger={trigger} "
                    f"requires non-empty {clause}="
                )
        for clause in ("net_deletion", "residual_glue"):
            values = reason_clauses(ceiling["reason"], clause)
            if not values:
                errors.append(
                    f"task[{task_index}] simplification_trigger={trigger} "
                    f"requires {clause}="
                )
            elif len(values) != 1:
                errors.append(
                    f"task[{task_index}] simplification_trigger={trigger} "
                    f"requires exactly one {clause}="
                )
            elif not values[0]:
                errors.append(
                    f"task[{task_index}] simplification_trigger={trigger} "
                    f"requires non-empty {clause}="
                )
    return errors


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: validate-plan.py <draft-ledger.json>", file=sys.stderr)
        return 2

    path = Path(argv[1])
    try:
        ledger = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        print(f"plan ledger is unreadable: {exc}", file=sys.stderr)
        return 1

    errors = validate_plan(ledger)
    if errors:
        for error in errors:
            print(f"FAIL: {error}", file=sys.stderr)
        return 1

    print(f"plan validation ok tasks={len(ledger['tasks'])}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
