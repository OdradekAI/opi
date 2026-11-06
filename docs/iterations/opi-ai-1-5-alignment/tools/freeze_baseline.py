"""Capture source identities without modifying the source or resolving dependencies.

Output is exclusive-create: a later capture must use a new name, never overwrite
the accepted baseline after a failure. The companion verifier detects drift.
"""
import argparse
import datetime
import hashlib
import json
import locale
import os
from pathlib import Path
import platform
import subprocess
import sys
import time
import tomllib


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def manifest(root, paths):
    rows = [{"path": str(p).replace("\\", "/"), "sha256": digest((root / p).read_bytes()),
             "bytes": (root / p).stat().st_size} for p in sorted(paths)]
    content = "".join(f"{r['sha256']}  {r['path']}\n" for r in rows).encode("utf-8")
    return {"algorithm": "sorted POSIX paths; SHA256(raw bytes) + two spaces + path + LF; SHA256(UTF-8 manifest)",
            "sha256": digest(content), "file_count": len(rows), "files": rows}


def command(repo, argv):
    result = subprocess.run(argv, cwd=repo, capture_output=True, text=True, encoding="utf-8", errors="replace", check=False)
    return {"command": argv, "exit_code": result.returncode, "stdout": result.stdout.strip(), "stderr": result.stderr.strip()}


def capture(repo):
    snapshot = repo / ".pi/pi-1.5.0"
    paths = [p.relative_to(snapshot).as_posix() for p in snapshot.rglob("*")
             if p.is_file() and not {".git", "node_modules"}.intersection(p.relative_to(snapshot).parts)]
    root_package = json.loads((snapshot / "package.json").read_text(encoding="utf-8"))
    ai_package = json.loads((snapshot / "packages/ai/package.json").read_text(encoding="utf-8"))
    lock = json.loads((snapshot / "package-lock.json").read_text(encoding="utf-8"))
    rust_lock = tomllib.loads((repo / "Cargo.lock").read_text(encoding="utf-8"))
    rust_root = tomllib.loads((repo / "Cargo.toml").read_text(encoding="utf-8"))
    rust_ai = tomllib.loads((repo / "crates/opi-ai/Cargo.toml").read_text(encoding="utf-8"))
    tracked = subprocess.check_output(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z", "crates", "scripts", ".github", ".cargo", "Cargo.toml", "Cargo.lock"], cwd=repo)
    rust_paths = [p.decode("utf-8") for p in tracked.split(b"\0") if p and (repo / p.decode("utf-8")).is_file()]
    direct = {}
    for dep, declared in ai_package["dependencies"].items():
        matches = {p: v for p, v in lock["packages"].items() if p == "node_modules/" + dep or p.endswith("/node_modules/" + dep)}
        direct[dep] = {"declared": declared, "lock_records": matches}
    rust_direct = {}
    for dep, declared in rust_ai["dependencies"].items():
        root = rust_root["workspace"]["dependencies"].get(dep)
        rust_direct[dep] = {"crate_declaration": declared, "workspace_declaration": root,
                            "lock_records": [p for p in rust_lock["package"] if p["name"] == dep]}
    return {
        "schema_version": 1, "captured_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "scope": "phase-one source baseline; not a release artifact or full runtime acceptance identity",
        "upstream": {"root": ".pi/pi-1.5.0", "git_identity": "snapshot has no .git; content identity is authoritative",
                     "version": ai_package["version"], "manifest": manifest(snapshot, paths),
                     "node_required": ai_package["engines"], "build_entrypoints": ai_package["scripts"],
                     "package_exports": ai_package["exports"], "package_bin": ai_package["bin"],
                     "root_dev_dependencies": root_package["devDependencies"],
                     "lockfile_version": lock["lockfileVersion"], "lock_package_count": len(lock["packages"]),
                     "direct_dependencies": direct,
                     "model_catalog_pin": json.loads((snapshot / "nix/model-catalog.json").read_text(encoding="utf-8")),
                     "material_presence": {p: (snapshot / p).exists() for p in (
                         "node_modules", "packages/ai/dist", "packages/ai/src/providers/data", "packages/schema/dist", "packages/telemetry/dist")}},
        "rust": {"head": command(repo, ["git", "rev-parse", "HEAD"]),
                 "workspace_package": rust_root["workspace"]["package"],
                 "source_manifest": manifest(repo, rust_paths),
                 "dependencies": rust_direct,
                 "target_dependencies": rust_ai.get("target", {}),
                 "features_declared": rust_ai.get("features", {}),
                 "lock_package_count": len(rust_lock["package"]),
                 "build_profile": "not built in phase one", "artifact": "not produced",
                 "toolchain_file": [p for p in ("rust-toolchain.toml", "rust-toolchain") if (repo / p).exists()]},
        "host": {"system": platform.system(), "release": platform.release(), "version": platform.version(),
                 "machine": platform.machine(), "timezone": time.tzname, "locale": locale.getlocale(),
                 "python": sys.version, "node": command(repo, ["node", "--version"]),
                 "rustc": command(repo, ["rustc", "--version", "--verbose"]), "cargo": command(repo, ["cargo", "--version"]),
                 "proxy_variable_presence_only": {n: n in os.environ for n in ("HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "NO_PROXY")},
                 "credentials": "not read or captured; source probes run with an explicit clean child environment",
                 "observation_clock_random": "event-loop barriers; no random inputs; timestamp=0 in synthetic messages avoids duration normalization"},
        "execution_state": {"sdk_install": "absent; declared/locked versions are not installed version evidence",
                            "msrv": "declared 1.97; host 1.99.1 is not an MSRV test",
                            "product_tests": "not-run", "rust_comparison": "not-run", "package_build": "not-run"}}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--repo", type=Path, required=True)
    p.add_argument("--output", type=Path, required=True)
    args = p.parse_args()
    # Refuse an existing target before expensive scanning.
    if args.output.exists():
        p.error("output already exists; choose a new capture path")
    result = capture(args.repo.resolve())
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("x", encoding="utf-8", newline="\n") as out:
        json.dump(result, out, ensure_ascii=False, indent=2)
        out.write("\n")
    print(json.dumps({"upstream": result["upstream"]["manifest"]["sha256"],
                      "upstream_files": result["upstream"]["manifest"]["file_count"],
                      "rust_files": result["rust"]["source_manifest"]["file_count"]}))


if __name__ == "__main__":
    main()
