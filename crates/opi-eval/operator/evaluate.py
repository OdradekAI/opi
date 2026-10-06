#!/usr/bin/env python3
"""Configure once; prepare baselines/current Opi; explicitly execute; compare offline."""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import sys

from operator_config import init_profile, load, resolve, retention_policy
from operator_catalog import Store, safe_path
from operator_lifecycle import maintenance_plan, apply_plan, restore
from operator_flow import REPO, external, compare, prepare, run_prepared
from operator_process import OperatorError, write_json


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="action", required=True)
    for action in ("init", "resolve", "baseline", "candidate"):
        command = commands.add_parser(action)
        command.add_argument("--config", type=Path, default=REPO / ".opi/eval.toml")
        if action != "init":
            command.add_argument("--set", action="append", default=[], metavar="KEY=TOML_VALUE")
        if action in {"baseline", "candidate"}:
            command.add_argument("--bench", choices=("tb21", "tb30", "deepswe", "all"))
            command.add_argument("--companion", type=Path, help="identified supplied Companion; candidate still builds current Opi")
            command.add_argument("--prepared", type=Path, help="operation directory produced by preparation")
            command.add_argument("--execute", action="store_true", help="authorize precisely the frozen operation and retry/batch allowance")
    command = commands.add_parser("compare")
    command.add_argument("--candidate", required=True, type=Path, help="candidate snapshot.json")
    command.add_argument("--baseline", type=Path, help="explicit pi snapshot.json; otherwise use registered eligible snapshot")
    command.add_argument("--out", required=True, type=Path, help="fresh external comparison directory")
    for action in ("list", "show", "import-index", "pin", "unpin", "retire", "unretire", "archive", "gc", "apply-plan", "restore"):
        command = commands.add_parser(action)
        command.add_argument("--config", type=Path, default=REPO / ".opi/eval.toml")
        if action == "list":
            command.add_argument("--action", dest="record_action", choices=("baseline", "candidate", "compare", "acceptance"))
            command.add_argument("--bench", choices=("tb21", "tb30", "deepswe"))
            for key in ("model", "version", "state"):
                command.add_argument("--" + key)
            command.add_argument("--limit", type=int, default=100)
            command.add_argument("--offset", type=int, default=0)
        if action in {"show", "pin", "unpin", "retire", "unretire", "archive", "restore"}:
            command.add_argument("--id", required=True)
        if action in {"pin", "unpin", "retire", "unretire"}:
            command.add_argument("--reason", required=True)
        if action == "import-index":
            command.add_argument("--index", required=True, type=Path)
        if action in {"archive", "gc", "apply-plan"}:
            command.add_argument("--plan", required=True, type=Path, help="fresh preview output, or exact reviewed input for apply-plan")
        if action == "gc":
            command.add_argument("--id", action="append")
    arguments = parser.parse_args(argv)
    try:
        if arguments.action == "init":
            init_profile(arguments.config)
            result = {"status": "initialized", "config": str(arguments.config.resolve())}
        elif arguments.action == "resolve":
            result = resolve(load(arguments.config), arguments.set)
        elif arguments.action == "compare":
            result = compare(arguments.candidate, arguments.baseline, arguments.out)
        elif arguments.action not in {"baseline", "candidate"}:
            profile = resolve(load(arguments.config))
            store = Store(external(profile["storage"]["workspace"]))
            action = arguments.action
            if action == "list":
                filters = {key: getattr(arguments, key) for key in ("bench", "model", "version", "state", "limit", "offset")
                           if getattr(arguments, key) is not None}
                if arguments.record_action:
                    filters["action"] = arguments.record_action
                result = {"records": store.list(**filters), "workspace": str(store.workspace)}
            elif action == "show":
                result = store.show(arguments.id)
            elif action == "import-index":
                result = store.import_index(safe_path(arguments.index.resolve()))
            elif action in {"pin", "unpin", "retire", "unretire"}:
                if not arguments.reason.strip() or len(arguments.reason) > 4096:
                    raise OperatorError("lifecycle reason must contain 1..4096 characters")
                with store.lease(arguments.id):
                    store.event(arguments.id, action, reason=arguments.reason)
                result = store.show(arguments.id)
            elif action in {"archive", "gc"}:
                destination = safe_path(external(arguments.plan))
                # Preview files must not change any owned evidence or lifecycle inputs.
                for protected in (store.evaluations, store.events, store.imports, store.leases, store.workspace / "archives"):
                    if destination == protected or protected in destination.parents:
                        raise OperatorError("maintenance plan overlaps protected storage")
                plan = maintenance_plan(store, retention_policy(profile), "archive" if action == "archive" else "purge",
                                        ids=[arguments.id] if action == "archive" else arguments.id)
                write_json(destination, plan)
                result = {"status": "preview", "plan": str(destination), "selected": plan["selected"],
                          "selected_bytes": plan["selected_bytes"], "reclaimable_bytes": plan["reclaimable_bytes"],
                          "hot_bytes": plan["hot_bytes"]}
            elif action == "apply-plan":
                path = safe_path(external(arguments.plan))
                if path.stat().st_size > 8 * 1024 * 1024:
                    raise OperatorError("maintenance plan exceeds byte bound")
                plan = json.loads(path.read_text(encoding="utf-8"))
                if not isinstance(plan, dict) or plan.get("policy") != retention_policy(profile):
                    raise OperatorError("maintenance policy changed; create and review a fresh preview")
                result = apply_plan(store, path)
            elif action == "restore":
                result = restore(store, arguments.id)
        elif arguments.prepared:
            if not arguments.execute or arguments.bench or arguments.set or arguments.companion:
                raise OperatorError("prepared execution requires --execute and cannot override its frozen inputs")
            result = run_prepared(arguments.prepared, arguments.action)
        else:
            if arguments.execute:
                raise OperatorError("finish preparation, review its handoff, then execute with --prepared --execute")
            if not arguments.bench:
                raise OperatorError("preparation requires --bench")
            root = prepare(arguments.config, arguments.bench, arguments.action,
                           explicit=arguments.set, companion=arguments.companion)
            result = {"status": "prepared", "operation": str(root), "handoff": str(root / "handoff.json"),
                      "evaluation": str(root if arguments.bench == "all" else root.parent.parent)}
        print(json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2))
        return 0 if result.get("status") not in {"stopped", "incomplete"} else 2
    except (OperatorError, OSError, KeyError, ValueError, TypeError, RecursionError, StopIteration):
        # Details are deliberately restricted: exceptions from profile decoding or
        # third-party tools can embed credentials. Command records give safe detail.
        error = sys.exc_info()[1]
        message = str(error) if isinstance(error, OperatorError) else "operation refused; invalid prerequisites or evidence (inspect preserved command records)"
        print("opi-eval operator: " + message, file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
