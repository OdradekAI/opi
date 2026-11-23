"""Verify opi-ai's integration test inventory against Cargo and its source tree."""

import argparse
import csv
import json
from pathlib import Path
import subprocess
import sys
import tomllib


REPO = Path(__file__).resolve().parents[2]
CRATE = REPO / "crates/opi-ai"
FIELDS = ["target", "owner", "layer", "cost", "cases", "runners"]
LAYERS = {"public-contract", "wire", "assembly", "feature-profile"}
COSTS = {"in-process", "local-io", "extended"}
RUNNERS = {"archive", "matrix", "differential", "ci"}


def require_same(label, expected, actual):
    if expected != actual:
        raise ValueError(
            f"{label}: missing {sorted(expected - actual)}, extra {sorted(actual - expected)}"
        )


def resolve_inventory(inventory, targets, explicit, crate, repo):
    by_name = {target["name"]: target for target in targets}
    require_same("explicit Cargo declarations", set(by_name), explicit)
    root_sources = set((crate / "tests").glob("*.rs"))
    target_sources = {Path(target["src_path"]) for target in targets}
    if root_sources - target_sources:
        raise ValueError(f"unowned root test sources: {sorted(root_sources - target_sources)}")

    rows = []
    seen = set()
    with inventory.open(encoding="utf-8", newline="") as stream:
        reader = csv.DictReader(stream, delimiter="\t")
        if reader.fieldnames != FIELDS:
            raise ValueError(f"expected inventory columns {FIELDS}")
        for row in reader:
            if None in row or not all(row.get(key) for key in FIELDS):
                raise ValueError(f"incomplete inventory row at line {reader.line_num}")
            name = row["target"]
            if name in seen:
                raise ValueError(f"duplicate inventory target: {name}")
            seen.add(name)
            if row["layer"] not in LAYERS or row["cost"] not in COSTS:
                raise ValueError(f"invalid layer or cost for {name}")
            for binding in row["runners"].split() if row["runners"] != "-" else []:
                kind, _, path = binding.partition("@")
                if kind not in RUNNERS or not (repo / path).is_file():
                    raise ValueError(f"invalid runner for {name}: {binding}")
            rows.append(row)
    require_same("inventory targets", set(by_name), seen)
    for row in rows:
        target = by_name[row["target"]]
        path = Path(target["src_path"])
        if not path.is_file():
            raise ValueError(f"missing test source: {path}")
        row["source"] = path.relative_to(crate).as_posix()
        row["required-features"] = ",".join(target.get("required-features", [])) or "-"
    return rows


def load_inventory(inventory=CRATE / "tests/targets.tsv"):
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked", "--offline"],
        cwd=REPO,
    ))
    package = next(p for p in metadata["packages"] if p["name"] == "opi-ai")
    targets = [t for t in package["targets"] if "test" in t["kind"]]
    manifest = tomllib.loads((CRATE / "Cargo.toml").read_text(encoding="utf-8"))
    explicit = {t["name"] for t in manifest["test"]}
    return resolve_inventory(inventory, targets, explicit, CRATE, REPO)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", type=Path, default=CRATE / "tests/targets.tsv")
    parser.add_argument("--list", action="store_true", help="print resolved rows as TSV")
    args = parser.parse_args()
    try:
        rows = load_inventory(args.inventory)
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        parser.exit(1, f"{exc}\n")
    if args.list:
        writer = csv.DictWriter(sys.stdout, ["target", "source", "required-features", *FIELDS[1:]],
                                delimiter="\t", lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
    else:
        print(f"{len(rows)} explicit targets, {len({r['source'] for r in rows})} test sources")


if __name__ == "__main__":
    main()
