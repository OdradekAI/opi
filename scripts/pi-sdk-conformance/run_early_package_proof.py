"""Offline packaged opi-ai public-Schema proof for an ordinary Cargo consumer.

Uses the existing marked workspace target cache and Cargo home. The fresh
external scratch holds a cargo-package artifact, its extracted sources, a
consumer and evidence; it is never deleted or reused by this command. This
bounded smoke proof does not complete the final SDK distribution acceptance.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import sys
import tarfile
import time
import tomllib

HARNESS = Path(__file__).resolve().parent
REPO = HARNESS.parents[1]


def validate_paths(repo: Path, scratch: Path, target: Path) -> None:
    if scratch.exists():
        raise ValueError(f"scratch already exists; choose a fresh path: {scratch}")
    if scratch.is_relative_to(repo) or scratch.is_relative_to(target):
        raise ValueError("scratch must be outside the workspace and Cargo target cache")
    marker = target / ".opi-cargo-cache.json"
    if not marker.is_file():
        raise ValueError(f"target must be an existing marked Opi Cargo cache: {target}")
    metadata = json.loads(marker.read_text(encoding="utf-8"))
    if (not isinstance(metadata, dict) or not isinstance(metadata.get("workspace"), str)
            or Path(metadata["workspace"]).resolve() != repo):
        raise ValueError("Cargo cache marker belongs to another workspace")


def unpack_package(archive_path: Path, destination: Path, prefix: str) -> Path:
    with tarfile.open(archive_path, "r:gz") as archive:
        for member in archive.getmembers():
            parts = PurePosixPath(member.name).parts
            if (not parts or parts[0] != prefix or ".." in parts or "\\" in member.name
                    or not (member.isfile() or member.isdir())):
                raise ValueError(f"unsafe or unexpected package member: {member.name}")
        archive.extractall(destination, filter="data")
    package = destination / prefix
    if not (package / "Cargo.toml").is_file():
        raise ValueError("package has no distributable Cargo.toml")
    return package


def check_graph(metadata: dict, package: Path, caller_defaults: bool) -> dict:
    packages = metadata["packages"]
    if any(p["name"].startswith("opi-vendor-") for p in packages):
        raise ValueError("retired vendor package present in consumer graph")
    sdk = [p for p in packages if p["name"] == "opi-ai"]
    if len(sdk) != 1 or Path(sdk[0]["manifest_path"]).resolve() != package / "Cargo.toml":
        raise ValueError("consumer must compile the extracted opi-ai package")
    features = {node["id"]: node["features"] for node in metadata["resolve"]["nodes"]}
    result = {}
    for name, version in [("regress", "0.12.0"), ("url", "2.5.8")]:
        matches = [p for p in packages if p["name"] == name]
        if (len(matches) != 1 or matches[0]["version"] != version
                or not (matches[0]["source"] or "").startswith("registry+")):
            raise ValueError(f"expected one ordinary registry {name} {version}")
        entry = matches[0]
        result[name] = {"version": version, "source": entry["source"],
                        "features": features[entry["id"]]}
    regress_features = set(result["regress"]["features"])
    if not {"std", "utf16", "prohibit-unsafe"}.issubset(regress_features):
        raise ValueError("required regress features missing")
    if ("default" in regress_features) != caller_defaults:
        raise ValueError("caller regress default-feature profile was not exercised")
    if ("backend-pikevm" in regress_features) != caller_defaults:
        raise ValueError("caller regress optional-backend feature union was not exercised")
    result["data_dependencies"] = [
        {"name": p["name"], "version": p["version"], "source": p["source"],
         "features": features.get(p["id"], [])}
        for p in packages if p["name"].startswith(("icu_", "idna", "unicode-"))
    ]
    return result


def check_report(report: dict) -> None:
    if not isinstance(report, dict):
        raise ValueError("consumer report must be an object")
    checks = report.get("checks", [])
    if not isinstance(checks, list) or any(
            not isinstance(check, dict) or not isinstance(check.get("name"), str) for check in checks):
        raise ValueError("consumer report must name every check")
    if (report.get("verdict") != "PASS" or report.get("failed") != 0
            or report.get("total") != 73 or len(checks) != 73
            or len({check["name"] for check in checks}) != 73
            or any(check.get("passed") is not True for check in checks)):
        raise ValueError("consumer did not pass the complete 73-check public battery")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", required=True, help="new workspace-external scratch directory")
    parser.add_argument("--target-dir", default=os.environ.get("CARGO_TARGET_DIR"),
                        help="existing marked workspace cache (or CARGO_TARGET_DIR)")
    args = parser.parse_args()
    if "ICU4X_DATA_DIR" in os.environ:
        parser.error("ICU4X_DATA_DIR must be unset to verify upstream bundled data")
    if not args.target_dir:
        parser.error("--target-dir or CARGO_TARGET_DIR must identify the existing workspace cache")
    scratch = Path(args.scratch).expanduser().resolve()
    target = Path(args.target_dir).expanduser().resolve()
    try:
        validate_paths(REPO, scratch, target)
    except (OSError, ValueError) as exc:
        parser.error(str(exc))
    scratch.mkdir(parents=True)
    env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_INCREMENTAL="1")
    env.setdefault("CARGO_BUILD_JOBS", "4")
    env.setdefault("CARGO_PROFILE_DEV_DEBUG", "0")
    env.setdefault("CARGO_PROFILE_TEST_DEBUG", "0")
    evidence = {"scope": "packaged public Schema smoke; not final SDK acceptance",
                "target": str(target), "incremental": "1", "steps": [], "profiles": [],
                "verdict": "FAIL", "inputs": {
                    str(path.relative_to(HARNESS)): hashlib.sha256(path.read_bytes()).hexdigest()
                    for path in [HARNESS / "current-profile.json", HARNESS / "consumer/src/main.rs",
                                 *sorted((HARNESS / "frozen").glob("*.json"))]
                }}

    def run(name: str, command: list[str], cwd: Path) -> str:
        started = time.monotonic()
        result = subprocess.run(command, cwd=cwd, env=env, capture_output=True,
                                text=True, encoding="utf-8", errors="replace", timeout=3600)
        (scratch / f"{name}.stdout").write_text(result.stdout, encoding="utf-8")
        (scratch / f"{name}.stderr").write_text(result.stderr, encoding="utf-8")
        evidence["steps"].append({"name": name, "command": command, "exit": result.returncode,
                                  "elapsed_seconds": round(time.monotonic() - started, 2)})
        if result.returncode:
            raise ValueError(f"{name} failed with exit {result.returncode}; see scratch logs")
        return result.stdout

    lease = [sys.executable, str(REPO / "scripts/opi-cargo-cache.py"), "lease"]
    leased = False
    try:
        run("lease-start", lease + ["start", "--target", str(target), "--pid", str(os.getpid())], REPO)
        leased = True
        version = tomllib.loads((REPO / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
        prefix = f"opi-ai-{version}"
        run("package", ["cargo", "package", "-p", "opi-ai", "--allow-dirty", "--no-verify",
                        "--offline", "--locked"], REPO)
        archive = scratch / f"{prefix}.crate"
        shutil.copyfile(target / "package" / archive.name, archive)
        evidence["package"] = {"file": archive.name,
                               "sha256": hashlib.sha256(archive.read_bytes()).hexdigest()}
        package = unpack_package(archive, scratch / "package", prefix)
        consumer = scratch / "consumer"
        shutil.copytree(HARNESS / "consumer", consumer)
        shutil.copytree(HARNESS / "frozen", consumer / "frozen")
        shutil.copyfile(HARNESS / "current-profile.json", consumer / "current-profile.json")
        manifest = consumer / "Cargo.toml"
        text = manifest.read_text(encoding="utf-8")
        manifest.write_text(text.replace('"@PACKAGE@"', json.dumps(package.as_posix())),
                            encoding="utf-8", newline="\n")
        run("consumer-lock", ["cargo", "generate-lockfile", "--offline"], consumer)
        for name, caller_defaults in [("defaults-off", False), ("caller-defaults-on", True)]:
            flags = ["--features", "caller-default-regress"] if caller_defaults else []
            metadata = json.loads(run(f"{name}-metadata", ["cargo", "metadata", "--format-version", "1",
                                                       "--offline", "--locked"] + flags, consumer))
            graph = check_graph(metadata, package, caller_defaults)
            report = json.loads(run(f"{name}-consumer", ["cargo", "run", "--offline", "--locked"]
                                    + flags, consumer))
            evidence["profiles"].append({"name": name, "graph": graph, "report": report})
            check_report(report)
        evidence["consumer_lock_sha256"] = hashlib.sha256((consumer / "Cargo.lock").read_bytes()).hexdigest()
        evidence["verdict"] = "PASS"
    except (OSError, ValueError, subprocess.TimeoutExpired, tarfile.TarError) as exc:
        evidence["error"] = str(exc)
    finally:
        if leased:
            try:
                run("lease-end", lease + ["end", "--target", str(target), "--pid", str(os.getpid())], REPO)
            except (OSError, ValueError, subprocess.TimeoutExpired) as exc:
                evidence["error"] = str(exc)
                evidence["verdict"] = "FAIL"
        (scratch / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"verdict": evidence["verdict"], "scratch": str(scratch),
                      "error": evidence.get("error")}, indent=2))
    return 0 if evidence["verdict"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
