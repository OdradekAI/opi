"""Offline packaged-opi-ai family proof for workspace-external consumers.

The generic entry point behind `--family <name>`: it packages `opi-ai` with
Cargo, safely extracts the artifact into a new external scratch, resolves an
ordinary consumer offline, validates its dependency identities/features and
runs the family's public-consumer profiles.

Families:

- ``crypto`` (task 20.14, RDF-23): the `consumer-crypto` template exercises
  the real public TLS/Models/Chat path (trust modes, mTLS, ALPN selection,
  typed negatives) in an ordinary profile and a caller feature-union
  profile that installs a process-global ring provider. Resolved rustls /
  aws-lc-rs / aws-lc-sys versions, sources, checksums and the actual native
  build provenance are recorded in the evidence file.

This is a bounded package proof, not the final SDK distribution acceptance
(task 20.57). There is no registry publication, live model call, patch,
registry server, source replacement or build-time generator. The scratch is
never deleted or reused by this command.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tarfile
import time
import tomllib
from pathlib import Path, PurePosixPath
from pathlib import Path as FsPath
from typing import Any

import run_early_package_proof as shared

HARNESS = shared.HARNESS
REPO = shared.REPO

FAMILIES: dict[str, dict[str, Any]] = {
    "crypto": {
        "consumer": "consumer-crypto",
        "profiles": [
            {"name": "ordinary", "features": []},
            {"name": "caller-union", "features": ["caller-union"]},
        ],
        "expected_checks": 6,
        # Exact resolved identities the consumer graph must pin for the
        # packaged SDK (RDF-23 baseline).
        "graph_requirements": [
            ("rustls", "0.23.40"),
            ("aws-lc-rs", "1.18.2"),
            ("aws-lc-sys", "0.46.0"),
        ],
    },
    "http": {
        "consumer": "consumer-http",
        "profiles": [
            {"name": "ordinary", "features": []},
            {"name": "caller-union", "features": ["caller-union"]},
        ],
        "expected_checks": 6,
        # Exact resolved identities the consumer graph must pin for the
        # packaged SDK (RDF-22 baseline).
        "graph_requirements": [
            ("reqwest", "0.12.28"),
            ("hyper", "1.9.0"),
            ("hyper-util", "0.1.20"),
            ("http", "1.4.0"),
        ],
    },
}


def check_family_graph(
    metadata: dict, package: Path, requirements: list[tuple[str, str]]
) -> dict:
    packages = metadata["packages"]
    sdk = [p for p in packages if p["name"] == "opi-ai"]
    if len(sdk) != 1 or Path(sdk[0]["manifest_path"]).resolve() != package / "Cargo.toml":
        raise ValueError("consumer must compile the extracted opi-ai package")
    features = {node["id"]: node["features"] for node in metadata["resolve"]["nodes"]}
    result = {}
    for name, version in requirements:
        matches = [p for p in packages if p["name"] == name]
        if len(matches) != 1 or matches[0]["version"] != version or not (
            matches[0]["source"] or ""
        ).startswith("registry+"):
            raise ValueError(
                f"expected one registry {name} {version}, found "
                + ", ".join(f"{p['version']}/{p.get('source')}" for p in matches)
            )
        entry = matches[0]
        result[name] = {
            "version": entry["version"],
            "source": entry["source"],
            "checksum": entry.get("checksum"),
            "features": features.get(entry["id"], []),
        }
    return result


def check_family_report(report: dict, family: dict) -> None:
    if not isinstance(report, dict) or not isinstance(report.get("checks"), list):
        raise ValueError("consumer report must be an object with a checks list")
    checks = report["checks"]
    expected = family["expected_checks"]
    if (
        report.get("verdict") != "PASS"
        or report.get("failed") != 0
        or report.get("total") != expected
        or len(checks) != expected
        or len({c["name"] for c in checks if isinstance(c, dict)}) != expected
        or any(c.get("passed") is not True for c in checks)
    ):
        raise ValueError(
            f"consumer did not pass the complete {expected}-check family battery: "
            + json.dumps(report)[:800]
        )


def native_build_provenance(target: Path) -> dict:
    """Summarize the actual aws-lc-sys native build evidence under the
    consumer-used target directory: builder choice, USE_SYSTEM handling and
    the static prefixed archive, plus the absence of a dynamic libcrypto in
    the dependency artifacts."""
    builders: set[str] = set()
    use_system: set[str] = set()
    static_libs: list[str] = []
    for build_dir in sorted(target.glob("debug/build/aws-lc-sys-*")):
        out = build_dir / "out"
        for archive in out.glob("libaws_lc_*.a"):
            static_libs.append(str(archive))
        log_path = build_dir / "output"
        if not log_path.is_file():
            continue
        for log in log_path.read_text(encoding="utf-8", errors="replace").splitlines():
            if log.startswith("cargo:warning=Building with:"):
                builders.add(log.split(":", 2)[2].strip())
            if "'AWS_LC_SYS_USE_SYSTEM'" in log:
                use_system.add(log)
            if log.startswith("cargo:warning=Value:") and " false." in log:
                use_system.add(log)
    dynamic = [
        str(p)
        for p in sorted(target.glob("debug/deps/*"))
        if p.suffix.lower() in (".dll", ".so", ".dylib")
        and "crypto" in p.name.lower()
        and "aws" not in p.name.lower()
        and not p.name.startswith("rustls")
    ]
    return {
        "builders": sorted(builders),
        "use_system_observed": sorted(use_system),
        "static_archives": static_libs,
        "dynamic_libcrypto_artifacts": dynamic,
        "scope": "provenance read from the shared consumer-built target cache",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--family", required=True, choices=sorted(FAMILIES))
    parser.add_argument("--offline", action="store_true", help="required offline mode marker")
    parser.add_argument("--scratch", required=True, help="new workspace-external scratch directory")
    parser.add_argument("--target-dir", default=os.environ.get("CARGO_TARGET_DIR"),
                        help="existing marked workspace cache (or CARGO_TARGET_DIR)")
    args = parser.parse_args()
    if not args.offline:
        parser.error("--offline is required; this proof never resolves from the network")
    if not args.target_dir:
        parser.error("--target-dir or CARGO_TARGET_DIR must identify the existing workspace cache")
    family = FAMILIES[args.family]
    scratch = FsPath(args.scratch).expanduser().resolve()
    target = FsPath(args.target_dir).expanduser().resolve()
    try:
        shared.validate_paths(REPO, scratch, target)
    except (OSError, ValueError) as exc:
        parser.error(str(exc))
    scratch.mkdir(parents=True)
    env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_INCREMENTAL="1")
    env.setdefault("AWS_LC_SYS_USE_SYSTEM", "0")
    env.setdefault("CARGO_BUILD_JOBS", "4")
    evidence: dict[str, Any] = {
        "family": args.family,
        "scope": "packaged public-consumer family proof; not final SDK acceptance",
        "target": str(target),
        "steps": [],
        "profiles": [],
        "verdict": "FAIL",
        "inputs": {
            str(path.relative_to(HARNESS)): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in [
                *sorted((HARNESS / family["consumer"] / "src").glob("*.rs")),
                HARNESS / family["consumer"] / "Cargo.toml",
            ]
        },
    }

    def run(name: str, command: list[str], cwd: Path) -> str:
        started = time.monotonic()
        result = subprocess.run(
            command, cwd=cwd, env=env, capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=3600,
        )
        (scratch / f"{name}.stdout").write_text(result.stdout, encoding="utf-8")
        (scratch / f"{name}.stderr").write_text(result.stderr, encoding="utf-8")
        evidence["steps"].append({
            "name": name, "command": command, "exit": result.returncode,
            "elapsed_seconds": round(time.monotonic() - started, 2),
        })
        if result.returncode:
            raise ValueError(f"{name} failed with exit {result.returncode}; see scratch logs")
        return result.stdout

    lease = [sys.executable, str(REPO / "scripts/opi-cargo-cache.py"), "lease"]
    leased = False
    try:
        run("lease-start", lease + ["start", "--target", str(target), "--pid", str(os.getpid())], REPO)
        leased = True
        version = tomllib.loads(
            (REPO / "Cargo.toml").read_text(encoding="utf-8")
        )["workspace"]["package"]["version"]
        prefix = f"opi-ai-{version}"
        run("package", ["cargo", "package", "-p", "opi-ai", "--allow-dirty", "--no-verify",
                        "--offline", "--locked"], REPO)
        archive = scratch / f"{prefix}.crate"
        archive.write_bytes((target / "package" / archive.name).read_bytes())
        evidence["package"] = {
            "file": archive.name,
            "sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
        }
        package = shared.unpack_package(archive, scratch / "package", prefix)
        consumer = scratch / "consumer"
        shutil.copytree(
            HARNESS / family["consumer"],
            consumer,
            ignore=shutil.ignore_patterns("target", "Cargo.lock"),
        )
        manifest = consumer / "Cargo.toml"
        text = manifest.read_text(encoding="utf-8")
        manifest.write_text(
            text.replace('"@PACKAGE@"', json.dumps(package.as_posix())),
            encoding="utf-8", newline="\n",
        )
        run("consumer-lock", ["cargo", "generate-lockfile", "--offline"], consumer)
        for profile in family["profiles"]:
            flags = ["--features", ",".join(profile["features"])] if profile["features"] else []
            metadata = json.loads(run(
                f"{profile['name']}-metadata",
                ["cargo", "metadata", "--format-version", "1", "--offline", "--locked"] + flags,
                consumer,
            ))
            graph = check_family_graph(metadata, package, family["graph_requirements"])
            report = json.loads(run(
                f"{profile['name']}-consumer",
                ["cargo", "run", "--offline", "--locked"] + flags,
                consumer,
            ))
            check_family_report(report, family)
            evidence["profiles"].append({
                "name": profile["name"],
                "graph": graph,
                "report": report,
            })
        evidence["consumer_lock_sha256"] = hashlib.sha256(
            (consumer / "Cargo.lock").read_bytes()
        ).hexdigest()
        lock = tomllib.loads((consumer / "Cargo.lock").read_text(encoding="utf-8"))
        lock_checksums = {
            p["name"]: p.get("checksum")
            for p in lock.get("package", [])
            if p["name"] in {name for name, _ in family["graph_requirements"]}
        }
        evidence["lock_checksums"] = lock_checksums
        missing = [n for n, c in lock_checksums.items() if not c]
        if missing:
            raise ValueError(f"consumer lock lacks registry checksums for {missing}")
        evidence["native_provenance"] = native_build_provenance(target)
        if not evidence["native_provenance"]["static_archives"]:
            raise ValueError("no static aws-lc archive found in the consumer-built target")
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
        (scratch / "evidence.json").write_text(
            json.dumps(evidence, indent=2) + "\n", encoding="utf-8"
        )
    print(json.dumps({
        "verdict": evidence["verdict"],
        "scratch": str(scratch),
        "error": evidence.get("error"),
    }, indent=2))
    return 0 if evidence["verdict"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
