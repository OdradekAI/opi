"""Run the opi-ai architecture and test-boundary checks as one read-only gate."""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys

import check_architecture_boundaries as architecture
import check_support_inventory as support_inventory
import check_test_inventory as test_inventory
import select_tests as selector


REPO = Path(__file__).resolve().parents[2]
CRATE = REPO / "crates/opi-ai"


def summary_hash(report):
    encoded = json.dumps(report, ensure_ascii=False, sort_keys=True,
                         separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def source_selection(rows):
    """Check every integration source can be selected through its real consumer."""

    failures = []
    checked = []
    zero_test_targets = []
    paths = sorted((CRATE / "tests").rglob("*.rs"))
    repo_paths = [f"crates/opi-ai/{path.relative_to(CRATE).as_posix()}" for path in paths]
    plan = selector.make_plan(rows, repo_paths, [])
    selected_targets = {entry["target"] for entry in plan["tests"]}
    failures.extend(plan["expansion"])
    for row in rows:
        files = selector.test_inputs(CRATE, row["source"])
        test_count = sum(len(selector.TEST_FN.findall(text)) for text in files.values())
        if test_count == 0:
            zero_test_targets.append(row["target"])
        if row["target"] not in selected_targets:
            failures.append(f"target root {row['source']} does not select {row['target']}")
        checked.append({
            "path": row["source"],
            "target": row["target"],
            "tests": test_count,
            "scope": "selected" if row["target"] in selected_targets else "missing",
        })

    return {
        "checked_sources": checked,
        "source_count": len(paths),
        "zero_test_targets": zero_test_targets,
        "failures": sorted(set(failures)),
    }


def changed_selection(rows, paths):
    normalized = [path.replace("\\", "/") for path in paths]
    crate_paths = [path for path in normalized if path.startswith("crates/opi-ai/")]
    if not crate_paths:
        return {
            "paths": normalized,
            "scope": "no-crate-selector-needed",
            "tests": [],
            "runners": [],
            "expansion": [],
        }
    plan = selector.make_plan(rows, crate_paths, [])
    return {
        "paths": normalized,
        "scope": plan["scope"],
        "tests": [
            {"target": entry["target"], "args": entry["args"], "reasons": entry["reasons"]}
            for entry in plan["tests"]
        ],
        "runners": plan["runners"],
        "expansion": plan["expansion"],
    }


def build_report(paths=()):
    rows = test_inventory.load_inventory()
    support_rows = support_inventory.resolve_inventory(rows=rows)
    architecture_report = architecture.architecture_report()
    architecture_report["violations"] = architecture.violations(architecture_report)
    selection = source_selection(rows)
    changed = changed_selection(rows, paths) if paths else {
        "paths": [], "scope": "not-requested", "tests": [], "runners": [], "expansion": [],
    }
    violations = [f"architecture: {item}" for item in architecture_report["violations"]]
    violations.extend(f"selector: {item}" for item in selection["failures"])
    if selection["zero_test_targets"]:
        violations.append(
            "inventory: zero-test targets " + ", ".join(selection["zero_test_targets"])
        )
    approved_exceptions = {
        "route_context_edges": architecture_report["api_route_context_edges"],
        "profile_api_edges": architecture_report["profile_api_edges"],
        "lower_layer_reviewed_contracts": architecture_report["lower_layer_dependencies"]["reviewed_contracts"],
        "lower_layer_known_exceptions": architecture_report["lower_layer_dependencies"]["known_exceptions"],
        "lower_layer_excluded_tests": architecture_report["lower_layer_dependencies"]["excluded_tests"],
    }
    payload = {
        "schema": 1,
        "scope": "opi-ai architecture, target inventory, shared support and selector",
        "architecture": architecture_report,
        "targets": rows,
        "support": support_rows,
        "selector": selection,
        "changed_selection": changed,
        "approved_exceptions": approved_exceptions,
        "violations": sorted(set(violations)),
    }
    payload["summary_sha256"] = summary_hash(payload)
    return payload


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="*", help="changed paths relative to the repository")
    parser.add_argument("--json", action="store_true", help="emit the complete report")
    parser.add_argument(
        "--strict-selection", action="store_true",
        help="fail when changed crate paths require broader checks",
    )
    args = parser.parse_args()
    try:
        report = build_report(args.paths)
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        parser.exit(2, f"{exc}\n")
    if args.strict_selection and report["changed_selection"]["expansion"]:
        report["violations"].append("selector: changed paths require broader checks")
        report["violations"] = sorted(set(report["violations"]))
        unsigned = dict(report)
        unsigned.pop("summary_sha256", None)
        report["summary_sha256"] = summary_hash(unsigned)
    output = report if args.json else {
        "summary_sha256": report["summary_sha256"],
        "counts": report["architecture"]["counts"],
        "targets": len(report["targets"]),
        "support_helpers": len(report["support"]),
        "selector_sources": report["selector"]["source_count"],
        "changed_scope": report["changed_selection"]["scope"],
        "changed_expansion": report["changed_selection"]["expansion"],
        "violations": report["violations"],
    }
    print(json.dumps(output, ensure_ascii=False, indent=2))
    return 1 if report["violations"] else 0


if __name__ == "__main__":
    sys.exit(main())
