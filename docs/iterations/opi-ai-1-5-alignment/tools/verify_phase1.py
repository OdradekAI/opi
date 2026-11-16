"""Verify frozen source identities and the phase-one mapping, using stdlib only."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def check_manifest(root, record, errors, discover=None):
    paths = [r["path"] for r in record["files"]]
    if paths != sorted(set(paths)):
        errors.append(f"{root}: manifest is not unique and sorted")
    content = "".join(f"{r['sha256']}  {r['path']}\n" for r in record["files"]).encode("utf-8")
    if digest(content) != record["sha256"]:
        errors.append(f"{root}: manifest aggregate mismatch")
    if len(paths) != record["file_count"]:
        errors.append(f"{root}: manifest count mismatch")
    if discover is not None and set(paths) != set(discover):
        errors.append(f"{root}: file-set drift")
    for row in record["files"]:
        file = root / row["path"]
        if not file.is_file() or digest(file.read_bytes()) != row["sha256"]:
            errors.append(f"source drift: {file}")


def unique(rows, key, label, errors):
    values = [r[key] for r in rows]
    if len(values) != len(set(values)):
        errors.append(f"duplicate {label}")


def check_index(repo, index, errors):
    contracts = {c["id"] for c in index["contracts"]}
    unique(index["contracts"], "id", "contract IDs", errors)
    for name in ("upstream_symbols", "rust_declarations", "test_declarations"):
        unique(index[name], "id", name, errors)
        frozen = index["extraction"]["declaration_sets"][name]
        ids = "".join(i + "\n" for i in sorted(r["id"] for r in index[name])).encode("utf-8")
        if len(index[name]) != frozen["count"] or digest(ids) != frozen["ids_sha256"]:
            errors.append(f"declaration-set drift: {name}")
    for name, frozen in index["extraction"]["navigation_sets"].items():
        unique(index[name], frozen["key"], name, errors)
        keys = "".join(i + "\n" for i in sorted(r[frozen["key"]] for r in index[name])).encode("utf-8")
        if len(index[name]) != frozen["count"] or digest(keys) != frozen["keys_sha256"]:
            errors.append(f"navigation-set drift: {name}")
    unique(index["source_files"], "path", "source files", errors)
    unique(index["test_files"], "path", "test files", errors)
    source_paths = {r["path"] for r in index["source_files"]}
    all_test_paths = {r["path"] for r in index["test_files"]}
    for row in index["source_files"] + index["test_files"]:
        ids = row.get("candidate_contract_ids", [])
        if not ids or set(ids) - contracts:
            errors.append(f"invalid/missing source/test ownership: {row['path']}")
        path = repo / row["path"]
        if not path.exists() or digest(path.read_bytes()) != row["sha256"]:
            errors.append(f"inventory file drift: {row['path']}")
    for row in index["upstream_symbols"] + index["rust_declarations"]:
        if not row.get("candidate_contract_ids") or set(row["candidate_contract_ids"]) - contracts:
            errors.append(f"unassigned symbol/declaration: {row['id']}")
    for row in index["test_declarations"]:
        if row["path"] not in source_paths | all_test_paths:
            errors.append(f"orphan test: {row['id']}")
        if not row.get("file_candidate_contract_ids") or set(row["file_candidate_contract_ids"]) - contracts:
            errors.append(f"unassigned test declaration: {row['id']}")
        # A file-family mapping must never be upgraded to pass/coverage here.
        if row["execution"] != "not-run":
            errors.append(f"unexpected product test execution claim: {row['id']}")
    for row in index["consumers"]:
        if not row["contract_ids"] or set(row["contract_ids"]) - contracts:
            errors.append(f"unassigned consumer: {row['path']}")
    rebase = index["acceptance_rebase_and_observation_design"]
    unique(rebase["clauses"], "id", "acceptance clauses", errors)
    for row in rebase["clauses"]:
        if not row["contract_ids"] or set(row["contract_ids"]) - contracts:
            errors.append(f"invalid clause ownership: {row['id']}")
    original = rebase["original_standard"]
    archived = repo / "docs/iterations/opi-ai-1-5-alignment" / original["archived_input"]
    if digest(archived.read_bytes()) != original["sha256"]:
        errors.append("original acceptance input drift")
    lines = archived.read_text(encoding="utf-8").splitlines()
    original_ids = {m[1] for line in lines if (m := re.match(r"^\|\s*([A-Z]\d{1,2})\b", line))}
    if {r["id"] for r in rebase["clauses"]} != original_ids:
        errors.append("acceptance clause set mismatch")
    for row in rebase["clauses"]:
        source_row = lines[row["source_line"] - 1]
        if row["source_row"] != source_row or digest(source_row.encode("utf-8")) != row["source_row_sha256"]:
            errors.append(f"acceptance clause input mismatch: {row['id']}")
    expected = {
        ".pi/pi-1.5.0/" + p.relative_to(repo / ".pi/pi-1.5.0").as_posix()
        for p in (repo / ".pi/pi-1.5.0/packages/ai/src").rglob("*.ts")}
    actual = {r["path"] for r in index["source_files"] if r["role"] == "ts-source"}
    if actual != expected:
        errors.append("upstream source inventory mismatch")
    expected = {p.relative_to(repo).as_posix() for p in (repo / "crates/opi-ai/src").rglob("*.rs")}
    if {r["path"] for r in index["source_files"] if r["role"] == "rust-source"} != expected:
        errors.append("Rust source inventory mismatch")
    expected = {p.relative_to(repo).as_posix() for p in (repo / ".pi/pi-1.5.0/packages/ai/test").rglob("*.ts")}
    expected |= {p.relative_to(repo).as_posix() for p in (repo / "crates/opi-ai/tests").rglob("*.rs")}
    if all_test_paths != expected:
        errors.append("test file inventory mismatch")
    if index["extraction"]["syntax_errors"] or index["extraction"]["unresolved_reexports"]:
        errors.append("unresolved extraction errors")
    # Validate explicit source:line evidence throughout curated records.
    def refs(value):
        if isinstance(value, dict):
            path = value.get("path", "")
            if isinstance(path, str) and path.startswith((".pi/", "crates/", "scripts/")):
                for key in ("line", "end_line"):
                    if isinstance(value.get(key), int):
                        yield path, value[key]
            for v in value.values():
                yield from refs(v)
        elif isinstance(value, list):
            for v in value:
                yield from refs(v)
        elif isinstance(value, str):
            m = re.fullmatch(r"((?:\.pi/|crates/|scripts/)[^\s:]+):(\d+)", value)
            if m:
                yield m[1], int(m[2])
    cache = {}
    for path, line in refs(index):
        if path not in cache:
            file = repo / path
            cache[path] = len(file.read_text(encoding="utf-8").splitlines()) if file.is_file() else 0
        if line < 1 or line > cache[path]:
            errors.append(f"invalid evidence: {path}:{line}")


def _inventory_file(repo, row):
    """Resolve one raw source-inventory row without changing its recorded path."""
    path = Path(row["path"])
    if row["role"] in ("ts-source", "ts-test"):
        return repo / ".pi/pi-1.5.0" / path
    if row["role"] in ("rust-source", "rust-test"):
        return repo / path
    return None


def check_current_inventory(repo, inventory, errors):
    """Validate a fresh source inventory against the current filesystem.

    The phase-one contract index is intentionally historical: it contains
    ownership and line evidence for the original mapping. Later refactors can
    split Rust modules and add tests without making that historical mapping a
    valid current source manifest. This check therefore validates only the
    self-contained facts emitted by ``source_inventory.py``: parser identity,
    exact file set/content hashes, and declaration/entrypoint uniqueness.
    """
    expected_parser_versions = {
        "tree-sitter": "0.25.2",
        "tree-sitter-typescript": "0.23.2",
        "tree-sitter-rust": "0.24.0",
    }
    if inventory.get("schema_version") != 1:
        errors.append("current inventory schema mismatch")
    if inventory.get("parser_versions") != expected_parser_versions:
        errors.append("current parser identity differs")
    if inventory.get("syntax_errors") or inventory.get("unresolved_reexports"):
        errors.append("current extraction has unresolved errors")

    rows = inventory.get("files", [])
    keys = [(row.get("role"), row.get("path")) for row in rows]
    if len(keys) != len(set(keys)):
        errors.append("current inventory file rows are not unique")
    for row in rows:
        path = _inventory_file(repo, row)
        if path is None:
            errors.append(f"unknown current inventory role: {row.get('role')}")
            continue
        if not path.is_file():
            errors.append(f"current inventory missing file: {row['path']}")
            continue
        raw = path.read_bytes()
        if len(raw) != row.get("bytes") or digest(raw) != row.get("sha256"):
            errors.append(f"current inventory content drift: {row['path']}")

    expected = {
        "ts-source": {
            p.relative_to(repo / ".pi/pi-1.5.0").as_posix()
            for p in (repo / ".pi/pi-1.5.0/packages/ai/src").rglob("*.ts")
        },
        "ts-test": {
            p.relative_to(repo / ".pi/pi-1.5.0").as_posix()
            for p in (repo / ".pi/pi-1.5.0/packages/ai/test").rglob("*.ts")
        },
        "rust-source": {
            p.relative_to(repo).as_posix()
            for p in (repo / "crates/opi-ai/src").rglob("*.rs")
        },
        "rust-test": {
            p.relative_to(repo).as_posix()
            for p in (repo / "crates/opi-ai/tests").rglob("*.rs")
        },
    }
    for role, paths in expected.items():
        actual = {row["path"] for row in rows if row.get("role") == role}
        if actual != paths:
            errors.append(f"current {role} file-set drift")

    for collection in ("entrypoints", "upstream_symbols", "rust_declarations", "test_declarations"):
        values = [row.get("id") if collection != "entrypoints" else row.get("entrypoint")
                  for row in inventory.get(collection, [])]
        if len(values) != len(set(values)):
            errors.append(f"duplicate current {collection}")


def verify_current(repo, baseline, inventory):
    """Verify a separately captured current baseline and raw inventory.

    This deliberately does not call ``check_index`` or the historical Pi
    observation checks. Those checks remain available through the default
    historical mode and retain the original phase-one contract boundary.
    """
    errors = []
    upstream = repo / baseline["upstream"]["root"]
    actual_upstream = [p.relative_to(upstream).as_posix() for p in upstream.rglob("*")
                       if p.is_file() and not {".git", "node_modules"}.intersection(p.relative_to(upstream).parts)]
    check_manifest(upstream, baseline["upstream"]["manifest"], errors, actual_upstream)
    tracked = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z",
         "crates", "scripts", ".github", ".cargo", "Cargo.toml", "Cargo.lock"],
        cwd=repo,
    )
    actual_rust = [p.decode("utf-8") for p in tracked.split(b"\0")
                   if p and (repo / p.decode("utf-8")).is_file()]
    check_manifest(repo, baseline["rust"]["source_manifest"], errors, actual_rust)
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip()
    if head != baseline["rust"]["head"]["stdout"]:
        errors.append("current Rust HEAD drift")
    check_current_inventory(repo, inventory, errors)
    return errors


def verify(repo, baseline, index):
    errors = []
    upstream = repo / baseline["upstream"]["root"]
    actual = [p.relative_to(upstream).as_posix() for p in upstream.rglob("*")
              if p.is_file() and not {".git", "node_modules"}.intersection(p.relative_to(upstream).parts)]
    check_manifest(upstream, baseline["upstream"]["manifest"], errors, actual)
    tracked = subprocess.check_output(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z", "crates", "scripts", ".github", ".cargo", "Cargo.toml", "Cargo.lock"], cwd=repo)
    actual_rust = [p.decode("utf-8") for p in tracked.split(b"\0") if p and (repo / p.decode("utf-8")).is_file()]
    check_manifest(repo, baseline["rust"]["source_manifest"], errors, actual_rust)
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip()
    if head != baseline["rust"]["head"]["stdout"]:
        errors.append("Rust HEAD drift")
    check_index(repo, index, errors)
    folder = repo / "docs/iterations/opi-ai-1-5-alignment"
    observation_file = folder / "evidence/phase1-observations.json"
    if observation_file.is_file():
        observation = json.loads(observation_file.read_text(encoding="utf-8"))
        if digest((folder / "tools/observe_upstream.mjs").read_bytes()) != observation["driverSha256"]:
            errors.append("source observation driver drift")
        for row in observation["sources"]:
            if digest((upstream / row["path"]).read_bytes()) != row["sha256"]:
                errors.append(f"observed source drift: {row['path']}")
        unique(observation["cases"], "id", "observation cases", errors)
        if len(observation["cases"]) != 9 or any(r["exitCode"] != 0 or r["sourceAssertions"] != "passed" for r in observation["cases"]):
            errors.append("incomplete source observations")
    else:
        errors.append("source observation evidence missing")
    return errors


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--repo", type=Path, required=True)
    p.add_argument("--baseline", type=Path, default=Path(__file__).resolve().parent.parent / "evidence/phase1-baseline.json")
    p.add_argument("--index", type=Path, default=Path(__file__).resolve().parent.parent / "contract-index.json")
    p.add_argument("--inventory", type=Path, help="Optional fresh raw extraction from source_inventory.py")
    p.add_argument("--mode", choices=("historical", "current"), default="historical",
                    help="Use the frozen phase-one contract (historical) or a separately captured current snapshot")
    args = p.parse_args()
    baseline = json.loads(args.baseline.read_text(encoding="utf-8"))
    if args.mode == "current":
        if not args.inventory:
            p.error("--inventory is required with --mode current")
        inventory = json.loads(args.inventory.read_text(encoding="utf-8"))
        errors = verify_current(args.repo.resolve(), baseline, inventory)
    else:
        index = json.loads(args.index.read_text(encoding="utf-8"))
        errors = verify(args.repo.resolve(), baseline, index)
    if args.inventory and args.mode == "historical":
        index = json.loads(args.index.read_text(encoding="utf-8"))
        raw = json.loads(args.inventory.read_text(encoding="utf-8"))
        if raw["syntax_errors"] or raw["unresolved_reexports"]:
            errors.append("fresh extraction has unresolved errors")
        for key in ("upstream_symbols", "rust_declarations", "test_declarations"):
            fresh_ids = {(".pi/pi-1.5.0/" + r["id"] if r["id"].startswith("packages/") else r["id"]) for r in raw[key]}
            if fresh_ids != {r["id"] for r in index[key]}:
                errors.append(f"fresh extraction differs: {key}")
        if raw["parser_versions"] != index["extraction"]["parser_versions"]:
            errors.append("fresh parser identity differs")
        if {r["entrypoint"] for r in raw["entrypoints"]} != {r["entrypoint"] for r in index["entrypoints"]}:
            errors.append("fresh extraction differs: entrypoints")
    if errors:
        print("FAIL\n" + "\n".join(errors))
        raise SystemExit(1)
    if args.mode == "current":
        print(f"PASS: current source identities; {len(inventory['upstream_symbols'])} source-projected exports; "
              f"{len(inventory['rust_declarations'])} Rust declarations; {len(inventory['test_declarations'])} test declarations; "
              "current file/content hashes and parser identity.")
    else:
        print(f"PASS: source identities; {len(index['contracts'])} families; {len(index['upstream_symbols'])} source-projected exports; "
              f"{len(index['test_declarations'])} test declarations; ownership/links/uniqueness. No product equivalence claim.")


if __name__ == "__main__":
    main()
