#!/usr/bin/env python3
"""Configure once; prepare baselines/current Opi; explicitly execute; compare offline."""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import sys

from operator_config import init_profile, load, resolve
from operator_flow import REPO, compare, prepare, run_prepared
from operator_process import OperatorError


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="action", required=True)
    for action in ("init", "resolve", "baseline", "candidate"):
        command = commands.add_parser(action)
        command.add_argument("--config", type=Path, default=REPO / ".opi/eval.toml")
        if action != "init":
            command.add_argument("--set", action="append", default=[], metavar="KEY=TOML_VALUE")
        if action in {"baseline", "candidate"}:
            command.add_argument("--bench", choices=("tb21", "tb30", "deepswe"))
            command.add_argument("--companion", type=Path, help="identified supplied Companion; candidate still builds current Opi")
            command.add_argument("--prepared", type=Path, help="operation directory produced by preparation")
            command.add_argument("--execute", action="store_true", help="authorize precisely the frozen operation and retry/batch allowance")
    command = commands.add_parser("compare")
    command.add_argument("--candidate", required=True, type=Path, help="candidate snapshot.json")
    command.add_argument("--baseline", type=Path, help="explicit pi snapshot.json; otherwise use registered eligible snapshot")
    command.add_argument("--out", required=True, type=Path, help="fresh external comparison directory")
    arguments = parser.parse_args(argv)
    try:
        if arguments.action == "init":
            init_profile(arguments.config)
            result = {"status": "initialized", "config": str(arguments.config.resolve())}
        elif arguments.action == "resolve":
            result = resolve(load(arguments.config), arguments.set)
        elif arguments.action == "compare":
            result = compare(arguments.candidate, arguments.baseline, arguments.out)
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
            result = {"status": "prepared", "operation": str(root), "handoff": str(root / "handoff.json")}
        print(json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2))
        return 0 if result.get("status") not in {"stopped", "incomplete"} else 2
    except (OperatorError, OSError, KeyError, ValueError, StopIteration):
        # Details are deliberately restricted: exceptions from profile decoding or
        # third-party tools can embed credentials. Command records give safe detail.
        error = sys.exc_info()[1]
        message = str(error) if isinstance(error, OperatorError) else "operation refused; invalid prerequisites or evidence (inspect preserved command records)"
        print("opi-eval operator: " + message, file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
