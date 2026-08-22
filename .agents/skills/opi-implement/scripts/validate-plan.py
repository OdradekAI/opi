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
SHARED_DECISION_CLAUSES = (
    "decision_id",
    "role",
    "owner_task",
    "module",
    "interface",
    "representation",
    "consumer_tasks",
    "criterion_ids",
    "legacy_paths",
    "closure_test",
    "trigger",
)
SHARED_DECISION_TRIGGERS = {
    "intrinsic-state",
    "multiple-consumers",
    "expand-contract",
    "recurrent-finding",
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


def csv_values(value: str) -> set[str]:
    if value == "none":
        return set()
    return {item.strip() for item in value.split(",") if item.strip()}


def dependency_closure(
    task_id: str, tasks_by_id: dict[str, dict[str, object]]
) -> set[str]:
    seen: set[str] = set()
    depends_on = tasks_by_id[task_id].get("depends_on", [])
    pending = list(depends_on) if isinstance(depends_on, list) else []
    while pending:
        dependency = pending.pop()
        if not isinstance(dependency, str) or dependency in seen:
            continue
        seen.add(dependency)
        task = tasks_by_id.get(dependency)
        if task is None:
            continue
        transitive = task.get("depends_on", [])
        if isinstance(transitive, list):
            pending.extend(transitive)
    return seen


def task_verification_tokens(task: dict[str, object]) -> set[str]:
    tokens: set[str] = set()
    verification = task.get("verification", {})
    if isinstance(verification, dict):
        for field in ("behavioral_tests", "library_gates"):
            values = verification.get(field, [])
            if isinstance(values, list):
                tokens.update(value for value in values if isinstance(value, str))
    scenarios = task.get("acceptance_scenarios", [])
    if isinstance(scenarios, list):
        for scenario in scenarios:
            if not isinstance(scenario, dict):
                continue
            values = scenario.get("verification", [])
            if isinstance(values, list):
                tokens.update(value for value in values if isinstance(value, str))
    return tokens


def validate_shared_decisions(tasks: list[object]) -> list[str]:
    errors: list[str] = []
    tasks_by_id = {
        task["id"]: task
        for task in tasks
        if isinstance(task, dict)
        and isinstance(task.get("id"), str)
        and task.get("status") != "archived"
    }
    grouped: dict[
        str, list[tuple[str, dict[str, object], dict[str, str]]]
    ] = {}

    for task_index, task in enumerate(tasks):
        if not isinstance(task, dict) or task.get("status") == "archived":
            continue
        task_id = task.get("id")
        if not isinstance(task_id, str):
            continue
        notes = task.get("inference_notes", [])
        if not isinstance(notes, list):
            continue
        for note in notes:
            if not isinstance(note, dict) or note.get("field") != "shared_decision":
                continue
            source = note.get("source")
            if not isinstance(source, str) or not source.strip():
                errors.append(
                    f"task[{task_index}] shared_decision requires non-empty source"
                )
            reason = note.get("reason")
            if not isinstance(reason, str) or not reason.strip():
                errors.append(
                    f"task[{task_index}] shared_decision requires non-empty reason"
                )
                continue
            clauses: dict[str, str] = {}
            malformed = False
            for clause in SHARED_DECISION_CLAUSES:
                values = reason_clauses(reason, clause)
                if len(values) != 1 or not values[0]:
                    errors.append(
                        f"task[{task_index}] shared_decision requires exactly one "
                        f"non-empty {clause}="
                    )
                    malformed = True
                else:
                    clauses[clause] = values[0]
            if malformed:
                continue
            grouped.setdefault(clauses["decision_id"], []).append(
                (task_id, task, clauses)
            )

    shared_fields = (
        "owner_task",
        "module",
        "interface",
        "representation",
        "consumer_tasks",
        "criterion_ids",
        "legacy_paths",
        "closure_test",
        "trigger",
    )
    for decision_id, entries in grouped.items():
        first = entries[0][2]
        participant_ids = [task_id for task_id, _, _ in entries]
        if len(participant_ids) != len(set(participant_ids)):
            errors.append(
                f"decision {decision_id} requires one note per participating task"
            )
        for _, _, clauses in entries[1:]:
            for field in shared_fields:
                if clauses[field] != first[field]:
                    errors.append(f"decision {decision_id} disagrees on {field}")

        roles = [clauses["role"] for _, _, clauses in entries]
        invalid_roles = sorted(
            {role for role in roles if role not in {"owner", "consumer"}}
        )
        if invalid_roles:
            errors.append(
                f"decision {decision_id} has invalid roles "
                f"{','.join(invalid_roles)}"
            )
        owners = [
            task_id
            for task_id, _, clauses in entries
            if clauses["role"] == "owner"
        ]
        if len(owners) != 1:
            errors.append(f"decision {decision_id} requires exactly one owner")
            continue
        owner_task = first["owner_task"]
        if owners[0] != owner_task:
            errors.append(
                f"decision {decision_id} owner note {owners[0]} does not match "
                f"owner_task {owner_task}"
            )

        declared_consumers = csv_values(first["consumer_tasks"])
        noted_consumers = {
            task_id
            for task_id, _, clauses in entries
            if clauses["role"] == "consumer"
        }
        if declared_consumers != noted_consumers:
            errors.append(f"decision {decision_id} consumer notes do not match")

        for task_id, task, _ in entries:
            if task.get("evaluator_required") is not True:
                errors.append(
                    f"task {task_id} shared decision requires "
                    "evaluator_required=true"
                )
        for consumer in sorted(declared_consumers):
            if consumer not in tasks_by_id:
                errors.append(f"decision {decision_id} consumer {consumer} is missing")
                continue
            if owner_task not in dependency_closure(consumer, tasks_by_id):
                errors.append(
                    f"task {consumer} must depend transitively on owner {owner_task}"
                )

        scenario_ids: set[str] = set()
        for _, task, _ in entries:
            scenarios = task.get("acceptance_scenarios", [])
            if not isinstance(scenarios, list):
                continue
            scenario_ids.update(
                scenario["id"]
                for scenario in scenarios
                if isinstance(scenario, dict)
                and isinstance(scenario.get("id"), str)
            )
        criterion_ids = csv_values(first["criterion_ids"])
        if not criterion_ids:
            errors.append(f"decision {decision_id} requires a sourced criterion")
        for criterion_id in sorted(criterion_ids):
            if criterion_id not in scenario_ids:
                errors.append(
                    f"decision {decision_id} criterion {criterion_id} has no scenario"
                )

        owner = tasks_by_id.get(owner_task)
        if owner is None:
            errors.append(f"decision {decision_id} owner task {owner_task} is missing")
        elif first["closure_test"] not in task_verification_tokens(owner):
            errors.append(
                f"decision {decision_id} owner {owner_task} lacks closure_test "
                f"{first['closure_test']}"
            )

        trigger = first["trigger"]
        if trigger not in SHARED_DECISION_TRIGGERS:
            errors.append(f"decision {decision_id} trigger {trigger} is not recognized")
        if trigger == "multiple-consumers":
            production_participants = sum(
                bool(task.get("production_call_sites")) for _, task, _ in entries
            )
            if production_participants < 2:
                errors.append(
                    f"decision {decision_id} requires two production participants"
                )
        if trigger == "expand-contract":
            if not declared_consumers:
                errors.append(
                    f"decision {decision_id} expand-contract requires a consumer task"
                )
            if first["legacy_paths"] == "none":
                errors.append(
                    f"decision {decision_id} expand-contract requires "
                    "legacy_paths other than none"
                )
    return errors


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
    errors.extend(validate_shared_decisions(tasks))
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
