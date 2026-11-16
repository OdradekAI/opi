"""Consume the packaged embedded catalog from an external core-only caller."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

import package_slice
import run_early_package_proof as shared


HARNESS = Path(__file__).resolve().parent
REPO = HARNESS.parents[1]
FORBIDDEN = {
    "reqwest",
    "rustls",
    "rustls-native-certs",
    "aws-lc-rs",
    "aws-lc-sys",
    "aws-smithy-query",
    "aws-smithy-xml",
    "p256",
    "hmac",
    "sha1",
    "xmlparser",
    "libz-rs-sys",
    "flate2",
}


def check_graph(metadata: dict, package: Path) -> dict:
    packages = metadata["packages"]
    sdk = [entry for entry in packages if entry["name"] == "opi-ai"]
    if len(sdk) != 1 or Path(sdk[0]["manifest_path"]).resolve() != package / "Cargo.toml":
        raise ValueError("consumer must resolve the extracted opi-ai package")
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    sdk_features = set(nodes[sdk[0]["id"]]["features"])
    if sdk_features != {"catalog-bundled"}:
        raise ValueError(f"unexpected catalog-only feature union: {sorted(sdk_features)}")
    leaked = FORBIDDEN.intersection(entry["name"] for entry in packages)
    if leaked:
        raise ValueError(f"catalog-only consumer contains forbidden backend: {sorted(leaked)}")
    local = {entry["id"] for entry in packages if entry.get("source") is None}
    allowed = {sdk[0]["id"], metadata["resolve"]["root"]}
    telemetry = [entry for entry in packages if entry["name"] == "opi-telemetry"]
    if len(telemetry) != 1:
        raise ValueError("catalog-only consumer must resolve one telemetry dependency")
    allowed.add(telemetry[0]["id"])
    if local != allowed:
        raise ValueError(f"unexpected local dependency closure: {sorted(local - allowed)}")
    return {
        "features": sorted(sdk_features),
        "forbidden_absent": sorted(FORBIDDEN),
        "packages": sorted(entry["name"] + "@" + entry["version"] for entry in packages),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--target-dir", required=True)
    parser.add_argument("--evidence-output", required=True)
    args = parser.parse_args()
    scratch = Path(args.scratch).expanduser().resolve()
    target = Path(args.target_dir).expanduser().resolve()
    output = Path(args.evidence_output).expanduser().resolve()
    if output.exists():
        parser.error("evidence already exists; select a new path")
    try:
        shared.validate_paths(REPO, scratch, target)
        configs = package_slice.check_cargo_configs(
            package_slice.controlled_environment(dict(os.environ), target),
            [REPO, scratch / "consumer", scratch / "package"],
        )
    except (OSError, ValueError) as exc:
        parser.error(str(exc))
    scratch.mkdir(parents=True)
    env = package_slice.controlled_environment(dict(os.environ), target)
    env.setdefault("CARGO_BUILD_JOBS", "4")
    env["CARGO_TARGET_DIR"] = str(target)
    evidence = {
        "family": "catalog-bundled-package",
        "scope": "external consumer of the catalog-bundled feature from the actual opi-ai archive",
        "target": str(target),
        "cargo_configs": configs,
        "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "steps": [],
        "verdict": "FAIL",
    }
    candidate = None

    def run(name: str, command: list[str], cwd: Path) -> str:
        nonlocal candidate
        if candidate is not None:
            command = candidate.command(command)
        started = time.monotonic()
        result = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=3600,
        )
        paths = {}
        for stream in ("stdout", "stderr"):
            path = scratch / f"{name}.{stream}"
            path.write_text(getattr(result, stream), encoding="utf-8", newline="\n")
            paths[stream] = {
                "path": str(path),
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            }
        evidence["steps"].append(
            {
                "name": name,
                "command": command,
                "exit": result.returncode,
                "elapsed_seconds": round(time.monotonic() - started, 2),
                **paths,
            }
        )
        if result.returncode:
            raise ValueError(f"{name} failed with exit {result.returncode}")
        if candidate is not None and command[0] == "cargo" and "metadata" in command:
            candidate.verify_graph(json.loads(result.stdout))
        return result.stdout

    lease = [sys.executable, str(REPO / "scripts/opi-cargo-cache.py"), "lease"]
    leased = False
    try:
        run("rustc-version", ["rustc", "-vV"], REPO)
        run("cargo-version", ["cargo", "-vV"], REPO)
        run("lease-start", lease + ["start", "--target", str(target), "--pid", str(os.getpid())], REPO)
        leased = True
        package, evidence["package"], candidate = shared.package_sdk(run, scratch, target)
        consumer = scratch / "consumer"
        (consumer / "src").mkdir(parents=True)
        manifest = (
            '[package]\nname = "opi-catalog-bundled-consumer"\nversion = "0.0.0"\n'
            'edition = "2024"\n[workspace]\n[dependencies]\n'
            f'opi-ai = {{ path = {json.dumps(package.as_posix())}, default-features = false, '
            'features = ["catalog-bundled"] }\n'
        )
        (consumer / "Cargo.toml").write_text(manifest, encoding="utf-8", newline="\n")
        (consumer / "src/main.rs").write_text(
            """fn main() {
    let catalog = opi_ai::catalog::Catalog::builtin().expect("embedded catalog loads");
    assert_eq!(catalog.provenance().counts.chat, 1608);
    let (catalog_bytes, manifest_bytes) = opi_ai::catalog::Catalog::builtin_bundle_bytes();
    let reparsed = opi_ai::catalog::Catalog::from_bytes(catalog_bytes, manifest_bytes)
        .expect("embedded bytes revalidate");
    assert_eq!(reparsed.provenance().revision, catalog.provenance().revision);
    println!("catalog={} chat={}", catalog.provenance().revision, catalog.provenance().counts.chat);
}
""",
            encoding="utf-8",
            newline="\n",
        )
        run("consumer-lock", ["cargo", "generate-lockfile", "--offline"], consumer)
        metadata = json.loads(run(
            "catalog-bundled-metadata",
            ["cargo", "metadata", "--format-version", "1", "--offline", "--locked"],
            consumer,
        ))
        evidence["graph"] = check_graph(metadata, package)
        run("catalog-bundled-build", ["cargo", "build", "--offline", "--locked"], consumer)
        output_text = run("catalog-bundled-run", ["cargo", "run", "--offline", "--locked"], consumer)
        if "chat=1608" not in output_text:
            raise ValueError("external consumer did not observe the pinned catalog count")
        evidence["consumer_sources"] = {
            "Cargo.toml": hashlib.sha256((consumer / "Cargo.toml").read_bytes()).hexdigest(),
            "src/main.rs": hashlib.sha256((consumer / "src/main.rs").read_bytes()).hexdigest(),
            "Cargo.lock": hashlib.sha256((consumer / "Cargo.lock").read_bytes()).hexdigest(),
        }
        evidence["verdict"] = "PASS"
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        evidence["error"] = str(exc)
    finally:
        if leased:
            try:
                run("lease-end", lease + ["end", "--target", str(target), "--pid", str(os.getpid())], REPO)
            except (OSError, ValueError, subprocess.SubprocessError) as exc:
                evidence["verdict"] = "FAIL"
                evidence["lease_error"] = str(exc)
        encoded = json.dumps(evidence, indent=2) + "\n"
        (scratch / "evidence.json").write_text(encoded, encoding="utf-8", newline="\n")
        output.parent.mkdir(parents=True, exist_ok=True)
        with output.open("x", encoding="utf-8", newline="\n") as handle:
            handle.write(encoded)
    return 0 if evidence["verdict"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
