#!/usr/bin/env python3
"""One-time extractor: build the fixed 72,413-case PMD boundary replay
fixtures from the sealed research oracle corpus (RDF-24 owned-WS research).

Reads oracle-cases.jsonl (the recorded C-zlib-oracle classification of the
deterministically generated corpus) and writes:

- boundary-seeds.json: the 13 fixed seed wires with their expected outputs.
- boundary-expectations.json: per-group case counts plus, for every valid
  case, the exact output (<= 64 bytes) or its SHA-256 (longer outputs).

The generated corpus is fully deterministic from the seed wires (exhaustive
0..=2-byte inputs, strict prefixes, bit flips over first/last 32 bytes, fixed
final-stream cases), so the test rebuilds every input and replays it against
these recorded expectations. Research identity: oracle-cases.jsonl sha256
4629de7b1cc02382595ce445bfd989f244c5e08ff6bc7329c1f40a78d9294063.
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path


def main() -> int:
    source = Path(sys.argv[1])
    out = Path(sys.argv[2])
    seeds = []
    expectations = {}
    groups: dict[str, dict[str, int]] = {}
    for line in source.read_text(encoding="utf-8").splitlines():
        row = json.loads(line)
        group = row["group"]
        counter = groups.setdefault(group, {"total": 0, "valid": 0})
        counter["total"] += 1
        if row["valid"]:
            counter["valid"] += 1
            output = bytes.fromhex(row["output_hex"])
            if len(output) <= 64:
                expectations[row["name"]] = {"len": len(output), "hex": row["output_hex"]}
            else:
                expectations[row["name"]] = {
                    "len": len(output),
                    "sha256": hashlib.sha256(output).hexdigest(),
                }
        if group == "seed":
            seeds.append({
                "name": row["name"],
                "wire_hex": row["wire_hex"],
                "valid": row["valid"],
            })
    summary = {
        "case_count": sum(c["total"] for c in groups.values()),
        "groups": groups,
        "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
    }
    (out / "boundary-seeds.json").write_text(
        json.dumps(seeds, indent=1) + "\n", encoding="utf-8", newline="\n"
    )
    (out / "boundary-expectations.json").write_text(
        json.dumps({"summary": summary, "cases": expectations}, indent=1) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(json.dumps(summary, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
