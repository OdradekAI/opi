"""Verify ownership and consumers for opi-ai integration-test support helpers."""

import argparse
import csv
from pathlib import Path
import subprocess
import sys

import check_test_inventory as test_inventory
import select_tests


REPO = Path(__file__).resolve().parents[2]
CRATE = REPO / "crates/opi-ai"
SUPPORT = CRATE / "tests/support"
DEFAULT_INVENTORY = SUPPORT / "OWNERS.tsv"
FIELDS = ["helper", "role", "owner", "layers", "cost", "runners", "consumers"]
ROLES = {"stable-tool", "protocol-fixture", "platform-fixture"}
LAYERS = set(test_inventory.LAYERS)
COSTS = set(test_inventory.COSTS)
RUNNERS = set(test_inventory.RUNNERS)


def parse_list(value, separator=","):
    return [item.strip() for item in value.split(separator) if item.strip()]


def actual_consumers(rows, helper):
    path = (SUPPORT / helper).resolve()
    consumers = set()
    for row in rows:
        source = CRATE / row["source"]
        if path in select_tests.test_inputs(CRATE, source):
            consumers.add(row["target"])
    return consumers


def validate_runner(binding):
    kind, separator, command = binding.partition("@")
    if not separator or kind not in RUNNERS:
        raise ValueError(f"invalid runner binding: {binding}")
    script = command.split(maxsplit=1)[0]
    if not (REPO / script).is_file():
        raise ValueError(f"invalid runner path: {binding}")


def resolve_inventory(path=DEFAULT_INVENTORY, rows=None):
    if rows is None:
        rows = test_inventory.load_inventory()
    helpers = {
        item.relative_to(SUPPORT).as_posix()
        for item in SUPPORT.rglob("*.rs")
    }
    declared = {}
    with path.open(encoding="utf-8", newline="") as stream:
        reader = csv.DictReader(stream, delimiter="\t")
        if reader.fieldnames != FIELDS:
            raise ValueError(f"expected support inventory columns {FIELDS}")
        for row in reader:
            if None in row or not all(row.get(key) for key in FIELDS):
                raise ValueError(f"incomplete support inventory row at line {reader.line_num}")
            helper = row["helper"]
            if helper in declared:
                raise ValueError(f"duplicate support helper: {helper}")
            if helper not in helpers:
                raise ValueError(f"missing support helper: {helper}")
            if row["role"] not in ROLES:
                raise ValueError(f"invalid support role for {helper}: {row['role']}")
            layers = set(parse_list(row["layers"]))
            if not layers or not layers <= LAYERS:
                raise ValueError(f"invalid support layers for {helper}: {row['layers']}")
            if row["cost"] not in COSTS:
                raise ValueError(f"invalid support cost for {helper}: {row['cost']}")
            runners = [] if row["runners"] == "-" else parse_list(row["runners"], separator=";")
            for runner in runners:
                validate_runner(runner)
            consumers = set(parse_list(row["consumers"]))
            if not consumers or "-" in consumers:
                raise ValueError(f"support helper has no consumers: {helper}")
            actual = actual_consumers(rows, helper)
            if consumers != actual:
                raise ValueError(
                    f"consumers for {helper}: missing {sorted(actual - consumers)}, "
                    f"extra {sorted(consumers - actual)}"
                )
            declared[helper] = row
    if declared.keys() != helpers:
        raise ValueError(
            f"support inventory helpers: missing {sorted(helpers - declared.keys())}, "
            f"extra {sorted(declared.keys() - helpers)}"
        )
    production = list((CRATE / "src").rglob("*.rs"))
    for helper in helpers:
        marker = f"tests/support/{helper}"
        for source in production:
            relative = source.relative_to(CRATE).as_posix()
            if "/tests/" in f"/{relative}/":
                continue
            if marker in source.read_text(encoding="utf-8"):
                raise ValueError(f"production source copies support helper path: {helper}")
    return [declared[helper] for helper in sorted(declared)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", type=Path, default=DEFAULT_INVENTORY)
    parser.add_argument("--list", action="store_true", help="print resolved rows as TSV")
    args = parser.parse_args()
    try:
        rows = resolve_inventory(args.inventory)
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        parser.exit(1, f"{exc}\n")
    if args.list:
        writer = csv.DictWriter(sys.stdout, FIELDS, delimiter="\t", lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
    else:
        consumers = sum(len(parse_list(row["consumers"])) for row in rows)
        print(f"{len(rows)} support helpers, {consumers} declared target edges")


if __name__ == "__main__":
    main()
